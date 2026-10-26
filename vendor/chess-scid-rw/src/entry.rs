//! Game metadata decoded from the index (`.si4` or `.si5`) — a NEUTRAL
//! structure, independent of the original disk format, shared by BOTH
//! index readers (`si4::index`/`si5::index`, very different disk formats
//! — see `docs/scid-si4-specification.md` §2 vs
//! `docs/scid-si5-specification.md` §2): `pgn_build::build_pgn` only knows
//! this structure, never the original disk format, which allows it to
//! be reused as-is for both.

/// Game metadata useful for reconstructing a PGN (the derived/heuristic
/// fields specific to each disk format — `FinalMatSig`,
/// `StoredLineCode`, `HomePawnData`, approximate counters for
/// variations/comments/NAGs, etc. — are read by the corresponding index
/// reader to advance the cursor correctly, but never kept
/// here: they have no bearing on the validity of a reconstructed PGN).
#[derive(Debug, Clone)]
pub struct IndexEntry {
    /// Offset of the game blob in the games file (`.sg4`/`.sg5`).
    pub offset: u64,
    /// Length of the game blob in the games file.
    pub length: u32,
    pub white_id: u32,
    pub black_id: u32,
    pub event_id: u32,
    pub site_id: u32,
    pub round_id: u32,
    /// Raw result (0=none, 1=White, 2=Black, 3=Draw).
    pub result: u8,
    pub eco_code: u16,
    /// Date of the game, raw `dateT` encoding.
    pub date: u32,
    pub white_elo: u16,
    pub black_elo: u16,
    /// `START` flag: non-standard starting position (custom FEN),
    /// decoded normally by [`crate::game_blob::decode_mainline`] (see
    /// `build_piece_lists`); kept here mostly for informational purposes
    /// (diagnostics, potential sorting).
    pub non_standard_start: bool,
    /// `DELETE` flag: the game is marked for deletion (a real SCID
    /// installation, and any other spec-compliant reader, understands
    /// this exactly like this crate does — the game is neither removed
    /// from the index nor from the games file, only flagged). Same bit
    /// (3) in both si4's and si5's raw flags field — see
    /// `crate::si4::database::Si4Database::delete_game`/
    /// `crate::si5::database::Si5Database::delete_game`.
    pub deleted: bool,
}
