//! Decoding a `.sg4`/`.sg5` game blob (extra tags, flags, optional FEN,
//! move list, variations) into a full [`GameState`]
//! (backed by [`crate::chess::game_tree::GameTree`], not just the main
//! line — see "Decoding variations" below).
//!
//! Blob structure, EXACT ORDER (see `docs/scid-si4-specification.md` §4.2,
//! identical for si5 §4.2):
//!   (a) non-standard PGN tags         -> ignored (skipped)
//!   (b)(c) 1 flag byte + optional FEN -> non-standard starting position,
//!          decoded (see "Non-standard starting positions" below);
//!          [`error::GameDecodeError::NonStandardStart`] is never returned
//!          by this module (kept in the enum for `Display`/exhaustiveness).
//!   (d) move list (control tokens included, nested variations decoded,
//!       see "Decoding variations" below)
//!   (e) comment block, read and attached to the corresponding moves (see
//!       "Comment decoding" below).
//!
//! Encountering a VARIATION (`ENCODE_START_MARKER` token) cannot be
//! blindly "skipped": the size of an encoded move (1 or 2 bytes) depends
//! on the TYPE of the piece at the origin square, which is only known by
//! actually replaying the variation on its own board state — exactly what
//! `Game::DecodeVariation` does in the SCID reference implementation (it
//! also decodes variations rather than skipping them blindly).
//!
//! ## Piece-index renumbering on capture
//!
//! Cross-checked against the SCID reference implementation's
//! `position.cpp` (`Position::DoSimpleMove`, "handle captures" section):
//! the piece index encoded in the high nibble of each byte (`byte >> 4`)
//! is NOT stable throughout the game. On every capture, the index of the
//! captured piece is REASSIGNED to the piece that until then held the
//! highest active index of its side (removal via "swap with the last
//! element" of a variable-size list, `Count[side]` decremented on every
//! capture) — the `standard_piece_list` table in `moves.rs` is therefore
//! only a valid index -> square mapping UNTIL the first capture. Handled
//! below by exactly replicating this reassignment (`count`/renumbering in
//! [`apply_piece_lists`]).
//!
//! ## Comment decoding
//!
//! Cross-checked against `game.cpp` (`decodeComments`, `encodeComments`,
//! `countComments`, `Game::Decode`). Two points worth noting:
//!
//! 1. The format theoretically supports a "dense" mode (no
//!    `ENCODE_COMMENT` token in the stream, one string — empty or not — per
//!    move, in order, for games that are mostly commented) BUT
//!    `Game::Encode` in the reference implementation FORCES
//!    `markComments = true` unconditionally — this dense mode is
//!    therefore never produced by a real encoder. Only the "sparse" mode
//!    (one `ENCODE_COMMENT` token per commented move, a block of strings
//!    terminated by `0x00` in the same order) is implemented here.
//! 2. An `ENCODE_COMMENT` token encountered BEFORE the very first move of
//!    the game ("pregame" comment) still consumes its string from the
//!    final block (so as not to desynchronize the following comments), but
//!    [`crate::chess`] has no notion of a pre-game comment (a `GameNode`
//!    always represents a move played) — this string is therefore read
//!    then silently discarded, the same principle applied to NAGs outside
//!    1-6: not corruption, just data this decoder does not represent.
//!
//! Ironwood change: a truncated comment block rejects the game rather than
//! silently dropping annotations from the imported copy.
//!
//! ## Non-standard starting positions
//!
//! Cross-checked against `position.cpp` (`Position::AddPiece`,
//! `Position::ReadFromFEN`). Non-trivial point: the
//! `moves::standard_piece_list` table (piece index -> starting square) is
//! valid ONLY for the standard starting position. For a custom FEN, the
//! piece indices are assigned by `AddPiece` IN THE ORDER ENCOUNTERED
//! while parsing the FEN (scanning rank 8 -> 1, file a -> h, exactly
//! the natural order of writing a FEN), with one exception: the King is
//! ALWAYS forced to index 0, regardless of its position in this scan
//! order — if it is not the first piece encountered for its side, the
//! piece that already held index 0 is moved to the last free index.
//! Reproduced below by [`build_piece_lists`], which replays this scan
//! directly on the [`Position`] already parsed by [`crate::chess`]
//! (strictly equivalent to replaying the FEN text itself, without having
//! to reparse it).
//!
//! ## Decoding variations
//!
//! Cross-checked against `game.cpp` (`Game::DecodeVariation`,
//! `AddVariation`, `MoveExitVariation`, `MoveForward`) and `bytebuf.h`
//! (`ByteBuffer::nextMove`). Key point: the reference implementation
//! decodes the ENTIRE game (main line and variations nested to any depth)
//! with a SINGLE flat loop over `nextMove`, having `CurrentMove`/
//! `CurrentPos` follow the structure of the already-built tree
//! (`AddVariation` steps back one move then descends into a new child
//! branch, `MoveExitVariation` goes back up to the parent) rather than by
//! recursion — matching the choice already made by
//! [`crate::chess::game_tree::GameTree`] itself (`flatten_expand`)
//! precisely to never risk a stack overflow on a pathologically nested
//! game. [`decode_variation_tree`] reproduces this choice with an
//! explicit stack (`Vec<VarFrame>`) rather than a recursive call: a
//! variation does NOT step back by "undoing" the last move played
//! (`Position::UndoSimpleMove`, non-trivial to replicate for
//! `list`/`count` because of the swap-based removal described above — a
//! swap is not trivially invertible without extra information), but by
//! restarting from a SNAPSHOT (cloned `pos`/`list`/`count`) taken right
//! before that move was played — strictly equivalent, without having to
//! write the inverse of the renumbering on capture.
//!
//! Important: moves decoded within a variation NEVER go through
//! `GameState::play` (which would make that variation the active
//! line/`path` — see [`crate::chess::history::History::branch_at`],
//! reserved for interactive exploration by a consumer), but go directly
//! through [`crate::chess::game_tree::GameTree::add_move`]: the main line
//! of the source SCID file must remain `children[0]`/the main line of
//! the imported tree, exactly as it was in the original file.
//! All the `list`/`count` update logic (renumbering on
//! capture, rook-index lookup on castling) is SHARED with the main
//! line via [`resolve_move`]/[`apply_piece_lists`]: a single
//! implementation, never two copies at risk of diverging.

use crate::bytes::{BeReader, Eof};
use crate::error::{GameDecodeError, GameEncodeError};
use crate::moves::{self, DecodedMove};
use crate::chess::game::GameState;
use crate::chess::game_tree::{GameTree, Nag};
use crate::chess::history::MoveRecord;
use crate::chess::movegen::generate_legal_moves;
use crate::chess::notation::move_to_san;
use crate::chess::rules::make_move;
use crate::chess::types::chess_move::{Move, MoveKind};
use crate::chess::types::piece::{Color, PieceKind};
use crate::chess::types::position::Position;
use crate::chess::types::square::Square;

const ENCODE_NAG: u8 = 11;
const ENCODE_COMMENT: u8 = 12;
const ENCODE_START_MARKER: u8 = 13;
const ENCODE_END_MARKER: u8 = 14;
const ENCODE_END_GAME: u8 = 15;

/// Maximum nesting depth accepted for variations (`ENCODE_START_MARKER`
/// without a matching `ENCODE_END_MARKER` yet) — robustness audit
/// (12/07/2026, finding 2): each nesting level pushes a [`VarFrame`]
/// (a cloned [`Position`] plus the `list`/`count` piece tables and
/// `pre_move`, itself another cloned `Position` — a few hundred bytes)
/// onto [`decode_variation_tree`]'s explicit stack, for as little as 2
/// bytes of file (one legal move + one `ENCODE_START_MARKER`). Left
/// unbounded, a crafted or corrupted `.sg4`/`.sg5` blob consisting
/// mostly of nested variations could amplify its own size roughly
/// ×200 into heap allocations, a memory-exhaustion risk out of
/// proportion with the file's actual size. No legitimate, humanly
/// authored SCID game comes anywhere close to this depth of nesting
/// (a handful of levels is already an unusually heavily annotated
/// game) — chosen generously high to never reject real data.
const MAX_VARIATION_DEPTH: usize = 1024;

