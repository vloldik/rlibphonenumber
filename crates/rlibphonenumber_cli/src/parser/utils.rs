use crate::parser::error::MetadataError;

use super::error::Result;
use regex::Regex;

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
