# Multi-Protocol Provider Support — Sprint 1 Implementation Plan

> **For Hermes:** Use subagent-driven-development skill to implement this plan task-by-task.

**Goal:** Refactor the OpenAI Codex CLI to support arbitrary LLM providers via a pluggable wire-protocol architecture. Extract the hardcoded OpenAI Responses API into a swappable adapter, design the trait boundary, and refactor `codex-api` and `codex-core` to be protocol-agnostic.

**Architecture:**
- Introduce a new `codex-wire-protocol` crate defining a `WireProtocol` trait that abstracts turn streaming, model listing, context compaction, and capability queries.
- Extract all OpenAI-specific code from `codex-api` into `codex-wire-protocol-openai` (an adapter implementing the trait).
- Refactor `codex-api` to become a thin dispatch layer over `Arc<dyn WireProtocol>`.
- Refactor `codex-core` `ModelClient` / `ModelClientSession` to consume the trait instead of hardcoded `ResponsesClient`.
- Preserve 100% backward compatibility for existing OpenAI users; no behavioral change when `provider = "openai"`.

**Tech Stack:** Rust 2024, Cargo workspace, tokio async, serde, reqwest, tokio-tungstenite.

**Target Providers:** 47 total (see Provider Registry below). Sprint 1 focuses on the trait + OpenAI extraction only.

---

## Current State Summary

From the Phase 0 audit:

| Crate | Hardcoded OpenAI Elements |
|-------|--------------------------|
| `codex-api` | Endpoint URLs (`/v1/responses`, `/v1/realtime`), SSE event parsing (`response.created`, `response.output_item.done`), headers (`x-openai-subagent`, `openai-model`), request/response structs (`ResponsesApiRequest`), model names (`gpt-*`) |
| `codex-core` | `ModelClient` hardcodes `ResponsesClient`, `is_openai()` gating compression, `OpenAI-Beta` header, `x-openai-memgen-request`, `gpt-realtime-1.5` default, `chatgpt_base_url`, `mcp_openai_file.rs` module |
| `codex-login` | OAuth endpoints (`auth.openai.com`), `OPENAI_API_KEY` env var, JWT claim namespaces (`api.openai.com`), `ChatGPT-Account-ID` header |
| `models-manager` | Bundled `models.json` contains only OpenAI models, `OpenAiModelsManager` struct name, `BASE_INSTRUCTIONS` from OpenAI prompt |
| `model-provider-info` | `OPENAI_PROVIDER_NAME`, `create_openai_provider()`, `requires_openai_auth`, `OpenAI-Organization` env header |

---

## Design Decisions

### 1. Why a New Crate (`codex-wire-protocol`) Instead of Modifying `codex-api`?

The `codex-api` crate is currently ~33 files with deep OpenAI entanglement. Adding trait definitions there would create circular dependencies (`codex-api` depends on `codex-client` for HTTP, but protocol adapters also need HTTP). A separate crate at the bottom of the dependency graph avoids this.

Dependency graph after refactor:
```
codex-wire-protocol (trait definitions only, no deps)
  ↑
codex-wire-protocol-openai (depends on codex-wire-protocol + codex-client)
codex-wire-protocol-anthropic (future)
codex-wire-protocol-gemini (future)
  ↑
codex-api (depends on codex-wire-protocol, dispatches to adapters)
  ↑
codex-core (depends on codex-api, uses WireProtocol trait)
```

### 2. Trait Design Philosophy

The trait must be **provider-agnostic** but **not lowest-common-denominator**. We preserve Codex's rich features (tool calling, reasoning, streaming, compaction, realtime) as optional capabilities that adapters can implement or decline.

Key insight: OpenAI's Responses API is actually a superset of most other APIs. Anthropic Messages API and Gemini both support tools, streaming, and multi-turn. The mapping is mostly structural (different JSON shapes, different SSE event names).

### 3. Backward Compatibility

The existing `ConfigToml` `model_provider = "openai"` must continue to work unchanged. The new `protocol` field is additive. When absent, we infer it from the provider ID (e.g., `openai` → `openai-responses`, `anthropic` → `anthropic-messages`).

### 4. Auth Abstraction (Sprint 1 Scope)

Sprint 1 does NOT refactor auth. The `WireProtocol` trait receives an `AuthToken` enum (bearer, api-key, aws-sigv4) and injects it into requests. Auth acquisition (OAuth flows, env var reading) stays in `codex-login` for now. Sprint 2 will refactor auth to be per-provider.

---

## Provider Registry (47 Providers)

### Protocol: `openai-responses` (OpenAI native)
1. **openai** — `https://api.openai.com/v1` — OAuth2 / API key (`OPENAI_API_KEY`)
2. **azure-openai** — `https://{resource}.openai.azure.com/openai` — API key (`AZURE_OPENAI_API_KEY`)