/// Decodes the main line of a `.sg4`/`.sg5` blob and returns the resulting
/// [`GameState`] (standard starting position, all moves of
/// the main line played). The game result (`GameState::result`)
/// is NOT set here (it comes from the `IndexEntry`, not the blob) — see
/// `pgn_build.rs`.
///
/// # Errors
/// See the variants of [`GameDecodeError`].
pub fn decode_mainline(blob: &[u8]) -> Result<GameState, GameDecodeError> {
    let mut r = BeReader::new(blob);

    skip_extra_tags(&mut r)?;

    let flags = r
        .read_u8()
        .map_err(|e| e.bad_move_stream("octet de flags de partie manquant"))?;

    // Bit 0 = non-standard starting
    // position (§4.4) — the FEN string that follows is read and becomes the
    // initial position, instead of `GameState::new()` (standard position). The
    // piece indices (`list`/`count`) are then derived from THIS position
    // via `build_piece_lists`, not from `moves::standard_piece_list` (see the
    // module doc for the full reasoning).
    let (mut game, mut list, mut count) = if flags & 0x01 != 0 {
        let fen = r
            .read_terminated_cstr()
            .map_err(|e| e.bad_move_stream("FEN de départ non standard manquant"))?;
        let game = GameState::from_fen(&fen)
            .map_err(|_| GameDecodeError::BadMoveStream("FEN de départ non standard invalide"))?;
        let (list, count) = build_piece_lists(game.position())?;
        (game, list, count)
    } else {
        let game = GameState::new();
        let list = [
            moves::standard_piece_list(Color::White),
            moves::standard_piece_list(Color::Black),
        ];
        // Number of active pieces per side: `list[side][0..count[side]]`
        // is the only up-to-date range, the rest becomes stale as
        // captures happen (see "Piece-index renumbering on capture" in the module doc).
        let count: [usize; 2] = [16, 16];
        (game, list, count)
    };

    // An `ENCODE_COMMENT` token encountered
    // at position `i` of this vector corresponds exactly to the `i`-th
    // string of the final comment block (§4.6, "sparse" mode — see the
    // module doc). `Some(node_id)` = move to attach the text to;
    // `None` = pregame comment (before the first move), with no move
    // to attach it to in [`crate::chess`]'s data model — its
    // string is still consumed from the block so as not to desynchronize the
    // following comments, just discarded after reading.
    let mut comment_targets: Vec<Option<usize>> = Vec::new();

    // Snapshot (position + piece
    // lists) taken right BEFORE each move of the main line — serves as
    // the starting point for a possible variation on THIS move (see the
    // module doc and [`decode_variation_tree`]). `None` as long as no move has
    // been played yet (a variation there would be a corrupted stream, detected by
    // the absence of `last_node_id`).
    let mut pre_move: Option<(Position, [[Square; 16]; 2], [usize; 2])> = None;

    loop {
        let byte = r
            .read_u8()
            .map_err(|e| e.bad_move_stream("fin de flux de coups inattendue"))?;

        match byte {
            ENCODE_NAG => {
                let code = r
                    .read_u8()
                    .map_err(|e| e.bad_move_stream("code NAG manquant"))?;
                apply_nag(&mut game, code);
            }
            ENCODE_COMMENT => {
                // Presence marker only (no inline data): the
                // actual text is read after `ENCODE_END_GAME`, see below.
                comment_targets.push(game.history().last_node_id());
            }
            ENCODE_START_MARKER => {
                let Some((snap_pos, snap_list, snap_count)) = pre_move.clone() else {
                    return Err(GameDecodeError::BadMoveStream(
                        "variante rencontrée avant tout coup de la ligne principale",
                    ));
                };
                let Some(cur_id) = game.history().last_node_id() else {
                    return Err(GameDecodeError::BadMoveStream(
                        "variante rencontrée avant tout coup de la ligne principale",
                    ));
                };
                let parent = game.history().tree().node(cur_id).and_then(|n| n.parent);
                decode_variation_tree(
                    game.history_mut().tree_mut(),
                    snap_pos,
                    snap_list,
                    snap_count,
                    parent,
                    &mut r,
                    &mut comment_targets,
                )?;
            }
            ENCODE_END_MARKER => {
                return Err(GameDecodeError::BadMoveStream(
                    "marqueur de fin de variante en dehors d'une variante",
                ));
            }
            ENCODE_END_GAME => break,
            _ => {
                pre_move = Some((game.position().clone(), list, count));
                apply_one_move(&mut game, &mut list, &mut count, byte, &mut r)?;
            }
        }
    }

    apply_comments(&mut game, &mut r, comment_targets)?;

    Ok(game)
}

/// Builds `list`/`count` (the same
/// structures as for the standard position, see "Piece-index renumbering
/// on capture" in the module doc) for a NON-standard starting position, by replaying on
/// `position` the index-assignment algorithm of `Position::AddPiece`
/// (`position.cpp`): scanning rank 8 -> 1, file a -> h (the natural
/// order for parsing a FEN), King ALWAYS forced to index 0 (the piece that
/// already held this index, if there was one, is moved to the last
/// free index).
///
/// # Errors
/// [`GameDecodeError::BadMoveStream`] if a side has more than 16 pieces (a
/// hard bound from `[Square; 16]`, identical to the `Count[c] > 15` rejection
/// of the reference implementation) — protects against an out-of-bounds
/// access panic on a malformed FEN rather than trusting external data.
fn build_piece_lists(position: &Position) -> Result<([[Square; 16]; 2], [usize; 2]), GameDecodeError> {
    let mut list = [[Square::new(0, 0); 16]; 2];
    let mut count: [usize; 2] = [0, 0];

    for rank in (0..8u8).rev() {
        for file in 0..8u8 {
            let sq = Square::new(file, rank);
            let Some(piece) = position.piece_at(sq) else { continue };
            let side_idx = usize::from(piece.color == Color::Black);

            if count[side_idx] >= 16 {
                return Err(GameDecodeError::BadMoveStream(
                    "plus de 16 pièces d'un camp dans le FEN de départ",
                ));
            }

            if piece.kind == PieceKind::King && count[side_idx] > 0 {
                // The King must occupy index 0: the piece that was
                // already there (encountered before it in the scan, therefore
                // placed by the `else` branch below in a previous
                // iteration) is moved to the last free index before the King
                // takes its place.
                let displaced = list[side_idx][0];
                list[side_idx][count[side_idx]] = displaced;
                list[side_idx][0] = sq;
            } else if piece.kind == PieceKind::King {
                // First piece encountered for this side: index 0 is
                // still free, the King takes it directly.
                list[side_idx][0] = sq;
            } else {
                // Non-King piece: added normally at the next free
                // index (will be moved later if the King arrives afterward and
                // this piece occupies index 0).
                list[side_idx][count[side_idx]] = sq;
            }
            count[side_idx] += 1;
        }
    }

    Ok((list, count))
}

