# Map Turn Events To Response Events

Status: completed
Labels: ready-for-agent

## Goal

Map Native Protocol `TurnEvent`s back into core `ResponseEvent`s so the session loop, tools, TUI, and app server keep receiving the contract they already understand.

## Scope

- Map content deltas.
- Map reasoning deltas when present.
- Map tool call start, argument delta, and completion events.
- Map turn completion into a valid completed response event.
- Map provider failures into stream errors.
- Decide which informational events, such as usage, are ignored initially.

## Files

- `codex-rs/core/src/client.rs`
- Optional extracted module: `codex-rs/core/src/native_protocol.rs`
- `codex-rs/core/src/client_tests.rs`
- `codex-rs/codex-wire-protocol/src/response.rs`

## Acceptance Criteria

- Tool-call completion preserves call id, function name, and arguments.
- Completion produces a response id and end-turn value usable by the existing session loop.
- Failure events surface as `CodexErr::Stream` or the nearest established stream error shape.
- Ignored events are documented by tests or comments where useful.

## Verification

- `cargo test -p codex-core turn_event_mapping_preserves`
- `cargo check -p codex-core`

## Comments

This is the critical compatibility layer. Do not redesign `ResponseEvent` as part of this issue.

Completed: `native_protocol::turn_event_to_response_event` preserves tool-call completion and maps streamed Native Protocol events back into core `ResponseEvent`s.
