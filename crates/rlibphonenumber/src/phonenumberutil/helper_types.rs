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

use crate::CountryCodeSource;

#[derive(Debug)]
pub struct PhoneNumberWithCountryCodeSource<'a> {
    pub phone_number: Cow<'a, str>,
    pub country_code_source: CountryCodeSource
}

impl<'a> PhoneNumberWithCountryCodeSource<'a> {
    pub fn new(phone_number: Cow<'a, str>, country_code_source: CountryCodeSource) -> Self {
        Self { phone_number, country_code_source }
    }
}