/// Reads the comment block (§4.6) —
/// exactly `comment_targets.len()` strings terminated by `0x00`, in the
/// same order as the `ENCODE_COMMENT` tokens encountered while decoding
/// the move stream — and attaches each to the corresponding move.
///
/// Ironwood change: missing/truncated strings return an error, preserving
/// the all-or-nothing conversion policy for annotated games.
fn apply_comments(game: &mut GameState, r: &mut BeReader<'_>, comment_targets: Vec<Option<usize>>) -> Result<(), GameDecodeError> {
    for target in comment_targets {
        let text = r.read_terminated_cstr().map_err(|e| e.bad_move_stream("truncated Scid comment block"))?;
        let Some(node_id) = target else { continue };
        if let Some(node) = game.history_mut().tree_mut().node_mut(node_id) {
            node.comment = Some(text);
        }
    }
    Ok(())
}

/// Attaches the actual value of a NAG to the LAST move played (rather
/// than discarding it) — the NAG always immediately follows the move it
/// annotates in the stream (same convention as `ENCODE_COMMENT`, already
/// handled this way).
///
/// [`crate::chess::game_tree::Nag`] only covers the 6 traditional glyphs
/// (PGN codes 1-6, `!!`/`!`/`!?`/`?!`/`?`/`??`) — a CLOSED type. Extending
/// it to the full NAG standard (up to `$255`, positional evaluation
/// symbols etc.) is out of scope for this decoder. A code outside 1-6, or
/// encountered before any move played (edge case, never observed), is
/// therefore simply ignored: this is not corruption, just an annotation
/// this decoder does not represent.
fn apply_nag(game: &mut GameState, code: u8) {
    let Some(nag) = Nag::from_code(code) else { return };
    let Some(id) = game.history().last_node_id() else { return };
    if let Some(node) = game.history_mut().tree_mut().node_mut(id) {
        node.nag = Some(nag);
    }
}

/// Result of resolving a
/// move byte into a full legal move, BEFORE actually applying it — a step
/// common to the main line ([`apply_one_move`]) and to variations
/// ([`decode_variation_tree`]), extracted with no behavior change from
/// the old single body of `apply_one_move` (see the module doc).
struct ResolvedMove {
    color: Color,
    side_idx: usize,
    piece_index: usize,
    candidate: Move,
    captured_square: Option<Square>,
    castle_kingside: bool,
    castle_queenside: bool,
}

/// Decodes the move byte `byte` (reading a 2nd byte if needed, diagonal
/// Queen move) into a [`ResolvedMove`] legal in `pos`, without modifying
/// anything (neither `pos`, nor `list`/`count`) — the caller then decides
/// how to actually apply the move (see [`apply_one_move`] for the
/// main line, [`decode_variation_tree`] for a variation).
///
/// `ply` is only used for diagnostics (`GameDecodeError::IllegalMove`) —
/// move number in the main line, or a local approximation within the
/// variation with no global meaning (see the caller).
fn resolve_move(
    pos: &Position,
    list: &[[Square; 16]; 2],
    byte: u8,
    r: &mut BeReader<'_>,
    ply: usize,
) -> Result<ResolvedMove, GameDecodeError> {
    let color = pos.side_to_move;
    let side_idx = usize::from(color == Color::Black);
    let piece_index = usize::from(byte >> 4);

    let from = list[side_idx][piece_index];
    let piece = pos
        .piece_at(from)
        .ok_or(GameDecodeError::BadMoveStream("aucune pièce sur la case déduite de l'indice"))?;
    if piece.color != color {
        return Err(GameDecodeError::BadMoveStream(
            "la pièce déduite de l'indice n'appartient pas au camp au trait",
        ));
    }

    let decoded = moves::decode_move(color, piece.kind, from, byte, r)?;

    let (real_to, promotion, castle_kingside, castle_queenside) = match decoded {
        DecodedMove::Normal { to, promotion } => (to, promotion, false, false),
        DecodedMove::CastleKingside  => (Square::new(6, from.rank()), None, true, false),
        DecodedMove::CastleQueenside => (Square::new(2, from.rank()), None, false, true),
        DecodedMove::NullMove => return Err(GameDecodeError::NullMove),
    };

    let candidate = generate_legal_moves(pos)
        .into_iter()
        .find(|m| m.from == from && m.to == real_to && m.promotion == promotion)
        .ok_or(GameDecodeError::IllegalMove { ply })?;

    // Square actually captured (if a capture), determined BEFORE playing the
    // move — the board must still reflect the pre-move state. En
    // passant capture: it is not `real_to` but the square of the captured pawn
    // (same file as the destination, same rank as the origin).
    let captured_square = if candidate.kind == MoveKind::EnPassant {
        Some(Square::new(real_to.file(), from.rank()))
    } else if pos.piece_at(real_to).is_some() {
        Some(real_to)
    } else {
        None
    };

    Ok(ResolvedMove {
        color,
        side_idx,
        piece_index,
        candidate,
        captured_square,
        castle_kingside,
        castle_queenside,
    })
}

/// Updates `list`/`count` after a [`ResolvedMove`] has actually been
/// played (square of the moving piece, Rook on castling, renumbering on
/// capture — see "Piece-index renumbering on capture" in the module doc
/// and [`crate::moves::castling_rook`]'s doc). Shared by the main line and
/// variations: a single implementation of this delicate logic.
fn apply_piece_lists(
    list: &mut [[Square; 16]; 2],
    count: &mut [usize; 2],
    resolved: &ResolvedMove,
) -> Result<(), GameDecodeError> {
    let side_idx = resolved.side_idx;
    list[side_idx][resolved.piece_index] = resolved.candidate.to;

    if resolved.castle_kingside || resolved.castle_queenside {
        // The Rook's index is
        // NOT assumed fixed (1/7) — see `moves::castling_rook`'s doc —
        // it is looked up by searching for its origin square in the
        // active range of `list`, which remains correct both in the
        // standard position and in a custom starting position.
        let (rook_from, rook_to) = moves::castling_rook(resolved.color, resolved.castle_kingside);
        let rook_idx = (0..count[side_idx])
            .find(|&i| list[side_idx][i] == rook_from)
            .ok_or(GameDecodeError::BadMoveStream(
                "tour de roque introuvable dans la liste des pièces (désynchronisation)",
            ))?;
        list[side_idx][rook_idx] = rook_to;
    }

    // Removal via "swap with the last active element",
    // exactly like `Position::DoSimpleMove` (capture section) — the index
    // of the captured piece is reassigned to the piece that until then held
    // the highest active index of the opposing side, then that side's active
    // piece count decreases by 1.
    if let Some(captured_square) = resolved.captured_square {
        let enemy_idx = 1 - side_idx;
        let captured_num = (0..count[enemy_idx])
            .find(|&i| list[enemy_idx][i] == captured_square)
            .ok_or(GameDecodeError::BadMoveStream(
                "case capturée introuvable dans la liste de pièces (désynchronisation)",
            ))?;
        count[enemy_idx] -= 1;
        list[enemy_idx][captured_num] = list[enemy_idx][count[enemy_idx]];
    }

    Ok(())
}

/// Decodes and plays a single move of the main line, updating the
/// `list` table (piece index -> current square, per side — see
/// `moves::standard_piece_list`) as well as `count` (number of active
/// pieces per side, see "Piece-index renumbering on capture" in the module doc).
fn apply_one_move(
    game: &mut GameState,
    list: &mut [[Square; 16]; 2],
    count: &mut [usize; 2],
    byte: u8,
    r: &mut BeReader<'_>,
) -> Result<(), GameDecodeError> {
    let ply = game.move_count();
    let resolved = resolve_move(game.position(), list, byte, r, ply)?;

    game.play(resolved.candidate).map_err(|_| GameDecodeError::IllegalMove { ply })?;

    apply_piece_lists(list, count, &resolved)
}

/// State of the "paused" line to
/// push when a variation starts for one of its moves, restored as-is
/// when that variation ends (`ENCODE_END_MARKER`) — see the module
/// doc for the equivalence with `AddVariation`/`MoveExitVariation` of
/// the C++ reference implementation.
struct VarFrame {
    pos: Position,
    list: [[Square; 16]; 2],
    count: [usize; 2],
    current_node_id: Option<usize>,
    pre_move: Option<(Position, [[Square; 16]; 2], [usize; 2])>,
}

