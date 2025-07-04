use std::borrow::Cow;

use regex::{Captures, Regex};

pub trait RegexFullMatch {
    /// Eq of C fullMatch
    fn full_match(&self, s: &str) -> bool;
}

pub trait RegexConsume {
    /// Eq of C Consume
    fn consume_start<'a>(&self, s: &'a str) -> Option<Cow<'a, str>> {
        self.consume_start_capturing(s).map(| res| res.0)
    }

    fn consume_start_capturing<'a>(&self, s: &'a str) -> Option<(Cow<'a, str>, Captures<'a>)>;
    
    fn find_and_consume<'a>(&self, s: &'a str) -> Option<Cow<'a, str>> {
        self.find_and_consume_capturing(s).map(| res| res.0)
    }
    
    fn find_and_consume_capturing<'a>(&self, s: &'a str) -> Option<(Cow<'a, str>, Captures<'a>)>;
}

trait RegexMatchStart {
    // Eq of looking_at
    fn match_start(&self, s: &str) -> bool;
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

impl RegexMatchStart for Regex {
    fn match_start(&self, s: &str) -> bool {
        let found = self.find(s);
        if let Some(matched) = found {
            return matched.start() == 0;
        }
        false
    }
}

impl RegexConsume for Regex {
    fn consume_start_capturing<'a>(&self, s: &'a str) -> Option<(Cow<'a, str>, Captures<'a>)> {
        _consume(self, s, true)
    }

    fn find_and_consume_capturing<'a>(&self, s: &'a str) -> Option<(Cow<'a, str>, Captures<'a>)> {
        _consume(self, s, false)
    }
}

fn _consume<'a>(
    r: &Regex, input: &'a str, 
    anchor_at_start: bool
) -> Option<(Cow<'a, str>, Captures<'a>)> {
    let captures = r.captures(input)?;
    let full_capture = captures.get(0)?;
    if anchor_at_start && full_capture.start() != 0 {
        return None
    }

    Some((Cow::Borrowed(&input[full_capture.end()..]), captures))
}