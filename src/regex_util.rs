use std::borrow::Cow;

use regex::Regex;

pub trait RegexFullMatch {
    /// Eq of C fullMatch
    fn full_match(&self, s: &str) -> bool;
}

pub trait RegexConsume {
    /// Eq of C Consume
    fn consume_start<'a>(&self, s: &'a str) -> Option<Cow<'a, str>> {
        self.consume_start_capturing(s, &mut [])
    }

    fn consume_start_capturing<'a, 'b>(&self, s: &'a str, groups: &mut [&'b str]) -> Option<Cow<'a, str>>
        where 'a: 'b;
    
    fn find_and_consume<'a>(&self, s: &'a str) -> Option<Cow<'a, str>> {
        self.find_and_consume_capturing(s, &mut [])
    }
    
    fn find_and_consume_capturing<'a, 'b>(&self, s: &'a str, groups: &mut [&'b str]) -> Option<Cow<'a, str>>
        where 'a: 'b;
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
    fn consume_start_capturing<'a, 'b>(&self, s: &'a str, groups: &mut [&'b str]) -> Option<Cow<'a, str>>
        where 'a: 'b {
        _consume(self, s, groups, true)
    }

    fn find_and_consume_capturing<'a, 'b>(&self, s: &'a str, groups: &mut [&'b str]) -> Option<Cow<'a, str>>
        where 'a: 'b {
        _consume(self, s, groups, false)
    }
}

fn _consume<'a, 'b>(
    r: &Regex, input: &'a str, 
    groups: &mut [&'b str], anchor_at_start: bool
) -> Option<Cow<'a, str>>
    where 'a: 'b {
    let captures = r.captures(input)?;
    let full_capture = captures.get(0)?;
    if anchor_at_start && full_capture.start() != 0 {
        return None
    }
    // Check if expected groups count is leq
    // captures.len includes full group (0), so take captures.len() - 1
    if groups.len() > captures.len() - 1 {
        return None;
    }

    // If less matches than expected - fail.
    for i in 1..=groups.len() {
        // Groups are counted from 1 rather than 0.
        if let Some(capture) = captures.get(i) {
            groups[i-1] = capture.as_str();
        } else {
            // should never happen
            return None
        }
    }
    Some(Cow::Borrowed(&input[full_capture.end()..]))
}