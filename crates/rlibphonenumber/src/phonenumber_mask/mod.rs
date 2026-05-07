mod hash;
mod helper_types;
mod phone_mask_util;
#[cfg(feature = "global_static")]
use std::sync::LazyLock;

#[cfg(feature = "digest")]
pub use hash::PhoneDigestHasher;

#[cfg(feature = "digest_mac")]
pub use hash::PhoneMacHasher;

pub use hash::PhoneStdHasher;
pub use helper_types::{Hashed, MaskDigitsConfig, MaxHashedLengthExceededError, Result};
pub use phone_mask_util::{PhoneMaskUtil, PhoneMaskUtilFallible};

#[cfg(feature = "global_static")]
use crate::{PHONE_NUMBER_UTIL, PhoneNumberUtil};

#[cfg(feature = "global_static")]
pub static MASK_UTIL: LazyLock<PhoneMaskUtil<PhoneNumberUtil, &'static PhoneNumberUtil>> =
    LazyLock::new(|| PhoneMaskUtil::new_for_util(&PHONE_NUMBER_UTIL));
