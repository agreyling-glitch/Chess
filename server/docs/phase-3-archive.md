# Phase 3: persistent archive and transfers

The native server now owns a transactional SQLite archive. The default database
is `archive/archive.sqlite3` inside the Companion data directory; `archive` may name another owner-local
file, or be `null` to disable the service. Relative paths resolve against the
configuration directory. Keep it on a local disk, with one owning Companion
process. Do not use a network filesystem. Docker's `/data` volume retains it
across container replacement. No cloud storage or automatic synchronization is used.

## Browser workflow

Open **Storage → Companion archive…**, enter the Companion address and owner
credential, and connect. Credentials remain in memory and are not included in
games, exports or backups. The panel can also reuse an engine connection's
in-memory credential for the same address. Reload requires entering it again.

- Choose browser games on a page and upload them, optionally into a collection.
  Originals stay in IndexedDB. Repeating an import returns duplicate counts.
- Search archived games by White/Black, date range, result, opening/ECO, training
  profile ID, or collection; browse pages of up to 100 (the UI uses 50).
- Copy selected archive games to the browser library. ID collisions create new
  browser records instead of overwriting existing games. Repeat copies skip
  identical records. Transfers do not write training profile/rating ledgers.
- Create/rename/remove collections, or add/remove selected games. Removing a
  collection removes only membership; it does not delete its games.
- Edit archive search metadata, download PGN or Companion JSON, and import PGN
  or Companion JSON files. Metadata edits use revisions; refresh/reopen after a
  conflict. PGN and original game data remain intact when search metadata changes.
- Explicit game deletion affects only the server game and its memberships.
  Browser copies and profile ratings are independent. The UI confirms deletion.

Browser documents preserve the complete original record, including the unchanged
saved JSON string, category, favorite, source and additional fields. This retains
starting FEN/counters, moves, comments, drawings, analysis and engine provenance.
Unknown transfer versions fail before any writes. The version-1 envelope labels
the currently supported Ironwood record format (board/history and optional data).

PGN imports use legal-move validation for Standard chess and Chess960, retain all
tags, comments, NAGs and variations, and preserve starting FEN/counters. Formatting
is normalized for PGN-only imports. Ordinary PGN comments remain in the stored
PGN; Ironwood-specific note/drawing directives are restored into browser controls.
The browser's existing PGN reader may reject variants it cannot display; export
the PGN file in that case. Use Companion JSON for lossless Ironwood transfers;
PGN cannot represent every Ironwood analysis field or training metadata.

## Transactions, revisions and limits

Imports validate the complete batch before a single transaction adds any games.
A malformed later game, invalid membership or interrupted request body cannot
leave an earlier game behind. SHA-256 fingerprints distinguish annotated versions;
only transport-local browser IDs and update timestamps are excluded. Changed notes,
drawings, analysis or other record metadata create a distinct version.

Updates/deletes require the current immutable game ID and revision. A transaction
rejects stale revisions with HTTP 409. Collection edits have independent revisions.
Imports may append memberships without overwriting another client's collection.
Cursor pages sort by immutable ID; refresh from the first page to see new inserts.

Requests are limited to 16 MiB; imports/exports to 1000 games; individual PGN to
2 MiB, saved JSON to 4 MiB, and PGN parsing to 10,000 plies/32 variation levels.
There are four bounded archive worker slots; excess work receives `busy`. The
existing request/rate/deadline limits still apply. A client timeout after a complete
request can have an uncertain outcome; retry identical imports safely or refresh
before retrying an edit. No client automatically replays writes.

## Backup, migration and recovery

The UI downloads a consistent JSON snapshot containing IDs, revisions, complete
documents, collections and memberships. JSON restore requires an empty archive
and validates versions/references in one transaction. Existing data is preserved
when restore is rejected. Browser preferences and rating ledgers are outside this
server archive and remain covered by the existing browser backup flow.

For larger archives use native SQLite snapshots, with Companion stopped:

```powershell
ironwood-server archive-backup --data-dir C:/IronwoodData --output D:/Backups/companion-2026-10-17.sqlite3
ironwood-server archive-restore --data-dir C:/IronwoodRecovered --input D:/Backups/companion-2026-10-17.sqlite3
```

The destination must not exist. Restore into a fresh dedicated data directory;
then configure the desired allowed origins and use its newly generated owner
credential. Existing files are never overwritten. Snapshots use SQLite's backup
API and validate integrity, known schema and foreign-key references. They do not
copy credentials or configuration; protect those separately. Copy backups to
another device. Do not copy a live database file alone: committed data may be in
its `-wal` file. JSON transfers are bounded; native snapshots have no HTTP limit.

The current schema is 2. Fresh databases create the complete schema atomically.
The known version-1 schema (games and collections) upgrades in a transaction after
a consistent `archive.pre-v1-<random>.sqlite3` snapshot is created. Failed upgrades
roll back; keep the pre-upgrade file for recovery into a new directory. Newer or
unrecognized schemas are rejected before writes. Keep all database sidecars with
the original until the owning process is stopped; do not delete them to fix errors.

## Validation record

Automated Windows native coverage includes exact JSON/annotation round trips,
duplicate imports, distinct annotated versions, malformed/interrupted imports,
SQL filter escaping, pagination, concurrent edits, collection deletion boundaries,
restart persistence, JSON and native SQLite restore, migration backups, and newer
schema rejection. Browser transfer tests preserve colliding local records and
verify that rating/profile storage is never written.

An isolated browser check on port 8792 created a collection, uploaded a local
game while keeping its original, skipped the existing browser copy on return,
copied a PGN-only game into the browser library, and skipped its repeated download.
The normal preview on port 8788 was refreshed with the new assets.

```sh
cargo test --locked -p ironwood-server
cargo clippy --locked -p ironwood-server -p ironwood-protocol --all-targets -- -D warnings
cargo build --locked -p ironwood-server
node --test tests/companion-archive.test.mjs tests/companion-archive-client.test.mjs tests/companion-server.test.mjs tests/companion-lifecycle.test.mjs tests/companion-uci.test.mjs tests/companion-engine-adapter.test.mjs
cargo test --locked -p battle-chess --lib
npm run build:wasm
npm run refresh:site
```

Linux Docker execution, clean-machine packaging and the deployed HTTPS/browser
permissions matrix remain release checks. No new public artifact has been published.


## Private-preview storage diagnostics

With the Companion preview flag enabled, Storage information adds a server archive
panel using authenticated `GET /api/v1/archive/stats`, opening-index status and
capabilities. Counts include categories, favorites, collections and memberships.
Sizes cover the SQLite database, write-ahead log and shared-memory file; missing
journal files count as zero, while unreadable file sizes are unavailable. SQLite
allocation and reusable pages are distinct from physical files and host free disk
space. Engines and Syzygy files are excluded. No host paths are returned. Browser
statistics remain independent of server availability; no connection prompt appears
until requested. This requires a Companion binary with the statistics endpoint.
