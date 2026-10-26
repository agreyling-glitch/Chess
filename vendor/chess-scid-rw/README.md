# chess-scid-rw

A pure-Rust, dependency-free framework for reading and writing
[SCID](https://scid.sourceforge.net/) chess database files: format
**si4** (`.si4`/`.sn4`/`.sg4`) and format **si5** (`.si5`/`.sn5`/`.sg5`).

## What this is for

SCID (Shane's Chess Information Database) is a widely used chess
database format, but its on-disk layout is a compact, undocumented
binary encoding — not something a Rust program can read or write
without a dedicated implementation. `chess-scid-rw` exists to fill that gap:
it fully decodes a database's index, its name tables (players, events,
sites, rounds), and every game's move stream (main line, nested
variations at any depth, comments, NAGs, non-standard starting
positions), and it can create and modify databases just as completely.
Both formats are supported symmetrically, in both directions.

The API never requires any knowledge of SCID's binary format: every
game goes in and comes out as plain PGN text — see
[`docs/pgn-reference.md`](docs/pgn-reference.md) for why, and exactly
what PGN dialect is involved.

## Goals

- **Complete, two-way format support.** Both si4 and si5, reading and
  writing, are first-class — not a read-only decoder with writing
  bolted on as an afterthought.
- **Zero external dependencies.** The chess representation needed to
  decode SCID's compact move encoding (position, legal move generation,
  SAN notation, a variation tree) is vendored in
  [`src/chess`](src/chess) rather than pulled from an external crate.
  This crate only ever depends on `std`.
- **Cross-platform.** Pure Rust, no platform-specific code — builds and
  runs identically on Windows, macOS, and Linux.
- **A simple, PGN-in/PGN-out API.** No struct of index fields to fill
  in by hand, no need to understand SCID's on-disk layout — see
  [`docs/api-reference.md`](docs/api-reference.md).
- **Built for real chess GUI use.** Originally extracted from a real
  chess GUI project's own needs (see [Origin](#origin)), not written
  as an abstract exercise.

## Status

| | si4 | si5 |
|---|---|---|
| Read index / name tables | done | done |
| Decode moves (main line) | done | done |
| Decode variations (nested) | done | done |
| Decode comments / NAGs | done | done |
| Non-standard starting positions (custom FEN) | done | done |
| ECO classification | done | done |
| Create a database, add games | done* | done* |
| Delete / undelete a game | done | done |
| Update an existing game (tags + moves) | done* | done* |
| Reclaim dead space (`compact`) | done | done |

\* `Si4Database`/`Si5Database::create`/`add_game`/`update_game`/`save`
cover the common case: create a new database (or open an existing one),
append games, and replace an already-added game's tags/moves entirely —
all given as full PGN text. `delete_game`/`undelete_game`/`is_deleted`
mark a game as deleted (the standard SCID `DELETE` flag, understood by a
real SCID installation too) without ever removing it from the file — a
deleted game stays fully readable and counted. A custom starting
position (`[FEN]`/`[SetUp]` tags) and an ECO classification (`[ECO]`
tag) are honored on write and re-emitted on read. si4's games file is
additionally capped at ~4 GB (a plain 32-bit offset field — the
historical reason si5 exists at all), against si5's ~140 TB — see
[Roadmap](#roadmap).

Not every field a real SCID installation stores per game round-trips
through this crate's PGN interface (e.g. `Annotator`, `PlyCount`,
`Source`) — see [`docs/pgn-reference.md`](docs/pgn-reference.md) for the
precise, honest list of what is and isn't preserved.

## Getting started

```rust
let paths = chess_scid_rw::Si4Paths::from_index_path("base.si4".as_ref());
let db = chess_scid_rw::Si4Database::open(&paths)?;

println!("{} games", db.game_count());
for n in 0..db.game_count() {
    match db.game_pgn(n) {
        Ok(pgn) => println!("{pgn}"),
        Err(e) => eprintln!("game {n}: skipped ({e})"),
    }
}
```

si5 works identically via `Si5Paths`/`Si5Database`. This is only a
taste — see [`docs/reading-examples.md`](docs/reading-examples.md) and
[`docs/writing-examples.md`](docs/writing-examples.md) for complete,
runnable examples of everything the API can do, and
[`examples/dump_games.rs`](examples/dump_games.rs)/
[`examples/extract_100_games.rs`](examples/extract_100_games.rs) for
runnable CLI tools built on it.

## Documentation

