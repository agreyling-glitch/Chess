# Ironwood Companion implementation plan

Status: Phases 1–5 are implemented in the working tree and validated on the Windows
development machine with native integration and isolated browser checks. Linux
Docker, clean-machine Windows, real engine binaries, six/seven-piece tablebases, and deployed-origin browser
acceptance remain pending. See [Phase 1 validation](../../server/docs/phase-1-validation.md),
[UCI validation](../../server/docs/phase-2-uci.md), and
[archive operations/validation](../../server/docs/phase-3-archive.md), and
[archive explorer](../../server/docs/phase-4-explorer.md), and
[local tablebases](../../server/docs/phase-5-tablebases.md).
Created: 2026-10-17

Product name: **Ironwood Companion**. Keep `ironwood-server` as the proposed
executable and container image basename, and retain this plan's filename.

## Purpose and scope

Build Ironwood Companion, an optional native Rust companion server for Ironwood Chess. Ship Linux
Docker images and a native Windows executable. Provide four services through one
authenticated interface: a UCI engine bridge, a persistent game archive, an
opening explorer over that archive, and local endgame tablebase probing.

Ironwood remains usable without the server, including its browser Stockfish
engine and local saved games. Connecting a server must be an explicit choice.
The first release targets one owner with multiple browser clients, rather than
a public multi-user hosting service.

## Existing integration points

- The browser application uses Rust/WASM and egui. Stockfish runs in a browser
  Worker. Preserve that engine path and add a selectable server engine provider.
- `web/game-storage.js` manages browser game storage. The server archive is an
  additional destination, not an automatic replacement or destructive migration.
- Opening Explorer already queries Lichess. Add an Archive source alongside the
  existing sources, preserving its filters and preview-navigation conventions
  where applicable.
- Tablebases already queries the Lichess HTTP API. Add a local server source,
  preserving exploration branches, counters, result perspective, and DTZ labels.
- Existing restrictions on assistance during training and online play also apply
  to server analysis, explorer results, and direct tablebase lookups. An engine
  opponent may use its own configured book or tablebases internally.

## Architecture

```text
Ironwood browser application (Rust/WASM + JavaScript)
    |
    | authenticated versioned HTTP + WebSocket
    v
Ironwood Companion (native Rust)
    |-- connection, authentication, capabilities, job supervision
    |-- engine sessions --------> configured native UCI processes
    |-- archive ----------------> database + managed game/analysis data
    |-- opening explorer -------> rebuildable position/move index
    `-- tablebase service ------> user-mounted local tablebase files
