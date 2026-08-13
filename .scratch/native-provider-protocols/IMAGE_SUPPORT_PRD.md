# Native Protocol Image Support PRD

Status: ready-for-agent

## Summary

Native Protocol v1 must preserve image input from Codex core through the Wire Protocol and into the Anthropic and Gemini adapters. The current implementation drops `ContentItem::InputImage` while building `TurnRequest`; this violates the updated v1 scope.

The fix is not for core to parse or fetch images. Core should preserve Codex's `Image URL` domain concept, and provider adapters should convert supported Image URL forms into provider-specific image payloads.

## Utility And Metric

Kartik/customer utility: images that Codex already carries as `InputImage` remain visible to Anthropic and Gemini turns instead of silently disappearing.

System-level metric: a prompt containing `ContentItem::InputImage { image_url }` produces a Native Protocol `TurnRequest` containing the same Image URL, and Anthropic/Gemini adapters convert supported inline image data URLs into provider image request parts.

## Hard Constraints

- Do not fetch network images in core.
- Do not read local filesystem image paths in core.
- Do not silently drop image inputs.
- Keep OpenAI Responses behavior unchanged.
- Preserve the `Image URL` value at the core-to-Wire Protocol seam.
- Use shared parsing for common Image URL forms rather than duplicating fragile parser logic in each adapter.
- Unsupported Image URL forms must fail clearly through `WireError::Unsupported` or a more specific wire error if one is introduced.

## Non-Goals

- Full remote URL download support.
- Local path or `file://` resolution inside Native Protocol core.
- Provider-specific remote image support beyond what is explicitly implemented and tested.
- Usage/token accounting.
- Redesigning `ContentItem::InputImage` or the broader Codex image tooling.

## Required Behavior

1. Change `TurnItem::Image` to preserve `image_url`.
2. Map `ContentItem::InputImage { image_url, .. }` to `TurnItem::Image { image_url }` in `core/src/native_protocol.rs`.
3. Add a shared Image URL Parser in `codex-wire-protocol`.
4. Native Protocol v1 parser support must include inline image data URLs of the form `data:image/<subtype>;base64,<payload>`, case-insensitive for `data:`, media type prefix, and `base64`.
5. Anthropic and Gemini adapters must use the shared parser and convert inline image data URLs into their existing inline image payload types.
6. Unsupported Image URL forms must return a clear wire error instead of being dropped or converted to text.
7. Tests must cover image preservation in core, parser behavior, adapter conversion, and unsupported-image errors.

## Recommended Architecture

- `core/src/native_protocol.rs` remains the orchestration-side Turn builder and preserves Image URLs.
- `codex-wire-protocol/src/request.rs` owns the provider-neutral `TurnItem::Image { image_url }` shape.
- `codex-wire-protocol` owns an Image URL Parser module, for example `image_url.rs`.
- `codex-wire-protocol-anthropic` and `codex-wire-protocol-gemini` call the Image URL Parser while mapping `TurnItem::Image`.

This creates a deeper module: core gets a small interface and adapter locality stays high.

## Acceptance Criteria

- Native Protocol request construction no longer drops `InputImage`.
- The generic Wire Protocol request shape preserves `image_url`.
- Inline image data URLs are parsed once in the shared Wire Protocol helper.
- Anthropic and Gemini adapter tests prove inline images become provider image payloads.
- Unsupported Image URLs produce a clear error path.
- Existing text/tool Native Protocol tests continue passing.

## Verification Plan

- `cargo fmt -- --config imports_granularity=Item`
- `cargo check -p codex-core -p codex-wire-protocol -p codex-wire-protocol-anthropic -p codex-wire-protocol-gemini`
- `cargo test -p codex-core build_turn_request_preserves_image_url`
- `cargo test -p codex-wire-protocol image_url`
- `cargo test -p codex-wire-protocol-anthropic`
- `cargo test -p codex-wire-protocol-gemini`

Adapter tests may require localhost-bind permissions because existing protocol tests use `wiremock`.

## Decisions

- Preserve Image URL at the Wire Protocol boundary.
- Keep parsing/fetching out of core.
- Use shared Wire Protocol parsing for common Image URL forms.
- Native Protocol v1 supports inline image data URLs first; other forms fail clearly until adapter-specific support is designed.
