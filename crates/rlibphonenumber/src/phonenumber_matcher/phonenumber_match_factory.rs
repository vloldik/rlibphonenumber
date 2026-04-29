use std::ops::Deref;
use std::sync::Arc;

use crate::alternate_formats::AlternateFormats;
use crate::enums::Region;
use crate::interfaces::AsOriginal;
use crate::phonenumber_matcher::leniency::Leniency;
use crate::phonenumber_matcher::matcher_internal::{
    PhoneNUmberMatcherFallible, PhoneNumberMatcher,
};
use crate::phonenumber_matcher::matcher_regex::MatcherRegex;
use crate::phonenumberutil::phonenumberutil_internal::PhoneNumberUtilInternal;

#[cfg(feature = "global_static")]
use crate::{PHONE_NUMBER_UTIL, PhoneNumberUtil};

/// A factory for creating phone number matchers.
///
/// This struct holds the underlying utility and regex configurations
/// needed to efficiently parse text and extract phone numbers.
#[derive(Clone)]
pub struct PhoneNumberMatcherFactory<
    U: AsOriginal<PhoneNumberUtilInternal>,
    T: Deref<Target = U> + Clone,
> {
    regexps: Arc<MatcherRegex>,
    alternate_formats: Option<Arc<AlternateFormats>>,
    phone_util: T,
}

impl<U: AsOriginal<PhoneNumberUtilInternal>, T: Deref<Target = U> + Clone>
    PhoneNumberMatcherFactory<U, T>
{
    /// Creates a new factory using the provided phone utility instance.
    pub fn new_for_util(phone_util: T) -> Self {
        Self::new_with_for_util_formats(phone_util, Some(Arc::new(AlternateFormats::new())))
    }

    /// Creates a new factory with a custom set of alternate formats.
    pub fn new_with_for_util_formats(
        phone_util: T,
        formats: Option<Arc<AlternateFormats>>,
    ) -> Self {
        Self {
            alternate_formats: formats,
            phone_util,
            regexps: Arc::new(MatcherRegex::new()),
        }
    }

    /// Starts building a matcher configuration via a fluent builder pattern.
    /// This is the recommended way to configure and instantiate a matcher.
    pub fn matcher_builder<'a, 'f>(&'f self, text: &'a str) -> MatcherBuilder<'a, 'f, U, T> {
        MatcherBuilder::new(self, text)
    }

    /// Creates a fallible matcher directly.
    /// (Consider using `matcher_builder` instead for a more ergonomic API).
    pub fn create_matcher_fallible<'a>(
        &self,
        text: &'a str,
        leniency: Leniency,
        max_tries: u64,
        preferred_region: Option<Region>,
    ) -> PhoneNUmberMatcherFallible<'a, U, T> {
        PhoneNUmberMatcherFallible::new_for_util(
            self.phone_util.clone(),
            self.regexps.clone(),
            text,
            preferred_region,
            leniency,
            max_tries,
            self.alternate_formats.clone(),
        )
    }

    /// Creates an standard (infallible) matcher directly.
    /// (Consider using `matcher_builder` instead for a more ergonomic API).
    pub fn create_matcher<'a>(
        &self,
        text: &'a str,
        leniency: Leniency,
        max_tries: u64,
        preferred_region: Option<Region>,
    ) -> PhoneNumberMatcher<'a, U, T> {
        PhoneNumberMatcher::new_for_util(
            self.phone_util.clone(),
            self.regexps.clone(),
            text,
            preferred_region,
            leniency,
            max_tries,
            self.alternate_formats.clone(),
        )
    }
}

#[cfg(feature = "global_static")]
impl PhoneNumberMatcherFactory<PhoneNumberUtil, &'static PhoneNumberUtil> {
    /// Creates a new factory using the globally static `PhoneNumberUtil`.
    pub fn new() -> Self {
        Self::new_for_util(&PHONE_NUMBER_UTIL)
    }
}

