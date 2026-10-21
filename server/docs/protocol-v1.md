# Companion protocol v1: foundation, UCI and archive

Status: experimental. The foundation, UCI and archive subset below is implemented. Breaking
changes to this subset require a protocol-version change. Additive fields may be
ignored by clients. Shared Rust types live in `crates/ironwood-protocol` and build
for both native Rust and WASM without server dependencies.

## HTTP

Every request requires an exact configured `Origin`. Non-browser tools must
explicitly supply an allowed Origin as well as authentication; Origin alone
provides no authorization. There are no wildcard origins or credential cookies.
No account registration or engine work is exposed without authentication.

For implemented HTTP resources send:

```text
Authorization: Bearer <owner credential>
X-Ironwood-Protocol: 1
Origin: https://ironwoodchess.com
```

Origin is checked first (403), protocol compatibility second (426), and the
credential third (401). Successful and allowed-origin error responses have
exact-origin CORS and `Cache-Control: no-store`. Preflights allow GET/POST and
Authorization, Content-Type, X-Ironwood-Protocol only. Preflights do not require
the credential and do not perform any service action.

| Method/path | Response |
| --- | --- |
| GET `/api/v1/capabilities` | Product, server/protocol version, service flags, engine IDs, optional archive schema version, tablebase coverage |
| POST `/api/v1/tickets` | `{ "ticket": "<opaque>", "expires_in_seconds": 30 }` |
| GET `/api/v1/socket` | Origin-checked WebSocket upgrade; first frame must authenticate |
| POST `/api/v1/shutdown` | Authenticated graceful stop; returns 202 |

GET `/healthz` and `/readyz` are minimal unauthenticated operational probes outside
the versioned API. Readiness is 503 while stopping. They expose no configuration
or user data. `ironwood-server healthcheck` uses the readiness route.

Capabilities reports registered engine IDs and `uci: true` when engines exist;
tablebase coverage lists registered material filenames; archive is enabled by default with schema 2. The opening
explorer is enabled with the archive; tablebase service is enabled when owner directories are configured. Do not confuse
successful transport with readiness of a service.

HTTP errors use `{ "code": "...", "message": "..." }`. Implemented codes are
`origin_denied`, `preflight_denied`, `protocol_mismatch`, `unauthorized`,
`busy`, `random_unavailable`, and `not_found`. Unknown authorized routes return
404. Handler/extractor protocol errors may use the framework's status response;
the foundation normalizes method and malformed upgrade rejections to structured
errors. Added codes include `method_not_allowed`, `invalid_request`,
`request_too_large`, `request_timeout`, `rate_limited`, and `shutting_down`.
Non-archive API endpoints accept no request body. Requests are bounded by configurable
size/deadline limits and a per-peer fixed-window HTTP budget.

## WebSocket

Browser WebSocket constructors cannot supply an Authorization header. Obtain a
ticket through authenticated HTTP, upgrade with no credential in the URL, and
send within five seconds:

```json
{"type":"authenticate","ticket":"<ticket from HTTP>"}
```

The ticket is 32 random bytes encoded as hex, bound to the requesting Origin,
valid for 30 seconds, and consumed once. Ticket storage is process-local and
lost on restart. A wrong-origin request cannot consume a valid ticket. There
are by default at most 64 pending tickets and 16 concurrent WebSocket connections, including
unauthenticated upgrades. Frames/messages default to 4096 bytes. These bounds and
authentication/idle/shutdown deadlines are configurable. No work is
performed before ticket acceptance. A missing, invalid, expired, or replayed
ticket produces an `unauthorized` error followed by close.

Successful authentication:

```json
{"type":"ready","protocol_version":"1"}
```

The only subsequent command is:

```json
{"type":"ping","request_id":"client-generated ID, at most 128 bytes"}
```

Response:

```json
{"type":"pong","request_id":"the same client-generated ID"}
```

Unknown/invalid commands return an `invalid_request` error. Idle connections
expire after 30 seconds. Close/restart never restores a session or replays work.
Each engine result includes session/search/position identity; raw
browser-supplied UCI commands or filesystem paths will not be a public API.

## UCI sessions

Authenticated GET `/api/v1/engines` returns registered IDs and labels. After the
WebSocket `ready` response, send `open_engine` with `request_id` and `engine_id`.
`engine_opened` returns the same request ID, an opaque `session_id`, and engine
identity/options (name, kind, default, min, max, choices).

```json
{"type":"search","session_id":"session","search_id":"unique-search","position_id":"client-position","fen":"rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1","moves":[],"chess960":false,"new_game":true,"options":{"Threads":"1"},"limit":{"kind":"time","milliseconds":1000}}
```

