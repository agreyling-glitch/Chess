//! Parses a full PGN game (tags + moves) into the pieces needed to insert
//! it into a SCID database — the write-side mirror of `pgn_build.rs`
//! (which goes the other way: `IndexEntry` + names + blob -> PGN text).
//!
//! Format-agnostic (shared by si4/si5, exactly like `pgn_build.rs`):
//! produces a [`crate::chess::game::GameState`] via
//! [`crate::chess::pgn::import_pgn`] (moves, NAGs, comments and RAV
//! variations already reconstructed into a full `GameTree`) plus the
//! handful of SCID index-level fields ([`ParsedTags`]) a bare `GameTree`
//! cannot express on its own — see [`crate::entry::IndexEntry`]'s doc.
//!
//! [`crate::chess::pgn::import_pgn`]'s tokenizer already skips any
//! `[...]` tag line wherever it appears, so it can be called directly on
//! the full PGN text (tags included) with no pre-stripping needed; the
//! tag values themselves are extracted independently by [`tag_value`]
//! below, a second, deliberately simple pass over the same text.

use crate::chess::game::GameState;
use crate::chess::pgn::{import_pgn, PgnError};
use crate::dates;
use crate::eco;

/// SCID index-level metadata extracted from a PGN's tag section.
#[derive(Debug, Clone)]
pub struct ParsedTags {
    pub white: String,
    pub black: String,
    pub event: String,
    pub site: String,
    pub round: String,
    /// Raw SCID `dateT` encoding (see `crate::dates`).
    pub date: u32,
    /// Raw SCID result byte (0 = none, 1 = White, 2 = Black, 3 = Draw —
    /// see `crate::entry::IndexEntry::result`).
    pub result: u8,
    /// 0 = unknown.
    pub white_elo: u16,
    /// 0 = unknown.
    pub black_elo: u16,
    /// Raw SCID `ecoT` encoding (see `crate::eco`), parsed from the PGN's
    /// `[ECO]` tag if present and a valid `"A00"`..`"E99"` base code — `0`
    /// (no classification) otherwise, exactly like every other tag here:
    /// never an error by itself.
    pub eco_code: u16,
}

/// A fully parsed game, ready to be handed to
/// `crate::si5::database::Si5Database::add_game` (and, later, its si4
/// equivalent): the move tree plus its SCID index metadata.
#[derive(Debug, Clone)]
pub struct ParsedGame {
    pub game: GameState,
    pub tags: ParsedTags,
}

/// Parses `pgn` (tags + moves, e.g. the exact text produced by
/// `crate::si5::database::Si5Database::game_pgn`) into a [`ParsedGame`].
///
/// # Errors
/// [`PgnError`] if the move text itself is invalid (illegal move,
/// unclosed variation, ...) — see [`import_pgn`]. A missing or malformed
/// tag is never an error: it falls back to the same "unknown" defaults
/// [`crate::chess::pgn::PgnTags::default`] uses on the read side (`"?"`
/// for text fields, `0` for numeric ones).
pub fn parse_game(pgn: &str) -> Result<ParsedGame, PgnError> {
    let game = import_pgn(pgn)?;
    let tags = parse_tags(pgn);
    Ok(ParsedGame { game, tags })
}

fn parse_tags(pgn: &str) -> ParsedTags {
    let text = |key: &str| tag_value(pgn, key).unwrap_or("?").to_owned();

    let result = match tag_value(pgn, "Result") {
        Some("1-0") => 1,
        Some("0-1") => 2,
        Some("1/2-1/2") => 3,
        _ => 0,
    };
    let date = tag_value(pgn, "Date").map_or(0, parse_pgn_date);
    let white_elo = tag_value(pgn, "WhiteElo").and_then(|s| s.parse().ok()).unwrap_or(0);
    let black_elo = tag_value(pgn, "BlackElo").and_then(|s| s.parse().ok()).unwrap_or(0);
    let eco_code = tag_value(pgn, "ECO").and_then(eco::eco_from_string).unwrap_or(0);

    ParsedTags {
        white: text("White"),
        black: text("Black"),
        event: text("Event"),
        site: text("Site"),
        round: text("Round"),
        date,
        result,
        white_elo,
        black_elo,
        eco_code,
    }
}

