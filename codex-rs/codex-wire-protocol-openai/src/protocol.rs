// codex-wire-protocol-openai/src/protocol.rs

use std::future::Future;

use codex_wire_protocol::auth::AuthToken;
use codex_wire_protocol::capabilities::ProtocolCapabilities;
use codex_wire_protocol::error::WireError;
use codex_wire_protocol::protocol::WireProtocol;
use codex_wire_protocol::request::{CompactRequest, TurnRequest};
use codex_wire_protocol::response::{CompactResponse, ModelInfo, TurnStream};
use futures::StreamExt;
use http::HeaderMap;
use reqwest::Client;

use crate::compaction::compact_context;
use crate::models::list_models;
use crate::request::turn_request_to_responses_api_request;
use crate::response::response_stream_to_turn_stream;
use crate::sse::spawn_response_stream;
use crate::types::ResponsesApiRequest;

/// OpenAI Responses API wire-protocol adapter.
pub struct OpenAiWireProtocol {
    client: Client,
    base_url: String,
    idle_timeout: std::time::Duration,
}

impl OpenAiWireProtocol {
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
            idle_timeout: std::time::Duration::from_secs(300),
        }
    }

    pub fn with_client(mut self, client: Client) -> Self {
        self.client = client;
        self
    }

    pub fn with_idle_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.idle_timeout = timeout;
        self
    }

    fn api_key(auth: &AuthToken) -> Result<String, WireError> {
        match auth {
            AuthToken::Bearer(token) => Ok(token.clone()),
            AuthToken::ApiKey { key, .. } => Ok(key.clone()),
            _ => Err(WireError::Auth("Unsupported auth type for OpenAI".into())),
        }
    }
}

impl WireProtocol for OpenAiWireProtocol {
    fn protocol_id(&self) -> &str {
        "openai-responses"
    }

    fn protocol_name(&self) -> &str {
        "OpenAI Responses API"
    }

    fn stream_turn(
        &self,
        request: TurnRequest,
        auth: AuthToken,
    ) -> impl Future<Output = Result<TurnStream, WireError>> + Send {
        let client = self.client.clone();
        let base_url = self.base_url.clone();
        let idle_timeout = self.idle_timeout;
        async move {
            let api_key = Self::api_key(&auth)?;
            let openai_request: ResponsesApiRequest =
                turn_request_to_responses_api_request(request);

            let body = serde_json::to_value(&openai_request)
                .map_err(WireError::Serialization)?;

            let url = format!("{base_url}/responses");
            let mut req = client
                .post(&url)
                .header("Authorization", format!("Bearer {api_key}"))
                .header("Accept", "text/event-stream")
                .json(&body);

            // Inject OpenAI-specific headers only for OpenAI protocol requests.
            req = req.header("OpenAI-Beta", "responses=v1");

            let resp = req.send().await.map_err(WireError::Http)?;
            let headers = resp.headers().clone();
            let bytes = resp.bytes_stream().map(|r| {
                r.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
            });
            let byte_stream: crate::sse::ByteStream = Box::pin(bytes);

            let openai_stream = spawn_response_stream(
                byte_stream,
                &headers,
                idle_timeout,
                None,
                None,
            );

            Ok(response_stream_to_turn_stream(openai_stream, String::new()))
        }
    }

    fn list_models(
        &self,
        auth: AuthToken,
    ) -> impl Future<Output = Result<Vec<ModelInfo>, WireError>> + Send {
        let client = self.client.clone();
        let base_url = self.base_url.clone();
        async move {
            let api_key = Self::api_key(&auth)?;
            let (models, _etag) = list_models(
                &client,
                &base_url,
                &api_key,
                "0.0.0",
                HeaderMap::new(),
            )
            .await?;
            Ok(models)
        }
    }

    fn compact_context(
        &self,
        request: CompactRequest,
        auth: AuthToken,
    ) -> impl Future<Output = Result<CompactResponse, WireError>> + Send {
        let client = self.client.clone();
        let base_url = self.base_url.clone();
        async move {
            let api_key = Self::api_key(&auth)?;
            compact_context(
                &client,
                &base_url,
                &api_key,
                request,
                HeaderMap::new(),
            )
            .await
        }
    }

    fn capabilities(&self) -> ProtocolCapabilities {
        ProtocolCapabilities {
            supports_streaming: true,
            supports_tools: true,
            supports_vision: true,
            supports_reasoning: true,
            supports_realtime: false,
            supports_compaction: true,
            supports_websockets: true,
            supports_parallel_tool_calls: true,
        }
    }
}
