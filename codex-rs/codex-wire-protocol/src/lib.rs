// codex-wire-protocol/src/lib.rs

#![deny(clippy::print_stdout, clippy::print_stderr)]

pub mod auth;
pub mod capabilities;
pub mod error;
pub mod protocol;
pub mod request;
pub mod response;

pub use auth::AuthError;
pub use auth::AuthProvider;
pub use auth::AuthToken;
pub use auth::AuthType;
pub use capabilities::ProtocolCapabilities;
pub use error::WireError;
pub use protocol::WireProtocol;
pub use request::CompactRequest;
pub use request::ReasoningConfig;
pub use request::ReasoningEffort;
pub use request::Role;
pub use request::ToolChoice;
pub use request::ToolSpec;
pub use request::TurnItem;
pub use request::TurnRequest;
pub use response::CompactResponse;
pub use response::ModelInfo;
pub use response::TurnEvent;
pub use response::TurnStream;
pub use response::Usage;
