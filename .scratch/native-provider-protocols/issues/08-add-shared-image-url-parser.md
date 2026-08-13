# Add Shared Image URL Parser

Status: ready-for-agent
Labels: ready-for-agent

## Goal

Add a shared `Image URL Parser` in the Wire Protocol crate so adapters do not duplicate data URL parsing.

## Scope

- Add a parser module in `codex-wire-protocol`, for example `src/image_url.rs`.
- Parse inline image data URLs of the form `data:image/<subtype>;base64,<payload>`.
- Treat `data:`, `image/`, and `base64` markers case-insensitively.
- Return structured inline image data with `mime_type` and base64 `data`.
- Return a clear unsupported error for non-image data URLs, non-base64 data URLs, remote URLs, file URLs, local paths, and malformed values.

## Files

- `codex-rs/codex-wire-protocol/src/image_url.rs`
- `codex-rs/codex-wire-protocol/src/lib.rs`
- `codex-rs/codex-wire-protocol/src/error.rs` if a more specific error variant is needed

## Acceptance Criteria

- Parser accepts `data:image/png;base64,AAA`.
- Parser accepts case-insensitive markers such as `DATA:IMAGE/PNG;BASE64,AAA`.
- Parser preserves the MIME type and base64 payload without decoding.
- Parser rejects `data:text/plain;base64,AAA`.
- Parser rejects `data:image/png,AAA`.
- Parser rejects `https://...`, `file://...`, and plain filesystem paths with a clear unsupported-image message.
- Tests cover accepted and rejected forms.

## Verification

- `cargo test -p codex-wire-protocol image_url`
- `cargo check -p codex-wire-protocol`

## Comments

Do not import or reuse the private token-estimation parser in `core/src/context_manager/history.rs`; that parser is scoped to estimation behavior.