Limits also support `depth`/`plies`, `nodes`/`nodes`, `infinite`, and `clock` with
`white_ms`, `black_ms`, `white_increment_ms`, `black_increment_ms`. Searches are
serialized per session; search IDs cannot be reused. Responses `search_started`,
`info` and `bestmove` repeat all three IDs. Info contains optional depth, nodes,
multipv, score and legal PV. Scores use `kind: cp|mate`, numeric value,
`perspective: side_to_move` and optional `bound: lowerbound|upperbound`.
Bestmove contains a legal UCI move, or null when no move exists.

`cancel` takes session/search IDs; after draining the engine's bestmove the server
returns `cancelled` without publishing that bestmove. Clients must suppress late
results immediately when requesting cancellation. `close_engine` takes session ID
and returns `engine_closed` after process cleanup. Disconnect/shutdown also closes
the process. Errors include `invalid_request`, `busy`, and `engine_failed`; fatal
engine failures require opening a new session. No automatic search replay occurs.
See the shared Rust enums for the complete machine-readable wire contract.

Authenticated owners may send `set_option` with `session_id`, a unique `request_id`,
an advertised `name`, and string `value` while the engine is idle. Success returns
`option_set` with matching session/request IDs. Button values are empty strings;
actions are applied once and must not be persisted in the search options map.
Spin/check/combo/string values obey the same validation as search overrides.
Newlines/NULs, unsupported names, and out-of-range resources are rejected. Chess960
and pondering remain protocol-managed. String paths refer to the Companion host.

## Persistent archive (schema 2)

Archive endpoints use the same exact-Origin, protocol-header and owner-credential
checks. All mutations use POST; no extra CORS methods/headers are necessary.
JSON requests are bounded to 16 MiB; see [archive operations](phase-3-archive.md).

| Method/path under `/api/v1/archive` | Request / response |
| --- | --- |
| GET `/games` | Query `after`, `limit` (1â€“100, default 50), `white`, `black`, `date_from`, `date_to` (PGN date form), `result`, `opening`, `profile`, `collection`; returns `{games:[summaries],next:cursor-or-null}` |
| GET `/games/{id}` | Complete game `{id,revision,created_at,updated_at,document}` |
| POST `/import` | `{documents:[...],collection?:id}` OR `{pgn:text,collection?:id}`; returns `{imported,duplicates,ids}` |
| POST `/games/{id}/update` | `{revision,document}`; returns updated game |
| POST `/games/{id}/delete` | `{revision}`; deletes only the archive game and memberships |
| GET `/collections` | Array of `{id,name,revision}` |
| POST `/collections` | `{action:"create",name}` |
| POST `/collections/{id}` | `{action:"rename",revision,name}`, `{action:"delete",revision}`, or `{action:"add"|"remove",revision,games:[ids]}` |
| POST `/export` | `{ids:[...],format:"json"|"pgn"}`; returns transfer envelope or `{pgn:text}` |
| GET `/backup` | Consistent snapshot of all games, collections and memberships |
| POST `/restore` | Backup envelope; archive must be empty |

A game document is `{format:"ironwood-companion-game",version:1,record:null|originalBrowserRecord,pgn:text,metadata:{tag:string,...}}`.
Metadata keys include White, Black, Result, Date, Opening, ECO, Variant, FEN and
TrainingProfileId when available. Record JSON is preserved as its original string.
JSON export uses `{format:"ironwood-companion-transfer",version:1,documents:[...]}`.
Backups use `{format:"ironwood-companion-archive",schema_version:2,games:[...],collections:[...],memberships:[[collectionID,gameID],...]}`.

IDs are immutable. Update/delete and collection edits return 409 `conflict` when
the revision is stale. Other archive errors include `unsupported_version`,
`invalid_request`, `not_found`, `archive_disabled`, `busy`, and `archive_failed`.
Validation errors never partially import a batch. Retries of identical imports
are idempotent; no automatic write replay or bidirectional synchronization occurs.

## Archive opening explorer

