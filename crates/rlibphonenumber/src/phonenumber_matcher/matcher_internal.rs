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

use core::str;
use std::{cell::Cell, convert::Infallible, ops::Deref, sync::Arc};

use log::trace;
use rlibphonenumbers_macro::{export, public_wrapper};

use crate::{
    CountryCodeSource, PhoneNumber,
    alternate_formats::AlternateFormats,
    enums::{MatchType, PhoneNumberFormat, Region},
    errors::InternalError,
    generated::{uniprops_currencies, uniprops_latin_letters},
    interfaces::AsOriginal,
    phonenumber_matcher::{
        leniency::Leniency, matcher_regex::MatcherRegex, phonenumber_match::PhoneNumberMatch,
    },
    phonenumberutil::{
        helper_functions::{
            get_national_significant_number, is_unwanted_end_char, normalize_digits,
        },
        phonenumberutil_internal::PhoneNumberUtilInternal,
    },
    unwrap_internal_infallible,
};

/// The potential states of a [`PhoneNumberMatcher`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum State {
    NotReady,
    Ready,
    Done,
}

#[derive(Debug, Clone, Copy)]
enum CheckerVariant {
    AllNumberGroupsRemainGrouped,
    AllNumberGroupsAreExactlyPresent,
}

/// A stateful struct that finds and extracts telephone numbers from text.
/// Instances are created via the factory methods in [`PhoneNumberUtil`].
///
/// Vanity numbers (phone numbers using alphabetic digits such as
/// `1-800-SIX-FLAGS`) are not found.
///
/// This struct is not thread-safe.
#[derive(Debug, Clone)]
pub struct PhoneNumberMatcherInternal<
    'a,
    U: AsOriginal<PhoneNumberUtilInternal>,
    T: Deref<Target = U>,
> {
    regexps: Arc<MatcherRegex>,
    // ── instance state ────────────────────────────────────────────────────────
    /// The phone number utility.
    _phone_util: T,
    /// The text searched for phone numbers.
    text: &'a str,
    /// The region (country) to assume for phone numbers without an
    /// international prefix, or `None` if only numbers with a leading plus
    /// should be considered.
    preferred_region: Option<Region>,
    /// The degree of validation requested.
    leniency: Leniency,
    /// The maximum number of retries after matching an invalid number.
    max_tries: Cell<u64>,

    /// The iteration tristate.
    state: State,
    /// The last successful match; `None` unless in [`State::Ready`].
    last_match: Option<PhoneNumberMatch<'a>>,
    /// The next index to start searching at.  Undefined in [`State::Done`].
    search_index: usize,

    alternate_formats: Option<Arc<AlternateFormats>>,
}

