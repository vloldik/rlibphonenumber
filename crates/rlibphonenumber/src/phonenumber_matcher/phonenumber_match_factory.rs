use std::ops::Deref;
use std::sync::Arc;

#[cfg(feature = "global_static")]
use std::sync::LazyLock;

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

// =============================================================================
// PhoneNumberMatcherFactory
// =============================================================================

/// A factory for creating phone number matchers.
///
/// This struct holds the underlying utility and regex configurations
/// needed to efficiently parse text and extract phone numbers. By caching
/// the compiled regular expressions and alternate formats, it significantly
/// improves performance when creating matchers for multiple strings.
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
    ///
    /// This constructor initializes the factory with default alternate formats.
    pub fn new_for_util(phone_util: T) -> Self {
        Self::new_for_util_with_formats(phone_util, Some(Arc::new(AlternateFormats::new())))
    }

    /// Creates a new factory with a custom set of alternate formats.
    ///
    /// If `formats` is `None`, alternate format matching will be disabled.
    pub fn new_for_util_with_formats(
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
    ///
    /// This is the recommended way to configure and instantiate a matcher,
    /// allowing you to specify leniency, maximum tries, and preferred region
    /// in a clear and readable manner.
    pub fn matcher_builder<'a, 'f>(&'f self, text: &'a str) -> MatcherBuilder<'a, 'f, U, T> {
        MatcherBuilder::new(self, text)
    }

    /// Creates a standard (infallible) matcher directly.
    ///
    /// The returned iterator yields valid phone numbers found in the text.
    /// Any underlying parsing errors will cause the segment to be skipped silently.
    ///
    /// *Note: Consider using[`matcher_builder`](Self::matcher_builder) instead for a more ergonomic API.*
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

    /// Creates a fallible matcher directly.
    ///
    /// The returned iterator yields `Result` values, allowing the caller to handle
    /// and inspect parsing errors instead of silently skipping them.
    ///
    /// *Note: Consider using [`matcher_builder`](Self::matcher_builder) instead for a more ergonomic API.*
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
}

#[cfg(feature = "global_static")]
impl PhoneNumberMatcherFactory<PhoneNumberUtil, &'static PhoneNumberUtil> {
    /// Creates a new factory using the globally static `PhoneNumberUtil`.
    pub fn new() -> Self {
        Self::new_for_util(&PHONE_NUMBER_UTIL)
    }
}

#[cfg(feature = "global_static")]
impl Default for PhoneNumberMatcherFactory<PhoneNumberUtil, &'static PhoneNumberUtil> {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// MatcherBuilder
// =============================================================================

/// A fluent builder for configuring phone number matchers.
///
/// This builder allows you to chain configuration methods to set up the region,
/// strictness (leniency), and iteration limits before building the final matcher
/// iterator.
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
    /// Constructs a new builder with sensible defaults.
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
    ///
    /// Leniency determines how strictly the text must comply to be considered
    /// a valid phone number. The default is `Leniency::Valid`.
    pub fn leniency(mut self, leniency: Leniency) -> Self {
        self.leniency = leniency;
        self
    }

    /// Sets the maximum number of iterations/tries.
    ///
    /// This is particularly useful for preventing excessive processing time
    /// on massive or highly complex strings. The default is `u64::MAX`.
    pub fn max_tries(mut self, max_tries: u64) -> Self {
        self.max_tries = max_tries;
        self
    }

    /// Sets the preferred region for parsing.
    ///
    /// The preferred region is used as a fallback for phone numbers that are
    /// found without explicit country codes (e.g., numbers without a `+` prefix).
    pub fn preferred_region(mut self, region: Region) -> Self {
        self.preferred_region = Some(region);
        self
    }

    /// Builds and returns the standard, infallible `PhoneNumberMatcher` iterator.
    ///
    /// Invalid phone numbers found in the text will be silently skipped.
    ///
    /// # Panics
    /// This iterator will **panic** if it encounters internal metadata errors
    /// (e.g., invalid regular expressions). If you are using the default built-in
    /// metadata, this will never happen. If you are using custom metadata, ensure
    /// it is pre-validated (e.g., via the `rlibphonenumber` CLI).
    pub fn build(self) -> PhoneNumberMatcher<'a, U, T> {
        self.factory.create_matcher(
            self.text,
            self.leniency,
            self.max_tries,
            self.preferred_region,
        )
    }

    /// Builds and returns a `PhoneNUmberMatcherFallible` iterator.
    ///
    /// This iterator yields `Result` values, providing access to underlying internal
    /// metadata errors instead of panicking.
    ///
    /// # Note on Errors
    /// The errors returned are **strictly internal errors** (e.g., malformed regexes
    /// from custom metadata), *not* text parsing errors. It is generally recommended
    /// to test custom metadata thoroughly using the `rlibphonenumber` CLI and use
    /// the infallible [`build`](Self::build) method instead.
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
/// A globally available instance of `PhoneNumberMatcherFactory`.
///
/// This avoids the overhead of recompiling regexes and reallocating alternate
/// formats each time a matcher is needed.
pub static PHONE_MATCHER_FACTORY: LazyLock<
    PhoneNumberMatcherFactory<PhoneNumberUtil, &'static PhoneNumberUtil>,
