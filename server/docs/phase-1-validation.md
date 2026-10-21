# Phase 1 validation record

Date: 2026-10-17. Status: implemented foundation; Windows development-machine checks passed. Linux Docker
and deployed-origin checks remain explicitly unverified in this environment.

## Scope

Native initialization and stop-before-rotation, durable credentials/configuration,
data-directory lock, health/readiness, configurable limits, structured errors and
redacted JSON logs, shutdown cancellation/deadline, Windows packaging, and a
non-root persistent-volume Docker recipe. All four chess-service flags remain
false, which does not prevent foundation readiness.

## Checks

- Rust unit tests cover auth/origins/protocol, ticket expiry/replay, configuration,
  health/readiness, structured method/body errors and rate-limit CORS responses.
- Native HTTP/WebSocket tests cover handshake, denied origins, replay, and
  unauthenticated commands.
- Lifecycle tests cover private provisioning, paths with spaces/Unicode, repeat
  initialization, restart persistence, locked rotation refusal, size/pending-ticket
  budgets, native healthcheck, active-socket shutdown, and old-token rejection
  following rotation. Logs are asserted not to contain tokens/tickets.
- Five Rust tests and both real-process Node integration tests passed. The Node
  suites also passed with COMPANION_BINARY pointing to the executable extracted
  from the ZIP. The lifecycle suite checked the Windows directory ACL explicitly.
- Clippy passes with warnings treated as errors for the server/protocol crates.
- The protocol builds for the browser WASM target. The browser WASM bundle was
  rebuilt, preserving the default browser workspace member.
- The unsigned development ZIP is under
  `output/companion-phase1/ironwood-companion-0.1.0-windows-x64.zip`, with a verified
  SHA-256 sidecar. Its 4106 archive entries include `.cargo/config.toml` and vendored
  source checksums. Its extracted source builds offline with
  `cargo build --offline --locked --release -p ironwood-server`.
- This is not an R2/GHCR publication or a signed release. Packaging copies a
  documentation snapshot; this repository record is the final authority for
  checks performed after the ZIP was assembled.

## Remaining environment acceptance

- Docker is not installed here: image startup, non-root volume permissions,
  SIGTERM, health state, resource limits and volume retention across replacement
  must be exercised on Linux before calling Docker validated.
- The deployed HTTPS origin and supported desktop-browser local-network matrix
  from Phase 0 remain pending. No browser-security bypass or deployment was used.
- A development-machine Windows pass does not substitute for testing the portable
  ZIP on a clean supported Windows machine. Signing, CI publication, image digest
  approval and base OS license/source audit remain release gates.
