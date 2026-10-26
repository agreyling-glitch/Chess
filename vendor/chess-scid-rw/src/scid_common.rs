//! Private plumbing shared by `si4::database` and `si5::database`.
//!
//! The two formats' orchestration code is identical apart from two
//! things: the namebase/games file extensions, and how the index bytes
//! are decoded/encoded (si4 has a header to read/write first, si5 does
//! not). Both `Si4Database`/`Si5Database::open` pass in their
//! format-specific index reader as a closure and their namebase reader as
//! a function pointer; `save` symmetrically takes format-specific writer
//! closures. Everything else (reading/writing the three files,
//! `game_count`, `add_game`'s offset bookkeeping, and the offset/length ->
//! blob -> `pgn_build::build_pgn` logic of `game_pgn`) lives here once.
//! `Si4Paths`/`Si5Paths`/`Si4Database`/`Si5Database` stay the crate's
//! public types — this module is not part of the public API and its
//! shape may change freely.

use std::path::{Path, PathBuf};

use crate::entry::IndexEntry;
use crate::error::{GameDecodeError, ScidError, StorageLimitError};
use crate::names::NameTables;
use crate::pgn_build;

#[cfg(feature = "chess-extras")]
use crate::error::AddGameError;

/// Derives the three sibling file paths of a SCID database from its
/// index file path, given the target format's namebase/games
/// extensions (`"sn4"`/`"sg4"` for si4, `"sn5"`/`"sg5"` for si5).
pub(crate) fn derive_paths(index_path: &Path, namebase_ext: &str, games_ext: &str) -> (PathBuf, PathBuf, PathBuf) {
    (
        index_path.to_path_buf(),
        index_path.with_extension(namebase_ext),
        index_path.with_extension(games_ext),
    )
}

/// A SCID database (si4 or si5) fully loaded in memory: index decoded,
/// names resolved, games file kept as-is (individual blobs are
/// extracted on demand by `game_pgn`). Shared body of `Si4Database`/
/// `Si5Database`, which each wrap one of these.
pub(crate) struct LoadedDatabase {
    entries: Vec<IndexEntry>,
    names:   NameTables,
    games:   Vec<u8>,
}

impl LoadedDatabase {
    /// Reads the three files and decodes the index/namebase via the
    /// format-specific readers supplied by the caller — the only two
    /// steps that actually differ between si4 and si5.
    ///
    /// # Errors
    /// See [`ScidError`]: I/O error on one of the three files, or
    /// whatever `read_entries`/`read_names` return (invalid magic,
    /// truncated file, ...).
    pub(crate) fn load(
        index_path: &Path,
        namebase_path: &Path,
        games_path: &Path,
        read_entries: impl FnOnce(&[u8]) -> Result<Vec<IndexEntry>, ScidError>,
        read_names: impl FnOnce(&[u8]) -> Result<NameTables, ScidError>,
    ) -> Result<Self, ScidError> {
        let index_bytes = std::fs::read(index_path)?;
        let names_bytes = std::fs::read(namebase_path)?;
        let games_bytes = std::fs::read(games_path)?;

        let entries = read_entries(&index_bytes)?;
        let names   = read_names(&names_bytes)?;

        Ok(Self { entries, names, games: games_bytes })
    }

    /// Number of games in the database.
    pub(crate) fn game_count(&self) -> usize {
        self.entries.len()
    }

    /// Decodes game number `n` (0-based) into full PGN text.
    ///
    /// # Errors
    /// [`GameDecodeError::BadOffset`] if `n` is out of bounds or if
    /// `(offset, length)` exceeds the size of the games file; the other
    /// variants of [`GameDecodeError`] come from decoding the blob
    /// itself (see `game_blob`/`pgn_build`).
    pub(crate) fn game_pgn(&self, n: usize) -> Result<String, GameDecodeError> {
        let entry = self.entries.get(n).ok_or(GameDecodeError::BadOffset)?;
        // Note: `entry.non_standard_start` is informational only — no
        // early filtering on it here, `game_blob` decodes non-standard
        // starting positions normally (see its module doc).

        let start = usize::try_from(entry.offset).map_err(|_| GameDecodeError::BadOffset)?;
        let end = start
            .checked_add(entry.length as usize)
            .ok_or(GameDecodeError::BadOffset)?;
        let blob = self.games.get(start..end).ok_or(GameDecodeError::BadOffset)?;

        pgn_build::build_pgn(entry, &self.names, blob)
    }

    // -----------------------------------------------------------------------
    // Writing
    // -----------------------------------------------------------------------

    /// Creates a new, empty in-memory database (no disk I/O) — the shared
    /// body of `Si4Database::create`/`Si5Database::create`.
    pub(crate) fn empty() -> Self {
        Self { entries: Vec::new(), names: NameTables::default(), games: Vec::new() }
    }

