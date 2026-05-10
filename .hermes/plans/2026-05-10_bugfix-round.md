# Multi-Protocol Provider Support — Bug Fix Round

> **For Hermes:** Fix all bugs found during Sprint 1+2 review before continuing.

**Goal:** Fix all issues found during the deep code review of Sprints 1 and 2. Ensure the codebase is solid before implementing new protocol adapters.

**Issues Found:** 18 across 9 crates. 0 critical, 8 medium, 14 low.

---

## Critical Issues (0)

None found.

---

## Medium Issues (8) — Must Fix

### Issue 1.3: Memory leak in `auth_type()` via `Box::leak`

**File:** `codex-auth/src/api_key.rs` (and oauth2.rs, aws_sigv4.rs, gcp.rs)
**Problem:** `auth_type()` uses `Box::leak` to return `&'static AuthType`. Every call leaks a new `AuthType`.
**Fix:** Use `OnceLock` to cache the leaked value.

```rust
// Before:
fn auth_type(&self) -> &AuthType {
    Box::leak(Box::new(AuthType::ApiKey { ... }))
}

// After:
fn auth_type(&self) -> &AuthType {
    static AUTH_TYPE: OnceLock<AuthType> = OnceLock::new();
    AUTH_TYPE.get_or_init(|| AuthType::ApiKey {
        env_var: self.env_var.clone(),
        header_name: self.header_name.clone(),
    })
}
```

### Issue 2.2: Empty `turn_id` in `stream_turn`

**File:** `codex-wire-protocol-openai/src/protocol.rs`
**Problem:** `response_stream_to_turn_stream(openai_stream, String::new())` passes empty turn_id.
**Fix:** Use the response ID from the first event or generate a UUID.

```rust
// Generate a turn_id from the response
let turn_id = format!("turn-{}", uuid::Uuid::new_v4());
response_stream_to_turn_stream(openai_stream, turn_id)
```

### Issue 2.7: `end_turn` information lost in `response_stream_to_turn_stream`

**File:** `codex-wire-protocol-openai/src/response.rs`
**Problem:** `ResponseEvent::Completed { end_turn, .. }` discards the `end_turn` boolean.
**Fix:** Add `end_turn` to `TurnEvent::Completed` or emit a separate event.

```rust
// Option A: Add end_turn to TurnEvent::Completed
pub enum TurnEvent {
    Completed { end_turn: bool },
    // ...
}

// Option B: Emit separate event
if end_turn {
    TurnEvent::EndTurn
} else {
    TurnEvent::Completed
}
```

### Issue 3.2: `AuthType::None` creates broken `ApiKeyAuthProvider`

**File:** `codex-auth/src/factory.rs`
**Problem:** For `AuthType::None`, factory creates `ApiKeyAuthProvider` with empty strings.
**Fix:** Create a proper `NoOpAuthProvider`.

```rust
// Add to codex-auth/src/noop.rs
pub struct NoOpAuthProvider;
impl AuthProvider for NoOpAuthProvider {
    fn authenticate(&self) -> Pin<Box<dyn Future<Output = Result<AuthToken, AuthError>> + Send>> {
        Box::pin(async { Ok(AuthToken::None) })
    }
    // ... refresh and revoke also return Ok
}

// In factory.rs:
AuthType::None => Box::new(NoOpAuthProvider),
```

### Issue 4.2: URL query parameter encoding

**File:** `codex-api/src/provider.rs`
**Problem:** Query params joined with raw `format!("{k}={v}")` — no URL encoding.
**Fix:** Use `Url::query_pairs_mut()`.

```rust
// Before:
let mut url = format!("{base_url}{path}");
for (k, v) in &self.query_params {
    url.push_str(&format!("{sep}{k}={v}"));
}

// After:
let mut url = Url::parse(&format!("{base_url}{path}"))?;
{
    let mut pairs = url.query_pairs_mut();
    for (k, v) in &self.query_params {
        pairs.append_pair(k, v);
    }
}
```

### Issue 6.1: `CodexAuthBridge::provider_id()` hardcodes "openai"

**File:** `codex-login/src/auth/bridge.rs`
**Problem:** Always returns "openai" regardless of actual auth type.
**Fix:** Derive from the underlying `CodexAuth` variant.