### Protocol: `openai-chat-completions` (OpenAI-compatible)
3. **openrouter** — `https://openrouter.ai/api/v1` — API key (`OPENROUTER_API_KEY`)
4. **groq** — `https://api.groq.com/openai/v1` — API key (`GROQ_API_KEY`)
5. **together** — `https://api.together.xyz/v1` — API key (`TOGETHER_API_KEY`)
6. **fireworks** — `https://api.fireworks.ai/inference/v1` — API key (`FIREWORKS_API_KEY`)
7. **anyscale** — `https://api.endpoints.anyscale.com/v1` — API key (`ANYSCALE_API_KEY`)
8. **deepinfra** — `https://api.deepinfra.com/v1/openai` — API key (`DEEPINFRA_API_KEY`)
9. **perplexity** — `https://api.perplexity.ai` — API key (`PERPLEXITY_API_KEY`)
10. **mistral** — `https://api.mistral.ai/v1` — API key (`MISTRAL_API_KEY`)
11. **cohere** — `https://api.cohere.com/v1` — API key (`COHERE_API_KEY`)
12. **ai21** — `https://api.ai21.com/studio/v1` — API key (`AI21_API_KEY`)
13. **novita** — `https://api.novita.ai/v3/openai` — API key (`NOVITA_API_KEY`)
14. **siliconflow** — `https://api.siliconflow.cn/v1` — API key (`SILICONFLOW_API_KEY`)
15. **hyperbolic** — `https://api.hyperbolic.xyz/v1` — API key (`HYPERBOLIC_API_KEY`)
16. **lambda** — `https://api.lambdalabs.com/v1` — API key (`LAMBDA_API_KEY`)
17. **friendli** — `https://api.friendli.ai/v1` — API key (`FRIENDLI_API_KEY`)
18. **sambanova** — `https://api.sambanova.ai/v1` — API key (`SAMBANOVA_API_KEY`)
19. **nvidia** — `https://integrate.api.nvidia.com/v1` — API key (`NVIDIA_API_KEY`)
20. **localai** — `http://localhost:8080/v1` — None
21. **xai** — `https://api.x.ai/v1` — API key (`XAI_API_KEY`)
22. **deepseek** — `https://api.deepseek.com/v1` — API key (`DEEPSEEK_API_KEY`)
23. **moonshot** — `https://api.moonshot.cn/v1` — API key (`MOONSHOT_API_KEY`)
24. **qwen** — `https://dashscope.aliyuncs.com/compatible-mode/v1` — API key (`DASHSCOPE_API_KEY`)
25. **minimax** — `https://api.minimax.chat/v1` — API key (`MINIMAX_API_KEY`)
26. **yi** — `https://api.01.ai/v1` — API key (`YI_API_KEY`)
27. **huggingface** — `https://api-inference.huggingface.co/v1` — API key (`HF_API_TOKEN`)
28. **cloudflare** — `https://api.cloudflare.com/client/v4/accounts/{account_id}/ai/v1` — API key (`CF_API_TOKEN`)
29. **predibase** — `https://serving.predibase.com/v1` — API key (`PREDIBASE_API_TOKEN`)
30. **octoai** — `https://text.octoai.run/v1` — API key (`OCTOAI_API_TOKEN`)
31. **lepton** — `https://{model}.lepton.run/api/v1` — API key (`LEPTON_API_TOKEN`)
32. **skywork** — `https://api.skywork.ai/v1` — API key (`SKYWORK_API_KEY`)
33. **zhipu** — `https://open.bigmodel.cn/api/paas/v4` — API key (`ZHIPU_API_KEY`)
34. **baichuan** — `https://api.baichuan-ai.com/v1` — API key (`BAICHUAN_API_KEY`)
35. **stepfun** — `https://api.stepfun.com/v1` — API key (`STEPFUN_API_KEY`)
36. **spark** — `https://spark-api-open.xf-yun.com/v1` — API key (`SPARK_API_KEY`)
37. **azure-ai** — `https://{resource}.services.ai.azure.com/models` — API key (`AZURE_AI_API_KEY`)
38. **ollama** — `http://localhost:11434/v1` — None
39. **lmstudio** — `http://localhost:1234/v1` — None

### Protocol: `anthropic-messages` (Anthropic native)
40. **anthropic** — `https://api.anthropic.com/v1` — API key (`ANTHROPIC_API_KEY`)
41. **claude-aws** — `https://bedrock-runtime.{region}.amazonaws.com` — AWS SigV4
42. **claude-gcp** — `https://{region}-aiplatform.googleapis.com/v1` — GCP auth

### Protocol: `gemini` (Google native)
43. **gemini** — `https://generativelanguage.googleapis.com/v1beta` — API key (`GOOGLE_API_KEY`)
44. **vertex-ai** — `https://{region}-aiplatform.googleapis.com/v1` — GCP auth

### Protocol: `bedrock` (Amazon native)
45. **bedrock** — `https://bedrock-runtime.{region}.amazonaws.com` — AWS SigV4

### Protocol: `replicate` (Replicate native)
46. **replicate** — `https://api.replicate.com/v1` — API key (`REPLICATE_API_TOKEN`)

### Protocol: `baseten` (Baseten native)
47. **baseten** — `https://model-{model_id}.api.baseten.co/production/predict` — API key (`BASETEN_API_KEY`)

---

## Task Breakdown

### Task 1: Create `codex-wire-protocol` Crate

**Objective:** Define the core trait and types that all protocol adapters will implement.

**Files:**
- Create: `codex-rs/codex-wire-protocol/Cargo.toml`
- Create: `codex-rs/codex-wire-protocol/src/lib.rs`
- Create: `codex-rs/codex-wire-protocol/src/protocol.rs` (trait definition)
- Create: `codex-rs/codex-wire-protocol/src/request.rs` (TurnRequest, ToolSpec, etc.)
- Create: `codex-rs/codex-wire-protocol/src/response.rs` (TurnEvent, TurnStream)
- Create: `codex-rs/codex-wire-protocol/src/auth.rs` (AuthToken enum)
- Create: `codex-rs/codex-wire-protocol/src/capabilities.rs` (ProtocolCapabilities)
- Create: `codex-rs/codex-wire-protocol/src/error.rs` (WireError)

**Step 1: Write Cargo.toml**

```toml
[package]
name = "codex-wire-protocol"
version.workspace = true
edition.workspace = true
license.workspace = true

[lints]
workspace = true

[dependencies]
anyhow = { workspace = true }
async-trait = { workspace = true }
bytes = { workspace = true }
futures = { workspace = true }
serde = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }
thiserror = { workspace = true }
tokio = { workspace = true, features = ["sync"] }
tracing = { workspace = true }
```

**Step 2: Write trait definition**

```rust
// codex-wire-protocol/src/protocol.rs

use crate::request::TurnRequest;
use crate::response::TurnStream;
use crate::auth::AuthToken;
use crate::capabilities::ProtocolCapabilities;
use crate::error::WireError;
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
```

