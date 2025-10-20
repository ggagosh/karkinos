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

fn main() -> AnyhowResult<()> {
    // initialize
    env_logger::init();

    let args = Cli::parse();

    let config_file_path = args.input.clone().into_os_string().into_string().unwrap();

    // Read file from args
    let config_file = read_to_string(args.input)
        .with_context(|| format!("could not read file `{}`", config_file_path))?;

    let config_serialized = serde_yaml::from_str::<ScrapeRoot>(config_file.as_str())
        .with_context(|| format!("invalid config"))?;

    config_serialized
        .validate()
        .with_context(|| format!("invalid config"))?;

    let urls = config_serialized.config.get_urls();

    // Process all URLs
    let all_results: Vec<ReturnedData> = urls
        .iter()
        .enumerate()
        .map(|(idx, url)| {
            // Rate limiting: delay between requests
            if idx > 0 && config_serialized.config.delay > 0 {
                thread::sleep(Duration::from_millis(config_serialized.config.delay));
            }

            let html = fetch_with_config(url, &config_serialized.config)?;
            Ok(populate_values(html, config_serialized.data.clone()))
        })
        .collect::<AnyhowResult<Vec<ReturnedData>>>()?;

    // If single URL, return single result; otherwise return array
    let output_data = if all_results.len() == 1 {
        serde_json::to_value(&all_results[0])?
    } else {
        serde_json::to_value(&all_results)?
    };

    // Write output
    match args.format.as_str() {
        "csv" => {
            write_csv_output(&all_results, args.output)?;
        }
        _ => {
            match args.output {
                None => {
                    serde_json::to_writer_pretty(stdout(), &output_data)
                        .with_context(|| format!("can't write output"))?;
                }
                Some(output) => {
                    let dest = File::create(output)?;
                    serde_json::to_writer_pretty(dest, &output_data)
                        .with_context(|| format!("can't write output"))?;
                }
            }
        }
    }

    Ok(())
}

fn get_cache_path(url: &str, cache_dir: &str) -> PathBuf {
    let mut hasher = Sha256::new();
    hasher.update(url.as_bytes());
    let hash = hex::encode(hasher.finalize());
    Path::new(cache_dir).join(format!("{}.html", hash))
}

fn fetch_with_config(
    url: &str,
    config: &types::ScrapeRootConfig,
) -> AnyhowResult<String> {
    // Check cache first
    if config.use_cache {
        if let Some(cache_dir) = &config.cache_dir {
            let cache_path = get_cache_path(url, cache_dir);
            if cache_path.exists() {
                log::info!("Using cached response for {}", url);
                return read_to_string(cache_path)
                    .with_context(|| format!("failed to read cache"));
            }
        }
    }

    // Build HTTP client
    let mut client_builder = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(config.timeout));

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
                let html = response
                    .text()
                    .with_context(|| format!("can't get url content"))?;

                // Cache response
                if let Some(cache_dir) = &config.cache_dir {
                    create_dir_all(cache_dir)?;
                    let cache_path = get_cache_path(url, cache_dir);
                    let mut file = File::create(cache_path)?;
                    file.write_all(html.as_bytes())?;
                }

                return Ok(html);
            }
            Err(e) => {
                if attempts >= max_attempts {
                    return Err(e.into());
                }
                log::warn!("Request failed (attempt {}/{}): {}", attempts, max_attempts, e);
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
                value = captures.get(0).map(|m| m.as_str()).unwrap_or("").to_string();
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
                        Some(selected_element) => match config.attr {
                            None => selected_element.inner_html(),
                            Some(attr) => selected_element
                                .value()
                                .attr(&attr)
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
                        let bool_val = matches!(
                            value.to_lowercase().trim(),
                            "true" | "1" | "yes" | "on"
                        );
                        ReturnedDataItem::BoolItem(bool_val)
                    } else {
                        ReturnedDataItem::StringItem(value)
                    };

                    ReturnedData::from([(name, item)])
                }
            }
        })
        .reduce(
            || ReturnedData::new(),
            |dt1, dt2| {
                dt2.iter().fold(dt1, |mut acc, (name, value)| {
                    acc.entry(name.clone()).or_insert(value.clone());

                    acc
                })
            },
        )
}

#[cfg(test)]
mod tests {
    use crate::ReturnedDataItem::{DataItems, StringItem};
    use crate::{populate_values, DataConfig};

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
}
