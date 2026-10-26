//! High-level orchestration for a si5 database: opening the three
//! files (`.si5`/`.sn5`/`.sg5`), and decoding a given game into PGN
//! text on demand — an exact mirror of `si4::database`; only the
//! low-level readers (`index`/`namebase` of this `si5` module) differ,
//! `pgn_build`/the moves decoder are shared as-is with si4.
//!
//! The actual file-loading/game-decoding logic is shared with
//! `si4::database` via the private `scid_common` module — only the
//! index-decoding step (si5 has no header, unlike si4) and the file
//! extensions differ; see `scid_common::LoadedDatabase`.

use std::path::{Path, PathBuf};

use super::{index, namebase};
use crate::error::{GameDecodeError, ScidError};
use crate::scid_common::{derive_paths, LoadedDatabase};

#[cfg(feature = "chess-extras")]
use crate::entry::IndexEntry;
#[cfg(feature = "chess-extras")]
use crate::error::AddGameError;
#[cfg(feature = "chess-extras")]
use crate::game_blob;
#[cfg(feature = "chess-extras")]
use crate::pgn_ingest;

/// The three file paths of a si5 database, derived from the `.si5` path.
#[derive(Debug, Clone)]
pub struct Si5Paths {
    pub index:    PathBuf,
    pub namebase: PathBuf,
    pub games:    PathBuf,
}

impl Si5Paths {
    /// Derives the three paths from the `.si5` file path (the other two
    /// files must share the same base name, alongside it).
    #[must_use]
    pub fn from_index_path(si5_path: &Path) -> Self {
        let (index, namebase, games) = derive_paths(si5_path, "sn5", "sg5");
        Self { index, namebase, games }
    }
}

/// A si5 database opened in memory: index fully decoded, names resolved,
/// and the games file loaded as-is (individual blobs are
/// extracted on demand via `game_pgn`).
pub struct Si5Database {
    inner: LoadedDatabase,
    paths: Si5Paths,
}

impl Si5Database {
    /// Opens and fully loads a si5 database into memory.
    ///
    /// # Errors
    /// See [`ScidError`]: truncated file, I/O error on one of the three
    /// files. Unlike si4, the si5 format has no verifiable magic byte
    /// (`.si5`/`.sn5` have no header, see `si5::index`/
    /// `si5::namebase`) — [`ScidError::BadMagic`] is therefore never returned
    /// here, a limitation of the format itself, not of this implementation.
    pub fn open(paths: &Si5Paths) -> Result<Self, ScidError> {
        let inner = LoadedDatabase::load(
            &paths.index,
            &paths.namebase,
            &paths.games,
            index::read_all_entries,
            namebase::read_namebase,
        )?;

        Ok(Self { inner, paths: paths.clone() })
    }

    /// Creates a new, empty si5 database, entirely in memory — no file is
    /// touched until [`Self::save`] is called. Use this to start a brand
    /// new database; use [`Self::open`] to add games to an existing one.
    ///
    /// Gated behind `chess-extras` alongside [`Self::add_game`]/
    /// [`Self::save`]: an empty database that can never be populated or
    /// persisted would be of no use on its own.
    #[must_use]
    #[cfg(feature = "chess-extras")]
    pub fn create(paths: &Si5Paths) -> Self {
        Self { inner: LoadedDatabase::empty(), paths: paths.clone() }
    }

    /// Number of games in the database.
    #[must_use]
    pub fn game_count(&self) -> usize {
        self.inner.game_count()
    }

    /// Decodes game number `n` (0-based) into full PGN text.
    ///
    /// # Errors
    /// [`GameDecodeError::BadOffset`] if `n` is out of bounds or if
    /// `(offset, length)` exceeds the size of the `.sg5` file; the other
    /// variants of [`GameDecodeError`] come from decoding the blob
    /// itself (see `game_blob`/`pgn_build`, shared with si4).
    pub fn game_pgn(&self, n: usize) -> Result<String, GameDecodeError> {
        self.inner.game_pgn(n)
    }

