use crate::rules::{Board, MoveGen};
use chess::{Color, Piece, Square};
use egui::{Color32, RichText, Sense, Vec2};
use std::sync::Arc;

type Cells = [Option<(Color, Piece)>; 64];
fn cells(board: &Board) -> Cells {
    std::array::from_fn(|i| {
        let s = square(i);
        board.color_on(s).zip(board.piece_on(s))
    })
}
fn square(i: usize) -> Square {
    Square::make_square(
        chess::Rank::from_index(i / 8),
        chess::File::from_index(i % 8),
    )
}
fn offset(i: usize, dx: i32, dy: i32) -> Option<usize> {
    let x = i as i32 % 8 + dx;
    let y = i as i32 / 8 + dy;
    ((0..8).contains(&x) && (0..8).contains(&y)).then_some((y * 8 + x) as usize)
}
fn attacks(cells: &Cells, from: usize, target: usize, color: Color, piece: Piece) -> bool {
    let dx = target as i32 % 8 - from as i32 % 8;
    let dy = target as i32 / 8 - from as i32 / 8;
    if dx == 0 && dy == 0 {
        return false;
    }
    match piece {
        Piece::Pawn => dx.abs() == 1 && dy == if color == Color::White { 1 } else { -1 },
        Piece::Knight => (dx.abs() == 1 && dy.abs() == 2) || (dx.abs() == 2 && dy.abs() == 1),
        Piece::King => dx.abs() <= 1 && dy.abs() <= 1,
        _ => {
            let diagonal = dx.abs() == dy.abs();
            let straight = dx == 0 || dy == 0;
            if !match piece {
                Piece::Bishop => diagonal,
                Piece::Rook => straight,
                Piece::Queen => diagonal || straight,
                _ => false,
            } {
                return false;
            }
            let mut cursor = offset(from, dx.signum(), dy.signum());
            while let Some(i) = cursor {
                if i == target {
                    return true;
                }
                if cells[i].is_some() {
                    return false;
                }
                cursor = offset(i, dx.signum(), dy.signum());
            }
            false
        }
    }
}
/// Geometric pressure includes pinned attackers; it is not a tactical verdict.
pub(crate) fn attackers(board: &Board, target: Square, color: Color) -> Vec<Square> {
    let occupancy = cells(board);
    occupancy
        .iter()
        .enumerate()
        .filter_map(|(i, entry)| {
            entry
                .filter(|(c, p)| *c == color && attacks(&occupancy, i, target.to_index(), *c, *p))
                .map(|_| square(i))
        })
        .collect()
}

/// Ordinary destinations, evaluated for either color independent of the move turn.
/// Castling/en passant are excluded; promotion counts as one destination.
pub fn destinations(board: &Board, from: Square) -> Vec<Square> {
    let cells = cells(board);
    let source = from.to_index();
    let Some((color, piece)) = cells[source] else {
        return Vec::new();
    };
    let mut result = Vec::new();
    for target in 0..64 {
        if cells[target].is_some_and(|(c, p)| c == color || p == Piece::King) {
            continue;
        }
        let candidate = if piece == Piece::Pawn {
            let step = if color == Color::White { 1 } else { -1 };
            let start_rank = if color == Color::White { 1 } else { 6 };
            (offset(source, 0, step) == Some(target) && cells[target].is_none())
                || (source / 8 == start_rank
                    && offset(source, 0, 2 * step) == Some(target)
                    && cells[target].is_none()
                    && offset(source, 0, step).is_some_and(|i| cells[i].is_none()))
                || (cells[target].is_some() && attacks(&cells, source, target, color, piece))
        } else {
            attacks(&cells, source, target, color, piece)
        };
        if !candidate {
            continue;
        }
        let mut after = cells;
        after[source] = None;
        after[target] = Some((color, piece));
        let Some(king) = after.iter().position(|p| *p == Some((color, Piece::King))) else {
            continue;
        };
        if !after.iter().enumerate().any(|(i, entry)| {
            entry.is_some_and(|(c, p)| c != color && attacks(&after, i, king, c, p))
        }) {
            result.push(square(target));
        }
    }
    result
}

