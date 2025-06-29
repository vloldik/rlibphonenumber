use std::sync::Arc;

use dashmap::DashMap;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("An error occurred while trying to create regex: {0}")]
pub struct ErrorInvalidRegex(#[from] regex::Error);

pub struct RegexCache {
    cache: DashMap<String, Arc<regex::Regex>>
}

impl RegexCache {
    pub fn new() -> Self {
        Self {
            cache: DashMap::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            cache: DashMap::with_capacity(capacity),
        }
    }

    pub fn get_regex(&self, pattern: &str) -> Result<Arc<regex::Regex>, ErrorInvalidRegex> {
        if let Some(regex) = self.cache.get(pattern) {
            Ok(regex.value().clone())
        } else {
            let entry = self.cache.entry(pattern.to_string()).or_try_insert_with(|| {
                regex::Regex::new(pattern).map(Arc::new)
            })?;
            Ok(entry.value().clone())
        }
    }
}