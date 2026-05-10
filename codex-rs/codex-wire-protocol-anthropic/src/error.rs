// codex-wire-protocol-anthropic/src/error.rs

use std::fmt::Display;
use std::fmt::Formatter;

#[derive(Debug)]
pub enum AnthropicError {
    Http(String),
    Serialization(String),
    Api { status: u16, message: String },
    Sse(String),
    Unsupported,
}

impl Display for AnthropicError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AnthropicError::Http(msg) => write!(f, "HTTP error: {msg}"),
            AnthropicError::Serialization(msg) => write!(f, "Serialization error: {msg}"),
            AnthropicError::Api { status, message } => {
                write!(f, "API error ({status}): {message}")
            }
            AnthropicError::Sse(msg) => write!(f, "SSE error: {msg}"),
            AnthropicError::Unsupported => write!(f, "Unsupported operation"),
        }
    }
}

impl From<AnthropicError> for codex_wire_protocol::error::WireError {
    fn from(err: AnthropicError) -> Self {
        match err {
            AnthropicError::Http(msg) => {
                codex_wire_protocol::error::WireError::Stream(format!("HTTP: {msg}"))
            }
            AnthropicError::Serialization(msg) => {
                let io_err = std::io::Error::other(msg);
                codex_wire_protocol::error::WireError::Serialization(serde_json::Error::io(io_err))
            }
            AnthropicError::Api { status, message } => {
                codex_wire_protocol::error::WireError::Provider {
                    code: status.to_string(),
                    message,
                }
            }
            AnthropicError::Sse(msg) => codex_wire_protocol::error::WireError::Stream(msg),
            AnthropicError::Unsupported => {
                codex_wire_protocol::error::WireError::Unsupported("anthropic".into())
            }
        }
    }
}
