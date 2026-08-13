# Preserve Image URL In Wire Protocol

Status: ready-for-agent
Labels: ready-for-agent

## Goal

Change the provider-neutral Wire Protocol image item so it preserves Codex's `Image URL` instead of requiring core to split images into `mime_type` and base64 `data`.

## Scope

- Change `TurnItem::Image` from `{ mime_type, data }` to `{ image_url }`.
- Update all compile errors caused by the shape change.
- Keep `TurnItem::Message` and `TurnItem::File` behavior unchanged.
- Keep OpenAI Responses behavior unchanged except where code must adapt to the new image shape.

## Files

- `codex-rs/codex-wire-protocol/src/request.rs`
- `codex-rs/codex-wire-protocol-openai/src/request.rs`
- `codex-rs/codex-wire-protocol-openai/src/compaction.rs`
- `codex-rs/codex-wire-protocol-anthropic/src/request.rs`
- `codex-rs/codex-wire-protocol-gemini/src/request.rs`

## Acceptance Criteria

- `TurnItem::Image` contains `image_url: String`.
- Existing callers compile against the new shape.
- OpenAI wire conversion still emits `ContentItem::InputImage { image_url, .. }`.
- No core code parses or fetches Image URLs.

## Verification

- `cargo check -p codex-wire-protocol -p codex-core -p codex-wire-protocol-openai -p codex-wire-protocol-anthropic -p codex-wire-protocol-gemini`

## Comments

Decision source: `docs/adr/0002-preserve-image-url-at-wire-protocol-boundary.md`.

