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

#[derive(Clone, Debug)]
pub struct OpponentThreat {
    pub key: String,
    pub text: String,
    pub source: Square,
    pub destination: Square,
    pub lines: Vec<(Square, Square)>,
}

/// Legal moves after a hypothetical pass. These are candidate threats, not
/// forced wins; the actual side to move can prevent them with its reply.
pub fn opponent_threats(board: &Board) -> Option<Vec<OpponentThreat>> {
    let opponent = board.opponent_turn_for_analysis()?;
    let side = opponent.side_to_move();
    let mut threats = Vec::new();
    for mv in MoveGen::new_legal(&opponent) {
        let after = opponent.make_move_new(mv);
        let castle = opponent.castle_side(mv).is_some();
        let destination = if castle { after.king_square(side) } else { mv.get_dest() };
        let captured = (!castle).then(|| opponent.piece_on(mv.get_dest())).flatten();
        let king = after.king_square(!side);
        let checks: Vec<_> = ALL_SQUARES.into_iter().filter(|&s| after.color_on(s) == Some(side) && attacks(&after, s, king)).collect();
        let fork_targets: Vec<_> = ALL_SQUARES.into_iter().filter(|&s| after.color_on(s) == Some(!side)
            && after.piece_on(s) != Some(Piece::Pawn) && attacks(&after, destination, s)).collect();
        let fork = !castle && fork_targets.len() >= 2;
        if captured.is_none() && checks.is_empty() && !fork { continue; }
        let mut labels = Vec::new();
        let mut lines = Vec::new();
        if !checks.is_empty() {
            labels.push("Check".to_owned());
            lines.extend(checks.into_iter().map(|s| (s, king)));
        }
        if let Some(piece) = captured { labels.push(format!("Capture {} on {}", piece_name(piece), mv.get_dest())); }
        if fork {
            labels.push(format!("Fork candidate: {}", fork_targets.iter().map(ToString::to_string).collect::<Vec<_>>().join(", ")));
            for target in fork_targets { if !lines.contains(&(destination, target)) { lines.push((destination, target)); } }
        }
        threats.push(OpponentThreat { key: mv.to_string(), text: format!("{} to {}{}: {}", mv.get_source(), destination,
            mv.get_promotion().map_or(String::new(), |p| format!(" (promote to {})", piece_name(p))), labels.join("; ")),
            source: mv.get_source(), destination, lines });
    }
    Some(threats)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeKind { OpenedLine, LostDefender, NewThreat }

#[derive(Clone, Debug)]
pub struct Change {
    pub kind: ChangeKind,
    pub target: Square,
    pub related: Vec<Square>,
    pub text: String,
}

/// Track surviving pieces, including moved targets, rather than comparing the
/// unrelated occupants of a capture square. Ambiguous identities are omitted.
fn previous_square(before: &Board, after: &Board, square: Square) -> Option<Square> {
    let identity = after.piece_on(square).zip(after.color_on(square))?;
    if before.piece_on(square).zip(before.color_on(square)) == Some(identity) { return Some(square); }
    let candidates: Vec<_> = ALL_SQUARES.into_iter().filter(|&s|
        before.piece_on(s).zip(before.color_on(s)) == Some(identity)
            && after.piece_on(s).zip(after.color_on(s)) != Some(identity)).collect();
    if candidates.len() == 1 { Some(candidates[0]) } else { None }
}

pub fn changes_after_move(before: &Board, after: &Board) -> Vec<Change> {
    let mut changes = Vec::new();
    if before == after { return changes; }
    let identities: [Option<Square>; 64] = std::array::from_fn(|i| previous_square(before, after, ALL_SQUARES[i]));
    let old_square = |s: Square| identities[s.to_index()];
    for target in ALL_SQUARES {
        let Some(old_target) = old_square(target) else { continue; };
        let side = after.color_on(target).unwrap();
        let new_attackers: Vec<_> = ALL_SQUARES.into_iter().filter(|&s|
            after.color_on(s) == Some(!side) && attacks(after, s, target)
                && !old_square(s).is_some_and(|old| attacks(before, old, old_target))).collect();
        let opened: Vec<_> = new_attackers.iter().copied().filter(|&s|
            old_target == target && old_square(s) == Some(s)
                && matches!(after.piece_on(s), Some(Piece::Bishop | Piece::Rook | Piece::Queen))).collect();
        for source in opened {
            changes.push(Change { kind: ChangeKind::OpenedLine, target, related: vec![source],
                text: format!("Opened attack line: {source} to {target}. The {} now attacks the {} through a cleared line.", piece_name(after.piece_on(source).unwrap()), piece_name(after.piece_on(target).unwrap())) });
        }
        if !new_attackers.is_empty() {
            let previously_attacked = controllers(before, old_target, !side) > 0;
            changes.push(Change { kind: ChangeKind::NewThreat, target, related: new_attackers.clone(),
                text: format!("{}: {} on {target} {} {}.", if previously_attacked { "New attacker" } else { "Newly threatened" },
                    piece_name(after.piece_on(target).unwrap()), if previously_attacked { "gained attacks from" } else { "is now attacked by" },
                    new_attackers.iter().map(ToString::to_string).collect::<Vec<_>>().join(", ")) });
        }
        let lost: Vec<_> = ALL_SQUARES.into_iter().filter(|&old|
            before.color_on(old) == Some(side) && attacks(before, old, old_target)
                && !ALL_SQUARES.into_iter().any(|current| after.color_on(current) == Some(side)
                    && old_square(current) == Some(old) && attacks(after, current, target))).collect();
        if !lost.is_empty() {
            changes.push(Change { kind: ChangeKind::LostDefender, target, related: lost.clone(),
                text: format!("Lost defenders: {} on {target} lost support from {} (previous squares). {} defenders remain.",
                    piece_name(after.piece_on(target).unwrap()), lost.iter().map(ToString::to_string).collect::<Vec<_>>().join(", "), controllers(after, target, side)) });
        }
    }
    changes
}

/// Geometric control includes pinned defenders; it is not a legal-capture test.
fn attacks(board: &Board, from: Square, to: Square) -> bool {
    attacks_in(from, to, |s| board.piece_on(s).zip(board.color_on(s)))
}

fn attacks_in(from: Square, to: Square, occupant: impl Fn(Square) -> Option<(Piece, Color)>) -> bool {
    if from == to { return false; }
    let Some((piece, color)) = occupant(from) else { return false; };
    let dx = to.get_file().to_index() as i32 - from.get_file().to_index() as i32;
    let dy = to.get_rank().to_index() as i32 - from.get_rank().to_index() as i32;
    match piece {
        Piece::Pawn => return dx.abs() == 1 && dy == if color == Color::White { 1 } else { -1 },
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
        if occupant(square).is_some() { return false; }
        x += dx.signum(); y += dy.signum();
    }
    true
}

/// Geometric controllers after a hypothetical knight route, including captures
/// and newly opened rays. Includes pinned controllers; not a trade evaluation.
pub fn knight_destination_safety(board: &Board, source: Square, first: Square, target: Square) -> (Vec<Square>, Vec<Square>) {
    let Some(side) = board.color_on(source) else { return (Vec::new(), Vec::new()); };
    let occupant = |s| {
        if s == target { Some((Piece::Knight, side)) }
        else if s == source || s == first { None }
        else { board.piece_on(s).zip(board.color_on(s)) }
    };
    let mut attackers = Vec::new();
    let mut defenders = Vec::new();
    for square in ALL_SQUARES {
        if attacks_in(square, target, occupant) {
            if occupant(square).is_some_and(|(_, color)| color == side) { defenders.push(square); }
            else { attackers.push(square); }
        }
    }
    (attackers, defenders)
}

#[derive(Clone, Debug)]
pub struct PinAwareSafety {
    pub attackers: Vec<Square>,
    pub defenders: Vec<Square>,
    pub captures: Vec<(Square, Vec<Square>)>,
    pub recapturers: Vec<Square>,
    pub blocked: Vec<Square>,
}

type Occupancy = [Option<(Piece, Color)>; 64];

fn capture_if_legal(position: &Occupancy, from: Square, target: Square) -> Option<Occupancy> {
    let (piece, side) = position[from.to_index()]?;
    let (victim, owner) = position[target.to_index()]?;
    if side == owner || victim == Piece::King || !attacks_in(from, target, |s| position[s.to_index()]) { return None; }
    let mut next = *position;
    next[from.to_index()] = None;
    next[target.to_index()] = Some((if piece == Piece::Pawn && matches!(target.get_rank(), chess::Rank::First | chess::Rank::Eighth) { Piece::Queen } else { piece }, side));
    let king = ALL_SQUARES.into_iter().find(|s| next[s.to_index()] == Some((Piece::King, side)))?;
    if ALL_SQUARES.into_iter().any(|s| next[s.to_index()].is_some_and(|(_, c)| c == !side)
        && attacks_in(s, king, |sq| next[sq.to_index()])) { return None; }
    Some(next)
}

fn safety_in(position: &Occupancy, target: Square) -> Option<PinAwareSafety> {
    let (_, side) = position[target.to_index()]?;
    let mut safety = PinAwareSafety { attackers: Vec::new(), defenders: Vec::new(), captures: Vec::new(), recapturers: Vec::new(), blocked: Vec::new() };
    for square in ALL_SQUARES {
        if attacks_in(square, target, |s| position[s.to_index()]) {
            if position[square.to_index()].is_some_and(|(_, c)| c == side) { safety.defenders.push(square); }
            else { safety.attackers.push(square); }
        }
    }
    for &attacker in &safety.attackers {
        if let Some(after) = capture_if_legal(position, attacker, target) {
            let defenders: Vec<_> = ALL_SQUARES.into_iter().filter(|&s| after[s.to_index()].is_some_and(|(_, c)| c == side)
                && capture_if_legal(&after, s, target).is_some()).collect();
            for &defender in &defenders { if !safety.recapturers.contains(&defender) { safety.recapturers.push(defender); } }
            safety.captures.push((attacker, defenders));
        }
    }
    if !safety.captures.is_empty() {
        safety.blocked = safety.defenders.iter().copied().filter(|s| !safety.recapturers.contains(s)).collect();
    }
    Some(safety)
}

pub fn square_pin_safety(board: &Board, target: Square) -> Option<PinAwareSafety> {
    let position = std::array::from_fn(|i| board.piece_on(ALL_SQUARES[i]).zip(board.color_on(ALL_SQUARES[i])));
    safety_in(&position, target)
}

pub fn knight_pin_safety(board: &Board, source: Square, first: Square, target: Square) -> Option<PinAwareSafety> {
    let side = board.color_on(source)?;
    let mut position: Occupancy = std::array::from_fn(|i| board.piece_on(ALL_SQUARES[i]).zip(board.color_on(ALL_SQUARES[i])));
    position[source.to_index()] = None;
    position[first.to_index()] = None;
    position[target.to_index()] = Some((Piece::Knight, side));
    safety_in(&position, target)
}
fn controllers(board: &Board, target: Square, color: Color) -> usize {
    ALL_SQUARES.iter().filter(|&&s| board.color_on(s) == Some(color) && attacks(board, s, target)).count()
}

/// Counts other pieces geometrically controlling each occupied square.
pub fn attack_defense_counts(board: &Board) -> Vec<(Square, usize, usize)> {
    ALL_SQUARES.iter().filter_map(|&square| {
        let owner = board.color_on(square)?;
        Some((square, controllers(board, square, !owner), controllers(board, square, owner)))
    }).collect()
}
fn piece_name(piece: Piece) -> &'static str {
    match piece { Piece::Pawn => "pawn", Piece::Knight => "knight", Piece::Bishop => "bishop", Piece::Rook => "rook", Piece::Queen => "queen", Piece::King => "king" }
}

