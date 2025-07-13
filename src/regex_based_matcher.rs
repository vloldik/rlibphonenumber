use log::{error};
use super::regex_util::{RegexFullMatch, RegexConsume};

use crate::{interfaces, phonemetadata::PhoneNumberDesc, regexp_cache::{ErrorInvalidRegex, RegexCache}};

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
    ) -> Result<bool, ErrorInvalidRegex> {
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