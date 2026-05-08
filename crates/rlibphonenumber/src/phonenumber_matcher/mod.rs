//! Phone number matching and extraction utilities.
//!
//! This module provides tools to scan plain text and extract valid phone numbers.
//! It handles different leniency levels, regional formatting, and text normalization.
//!
//! # Fallibility and Custom Metadata
//!
//! When configuring a matcher, you can choose between an **infallible** matcher
//! ([`PhoneNumberMatcher`]) and a **fallible** matcher ([`PhoneNUmberMatcherFallible`]).
//!
//! It is crucial to understand that the errors returned by the fallible matcher are
//! **strictly internal errors** caused by broken or malformed metadata (e.g., invalid
//! regular expressions or logical flaws in the metadata tree). They are *not* parsing
//! errors for the user's text (invalid phone numbers in the text are simply skipped
//! by both matchers).
//!
//! - **Infallible (`PhoneNumberMatcher`)**: This is the default and recommended matcher.
//!   If it encounters corrupted metadata, it will **panic**. This behavior is perfectly
//!   safe and expected when using the default, built-in metadata.
//! - **Fallible (`PhoneNUmberMatcherFallible`)**: Yields `Result` items, allowing you to
//!   catch internal metadata errors. You should only use this if you are dynamically
//!   loading arbitrary, untested custom metadata at runtime and need to prevent panics.
//!
//! **Best Practice Recommendation**:
//! Instead of handling metadata errors at runtime via the fallible matcher, it is highly
//! recommended to pre-test and validate any custom metadata using the `rlibphonenumber` CLI
//! tool before compiling or deploying. Once validated, you can safely use the infallible
//! matcher for better ergonomics and performance.

mod leniency;
mod matcher_internal;
mod matcher_regex;
mod phonenumber_match;
mod phonenumber_match_factory;

pub use {
    leniency::Leniency,
    matcher_internal::{PhoneNumberMatcher, PhoneNumberMatcherFallible},
    phonenumber_match::PhoneNumberMatch,
    phonenumber_match_factory::{MatcherBuilder, PhoneNumberMatcherFactory},
};

#[cfg(feature = "global_static")]
pub use phonenumber_match_factory::{FindNumberExt, PHONE_MATCHER_FACTORY};

#[cfg(test)]
pub use matcher_internal::PhoneNumberMatcherInternal;
