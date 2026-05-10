# Codex Provider Architecture Audit

## Executive Summary

Codex already has a **surprisingly mature multi-provider abstraction**. It is not locked to OpenAI. The architecture supports arbitrary OpenAI-compatible API providers via config, plus first-class integrations for Amazon Bedrock, Ollama, and LMStudio. The main limitation is that the **wire protocol is hardcoded to the OpenAI Responses API** (`/v1/responses`). Any provider you add must speak that protocol or be proxied/translated to it.

---

## 1. The Provider Stack (Bottom-Up)

### Layer 1: HTTP Transport (`codex-client`)
- Generic HTTP client with retry, SSE streaming, WebSocket, telemetry, custom CA support.
- Knows nothing about OpenAI. Just executes `Request` -> `Response`/`StreamResponse`.

### Layer 2: API Client (`codex-api`)
- Builds on `codex-client`.
- Defines `Provider` struct: base_url, headers, query params, retry config, idle timeout.
- Defines endpoint clients: `ResponsesClient`, `ModelsClient`, `CompactClient`, `MemoriesClient`, `RealtimeCallClient`, `RealtimeWebsocketClient`, `ResponsesWebsocketClient`.
- **All endpoints are OpenAI Responses API paths**: `responses`, `models`, `responses/compact`, `memories/trace_summarize`, `realtime/calls`.
- Auth is injected via `AuthProvider` trait (header-only or request-signing).

### Layer 3: Provider Metadata (`codex-model-provider-info`)
- `ModelProviderInfo`: the serializable provider definition.
- Built-in providers:
  - `openai` -> `https://api.openai.com/v1` (requires OpenAI auth)
  - `amazon-bedrock` -> `https://bedrock-mantle.us-east-1.api.aws/openai/v1` (AWS SigV4)
  - `ollama` -> `http://localhost:11434/v1` (no auth)
  - `lmstudio` -> `http://localhost:1234/v1` (no auth)
- User-defined providers: add to `model_providers` table in `~/.codex/config.toml`.
- Auth options per provider:
  - `env_key`: read API key from env var
  - `experimental_bearer_token`: hardcoded token (discouraged)
  - `auth.command`: command-backed dynamic token
  - `aws`: AWS SigV4 signing
  - `requires_openai_auth`: triggers Codex login flow

### Layer 4: Runtime Provider (`codex-model-provider`)
- `ModelProvider` trait: the runtime abstraction used by core.
- `create_model_provider()` dispatches:
  - Amazon Bedrock -> `AmazonBedrockModelProvider` (custom)
  - Everything else -> `ConfiguredModelProvider` (generic OpenAI-compatible)
- Provides `api_provider()` (HTTP config) and `api_auth()` (auth headers).
- `models_manager()` returns either `OpenAiModelsManager` (fetches `/models`) or `StaticModelsManager` (bundled catalog).

### Layer 5: Model Manager (`codex-models-manager`)
- `ModelsManager` trait: list models, get model info, pick defaults.
- `OpenAiModelsManager`: fetches from provider's `/models`, caches to disk, merges with bundled catalog.
- `StaticModelsManager`: uses in-process catalog (for tests or fixed deployments).
- Model metadata (`ModelInfo`) includes: slug, display_name, shell_type, reasoning levels, context window, truncation policy, supported tools, etc.

### Layer 6: Core Session (`codex-core`)
- `ModelClient`: per-session handle that talks to the provider.
- `ModelClientSession`: per-turn handle that streams Responses API requests.
- Caches WebSocket connections for sticky routing.
- All turn execution goes through `ResponsesClient::stream_request()`.

---

## 2. The Wire Protocol Constraint

The **fundamental lock-in is not the provider, it's the wire protocol**.

Codex speaks the **OpenAI Responses API** exclusively. The request shape is `ResponsesApiRequest`:
```rust
pub struct ResponsesApiRequest {
    pub model: String,
    pub instructions: String,
    pub input: Vec<ResponseItem>,
    pub tools: Vec<serde_json::Value>,
    pub tool_choice: String,
    pub parallel_tool_calls: bool,
    pub reasoning: Option<Reasoning>,
    pub store: bool,
    pub stream: bool,
    pub include: Vec<String>,
    pub service_tier: Option<String>,
    pub prompt_cache_key: Option<String>,
    pub text: Option<TextControls>,
    pub client_metadata: Option<HashMap<String, String>>,
}
```

And the response is parsed as SSE events (`response.created`, `response.output_item.done`, `response.output_text.delta`, `response.completed`, etc.).

**Implication**: To add a provider like Anthropic Claude or Google Gemini, you have three options:
1. **Proxy/translate**: Run a local proxy that converts Responses API -> Anthropic Messages API. Codex already does this internally for some backends.
2. **Native support**: Add a new wire protocol variant to Codex (major refactor, touches `codex-api`, `codex-core`, TUI, app-server).
3. **Provider-side compatibility**: Use a provider that already offers an OpenAI-compatible Responses API endpoint (e.g., Azure OpenAI, some third-party proxies).

---

## 3. Auth System

`codex-login` crate (`login/src/auth/manager.rs`):

```rust
pub enum CodexAuth {
    ApiKey(ApiKeyAuth),
    Chatgpt(ChatgptAuth),          // OAuth2 device code flow
    ChatgptAuthTokens(ChatgptAuthTokens),
    AgentIdentity(AgentIdentityAuth), // JWT-based agent identity
}
```

