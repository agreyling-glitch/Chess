# Scid import validation — 2026-10-24

## Implementation

Scid 4 retains the 64 MiB conversion path and existing PGN selection dialog.
Browser Scid 5 imports retain File objects, load the name table once (128 MiB
maximum), and read index records in chunks of 1024 × 56 bytes. Game reads use
a 4 MiB cache. No entire index, game file, decoded collection or duplicate set
is accumulated in memory. The vendored GPL-3.0-or-later decoder is unchanged
apart from rejecting truncated comment blocks; see its IRONWOOD.md provenance.

Preview pages contain at most 100 games. Selected previews hand off to the
existing PGN flow. Range imports use existing PGN preparation, fingerprints and
four-half-move minimum, committing at most 32 games per transaction. IndexedDB
version 2 adds a non-unique fingerprint index and backfills legacy records in
the upgrade transaction. On-disk checks and a per-batch set handle duplicates.

Storage aborts preserve the previous checkpoint. Stop commits the pending
batch, and resume rereads after the durable checkpoint. Periodic checkpoint
persistence may cause a few already-saved games to be reread after an abrupt
close; disk duplicate checks prevent additional copies. Invalid individual
games are counted as failed. Index corruption and storage failures stop the
operation. A complete import remains subject to browser disk quota.

## Private-preview archive destination

The Scid range dialog offers an Import to Browser button and a separate
Import to Archive button only when the shared Companion feature
flag is enabled and the connected service advertises archive support. Range
batches use ArchiveLibrary.importPgnBatch, including the existing pacing,
retry and server-side duplicate detection. They do not prepare or write browser copies.
Each batch checks the feature flag and selected endpoint. Disconnection or
endpoint changes stop the operation rather than switching destinations.
Archive checkpoints are keyed by endpoint and stored separately from Browser
checkpoints. Resume identifies the destination and selects the newest available
checkpoint. Browser quota checks are only applied to Browser imports.
Archive range imports now use 128-game batches (Browser remains at 32), reuse
one ArchiveLibrary/client for the session, and retain its response budget and
request pacing state. Healthy requests are spread across the remaining budget,
with the existing reserve and retries retained. The dialog displays saving,
capacity-wait and retry messages separately from committed counts. Tests verify
128/128/44 batching for 300 games, client reuse across batches, status forwarding,
and budget-aware delays using a simulated clock.

## Direct PGN path and stage benchmark

The Scid-to-Archive path sends `{pgn: ...}` directly to the existing archive
import endpoint. The server performs legal-move validation, short-game
discarding, metadata extraction, fingerprinting and transactional saving.
Browser imports and transfers of Browser records with analysis runs retain
their existing paths. No server binary change is required.

Direct batches remain at 128 games, split at 15 MiB including JSON escaping.
Individual games retain the 1 MiB limit. A validation rejection commits nothing;
the client bisects that batch to isolate failed games. Authentication, connection
and storage failures stop the operation. Retries reuse the exact PGN body and
the existing content deduplication. Packing yields to the UI on an 8 ms work
budget instead of yielding after every eight cheaply copied strings.

`node scripts/scid-import-benchmark.mjs legacy|direct 512` reads the first 512
games from the owner's extracted database, uses the actual compiled WASM, and
writes only to a freshly initialized disposable Archive HTTP server. Three
alternating baseline/direct pairs produced these median observations:

| Stage/measurement | Browser-record path | Direct PGN |
| --- | ---: | ---: |
| Name-table load | 66.51 ms | 63.94 ms |
| Index read/parse | 11.87 ms | 15.47 ms |
| Scid decode | 521.67 ms | 514.22 ms |
| Browser PGN preparation | 125.22 ms | 0 ms |
| Browser metadata and fingerprint | 133.66 ms | 0 ms |
| Document metadata/export callbacks | 213.74 ms | 0 ms |
| HTTP request elapsed | 268.99 ms | 244.33 ms |
| Pacing, yields and other overhead | 2,155.65 ms | 898.65 ms |
| Total including source loading | **3,496.10 ms** | **1,727.84 ms** |
| Checked games/second | **146.45** | **296.32** |
| Upload bytes | **3,111,045** | **1,364,004** |
| Import requests | 4 | 4 |