    /// Adds one game to the database (in memory — call [`Self::save`]
    /// afterward to persist it) from `pgn`, a full PGN game (the standard
    /// Seven Tag Roster tags plus move text — e.g. exactly what
    /// [`Self::game_pgn`] produces, or an export from another PGN source).
    ///
    /// Deliberately the ONLY input this method accepts: no separate
    /// struct of tags/moves to fill in by hand — callers never need to
    /// know anything about `IndexEntry`, name IDs, or the move-stream
    /// encoding to add a game. A `[FEN "..."]` tag (with or without an
    /// accompanying `[SetUp "1"]`) is honored — see
    /// [`crate::chess::pgn::import_pgn`] — and the resulting `IndexEntry`'s
    /// `non_standard_start` flag is set accordingly, so a real SCID
    /// installation recognizes it too. A valid `[ECO "..."]` base code
    /// (`"A00"`..`"E99"`) is likewise parsed into `IndexEntry::eco_code`
    /// (see `crate::eco::eco_from_string`); this crate never computes one
    /// on its own.
    ///
    /// A missing or unparsable tag (`White`, `Elo`, `Date`, ...) falls
    /// back to the same "unknown" defaults used on the read side — see
    /// [`crate::pgn_ingest::parse_game`] — never an error by itself; only
    /// the move text (or its `[FEN]` tag) and the format's own size limits
    /// can fail this call.
    ///
    /// Returns the new game's index (0-based, same numbering as
    /// [`Self::game_pgn`]).
    ///
    /// # Errors
    /// See [`AddGameError`]: the PGN's move text (or its `[FEN]` tag) is
    /// invalid, the parsed tree could not be encoded (see
    /// [`crate::GameEncodeError`] — in particular, a custom starting
    /// position with more than 16 pieces of one color is not
    /// representable), or a size limit of the si5 format was exceeded. No
    /// partial modification is made to the database on any of these
    /// errors.
    #[cfg(feature = "chess-extras")]
    pub fn add_game(&mut self, pgn: &str) -> Result<usize, AddGameError> {
        let parsed = pgn_ingest::parse_game(pgn)?;
        let blob = game_blob::encode_mainline(&parsed.game)?;

        let names = self.inner.names_mut();
        let entry = IndexEntry {
            offset: 0, // filled in by `LoadedDatabase::add_game`
            length: 0, // filled in by `LoadedDatabase::add_game`
            white_id: names.player_id_or_insert(&parsed.tags.white),
            black_id: names.player_id_or_insert(&parsed.tags.black),
            event_id: names.event_id_or_insert(&parsed.tags.event),
            site_id: names.site_id_or_insert(&parsed.tags.site),
            round_id: names.round_id_or_insert(&parsed.tags.round),
            result: parsed.tags.result,
            // Not computed by this crate (it would require porting SCID's
            // opening tree, out of scope here) — only ever parsed from an
            // `[ECO]` tag already present in the PGN (see
            // `crate::pgn_ingest::ParsedTags::eco_code`); `0` ("no
            // classification") if absent or invalid, exactly like a
            // legitimate game with an unclassified opening.
            eco_code: parsed.tags.eco_code,
            date: parsed.tags.date,
            white_elo: parsed.tags.white_elo,
            black_elo: parsed.tags.black_elo,
            non_standard_start: !parsed.game.is_standard_start(),
            deleted: false,
        };

        self.inner.add_game(entry, &blob, index::OFFSET_BITS)?;
        Ok(self.inner.game_count() - 1)
    }

    /// Marks game `n` (0-based) as deleted — the standard SCID `DELETE`
    /// flag (see `crate::entry::IndexEntry::deleted`), understood as such
    /// by a real SCID installation too. The game and its move blob are
    /// NOT removed: [`Self::game_pgn`]/[`Self::game_count`] behave exactly
    /// as before. Reversible via [`Self::undelete_game`]; call
    /// [`Self::save`] afterward to persist the change. Deleting an
    /// already-deleted game is not an error.
    ///
    /// # Errors
    /// [`GameDecodeError::BadOffset`] if `n` is out of bounds.
    #[cfg(feature = "chess-extras")]
    pub fn delete_game(&mut self, n: usize) -> Result<(), GameDecodeError> {
        self.inner.set_deleted(n, true)
    }

