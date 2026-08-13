# Convert Gemini Image URLs In Adapter

Status: ready-for-agent
Labels: ready-for-agent

## Goal

Make the Gemini adapter convert supported `Image URL`s into Gemini inline image parts.

## Scope

- Update `codex-wire-protocol-gemini/src/request.rs` for the new `TurnItem::Image { image_url }` shape.
- Use the shared `Image URL Parser`.
- Convert inline image data URLs into `Part::InlineData`.
- Return a clear `WireError::Unsupported` path for unsupported Image URLs.
- Add adapter request-conversion tests independent of network streaming where possible.

## Files

- `codex-rs/codex-wire-protocol-gemini/src/request.rs`
- `codex-rs/codex-wire-protocol-gemini/src/protocol.rs` if request conversion must return `Result`
- `codex-rs/codex-wire-protocol-gemini/src/protocol_tests.rs` or a new request test module

## Acceptance Criteria

- Inline image data URLs map to Gemini `inline_data` parts.
- Unsupported Image URLs produce a clear wire error before request dispatch.
- Existing Gemini text streaming tests continue passing.

## Verification

- `cargo test -p codex-wire-protocol-gemini`
- `cargo check -p codex-wire-protocol-gemini`

## Comments

If `turn_request_to_gemini_request` currently cannot fail, prefer changing it to return a `Result` over silently degrading unsupported images.

