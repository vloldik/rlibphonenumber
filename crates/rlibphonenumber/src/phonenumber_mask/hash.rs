use std::hash::{Hash, Hasher as StdHasher};

#[cfg(feature = "digest")]
use digest::{Digest as TraitDigest, Update as TraitUpdate};

#[cfg(feature = "digest_mac")]
use digest::Mac as TraitMac;

use crate::phonenumber_mask::helper_types;
use crate::{PhoneNumber as LocalPhoneNumber, interfaces::PhoneHasher as TraitPhoneHasher};
use crate::{interfaces::OptionalHasher, phonenumber_mask::Hashed as LocalHashed};

/// A wrapper for standard, non-cryptographic Rust Hashers (e.g., `DefaultHasher`, `AHash`).
///
/// Output size is always fixed to 8 bytes (`u64`).
#[repr(transparent)]
pub struct PhoneStdHasher<T: StdHasher>(pub T);

/// A wrapper for cryptographic algorithms from the `RustCrypto` ecosystem (e.g., `Sha256`).
#[cfg(feature = "digest")]
#[repr(transparent)]
pub struct PhoneDigestHasher<T: TraitDigest + TraitUpdate>(pub T);

/// A wrapper for cryptographic Message Authentication Codes (e.g., `Hmac<Sha256>`).
#[cfg(feature = "digest_mac")]
#[repr(transparent)]
pub struct PhoneMacHasher<T: TraitMac + TraitUpdate>(pub T);

impl<T: StdHasher> TraitPhoneHasher for PhoneStdHasher<T> {
    fn hash_phone(mut self, phone: &LocalPhoneNumber) -> helper_types::Result<LocalHashed> {
        phone.hash(&mut self.0);
        LocalHashed::from_slice(&self.0.finish().to_be_bytes())
    }
}

#[cfg(feature = "digest")]
impl<T: TraitDigest + TraitUpdate> PhoneDigestHasher<T> {
    /// Initializes a cryptographic digest and immediately applies a salt.
    pub fn new_with_salt(mut digest: T, salt: &[u8]) -> Self {
        TraitUpdate::update(&mut digest, salt);
        Self(digest)
    }
}

/// Feeds standardized, canonical fields of a phone number into the underlying digest/MAC stream.
///
/// This guarantees that visually different string representations of the same logical
/// number yield the same cryptographic hash.
#[cfg(feature = "digest")]
fn feed_phone_bytes(updater: &mut impl TraitUpdate, phone: &LocalPhoneNumber) {
    updater.update(&phone.country_code.to_be_bytes());
    updater.update(&phone.national_number.to_be_bytes());
    updater.update(&phone.extension().len().to_be_bytes());
    updater.update(phone.extension().as_bytes());
    updater.update(&[phone.italian_leading_zero() as u8]);
    updater.update(&phone.number_of_leading_zeros().to_be_bytes());
    updater.update(&phone.raw_input().len().to_be_bytes());
    updater.update(phone.raw_input().as_bytes());
    updater.update(&(phone.country_code_source() as i32).to_be_bytes());
    updater.update(phone.preferred_domestic_carrier_code().as_bytes());
}

#[cfg(feature = "digest")]
impl<D: TraitDigest + TraitUpdate> TraitPhoneHasher for PhoneDigestHasher<D> {
    fn hash_phone(mut self, phone: &LocalPhoneNumber) -> helper_types::Result<LocalHashed> {
        feed_phone_bytes(&mut self.0, phone);
        let out = self.0.finalize();

        LocalHashed::from_slice(out)
    }
}

#[cfg(feature = "digest_mac")]
impl<M: TraitMac + TraitUpdate> TraitPhoneHasher for PhoneMacHasher<M> {
    fn hash_phone(mut self, phone: &LocalPhoneNumber) -> helper_types::Result<LocalHashed> {
        feed_phone_bytes(&mut self.0, phone);
        let out = self.0.finalize();

        LocalHashed::from_slice(&out.as_bytes())
    }
}

impl OptionalHasher for () {
    fn hash_phone(self, _: &LocalPhoneNumber) -> helper_types::Result<Option<LocalHashed>> {
        Ok(None)
    }
}
