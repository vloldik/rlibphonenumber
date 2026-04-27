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

/// This module is automatically generated from /resources/*.proto
mod generated;
mod phonenumberutil;
mod regex_based_matcher;
pub(crate) mod regexp;
pub(crate) mod string_util;

pub mod enums;
pub mod errors;
pub mod interfaces;
pub mod phonenumber_matcher;

pub use enums::*;
pub use errors::*;
pub use generated::proto::phone_number::CountryCodeSource;
pub use generated::proto::{
    NumberFormat, PhoneMetadata, PhoneMetadataCollection, PhoneNumber, PhoneNumberDesc,
};
pub use phonenumberutil::helper_functions::get_national_significant_number_owned;

#[cfg(feature = "global_static")]
mod phone_ext;
#[cfg(feature = "global_static")]
pub use crate::phonenumberutil::PHONE_NUMBER_UTIL;

#[cfg(feature = "serde")]
pub mod serde;

pub use phonenumberutil::phonenumberutil_internal::PhoneNumberUtil;
mod tests;
