# Map Core Input Images To Turn Images

Status: ready-for-agent
Labels: ready-for-agent

## Goal

Ensure Native Protocol request construction preserves `ContentItem::InputImage` instead of dropping it.

## Scope

- Update `core/src/native_protocol.rs` so `ResponseItem::Message` content can produce both text and image `TurnItem`s.
- Preserve message role behavior for text messages.
- Preserve `image_url` exactly.
- Avoid parsing, fetching, reading, or validating Image URLs in core.
- Add a focused core test for image preservation.

## Files

- `codex-rs/core/src/native_protocol.rs`
- `codex-rs/core/src/client_tests.rs`

## Acceptance Criteria

- A prompt with `ContentItem::InputImage { image_url }` produces a `TurnItem::Image { image_url }`.
- Mixed text-plus-image input preserves both the text and the image.
- The image is not converted to text.
- The image is not silently dropped.

## Verification

- `cargo test -p codex-core build_turn_request_preserves_image_url`
- `cargo test -p codex-core build_turn_request_carries`
- `cargo check -p codex-core`

## Comments

This issue is intentionally core-only. Adapter parsing belongs in issues `08`, `10`, and `11`.

