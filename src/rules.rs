//! Keep the existing standard-chess representation; delegate Chess960 rules to Shakmaty.
use chess::{BoardStatus, ChessMove, Piece};
use shakmaty::{CastlingMode, Chess, EnPassantMode, Position, fen::Fen, uci::UciMove};
use std::{fmt, ops::Deref, str::FromStr};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Board {
    inner: chess::Board,
    rook_files: [u8; 2],
    chess960: bool,
    ep_square: Option<chess::Square>,
}

impl Default for Board {
    fn default() -> Self {
        Self {
            inner: chess::Board::default(),
            rook_files: [0; 2],
            chess960: false,
            ep_square: None,
        }
    }
}

impl Deref for Board {
    type Target = chess::Board;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let base = self.inner.to_string();
        if !self.chess960 {
            return f.write_str(&base);
        }
        let mut fields: Vec<_> = base.split_whitespace().map(str::to_owned).collect();
        let mut rights = String::new();
        for (color, files) in self.rook_files.iter().enumerate() {
            for file in 0..8 {
                if files & (1 << file) != 0 {
                    rights.push((if color == 0 { b'A' } else { b'a' } + file) as char);
                }
            }
        }
        fields[2] = if rights.is_empty() {
            "-".into()
        } else {
            rights
        };
        fields[3] = self.ep_square.map_or_else(|| "-".into(), |square| square.to_string());
        f.write_str(&fields.join(" "))
    }
}

impl FromStr for Board {
    type Err = String;
    fn from_str(fen: &str) -> Result<Self, Self::Err> {
        let parsed = Fen::from_ascii(fen.as_bytes()).map_err(|e| e.to_string())?;
        let explicit_960 = fen
            .split_whitespace()
            .nth(2)
            .is_some_and(|rights| rights.chars().any(|c| !"KQkq-".contains(c)));
        if !explicit_960
            && parsed
                .clone()
                .into_position::<Chess>(CastlingMode::Standard)
                .is_ok()
            && let Ok(inner) = chess::Board::from_str(fen)
        {
            return Ok(Self {
                inner,
                rook_files: [0; 2],
                chess960: false,
                ep_square: None,
            });
        }
        let position: Chess = parsed
            .into_position(CastlingMode::Chess960)
            .map_err(|e| e.to_string())?;
        Self::from_960_position(&position)
    }
}

impl Board {
    pub fn is_chess960(&self) -> bool {
        self.chess960
    }

    fn position_960(&self) -> Chess {
        Fen::from_ascii(self.to_string().as_bytes())
            .expect("validated FEN")
            .into_position(CastlingMode::Chess960)
            .expect("validated Chess960 position")
    }

    fn from_960_position(position: &Chess) -> Result<Self, String> {
        let fen = Fen::from_position(position, EnPassantMode::Legal).to_string();
        let mut fields: Vec<_> = fen.split_whitespace().collect();
        let ep_square = fields[3].parse().ok();
        fields[2] = "-";
        let inner = chess::Board::from_str(&fields.join(" ")).map_err(|e| e.to_string())?;
        let mut rook_files = [0; 2];
        for square in position.to_setup(EnPassantMode::Legal).castling_rights {
            let color = if square.rank() == shakmaty::Rank::First {
                0
            } else {
                1
            };
            rook_files[color] |= 1 << u32::from(square.file());
        }
        Ok(Self {
            inner,
            rook_files,
            chess960: true,
            ep_square,
        })
    }

    /// UCI Chess960 encodes castling as king-to-rook, including stationary kings.
    pub fn castle_side(&self, mv: ChessMove) -> Option<bool> {
        if self.piece_on(mv.get_source()) != Some(Piece::King) {
            return None;
        }
        if self.chess960 {
            (self.piece_on(mv.get_dest()) == Some(Piece::Rook)
                && self.color_on(mv.get_dest()) == Some(self.side_to_move()))
            .then(|| mv.get_dest().get_file() > mv.get_source().get_file())
        } else {
            (mv.get_source()
                .get_file()
                .to_index()
                .abs_diff(mv.get_dest().get_file().to_index())
                == 2)
                .then(|| mv.get_dest().get_file() == chess::File::G)
        }
    }

    pub fn make_move_new(&self, mv: ChessMove) -> Self {
        if !self.chess960 {
            return Self {
                inner: self.inner.make_move_new(mv),
                ..*self
            };
        }
        let mut position = self.position_960();
        let uci = UciMove::from_str(&mv.to_string()).expect("valid UCI move");
        let chess_move = uci.to_move(&position).expect("legal Chess960 move");
        position.play_unchecked(chess_move);
        Self::from_960_position(&position).expect("legal Chess960 position")
    }

    pub fn status(&self) -> BoardStatus {
        if !self.chess960 {
            return self.inner.status();
        }
        let position = self.position_960();
        if !position.legal_moves().is_empty() {
            BoardStatus::Ongoing
        } else if position.is_check() {
            BoardStatus::Checkmate
        } else {
            BoardStatus::Stalemate
        }
    }

