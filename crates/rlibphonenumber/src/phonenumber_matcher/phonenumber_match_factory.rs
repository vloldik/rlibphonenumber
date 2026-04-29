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
    pub fn new(phone_util: T) -> Self {
        Self::new_with_formats(phone_util, Some(Arc::new(AlternateFormats::new())))
    }

    pub fn new_with_formats(phone_util: T, formats: Option<Arc<AlternateFormats>>) -> Self {
        Self {
            alternate_formats: formats,
            phone_util,
            regexps: Arc::new(MatcherRegex::new()),
        }
    }
    pub fn create_matcher_fallible<'a>(
        &self,
        text: &'a str,
        leniency: Leniency,
        max_tries: u64,
        preferred_region: Option<Region>,
    ) -> PhoneNUmberMatcherFallible<'a, U, T> {
        PhoneNUmberMatcherFallible::new(
            self.phone_util.clone(),
            self.regexps.clone(),
            text,
            preferred_region,
            leniency,
            max_tries,
            self.alternate_formats.clone(),
        )
    }

    pub fn create_matcher<'a>(
        &self,
        text: &'a str,
        leniency: Leniency,
        max_tries: u64,
        preferred_region: Option<Region>,
    ) -> PhoneNumberMatcher<'a, U, T> {
        PhoneNumberMatcher::new(
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
