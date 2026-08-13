# Build Native Turn Request

Status: completed
Labels: ready-for-agent

## Goal

Build a provider-neutral `TurnRequest` from Codex core prompt state for the Native Protocol path.

## Scope

- Preserve system instructions.
- Preserve formatted input items as `TurnItem`s.
- Convert supported function and namespace tools into wire `ToolSpec`s.
- Carry model id, reasoning effort, metadata, and stream preference.
- Keep unsupported or lossy mappings explicit in code and tests.

## Files

- `codex-rs/core/src/client.rs`
- Optional extracted module: `codex-rs/core/src/native_protocol.rs`
- `codex-rs/codex-wire-protocol/src/request.rs`
- `codex-rs/core/src/client_tests.rs`

## Acceptance Criteria

- A prompt with instructions, text input, and tools produces the expected `TurnRequest`.
- OpenAI-specific request fields that have no Native Protocol equivalent are either intentionally omitted or represented in provider-neutral fields.
- Request construction does not read secrets directly.

## Verification

- `cargo test -p codex-core build_turn_request_carries`
- `cargo check -p codex-core`

## Comments

This issue should avoid changing provider-specific adapter request serializers unless the generic `TurnRequest` contract proves insufficient.

Completed: `native_protocol::build_turn_request` builds provider-neutral turns from core prompt state.
