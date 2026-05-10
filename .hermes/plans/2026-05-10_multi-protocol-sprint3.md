# Multi-Protocol Provider Support — Sprint 3 Implementation Plan

> **For Hermes:** Use subagent-driven-development skill to implement this plan task-by-task.

**Goal:** Implement `codex-wire-protocol-anthropic` and `codex-wire-protocol-gemini` crates, wire them into the protocol dispatch layer, and add integration tests verifying non-OpenAI providers work end-to-end.

**Architecture:**
- Each new crate implements the `WireProtocol` trait for its native API.
- `codex-api` dispatches to the correct adapter via `create_protocol()` based on `protocol_id`.
- `codex-core` uses the abstracted `ModelClient` which already works with any `WireProtocol`.
- Integration tests use mock servers (wiremock) to simulate Anthropic/Gemini responses.

**Tech Stack:** Rust 2024, tokio, reqwest, serde, serde_json, thiserror, futures, bytes, uuid, wiremock (for tests).

---

## Current State (post-Sprint 2 + bugfixes)

From Sprints 1-2 we have:
- `codex-wire-protocol` with `WireProtocol` trait, `TurnRequest`, `TurnEvent`, `AuthToken`
- `codex-wire-protocol-openai` implementing the trait for OpenAI Responses API
- `codex-api` dispatching to `create_protocol()` which currently only handles `openai-*` protocols
- `codex-core` `ModelClient` using `Arc<dyn WireProtocol>`
- `codex-auth` with `ApiKeyAuthProvider` (works for Anthropic, Gemini, and most other providers)
- 47 provider presets defined with correct `protocol_id` values

What we need to add:
- `codex-wire-protocol-anthropic` — Anthropic Messages API v1
- `codex-wire-protocol-gemini` — Google Gemini API v1beta
- Update `codex-api::create_protocol()` to dispatch to new adapters
- Integration tests for both protocols

---

## Design Decisions

### 1. Anthropic Messages API Protocol

**API:** `POST https://api.anthropic.com/v1/messages` with SSE streaming.

**Request format:**
```json
{
  "model": "claude-sonnet-4-20250514",
  "max_tokens": 4096,
  "system": "You are a helpful assistant.",
  "messages": [
    {"role": "user", "content": "Hello"}
  ],
  "tools": [...],
  "stream": true
}
```

**Response format (SSE):**
```
event: message_start
data: {"type": "message_start", "message": {"id": "msg_...", "type": "message", "role": "assistant", "model": "...", "content": [], "stop_reason": null, "stop_sequence": null, "usage": {"input_tokens": 10, "output_tokens": 0}}}

event: content_block_start
data: {"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}}

event: content_block_delta
data: {"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "Hello"}}

event: content_block_stop
data: {"type": "content_block_stop", "index": 0}

event: message_delta
data: {"type": "message_delta", "delta": {"stop_reason": "end_turn", "stop_sequence": null}, "usage": {"output_tokens": 5}}

event: message_stop
data: {"type": "message_stop"}
```

**Tool calls:** Anthropic uses `tool_use` content blocks instead of function calls.

### 2. Gemini API Protocol

**API:** `POST https://generativelanguage.googleapis.com/v1beta/models/{model}:streamGenerateContent` with SSE streaming.

**Request format:**
```json
{
  "contents": [
    {"role": "user", "parts": [{"text": "Hello"}]}
  ],
  "systemInstruction": {"parts": [{"text": "You are helpful."}]},
  "tools": [...],
  "generationConfig": {"maxOutputTokens": 4096}
}
```

**Response format (SSE):**
```
data: {"candidates": [{"content": {"role": "model", "parts": [{"text": "Hello"}]}, "finishReason": "STOP"}]}
```

**Tool calls:** Gemini uses `functionCall` parts in the response.

### 3. Protocol Dispatch

