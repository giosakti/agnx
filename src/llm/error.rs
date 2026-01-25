//! LLM error types.

use std::fmt;

/// Errors that can occur when making LLM API calls.
#[derive(Debug)]
pub enum LLMError {
    /// HTTP request failed
    Request(reqwest::Error),
    /// API returned an error response
    Api { status: u16, message: String },
}

impl fmt::Display for LLMError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LLMError::Request(e) => write!(f, "HTTP request failed: {e}"),
            LLMError::Api { status, message } => {
                write!(f, "API error (status {status}): {message}")
            }
        }
    }
}

impl std::error::Error for LLMError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LLMError::Request(e) => Some(e),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for LLMError {
    fn from(err: reqwest::Error) -> Self {
        LLMError::Request(err)
    }
}