- [`docs/architecture.md`](docs/architecture.md) — how the crate is put
  together internally (module map, read/write data flow, the vendored
  chess engine's layering) — for anyone looking to fix a bug or extend
  the crate, not just use it.
- [`docs/api-reference.md`](docs/api-reference.md) — every public
  type/method: arguments, purpose, and the conditions for it to work.
- [`docs/reading-examples.md`](docs/reading-examples.md) — complete,
  read-only example programs (si4 and si5).
- [`docs/writing-examples.md`](docs/writing-examples.md) — complete
  example programs for creating and modifying a database (si4 and
  si5).
- [`docs/pgn-reference.md`](docs/pgn-reference.md) — the exact PGN
  dialect this crate reads and writes: recognized tags, move syntax,
  and what does and doesn't round-trip.
- [`docs/scid-si4-specification.md`](docs/scid-si4-specification.md)
  and [`docs/scid-si5-specification.md`](docs/scid-si5-specification.md)
  — independent, reverse-engineered specifications of the two binary
  formats (not official SCID documents), produced by cross-checking the
  reference SCID C++ source tree. They are the authoritative reference
  for this implementation and are extensively cross-referenced from the
  source code's doc comments.

## Feature flags

| Flag | Default | Enables |
|---|---|---|
| `chess-extras` | on | The parts of the vendored chess engine that the SCID decoder itself never uses: Polyglot opening-book reading, PGN import (`import_pgn`/`import_pgn_trusted`), undo/redo, playing a variation, promoting/removing a variation, and the linear "flatten" tree view. Also gates the whole si4/si5 write path (`Si4Database`/`Si5Database::create`/`add_game`/`update_game`/`delete_game`/`undelete_game`/`is_deleted`/`compact`/`save`, and `pgn_ingest`), since `add_game`/`update_game` parse their PGN input via `import_pgn`. |

On by default so nothing changes for existing users. If you only need
to read SCID databases and want a smaller, faster-compiling build:

```toml
chess-scid-rw = { version = "1.0", default-features = false }
```

## Origin

This decoder was originally written for, and is used by, the
[Vendetta Chess GUI](https://github.com/Fab2bprog/Rust-Vendetta-Chess-GUI)
project, then extracted here as a standalone crate for the wider Rust
chess community. `src/chess` is a vendored copy of Vendetta's own
dependency-free chess engine (position, move generation, SAN, PGN,
Polyglot books, a variation tree) — kept only as much as this crate
needs to decode SCID's move encoding; it is not the focus of this
crate's API stability guarantees and may be trimmed down over time.

## Roadmap

- [x] Writing/creating `.si5`/`.sn5`/`.sg5` databases — append-only.
- [x] Writing/creating `.si4`/`.sn4`/`.sg4` databases — same approach and
      scope as si5, sharing the same move-stream encoder
      (`game_blob::encode_mainline`) and `pgn_ingest`; only the header/
      counters and the sorted, front-coded name table are si4-specific.
- [x] Deleting/undeleting a game (`delete_game`/`undelete_game`/
      `is_deleted`, the standard SCID `DELETE` flag) and updating an
      already-added game's tags/moves entirely (`update_game`) — both
      symmetric across si4/si5, both implemented once in the shared
      `scid_common` module.
- [x] Reclaiming the dead space left behind in the `.sg4`/`.sg5` games
      file by `update_game` — `Si4Database`/`Si5Database::compact`,
      manual (not automatic on `save`, so its cost — proportional to the
      total games-file size — is only ever paid when actually asked for).
      A game's numbering/PGN content/deleted status are unaffected; only
      each entry's internal storage offset changes.
- [x] Writing a non-standard starting position (custom FEN) —
      `add_game`/`update_game` honor a `[FEN]` tag via
      [`chess::pgn::import_pgn`](src/chess/pgn.rs), `game_blob::encode_mainline`
      encodes it, and reading such a game back out emits the matching
      `[SetUp]`/`[FEN]` tags again.
- [x] Preserving a game's ECO classification (`[ECO]` tag) through
      `add_game`/`update_game`, instead of always discarding it.
- [x] A public helper (`chess::pgn::sanitize_tag_value`) for safely
      embedding a free-form value into a hand-built PGN tag before
      calling `add_game`/`update_game` — see
      [`docs/pgn-reference.md`](docs/pgn-reference.md).
- [x] Validated locally against real, external, multi-gigabyte `.si4`/`.si5`
      databases (not shipped with the crate) to confirm the reader
      handles real-world data, not just purpose-built round-trip tests.
- [ ] A fixture-file test suite that ships with the crate (small,
      purpose-built `.si4`/`.si5` files committed to the repository) —
      `tests/pure_functions.rs` currently only covers the
      file-independent building blocks (`bytes`, `dates`, `eco`); the
      write path is otherwise covered by in-memory round-trip unit
      tests (`src/game_blob.rs`, `src/si4/index.rs`,
      `src/si4/namebase.rs`, `src/si4/database.rs`, `src/si5/index.rs`,
      `src/si5/namebase.rs`, `src/si5/database.rs`) rather than real
      on-disk fixtures.
- [ ] Publish to [crates.io](https://crates.io) once the above lands and
      the crate name is confirmed available.
- [ ] Consider translating the (currently French) `Display` text of
      [`ScidError`]/[`GameDecodeError`] to English, or making it
      swappable, for a fully international audience.

## License

GNU General Public License v3 (GPL-3.0-or-later, see
[`LICENSE`](LICENSE)) — inherited from the original
[SCID](https://scid.sourceforge.net/) project (GPL) and from Vendetta
Chess GUI, since parts of the move decoder are ported directly from
SCID's C++ reference implementation.
