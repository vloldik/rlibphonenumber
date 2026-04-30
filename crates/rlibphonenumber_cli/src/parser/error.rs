use thiserror::Error;

#[derive(Debug, Error)]
pub enum MetadataError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("XML parsing error: {0}")]
    Xml(#[from] roxmltree::Error),

    #[error("Regex compilation error: {0}")]
    Regex(#[from] regex::Error),

    #[error("CEL execution/compilation error: {0}")]
    Cel(String),

    #[error("Parse integer error: {0}")]
    ParseInt(#[from] std::num::ParseIntError),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Build error: {0}")]
    Build(String),
}

pub type Result<T> = std::result::Result<T, MetadataError>;
