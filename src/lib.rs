mod shortnumberinfo;
mod interfaces;
/// This module is automatically generated from /resources/*.proto
mod proto_gen;
mod phonenumberutil;
mod regexp_cache;
mod regex_based_matcher;
pub mod i18n;
pub(crate) mod regex_util;
pub(crate) mod string_util;

/// I decided to create this module because there are many 
/// boilerplate places in the code that can be replaced with macros, 
/// the name of which will describe what is happening more 
/// clearly than a few lines of code.
mod macros;