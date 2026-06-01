use std::ops::Deref;
use std::sync::Arc;

#[cfg(feature = "global_static")]
use std::sync::LazyLock;

use crate::alternate_formats::AlternateFormats;
use crate::enums::Region;
use crate::interfaces::AsOriginal;
use crate::phonenumber_matcher::leniency::Leniency;
use crate::phonenumber_matcher::matcher_internal::{
    PhoneNumberMatcher, PhoneNumberMatcherFallible,
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
    #[cfg(feature = "builtin_metadata")]
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
    ) -> PhoneNumberMatcherFallible<'a, U, T> {
        PhoneNumberMatcherFallible::new_for_util(
            self.phone_util.clone(),
            self.regexps.clone(),
            text,
            preferred_region,
            leniency,
            max_tries,
            self.alternate_formats.clone(),
        )
    }

    /// Creates a standard (infallible) matcher that auto-detects the region of
    /// national-format numbers instead of assuming a fixed one.
    ///
    /// Numbers written in international format are resolved exactly as with
    /// [`create_matcher`](Self::create_matcher). National-format numbers are
    /// tried against every supported region, preferring the most-recently
    /// matched one to keep ambiguous numbers stable and fast to resolve.
    ///
    /// * `initial_region` – an optional seed for the most-recently used region
    ///   (e.g. the region detected in a previously processed text chunk), or
    ///   `None` to start without a preference.
    ///
    /// *Note: Consider using [`matcher_builder`](Self::matcher_builder) instead for a more ergonomic API.*
    pub fn create_matcher_auto<'a>(
        &self,
        text: &'a str,
        leniency: Leniency,
        max_tries: u64,
        initial_region: Option<Region>,
    ) -> PhoneNumberMatcher<'a, U, T> {
        PhoneNumberMatcher::new_for_util_auto_region(
            self.phone_util.clone(),
            self.regexps.clone(),
            text,
            initial_region,
            leniency,
            max_tries,
            self.alternate_formats.clone(),
        )
    }

    /// Creates a fallible matcher that auto-detects the region of
    /// national-format numbers. See [`create_matcher_auto`](Self::create_matcher_auto)
    /// for the region-resolution behaviour and [`create_matcher_fallible`](Self::create_matcher_fallible)
    /// for the meaning of "fallible".
    pub fn create_matcher_auto_fallible<'a>(
        &self,
        text: &'a str,
        leniency: Leniency,
        max_tries: u64,
        initial_region: Option<Region>,
    ) -> PhoneNumberMatcherFallible<'a, U, T> {
        PhoneNumberMatcherFallible::new_for_util_auto_region(
            self.phone_util.clone(),
            self.regexps.clone(),
            text,
            initial_region,
            leniency,
            max_tries,
            self.alternate_formats.clone(),
        )
    }

    /// Creates a standard (infallible) matcher that auto-detects the region of
    /// national-format numbers, but only probes the provided **subset** of
    /// `regions`.
    ///
    /// This mirrors [`create_matcher_auto`](Self::create_matcher_auto) except
    /// that candidates are tried only against the given list, reducing the
    /// number of parse attempts per candidate and eliminating spurious matches
    /// from unrelated regions.
    ///
    /// The regions are sorted internally, so order does not matter.
    ///
    /// *Note: Consider using [`matcher_builder`](Self::matcher_builder) +
    /// [`MatcherBuilder::regions`] instead for a more ergonomic API.*
    pub fn create_matcher_with_regions<'a>(
        &self,
        text: &'a str,
        leniency: Leniency,
        max_tries: u64,
        regions: impl IntoIterator<Item = Region>,
        initial_region: Option<Region>,
    ) -> PhoneNumberMatcher<'a, U, T> {
        PhoneNumberMatcher::new_for_util_with_regions(
            self.phone_util.clone(),
            self.regexps.clone(),
            text,
            sorted_regions(regions),
            initial_region,
            leniency,
            max_tries,
            self.alternate_formats.clone(),
        )
    }

    /// Fallible variant of [`create_matcher_with_regions`](Self::create_matcher_with_regions).
    pub fn create_matcher_with_regions_fallible<'a>(
        &self,
        text: &'a str,
        leniency: Leniency,
        max_tries: u64,
        regions: impl IntoIterator<Item = Region>,
        initial_region: Option<Region>,
    ) -> PhoneNumberMatcherFallible<'a, U, T> {
        PhoneNumberMatcherFallible::new_for_util_with_regions(
            self.phone_util.clone(),
            self.regexps.clone(),
            text,
            sorted_regions(regions),
            initial_region,
            leniency,
            max_tries,
            self.alternate_formats.clone(),
        )
    }
}

