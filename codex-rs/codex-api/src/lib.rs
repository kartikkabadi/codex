// codex-api/src/lib.rs

pub(crate) mod api_bridge;
pub(crate) mod auth;
pub(crate) mod common;
pub(crate) mod endpoint;
pub(crate) mod error;
pub(crate) mod files;
pub(crate) mod images;
pub(crate) mod provider;
pub(crate) mod rate_limits;
pub(crate) mod requests;
pub(crate) mod safety_buffering;
pub(crate) mod search;
pub(crate) mod sse;
pub(crate) mod telemetry;

// ---------------------------------------------------------------------------
// Re-exports from codex-wire-protocol (protocol-agnostic types)
// ---------------------------------------------------------------------------
pub use codex_wire_protocol::auth::AuthToken;
pub use codex_wire_protocol::capabilities::ProtocolCapabilities;
pub use codex_wire_protocol::error::WireError;
pub use codex_wire_protocol::protocol::WireProtocol;
pub use codex_wire_protocol::request::CompactRequest;
pub use codex_wire_protocol::request::ReasoningConfig;
pub use codex_wire_protocol::request::ReasoningEffort;
pub use codex_wire_protocol::request::Role;
pub use codex_wire_protocol::request::ToolChoice;
pub use codex_wire_protocol::request::ToolSpec;
pub use codex_wire_protocol::request::TurnItem;
pub use codex_wire_protocol::request::TurnRequest;
pub use codex_wire_protocol::response::CompactResponse;
pub use codex_wire_protocol::response::ModelInfo;
pub use codex_wire_protocol::response::TurnEvent;
pub use codex_wire_protocol::response::TurnStream;
pub use codex_wire_protocol::response::Usage;

// ---------------------------------------------------------------------------
// New protocol-agnostic dispatch layer
// ---------------------------------------------------------------------------
use std::sync::Arc;

use codex_wire_protocol_anthropic::AnthropicWireProtocol;
use codex_wire_protocol_gemini::GeminiWireProtocol;
use codex_wire_protocol_openai::OpenAiWireProtocol;

/// Thin dispatch client that holds a protocol adapter and delegates all
/// turn-based, model-listing, and compaction operations to it.
///
/// This is the preferred entry-point for new code. Legacy endpoint-specific
/// clients (`ResponsesClient`, `ModelsClient`, …) remain available for
/// backward compatibility but are deprecated.
pub struct ApiClient {
    protocol: Arc<dyn WireProtocol>,
}

impl ApiClient {
    /// Create a new `ApiClient` backed by the given protocol adapter.
    pub fn new(protocol: Arc<dyn WireProtocol>) -> Self {
        Self { protocol }
    }

    /// Access the underlying protocol adapter.
    pub fn protocol(&self) -> &Arc<dyn WireProtocol> {
        &self.protocol
    }

    /// Stream a single turn (prompt -> response).
    pub async fn stream_turn(
        &self,
        request: TurnRequest,
        auth: AuthToken,
    ) -> Result<TurnStream, WireError> {
        self.protocol.stream_turn(request, auth).await
    }

    /// List available models from the provider.
    pub async fn list_models(&self, auth: AuthToken) -> Result<Vec<ModelInfo>, WireError> {
        self.protocol.list_models(auth).await
    }

    /// Compact / summarize conversation context.
    ///
    /// Returns `Err(WireError::Unsupported)` when the protocol does not
    /// implement native compaction.
    pub async fn compact_context(
        &self,
        request: CompactRequest,
        auth: AuthToken,
    ) -> Result<CompactResponse, WireError> {
        self.protocol.compact_context(request, auth).await
    }

    /// Query the protocol capabilities.
    pub fn capabilities(&self) -> ProtocolCapabilities {
        self.protocol.capabilities()
    }
}

/// Factory that creates a concrete protocol adapter from a protocol id.
///
/// Supported identifiers:
/// * `"openai-responses"` | `"openai-chat-completions"` → [`OpenAiWireProtocol`]
/// * `"anthropic-messages"` → [`AnthropicWireProtocol`]
/// * `"gemini"` → [`GeminiWireProtocol`]
///
/// Returns `Err(ApiError::UnsupportedProtocol)` for unknown identifiers.
pub fn create_protocol(
    protocol_id: &str,
    base_url: String,
) -> Result<Arc<dyn WireProtocol>, crate::error::ApiError> {
    match protocol_id {
        "openai-responses" | "openai-chat-completions" => {
            let adapter = OpenAiWireProtocol::new(base_url);
            Ok(Arc::new(adapter))
        }
        "anthropic-messages" => {
            let adapter = AnthropicWireProtocol::new(base_url, String::new());
            Ok(Arc::new(adapter))
        }
        "gemini" => {
            let adapter = GeminiWireProtocol::new(base_url);
            Ok(Arc::new(adapter))
        }
        "replicate" => {
            // Replicate uses an OpenAI-compatible chat completions endpoint
            let adapter = OpenAiWireProtocol::new(base_url);
            Ok(Arc::new(adapter))
        }
        _ => Err(crate::error::ApiError::UnsupportedProtocol(
            protocol_id.to_string(),
        )),
    }
}

// ---------------------------------------------------------------------------
// Backward-compatible re-exports (legacy endpoint modules)
// ---------------------------------------------------------------------------

pub use crate::requests::headers::build_session_headers;
pub use codex_client::RequestTelemetry;
pub use codex_client::ReqwestTransport;
pub use codex_client::TransportError;