#[derive(Clone)]
struct Sample {
    square: Square,
    piece: Piece,
    moves: Vec<Square>,
}
#[derive(Clone)]
struct Track {
    name: String,
    samples: Vec<Option<Sample>>,
}
#[derive(Clone)]
struct History {
    positions: Vec<Board>,
    tracks: Vec<Track>,
}
fn piece_name(piece: Piece) -> &'static str {
    match piece {
        Piece::Pawn => "pawn",
        Piece::Knight => "knight",
        Piece::Bishop => "bishop",
        Piece::Rook => "rook",
        Piece::Queen => "queen",
        Piece::King => "king",
    }
}
fn history(positions: &[Board]) -> History {
    let Some(first) = positions.first() else {
        return History {
            positions: Vec::new(),
            tracks: Vec::new(),
        };
    };
    let mut identities: Vec<_> = cells(first)
        .iter()
        .enumerate()
        .filter_map(|(i, p)| p.map(|(c, p)| (c, p, Some(square(i)))))
        .collect();
    let mut tracks: Vec<_> = identities
        .iter()
        .map(|(c, p, s)| Track {
            name: format!(
                "{} {} {}",
                if *c == Color::White { "White" } else { "Black" },
                piece_name(*p),
                s.unwrap()
            ),
            samples: Vec::new(),
        })
        .collect();
    for (index, board) in positions.iter().enumerate() {
        if index > 0 {
            let previous = &positions[index - 1];
            let fen = board.to_string();
            if let Some(mv) = MoveGen::new_legal(previous)
                .find(|&mv| previous.make_move_new(mv).to_string() == fen)
            {
                let castle = previous.castle_side(mv);
                let mover = identities.iter().position(|(c, _, s)| {
                    *c == previous.side_to_move() && *s == Some(mv.get_source())
                });
                if let Some(mover) = mover {
                    let color = identities[mover].0;
                    if let Some(kingside) = castle {
                        let rank = mv.get_source().get_rank();
                        let rook_from = if previous.color_on(mv.get_dest()) == Some(color) {
                            mv.get_dest()
                        } else {
                            Square::make_square(
                                rank,
                                if kingside {
                                    chess::File::H
                                } else {
                                    chess::File::A
                                },
                            )
                        };
                        let rook = identities.iter().position(|(c, p, s)| {
                            *c == color && *p == Piece::Rook && *s == Some(rook_from)
                        });
                        identities[mover].2 = Some(Square::make_square(
                            rank,
                            if kingside {
                                chess::File::G
                            } else {
                                chess::File::C
                            },
                        ));
                        if let Some(rook) = rook {
                            identities[rook].2 = Some(Square::make_square(
                                rank,
                                if kingside {
                                    chess::File::F
                                } else {
                                    chess::File::D
                                },
                            ));
                        }
                    } else {
                        for (i, (_, _, location)) in identities.iter_mut().enumerate() {
                            if i != mover && *location == Some(mv.get_dest()) {
                                *location = None;
                            }
                        }
                        identities[mover].2 = Some(mv.get_dest());
                        if let Some(promotion) = mv.get_promotion() {
                            identities[mover].1 = promotion;
                        }
                    }
                }
            }
            // Also removes an en passant victim, which is not on the destination.
            for (c, p, s) in &mut identities {
                if s.is_some_and(|square| {
                    board.color_on(square) != Some(*c) || board.piece_on(square) != Some(*p)
                }) {
                    *s = None;
                }
            }
        }
        for (track, (_, piece, location)) in tracks.iter_mut().zip(identities.iter()) {
            track.samples.push(location.map(|square| Sample {
                square,
                piece: *piece,
                moves: destinations(board, square),
            }));
        }
    }
    History {
        positions: positions.to_vec(),
        tracks,
    }
}

pub fn request_selection(ctx: &egui::Context, square: Square, compare: bool) {
    ctx.data_mut(|store| store.insert_temp(egui::Id::new("mobility_pick"), (square, compare)));
    ctx.request_repaint();
}