**Step 3: Write request types**

```rust
// codex-wire-protocol/src/request.rs

use serde::{Deserialize, Serialize};

/// A generic turn request, protocol-agnostic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnRequest {
    pub model: String,
    pub instructions: String,
    pub input: Vec<TurnItem>,
    pub tools: Vec<ToolSpec>,
    pub tool_choice: ToolChoice,
    pub reasoning: Option<ReasoningConfig>,
    pub stream: bool,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub stop_sequences: Option<Vec<String>>,
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TurnItem {
    Message { role: Role, content: String },
    Image { mime_type: String, data: String }, // base64
    File { name: String, content: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value, // JSON Schema
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolChoice {
    Auto,
    Required,
    None,
    Named(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningConfig {
    pub effort: ReasoningEffort,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffort {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactRequest {
    pub model: String,
    pub context: Vec<TurnItem>,
    pub instructions: String,
}
```

**Step 4: Write response types**

```rust
// codex-wire-protocol/src/response.rs

use serde::{Deserialize, Serialize};

/// A stream of turn events from the provider.
pub struct TurnStream {
    pub events: tokio::sync::mpsc::Receiver<TurnEvent>,
    pub turn_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TurnEvent {
    Started { turn_id: String },
    ContentDelta { text: String },
    ReasoningDelta { text: String },
    ToolCallStarted { call_id: String, name: String },
    ToolCallArgumentsDelta { call_id: String, arguments: String },
    ToolCallCompleted { call_id: String, name: String, arguments: String },
    ToolResult { call_id: String, output: String },
    Completed,
    Failed { error: String, code: Option<String> },
    Usage { input_tokens: u32, output_tokens: u32, reasoning_tokens: Option<u32> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub display_name: String,
    pub context_window: u64,
    pub supports_tools: bool,
    pub supports_vision: bool,
    pub supports_reasoning: bool,
    pub supports_realtime: bool,
    pub max_output_tokens: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactResponse {
    pub summary: String,
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}
```

**Step 5: Write auth token enum**

```rust
// codex-wire-protocol/src/auth.rs

#[derive(Debug, Clone)]
pub enum AuthToken {
    Bearer(String),
    ApiKey { key: String, header_name: String },
    AwsSigV4 {
        access_key: String,
        secret_key: String,
        session_token: Option<String>,
        region: String,
        service: String,
    },
    GcpAuth {
        token: String,
    },
    None,
}
```

**Step 6: Write capabilities**

```rust
// codex-wire-protocol/src/capabilities.rs

#[derive(Debug, Clone, Copy, Default)]
pub struct ProtocolCapabilities {
    pub supports_streaming: bool,
    pub supports_tools: bool,
    pub supports_vision: bool,
    pub supports_reasoning: bool,
    pub supports_realtime: bool,
    pub supports_compaction: bool,
    pub supports_websockets: bool,
    pub supports_parallel_tool_calls: bool,
}
```

**Step 7: Write error type**

```rust
// codex-wire-protocol/src/error.rs

use thiserror::Error;

#[derive(Error, Debug)]
pub enum WireError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Stream error: {0}")]
    Stream(String),
    #[error("Unsupported operation: {0}")]
    Unsupported(String),
    #[error("Authentication error: {0}")]
    Auth(String),
    #[error("Rate limited: {0}")]
    RateLimited(String),
    #[error("Provider error ({code}): {message}")]
    Provider { code: String, message: String },
    #[error("Other: {0}")]
    Other(String),
}
```

**Step 8: Write lib.rs**

```rust
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
pub use request::{CompactRequest, ReasoningConfig, ReasoningEffort, Role, ToolChoice, ToolSpec, TurnItem, TurnRequest};
pub use response::{CompactResponse, ModelInfo, TurnEvent, TurnStream, Usage};
```

**Step 9: Add to workspace Cargo.toml**

Add `codex-wire-protocol` to the `members` array in `codex-rs/Cargo.toml`.

Add workspace dependency:
```toml
codex-wire-protocol = { path = "codex-wire-protocol" }
```

**Step 10: Verify compilation**

```bash
cd /Users/user/Documents/Projects/opencodex/codex/codex-rs
cargo check -p codex-wire-protocol
```

Expected: Clean compilation (no errors, no warnings).

**Step 11: Commit**

```bash
git add codex-rs/codex-wire-protocol/
git add codex-rs/Cargo.toml
git commit -m "feat(wire-protocol): add codex-wire-protocol crate with WireProtocol trait"
```

---

### Task 2: Create `codex-wire-protocol-openai` Crate (Extract OpenAI Adapter)

**Objective:** Move all OpenAI-specific code from `codex-api` into a new adapter crate that implements `WireProtocol`.

**Files:**
- Create: `codex-rs/codex-wire-protocol-openai/Cargo.toml`
- Create: `codex-rs/codex-wire-protocol-openai/src/lib.rs`
- Create: `codex-rs/codex-wire-protocol-openai/src/protocol.rs` (OpenAiWireProtocol impl)
- Create: `codex-rs/codex-wire-protocol-openai/src/request.rs` (TurnRequest -> ResponsesApiRequest mapping)
- Create: `codex-rs/codex-wire-protocol-openai/src/response.rs` (SSE -> TurnEvent mapping)
- Create: `codex-rs/codex-wire-protocol-openai/src/sse.rs` (SSE event parsing, extracted from codex-api/sse/responses.rs)
- Create: `codex-rs/codex-wire-protocol-openai/src/websocket.rs` (WebSocket handling, extracted from codex-api)
- Create: `codex-rs/codex-wire-protocol-openai/src/types.rs` (OpenAI-specific request/response structs)
- Create: `codex-rs/codex-wire-protocol-openai/src/models.rs` (model listing)
- Create: `codex-rs/codex-wire-protocol-openai/src/compaction.rs` (context compaction)
- Modify: `codex-rs/codex-api/src/lib.rs` (remove OpenAI-specific exports, add generic dispatch)
- Modify: `codex-rs/codex-api/src/endpoint/responses.rs` (deprecate, redirect to adapter)
- Modify: `codex-rs/codex-api/src/sse/responses.rs` (deprecate, redirect to adapter)

