# Phase 0 design decisions and threat model

## Chosen direction

Use native Rust with Axum/Tokio for HTTP and WebSocket. Keep only wire types in
the shared WASM-compatible crate. The root Cargo default member remains the
browser application, so normal browser builds do not build the server. All four
service modules remain unimplemented until their respective phases.

Use a local configuration file validated against `server/config.schema.json`
and native startup checks. Addresses are IP socket addresses; origins are exact
serialized HTTP(S) origins without paths or trailing slashes. Unknown config keys
are rejected. Owner credentials are loaded from a separate file relative to the
configuration directory, never from CLI arguments or browser URLs.

The Phase 0 provisioner writes a 256-bit random hex credential, exclusively creates
files, and prints only their directory. The owner can configure an allowed origin
and a listen address; browser users cannot reconfigure these through HTTP.

## Threats and controls

| Threat | Phase 0 control / remaining work |
| --- | --- |
| Another website drives a local server | Exact Origin checks on HTTP, preflight, and WebSocket; owner credential on HTTP; origin-bound socket ticket |
| Credential disclosure through URLs/logs | Bearer HTTP auth and first-frame socket tickets; no query tokens, request logging, analytics, or persistence in the probe |
| Ticket replay / abandoned pairing | Single-use random tickets, 30-second expiry, five-second first-frame deadline, bounded pending tickets |
| Unauthenticated socket exhaustion | 16 connection permits, frame/message limits and first-frame timeout; full rate limits and connection/request budgets remain Phase 1 |
| Reconnect duplicates a move/import | No state-changing services yet; future operations require IDs/idempotency rules and no automatic replay |
| Remote access accidentally enabled | Loopback default, explicit non-loopback opt-in, Docker loopback host publication |
| Malicious engine / imported archive | No processes/imports in Phase 0; future owner-installed engines remain trusted code, with limits and strict path boundaries |
| Credential theft from the local machine | Private user-controlled file; hardened ACLs/provisioning/rotation are Phase 1; no protection against a compromised OS |
| Malicious browser extension or Ironwood XSS | Out of scope for Origin checks; must retain browser application's normal security controls |
| Credential sent to the wrong endpoint | Probe requires an explicit origin without embedded credentials; redirects are rejected; verify address before pairing |

HTTP on loopback is a local development mode. Do not expose this prototype
unencrypted to a LAN or internet. Docker's internal `0.0.0.0` binding requires
loopback publication on the host. Future remote mode needs HTTPS/WSS and a
documented TLS/authentication deployment. Do not disable browser security or
silently add a credential proxy on Cloudflare.

## Browser connectivity gate

`web/companion-check.html` is served by the normal Ironwood asset host, including
its COOP/COEP/CORP headers. This tests actual browser fetch/CORS and WebSocket
behavior instead of treating a command-line request as browser validation.
The page does not call the game worker or local game storage.

Localhost success is only one matrix entry. The deployed HTTPS page reaching
loopback may require local-network permission or be blocked in some browsers;
older private-network preflight behavior also differs. Non-loopback plain HTTP
from an HTTPS page is not an assumed supported path. No private-network allow
header or browser-security bypass is added on speculation.

Before accepting Phase 0, test Windows and Docker from the deployed Ironwood
origin in supported desktop browsers and record results. If direct local access
fails, evaluate trusted HTTPS for Companion or optional same-origin local app
hosting. That hosting must preserve browser engine isolation headers and avoid
unexpected game-storage origin migration. Do not claim a final topology until
the matrix is complete.

## Existing data and assistance boundaries

- `web/game-storage.js` owns browser storage/backup behavior. Phase 0 never opens
  IndexedDB. Later upload/download must copy explicitly and preserve originals.
- `PersistedGame` and exported Ironwood analysis JSON carry different purposes;
  design archive envelopes around versioned lossless payloads instead of assuming
  PGN or the browser database row is sufficient for every annotation/analysis.
- Archive schema version is null until migrations and round-trip fixtures exist.
- `src/opening_explorer.rs` and the browser explorer transport retain current
  Lichess sources. Archive will be a distinct source with its own generation.
- Local tablebases will be a separate source alongside the existing Lichess API.
  Preserve side-to-move/move-maker perspective, halfmove clocks, rounded DTZ,
  cursed wins/blessed losses, and branching previews.
- Existing UI/engine gates for training and online play remain authoritative.
  No Phase 0 route accepts analysis positions or returns chess assistance.
  Engine bridging initially applies only to casual play/analysis; training Elo
  must not silently transfer from Stockfish to another engine.

## Boundaries for later phases

Phase 1 adds built-in provisioning/rotation, hardened error handling and limits,
health/readiness, shutdown supervision, and polished packaging. Phase 2 adds UCI.
Database/probing/book crates and external datasets are intentionally not selected
or bundled in this phase. No R2/GHCR publication or public service was performed.
