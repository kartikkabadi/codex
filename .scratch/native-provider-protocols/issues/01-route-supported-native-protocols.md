# Route Supported Native Protocols

Status: completed
Labels: ready-for-agent

## Goal

Route configured Anthropic and Gemini providers through the Native Protocol path while preserving the existing OpenAI Responses path.

## Scope

- Add or refine a routing predicate for Native Protocol support.
- Supported `protocol_id`s: `anthropic-messages`, `gemini`.
- Explicitly exclude `openai-responses` and `openai-chat-completions`.
- Keep routing orchestration in `ModelClientSession`.

## Files

- `codex-rs/core/src/client.rs`
- Optional extracted module: `codex-rs/core/src/native_protocol.rs`
- `codex-rs/core/src/client_tests.rs`

## Acceptance Criteria

- Anthropic and Gemini return `true` for Native Protocol support.
- OpenAI Responses and OpenAI-Compatible Providers return `false`.
- Existing Responses websocket/HTTP behavior is unchanged for OpenAI providers.

## Verification

- `cargo test -p codex-core native_protocol`
- `cargo check -p codex-core`

## Comments

Native Protocol scope is defined in `CONTEXT.md` and ADR-0001.

Completed: routing is limited to `anthropic-messages` and `gemini`; OpenAI Responses and OpenAI-Compatible Providers stay on the existing path.
