mod helper_constants;
mod helper_functions;
pub mod errors;
pub mod enums;
pub mod phonenumberutil;
mod phone_number_regexps_and_mappings;
pub(self) mod helper_types;
pub(self) mod comparisons;

use std::sync::LazyLock;

pub use enums::{MatchType, PhoneNumberFormat, PhoneNumberType, NumberLengthType};
use crate::phonenumberutil::phonenumberutil::PhoneNumberUtil;

pub static PHONE_NUMBER_UTIL: LazyLock<PhoneNumberUtil> = LazyLock::new(|| {
    PhoneNumberUtil::new()
});