Each run saved 510 games and discarded two short games. These results show
about 2.0× throughput and 56% fewer upload bytes on this sample. Per-stage
medians are independent and do not necessarily add to the median total.
Conversion timing measures metadata/export callbacks; other serialization and
cooperative yields are included in the overhead column. The initial exploratory
baseline omitted some production yields; the comparison above uses the real
production batch method and preparation yields on both paths.

This is a Node/WASM plus real local HTTP stage benchmark, not an Edge UI or
full-database throughput guarantee. No active engine was running in the test
archive. JSON reports are kept under ignored output/scid-import-benchmark.

Direct-PGN unit tests cover JSON-size splitting, complete acknowledgements,
short-game counts, failed-game isolation and stopping on authentication errors.
A real disposable-archive integration test confirms comments, clock directives,
variations, NAGs and custom FEN survive; games open through the real browser WASM
preparer without an attached Browser record. A lost response after server commit
is retried and reported as a duplicate, with no extra game created. Gateway tests
make Browser preparation throw to prove Archive imports bypass it.

## One-batch look-ahead

Archive imports now overlap preparation of the following 128-game batch with
the current save. Browser imports retain the sequential path. There is at most
one saving batch and one prepared/preparing batch, with one HTTP writer. Source
reading/decoding yields periodically so acknowledgements and Stop can run.

Prepared counts never advance the saved counters, ETA progress or durable
checkpoint. Stop completes the in-flight save, discards look-ahead, and resumes
after the last acknowledged game. A save failure does not checkpoint prepared
games. A look-ahead read error waits for the current save and checkpoints that
save before reporting the error. Outstanding reads are settled and the iterator
is closed before the source's WASM reader may be freed.

Four deterministic deferred-promise tests verify overlap, the two-batch bound,
one writer, Stop/resume without missing games, failed-save checkpoints, and a
producer read failure while an earlier save is pending.

The same 512-game local sample, with real source decoding inside the streaming
loop and a fresh archive per run, gave median totals of **1,796.01 ms sequential
versus 1,460.68 ms pipelined** over three alternating pairs. That corresponds to
285.08 versus 350.52 checked games/second (about 23% higher throughput). Each run
saved 510 games, discarded two, and sent four requests with identical upload
bytes. These are local Node/WASM/HTTP observations rather than an Edge or
full-database guarantee. The benchmark adds `serial-stream` and `pipeline`
modes; the source decode time is included in their wall time rather than added
again, since preparation and saving overlap.

## Archive total display

The private-preview import window shows the connected archive's total with
explicit en-US grouping (for example, 1,382,310). Source totals, progress counts
and preview row numbers also use comma grouping. Numeric range inputs remain
native number fields.

The total is read on opening, refreshed before an Archive run, and incremented
only for acknowledged new games. A server snapshot at a save boundary every
30 seconds reconciles other activity and uncertain replies without racing a
following save or double-counting. Idle refreshes also run at most every 30
seconds; Stop/completion refresh the authoritative total. Count-query failures
do not fail already committed imports. Feature gating and endpoint checks apply
to count reads as to Archive imports.
Tests cover destination visibility/routing, feature-gating, endpoint mismatch,
archive result counts, and preservation of browser records.

## Owner-provided database

The owner's downloaded LumbrasGigabase_OTB_si5.7z was extracted into ignored
output/scid5-validation without modifying the download or browser library.
No owner database content is included in source fixtures.

| File | Bytes |
| --- | ---: |
| LumbrasGigaBase_OTB.si5 | 688,517,032 |
| LumbrasGigaBase_OTB.sn5 | 19,880,467 |
| LumbrasGigaBase_OTB.sg5 | 4,226,946,494 |

Using the real rebuilt browser WASM from a Node harness with sliced filesystem
reads, all **12,294,947 index entries** passed name-reference and game-offset
validation. No entries were marked deleted. Highest game offset: 4,226,944,183.
The scan took 181.356 seconds on this machine. Final process RSS was 226,873,344
bytes; this is a point-in-time observation, not a peak-memory guarantee.

Seventy games sampled in seven groups across the database, including the first
and last games, decoded and passed Ironwood's real PGN preparation with zero
failures. The sample harness used six bounded game-file reads. These checks
do not establish that every move stream in the database is valid. The entire
database was not copied into the owner's browser storage.