```rust
fn provider_id(&self) -> &str {
    match &self.auth {
        CodexAuth::ApiKey(_) => "openai-api-key",
        CodexAuth::Chatgpt(_) => "openai-chatgpt",
        CodexAuth::ChatgptAuthTokens(_) => "openai-chatgpt-tokens",
        CodexAuth::AgentIdentity(_) => "openai-agent-identity",
    }
}
```

### Issue 7.1: Synchronous file I/O in async context

**File:** `codex-models-manager/src/manager.rs`
**Problem:** `load_remote_models_from_file()` called inside async `apply_remote_models`.
**Fix:** Load file once at initialization, cache result.

```rust
// In OpenAiModelsManager::new, pre-load the file:
let cached_remote_models = load_remote_models_from_file(&codex_home).ok();

// Store in struct:
struct OpenAiModelsManager {
    cached_remote_models: Option<ModelsResponse>,
    // ...
}

// In apply_remote_models, use cached value:
if let Some(ref cached) = self.cached_remote_models {
    // use cached
}
```

### Issue 9.3: `scroll_offset` field never updated

**File:** `codex-tui/src/oss_selection.rs`
**Problem:** `self.scroll_offset` initialized to 0 but never updated. `adjusted_scroll` computed locally.
**Fix:** Either remove the field or update it in `handle_select_key`.

```rust
// Option A: Remove field
// Remove `scroll_offset: usize` from struct

// Option B: Update it
fn handle_select_key(&mut self, delta: isize) {
    let new_index = (self.selected_index as isize + delta)
        .clamp(0, self.filtered_providers.len().saturating_sub(1) as isize) as usize;
    self.selected_index = new_index;
    // Update scroll_offset to keep selected item visible
    let visible_count = self.visible_count();
    if new_index < self.scroll_offset {
        self.scroll_offset = new_index;
    } else if new_index >= self.scroll_offset + visible_count {
        self.scroll_offset = new_index.saturating_sub(visible_count - 1);
    }
}
```

---

## Low Issues (14) — Should Fix

### Issue 1.1: `async-trait` unused dependency
**File:** `codex-wire-protocol/Cargo.toml`
**Fix:** Remove `async-trait` from dependencies.

### Issue 1.2: `AuthToken` lacks `Zeroize`
**File:** `codex-wire-protocol/src/auth.rs`
**Fix:** Add `secrecy` crate or implement `Zeroize` for `AuthToken`.

### Issue 1.4: Unbounded channel in `TurnStream`
**File:** `codex-wire-protocol/src/response.rs`
**Fix:** Document backpressure semantics or use bounded channel.

### Issue 1.5: `u32` token count may overflow
**File:** `codex-wire-protocol/src/response.rs`
**Fix:** Change to `u64`.

### Issue 1.6: Missing rustdoc
**File:** `codex-wire-protocol/src/request.rs`
**Fix:** Add doc comments to all public types.

### Issue 2.1: `api_key()` clones token unnecessarily
**File:** `codex-wire-protocol-openai/src/protocol.rs`
**Fix:** Return `Option<&str>` instead of `Option<String>`.

### Issue 2.3: SSE silently drops unparsable events
**File:** `codex-wire-protocol-openai/src/sse.rs`
**Fix:** Escalate to `warn` after N consecutive failures.

### Issue 2.4: Inconsistent send error handling
**File:** `codex-wire-protocol-openai/src/sse.rs`
**Fix:** Be consistent — always abort or always ignore.

### Issue 2.5: Regex `unwrap` needs documentation
**File:** `codex-wire-protocol-openai/src/sse.rs`
**Fix:** Add comment explaining why `unwrap` is safe.

### Issue 2.6: Stream terminal state handling
**File:** `codex-wire-protocol-openai/src/types.rs`
**Fix:** Document that consumers must handle both `Some(Err)` and `None`.

### Issue 2.8: `ContentDelta` vs `ContentDone` semantics
**File:** `codex-wire-protocol-openai/src/response.rs`
**Fix:** Add `ContentDone` variant to `TurnEvent`.

