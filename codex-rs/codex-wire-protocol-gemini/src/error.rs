// codex-wire-protocol-gemini/src/error.rs

use std::fmt::Display;
use std::fmt::Formatter;

#[derive(Debug)]
pub enum GeminiError {
    Http(String),
    Serialization(String),
    Api { status: u16, message: String },
    Sse(String),
    Unsupported,
}

impl Display for GeminiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GeminiError::Http(msg) => write!(f, "HTTP error: {msg}"),
            GeminiError::Serialization(msg) => write!(f, "Serialization error: {msg}"),
            GeminiError::Api { status, message } => {
                write!(f, "API error ({status}): {message}")
            }
            GeminiError::Sse(msg) => write!(f, "SSE error: {msg}"),
            GeminiError::Unsupported => write!(f, "Unsupported operation"),
        }
    }
}

impl std::error::Error for GeminiError {}

impl From<GeminiError> for codex_wire_protocol::error::WireError {
    fn from(err: GeminiError) -> Self {
        match err {
            GeminiError::Http(msg) => codex_wire_protocol::error::WireError::Stream(format!("HTTP: {msg}")),
            GeminiError::Serialization(msg) => {
                let io_err = std::io::Error::other(msg);
                codex_wire_protocol::error::WireError::Serialization(serde_json::Error::io(io_err))
            }
            GeminiError::Api { status, message } => {
                codex_wire_protocol::error::WireError::Provider {
                    code: status.to_string(),
                    message,
                }
            }
            GeminiError::Sse(msg) => codex_wire_protocol::error::WireError::Stream(msg),
            GeminiError::Unsupported => {
                codex_wire_protocol::error::WireError::Unsupported("gemini".into())
            }
        }
    }
}