Update `codex-api/src/lib.rs`:
```rust
pub fn create_protocol(
    protocol_id: &str,
    http_client: CodexHttpClient,
    api_provider: ApiProvider,
    auth_token: Option<AuthToken>,
) -> Result<Arc<dyn WireProtocol>, ApiError> {
    match protocol_id {
        "openai-responses" | "openai-chat-completions" => {
            Ok(Arc::new(OpenAiWireProtocol::new(http_client, api_provider, auth_token)))
        }
        "anthropic-messages" => {
            Ok(Arc::new(AnthropicWireProtocol::new(http_client, api_provider, auth_token)))
        }
        "gemini" => {
            Ok(Arc::new(GeminiWireProtocol::new(http_client, api_provider, auth_token)))
        }
        _ => Err(ApiError::UnsupportedProtocol(protocol_id.to_string())),
    }
}
```

---

## Task Breakdown

### Task 1: Create `codex-wire-protocol-anthropic` Crate

**Objective:** Implement `WireProtocol` for Anthropic Messages API.

**Files:**
- Create: `codex-rs/codex-wire-protocol-anthropic/Cargo.toml`
- Create: `codex-rs/codex-wire-protocol-anthropic/src/lib.rs`
- Create: `codex-rs/codex-wire-protocol-anthropic/src/protocol.rs`
- Create: `codex-rs/codex-wire-protocol-anthropic/src/sse.rs`
- Create: `codex-rs/codex-wire-protocol-anthropic/src/types.rs`
- Create: `codex-rs/codex-wire-protocol-anthropic/src/request.rs`
- Create: `codex-rs/codex-wire-protocol-anthropic/src/response.rs`
- Create: `codex-rs/codex-wire-protocol-anthropic/src/error.rs`
- Create: `codex-rs/codex-wire-protocol-anthropic/src/models.rs`

**Step 1: Cargo.toml**

```toml
[package]
name = "codex-wire-protocol-anthropic"
version.workspace = true
edition.workspace = true
license.workspace = true

[lints]
workspace = true

[dependencies]
anyhow = { workspace = true }
bytes = { workspace = true }
codex-wire-protocol = { workspace = true }
futures = { workspace = true }
reqwest = { workspace = true, features = ["json", "stream", "rustls-tls"] }
serde = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }
thiserror = { workspace = true }
tokio = { workspace = true, features = ["sync"] }
tracing = { workspace = true }
uuid = { workspace = true, features = ["v4"] }

[dev-dependencies]
wiremock = { workspace = true }
tokio = { workspace = true, features = ["macros", "rt-multi-thread"] }
```

