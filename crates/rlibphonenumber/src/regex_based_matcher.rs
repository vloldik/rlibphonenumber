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

use crate::{
    InvalidRegexError, interfaces,
    phonenumberutil::regex_wrapper_types::{PhoneNumberDescWrapper, RegexTriplets},
};

#[derive(Debug, Clone, Copy, Default)]
pub struct RegexBasedMatcher {}

impl RegexBasedMatcher {
    fn match_number(
        &self,
        phone_number: &str,
        number_pattern: &RegexTriplets,
        allow_prefix_match: bool,
    ) -> Result<bool, InvalidRegexError> {
        let Some(rule) = (if allow_prefix_match {
            number_pattern.anchor_start()?
        } else {
            number_pattern.anchor_full()?
        }) else {
            return Ok(false);
        };

        // find first occurrence
        Ok(rule.is_match(phone_number))
    }
}

impl interfaces::MatcherApi for RegexBasedMatcher {
    fn match_national_number(
        &self,
        number: &str,
        number_desc: &PhoneNumberDescWrapper,
        allow_prefix_match: bool,
    ) -> Result<bool, InvalidRegexError> {
        let national_number_pattern = number_desc.national_number_pattern();
        // We don't want to consider it a prefix match when matching non-empty input
        // against an empty pattern.

        self.match_number(number, national_number_pattern, allow_prefix_match)
    }
}
