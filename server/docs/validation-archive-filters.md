# Archive filter index validation

Internal validation - 2026-10-23. No public Companion announcement.

## Dataset and method

Read-only access to the existing archive produced a disposable, library-only SQLite snapshot with **181,162 games**. Both versions ran the same native `archive_library::query` benchmark against those identical card rows. Results include SQL, JSON handling and the full dropdown facets response, but exclude HTTP and browser rendering. Debug builds were used on the same machine. The application query cache was cleared before each first request; operating-system disk caches were not flushed. The second request used the application cache. Index creation time is excluded.

| Request | Before, uncached ms | After, uncached ms | Before, cached ms | After, cached ms |
|---|---:|---:|---:|---:|
| all | 4151.5 | 243.3 | 23.9 | 23.5 |
| player | 4209.9 | 233.9 | 91.7 | 24.1 |
| source | 5316.3 | 230.7 | 730.5 | 22.3 |
| date | 4848.2 | 223.9 | 419.3 | 22.4 |
| opening | 4452.8 | 229.2 | 52.1 | 23.5 |
| combined | 4359.5 | 236.4 | 226.2 | 22.2 |
| favorites | 1123.3 | 37.6 | 201.3 | 1.2 |
| analysis | 4122.1 | 243.5 | 23.2 | 22.8 |
| deep_page | 4164.8 | 243.5 | 50.1 | 32.8 |

All nine cases returned exactly the same games, totals, category counts and Player/Source/Date/Opening choices before and after.

## Implementation

- Nine additional indexes cover White, Black, Favorites, training profile, analysis status, normalized Source and Date, normalized ECO + Opening, and legacy ECO prefix filtering. Existing category, recent-save and move-count indexes remain.
- Index expressions use the same normalization as the filter predicates; the stored card format and original game documents are unchanged.
- Queries bind user values and omit inactive predicates, allowing equality/range seeks and a union of both player indexes.
- Facet queries omit inactive category guards, allowing ordered covering index scans instead of sorting JSON values.
- Planner statistics are populated on migration and SQLite optimize runs at startup to refresh stale/missing statistics after archive growth. A broad Imported Games category can therefore use the selective Player indexes.
- Legacy arbitrary substring searches still scan candidates; the dedicated exact Opening selection uses the composite index.
- Extra indexes trade disk space and insert/update work for faster filtering. SQLite maintains them transactionally.

## Verification

- Query-plan tests verify filter seeks and ordered indexed choice queries.
- Regression tests cover unknown values, ECO prefixes, bound filter values, combined filters, updates/deletes and repeated initialization.
- Migration tests preserve original notes and saved analysis metadata/payloads.
- Complete Rust workspace suite: 295 application tests, 57 server library tests, and both native binary tests passed; four opt-in benchmarks ignored.
- JavaScript regression suite: 288 passed.
- Browser WASM rebuilt and local site refreshed.
- Local Companion rebuilt in `output/companion/current`, preserving its private update feed/public verification key and configured data folder. No upload or public release.

## Preservation

Before/after SHA-256 hashes cover every complete game row (including document, notes, fingerprints and revisions), saved analysis row, collection and membership. The archive contains 181,162 game rows and three saved analysis runs. Validation artifacts and the disposable benchmark database remain under ignored `tmp/archive-filter-*` paths.


## Growth benchmark: 448,546 games

Read-only access on 2026-10-23 created a disposable library-card snapshot at `tmp/archive-448k-benchmark.sqlite3`. All 448,546 live games had current cards (zero dirty entries). Original game documents, notes and analyses were neither copied into this benchmark nor written. Same native debug-build benchmark as above; application query caches cleared for each first request, OS caches not flushed. These timings exclude HTTP and browser rendering. Selected filter values differ with the newest game, so selective-filter timings are indicative rather than identical-workload comparisons.

| Request | 181,162 uncached ms | 448,546 uncached ms | 448,546 cached ms |
|---|---:|---:|---:|
| All | 243.3 | 411.4 | 33.6 |
| Player | 233.9 | 406.6 | 36.7 |
| Source | 230.7 | 419.5 | 33.7 |
| Date | 223.9 | 404.6 | 33.6 |
| Opening | 229.2 | 395.3 | 33.0 |
| Combined | 236.4 | 391.2 | 33.8 |
| Favorites | 37.6 | 88.4 | 1.3 |
| Unanalyzed | 243.5 | 433.8 | 34.1 |
| Deep page | 243.5 | 419.9 | 42.6 |

Additional direct SQLite checks (three repetitions, excluding facets/HTTP):

- Selection projection, first 1,000: 1.5–2.9 ms; last batch at offset 448,000: 18.0–18.4 ms. This is per-batch SQL, not end-to-end Select all time.
- Final page: 15.8–15.9 ms using the recent-save index.
- Fewest moves: 0.13–0.29 ms using `library_moves`.
- Most moves: 238–241 ms, full table scan and temporary ordering B-tree. Existing ascending index cannot satisfy the mixed descending/ascending ordering.
- Experimental index `library_cards(moves DESC,updated DESC,id)` on the disposable copy only: 0.13–0.31 ms, indexed scan. Creation took 422 ms. No production index or application code was changed.
- Facets: 35,490 players, 3,063 sources, 11,208 dates, 9,859 openings; approximately 1.55 MB as uncompressed JSON. Initial requests still build/return entire choice lists, and cached requests still serialize them unless callers supply the matching facets revision.

Recommended next work: add and regression-test the descending move-count index, then measure actual browser dropdown/rendering and end-to-end selection/export. At this size there is no need to import more games to expose these issues. Raw artifacts: `tmp/archive-448k-results.json`, `tmp/archive-448k-extra.json`, `tmp/archive-448k-candidate.json` and benchmark log.
