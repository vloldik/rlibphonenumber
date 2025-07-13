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

use regex::{Captures, Match, Regex};

pub trait RegexFullMatch {
    /// Eq of C fullMatch
    fn full_match(&self, s: &str) -> bool;
}

pub trait RegexConsume {
    fn matches_start<'a>(&self, s: &'a str) -> bool {
        self.find_start(s).is_some()
    }

    fn captures_start<'a>(&self, s: &'a str) -> Option<Captures<'a>>;
    fn find_start<'a>(&self, s: &'a str) -> Option<Match<'a>>;
}

impl RegexFullMatch for Regex {
    fn full_match(&self, s: &str) -> bool {
        let found = self.find(s);
        if let Some(matched) = found {
            return matched.start() == 0 && matched.end() == s.len();
        }
        false
    }
}

impl RegexConsume for Regex {
    fn captures_start<'a>(&self, s: &'a str) -> Option<Captures<'a>> {
        let captures = self.captures(s)?;
        let full_capture = captures.get(0)?;
        if full_capture.start() != 0 {
            return None
        }

        Some(captures)
    }

    fn find_start<'a>(&self, s: &'a str) -> Option<Match<'a>> {
        let found = self.find(s)?;
        if found.start() != 0 {
            return None
        }
        Some(found)
    }
}