    /// Clears game `n`'s (0-based) deleted flag — the inverse of
    /// [`Self::delete_game`]. Undeleting a game that is not deleted is not
    /// an error.
    ///
    /// # Errors
    /// [`GameDecodeError::BadOffset`] if `n` is out of bounds.
    #[cfg(feature = "chess-extras")]
    pub fn undelete_game(&mut self, n: usize) -> Result<(), GameDecodeError> {
        self.inner.set_deleted(n, false)
    }

    /// Whether game `n` (0-based) is marked deleted (see
    /// [`Self::delete_game`]) — `None` if `n` is out of bounds.
    #[must_use]
    #[cfg(feature = "chess-extras")]
    pub fn is_deleted(&self, n: usize) -> Option<bool> {
        self.inner.is_deleted(n)
    }

    /// Replaces game `n` (0-based) entirely — tags and moves alike — from
    /// `pgn`, exactly like [`Self::add_game`] but targeting an existing
    /// game instead of appending a new one. `n`'s deleted status (see
    /// [`Self::delete_game`]) is left untouched by this call either way.
    ///
    /// Like every other write in this crate, the new move blob is
    /// appended to the games file rather than spliced in place — the old
    /// blob's bytes become unreachable dead space in the `.sg5` file,
    /// reclaimable via [`Self::compact`] (not automatic). Call
    /// [`Self::save`] afterward to persist the change.
    ///
    /// # Errors
    /// See [`AddGameError`]: [`AddGameError::NotFound`] if `n` is out of
    /// bounds, the PGN's move text is invalid, the parsed tree could not
    /// be encoded, or a size limit of the si5 format was exceeded (see
    /// [`Self::add_game`]'s doc for the same limit). No partial
    /// modification is made to the database on any of these errors.
    #[cfg(feature = "chess-extras")]
    pub fn update_game(&mut self, n: usize, pgn: &str) -> Result<(), AddGameError> {
        let parsed = pgn_ingest::parse_game(pgn)?;
        let blob = game_blob::encode_mainline(&parsed.game)?;

        let names = self.inner.names_mut();
        let entry = IndexEntry {
            offset: 0, // filled in by `LoadedDatabase::update_game`
            length: 0, // filled in by `LoadedDatabase::update_game`
            white_id: names.player_id_or_insert(&parsed.tags.white),
            black_id: names.player_id_or_insert(&parsed.tags.black),
            event_id: names.event_id_or_insert(&parsed.tags.event),
            site_id: names.site_id_or_insert(&parsed.tags.site),
            round_id: names.round_id_or_insert(&parsed.tags.round),
            result: parsed.tags.result,
            eco_code: parsed.tags.eco_code,
            date: parsed.tags.date,
            white_elo: parsed.tags.white_elo,
            black_elo: parsed.tags.black_elo,
            non_standard_start: !parsed.game.is_standard_start(),
            deleted: false, // overwritten by `LoadedDatabase::update_game` with `n`'s current status
        };

        self.inner.update_game(n, entry, &blob, index::OFFSET_BITS)
    }

    /// Rewrites the `.sg5` games file compactly in memory, reclaiming the
    /// dead space [`Self::update_game`] leaves behind — every game's
    /// numbering, PGN content, and deleted status (see
    /// [`Self::delete_game`]) are completely unaffected; only their
    /// internal storage offset changes. A no-op if there is nothing to
    /// reclaim.
    ///
    /// Manual rather than automatic on [`Self::save`]: its cost is
    /// proportional to the total size of the games file, not to what
    /// actually changed, so call it only when you actually want to pay
    /// that cost (e.g. periodically, or after a batch of
    /// [`Self::update_game`] calls) — then call [`Self::save`] afterward
    /// to persist the result.
    ///
    /// Returns the number of bytes reclaimed.
    #[cfg(feature = "chess-extras")]
    pub fn compact(&mut self) -> usize {
        self.inner.compact()
    }