## Automated coverage

- Real WASM si4/si5 decoding, metadata, comments, variations and deleted games.
- Custom starting positions, promotions, and unsigned offsets above 4 GiB.
- A virtual 12,294,947-game index and sparse game file: no whole-file reads.
- Corrupt/truncated input, mismatched parts and out-of-range offsets.
- Bounded batches, filters, failed-game counts, cancellation and resume.
- Preview selection handoff to the usual import text workflow.
- Legacy fingerprint migration, duplicates within/across transactions, quota
  rollback and successful retry using the storage test's IndexedDB double.
- Existing PGN/ZIP import, saved-game selection, and analysis-storage regressions.

The filesystem harness and DOM/IndexedDB test doubles do not replace testing
the native file picker or browser-specific disk quotas in the app.


## Companion Import folder: 7z (2026-10-24)

Added native streaming 7z extraction and read-only Scid 4/5 readers to the private
Companion queue. Supports PGNs and matching Scid triples in nested archive folders.
Extraction validates complete file groups and integrity before archive writes;
transactional batch receipts preserve duplicate and deleted counts on restart.
Source removal follows the durable report, with failed sources retained.

Validation: 67 server library tests passed (two opt-in tests excluded); final
import-focused checks passed 20 tests. Full extraction/integrity verification of
owner-provided LumbrasGigabase_OTB_si5.7z (1,461,690,738 compressed bytes;
4,935,343,993 expanded bytes) plus native decoding and PGN validation of its first
128 games passed in 57.45 seconds. The four-million name limit was checked against
its 1,151,972 name records. This did not import the full database into the owner's
archive. Extraction staging was removed afterward.

Packaged executable smoke check used a disposable archive: Scid 4, Scid 5 and PGN
7z files plus one deliberately corrupted archive yielded two unique saved games,
four reports, one retained failed source, correct deleted counts and no leftover
staging. Default package regenerated at output/companion/current, including native
reader sources, exact decoder licenses, registry dependency sources and synthetic
fixtures. Restarted Companion with the original data directory and verified its
archive API; browser WASM rebuilt and port 8788 assets refreshed.


## Companion folder progress in app (2026-10-24)

Added authenticated archive-read-budget progress endpoint and a shared, throttled
browser monitor. Import Games, Scid import and Companion archive views display
filename, committed counts, substantial progress bar, speed and ETA. Preparatory
unpacking/counting, engine waits, final verification, completion and errors have
separate text; unknown/preparation totals remain indeterminate. Totals use Scid
index metadata or a streaming PGN boundary-count pass. Rates exclude replayed
receipts on recovery, and the completed snapshot persists until another job.
Feature-disabled/disconnected clients send no progress requests; no credentials
or local source paths enter the displayed snapshot.

Validation: 69 server unit tests passed (two opt-in tests excluded), 18 focused
folder-progress/Scid/prefetch browser tests passed, and 34 app-startup/connection
checks passed. Rebuilt browser WASM and refreshed port 8788. Default Companion
package regenerated; actual Import folder smoke check in a disposable archive
observed idle, complete and failed progress snapshots, accurate totals/deleted
counts, four reports, and preservation of the corrupted source. The same owner
data directory was reused for Companion restart.

The larger packaged smoke test imported a 20,000-game synthetic PGN and captured
live progress with processed/total counts, positive games per second and finite
ETA; it also observed complete/failed states, five reports and retained corruption.
The browser-to-archive round-trip integration test passed against the packaged
server. Owner Companion came back online after its archive startup scan; its
progress endpoint reported the actual recovering Lumbras 7z job in unpacking,
with 37,248 durable processed games. Verified the new WASM bridge on port 8788
and installed executable matching the final native build.


## Background indexing no longer blocks folder import (2026-10-24)

Replaced the all-slots-free/database-unlocked idle condition with a foreground
request activity counter. Position indexing now uses the same bounded background
admission queue as import commits, allowing both to proceed without an idle gap.
Foreground preparation/requests retain their activity guard until actual work
finishes, including cancellation. Engine search, setup, quiet period, foreground
archive requests, and update/import reservation waits have specific messages.

