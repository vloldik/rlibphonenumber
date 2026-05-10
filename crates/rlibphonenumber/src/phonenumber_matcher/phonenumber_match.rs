/*
 * Copyright (C) 2011 The Libphonenumber Authors
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 * http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use crate::PhoneNumber;

/**
* The immutable match of a phone number within a piece of text. Matches may be found using
* {@link PhoneNumberUtil#findNumbers}.
*
* <p>A match consists of the {@linkplain #number() phone number} as well as the
* {@linkplain #start() start} and {@linkplain #end() end} offsets of the corresponding subsequence
* of the searched text. Use {@link #rawString()} to obtain a copy of the matched subsequence.
*
* <p>The following annotated example clarifies the relationship between the searched text, the
* match offsets, and the parsed number:

* <pre>
* CharSequence text = "Call me at +1 425 882-8080 for details.";
* String country = "US";
* PhoneNumberUtil util = PhoneNumberUtil.getInstance();
*
* // Find the first phone number match:
* PhoneNumberMatch m = util.findNumbers(text, country).iterator().next();
*
* // rawString() contains the phone number as it appears in the text.
* "+1 425 882-8080".equals(m.rawString());
*
* // start() and end() define the range of the matched subsequence.
* CharSequence subsequence = text.subSequence(m.start(), m.end());
* "+1 425 882-8080".contentEquals(subsequence);
*
* // number() returns the the same result as PhoneNumberUtil.{@link PhoneNumberUtil#parse parse()}
* // invoked on rawString().
* util.parse(m.rawString(), country).equals(m.number());
* </pre>
*/
#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct PhoneNumberMatch<'a> {
    /** The start index into the text. */
    pub start: usize,
    /** The raw substring matched. */
    pub raw_string: &'a str,
    /** The matched phone number. */
    pub number: PhoneNumber,
}

impl<'a> PhoneNumberMatch<'a> {
    pub fn new(start: usize, string: &'a str, number: PhoneNumber) -> Self {
        Self {
            start,
            raw_string: string,
            number,
        }
    }

    pub fn end(&self) -> usize {
        self.start + self.raw_string.len()
    }
}
