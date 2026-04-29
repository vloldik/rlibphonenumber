mod leniency;
mod matcher_internal;
mod matcher_regex;
mod phonenumber_match;
mod phonenumber_match_factory;

pub use {
    leniency::Leniency,
    matcher_internal::{PhoneNUmberMatcherFallible, PhoneNumberMatcher},
    phonenumber_match::PhoneNumberMatch,
    phonenumber_match_factory::PhoneNumberMatcherFactory,
};

#[cfg(test)]
pub use matcher_internal::PhoneNumberMatcherInternal;