Validation: 71 server library tests passed (two opt-in tests excluded), including
continuous indexing alongside 1,025-game import, background-lock import admission,
foreground-request exclusion, engine waits and update reservations. Rebuilt WASM,
refreshed port 8788, packaged with the default Companion output and restarted using
the existing data directory. Packaged synthetic smoke test imported 20,000 PGN
games with positive live speed/ETA, five reports and a correctly retained damaged
source. Owner progress endpoint reported resumed Lumbras recovery/unpacking from
55,936 durable processed games. Installed executable matched the final build.

Live owner verification after recovery: Lumbras advanced from the 55,936-game
checkpoint to 63,360 committed processed games, with positive measured speed and
finite ETA. Position indexing progressed concurrently (1,453,003 indexed games
and 233,442 pending at the first importing snapshot). The source was not restarted
as a new job and committed receipts remained intact.


## Native Companion status refresh (2026-10-24)

The Windows status window previously repainted on manual Refresh or software-update
state changes only. Its existing one-second WM_TIMER now refreshes visible,
non-minimized status windows independently. Automatic refresh does not activate,
restore or reopen the window. Identical text is not rewritten; changed text
preserves selection and first visible line. The window now includes comma-formatted
import counts, speed/ETA and detailed waiting messages, and has sufficient text
space plus scrolling. Tray status also reflects active import phases.

Validation: all 71 server library tests and both native-binary test groups passed;
status formatting checks cover comma grouping, import/wait labels and paused ETA.
Browser WASM rebuilt; default Companion package regenerated, installed and matched
the final executable hash. Original data directory and checkpoint preserved.
Computer-use verification of the actual native window observed unpacking changing
to importing and batch 186 advancing to 276 with no Refresh button press. A
screenshot confirmed the new details fit the status window. Recovery retained
745,472 processed games (74,907 saved, 670,077 duplicates, 488 short games).


## Resume from committed batches without game replay (2026-10-24)

Recovery loads and validates the contiguous transactionally saved batch prefix
before preparing games. Existing serialized receipt formats remain supported.
Scid readers seek past saved index entries without reading/decoding saved blobs;
PGN readers skip boundaries without legal-move replay or document preparation.
Partial receipts must end a source file, preserving multi-file archive ordering.
Missing fingerprints, receipt gaps and inconsistent file boundaries retain the
source instead of guessing a resume point. Rates restart after recovery skips.

Added a private extraction cache: source fingerprint, generated relative filenames,
sizes and SHA-256 hashes are atomically recorded only after extraction integrity
checks and file synchronization. Valid extracted files are reused across restarts;
missing/changed/corrupt files trigger fresh extraction. Paths are validated and
changed original sources still fail verification before any skip. Legacy jobs
need one extraction to create the manifest. Cache/staging cleanup still follows
the durable final report.

Validation: 76 server library tests passed, with three opt-in tests excluded;
both native-binary groups passed. Recovery tests cover old receipts, gaps,
missing fingerprints, partial/full file boundaries, Scid 4/5 seeks, PGN boundary
skips without replay, extraction mtime preservation across recovery, source hash
mismatch, corrupted stage recovery and invalid cache paths. Eighteen focused
browser tests passed. The opt-in actual Lumbras Scid test directly sought past
745,472 entries in 8.3 microseconds without reading any game blobs, then decoded
and validated only the next game (full test, including names, 0.14 seconds).
This measures the reader seek, not end-to-end recovery hashing/cache checks.
Browser WASM rebuilt, port 8788 refreshed, and the default native package installed
with the original data directory and 745,472-game checkpoint retained.

Live owner verification: the retained 745,472-game checkpoint corresponds to next
batch 5,825. After startup/preparation the native job was observed at batch 5,973,
with 764,416 processed games, positive speed (449.6 games/s) and finite ETA. Counts
advanced beyond the saved prefix immediately after recovery, rather than spending
time replaying it. A version-1 extraction manifest for the three verified Scid
files was present for future reuse. Final packaged smoke test also passed with
live speed/ETA, five durable reports and one correctly retained damaged source.


## Overflow-only gold native status scrollbar (2026-10-24)