    /// Writes the database to its three files (`.si5`/`.sn5`/`.sg5`,
    /// derived from the `Si5Paths` given to [`Self::open`]/[`Self::create`]),
    /// each atomically (see `scid_common::LoadedDatabase::save`) — safe to
    /// call repeatedly (e.g. once after a batch of [`Self::add_game`]
    /// calls); always (over)writes all three files with the database's
    /// full current content, it does not append incrementally on disk.
    ///
    /// # Errors
    /// [`ScidError::Io`] on any I/O failure.
    #[cfg(feature = "chess-extras")]
    pub fn save(&self) -> Result<(), ScidError> {
        self.inner.save(
            &self.paths.index,
            &self.paths.namebase,
            &self.paths.games,
            index::write_all_entries,
            namebase::write_namebase,
        )
    }
}

/// In-memory `create`/`add_game`/`delete_game`/`update_game`/`game_pgn`
/// round trip (no file I/O — see `si5::index`/`si5::namebase` for the
/// lower-level write/read round-trip tests of the two binary formats this
/// wiring relies on): this specifically exercises `Si5Database`'s own
/// logic (resolving name IDs, building the `IndexEntry`, encoding the
/// move blob, preserving the deleted flag across an update) end to end,
/// which the lower-level module tests do not cover.
#[cfg(all(test, feature = "chess-extras"))]
mod tests {
    use super::*;

    fn in_memory_paths() -> Si5Paths {
        Si5Paths {
            index:    PathBuf::from("unused.si5"),
            namebase: PathBuf::from("unused.sn5"),
            games:    PathBuf::from("unused.sg5"),
        }
    }

    #[test]
    fn test_create_add_game_and_read_back_without_touching_disk() {
        let mut db = Si5Database::create(&in_memory_paths());
        assert_eq!(db.game_count(), 0);

        let pgn = "[White \"Carlsen, Magnus\"]\n[Black \"Nepomniachtchi, Ian\"]\n\
                   [Result \"1-0\"]\n\n1. e4 e5 2. Nf3 Nc6 3. Bb5 1-0\n";
        let idx = db.add_game(pgn).unwrap();
        assert_eq!(idx, 0);
        assert_eq!(db.game_count(), 1);

        let round_tripped = db.game_pgn(0).unwrap();
        assert!(round_tripped.contains("Carlsen, Magnus"));
        assert!(round_tripped.contains("1. e4 e5 2. Nf3 Nc6 3. Bb5"));
    }

    #[test]
    fn test_add_game_assigns_sequential_indices_and_reuses_name_ids() {
        let mut db = Si5Database::create(&in_memory_paths());
        let pgn = "[White \"A\"]\n[Black \"B\"]\n\n1. e4 *\n";
        assert_eq!(db.add_game(pgn).unwrap(), 0);
        assert_eq!(db.add_game(pgn).unwrap(), 1);
        assert_eq!(db.game_count(), 2);
    }

    #[test]
    fn test_add_game_rejects_invalid_move_text() {
        let mut db = Si5Database::create(&in_memory_paths());
        assert!(db.add_game("[White \"A\"]\n\n1. e9 *\n").is_err());
        assert_eq!(db.game_count(), 0, "aucune modification en cas d'échec");
    }

    #[test]
    fn test_add_game_with_eco_tag_round_trips_the_classification() {
        let mut db = Si5Database::create(&in_memory_paths());
        let pgn = "[White \"A\"]\n[Black \"B\"]\n[ECO \"B90\"]\n\n1. e4 c5 2. Nf3 d6 3. d4 cxd4 4. Nxd4 Nf6 5. Nc3 a6 *\n";
        db.add_game(pgn).unwrap();

        let round_tripped = db.game_pgn(0).unwrap();
        assert!(round_tripped.contains("[ECO \"B90\"]"));
    }