    /// Mutable access to the name tables, for the caller to resolve/create
    /// player/event/site/round IDs before building the `IndexEntry` passed
    /// to [`Self::add_game`] (see `crate::names::NameTables::player_id_or_insert`
    /// and its siblings).
    pub(crate) fn names_mut(&mut self) -> &mut NameTables {
        &mut self.names
    }

    /// Appends `blob` to the games file and `entry` to the index —
    /// `entry.offset`/`entry.length` are computed here (any value the
    /// caller set on those two fields is overwritten), everything else on
    /// `entry` is used as provided.
    ///
    /// `max_offset_bits` is the width of the target format's `offset`
    /// field (see `si4::index::OFFSET_BITS`/`si5::index::OFFSET_BITS`,
    /// which callers pass in) — the two formats' index encodings can
    /// address very different games-file sizes (si4: ~4 GB, si5: ~140 TB),
    /// so this bound cannot be a single constant shared by both.
    ///
    /// # Errors
    /// [`StorageLimitError`] if `blob` or the resulting games file would
    /// exceed what the target format's on-disk index can address — checked
    /// BEFORE any mutation, so a failed call leaves the database
    /// completely unchanged.
    pub(crate) fn add_game(
        &mut self,
        mut entry: IndexEntry,
        blob: &[u8],
        max_offset_bits: u32,
    ) -> Result<(), StorageLimitError> {
        let length = u32::try_from(blob.len()).map_err(|_| StorageLimitError::GameTooLarge)?;
        if length > 0x1_FFFF {
            return Err(StorageLimitError::GameTooLarge);
        }
        let offset = u64::try_from(self.games.len()).map_err(|_| StorageLimitError::DatabaseTooLarge)?;
        if offset >= (1u64 << max_offset_bits) {
            return Err(StorageLimitError::DatabaseTooLarge);
        }

        entry.offset = offset;
        entry.length = length;
        self.games.extend_from_slice(blob);
        self.entries.push(entry);
        Ok(())
    }

    /// Sets game `n`'s (0-based) `deleted` status — the shared body of
    /// `Si4Database`/`Si5Database`'s `delete_game`/`undelete_game`. Setting
    /// the same status again (deleting an already-deleted game, or
    /// undeleting one that is not deleted) is not an error.
    ///
    /// # Errors
    /// [`GameDecodeError::BadOffset`] if `n` is out of bounds — reused here
    /// rather than a dedicated error type, exactly as [`Self::game_pgn`]
    /// already does for the same kind of out-of-bounds `n`.
    pub(crate) fn set_deleted(&mut self, n: usize, deleted: bool) -> Result<(), GameDecodeError> {
        let entry = self.entries.get_mut(n).ok_or(GameDecodeError::BadOffset)?;
        entry.deleted = deleted;
        Ok(())
    }

    /// Whether game `n` (0-based) is marked deleted — `None` if `n` is out
    /// of bounds, rather than an error: this is a query, not an operation
    /// that can meaningfully fail.
    pub(crate) fn is_deleted(&self, n: usize) -> Option<bool> {
        self.entries.get(n).map(|entry| entry.deleted)
    }

    /// Replaces game `n`'s (0-based) `IndexEntry`/blob entirely — the
    /// shared body of `Si4Database`/`Si5Database`'s `update_game`.
    ///
    /// Like [`Self::add_game`], `entry.offset`/`entry.length` are computed
    /// here (any value the caller set on those two fields is overwritten),
    /// and the new blob is appended to the games file rather than spliced
    /// in place — the old blob's bytes become unreachable dead space,
    /// reclaimable via [`Self::compact`] (not automatic). Unlike
    /// `add_game`, `entry.deleted` is ALSO overwritten here — with game
    /// `n`'s CURRENT status, regardless of what the caller set — since
    /// updating a game's tags/moves is not meant to also silently change
    /// whether it is deleted; use [`Self::set_deleted`] for that,
    /// independently.
    ///
    /// Returns [`AddGameError`] (rather than [`StorageLimitError`] like
    /// `add_game`) because, unlike an append, an update can also fail with
    /// [`AddGameError::NotFound`] if `n` does not exist — checked BEFORE
    /// any mutation (and before the storage-limit checks below), so a
    /// failed call leaves the database completely unchanged.
    ///
    /// Gated behind `chess-extras`: [`AddGameError`] itself is (it
    /// composes [`crate::chess::pgn::PgnError`]), so this method — unlike
    /// every other method of [`LoadedDatabase`] — cannot be compiled
    /// without the feature either.
    ///
    /// # Errors
    /// See [`AddGameError`]: `n` out of bounds, or the same
    /// [`StorageLimitError`] conditions as [`Self::add_game`].
    #[cfg(feature = "chess-extras")]
    pub(crate) fn update_game(
        &mut self,
        n: usize,
        mut entry: IndexEntry,
        blob: &[u8],
        max_offset_bits: u32,
    ) -> Result<(), AddGameError> {
        if n >= self.entries.len() {
            return Err(AddGameError::NotFound);
        }

        let length = u32::try_from(blob.len()).map_err(|_| StorageLimitError::GameTooLarge)?;
        if length > 0x1_FFFF {
            return Err(StorageLimitError::GameTooLarge.into());
        }
        let offset = u64::try_from(self.games.len()).map_err(|_| StorageLimitError::DatabaseTooLarge)?;
        if offset >= (1u64 << max_offset_bits) {
            return Err(StorageLimitError::DatabaseTooLarge.into());
        }

        entry.offset = offset;
        entry.length = length;
        entry.deleted = self.entries[n].deleted;
        self.games.extend_from_slice(blob);
        self.entries[n] = entry;
        Ok(())
    }

