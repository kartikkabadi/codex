# Native Provider Protocols PRD

Status: completed

## Summary

Codex needs a focused Native Protocol path for provider-specific APIs, starting with Anthropic Messages and Gemini. The implementation should let `core` keep its existing `ResponseEvent` contract while routing supported Native Protocol providers through `codex-api` and the `WireProtocol` adapters.

This PRD intentionally excludes OpenAI-Compatible Providers that use `openai-chat-completions`. Those providers should continue through the existing path until a separate chat-completions adapter contract is designed and tested.

## Utility And Metric

Kartik/customer utility: local Codex can use Anthropic and Gemini through their real provider APIs without pretending they are OpenAI Responses providers.

System-level metric: a configured provider with `protocol_id` `anthropic-messages` or `gemini` can complete a streamed Turn through the Native Protocol path and produce the same core `ResponseEvent` shapes expected by the session loop, tools, TUI, and app server.

## Hard Constraints

- Correctness: request and event translation must preserve instructions, user-visible text, tool calls, and turn completion.
- Safety: do not change sandbox environment variables or sandbox policy behavior.
- Security and privacy: do not log or expose provider auth tokens; do not read local secrets while implementing.
- Compatibility: OpenAI Responses behavior must remain unchanged.
- Maintainability: keep mapping logic isolated enough that new adapters do not bloat `core/src/client.rs`.
- Cost and latency: avoid additional network calls, background workers, queues, or catalog refreshes in the model turn path.
- Compliance: provider-specific auth should use existing project auth abstractions.

## Non-Goals

- Full OpenAI Chat Completions migration.
- Support for Replicate, Bedrock, OSS, or arbitrary OpenAI-Compatible Providers through the Native Protocol path.
- Redesigning `ResponseEvent`, the session loop, app-server protocol, or TUI event handling.
- Replacing provider metadata, model catalog selection, or auth storage.
- Building a new user-facing provider setup flow.

## Current Code Map

- `codex-rs/model-provider-info/src/lib.rs`: provider presets and `protocol_id` metadata.
- `codex-rs/core/src/model_provider.rs`: runtime provider resolution and auth metadata.
- `codex-rs/core/src/client.rs`: model turn execution, current Responses path, and temporary Native Protocol glue.
- `codex-rs/codex-api/src/lib.rs`: `ApiClient` and `create_protocol` factory.
- `codex-rs/codex-wire-protocol/src/request.rs`: provider-neutral `TurnRequest`.
- `codex-rs/codex-wire-protocol/src/response.rs`: provider-neutral `TurnEvent`.
- `codex-rs/codex-wire-protocol-anthropic`: Anthropic adapter.
- `codex-rs/codex-wire-protocol-gemini`: Gemini adapter.

## Required Behavior

1. When the selected provider has `protocol_id = "anthropic-messages"` or `"gemini"`, `ModelClientSession` routes the Turn through `codex-api::ApiClient` and the matching `WireProtocol`.
2. When the selected provider uses `openai-responses` or `openai-chat-completions`, existing behavior remains unchanged.
3. The Native Protocol path builds a `TurnRequest` from the existing prompt, model info, tools, reasoning effort, and provider metadata.
4. The Native Protocol path maps streamed `TurnEvent`s back into `ResponseEvent`s.
5. Provider auth is pulled through existing provider auth abstractions, passed to the adapter/client layer, and never logged.
6. Unsupported Native Protocol failures surface as clear model-stream errors rather than silent fallback.

## Recommended Architecture

Keep routing choice in `ModelClientSession`, but move translation details into a focused core module:

- `core/src/native_protocol.rs` or `core/src/wire_protocol.rs`
- request builder: prompt/model/tool metadata to `TurnRequest`
- event mapper: `TurnEvent` to `ResponseEvent`
- auth helper: provider auth to wire client auth value
- routing predicate: `protocol_id` allowlist for supported Native Protocols

`client.rs` should remain the orchestrator. It should not permanently own every conversion helper.

## Acceptance Criteria

- Anthropic and Gemini providers use the Native Protocol path.
- OpenAI Responses and OpenAI-Compatible Providers do not use the Native Protocol path.
- Existing OpenAI Responses tests continue to pass.
- Unit tests cover routing policy, request translation, event translation, tool-call mapping, and unsupported protocol error mapping.
- Adapter tests cover successful streaming for Anthropic and Gemini.
- There is at least one integration-style core test proving provider auth is attached through the new path without exposing token values.
- Glossary and ADR docs define the Native Protocol scope.

## Verification Plan

- `cargo fmt -- --config imports_granularity=Item`
- `cargo check -p codex-core -p codex-api -p codex-wire-protocol -p codex-wire-protocol-anthropic -p codex-wire-protocol-gemini`
- `cargo test -p codex-core native_protocol`
- `cargo test -p codex-core build_turn_request_carries`
- `cargo test -p codex-core turn_event_mapping_preserves`
- `cargo test -p codex-wire-protocol-anthropic`
- `cargo test -p codex-wire-protocol-gemini`

Note: adapter tests use local HTTP servers and may require permissions that allow binding localhost ports.

## Open Decisions Made For This Plan

- Native Protocol means provider-specific non-OpenAI APIs only.
- `anthropic-messages` and `gemini` are in scope.
- `openai-chat-completions` is out of scope for this Native Protocol PRD.
- Translation should move toward a dedicated module rather than accumulating in `client.rs`.
