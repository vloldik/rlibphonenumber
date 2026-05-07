use std::{hash::Hasher, io::ErrorKind};

#[cfg(feature = "digest_mac")]
use digest::Mac;
#[cfg(feature = "digest")]
use digest::{Digest, Update};
use thiserror::Error;

use crate::{PhoneNumber, interfaces::PhoneHasher, phonenumber_mask::hash::PhoneStdHasher};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error, Hash)]
#[error("Invalid length: digest must be at most 64 bytes long, got: {0}")]
pub struct MaxHashedLengthExceededError(pub usize);

impl From<MaxHashedLengthExceededError> for std::io::Error {
    fn from(value: MaxHashedLengthExceededError) -> Self {
        std::io::Error::new(ErrorKind::InvalidData, value)
    }
}

pub type Result<T> = std::result::Result<T, MaxHashedLengthExceededError>;

/// Configuration for character-by-character digit masking.
#[derive(Debug, Clone, Copy)]
pub struct MaskDigitsConfig {
    /// The UTF-8 character used to obscure sensitive digits (e.g., `'*'`, `'X'`, or `'🔒'`).
    pub mask_char: char,

    pub min_masked: usize,
    pub max_unmasked: usize,
}

/// A stack-allocated buffer capable of storing up to 64 bytes of hash data.
///
/// Designed to avoid heap allocations when working with hash outputs (e.g., SHA-256, HMAC).
#[derive(Clone, Copy)]
pub struct Hashed([u8; 64], usize);

impl MaskDigitsConfig {
    /// Creates a new configuration for digit masking.
    pub fn new(mask_char: char, min_masked: usize, max_unmasked: usize) -> Self {
        Self {
            mask_char,
            min_masked,
            max_unmasked,
        }
    }
}

impl Default for MaskDigitsConfig {
    /// Default configuration: masks using `'*'` and preserves the last 4 digits.
    fn default() -> Self {
        Self::new('*', 3, 4)
    }
}

impl Hashed {
    /// Creates a `Hashed` instance using a standard Rust `Hasher` (e.g., `DefaultHasher`, `SipHash`).
    pub fn from_hasher(hasher: impl Hasher, phone: &PhoneNumber) -> Self {
        PhoneStdHasher(hasher).hash_phone(phone).unwrap() // Always returns Some() and hasher returns u64
    }

    /// Creates a `Hashed` instance using a cryptographic digest (e.g., `Sha256`).
    ///
    /// Returns `None` if the chosen digest's output exceeds 64 bytes.
    #[cfg(feature = "digest")]
    pub fn from_digest(digest: impl Digest + Update, phone: &PhoneNumber) -> Result<Self> {
        use crate::phonenumber_mask::hash::PhoneDigestHasher;
        // Always returns some
        PhoneDigestHasher(digest).hash_phone(phone)
    }

    /// Creates a `Hashed` instance using a cryptographic digest seeded with salt bytes.
    #[cfg(feature = "digest")]
    pub fn from_salted_digest(
        digest: impl Digest + Update,
        salt: &[u8],
        phone: &PhoneNumber,
    ) -> Result<Self> {
        use crate::phonenumber_mask::hash::PhoneDigestHasher;
        // Always returns Some
        PhoneDigestHasher::new_with_salt(digest, salt).hash_phone(phone)
    }

    /// Creates a `Hashed` instance using a Message Authentication Code (e.g., `HMAC`).
    #[cfg(feature = "digest_mac")]
    pub fn from_mac(mac: impl Mac + Update, phone: &PhoneNumber) -> Result<Self> {
        use crate::phonenumber_mask::hash::PhoneMacHasher;
        // Always returns Some
        PhoneMacHasher(mac).hash_phone(phone)
    }

    /// Constructs a `Hashed` buffer from a raw byte slice.
    ///
    /// # Returns
    /// Returns `Some(Self)` if `bytes.len() <= 64`. Returns `None` otherwise.
    pub fn from_slice(bytes: impl AsRef<[u8]>) -> Result<Self> {
        let len = bytes.as_ref().len();
        if len > 64 {
            return Err(MaxHashedLengthExceededError(len));
        }

        let mut buf = [0u8; 64];
        buf[..len].copy_from_slice(bytes.as_ref());

        Ok(Self(buf, len))
    }

    /// Retrieves the underlying payload as a slice of bytes.
    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        &self.0[..self.1]
    }

    #[inline]
    pub fn len(&self) -> usize {
        return self.1;
    }
}

// Standard trait implementations for `Hashed`
impl PartialEq for Hashed {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}
impl Eq for Hashed {}

impl std::hash::Hash for Hashed {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_slice().hash(state);
    }
}

impl std::fmt::Debug for Hashed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Hash(\"")?;
        for byte in self.as_slice() {
            write!(f, "{:02x}", byte)?;
        }
        write!(f, "\")")
    }
}

impl AsRef<[u8]> for Hashed {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}
