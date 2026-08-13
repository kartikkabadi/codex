# Native Provider Protocols Implementation Plan

Status: completed

## Working Order

1. Stabilize the routing policy: only `anthropic-messages` and `gemini` use the Native Protocol path.
2. Stabilize `TurnRequest` construction from core prompt state.
3. Stabilize `TurnEvent` to `ResponseEvent` mapping.
4. Wire provider auth through existing abstractions.
5. Run Anthropic and Gemini adapter tests.
6. Extract the Native Protocol boundary from `client.rs` once behavior is green.

## Current Best Starting Point

The current working tree already contains functional glue in `core/src/client.rs` and tests in `core/src/client_tests.rs`. Start by validating that behavior, then do the extraction in issue `06` rather than rewriting from scratch.

## Suggested Commit Shape

- Commit 1: docs and planning artifacts.
- Commit 2: functional Native Protocol routing and translation.
- Commit 3: adapter test fixes and auth fixture correction.
- Commit 4: extraction of Native Protocol helpers into a dedicated module.

## Verification Gate

Before claiming implementation complete, run:

```bash
cargo check -p codex-core -p codex-api -p codex-wire-protocol -p codex-wire-protocol-anthropic -p codex-wire-protocol-gemini
cargo test -p codex-core native_protocol
cargo test -p codex-core build_turn_request_carries
cargo test -p codex-core turn_event_mapping_preserves
cargo test -p codex-wire-protocol-anthropic
cargo test -p codex-wire-protocol-gemini
```

If `wiremock` tests fail with local port bind errors in the sandbox, rerun those adapter tests with permission to bind localhost ports.

## Risks To Watch

- `openai-chat-completions` being accidentally routed through the Native Protocol path.
- Tool-call argument deltas producing incomplete or duplicated final function-call arguments.
- Provider auth being accepted by core but lost before adapter request construction.
- Usage/token accounting being silently dropped where callers expect it.
- `client.rs` becoming harder to maintain if mapping helpers stay there permanently.
