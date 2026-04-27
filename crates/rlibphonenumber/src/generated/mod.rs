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

pub mod metadata;
#[allow(deprecated)]
pub mod proto;

pub mod uniprops_digits {
    include!(concat!(env!("OUT_DIR"), "/uniprops_digits.rs"));
}

pub mod uniprops_without_nl {
    include!(concat!(env!("OUT_DIR"), "/uniprops_without_nl.rs"));
}

pub mod uniprops_latin_letters {
    include!(concat!(env!("OUT_DIR"), "/uniprops_latin_letters.rs"));
}

pub mod uniprops_currencies {
    include!(concat!(env!("OUT_DIR"), "/uniprops_currencies.rs"));
}

#[cfg(all(feature = "lite", not(feature = "regex")))]
pub mod uniprops_digits_pat {
    include!(concat!(env!("OUT_DIR"), "/uniprops_digits_pat.rs"));
}

#[cfg(all(feature = "lite", not(feature = "regex")))]
pub mod uniprops_separators_pat {
    include!(concat!(env!("OUT_DIR"), "/uniprops_separators_pat.rs"));
}
