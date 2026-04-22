/*
 * Copyright (C) 2011 The Libphonenumber Authors
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 * http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

// Generated Unicode lookup tables (produced by build.rs via UnipropsBuilder).
// Each included file exposes a single `pub fn is_<name>(c: char) -> bool`.

use core::{num, str};
use std::{
    cell::Cell,
    ops::{Deref, Index},
    sync::Arc,
};

use crate::{
    CountryCodeSource, MatchType, PhoneNumber, PhoneNumberFormat, PhoneNumberUtil,
    generated::uniprops_without_nl,
    phonenumber_matcher::{
        alternate_formats::AlternateFormats, leniency::Leniency,
        phonenumber_match::PhoneNumberMatch,
    },
    phonenumberutil::{
        helper_constants::{
            CAPTURE_UP_TO_SECOND_NUMBER_START, DIGITS, MAX_LENGTH_COUNTRY_CODE, MAX_LENGTH_FOR_NSN,
            PLUS_CHARS, SEPARATORS, VALID_PUNCTUATION,
        },
        helper_functions::{
            create_extn_pattern, get_national_significant_number, is_unwanted_end_char,
            normalize_digits,
        },
    },
    regexp::Regex,
    unwrap_internal, unwrap_regex_error,
};

// ── Error type ────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum PhoneNumberMatcherError {
    #[error("max_tries must be >= 0, got {0}")]
    NegativeMaxTries(i64),
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Returns a regular expression quantifier with an upper and lower limit.
fn limit(lower: usize, upper: usize) -> String {
    debug_assert!(
        upper > 0 && upper >= lower,
        "invalid limit bounds: lower={lower}, upper={upper}"
    );
    format!("{{{lower},{upper}}}")
}

// ── State ─────────────────────────────────────────────────────────────────────

/// The potential states of a [`PhoneNumberMatcher`].
#[derive(PartialEq)]
enum State {
    NotReady,
    Ready,
    Done,
}

#[derive(Debug, Clone, Copy)]
enum CheckerVariant {
    AllNumberGroupsRemainGrouped,
    AllNumberGroupsAreExactlyPresent,
}

// ── Main struct ───────────────────────────────────────────────────────────────

/// A stateful struct that finds and extracts telephone numbers from text.
/// Instances are created via the factory methods in [`PhoneNumberUtil`].
///
/// Vanity numbers (phone numbers using alphabetic digits such as
/// `1-800-SIX-FLAGS`) are not found.
///
/// This struct is not thread-safe.
pub struct PhoneNumberMatcher<'a, T: Deref<Target = PhoneNumberUtil>> {
    /// The phone number pattern used by [`find`], similar to
    /// `PhoneNumberUtil::VALID_PHONE_NUMBER`, but with the following
    /// differences:
    /// * All captures are limited to place an upper bound on the matched text:
    ///   * Leading punctuation / plus signs are limited.
    ///   * Consecutive occurrences of punctuation are limited.
    ///   * Number of digits is limited.
    /// * No whitespace is allowed at the start or end.
    /// * No alpha digits (vanity numbers such as `1-800-SIX-FLAGS`) are
    ///   currently supported.
    pattern: Regex,

    /// Matches strings that look like publication pages.  Example:
    ///
    /// * Computing Complete Answers to Queries in the Presence of Limited
    /// * Access Patterns.  Chen Li. VLDB J. 12(3): 211-227 (2003).
    ///
    /// The string `"211-227 (2003)"` is not a telephone number.
    pub_pages: Regex,

    /// Matches strings that look like dates using `"/"` as a separator.
    /// Examples: `3/10/2011`, `31/10/96`, or `08/31/95`.
    slash_separated_dates: Regex,

    /// Matches timestamps.  Example: `"2012-01-02 08:00"`.  Note that the
    /// regex does not include the trailing `":\d\d"` — that is covered by
    /// [`time_stamps_suffix`].
    time_stamps: Regex,
    time_stamps_suffix: Regex,

    /// Pattern to check that brackets match.  Opening brackets should be
    /// closed within a phone number.  This also checks that there is something
    /// inside the brackets.  Having no brackets at all is also fine.
    matching_brackets_full_match: Regex,

    /// Patterns used to extract phone numbers from a larger
    /// phone-number-like pattern.  These are ordered according to specificity.
    /// For example, white-space is last since that is frequently used in
    /// numbers, not just to separate two numbers.  We have separate patterns
    /// since we don't want to break up the phone-number-like text on more than
    /// one different kind of symbol at one time, although symbols of the same
    /// type (e.g. space) can be safely grouped together.
    ///
    /// Note that if there is a match, we will always check any text found up
    /// to the first match as well.
    inner_matches: Vec<Regex>,

    /// Punctuation that may be at the start of a phone number — brackets and
    /// plus signs.
    lead_class: Regex,

    // ── instance state ────────────────────────────────────────────────────────
    /// The phone number utility.
    phone_util: T,
    /// The text searched for phone numbers.
    text: String,
    /// The region (country) to assume for phone numbers without an
    /// international prefix, or `None` if only numbers with a leading plus
    /// should be considered.
    preferred_region: Option<String>,
    /// The degree of validation requested.
    leniency: Leniency,
    /// The maximum number of retries after matching an invalid number.
    max_tries: Cell<i64>,

    /// The iteration tristate.
    state: State,
    /// The last successful match; `None` unless in [`State::Ready`].
    last_match: Option<PhoneNumberMatch<'a>>,
    /// The next index to start searching at.  Undefined in [`State::Done`].
    search_index: usize,

    alternate_formats: Option<Arc<AlternateFormats>>,
}

impl<'a, T: Deref<Target = PhoneNumberUtil>> PhoneNumberMatcher<'a, T> {
    /// Creates a new instance.  See the factory methods in [`PhoneNumberUtil`]
    /// on how to obtain a new instance.
    ///
    /// * `util`      – the phone number utility to use
    /// * `text`      – the character sequence to search; `None` means no text
    /// * `country`   – the country to assume for phone numbers not written in
    ///   international format (with a leading plus, or with the international
    ///   dialling prefix of the specified region).  Pass `None` if only
    ///   numbers with a leading plus should be considered.
    /// * `leniency`  – the leniency to use when evaluating candidates
    /// * `max_tries` – the maximum number of invalid numbers to try before
    ///   giving up on the text.  This covers degenerate cases where the text
    ///   has many false positives.  Must be `>= 0`.
    pub fn new(
        util: T,
        text: Option<String>,
        country: Option<String>,
        leniency: Leniency,
        max_tries: i64,
        alternate_formats: Option<Arc<AlternateFormats>>,
    ) -> Result<Self, PhoneNumberMatcherError> {
        if max_tries < 0 {
            return Err(PhoneNumberMatcherError::NegativeMaxTries(max_tries));
        }

        /* Build the `matching_brackets` and `pattern` regular expressions.
         * The building blocks below exist to make the pattern more easily
         * understood. */

        let opening_parens = "(\\[\u{FF08}\u{FF3B}";
        let closing_parens = ")\\]\u{FF09}\u{FF3D}";
        let non_parens = format!("[^{opening_parens}{closing_parens}]");

        /* Limit on the number of pairs of brackets in a phone number. */
        let bracket_pair_limit = limit(0, 3);

        /* An opening bracket at the beginning may not be closed, but
         * subsequent ones should be.  It's also possible that the leading
         * bracket was dropped, so we shouldn't be surprised if we see a
         * closing bracket first.  We limit the sets of brackets in a phone
         * number to four. */
        let matching_brackets_full_match = Regex::new(&format!(
            "^(?:(?:[{opening_parens}])?(?:{non_parens}+[{closing_parens}])?{non_parens}+\
             (?:[{opening_parens}]{non_parens}+[{closing_parens}]){bracket_pair_limit}{non_parens}*)$"
        ))
        .unwrap();

        /* Limit on the number of leading (plus) characters. */
        let lead_limit = limit(0, 2);
        /* Limit on the number of consecutive punctuation characters. */
        let punctuation_limit = limit(0, 4);
        /* The maximum number of digits allowed in a digit-separated block.
         * As we allow all digits in a single block, set high enough to
         * accommodate the entire national number and the international country
         * code. */
        let digit_block_limit = MAX_LENGTH_FOR_NSN + MAX_LENGTH_COUNTRY_CODE;
        /* Limit on the number of blocks separated by punctuation.  Uses
         * `digit_block_limit` since some formats use spaces to separate each
         * digit. */
        let block_limit = limit(0, digit_block_limit);

        /* A punctuation sequence allowing white space. */
        let punctuation = format!("[{}]{punctuation_limit}", VALID_PUNCTUATION);
        let digit_sequence = format!("[{}]{}", DIGITS, limit(1, digit_block_limit));

        let lead_class_chars = format!("{opening_parens}{}", PLUS_CHARS);
        let lead_class_str = format!("[{lead_class_chars}]");
        let lead_class = Regex::new(&lead_class_str).unwrap();

        /* Phone number pattern allowing optional punctuation. */
        let pattern = Regex::new(&format!(
            "(?:{lead_class_str}{punctuation}){lead_limit}\
             {digit_sequence}(?:{punctuation}{digit_sequence}){block_limit}\
             (?:{})?",
            create_extn_pattern(false),
        ))
        .unwrap();

        let inner_matches = vec![
            // Breaks on the slash — e.g. "651-234-2345/332-445-1234"
            Regex::new("/+(.*)").unwrap(),
            // Note that the bracket here is inside the capturing group, since
            // we consider it part of the phone number.  Will match a pattern
            // like "(650) 223 3345 (754) 223 3321".
            Regex::new("(\\([^(]*)").unwrap(),
            // Breaks on a hyphen — e.g. "12345 - 332-445-1234 is my number."
            // We require a space on either side of the hyphen for it to be
            // considered a separator.
            Regex::new(
                format!(
                    "(?:[{}]-|-[{}])[{}]*(.+)",
                    SEPARATORS, SEPARATORS, SEPARATORS
                )
                .as_str(),
            )
            .unwrap(),
            // Various types of wide hyphens.  Note we have decided not to
            // enforce a space here, since it's possible that it's supposed to
            // be used to break two numbers without spaces, and we haven't seen
            // many instances of it used within a number.
            Regex::new(format!("[\u{2012}-\u{2015}\u{FF0D}][{}]*(.+)", SEPARATORS).as_str())
                .unwrap(),
            // Breaks on a full stop — e.g. "12345. 332-445-1234 is my number."
            Regex::new(format!("\\.+[{}]*([^.]+)", SEPARATORS).as_str()).unwrap(),
            // Breaks on space — e.g. "3324451234 8002341234"
            Regex::new(format!("[{}]+([{}]+)", SEPARATORS, SEPARATORS).as_str()).unwrap(),
        ];

        Ok(PhoneNumberMatcher {
            pattern,
            pub_pages: Regex::new("\\d{1,5}-+\\d{1,5}\\s{0,4}\\(\\d{1,4}").unwrap(),
            slash_separated_dates: Regex::new(
                "(?:(?:[0-3]?\\d/[01]?\\d)|(?:[01]?\\d/[0-3]?\\d))/(?:[12]\\d)?\\d{2}",
            )
            .unwrap(),
            time_stamps: Regex::new("[12]\\d{3}[-/]?[01]\\d[-/]?[0-3]\\d +[0-2]\\d$").unwrap(),
            time_stamps_suffix: Regex::new(":[0-5]\\d").unwrap(),
            matching_brackets_full_match,
            inner_matches,
            lead_class,
            phone_util: util,
            text: text.unwrap_or_default(),
            preferred_region: country,
            leniency,
            max_tries: Cell::new(max_tries),
            state: State::NotReady,
            last_match: None,
            search_index: 0,
            alternate_formats,
        })
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    /// Attempts to find the next subsequence in the searched text on or after
    /// `index` that represents a phone number.  Returns the next match, or
    /// `None` if none was found.
    fn find(&self, index: usize) -> Option<PhoneNumberMatch<'a>> {
        let mut pos = index;

        while self.max_tries.get() > 0 {
            let text = self.text.as_str();
            let m = self.pattern.find_at(text, pos)?;
            let start = m.start();
            let mut candidate = &text[start..m.end()];

            // Check for extra numbers at the end.
            // TODO: This is the place to start when trying to support
            // extraction of multiple phone numbers from split notations
            // (+41 79 123 45 67 / 68).
            candidate = Self::trim_after_first_match(
                |s| {
                    self.phone_util
                        .util_internal()
                        .reg_exps
                        .capture_up_to_second_number_start_pattern
                        .find(s)
                        .map(|s| s.start())
                },
                &candidate,
            );

            let extract_match = self.extract_match(&candidate, start);
            if let Some(result) = extract_match {
                return Some(result);
            } else {
                pos = start + candidate.len();
                self.decrement_tries();
            }
        }

        None
    }

    /// Trims away any characters after the first match of `pattern` in
    /// `candidate`, returning the trimmed version.
    fn trim_after_first_match<'b, P: Fn(&str) -> Option<usize>>(
        pattern: P,
        candidate: &'b str,
    ) -> &'b str {
        match pattern(candidate) {
            Some(m) => &candidate[..m],
            None => candidate,
        }
    }

    /// Helper method to determine if a character is a Latin-script letter or
    /// not.  For our purposes, combining marks should also return `true` since
    /// we assume they have been added to a preceding Latin character.
    // #[cfg(test)]
    pub fn is_latin_letter(letter: char) -> bool {
        // Combining marks are a subset of non-spacing-mark.
        // `is_non_spacing_mark` is generated by UnipropsBuilder for Unicode
        // general category Mn.
        if !letter.is_alphabetic() && !is_non_spacing_mark(letter) {
            return false;
        }
        // `is_latin_script` is generated by UnipropsBuilder filtering on:
        //   Basic Latin, Latin-1 Supplement, Latin Extended-A, Latin Extended-B,
        //   Latin Extended Additional, Combining Diacritical Marks.
        is_latin_script(letter)
    }

    fn is_invalid_punctuation_symbol(character: char) -> bool {
        // `is_currency_symbol` is generated by UnipropsBuilder for Unicode
        // general category Sc.
        character == '%' || is_currency_symbol(character)
    }

    /// Attempts to extract a match from a `candidate` character sequence.
    ///
    /// * `candidate` – the candidate text that might contain a phone number
    /// * `offset`    – the offset of `candidate` within [`Self::text`]
    fn extract_match(&self, candidate: &str, offset: usize) -> Option<PhoneNumberMatch<'a>> {
        // Skip a match that is more likely to be a date.
        if self.slash_separated_dates.find(candidate).is_some() {
            return None;
        }

        // Skip potential time-stamps.
        if self.time_stamps.find(candidate).is_some() {
            let following_text = &self.text[offset + candidate.len()..];
            if self.time_stamps_suffix.is_match_at(following_text, 0) {
                return None;
            }
        }

        // Try to come up with a valid match given the entire candidate.
        if let Some(result) = self.parse_and_verify(candidate, offset) {
            return Some(result);
        }

        // If that failed, try to find an "inner match" — there might be a
        // phone number within this candidate.
        self.extract_inner_match(candidate, offset)
    }

    /// Attempts to extract a match from `candidate` if the whole candidate
    /// does not qualify as a match.
    ///
    /// * `candidate` – the candidate text that might contain a phone number
    /// * `offset`    – the current offset of `candidate` within [`Self::text`]
    fn extract_inner_match(&self, candidate: &str, offset: usize) -> Option<PhoneNumberMatch<'a>> {
        // Clone to satisfy the borrow checker — `inner_matches` is borrowed
        // immutably by the loop while `self` must also be borrowed mutably for
        // the parse calls.
        let patterns: Vec<Regex> = self.inner_matches.iter().cloned().collect();

        for possible_inner_match in &patterns {
            let mut is_first_match = true;
            let mut search_pos = 0;

            while self.max_tries.get() > 0 {
                let group_m = match possible_inner_match.find_at(candidate, search_pos) {
                    Some(m) => m,
                    None => break,
                };

                if is_first_match {
                    // We should handle any group before this one too.
                    let before = Self::trim_after_first_match(
                        |s| s.find(is_unwanted_end_char),
                        &candidate[..group_m.start()],
                    );
                    if let Some(result) = self.parse_and_verify(&before, offset) {
                        return Some(result);
                    }
                    self.decrement_tries();
                    is_first_match = false;
                }

                let group1 = possible_inner_match
                    .captures_at(candidate, group_m.start())
                    .and_then(|c| c.get(1))
                    .map(|m| m.as_str())
                    .unwrap_or("");

                let group = Self::trim_after_first_match(|s| s.find(is_unwanted_end_char), group1);
                let group_offset =
                    offset + (group1.as_ptr() as usize - candidate.as_ptr() as usize);

                if let Some(result) = self.parse_and_verify(&group, group_offset) {
                    return Some(result);
                }
                self.decrement_tries();

                search_pos = group_m.end();
            }
        }
        None
    }

    /// Parses a phone number from `candidate` using
    /// [`PhoneNumberUtil::parse_and_keep_raw_input`] and verifies it matches
    /// the requested [`leniency`].  Returns a [`PhoneNumberMatch`] on success,
    /// or `None` otherwise.
    fn parse_and_verify(&self, candidate: &str, offset: usize) -> Option<PhoneNumberMatch<'a>> {
        // Check the candidate doesn't contain any formatting which would
        // indicate that it really isn't a phone number.
        if !self.matching_brackets_full_match.is_match(candidate)
            || self.pub_pages.find(candidate).is_some()
        {
            return None;
        }

        // If leniency is set to VALID or stricter, we also want to skip
        // numbers that are surrounded by Latin alphabetic characters, to skip
        // cases like abc8005001234 or 8005001234def.
        if self.leniency >= Leniency::Valid {
            // If the candidate is not at the start of the text, and does not
            // start with phone-number punctuation, check the previous
            // character.
            if offset > 0 && !self.lead_class.is_match_at(candidate, 0) {
                let previous_char = self.text[..offset].chars().last()?;
                // We return None if it is a latin letter or an invalid
                // punctuation symbol.
                if Self::is_invalid_punctuation_symbol(previous_char)
                    || Self::is_latin_letter(previous_char)
                {
                    return None;
                }
            }
            let last_char_index = offset + candidate.len();
            if last_char_index < self.text.len() {
                let next_char = self.text[last_char_index..].chars().next()?;
                if Self::is_invalid_punctuation_symbol(next_char)
                    || Self::is_latin_letter(next_char)
                {
                    return None;
                }
            }
        }

        let number = match self.preferred_region {
            Some(region) => self
                .phone_util
                .parse_and_keep_raw_input_with_default_region(candidate, region),
            None => self.phone_util.parse(candidate),
        }
        .ok()?;

        if self
            .leniency
            .verify(&number, candidate, &self.phone_util, self)
        {
            // We used `parse_and_keep_raw_input` to create this number, but
            // for now we don't return the extra values parsed.
            // TODO: stop clearing all values here and switch all users over to
            // using `raw_input()` rather than the `raw_string()` of
            // `PhoneNumberMatch`.
            let mut number = number;
            number.country_code_source = None;
            number.raw_input = None;
            number.preferred_domestic_carrier_code = None;
            return Some(PhoneNumberMatch::new(offset, candidate, number));
        }

        None
    }

    // ── Public static helpers ─────────────────────────────────────────────────

    pub fn all_number_groups_remain_grouped(
        util: &PhoneNumberUtil,
        number: &PhoneNumber,
        normalized_candidate: &mut String,
        formatted_number_groups: &[String],
    ) -> bool {
        let mut from_index = 0usize;
        if number.country_code_source() != CountryCodeSource::FromDefaultCountry {
            // First skip the country code if the normalised candidate
            // contained it.
            let country_code = number.country_code.to_string();
            from_index = normalized_candidate
                .find(&country_code)
                .map_or(0, |i| i + country_code.len());
        }
        // Check each group of consecutive digits is not broken into separate
        // groupings in `normalized_candidate`.
        for (i, group) in formatted_number_groups.iter().enumerate() {
            // Fails if the substring of `normalized_candidate` starting from
            // `from_index` doesn't contain the consecutive digits in
            // `formatted_number_groups[i]`.
            from_index = match normalized_candidate[from_index..].find(group.as_str()) {
                Some(pos) => from_index + pos,
                None => return false,
            };
            // Move `from_index` forward.
            from_index += group.len();
            if i == 0 && from_index < normalized_candidate.len() {
                // We are at the position right after the NDC.  We get the
                // region used for formatting information based on the country
                // code in the phone number, rather than the number itself, as
                // we do not need to distinguish between different countries
                // with the same country calling code and this is faster.
                let Some(region) = util.get_region_code_for_country_code(number.country_code)
                else {
                    continue;
                };
                if util
                    .util_internal()
                    .get_ndd_prefix_for_region(&region, true)
                    .is_some()
                {
                    let next_is_digit = normalized_candidate[from_index..]
                        .chars()
                        .next()
                        .map(|c| c.is_ascii_digit())
                        .unwrap_or(false);
                    if next_is_digit {
                        // This means there is no formatting symbol after the
                        // NDC.  In this case, we only accept the number if
                        // there is no formatting symbol at all in the number,
                        // except for extensions.  This is only important for
                        // countries with national prefixes.
                        let nsn = util.get_national_significant_number(number);
                        let start = from_index - group.len();
                        return normalized_candidate[start..].starts_with(&*nsn);
                    }
                }
            }
        }
        // The check here makes sure that we haven't mistakenly already used
        // the extension to match the last group of the subscriber number.
        // Note the extension cannot have formatting in-between digits.
        normalized_candidate[from_index..].contains(number.extension())
    }

    pub fn all_number_groups_are_exactly_present<'b>(
        &self,
        number: &PhoneNumber,
        normalized_candidate: &mut String,
        mut formatted_number_groups: impl DoubleEndedIterator<Item = &'b str> + Clone,
    ) -> bool {
        let mut candidate_groups = normalized_candidate
            .split(|c: char| !c.is_ascii_digit())
            .rev()
            .peekable();

        // Set this to the last group, skipping it if the number has an
        // extension.
        let (mut candidate, is_single_group) = if number.extension.is_some() {
            let candidate = candidate_groups.nth(1);
            (candidate, candidate.is_none())
        } else {
            (candidate_groups.next(), candidate_groups.peek().is_none())
        };

        // First check if the national significant number is formatted as a
        // block.  We use `contains` and not `==`, since the national
        // significant number may be present with a prefix such as a national
        // number prefix, or the country code itself.
        if is_single_group
            || candidate.is_some_and(|g| {
                g.contains(&self.phone_util.get_national_significant_number(number))
            })
        {
            return true;
        }

        let first_formatted = formatted_number_groups.next();
        let formatted_rev = formatted_number_groups.rev();

        for next_formatted in formatted_rev {
            if Some(next_formatted) != candidate {
                return false;
            }
            candidate = candidate_groups.next();
        }

        match (candidate, first_formatted) {
            (Some(c), Some(f)) => c.ends_with(f),
            _ => false,
        }
    }

    /// Helper method to get the national-number part of a number, formatted
    /// without any national prefix, as a set of digit blocks.
    ///
    /// When `formatting_pattern` is `None`, standard RFC 3966 formatting is
    /// used (splitting on `'-'` after stripping the country code).  When it is
    /// `Some`, the NSN is formatted according to the supplied pattern before
    /// splitting.
    fn get_national_number_groups<'b>(
        &self,
        mut formatted_rfc_number: &'b str,
    ) -> Option<str::Split<'b, char>> {
        // We remove the extension part from the formatted string before splitting
        // it into different groups.
        if let Some(index) = formatted_rfc_number.find(';') {
            formatted_rfc_number = (&formatted_rfc_number)[..index].into();
        }

        if let Some(start_index) = formatted_rfc_number.find('-') {
            return formatted_rfc_number[start_index + 1..].split('-').into();
        }

        None
    }

    fn get_national_number_groups_for_pattern<'b>(
        &self,
        formatted_rfc_number: &'b str,
    ) -> str::Split<'b, char> {
        formatted_rfc_number.split('-')
    }

    pub fn check_number_grouping_is_valid(
        &self,
        number: &PhoneNumber,
        candidate: &str,
        util: &PhoneNumberUtil,
        checker_variant: CheckerVariant,
    ) -> bool {
        let normalized_candidate = normalize_digits(candidate);
        let formatted_rfc_number = self.phone_util.format(number, PhoneNumberFormat::RFC3966);
        let Some(formatted_number_groups) = self.get_national_number_groups(&formatted_rfc_number)
        else {
            return false;
        };
        if self.checker(
            checker_variant,
            number,
            &normalized_candidate,
            formatted_number_groups,
        ) {
            return true;
        }
        // If this didn't pass, see if there are any alternate formats that
        // match, and try them instead.
        let alternate_formats = self
            .alternate_formats
            .as_deref()
            .and_then(|formats| formats.get_alternate_formats_for_country(number.country_code));

        let nsn = util.get_national_significant_number(number);
        if let Some(alternate_formats) = alternate_formats {
            for alternate_format in &alternate_formats.number_format {
                if let Some(pattern) = alternate_format.leading_digits_pattern().get(0) {
                    // There is only one leading digits pattern for alternate
                    // formats.
                    if !pattern
                        .anchor_start()
                        .map_err(unwrap_regex_error)
                        .unwrap_or_else(|err| match err {})
                        .is_some_and(|pat| pat.is_match(&nsn))
                    {
                        // Leading digits don't match; try another one.
                        continue;
                    }
                }
                let mut buf = zeroes_itoa::LeadingZeroBuffer::new();
                let nsn = get_national_significant_number(number, &mut buf);
                let nsn_formatted = self
                    .phone_util
                    .util_internal()
                    .format_nsn_using_pattern(&nsn, alternate_format, PhoneNumberFormat::RFC3966)
                    .map_err(unwrap_internal)
                    .unwrap_or_else(|err| match err {});

                let formatted_number_groups =
                    self.get_national_number_groups_for_pattern(&nsn_formatted);
                if self.checker(
                    checker_variant,
                    number,
                    &normalized_candidate,
                    formatted_number_groups,
                ) {
                    return true;
                }
            }
        }
        false
    }

    pub fn contains_more_than_one_slash_in_national_number(
        &self,
        number: &PhoneNumber,
        candidate: &str,
    ) -> bool {
        let first_slash = match candidate.find('/') {
            Some(i) => i,
            None => return false, // No slashes, this is okay.
        };
        // Now look for a second one.
        let second_slash = match candidate[first_slash + 1..].find('/') {
            Some(i) => first_slash + 1 + i,
            None => return false, // Only one slash, this is okay.
        };

        // If the first slash is after the country calling code, this is
        // permitted.
        let candidate_has_country_code = matches!(
            number.country_code_source(),
            CountryCodeSource::FromNumberWithPlusSign
                | CountryCodeSource::FromNumberWithoutPlusSign
        );
        if candidate_has_country_code {
            let digits_before_slash = self
                .phone_util
                .normalize_digits_only(&candidate[..first_slash]);
            let mut buf = itoa::Buffer::new();
            let source = number.country_code_source();
            let source_str = buf.format(source as i32);
            if digits_before_slash == source_str {
                // Any more slashes and this is illegal.
                return candidate[second_slash + 1..].contains('/');
            }
        }
        true
    }

    pub fn contains_only_valid_x_chars(&self, number: &PhoneNumber, candidate: &str) -> bool {
        // The characters 'x' and 'X' can be (1) a carrier code, in which case
        // they always precede the national significant number or (2) an
        // extension sign, in which case they always precede the extension
        // number.  We assume a carrier code is more than 1 digit, so the
        // first case has to have more than 1 consecutive 'x' or 'X', whereas
        // the second case can only have exactly 1 'x' or 'X'.  We ignore the
        // character if it appears as the last character of the string.
        let mut iter = candidate.char_indices().peekable();
        while let Some((index, c)) = iter.next() {
            let Some((_, next)) = iter.peek().cloned() else {
                break;
            };
            if c != 'x' && c != 'X' {
                continue;
            }
            if next == 'x' || next == 'X' {
                // This is the carrier code case, in which the 'X's always
                // precede the national significant number.
                iter.next();
                let rest = &candidate[index + 1..];
                if self
                    .phone_util
                    .is_number_match_with_one_string(number, rest)
                    != Ok(MatchType::NsnMatch)
                {
                    return false;
                }
            } else {
                // This is the extension sign case, in which the 'x' or
                // 'X' should always precede the extension number.
                let rest = &candidate[index..];
                if self.phone_util.normalize_digits_only(rest) != number.extension() {
                    return false;
                }
            }
        }
        true
    }

    pub fn is_national_prefix_present_if_required(
        &self,
        number: &PhoneNumber,
        util: &PhoneNumberUtil,
    ) -> bool {
        // First, check how we deduced the country code.  If it was written in
        // international format, then the national prefix is not required.
        if number.country_code_source() != CountryCodeSource::FromDefaultCountry {
            return true;
        }
        let Some(phone_number_region) = util.get_region_code_for_country_code(number.country_code)
        else {
            return true;
        };
        let Some(metadata) = util
            .util_internal()
            .get_metadata_for_region(phone_number_region)
        else {
            return true;
        };
        // Check if a national prefix should be present when formatting this
        // number.
        let national_number = util.get_national_significant_number(number);
        let format_rule = util
            .util_internal()
            .choose_formatting_pattern_for_number(&metadata.number_format, &national_number)
            .map_err(unwrap_internal)
            .unwrap_or_else(|err| match err {});
        // To do this, we check that a national prefix formatting rule was
        // present and that it wasn't just the first-group symbol ($1) with
        // punctuation.
        if let Some(rule) = format_rule {
            if !rule.original.national_prefix_formatting_rule().is_empty() {
                if rule.original.national_prefix_optional_when_formatting() {
                    // The national-prefix is optional in these cases, so we
                    // don't need to check if it was present.
                    return true;
                }
                if self
                    .phone_util
                    .util_internal()
                    .formatting_rule_has_first_group_only(
                        rule.original.national_prefix_formatting_rule(),
                    )
                {
                    // National prefix not needed for this number.
                    return true;
                }
                // Normalize the remainder.
                let raw_input_copy = self.phone_util.normalize_digits_only(number.raw_input());
                // Check if we found a national prefix and/or carrier code at
                // the start of the raw input, and return the result.
                return util
                    .util_internal()
                    .maybe_strip_national_prefix_and_carrier_code(metadata, &raw_input_copy)
                    .map_err(unwrap_internal)
                    .unwrap_or_else(|err| match err {})
                    .1
                    .is_some();
            }
        }
        true
    }

    fn decrement_tries(&self) {
        self.max_tries.update(|t| t - 1);
    }

    fn checker<'b>(
        &self,
        variant: CheckerVariant,
        number: &PhoneNumber,
        normalized_candidate: &str,
        formatted_number_groups: impl Iterator<Item = &'b str>,
    ) -> bool {
    }
}

// ── Iterator ──────────────────────────────────────────────────────────────────

impl<'a, T: Deref<Target = PhoneNumberUtil>> Iterator for PhoneNumberMatcher<'a, T> {
    type Item = PhoneNumberMatch<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.state == State::NotReady {
            let index = self.search_index;
            self.last_match = PhoneNumberMatcher::find(&self, index);
            if self.last_match.is_none() {
                self.state = State::Done;
            } else {
                self.search_index = self.last_match.as_ref().unwrap().end();
                self.state = State::Ready;
            }
        }

        if self.state != State::Ready {
            return None;
        }

        // Don't retain that memory any longer than necessary.
        let result = self.last_match.take();
        self.state = State::NotReady;
        result
    }
}
