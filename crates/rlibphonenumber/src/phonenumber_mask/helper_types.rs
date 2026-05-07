use std::{hash::Hasher, io::ErrorKind};

#[cfg(feature = "digest_mac")]
use digest::Mac;
#[cfg(feature = "digest")]
use digest::{Digest, Update};
use thiserror::Error;

use crate::{
    PhoneNumber,
    interfaces::{LenWrite, PhoneHasher},
    phonenumber_mask::hash::PhoneStdHasher,
};

/// An error indicating that a cryptographic digest or MAC output exceeded the maximum allowed length.
///
/// The stack-allocated buffer for hashed phone numbers supports up to 64 bytes.
/// Using a hashing algorithm with a larger output size will trigger this error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error, Hash)]
#[error("Invalid length: digest must be at most 64 bytes long, got: {0}")]
pub struct MaxHashedLengthExceededError(pub usize);

impl From<MaxHashedLengthExceededError> for std::io::Error {
    fn from(value: MaxHashedLengthExceededError) -> Self {
        std::io::Error::new(ErrorKind::InvalidData, value)
    }
}

#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct LenWriteString(String);

/// A specialized `Result` type for phone number hashing operations.
pub type Result<T> = std::result::Result<T, MaxHashedLengthExceededError>;

/// Configuration for character-by-character digit masking.
///
/// Defines how raw phone numbers should be partially obscured (e.g., `***-**55`).
#[derive(Debug, Clone, Copy)]
pub struct MaskDigitsConfig {
    /// The UTF-8 character used to obscure sensitive digits (e.g., `'*'`, `'X'`, or `'🔒'`).
    ///
    /// **'*' by default**
    pub mask_char: char,

    /// The minimum number of digits that must be masked, regardless of the total length.
    ///
    /// **Default is 3**
    pub min_masked: usize,

    /// The maximum number of trailing digits to leave unmasked (visible) at the end of the number.
    ///
    /// **Default is 4**
    pub max_unmasked: usize,
}

impl MaskDigitsConfig {
    /// Creates a new configuration for digit masking.
    ///
    /// # Arguments
    /// * `mask_char` - The character to replace digits with.
    /// * `min_masked` - The minimum amount of digits to obscure.
    /// * `max_unmasked` - The maximum amount of trailing digits to reveal.
    pub fn new(mask_char: char, min_masked: usize, max_unmasked: usize) -> Self {
        Self {
            mask_char,
            min_masked,
            max_unmasked,
        }
    }
}

impl Default for MaskDigitsConfig {
    /// Default configuration: masks using `'*'`, requires at least 3 masked digits,
    /// and preserves a maximum of the last 4 digits.
    fn default() -> Self {
        Self::new('*', 3, 4)
    }
}

/// A stack-allocated buffer capable of storing up to 64 bytes of hash data.
///
/// Designed to completely avoid heap allocations when working with hash outputs
/// (e.g., SHA-256, HMAC, SipHash).
#[derive(Clone, Copy)]
pub struct Hashed([u8; 64], usize);

impl Hashed {
    /// Creates a `Hashed` instance using a standard Rust `Hasher` (e.g., `DefaultHasher`, `SipHash`).
    pub fn from_hasher(hasher: impl Hasher, phone: &PhoneNumber) -> Self {
        PhoneStdHasher(hasher).hash_phone(phone).unwrap() // Always returns Some() and hasher returns u64
    }

    /// Creates a `Hashed` instance using a cryptographic digest (e.g., `Sha256`).
    ///
    /// # Errors
    /// Returns an error if the chosen digest's output exceeds 64 bytes.
    #[cfg(feature = "digest")]
    pub fn from_digest(digest: impl Digest + Update, phone: &PhoneNumber) -> Result<Self> {
        use crate::phonenumber_mask::hash::PhoneDigestHasher;
        PhoneDigestHasher(digest).hash_phone(phone)
    }

    /// Creates a `Hashed` instance using a cryptographic digest seeded with a salt.
    ///
    /// # Errors
    /// Returns an error if the chosen digest's output exceeds 64 bytes.
    #[cfg(feature = "digest")]
    pub fn from_salted_digest(
        digest: impl Digest + Update,
        salt: &[u8],
        phone: &PhoneNumber,
    ) -> Result<Self> {
        use crate::phonenumber_mask::hash::PhoneDigestHasher;
        PhoneDigestHasher::new_with_salt(digest, salt).hash_phone(phone)
    }

    /// Creates a `Hashed` instance using a Message Authentication Code (e.g., `HMAC`).
    ///
    /// # Errors
    /// Returns an error if the MAC output exceeds 64 bytes.
    #[cfg(feature = "digest_mac")]
    pub fn from_mac(mac: impl Mac + Update, phone: &PhoneNumber) -> Result<Self> {
        use crate::phonenumber_mask::hash::PhoneMacHasher;
        PhoneMacHasher(mac).hash_phone(phone)
    }

    /// Constructs a `Hashed` buffer directly from a raw byte slice.
    ///
    /// # Errors
    /// Returns a `MaxHashedLengthExceededError` if `bytes.len() > 64`.
    pub fn from_slice(bytes: impl AsRef<[u8]>) -> Result<Self> {
        let len = bytes.as_ref().len();
        if len > 64 {
            return Err(MaxHashedLengthExceededError(len));
        }

        let mut buf = [0u8; 64];
        buf[..len].copy_from_slice(bytes.as_ref());

        Ok(Self(buf, len))
    }

    /// Retrieves the underlying payload as a byte slice.
    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        &self.0[..self.1]
    }

    /// Returns the length of the valid hashed data.
    #[inline]
    pub fn len(&self) -> usize {
        self.1
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

impl LenWrite for LenWriteString {
    fn grow(&mut self, len: usize) {
        self.0.reserve_exact(len);
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        let s = std::str::from_utf8(buf)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        self.0.push_str(s);
        Ok(())
    }
}

impl From<LenWriteString> for String {
    fn from(value: LenWriteString) -> Self {
        value.0
    }
}

impl LenWriteString {
    pub fn new() -> Self {
        Self(String::new())
    }
}

