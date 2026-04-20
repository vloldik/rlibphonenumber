/**
Leniency when {@linkplain PhoneNumberUtil#findNumbers finding} potential phone numbers in text
segments. The levels here are ordered in increasing strictness.
*/
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Leniency {
    /// Phone numbers accepted are possible, but not necessarily valid.
    Possible = 0,
    /**
    Phone numbers accepted are possible
    and valid. Numbers written
    in national format must have their national-prefix present if it is usually written for a
    number of this type.
    */
    Valid = 1,
    /**
    Phone numbers accepted are valid and
    are grouped in a possible way for this locale. For example, a US number written as
    "65 02 53 00 00" and "650253 0000" are not accepted at this leniency level, whereas
    "650 253 0000", "650 2530000" or "6502530000" are.
    Numbers with more than one '/' symbol in the national significant number are also dropped at
    this level.
    <p>
    <b>Warning</b>: This level might result in lower coverage especially for regions outside of country
    code "+1". If you are not sure about which level to use, email the discussion group
    libphonenumber-discuss@googlegroups.com.
    */
    StrictGrouping = 2,
    /**
    Phone numbers accepted are valid and
    are grouped in the same way that we would have formatted it, or as a single block. For
    example, a US number written as "650 2530000" is not accepted at this leniency level, whereas
    "650 253 0000" or "6502530000" are.
    Numbers with more than one '/' symbol are also dropped at this level.
    <p>
    <b>Warning</b>: This level might result in lower coverage especially for regions outside of country
    code "+1". If you are not sure about which level to use, email the discussion group
    libphonenumber-discuss@googlegroups.com.
    */
    ExactGrouping = 3,
}