pub fn panel(
    ui: &mut egui::Ui,
    positions: &[Board],
    labels: &[String],
    current: usize,
) -> Option<usize> {
    ui.scope(|ui| content(ui, positions, labels, current)).inner
}
fn content(
    ui: &mut egui::Ui,
    positions: &[Board],
    labels: &[String],
    current: usize,
) -> Option<usize> {
    let cache_id = ui.id().with("mobility_history");
    let cached = ui.data_mut(|data| data.get_temp::<Arc<History>>(cache_id));
    let data = match cached {
        Some(cache) if cache.positions == positions => cache,
        _ => {
            let cache = Arc::new(history(positions));
            ui.data_mut(|data| data.insert_temp(cache_id, cache.clone()));
            cache
        }
    };
    if data.tracks.is_empty() {
        ui.label("Play or import a game to explore mobility.");
        return None;
    }
    ui.spacing_mut().item_spacing.y = 8.0;
    let selection_id = egui::Id::new("mobility_selection");
    let (mut selected, mut comparison) = ui.data_mut(|store| {
        store
            .get_persisted::<(usize, Option<usize>)>(selection_id)
            .unwrap_or((
                data.tracks
                    .iter()
                    .position(|track| {
                        track.samples[0]
                            .as_ref()
                            .is_some_and(|sample| sample.piece == Piece::Knight)
                    })
                    .unwrap_or(0),
                None,
            ))
    });
    selected = selected.min(data.tracks.len() - 1);
    comparison = comparison.filter(|&i| i < data.tracks.len());
    if let Some((square, compare)) = ui
        .ctx()
        .data_mut(|store| store.remove_temp::<(Square, bool)>(egui::Id::new("mobility_pick")))
    {
        if let Some(track) = data.tracks.iter().position(|track| {
            track
                .samples
                .get(current)
                .and_then(Option::as_ref)
                .is_some_and(|sample| sample.square == square)
        }) {
            if compare {
                comparison = Some(track);
            } else {
                selected = track;
            }
        }
    }
    ui.label(
        RichText::new(
            "Click a piece on either board to select it. Shift-click a piece to compare it.",
        )
        .size(14.0)
        .color(Color32::LIGHT_GRAY),
    );
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("mobility_piece")
            .width((ui.available_width() - 8.0) * 0.5)
            .selected_text(&data.tracks[selected].name)
            .show_ui(ui, |ui| {
                for (i, track) in data.tracks.iter().enumerate() {
                    ui.selectable_value(&mut selected, i, &track.name);
                }
            });
        egui::ComboBox::from_id_salt("mobility_compare")
            .width(ui.available_width())
            .selected_text(
                comparison
                    .map(|i| data.tracks[i].name.clone())
                    .unwrap_or("Compare…".into()),
            )
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut comparison, None, "No comparison");
                for (i, track) in data.tracks.iter().enumerate() {
                    if i != selected {
                        ui.selectable_value(&mut comparison, Some(i), &track.name);
                    }
                }
            });
    });
    if comparison == Some(selected) {
        comparison = None;
    }
    ui.data_mut(|data| data.insert_persisted(selection_id, (selected, comparison)));
    ui.label(RichText::new("Available destinations · respects pins and king safety · excludes castling and en passant").size(14.0).color(Color32::LIGHT_GRAY));
    let gold = Color32::from_rgb(230, 178, 65);
    let blue = Color32::from_rgb(106, 190, 231);
    for (track, color) in std::iter::once((selected, gold)).chain(comparison.map(|i| (i, blue))) {
        ui.horizontal(|ui| {
            let (marker, _) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
            ui.painter().circle_filled(marker.center(), 4.0, color);
            ui.label(RichText::new(&data.tracks[track].name).color(color));
        });
    }
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), 200.0),
        Sense::click_and_drag(),
    );
    ui.painter()
        .rect_filled(rect, 6.0, Color32::from_rgb(13, 17, 22));
    let plot = egui::Rect::from_min_max(
        rect.min + Vec2::new(30.0, 15.0),
        rect.max - Vec2::new(12.0, 28.0),
    );
    let maximum = std::iter::once(selected)
        .chain(comparison)
        .flat_map(|i| {
            data.tracks[i]
                .samples
                .iter()
                .flatten()
                .map(|s| s.moves.len())
        })
        .max()
        .unwrap_or(0)
        .max(4)
        .div_ceil(4)
        * 4;
    let last = positions.len().saturating_sub(1);
    let point = |i: usize, n: usize| {
        egui::pos2(
            plot.left() + plot.width() * i as f32 / last.max(1) as f32,
            plot.bottom() - plot.height() * n as f32 / maximum as f32,
        )
    };
    for tick in 0..=4 {
        let n = maximum * tick / 4;
        let y = point(0, n).y;
        ui.painter().line_segment(
            [egui::pos2(plot.left(), y), egui::pos2(plot.right(), y)],
            egui::Stroke::new(1.0, Color32::from_white_alpha(25)),
        );
        ui.painter().text(
            egui::pos2(plot.left() - 6.0, y),
            egui::Align2::RIGHT_CENTER,
            n.to_string(),
            egui::FontId::proportional(12.0),
            Color32::LIGHT_GRAY,
        );
    }
    for (i, color) in std::iter::once((selected, gold)).chain(comparison.map(|i| (i, blue))) {
        let mut previous = None;
        for (index, sample) in data.tracks[i].samples.iter().enumerate() {
            let Some(sample) = sample else {
                previous = None;
                continue;
            };
            let p = point(index, sample.moves.len());
            if let Some(prev) = previous {
                ui.painter()
                    .line_segment([prev, p], egui::Stroke::new(2.0, color));
            }
            ui.painter().circle_filled(p, 2.5, color);
            previous = Some(p);
        }
    }
    let x = point(current.min(last), 0).x;
    ui.painter().line_segment(
        [egui::pos2(x, plot.top()), egui::pos2(x, plot.bottom())],
        egui::Stroke::new(1.0, Color32::WHITE),
    );
    for (i, align) in [(0, egui::Align2::LEFT_TOP), (last, egui::Align2::RIGHT_TOP)] {
        ui.painter().text(
            egui::pos2(point(i, 0).x, plot.bottom() + 8.0),
            align,
            labels.get(i).map(String::as_str).unwrap_or("Position"),
            egui::FontId::proportional(12.0),
            Color32::LIGHT_GRAY,
        );
    }
    let mut jump = None;
    if let Some(pos) = response.hover_pos().or(response.interact_pointer_pos()) {
        let index =
            (((pos.x - plot.left()) / plot.width()).clamp(0.0, 1.0) * last as f32).round() as usize;
        let description = std::iter::once(selected)
            .chain(comparison)
            .map(|i| {
                format!(
                    "{}: {}",
                    data.tracks[i].name,
                    data.tracks[i].samples[index]
                        .as_ref()
                        .map(|s| format!("{} destinations", s.moves.len()))
                        .unwrap_or("captured".into())
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        response.clone().on_hover_text(format!(
            "{}\n{description}",
            labels.get(index).map(String::as_str).unwrap_or("Position")
        ));
        if response.clicked() || response.dragged() {
            jump = Some(index);
        }
    }
    ui.label(
        RichText::new(
            "Click or drag the graph to inspect a position. A captured piece’s line ends.",
        )
        .size(14.0)
        .color(Color32::LIGHT_GRAY),
    );
    let index = current.min(last);
    ui.label(
        RichText::new(labels.get(index).map(String::as_str).unwrap_or("Position"))
            .size(17.0)
            .strong(),
    );
    for i in std::iter::once(selected).chain(comparison) {
        let track = &data.tracks[i];
        ui.label(match &track.samples[index] {
            Some(s) => format!(
                "{} · {} on {} · {} destinations",
                track.name,
                piece_name(s.piece),
                s.square,
                s.moves.len()
            ),
            None => format!("{} · captured", track.name),
        });
    }
    let size = ui.available_width().min(320.0);
    let (board_rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let cell = size / 8.0;
    let board = &positions[index];
    for i in 0..64 {
        let sq = square(i);
        let tile = egui::Rect::from_min_size(
            board_rect.min + Vec2::new((i % 8) as f32 * cell, (7 - i / 8) as f32 * cell),
            Vec2::splat(cell),
        );
        ui.painter().rect_filled(
            tile,
            0.0,
            if (i % 8 + i / 8) % 2 == 0 {
                Color32::from_rgb(70, 77, 83)
            } else {
                Color32::from_rgb(115, 121, 126)
            },
        );
        for (track, color) in std::iter::once((selected, gold)).chain(comparison.map(|i| (i, blue)))
        {
            if let Some(sample) = &data.tracks[track].samples[index] {
                if sample.square == sq {
                    ui.painter().rect_stroke(
                        tile.shrink(2.0),
                        2.0,
                        egui::Stroke::new(3.0, color),
                        egui::StrokeKind::Inside,
                    );
                }
                if sample.moves.contains(&sq) {
                    ui.painter().circle_filled(
                        tile.min + Vec2::new(cell * 0.8, cell * 0.2),
                        cell * 0.12,
                        color,
                    );
                }
            }
        }
        if let (Some(piece), Some(color)) = (board.piece_on(sq), board.color_on(sq)) {
            let glyph = match piece {
                Piece::Pawn => "♟",
                Piece::Knight => "♞",
                Piece::Bishop => "♝",
                Piece::Rook => "♜",
                Piece::Queen => "♛",
                Piece::King => "♚",
            };
            ui.painter().text(
                tile.center(),
                egui::Align2::CENTER_CENTER,
                glyph,
                egui::FontId::proportional(cell * 0.8),
                if color == Color::White {
                    Color32::WHITE
                } else {
                    Color32::BLACK
                },
            );
        }
        let response = ui
            .interact(tile, ui.id().with(("mobility_square", i)), Sense::click())
            .on_hover_text(format!("{sq} · Click to select · Shift-click to compare"));
        if response.clicked() && board.piece_on(sq).is_some() {
            request_selection(ui.ctx(), sq, ui.input(|input| input.modifiers.shift));
        }
    }
    ui.label(
        RichText::new("White at bottom · Click a piece to select · Shift-click to compare")
            .size(14.0)
            .color(Color32::LIGHT_GRAY),
    );
    jump
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    #[test]
    fn starting_mobility_is_independent_of_turn() {
        let b = Board::default();
        assert_eq!(destinations(&b, Square::B1).len(), 2);
        assert_eq!(destinations(&b, Square::B8).len(), 2);
        assert_eq!(destinations(&b, Square::E2).len(), 2);
        assert!(destinations(&b, Square::A1).is_empty());
    }
    #[test]
    fn pinned_piece_cannot_expose_king() {
        let b = Board::from_str("4r2k/8/8/8/8/8/4N3/4K3 w - - 0 1").unwrap();
        assert!(destinations(&b, Square::E2).is_empty());
    }
    #[test]
    fn matches_ordinary_legal_moves_in_playouts() {
        let mut b = Board::default();
        for ply in 0..140 {
            let legal: Vec<_> = MoveGen::new_legal(&b).collect();
            for from in *b.color_combined(b.side_to_move()) {
                let mut expected: Vec<_> = legal
                    .iter()
                    .filter(|m| {
                        m.get_source() == from
                            && b.castle_side(**m).is_none()
                            && !(b.piece_on(from) == Some(Piece::Pawn)
                                && m.get_dest().get_file() != from.get_file()
                                && b.piece_on(m.get_dest()).is_none())
                    })
                    .map(|m| m.get_dest())
                    .collect();
                expected.sort_by_key(|s| s.to_index());
                expected.dedup();
                assert_eq!(destinations(&b, from), expected, "{b} {from}");
            }
            if legal.is_empty() {
                break;
            }
            b = b.make_move_new(legal[(ply * 17 + 3) % legal.len()]);
        }
    }
    #[test]
    fn en_passant_victim_is_captured_and_destination_is_excluded() {
        let mut board = Board::default();
        let mut positions = vec![board];
        for text in ["e2e4", "a7a6", "e4e5", "d7d5", "e5d6"] {
            if text == "e5d6" {
                assert!(!destinations(&board, Square::E5).contains(&Square::D6));
            }
            board = board.make_move_new(chess::ChessMove::from_str(text).unwrap());
            positions.push(board);
        }
        let h = history(&positions);
        let victim = h
            .tracks
            .iter()
            .find(|track| track.name == "Black pawn d7")
            .unwrap();
        assert!(victim.samples.last().unwrap().is_none());
        let attacker = h
            .tracks
            .iter()
            .find(|track| track.name == "White pawn e2")
            .unwrap();
        assert_eq!(
            attacker.samples.last().unwrap().as_ref().unwrap().square,
            Square::D6
        );
    }
    #[test]
    fn chess960_castling_tracks_both_pieces() {
        let board = Board::from_str("4k3/8/8/8/8/8/8/1R1K2R1 w BG - 0 1").unwrap();
        for mv in MoveGen::new_legal(&board).filter(|&mv| board.castle_side(mv).is_some()) {
            let kingside = board.castle_side(mv).unwrap();
            let h = history(&[board, board.make_move_new(mv)]);
            let king = h.tracks.iter().find(|t| t.name == "White king d1").unwrap();
            let rook = h
                .tracks
                .iter()
                .find(|t| t.samples[0].as_ref().unwrap().square == mv.get_dest())
                .unwrap();
            assert_eq!(
                king.samples[1].as_ref().unwrap().square,
                if kingside { Square::G1 } else { Square::C1 }
            );
            assert_eq!(
                rook.samples[1].as_ref().unwrap().square,
                if kingside { Square::F1 } else { Square::D1 }
            );
        }
    }
    #[test]
    fn tracking_survives_castling_and_promotion() {
        for (fen, moves, origin, final_square, piece) in [
            (
                "4k3/8/8/8/8/8/8/4K2R w K - 0 1",
                vec!["e1g1"],
                Square::H1,
                Square::F1,
                Piece::Rook,
            ),
            (
                "7k/P7/8/8/8/8/8/7K w - - 0 1",
                vec!["a7a8q"],
                Square::A7,
                Square::A8,
                Piece::Queen,
            ),
        ] {
            let mut b = Board::from_str(fen).unwrap();
            let mut positions = vec![b];
            for mv in moves {
                b = b.make_move_new(chess::ChessMove::from_str(mv).unwrap());
                positions.push(b);
            }
            let h = history(&positions);
            let t = h
                .tracks
                .iter()
                .find(|t| t.samples[0].as_ref().unwrap().square == origin)
                .unwrap();
            assert_eq!(t.samples[1].as_ref().unwrap().square, final_square);
            assert_eq!(t.samples[1].as_ref().unwrap().piece, piece);
        }
    }
}