**Step 2: types.rs**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct MessagesRequest {
    pub model: String,
    pub max_tokens: u32,
    pub system: Option<String>,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    pub stream: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Message {
    pub role: String,
    pub content: Vec<Content>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum Content {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { source: ImageSource },
}

#[derive(Debug, Clone, Serialize)]
pub struct ImageSource {
    pub type_: String,
    pub media_type: String,
    pub data: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessagesResponse {
    pub id: String,
    pub type_: String,
    pub role: String,
    pub content: Vec<ResponseContent>,
    pub stop_reason: Option<String>,
    pub usage: Usage,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ResponseContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse { id: String, name: String, input: serde_json::Value },
}

#[derive(Debug, Clone, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}
```

**Step 3: sse.rs**

```rust
use bytes::Bytes;
use futures::Stream;
use serde_json;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct AnthropicSseStream {
    inner: Pin<Box<dyn Stream<Item = Result<Bytes, reqwest::Error>> + Send>>,
    buffer: String,
}

impl AnthropicSseStream {
    pub fn new(inner: Pin<Box<dyn Stream<Item = Result<Bytes, reqwest::Error>> + Send>>) -> Self {
        Self { inner, buffer: String::new() }
    }
}

impl Stream for AnthropicSseStream {
    type Item = Result<SseEvent, AnthropicError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Parse SSE events: event: XXX\ndata: {...}\n\n
        // ... implementation similar to codex-wire-protocol-openai/sse.rs
        todo!()
    }
}

#[derive(Debug, Clone)]
pub enum SseEvent {
    MessageStart { id: String },
    ContentBlockDelta { index: usize, text: String },
    ContentBlockStop { index: usize },
    MessageDelta { stop_reason: Option<String>, usage: Option<Usage> },
    MessageStop,
    Ping,
}
```

**Step 4: protocol.rs**

```rust
use codex_wire_protocol::protocol::WireProtocol;
use codex_wire_protocol::request::{TurnRequest, TurnItem, ToolSpec};
use codex_wire_protocol::response::{TurnStream, TurnEvent, ModelInfo};
use codex_wire_protocol::auth::AuthToken;
use codex_wire_protocol::capabilities::ProtocolCapabilities;
use codex_wire_protocol::error::WireError;
use std::pin::Pin;
use std::future::Future;

pub struct AnthropicWireProtocol {
    http_client: CodexHttpClient,
    api_provider: ApiProvider,
    auth_token: Option<AuthToken>,
}

impl AnthropicWireProtocol {
    pub fn new(
        http_client: CodexHttpClient,
        api_provider: ApiProvider,
        auth_token: Option<AuthToken>,
    ) -> Self {
        Self { http_client, api_provider, auth_token }
    }
}

impl WireProtocol for AnthropicWireProtocol {
    fn protocol_id(&self) -> &str {
        "anthropic-messages"
    }

    fn stream_turn(
        &self,
        request: TurnRequest,
    ) -> Pin<Box<dyn Future<Output = Result<TurnStream, WireError>> + Send>> {
        let client = self.http_client.clone();
        let provider = self.api_provider.clone();
        let auth = self.auth_token.clone();
        Box::pin(async move {
            let anthropic_request = turn_request_to_messages_request(request);
            let url = format!("{}/messages", provider.base_url);
            let mut req = client.post(&url).json(&anthropic_request);
            if let Some(auth) = auth {
                req = req.header("x-api-key", auth.api_key()?);
                req = req.header("anthropic-version", "2023-06-01");
            }
            let response = req.send().await.map_err(|e| WireError::Http(e.to_string()))?;
            let sse_stream = AnthropicSseStream::new(Box::pin(response.bytes_stream()));
            let (tx, rx) = tokio::sync::mpsc::channel(100);
            tokio::spawn(async move {
                // Process SSE events and send TurnEvents
                // ...
            });
            Ok(TurnStream::new(rx, format!("turn-{}", uuid::Uuid::new_v4())))
        })
    }

    fn list_models(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<ModelInfo>, WireError>> + Send>> {
        // Anthropic doesn't have a /models endpoint, return bundled models
        Box::pin(async {
            Ok(vec![
                ModelInfo {
                    id: "claude-sonnet-4-20250514".to_string(),
                    display_name: "Claude Sonnet 4".to_string(),
                    context_window: 200000,
                    supports_tools: true,
                    supports_vision: true,
                    supports_reasoning: true,
                    supports_realtime: false,
                    max_output_tokens: 8192,
                },
                // ... more models
            ])
        })
    }

    fn supports_realtime(&self) -> bool {
        false
    }

    fn supports_compaction(&self) -> bool {
        false
    }
}
```

**Step 5: Verify compilation**

```bash
cargo check -p codex-wire-protocol-anthropic
```

**Step 6: Commit**

```bash
git add codex-rs/codex-wire-protocol-anthropic/
git commit -m "feat(anthropic): add codex-wire-protocol-anthropic crate"
```

---

### Task 2: Create `codex-wire-protocol-gemini` Crate

**Objective:** Implement `WireProtocol` for Google Gemini API.

**Files:**
- Create: `codex-rs/codex-wire-protocol-gemini/Cargo.toml`
- Create: `codex-rs/codex-wire-protocol-gemini/src/lib.rs`
- Create: `codex-rs/codex-wire-protocol-gemini/src/protocol.rs`
- Create: `codex-rs/codex-wire-protocol-gemini/src/sse.rs`
- Create: `codex-rs/codex-wire-protocol-gemini/src/types.rs`
- Create: `codex-rs/codex-wire-protocol-gemini/src/request.rs`
- Create: `codex-rs/codex-wire-protocol-gemini/src/response.rs`
- Create: `codex-rs/codex-wire-protocol-gemini/src/error.rs`

**Step 1: Cargo.toml**

```toml
[package]
name = "codex-wire-protocol-gemini"
version.workspace = true
edition.workspace = true
license.workspace = true

[lints]
workspace = true

[dependencies]
anyhow = { workspace = true }
bytes = { workspace = true }
codex-wire-protocol = { workspace = true }
futures = { workspace = true }
reqwest = { workspace = true, features = ["json", "stream", "rustls-tls"] }
serde = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }
thiserror = { workspace = true }
tokio = { workspace = true, features = ["sync"] }
tracing = { workspace = true }
uuid = { workspace = true, features = ["v4"] }

[dev-dependencies]
wiremock = { workspace = true }
tokio = { workspace = true, features = ["macros", "rt-multi-thread"] }
```

**Step 2: types.rs**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct GenerateContentRequest {
    pub contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub systemInstruction: Option<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generationConfig: Option<GenerationConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    pub role: String,
    pub parts: Vec<Part>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Part {
    Text { text: String },
    InlineData { mimeType: String, data: String },
    FunctionCall { name: String, args: serde_json::Value },
    FunctionResponse { name: String, response: serde_json::Value },
}

#[derive(Debug, Clone, Serialize)]
pub struct Tool {
    pub functionDeclarations: Vec<FunctionDeclaration>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FunctionDeclaration {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct GenerationConfig {
    pub maxOutputTokens: u32,
    pub temperature: Option<f32>,
    pub topP: Option<f32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GenerateContentResponse {
    pub candidates: Vec<Candidate>,
    pub usageMetadata: Option<UsageMetadata>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Candidate {
    pub content: Content,
    pub finishReason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UsageMetadata {
    pub promptTokenCount: u32,
    pub candidatesTokenCount: u32,
}
```

**Step 3: protocol.rs**

```rust
use codex_wire_protocol::protocol::WireProtocol;
use codex_wire_protocol::request::TurnRequest;
use codex_wire_protocol::response::{TurnStream, TurnEvent, ModelInfo};
use codex_wire_protocol::auth::AuthToken;
use codex_wire_protocol::error::WireError;
use std::pin::Pin;
use std::future::Future;

pub struct GeminiWireProtocol {
    http_client: CodexHttpClient,
    api_provider: ApiProvider,
    auth_token: Option<AuthToken>,
}

impl GeminiWireProtocol {
    pub fn new(
        http_client: CodexHttpClient,
        api_provider: ApiProvider,
        auth_token: Option<AuthToken>,
    ) -> Self {
        Self { http_client, api_provider, auth_token }
    }
}

impl WireProtocol for GeminiWireProtocol {
    fn protocol_id(&self) -> &str {
        "gemini"
    }

    fn stream_turn(
        &self,
        request: TurnRequest,
    ) -> Pin<Box<dyn Future<Output = Result<TurnStream, WireError>> + Send>> {
        let client = self.http_client.clone();
        let provider = self.api_provider.clone();
        let auth = self.auth_token.clone();
        Box::pin(async move {
            let gemini_request = turn_request_to_gemini_request(request);
            let model = gemini_request.model.clone();
            let url = format!("{}/models/{}:streamGenerateContent", provider.base_url, model);
            let mut req = client.post(&url).json(&gemini_request);
            if let Some(auth) = auth {
                // Gemini uses API key as query param
                req = req.query(&[("key", auth.api_key()?)])
            }
            let response = req.send().await.map_err(|e| WireError::Http(e.to_string()))?;
            let sse_stream = GeminiSseStream::new(Box::pin(response.bytes_stream()));
            let (tx, rx) = tokio::sync::mpsc::channel(100);
            tokio::spawn(async move {
                // Process SSE events and send TurnEvents
                // ...
            });
            Ok(TurnStream::new(rx, format!("turn-{}", uuid::Uuid::new_v4())))
        })
    }

    fn list_models(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<ModelInfo>, WireError>> + Send>> {
        Box::pin(async {
            Ok(vec![
                ModelInfo {
                    id: "gemini-2.5-pro".to_string(),
                    display_name: "Gemini 2.5 Pro".to_string(),
                    context_window: 1000000,
                    supports_tools: true,
                    supports_vision: true,
                    supports_reasoning: true,
                    supports_realtime: false,
                    max_output_tokens: 8192,
                },
            ])
        })
    }

    fn supports_realtime(&self) -> bool {
        false
    }

    fn supports_compaction(&self) -> bool {
        false
    }
}
```

**Step 4: Verify compilation**

```bash
cargo check -p codex-wire-protocol-gemini
```

**Step 5: Commit**

```bash
git add codex-rs/codex-wire-protocol-gemini/
git commit -m "feat(gemini): add codex-wire-protocol-gemini crate"
```

---

### Task 3: Update `codex-api` Protocol Dispatch

**Objective:** Wire new protocol adapters into `create_protocol()`.

**Files:**
- Modify: `codex-rs/codex-api/src/lib.rs`

**Step 1: Add dependencies**

```toml
# In codex-api/Cargo.toml
codex-wire-protocol-anthropic = { workspace = true }
codex-wire-protocol-gemini = { workspace = true }
```

**Step 2: Update create_protocol()**

```rust
use codex_wire_protocol_openai::OpenAiWireProtocol;
use codex_wire_protocol_anthropic::AnthropicWireProtocol;
use codex_wire_protocol_gemini::GeminiWireProtocol;

pub fn create_protocol(
    protocol_id: &str,
    http_client: CodexHttpClient,
    api_provider: ApiProvider,
    auth_token: Option<AuthToken>,
) -> Result<Arc<dyn WireProtocol>, ApiError> {
    match protocol_id {
        "openai-responses" | "openai-chat-completions" => {
            Ok(Arc::new(OpenAiWireProtocol::new(http_client, api_provider, auth_token)))
        }
        "anthropic-messages" => {
            Ok(Arc::new(AnthropicWireProtocol::new(http_client, api_provider, auth_token)))
        }
        "gemini" => {
            Ok(Arc::new(GeminiWireProtocol::new(http_client, api_provider, auth_token)))
        }
        _ => Err(ApiError::UnsupportedProtocol(protocol_id.to_string())),
    }
}
```

**Step 3: Add UnsupportedProtocol error variant**

```rust
// In codex-api/src/error.rs
#[derive(Error, Debug)]
pub enum ApiError {
    // ... existing variants
    #[error("Unsupported protocol: {0}")]
    UnsupportedProtocol(String),
}
```

**Step 4: Verify compilation**

```bash
cargo check -p codex-api
```

**Step 5: Commit**

```bash
git add codex-rs/codex-api/
git commit -m "feat(api): dispatch to anthropic and gemini protocol adapters"
```

---

### Task 4: Add Integration Tests for Anthropic Protocol

**Objective:** Test the Anthropic adapter against a mock server.

**Files:**
- Create: `codex-rs/codex-wire-protocol-anthropic/src/protocol_tests.rs`

**Step 1: Test streaming turn**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path, header};
    use codex_wire_protocol::request::{TurnRequest, TurnItem};
    use codex_wire_protocol::auth::AuthToken;

    #[tokio::test]
    async fn test_stream_turn_returns_content_delta() {
        let server = MockServer::start().await;
        
        let sse_body = r#"
event: message_start
data: {"type": "message_start", "message": {"id": "msg_123", "type": "message", "role": "assistant", "model": "claude-test", "content": [], "stop_reason": null, "stop_sequence": null, "usage": {"input_tokens": 10, "output_tokens": 0}}}

event: content_block_start
data: {"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}}

event: content_block_delta
data: {"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "Hello"}}

event: content_block_stop
data: {"type": "content_block_stop", "index": 0}

event: message_delta
data: {"type": "message_delta", "delta": {"stop_reason": "end_turn", "stop_sequence": null}, "usage": {"output_tokens": 1}}

event: message_stop
data: {"type": "message_stop"}
"#;

        Mock::given(method("POST"))
            .and(path("/messages"))
            .and(header("x-api-key", "test-key"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_string(sse_body)
                .insert_header("content-type", "text/event-stream"))
            .mount(&server)
            .await;

        let protocol = AnthropicWireProtocol::new(
            CodexHttpClient::new(),
            ApiProvider { base_url: server.uri(), ... },
            Some(AuthToken::ApiKey { key: "test-key".to_string(), header_name: "x-api-key".to_string() }),
        );

        let request = TurnRequest {
            model: "claude-test".to_string(),
            instructions: "You are helpful.".to_string(),
            input: vec![TurnItem::Text { text: "Hello".to_string() }],
            tools: vec![],
            tool_choice: ToolChoice::Auto,
            reasoning: None,
            stream: true,
        };

        let mut stream = protocol.stream_turn(request).await.unwrap();
        
        // Collect events
        let mut events = vec![];
        while let Some(event) = stream.rx.recv().await {
            events.push(event);
        }

        // Verify we got content delta
        assert!(events.iter().any(|e| matches!(e, TurnEvent::ContentDelta { text } if text == "Hello")));
        assert!(events.iter().any(|e| matches!(e, TurnEvent::Completed { .. })));
    }
}
```

**Step 2: Verify tests pass**

```bash
cargo test -p codex-wire-protocol-anthropic
```

**Step 3: Commit**

```bash
git add codex-rs/codex-wire-protocol-anthropic/
git commit -m "test(anthropic): add integration tests for streaming turn"
```

---

### Task 5: Add Integration Tests for Gemini Protocol

**Objective:** Test the Gemini adapter against a mock server.

**Files:**
- Create: `codex-rs/codex-wire-protocol-gemini/src/protocol_tests.rs`

**Step 1: Test streaming turn**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path, query_param};

    #[tokio::test]
    async fn test_stream_turn_returns_content_delta() {
        let server = MockServer::start().await;
        
        let sse_body = r#"
data: {"candidates": [{"content": {"role": "model", "parts": [{"text": "Hello"}]}, "finishReason": "STOP"}]}
"#;

        Mock::given(method("POST"))
            .and(path("/models/gemini-test:streamGenerateContent"))
            .and(query_param("key", "test-key"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_string(sse_body)
                .insert_header("content-type", "text/event-stream"))
            .mount(&server)
            .await;

        let protocol = GeminiWireProtocol::new(
            CodexHttpClient::new(),
            ApiProvider { base_url: server.uri(), ... },
            Some(AuthToken::ApiKey { key: "test-key".to_string(), header_name: "Authorization".to_string() }),
        );

        let request = TurnRequest {
            model: "gemini-test".to_string(),
            instructions: "You are helpful.".to_string(),
            input: vec![TurnItem::Text { text: "Hello".to_string() }],
            tools: vec![],
            tool_choice: ToolChoice::Auto,
            reasoning: None,
            stream: true,
        };

        let mut stream = protocol.stream_turn(request).await.unwrap();
        
        let mut events = vec![];
        while let Some(event) = stream.rx.recv().await {
            events.push(event);
        }

        assert!(events.iter().any(|e| matches!(e, TurnEvent::ContentDelta { text } if text == "Hello")));
        assert!(events.iter().any(|e| matches!(e, TurnEvent::Completed { .. })));
    }
}
```

**Step 2: Verify tests pass**

```bash
cargo test -p codex-wire-protocol-gemini
```

**Step 3: Commit**

```bash
git add codex-rs/codex-wire-protocol-gemini/
git commit -m "test(gemini): add integration tests for streaming turn"
```

---

### Task 6: End-to-End Test with `codex-core`

**Objective:** Verify that `codex-core` can use Anthropic/Gemini providers through the full stack.

**Files:**
- Create: `codex-rs/core/src/anthropic_integration_tests.rs`
- Create: `codex-rs/core/src/gemini_integration_tests.rs`

**Step 1: Test ModelClient with Anthropic provider**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use codex_wire_protocol::auth::AuthToken;
    use codex_model_provider_info::ModelProviderInfo;

    #[tokio::test]
    async fn test_model_client_with_anthropic_provider() {
        let provider_info = ModelProviderInfo {
            id: "anthropic".to_string(),
            name: "Anthropic".to_string(),
            base_url: Some("https://api.anthropic.com/v1".to_string()),
            env_key: Some("ANTHROPIC_API_KEY".to_string()),
            protocol_id: "anthropic-messages".to_string(),
            // ... other fields
        };

        // Set up auth
        std::env::set_var("ANTHROPIC_API_KEY", "test-key");
        
        // Create ModelClient
        let auth_manager = AuthManager::new(...);
        let model_client = ModelClient::new(
            Some(Arc::new(auth_manager)),
            SessionId::new(),
            ThreadId::new(),
            "test-install".to_string(),
            provider_info,
            SessionSource::Cli,
            None,
            false,
            false,
            None,
            None,
        );

        // Verify the protocol is Anthropic, not OpenAI
        assert_eq!(model_client.protocol_id(), "anthropic-messages");
    }
}
```

**Step 2: Verify compilation**

```bash
cargo check -p codex-core
```

**Step 3: Commit**

```bash
git add codex-rs/core/
git commit -m "test(core): add e2e integration tests for anthropic and gemini providers"
```

---

### Task 7: Update Provider Presets to Enable New Protocols

**Objective:** Ensure the 47 presets correctly reference the new protocols.

**Files:**
- Verify: `codex-rs/model-provider-info/src/lib.rs`

**Step 1: Verify protocol_id assignments**

Already done in Sprint 2:
- `anthropic` → `"anthropic-messages"`
- `claude-aws` → `"anthropic-messages"`
- `claude-gcp` → `"anthropic-messages"`
- `gemini` → `"gemini"`
- `vertex-ai` → `"gemini"`

**Step 2: Verify `create_protocol()` handles all protocol_ids**

Ensure no preset has a `protocol_id` that `create_protocol()` doesn't handle.

**Step 3: Add test for all presets**

```rust
#[test]
fn test_all_provider_presets_have_valid_protocol_ids() {
    let presets = built_in_model_providers(None);
    for (id, info) in presets {
        assert!(
            matches!(info.protocol_id.as_str(),
                "openai-responses" | "openai-chat-completions" |
                "anthropic-messages" | "gemini" | "bedrock" |
                "replicate" | "baseten" | ""
            ),
            "Provider {} has unknown protocol_id: {}",
            id, info.protocol_id
        );
    }
}
```

**Step 4: Commit**

```bash
git add codex-rs/model-provider-info/
git commit -m "test(provider-info): validate all presets have known protocol_ids"
```

---

### Task 8: Full Workspace Verification

**Objective:** Ensure everything compiles and tests pass.

**Step 1: Check all crates**

```bash
cargo check --workspace
```

**Step 2: Run all tests**

```bash
RUST_MIN_STACK=16777216 cargo test -p codex-core --lib -- --test-threads=2
RUST_MIN_STACK=16777216 cargo test -p codex-tui --lib -- --test-threads=2
cargo test -p codex-api --lib
cargo test -p codex-wire-protocol-anthropic
cargo test -p codex-wire-protocol-gemini
cargo test -p codex-models-manager --lib
cargo test -p codex-model-provider-info
```

**Step 3: Format and fix**

```bash
cargo fmt
cargo fix --lib -p codex-wire-protocol-anthropic -p codex-wire-protocol-gemini --allow-dirty
```

**Step 4: Commit**

```bash
git add -A
git commit -m "feat(multi-protocol): Sprint 3 complete — anthropic and gemini protocol adapters"
```

---

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Anthropic SSE format changes | Low | High | Follow their API docs closely, add version header |
| Gemini API key as query param is insecure | Medium | Medium | Document this is provider behavior, not our choice |
| Tool call format mismatch between protocols | Medium | High | Map each protocol's tool format to generic `TurnEvent::ToolCall` |
| Integration tests flaky due to timing | Low | Medium | Use wiremock, not real APIs |
| Bazel build breaks with new crates | Medium | High | Add BUILD.bazel files for new crates |

## Verification Checklist

- [ ] `cargo check -p codex-wire-protocol-anthropic` passes
- [ ] `cargo check -p codex-wire-protocol-gemini` passes
- [ ] `cargo check -p codex-api` passes
- [ ] `cargo check -p codex-core` passes
- [ ] `cargo check --workspace` passes
- [ ] `cargo test -p codex-wire-protocol-anthropic` passes
- [ ] `cargo test -p codex-wire-protocol-gemini` passes
- [ ] `cargo test -p codex-core --lib` passes
- [ ] `cargo test -p codex-tui --lib` passes
- [ ] `cargo test -p codex-api --lib` passes
- [ ] All 47 presets have valid protocol_ids
- [ ] `create_protocol()` handles all protocol_ids
- [ ] `cargo fmt` run
- [ ] `just write-config-schema` run if config changed

---

*Plan written: 2026-05-10*
*Saved to: `.hermes/plans/2026-05-10_multi-protocol-sprint3.md`*