/// Decodes recursively (but WITHOUT
/// Rust recursion — see the module doc) the variation subtree that
/// starts at the first `ENCODE_START_MARKER` encountered, up to its
/// matching `ENCODE_END_MARKER` (depth back to 0).
///
/// `pos`/`list`/`count` = snapshot of the position/piece lists right
/// BEFORE the varied move (see [`decode_mainline`], `pre_move` field).
/// `parent_node_id` = tree node the FIRST move of this
/// variation must attach to (the parent of the varied move, not the varied
/// move itself — a variation REPLACES that move, it is not its continuation).
///
/// Never writes to `tree` outside of `add_move`/`node_mut` (never
/// modifies `path`, which does not even exist at this level — see the module
/// doc): the "main line" node in [`crate::chess`]'s sense remains
/// exactly the one already in place (`children[0]`), the variation moves
/// are added normally afterward under `children[1..]`.
fn decode_variation_tree(
    tree: &mut GameTree,
    mut pos: Position,
    mut list: [[Square; 16]; 2],
    mut count: [usize; 2],
    parent_node_id: Option<usize>,
    r: &mut BeReader<'_>,
    comment_targets: &mut Vec<Option<usize>>,
) -> Result<(), GameDecodeError> {
    let mut current_node_id = parent_node_id;
    let mut pre_move: Option<(Position, [[Square; 16]; 2], [usize; 2])> = None;
    let mut stack: Vec<VarFrame> = Vec::new();

    loop {
        let byte = r
            .read_u8()
            .map_err(|e| e.bad_move_stream("fin de flux de coups inattendue (variante)"))?;

        match byte {
            ENCODE_NAG => {
                let code = r
                    .read_u8()
                    .map_err(|e| e.bad_move_stream("code NAG manquant (variante)"))?;
                if let (Some(nag), Some(id)) = (Nag::from_code(code), current_node_id) {
                    if let Some(node) = tree.node_mut(id) {
                        node.nag = Some(nag);
                    }
                }
            }
            ENCODE_COMMENT => {
                comment_targets.push(current_node_id);
            }
            ENCODE_START_MARKER => {
                let Some((snap_pos, snap_list, snap_count)) = pre_move.clone() else {
                    return Err(GameDecodeError::BadMoveStream(
                        "variante rencontrée avant tout coup de sa propre ligne",
                    ));
                };
                let Some(cur_id) = current_node_id else {
                    return Err(GameDecodeError::BadMoveStream(
                        "variante rencontrée avant tout coup de sa propre ligne",
                    ));
                };
                let parent = tree.node(cur_id).and_then(|n| n.parent);

                // Robustness audit (12/07/2026, finding 2): bounds the
                // memory amplification of a maliciously/pathologically
                // nested `.sg4`/`.sg5` blob — see `MAX_VARIATION_DEPTH`'s
                // doc. Checked here, right before the push that would
                // grow `stack` one level deeper, rather than after: a
                // legitimate file never comes close to this bound, so
                // this check costs nothing in the common case and never
                // truncates a variation's actual content when it does
                // trigger (the error is returned before any of this
                // deeper level's moves are read/decoded).
                if stack.len() >= MAX_VARIATION_DEPTH {
                    return Err(GameDecodeError::BadMoveStream(
                        "variantes imbriquées trop profondément (limite de robustesse)",
                    ));
                }

                // Push the state of THE CURRENT LINE (not that of the
                // new variation) so we can return to it exactly at its
                // `ENCODE_END_MARKER`.
                stack.push(VarFrame {
                    pos: pos.clone(),
                    list,
                    count,
                    current_node_id,
                    pre_move: pre_move.clone(),
                });

                pos = snap_pos;
                list = snap_list;
                count = snap_count;
                current_node_id = parent;
                pre_move = None; // no move played yet in this new branch
            }
            ENCODE_END_MARKER => {
                let Some(frame) = stack.pop() else {
                    // Depth back to 0: this is the end OF THIS variation
                    // (the one for which `decode_variation_tree` was
                    // called) — returns to the caller (`decode_mainline` or
                    // an enclosing variation level).
                    return Ok(());
                };
                pos = frame.pos;
                list = frame.list;
                count = frame.count;
                current_node_id = frame.current_node_id;
                pre_move = frame.pre_move;
            }
            ENCODE_END_GAME => {
                return Err(GameDecodeError::BadMoveStream(
                    "fin de partie rencontrée à l'intérieur d'une variante",
                ));
            }
            _ => {
                // Diagnostics only (see `resolve_move`'s doc): no
                // global notion of "ply" inside a variation, we simply
                // reuse the already-known local depth.
                let ply = current_node_id.and_then(|id| tree.ply_index(id)).map_or(0, |p| p + 1);
                let resolved = resolve_move(&pos, &list, byte, r, ply)?;

                let san = move_to_san(&pos, resolved.candidate);
                let fen_before = pos.to_fen();
                let new_pos = make_move(&pos, resolved.candidate)
                    .map_err(|_| GameDecodeError::IllegalMove { ply })?;

                // Captured BEFORE `apply_piece_lists` mutates `list`/`count`
                // in place, unlike a previous version of this function
                // which captured it afterward: `pre_move` must reflect the
                // state right BEFORE this move — position AND piece-index
                // table together — so that a later variation replacing
                // this move (see the module doc, "Decoding variations")
                // restarts from a genuinely consistent snapshot. Capturing
                // it after the mutation left `pos` (still pre-move) paired
                // with `list`/`count` (already post-move), an inconsistent
                // snapshot that corrupted any variation chained directly
                // off a move played inside another variation — e.g. a
                // repeated `1. e4 (1. e4 (1. e4 (...)))` chain, exactly
                // the shape `test_decode_accepts_variations_nested_within_the_depth_limit`
                // exercises, which is how this bug was found (bugfix
                // 12/07/2026, no test previously exercised a variation
                // nested more than one level deep off another variation's
                // own move).
                pre_move = Some((pos.clone(), list, count));

                apply_piece_lists(&mut list, &mut count, &resolved)?;

                let id = tree
                    .add_move(current_node_id, MoveRecord {
                        mv: resolved.candidate,
                        san,
                        fen_before,
                        from_book: false,
                    })
                    .ok_or(GameDecodeError::BadMoveStream(
                        "nœud parent de variante introuvable dans l'arbre (désynchronisation)",
                    ))?;

                current_node_id = Some(id);
                pos = new_pos;
            }
        }
    }
}

/// Skips section (a) of the blob: the list of non-standard PGN tags.
/// See `docs/scid-si4-specification.md` §4.3 (identical in si5, §4.3, with the
/// minor exception of codes 251-254 which became undefined rather than
/// "reserved unused" — handled here identically and safely in both
/// cases: they are in any case "common" tags with no name to read).
fn skip_extra_tags(r: &mut BeReader<'_>) -> Result<(), GameDecodeError> {
    let trunc = |e: Eof| e.bad_move_stream("tags PGN annexes corrompus");
    loop {
        let name_code = r.read_u8().map_err(trunc)?;
        if name_code == 0 {
            return Ok(());
        }
        if name_code == 255 {
            // Legacy case (SCID2 compat.): EventDate packed into 3 raw
            // bytes, with no value-length byte.
            r.skip(3).map_err(trunc)?;
            continue;
        }
        if name_code <= 240 {
            // Custom tag: name_code = length of the tag NAME to skip.
            r.skip(usize::from(name_code)).map_err(trunc)?;
        }
        // 241-254: "common" tag, encoded in this single byte (no name to read).

        let value_len = r.read_u8().map_err(trunc)?;
        r.skip(usize::from(value_len)).map_err(trunc)?;
    }
}