pub struct Activity {
    pub title: String,
    pub score: i32,
    pub rating: &'static str,
    pub reasons: Vec<String>,
}

/// An explainable positional heuristic, not an engine or material valuation.
pub fn positional_activity(board: &Board, source: Square) -> Option<Activity> {
    let piece = board.piece_on(source)?;
    let side = board.color_on(source)?;
    let mut reasons = Vec::new();
    let mut mobility = ALL_SQUARES.into_iter().filter(|&s| board.color_on(s) != Some(side)
        && board.piece_on(s) != Some(Piece::King) && attacks(board, source, s)
        && (piece != Piece::Pawn || board.color_on(s) == Some(!side))).count();
    if piece == Piece::Pawn {
        let rank = source.get_rank().to_index() as i32 + if side == Color::White { 1 } else { -1 };
        if (0..8).contains(&rank) {
            let ahead = Square::make_square(chess::Rank::from_index(rank as usize), source.get_file());
            if board.piece_on(ahead).is_none() { mobility += 1; }
            else { reasons.push(format!("Forward advance blocked on {ahead}")); }
        }
    }
    let capacity = match piece { Piece::Pawn => 3, Piece::Knight | Piece::King => 8, Piece::Bishop => 13, Piece::Rook => 14, Piece::Queen => 27 };
    let mut score = 20 + (mobility * 45 / capacity).min(45) as i32;
    reasons.push(format!("{mobility} geometric destinations (before king-safety checks)"));
    let central = [Square::D4, Square::E4, Square::D5, Square::E5].into_iter().filter(|&s| attacks(board, source, s)).count();
    if central > 0 { score += (central * 4) as i32; reasons.push(format!("Controls {central} central squares")); }
    let targets: Vec<_> = ALL_SQUARES.into_iter().filter(|&s| board.color_on(s) == Some(!side)
        && matches!(board.piece_on(s), Some(Piece::Knight | Piece::Bishop | Piece::Rook | Piece::Queen))
        && attacks(board, source, s)).collect();
    if !targets.is_empty() { score += (targets.len().min(2) * 6) as i32;
        reasons.push(format!("Pressures enemy pieces on {}", targets.iter().map(ToString::to_string).collect::<Vec<_>>().join(", "))); }
    let defenders = controllers(board, source, side);
    if defenders > 0 { score += 8; reasons.push(format!("Supported by {defenders} friendly pieces (including pinned defenders)")); }
    else { reasons.push("No friendly piece defends this square".into()); }
    let pawn_threat = ALL_SQUARES.into_iter().any(|s| board.color_on(s) == Some(!side)
        && board.piece_on(s) == Some(Piece::Pawn) && attacks(board, s, source));
    if pawn_threat { score -= 15; reasons.push("Attacked by an enemy pawn".into()); }
    if piece != Piece::King {
        let king = board.king_square(side);
        let pinned = ALL_SQUARES.into_iter().any(|s| board.color_on(s) == Some(!side)
            && !attacks(board, s, king) && attacks_in(s, king, |sq|
                if sq == source { None } else { board.piece_on(sq).zip(board.color_on(sq)) }));
        if pinned { score -= 25; reasons.push("Absolutely pinned: moving off the line exposes the king".into()); }
    }
    if piece == Piece::Rook {
        let own_pawn = ALL_SQUARES.into_iter().any(|s| s.get_file() == source.get_file()
            && board.piece_on(s) == Some(Piece::Pawn) && board.color_on(s) == Some(side));
        let enemy_pawn = ALL_SQUARES.into_iter().any(|s| s.get_file() == source.get_file()
            && board.piece_on(s) == Some(Piece::Pawn) && board.color_on(s) == Some(!side));
        if !own_pawn { score += if enemy_pawn { 8 } else { 14 };
            reasons.push(if enemy_pawn { "On a semi-open file" } else { "On an open file" }.into()); }
    }
    if piece == Piece::Knight && defenders > 0 && !pawn_threat {
        let rank = source.get_rank().to_index();
        if (side == Color::White && rank >= 4) || (side == Color::Black && rank <= 3) {
            score += 10; reasons.push("Supported advanced post; no current enemy pawn attack".into());
        }
    }
    if piece == Piece::King {
        let heavy = ALL_SQUARES.into_iter().any(|s| board.color_on(s) == Some(!side)
            && matches!(board.piece_on(s), Some(Piece::Queen | Piece::Rook)));
        if heavy { score = score.min(49); reasons.push("Enemy heavy pieces remain: king activity needs caution".into()); }
        else { reasons.push("No enemy rooks or queens: king activity is more useful".into()); }
    }
    let score = score.clamp(0, 100);
    Some(Activity { title: format!("{} on {source}", piece_name(piece)), score,
        rating: if score < 35 { "Restricted" } else if score < 55 { "Balanced" } else if score < 75 { "Active" } else { "Dominant" }, reasons })
}

