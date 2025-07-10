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