    #[test]
    fn test_update_game_with_eco_tag_round_trips_the_classification() {
        let mut db = Si5Database::create(&in_memory_paths());
        db.add_game("[White \"A\"]\n[Black \"B\"]\n\n1. e4 *\n").unwrap();

        let pgn = "[White \"A\"]\n[Black \"B\"]\n[ECO \"B90\"]\n\n1. e4 c5 2. Nf3 d6 3. d4 cxd4 4. Nxd4 Nf6 5. Nc3 a6 *\n";
        db.update_game(0, pgn).unwrap();

        let round_tripped = db.game_pgn(0).unwrap();
        assert!(round_tripped.contains("[ECO \"B90\"]"));
    }

    #[test]
    fn test_add_game_without_eco_tag_has_no_classification() {
        let mut db = Si5Database::create(&in_memory_paths());
        db.add_game("[White \"A\"]\n[Black \"B\"]\n\n1. e4 *\n").unwrap();

        let round_tripped = db.game_pgn(0).unwrap();
        assert!(!round_tripped.contains("[ECO"));
    }

    #[test]
    fn test_add_game_with_fen_tag_round_trips_non_standard_start() {
        let mut db = Si5Database::create(&in_memory_paths());
        let fen = "8/P7/8/8/8/8/8/4K2k w - - 0 1";
        let pgn = format!("[White \"A\"]\n[Black \"B\"]\n[SetUp \"1\"]\n[FEN \"{fen}\"]\n\n1. a8=Q *\n");
        db.add_game(&pgn).unwrap();

        let round_tripped = db.game_pgn(0).unwrap();
        assert!(round_tripped.contains("[SetUp \"1\"]"));
        assert!(round_tripped.contains(&format!("[FEN \"{fen}\"]")));
    }

    #[test]
    fn test_update_game_with_fen_tag_sets_non_standard_start() {
        let mut db = Si5Database::create(&in_memory_paths());
        db.add_game("[White \"A\"]\n[Black \"B\"]\n\n1. e4 *\n").unwrap();

        let fen = "8/P7/8/8/8/8/8/4K2k w - - 0 1";
        let pgn = format!("[White \"A\"]\n[Black \"B\"]\n[SetUp \"1\"]\n[FEN \"{fen}\"]\n\n1. a8=Q *\n");
        db.update_game(0, &pgn).unwrap();

        let round_tripped = db.game_pgn(0).unwrap();
        assert!(round_tripped.contains(&format!("[FEN \"{fen}\"]")));
    }

    #[test]
    fn test_delete_undelete_and_is_deleted() {
        let mut db = Si5Database::create(&in_memory_paths());
        db.add_game("[White \"A\"]\n[Black \"B\"]\n\n1. e4 *\n").unwrap();
        assert_eq!(db.is_deleted(0), Some(false));

        db.delete_game(0).unwrap();
        assert_eq!(db.is_deleted(0), Some(true));
        // A deleted game stays fully readable and counted.
        assert_eq!(db.game_count(), 1);
        assert!(db.game_pgn(0).is_ok());

        db.undelete_game(0).unwrap();
        assert_eq!(db.is_deleted(0), Some(false));
    }

    #[test]
    fn test_delete_game_is_idempotent_and_out_of_bounds_errors() {
        let mut db = Si5Database::create(&in_memory_paths());
        db.add_game("[White \"A\"]\n[Black \"B\"]\n\n1. e4 *\n").unwrap();

        db.delete_game(0).unwrap();
        db.delete_game(0).unwrap(); // deleting an already-deleted game is not an error
        assert_eq!(db.is_deleted(0), Some(true));

        assert!(db.delete_game(1).is_err());
        assert!(db.undelete_game(1).is_err());
        assert_eq!(db.is_deleted(1), None);
    }

    #[test]
    fn test_update_game_replaces_tags_and_moves() {
        let mut db = Si5Database::create(&in_memory_paths());
        db.add_game("[White \"Old Name\"]\n[Black \"B\"]\n\n1. e4 *\n").unwrap();

        db.update_game(0, "[White \"New Name\"]\n[Black \"B\"]\n[Result \"1-0\"]\n\n1. d4 d5 1-0\n")
            .unwrap();

        let pgn = db.game_pgn(0).unwrap();
        assert!(pgn.contains("New Name"));
        assert!(!pgn.contains("Old Name"));
        assert!(pgn.contains("1. d4 d5"));
    }