### Issue 2.9: WebSocket URL scheme validation
**File:** `codex-wire-protocol-openai/src/websocket.rs`
**Fix:** Validate or convert scheme.

### Issue 2.10: `WsStream::drop` aborts without graceful close
**File:** `codex-wire-protocol-openai/src/websocket.rs`
**Fix:** Send close frame with timeout.

### Issue 3.1: Stub providers need TODO comments
**File:** `codex-auth/src/oauth2.rs`, `aws_sigv4.rs`, `gcp.rs`
**Fix:** Add `TODO(Sprint-3): implement OAuth2 device code flow` etc.

### Issue 4.1: Unreachable websocket URL fallthrough
**File:** `codex-api/src/provider.rs`
**Fix:** Return `Err` for unsupported schemes.

### Issue 5.1: `api_auth_provider` field unused in shown code
**File:** `codex-core/src/client.rs`
**Fix:** Verify usage in downstream methods.

### Issue 5.2: `StdMutex` in async context
**File:** `codex-core/src/client.rs`
**Fix:** Consider `tokio::sync::Mutex` if contention is possible.

### Issue 6.2: `AgentIdentity` error mapping misleading
**File:** `codex-login/src/auth/bridge.rs`
**Fix:** Handle `AgentIdentity` separately with clearer error.

### Issue 6.3: `as_auth_provider()` clones on every call
**File:** `codex-login/src/auth/manager.rs`
**Fix:** Cache the `Arc<dyn AuthProvider>`.

### Issue 7.2: `StaticModelsManager` unnecessary filtering
**File:** `codex-models-manager/src/manager.rs`
**Fix:** Skip filtering for static manager.

### Issue 7.3: Subtle `find_model_by_namespaced_suffix` logic
**File:** `codex-models-manager/src/manager.rs`
**Fix:** Add comment or test.

### Issue 8.1: Incorrect discussion number
**File:** `codex-model-provider-info/src/lib.rs`
**Fix:** Verify and correct.

### Issue 8.2: `preset()` hardcodes `supports_websockets: false`
**File:** `codex-model-provider-info/src/lib.rs`
**Fix:** Make configurable.

### Issue 8.3: Bedrock validation error message
**File:** `codex-model-provider-info/src/lib.rs`
**Fix:** List offending fields in error.

### Issue 8.4: `CODEX_OSS_PORT` invalid value handling
**File:** `codex-model-provider-info/src/lib.rs`
**Fix:** Log warning for invalid port.

### Issue 9.1: `OssSelectionWidget::new()` error handling
**File:** `codex-tui/src/oss_selection.rs`
**Status:** Acceptable — infallible.

### Issue 9.2: `KeyEventKind::Repeat` not handled
**File:** `codex-tui/src/oss_selection.rs`
**Fix:** Handle `Repeat` same as `Press`.

### Issue 9.4: "API key" label for command auth
**File:** `codex-tui/src/oss_selection.rs`
**Fix:** Change label to "Command auth".

---

## Fix Priority

| Priority | Issues | Time |
|----------|--------|------|
| P0 (must fix now) | 1.3, 2.2, 2.7, 3.2, 4.2, 6.1, 7.1, 9.3 | 2-3 hours |
| P1 (should fix now) | 1.1, 1.2, 1.4, 1.5, 1.6, 2.1, 2.3, 2.4, 2.8, 2.9, 3.1, 6.2, 6.3, 9.2, 9.4 | 3-4 hours |
| P2 (nice to have) | 2.5, 2.6, 2.10, 4.1, 5.1, 5.2, 7.2, 7.3, 8.1-8.4 | 2 hours |

---

## Verification After Fixes

After each fix batch:
1. `cargo check --workspace` — must be clean
2. `cargo test -p codex-core --lib -- --test-threads=2` — 1746 passed
3. `cargo test -p codex-api --lib` — 101 passed
4. `cargo test -p codex-tui --lib -- --test-threads=2` — 2307 passed
5. `cargo test -p codex-models-manager --lib` — 26 passed
6. `cargo test -p codex-model-provider-info` — 20 passed

---

*Report generated: 2026-05-10*
*Plan saved to: `.hermes/plans/2026-05-10_bugfix-round.md`*
