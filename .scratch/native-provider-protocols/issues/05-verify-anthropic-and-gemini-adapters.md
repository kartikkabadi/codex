# Verify Anthropic And Gemini Adapters

Status: completed
Labels: ready-for-agent

## Goal

Keep Anthropic and Gemini adapter behavior verified independently from core routing.

## Scope

- Anthropic streaming test for text, tool calls, and completion.
- Gemini streaming test for text and completion.
- Model listing tests where the adapter owns a static or provider-specific catalog.
- Remove unused fields/imports exposed by stricter checks.

## Files

- `codex-rs/codex-wire-protocol-anthropic/src/`
- `codex-rs/codex-wire-protocol-anthropic/src/protocol_tests.rs`
- `codex-rs/codex-wire-protocol-gemini/src/`
- `codex-rs/codex-wire-protocol-gemini/src/protocol_tests.rs`

## Acceptance Criteria

- Anthropic adapter tests pass.
- Gemini adapter tests pass.
- Tests assert completion instead of only setting unused local state.
- No warnings from unused test variables or stale deserialization fields.

## Verification

- `cargo test -p codex-wire-protocol-anthropic`
- `cargo test -p codex-wire-protocol-gemini`

## Comments

These tests bind local ports through `wiremock`, so sandboxed runs may need elevated localhost-bind permissions.

Completed: Anthropic and Gemini adapter tests pass with localhost-bind permissions.
