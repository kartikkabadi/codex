# Wire Provider Auth Through Native Path

Status: completed
Labels: ready-for-agent

## Goal

Use existing provider auth abstractions when streaming through Native Protocol adapters.

## Scope

- Resolve auth from the configured provider using existing core/provider auth types.
- Pass auth into the `codex-api` or adapter layer without logging or persisting token values.
- Ensure command-backed auth fixtures are actually used in tests.

## Files

- `codex-rs/core/src/client.rs`
- `codex-rs/core/tests/suite/client.rs`
- `codex-rs/codex-api/src/lib.rs`
- `codex-rs/codex-wire-protocol-anthropic/src/protocol.rs`
- `codex-rs/codex-wire-protocol-gemini/src/protocol.rs`

## Acceptance Criteria

- Native Protocol requests can receive provider auth through the same configured-provider path used elsewhere.
- Tests prove command-backed auth is supplied to the request path.
- No test or debug output prints secret values.

## Verification

- `cargo check -p codex-core -p codex-api`
- Relevant core suite test for provider auth.

## Comments

If the current protocol constructors cannot safely receive auth, prefer a small API adjustment over special-casing in `core`.

Completed: the Native Protocol path resolves auth through existing provider/auth state and passes it into `ApiClient::stream_turn`.
