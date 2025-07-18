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


// The minimum and maximum length of the national significant number.
pub const MIN_LENGTH_FOR_NSN: usize = 2;
// The ITU says the maximum length should be 15, but we have found longer
// numbers in Germany.
pub const MAX_LENGTH_FOR_NSN: usize = 17;
/// The maximum length of the country calling code.
pub const MAX_LENGTH_COUNTRY_CODE: usize = 3;
pub const PLUS_CHARS: &'static str = "+\u{FF0B}";
// Regular expression of acceptable punctuation found in phone numbers. This
// excludes punctuation found as a leading character only. This consists of
// dash characters, white space characters, full stops, slashes, square
// brackets, parentheses and tildes. It also includes the letter 'x' as that
// is found as a placeholder for carrier information in some phone numbers.
// Full-width variants are also present.
pub const VALID_PUNCTUATION: &'static str = "-x\
\u{2010}-\u{2015}\u{2212}\u{30FC}\u{FF0D}-\u{FF0F} \u{00A0}\
\u{00AD}\u{200B}\u{2060}\u{3000}()\u{FF08}\u{FF09}\u{FF3B}\
\u{FF3D}.\\[\\]/~\u{2053}\u{223C}";

// Regular expression of characters typically used to start a second phone
// number for the purposes of parsing. This allows us to strip off parts of
// the number that are actually the start of another number, such as for:
// (530) 583-6985 x302/x2303 -> the second extension here makes this actually
// two phone numbers, (530) 583-6985 x302 and (530) 583-6985 x2303. We remove
// the second extension so that the first number is parsed correctly. The
// string preceding this is captured.
// This corresponds to SECOND_NUMBER_START in the java version.
pub const CAPTURE_UP_TO_SECOND_NUMBER_START: &'static str = r"(.*)[\\/] *x";


pub const REGION_CODE_FOR_NON_GEO_ENTITY: &'static str = "001";

pub const PLUS_SIGN: &'static str = "+";
pub const STAR_SIGN: &'static str = "*";
pub const RFC3966_EXTN_PREFIX: &'static str = ";ext=";
pub const RFC3966_PREFIX: &'static str = "tel:";
pub const RFC3966_PHONE_CONTEXT: &'static str = ";phone-context=";
pub const RFC3966_ISDN_SUBADDRESS: &'static str = ";isub=";
pub const RFC3966_VISUAL_SEPARATOR: &'static str = r"[\-\.\(\)]?";

pub const DIGITS: &'static str = r"\p{Nd}";

pub const VALID_ALPHA: &'static str = "a-z";
pub const VALID_ALPHA_INCL_UPPERCASE: &'static str = "A-Za-z";

// Default extension prefix to use when formatting. This will be put in front of
// any extension component of the number, after the main national number is
// formatted. For example, if you wish the default extension formatting to be "
// extn: 3456", then you should specify " extn: " here as the default extension
// prefix. This can be overridden by region-specific preferences.
pub const DEFAULT_EXTN_PREFIX: &'static str = " ext. ";

pub const POSSIBLE_SEPARATORS_BETWEEN_NUMBER_AND_EXT_LABEL: &'static str = "[ \u{00A0}\\t,]*";

// Optional full stop (.) or colon, followed by zero or more
// spaces/tabs/commas.
pub const POSSIBLE_CHARS_AFTER_EXT_LABEL: &'static str = "[:\\.\u{FF0E}]?[ \u{00A0}\\t,-]*";
pub const OPTIONAL_EXT_SUFFIX: &'static str = "#?";

pub const NANPA_COUNTRY_CODE: i32 = 1;
