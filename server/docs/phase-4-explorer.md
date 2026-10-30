# Phase 4: archive opening explorer

The Opening Explorer now offers an **Archive** source alongside the existing
Lichess sources. It uses your Companion's saved games, without a Lichess sign-in
or any external database. No opening books or third-party datasets are bundled.

## Use

1. Start the rebuilt Companion. Existing archives are indexed in the background.
2. Connect through **Settings → Companion connection…**, then close the archive panel.
   The explorer can also reuse an existing Companion engine connection in memory.
3. Open **Opening Explorer** in the board's analysis details and choose **Archive**.
4. Use **Archive filters…** to select a collection, player/color, inclusive dates,
   or separate White/Black rating ranges. Blank fields include all known/unknown
   values; applying a date/rating filter excludes records missing that field.
5. Click a continuation to preview it. Back/Back to game preserve the original
   game, existing navigation and move history. Matching games can be paged and
   copied to Saved games; this does not replace the current board.

The current training/online restrictions on Opening Explorer remain in force.
Archive credentials stay in memory and require reconnection after reload. Data
requests go only to the selected Companion; no Lichess fallback is automatic.

## Counting and metadata

The index includes legal mainline positions from Standard chess games, including
setup FEN games. Variations and Chess960 games do not contribute; other variants
are reported as excluded. Game moves/counters and annotations remain unchanged.

Position keys contain placement, side to move, castling rights and **legal**
en-passant availability. Halfmove/fullmove counters are excluded. This merges
transpositions and inert/pinned en-passant targets while retaining genuinely
different castling/en-passant rights.

Each archived game ID contributes once per position and once per distinct
position/continuation. Repeating the same move after a repetition does not inflate
it. A game that revisits a position and chooses different continuations can count
once for each continuation, so the sum of move counts can exceed position games.
Identical imports deduplicate in the archive; deliberately distinct annotated
archive versions retain distinct IDs and count as separate archived records.

White wins, draws, Black wins and unfinished/unknown results are separate counts.
Result percentages use completed results only; an all-unknown row has no result
percentage. Position totals include games ending at that position, even if they
have no continuation. Missing player/date/rating counts describe the matching
indexed records; metadata is never inferred from a training rating or an engine's
strength setting. Archive metadata overrides PGN tags where supplied, while
original PGN tags provide additional fields such as WhiteElo/BlackElo.

## Index lifecycle

The derived index has its own version (currently 1); the source archive and JSON
backup schema remain 2. SQLite triggers persist a queue and bump the generation
for game edits/additions/deletions and collection membership changes. An older
Companion writing the same database still runs these triggers. One worker replays
one game at a time outside the archive connection lock, then commits its bounded
index transaction. Normal archive operations can interleave between games.

Queries join only index rows whose revision matches the current game, in a single
read snapshot. Pending/failed games never contribute stale statistics. Responses
include generation, index progress and a partial flag; partial totals are labeled
in the browser. Missing metadata and excluded variants are explicit. Jobs check
both revision and content fingerprint: even a restore reusing an ID/revision
cannot publish an earlier job's obsolete entries. Graceful shutdown drains
the active job before releasing the server's data-directory lock.

The visible browser query refreshes periodically (500 ms during indexing, 2 seconds
when settled). Position/filter/connection changes discard old in-flight responses;
generation changes reset matching-game pagination. There is no persistent query
cache containing credentials or statistics.

Use **Rebuild opening index** in Archive filters to regenerate derived data. It
preserves all source games and queues them again; partial statistics remain labeled
until completion. Old index rows are hidden immediately and replaced one game at
a time, avoiding a bulk position deletion under the archive lock. JSON backups
exclude the derived index and restore queues it
automatically. Native SQLite snapshots include it, with startup consistency checks
queuing any missing or outdated game revisions.

## HTTP

All routes require the existing exact Origin, protocol header and owner credential.

- GET `/api/v1/archive/explorer`: `fen` (six fields), optional `variant=standard`,
  `collection`, `player`, `color=either|white|black`, `date_from`/`date_to`
  (`YYYY-MM-DD` or PGN dotted dates), `white_elo_min`/`white_elo_max`,
  `black_elo_min`/`black_elo_max` (1–5000), `game_after`, `game_limit` (1–50,
  default 10), and optional legal `uci` to restrict matching-game retrieval.
- GET `/api/v1/archive/explorer/status`: generation/version, total/indexed/pending,
  failed and unsupported-variant counts, and `complete`.
- POST `/api/v1/archive/explorer/rebuild`: queue a rebuild; no source mutation.

Query responses contain `position_key`, `generation`, `index`, `indexing`, `partial`,
`missing_metadata`, result counts (`total`, `white`, `draws`, `black`, `unknown`,
`completed`, `percentages`), `moves` (the same counts plus UCI/SAN), `recentGames`
(archive IDs/revisions and available metadata), and `next` for matching games.
Percentages are null if there are no completed results. No engine evaluation,
third-party opening classification or opening-book weight is fabricated.

## Validation

Deterministic fixtures cover transpositions, repeated moves, castling rights,
capturable/inert/pinned en-passant, setup counters, terminal positions, unknown
results, duplicate imports, missing metadata, variants, rating/date/player and
collection filters, matching-game pages, revision races, deletion, rebuild,
restart, and automatic indexing after JSON restore. Browser client tests cover
stale position/filter/connection responses, generation changes and pagination.
The existing preview-scoping test protects the actual game during exploration.
An isolated browser check on port 8792 verified expected continuation counts,
player/color filters, matching-game copies, previewing Nf3 and returning to the
original e4–e5 game without changing its move history. Port 8788 assets were refreshed.

```sh
cargo test --locked -p ironwood-server
cargo clippy --locked -p ironwood-server -p ironwood-protocol --all-targets -- -D warnings
cargo build --locked -p ironwood-server
node --test tests/companion-explorer.test.mjs tests/companion-explorer-client.test.mjs tests/companion-archive.test.mjs tests/companion-lifecycle.test.mjs
cargo test --locked -p battle-chess --lib opening_explorer
npm run build:wasm
npm run refresh:site
```

Linux Docker execution, deployed HTTPS/browser permissions, clean-machine
packaging and large-archive performance remain release checks. No public artifact
or database download was published for this phase.
