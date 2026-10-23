# Private Archive import performance validation — 2026-10-23

The client now converts records incrementally, yields every eight records, and sends up to 128 games per request, retaining the 15 MiB encoded-document budget and one-second bulk request pacing. PGN collection preparation processes up to eight games per frame with an eight-millisecond soft budget and 128 pending records. The queue debounces partial batches for 500 ms; full batches flush immediately. Duplicate detection, transaction handling, analysis merging and retry semantics on the server are unchanged.

The paste panel adds a gray raised Browse button. A selection fills the paste area without importing or replacing the live board; Continue is still required. Cancel and read failures preserve existing text. The picker rejects files larger than 64 MiB.

## Measured local benchmark

Windows x64, Node 24.14.0; 256 distinct short PGNs (two plies), each containing notes and two saved analysis runs. Both clients used separate temporary archives and the same real local Companion HTTP server. A deterministic JavaScript exporter supplied PGN; browser WASM export, UI rendering and network conditions beyond loopback are excluded. This is one measured comparison, not a general speed guarantee.

| Metric | Before | After |
| --- | ---: | ---: |
| Initial import | 7.081 s | 1.260 s |
| Throughput | 36 games/s | 203 games/s |
| Import requests | 8 | 2 |
| Peak converted, unacknowledged games | 256 | 128 |
| First request | 2 ms | 243 ms |

Measured initial-import speedup: 5.62x. The first request is later because incremental conversion yields to the event loop. That trades a short startup delay for bounded preparation and responsiveness.

Both clients then reimported all 256 games: all were detected as duplicates. Every game was exported and its original JSON/notes and both saved analysis runs were compared with the source (512 runs verified per client). User archives were never opened by the benchmark.

## Validation

- JavaScript suite: 256 passed.
- Application Rust suite: 293 passed, 3 ignored.
- Browser WASM bundle rebuilt and local site refreshed.
- Tests cover incremental conversion before acknowledgement, bounded partial progress on failure, retry payload stability, file selection/cancel/read/size errors, and existing real archive analysis transfers.

## Reproduce

Save the previous `web/companion-library.js` to a temporary `.mjs` file. With the local package in `output/companion/current`, run:

```powershell
node scripts/benchmark-archive-import.mjs 256 tmp/archive-import-before.mjs
```

The script creates and removes isolated directories under `tmp`, authenticates locally without printing credentials, and writes `tmp/archive-import-benchmark.json`. It never connects to the user's configured archive. Use the same server build for both clients.