**Step 1: Write Cargo.toml**

```toml
[package]
name = "codex-wire-protocol-openai"
version.workspace = true
edition.workspace = true
license.workspace = true

[lints]
workspace = true

[dependencies]
anyhow = { workspace = true }
async-channel = { workspace = true }
async-trait = { workspace = true }
base64 = { workspace = true }
bytes = { workspace = true }
chrono = { workspace = true }
codex-client = { workspace = true }
codex-wire-protocol = { workspace = true }
eventsource-stream = { workspace = true }
futures = { workspace = true }
http = { workspace = true }
reqwest = { workspace = true, features = ["json", "stream"] }
serde = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }
thiserror = { workspace = true }
tokio = { workspace = true, features = ["fs", "macros", "net", "rt", "sync", "time"] }
tokio-tungstenite = { workspace = true }
tungstenite = { workspace = true }
tracing = { workspace = true }
```

**Step 2: Extract SSE parsing**

Read `codex-rs/codex-api/src/sse/responses.rs` and copy it into `codex-wire-protocol-openai/src/sse.rs`. Then modify to:
- Parse OpenAI SSE events (`response.created`, `response.output_item.done`, etc.)
- Map each event to `codex_wire_protocol::TurnEvent`
- Remove any hardcoded `gpt-*` model references from production code (keep in tests)

Key mapping:
```rust
// OpenAI SSE -> generic TurnEvent
fn map_sse_event(event: OpenAiSseEvent) -> Option<TurnEvent> {
    match event {
        OpenAiSseEvent::ResponseCreated { id } => Some(TurnEvent::Started { turn_id: id }),
        OpenAiSseEvent::OutputTextDelta { delta } => Some(TurnEvent::ContentDelta { text: delta }),
        OpenAiSseEvent::FunctionCallArgumentsDelta { call_id, delta } => {
            Some(TurnEvent::ToolCallArgumentsDelta { call_id, arguments: delta })
        }
        OpenAiSseEvent::OutputItemDone { item } => {
            match item {
                OutputItem::FunctionCall { call_id, name, arguments } => {
                    Some(TurnEvent::ToolCallCompleted { call_id, name, arguments })
                }
                _ => None,
            }
        }
        OpenAiSseEvent::Completed => Some(TurnEvent::Completed),
        OpenAiSseEvent::Failed { error } => {
            Some(TurnEvent::Failed { error: error.message, code: error.code })
        }
        _ => None,
    }
}
```

**Step 3: Extract request/response types**

Read `codex-rs/codex-api/src/common.rs` and copy OpenAI-specific structs into `codex-wire-protocol-openai/src/types.rs`:
- `ResponsesApiRequest`
- `ResponsesApiResponse`
- `ResponseEvent` (SSE event types)
- `ResponseItem`
- `FunctionCallOutput`
- `ToolChoice` (OpenAI-specific)
- `Reasoning` (OpenAI-specific)

These stay as OpenAI-native types. The adapter maps to/from generic `TurnRequest`/`TurnEvent`.

**Step 4: Implement WireProtocol for OpenAI**

```rust
// codex-wire-protocol-openai/src/protocol.rs

use codex_wire_protocol::{WireProtocol, TurnRequest, TurnStream, AuthToken, WireError, ProtocolCapabilities};
use codex_client::CodexHttpClient;

pub struct OpenAiWireProtocol {
    client: CodexHttpClient,
    base_url: String,
}

impl OpenAiWireProtocol {
    pub fn new(client: CodexHttpClient, base_url: String) -> Self {
        Self { client, base_url }
    }
}

impl WireProtocol for OpenAiWireProtocol {
    fn protocol_id(&self) -> &str {
        "openai-responses"
    }

    fn protocol_name(&self) -> &str {
        "OpenAI Responses API"
    }

    async fn stream_turn(&self, request: TurnRequest, auth: AuthToken) -> Result<TurnStream, WireError> {
        // 1. Map generic TurnRequest -> ResponsesApiRequest
        let openai_request = self.map_request(request)?;
        
        // 2. Build HTTP request with auth
        let mut http_request = self.client.post(format!("{}/responses", self.base_url))
            .json(&openai_request);
        
        http_request = inject_auth(http_request, auth)?;
        
        // 3. Send and parse SSE stream
        let response = http_request.send().await.map_err(WireError::Http)?;
        let events = self.parse_sse_stream(response).await?;
        
        Ok(TurnStream { events, turn_id: openai_request.id })
    }

    async fn list_models(&self, auth: AuthToken) -> Result<Vec<codex_wire_protocol::ModelInfo>, WireError> {
        let mut req = self.client.get(format!("{}/models", self.base_url));
        req = inject_auth(req, auth)?;
        let response = req.send().await.map_err(WireError::Http)?;
        let models: OpenAiModelsResponse = response.json().await.map_err(WireError::Http)?;
        Ok(models.data.into_iter().map(map_model).collect())
    }

    async fn compact_context(&self, request: codex_wire_protocol::CompactRequest, auth: AuthToken) -> Result<codex_wire_protocol::CompactResponse, WireError> {
        // Map to OpenAI compaction endpoint
        // ...
    }

    fn capabilities(&self) -> ProtocolCapabilities {
        ProtocolCapabilities {
            supports_streaming: true,
            supports_tools: true,
            supports_vision: true,
            supports_reasoning: true,
            supports_realtime: true,
            supports_compaction: true,
            supports_websockets: true,
            supports_parallel_tool_calls: true,
        }
    }
}
```

**Step 5: Write helper functions**

