use std::hash::{Hash, Hasher};

#[cfg(feature = "digest")]
use digest::{Digest, OutputSizeUser, Update};

#[cfg(feature = "digest_mac")]
use digest::Mac;

use crate::phonenumber_mask::Hashed;
use crate::{PhoneNumber, interfaces::PhoneHasher};

#[repr(transparent)]
pub struct PhoneStdHasher<T: Hasher>(pub T);

#[cfg(feature = "digest")]
#[repr(transparent)]
pub struct PhoneDigestHasher<T: Digest + Update>(pub T);

#[cfg(feature = "digest_mac")]
#[repr(transparent)]
pub struct PhoneMacHasher<T: Mac + Update>(pub T);

impl<T: Hasher> PhoneHasher for PhoneStdHasher<T> {
    fn hash_phone(mut self, phone: &PhoneNumber) -> Option<Hashed> {
        phone.hash(&mut self.0);
        Hashed::from_slice(&self.0.finish().to_be_bytes())
    }
}

#[cfg(feature = "digest")]
impl<T: Digest + Update> PhoneDigestHasher<T> {
    pub fn new_with_salt(mut digest: T, salt: &[u8]) -> Self {
        Update::update(&mut digest, salt);
        Self(digest)
    }
}

#[cfg(feature = "digest")]
fn feed_phone_bytes(updater: &mut impl Update, phone: &PhoneNumber) {
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
impl<D: Digest + Update> PhoneHasher for PhoneDigestHasher<D> {
    fn hash_phone(mut self, phone: &PhoneNumber) -> Option<Hashed> {
        if <D as OutputSizeUser>::output_size() > 64 {
            return None;
        }
        feed_phone_bytes(&mut self.0, phone);
        let out = self.0.finalize();

        Hashed::from_slice(out)
    }
}

#[cfg(feature = "digest_mac")]
impl<M: Mac + Update> PhoneHasher for PhoneMacHasher<M> {
    fn hash_phone(mut self, phone: &PhoneNumber) -> Option<Hashed> {
        if <M as OutputSizeUser>::output_size() > 64 {
            return None;
        }

        feed_phone_bytes(&mut self.0, phone);
        let out = self.0.finalize();

        Hashed::from_slice(out.as_bytes())
    }
}
