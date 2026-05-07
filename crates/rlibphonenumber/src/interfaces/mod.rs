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

use crate::{
    PhoneNumber,
    errors::InvalidRegexError,
    phonenumber_mask::{self, Hashed},
    phonenumberutil::regex_wrapper_types::PhoneNumberDescWrapper,
};

/// Internal phonenumber matching API used to isolate the underlying
/// implementation of the matcher and allow different implementations to be
/// swapped in easily.
pub(crate) trait MatcherApi: Send + Sync {
    /// Returns whether the given national number (a string containing only decimal
    /// digits) matches the national number pattern defined in the given
    /// PhoneNumberDesc message.
    fn match_national_number(
        &self,
        number: &str,
        number_desc: &PhoneNumberDescWrapper,
        allow_prefix_match: bool,
    ) -> Result<bool, InvalidRegexError>;
}

// Used for wrappers to get common behavior on different wrappers
pub trait AsOriginal<T> {
    fn as_original(&self) -> &T;
}

pub trait LenWrite {
    fn grow(&mut self, len: usize);
    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()>;
}

impl LenWrite for String {
    fn grow(&mut self, len: usize) {
        self.reserve_exact(len);
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        let s = std::str::from_utf8(buf)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        self.push_str(s);
        Ok(())
    }
}

macro_rules! impl_len_write {
    ($($t:ty),+) => {
        $(impl LenWrite for $t {
            fn grow(&mut self, _: usize) {}

            fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
                std::io::Write::write_all(self, buf)
            }
        })+
    };
}

impl_len_write! {
    ::std::fs::File,
    ::std::io::Stdout
}

pub trait PhoneHasher {
    fn hash_phone(self, phone: &PhoneNumber) -> phonenumber_mask::Result<Hashed>;
}

pub trait OptionalHasher {
    fn hash_phone(self, phone: &PhoneNumber) -> phonenumber_mask::Result<Option<Hashed>>;
}

impl<T: PhoneHasher> OptionalHasher for T {
    fn hash_phone(self, phone: &PhoneNumber) -> phonenumber_mask::Result<Option<Hashed>> {
        Ok(Some(PhoneHasher::hash_phone(self, phone)?))
    }
}
