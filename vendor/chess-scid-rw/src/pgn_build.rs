//! Final assembly: `IndexEntry` + name tables + `.sg4`/`.sg5` blob of a
//! game -> full PGN text. This module only produces PGN text, it never
//! touches any storage backend — how the caller uses the resulting
//! `String` (write to a `.pgn` file, feed a database importer, ...) is
//! entirely up to it.
//!
//! `IndexEntry`/`NameTables` are NEUTRAL structures ([`crate::entry`]/
//! [`crate::names`], shared by [`crate::si4`] and [`crate::si5`]): this
//! module therefore knows nothing about the original disk format (si4 or
//! si5) — it works identically for both, as long as the caller provides it
//! with these already-resolved structures.

use std::fmt::Write as _;

use crate::dates;
use crate::eco;
use crate::entry::IndexEntry;
use crate::error::GameDecodeError;
use crate::game_blob;
use crate::names::NameTables;
use crate::chess::game::GameState;
use crate::chess::pgn::{export_pgn, sanitize_tag_value, PgnTags};
use crate::chess::types::game_state::GameResult;

/// Builds the full PGN text of a game described by `entry` (resolved
/// via `names`) whose moves are encoded in `blob` (the bytes
/// `[entry.offset, entry.offset + entry.length)` of the `.sg4`/`.sg5`, already
/// extracted by the caller — see `si4::database`).
///
/// # Errors
/// See [`GameDecodeError`]: the game is skipped (not the whole database)
/// on failure — corrupted move stream, illegal move once resolved,
/// truncated blob... A non-standard starting position is decoded
/// normally (see `game_blob`'s module doc) and never causes a failure by
/// itself.
pub fn build_pgn(entry: &IndexEntry, names: &NameTables, blob: &[u8]) -> Result<String, GameDecodeError> {
    let mut game = game_blob::decode_mainline(blob)?;

    // The result comes from the IndexEntry (§1.1: 0=none, 1=White, 2=Black,
    // 3=Draw), NOT from automatic checkmate/stalemate detection by
    // `crate::chess` (most real games end by resignation, not by a terminal
    // state detectable on the board). `export_pgn` overwrites the Result
    // tag with `game.result` anyway (see `pgn.rs`), so it is THIS field
    // that needs to be set correctly, not the tags.
    game.result = match entry.result {
        1 => GameResult::WhiteWins,
        2 => GameResult::BlackWins,
        3 => GameResult::Draw,
        _ => GameResult::Ongoing,
    };

    let tags = PgnTags {
        event:  sanitize_tag_value(names.event(entry.event_id)),
        site:   sanitize_tag_value(names.site(entry.site_id)),
        date:   dates::date_to_pgn(entry.date),
        round:  sanitize_tag_value(names.round(entry.round_id)),
        white:  sanitize_tag_value(names.player(entry.white_id)),
        black:  sanitize_tag_value(names.player(entry.black_id)),
        result: String::new(), // overwritten by `export_pgn`, see above
    };

    let mut pgn = export_pgn(&game, Some(tags));
    splice_extra_tags(&mut pgn, entry, &game);
    Ok(pgn)
}

/// Inserts the `[SetUp]`/`[FEN]` tags (if `initial_fen` is not the
/// standard starting position) and the `ECO`/`WhiteElo`/`BlackElo` tags —
/// none handled by `export_pgn` (Seven Tag Roster only, see `pgn.rs`)
/// since callers that need them typically extract them from the PGN text
/// directly (simple tag search) — right before the blank line that
/// separates the tags from the moves.
fn splice_extra_tags(pgn: &mut String, entry: &IndexEntry, game: &GameState) {
    let mut extra = String::new();

    // `initial_fen` ultimately comes from the `.sg4`/`.sg5` blob (via
    // `game_blob::decode_mainline` -> `GameState::from_fen`) — sanitized
    // like every other value sourced from file data, even though a FEN's
    // own syntax cannot legitimately contain the characters stripped
    // below.
    if !game.is_standard_start() {
        let _ = writeln!(extra, "[SetUp \"1\"]");
        let _ = writeln!(extra, "[FEN \"{}\"]", sanitize_tag_value(game.initial_fen()));
    }

    // `eco_to_string` comes from a fixed lookup table (`eco.rs`),
    // not from external data — sanitized only out of symmetric caution,
    // no real risk identified here.
    if let Some(eco_str) = eco::eco_to_string(entry.eco_code) {
        let _ = writeln!(extra, "[ECO \"{}\"]", sanitize_tag_value(&eco_str));
    }
    if entry.white_elo > 0 {
        let _ = writeln!(extra, "[WhiteElo \"{}\"]", entry.white_elo);
    }
    if entry.black_elo > 0 {
        let _ = writeln!(extra, "[BlackElo \"{}\"]", entry.black_elo);
    }
    if extra.is_empty() {
        return;
    }

    if let Some(pos) = pgn.find("\n\n") {
        pgn.insert_str(pos + 1, &extra);
    } else {
        // Should never happen: `export_pgn` always produces this
        // separator (see `pgn.rs`). Out of caution, we don't lose
        // the information rather than panicking or ignoring it.
        pgn.insert_str(0, &extra);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A neutral `IndexEntry` with no ECO/Elo (so `splice_extra_tags` only
    /// ever adds `[SetUp]`/`[FEN]`, keeping this module's assertions
    /// unambiguous) — `offset`/`length` are unused by [`build_pgn`]
    /// itself (only by its callers, which slice the games buffer before
    /// calling it), so left at `0`.
    fn sample_entry() -> IndexEntry {
        IndexEntry {
            offset: 0,
            length: 0,
            white_id: 0,
            black_id: 0,
            event_id: 0,
            site_id: 0,
            round_id: 0,
            result: 0,
            eco_code: 0,
            date: 0,
            white_elo: 0,
            black_elo: 0,
            non_standard_start: true,
            deleted: false,
        }
    }

    #[test]
    fn test_build_pgn_emits_setup_and_fen_for_non_standard_start() {
        let fen = "8/P7/8/8/8/8/8/4K2k w - - 0 1";
        let game = GameState::from_fen(fen).unwrap();
        let blob = game_blob::encode_mainline(&game).unwrap();

        let pgn = build_pgn(&sample_entry(), &NameTables::default(), &blob).unwrap();

        assert!(pgn.contains("[SetUp \"1\"]"));
        assert!(pgn.contains(&format!("[FEN \"{fen}\"]")));
    }

    #[test]
    fn test_build_pgn_omits_setup_and_fen_for_standard_start() {
        let blob = game_blob::encode_mainline(&GameState::new()).unwrap();

        let pgn = build_pgn(&sample_entry(), &NameTables::default(), &blob).unwrap();

        assert!(!pgn.contains("[SetUp"));
        assert!(!pgn.contains("[FEN"));
    }
}
