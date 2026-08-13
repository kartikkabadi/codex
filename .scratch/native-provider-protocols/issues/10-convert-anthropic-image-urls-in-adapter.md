# Convert Anthropic Image URLs In Adapter

Status: ready-for-agent
Labels: ready-for-agent

## Goal

Make the Anthropic adapter convert supported `Image URL`s into Anthropic image content blocks.

## Scope

- Update `codex-wire-protocol-anthropic/src/request.rs` for the new `TurnItem::Image { image_url }` shape.
- Use the shared `Image URL Parser`.
- Convert inline image data URLs into `Content::Image` with base64 source data.
- Return a clear `WireError::Unsupported` path for unsupported Image URLs.
- Add adapter request-conversion tests independent of network streaming where possible.

## Files

- `codex-rs/codex-wire-protocol-anthropic/src/request.rs`
- `codex-rs/codex-wire-protocol-anthropic/src/protocol.rs` if request conversion must return `Result`
- `codex-rs/codex-wire-protocol-anthropic/src/protocol_tests.rs` or a new request test module

## Acceptance Criteria

- Inline image data URLs map to Anthropic base64 image source blocks.
- Unsupported Image URLs produce a clear wire error before request dispatch.
- Existing Anthropic text and tool-call tests continue passing.

## Verification

- `cargo test -p codex-wire-protocol-anthropic`
- `cargo check -p codex-wire-protocol-anthropic`

## Comments

If `turn_request_to_messages_request` currently cannot fail, prefer changing it to return a `Result` over silently degrading unsupported images.

