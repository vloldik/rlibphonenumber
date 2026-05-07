mod hash;
mod helper_types;
mod mask_util;
#[cfg(feature = "global_static")]
use std::sync::LazyLock;

pub use hash::{PhoneDigestHasher, PhoneMacHasher, PhoneStdHasher};
pub use helper_types::{Hashed, MaskDigitsConfig, MaxHashedLengthExceededError, Result};
pub use mask_util::{MaskUtil, MaskUtilFallible};

#[cfg(feature = "global_static")]
use crate::{PHONE_NUMBER_UTIL, PhoneNumberUtil};

#[cfg(feature = "global_static")]
pub static MASK_UTIL: LazyLock<MaskUtil<PhoneNumberUtil, &'static PhoneNumberUtil>> =
    LazyLock::new(|| MaskUtil::new_for_util(&PHONE_NUMBER_UTIL));
