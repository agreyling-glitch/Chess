//! Explain geometric patterns without claiming an engine-proven material gain.
use crate::rules::{Board, MoveGen};
use chess::{Color, Piece, Square, ALL_SQUARES};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind { Danger, Pin, Fork }
#[derive(Clone, Debug)]
pub struct Finding {
    pub kind: Kind,
    pub squares: Vec<Square>,
    pub text: String,
}

/// Geometric control includes pinned defenders; it is not a legal-capture test.
fn attacks(board: &Board, from: Square, to: Square) -> bool {
    if from == to { return false; }
    let Some(piece) = board.piece_on(from) else { return false; };
    let dx = to.get_file().to_index() as i32 - from.get_file().to_index() as i32;
    let dy = to.get_rank().to_index() as i32 - from.get_rank().to_index() as i32;
    match piece {
        Piece::Pawn => return dx.abs() == 1 && dy == if board.color_on(from) == Some(Color::White) { 1 } else { -1 },
        Piece::Knight => return (dx.abs() == 1 && dy.abs() == 2) || (dx.abs() == 2 && dy.abs() == 1),
        Piece::King => return dx.abs() <= 1 && dy.abs() <= 1,
        Piece::Bishop if dx.abs() != dy.abs() => return false,
        Piece::Rook if dx != 0 && dy != 0 => return false,
        Piece::Queen if dx != 0 && dy != 0 && dx.abs() != dy.abs() => return false,
        _ => {}
    }
    let (mut x, mut y) = (from.get_file().to_index() as i32 + dx.signum(), from.get_rank().to_index() as i32 + dy.signum());
    while (x, y) != (to.get_file().to_index() as i32, to.get_rank().to_index() as i32) {
        let square = Square::make_square(chess::Rank::from_index(y as usize), chess::File::from_index(x as usize));
        if board.piece_on(square).is_some() { return false; }
        x += dx.signum(); y += dy.signum();
    }
    true
}
fn controllers(board: &Board, target: Square, color: Color) -> usize {
    ALL_SQUARES.iter().filter(|&&s| board.color_on(s) == Some(color) && attacks(board, s, target)).count()
}
fn piece_name(piece: Piece) -> &'static str {
    match piece { Piece::Pawn => "pawn", Piece::Knight => "knight", Piece::Bishop => "bishop", Piece::Rook => "rook", Piece::Queen => "queen", Piece::King => "king" }
}

