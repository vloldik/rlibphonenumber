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

use strum::EnumIter;

/// Defines the various standardized formats for representing phone numbers.
///
/// `INTERNATIONAL` and `NATIONAL` formats align with the ITU-T E.123 recommendation,
/// but use local conventions like hyphens (-) instead of spaces for separators.
///
/// For example, the Google Switzerland office number would be:
/// - **INTERNATIONAL**: `+41 44 668 1800`
/// - **NATIONAL**: `044 668 1800`
/// - **E164**: `+41446681800` (international format without formatting)
/// - **RFC3966**: `tel:+41-44-668-1800` (hyphen-separated with a "tel:" prefix)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PhoneNumberFormat {
    /// **E.164 format.**
    /// This is a standardized international format with no spaces or symbols,
    /// always starting with a `+` followed by the country code.
    /// Example: `+41446681800`.
    E164,
    /// **International format.**
    /// This format includes the country code and is formatted with spaces
    /// for readability, as recommended for international display.
    /// Example: `+41 44 668 1800`.
    International,
    /// **National format.**
    /// This format is used for dialing within the number's own country.
    /// It may include a national prefix (like '0') and uses local formatting conventions.
    /// Example: `044 668 1800`.
    National,
    /// **RFC3966 format.**
    /// A technical format used in contexts like web links. It starts with "tel:",
    /// uses hyphens as separators, and can include extensions.
    /// Example: `tel:+41-44-668-1800`.
    RFC3966,
}

/// Categorizes phone numbers based on their primary use.
#[derive(Debug, EnumIter, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PhoneNumberType {
    /// **Fixed-line numbers.**
    /// These are traditional landline telephone numbers tied to a specific geographic location.
    FixedLine,
    /// **Mobile numbers.**
    /// These numbers are assigned to wireless devices like mobile phones.
    Mobile,
    /// **Fixed-line or mobile.**
    /// Used in regions (e.g., the USA) where it's impossible to distinguish between
    /// fixed-line and mobile numbers by looking at the phone number itself.
    FixedLineOrMobile,
    /// **Toll-free numbers.**
    /// Calls to these numbers are free for the caller, with the cost being paid by the recipient.
    /// Examples include "800" or "888" numbers in the US.
    TollFree,
    /// **Premium-rate numbers.**
    /// These numbers charge a higher rate than normal calls, often used for services
    /// like horoscopes, adult chat lines, or tech support.
    PremiumRate,
    /// **Shared-cost numbers.**
    /// The cost of the call is split between the caller and the recipient. These calls
    /// are typically cheaper than premium-rate numbers.
    SharedCost,
    /// **Voice over IP (VoIP) numbers.**
    /// These numbers are used for services that transmit voice calls over the internet.
    VoIP,
    /// **Personal numbers.**
    /// A number associated with a person, not a location or device. It can be routed
    /// to different destinations (mobile or fixed-line) as configured by the user.
    PersonalNumber,
    /// **Pagers.**
    /// Numbers used for sending messages to paging devices.
    Pager,
    /// **Universal Access Numbers (UAN).**
    /// A single number that a company can use to route calls to different offices or departments.
    UAN,
    /// **Voicemail access numbers.**
    /// Numbers used to directly access a voicemail service.
    VoiceMail,
    /// **Unknown type.**
    /// The number does not match any of the known patterns for its region and its type
    /// cannot be determined.
    Unknown,
}

/// Describes the degree of similarity between two phone numbers.
#[derive(Debug, Clone, Copy, PartialEq, Hash)]
pub enum MatchType {
    /// **No match.**
    /// The two numbers are entirely different.
    NoMatch,
    /// **Short National Significant Number match.**
    /// One number is a shorter version of the other's National Significant Number (NSN).
    /// For example, `6502530000` is a short match for `16502530000`.
    ShortNsnMatch,
    /// **National Significant Number (NSN) match.**
    /// The numbers share the same NSN but may have different country codes or formatting.
    /// For example, `0446681800` (national) and `+41446681800` (international) are an NSN match.
    NsnMatch,
    /// **Exact match.**
    /// The two numbers are identical in every aspect, including country code, NSN, and
    /// any specified extensions.
    ExactMatch,
}


// Separated enum ValidationResult into ValidationResult err and
// ValidationResultOk for using Result<Ok, Err>

/// Represents the possible outcomes when checking if a phone number's length is valid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NumberLengthType {
    /// **The length is valid for a dialable number.**
    /// The number's length matches the expected length for a complete, dialable
    /// number in its region.
    IsPossible,
    /// **The length is valid for a local-only number.**
    /// The number's length is too short for a full national number but matches a pattern
    /// for a number that can be dialed within a specific local area (e.g., without the area code).
    IsPossibleLocalOnly,
}