/// Finds the value of tag `key` (e.g. `"White"`) in a PGN's `[Key "value"]`
/// header lines. Deliberately simple, line-oriented scan — the Seven Tag
/// Roster header is always a handful of short lines, no need for a real
/// tokenizer here (unlike [`import_pgn`], which must also handle the
/// move text and therefore cannot use this shortcut).
fn tag_value<'a>(pgn: &'a str, key: &str) -> Option<&'a str> {
    for line in pgn.lines() {
        let line = line.trim();
        let Some(inner) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) else { continue };
        let Some((name, rest)) = inner.split_once(' ') else { continue };
        if name != key {
            continue;
        }
        let rest = rest.trim();
        let Some(value) = rest.strip_prefix('"').and_then(|s| s.strip_suffix('"')) else { continue };
        return Some(value);
    }
    None
}

/// Parses a PGN date tag (`"YYYY.MM.DD"`, any field possibly `"??"`/`"?"`
/// for unknown) into the raw SCID `dateT` encoding. A non-numeric or
/// missing field is treated as unknown (`0`), never an error — a PGN date
/// tag is inherently allowed to be partially unknown.
fn parse_pgn_date(s: &str) -> u32 {
    let mut parts = s.split('.');
    let year = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let month = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let day = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    dates::make(year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_PGN: &str = concat!(
        "[Event \"World Championship\"]\n",
        "[Site \"Astana\"]\n",
        "[Date \"2024.07.12\"]\n",
        "[Round \"5\"]\n",
        "[White \"Carlsen, Magnus\"]\n",
        "[Black \"Nepomniachtchi, Ian\"]\n",
        "[Result \"1-0\"]\n",
        "[WhiteElo \"2830\"]\n",
        "[BlackElo \"2792\"]\n",
        "\n",
        "1. e4 e5 2. Nf3 Nc6 1-0\n",
    );

    #[test]
    fn test_parse_game_extracts_tags() {
        let parsed = parse_game(SAMPLE_PGN).unwrap();
        assert_eq!(parsed.tags.white, "Carlsen, Magnus");
        assert_eq!(parsed.tags.black, "Nepomniachtchi, Ian");
        assert_eq!(parsed.tags.event, "World Championship");
        assert_eq!(parsed.tags.site, "Astana");
        assert_eq!(parsed.tags.round, "5");
        assert_eq!(parsed.tags.result, 1);
        assert_eq!(parsed.tags.white_elo, 2830);
        assert_eq!(parsed.tags.black_elo, 2792);
        assert_eq!(dates::get_year(parsed.tags.date), 2024);
        assert_eq!(dates::get_month(parsed.tags.date), 7);
        assert_eq!(dates::get_day(parsed.tags.date), 12);
    }

    #[test]
    fn test_parse_game_extracts_moves() {
        let parsed = parse_game(SAMPLE_PGN).unwrap();
        let sans: Vec<&str> = parsed.game.history().san_list().collect();
        assert_eq!(sans, ["e4", "e5", "Nf3", "Nc6"]);
    }

    #[test]
    fn test_parse_game_missing_tags_fall_back_to_defaults() {
        let parsed = parse_game("1. e4 e5 *").unwrap();
        assert_eq!(parsed.tags.white, "?");
        assert_eq!(parsed.tags.result, 0);
        assert_eq!(parsed.tags.white_elo, 0);
        assert_eq!(parsed.tags.date, 0);
        assert_eq!(parsed.tags.eco_code, 0);
    }

    #[test]
    fn test_parse_game_extracts_eco_tag() {
        let pgn = "[ECO \"B90\"]\n\n1. e4 c5 2. Nf3 d6 3. d4 cxd4 4. Nxd4 Nf6 5. Nc3 a6 *";
        let parsed = parse_game(pgn).unwrap();
        assert_eq!(parsed.tags.eco_code, crate::eco::eco_from_string("B90").unwrap());
    }

    #[test]
    fn test_parse_game_invalid_eco_tag_falls_back_to_zero() {
        // Never an error, exactly like every other tag here (see doc).
        let pgn = "[ECO \"not an eco code\"]\n\n1. e4 *";
        let parsed = parse_game(pgn).unwrap();
        assert_eq!(parsed.tags.eco_code, 0);
    }

    #[test]
    fn test_parse_game_invalid_moves_is_error() {
        assert!(parse_game("1. e4 Qh5 2. Nc9 *").is_err());
    }

    #[test]
    fn test_parse_pgn_date_partial_unknown() {
        assert_eq!(parse_pgn_date("2024.??.??"), dates::make(2024, 0, 0));
        assert_eq!(parse_pgn_date("????.??.??"), 0);
    }
}