```

### Code organization

Keep the implementation in this repository initially. Proposed layout:

- `server/`: native Rust binary, configuration, migrations, API, and services.
- `crates/ironwood-protocol/`: small shared request/response types and protocol
  versioning, compiled for native Rust and browser WASM.
- `server/tests/`: protocol, persistence, engine-process, and deployment tests.
- `deploy/ironwood-server/`: Dockerfile, Compose example, Windows startup example,
  volume layout, and deployment instructions.

Introduce Cargo workspace changes only when implementation starts. Keep native
server dependencies out of the browser build. Share UCI parsing only where its
semantics fit both providers; do not make the server depend on egui or the UI.

### Connection and API

- Use `/api/v1` for HTTP resources and a versioned WebSocket endpoint for engine
  commands and streamed events. WebSocket frames carry structured messages with
  request IDs and engine session/search IDs; the bridge translates to UCI lines.
- Initial capabilities report protocol compatibility, enabled services, available
  engine IDs, archive schema version, and tablebase coverage. Negotiate optional
  fields instead of assuming every server has every service enabled.
- HTTP operations cover archive CRUD, collections, imports/exports, explorer
  queries, tablebase probes, and job status. Long imports/indexing use cancellable
  jobs with progress and bounded resource use.
- Errors distinguish invalid input, unauthorized access, unsupported capability,
  busy engine, missing data, incompatible protocol, and internal failure.
- Disconnects invalidate active search IDs. Reconnect never silently replays a
  move, repeats an import, or starts another engine process.

### Security and operating modes

- Native Windows defaults to loopback. Docker listens inside the container and
  its default Compose mapping publishes only to the host's loopback interface.
- Generate an owner credential on first start, store it outside the image, and
  provide rotation. Avoid credentials in query strings, URLs, or logs. Browser
  WebSocket authentication must use an explicit supported flow, such as a
  short-lived single-use connection ticket obtained through authenticated HTTP.
- Check browser Origin against configured Ironwood origins for HTTP and WebSocket
  requests. CORS is not authentication. Authenticate before allowing engine work.
- Engines are configured locally by the owner using an executable path and an
  argument list. Browser clients select engine IDs; they cannot supply shell
  commands, executable paths, or unrestricted filesystem paths.
- Scope file access to configured roots. Validate imports, filenames, sizes,
  pagination limits, and path traversal. Serve no arbitrary host files.
- Treat engines as trusted software installed by the owner: a child process is
  not a security sandbox. Run the server/container without administrator/root
  privileges and enforce process, memory, thread, and request limits.
- LAN/remote access is opt-in, with authenticated HTTPS/WSS through a documented
  deployment. Verify browser secure-context, local-network permission, mixed
  content, WebSocket, and cross-origin isolation behavior against the actual
  deployed Ironwood app. Do not assume an HTTPS page can reach any local address.

### UCI bridge

- Spawn engines directly with piped stdin/stdout; capture bounded stderr logs
  separately. Handle `uci`, advertised options, `isready`, `ucinewgame`,
  `position`, `go`, `stop`, and `quit` with timeouts and orderly shutdown.
- Discover identity, option types/defaults/ranges, Chess960 capability, and
  supported search limits. Never send Stockfish-only options indiscriminately.
- Support play and analysis, MultiPV where available, and streamed scores,
  depth, nodes, principal variations, and best moves. Normalize score perspective
  explicitly and retain mate scores as mate scores.
- Give each active client session its own engine process within a configurable
  concurrency limit. Serialize searches within a process. Associate every result
  with a position/search ID so late output cannot alter another position or game.
- Stop searches on cancellation/disconnect, terminate hung engines after a grace
  period, and reap processes. Crashes produce actionable errors without taking
  down the archive service. Do not restart and play a move automatically.
- Keep provider selection explicit. If a server engine disconnects during play,
  pause and offer recovery rather than silently switching engine or strength.
- Existing personal training ratings are calibrated around Stockfish targets.
  Keep rated training on its current engine initially; alternative engines are
  available for casual play/analysis until separate rating rules are designed.

### Game archive

- Use embedded SQLite through the MIT-licensed `rusqlite` adapter, with bundled
  public-domain SQLite. Keep the database on local server
  storage, with one owning server process. Network-mounted database files are
  outside the supported first-release configuration.
- Preserve full Ironwood game data, annotations, analysis, engine provenance,
  variant, starting FEN with counters, moves, result, and source metadata. PGN is
  an interchange format, not a replacement for lossless Ironwood data.
- Assign immutable archive IDs, content fingerprints for import deduplication,
  revisions for edits, and collection membership. Keep distinct annotated versions
  when merging would lose data; a position/move match alone must not erase notes.
- Initial transfer is explicit upload/download/import/export. Browser data stays
  intact after upload. Automatic bidirectional synchronization is deferred.
- Support paginated search by players, dates, result, collection, and opening
  metadata where available. Use optimistic concurrency to reject stale edits.
- Treat deletion of a game, collection membership, and training-profile rating
  history as separate operations. Preserve current browser semantics.
- Provide consistent backups and tested restores, including schema version and
  required files. Database migrations require a recovery path and a pre-upgrade
  backup. Reject unsupported newer schemas without changing them.

### Opening explorer

- Build a rebuildable index from archived standard-chess games. No bundled
  third-party game database is required for the first release.
- Define a canonical position key using piece placement, side to move, castling,
  and legal en-passant availability. Exclude move counters for opening statistics;
  preserve them in the source game. Keep variants separate.
- Aggregate continuation counts, White wins/draws/Black wins, and matching games.
  Count each game once per position/continuation, even when a repetition revisits
  it. Unfinished/unknown results may contribute occurrence counts but must be
  reported separately and excluded from completed-result percentages.
- Support collection, player/color, date, and rating filters where metadata
  exists. Report missing metadata and index progress instead of inventing values.
- Handle transpositions, archive additions, edits, and deletions consistently.
  Expose index generation so clients invalidate stale cached statistics.
- Opening-book reading is a later extension. Select a format after checking its
  implementation licensing; keep user-supplied book files separate from archive
  statistics and label book weights differently from game counts.

### Endgame tablebases

- Probe user-supplied local Syzygy files through a Rust-compatible probing
  implementation selected only after a code-license review. Tablebase file terms
  are checked independently before any recommendation or redistribution.
- Report installed material coverage and distinguish unsupported positions,
  missing files, corrupt files, and valid draw results. Partial installations are
  supported; do not advertise a complete piece-count set from one file.
- Return legal continuations, WDL categories, DTZ and rounding information when
  available. Honor the halfmove clock and explain cursed wins/blessed losses.
  Do not label DTZ as distance to mate or promise DTM when it is unavailable.
- Validate standard positions, castling/en-passant handling, and required
  successor tables. Cache by all inputs affecting the result, including relevant
  rule counters and the installed dataset generation.
- Let compatible server engines use the same configured tablebase folders through
  their advertised options. Direct UI probing and internal engine probing remain
  distinct capabilities. No automatic third-party API fallback or download.

## Packaging and storage

Linux Docker is the first container target, initially `linux/amd64`, followed by
`linux/arm64` after native validation. Publish versioned tags and immutable digests.
The image contains the server and approved runtime dependencies, with no bundled
engine executables, networks, opening books, or tablebase datasets initially.

| Container path | Purpose | Access |
| --- | --- | --- |
| `/data` | Database, configuration, credentials, managed archive data | Read/write |
| `/data/engines` | Owner-installed Linux engines and associated files | Read-only mount by default |
| `/books` | Optional user-supplied opening books | Read-only |
| `/tablebases` | User-supplied local tablebases | Read-only |

Engines must match the container OS/CPU and its runtime libraries, regardless of
the host desktop OS. Document supported engine runtime requirements; matching
Linux/CPU alone is insufficient. Avoid CPU emulation for engine-performance
validation. GPU engines are deferred until explicit device support is designed.

Native Windows starts as a portable x64 executable with configuration and data
in an explicit user-writable directory. No Rust or Python installation is required.
Handle paths with spaces/non-ASCII characters and launch child engines without
visible console windows. Service installation and tray UI are later conveniences.

Ship the same protocol and capabilities on both platforms. Support graceful stop,
health/readiness checks, redacted logs, configurable resource limits, and documented
backup/upgrade procedures. Persistent volumes survive container replacement;
backups must also protect against volume deletion or disk loss.

### Release distribution

Distribute Ironwood Companion through the existing Ironwood website and separate
artifact hosting. Cloudflare delivers downloads; users run Companion on their own
machines. Engine processes, archives, books, and tablebases remain on the user's
server unless the user explicitly configures another destination.

| Item | Distribution location |
| --- | --- |
| Companion download page, release notes, setup instructions | Existing Cloudflare-hosted Ironwood website |
| Native Windows ZIPs, SHA-256 checksums, release manifests, and signatures when available | Cloudflare R2 through a custom download domain |
| Multi-platform Linux Docker images | GitHub Container Registry (GHCR) |
| Source, tags, release history, and corresponding source required by licenses | GitHub |

Use `downloads.ironwoodchess.com` as the proposed R2 custom domain; provisioning
and account access are implementation work, not completed configuration. Confirm
the GHCR namespace before publishing an image reference. These are publication
destinations, not locations for user archive storage.

- Build release artifacts in CI for each supported platform. Publish immutable
  versioned object paths, image tags, and image digests tied to a source commit.
- A release manifest identifies product/version, supported OS/CPU, artifact URLs,
  sizes, hashes, protocol compatibility, and release notes. Publish artifacts
  before updating any stable/latest pointer, avoiding partial releases.
- Serve R2 downloads over HTTPS with suitable content types, download filenames,
  and cache headers. Keep immutable artifacts long-lived and release pointers
  refreshable. Verify actual downloads and hashes through the public domain.
- Keep credentials and signing keys in CI secrets with narrowly scoped publishing
  access. Checksums detect corruption; signed manifests/artifacts add publisher
  authentication. Never treat a checksum alone as a signature.
- Native Windows private-preview builds provide an explicit signed update check
  and idle-only Update and restart, with retained previous builds and archive
  snapshots. No updates install without the user's action. A signed Cloudflare
  feed and pinned publisher certificate must be provisioned before enabling
  delivery; see [Windows updater operations](../../server/docs/windows-updates.md).
  Rollbacks must respect archive schema compatibility and restore a tested backup.
- Keep licenses, notices, and required corresponding-source access linked from
  the release page and shipped artifacts. Docker base/runtime components remain
  part of the distribution licensing review.

Reference: [R2 custom-domain public downloads](https://developers.cloudflare.com/r2/buckets/public-buckets/).

## Licensing gate

Follow repository licensing requirements before selecting third-party crates,
runtime images, engine examples, probing code, datasets, books, or weights. Record
exact versions, licenses, provenance, notices, source obligations, and asset terms.
Prefer explicit licenses with straightforward compliance. No dependency or dataset
is approved merely by appearing in this plan.

A generic connection to an independently installed engine is distinct from
redistributing it. Users supply their engines and data; Ironwood remains
responsible for everything included in its own executable/image. Reassess terms
before introducing bundled downloads, recommended assets, or hosted engines.

## Implementation phases and acceptance checks

### Phase 0: protocol and deployment feasibility

Implementation: `server/`, `crates/ironwood-protocol/`, the standalone
`web/companion-check.html` probe, and `deploy/ironwood-server/`. The experimental
[protocol](../../server/docs/protocol-v1.md), [configuration schema](../../server/config.schema.json),
[threat model](../../server/docs/phase-0-design.md), and [licensing inventory](../../server/docs/licensing.md)
are recorded. Service capabilities remain disabled until their implementation phases.

Define the versioned protocol, configuration schema, service capabilities, threat
model, browser connection flow, and licensing inventory. Confirm integration with
the existing archive/export formats and assistance restrictions.

Acceptance:
- A minimal native Rust service connects to the deployed browser app on supported
  Windows and Linux Docker setups, with documented browser permissions/TLS needs.
- Valid pairing succeeds; wrong credentials, disallowed origins, expired tickets,
  and unsupported protocol versions are rejected clearly.
- Dependencies and base image have recorded, acceptable licenses.

### Phase 1: server foundation and packaging

Implementation includes native initialization/first-start provisioning and
stop-before-rotation, data-directory locking, health/readiness, configurable
budgets/deadlines, redacted JSON logs, coordinated shutdown, a persistent-volume
Docker recipe, and Windows development ZIP packaging with matching vendored source.
See [operations](../../server/docs/phase-1-operations.md). Platform acceptance
remains open where the [validation record](../../server/docs/phase-1-validation.md)
states it has not been exercised. No image or executable was publicly published.

Implement configuration, owner credentials, capabilities, health/readiness,
structured errors, limits, logging, graceful shutdown, Windows artifact, and Linux
x64 image/Compose template.

Acceptance:
- Clean Windows and Linux Docker installations start using documented steps.
- Default deployment is accessible locally and not published to the LAN.
- Configuration/data survive restart and container replacement; logs redact secrets.
- Disabled services are reported accurately and do not prevent enabled services
  from operating. Shutdown cancels jobs and releases files/ports.

### Phase 2: UCI engine bridge and browser provider

Implemented in the working tree. Windows native fixture/process tests and browser
analysis are covered; Linux Docker, real owner-provided engines, and the deployed
HTTPS browser matrix remain release acceptance checks. See
`server/docs/phase-2-uci.md` for setup, scope, and validation commands.

Implement engine registration, discovery, sessions, search streaming/cancellation,
and browser selection for casual play and analysis.

Acceptance:
- A deterministic fake UCI process covers handshake, options, partial output,
  delayed bestmove, malformed lines, crash, stop timeout, and process cleanup.
- Owner-provided engines work on Windows and Linux without shell interpretation;
  unsupported options are not sent and out-of-range settings are rejected.
- Play applies exactly one legal move for the current search; stale output after
  stop, navigation, reconnect, or a new game never changes the board.
- Concurrent clients remain isolated; limits prevent unbounded engine processes.
- Browser Stockfish still works with no server. Existing online/training
  assistance restrictions hold. Disconnection offers explicit recovery.

### Phase 3: persistent archive and transfers

Implemented in the working tree with SQLite schema 2, explicit browser transfers,
PGN/JSON import/export, collections, revision checks, cursor search and consistent
JSON/native SQLite snapshots. See `server/docs/phase-3-archive.md` for operation
and validation. Linux Docker, clean-machine packaging and deployed browser checks
remain release acceptance work.

Implement schema/migrations, collections, CRUD, lossless browser transfers, PGN
import/export, deduplication, paginated search, and backup/restore.

Acceptance:
- Round trips preserve moves, initial FEN/counters, comments, drawings, analysis,
  engine provenance, and supported metadata; unsupported versions fail clearly.
- Repeated imports are idempotent; interrupted imports leave no partial games.
- Two clients editing one revision receive a conflict instead of silent data loss.
- Deletions honor collection/profile boundaries. Browser originals remain intact.
- Restore a backup into a clean server and compare archive contents. Test upgrade
  recovery and rejection of a newer unsupported schema.

### Phase 4: archive opening explorer

Implemented with a versioned derived index, background per-game jobs, position and
continuation statistics, filters, matching games and the browser Archive source.
See `server/docs/phase-4-explorer.md` for counting rules and validation. Release
checks for Linux/deployed browser support and large-archive performance remain.

Build the position index, filters, move statistics, matching-game retrieval, and
browser Archive source.

Acceptance:
- Fixtures cover transpositions, repetitions, castling and en-passant rights,
  setup FENs, unfinished results, duplicate imports, and missing metadata.
- Counts and percentages match independently calculated fixture expectations.
- Adding/editing/deleting games updates statistics; rebuilding gives the same
  result. Partial indexing is visibly labeled and does not block archive reads.
- Continuation previews preserve the actual game and existing navigation behavior.

### Phase 5: local endgame tablebases

Implementation is available; see [Phase 5 setup and validation](../../server/docs/phase-5-tablebases.md).
Selected three-to-five-piece real-data comparisons passed on Windows x64; see
[Phase 6 evidence](../../server/docs/phase-6-tablebase-validation.md). Linux Docker
and six/seven-piece acceptance remain pending.

Implement coverage discovery, probing, cache invalidation, structured results,
browser source selection, and compatible engine tablebase configuration.

Acceptance:
- Use documented test data with verified permissions; no unreviewed datasets
  enter the repository or images.
- Compare known win/draw/loss, cursed/blessed, halfmove-clock, and DTZ cases with
  an independent trusted reference; test en-passant and castling restrictions.
- Missing/corrupt tables yield errors rather than fabricated outcomes. Dataset
  changes invalidate caches. Root and move result perspectives remain correct.
- Existing Lichess lookup remains available as an explicitly selected source.

### Phase 6: release hardening and optional extensions

In progress. The Windows development real-tablebase gate passed: 136 root
positions and 1,384 continuations against python-chess, with dataset permissions
and hashes recorded. A checkmate/clock-boundary bug was fixed. See
[the reproducible validation record](../../server/docs/phase-6-tablebase-validation.md).
Portable Windows development-package checks now pass with static CRT linkage
and an offline source rebuild. Clean Windows VM and Linux Docker/NAS execution
remain pending; see [platform validation](../../server/docs/phase-6-platform-validation.md).
Load and publication gates below remain open.

Validate limits/performance, documentation, upgrade paths, and release artifacts.
Add ARM64 Docker only after real ARM64 testing. Consider user-supplied opening
books, Windows background service support, and remote deployment templates.

Acceptance:
- Run the same service/protocol suite on Windows x64 and Linux x64; add ARM64
  coverage before advertising it. Test an engine and persistent archive on each.
- Test representative large PGN imports, paginated searches, index rebuilds,
  simultaneous analysis/probes, and shutdown under load. Record dataset size,
  hardware, latency, memory, and disk use; set release budgets from measurements.
- Confirm restore, credential rotation, non-root execution, process cleanup,
  configured resource limits, and version compatibility.
- Rebuild browser changes with `npm run build:wasm`; refresh preview assets when
  testing through Wrangler. Perform focused browser checks for changed flows.
- Publish notices, supported-platform/runtime requirements, Compose configuration,
  backup instructions, and known limitations with each release.
- Validate the Companion download page, R2 custom-domain Windows download,
  published manifest/checksum, and GHCR image pull by version and digest from a
  clean machine. Verify that all release artifacts resolve to the same source
  version and that incomplete publication never advances the release pointer.

## Deferred decisions

- Exact Rust HTTP/WebSocket, database adapter, and tablebase crates, following
  license and compatibility review.
- Browser connection details and optional same-origin app hosting, decided from
  Phase 0 connectivity tests rather than assumed localhost behavior.
- Minimum Windows/Linux versions, image base, CPU baseline, and measured archive
  size/performance targets.
- Opening-book formats and whether they drive casual engine play, explorer
  suggestions, or both.
- Automatic synchronization/conflict merging, public multi-user accounts,
  internet hosting, GPU engines, native macOS packaging, and engine tournaments
  are outside the initial release.
