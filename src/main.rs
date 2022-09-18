use anyhow::{Context, Result as AnyhowResult};
use clap::Parser;
use scraper::{ElementRef, Html};
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
        let selector = item_config.get_item_selector();
        let element = root_element.select(&selector);

        let values = match item_config {
            ItemConfig {
                data: Some(inner_config),
                ..
            } => {
                let mut inner_values = vec![];
                element.for_each(|elem| {
                    let inner_value = populate_values(elem, inner_config.clone());

                    inner_values.push(inner_value);
                });

                ReturnedDataItem::DataItems(inner_values)
            }
            _ => {
                let nth_element = element.clone().nth(item_config.nth);
                let value = match nth_element {
                    None => String::from(""),
                    Some(nth_element) => match item_config.attr {
                        None => nth_element.inner_html(),
                        Some(attr) => nth_element.value().attr(&attr).unwrap_or("").to_string(),
                    },
                };

                ReturnedDataItem::StringItem(value)
            }
        };
        element_data.insert(name, values);
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
            selector: article
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
