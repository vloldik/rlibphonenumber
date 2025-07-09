use strum::EnumIter;
use thiserror::Error;

/// INTERNATIONAL and NATIONAL formats are consistent with the definition
/// in ITU-T Recommendation E.123. However we follow local conventions such as
/// using '-' instead of whitespace as separators. For example, the number of
/// the Google Switzerland office will be written as "+41 44 668 1800" in
/// INTERNATIONAL format, and as "044 668 1800" in NATIONAL format. E164
/// format is as per INTERNATIONAL format but with no formatting applied e.g.
/// "+41446681800". RFC3966 is as per INTERNATIONAL format, but with all spaces
/// and other separating symbols replaced with a hyphen, and with any phone
/// number extension appended with ";ext=". It also will have a prefix of
/// "tel:" added, e.g. "tel:+41-44-668-1800".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PhoneNumberFormat {
    E164,
    International,
    National,
    RFC3966,
}

/// Type of phone numbers.
#[derive(Debug, EnumIter, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PhoneNumberType {
    FixedLine,
    Mobile,
    /// In some regions (e.g. the USA), it is impossible to distinguish between
    /// fixed-line and mobile numbers by looking at the phone number itself.
    FixedLineOrMobile,
    /// Freephone lines
    TollFree,
    PremiumRate,
    /// The cost of this call is shared between the caller and the recipient, and
    /// is hence typically less than PREMIUM_RATE calls. See
    /// http://en.wikipedia.org/wiki/Shared_Cost_Service for more information.
    SharedCost,
    /// Voice over IP numbers. This includes TSoIP (Telephony Service over IP).
    VoIP,
    /// A personal number is associated with a particular person, and may be
    /// routed to either a MOBILE or FIXED_LINE number. Some more information can
    /// be found here: http://en.wikipedia.org/wiki/Personal_Numbers
    PersonalNumber,
    Pager,
    /// Used for "Universal Access Numbers" or "Company Numbers". They may be
    /// further routed to specific offices, but allow one number to be used for a
    /// company.
    UAN,
    /// Used for "Voice Mail Access Numbers".
    VoiceMail,
    /// A phone number is of type UNKNOWN when it does not fit any of the known
    /// patterns for a specific region.
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Hash)]
pub enum MatchType {
    NoMatch,
    ShortNsnMatch,
    NsnMatch,
    ExactMatch,
}


// Separated enum ValidationResult into ValidationResult err and 
// ValidationResultOk for using Result<Ok, Err>

/// Possible outcomes when testing if a PhoneNumber is possible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Error)]
pub enum ValidationResultErr {
    /// The number has an invalid country calling code.
    #[error("The number has an invalid country calling code")]
    InvalidCountryCode,
    /// The number is shorter than all valid numbers for this region.
    #[error("The number is shorter than all valid numbers for this region")]
    TooShort,
    /// The number is longer than the shortest valid numbers for this region,
    /// shorter than the longest valid numbers for this region, and does not
    /// itself have a number length that matches valid numbers for this region.
    /// This can also be returned in the case where
    /// IsPossibleNumberForTypeWithReason was called, and there are no numbers of
    /// this type at all for this region.
    #[error("\
    The number is longer than the shortest valid numbers for this region,\
    shorter than the longest valid numbers for this region, and does not\
    itself have a number length that matches valid numbers for this region\
    ")]
    InvalidLength,
    /// The number is longer than all valid numbers for this region.
    #[error("The number is longer than all valid numbers for this region")]
    TooLong,
}

/// Possible outcomes when testing if a PhoneNumber is possible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValidNumberLenType {
    /// The number length matches that of valid numbers for this region.
    IsPossible,
    /// The number length matches that of local numbers for this region only
    /// (i.e. numbers that may be able to be dialled within an area, but do not
    /// have all the information to be dialled from anywhere inside or outside
    /// the country).
    IsPossibleLocalOnly,
}
