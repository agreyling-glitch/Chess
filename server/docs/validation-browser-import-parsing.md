# Private Archive browser import parsing validation — 2026-10-23

## Changes

- Successful legal PGN timelines are reused by the browser preparation/export path. The exact PGN text is the key; timelines are bounded to 128 entries and approximately 8 MiB, with FIFO/LRU eviction. Invalid parses are not cached. Derived comment/NAG token scans are also reused only when the complete position timeline agrees. Notes and analysis from a different saved JSON record are never substituted.
- Companion import preparation runs in a bounded blocking worker sharing the existing four-slot admission budget. It validates documents, legal PGN moves, browser JSON and analysis results; computes fingerprints; serializes results; and builds library cards without holding the database mutex.
- The commit phase retains a single atomic SQLite transaction: duplicate lookup, inserts, analysis conflict/revision handling, collection membership and library-card writes. Fingerprint semantics are unchanged. Original JSON and notes remain intact.
- Raw PGN requests reuse the initial parse. Imported library cards reuse the validated parse instead of parsing again on first browse. The independent opening-position explorer remains a background job.

## Real browser benchmark

`node scripts/browser-import-benchmark.mjs before tmp/import-parser-baseline.exe 8808` starts a loopback-only test page and disposable archive. The baseline executable was copied from the current local package before this change. The before measurement was completed before rebuilding the browser. The after run used the new package and WASM on the same origin in the same Chromium 155 browser.

512 distinct, legally parsed 33-ply PGNs (the Opera Game move sequence with separate synthetic headers) were fetched as a PGN collection. The test invokes the real browser WASM preparation and annotated export functions, then the production ArchiveLibrary importer. Each saved record contains one explicit note and two saved analysis runs. Batches of 128, event-loop yields and the existing one-second request pacing remain enabled. Native file-picker interaction, collection-selection UI, user archives and production services are excluded. This is a single local before/after observation, not a general throughput guarantee.

| Metric | Before | After |
| --- | ---: | ---: |
| Initial preparation + import | 3,197 ms | 3,203 ms |
| Browser preparation, including yields | 241 ms | 234 ms |
| Conversion + import, including pacing | 2,955 ms | 2,969 ms |
| Summed import HTTP latency | 117 ms | 157 ms |
| Pending library cards after import | 512 | 0 |
| Duplicates detected on reimport | 512 | 512 |
| Saved notes verified | 512 | 512 |
| Saved analysis runs verified | 1,024 | 1,024 |

**Interpretation:** end-to-end time is effectively unchanged (about +0.2%, within ordinary run-to-run noise). Request pacing dominates this short-game workload. Initial HTTP responses do more work because cards are now prepared immediately outside the database lock. Previously 512 cards still needed work on first browse. Do not claim an end-to-end speedup from these results.

Every exported saved JSON record was compared with its input, including notes. Both analysis payloads were compared for every game; all 1,024 engine names/settings were additionally checked in the synthetic archive. No user archive was used for the benchmark.

## Tests

- 256 JavaScript tests passed, including real HTTP import/duplicate/analysis transfers.
- 40 Companion library tests passed.
- 295 application tests passed, 3 ignored, including the annotation-scan regression.
- A parse-count regression verifies exactly one raw-PGN parse through preparation, commit and library synchronization.
- A concurrency regression holds import preparation in a worker and verifies a database read completes before that worker is released.
- Cache tests cover bounds, invalid input, per-record note isolation, and preservation of evaluations, notes, board marks, NAGs and variations.
- WASM rebuilt, local site refreshed and local Companion package rebuilt with the existing private update-channel settings and data directory.

Raw reports: `tmp/browser-pgn-before.json`, `tmp/browser-pgn-after.json`, and `tmp/browser-pgn-library-readiness.json`. Screenshot: `tmp/browser-pgn-import-after.png`. The benchmark creates synthetic credentials only for disposable loopback archives; it never prints or uses the user's credential.