Removed the always-visible system scrollbar from the status edit control. A local
gold thumb/dark-track scrollbar appears only when wrapped status text exceeds the
visible line count; full text width is restored when overflow ends. Measuring at
full width avoids scrollbar-induced wrapping keeping it visible. Scroll restoration
is clamped to the current text range. Wheel preferences/high-resolution deltas,
normal text keyboard scrolling, thumb dragging and track paging are supported.
Automatic status refresh and token handling remain unchanged.

Validation: 76 library tests passed (three opt-in tests excluded); each Windows
binary passed four tests covering visibility for short/long/short text, restored
text width, native wheel scrolling, thumb bounds and actual gold GDI pixel color.
Browser WASM rebuilt and default Companion package regenerated. Installed binary
matched the final native build. Actual status-window screenshot verified that
normal import text has no visible scrollbar. The existing job resumed beyond its
3,827,456-game checkpoint and continued processing (3,829,632 at API verification,
3,833,856 in the captured status window).

## Main-window opening-index progress

Added enabled-only, visibility-aware status-bar polling with grouped indexed and
remaining counts, a recent-speed sample and ETA for the current backlog. Progress
reads use transactional derived counters, seeded once for existing archives,
instead of scanning all indexed games on every poll. Connection changes, rebuilds,
completion, retry/backoff and older-server polling are covered.

Validation: 78 server library tests passed (three opt-in tests ignored), plus four
tests for each native binary; 34 browser tests passed. Browser WASM rebuilt and
port 8788 assets refreshed. Default Companion package installed with the original
data directory. Live status returned progress_version=1 in 92 ms; the actual client
formatter reported 2,153,485 indexed, 3,007,067 remaining and 66.7 games/s with ETA.
The resumed source reached 4,147,072 processed games without replaying saved batches.

## Consistent opening labels and code-wide selections

Source ECO codes now constrain fallback opening names. Recognized labels belonging
to other codes are replaced with a matching catalog label; custom source labels
are preserved. A one-time migration repairs derived library cards using distinct
code/name pairs and indexed updates, without reparsing or rewriting original game
documents. Each ECO code gets an All-code-games picker option, alongside exact
variation choices, for both Browser and Archive sources.

Validation: 80 server library tests passed (three opt-in tests ignored), including
all catalog codes, migration idempotence, preserved source documents and code-wide
selection. Five browser tests passed for picker options, remote pagination and
previews. Browser WASM rebuilt and port 8788 assets refreshed.
Installed the default Companion package and completed the owner archive migration.
Live All A85 selection returned 24,027 games; the erroneous A85 / Queen's Gambit
pair returned zero, and sampled A85 cards displayed Dutch Defense: Queen's Knight
Variation. Folder import resumed beyond 10,770,560 processed games.

## Clean schema-4 binary archive

At the owner's request, archived the previous archive and pending Import directory
together under `C:\Users\Dad\AppData\Local\Ironwood\Companion\archive-reset-20261025-070259`.
No old database, source archive or checkpoint was deleted. Config, credentials,
engines, Reports and browser storage were retained. The default updated Companion
now runs against empty replacement archive/Import directories; the old queue is
not replayed into the new database.

Schema 4 persists PGN move packets with shakmaty PackedUciMove in compressed SQLite
BLOBs, with separate compressed browser snapshots. Shared `eco_openings` contains
500 code descriptions; game envelopes/cards retain codes without repeated labels.
Export resolves labels from the lookup; fingerprints omit resolved descriptions.
Opening indexing consumes packed moves directly. Legacy text and schema-2/3 JSON
backups remain readable; all new writes, edits and restores use binary payloads.

Validation: 82 library tests plus eight native window tests passed (three opt-in
tests ignored), including byte-preserving codec round trips, nested variations,
Unicode annotations, castling, en passant, promotion/underpromotion, mate and
Chess960. Tested payload deletion, normalized labels, duplicate export/reimport,
JSON restore and preserved browser state. Against the installed release executable,
92 HTTP/browser/WASM tests passed; 16 additional export/ZIP/pacing/storage tests
passed. These include actual PGN/JSON exports, Scid 4/5 import, 7z queue fixtures,
restart recovery, revisions/collections, opening-index rebuild and analysis runs.
Owner archive verified at schema 4 with zero games/payloads, 500 lookup rows,
quick_check=ok and zero foreign-key violations. Import progress is idle. Browser
WASM rebuilt and port 8788 serves the schema-4-compatible client.
