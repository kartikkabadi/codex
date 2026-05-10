# Multi-Protocol Provider Support — Sprint 2 Implementation Plan

> **For Hermes:** Use subagent-driven-development skill to implement this plan task-by-task.

**Goal:** Refactor auth system to be per-provider, add per-provider model catalogs with bundled metadata, and define the first 15 provider presets with correct base URLs, auth types, and model lists.

**Architecture:**
- Introduce `AuthProvider` trait in `codex-wire-protocol` (or new `codex-auth` crate) that abstracts OAuth2, API key, AWS SigV4, and GCP auth.
- Each provider preset bundles its own `AuthProvider` implementation, model catalog, and base instructions.
- `codex-login` becomes a dispatcher that routes to the correct `AuthProvider` based on the active provider's `auth_type`.
- `models-manager` gains a `ModelCatalog` trait; each provider implements it with bundled JSON + remote `/models` fallback.

**Tech Stack:** Rust 2024, Cargo workspace, tokio async, serde, schemars (for JSON schema), thiserror.

**Target:** 15 provider presets ready to use.

---

## Current State (post-Sprint 1)

From Sprint 1 we have:
- `codex-wire-protocol` with `WireProtocol` trait (dyn-compatible via `Pin<Box<dyn Future>>`)
- `codex-wire-protocol-openai` adapter implementing the trait for OpenAI Responses API
- `codex-api` is a thin dispatch layer over `Arc<dyn WireProtocol>`
- `codex-core` `ModelClient` uses the trait (but still has OpenAI-specific auth path)
- `model-provider-info` has `protocol_id` field on `ModelProviderInfo`
- Config schema supports `protocol_id` with `#[serde(default)]`

What Sprint 1 did NOT do:
- Auth is still hardcoded to OpenAI flows (OAuth2 device code, `OPENAI_API_KEY`, ChatGPT login)
- Model catalog is still a single `models.json` with only OpenAI models
- No provider presets beyond the 4 built-in (openai, amazon-bedrock, ollama, lmstudio)

---

## Design Decisions

### 1. Auth Abstraction

Current `CodexAuth` enum in `codex-login`:
```rust
pub enum CodexAuth {
    ApiKey(ApiKeyAuth),           // Generic — keep
    Chatgpt(ChatgptAuth),          // OpenAI-specific OAuth2
    ChatgptAuthTokens(ChatgptAuthTokens), // OpenAI-specific tokens
    AgentIdentity(AgentIdentityAuth), // JWT
}
```

New design:
```rust
// codex-wire-protocol/src/auth.rs (existing — extend)
pub trait AuthProvider: Send + Sync {
    fn auth_type(&self) -> AuthType;
    fn authenticate(&self) -> Pin<Box<dyn Future<Output = Result<AuthToken, AuthError>> + Send>>;
    fn refresh(&self) -> Pin<Box<dyn Future<Output = Result<AuthToken, AuthError>> + Send>>;
    fn revoke(&self) -> Pin<Box<dyn Future<Output = Result<(), AuthError>> + Send>>;
}

pub enum AuthType {
    ApiKey { env_var: String },
    OAuth2 { authorize_url: String, token_url: String, scopes: Vec<String> },
    AwsSigV4,
    GcpAuth,
    None,
}
```

Per-provider implementations:
- `OpenAiAuthProvider` — wraps existing ChatGPT OAuth2 + API key logic
- `ApiKeyAuthProvider` — generic env-var API key (used by Anthropic, Gemini, Groq, etc.)
- `AwsSigV4AuthProvider` — AWS signing (Bedrock)
- `GcpAuthProvider` — GCP service account / ADC (Vertex AI)

### 2. Model Catalog Abstraction

Current: `models-manager` has a single `models.json` with OpenAI models.

New design:
```rust
// codex-models-manager/src/catalog.rs
pub trait ModelCatalog: Send + Sync {
    fn provider_id(&self) -> &str;
    /// Bundled models that ship with the binary (fallback when remote fetch fails)
    fn bundled_models(&self) -> Vec<ModelInfo>;
    /// Base instructions / system prompt for this provider's models
    fn base_instructions(&self) -> Option<&str>;
    /// Fetch remote models from provider's /models endpoint
    fn fetch_remote_models(
        &self,
        auth: AuthToken,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<ModelInfo>, ModelError>> + Send>>;
}
```

Each provider preset bundles:
- `models.json` — list of models with metadata
- `prompt.md` — base instructions / system prompt

### 3. Provider Preset Definition

```rust
pub struct ProviderPreset {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub protocol_id: String,
    pub auth_type: AuthType,
    pub default_model: String,
    pub supports_websockets: bool,
    pub supports_realtime: bool,
    pub env_headers: Vec<(String, String)>, // (env_var, header_name)
}
```

---

## Provider Presets (15 for Sprint 2)

### OpenAI Responses API protocol
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

### Task 1: Extend `codex-wire-protocol` Auth Types

**Objective:** Add `AuthType` enum and `AuthProvider` trait to `codex-wire-protocol`.

**Files:**
- Modify: `codex-wire-protocol/src/auth.rs`
- Modify: `codex-wire-protocol/src/lib.rs`

