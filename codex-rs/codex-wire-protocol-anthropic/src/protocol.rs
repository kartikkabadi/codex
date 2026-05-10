// codex-wire-protocol-anthropic/src/protocol.rs

use codex_wire_protocol::auth::AuthToken;
use codex_wire_protocol::capabilities::ProtocolCapabilities;
use codex_wire_protocol::error::WireError;
use codex_wire_protocol::protocol::WireProtocol;
use codex_wire_protocol::request::CompactRequest;
use codex_wire_protocol::request::TurnRequest;
use codex_wire_protocol::response::CompactResponse;
use codex_wire_protocol::response::ModelInfo;
use codex_wire_protocol::response::TurnStream;
use reqwest::Client;

use crate::models::list_anthropic_models;
use crate::request::turn_request_to_messages_request;
use crate::response::response_stream_to_turn_stream;
use crate::sse::AnthropicSseStream;
use crate::types::MessagesRequest;

/// Anthropic Messages API wire-protocol adapter.
pub struct AnthropicWireProtocol {
    http_client: Client,
    api_provider: String,
    auth_token: String,
}

impl AnthropicWireProtocol {
    pub fn new(api_provider: String, auth_token: String) -> Self {
        Self {
            http_client: Client::new(),
            api_provider,
            auth_token,
        }
    }

    pub fn with_client(mut self, client: Client) -> Self {
        self.http_client = client;
        self
    }

    fn api_key(auth: &AuthToken) -> Result<String, WireError> {
        match auth {
            AuthToken::Bearer(token) => Ok(token.clone()),
            AuthToken::ApiKey { key, .. } => Ok(key.clone()),
            _ => Err(WireError::Auth(
                "Unsupported auth type for Anthropic".into(),
            )),
        }
    }
}

impl WireProtocol for AnthropicWireProtocol {
    fn protocol_id(&self) -> &str {
        "anthropic-messages"
    }

    fn protocol_name(&self) -> &str {
        "Anthropic Messages API"
    }

    fn stream_turn(
        &self,
        request: TurnRequest,
        auth: AuthToken,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<TurnStream, WireError>> + Send>>
    {
        let client = self.http_client.clone();
        let base_url = self.api_provider.clone();
        let api_key = match Self::api_key(&auth) {
            Ok(k) => k,
            Err(e) => return Box::pin(async move { Err(e) }),
        };
        Box::pin(async move {
            let anthropic_request: MessagesRequest = turn_request_to_messages_request(request);
            let body =
                serde_json::to_value(&anthropic_request).map_err(WireError::Serialization)?;

            let url = format!("{base_url}/messages");
            let resp = client
                .post(&url)
                .header("x-api-key", &api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(WireError::Http)?;

            let bytes = resp.bytes_stream();
            let byte_stream: crate::sse::ByteStream = Box::pin(bytes);
            let sse_stream = AnthropicSseStream::new(byte_stream);

            let turn_id = format!("turn-{}", uuid::Uuid::new_v4());
            Ok(response_stream_to_turn_stream(sse_stream, turn_id))
        })
    }

    fn list_models(
        &self,
        _auth: AuthToken,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Vec<ModelInfo>, WireError>> + Send>,
    > {
        Box::pin(async move { Ok(list_anthropic_models()) })
    }

    fn compact_context(
        &self,
        _request: CompactRequest,
        _auth: AuthToken,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<CompactResponse, WireError>> + Send>,
    > {
        Box::pin(async move {
            Err(WireError::Unsupported(
                "Anthropic does not support native compaction".into(),
            ))
        })
    }

    fn capabilities(&self) -> ProtocolCapabilities {
        ProtocolCapabilities {
            supports_streaming: true,
            supports_tools: true,
            supports_vision: true,
            supports_reasoning: false,
            supports_realtime: false,
            supports_compaction: false,
            supports_websockets: false,
            supports_parallel_tool_calls: false,
        }
    }
}
