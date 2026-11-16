//! Shared error type for the library.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SfaiError {
    #[error("validation error: {0}")]
    Validation(String),
    #[error("storage error: {0}")]
    Storage(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("policy denied: {0}")]
    Policy(String),
    #[error("tool error: {0}")]
    Tool(String),
    #[error("model error: {0}")]
    Model(String),
    #[error("intent validation error: {0}")]
    Intent(String),
    #[error("internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, SfaiError>;

impl From<String> for SfaiError {
    fn from(s: String) -> Self {
        SfaiError::Validation(s)
    }
}

impl From<rusqlite::Error> for SfaiError {
    fn from(e: rusqlite::Error) -> Self {
        SfaiError::Storage(e.to_string())
    }
}

impl From<serde_json::Error> for SfaiError {
    fn from(e: serde_json::Error) -> Self {
        SfaiError::Validation(e.to_string())
    }
}

impl From<std::io::Error> for SfaiError {
    fn from(e: std::io::Error) -> Self {
        SfaiError::Storage(e.to_string())
    }
}