/// A fluent builder for configuring phone number matchers.
///
/// Allows chaining configuration methods to set up region, strictness,
/// and limitations before building the final matcher iterator.
pub struct MatcherBuilder<
    'a,
    'f,
    U: AsOriginal<PhoneNumberUtilInternal>,
    T: Deref<Target = U> + Clone,
> {
    factory: &'f PhoneNumberMatcherFactory<U, T>,
    text: &'a str,
    leniency: Leniency,
    max_tries: u64,
    preferred_region: Option<Region>,
}

impl<'a, 'f, U: AsOriginal<PhoneNumberUtilInternal>, T: Deref<Target = U> + Clone>
    MatcherBuilder<'a, 'f, U, T>
{
    fn new(factory: &'f PhoneNumberMatcherFactory<U, T>, text: &'a str) -> Self {
        Self {
            factory,
            text,
            leniency: Leniency::Valid, // Sensible default
            max_tries: u64::MAX,       // Sensible default
            preferred_region: None,
        }
    }

    /// Sets the leniency level for the matcher.
    /// Determines how strictly the text must comply to be considered a phone number.
    pub fn leniency(mut self, leniency: Leniency) -> Self {
        self.leniency = leniency;
        self
    }

    /// Sets the maximum number of iterations/tries.
    /// Useful for preventing excessive processing time on massive or complex strings.
    pub fn max_tries(mut self, max_tries: u64) -> Self {
        self.max_tries = max_tries;
        self
    }

    /// Sets the preferred region for parsing phone numbers without explicit country codes.
    pub fn preferred_region(mut self, region: Region) -> Self {
        self.preferred_region = Some(region);
        self
    }

    /// Builds and returns the standard, infallible `PhoneNumberMatcher` iterator.
    /// Any underlying parsing errors will cause the segment to be skipped silently.
    pub fn build(self) -> PhoneNumberMatcher<'a, U, T> {
        self.factory.create_matcher(
            self.text,
            self.leniency,
            self.max_tries,
            self.preferred_region,
        )
    }

    /// Builds and returns the `PhoneNUmberMatcherFallible` iterator.
    /// This iterator yields `Result` values, allowing caller to handle parsing errors.
    pub fn build_fallible(self) -> PhoneNUmberMatcherFallible<'a, U, T> {
        self.factory.create_matcher_fallible(
            self.text,
            self.leniency,
            self.max_tries,
            self.preferred_region,
        )
    }
}

#[cfg(feature = "global_static")]
/// Extension trait adding immediate phone number matching capabilities to strings.
pub trait PhoneNumberExt {
    /// Extracts valid phone numbers from the string using default settings
    /// (Globally static utility, `Leniency::Valid`).
    ///
    /// # Example
    /// ```rust
    /// use crate::PhoneNumberExt;
    /// let numbers = "Call +1 555-0199".find_phone_numbers();
    /// ```
    fn find_phone_numbers(
        &self,
    ) -> PhoneNumberMatcher<'_, PhoneNumberUtil, &'static PhoneNumberUtil>;

    /// Extracts phone numbers fallibly, allowing the user to handle internal errors
    /// instead of silently skipping them.
    fn find_phone_numbers_fallible(
        &self,
    ) -> PhoneNUmberMatcherFallible<'_, PhoneNumberUtil, &'static PhoneNumberUtil>;
}

#[cfg(feature = "global_static")]
impl PhoneNumberExt for str {
    fn find_phone_numbers(
        &self,
    ) -> PhoneNumberMatcher<'_, PhoneNumberUtil, &'static PhoneNumberUtil> {
        PhoneNumberMatcherFactory::new()
            .matcher_builder(self)
            .build()
    }

    fn find_phone_numbers_fallible(
        &self,
    ) -> PhoneNUmberMatcherFallible<'_, PhoneNumberUtil, &'static PhoneNumberUtil> {
        PhoneNumberMatcherFactory::new()
            .matcher_builder(self)
            .build_fallible()
    }
}
