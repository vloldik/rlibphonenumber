// Copyright (C) 2009 The Libphonenumber Authors
// Copyright (C) 2025 Kashin Vladislav (Rust adaptation author)
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


use log::{error};
use super::regex_util::{RegexFullMatch, RegexConsume};

use crate::{interfaces, generated::proto::phonemetadata::PhoneNumberDesc, regexp_cache::{InvalidRegexError, RegexCache}};

pub struct RegexBasedMatcher {
    cache: RegexCache,   
}

impl RegexBasedMatcher {
    pub fn new() -> Self {
        Self { cache: RegexCache::with_capacity(128) }
    }

    fn match_number(
        &self, phone_number: &str, 
        number_pattern: &str,
        allow_prefix_match: bool
    ) -> Result<bool, InvalidRegexError> {
        let regexp = self.cache.get_regex(number_pattern)?;

        // find first occurrence
        if allow_prefix_match {
            Ok(regexp.matches_start(phone_number))
        } else {
            Ok(regexp.full_match(phone_number))
        }
    }
}

impl interfaces::MatcherApi for RegexBasedMatcher {
    fn match_national_number(
        &self, number: &str, 
        number_desc: &PhoneNumberDesc, 
        allow_prefix_match: bool
    ) -> bool {
        let national_number_pattern = number_desc.national_number_pattern();
        // We don't want to consider it a prefix match when matching non-empty input
        // against an empty pattern.
        if national_number_pattern.is_empty() {
            return false;
        }
        if let Ok(res) = self.match_number(number, national_number_pattern, allow_prefix_match) {
            res
        } else {
            error!("Invalid regex! {}", national_number_pattern);
            false
        }
    }
}