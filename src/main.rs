use anyhow::{Context, Result as AnyhowResult};
use clap::Parser;
use scraper::{ElementRef, Html, Selector};
use std::fs::{read_to_string, File};
use std::io::stdout;
use std::path::PathBuf;
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

    let url = config_serialized.config.url;

    let site_body = reqwest::blocking::get(url)?
        .text()
        .with_context(|| format!("can't get url content"))?;

    let document = Html::parse_document(&site_body);
    let root_element = document.root_element();

    let data = populate_values(root_element, config_serialized.data.clone());

    match args.output {
        None => {
            serde_json::to_writer_pretty(stdout(), &data)
                .with_context(|| format!("can't write output"))?;
        }
        Some(output) => {
            let dest = File::create(output).unwrap();
            serde_json::to_writer_pretty(dest, &data)
                .with_context(|| format!("can't write output"))?;
        }
    }

    Ok(())
}

fn populate_values(root_element: ElementRef, config: DataConfig) -> ReturnedData {
    let mut element_data = ReturnedData::new();

    for (name, item_config) in config.into_iter() {
        match item_config {
            ItemConfig {
                list_item: Some(selector_str),
                data: Some(data),
                ..
            } => {
                let selector = Selector::parse(&selector_str).unwrap();
                let element = root_element.select(&selector);
                let mut list_data: Vec<ReturnedData> = vec![];
                element.for_each(|element| {
                    let element_data = populate_values(element, data.clone());

                    list_data.push(element_data);
                });

                element_data.insert(name, ReturnedDataItem::DataItems(list_data));
            }
            item_config => {
                let selector_str = item_config.selector.unwrap();

                let selector = Selector::parse(&selector_str).unwrap();
                let element = root_element.select(&selector).next();
                let mut value = match element {
                    None => String::from(""),
                    Some(value) => match item_config.attr {
                        None => value.inner_html(),
                        Some(attr) => value.value().attr(&attr).unwrap_or("").to_string(),
                    },
                };
                if item_config.trim {
                    value = value.trim().to_string();
                }

                element_data.insert(name, ReturnedDataItem::StringItem(value));
            }
        }
    }

    element_data
}

#[cfg(test)]
mod tests {
    use crate::ReturnedDataItem::{DataItems, StringItem};
    use crate::{populate_values, DataConfig};
    use scraper::Html;

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
        "#;
        let fragment = Html::parse_fragment(html);
        let root_element = fragment.root_element();

        let yaml_config = r#"
        title:
            selector: h1
        articles:
            listItem: article
            data:
                title:
                    selector: h2
                description: 
                    selector: p
        "#;
        let data_config = serde_yaml::from_str::<DataConfig>(yaml_config).unwrap();

        let data = populate_values(root_element, data_config);

        assert_eq!(
            data.get("title").unwrap().clone(),
            StringItem(String::from("The article element"))
        );
        let articles = data.get("articles").unwrap().clone();
        assert!(matches!(articles, DataItems { .. }));
    }
}
