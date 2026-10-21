# Phase 5: owner-local endgame tablebases

Companion probes owner-supplied standard-chess Syzygy `.rtbw` (WDL) and `.rtbz`
(DTZ) files. No engine, dataset, downloader or online fallback is included.

## Configure and connect

Add `"tablebase_directories": ["C:/Chess/Syzygy"]` to the native Windows
`config.json` and restart Companion. Relative directories resolve beside that
configuration. Up to 16 directories are scanned nonrecursively, with a combined
10,000-entry budget. Symlink files are skipped. Files are opened read-only using
ordinary I/O, without memory mapping. Install all successor materials needed by
the positions you want to probe, including capture and promotion successors.

For Docker, mount your directory read-only, e.g.
`./syzygy:/tablebases:ro`, and configure `["/tablebases"]`. The existing Docker
recipe has not been runtime tested here; its memory budget may need adjustment
for your dataset. Keep files stable during a probe. Replace a dataset between
requests, or stop Companion while updating it.

In Ironwood's endgame tablebase panel choose **Local Companion**, then
**Connect Companion / inspect local coverage**. Enter the Companion address and
owner credential, inspect registered materials, close the connection window and
choose **Look up this position**. Existing in-memory archive/engine connections
are also usable. Credentials are never persisted by this feature. **Lichess
online** remains a separate explicit source. There is no automatic fallback.
The local selection persists when moving to a new game position.

Owner directories are also passed to engines advertising a string `SyzygyPath`
option during their UCI handshake. An explicit owner-configured engine option
wins. Unsupported engines receive no extra option. Remote clients still cannot
set path/string options. Engine-internal probes and this UI service are separate.

## Authenticated protocol v1

Both routes require the usual exact Origin, owner bearer credential and protocol
header, and use the existing request rate/deadline controls:

- `GET /api/v1/tablebases`: per-material registered extensions, rejected-file and
  unavailable-directory counts. Registration validates filename and size; it
  does **not** certify checksum, contents or complete N-piece coverage.
- `GET /api/v1/tablebases/probe?fen=<complete-FEN>`: root category, signed DTZ,
  `dtzRounded`, nullable DTM (always null), legal UCI/SAN continuations, zeroing
  flags, source `companion`, and coverage report. Continuation values describe
  the side to move **after** that move, matching the existing panel contract.

Categories include win/loss/draw, cursed-win/blessed-loss, and maybe-win/maybe-loss
for rounding ambiguity. The library accounts for the halfmove clock. Cursed wins
and blessed losses distinguish theoretical results from fifty-move-rule results.
DTZ is distance to a zeroing move or mate, not DTM; rounded values are identified.
Castling rights and positions above seven pieces are unsupported; legal en
passant is passed through unchanged. Invalid FENs are rejected.

`disabled`, `busy`, `invalid_position`, `unsupported_position`, `missing_table`
and `corrupt_table` are distinct errors. A failed successor probe fails the
request instead of fabricating a continuation. Unreadable/corrupt table contents
return `corrupt_table`; invalid filenames/sizes are reported as rejected coverage
entries and cannot satisfy a required material. Missing directories do not prevent
other Companion services from starting.

Only one tablebase operation runs at once, outside Tokio's async executor.
Shutdown waits for its permit. Each request constructs new file handles and
results; there is deliberately no cross-request position/file cache. Dataset
changes therefore invalidate previous answers automatically, including prior
errors. Browser repaint polling only retains the current request's response;
every new lookup or refresh probes again. Capabilities expose configured-service
availability and registered filenames; the coverage route is authoritative for
per-material detail. Busy capability queries may omit the filename snapshot.

## Validation and remaining release acceptance

Automated checks cover authenticated HTTP, bare-king draw and child perspective,
illegal FEN/castling rejection, legal EP acceptance, halfmove/rounding category
boundaries, partial coverage, corrupt headers, removed/replaced files, owner-only
UCI path forwarding, stale browser responses, local authentication and absence
of online fallback. Corrupt fixtures are original zero-filled invalid bytes,
created and removed by tests; they are not a tablebase dataset.

Selected three-to-five-piece real-data validation is now recorded in
[Phase 6 validation](phase-6-tablebase-validation.md), including independently
verified dataset terms and python-chess comparisons. Six/seven-piece datasets,
Linux Docker and large-dataset latency/memory/shutdown measurements remain
release checks. Dataset terms are independent of probing-library code licenses.

Validation on 2026-10-17: 18 native unit tests; 22 combined Companion/tablebase
Node tests; 269 browser-application Rust tests passed (3 existing ignored).
Server/protocol Clippy passed with warnings denied. The WASM bundle was rebuilt
and preview assets refreshed. An isolated browser on 8792 connected to disposable
Companion 8793, inspected empty coverage, loaded an original bare-kings PGN,
retained the local source across the position change, and displayed a local draw
with two legal continuations. Proof images are under
`tmp/companion-phase5-browser/`. No real tablebase dataset was used.
