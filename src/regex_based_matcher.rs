use log::{error};

use crate::{interfaces, proto_gen::phonemetadata::PhoneNumberDesc, regexp_cache::{self, RegexCache}};

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
    ) -> Result<bool, regexp_cache::ErrorInvalidRegex> {
        let regexp = self.cache.get_regex(number_pattern)?;

        // find first occurrence
        if let Some(mat) = regexp.find(phone_number) {
            // if first position is not matched none of scenarios are  possible
            if mat.start() != 0 {
                return Ok(false);
            }
            // full match
            if mat.end() == phone_number.len() {
                return Ok(true);
            } else if allow_prefix_match {
                return Ok(true);
            }
        }
        Ok(false)
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