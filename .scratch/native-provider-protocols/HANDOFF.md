# Native Provider Protocols Handoff

Status: completed

## Start Here

Read these first:

1. `CONTEXT.md`
2. `docs/adr/0001-native-protocol-scope.md`
3. `.scratch/native-provider-protocols/PRD.md`
4. `.scratch/native-provider-protocols/IMPLEMENTATION_PLAN.md`

Then work issues in order:

1. `.scratch/native-provider-protocols/issues/01-route-supported-native-protocols.md`
2. `.scratch/native-provider-protocols/issues/02-build-native-turn-request.md`
3. `.scratch/native-provider-protocols/issues/03-map-turn-events-to-response-events.md`
4. `.scratch/native-provider-protocols/issues/04-wire-provider-auth-through-native-path.md`
5. `.scratch/native-provider-protocols/issues/05-verify-anthropic-and-gemini-adapters.md`
6. `.scratch/native-provider-protocols/issues/06-extract-native-protocol-boundary.md`

## Current Repo Reality

The current working tree already contains Native Protocol implementation work in `codex-rs/core/src/client.rs`, adapter cleanup in Anthropic/Gemini protocol crates, and tests in `codex-rs/core/src/client_tests.rs`.

Treat that code as the first implementation draft. Do not restart from scratch. Validate it, tighten it, then extract the boundary.

## Key Decision

Native Protocol support means provider-specific non-OpenAI APIs only:

- in scope: `anthropic-messages`, `gemini`
- out of scope: `openai-responses`, `openai-chat-completions`, Replicate, Bedrock, OSS, arbitrary OpenAI-Compatible Providers

## Recommended First Coding Move

Run the existing focused checks, then implement issue `06` if they pass:

```bash
cargo check -p codex-core -p codex-api -p codex-wire-protocol -p codex-wire-protocol-anthropic -p codex-wire-protocol-gemini
cargo test -p codex-core native_protocol
cargo test -p codex-core build_turn_request_carries
cargo test -p codex-core turn_event_mapping_preserves
```

After that, run adapter tests:

```bash
cargo test -p codex-wire-protocol-anthropic
cargo test -p codex-wire-protocol-gemini
```

If adapter tests fail with a localhost bind permission error, rerun them with permission to bind local ports.

## Done Means

- Native routing is limited to Anthropic and Gemini.
- Request translation is tested.
- Event translation is tested.
- Provider auth reaches the Native Protocol path without exposing token values.
- Anthropic and Gemini adapter tests pass.
- `client.rs` delegates mapping details to a dedicated Native Protocol module.
- Documentation, PRD, issues, and ADR agree on the same vocabulary.

## Completion

Completed in this working tree. See `EXECUTION_REPORT.md` for the verification log.

## Follow-Up: Image Support

Native Protocol image support is planned but not implemented. Start with:

1. `.scratch/native-provider-protocols/IMAGE_SUPPORT_PRD.md`
2. `.scratch/native-provider-protocols/IMAGE_SUPPORT_IMPLEMENTATION_PLAN.md`
3. `.scratch/native-provider-protocols/issues/07-preserve-image-url-in-wire-protocol.md`

The key decision is ADR-0002: preserve Codex's `Image URL` at the Wire Protocol boundary, keep parsing out of core, and let adapters convert provider-specific image payloads.
