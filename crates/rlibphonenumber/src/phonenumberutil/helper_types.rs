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

use std::borrow::Cow;

use crate::{
    CountryCodeSource, PhoneNumber, ValidationError,
    enums::PhoneNumberFormat,
    phonenumberutil::{
        helper_functions::{
            get_number_prefix_by_format_and_calling_code, test_number_length_with_unknown_type,
        },
        phonenumberutil_internal::{PhoneNumberUtilInternal, RegexResult},
        regex_wrapper_types::{NumberFormatWrapper, PhoneMetadataWrapper},
    },
};

#[derive(Debug)]
pub struct PhoneNumberWithCountryCodeSource<'a> {
    pub phone_number: Cow<'a, str>,
    pub country_code_source: CountryCodeSource,
}

impl<'a> PhoneNumberWithCountryCodeSource<'a> {
    pub fn new(phone_number: Cow<'a, str>, country_code_source: CountryCodeSource) -> Self {
        Self {
            phone_number,
            country_code_source,
        }
    }
}

type PhoneExt<'a> = Option<(&'a str, &'a str)>;
pub struct PrefixParts<'a>(pub [&'a str; 4], pub usize);

#[derive(Clone, Copy)]
pub enum FormatNsnArguments<'a> {
    Default,
    Format(&'a PhoneMetadataWrapper, PhoneNumberFormat),
    MobileDialing(&'a PhoneMetadataWrapper),
    RawBorrowed(&'a str),
    FormatAsNumber(PhoneNumberFormat),
    WithCarrier(&'a PhoneMetadataWrapper, PhoneNumberFormat, &'a str),
    WithPreferredCarrier,
    WithUserDefinedFormats(
        &'a PhoneMetadataWrapper,
        &'a [NumberFormatWrapper],
        PhoneNumberFormat,
    ),
}

#[derive(Clone, Copy)]
pub enum GetPrefixArguments<'a> {
    Default,
    WithFormat(PhoneNumberFormat),
    InternationalPrefix(&'a str),
}

pub struct FormattedNumberBuilder<'a, 'b> {
    util: &'b PhoneNumberUtilInternal,
    leading_zeroes: usize,
    number: &'a PhoneNumber,
    format_nsn_args: FormatNsnArguments<'a>,
    get_prefix_args: GetPrefixArguments<'a>,
    ext: PhoneExt<'a>,
}

pub fn new_formatted_number_builder<'a, 'b>(
    util: &'b PhoneNumberUtilInternal,
    number: &'a PhoneNumber,
) -> FormattedNumberBuilder<'a, 'b> {
    FormattedNumberBuilder {
        util,
        leading_zeroes: if number.italian_leading_zero() {
            number.number_of_leading_zeros().try_into().unwrap_or(0)
        } else {
            0
        },
        number,
        format_nsn_args: FormatNsnArguments::Default,
        get_prefix_args: GetPrefixArguments::Default,
        ext: None,
    }
}

impl<'a, 'b> FormattedNumberBuilder<'a, 'b> {
    pub fn with_ext(mut self, ext: PhoneExt<'a>) -> Self {
        self.ext = ext;
        self
    }

    pub fn with_get_prefix_args(mut self, get_prefix_args: GetPrefixArguments<'a>) -> Self {
        self.get_prefix_args = get_prefix_args;
        self
    }

    fn format_nsn<'n>(&self, nsn: &'n str) -> RegexResult<Cow<'n, str>>
    where
        'a: 'n,
    {
        match self.format_nsn_args {
            FormatNsnArguments::Default => Ok(Cow::Borrowed(nsn)),
            FormatNsnArguments::Format(phone_metadata_wrapper, phone_number_format) => self
                .util
                .format_nsn(nsn, phone_metadata_wrapper, phone_number_format),
            FormatNsnArguments::MobileDialing(phone_metadata_wrapper) => {
                let format = if self.util.can_be_internationally_dialled(self.number)?
                    && !test_number_length_with_unknown_type(nsn, phone_metadata_wrapper)
                        .is_err_and(|e| matches!(e, ValidationError::TooShort))
                {
                    PhoneNumberFormat::International
                } else {
                    PhoneNumberFormat::National
                };
                self.util.format(self.number, format)
            }
            FormatNsnArguments::RawBorrowed(b) => Ok(b.into()),
            FormatNsnArguments::FormatAsNumber(phone_number_format) => {
                self.util.format(self.number, phone_number_format)
            }
            FormatNsnArguments::WithCarrier(
                phone_metadata_wrapper,
                phone_number_format,
                carrier,
            ) => self.util.format_nsn_with_carrier(
                nsn,
                phone_metadata_wrapper,
                phone_number_format,
                carrier,
            ),
            FormatNsnArguments::WithPreferredCarrier => Ok(self
                .util
                .format_national_number_with_preferred_carrier_code(self.number, "")?
                .into()),
            FormatNsnArguments::WithUserDefinedFormats(
                metadata,
                number_format_wrappers,
                phone_number_format,
            ) => {
                let formatting_pattern = self
                    .util
                    .choose_formatting_pattern_for_number(number_format_wrappers, nsn)?;

                if let Some(formatting_pattern) = formatting_pattern {
                    // Before we do a replacement of the national prefix pattern $NP with the
                    // national prefix, we need to copy the rule so that subsequent replacements
                    // for different numbers have the appropriate national prefix.
                    let mut num_format_copy = formatting_pattern.clone();

                    let national_prefix_formatting_rule = formatting_pattern
                        .original
                        .national_prefix_formatting_rule();
                    if !national_prefix_formatting_rule.is_empty() {
                        let national_prefix = metadata.original.national_prefix();
                        if !national_prefix.is_empty() {
                            // Replace $NP with national prefix and $FG with the first group ($1).
                            let rule = national_prefix_formatting_rule
                                .replace("$NP", national_prefix)
                                .replace("$FG", "$1");
                            num_format_copy.original.national_prefix_formatting_rule = Some(rule);
                        } else {
                            // We don't want to have a rule for how to format the national prefix if
                            // there isn't one.
                            num_format_copy.original.national_prefix_formatting_rule = None;
                        }
                    }
                    self.util
                        .format_nsn_using_pattern(nsn, &num_format_copy, phone_number_format)
                } else {
                    Ok(nsn.into())
                }
            }
        }
    }

    fn get_prefix<'c>(&self, country_code: &'c str) -> PrefixParts<'c>
    where
        'a: 'c,
    {
        match self.get_prefix_args {
            GetPrefixArguments::Default => PrefixParts([""; 4], 0),
            GetPrefixArguments::WithFormat(phone_number_format) => {
                get_number_prefix_by_format_and_calling_code(country_code, phone_number_format)
            }
            GetPrefixArguments::InternationalPrefix(international_prefix_for_formatting) => {
                PrefixParts(
                    [international_prefix_for_formatting, " ", country_code, " "],
                    4,
                )
            }
        }
    }

    pub fn with_format_nsn_args(mut self, format_nsn_args: FormatNsnArguments<'a>) -> Self {
        self.format_nsn_args = format_nsn_args;
        self
    }

    pub fn early_exit(self) -> String {
        let mut nsn_buf = zeroes_itoa::LeadingZeroBuffer::new();
        let nsn = nsn_buf.format(self.number.national_number, self.leading_zeroes);
        nsn.to_string()
    }

    pub fn build(self) -> RegexResult<String> {
        let mut nsn_buf = zeroes_itoa::LeadingZeroBuffer::new();
        let mut country_code_buf = itoa::Buffer::new();

        let nsn = nsn_buf.format(self.number.national_number, self.leading_zeroes);
        let significant_national_number = self.format_nsn(&nsn)?;

        let country_code = country_code_buf.format(self.number.country_code);
        let prefix = self.get_prefix(country_code);

        let prefix_len = prefix.0[..prefix.1]
            .iter()
            .map(|item| item.len())
            .reduce(|acc, i| acc + i)
            .unwrap_or(0);

        let mut output = String::with_capacity(
            prefix_len
                + significant_national_number.len()
                + self.ext.map(|ext| ext.0.len() + ext.1.len()).unwrap_or(0),
        );

        prefix.0[..prefix.1].iter().for_each(|s| output.push_str(s));
        output.push_str(&significant_national_number);
        self.ext.inspect(|ext| {
            output.push_str(ext.0);
            output.push_str(ext.1);
        });

        Ok(output)
    }
}
