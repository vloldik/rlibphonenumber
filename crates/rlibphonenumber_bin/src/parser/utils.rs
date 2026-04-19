use crate::parser::error::MetadataError;

use super::error::Result;
use regex::Regex;
use rlibphonenumber::PhoneMetadataCollection;
use std::collections::BTreeMap;

pub fn validate_re(regex: &str, remove_whitespace: bool) -> Result<String> {
    let compressed: String = if remove_whitespace {
        regex.chars().filter(|c| !c.is_whitespace()).collect()
    } else {
        regex.to_string()
    };

    if compressed.contains("|)") {
        return Err(MetadataError::Validation(format!(
            "| followed by ) in regex: {}",
            compressed
        )));
    }

    Regex::new(&compressed)?;
    Ok(compressed)
}

pub fn build_country_code_to_region_code_map(
    collection: &PhoneMetadataCollection,
) -> BTreeMap<i32, Vec<String>> {
    let mut map: BTreeMap<i32, Vec<String>> = BTreeMap::new();
    for meta in &collection.metadata {
        let region_code = meta.id.clone();
        let Some(country_code) = meta.country_code else {
            continue;
        };
        let list = map.entry(country_code).or_default();

        if !region_code.is_empty() {
            if meta.main_country_for_code.unwrap_or_default() {
                list.insert(0, region_code);
            } else {
                list.push(region_code);
            }
        }
    }
    map
}

pub fn build_region_code_list(collection: &PhoneMetadataCollection) -> Vec<String> {
    collection.metadata.iter().map(|m| m.id.clone()).collect()
}