GET `/api/v1/archive/explorer`, GET `/api/v1/archive/explorer/status`, and POST
`/api/v1/archive/explorer/rebuild` are implemented. They use the archive's existing
authentication and carry explicit index generation/progress. See the
[query contract and counting rules](phase-4-explorer.md#http). The derived index
version is independent of the source archive's schema version.

## Future resource families (not implemented)

- Tablebase queries specify FEN/rule counters and return coverage/source/generation.
- Job resources expose bounded progress, cancellation, and terminal results.

Define precise schemas when implementing each phase, with explicit service flags.
These are design reservations, not endpoints clients can call today.

## Local tablebases

Phase 5 adds authenticated GET `/api/v1/tablebases` and
`/api/v1/tablebases/probe?fen=...`. See [the tablebase contract](phase-5-tablebases.md)
for coverage, errors, perspective, fifty-move and rounding semantics.

### Independent HTTP request budgets

Fixed one-minute windows are tracked per client IP and budget.
`limits.requests_per_minute` retains its default of 240 for operations, including
imports, mutations, backups, engine registration and connection tickets.
`limits.archive_reads_per_minute` defaults to 1200 for archive GET requests
(excluding backup) and the read-only POST `/api/v1/archive/export`.
`limits.preflight_requests_per_minute` defaults to 2400 for OPTIONS requests.
Existing configuration files inherit both new defaults without editing.
Each budget accepts 1–10000 requests per minute and tracks at most 1024 client IPs.
Health/readiness probes remain exempt. Authentication, allowed origins, protocol,
body/deadline limits and engine concurrency limits still apply.

A 429 response includes `Retry-After` in seconds until that window resets; CORS
exposes this header so browser clients can wait and retry. Exhausting archive
reads or preflight checks does not spend the operations budget, or vice versa.

### Paged Saved games library

GET `/api/v1/archive/library` serves compact card summaries without PGN, saved
JSON, analysis arrays, or move histories. Parameters: `category` (all, mine,
training, puzzles, observed, imported, favorites), `player` (exact name, `*` for
all, `?` for unknown), `profile`, `search`, `analysis` (*, complete, partial,
unanalyzed), `sort` (recent, moves-asc, moves-desc), zero-based `page`, and `limit`
(default 10, maximum 100). Sort ties use saved time descending, then archive ID.
The response contains games, total matching count, the clamped page number,
global category/favorite counts, category player names, and training profiles.
Counts and pages are read in one transaction.

`selection=true` returns only IDs, revisions and titles, with 100 matches per
page, for explicit Select all matching. Fetch full documents through existing
detail/export routes only when opening, previewing, editing, exporting or
transferring games. Browser storage retains its existing local behavior.

A rebuildable SQLite library index stores card fields and sort indexes; triggers
track imports, edits, restores and deletions. Existing archives backfill this
derived index at startup, streaming documents one at a time. Subsequent reads
process only dirty records. Original documents and the archive backup format
remain unchanged. Offset pages reflect current archive contents, so concurrent
edits can shift page boundaries; revision checks still protect mutations.

Library responses also include a monotonic `revision`. A client holding counts,
players and profiles for the requested category can send `facets_revision` with
that revision. When unchanged, the response omits those three fields; pages still
include their matching total. A changed archive automatically returns fresh
facets. Selection-only requests omit facets.

Companion caches matching totals by normalized filters independently of page and
sort, and caches category facets separately. The derived cache is bounded to 128
entries; game insert/update/delete triggers clear it and advance the revision in
the same transaction. Browser facets and page summaries have separate bounded
caches. After a foreground page, Ironwood speculatively fetches only its next page
without full game data or automatic retries. Navigation, connection changes and
mutations prevent stale speculative responses from entering the page cache.

### Per-game analysis runs (private preview)

Archive schema 3 adds `analysis_runs` with a foreign key to the game. Migration
creates a pre-upgrade SQLite snapshot. Game documents, annotations, and content
fingerprints stay separate from analysis-run payloads.

- GET `/api/v1/archive/games/{id}/analysis-runs`: metadata only, newest first.
- GET `/api/v1/archive/games/{id}/analysis-runs/{run}`: one complete result payload.
- POST to that run path: `{expected_revision,metadata,payload}`. Revision zero
  creates a run; updates require its current revision. Conflicts return 409.

Metadata records the engine label/identity, configured UCI values, search budget,
verification mode, parent run, completed positions, and completion status. Run
payloads contain position analysis and verification state; game notes, ratings,
moves and board annotations are not copied into them. Default engine network
identity is not inferred when it is absent from configured engine options.

Starting or resuming analysis and manual verification creates a new run. Changing
engine/settings before continuing starts a fresh full analysis to avoid mixing
engines in one run. Existing unattributed results are preserved as a legacy run
before replacement. Selecting a run changes displayed results without changing
the current engine or game annotations. Historical runs are loaded on demand.

JSON archive transfer documents may contain `analysis_runs`; imports merge these
without changing game identity or overwriting conflicting run IDs. Native/JSON
archive backups include runs; schema 2 JSON backups remain readable. Deleting a
game removes its runs transactionally. All routes retain existing owner, origin,
protocol, body, timeout and budget guards. Game Analysis controls remain behind
the shared Companion flag and appear only for an active archived game.
