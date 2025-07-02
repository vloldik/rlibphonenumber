use thiserror::Error;

use crate::regexp_cache::ErrorInvalidRegex;

#[derive(Debug, PartialEq, Error)]
pub enum PhoneNumberUtilError {
    #[error("{0}")]
    InvalidRegexError(#[from] ErrorInvalidRegex)
}