# Medium Security Cloud findings — internal validation

Reviewed 2026-10-24 against the current working tree. Source: completed Chess repository scan `wfr_dfb21dac0be6392098cc849dd3b4c4fda56a9161f652570dd45d06027dfc01df`, observed revision `e855ef3dcc02df33abb9f086aa732658a685b2a9`. The previously repaired high-severity engine proxy remains covered by its regression tests. No scan was launched, finding status changed, or production deployment performed.

## FICS resource exhaustion

Finding `csf_58c9a327a905155f0822e8f4`.

- The public route validates WebSocket upgrade and same Origin, then delegates to one named Durable Object. Origin is browser isolation, not authentication. Admission uses Cloudflare's client IP header and is global across edge isolates: 128 simultaneous upstream sessions, four per IP, eight attempts per IP per minute. Attempt tracking is bounded to 4,096 IP entries with expiration.
- Admission occurs before TCP allocation; connection errors, client close/error and deadlines release capacity.
- Incoming messages are capped at 1,024 characters before JSON parsing. Limits are 120 commands and 2,000 total frames per ten seconds. Password and unrestricted-console grammar remains supported in its existing authenticated session states.
- Pending TCP writes are bounded to 32 KiB, each write has a ten-second deadline, login has a two-minute deadline, idle sessions expire after thirty minutes, and sessions have a six-hour lifetime ceiling.
- Upstream output is capped at 128 KiB per chunk, 512 KiB per second and 256 MiB per session. Random, non-predictable receipt acknowledgements bound unread output to 512 KiB / 256 chunks and expire within thirty seconds (checked every ten seconds). This does not depend solely on a runtime `bufferedAmount` property. The browser unwraps and acknowledges data before normal FICS parsing; legacy raw upstream messages remain understood by the browser.
- Wrangler includes the Durable Object binding and SQLite namespace migration. It has only been exercised locally and in deployment dry runs; the production namespace will be created by a future authorized deployment.

Tests exercise global/per-client admission before TCP allocation, rejected/expired sessions, oversized JSON frames, command floods, blocked writers, output budgets, unknown/expired receipts, and guest/registered login plus console commands. A local real Worker WebSocket connection received the bounded envelope, then closed without logging in or playing a game.

## PGN allocation and variation-processing complexity

Finding `csf_8b93cd71972a293e06528398`.

- Dropped PGN/JSON/ZIP files now share the existing file-picker's 64 MiB pre-read bound; expanded ZIP input already has its own bound and integrity checks.
- PGN collection splitting rejects over 64 MiB or 100,000 games. Per-game limits are 1 MiB, 64 nesting levels, 256 KiB per brace comment, and 4,096-byte tokens/tag lines. Mainline validation limits games to 4,096 half-moves.
- The selected game strings are moved into the incremental import queue instead of copied while retaining the selection list. Existing paced conversion and bounded storage batches remain active.
- Saved-game move counting now traverses the notation once, skipping tags, nested comments, semicolon comments and variations. It no longer repeatedly removes innermost variations with whole-string regular expressions. This also handles previously saved deeply nested input safely.
- Original notation, notes, variations, duplicate fingerprints and saved analyses remain unchanged for supported games; errors report the applicable limit instead of truncating a game or partially accepting a collection.

Regression tests cover extreme nesting (including 50,000 levels in legacy saved metadata), game/token/comment/ply limits, ordinary annotated PGN, fingerprints, ZIP importing and Saved Games operations.

## FICS plaintext password hop remains open

Finding `csf_d5352cb6e98979add34dde15` is not resolved by these changes. The browser-to-Worker hop uses WSS in production, but `freechess.org:5000` remains raw TCP. FICS's published connection documentation specifies telnet at that address; no supported certificate-validated FICS TLS endpoint was verified during this review. See <https://www.freechess.org/Help/HelpFiles/addresses.html> and <https://www.freechess.org/login.html>.

Registered login remains available as requested. Disabling it, forcing guests, or adding TLS only to an intermediary while leaving the intermediary-to-FICS hop unprotected would not meet the requested outcome. Resolution requires a verified protected FICS endpoint or a protected transport terminating within FICS's trusted network. No credentials were used in live validation.

## Validation artifacts

- `tmp/security-rust-full.log`: 307 application tests passed, three opt-in benchmarks ignored.
- `tmp/security-node-verified.log`: final JavaScript regression run; bounded parallelism avoids known timing-sensitive process tests competing with compilation.
- `tmp/security-worker-final.log`: Wrangler deployment dry run validates the Worker and Durable Object binding; nothing uploaded.
- `tmp/security-medium-wasm.log` and `tmp/security-medium-site.log`: required WASM rebuild and local site refresh.
- Security-specific tests: `tests/fics-worker-security.test.mjs`, `tests/pgn-complexity.test.mjs`, browser FICS bridge tests, and Rust PGN budget tests.