// ---------------------------------------------------------------------------
// Encoding: GameState -> .sg4/.sg5 blob (the write-side mirror of decode_mainline)
// ---------------------------------------------------------------------------

/// Encodes `game`'s entire move tree (main line, every nested variation,
/// NAGs, comments) into `.sg4`/`.sg5` move-stream bytes — the exact
/// inverse of [`decode_mainline`], byte-for-byte compatible with it (this
/// crate can always read back what it just wrote).
///
/// Reuses [`apply_piece_lists`]/[`ResolvedMove`] as-is, so the SCID
/// piece-index bookkeeping (see the module doc, "Piece-index renumbering
/// on capture") is computed by a SINGLE implementation shared with
/// decoding, never two copies at risk of diverging.
///
/// Walks the tree with an explicit stack rather than Rust recursion, for
/// the same reason as [`decode_variation_tree`]/
/// [`crate::chess::pgn::write_siblings`]: an arbitrarily long main line
/// (thousands of moves) must never risk a stack overflow. The traversal
/// mirrors [`crate::chess::pgn::write_siblings`]'s `Task::Group` shape,
/// with the chess position/piece-index state threaded through instead of
/// PGN tokens, and `ENCODE_START_MARKER`/`ENCODE_END_MARKER` written
/// eagerly around a variation's `Task::Group` — the same "write on entry,
/// close once fully drained" pattern as
/// [`crate::chess::game_tree::GameTree::render_continue`]'s
/// `OpenVariation`/`CloseParen`.
///
/// A non-standard starting position (custom FEN, see the module doc,
/// "Non-standard starting positions") is fully supported: `game`'s
/// `initial_fen` (see [`GameState::initial_fen`]) is written verbatim as
/// section (b)(c) of the blob when it differs from the standard starting
/// position, and [`build_piece_lists`] (shared as-is with the decode
/// side) seeds the piece-index table from it instead of
/// [`moves::standard_piece_list`].
///
/// # Errors
/// See [`GameEncodeError`]. Never fails for a tree built by legal play
/// (`import_pgn`, or the `chess::game`/`chess::history` API) starting
/// from a position with at most 16 pieces per side (any position
/// actually reachable by [`crate::chess`]'s own move generation always
/// satisfies this — [`GameEncodeError::TooManyPieces`] only exists for a
/// custom starting position pieced together directly via a `[FEN]` tag).
pub fn encode_mainline(game: &GameState) -> Result<Vec<u8>, GameEncodeError> {
    let (mut out, initial_position, init_list, init_count) = if game.is_standard_start() {
        let out = vec![0u8, 0u8]; // (a) no extra tags, (b) standard starting position, no FEN to follow
        let list = [
            moves::standard_piece_list(Color::White),
            moves::standard_piece_list(Color::Black),
        ];
        (out, Position::starting(), list, [16usize, 16])
    } else {
        // `game.initial_fen()` was already successfully parsed once, either
        // by `GameState::new()` (always the standard position, handled
        // above) or by `GameState::from_fen` (returns `Err` on an invalid
        // FEN, so a `GameState` can never be observed to hold one) —
        // `GameState` exposes no way to mutate `initial_fen` after
        // construction, so re-parsing it here can only ever succeed.
        let initial_position = Position::from_fen(game.initial_fen())
            .expect("GameState::initial_fen() est toujours un FEN déjà validé à la construction");
        let (list, count) =
            build_piece_lists(&initial_position).map_err(|_| GameEncodeError::TooManyPieces)?;

        let mut out = vec![0u8, 0x01]; // (a) no extra tags, (b) non-standard starting position
        out.extend_from_slice(game.initial_fen().as_bytes());
        out.push(0); // null terminator, mirrors `BeReader::read_terminated_cstr`

        (out, initial_position, list, count)
    };
    let mut comments: Vec<String> = Vec::new();

    let tree = game.history().tree();

    /// One pending unit of work on the explicit stack: either encode a
    /// sibling group (`group[0]` = continuation from `pos`/`list`/`count`,
    /// `group[1..]` = variations replacing it), or close a variation
    /// opened by a previously-processed `Group` task (see this function's
    /// doc).
    ///
    /// `pos` is boxed: `Position` is a few hundred bytes (a full board
    /// plus metadata), and `EndVariation` carries no data at all — without
    /// boxing, every `EndVariation` on the stack would still pay for a
    /// full `Position`-sized slot (`clippy::large_enum_variant`).
    enum Task {
        Group { group: Vec<usize>, pos: Box<Position>, list: [[Square; 16]; 2], count: [usize; 2], is_variation: bool },
        EndVariation,
    }

    let mut stack = vec![Task::Group {
        group: tree.roots().to_vec(),
        pos: Box::new(initial_position),
        list: init_list,
        count: init_count,
        is_variation: false,
    }];

    while let Some(task) = stack.pop() {
        match task {
            Task::EndVariation => out.push(ENCODE_END_MARKER),

            Task::Group { group, pos, list, count, is_variation } => {
                let Some((&mv_id, variations)) = group.split_first() else { continue };
                let Some(node) = tree.node(mv_id) else { continue };

                // Eager write on entry — matches exactly the position in
                // the stream a real START_MARKER occupies: right before
                // the variation's own first move (see the module doc,
                // "Decoding variations", and this function's doc).
                if is_variation {
                    out.push(ENCODE_START_MARKER);
                }

                let resolved = resolve_for_encode(&pos, &list, node.record.mv)?;
                let (code, extra_byte) = encode_move_byte(&pos, &resolved)?;
                #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                let raw_byte = ((resolved.piece_index as u8) << 4) | (code as u8 & 0x0F);
                out.push(raw_byte);
                if let Some(b2) = extra_byte {
                    out.push(b2);
                }

                let mut new_list = list;
                let mut new_count = count;
                apply_piece_lists(&mut new_list, &mut new_count, &resolved)
                    .map_err(|_| GameEncodeError::UnknownPieceIndex)?;
                let new_pos = make_move(&pos, node.record.mv)
                    .map_err(|_| GameEncodeError::UnencodableMove)?;

                if let Some(nag) = node.nag {
                    out.push(ENCODE_NAG);
                    out.push(nag.code());
                }
                if let Some(comment) = &node.comment {
                    out.push(ENCODE_COMMENT);
                    comments.push(comment.clone());
                }

                // Stacked in reverse order of the desired processing —
                // same principle as `write_siblings`/`flatten_expand`:
                // the continuation must resume last (after ALL variations,
                // each fully encoded), so its task goes to the bottom;
                // each variation is paired with the `EndVariation` task
                // that closes it once its own `Group` (and everything it
                // recursively pushes) has fully drained.
                if is_variation {
                    stack.push(Task::EndVariation);
                }
                stack.push(Task::Group {
                    group: node.children.clone(),
                    pos: Box::new(new_pos),
                    list: new_list,
                    count: new_count,
                    is_variation: false,
                });
                for &variation in variations.iter().rev() {
                    stack.push(Task::Group {
                        group: vec![variation],
                        pos: pos.clone(),
                        list,
                        count,
                        is_variation: true,
                    });
                }
            }
        }
    }

    out.push(ENCODE_END_GAME);

    // Comment block (§4.6, "sparse" mode — see the module doc): one
    // NUL-terminated string per `ENCODE_COMMENT` token, in the same order
    // they were written above. An embedded NUL byte in a comment (not
    // representable: it would prematurely terminate the string on
    // read-back) is stripped rather than rejected — a purely defensive,
    // extremely unlikely edge case.
    for comment in &comments {
        out.extend(comment.bytes().filter(|&b| b != 0));
        out.push(0);
    }

    Ok(out)
}

