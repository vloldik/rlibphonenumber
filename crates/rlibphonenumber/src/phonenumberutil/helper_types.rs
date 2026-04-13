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
    CountryCodeSource, PhoneNumber, PhoneNumberFormat,
    phonenumberutil::{
        helper_functions::get_number_prefix_by_format_and_calling_code,
        phonenumberutil_internal::RegexResult,
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

type PrefixPartsRaw<'a, const PREFIX_LEN: usize> = [Cow<'a, str>; PREFIX_LEN];
type PhoneExt<'a, 'b> = Option<(&'a str, &'b str)>;

pub enum PrefixParts<'a> {
    Parts2(PrefixPartsRaw<'a, 2>),
    Parts3(PrefixPartsRaw<'a, 3>),
    Parts4(PrefixPartsRaw<'a, 4>),
    Empty,
}

macro_rules! dispatch {
    ($self:ident.$method:ident ($(, $args:expr)*) ?? $default:expr) => {
        match $self {
            PrefixParts::Parts2(inner) => inner.$method($($args),*),
            PrefixParts::Parts3(inner) => inner.$method($($args),*),
            PrefixParts::Parts4(inner) => inner.$method($($args),*),
            PrefixParts::Empty => $default,
        }
    };
}

impl<'a> PrefixParts<'a> {
    fn iter(&'_ self) -> impl Iterator<Item = &'_ Cow<'a, str>> {
        dispatch!(self.iter() ?? Default::default())
    }
}

pub struct FormattedNumberBuilder<
    'a,
    'b,
    F: FnOnce(&str) -> RegexResult<Cow<'_, str>>,
    FP: FnOnce(&str) -> PrefixParts<'_>,
> {
    leading_zeroes: usize,
    number: &'a PhoneNumber,
    format_number_fn: F,
    get_prefix_fn: FP,
    ext: PhoneExt<'b, 'a>,
}

pub fn new_formatted_number_builder<'a>(
    number: &'a PhoneNumber,
    format: Option<PhoneNumberFormat>,
) -> FormattedNumberBuilder<
    'a,
    'a,
    impl FnOnce(&str) -> RegexResult<Cow<'_, str>>,
    impl FnOnce(&str) -> PrefixParts<'_>,
> {
    fn format_number_fn(number: &str) -> RegexResult<Cow<'_, str>> {
        Ok(number.into())
    }

    FormattedNumberBuilder {
        leading_zeroes: if number.italian_leading_zero() {
            number.number_of_leading_zeros().try_into().unwrap_or(0)
        } else {
            0
        },
        number,
        format_number_fn,
        get_prefix_fn: move |code| {
            format.map_or(PrefixParts::Empty, |format| {
                get_number_prefix_by_format_and_calling_code(code, format)
            })
        },
        ext: None,
    }
}

impl<'a, 'b, F: FnOnce(&str) -> RegexResult<Cow<'_, str>>, FP: FnOnce(&str) -> PrefixParts<'_>>
    FormattedNumberBuilder<'a, 'b, F, FP>
{
    pub fn with_ext(mut self, ext: PhoneExt<'b, 'a>) -> Self {
        self.ext = ext;
        self
    }

    pub fn with_get_prefix_function<N: FnOnce(&str) -> PrefixParts<'_>>(
        self,
        get_prefix_fn: N,
    ) -> FormattedNumberBuilder<'a, 'b, F, N> {
        FormattedNumberBuilder {
            leading_zeroes: self.leading_zeroes,
            number: self.number,
            format_number_fn: self.format_number_fn,
            get_prefix_fn,
            ext: self.ext,
        }
    }

    pub fn with_format_nsn_function<N: FnOnce(&str) -> RegexResult<Cow<str>>>(
        self,
        number: N,
    ) -> FormattedNumberBuilder<'a, 'b, N, FP> {
        FormattedNumberBuilder {
            leading_zeroes: self.leading_zeroes,
            number: self.number,
            format_number_fn: number,
            get_prefix_fn: self.get_prefix_fn,
            ext: self.ext,
        }
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
        let significant_national_number = (self.format_number_fn)(&nsn)?;

        let country_code = country_code_buf.format(self.number.country_code);
        let prefix = (self.get_prefix_fn)(country_code);

        let prefix_len = prefix
            .iter()
            .map(|item| item.len())
            .reduce(|acc, i| acc + i)
            .unwrap_or(0);

        let mut output = String::with_capacity(
            prefix_len
                + significant_national_number.len()
                + self.ext.map(|ext| ext.0.len() + ext.1.len()).unwrap_or(0),
        );

        prefix.iter().for_each(|s| output.push_str(s));
        output.push_str(&significant_national_number);
        self.ext.inspect(|ext| {
            output.push_str(ext.0);
            output.push_str(ext.1);
        });

        Ok(output)
    }
}