- `ApiKey`: simple `Authorization: Bearer sk-...`
- `Chatgpt`: full OAuth2 with refresh tokens, stored in `~/.codex/auth.json`
- `AgentIdentity`: JWT signing for agent-to-agent auth
- `BearerAuthProvider`: generic bearer token header injection
- `UnauthenticatedAuthProvider`: no headers (for local OSS providers)
- `AgentIdentityAuthProvider`: signs requests with JWT
- `AmazonBedrockModelProvider`: uses AWS SigV4 signing

**For OpenCodex**: The ChatGPT/OAuth2 auth path is OpenAI-specific. You'd likely want to make `ApiKey` the default and skip the login server for non-OpenAI providers.

---

## 4. Config System

`~/.codex/config.toml`:

```toml
model = "gpt-5"
model_provider = "openai"

[model_providers.my-provider]
name = "My Provider"
base_url = "https://api.myprovider.com/v1"
env_key = "MY_PROVIDER_API_KEY"
wire_api = "responses"
supports_websockets = false
```

- `model_provider` selects which provider ID to use.
- `model_providers` extends the built-in list. Built-in IDs cannot be overridden.
- Validation in `config/src/config_toml.rs` ensures no conflicts (e.g., `aws` auth vs `env_key`).

---

## 5. What's Actually Missing for "OpenCodex"

### Already Works
- Multiple providers via config
- API key auth
- Command-backed dynamic auth
- AWS SigV4
- Local OSS providers (Ollama, LMStudio)
- Model discovery via `/models`
- Provider-specific headers, query params, retry config

### Missing / Hardcoded
1. **Wire protocol**: Only Responses API. No native Anthropic Messages, Gemini, or other protocols.
2. **Built-in providers**: Only OpenAI, Bedrock, Ollama, LMStudio. No Anthropic, Google, Groq, Cerebras, etc.
3. **Auth flows**: The default CLI login flow assumes OpenAI OAuth2. No built-in support for Anthropic API keys, Gemini API keys, etc., as first-class citizens.
4. **Model catalog**: `models-manager/src/models.json` is bundled with OpenAI model metadata. Other providers' models won't have the right metadata (shell_type, reasoning levels, etc.) unless you provide a custom catalog or proxy.
5. **Telemetry**: OpenAI-specific headers (`x-openai-subagent`, `openai-model`, `x-request-id`, etc.) and telemetry events are hardcoded.
6. **Realtime/WebRTC**: Hardcoded to OpenAI's `realtime/calls` and WebRTC signaling.
7. **App-server protocol**: The TypeScript-generated types and RPC methods are designed around OpenAI's API surface.

---

## 6. Strategic Options

### Option A: Config-Only Expansion (Easiest)
Add more built-in provider IDs and document how to configure any OpenAI-compatible endpoint. This is what Codex already does, just expanded.

**Effort**: Low. Add entries to `built_in_model_providers()` in `model-provider-info/src/lib.rs`.
**Limitation**: Only works for providers that already speak Responses API.

### Option B: Proxy/Adapter Layer (Moderate)
Build a local proxy (or integrate with an existing one like LiteLLM) that translates Responses API to other protocols. Codex talks to the proxy; the proxy talks to Anthropic/Gemini/etc.

**Effort**: Medium. Could be a new crate or external dependency.
**Advantage**: No changes to Codex core. Any provider works.

### Option C: Multi-Protocol Native Support (Hard)
Refactor `codex-api` and `codex-core` to support multiple wire protocols. Add a `WireApi` enum variant for each protocol (currently only `Responses`).

**Effort**: High. Touches SSE parsing, request serialization, WebSocket handling, model metadata, TUI, app-server.
**Advantage**: True native multi-provider support.

### Option D: Fork/Rebrand (Very Hard)
Strip OpenAI-specific code, rename crates, rebuild auth/telemetry/model catalog from scratch.

**Effort**: Very high. Basically a new project.
**Advantage**: Clean slate, no OpenAI baggage.

---

## 7. Recommended Next Steps

1. **Try Option A first**: Add a few popular OpenAI-compatible providers (e.g., Groq, Together AI, OpenRouter) as built-ins. Verify they work with the existing Responses API shape.
2. **Document the proxy approach**: Show how to use LiteLLM or a custom proxy to bridge non-Responses providers.
3. **Evaluate Option C** only if you hit fundamental limitations with the proxy approach (e.g., tool calling differences, streaming format incompatibilities).
4. **Avoid Option D** unless you want to maintain a full fork long-term. The codebase is large and actively developed by OpenAI.

---

## Files of Interest

| File | Role |
|------|------|
| `codex-rs/model-provider-info/src/lib.rs` | Built-in provider definitions, `ModelProviderInfo` |
| `codex-rs/model-provider/src/provider.rs` | Runtime `ModelProvider` trait, dispatch |
| `codex-rs/model-provider/src/auth.rs` | Auth resolution per provider |
| `codex-rs/codex-api/src/provider.rs` | HTTP `Provider` config (base_url, headers, retry) |
| `codex-rs/codex-api/src/endpoint/responses.rs` | Responses API client |
| `codex-rs/codex-api/src/common.rs` | `ResponsesApiRequest`, `ResponseEvent` |
| `codex-rs/codex-api/src/sse/responses.rs` | SSE event parsing (OpenAI-specific events) |
| `codex-rs/models-manager/src/manager.rs` | Model discovery and caching |
| `codex-rs/models-manager/src/model_info.rs` | Fallback model metadata |
| `codex-rs/config/src/config_toml.rs` | Config schema, `model_providers` validation |
| `codex-rs/login/src/auth/manager.rs` | `CodexAuth` enum, auth flows |
| `codex-rs/core/src/client.rs` | `ModelClient`, session-level API calls |