```rust
// codex-wire-protocol-openai/src/protocol.rs (continued)

fn inject_auth(req: reqwest::RequestBuilder, auth: AuthToken) -> Result<reqwest::RequestBuilder, WireError> {
    match auth {
        AuthToken::Bearer(token) => Ok(req.header("Authorization", format!("Bearer {token}"))),
        AuthToken::ApiKey { key, header_name } => Ok(req.header(header_name, key)),
        AuthToken::AwsSigV4 { .. } => Err(WireError::Auth("AWS SigV4 not supported for OpenAI protocol".into())),
        AuthToken::GcpAuth { .. } => Err(WireError::Auth("GCP auth not supported for OpenAI protocol".into())),
        AuthToken::None => Ok(req),
    }
}
```

**Step 6: Add to workspace**

Add `codex-wire-protocol-openai` to `codex-rs/Cargo.toml` members and dependencies.

**Step 7: Verify compilation**

```bash
cargo check -p codex-wire-protocol-openai
```

Expected: Clean compilation.

**Step 8: Commit**

```bash
git add codex-rs/codex-wire-protocol-openai/
git add codex-rs/Cargo.toml
git commit -m "feat(wire-protocol-openai): extract OpenAI Responses API into WireProtocol adapter"
```

---

### Task 3: Refactor `codex-api` to Be Protocol-Agnostic

**Objective:** Remove OpenAI-specific code from `codex-api`. Make it a thin dispatch layer that holds a `Arc<dyn WireProtocol>` and delegates all operations.

**Files:**
- Modify: `codex-rs/codex-api/src/lib.rs`
- Modify: `codex-rs/codex-api/src/provider.rs`
- Modify: `codex-rs/codex-api/src/endpoint/mod.rs`
- Modify: `codex-rs/codex-api/src/endpoint/responses.rs` (deprecate)
- Modify: `codex-rs/codex-api/src/endpoint/models.rs` (deprecate)
- Modify: `codex-rs/codex-api/src/endpoint/memories.rs` (deprecate)
- Modify: `codex-rs/codex-api/src/endpoint/realtime_call.rs` (deprecate)
- Modify: `codex-rs/codex-api/src/endpoint/realtime_websocket/` (deprecate)
- Modify: `codex-rs/codex-api/src/endpoint/responses_websocket.rs` (deprecate)
- Modify: `codex-rs/codex-api/src/sse/responses.rs` (deprecate)
- Modify: `codex-rs/codex-api/src/common.rs` (deprecate OpenAI types, re-export from adapter)
- Modify: `codex-rs/codex-api/Cargo.toml` (add codex-wire-protocol, codex-wire-protocol-openai deps)

**Step 1: Update Cargo.toml**

```toml
[dependencies]
# ... existing deps ...
codex-wire-protocol = { workspace = true }
codex-wire-protocol-openai = { workspace = true }
```

**Step 2: Redesign ApiClient**

```rust
// codex-rs/codex-api/src/lib.rs

pub use codex_wire_protocol::{AuthToken, TurnRequest, TurnStream, WireProtocol, WireError};

use std::sync::Arc;

/// Protocol-agnostic API client.
///
/// Holds a `WireProtocol` implementation and delegates all operations to it.
pub struct ApiClient {
    protocol: Arc<dyn WireProtocol>,
}

impl ApiClient {
    pub fn new(protocol: Arc<dyn WireProtocol>) -> Self {
        Self { protocol }
    }

    pub async fn stream_turn(&self, request: TurnRequest, auth: AuthToken) -> Result<TurnStream, WireError> {
        self.protocol.stream_turn(request, auth).await
    }

    pub async fn list_models(&self, auth: AuthToken) -> Result<Vec<codex_wire_protocol::ModelInfo>, WireError> {
        self.protocol.list_models(auth).await
    }

    pub async fn compact_context(&self, request: codex_wire_protocol::CompactRequest, auth: AuthToken) -> Result<codex_wire_protocol::CompactResponse, WireError> {
        self.protocol.compact_context(request, auth).await
    }

    pub fn capabilities(&self) -> codex_wire_protocol::ProtocolCapabilities {
        self.protocol.capabilities()
    }
}

/// Factory for creating protocol instances from provider config.
pub fn create_protocol(
    protocol_id: &str,
    http_client: codex_client::CodexHttpClient,
    base_url: String,
) -> Result<Arc<dyn WireProtocol>, WireError> {
    match protocol_id {
        "openai-responses" | "openai-chat-completions" => {
            Ok(Arc::new(codex_wire_protocol_openai::OpenAiWireProtocol::new(http_client, base_url)))
        }
        // Future protocols:
        // "anthropic-messages" => Ok(Arc::new(codex_wire_protocol_anthropic::AnthropicWireProtocol::new(...))),
        // "gemini" => Ok(Arc::new(codex_wire_protocol_gemini::GeminiWireProtocol::new(...))),
        _ => Err(WireError::Other(format!("Unknown protocol: {protocol_id}"))),
    }
}
```

**Step 3: Deprecate old endpoint modules**

For each deprecated module, add a doc comment and re-export from the adapter:

```rust
// codex-rs/codex-api/src/endpoint/responses.rs

//! DEPRECATED: This module is being migrated to `codex-wire-protocol-openai`.
//!
//! Use `codex_wire_protocol_openai::OpenAiWireProtocol` instead.

pub use codex_wire_protocol_openai::responses::ResponsesClient;
```

**Step 4: Verify compilation**

```bash
cargo check -p codex-api
```

Expected: Clean compilation. Warnings about deprecated re-exports are acceptable.

**Step 5: Commit**

```bash
git add codex-rs/codex-api/
git commit -m "refactor(codex-api): make protocol-agnostic, delegate to WireProtocol adapters"
```

---

### Task 4: Refactor `codex-core` ModelClient to Use WireProtocol

**Objective:** Replace hardcoded `ResponsesClient` usage in `codex-core` with the generic `WireProtocol` trait. This is the most critical refactor — it removes the OpenAI lock-in from the core session logic.

