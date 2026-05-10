// codex-wire-protocol-openai/src/lib.rs

#![deny(clippy::print_stdout, clippy::print_stderr)]

pub mod compaction;
pub mod error;
pub mod models;
pub mod protocol;
pub mod request;
pub mod response;
pub mod sse;
pub mod types;
pub mod websocket;

pub use protocol::OpenAiWireProtocol;
