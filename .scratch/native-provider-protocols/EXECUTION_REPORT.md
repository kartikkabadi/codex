# Native Provider Protocols Execution Report

Status: completed

## Completed Issues

- `01-route-supported-native-protocols.md`: completed.
- `02-build-native-turn-request.md`: completed.
- `03-map-turn-events-to-response-events.md`: completed.
- `04-wire-provider-auth-through-native-path.md`: completed.
- `05-verify-anthropic-and-gemini-adapters.md`: completed.
- `06-extract-native-protocol-boundary.md`: completed.

## Implementation Summary

- Added a focused Native Protocol module at `codex-rs/core/src/native_protocol.rs`.
- Kept `ModelClientSession` responsible for route orchestration.
- Moved Native Protocol request construction, auth token resolution, wire error mapping, stream mapping, and `TurnEvent` to `ResponseEvent` mapping into the new module.
- Preserved the routing policy from ADR-0001: only `anthropic-messages` and `gemini` use the Native Protocol path.
- Left OpenAI Responses and OpenAI-Compatible Providers on the existing path.

## Verification Run

```bash
cargo fmt -- --config imports_granularity=Item
cargo check -p codex-core -p codex-api -p codex-wire-protocol -p codex-wire-protocol-anthropic -p codex-wire-protocol-gemini
cargo test -p codex-core native_protocol
cargo test -p codex-core build_turn_request_carries
cargo test -p codex-core turn_event_mapping_preserves
cargo test -p codex-wire-protocol-anthropic
cargo test -p codex-wire-protocol-gemini
```

## Results

- Formatting completed. Stable rustfmt warned that `imports_granularity=Item` is nightly-only.
- Multi-crate `cargo check` passed.
- Focused core Native Protocol tests passed.
- Anthropic adapter tests passed with localhost-bind permissions.
- Gemini adapter tests passed with localhost-bind permissions.

## Notes

The adapter tests fail in the default sandbox because `wiremock` cannot bind local mock-server ports. They pass when rerun with permission to bind localhost ports.

Native Protocol v1 intentionally defers usage/token accounting. `TurnEvent::Usage` is currently ignored while the first slice focuses on functional streaming, text and image input, tool-call compatibility, provider auth, and core `ResponseEvent` compatibility.

Follow-up from grilling: Native Protocol should preserve Codex's `image_url` at the wire protocol boundary. `core` should map `ContentItem::InputImage { image_url, .. }` to a provider-neutral image item without parsing or fetching; Anthropic/Gemini adapters should own conversion from Image URL forms into provider-specific payloads.