> = LazyLock::new(PhoneNumberMatcherFactory::new);

// =============================================================================
// Extension Traits
// =============================================================================

#[cfg(feature = "global_static")]
/// Extension trait adding immediate phone number matching capabilities to strings.
///
/// This trait is available when the `global_static` feature is enabled. It provides
/// convenient methods directly on string types (`&str`, `String`) to find phone numbers
/// without having to manually instantiate factories or utilities.
pub trait PhoneNumberExt {
    /// Extracts valid phone numbers from the string using default settings.
    ///
    /// Uses `Leniency::Valid` and no preferred region.
    ///
    /// # Example
    /// ```rust
    /// use rlibphonenumber::PhoneNumberExt;
    ///
    /// let text = "Call +1 555-0199 for more details.";
    /// for match_result in text.find_phone_numbers() {
    ///     // Process matches
    /// }
    /// ```
    fn find_phone_numbers(
        &self,
    ) -> PhoneNumberMatcher<'_, PhoneNumberUtil, &'static PhoneNumberUtil>;

    /// Extracts phone numbers from the string using a specific leniency level.
    fn find_phone_numbers_with_leniency(
        &self,
        leniency: Leniency,
    ) -> PhoneNumberMatcher<'_, PhoneNumberUtil, &'static PhoneNumberUtil>;

    /// Extracts phone numbers using a preferred region for numbers without country codes.
    fn find_phone_numbers_with_preferred_region(
        &self,
        region: Region,
    ) -> PhoneNumberMatcher<'_, PhoneNumberUtil, &'static PhoneNumberUtil>;

    /// Returns a `MatcherBuilder` to fully configure the matching process directly
    /// from the string.
    ///
    /// # Example
    /// ```rust
    /// use rlibphonenumber::PhoneNumberExt;
    /// use rlibphonenumber::phonenumber_matcher::leniency::Leniency;
    /// use rlibphonenumber::enums::Region;
    ///
    /// let text = "Contact us at 020 7183 8750";
    /// let matcher = text.phone_number_matcher_builder()
    ///     .leniency(Leniency::Possible)
    ///     .preferred_region(Region::GB)
    ///     .max_tries(10)
    ///     .build();
    /// ```
    fn phone_number_matcher_builder(
        &self,
    ) -> MatcherBuilder<'_, 'static, PhoneNumberUtil, &'static PhoneNumberUtil>;
}

#[cfg(feature = "global_static")]
impl PhoneNumberExt for str {
    fn find_phone_numbers(
        &self,
    ) -> PhoneNumberMatcher<'_, PhoneNumberUtil, &'static PhoneNumberUtil> {
        self.phone_number_matcher_builder().build()
    }

    fn find_phone_numbers_with_leniency(
        &self,
        leniency: Leniency,
    ) -> PhoneNumberMatcher<'_, PhoneNumberUtil, &'static PhoneNumberUtil> {
        self.phone_number_matcher_builder()
            .leniency(leniency)
            .build()
    }

    fn find_phone_numbers_with_preferred_region(
        &self,
        region: Region,
    ) -> PhoneNumberMatcher<'_, PhoneNumberUtil, &'static PhoneNumberUtil> {
        self.phone_number_matcher_builder()
            .preferred_region(region)
            .build()
    }

    fn phone_number_matcher_builder(
        &self,
    ) -> MatcherBuilder<'_, 'static, PhoneNumberUtil, &'static PhoneNumberUtil> {
        PHONE_MATCHER_FACTORY.matcher_builder(self)
    }
}
