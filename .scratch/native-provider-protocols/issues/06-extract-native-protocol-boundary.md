# Extract Native Protocol Boundary

Status: completed
Labels: ready-for-agent

## Goal

Prevent `core/src/client.rs` from permanently owning all Native Protocol conversion logic.

## Scope

- Move routing predicate, request construction, auth helper, error mapping, stream mapping, and event mapping into a dedicated module.
- Keep public surface small and testable.
- Preserve behavior while moving code.

## Files

- `codex-rs/core/src/client.rs`
- `codex-rs/core/src/native_protocol.rs`
- `codex-rs/core/src/lib.rs` if module export wiring is required
- `codex-rs/core/src/client_tests.rs` or a new module-local test block

## Acceptance Criteria

- `client.rs` delegates Native Protocol details to the new module.
- Existing Native Protocol tests still pass.
- The module names use the glossary vocabulary from `CONTEXT.md`.
- No unrelated formatting churn outside touched modules.

## Verification

- `cargo fmt -- --config imports_granularity=Item`
- `cargo check -p codex-core`
- `cargo test -p codex-core native_protocol`
- `cargo test -p codex-core build_turn_request_carries`
- `cargo test -p codex-core turn_event_mapping_preserves`

## Comments

This is the recommended next implementation issue if the current working tree already has functional Native Protocol glue inside `client.rs`.

Completed: Native Protocol mapping and auth helpers now live in `codex-rs/core/src/native_protocol.rs`; `client.rs` delegates through that module.
