# Private-preview archive storage, schema 4

The owner-local archive retains its existing HTTP document/PGN transfer contract.
SQLite stores searchable game metadata as JSON, with the PGN field empty. Each
game's `game_payloads.moves` is a versioned BLOB using shakmaty `PackedUciMove`
(two bytes per move), literal annotation packets, variation boundaries and an
initial position/castling mode. The packet stream is zlib-compressed using the
existing flate2 dependency. Noncanonical SAN spelling is retained as a move
override. Comments, NAGs, nesting, Unicode and whitespace are retained. Decode
and replay enforce size/move/nesting bounds. Opening indexing replays packed
mainline moves directly without generating/reparsing PGN.

Original browser snapshots are stored separately as bounded compressed BLOBs;
the small JSON envelope retains category/favorite/source metadata for searches.
This preserves annotations, training state, marks, analysis caches and browser
round trips without keeping a second uncompressed move history in SQLite JSON.
Independent analysis runs remain in their existing tables.

`eco_openings` holds one description per ECO code, seeded from the existing CC0
opening dataset. The shortest catalog move line represents each code (ties prefer
the shorter label), avoiding accidental selection of an unusual transposition.
Only ECO codes are persisted in game metadata and library cards. Source Opening
tags and cached opening labels are removed; outgoing PGN/documents/cards resolve
the description from the lookup table. These descriptions are intentionally
normalized rather than retaining an inconsistent source label per game.

Fingerprints exclude resolved labels, so exporting and reimporting the same
annotated version remains a duplicate. Imports, edits, analysis-cache updates,
JSON restores and deletes update binary payloads in the same transaction as the
game row. Foreign-key cascading removes payloads on deletion. JSON backups remain
portable documents with PGN; schema 2/3 backups remain importable, and schema 4
backups are supported by the updated browser bridge. Legacy text records remain
readable, while new/replaced records use binary storage.

For the owner's explicitly requested clean reset, stop Companion gracefully,
preserve the complete archive directory and Import directory together on the same
volume, and create empty replacements. Never delete the owner credential, config,
engines or Reports. Keeping the old import queue with its database prevents its
batch receipts from being reused against a new archive. Restart the default
`output/companion/current` build with the same data directory and verify zero games.