/// Builds the [`ResolvedMove`] for `mv`, played in `pos` with the current
/// piece-index table `list` — the encode-side equivalent of
/// [`resolve_move`] (which instead decodes a raw byte into a move): here
/// the move is already known, only the piece index and captured square
/// need to be looked up. Feeds [`apply_piece_lists`], shared as-is with
/// decoding.
fn resolve_for_encode(pos: &Position, list: &[[Square; 16]; 2], mv: Move) -> Result<ResolvedMove, GameEncodeError> {
    let color = pos.side_to_move;
    let side_idx = usize::from(color == Color::Black);
    let piece_index = (0..16)
        .find(|&i| list[side_idx][i] == mv.from)
        .ok_or(GameEncodeError::UnknownPieceIndex)?;

    let captured_square = if mv.kind == MoveKind::EnPassant {
        Some(Square::new(mv.to.file(), mv.from.rank()))
    } else if pos.piece_at(mv.to).is_some() {
        Some(mv.to)
    } else {
        None
    };

    let (castle_kingside, castle_queenside) = if mv.kind == MoveKind::Castle {
        (mv.to.file() > mv.from.file(), mv.to.file() < mv.from.file())
    } else {
        (false, false)
    };

    Ok(ResolvedMove {
        color,
        side_idx,
        piece_index,
        candidate: mv,
        captured_square,
        castle_kingside,
        castle_queenside,
    })
}

/// Encodes `resolved.candidate` into its move code (low nibble of the
/// stream byte) and, for a Queen's diagonal move only, the extra byte
/// that follows it — the exact inverse of [`moves::decode_move`] (see
/// that function's tables, which this one mirrors one-for-one).
fn encode_move_byte(pos: &Position, resolved: &ResolvedMove) -> Result<(i32, Option<u8>), GameEncodeError> {
    if resolved.castle_kingside {
        return Ok((10, None));
    }
    if resolved.castle_queenside {
        return Ok((9, None));
    }

    let mv = resolved.candidate;
    let from = mv.from;
    let to = mv.to;
    let from_idx = i32::from(from.index());
    let to_idx = i32::from(to.index());
    let piece = pos.piece_at(from).ok_or(GameEncodeError::UnknownPieceIndex)?;

    match piece.kind {
        PieceKind::Pawn => {
            let diff = if resolved.color == Color::White { to_idx - from_idx } else { from_idx - to_idx };
            if diff == 16 {
                return Ok((15, None));
            }
            let group = match diff {
                7 => 0,
                8 => 1,
                9 => 2,
                _ => return Err(GameEncodeError::UnencodableMove),
            };
            let row = match mv.promotion {
                None => 0,
                Some(PieceKind::Queen) => 1,
                Some(PieceKind::Rook) => 2,
                Some(PieceKind::Bishop) => 3,
                Some(PieceKind::Knight) => 4,
                Some(_) => return Err(GameEncodeError::UnencodableMove),
            };
            Ok((row * 3 + group, None))
        }
        PieceKind::Knight => {
            let code = match to_idx - from_idx {
                -17 => 1,
                -15 => 2,
                -10 => 3,
                -6 => 4,
                6 => 5,
                10 => 6,
                15 => 7,
                17 => 8,
                _ => return Err(GameEncodeError::UnencodableMove),
            };
            Ok((code, None))
        }
        PieceKind::Bishop => {
            let rank_diff = i32::from(to.rank()) - i32::from(from.rank());
            let file_diff = i32::from(to.file()) - i32::from(from.file());
            if file_diff != 0 && rank_diff == file_diff {
                Ok((i32::from(to.file()), None))
            } else if file_diff != 0 && rank_diff == -file_diff {
                Ok((8 + i32::from(to.file()), None))
            } else {
                Err(GameEncodeError::UnencodableMove)
            }
        }
        PieceKind::Rook => encode_rook_like(from, to).map(|c| (c, None)),
        PieceKind::Queen => {
            if let Ok(code) = encode_rook_like(from, to) {
                return Ok((code, None));
            }
            // Diagonal move: 2-byte form — marker code (the current file,
            // never a valid horizontal-move code since that would require
            // `to.file() == from.file()`, impossible for an actual move)
            // followed by the destination square index, offset by 64 (see
            // `moves::decode_move`'s Queen case).
            let byte2 = u8::try_from(to_idx + 64).map_err(|_| GameEncodeError::UnencodableMove)?;
            Ok((i32::from(from.file()), Some(byte2)))
        }
        PieceKind::King => {
            let code = match to_idx - from_idx {
                -9 => 1,
                -8 => 2,
                -7 => 3,
                -1 => 4,
                1 => 5,
                7 => 6,
                8 => 7,
                9 => 8,
                _ => return Err(GameEncodeError::UnencodableMove),
            };
            Ok((code, None))
        }
    }
}