**Files:**
- Modify: `codex-rs/core/src/client.rs` (ModelClient, ModelClientSession)
- Modify: `codex-rs/core/src/client_common.rs` (Prompt, ResponseEvent, ResponseStream)
- Modify: `codex-rs/core/src/session/mod.rs` (session startup, config)
- Modify: `codex-rs/core/src/realtime_conversation.rs` (realtime model selection)
- Modify: `codex-rs/core/src/thread_manager.rs` (protocol selection)
- Modify: `codex-rs/core/Cargo.toml` (add codex-wire-protocol dep)

**Step 1: Update Cargo.toml**

```toml
[dependencies]
# ... existing deps ...
codex-wire-protocol = { workspace = true }
```

**Step 2: Redesign ModelClient**

Current `ModelClient` (simplified):
```rust
pub struct ModelClient {
    responses_client: ResponsesClient,
    realtime_call_client: RealtimeCallClient,
    // ...
}
```

New `ModelClient`:
```rust
// codex-rs/core/src/client.rs

use codex_wire_protocol::{WireProtocol, TurnRequest, TurnStream, AuthToken, ProtocolCapabilities};
use std::sync::Arc;

pub struct ModelClient {
    protocol: Arc<dyn WireProtocol>,
    auth: AuthToken,
    // Provider-specific config
    provider_info: codex_model_provider_info::ModelProviderInfo,
}

impl ModelClient {
    pub fn new(
        protocol: Arc<dyn WireProtocol>,
        auth: AuthToken,
        provider_info: codex_model_provider_info::ModelProviderInfo,
    ) -> Self {
        Self { protocol, auth, provider_info }
    }

    pub async fn stream_request(&self, request: TurnRequest) -> Result<TurnStream, WireError> {
        self.protocol.stream_turn(request, self.auth.clone()).await
    }

    pub fn supports_realtime(&self) -> bool {
        self.protocol.capabilities().supports_realtime
    }

    pub fn supports_compaction(&self) -> bool {
        self.protocol.capabilities().supports_compaction
    }

    // Remove is_openai() — replace with capability checks
}
```

**Step 3: Update ModelClientSession**

Current `ModelClientSession` holds a `ResponsesClientSession` (per-turn handle). Replace with generic turn streaming:

```rust
pub struct ModelClientSession {
    turn_stream: TurnStream,
    // ...
}

impl ModelClientSession {
    pub async fn next_event(&mut self) -> Option<TurnEvent> {
        self.turn_stream.events.recv().await
    }
}
```

**Step 4: Remove is_openai() checks**

Search for all `is_openai()` calls in `codex-core` and replace:

| Location | Current | New |
|----------|---------|-----|
| `client.rs:1185` | `is_openai()` gating compression | `supports_compression()` or always compress |
| `realtime_conversation.rs:962` | `is_openai()` check for API key fallback | Check `auth` type instead |
| `session/mod.rs:2467` | `gpt-5.2`/`gpt-5.3-codex` warning | Generic model fallback warning |

**Step 5: Update session startup**

In `session/mod.rs`, when creating `ModelClient`, use the protocol factory:

```rust
let protocol = codex_api::create_protocol(
    &config.protocol_id,
    http_client,
    provider_info.base_url().to_string(),
)?;

let model_client = ModelClient::new(protocol, auth, provider_info);
```

**Step 6: Verify compilation**

```bash
cargo check -p codex-core
```

Expected: Clean compilation. This is the critical gate.

**Step 7: Run core tests**

```bash
cargo test -p codex-core
```

Expected: All tests pass. If tests fail due to removed `is_openai()` or changed types, fix them.

**Step 8: Commit**

```bash
git add codex-rs/core/
git commit -m "refactor(codex-core): ModelClient uses WireProtocol trait, remove is_openai() checks"
```

---

### Task 5: Update `codex-model-provider` to Use Protocol IDs

**Objective:** Connect the provider metadata layer to the new protocol system. When a provider is selected, derive the correct `protocol_id` and pass it to `codex-core`.

**Files:**
- Modify: `codex-rs/model-provider/src/provider.rs`
- Modify: `codex-rs/model-provider-info/src/lib.rs`
- Modify: `codex-rs/model-provider-info/src/lib.rs` (add protocol_id field to presets)

**Step 1: Add protocol_id to ModelProviderInfo**

```rust
// codex-rs/model-provider-info/src/lib.rs

pub struct ModelProviderInfo {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub protocol_id: String, // NEW
    pub auth_type: AuthType,
    // ...
}
```

**Step 2: Update built-in providers**

```rust
pub fn built_in_model_providers() -> Vec<ModelProviderInfo> {
    vec![
        ModelProviderInfo {
            id: "openai".to_string(),
            name: "OpenAI".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            protocol_id: "openai-responses".to_string(),
            auth_type: AuthType::OAuth2OrApiKey { env_var: "OPENAI_API_KEY".to_string() },
            // ...
        },
        ModelProviderInfo {
            id: "anthropic".to_string(),
            name: "Anthropic".to_string(),
            base_url: "https://api.anthropic.com/v1".to_string(),
            protocol_id: "anthropic-messages".to_string(),
            auth_type: AuthType::ApiKey { env_var: "ANTHROPIC_API_KEY".to_string() },
            // ...
        },
        // ... etc
    ]
}
```

**Step 3: Update runtime provider dispatch**

In `model-provider/src/provider.rs`, when creating the API client:

```rust
let protocol = codex_api::create_protocol(
    &provider_info.protocol_id,
    http_client,
    provider_info.base_url.clone(),
)?;
```

**Step 4: Verify compilation**

```bash
cargo check -p codex-model-provider
cargo check -p codex-model-provider-info
```

**Step 5: Commit**