    pub fn chess960(index: u16) -> Self {
        let mut n = usize::from(index % 960);
        let mut rank = [b' '; 8];
        rank[2 * (n % 4) + 1] = b'B';
        n /= 4;
        rank[2 * (n % 4)] = b'B';
        n /= 4;
        let empty = |rank: &[u8; 8]| (0..8).filter(|i| rank[*i] == b' ').collect::<Vec<_>>();
        rank[empty(&rank)[n % 6]] = b'Q';
        n /= 6;
        let pairs = [
            (0, 1),
            (0, 2),
            (0, 3),
            (0, 4),
            (1, 2),
            (1, 3),
            (1, 4),
            (2, 3),
            (2, 4),
            (3, 4),
        ];
        let slots = empty(&rank);
        let (a, b) = pairs[n];
        rank[slots[a]] = b'N';
        rank[slots[b]] = b'N';
        let slots = empty(&rank);
        for (slot, piece) in slots.iter().zip(b"RKR") {
            rank[*slot] = *piece;
        }
        let white = String::from_utf8(rank.to_vec()).unwrap();
        let rights: String = slots
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != 1)
            .map(|(_, file)| (b'A' + *file as u8) as char)
            .collect();
        let fen = format!(
            "{}/pppppppp/8/8/8/8/PPPPPPPP/{white} w {}{} - 0 1",
            white.to_lowercase(),
            rights,
            rights.to_lowercase()
        );
        let position = Fen::from_ascii(fen.as_bytes())
            .unwrap()
            .into_position(CastlingMode::Chess960)
            .unwrap();
        Self::from_960_position(&position).unwrap()
    }
}

pub struct MoveGen;
impl MoveGen {
    pub fn new_legal(board: &Board) -> std::vec::IntoIter<ChessMove> {
        let moves: Vec<_> = if board.chess960 {
            board
                .position_960()
                .legal_moves()
                .iter()
                .map(|mv| {
                    ChessMove::from_str(
                        &UciMove::from_move(*mv, CastlingMode::Chess960).to_string(),
                    )
                    .unwrap()
                })
                .collect()
        } else {
            chess::MoveGen::new_legal(&board.inner).collect()
        };
        moves.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chess960_playouts_keep_positions_valid() {
        let mut seed = 42u64;
        for index in 0..960 {
            let mut board = Board::chess960(index);
            for _ in 0..120 {
                let moves: Vec<_> = MoveGen::new_legal(&board).collect();
                if moves.is_empty() { break; }
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                board = board.make_move_new(moves[(seed >> 32) as usize % moves.len()]);
                let _ = board.status();
            }
        }
    }
    #[test]
    fn all_960_starts_are_unique_and_legal() {
        let starts: std::collections::HashSet<_> =
            (0..960).map(|i| Board::chess960(i).to_string()).collect();
        assert_eq!(starts.len(), 960);
        assert!(Board::chess960(518).to_string().starts_with("rnbqkbnr/"));
    }
    #[test]
    fn castling_handles_stationary_king_rook_and_swapped_squares() {
        for fen in [
            "4k3/8/8/8/8/8/8/6KR w H - 0 1",
            "4k3/8/8/8/8/8/8/4KR2 w F - 0 1",
            "4k3/8/8/8/8/8/8/5KR1 w G - 0 1",
        ] {
            let board = Board::from_str(fen).unwrap();
            let mv = MoveGen::new_legal(&board)
                .find(|mv| board.castle_side(*mv) == Some(true))
                .unwrap();
            let next = board.make_move_new(mv);
            assert_eq!(next.piece_on(chess::Square::G1), Some(Piece::King));
            assert_eq!(next.piece_on(chess::Square::F1), Some(Piece::Rook));
            assert_eq!(next.side_to_move(), chess::Color::Black);
        }
    }

    #[test]
    fn chess960_castling_respects_attacks_and_blockers() {
        for fen in [
            "k5r1/8/8/8/8/8/8/5K1R w H - 0 1",
            "4k3/8/8/8/8/8/8/4KBR1 w G - 0 1",
            "4k3/8/8/8/8/8/8/rRK5 w B - 0 1",
        ] {
            let board = Board::from_str(fen).unwrap();
            assert!(
                !MoveGen::new_legal(&board).any(|mv| board.castle_side(mv).is_some()),
                "{fen}"
            );
        }
    }

    #[test]
    fn chess960_black_and_queenside_castles_survive_fen_roundtrip() {
        for (fen, king, rook) in [
            (
                "5kr1/8/8/8/8/8/8/4K3 b g - 0 1",
                chess::Square::G8,
                chess::Square::F8,
            ),
            (
                "4k3/8/8/8/8/8/8/1RK5 w B - 0 1",
                chess::Square::C1,
                chess::Square::D1,
            ),
        ] {
            let board = Board::from_str(fen).unwrap();
            let restored = Board::from_str(&board.to_string()).unwrap();
            assert_eq!(board, restored);
            let castle = MoveGen::new_legal(&restored)
                .find(|mv| restored.castle_side(*mv).is_some())
                .unwrap();
            let next = restored.make_move_new(castle);
            assert_eq!(next.piece_on(king), Some(Piece::King));
            assert_eq!(next.piece_on(rook), Some(Piece::Rook));
            assert!(next.to_string().split_whitespace().nth(2) == Some("-"));
        }
    }
}
