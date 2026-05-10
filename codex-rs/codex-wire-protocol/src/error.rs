// codex-wire-protocol/src/error.rs

use thiserror::Error;

#[derive(Error, Debug)]
pub enum WireError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Stream error: {0}")]
    Stream(String),
    #[error("Unsupported operation: {0}")]
    Unsupported(String),
    #[error("Authentication error: {0}")]
    Auth(String),
    #[error("Rate limited: {0}")]
    RateLimited(String),
    #[error("Provider error ({code}): {message}")]
    Provider { code: String, message: String },
    #[error("Other: {0}")]
    Other(String),
}