pub use crate::api_bridge::map_api_error;
pub use crate::auth::AgentIdentityTelemetry;
pub use crate::auth::AuthError;
pub use crate::auth::AuthHeaderTelemetry;
pub use crate::auth::AuthProvider;
pub use crate::auth::AuthProviderFuture;
pub use crate::auth::SharedAuthProvider;
pub use crate::auth::auth_header_telemetry;
pub use crate::common::CompactionInput;
pub use crate::common::MemorySummarizeInput;
pub use crate::common::MemorySummarizeOutput;
pub use crate::common::OpenAiVerbosity;
pub use crate::common::RawMemory;
pub use crate::common::RawMemoryMetadata;
pub use crate::common::Reasoning;
pub use crate::common::ReasoningContext;
pub use crate::common::ReasoningSummaryDelivery;
pub use crate::common::ResponseCreateWsRequest;
pub use crate::common::ResponseEvent;
pub use crate::common::ResponseStream;
pub use crate::common::ResponsesApiRequest;
pub use crate::common::ResponsesApiTools;
pub use crate::common::ResponsesWsRequest;
pub use crate::common::StreamOptions;
pub use crate::common::TextControls;
pub use crate::common::WS_REQUEST_HEADER_TRACEPARENT_CLIENT_METADATA_KEY;
pub use crate::common::WS_REQUEST_HEADER_TRACESTATE_CLIENT_METADATA_KEY;
pub use crate::common::create_text_param_for_request;
pub use crate::common::response_create_client_metadata;

/// **Deprecated** — OpenAI-specific responses endpoint client.
/// Use [`ApiClient`] with an `openai-responses` protocol instead.
pub use crate::endpoint::CompactClient;
pub use crate::endpoint::ImagesClient;
/// **Deprecated** — OpenAI-specific responses endpoint client.
/// Use [`ApiClient`] with an `openai-responses` protocol instead.
pub use crate::endpoint::MemoriesClient;
/// **Deprecated** — OpenAI-specific responses endpoint client.
/// Use [`ApiClient`] with an `openai-responses` protocol instead.
pub use crate::endpoint::ModelsClient;
/// **Deprecated** — OpenAI-specific responses endpoint client.
/// Use [`ApiClient`] with an `openai-responses` protocol instead.
pub use crate::endpoint::RealtimeCallClient;
pub use crate::endpoint::RealtimeCallResponse;
pub use crate::endpoint::RealtimeContextAppendChannel;
pub use crate::endpoint::RealtimeEventParser;
pub use crate::endpoint::RealtimeOutputModality;
pub use crate::endpoint::RealtimeSessionConfig;
pub use crate::endpoint::RealtimeSessionMode;
pub use crate::endpoint::RealtimeWebsocketClient;
pub use crate::endpoint::RealtimeWebsocketConnection;
pub use crate::endpoint::RealtimeWebsocketEvents;
pub use crate::endpoint::RealtimeWebsocketWriter;
/// **Deprecated** — OpenAI-specific responses endpoint client.
/// Use [`ApiClient`] with an `openai-responses` protocol instead.
pub use crate::endpoint::ResponsesClient;
/// **Deprecated** — OpenAI-specific responses endpoint client.
/// Use [`ApiClient`] with an `openai-responses` protocol instead.
pub use crate::endpoint::ResponsesOptions;
pub use crate::endpoint::ResponsesWebsocketClient;
pub use crate::endpoint::ResponsesWebsocketClose;
pub use crate::endpoint::ResponsesWebsocketConnection;
pub use crate::endpoint::ResponsesWebsocketProbe;
pub use crate::endpoint::SearchClient;
pub use crate::endpoint::session_update_session_json;
pub use crate::error::ApiError;
pub use crate::files::HostedFileUploadContext;
pub use crate::files::OPENAI_FILE_UPLOAD_LIMIT_BYTES;
pub use crate::files::upload_openai_file;
pub use crate::images::ImageBackground;
pub use crate::images::ImageData;
pub use crate::images::ImageEditRequest;
pub use crate::images::ImageGenerationRequest;
pub use crate::images::ImageQuality;
pub use crate::images::ImageResponse;
pub use crate::images::ImageUrl;
pub use crate::provider::Provider;
pub use crate::provider::RetryConfig;
pub use crate::provider::is_azure_responses_provider;
pub use crate::requests::Compression;
pub use crate::search::AllowedCaller;
pub use crate::search::ApproximateLocation;
pub use crate::search::ClickOperation;
pub use crate::search::ExternalWebAccess;
pub use crate::search::ExternalWebAccessMode;
pub use crate::search::FinanceAssetType;
pub use crate::search::FinanceOperation;
pub use crate::search::FindOperation;
pub use crate::search::LocationType;
pub use crate::search::OpenOperation;
pub use crate::search::ScreenshotOperation;
pub use crate::search::SearchCommands;
pub use crate::search::SearchContextSize;
pub use crate::search::SearchFilters;
pub use crate::search::SearchImageSettings;
pub use crate::search::SearchInput;
pub use crate::search::SearchQuery;
pub use crate::search::SearchRequest;
pub use crate::search::SearchResponse;
pub use crate::search::SearchResponseLength;
pub use crate::search::SearchSettings;
pub use crate::search::SportsFunction;
pub use crate::search::SportsLeague;
pub use crate::search::SportsOperation;
pub use crate::search::SportsToolName;
pub use crate::search::TimeOperation;
pub use crate::search::WeatherOperation;
pub use crate::telemetry::SseTelemetry;
pub use crate::telemetry::WebsocketTelemetry;
pub use codex_protocol::protocol::RealtimeAudioFrame;
pub use codex_protocol::protocol::RealtimeEvent;
