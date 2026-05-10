// codex-wire-protocol-openai/src/error.rs

use std::time::Duration;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum OpenAiError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Stream error: {0}")]
    Stream(String),
    #[error("Context window exceeded")]
    ContextWindowExceeded,
    #[error("Quota exceeded")]
    QuotaExceeded,
    #[error("Usage not included")]
    UsageNotIncluded,
    #[error("Cyber policy: {message}")]
    CyberPolicy { message: String },
    #[error("Invalid request: {message}")]
    InvalidRequest { message: String },
    #[error("Server overloaded")]
    ServerOverloaded,
    #[error("Retryable: {message}")]
    Retryable {
        message: String,
        delay: Option<Duration>,
    },
    #[error("WebSocket error: {0}")]
    WebSocket(String),
}

impl From<OpenAiError> for codex_wire_protocol::error::WireError {
    fn from(err: OpenAiError) -> Self {
        match err {
            OpenAiError::Http(e) => codex_wire_protocol::error::WireError::Http(e),
            OpenAiError::Serialization(e) => {
                codex_wire_protocol::error::WireError::Serialization(e)
            }
            OpenAiError::Stream(msg) => codex_wire_protocol::error::WireError::Stream(msg),
            OpenAiError::ContextWindowExceeded => {
                codex_wire_protocol::error::WireError::Provider {
                    code: "context_length_exceeded".into(),
                    message: "Context window exceeded".into(),
                }
            }
            OpenAiError::QuotaExceeded => codex_wire_protocol::error::WireError::Provider {
                code: "insufficient_quota".into(),
                message: "Quota exceeded".into(),
            },
            OpenAiError::UsageNotIncluded => codex_wire_protocol::error::WireError::Provider {
                code: "usage_not_included".into(),
                message: "Usage not included".into(),
            },
            OpenAiError::CyberPolicy { message } => {
                codex_wire_protocol::error::WireError::Provider {
                    code: "cyber_policy".into(),
                    message,
                }
            }
            OpenAiError::InvalidRequest { message } => {
                codex_wire_protocol::error::WireError::Provider {
                    code: "invalid_request".into(),
                    message,
                }
            }
            OpenAiError::ServerOverloaded => {
                codex_wire_protocol::error::WireError::Provider {
                    code: "server_overloaded".into(),
                    message: "Server overloaded".into(),
                }
            }
            OpenAiError::Retryable { message, delay } => {
                codex_wire_protocol::error::WireError::RateLimited(format!(
                    "{message} (retry after: {:?})",
                    delay
                ))
            }
            OpenAiError::WebSocket(msg) => codex_wire_protocol::error::WireError::Stream(msg),
        }
    }
}
