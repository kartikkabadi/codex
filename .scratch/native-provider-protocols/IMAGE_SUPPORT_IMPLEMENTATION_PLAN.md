# Native Protocol Image Support Implementation Plan

Status: ready-for-agent

## Working Order

1. Update `TurnItem::Image` to preserve `image_url`.
2. Add the shared `Image URL Parser` in `codex-wire-protocol`.
3. Update core request construction to preserve `ContentItem::InputImage`.
4. Update Anthropic adapter conversion to use the parser.
5. Update Gemini adapter conversion to use the parser.
6. Run the image-support verification gate.

## Design Decisions

- Core preserves Image URLs and does not parse, fetch, or read them.
- The Wire Protocol owns shared Image URL parsing.
- Adapters own provider-specific image payload conversion.
- Native Protocol v1 supports inline image data URLs first.
- Unsupported Image URL forms fail clearly instead of being silently dropped.

## Verification Gate

```bash
cargo fmt -- --config imports_granularity=Item
cargo check -p codex-core -p codex-wire-protocol -p codex-wire-protocol-openai -p codex-wire-protocol-anthropic -p codex-wire-protocol-gemini
cargo test -p codex-wire-protocol image_url
cargo test -p codex-core build_turn_request_preserves_image_url
cargo test -p codex-wire-protocol-anthropic
cargo test -p codex-wire-protocol-gemini
```

## Risks

- Accidentally reintroducing image parsing into core.
- Silently converting unsupported Image URLs to text.
- Duplicating data URL parsing across adapters.
- Breaking OpenAI wire protocol conversions when changing `TurnItem::Image`.
- Adapter conversion functions may need to become fallible to return clear unsupported-image errors.

