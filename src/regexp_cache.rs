// Copyright (C) 2009 The Libphonenumber Authors
// Copyright (C) 2025 The Kashin Vladislav (Rust adaptation author)
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::sync::Arc;

use dashmap::DashMap;
use thiserror::Error;

#[derive(Debug, PartialEq, Error)]
#[error("An error occurred while trying to create regex: {0}")]
pub struct ErrorInvalidRegex(#[from] regex::Error);

pub struct RegexCache {
    cache: DashMap<String, Arc<regex::Regex>>
}

impl RegexCache {
    
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