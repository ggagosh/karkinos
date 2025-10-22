use anyhow::{Context, Result as AnyhowResult};
use clap::Parser;
use rayon::prelude::*;
use regex::Regex;
use scraper::Html;
use sha2::{Digest, Sha256};
use std::fs::{create_dir_all, read_to_string, File};
use std::io::{stdout, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use validator::Validate;

use crate::types::{DataConfig, ItemConfig, ReturnedData, ReturnedDataItem, ScrapeRoot};

mod types;

/// CLI application to scrape website based on yml config 🦀
/// Inspired by: https://github.com/IonicaBizau/scrape-it ❤️
#[derive(Parser)]
#[clap(version, about, verbatim_doc_comment, long_about = None)]
struct Cli {
    /// Config file location
    #[clap(parse(from_os_str), last = false)]
    input: PathBuf,

    /// Output file location
    #[clap(parse(from_os_str), short, long)]
    output: Option<PathBuf>,

    /// Output format: json, csv
    #[clap(short, long, default_value = "json")]
    format: String,
}

fn generate_paginated_urls(
    base_url: &str,
    pagination: &types::PaginationConfig,
    config: &types::ScrapeRootConfig,
    data_config: &DataConfig,
) -> AnyhowResult<Vec<String>> {
    let mut urls = Vec::new();

    // Strategy 1: URL pattern with page numbers
    if let Some(pattern) = &pagination.page_pattern {
        log::info!("Using URL pattern pagination strategy");
        let start = pagination.start_page;
        let end = if let Some(end_page) = pagination.end_page {
            log::debug!("End page specified: {}", end_page);
            end_page
        } else if pagination.max_pages > 0 {
            log::debug!("Max pages specified: {}", pagination.max_pages);
            start + pagination.max_pages - 1
        } else {
            log::debug!("Using default: 10 pages");
            start + 9 // Default to 10 pages if no limit specified
        };

        log::info!(
            "Pages: {} to {} (total: {} pages)",
            start,
            end,
            end - start + 1
        );
        if pagination.stop_on_empty {
            log::info!("Stop on empty: enabled");
        }

        for page_num in start..=end {
            let url = if pattern.contains("{page}") {
                pattern.replace("{page}", &page_num.to_string())
            } else {
                format!(
                    "{}{}",
                    base_url,
                    pattern.replace("{page}", &page_num.to_string())
                )
            };

            // If base_url is not in the pattern, prepend it
            let full_url = if url.starts_with("http") {
                url
            } else {
                format!("{}{}", base_url, url)
            };

            log::debug!("Generated URL for page {}: {}", page_num, full_url);
            urls.push(full_url);

            // Check if we should stop on empty (fetch and check)
            if pagination.stop_on_empty && page_num > start {
                // Fetch the page and check if it has results
                if let Ok(html) = fetch_with_config(&urls[urls.len() - 1], config) {
                    let data = populate_values(html, data_config.clone());
                    if is_empty_result(&data) {
                        eprintln!("⚠️  Empty page at page {}, stopping", page_num);
                        urls.pop(); // Remove the empty page
                        break;
                    }
                }
            }
        }
    }
    // Strategy 2: Follow "next" links
    else if let Some(next_selector) = &pagination.next_selector {
        log::info!("Using 'next link' pagination strategy");
        log::info!("Next link selector: {}", next_selector);

        let mut current_url = base_url.to_string();
        let mut page_count = 0;
        let max_pages = if pagination.max_pages > 0 {
            log::info!("Max pages limit: {}", pagination.max_pages);
            pagination.max_pages
        } else {
            log::debug!("Using safety limit: 1000 pages");
            1000 // Safety limit
        };

        if pagination.stop_on_empty {
            log::info!("Stop on empty: enabled");
        }

        urls.push(current_url.clone());
        page_count += 1;
        log::debug!("Page {}: {}", page_count, current_url);

        while page_count < max_pages {
            // Fetch current page
            log::debug!("Fetching page {} to find next link...", page_count);
            let html = fetch_with_config(&current_url, config)?;

            // Check if we should stop on empty
            if pagination.stop_on_empty {
                let data = populate_values(html.clone(), data_config.clone());
                if is_empty_result(&data) {
                    eprintln!("⚠️  Empty page at page {}, stopping", page_count);
                    break;
                }
            }

            // Find next link
            let parsed_html = Html::parse_document(&html);
            let selector = scraper::Selector::parse(next_selector)
                .map_err(|e| anyhow::anyhow!("Invalid next_selector: {:?}", e))?;

            if let Some(next_element) = parsed_html.select(&selector).next() {
                if let Some(next_href) = next_element.value().attr("href") {
                    // Make absolute URL if needed
                    current_url = if next_href.starts_with("http") {
                        next_href.to_string()
                    } else if next_href.starts_with("/") {
                        // Extract base domain from current_url
                        let base = current_url
                            .split('/')
                            .take(3)
                            .collect::<Vec<&str>>()
                            .join("/");
                        format!("{}{}", base, next_href)
                    } else {
                        // Relative URL
                        let base = current_url
                            .rsplit_once('/')
                            .map(|(b, _)| b)
                            .unwrap_or(&current_url);
                        format!("{}/{}", base, next_href)
                    };

                    urls.push(current_url.clone());
                    page_count += 1;
                    log::debug!("Page {}: {}", page_count, current_url);

                    // Rate limiting between pagination requests
                    if config.delay > 0 {
                        thread::sleep(Duration::from_millis(config.delay));
                    }
                } else {
                    eprintln!("⚠️  Next link has no href, stopping at page {}", page_count);
                    break; // No href attribute
                }
            } else {
                log::info!(
                    "No more 'next' links found, stopping at page {}",
                    page_count
                );
                break; // No next link found
            }
        }

        if page_count >= max_pages {
            eprintln!("⚠️  Reached max pages limit ({})", max_pages);
        }
    }

    Ok(urls)
}

fn is_empty_result(data: &ReturnedData) -> bool {
    if data.is_empty() {
        return true;
    }

    // Check if all values are empty
    for value in data.values() {
        match value {
            ReturnedDataItem::StringItem(s) if !s.is_empty() => return false,
            ReturnedDataItem::DataItems(items) if !items.is_empty() => return false,
            ReturnedDataItem::NumberItem(_) => return false,
            ReturnedDataItem::BoolItem(_) => return false,
            _ => {}
        }
    }

    true
}

fn main() -> AnyhowResult<()> {
    // Initialize logger with default to 'warn' if RUST_LOG not set
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let args = Cli::parse();

    let config_file_path = args.input.clone().into_os_string().into_string().unwrap();

    eprintln!("📄 Reading config: {}", config_file_path);

    // Read file from args
    let config_file = read_to_string(args.input)
        .with_context(|| format!("could not read file `{}`", config_file_path))?;

    log::debug!("Config file loaded successfully");

    let config_serialized = serde_yaml::from_str::<ScrapeRoot>(config_file.as_str())
        .with_context(|| "invalid config".to_string())?;

    log::debug!("Config parsed successfully");

    config_serialized
        .validate()
        .with_context(|| "invalid config".to_string())?;

    log::debug!("Config validated successfully");

    let mut urls = config_serialized.config.get_urls();

    // Handle pagination
    if let Some(pagination) = &config_serialized.config.pagination {
        if urls.len() == 1 {
            eprintln!("🔄 Pagination enabled");
            let base_url = &urls[0];
            urls = generate_paginated_urls(
                base_url,
                pagination,
                &config_serialized.config,
                &config_serialized.data,
            )?;
            eprintln!("📊 Scraping {} pages", urls.len());
        }
    } else {
        eprintln!("📊 Scraping {} URLs", urls.len());
    }

    // Process all URLs
    let all_results: Vec<ReturnedData> = urls
        .iter()
        .enumerate()
        .map(|(idx, url)| {
            eprintln!("🌐 [{}/{}] {}", idx + 1, urls.len(), url);
            log::info!("Fetching URL: {}", url);

            // Rate limiting: delay between requests
            if idx > 0 && config_serialized.config.delay > 0 {
                log::debug!(
                    "Waiting {}ms before next request",
                    config_serialized.config.delay
                );
                thread::sleep(Duration::from_millis(config_serialized.config.delay));
            }

            let html = fetch_with_config(url, &config_serialized.config)?;
            log::debug!("HTML fetched, extracting data...");

            let result = populate_values(html, config_serialized.data.clone());
            log::info!("Data extracted from page {}", idx + 1);

            Ok(result)
        })
        .collect::<AnyhowResult<Vec<ReturnedData>>>()?;

    eprintln!("✅ Scraping complete!");

    // If single URL, return single result; otherwise return array
    let output_data = if all_results.len() == 1 {
        serde_json::to_value(&all_results[0])?
    } else {
        serde_json::to_value(&all_results)?
    };

    // Write output
    match args.format.as_str() {
        "csv" => {
            log::debug!("Writing output to CSV format");
            write_csv_output(&all_results, args.output)?;
        }
        _ => match &args.output {
            None => {
                log::debug!("Writing JSON output to stdout");
                serde_json::to_writer_pretty(stdout(), &output_data)
                    .with_context(|| "can't write output".to_string())?;
            }
            Some(output) => {
                eprintln!("💾 Saving to: {}", output.display());
                let dest = File::create(output)?;
                serde_json::to_writer_pretty(dest, &output_data)
                    .with_context(|| "can't write output".to_string())?;
                eprintln!("✅ Saved successfully!");
            }
        },
    }

    Ok(())
}

fn get_cache_path(url: &str, cache_dir: &str) -> PathBuf {
    let mut hasher = Sha256::new();
    hasher.update(url.as_bytes());
    let hash = hex::encode(hasher.finalize());
    Path::new(cache_dir).join(format!("{}.html", hash))
}

fn fetch_with_config(url: &str, config: &types::ScrapeRootConfig) -> AnyhowResult<String> {
    // Check cache first
    if config.use_cache {
        if let Some(cache_dir) = &config.cache_dir {
            let cache_path = get_cache_path(url, cache_dir);
            if cache_path.exists() {
                log::debug!("Using cached response for {}", url);
                return read_to_string(cache_path)
                    .with_context(|| "failed to read cache".to_string());
            } else {
                log::debug!("Cache miss for {}", url);
            }
        }
    }

    // Build HTTP client
    let mut client_builder =
        reqwest::blocking::Client::builder().timeout(Duration::from_secs(config.timeout));

    if let Some(proxy_url) = &config.proxy {
        let proxy = reqwest::Proxy::all(proxy_url)?;
        client_builder = client_builder.proxy(proxy);
    }

    let client = client_builder.build()?;

    // Retry logic
    let mut attempts = 0;
    let max_attempts = config.retries + 1;

    loop {
        attempts += 1;

        let mut request = client.get(url);

        // Add custom headers
        if let Some(headers) = &config.headers {
            for (key, value) in headers {
                request = request.header(key, value);
            }
        }

        match request.send() {
            Ok(response) => {
                log::debug!("HTTP request successful");
                let html = response
                    .text()
                    .with_context(|| "can't get url content".to_string())?;

                // Cache response
                if let Some(cache_dir) = &config.cache_dir {
                    create_dir_all(cache_dir)?;
                    let cache_path = get_cache_path(url, cache_dir);
                    let mut file = File::create(cache_path)?;
                    file.write_all(html.as_bytes())?;
                    log::debug!("Response cached");
                }

                return Ok(html);
            }
            Err(e) => {
                if attempts >= max_attempts {
                    return Err(e.into());
                }
                eprintln!(
                    "⚠️  Request failed (attempt {}/{}): {}",
                    attempts, max_attempts, e
                );
                log::debug!("Retrying in 1 second...");
                thread::sleep(Duration::from_secs(1));
            }
        }
    }
}

fn write_csv_output(results: &[ReturnedData], output: Option<PathBuf>) -> AnyhowResult<()> {
    let writer: Box<dyn Write> = match output {
        Some(path) => Box::new(File::create(path)?),
        None => Box::new(stdout()),
    };

    let mut csv_writer = csv::Writer::from_writer(writer);

    // Collect all keys
    let mut all_keys = std::collections::HashSet::new();
    for result in results {
        for key in result.keys() {
            all_keys.insert(key.clone());
        }
    }
    let mut keys: Vec<String> = all_keys.into_iter().collect();
    keys.sort();

    // Write header
    csv_writer.write_record(&keys)?;

    // Write data
    for result in results {
        let record: Vec<String> = keys
            .iter()
            .map(|key| match result.get(key) {
                Some(ReturnedDataItem::StringItem(s)) => s.clone(),
                Some(ReturnedDataItem::NumberItem(n)) => n.to_string(),
                Some(ReturnedDataItem::BoolItem(b)) => b.to_string(),
                Some(ReturnedDataItem::DataItems(items)) => {
                    serde_json::to_string(items).unwrap_or_default()
                }
                None => String::new(),
            })
            .collect();
        csv_writer.write_record(&record)?;
    }

    csv_writer.flush()?;
    Ok(())
}

fn apply_transformations(mut value: String, config: &ItemConfig) -> String {
    // Trim whitespace
    if config.trim {
        value = value.trim().to_string();
    }

    // Strip HTML tags
    if config.strip_html {
        let fragment = Html::parse_fragment(&value);
        value = fragment.root_element().text().collect::<String>();
    }

    // Apply regex extraction
    if let Some(regex_pattern) = &config.regex {
        if let Ok(re) = Regex::new(regex_pattern) {
            if let Some(captures) = re.captures(&value) {
                value = captures
                    .get(0)
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .to_string();
            }
        }
    }

    // Apply text replacement
    if let Some(replace) = &config.replace {
        if replace.len() == 2 {
            value = value.replace(&replace[0], &replace[1]);
        }
    }

    // Apply case transformations
    if config.uppercase {
        value = value.to_uppercase();
    } else if config.lowercase {
        value = value.to_lowercase();
    }

    value
}

fn populate_values(html: String, config: DataConfig) -> ReturnedData {
    let html = Arc::new(html.clone());

    config
        .into_par_iter()
        .map(|(name, config)| {
            let html_parsed = Html::parse_fragment(html.as_str());
            let selector = config.get_item_selector();
            let selected_element = html_parsed.root_element().select(&selector);

            match config {
                ItemConfig {
                    data: Some(inner), ..
                } => {
                    let inner_htmls = selected_element
                        .map(|elem| elem.html())
                        .collect::<Vec<String>>();
                    let inner_values = inner_htmls
                        .into_par_iter()
                        .map(|elem| populate_values(elem, inner.clone()))
                        .collect::<Vec<ReturnedData>>();

                    ReturnedData::from([(name, ReturnedDataItem::DataItems(inner_values))])
                }
                _ => {
                    let selected_element_nth = selected_element.clone().nth(config.nth);
                    let mut value = match selected_element_nth {
                        None => config.default.clone().unwrap_or_else(|| String::from("")),
                        Some(selected_element) => match &config.attr {
                            None => selected_element.inner_html(),
                            Some(attr) => selected_element
                                .value()
                                .attr(attr)
                                .unwrap_or("")
                                .to_string(),
                        },
                    };

                    // Apply transformations
                    value = apply_transformations(value, &config);

                    // Convert to appropriate type
                    let item = if config.to_number {
                        match value.trim().parse::<f64>() {
                            Ok(n) => ReturnedDataItem::NumberItem(n),
                            Err(_) => ReturnedDataItem::StringItem(value),
                        }
                    } else if config.to_boolean {
                        let bool_val =
                            matches!(value.to_lowercase().trim(), "true" | "1" | "yes" | "on");
                        ReturnedDataItem::BoolItem(bool_val)
                    } else {
                        ReturnedDataItem::StringItem(value)
                    };

                    ReturnedData::from([(name, item)])
                }
            }
        })
        .reduce(ReturnedData::new, |dt1, dt2| {
            dt2.iter().fold(dt1, |mut acc, (name, value)| {
                acc.entry(name.clone()).or_insert(value.clone());

                acc
            })
        })
}

#[cfg(test)]
mod tests {
    use crate::ReturnedDataItem::{BoolItem, DataItems, NumberItem, StringItem};
    use crate::{apply_transformations, populate_values, DataConfig, ItemConfig, ReturnedData};

    #[test]
    fn populate_values_test() {
        // From w3schools
        let html = r#"
            <!DOCTYPE html>
            <html>
               <body>
                  <h1>The article element</h1>
                  <article>
                     <h2>Google Chrome</h2>
                     <p>Google Chrome is a web browser developed by Google, released in 2008. Chrome is the world's most popular web browser today!</p>
                  </article>
                  <article>
                     <h2>Mozilla Firefox</h2>
                     <p>Mozilla Firefox is an open-source web browser developed by Mozilla. Firefox has been the second most popular web browser since January, 2018.</p>
                  </article>
                  <article>
                     <h2>Microsoft Edge</h2>
                     <p>Microsoft Edge is a web browser developed by Microsoft, released in 2015. Microsoft Edge replaced Internet Explorer.</p>
                  </article>
               </body>
            </html>
        "#.to_string();

        let yaml_config = r#"
        title:
            selector: h1
        articles:
            selector: article
            data:
                title:
                    selector: h2
                description:
                    selector: p
        "#;
        let data_config = serde_yaml::from_str::<DataConfig>(yaml_config).unwrap();

        let data = populate_values(html, data_config);

        assert_eq!(
            data.get("title").unwrap().clone(),
            StringItem(String::from("The article element"))
        );
        let articles = data.get("articles").unwrap().clone();
        assert!(matches!(articles, DataItems { .. }));
    }

    #[test]
    fn test_number_conversion() {
        let html = r#"<div class="price">$99.99</div>"#.to_string();

        let yaml_config = r#"
        price:
            selector: .price
            regex: '\d+\.\d+'
            toNumber: true
        "#;
        let data_config = serde_yaml::from_str::<DataConfig>(yaml_config).unwrap();

        let data = populate_values(html, data_config);

        match data.get("price").unwrap() {
            NumberItem(n) => assert_eq!(*n, 99.99),
            _ => panic!("Expected NumberItem"),
        }
    }

    #[test]
    fn test_boolean_conversion() {
        let html = r#"<div class="status">true</div>"#.to_string();

        let yaml_config = r#"
        status:
            selector: .status
            toBoolean: true
        "#;
        let data_config = serde_yaml::from_str::<DataConfig>(yaml_config).unwrap();

        let data = populate_values(html, data_config);

        match data.get("status").unwrap() {
            BoolItem(b) => assert!(*b),
            _ => panic!("Expected BoolItem"),
        }
    }

    #[test]
    fn test_default_value() {
        let html = r#"<div class="content">Hello</div>"#.to_string();

        let yaml_config = r#"
        missing:
            selector: .nonexistent
            default: "Default Value"
        "#;
        let data_config = serde_yaml::from_str::<DataConfig>(yaml_config).unwrap();

        let data = populate_values(html, data_config);

        assert_eq!(
            data.get("missing").unwrap().clone(),
            StringItem(String::from("Default Value"))
        );
    }

    #[test]
    fn test_uppercase_transformation() {
        let config = ItemConfig {
            selector: "test".to_string(),
            attr: None,
            data: None,
            trim: true,
            nth: 0,
            default: None,
            regex: None,
            replace: None,
            uppercase: true,
            lowercase: false,
            to_number: false,
            to_boolean: false,
            strip_html: false,
        };

        let result = apply_transformations("hello world".to_string(), &config);
        assert_eq!(result, "HELLO WORLD");
    }

    #[test]
    fn test_lowercase_transformation() {
        let config = ItemConfig {
            selector: "test".to_string(),
            attr: None,
            data: None,
            trim: true,
            nth: 0,
            default: None,
            regex: None,
            replace: None,
            uppercase: false,
            lowercase: true,
            to_number: false,
            to_boolean: false,
            strip_html: false,
        };

        let result = apply_transformations("HELLO WORLD".to_string(), &config);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_regex_extraction() {
        let config = ItemConfig {
            selector: "test".to_string(),
            attr: None,
            data: None,
            trim: true,
            nth: 0,
            default: None,
            regex: Some(r"\d+\.\d+".to_string()),
            replace: None,
            uppercase: false,
            lowercase: false,
            to_number: false,
            to_boolean: false,
            strip_html: false,
        };

        let result = apply_transformations("Price: $99.99 USD".to_string(), &config);
        assert_eq!(result, "99.99");
    }

    #[test]
    fn test_text_replacement() {
        let config = ItemConfig {
            selector: "test".to_string(),
            attr: None,
            data: None,
            trim: true,
            nth: 0,
            default: None,
            regex: None,
            replace: Some(vec!["Breaking: ".to_string(), "".to_string()]),
            uppercase: false,
            lowercase: false,
            to_number: false,
            to_boolean: false,
            strip_html: false,
        };

        let result = apply_transformations("Breaking: News Title".to_string(), &config);
        assert_eq!(result, "News Title");
    }

    #[test]
    fn test_html_stripping() {
        let config = ItemConfig {
            selector: "test".to_string(),
            attr: None,
            data: None,
            trim: true,
            nth: 0,
            default: None,
            regex: None,
            replace: None,
            uppercase: false,
            lowercase: false,
            to_number: false,
            to_boolean: false,
            strip_html: true,
        };

        let result =
            apply_transformations("<p>Hello <strong>World</strong></p>".to_string(), &config);
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn test_combined_transformations() {
        let html = r#"<div class="product"><strong>Price: $99.99</strong></div>"#.to_string();

        let yaml_config = r#"
        price:
            selector: .product
            stripHtml: true
            regex: '\d+\.\d+'
            toNumber: true
        "#;
        let data_config = serde_yaml::from_str::<DataConfig>(yaml_config).unwrap();

        let data = populate_values(html, data_config);

        match data.get("price").unwrap() {
            NumberItem(n) => assert_eq!(*n, 99.99),
            _ => panic!("Expected NumberItem"),
        }
    }

    #[test]
    fn test_trim_whitespace() {
        let html = r#"<div class="content">   Hello World   </div>"#.to_string();

        let yaml_config = r#"
        content:
            selector: .content
            trim: true
        "#;
        let data_config = serde_yaml::from_str::<DataConfig>(yaml_config).unwrap();

        let data = populate_values(html, data_config);

        assert_eq!(
            data.get("content").unwrap().clone(),
            StringItem(String::from("Hello World"))
        );
    }

    #[test]
    fn test_is_empty_result_empty() {
        use crate::is_empty_result;
        let data = ReturnedData::new();
        assert!(is_empty_result(&data));
    }

    #[test]
    fn test_is_empty_result_with_empty_string() {
        use crate::is_empty_result;
        let mut data = ReturnedData::new();
        data.insert("test".to_string(), StringItem(String::new()));
        assert!(is_empty_result(&data));
    }

    #[test]
    fn test_is_empty_result_with_content() {
        use crate::is_empty_result;
        let mut data = ReturnedData::new();
        data.insert("test".to_string(), StringItem("content".to_string()));
        assert!(!is_empty_result(&data));
    }

    #[test]
    fn test_is_empty_result_with_number() {
        use crate::is_empty_result;
        let mut data = ReturnedData::new();
        data.insert("test".to_string(), NumberItem(42.0));
        assert!(!is_empty_result(&data));
    }

    #[test]
    fn test_is_empty_result_with_bool() {
        use crate::is_empty_result;
        let mut data = ReturnedData::new();
        data.insert("test".to_string(), BoolItem(false));
        assert!(!is_empty_result(&data));
    }

    #[test]
    fn test_is_empty_result_with_empty_array() {
        use crate::is_empty_result;
        let mut data = ReturnedData::new();
        data.insert("test".to_string(), DataItems(vec![]));
        assert!(is_empty_result(&data));
    }

    #[test]
    fn test_pagination_config_parsing() {
        let yaml_config = r#"
        url: https://example.com
        pagination:
            pagePattern: "?page={page}"
            startPage: 1
            endPage: 5
        "#;

        let config: crate::types::ScrapeRootConfig = serde_yaml::from_str(yaml_config).unwrap();

        assert!(config.pagination.is_some());
        let pagination = config.pagination.unwrap();
        assert_eq!(pagination.page_pattern, Some("?page={page}".to_string()));
        assert_eq!(pagination.start_page, 1);
        assert_eq!(pagination.end_page, Some(5));
    }

    #[test]
    fn test_pagination_next_selector_parsing() {
        let yaml_config = r#"
        url: https://example.com
        pagination:
            nextSelector: "a.next"
            maxPages: 10
        "#;

        let config: crate::types::ScrapeRootConfig = serde_yaml::from_str(yaml_config).unwrap();

        assert!(config.pagination.is_some());
        let pagination = config.pagination.unwrap();
        assert_eq!(pagination.next_selector, Some("a.next".to_string()));
        assert_eq!(pagination.max_pages, 10);
    }
}