```bash
git add codex-rs/model-provider/
git add codex-rs/model-provider-info/
git commit -m "feat(model-provider): add protocol_id to provider metadata, dispatch to WireProtocol"
```

---

### Task 6: Update Config Schema

**Objective:** Add `protocol` field to config TOML schema. When absent, infer from `model_provider`.

**Files:**
- Modify: `codex-rs/config/src/config_toml.rs`
- Modify: `codex-rs/protocol/src/config_types.rs` (if protocol config types live here)
- Modify: `codex-rs/core/config.schema.json` (run `just write-config-schema` after)

**Step 1: Add protocol field**

```rust
// codex-rs/config/src/config_toml.rs

pub struct ConfigToml {
    pub model: Option<String>,
    pub model_provider: Option<String>,
    pub protocol: Option<String>, // NEW — overrides inferred protocol
    pub model_providers: HashMap<String, ModelProviderConfig>,
    // ...
}
```

**Step 2: Add inference logic**

```rust
pub fn resolve_protocol(&self) -> String {
    if let Some(protocol) = &self.protocol {
        return protocol.clone();
    }
    // Infer from provider ID
    match self.model_provider.as_deref() {
        Some("openai") | Some("azure-openai") => "openai-responses".to_string(),
        Some("anthropic") | Some("claude-aws") | Some("claude-gcp") => "anthropic-messages".to_string(),
        Some("gemini") | Some("vertex-ai") => "gemini".to_string(),
        Some("bedrock") => "bedrock".to_string(),
        Some("replicate") => "replicate".to_string(),
        Some("ollama") | Some("lmstudio") | Some("localai") => "openai-chat-completions".to_string(),
        // All other providers default to OpenAI-compatible chat completions
        _ => "openai-chat-completions".to_string(),
    }
}
```

**Step 3: Update model provider config**

```rust
pub struct ModelProviderConfig {
    pub name: String,
    pub base_url: String,
    pub protocol: Option<String>, // NEW
    pub env_key: Option<String>,
    // ...
}
```

**Step 4: Run config schema generator**

```bash
cd /Users/user/Documents/Projects/opencodex/codex/codex-rs
just write-config-schema
```

Expected: `core/config.schema.json` updated with new `protocol` fields.

**Step 5: Verify compilation**

```bash
cargo check -p codex-config
```

**Step 6: Commit**

```bash
git add codex-rs/config/
git add codex-rs/protocol/
git add codex-rs/core/config.schema.json
git commit -m "feat(config): add protocol field to config schema with provider inference"
```

---

### Task 7: Integration Test — Verify OpenAI Still Works

**Objective:** End-to-end test that the refactored code still works with OpenAI provider. We can't make live API calls, but we can test the wiring.

**Files:**
- Create: `codex-rs/core/tests/wire_protocol_integration.rs`
- Modify: `codex-rs/core/tests/common/mod.rs` (add protocol factory helper)

**Step 1: Write integration test**

```rust
// codex-rs/core/tests/wire_protocol_integration.rs

use codex_core::ModelClient;
use codex_api::create_protocol;
use codex_wire_protocol::{TurnRequest, AuthToken, Role, TurnItem};

#[tokio::test]
async fn test_openai_protocol_creation() {
    let http_client = codex_client::CodexHttpClient::new();
    let protocol = create_protocol(
        "openai-responses",
        http_client,
        "https://api.openai.com/v1".to_string(),
    ).unwrap();
    
    assert_eq!(protocol.protocol_id(), "openai-responses");
    assert!(protocol.capabilities().supports_streaming);
    assert!(protocol.capabilities().supports_tools);
}

#[tokio::test]
async fn test_model_client_with_openai_protocol() {
    let http_client = codex_client::CodexHttpClient::new();
    let protocol = create_protocol(
        "openai-responses",
        http_client,
        "https://api.openai.com/v1".to_string(),
    ).unwrap();
    
    let auth = AuthToken::Bearer("test-key".to_string());
    let provider_info = codex_model_provider_info::ModelProviderInfo {
        id: "openai".to_string(),
        name: "OpenAI".to_string(),
        base_url: "https://api.openai.com/v1".to_string(),
        protocol_id: "openai-responses".to_string(),
        // ...
    };
    
    let client = ModelClient::new(protocol, auth, provider_info);
    assert!(client.supports_realtime());
    assert!(client.supports_compaction());
}
```

**Step 2: Run test**

```bash
cargo test -p codex-core --test wire_protocol_integration
```

Expected: Tests pass.

**Step 3: Commit**

```bash
git add codex-rs/core/tests/
git commit -m "test(core): add WireProtocol integration tests for OpenAI adapter"
```

---

### Task 8: Update TUI to Remove is_openai() Assumptions

**Objective:** Find and fix TUI code that assumes OpenAI-specific behavior. Replace with capability checks or generic messaging.

**Files:**
- Modify: `codex-rs/tui/src/lib.rs` (remove OpenAI-specific defaults)
- Modify: `codex-rs/tui/src/onboarding/mod.rs` (auth flows)
- Modify: `codex-rs/tui/src/status/mod.rs` (status line)
- Search: All `is_openai()` or `chatgpt` references in `codex-rs/tui/src/`

**Step 1: Search for hardcoded references**

```bash
cd /Users/user/Documents/Projects/opencodex/codex/codex-rs
grep -r "is_openai\|chatgpt\|openai\.com\|gpt-5" tui/src/ --include="*.rs" | grep -v "_tests.rs" | grep -v "test_"
```

**Step 2: Replace each occurrence**

For each found reference, determine if it's:
- **UI text** → Replace with generic text (e.g., "Sign in with ChatGPT" → "Sign in with provider")
- **Capability check** → Replace with `protocol.capabilities().supports_*`
- **URL** → Make configurable per-provider
- **Model name** → Read from provider's default model config

**Step 3: Verify compilation**

```bash
cargo check -p codex-tui
```

**Step 4: Run TUI tests**

```bash
cargo test -p codex-tui
```