/// Common Rook/Queen "rook-like" encoding (horizontal or vertical move) —
/// the inverse of `moves::rook_like`. Returns an error for anything else
/// (a genuine Rook move is always horizontal or vertical; a diagonal
/// Queen move is handled separately by the caller).
fn encode_rook_like(from: Square, to: Square) -> Result<i32, GameEncodeError> {
    if to.rank() == from.rank() && to.file() != from.file() {
        Ok(i32::from(to.file()))
    } else if to.file() == from.file() && to.rank() != from.rank() {
        Ok(8 + i32::from(to.rank()))
    } else {
        Err(GameEncodeError::UnencodableMove)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a well-formed `.sg4`-shaped blob (see [`decode_mainline`]'s
    /// doc for the exact section order) whose move stream opens
    /// `n_start_markers` nested variations, each replaying `1. e4` from
    /// the standard starting position (byte `0xCF`: high nibble `0xC` =
    /// piece-index 12, the e-pawn in [`moves::standard_piece_list`]; low
    /// nibble `0xF` = index 15 of `moves.rs`'s pawn `SQDIFF`/`PROMO`
    /// tables, the double-step). The position resets to the standard
    /// start at the entrance of every nesting level (the snapshot taken
    /// right before each `1. e4` is always the same standard start), so
    /// this single fixed 2-byte pattern is enough to nest arbitrarily
    /// deep without needing a distinct legal move per level. Each
    /// `ENCODE_START_MARKER` is matched by a trailing `ENCODE_END_MARKER`
    /// (same count, properly closing every level) followed by
    /// `ENCODE_END_GAME`, so the blob decodes cleanly as long as
    /// `n_start_markers` stays within [`MAX_VARIATION_DEPTH`].
    fn variation_chain_blob(n_start_markers: usize) -> Vec<u8> {
        let mut blob = vec![0x00, 0x00]; // (a) no extra tags, (b) standard start
        for _ in 0..n_start_markers {
            blob.push(0xCF); // 1. e4, from the standard start (reset every level)
            blob.push(ENCODE_START_MARKER);
        }
        blob.resize(blob.len() + n_start_markers, ENCODE_END_MARKER);
        blob.push(ENCODE_END_GAME);
        blob
    }

    #[test]
    fn test_decode_rejects_variations_nested_past_the_depth_limit() {
        // Robustness audit (12/07/2026, finding 2): before
        // `MAX_VARIATION_DEPTH` existed, this blob (deliberately
        // pathological, but only a couple of KB) would have pushed one
        // `VarFrame` per nesting level with no bound, amplifying its own
        // size roughly ×200 in heap allocations. Comfortably past the
        // limit here — the decoder must reject the blob with a clean
        // `BadMoveStream` error rather than allocating without bound.
        let blob = variation_chain_blob(MAX_VARIATION_DEPTH + 3);
        let result = decode_mainline(&blob);
        assert!(
            matches!(result, Err(GameDecodeError::BadMoveStream(_))),
            "résultat inattendu : {result:?}"
        );
    }

    #[test]
    fn test_decode_accepts_variations_nested_within_the_depth_limit() {
        // Non-regression companion to the test above: a nesting depth
        // comfortably UNDER the limit, in an otherwise well-formed blob,
        // must decode without error — `MAX_VARIATION_DEPTH` must never
        // reject realistic files (no real SCID game comes anywhere close
        // to 16 levels of nested variations, let alone `MAX_VARIATION_DEPTH`).
        let blob = variation_chain_blob(16);
        let result = decode_mainline(&blob);
        assert!(result.is_ok(), "résultat inattendu : {result:?}");
    }

    // ── encode_mainline: byte-level round trip of encode_move_byte ──────────
    // Focused tests of the low-level encoder against the real decoder
    // (`moves::decode_move`), independent of the tree/GameState plumbing —
    // the most direct way to check each piece type's encoding table
    // one-for-one against its decoding counterpart.

    fn sq(s: &str) -> Square {
        Square::from_algebraic(s).unwrap()
    }

    /// Encodes `mv` played by `color` (piece `kind`) in `pos`, then decodes
    /// the resulting byte(s) straight back via `moves::decode_move` — the
    /// assertion of every test below.
    fn assert_move_byte_roundtrips(pos: &Position, list: &[[Square; 16]; 2], mv: Move, kind: PieceKind) {
        let resolved = resolve_for_encode(pos, list, mv).unwrap();
        let (code, extra) = encode_move_byte(pos, &resolved).unwrap();
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let byte = ((resolved.piece_index as u8) << 4) | (code as u8 & 0x0F);

        let extra_bytes: Vec<u8> = extra.into_iter().collect();
        let mut cursor = BeReader::new(&extra_bytes);
        let decoded = moves::decode_move(resolved.color, kind, mv.from, byte, &mut cursor).unwrap();
        match decoded {
            DecodedMove::Normal { to, promotion } => {
                assert_eq!(to, mv.to, "case d'arrivée incorrecte après aller-retour");
                assert_eq!(promotion, mv.promotion, "promotion incorrecte après aller-retour");
            }
            DecodedMove::CastleKingside => assert_eq!(mv.kind, MoveKind::Castle),
            DecodedMove::CastleQueenside => assert_eq!(mv.kind, MoveKind::Castle),
            DecodedMove::NullMove => panic!("coup nul inattendu"),
        }
    }

    fn standard_lists() -> [[Square; 16]; 2] {
        [moves::standard_piece_list(Color::White), moves::standard_piece_list(Color::Black)]
    }

    #[test]
    fn test_encode_move_byte_pawn_single_and_double_step() {
        let pos = Position::starting();
        let list = standard_lists();
        assert_move_byte_roundtrips(&pos, &list, Move::normal(sq("e2"), sq("e4")), PieceKind::Pawn);
        assert_move_byte_roundtrips(&pos, &list, Move::normal(sq("e2"), sq("e3")), PieceKind::Pawn);
    }

    #[test]
    fn test_encode_move_byte_pawn_promotion_every_piece() {
        let pos = Position::from_fen("8/P7/8/8/8/8/8/4K2k w - - 0 1").unwrap();
        let (list, _count) = build_piece_lists(&pos).unwrap();
        for piece in [PieceKind::Queen, PieceKind::Rook, PieceKind::Bishop, PieceKind::Knight] {
            let mv = Move::promotion(sq("a7"), sq("a8"), piece);
            assert_move_byte_roundtrips(&pos, &list, mv, PieceKind::Pawn);
        }
    }

    #[test]
    fn test_encode_move_byte_en_passant() {
        // White just played e4-e5, Black just played d7-d5: exd6 en passant.
        let pos = Position::from_fen("rnbqkbnr/ppp1pppp/8/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3").unwrap();
        let (list, _count) = build_piece_lists(&pos).unwrap();
        let mv = Move::en_passant(sq("e5"), sq("d6"));
        assert_move_byte_roundtrips(&pos, &list, mv, PieceKind::Pawn);
    }

    #[test]
    fn test_encode_move_byte_knight_all_directions() {
        let pos = Position::starting();
        let list = standard_lists();
        assert_move_byte_roundtrips(&pos, &list, Move::normal(sq("b1"), sq("a3")), PieceKind::Knight);
        assert_move_byte_roundtrips(&pos, &list, Move::normal(sq("b1"), sq("c3")), PieceKind::Knight);
        assert_move_byte_roundtrips(&pos, &list, Move::normal(sq("g1"), sq("f3")), PieceKind::Knight);
        assert_move_byte_roundtrips(&pos, &list, Move::normal(sq("g1"), sq("h3")), PieceKind::Knight);
    }

    #[test]
    fn test_encode_move_byte_bishop_both_diagonals() {
        let pos = Position::from_fen("4k3/8/8/8/3B4/8/8/4K3 w - - 0 1").unwrap();
        let (list, _count) = build_piece_lists(&pos).unwrap();
        assert_move_byte_roundtrips(&pos, &list, Move::normal(sq("d4"), sq("a1")), PieceKind::Bishop); // "/" diagonal
        assert_move_byte_roundtrips(&pos, &list, Move::normal(sq("d4"), sq("g1")), PieceKind::Bishop); // "\" diagonal
    }

    #[test]
    fn test_encode_move_byte_rook_horizontal_and_vertical() {
        let pos = Position::from_fen("4k3/8/8/8/3R4/8/8/4K3 w - - 0 1").unwrap();
        let (list, _count) = build_piece_lists(&pos).unwrap();
        assert_move_byte_roundtrips(&pos, &list, Move::normal(sq("d4"), sq("a4")), PieceKind::Rook);
        assert_move_byte_roundtrips(&pos, &list, Move::normal(sq("d4"), sq("d8")), PieceKind::Rook);
    }

    #[test]
    fn test_encode_move_byte_queen_diagonal_two_byte_form() {
        let pos = Position::from_fen("4k3/8/8/8/3Q4/8/8/4K3 w - - 0 1").unwrap();
        let (list, _count) = build_piece_lists(&pos).unwrap();
        // Diagonal (2-byte form) and straight (1-byte, rook-like form).
        assert_move_byte_roundtrips(&pos, &list, Move::normal(sq("d4"), sq("a7")), PieceKind::Queen);
        assert_move_byte_roundtrips(&pos, &list, Move::normal(sq("d4"), sq("d1")), PieceKind::Queen);
    }

    #[test]
    fn test_encode_move_byte_king_normal_and_castling() {
        let pos = Position::starting();
        let list = standard_lists();
        // Castling requires an empty/legal setup; a direct byte-level check
        // only needs the encoder/decoder to agree on the raw `Move`, not a
        // fully legal position — `resolve_for_encode` never validates legality.
        let castle_k = Move::castle(sq("e1"), sq("g1"));
        let castle_q = Move::castle(sq("e1"), sq("c1"));
        assert_move_byte_roundtrips(&pos, &list, castle_k, PieceKind::King);
        assert_move_byte_roundtrips(&pos, &list, castle_q, PieceKind::King);
    }

    // ── encode_mainline: full tree round trip ────────────────────────────

    #[test]
    fn test_encode_then_decode_roundtrip_simple_mainline() {
        let mut g = GameState::new();
        g.play(Move::normal(sq("e2"), sq("e4"))).unwrap();
        g.play(Move::normal(sq("e7"), sq("e5"))).unwrap();
        g.play(Move::normal(sq("g1"), sq("f3"))).unwrap();
        g.play(Move::normal(sq("b8"), sq("c6"))).unwrap();

        let blob = encode_mainline(&g).unwrap();
        let decoded = decode_mainline(&blob).unwrap();

        let sans: Vec<&str> = decoded.history().san_list().collect();
        assert_eq!(sans, ["e4", "e5", "Nf3", "Nc6"]);
    }

    #[test]
    fn test_encode_then_decode_roundtrip_variation_nag_and_comment() {
        let mut g = GameState::new();
        g.play(Move::normal(sq("e2"), sq("e4"))).unwrap();
        g.play(Move::normal(sq("e7"), sq("e5"))).unwrap();
        let nf3 = g.play(Move::normal(sq("g1"), sq("f3"))).is_ok();
        assert!(nf3);
        let nf3_id = g.history().last_node_id().unwrap();
        g.history_mut().tree_mut().node_mut(nf3_id).unwrap().nag = Some(crate::chess::game_tree::Nag::Good);
        g.history_mut().tree_mut().node_mut(nf3_id).unwrap().comment = Some("Solide".into());

        // Variation: 2...Nc6 instead of 3.Nf3's sibling — add 3.Nc3 as an
        // alternative to Nf3, itself a child of e5.
        let e5_id = g.history().node_id_at(1).unwrap();
        let nc3_id = g
            .history_mut()
            .tree_mut()
            .add_move(Some(e5_id), MoveRecord {
                mv: Move::normal(sq("b1"), sq("c3")),
                san: "Nc3".into(),
                fen_before: String::new(),
                from_book: false,
            })
            .unwrap();
        g.history_mut()
            .tree_mut()
            .add_move(Some(nc3_id), MoveRecord {
                mv: Move::normal(sq("b8"), sq("c6")),
                san: "Nc6".into(),
                fen_before: String::new(),
                from_book: false,
            })
            .unwrap();

        let blob = encode_mainline(&g).unwrap();
        let decoded = decode_mainline(&blob).unwrap();

        let sans: Vec<&str> = decoded.history().san_list().collect();
        assert_eq!(sans, ["e4", "e5", "Nf3"], "la ligne active reste le main line d'origine");

        let decoded_nf3_id = decoded.history().last_node_id().unwrap();
        assert_eq!(decoded.history().tree().node(decoded_nf3_id).unwrap().nag, Some(crate::chess::game_tree::Nag::Good));
        assert_eq!(decoded.history().tree().node(decoded_nf3_id).unwrap().comment.as_deref(), Some("Solide"));

        let decoded_e5_id = decoded.history().node_id_at(1).unwrap();
        let variation_id = decoded.history().tree().node(decoded_e5_id).unwrap().children[1];
        assert_eq!(decoded.history().tree().node(variation_id).unwrap().record.san, "Nc3");
        let sub_id = decoded.history().tree().node(variation_id).unwrap().children[0];
        assert_eq!(decoded.history().tree().node(sub_id).unwrap().record.san, "Nc6");
    }

    #[test]
    fn test_encode_then_decode_roundtrip_with_capture() {
        // Exercises the capture-renumbering path of `apply_piece_lists`
        // (see the module doc, "Piece-index renumbering on capture")
        // through the full `encode_mainline`/`decode_mainline` pipeline,
        // not just the isolated byte-level tests above.
        let mut g = GameState::new();
        g.play(Move::normal(sq("e2"), sq("e4"))).unwrap();
        g.play(Move::normal(sq("d7"), sq("d5"))).unwrap();
        g.play(Move::normal(sq("e4"), sq("d5"))).unwrap(); // exd5

        let blob = encode_mainline(&g).unwrap();
        let decoded = decode_mainline(&blob).unwrap();

        let sans: Vec<&str> = decoded.history().san_list().collect();
        assert_eq!(sans, ["e4", "d5", "exd5"]);
    }

    #[test]
    fn test_encode_then_decode_roundtrip_castling() {
        let mut g = GameState::new();
        for (from, to) in [("e2", "e4"), ("e7", "e5"), ("g1", "f3"), ("b8", "c6"), ("f1", "c4"), ("f8", "c5")] {
            g.play(Move::normal(sq(from), sq(to))).unwrap();
        }
        let legal = generate_legal_moves(g.position());
        let castle = legal
            .into_iter()
            .find(|m| m.kind == MoveKind::Castle && m.to.file() > m.from.file())
            .expect("le petit roque doit être légal ici");
        g.play(castle).unwrap();

        let blob = encode_mainline(&g).unwrap();
        let decoded = decode_mainline(&blob).unwrap();

        let sans: Vec<&str> = decoded.history().san_list().collect();
        assert_eq!(sans, ["e4", "e5", "Nf3", "Nc6", "Bc4", "Bc5", "O-O"]);
    }

    #[test]
    fn test_encode_then_decode_roundtrip_non_standard_starting_position() {
        let fen = "8/P7/8/8/8/8/8/4K2k w - - 0 1";
        let mut g = GameState::from_fen(fen).unwrap();
        g.play(Move::promotion(sq("a7"), sq("a8"), PieceKind::Queen)).unwrap();

        let blob = encode_mainline(&g).unwrap();
        let decoded = decode_mainline(&blob).unwrap();

        assert_eq!(decoded.initial_fen(), fen);
        let sans: Vec<&str> = decoded.history().san_list().collect();
        assert_eq!(sans, ["a8=Q+"], "la dame promue attaque le roi noir en h1 sur la diagonale a8-h1");
    }

    #[test]
    fn test_encode_non_standard_start_writes_the_start_flag_and_fen_bytes() {
        let fen = "8/P7/8/8/8/8/8/4K2k w - - 0 1";
        let g = GameState::from_fen(fen).unwrap();
        let blob = encode_mainline(&g).unwrap();

        assert_eq!(blob[0], 0, "(a) pas de tags annexes");
        assert_eq!(blob[1], 0x01, "(b) bit START");
        assert_eq!(&blob[2..2 + fen.len()], fen.as_bytes());
        assert_eq!(blob[2 + fen.len()], 0, "FEN terminée par NUL");
    }

    #[test]
    fn test_encode_standard_start_writes_no_fen() {
        let blob = encode_mainline(&GameState::new()).unwrap();
        assert_eq!(&blob[..2], &[0u8, 0u8], "aucun FEN pour la position standard");
    }

    #[test]
    fn test_encode_rejects_starting_position_with_too_many_pieces_of_one_side() {
        // 17 white pawns plus both kings: one side over the 16-piece limit
        // `build_piece_lists`/`[Square; 16]` can represent.
        let fen = "4k3/8/8/8/PPPPPPPP/PPPPPPPP/P7/4K3 w - - 0 1";
        let g = GameState::from_fen(fen).unwrap();
        assert!(matches!(encode_mainline(&g), Err(GameEncodeError::TooManyPieces)));
    }

    #[test]
    fn test_encode_extremely_long_mainline_does_not_overflow_stack() {
        // Same principle as `chess::pgn`'s and `GameTree`'s equivalent
        // tests: an explicit-stack traversal must never risk a Rust
        // call-stack overflow, regardless of how long the main line is.
        // Real, legal moves are used here (unlike the fake records used by
        // the `chess::pgn`/`GameTree` equivalents) since `encode_mainline`
        // needs an actual `Position` to resolve each move's piece index.
        const DEPTH: usize = 20_000;
        let mut g = GameState::new();
        let mut white_out = false;
        let mut black_out = false;
        for i in 0..DEPTH {
            let mv = if i % 2 == 0 {
                let mv = if white_out { Move::normal(sq("f3"), sq("g1")) } else { Move::normal(sq("g1"), sq("f3")) };
                white_out = !white_out;
                mv
            } else {
                let mv = if black_out { Move::normal(sq("f6"), sq("g8")) } else { Move::normal(sq("g8"), sq("f6")) };
                black_out = !black_out;
                mv
            };
            g.play(mv).unwrap();
        }

        let blob = encode_mainline(&g).unwrap();
        assert!(!blob.is_empty());
    }
}
