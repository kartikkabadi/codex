// codex-wire-protocol/src/lib.rs

#![deny(clippy::print_stdout, clippy::print_stderr)]

pub mod auth;
pub mod capabilities;
pub mod error;
pub mod protocol;
pub mod request;
pub mod response;

pub use auth::AuthToken;
pub use capabilities::ProtocolCapabilities;
pub use error::WireError;
pub use protocol::WireProtocol;
pub use request::{
    CompactRequest, ReasoningConfig, ReasoningEffort, Role, ToolChoice, ToolSpec, TurnItem,
    TurnRequest,
};
pub use response::{CompactResponse, ModelInfo, TurnEvent, TurnStream, Usage};
