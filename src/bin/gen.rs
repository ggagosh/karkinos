extern crate schemars;
extern crate serde_json;

use schemars::schema_for;
use std::fs::File;

#[path = "../types.rs"]
mod types;

fn main() {
    let dest = File::create("../../krk-schema.json").expect("can't create json schema file :(");
    let schema = schema_for!(types::ScrapeRoot);

    serde_json::to_writer_pretty(dest, &schema).unwrap();
}