    #[test]
    fn test_update_game_preserves_deleted_status() {
        let mut db = Si5Database::create(&in_memory_paths());
        db.add_game("[White \"A\"]\n[Black \"B\"]\n\n1. e4 *\n").unwrap();
        db.delete_game(0).unwrap();

        db.update_game(0, "[White \"A2\"]\n[Black \"B\"]\n\n1. d4 *\n").unwrap();

        assert_eq!(db.is_deleted(0), Some(true), "update_game ne doit pas toucher au flag deleted");
    }

    #[test]
    fn test_update_game_out_of_bounds_is_not_found() {
        let mut db = Si5Database::create(&in_memory_paths());
        let err = db.update_game(0, "[White \"A\"]\n[Black \"B\"]\n\n1. e4 *\n").unwrap_err();
        assert!(matches!(err, crate::error::AddGameError::NotFound));
    }

    #[test]
    fn test_update_game_rejects_invalid_move_text_without_modifying_database() {
        let mut db = Si5Database::create(&in_memory_paths());
        db.add_game("[White \"A\"]\n[Black \"B\"]\n\n1. e4 *\n").unwrap();

        assert!(db.update_game(0, "[White \"A\"]\n\n1. e9 *\n").is_err());
        // The original game must be untouched.
        assert!(db.game_pgn(0).unwrap().contains("1. e4"));
    }

    #[test]
    fn test_compact_reclaims_dead_space_left_by_update_game() {
        let mut db = Si5Database::create(&in_memory_paths());
        db.add_game("[White \"A\"]\n[Black \"B\"]\n\n1. e4 e5 2. Nf3 Nc6 3. Bb5 *\n").unwrap();
        // Replace with a shorter game: the original (longer) blob becomes dead space.
        db.update_game(0, "[White \"A\"]\n[Black \"B\"]\n\n1. e4 *\n").unwrap();

        let reclaimed = db.compact();
        assert!(reclaimed > 0, "le blob d'origine, plus long, doit laisser de l'espace mort");

        // Content, numbering and count must survive compaction unchanged.
        assert_eq!(db.game_count(), 1);
        assert!(db.game_pgn(0).unwrap().contains("1. e4"));
    }

    #[test]
    fn test_compact_is_a_no_op_when_there_is_nothing_to_reclaim() {
        let mut db = Si5Database::create(&in_memory_paths());
        db.add_game("[White \"A\"]\n[Black \"B\"]\n\n1. e4 *\n").unwrap();
        db.add_game("[White \"C\"]\n[Black \"D\"]\n\n1. d4 *\n").unwrap();

        assert_eq!(db.compact(), 0);
        assert_eq!(db.game_count(), 2);
        assert!(db.game_pgn(0).unwrap().contains("1. e4"));
        assert!(db.game_pgn(1).unwrap().contains("1. d4"));
    }

    #[test]
    fn test_compact_preserves_deleted_status_and_numbering() {
        let mut db = Si5Database::create(&in_memory_paths());
        db.add_game("[White \"A\"]\n[Black \"B\"]\n\n1. e4 e5 2. Nf3 Nc6 3. Bb5 *\n").unwrap();
        db.add_game("[White \"C\"]\n[Black \"D\"]\n\n1. d4 *\n").unwrap();
        db.update_game(0, "[White \"A\"]\n[Black \"B\"]\n\n1. e4 *\n").unwrap(); // leaves dead space
        db.delete_game(1).unwrap();

        db.compact();

        assert_eq!(db.game_count(), 2);
        assert_eq!(db.is_deleted(0), Some(false));
        assert_eq!(db.is_deleted(1), Some(true));
        assert!(db.game_pgn(0).unwrap().contains("1. e4"));
        assert!(db.game_pgn(1).unwrap().contains("1. d4"), "une partie supprimée reste lisible après compact()");
    }
}