Expected: Tests pass. Snapshot tests may need updating (Task 9).

**Step 5: Commit**

```bash
git add codex-rs/tui/
git commit -m "refactor(tui): remove OpenAI-specific assumptions, use capability checks"
```

---

### Task 9: Update Snapshot Tests

**Objective:** If TUI snapshot tests fail due to changed text/output, review and accept updates.

**Files:**
- All `codex-rs/tui/tests/*.rs` and `codex-rs/tui/src/*_tests.rs`
- All `codex-rs/tui/tests/snapshots/*.snap`

**Step 1: Run TUI tests to generate new snapshots**

```bash
cargo test -p codex-tui
```

**Step 2: Review pending snapshots**

```bash
cargo insta pending-snapshots -p codex-tui
```

**Step 3: Accept intentional changes**

```bash
cargo insta accept -p codex-tui
```

Only accept if the changes are due to our refactor (generic text replacing OpenAI-specific text).

**Step 4: Commit**

```bash
git add codex-rs/tui/tests/snapshots/
git commit -m "test(tui): update snapshots for generic provider text"
```

---

### Task 10: Full Workspace Compilation Check

**Objective:** Verify the entire workspace compiles after all changes.

**Step 1: Check all crates**

```bash
cd /Users/user/Documents/Projects/opencodex/codex/codex-rs
cargo check --workspace
```

Expected: Clean compilation across all 117 crates.

**Step 2: Run full test suite (ask user first per AGENTS.md)**

The AGENTS.md says: "project-specific or individual tests can be run without asking the user, but do ask the user before running the complete test suite."

We already ran `cargo test -p codex-core` and `cargo test -p codex-tui`. For the full suite:

```bash
just test
```

This requires `cargo-nextest`. If not installed:
```bash
cargo install --locked cargo-nextest
```

**Step 3: Run formatter**

```bash
just fmt
```

**Step 4: Run fix**

```bash
just fix -p codex-core -p codex-api -p codex-tui -p codex-model-provider -p codex-config
```

**Step 5: Final commit**

```bash
git add -A
git commit -m "feat(multi-protocol): Sprint 1 complete — WireProtocol trait, OpenAI adapter extraction, core refactor"
```

---

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Breaking existing OpenAI users | High | Critical | Maintain 100% backward compatibility. `model_provider = "openai"` must work unchanged. Infer `protocol_id` when absent. |
| Test failures due to type changes | High | Medium | Run `cargo test -p codex-core` after every task. Fix tests incrementally. |
| Snapshot test churn | Medium | Low | Review each snapshot change. Only accept intentional text changes. |
| Circular dependencies | Medium | High | Keep `codex-wire-protocol` at bottom of dep graph (no deps on other codex crates). |
| Performance regression | Low | Medium | `Arc<dyn WireProtocol>` adds one vtable dispatch per turn. Negligible compared to network latency. |
| Bazel build failures | Medium | Medium | Update `BUILD.bazel` files for new crates. Run `just bazel-lock-check`. |

## Open Questions

1. **Realtime/WebRTC**: OpenAI's realtime API is deeply custom (WebRTC + custom binary protocol). Should we make realtime a separate trait (`RealtimeProtocol`) or keep it in `WireProtocol` with `supports_realtime()`?
   - **Decision**: Keep in `WireProtocol` for now. Anthropic and Gemini don't have realtime equivalents yet. When they do, we'll add a separate trait.

2. **Compaction**: OpenAI has a custom `/responses/compact` endpoint. Anthropic doesn't. Should compaction be optional?
   - **Decision**: Yes. `compact_context()` returns `Err(WireError::Unsupported)` for protocols that don't support it. Core falls back to client-side truncation.

3. **Auth in Sprint 1**: Should we refactor auth now or in Sprint 2?
   - **Decision**: Sprint 2. Sprint 1 passes `AuthToken` through. Sprint 2 will make auth acquisition per-provider.

4. **Model Catalog**: Should each protocol adapter bundle its own model catalog?
   - **Decision**: Yes. Sprint 2 will add `bundled_models()` to `WireProtocol` or a companion trait.

---

## Verification Checklist

- [ ] `cargo check -p codex-wire-protocol` passes
- [ ] `cargo check -p codex-wire-protocol-openai` passes
- [ ] `cargo check -p codex-api` passes
- [ ] `cargo check -p codex-core` passes
- [ ] `cargo check -p codex-tui` passes
- [ ] `cargo check -p codex-model-provider` passes
- [ ] `cargo check -p codex-config` passes
- [ ] `cargo check --workspace` passes
- [ ] `cargo test -p codex-core` passes
- [ ] `cargo test -p codex-tui` passes (snapshots reviewed)
- [ ] `just fmt` run and clean
- [ ] `just fix` run for touched crates
- [ ] `just write-config-schema` run
- [ ] `just bazel-lock-check` passes (if dependencies changed)
- [ ] No `is_openai()` or `chatgpt` references remain in production code (tests OK)
- [ ] OpenAI provider still works (integration test verifies protocol creation)
- [ ] All new crates have `BUILD.bazel` files

---

## Post-Sprint 1: Next Sprints

**Sprint 2 (Auth + Model Catalog):**
- Refactor `codex-login` to per-provider `AuthProvider` trait
- Add `bundled_models()` to `WireProtocol`
- Refactor `models-manager` to per-provider catalogs
- Add 10 provider presets

**Sprint 3 (Anthropic + Gemini):**
- Implement `codex-wire-protocol-anthropic`
- Implement `codex-wire-protocol-gemini`
- Add Anthropic and Gemini presets
- Integration tests

**Sprint 4 (Scale + Polish):**
- Add remaining 35 provider presets
- Credential import/discovery
- TUI provider selection UI
- Full test suite

---

*Plan written: 2026-05-10*
*Saved to: `.hermes/plans/2026-05-10_multi-protocol-sprint1.md`*
