use thiserror::Error;

pub type FormatResult<T> = Result<T, FormatError>;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum FormatError {
    #[error("missing section: {0}")]
    MissingSection(String),
    #[error("invalid size: {0}")]
    InvalidSize(String),
    #[error("grid row count mismatch: expected {expected}, got {got}")]
    RowCountMismatch { expected: u16, got: usize },
    #[error("unknown character: {0}")]
    UnknownChar(char),
    #[error("parse error: {0}")]
    Parse(String),
}