pub fn detect(board: &Board, previous: Option<&Board>) -> Vec<Finding> {
    let mut findings = Vec::new();
    if let Some(before) = previous {
        // Compare surviving pieces on their original squares, not a moved piece's old location.
        let owner = before.side_to_move();
        for target in ALL_SQUARES {
            if board.color_on(target) != Some(owner) || before.color_on(target) != Some(owner)
                || board.piece_on(target) != before.piece_on(target) { continue; }
            let attackers = controllers(board, target, !owner);
            if attackers == 0 { continue; }
            let old_attackers = controllers(before, target, !owner);
            let old_defenders = controllers(before, target, owner);
            let defenders = controllers(board, target, owner);
            if attackers > old_attackers || defenders < old_defenders {
                let reason = if attackers > old_attackers { "gained an attacker" } else { "lost a defender" };
                findings.push(Finding { kind: Kind::Danger, squares: vec![target], text: format!("Danger: the {} on {target} {reason} after the last move ({attackers} attackers, {defenders} defenders). Geometric control; not a proven loss.", piece_name(board.piece_on(target).unwrap())) });
            }
        }
    }
    for color in [Color::White, Color::Black] {
        let king = board.king_square(color);
        for (dx, dy) in [(1,0),(-1,0),(0,1),(0,-1),(1,1),(1,-1),(-1,1),(-1,-1)] {
            let (mut x, mut y) = (king.get_file().to_index() as i32 + dx, king.get_rank().to_index() as i32 + dy);
            let mut blocker = None;
            while (0..8).contains(&x) && (0..8).contains(&y) {
                let square = Square::make_square(chess::Rank::from_index(y as usize), chess::File::from_index(x as usize));
                if let Some(piece) = board.piece_on(square) {
                    if blocker.is_none() && board.color_on(square) == Some(color) { blocker = Some(square); }
                    else {
                        let slider = piece == Piece::Queen || if dx == 0 || dy == 0 { piece == Piece::Rook } else { piece == Piece::Bishop };
                        if let Some(pinned) = blocker && board.color_on(square) == Some(!color) && slider {
                            findings.push(Finding { kind: Kind::Pin, squares: vec![square, pinned, king], text: format!("Pin: the {} on {pinned} shields the king on {king} from the {} on {square}. It cannot leave this line while exposing its king.", piece_name(board.piece_on(pinned).unwrap()), piece_name(piece)) });
                        }
                        break;
                    }
                }
                x += dx; y += dy;
            }
        }
    }
    // Only legal moves for the actual side to move; all promotions are considered.
    let side = board.side_to_move();
    for mv in MoveGen::new_legal(board) {
        if board.castle_side(mv).is_some() { continue; }
        let after = board.make_move_new(mv);
        let targets: Vec<_> = ALL_SQUARES.into_iter().filter(|&s| after.color_on(s) == Some(!side)
            && after.piece_on(s) != Some(Piece::Pawn) && attacks(&after, mv.get_dest(), s)).collect();
        if targets.len() >= 2 {
            let mut squares = vec![mv.get_source(), mv.get_dest()];
            squares.extend(&targets);
            findings.push(Finding { kind: Kind::Fork, squares, text: format!("Fork candidate: {mv} attacks {}. Check the opponent's replies; this pattern does not prove a material win.", targets.iter().map(ToString::to_string).collect::<Vec<_>>().join(" and ")) });
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    #[test]
    fn finds_absolute_pin_but_not_a_blocked_ray() {
        let board = Board::from_str("k3r3/8/8/8/8/8/4R3/4K3 w - - 0 1").unwrap();
        assert!(detect(&board, None).iter().any(|f| f.kind == Kind::Pin && f.squares == vec![Square::E8, Square::E2, Square::E1]));
        let blocked = Board::from_str("k3r3/8/8/8/4p3/8/4R3/4K3 w - - 0 1").unwrap();
        assert!(!detect(&blocked, None).iter().any(|f| f.kind == Kind::Pin));
    }
    #[test]
    fn legal_knight_fork_and_illegal_pinned_knight() {
        let board = Board::from_str("8/5k1q/8/8/8/5N2/8/4K3 w - - 0 1").unwrap();
        assert!(detect(&board, None).iter().any(|f| f.kind == Kind::Fork && f.text.contains("f3g5")));
        let pinned = Board::from_str("k3r3/8/8/8/8/8/4N3/4K3 w - - 0 1").unwrap();
        assert!(!detect(&pinned, None).iter().any(|f| f.kind == Kind::Fork));
    }
    #[test]
    fn detects_lost_defender_only_for_attacked_surviving_piece() {
        let before = Board::from_str("k7/8/8/8/8/4r3/3R4/3RK3 w - - 0 1").unwrap();
        let after = before.make_move_new(chess::ChessMove::from_str("d2a2").unwrap());
        // d1 is not attacked: losing its defender alone must not label it dangerous.
        assert!(!detect(&after, Some(&before)).iter().any(|f| f.kind == Kind::Danger));
        let before = Board::from_str("k2r4/8/8/8/8/8/3R4/3RK3 w - - 0 1").unwrap();
        let after = before.make_move_new(chess::ChessMove::from_str("d2a2").unwrap());
        assert!(detect(&after, Some(&before)).iter().any(|f| f.kind == Kind::Danger && f.squares == vec![Square::D1]));
    }
}