**Step 1: Extend AuthToken**

Current `AuthToken`:
```rust
pub enum AuthToken {
    Bearer(String),
    ApiKey { key: String, header_name: String },
    AwsSigV4 { ... },
    GcpAuth { token: String },
    None,
}
```

Add `AuthType` (describes what auth a provider needs):
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthType {
    ApiKey { env_var: String, header_name: String },
    OAuth2 {
        authorize_url: String,
        token_url: String,
        scopes: Vec<String>,
        device_code_url: Option<String>,
    },
    AwsSigV4,
    GcpAuth,
    None,
}
```

**Step 2: Add AuthProvider trait**

```rust
pub trait AuthProvider: Send + Sync {
    fn provider_id(&self) -> &str;
    fn auth_type(&self) -> &AuthType;
    fn authenticate(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<AuthToken, AuthError>> + Send>>;
    fn refresh(
        &self,
        token: &AuthToken,
    ) -> Pin<Box<dyn Future<Output = Result<AuthToken, AuthError>> + Send>>;
    fn revoke(
        &self,
        token: &AuthToken,
    ) -> Pin<Box<dyn Future<Output = Result<(), AuthError>> + Send>>;
}

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Auth failed: {0}")]
    Failed(String),
    #[error("Unsupported auth type: {0}")]
    Unsupported(String),
    #[error("Environment variable {env_var} not set")]
    MissingEnvVar { env_var: String },
    #[error("OAuth error: {0}")]
    OAuth(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

**Step 3: Verify compilation**

```bash
cargo check -p codex-wire-protocol
```

**Step 4: Commit**

```bash
git add codex-rs/codex-wire-protocol/
git commit -m "feat(wire-protocol): add AuthType enum and AuthProvider trait"
```

---

### Task 2: Create `codex-auth` Crate with Per-Provider Implementations

**Objective:** Create a new crate that implements `AuthProvider` for each auth type.

**Files:**
- Create: `codex-rs/codex-auth/Cargo.toml`
- Create: `codex-rs/codex-auth/src/lib.rs`
- Create: `codex-rs/codex-auth/src/api_key.rs`
- Create: `codex-rs/codex-auth/src/oauth2.rs`
- Create: `codex-rs/codex-auth/src/aws_sigv4.rs`
- Create: `codex-rs/codex-auth/src/gcp.rs`
- Create: `codex-rs/codex-auth/src/factory.rs`

**Step 1: Cargo.toml**

```toml
[package]
name = "codex-auth"
version.workspace = true
edition.workspace = true
license.workspace = true

[lints]
workspace = true

[dependencies]
anyhow = { workspace = true }
async-trait = { workspace = true }
codex-wire-protocol = { workspace = true }
reqwest = { workspace = true }
serde = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }
thiserror = { workspace = true }
tokio = { workspace = true, features = ["time"] }
tracing = { workspace = true }
# OAuth2 deps
oauth2 = { version = "4", features = ["reqwest"] }
# AWS deps
aws-config = { workspace = true }
aws-sigv4 = { workspace = true }
aws-credential-types = { workspace = true }
# GCP deps
gcp_auth = "0.12"
```

**Step 2: Generic API Key AuthProvider**

```rust
// codex-auth/src/api_key.rs

use codex_wire_protocol::auth::{AuthProvider, AuthToken, AuthType, AuthError};
use std::pin::Pin;
use std::future::Future;

pub struct ApiKeyAuthProvider {
    provider_id: String,
    env_var: String,
    header_name: String,
}

impl ApiKeyAuthProvider {
    pub fn new(provider_id: String, env_var: String, header_name: String) -> Self {
        Self { provider_id, env_var, header_name }
    }
}

impl AuthProvider for ApiKeyAuthProvider {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }

    fn auth_type(&self) -> &AuthType {
        // Return a static or owned AuthType
        todo!()
    }

    fn authenticate(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<AuthToken, AuthError>> + Send>> {
        let env_var = self.env_var.clone();
        let header_name = self.header_name.clone();
        Box::pin(async move {
            let key = std::env::var(&env_var)
                .map_err(|_| AuthError::MissingEnvVar { env_var })?;
            Ok(AuthToken::ApiKey { key, header_name })
        })
    }

    fn refresh(
        &self,
        _token: &AuthToken,
    ) -> Pin<Box<dyn Future<Output = Result<AuthToken, AuthError>> + Send>> {
        self.authenticate()
    }

    fn revoke(
        &self,
        _token: &AuthToken,
    ) -> Pin<Box<dyn Future<Output = Result<(), AuthError>> + Send>> {
        Box::pin(async { Ok(()) })
    }
}
```

**Step 3: OAuth2 AuthProvider**

Extract existing OAuth2 logic from `codex-login` into `codex-auth/src/oauth2.rs`.

```rust
pub struct OAuth2AuthProvider {
    provider_id: String,
    authorize_url: String,
    token_url: String,
    device_code_url: Option<String>,
    scopes: Vec<String>,
}

impl AuthProvider for OAuth2AuthProvider {
    fn authenticate(&self,
    ) -> Pin<Box<dyn Future<Output = Result<AuthToken, AuthError>> + Send>> {
        // Use oauth2 crate for device code flow
        // Return AuthToken::Bearer(token)
        todo!()
    }
    // ...
}
```

**Step 4: Factory function**

```rust
// codex-auth/src/factory.rs

use codex_wire_protocol::auth::AuthType;
use crate::api_key::ApiKeyAuthProvider;
use crate::oauth2::OAuth2AuthProvider;
use crate::aws_sigv4::AwsSigV4AuthProvider;
use crate::gcp::GcpAuthProvider;

pub fn create_auth_provider(
    provider_id: &str,
    auth_type: AuthType,
) -> Box<dyn AuthProvider> {
    match auth_type {
        AuthType::ApiKey { env_var, header_name } => {
            Box::new(ApiKeyAuthProvider::new(provider_id.to_string(), env_var, header_name))
        }
        AuthType::OAuth2 { authorize_url, token_url, scopes, device_code_url } => {
            Box::new(OAuth2AuthProvider::new(
                provider_id.to_string(),
                authorize_url,
                token_url,
                device_code_url,
                scopes,
            ))
        }
        AuthType::AwsSigV4 => Box::new(AwsSigV4AuthProvider::new(provider_id.to_string())),
        AuthType::GcpAuth => Box::new(GcpAuthProvider::new(provider_id.to_string())),
        AuthType::None => Box::new(NoOpAuthProvider::new(provider_id.to_string())),
    }
}
```

**Step 5: Add to workspace**

Add `codex-auth` to `codex-rs/Cargo.toml` members and dependencies.

**Step 6: Verify compilation**

```bash
cargo check -p codex-auth
```

**Step 7: Commit**

```bash
git add codex-rs/codex-auth/
git add codex-rs/Cargo.toml
git commit -m "feat(auth): add codex-auth crate with per-provider AuthProvider implementations"
```

---

### Task 3: Refactor `codex-login` to Use `codex-auth`

**Objective:** Replace hardcoded OpenAI auth flows in `codex-login` with generic `AuthProvider` dispatch.

**Files:**
- Modify: `codex-rs/login/src/lib.rs`
- Modify: `codex-rs/login/src/auth/manager.rs`
- Modify: `codex-rs/login/src/auth/storage.rs`
- Modify: `codex-rs/login/Cargo.toml` (add codex-auth dep)

**Step 1: Update Cargo.toml**

```toml
[dependencies]
codex-auth = { workspace = true }
```

**Step 2: Refactor AuthManager**

Current `AuthManager` holds `CodexAuth` enum. New design:

```rust
pub struct AuthManager {
    provider: Arc<dyn AuthProvider>,
    storage: AuthStorage,
}

impl AuthManager {
    pub fn new(provider: Arc<dyn AuthProvider>) -> Self {
        Self { provider, storage: AuthStorage::default() }
    }

    pub async fn authenticate(&self) -> Result<AuthToken, AuthError> {
        // Check storage for cached token
        if let Some(token) = self.storage.load() {
            // Try refresh
            match self.provider.refresh(&token).await {
                Ok(new_token) => {
                    self.storage.save(&new_token);
                    return Ok(new_token);
                }
                Err(_) => {
                    // Fall through to fresh auth
                }
            }
        }
        let token = self.provider.authenticate().await?;
        self.storage.save(&token);
        Ok(token)
    }
}
```

**Step 3: Keep backward compatibility**

Existing `read_access_token_from_stdin`, `run_login_with_chatgpt`, etc. should still work by creating the appropriate `AuthProvider` internally.

**Step 4: Verify compilation**

```bash
cargo check -p codex-login
```

**Step 5: Commit**

```bash
git add codex-rs/login/
git commit -m "refactor(login): use generic AuthProvider trait from codex-auth"
```

---

### Task 4: Add Per-Provider Model Catalogs

**Objective:** Create `ModelCatalog` trait and implement it for each provider with bundled models.

**Files:**
- Modify: `codex-rs/models-manager/src/lib.rs`
- Modify: `codex-rs/models-manager/src/manager.rs`
- Create: `codex-rs/models-manager/src/catalog.rs`
- Create: `codex-rs/models-manager/src/catalogs/openai.rs`
- Create: `codex-rs/models-manager/src/catalogs/anthropic.rs`
- Create: `codex-rs/models-manager/src/catalogs/gemini.rs`
- Create: `codex-rs/models-manager/src/catalogs/generic.rs`
- Create: `codex-rs/models-manager/catalogs/openai/models.json`
- Create: `codex-rs/models-manager/catalogs/openai/prompt.md`
- Create: `codex-rs/models-manager/catalogs/anthropic/models.json`
- Create: `codex-rs/models-manager/catalogs/anthropic/prompt.md`
- Create: `codex-rs/models-manager/catalogs/gemini/models.json`
- Create: `codex-rs/models-manager/catalogs/gemini/prompt.md`

**Step 1: Define ModelCatalog trait**

```rust
// codex-rs/models-manager/src/catalog.rs

use codex_wire_protocol::response::ModelInfo;
use codex_wire_protocol::auth::AuthToken;
use std::pin::Pin;
use std::future::Future;

pub trait ModelCatalog: Send + Sync {
    fn provider_id(&self) -> &str;
    fn bundled_models(&self) -> Vec<ModelInfo>;
    fn base_instructions(&self) -> Option<&str>;
    fn fetch_remote_models(
        &self,
        auth: AuthToken,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<ModelInfo>, ModelError>> + Send>>;
}

#[derive(thiserror::Error, Debug)]
pub enum ModelError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Provider error: {0}")]
    Provider(String),
}
```

**Step 2: Create bundled model catalogs**

For OpenAI (`catalogs/openai/models.json`):
```json
[
  {
    "id": "gpt-5",
    "display_name": "GPT-5",
    "context_window": 128000,
    "supports_tools": true,
    "supports_vision": true,
    "supports_reasoning": true,
    "supports_realtime": false,
    "max_output_tokens": 16384
  },
  {
    "id": "gpt-5.4",
    "display_name": "GPT-5.4",
    "context_window": 256000,
    "supports_tools": true,
    "supports_vision": true,
    "supports_reasoning": true,
    "supports_realtime": false,
    "max_output_tokens": 32768
  }
]
```

For Anthropic (`catalogs/anthropic/models.json`):
```json
[
  {
    "id": "claude-sonnet-4-20250514",
    "display_name": "Claude Sonnet 4",
    "context_window": 200000,
    "supports_tools": true,
    "supports_vision": true,
    "supports_reasoning": true,
    "supports_realtime": false,
    "max_output_tokens": 8192
  },
  {
    "id": "claude-opus-4-20250514",
    "display_name": "Claude Opus 4",
    "context_window": 200000,
    "supports_tools": true,
    "supports_vision": true,
    "supports_reasoning": true,
    "supports_realtime": false,
    "max_output_tokens": 16384
  }
]
```

For Gemini (`catalogs/gemini/models.json`):
```json
[
  {
    "id": "gemini-2.5-pro",
    "display_name": "Gemini 2.5 Pro",
    "context_window": 1000000,
    "supports_tools": true,
    "supports_vision": true,
    "supports_reasoning": true,
    "supports_realtime": false,
    "max_output_tokens": 8192
  }
]
```

**Step 3: Implement catalog loaders**

```rust
// models-manager/src/catalogs/openai.rs

use crate::catalog::ModelCatalog;
use codex_wire_protocol::response::ModelInfo;

pub struct OpenAiModelCatalog;

impl ModelCatalog for OpenAiModelCatalog {
    fn provider_id(&self) -> &str {
        "openai"
    }

    fn bundled_models(&self) -> Vec<ModelInfo> {
        serde_json::from_str(include_str!("../../catalogs/openai/models.json"))
            .expect("valid openai models.json")
    }

    fn base_instructions(&self) -> Option<&str> {
        Some(include_str!("../../catalogs/openai/prompt.md"))
    }

    fn fetch_remote_models(
        &self,
        auth: AuthToken,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<ModelInfo>, ModelError>> + Send>> {
        // Use WireProtocol adapter to list models
        todo!()
    }
}
```

**Step 4: Update models-manager to use catalogs**

```rust
// models-manager/src/manager.rs

pub struct ModelsManager {
    catalog: Arc<dyn ModelCatalog>,
    remote_cache: RwLock<Vec<ModelInfo>>,
}

impl ModelsManager {
    pub fn new(catalog: Arc<dyn ModelCatalog>) -> Self {
        Self { catalog, remote_cache: RwLock::new(Vec::new()) }
    }

    pub async fn list_models(&self,
        auth: AuthToken,
    ) -> Result<Vec<ModelInfo>, ModelError> {
        let bundled = self.catalog.bundled_models();
        let remote = self.catalog.fetch_remote_models(auth).await.unwrap_or_default();
        // Merge: remote overrides bundled for same ID
        let mut merged: HashMap<String, ModelInfo> = bundled.into_iter()
            .map(|m| (m.id.clone(), m))
            .collect();
        for m in remote {
            merged.insert(m.id.clone(), m);
        }
        Ok(merged.into_values().collect())
    }
}
```

**Step 5: Update BUILD.bazel for include_str!**

Add `compile_data` for the JSON/MD files in the crate's `BUILD.bazel`.

**Step 6: Verify compilation**

```bash
cargo check -p codex-models-manager
```

**Step 7: Commit**

```bash
git add codex-rs/models-manager/
git commit -m "feat(models-manager): add per-provider ModelCatalog trait and bundled catalogs"
```

---

### Task 5: Define Provider Presets in `model-provider-info`

**Objective:** Add 15 provider presets with correct base URLs, auth types, and model defaults.

**Files:**
- Modify: `codex-rs/model-provider-info/src/lib.rs`
- Modify: `codex-rs/model-provider-info/src/presets.rs` (new file)

**Step 1: Create presets module**

```rust
// model-provider-info/src/presets.rs

use codex_wire_protocol::auth::AuthType;
use crate::ModelProviderInfo;
use crate::WireApi;

pub fn all_provider_presets() -> Vec<ModelProviderInfo> {
    vec![
        // OpenAI Responses API
        ModelProviderInfo {
            id: "openai".to_string(),
            name: "OpenAI".to_string(),
            base_url: Some("https://api.openai.com/v1".to_string()),
            env_key: Some("OPENAI_API_KEY".to_string()),
            env_key_instructions: None,
            experimental_bearer_token: None,
            auth: None,
            aws: None,
            wire_api: WireApi::Responses,
            query_params: None,
            http_headers: None,
            env_http_headers: None,
            request_max_retries: None,
            stream_max_retries: None,
            stream_idle_timeout_ms: None,
            websocket_connect_timeout_ms: None,
            requires_openai_auth: true,
            supports_websockets: true,
            protocol_id: "openai-responses".to_string(),
        },
        ModelProviderInfo {
            id: "azure-openai".to_string(),
            name: "Azure OpenAI".to_string(),
            base_url: None, // Must be configured
            env_key: Some("AZURE_OPENAI_API_KEY".to_string()),
            env_key_instructions: Some("Set AZURE_OPENAI_API_KEY and AZURE_OPENAI_ENDPOINT".to_string()),
            experimental_bearer_token: None,
            auth: None,
            aws: None,
            wire_api: WireApi::Responses,
            query_params: None,
            http_headers: None,
            env_http_headers: None,
            request_max_retries: None,
            stream_max_retries: None,
            stream_idle_timeout_ms: None,
            websocket_connect_timeout_ms: None,
            requires_openai_auth: false,
            supports_websockets: false,
            protocol_id: "openai-responses".to_string(),
        },
        // OpenAI Chat Completions
        ModelProviderInfo {
            id: "openrouter".to_string(),
            name: "OpenRouter".to_string(),
            base_url: Some("https://openrouter.ai/api/v1".to_string()),
            env_key: Some("OPENROUTER_API_KEY".to_string()),
            env_key_instructions: None,
            experimental_bearer_token: None,
            auth: None,
            aws: None,
            wire_api: WireApi::Responses,
            query_params: None,
            http_headers: None,
            env_http_headers: None,
            request_max_retries: None,
            stream_max_retries: None,
            stream_idle_timeout_ms: None,
            websocket_connect_timeout_ms: None,
            requires_openai_auth: false,
            supports_websockets: false,
            protocol_id: "openai-chat-completions".to_string(),
        },
        ModelProviderInfo {
            id: "groq".to_string(),
            name: "Groq".to_string(),
            base_url: Some("https://api.groq.com/openai/v1".to_string()),
            env_key: Some("GROQ_API_KEY".to_string()),
            env_key_instructions: None,
            experimental_bearer_token: None,
            auth: None,
            aws: None,
            wire_api: WireApi::Responses,
            query_params: None,
            http_headers: None,
            env_http_headers: None,
            request_max_retries: None,
            stream_max_retries: None,
            stream_idle_timeout_ms: None,
            websocket_connect_timeout_ms: None,
            requires_openai_auth: false,
            supports_websockets: false,
            protocol_id: "openai-chat-completions".to_string(),
        },
        ModelProviderInfo {
            id: "together".to_string(),
            name: "Together AI".to_string(),
            base_url: Some("https://api.together.xyz/v1".to_string()),
            env_key: Some("TOGETHER_API_KEY".to_string()),
            env_key_instructions: None,
            experimental_bearer_token: None,
            auth: None,
            aws: None,
            wire_api: WireApi::Responses,
            query_params: None,
            http_headers: None,
            env_http_headers: None,
            request_max_retries: None,
            stream_max_retries: None,
            stream_idle_timeout_ms: None,
            websocket_connect_timeout_ms: None,
            requires_openai_auth: false,
            supports_websockets: false,
            protocol_id: "openai-chat-completions".to_string(),
        },
        ModelProviderInfo {
            id: "fireworks".to_string(),
            name: "Fireworks AI".to_string(),
            base_url: Some("https://api.fireworks.ai/inference/v1".to_string()),
            env_key: Some("FIREWORKS_API_KEY".to_string()),
            env_key_instructions: None,
            experimental_bearer_token: None,
            auth: None,
            aws: None,
            wire_api: WireApi::Responses,
            query_params: None,
            http_headers: None,
            env_http_headers: None,
            request_max_retries: None,
            stream_max_retries: None,
            stream_idle_timeout_ms: None,
            websocket_connect_timeout_ms: None,
            requires_openai_auth: false,
            supports_websockets: false,
            protocol_id: "openai-chat-completions".to_string(),
        },
        ModelProviderInfo {
            id: "mistral".to_string(),
            name: "Mistral AI".to_string(),
            base_url: Some("https://api.mistral.ai/v1".to_string()),
            env_key: Some("MISTRAL_API_KEY".to_string()),
            env_key_instructions: None,
            experimental_bearer_token: None,
            auth: None,
            aws: None,
            wire_api: WireApi::Responses,
            query_params: None,
            http_headers: None,
            env_http_headers: None,
            request_max_retries: None,
            stream_max_retries: None,
            stream_idle_timeout_ms: None,
            websocket_connect_timeout_ms: None,
            requires_openai_auth: false,
            supports_websockets: false,
            protocol_id: "openai-chat-completions".to_string(),
        },
        ModelProviderInfo {
            id: "deepseek".to_string(),
            name: "DeepSeek".to_string(),
            base_url: Some("https://api.deepseek.com/v1".to_string()),
            env_key: Some("DEEPSEEK_API_KEY".to_string()),
            env_key_instructions: None,
            experimental_bearer_token: None,
            auth: None,
            aws: None,
            wire_api: WireApi::Responses,
            query_params: None,
            http_headers: None,
            env_http_headers: None,
            request_max_retries: None,
            stream_max_retries: None,
            stream_idle_timeout_ms: None,
            websocket_connect_timeout_ms: None,
            requires_openai_auth: false,
            supports_websockets: false,
            protocol_id: "openai-chat-completions".to_string(),
        },
        ModelProviderInfo {
            id: "xai".to_string(),
            name: "xAI Grok".to_string(),
            base_url: Some("https://api.x.ai/v1".to_string()),
            env_key: Some("XAI_API_KEY".to_string()),
            env_key_instructions: None,
            experimental_bearer_token: None,
            auth: None,
            aws: None,
            wire_api: WireApi::Responses,
            query_params: None,
            http_headers: None,
            env_http_headers: None,
            request_max_retries: None,
            stream_max_retries: None,
            stream_idle_timeout_ms: None,
            websocket_connect_timeout_ms: None,
            requires_openai_auth: false,
            supports_websockets: false,
            protocol_id: "openai-chat-completions".to_string(),
        },
        ModelProviderInfo {
            id: "ollama".to_string(),
            name: "Ollama".to_string(),
            base_url: Some("http://localhost:11434/v1".to_string()),
            env_key: None,
            env_key_instructions: None,
            experimental_bearer_token: None,
            auth: None,
            aws: None,
            wire_api: WireApi::Responses,
            query_params: None,
            http_headers: None,
            env_http_headers: None,
            request_max_retries: None,
            stream_max_retries: None,
            stream_idle_timeout_ms: None,
            websocket_connect_timeout_ms: None,
            requires_openai_auth: false,
            supports_websockets: false,
            protocol_id: "openai-chat-completions".to_string(),
        },
        ModelProviderInfo {
            id: "lmstudio".to_string(),
            name: "LMStudio".to_string(),
            base_url: Some("http://localhost:1234/v1".to_string()),
            env_key: None,
            env_key_instructions: None,
            experimental_bearer_token: None,
            auth: None,
            aws: None,
            wire_api: WireApi::Responses,
            query_params: None,
            http_headers: None,
            env_http_headers: None,
            request_max_retries: None,
            stream_max_retries: None,
            stream_idle_timeout_ms: None,
            websocket_connect_timeout_ms: None,
            requires_openai_auth: false,
            supports_websockets: false,
            protocol_id: "openai-chat-completions".to_string(),
        },
        // Anthropic Messages API
        ModelProviderInfo {
            id: "anthropic".to_string(),
            name: "Anthropic".to_string(),
            base_url: Some("https://api.anthropic.com/v1".to_string()),
            env_key: Some("ANTHROPIC_API_KEY".to_string()),
            env_key_instructions: None,
            experimental_bearer_token: None,
            auth: None,
            aws: None,
            wire_api: WireApi::Responses,
            query_params: None,
            http_headers: None,
            env_http_headers: None,
            request_max_retries: None,
            stream_max_retries: None,
            stream_idle_timeout_ms: None,
            websocket_connect_timeout_ms: None,
            requires_openai_auth: false,
            supports_websockets: false,
            protocol_id: "anthropic-messages".to_string(),
        },
        // Gemini API
        ModelProviderInfo {
            id: "gemini".to_string(),
            name: "Google Gemini".to_string(),
            base_url: Some("https://generativelanguage.googleapis.com/v1beta".to_string()),
            env_key: Some("GOOGLE_API_KEY".to_string()),
            env_key_instructions: None,
            experimental_bearer_token: None,
            auth: None,
            aws: None,
            wire_api: WireApi::Responses,
            query_params: None,
            http_headers: None,
            env_http_headers: None,
            request_max_retries: None,
            stream_max_retries: None,
            stream_idle_timeout_ms: None,
            websocket_connect_timeout_ms: None,
            requires_openai_auth: false,
            supports_websockets: false,
            protocol_id: "gemini".to_string(),
        },
        // Amazon Bedrock
        ModelProviderInfo {
            id: "amazon-bedrock".to_string(),
            name: "Amazon Bedrock".to_string(),
            base_url: Some("https://bedrock-runtime.us-east-1.amazonaws.com".to_string()),
            env_key: None,
            env_key_instructions: None,
            experimental_bearer_token: None,
            auth: None,
            aws: Some(ModelProviderAwsAuthInfo {
                profile: None,
                region: Some("us-east-1".to_string()),
            }),
            wire_api: WireApi::Responses,
            query_params: None,
            http_headers: None,
            env_http_headers: None,
            request_max_retries: None,
            stream_max_retries: None,
            stream_idle_timeout_ms: None,
            websocket_connect_timeout_ms: None,
            requires_openai_auth: false,
            supports_websockets: false,
            protocol_id: "bedrock".to_string(),
        },
        // LocalAI
        ModelProviderInfo {
            id: "localai".to_string(),
            name: "LocalAI".to_string(),
            base_url: Some("http://localhost:8080/v1".to_string()),
            env_key: None,
            env_key_instructions: None,
            experimental_bearer_token: None,
            auth: None,
            aws: None,
            wire_api: WireApi::Responses,
            query_params: None,
            http_headers: None,
            env_http_headers: None,
            request_max_retries: None,
            stream_max_retries: None,
            stream_idle_timeout_ms: None,
            websocket_connect_timeout_ms: None,
            requires_openai_auth: false,
            supports_websockets: false,
            protocol_id: "openai-chat-completions".to_string(),
        },
    ]
}
```

**Step 2: Wire presets into built_in_model_providers()**

```rust
// model-provider-info/src/lib.rs

pub fn built_in_model_providers(openai_base_url: Option<String>) -> HashMap<String, ModelProviderInfo> {
    let mut presets = presets::all_provider_presets();
    
    // Override openai base_url if provided
    if let Some(base_url) = openai_base_url {
        if let Some(openai) = presets.iter_mut().find(|p| p.id == "openai") {
            openai.base_url = Some(base_url);
        }
    }
    
    presets.into_iter().map(|p| (p.id.clone(), p)).collect()
}
```

**Step 3: Verify compilation**

```bash
cargo check -p codex-model-provider-info
```

**Step 4: Commit**

```bash
git add codex-rs/model-provider-info/
git commit -m "feat(provider-presets): add 15 provider presets with protocol_id and auth config"
```

---

### Task 6: Wire AuthProvider into `codex-core` Session Startup

**Objective:** When creating a `ModelClient`, use the provider's `AuthProvider` instead of hardcoded OpenAI auth.

**Files:**
- Modify: `codex-rs/core/src/client.rs`
- Modify: `codex-rs/core/src/thread_manager.rs`

**Step 1: Update ModelClient::new**

```rust
impl ModelClient {
    pub fn new(
        auth_manager: Option<Arc<AuthManager>>, // Now generic
        session_id: SessionId,
        thread_id: ThreadId,
        installation_id: String,
        provider_info: ModelProviderInfo,
        session_source: SessionSource,
        model_verbosity: Option<VerbosityConfig>,
        enable_request_compression: bool,
        include_timing_metrics: bool,
        beta_features_header: Option<String>,
        attestation_provider: Option<Arc<dyn AttestationProvider>>,
    ) -> Self {
        let model_provider = create_model_provider(provider_info.clone(), auth_manager);
        // ...
    }
}
```

**Step 2: Update thread_manager to create auth from preset**

```rust
// core/src/thread_manager.rs

use codex_auth::create_auth_provider;
use codex_wire_protocol::auth::AuthType;

fn build_auth_manager(provider_info: &ModelProviderInfo) -> Arc<AuthManager> {
    let auth_type = if provider_info.requires_openai_auth {
        AuthType::OAuth2 {
            authorize_url: "https://auth.openai.com/authorize".to_string(),
            token_url: "https://auth.openai.com/oauth/token".to_string(),
            scopes: vec!["openid".to_string(), "profile".to_string()],
            device_code_url: Some("https://auth.openai.com/oauth/device/code".to_string()),
        }
    } else if let Some(env_key) = &provider_info.env_key {
        AuthType::ApiKey {
            env_var: env_key.clone(),
            header_name: "Authorization".to_string(),
        }
    } else if provider_info.aws.is_some() {
        AuthType::AwsSigV4
    } else {
        AuthType::None
    };
    
    let auth_provider = create_auth_provider(&provider_info.id, auth_type);
    Arc::new(AuthManager::new(auth_provider))
}
```

**Step 3: Verify compilation**

```bash
cargo check -p codex-core
```

**Step 4: Run tests**

```bash
RUST_MIN_STACK=16777216 cargo test -p codex-core --lib -- --test-threads=2
```

**Step 5: Commit**

```bash
git add codex-rs/core/
git commit -m "feat(core): wire per-provider AuthProvider into session startup"
```

---

### Task 7: Wire ModelCatalog into `codex-core` ThreadManager

**Objective:** Use per-provider `ModelCatalog` when building the `ModelsManager`.

**Files:**
- Modify: `codex-rs/core/src/thread_manager.rs`

**Step 1: Import catalog types**

```rust
use codex_models_manager::catalog::ModelCatalog;
use codex_models_manager::catalogs::openai::OpenAiModelCatalog;
use codex_models_manager::catalogs::anthropic::AnthropicModelCatalog;
use codex_models_manager::catalogs::gemini::GeminiModelCatalog;
use codex_models_manager::catalogs::generic::GenericModelCatalog;
```

**Step 2: Create catalog from provider**

```rust
fn build_model_catalog(provider_id: &str) -> Arc<dyn ModelCatalog> {
    match provider_id {
        "openai" | "azure-openai" => Arc::new(OpenAiModelCatalog),
        "anthropic" | "claude-aws" | "claude-gcp" => Arc::new(AnthropicModelCatalog),
        "gemini" | "vertex-ai" => Arc::new(GeminiModelCatalog),
        _ => Arc::new(GenericModelCatalog::new(provider_id)),
    }
}
```

**Step 3: Update build_models_manager**

```rust
pub fn build_models_manager(provider_info: &ModelProviderInfo) -> Arc<ModelsManager> {
    let catalog = build_model_catalog(&provider_info.id);
    Arc::new(ModelsManager::new(catalog))
}
```

**Step 4: Verify compilation and tests**

```bash
cargo check -p codex-core
RUST_MIN_STACK=16777216 cargo test -p codex-core --lib -- --test-threads=2
```

**Step 5: Commit**

```bash
git add codex-rs/core/
git commit -m "feat(core): wire per-provider ModelCatalog into thread manager"
```

---

### Task 8: Update TUI Provider Selection

**Objective:** Update the TUI to show all 15 provider presets in the provider selection UI.

**Files:**
- Modify: `codex-rs/tui/src/onboarding/mod.rs` (or wherever provider selection happens)
- Search: All `is_openai()` or provider-specific UI code in tui/src/

**Step 1: Find provider selection UI**

```bash
grep -rn "model_provider\|provider.*select\|openai.*select" codex-rs/tui/src/ --include="*.rs" | grep -v test
```

**Step 2: Update to show all presets**

Replace hardcoded OpenAI/ChatGPT selection with a generic list populated from `built_in_model_providers()`.

**Step 3: Verify compilation**

```bash
cargo check -p codex-tui
```

**Step 4: Commit**

```bash
git add codex-rs/tui/
git commit -m "feat(tui): update provider selection to show all 15 presets"
```

---

### Task 9: Full Workspace Compilation + Test

**Objective:** Verify everything compiles and tests pass.

**Step 1: Check all crates**

```bash
cargo check --workspace
```

**Step 2: Run core tests**

```bash
RUST_MIN_STACK=16777216 cargo test -p codex-core --lib -- --test-threads=2
```

**Step 3: Run api tests**

```bash
cargo test -p codex-api --lib
```

**Step 4: Run model-provider-info tests**

```bash
cargo test -p codex-model-provider-info
```

**Step 5: Run tui tests**

```bash
cargo test -p codex-tui
```

**Step 6: Format and fix**

```bash
cargo fmt
cargo fix --lib -p codex-core -p codex-api -p codex-auth -p codex-models-manager -p codex-model-provider-info -p codex-tui --allow-dirty
```

**Step 7: Commit**

```bash
git add -A
git commit -m "feat(multi-protocol): Sprint 2 complete — per-provider auth, model catalogs, 15 presets"
```

---

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| OAuth2 crate dependency conflicts | Medium | High | Use `oauth2` v4 which is widely compatible; pin version in workspace |
| AWS SDK bloat | Medium | Medium | Only use `aws-sigv4` + `aws-credential-types`, not full SDK |
| GCP auth crate unmaintained | Low | Medium | Have fallback to read `gcloud auth print-access-token` |
| Model catalog JSON too large | Low | Low | Keep bundled catalogs minimal (top 5 models per provider) |
| TUI provider selection UX | Medium | Medium | Use fuzzy search / filter for 15+ items |

## Open Questions

1. **Should `codex-auth` be a separate crate or live in `codex-wire-protocol`?**
   - **Decision:** Separate crate. Auth has heavy deps (oauth2, aws-sdk, gcp_auth) that not all consumers need.

2. **How do we handle OAuth2 redirect URI for desktop apps?**
   - **Answer:** Use device code flow (no redirect URI needed) as default. Browser-based OAuth2 as optional fallback.

3. **Should we support API key from keychain/1Password/etc?**
   - **Sprint 2:** No. Sprint 3 or 4 can add credential discovery.

4. **What about models that don't support tools?**
   - **Answer:** `ModelInfo.supports_tools` flag. Core checks this before adding tools to the request.

---

## Verification Checklist

- [ ] `cargo check -p codex-wire-protocol` passes
- [ ] `cargo check -p codex-auth` passes
- [ ] `cargo check -p codex-api` passes
- [ ] `cargo check -p codex-core` passes
- [ ] `cargo check -p codex-tui` passes
- [ ] `cargo check -p codex-models-manager` passes
- [ ] `cargo check -p codex-model-provider-info` passes
- [ ] `cargo check --workspace` passes
- [ ] `cargo test -p codex-core --lib` passes
- [ ] `cargo test -p codex-api --lib` passes
- [ ] `cargo test -p codex-auth` passes
- [ ] `cargo test -p codex-models-manager` passes
- [ ] `cargo test -p codex-model-provider-info` passes
- [ ] `cargo test -p codex-tui` passes (snapshots reviewed)
- [ ] `cargo fmt` run
- [ ] `cargo fix` run for touched crates
- [ ] `just write-config-schema` run (if config structs changed)
- [ ] `just bazel-lock-check` passes
- [ ] 15 provider presets defined with correct base URLs
- [ ] Each preset has correct `auth_type` and `protocol_id`
- [ ] OpenAI provider still works (backward compatibility)

---

*Plan written: 2026-05-10*
*Saved to: `.hermes/plans/2026-05-10_multi-protocol-sprint2.md`*