    /// Rewrites the games file compactly in memory, dropping any bytes not
    /// referenced by a current entry's `(offset, length)` span — reclaims
    /// the dead space [`Self::update_game`] (and any future
    /// permanent-delete) leaves behind. Every game's numbering (its
    /// position in the index, used everywhere else in this crate's API),
    /// PGN content, and deleted status (see [`Self::set_deleted`]) are
    /// completely unaffected: only each `IndexEntry`'s internal `offset`
    /// changes.
    ///
    /// A deleted-but-not-yet-updated game is NOT dead space and is kept
    /// as-is: soft-delete (see [`Self::set_deleted`]) never orphans a
    /// game's bytes, it only flags the (still fully valid) entry that
    /// already points to them — only a game's PREVIOUS blob, left behind
    /// by [`Self::update_game`], is ever reclaimed here.
    ///
    /// A no-op (returns 0, no reallocation of meaningfully different
    /// content) if there is no dead space to reclaim — safe to call
    /// unconditionally, e.g. before every [`Self::save`]. Manual rather
    /// than automatic: unlike every other method here, its cost is
    /// proportional to the TOTAL size of the games file rather than to
    /// the size of what actually changed, so this crate never pays it
    /// without being explicitly asked to.
    ///
    /// Call [`Self::save`] afterward to persist the result; like every
    /// other mutation in this crate, `compact` only touches the in-memory
    /// copy.
    ///
    /// Returns the number of bytes reclaimed.
    pub(crate) fn compact(&mut self) -> usize {
        let mut new_games = Vec::with_capacity(self.games.len());
        for entry in &mut self.entries {
            let start = usize::try_from(entry.offset).unwrap_or(usize::MAX);
            let end = start.saturating_add(entry.length as usize);
            let blob = self.games.get(start..end).unwrap_or(&[]);
            debug_assert_eq!(
                blob.len(),
                entry.length as usize,
                "une IndexEntry existante doit toujours pointer vers un intervalle valide de `games`"
            );

            let new_offset = new_games.len();
            new_games.extend_from_slice(blob);
            entry.offset = u64::try_from(new_offset).unwrap_or(u64::MAX);
        }

        let reclaimed = self.games.len().saturating_sub(new_games.len());
        self.games = new_games;
        reclaimed
    }

    /// Serializes the three files (via the format-specific writers
    /// supplied by the caller) and writes them to disk atomically (each
    /// file: write to a sibling temporary file, then rename over the
    /// final path) — a crash or I/O error partway through leaves the
    /// previous, still-valid files in place rather than a half-written
    /// database.
    ///
    /// # Errors
    /// [`ScidError::Io`] on any I/O failure. Note: if writing succeeds for
    /// some of the three files but fails partway through the others, those
    /// already-written files ARE left updated (this method does not roll
    /// back earlier successful writes) — full transactional all-or-nothing
    /// semantics across three separate files would need a journal/lock
    /// file this crate does not implement; each INDIVIDUAL file's write is
    /// still atomic, which is what protects against a torn/partial file.
    pub(crate) fn save(
        &self,
        index_path: &Path,
        namebase_path: &Path,
        games_path: &Path,
        write_entries: impl FnOnce(&[IndexEntry]) -> Vec<u8>,
        write_names: impl FnOnce(&NameTables) -> Vec<u8>,
    ) -> Result<(), ScidError> {
        write_atomically(index_path, &write_entries(&self.entries))?;
        write_atomically(namebase_path, &write_names(&self.names))?;
        write_atomically(games_path, &self.games)?;
        Ok(())
    }
}

/// Writes `data` to `path` atomically: written in full to a sibling
/// temporary file first, then renamed over `path` (a rename is atomic on
/// the same filesystem) — a process crash or I/O error during the write
/// itself leaves the previous file at `path` untouched rather than
/// truncated/corrupted.
fn write_atomically(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let tmp_name = format!("{}.tmp", path.file_name().and_then(|n| n.to_str()).unwrap_or("scid"));
    let tmp_path = path.with_file_name(tmp_name);
    std::fs::write(&tmp_path, data)?;
    std::fs::rename(&tmp_path, path)
}
