# Storage Information Archive performance — internal validation

2026-10-24, local Windows Companion, 465,106 games. No public release or upload.

The Archive tab requests `archive/stats`, `archive/explorer/status`, and `capabilities` concurrently. The first two share the archive connection lock. Previously, category and favorite counts parsed complete JSON documents, then the Explorer status join fetched game revisions from game-table rows. These requests took about 7.3 seconds together on the actual archive.

## Change

- An expression index stores the existing category CASE expression, preserving puzzle/training/legacy-category precedence exactly.
- A small partial index covers favorite games under the existing predicate.
- Covering indexes on `games(id,revision)` and `explorer_games(status,game_id,revision)` let Explorer status verify current revisions without loading complete documents.
- SQLite maintains every index transactionally. Results are current after imports, edits and deletes; no stale count cache or shortened statistics response is introduced.
- Indexes are created once at startup; this adds a one-time build cost and some ongoing disk/write overhead. Existing documents, notes and saved analyses are untouched.

## Timings

Authenticated loopback HTTP requests use the same three-way concurrent request pattern as the UI. Times include request/response processing and lock waits, but exclude browser connection discovery and rendering. Operating-system disk caches were not flushed. Both packages are optimized native builds.

| Request | Before ms | After first request ms | After repeat ms |
|---|---:|---:|---:|
| Archive statistics | 5,046 | 42 | 25 |
| Opening index status, including lock wait | 7,297 | 162 | 158–161 |
| Capabilities | 6 | 6 | 0.6–1 |
| All requests complete | about 7,300 | 176 | 158–161 |

Approximately 45× faster for the request group. Game, category, favorite, collection and membership counts match the baseline exactly (465,106 games). Index/file sizes naturally change when indexes are added.

## Validation

- 63 server library tests passed; one opt-in benchmark ignored.
- New regression tests cover indexed statistics, metadata-only training categories, puzzle precedence, favorite changes, idempotent index creation, original document preservation and covering query plans. Existing Explorer revision/invalidation, imports and saved-analysis tests also pass.
- Updated package built with `scripts/package-companion.ps1` at `output/companion/current`, retaining its private signing/update channel. Companion restarted with the original data directory.
- Required browser WASM rebuild and local site refresh completed.
- Raw local artifacts: `tmp/storage-info-before.json`, `tmp/storage-info-after.json`, `tmp/storage-info-server-tests.log`, `tmp/storage-info-package.log`, `tmp/storage-info-wasm.log` and `tmp/storage-info-site.log`.
