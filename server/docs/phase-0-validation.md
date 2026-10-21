# Phase 0 validation record

Date: 2026-10-17. Status: local Windows prototype validated; full Phase 0 acceptance
remains open for Linux Docker and the deployed HTTPS browser-origin matrix.

## Verified here

- Native Windows x64 Rust build and three Rust tests: exact origin/auth/protocol
  handling, ticket origin binding/expiry/replay, and fail-closed configuration.
- Native process test over real HTTP/WebSocket: CORS preflight, bad credentials,
  unsupported protocol, denied HTTP/WS Origin, correlated ping, ticket replay,
  and rejection of a ping before authentication.
- Shared protocol crate compiles for `wasm32-unknown-unknown`.
- Clippy passes for the server/protocol crates and all their targets with warnings
  treated as errors (`cargo clippy ... --all-targets -- -D warnings`).
- Browser diagnostic at `http://localhost:8788/companion-check.html` connecting to
  native `http://127.0.0.1:8790`, using a disposable no-data test configuration:
  secure context true, cross-origin isolated true, authenticated capabilities,
  wrong credential rejection, protocol mismatch rejection, authenticated
  WebSocket/ping, and replay rejection all passed in the in-app browser.
- Browser WASM rebuilt with `npm run build:wasm`; Wrangler assets refreshed.
- Windows/Linux Cargo dependency license inventory generated with explicit terms
  for 72 resolved dependency versions (including the project's own crates).

## Still required before Phase 0 can be marked complete

| Environment | Check | Status |
| --- | --- | --- |
| Linux x64 Docker | Build candidate image; run native tests and diagnostic against container | Pending: Docker is not installed in this environment |
| Deployed `https://ironwoodchess.com` → Windows loopback | Real browser fetch/CORS/WS with isolation, local-network permission and denial/retry | Pending: diagnostic not deployed |
| Deployed HTTPS origin → Docker published loopback | Same checks on supported host/browser combinations | Pending |
| Supported external desktop browsers | Record versions, permission prompts, cross-origin isolation and exact failures | Pending |
| Release image | Pin resolved base digest, inspect package license/NOTICE/source requirements | Pending; no image published |

Command-line requests that supply a production Origin do not count as the
deployed-browser acceptance check. Do not turn pending matrix entries into
passes based on the successful localhost probe.

The native test process uses temporary folders under ignored `tmp/` and an
ephemeral port; the browser probe also used a disposable credential. No user
game data, engine executable, archive, or dataset was accessed.