fn sorted_regions(regions: impl IntoIterator<Item = Region>) -> Arc<[Region]> {
    let mut v: Vec<Region> = regions.into_iter().collect();
    v.sort_unstable();
    v.dedup();
    v.into()
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

/// How the matcher should determine the region of national-format numbers.
#[derive(Debug, Clone)]
enum RegionMode {
    /// Assume a single fixed region (`None` = only international `+` numbers).
    Fixed(Option<Region>),
    /// Auto-detect against *all* supported regions (optionally seeded).
    Auto(Option<Region>),
    /// Auto-detect against a *specific subset* of regions (pre-sorted, deduped).
    Subset {
        regions: Arc<[Region]>,
        hint: Option<Region>,
    },
}

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
    region_mode: RegionMode,
}

impl<'a, 'f, U: AsOriginal<PhoneNumberUtilInternal>, T: Deref<Target = U> + Clone>
    MatcherBuilder<'a, 'f, U, T>
{
    /// Constructs a new builder with sensible defaults.
    fn new(factory: &'f PhoneNumberMatcherFactory<U, T>, text: &'a str) -> Self {
        Self {
            factory,
            text,
            leniency: Leniency::Valid,            // Sensible default
            max_tries: u64::MAX,                  // Sensible default
            region_mode: RegionMode::Fixed(None), // Sensible default
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
    ///
    /// Calling this method switches the builder back to fixed-region mode,
    /// overriding any previous call to [`auto_region`](Self::auto_region).
    pub fn preferred_region(mut self, region: impl Into<Option<Region>>) -> Self {
        self.region_mode = RegionMode::Fixed(region.into());
        self
    }

    /// Enables automatic region detection.
    ///
    /// Instead of assuming a single region, national-format numbers are tried
    /// against every supported region until one yields a valid number,
    /// preferring the most-recently matched region. International (`+`) numbers
    /// are resolved from their own country code as usual.
    ///
    /// This is the recommended way to find numbers in text that may contain
    /// numbers from arbitrary regions. Note that it is more expensive than a
    /// fixed region, since a candidate may be tried against many regions.
    ///
    /// Calling this method overrides any previous call to
    /// [`preferred_region`](Self::preferred_region).
    pub fn auto_region(mut self) -> Self {
        self.region_mode = RegionMode::Auto(None);
        self
    }

    /// Like [`auto_region`](Self::auto_region), but seeds the most-recently
    /// used region with `region`.
    ///
    /// This is useful when processing text in independent chunks: pass the
    /// region detected in the previous chunk so that detection stays
    /// consistent across the boundary. Pass `None` for no initial preference.
    pub fn auto_region_with_hint(mut self, region: impl Into<Option<Region>>) -> Self {
        self.region_mode = RegionMode::Auto(region.into());
        self
    }

    /// Restricts automatic region detection to the provided **subset** of
    /// regions.
    ///
    /// This is the recommended option when domain knowledge limits the set of
    /// plausible regions (e.g. a product that only handles a few countries).
    /// Compared to [`auto_region`](Self::auto_region) it:
    ///
    /// * **reduces parse attempts** — candidates are tried only against this
    ///   list, not all ~250 supported regions;
    /// * **eliminates spurious matches** — a number cannot accidentally be
    ///   attributed to a region outside the list;
    /// * **preserves the MRU optimisation** — the most-recently matched region
    ///   in the list is still tried first.
    ///
    /// The regions may be given in any order; they are sorted and deduplicated
    /// internally for deterministic iteration.
    ///
    /// Calling this method overrides any previous call to
    /// [`preferred_region`](Self::preferred_region), [`auto_region`](Self::auto_region), or
    /// a prior [`regions`](Self::regions) call.
    pub fn regions(mut self, regions: impl IntoIterator<Item = Region>) -> Self {
        self.region_mode = RegionMode::Subset {
            regions: sorted_regions(regions),
            hint: None,
        };
        self
    }

    /// Like [`regions`](Self::regions), but additionally seeds the MRU cache
    /// with `hint`.
    ///
    /// Pass the region detected in the previous text chunk so that detection
    /// stays consistent across sliding-window boundaries.
    pub fn regions_with_hint(
        mut self,
        regions: impl IntoIterator<Item = Region>,
        hint: impl Into<Option<Region>>,
    ) -> Self {
        self.region_mode = RegionMode::Subset {
            regions: sorted_regions(regions),
            hint: hint.into(),
        };
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
        match self.region_mode {
            RegionMode::Fixed(region) => {
                self.factory
                    .create_matcher(self.text, self.leniency, self.max_tries, region)
            }
            RegionMode::Auto(hint) => self.factory.create_matcher_auto(
                self.text,
                self.leniency,
                self.max_tries,
                hint,
            ),
            RegionMode::Subset { regions, hint } => self.factory.create_matcher_with_regions(
                self.text,
                self.leniency,
                self.max_tries,
                regions.iter().copied(),
                hint,
            ),
        }
    }

    /// Builds and returns a `PhoneNumberMatcherFallible` iterator.
    ///
    /// This iterator yields `Result` values, providing access to underlying internal
    /// metadata errors instead of panicking.
    ///
    /// # Note on Errors
    /// The errors returned are **strictly internal errors** (e.g., malformed regexes
    /// from custom metadata), *not* text parsing errors. It is generally recommended
    /// to test custom metadata thoroughly using the `rlibphonenumber` CLI and use
    /// the infallible [`build`](Self::build) method instead.
    pub fn build_fallible(self) -> PhoneNumberMatcherFallible<'a, U, T> {
        match self.region_mode {
            RegionMode::Fixed(region) => self.factory.create_matcher_fallible(
                self.text,
                self.leniency,
                self.max_tries,
                region,
            ),
            RegionMode::Auto(hint) => self.factory.create_matcher_auto_fallible(
                self.text,
                self.leniency,
                self.max_tries,
                hint,
            ),
            RegionMode::Subset { regions, hint } => {
                self.factory.create_matcher_with_regions_fallible(
                    self.text,
                    self.leniency,
                    self.max_tries,
                    regions.iter().copied(),
                    hint,
                )
            }
        }
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
pub trait FindNumberExt {
    /// Extracts valid phone numbers from the string using default settings.
    ///
    /// Uses `Leniency::Valid` and no preferred region.
    ///
    /// # Example
    /// ```rust
    /// use crate::rlibphonenumber::phonenumber_matcher::FindNumberExt;
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
        region: impl Into<Option<Region>>,
    ) -> PhoneNumberMatcher<'_, PhoneNumberUtil, &'static PhoneNumberUtil>;

    /// Extracts phone numbers using automatic region detection restricted to the
    /// provided **subset** of `regions`.
    ///
    /// This is the drop-in replacement for code that maintains a manual list of
    /// regions to try: instead of writing your own loop over `phonenumber::parse`
    /// calls you can pass that list here and let the matcher do the rest,
    /// including the MRU optimisation that keeps consecutive same-region numbers
    /// fast.
    ///
    /// # Example
    /// ```rust
    /// use rlibphonenumber::phonenumber_matcher::FindNumberExt;
    /// use rlibphonenumber::enums::Region;
    ///
    /// let text = "US: (415) 555-2671  GB: 020 7946 0958";
    /// for m in text.find_phone_numbers_with_regions([Region::US, Region::GB]) {
    ///     println!("{}", m.number.country_code);
    /// }
    /// ```
    fn find_phone_numbers_with_regions(
        &self,
        regions: impl IntoIterator<Item = Region>,
    ) -> PhoneNumberMatcher<'_, PhoneNumberUtil, &'static PhoneNumberUtil>;

    /// Extracts phone numbers without committing to a single region.
    ///
    /// National-format numbers are auto-detected by trying every supported
    /// region (preferring the most-recently matched one), while international
    /// (`+`) numbers are resolved from their own country code. Use this when
    /// the text may contain numbers from arbitrary regions.
    ///
    /// # Example
    /// ```rust
    /// use crate::rlibphonenumber::phonenumber_matcher::FindNumberExt;
    ///
    /// let text = "GB: 020 7183 8750, FR: 01 70 18 99 00";
    /// for number in text.find_phone_numbers_auto_region() {
    ///     // Each number carries the country code of its detected region.
    /// }
    /// ```
    fn find_phone_numbers_auto_region(
        &self,
    ) -> PhoneNumberMatcher<'_, PhoneNumberUtil, &'static PhoneNumberUtil>;

    /// Returns a `MatcherBuilder` to fully configure the matching process directly
    /// from the string.
    ///
    /// # Example
    /// ```rust
    /// use crate::rlibphonenumber::phonenumber_matcher::FindNumberExt;
    /// use rlibphonenumber::phonenumber_matcher::Leniency;
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
impl FindNumberExt for str {
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
        region: impl Into<Option<Region>>,
    ) -> PhoneNumberMatcher<'_, PhoneNumberUtil, &'static PhoneNumberUtil> {
        self.phone_number_matcher_builder()
            .preferred_region(region)
            .build()
    }

    fn find_phone_numbers_with_regions(
        &self,
        regions: impl IntoIterator<Item = Region>,
    ) -> PhoneNumberMatcher<'_, PhoneNumberUtil, &'static PhoneNumberUtil> {
        self.phone_number_matcher_builder().regions(regions).build()
    }

    fn find_phone_numbers_auto_region(
        &self,
    ) -> PhoneNumberMatcher<'_, PhoneNumberUtil, &'static PhoneNumberUtil> {
        self.phone_number_matcher_builder().auto_region().build()
    }

    fn phone_number_matcher_builder(
        &self,
    ) -> MatcherBuilder<'_, 'static, PhoneNumberUtil, &'static PhoneNumberUtil> {
        PHONE_MATCHER_FACTORY.matcher_builder(self)
    }
}
