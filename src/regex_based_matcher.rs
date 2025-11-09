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

use super::regex_util::{RegexConsume, RegexFullMatch};
use log::error;

use crate::{
    generated::proto::phonemetadata::PhoneNumberDesc,
    interfaces,
    regexp_cache::{InvalidRegexError, RegexCache},
};

pub struct RegexBasedMatcher {
    cache: RegexCache,
}

impl RegexBasedMatcher {
    pub fn new() -> Self {
        Self {
            cache: RegexCache::with_capacity(128),
        }
    }

    fn match_number(
        &self,
        phone_number: &str,
        number_pattern: &str,
        allow_prefix_match: bool,
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
        &self,
        number: &str,
        number_desc: &PhoneNumberDesc,
        allow_prefix_match: bool,
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

#[cfg(test)]
mod tests {
    use crate::generated::proto::phonemetadata::PhoneNumberDesc;
    use crate::interfaces::MatcherApi;
    use crate::regex_based_matcher::RegexBasedMatcher;

    fn to_string(desc: &PhoneNumberDesc) -> String {
        let pattern = if !desc.national_number_pattern().is_empty() {
            desc.national_number_pattern()
        } else {
            "none"
        };
        format!("pattern: {}", pattern)
    }

    fn expect_matched(matcher: &impl MatcherApi, number: &str, desc: &PhoneNumberDesc) {
        assert!(
            matcher.match_national_number(number, desc, false),
            "{} should have matched {}",
            number,
            to_string(desc)
        );
        assert!(
            matcher.match_national_number(number, desc, true),
            "{} should have matched {}",
            number,
            to_string(desc)
        );
    }

    fn expect_invalid(matcher: &impl MatcherApi, number: &str, desc: &PhoneNumberDesc) {
        assert!(
            !matcher.match_national_number(number, desc, false),
            "{} should not have matched {}",
            number,
            to_string(desc)
        );
        assert!(
            !matcher.match_national_number(number, desc, true),
            "{} should not have matched {}",
            number,
            to_string(desc)
        );
    }

    fn expect_too_long(matcher: &impl MatcherApi, number: &str, desc: &PhoneNumberDesc) {
        assert!(
            !matcher.match_national_number(number, desc, false),
            "{} should have been too long for {}",
            number,
            to_string(desc)
        );
        assert!(
            matcher.match_national_number(number, desc, true),
            "{} should have been too long for {}",
            number,
            to_string(desc)
        );
    }

    fn create_desc(national_number_pattern: &str) -> PhoneNumberDesc {
        let mut desc = PhoneNumberDesc::default();
        if !national_number_pattern.is_empty() {
            desc.set_national_number_pattern(national_number_pattern.to_string());
        }
        desc
    }

    fn check_matcher_behaves_as_expected(matcher: &impl MatcherApi) {
        let mut desc;

        desc = create_desc("");
        // Test if there is no matcher data.
        expect_invalid(matcher, "1", &desc);

        desc = create_desc(r"9\d{2}");
        expect_invalid(matcher, "91", &desc);
        expect_invalid(matcher, "81", &desc);
        expect_matched(matcher, "911", &desc);
        expect_invalid(matcher, "811", &desc);
        expect_too_long(matcher, "9111", &desc);
        expect_invalid(matcher, "8111", &desc);

        desc = create_desc(r"\d{1,2}");
        expect_matched(matcher, "2", &desc);
        expect_matched(matcher, "20", &desc);

        desc = create_desc("20?");
        expect_matched(matcher, "2", &desc);
        expect_matched(matcher, "20", &desc);

        desc = create_desc("2|20");
        expect_matched(matcher, "2", &desc);
        // Subtle case where lookingAt() and matches() result in different end()s.
        expect_matched(matcher, "20", &desc);
    }

    #[test]
    fn test_regex_based_matcher() {
        let matcher = RegexBasedMatcher::new();
        check_matcher_behaves_as_expected(&matcher);
    }
}
