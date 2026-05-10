// codex-wire-protocol/src/protocol.rs

use crate::auth::AuthToken;
use crate::capabilities::ProtocolCapabilities;
use crate::error::WireError;
use crate::request::TurnRequest;
use crate::response::TurnStream;
use std::future::Future;

/// A wire protocol adapter for communicating with an LLM provider.
///
/// Implementations translate between Codex's generic turn-based model
/// and provider-specific API formats (OpenAI Responses, Anthropic Messages,
/// Gemini, etc.).
pub trait WireProtocol: Send + Sync {
    /// Unique protocol identifier, e.g. "openai-responses", "anthropic-messages".
    fn protocol_id(&self) -> &str;

    /// Human-readable protocol name.
    fn protocol_name(&self) -> &str;

    /// Stream a single turn (prompt -> response).
    ///
    /// Returns a `TurnStream` that yields events incrementally.
    fn stream_turn(
        &self,
        request: TurnRequest,
        auth: AuthToken,
    ) -> impl Future<Output = Result<TurnStream, WireError>> + Send;

    /// List available models from the provider.
    fn list_models(
        &self,
        auth: AuthToken,
    ) -> impl Future<Output = Result<Vec<crate::response::ModelInfo>, WireError>> + Send;

    /// Compact/summarize conversation context (optional).
    ///
    /// Returns `Err(WireError::Unsupported)` if the protocol does not
    /// support native compaction.
    fn compact_context(
        &self,
        request: crate::request::CompactRequest,
        auth: AuthToken,
    ) -> impl Future<Output = Result<crate::response::CompactResponse, WireError>> + Send;

    /// Query protocol capabilities.
    fn capabilities(&self) -> ProtocolCapabilities;
}
