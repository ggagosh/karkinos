use log::error;
use schemars::JsonSchema;
use scraper::Selector;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use validator::Validate;

/// for serde defaults
fn _default_true() -> bool {
    return true;
}

/// Types for Config

pub type DataConfig = HashMap<String, ItemConfig>;

#[derive(Serialize, Deserialize, Debug, Validate, Clone, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScrapeRoot {
    #[validate]
    pub config: ScrapeRootConfig,

    pub data: DataConfig,
}

#[derive(Serialize, Deserialize, Debug, Validate, Clone, JsonSchema)]
pub struct ScrapeRootConfig {
    #[validate(url)]
    pub url: String,
}

#[derive(Serialize, Deserialize, Debug, Validate, Clone, JsonSchema)]
pub struct ItemConfig {
    pub selector: String,
    pub attr: Option<String>,

    pub data: Option<DataConfig>,

    #[serde(default = "_default_true")]
    pub trim: bool,

    #[serde(default)]
    pub nth: usize,
}

impl ItemConfig {
    #[allow(dead_code)]
    pub fn get_item_selector(&self) -> Selector {
        match Selector::parse(&self.selector) {
            Ok(selector) => selector,
            Err(error) => {
                error!("selector parse error: {:?}", error);

                panic!("invalid selector: {}", self.selector)
            }
        }
    }
}

/// Types for Output

pub type ReturnedData = HashMap<String, ReturnedDataItem>;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum ReturnedDataItem {
    StringItem(String),
    DataItems(Vec<ReturnedData>),
}
