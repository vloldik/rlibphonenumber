mod helper_constants;
pub mod helper_functions;
mod errors;
mod enums;
mod phonenumberutil;
mod phone_number_regexps_and_mappings;
pub(self) mod helper_types;

use std::sync::LazyLock;

pub use enums::{MatchType, PhoneNumberFormat, PhoneNumberType, ValidationResultErr, ValidNumberLenType};
use thiserror::Error;

// use crate::phonenumberutil::phonenumberutil::PhoneNumberUtil;

// static PHONE_NUMBER_UTIL: LazyLock<PhoneNumberUtil> = LazyLock::new(|| {
//     PhoneNumberUtil::new()
// });