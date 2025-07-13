// Copyright (C) 2009 The Libphonenumber Authors
// Copyright (C) 2025 The Kashin Vladislav (Rust adaptation author)
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

mod interfaces;
/// This module is automatically generated from /resources/*.proto
mod generated;
mod phonenumberutil;
mod regexp_cache;
mod regex_based_matcher;
pub mod region_code;
pub(crate) mod regex_util;
pub(crate) mod string_util;

/// I decided to create this module because there are many 
/// boilerplate places in the code that can be replaced with macros, 
/// the name of which will describe what is happening more 
/// clearly than a few lines of code.
mod macros;

pub use phonenumberutil::{
    PHONE_NUMBER_UTIL,
    phonenumberutil::{
        RegexResult,
        MatchResult,
        ParseResult,
        ValidationResult,
        ExampleNumberResult,
        InternalLogicResult,
        ExtractNumberResult,
        PhoneNumberUtil
    },
    errors::{*},
    enums::{*},
};
pub use generated::proto::phonemetadata::{*};
pub use generated::proto::phonenumber::PhoneNumber;
pub use generated::proto::phonenumber::phone_number::CountryCodeSource;
mod tests;