/// Legal first moves, followed by geometric reach with all other pieces stationary.
pub fn knight_routes(board: &Board, source: Square) -> Vec<(Square, Vec<(Square, bool)>)> {
    if board.piece_on(source) != Some(Piece::Knight) { return Vec::new(); }
    if board.color_on(source) != Some(board.side_to_move()) {
        let side = board.color_on(source).unwrap();
        let jump = |from: Square, to: Square| {
            matches!((from.get_file().to_index().abs_diff(to.get_file().to_index()),
                from.get_rank().to_index().abs_diff(to.get_rank().to_index())), (1, 2) | (2, 1))
        };
        return ALL_SQUARES.into_iter().filter(|&first| jump(source, first)
            && board.color_on(first) != Some(side) && board.piece_on(first) != Some(Piece::King))
            .map(|first| (first, ALL_SQUARES.into_iter().filter(|&target| jump(first, target)
                && (target == source || board.color_on(target) != Some(side))
                && board.piece_on(target) != Some(Piece::King)).map(|target| {
                    let fork = ALL_SQUARES.into_iter().filter(|&enemy| enemy != first && enemy != target
                        && board.color_on(enemy) == Some(!side)
                        && matches!(board.piece_on(enemy), Some(Piece::King | Piece::Queen | Piece::Rook))
                        && jump(target, enemy)).count() >= 2;
                    (target, fork)
                }).collect())).collect();
    }
    let side = board.side_to_move();
    MoveGen::new_legal(board).filter(|mv| mv.get_source() == source).map(|mv| {
        let after = board.make_move_new(mv);
        let next = ALL_SQUARES.into_iter().filter(|&target| {
            after.color_on(target) != Some(side)
                && after.piece_on(target) != Some(Piece::King)
                && attacks(&after, mv.get_dest(), target)
        }).map(|target| {
            let fork = ALL_SQUARES.into_iter().filter(|&enemy| {
                if enemy == target || after.color_on(enemy) != Some(!side)
                    || !matches!(after.piece_on(enemy), Some(Piece::King | Piece::Queen | Piece::Rook)) { return false; }
                let dx = target.get_file().to_index().abs_diff(enemy.get_file().to_index());
                let dy = target.get_rank().to_index().abs_diff(enemy.get_rank().to_index());
                matches!((dx, dy), (1, 2) | (2, 1))
            }).count() >= 2;
            (target, fork)
        }).collect();
        (mv.get_dest(), next)
    }).collect()
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
    fn pin_safety_excludes_pinned_defender_but_allows_moves_along_pin() {
        let board = Board::from_str("4r2k/8/8/8/8/2b5/3NR3/4K3 w - - 0 1").unwrap();
        let safety = square_pin_safety(&board, Square::D2).unwrap();
        assert!(safety.defenders.contains(&Square::E2));
        assert!(safety.blocked.contains(&Square::E2));
        assert!(!safety.recapturers.contains(&Square::E2));
        let board = Board::from_str("4r2k/8/8/8/5b2/4P3/4R3/4K3 w - - 0 1").unwrap();
        assert!(square_pin_safety(&board, Square::E3).unwrap().recapturers.contains(&Square::E2));
    }
    #[test]
    fn pin_safety_checks_attackers_kings_and_uncovered_recapturers() {
        let board = Board::from_str("4k3/4n3/8/3P4/8/8/8/K3R3 w - - 0 1").unwrap();
        let safety = square_pin_safety(&board, Square::D5).unwrap();
        assert!(safety.attackers.contains(&Square::E7) && safety.captures.is_empty());
        let board = Board::from_str("3r3k/8/8/8/8/2b5/3N4/2K5 w - - 0 1").unwrap();
        assert!(square_pin_safety(&board, Square::D2).unwrap().blocked.contains(&Square::C1));
        let board = Board::from_str("7k/8/8/8/8/8/R1r1N3/K7 w - - 0 1").unwrap();
        let safety = square_pin_safety(&board, Square::E2).unwrap();
        assert!(!safety.defenders.contains(&Square::A2));
        assert!(safety.captures.iter().any(|(attacker, defenders)| *attacker == Square::C2 && defenders.contains(&Square::A2)));
    }
    #[test]
    fn opponent_candidates_include_checks_captures_and_forks() {
        let board = Board::from_str("7k/8/5n2/3P4/1Q3R2/8/8/K7 w - - 0 1").unwrap();
        let threats = opponent_threats(&board).unwrap();
        let fork = threats.iter().find(|t| t.key == "f6d5").unwrap();
        assert!(fork.text.contains("Capture pawn") && fork.text.contains("Fork candidate"));
        assert!(fork.lines.contains(&(Square::D5, Square::B4)) && fork.lines.contains(&(Square::D5, Square::F4)));
        let board = Board::from_str("4r2k/8/8/8/8/8/4P3/4K3 w - - 0 1").unwrap();
        let threats = opponent_threats(&board).unwrap();
        let check = threats.iter().find(|t| t.key == "e8e2").unwrap();
        assert!(check.text.contains("Check") && check.lines.contains(&(Square::E2, Square::E1)));
    }
    #[test]
    fn opponent_candidates_respect_pins_check_and_promotions() {
        let pinned = Board::from_str("4k3/4n3/8/8/8/8/8/K3R3 w - - 0 1").unwrap();
        assert!(!opponent_threats(&pinned).unwrap().iter().any(|t| t.source == Square::E7));
        let checked = Board::from_str("4r2k/8/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        assert!(opponent_threats(&checked).is_none());
        let promotion = Board::from_str("7k/8/8/8/8/8/1p6/R6K w - - 0 1").unwrap();
        assert_eq!(opponent_threats(&promotion).unwrap().iter().filter(|t| t.key.starts_with("b2a1")).count(), 4);
        assert_eq!(opponent_threats(&Board::default()).unwrap().len(), 0);
    }
    #[test]
    fn move_changes_find_opened_rays_and_new_threats() {
        let before = Board::from_str("7k/8/8/8/b7/8/2P5/3R3K w - - 0 1").unwrap();
        let after = before.make_move_new(chess::ChessMove::new(Square::C2, Square::C3, None));
        let changes = changes_after_move(&before, &after);
        assert!(changes.iter().any(|c| c.kind == ChangeKind::OpenedLine && c.target == Square::D1 && c.related == vec![Square::A4]));
        assert!(changes.iter().any(|c| c.kind == ChangeKind::NewThreat && c.target == Square::D1));
        assert!(changes_after_move(&before, &before).is_empty());
    }
    #[test]
    fn move_changes_track_lost_support_and_moved_targets() {
        let before = Board::from_str("7k/8/8/8/8/2N5/8/K2R4 w - - 0 1").unwrap();
        let after = before.make_move_new(chess::ChessMove::new(Square::C3, Square::B5, None));
        assert!(changes_after_move(&before, &after).iter().any(|c| c.kind == ChangeKind::LostDefender
            && c.target == Square::D1 && c.related.contains(&Square::C3)));
        let before = Board::from_str("7k/8/8/8/b7/8/8/3R3K w - - 0 1").unwrap();
        let after = before.make_move_new(chess::ChessMove::new(Square::D1, Square::D2, None));
        assert!(!changes_after_move(&before, &after).iter().any(|c| c.kind == ChangeKind::NewThreat && c.target == Square::D2));
        let before = Board::from_str("7k/8/8/8/1p6/2N5/8/K2R4 b - - 0 1").unwrap();
        let after = before.make_move_new(chess::ChessMove::new(Square::B4, Square::C3, None));
        assert!(changes_after_move(&before, &after).iter().any(|c| c.kind == ChangeKind::LostDefender
            && c.target == Square::D1 && c.related == vec![Square::C3]));
    }
    #[test]
    fn activity_rewards_central_knight_and_explains_pin_and_open_file() {
        let central = Board::from_str("7k/8/8/3N4/8/8/8/K7 w - - 0 1").unwrap();
        let rim = Board::from_str("7k/8/8/N7/8/8/8/K7 w - - 0 1").unwrap();
        assert!(positional_activity(&central, Square::D5).unwrap().score > positional_activity(&rim, Square::A5).unwrap().score);
        let pinned = Board::from_str("k3r3/8/8/8/8/8/4N3/4K3 w - - 0 1").unwrap();
        assert!(positional_activity(&pinned, Square::E2).unwrap().reasons.iter().any(|r| r.contains("Absolutely pinned")));
        let rook = Board::from_str("7k/8/8/8/8/8/8/K2R4 w - - 0 1").unwrap();
        assert!(positional_activity(&rook, Square::D1).unwrap().reasons.iter().any(|r| r == "On an open file"));
        assert!(positional_activity(&rook, Square::D2).is_none());
        assert!(positional_activity(&rook, Square::H8).unwrap().reasons.iter().any(|r| r.contains("heavy pieces remain")));
    }
    #[test]
    fn knight_safety_removes_captures_and_opens_rays() {
        let board = Board::from_str("5rk1/8/8/8/8/5p2/8/K5NR w - - 0 1").unwrap();
        let (attackers, defenders) = knight_destination_safety(&board, Square::G1, Square::F3, Square::H2);
        assert!(!attackers.contains(&Square::F3));
        assert!(defenders.contains(&Square::H1));
        let (attackers, _) = knight_destination_safety(&board, Square::G1, Square::F3, Square::F3);
        assert!(attackers.contains(&Square::F8));
        let opened = Board::from_str("7k/8/8/8/8/8/8/KrN5 w - - 0 1").unwrap();
        assert!(knight_destination_safety(&opened, Square::C1, Square::D3, Square::E1).0.contains(&Square::B1));
    }
    #[test]
    fn knight_reach_respects_pins_captures_and_vacated_source() {
        let start = Board::default();
        let routes = knight_routes(&start, Square::G1);
        assert_eq!(routes.len(), 2);
        let (_, next) = routes.iter().find(|(s, _)| *s == Square::F3).unwrap();
        assert!(next.iter().any(|(s, _)| *s == Square::G1));
        assert!(!next.iter().any(|(s, _)| *s == Square::D2));
        assert_eq!(knight_routes(&start, Square::B8).len(), 2);
        let pinned = Board::from_str("k3r3/8/8/8/8/8/4N3/4K3 w - - 0 1").unwrap();
        assert!(knight_routes(&pinned, Square::E2).is_empty());
        let capture = Board::from_str("7k/8/8/8/8/5p2/8/4K1N1 w - - 0 1").unwrap();
        assert!(knight_routes(&capture, Square::G1).iter().any(|(s, _)| *s == Square::F3));
    }

    #[test]
    fn knight_second_hop_marks_major_piece_forks() {
        let board = Board::from_str("7k/8/8/1q3r2/8/8/8/K5N1 w - - 0 1").unwrap();
        let routes = knight_routes(&board, Square::G1);
        let (_, next) = routes.iter().find(|(s, _)| *s == Square::F3).unwrap();
        assert!(next.contains(&(Square::D4, true)));
    }
    #[test]
    fn attack_defense_counts_include_other_controllers_and_blocked_rays() {
        let board = Board::from_str("3r3k/8/3p4/2p1p3/3P4/4PN2/8/4K3 w - - 0 1").unwrap();
        let counts = attack_defense_counts(&board);
        assert!(counts.contains(&(Square::D4, 2, 2)));
        assert!(counts.contains(&(Square::H8, 0, 1))
            && counts.contains(&(Square::E3, 0, 0)));
        assert_eq!(counts.len(), 9);
        let pinned = Board::from_str("k3r3/8/8/8/8/8/4R3/3QK3 w - - 0 1").unwrap();
        // The king and queen defend the pinned rook; the enemy rook attacks it.
        assert!(attack_defense_counts(&pinned).contains(&(Square::E2, 1, 2)));
    }

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