#[public_wrapper(
    PhoneNumberMatcher {
        ret: Self -> Self => | v | Self { inner: v },
        ret: Result<$t, InternalError<Infallible>> -> $t => | v | unwrap_internal_infallible(v)
    },

    PhoneNUmberMatcherFallible {
        ret: Self -> Self => | v | Self { inner: v },
    }
)]
impl<'a, U: AsOriginal<PhoneNumberUtilInternal>, T: Deref<Target = U>>
    PhoneNumberMatcherInternal<'a, U, T>
{
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
    #[export]
    pub fn new_for_util(
        util: T,
        regexps: Arc<MatcherRegex>,
        text: &'a str,
        preferred_region: Option<Region>,
        leniency: Leniency,
        max_tries: u64,
        alternate_formats: Option<Arc<AlternateFormats>>,
    ) -> Self {
        Self {
            regexps,
            _phone_util: util,
            text,
            preferred_region,
            leniency,
            max_tries: Cell::new(max_tries),
            state: State::NotReady,
            last_match: None,
            search_index: 0,
            alternate_formats,
        }
    }

    fn phone_util(&self) -> &PhoneNumberUtilInternal {
        self._phone_util.as_original()
    }

    /// Attempts to find the next subsequence in the searched text on or after
    /// `index` that represents a phone number.  Returns the next match, or
    /// `None` if none was found.
    fn find(
        &self,
        index: usize,
    ) -> Result<Option<PhoneNumberMatch<'a>>, InternalError<Infallible>> {
        let mut pos = index;

        while self.max_tries.get() > 0 {
            let Some(m) = self.regexps.pattern.find_at(self.text, pos) else {
                return Ok(None);
            };
            let start = m.start();
            let mut candidate = m.as_str();
            trace!("Found candidate: {candidate}, {start}");

            // Check for extra numbers at the end.
            // TODO: This is the place to start when trying to support
            // extraction of multiple phone numbers from split notations
            // (+41 79 123 45 67 / 68).
            candidate = self
                .phone_util()
                .reg_exps
                .capture_up_to_second_number_start_pattern
                .captures(candidate)
                .and_then(|m| m.get(1))
                .map(|c| c.as_str())
                .unwrap_or(candidate);

            trace!("Stripped candidate: {candidate}, {start}");

            let extract_match = self.extract_match(candidate, start)?;
            if let Some(result) = extract_match {
                return Ok(Some(result));
            } else {
                pos = start + candidate.len();
                self.decrement_tries();
            }
        }

        Ok(None)
    }

    /// Helper method to determine if a character is a Latin-script letter or
    /// not.  For our purposes, combining marks should also return `true` since
    /// we assume they have been added to a preceding Latin character.
    pub fn is_latin_letter(letter: char) -> bool {
        uniprops_latin_letters::uniprops::Category::from_char(letter).is_some()
    }

    pub fn is_invalid_punctuation_symbol(character: char) -> bool {
        character == '%'
            || uniprops_currencies::uniprops::Category::from_char(character)
                == Some(uniprops_currencies::uniprops::Category::Sc)
    }

    /// Attempts to extract a match from a `candidate` character sequence.
    ///
    /// * `candidate` – the candidate text that might contain a phone number
    /// * `offset`    – the offset of `candidate` within [`Self::text`]
    fn extract_match(
        &self,
        candidate: &'a str,
        offset: usize,
    ) -> Result<Option<PhoneNumberMatch<'a>>, InternalError<Infallible>> {
        // Skip a match that is more likely to be a date.
        if self.regexps.slash_separated_dates.find(candidate).is_some() {
            return Ok(None);
        }

        // Skip potential time-stamps.
        if self.regexps.time_stamps.find(candidate).is_some() {
            let following_text = &self.text[offset + candidate.len()..];
            if self.regexps.time_stamps_suffix.is_match(following_text) {
                return Ok(None);
            }
        }

        // Try to come up with a valid match given the entire candidate.
        if let Some(result) = self.parse_and_verify(candidate, offset)? {
            return Ok(Some(result));
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
    fn extract_inner_match(
        &self,
        candidate: &'a str,
        offset: usize,
    ) -> Result<Option<PhoneNumberMatch<'a>>, InternalError<Infallible>> {
        trace!("Extracting inner match");
        // Clone to satisfy the borrow checker — `inner_matches` is borrowed
        // immutably by the loop while `self` must also be borrowed mutably for
        // the parse calls.

        for possible_inner_match in &self.regexps.inner_matches {
            let mut is_first_match = true;
            let mut search_pos = 0;

            while self.max_tries.get() > 0 {
                let group_m = match possible_inner_match.find_at(candidate, search_pos) {
                    Some(m) => m,
                    None => break,
                };
                trace!("Found group {}", group_m.as_str());

                if is_first_match {
                    // We should handle any group before this one too.
                    let before =
                        candidate[..group_m.start()].trim_end_matches(is_unwanted_end_char);
                    if let Some(result) = self.parse_and_verify(before, offset)? {
                        return Ok(Some(result));
                    }
                    trace!("Parsed before: {}", before);
                    self.decrement_tries();
                    is_first_match = false;
                }

                let group1 = possible_inner_match
                    .captures_at(candidate, group_m.start())
                    .and_then(|c| c.get(1))
                    .map(|m| m.as_str())
                    .unwrap_or("");

                let group = group1.trim_end_matches(is_unwanted_end_char);

                trace!("Found group: {}", group);
                let group_offset =
                    offset + (group1.as_ptr() as usize - candidate.as_ptr() as usize);

                if let Some(result) = self.parse_and_verify(group, group_offset)? {
                    return Ok(Some(result));
                }
                self.decrement_tries();

                search_pos = group_m.end();
            }
        }
        Ok(None)
    }

    /// Parses a phone number from `candidate` using
    /// [`PhoneNumberUtil::parse_and_keep_raw_input`] and verifies it matches
    /// the requested [`leniency`].  Returns a [`PhoneNumberMatch`] on success,
    /// or `None` otherwise.
    fn parse_and_verify(
        &self,
        candidate: &'a str,
        offset: usize,
    ) -> Result<Option<PhoneNumberMatch<'a>>, InternalError<Infallible>> {
        // Check the candidate doesn't contain any formatting which would
        // indicate that it really isn't a phone number.
        if !self
            .regexps
            .matching_brackets_full_match
            .is_match(candidate)
            || self.regexps.pub_pages.find(candidate).is_some()
        {
            return Ok(None);
        }

        // If leniency is set to VALID or stricter, we also want to skip
        // numbers that are surrounded by Latin alphabetic characters, to skip
        // cases like abc8005001234 or 8005001234def.
        if self.leniency >= Leniency::Valid {
            // If the candidate is not at the start of the text, and does not
            // start with phone-number punctuation, check the previous
            // character.
            if offset > 0 && !self.regexps.lead_class.is_match(candidate) {
                let Some(previous_char) = self.text[..offset].chars().last() else {
                    return Ok(None);
                };
                // We return None if it is a latin letter or an invalid
                // punctuation symbol.
                if Self::is_invalid_punctuation_symbol(previous_char)
                    || Self::is_latin_letter(previous_char)
                {
                    return Ok(None);
                }
            }
            let last_char_index = offset + candidate.len();
            if last_char_index < self.text.len() {
                let Some(next_char) = self.text[last_char_index..].chars().next() else {
                    return Ok(None);
                };
                if Self::is_invalid_punctuation_symbol(next_char)
                    || Self::is_latin_letter(next_char)
                {
                    return Ok(None);
                }
            }
        }

        let number = match self
            .phone_util()
            .parse_and_keep_raw_input(candidate, self.preferred_region)
        {
            Ok(number) => number,
            Err(InternalError::RegexError(e)) => return Err(InternalError::RegexError(e)),
            Err(InternalError::Wrapped(_)) => {
                return Ok(None);
            }
        };
        if self.verify_according_to_leniency(&number, candidate)? {
            // We used `parse_and_keep_raw_input` to create this number, but
            // for now we don't return the extra values parsed.
            // TODO: stop clearing all values here and switch all users over to
            // using `raw_input()` rather than the `raw_string()` of
            // `PhoneNumberMatch`.
            let mut number = number;
            number.country_code_source = None;
            number.raw_input = None;
            number.preferred_domestic_carrier_code = None;
            return Ok(Some(PhoneNumberMatch::new(offset, candidate, number)));
        }
        trace!("Failed to verify leniency for number, {number}, {candidate}");

        Ok(None)
    }

    fn all_number_groups_remain_grouped<'b>(
        &self,
        number: &PhoneNumber,
        normalized_candidate: &str,
        formatted_number_groups: impl DoubleEndedIterator<Item = &'b str>,
    ) -> Result<bool, InternalError<Infallible>> {
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
        for (i, group) in formatted_number_groups.enumerate() {
            // Fails if the substring of `normalized_candidate` starting from
            // `from_index` doesn't contain the consecutive digits in
            // `formatted_number_groups[i]`.
            from_index = match normalized_candidate[from_index..].find(group) {
                Some(pos) => from_index + pos,
                None => return Ok(false),
            };
            // Move `from_index` forward.
            from_index += group.len();
            if i == 0 && from_index < normalized_candidate.len() {
                // We are at the position right after the NDC.  We get the
                // region used for formatting information based on the country
                // code in the phone number, rather than the number itself, as
                // we do not need to distinguish between different countries
                // with the same country calling code and this is faster.
                let Some(region) = self
                    .phone_util()
                    .get_region_for_country_code(number.country_code)
                else {
                    continue;
                };
                if self
                    .phone_util()
                    .get_ndd_prefix_for_region(region, true)
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
                        let mut buf = zeroes_itoa::LeadingZeroBuffer::new();
                        let nsn = get_national_significant_number(number, &mut buf);
                        let start = from_index - group.len();
                        return Ok(normalized_candidate[start..].starts_with(&*nsn));
                    }
                }
            }
        }
        // The check here makes sure that we haven't mistakenly already used
        // the extension to match the last group of the subscriber number.
        // Note the extension cannot have formatting in-between digits.
        Ok(normalized_candidate[from_index..].contains(number.extension()))
    }

    fn all_number_groups_are_exactly_present<'b>(
        &self,
        number: &PhoneNumber,
        normalized_candidate: &str,
        mut formatted_number_groups: impl DoubleEndedIterator<Item = &'b str> + Clone,
    ) -> bool {
        let mut candidate_groups = normalized_candidate
            .split(|c: char| !c.is_ascii_digit())
            .filter(|s| !s.is_empty())
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
                let mut buf = zeroes_itoa::LeadingZeroBuffer::new();
                let nsn = get_national_significant_number(number, &mut buf);

                g.contains(nsn.deref())
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
            formatted_rfc_number = formatted_rfc_number[..index].into();
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

    fn check_number_grouping_is_valid(
        &self,
        number: &PhoneNumber,
        candidate: &str,
        checker_variant: CheckerVariant,
    ) -> Result<bool, InternalError<Infallible>> {
        let normalized_candidate = normalize_digits(candidate);
        let formatted_rfc_number = self
            .phone_util()
            .format(number, PhoneNumberFormat::RFC3966)?;
        let Some(formatted_number_groups) = self.get_national_number_groups(&formatted_rfc_number)
        else {
            return Ok(false);
        };
        if self.checker(
            checker_variant,
            number,
            &normalized_candidate,
            formatted_number_groups,
        )? {
            return Ok(true);
        }
        // If this didn't pass, see if there are any alternate formats that
        // match, and try them instead.
        let alternate_formats = self
            .alternate_formats
            .as_deref()
            .and_then(|formats| formats.get_alternate_formats_for_country(number.country_code));

        let mut buf = zeroes_itoa::LeadingZeroBuffer::new();
        let nsn = get_national_significant_number(number, &mut buf);
        if let Some(alternate_formats) = alternate_formats {
            for alternate_format in &alternate_formats.number_format {
                if let Some(pattern) = alternate_format.leading_digits_pattern().first() {
                    // There is only one leading digits pattern for alternate
                    // formats.
                    if !pattern
                        .anchor_start()?
                        .is_some_and(|pat| pat.is_match(&nsn))
                    {
                        // Leading digits don't match; try another one.
                        continue;
                    }
                }
                let mut buf = zeroes_itoa::LeadingZeroBuffer::new();
                let nsn = get_national_significant_number(number, &mut buf);
                let nsn_formatted = self.phone_util().format_nsn_using_pattern(
                    &nsn,
                    alternate_format,
                    PhoneNumberFormat::RFC3966,
                )?;

                let formatted_number_groups =
                    self.get_national_number_groups_for_pattern(&nsn_formatted);
                if self.checker(
                    checker_variant,
                    number,
                    &normalized_candidate,
                    formatted_number_groups,
                )? {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    pub fn contains_more_than_one_slash_in_national_number(
        &self,
        number: &PhoneNumber,
        candidate: &str,
    ) -> bool {
        let first_slash = match candidate.find('/') {
            Some(i) => i,
            None => return false,
        };
        // Now look for a second one.
        let second_slash = match candidate[first_slash + 1..].find('/') {
            Some(i) => first_slash + 1 + i,
            None => return false,
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
                .phone_util()
                .normalize_digits_only(&candidate[..first_slash]);
            let mut buf = itoa::Buffer::new();
            let cc_str = buf.format(number.country_code);
            if digits_before_slash == cc_str {
                return candidate[second_slash + 1..].contains('/');
            }
        }
        true
    }

    fn contains_only_valid_x_chars(&self, number: &PhoneNumber, candidate: &str) -> bool {
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
                    .phone_util()
                    .is_number_match_with_one_string(number, rest)
                    != Ok(MatchType::NsnMatch)
                {
                    return false;
                }
            } else {
                // This is the extension sign case, in which the 'x' or
                // 'X' should always precede the extension number.
                let rest = &candidate[index..];
                if self.phone_util().normalize_digits_only(rest) != number.extension() {
                    return false;
                }
            }
        }
        true
    }

    fn is_national_prefix_present_if_required(
        &self,
        number: &PhoneNumber,
    ) -> Result<bool, InternalError<Infallible>> {
        // First, check how we deduced the country code.  If it was written in
        // international format, then the national prefix is not required.
        if number.country_code_source() != CountryCodeSource::FromDefaultCountry {
            return Ok(true);
        }
        let Some(phone_number_region) = self
            .phone_util()
            .get_region_for_country_code(number.country_code)
        else {
            return Ok(true);
        };
        let Some(metadata) = self
            .phone_util()
            .get_metadata_for_region(phone_number_region)
        else {
            return Ok(true);
        };
        // Check if a national prefix should be present when formatting this
        // number.
        let mut buf = zeroes_itoa::LeadingZeroBuffer::new();
        let nsn = get_national_significant_number(number, &mut buf);

        let format_rule = self
            .phone_util()
            .choose_formatting_pattern_for_number(&metadata.number_format, &nsn)?;
        // To do this, we check that a national prefix formatting rule was
        // present and that it wasn't just the first-group symbol ($1) with
        // punctuation.
        if let Some(rule) = format_rule
            && rule.original.national_prefix_formatting_rule.is_some()
        {
            if rule.original.national_prefix_optional_when_formatting() {
                // The national-prefix is optional in these cases, so we
                // don't need to check if it was present.
                return Ok(true);
            }
            if self.phone_util().formatting_rule_has_first_group_only(
                rule.original.national_prefix_formatting_rule(),
            ) {
                // National prefix not needed for this number.
                return Ok(true);
            }
            // Normalize the remainder.
            let raw_input_copy = self.phone_util().normalize_digits_only(number.raw_input());
            // Check if we found a national prefix and/or carrier code at
            // the start of the raw input, and return the result.

            return Ok(self
                .phone_util()
                .maybe_strip_national_prefix_and_carrier_code(metadata, &raw_input_copy)?
                .0
                != raw_input_copy);
        }
        Ok(true)
    }

    fn decrement_tries(&self) {
        self.max_tries.update(|t| t - 1);
    }

    fn checker<'b>(
        &self,
        variant: CheckerVariant,
        number: &PhoneNumber,
        normalized_candidate: &str,
        formatted_number_groups: impl DoubleEndedIterator<Item = &'b str> + Clone,
    ) -> Result<bool, InternalError<Infallible>> {
        match variant {
            CheckerVariant::AllNumberGroupsAreExactlyPresent => Ok(self
                .all_number_groups_are_exactly_present(
                    number,
                    normalized_candidate,
                    formatted_number_groups,
                )),
            CheckerVariant::AllNumberGroupsRemainGrouped => self.all_number_groups_remain_grouped(
                number,
                normalized_candidate,
                formatted_number_groups,
            ),
        }
    }

    fn verify_according_to_leniency(
        &self,
        phone_number: &PhoneNumber,
        candidate: &str,
    ) -> Result<bool, InternalError<Infallible>> {
        trace!(
            "IS POSSIBLE {candidate}, {phone_number}, {:?}",
            self.phone_util()
                .is_possible_number_with_reason(phone_number)
        );
        let is_valid = || {
            Ok::<_, InternalError<Infallible>>(
                self.phone_util().is_valid_number(phone_number)?
                    && self.contains_only_valid_x_chars(phone_number, candidate)
                    && self.is_national_prefix_present_if_required(phone_number)?,
            )
        };
        let result = match self.leniency {
            Leniency::Possible => self.phone_util().is_possible_number(phone_number),
            Leniency::Valid => is_valid()?,
            Leniency::StrictGrouping => {
                is_valid()?
                    && !self
                        .contains_more_than_one_slash_in_national_number(phone_number, candidate)
                    && self.check_number_grouping_is_valid(
                        phone_number,
                        candidate,
                        CheckerVariant::AllNumberGroupsRemainGrouped,
                    )?
            }
            Leniency::ExactGrouping => {
                is_valid()?
                    && !self
                        .contains_more_than_one_slash_in_national_number(phone_number, candidate)
                    && self.check_number_grouping_is_valid(
                        phone_number,
                        candidate,
                        CheckerVariant::AllNumberGroupsAreExactlyPresent,
                    )?
            }
        };

        Ok(result)
    }
}

impl<'a, U: AsOriginal<PhoneNumberUtilInternal>, T: Deref<Target = U>> Iterator
    for PhoneNumberMatcherInternal<'a, U, T>
{
    type Item = Result<PhoneNumberMatch<'a>, InternalError<Infallible>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.state == State::NotReady {
            let index = self.search_index;
            let new_match = PhoneNumberMatcherInternal::<'a>::find(self, index);
            self.last_match = match new_match {
                Ok(last_match) => last_match,
                Err(err) => return Some(Err(err)),
            };
            if let Some(item) = &self.last_match {
                self.search_index = item.end();
                self.state = State::Ready;
            } else {
                self.state = State::Done;
            }
        }

        if self.state != State::Ready {
            return None;
        }

        // Don't retain that memory any longer than necessary.
        let result = self.last_match.take();
        self.state = State::NotReady;
        Ok(result).transpose()
    }
}

