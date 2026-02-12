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

/// Strips prefix of given string Cow. Returns option with `Some` if 
/// prefix found and stripped.
/// 
/// Calls `drain` if string is owned and returns slice if string is borrowed
pub fn strip_cow_prefix<'a>(cow: Cow<'a, str>, prefix: &str) -> Option<Cow<'a, str>> {
    match cow {
        Cow::Borrowed(s) => s.strip_prefix(prefix).map(| s | Cow::Borrowed(s)),
        Cow::Owned(mut s) => {
            if s.starts_with(prefix) {
                s.drain(0..prefix.len());                               
                return Some(Cow::Owned(s));
            }
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use crate::string_util::strip_cow_prefix;

    #[test]
    fn test_usage() {
        let str_to_strip = Cow::Owned("test0:test".to_owned());
        let stripped = strip_cow_prefix(str_to_strip, "test0");
        assert_eq!(stripped, Some(Cow::Owned(":test".to_owned())));

        let str_to_strip = Cow::Owned("test:test0".to_owned());
        let stripped = strip_cow_prefix(str_to_strip, "test0");
        assert_eq!(stripped, None)
    }
}