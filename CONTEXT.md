# Opencodex

Opencodex is a Rust-first Codex fork centered around provider routing, model interaction, terminal/chat UI, app-server integration, and sandboxed agent execution.

## Language

- **Native Protocol**: a provider-specific API adapter for a non-OpenAI protocol, currently `anthropic-messages` and `gemini`.
  _Avoid_: using this term for OpenAI-compatible providers unless there is an explicit, tested adapter contract for that protocol.
- **OpenAI-Compatible Provider**: a provider exposed through OpenAI-style chat or responses semantics, such as providers using `openai-chat-completions`.
  _Avoid_: assuming OpenAI-compatible behavior is equivalent to native provider support.
- **Wire Protocol**: the provider-neutral trait and request/event vocabulary used by protocol adapters.
  _Avoid_: treating the wire protocol as Codex's public session event contract.
- **Turn**: one model interaction built from instructions, conversation items, tool specifications, and model options.
  _Avoid_: using turn to describe the entire task/session loop.
- **Image URL**: Codex's canonical image reference carried by `InputImage`, usually a data URL produced by image tooling but not restricted to that representation at the core boundary.
  _Avoid_: converting Image URLs into provider-specific inline image payloads inside core.
- **Image URL Parser**: the Wire Protocol helper that classifies Image URLs into provider-usable image inputs or clear unsupported-image errors.
  _Avoid_: duplicating Image URL parsing in each provider adapter or putting provider-specific parsing in core.
- **ResponseEvent**: the core event stream consumed by Codex's session loop, tool orchestration, TUI, and app-server layers.
  _Avoid_: replacing this term with provider-specific event names in core-facing docs.
- **Native Protocol v1**: the first Native Protocol integration slice, focused on functional streaming, text and image input, tool-call compatibility, provider auth, and core event compatibility.
  _Avoid_: assuming Native Protocol v1 includes usage/token accounting.

## Relationships

- `model-provider-info` defines provider metadata, including `protocol_id`.
- `core` resolves configured providers, builds model turns, and emits `ResponseEvent`s.
- `codex-api` chooses a `WireProtocol` adapter and streams provider-neutral `TurnEvent`s.
- Wire protocol adapters translate provider-specific APIs into `TurnEvent`s.
- Native Protocol support is intentionally scoped to `anthropic-messages` and `gemini` until another provider-specific adapter contract is explicitly designed and tested.
- Native Protocol v1 maps text and Image URL input into provider-neutral `TurnItem`s, then lets provider adapters convert Image URLs into provider-specific image payloads.
- The Wire Protocol owns Image URL parsing helpers; adapters use those helpers before building provider-specific image payloads.
- Native Protocol v1 maps streamed `TurnEvent`s into `ResponseEvent`s while intentionally deferring usage/token accounting.

## Example dialogue

- "Anthropic and Gemini use native protocols."
- "An OpenAI-compatible provider is not automatically native protocol support."
- "The native protocol path maps `TurnEvent`s back into core `ResponseEvent`s."

## Flagged ambiguities

- Whether `TurnRequest`/`TurnEvent` translation should remain inside `core/src/client.rs` or move behind a dedicated boundary.
- How Native Protocol usage/token accounting should map into core telemetry once v1 behavior is stable.
- Which Image URL forms each Native Protocol adapter should accept beyond inline image data URLs.
