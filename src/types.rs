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

fn _default_timeout() -> u64 {
    30
}

fn _default_retries() -> u32 {
    0
}

fn _default_delay() -> u64 {
    0
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
#[serde(rename_all = "camelCase")]
#[validate(schema(function = "validate_urls"))]
pub struct ScrapeRootConfig {
    /// Primary URL to scrape (can be overridden by urls)
    #[validate(url)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    /// Multiple URLs to scrape in batch
    #[serde(skip_serializing_if = "Option::is_none")]
    pub urls: Option<Vec<String>>,

    /// HTTP headers to send with request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,

    /// Request timeout in seconds
    #[serde(default = "_default_timeout")]
    pub timeout: u64,

    /// Number of retry attempts on failure
    #[serde(default = "_default_retries")]
    pub retries: u32,

    /// Delay between requests in milliseconds (for rate limiting)
    #[serde(default = "_default_delay")]
    pub delay: u64,

    /// Proxy URL (e.g., http://proxy:8080)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy: Option<String>,

    /// Cache directory for storing responses
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_dir: Option<String>,

    /// Use cached responses if available
    #[serde(default)]
    pub use_cache: bool,

    /// Pagination configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagination: Option<PaginationConfig>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaginationConfig {
    /// CSS selector for the "next page" link
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_selector: Option<String>,

    /// URL pattern with {page} placeholder (e.g., "?page={page}")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_pattern: Option<String>,

    /// Starting page number (default: 1)
    #[serde(default = "_default_start_page")]
    pub start_page: usize,

    /// Maximum number of pages to scrape (0 = unlimited for next_selector)
    #[serde(default)]
    pub max_pages: usize,

    /// Ending page number (only used with page_pattern)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_page: Option<usize>,

    /// Stop if no results found on page
    #[serde(default)]
    pub stop_on_empty: bool,
}

fn _default_start_page() -> usize {
    1
}

fn validate_urls(config: &ScrapeRootConfig) -> Result<(), validator::ValidationError> {
    if config.url.is_none() && config.urls.is_none() {
        return Err(validator::ValidationError::new(
            "Either 'url' or 'urls' must be provided",
        ));
    }
    Ok(())
}

impl ScrapeRootConfig {
    pub fn get_urls(&self) -> Vec<String> {
        if let Some(urls) = &self.urls {
            urls.clone()
        } else if let Some(url) = &self.url {
            vec![url.clone()]
        } else {
            vec![]
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Validate, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ItemConfig {
    pub selector: String,
    pub attr: Option<String>,

    pub data: Option<DataConfig>,

    #[serde(default = "_default_true")]
    pub trim: bool,

    #[serde(default)]
    pub nth: usize,

    /// Default value if extraction fails
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,

    /// Regex pattern to extract from the text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regex: Option<String>,

    /// Text replacement: [pattern, replacement]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replace: Option<Vec<String>>,

    /// Convert text to uppercase
    #[serde(default)]
    pub uppercase: bool,

    /// Convert text to lowercase
    #[serde(default)]
    pub lowercase: bool,

    /// Convert to number
    #[serde(default)]
    pub to_number: bool,

    /// Convert to boolean
    #[serde(default)]
    pub to_boolean: bool,

    /// Remove HTML tags
    #[serde(default)]
    pub strip_html: bool,
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
    NumberItem(f64),
    BoolItem(bool),
    DataItems(Vec<ReturnedData>),
}