impl<'a, U: AsOriginal<PhoneNumberUtilInternal>, T: Deref<Target = U>> Iterator
    for PhoneNumberMatcher<'a, U, T>
{
    type Item = PhoneNumberMatch<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        unwrap_internal_infallible(self.inner.next().transpose())
    }
}

impl<'a, U: AsOriginal<PhoneNumberUtilInternal>, T: Deref<Target = U>> Iterator
    for PhoneNUmberMatcherFallible<'a, U, T>
{
    type Item = Result<PhoneNumberMatch<'a>, InternalError<Infallible>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<'a, U: AsOriginal<PhoneNumberUtilInternal>, T: Deref<Target = U>>
    AsOriginal<PhoneNumberMatcherInternal<'a, U, T>> for PhoneNumberMatcher<'a, U, T>
{
    fn as_original(&self) -> &PhoneNumberMatcherInternal<'a, U, T> {
        &self.inner
    }
}

impl<'a, U: AsOriginal<PhoneNumberUtilInternal>, T: Deref<Target = U>>
    AsOriginal<PhoneNumberMatcherInternal<'a, U, T>> for PhoneNUmberMatcherFallible<'a, U, T>
{
    fn as_original(&self) -> &PhoneNumberMatcherInternal<'a, U, T> {
        &self.inner
    }
}

impl<'a, U: AsOriginal<PhoneNumberUtilInternal>, T: Deref<Target = U>>
    AsOriginal<PhoneNumberMatcherInternal<'a, U, T>> for PhoneNumberMatcherInternal<'a, U, T>
{
    fn as_original(&self) -> &PhoneNumberMatcherInternal<'a, U, T> {
        self
    }
}
