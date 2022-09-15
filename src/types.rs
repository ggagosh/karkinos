use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use validator::Validate;

/// for serde default bool true
fn _default_true() -> bool {
    return true;
}

/// Types for Config

pub type DataConfig = HashMap<String, ItemConfig>;

#[derive(Serialize, Deserialize, Debug, Validate, Clone, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScrapeRoot {
    #[serde(rename = "config")]
    #[validate]
    pub config: ScrapeRootConfig,

    #[serde(rename = "data")]
    pub data: DataConfig,
}

#[derive(Serialize, Deserialize, Debug, Validate, Clone, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScrapeRootConfig {
    #[serde(rename = "url")]
    #[validate(url)]
    pub url: String,
}

#[derive(Serialize, Deserialize, Debug, Validate, Clone, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemConfig {
    #[serde(rename = "selector")]
    pub selector: Option<String>,

    #[serde(rename = "attr")]
    pub attr: Option<String>,

    #[serde(rename = "listItem")]
    pub list_item: Option<String>,

    #[serde(rename = "data")]
    pub data: Option<DataConfig>,

    #[serde(rename = "trim", default = "_default_true")]
    pub trim: bool,
}

/// Types for Output

pub type ReturnedData = HashMap<String, ReturnedDataItem>;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum ReturnedDataItem {
    StringItem(String),
    DataItems(Vec<ReturnedData>),
}
