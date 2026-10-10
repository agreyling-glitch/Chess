use crate::{mobility, rules::Board};
use chess::{Color, Piece, Square};
use egui::{Color32, RichText, Sense, Vec2};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Assessment {
    Sheltered,
    Weakened,
    Exposed,
    Endgame,
    Check,
}
impl Assessment {
    fn label(self) -> &'static str {
        match self {
            Self::Sheltered => "Sheltered",
            Self::Weakened => "Weakened",
            Self::Exposed => "Exposed",
            Self::Endgame => "Endgame",
            Self::Check => "In check",
        }
    }
    fn color(self) -> Color32 {
        match self {
            Self::Sheltered => Color32::from_rgb(112, 193, 133),
            Self::Weakened => Color32::from_rgb(230, 178, 65),
            Self::Exposed | Self::Check => Color32::from_rgb(232, 112, 112),
            Self::Endgame => Color32::from_rgb(145, 198, 224),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Safety {
    pub king: Square,
    pub shield: Vec<Square>,
    pub missing: Vec<usize>,
    pub open: Vec<usize>,
    pub semi_open: Vec<usize>,
    pub pressure: Vec<Square>,
    pub attackers: Vec<Square>,
    pub assessment: Assessment,
    pub endgame: bool,
}
fn sq(file: usize, rank: usize) -> Square {
    Square::make_square(chess::Rank::from_index(rank), chess::File::from_index(file))
}
fn material(board: &Board) -> u32 {
    [Piece::Knight, Piece::Bishop, Piece::Rook, Piece::Queen]
        .iter()
        .map(|&p| {
            board.pieces(p).popcnt()
                * match p {
                    Piece::Knight | Piece::Bishop => 3,
                    Piece::Rook => 5,
                    Piece::Queen => 9,
                    _ => 0,
                }
        })
        .sum()
}
pub fn assess(board: &Board, color: Color) -> Safety {
    let king = board.king_square(color);
    let file = king.get_file().to_index();
    let rank = king.get_rank().to_index();
    let endgame = material(board) <= 26;
    let mut shield = Vec::new();
    let mut missing = Vec::new();
    let mut open = Vec::new();
    let mut semi_open = Vec::new();
    for f in file.saturating_sub(1)..=(file + 1).min(7) {
        let mut nearby = false;
        let mut own = false;
        let mut enemy = false;
        for r in 0..8 {
            let square = sq(f, r);
            if board.piece_on(square) != Some(Piece::Pawn) {
                continue;
            }
            if board.color_on(square) == Some(color) {
                own = true;
                let distance = if color == Color::White {
                    r as i32 - rank as i32
                } else {
                    rank as i32 - r as i32
                };
                if (1..=2).contains(&distance) {
                    nearby = true;
                    shield.push(square);
                }
            } else {
                enemy = true;
            }
        }
        if !nearby {
            missing.push(f);
        }
        if !own && !enemy {
            open.push(f);
        } else if !own {
            semi_open.push(f);
        }
    }
    let mut pressure = Vec::new();
    let mut attackers = Vec::new();
    for f in file.saturating_sub(1)..=(file + 1).min(7) {
        for r in rank.saturating_sub(1)..=(rank + 1).min(7) {
            let target = sq(f, r);
            let sources = mobility::attackers(board, target, !color);
            if !sources.is_empty() {
                pressure.push(target);
                attackers.extend(sources);
            }
        }
    }
    attackers.sort_by_key(|s| s.to_index());
    attackers.dedup();
    let check = !mobility::attackers(board, king, !color).is_empty();
    let exposure = missing.len() + open.len() * 2 + semi_open.len() + pressure.len();
    let assessment = if check {
        Assessment::Check
    } else if endgame {
        if pressure.len() >= 3 && attackers.len() >= 2 {
            Assessment::Exposed
        } else {
            Assessment::Endgame
        }
    } else if exposure >= 5 {
        Assessment::Exposed
    } else if exposure > 0 {
        Assessment::Weakened
    } else {
        Assessment::Sheltered
    };
    Safety {
        king,
        shield,
        missing,
        open,
        semi_open,
        pressure,
        attackers,
        assessment,
        endgame,
    }
}
fn files(values: &[usize]) -> String {
    values
        .iter()
        .map(|i| ((b'a' + *i as u8) as char).to_string())
        .collect::<Vec<_>>()
        .join(", ")
}
fn squares(values: &[Square]) -> String {
    values
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}
fn reasons(value: &Safety) -> Vec<String> {
    let mut result = Vec::new();
    if value.assessment == Assessment::Check {
        result.push("The king is currently in check.".into());
    }
    if value.endgame {
        result.push("Low remaining material: an active king can be useful. Missing shield pawns and open files do not reduce this assessment.".into());
    }
    if !value.shield.is_empty() {
        result.push(format!("Nearby shield pawns: {}.", squares(&value.shield)));
    }
    if !value.missing.is_empty() {
        result.push(format!(
            "No shield pawn within two ranks ahead on {} file(s).",
            files(&value.missing)
        ));
    }
    if !value.open.is_empty() {
        result.push(format!(
            "Pawn-open files near the king: {}.",
            files(&value.open)
        ));
    }
    if !value.semi_open.is_empty() {
        result.push(format!(
            "No friendly pawn on nearby {} file(s).",
            files(&value.semi_open)
        ));
    }
    if !value.pressure.is_empty() {
        result.push(format!(
            "Enemy pressure on {} king-area squares, from {}.",
            value.pressure.len(),
            squares(&value.attackers)
        ));
    } else {
        result.push("No enemy piece currently attacks the king’s immediate neighborhood.".into());
    }
    result
}
#[derive(Clone)]
struct History {
    positions: Vec<Board>,
    values: Vec<[Safety; 2]>,
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
    if positions.is_empty() {
        ui.label("Play or import a game to inspect king safety.");
        return None;
    }
    ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
    ui.style_mut()
        .text_styles
        .insert(egui::TextStyle::Body, egui::FontId::proportional(14.0));
    let cache_id = ui.id().with("king_safety_history");
    let cached = ui.data_mut(|data| data.get_temp::<Arc<History>>(cache_id));
    let history = match cached {
        Some(h) if h.positions == positions => h,
        _ => {
            let h = Arc::new(History {
                positions: positions.to_vec(),
                values: positions
                    .iter()
                    .map(|b| [assess(b, Color::White), assess(b, Color::Black)])
                    .collect(),
            });
            ui.data_mut(|data| data.insert_temp(cache_id, h.clone()));
            h
        }
    };
    let current = current.min(positions.len() - 1);
    let values = &history.values[current];
    ui.label(RichText::new("King Safety").size(18.0).strong());
    ui.label("A structural assessment of pawn shelter, nearby files and enemy pressure. It is independent of engine evaluation.");
    let side_id = ui.id().with("king_safety_side");
    let mut side = ui
        .data_mut(|data| data.get_persisted::<usize>(side_id).unwrap_or(0))
        .min(1);
    ui.horizontal(|ui| {
        ui.selectable_value(&mut side, 0, "White king");
        ui.selectable_value(&mut side, 1, "Black king");
    });
    ui.data_mut(|data| data.insert_persisted(side_id, side));
    ui.label(
        RichText::new(
            labels
                .get(current)
                .map(String::as_str)
                .unwrap_or("Position"),
        )
        .strong(),
    );
    let width = ui.available_width();
    let gap = 18.0;
    let board_width = (width * 0.42).min(280.0);
    let details_width = (width - board_width - gap).max(100.0);
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        ui.allocate_ui_with_layout(
            Vec2::new(board_width, 0.0),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.set_width(board_width);
                diagram(ui, &positions[current], &values[side]);
            },
        );
        ui.allocate_ui_with_layout(
            Vec2::new(details_width, 0.0),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.set_width(details_width);
                for (name, value) in ["White", "Black"].iter().zip(values.iter()) {
                    egui::Frame::new()
                        .fill(Color32::from_rgb(25, 30, 36))
                        .corner_radius(6.0)
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.spacing_mut().item_spacing.y = 3.0;
                            ui.label(
                                RichText::new(format!("{name} · {}", value.king))
                                    .size(14.0)
                                    .strong(),
                            );
                            ui.colored_label(
                                value.assessment.color(),
                                RichText::new(value.assessment.label()).size(15.0).strong(),
                            );
                            for reason in reasons(value) {
                                ui.add(egui::Label::new(RichText::new(reason).size(13.0)).wrap());
                            }
                        });
                }
            },
        );
    });
    ui.label(RichText::new("Gold: king · Green: shield pawns · Red: attacked squares · Amber: nearby pawn-open files").size(13.0).color(Color32::LIGHT_GRAY));
    ui.label(RichText::new("Pressure includes pinned attackers. Open files may still be blocked by other pieces; the assessment does not establish a forced attack.").size(13.0).color(Color32::LIGHT_GRAY));
    ui.separator();
    ui.label(RichText::new("Safety evolution").size(18.0).strong());
    ui.label("Click a row to inspect a change in either king’s shelter or pressure.");
    let width = (ui.available_width() - ui.spacing().scroll.allocated_width()).max(160.0);
    let (header, _) = ui.allocate_exact_size(Vec2::new(width, 30.0), Sense::hover());
    ui.painter()
        .rect_filled(header, 4.0, Color32::from_rgb(32, 38, 45));
    for (i, name) in ["Position", "White", "Black"].iter().enumerate() {
        ui.painter().text(
            header.min + Vec2::new(width * i as f32 / 3.0 + 8.0, 15.0),
            egui::Align2::LEFT_CENTER,
            name,
            egui::FontId::proportional(14.0),
            Color32::WHITE,
        );
    }
    let mut jump = None;
    egui::ScrollArea::vertical()
        .id_salt("king_safety_timeline")
        .max_height(300.0)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let mut previous = None;
            let mut row = 0;
            for (index, values) in history.values.iter().enumerate() {
                if previous == Some(values) {
                    continue;
                }
                let (rect, response) =
                    ui.allocate_exact_size(Vec2::new(width, 30.0), Sense::click());
                ui.painter().rect_filled(
                    rect,
                    3.0,
                    if index == current {
                        Color32::from_rgb(86, 67, 35)
                    } else if response.hovered() {
                        Color32::from_rgb(43, 50, 59)
                    } else if row % 2 == 0 {
                        Color32::from_rgb(24, 29, 35)
                    } else {
                        Color32::from_rgb(18, 23, 29)
                    },
                );
                for (i, (text, color)) in [
                    (
                        labels.get(index).map(String::as_str).unwrap_or("Position"),
                        Color32::WHITE,
                    ),
                    (values[0].assessment.label(), values[0].assessment.color()),
                    (values[1].assessment.label(), values[1].assessment.color()),
                ]
                .iter()
                .enumerate()
                {
                    let cell = egui::Rect::from_min_size(
                        rect.min + Vec2::new(width * i as f32 / 3.0, 0.0),
                        Vec2::new(width / 3.0, 30.0),
                    );
                    ui.painter()
                        .with_clip_rect(cell.shrink2(Vec2::new(4.0, 0.0)))
                        .text(
                            cell.min + Vec2::new(8.0, 15.0),
                            egui::Align2::LEFT_CENTER,
                            text,
                            egui::FontId::proportional(13.0),
                            *color,
                        );
                }
                if response.clicked() {
                    jump = Some(index);
                }
                previous = Some(values);
                row += 1;
            }
        });
    jump
}
fn diagram(ui: &mut egui::Ui, board: &Board, value: &Safety) {
    let size = (ui.available_width() - 24.0).clamp(80.0, 256.0);
    let cell = size / 8.0;
    let (outer, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), size + 24.0), Sense::hover());
    let origin = outer.min + Vec2::new(((outer.width() - size) * 0.5).max(12.0), 0.0);
    for r in 0..8 {
        for f in 0..8 {
            let square = sq(f, r);
            let rect = egui::Rect::from_min_size(
                origin + Vec2::new(f as f32 * cell, (7 - r) as f32 * cell),
                Vec2::splat(cell),
            );
            ui.painter().rect_filled(
                rect,
                0.0,
                if (f + r) % 2 == 0 {
                    Color32::from_rgb(70, 77, 83)
                } else {
                    Color32::from_rgb(115, 121, 126)
                },
            );
            if value.open.contains(&f) {
                ui.painter().rect_filled(
                    rect,
                    0.0,
                    Color32::from_rgba_unmultiplied(230, 178, 65, 45),
                );
            }
            if value.shield.contains(&square) {
                ui.painter()
                    .rect_filled(rect.shrink(1.0), 2.0, Color32::from_rgb(51, 111, 74));
            }
            if let (Some(piece), Some(color)) = (board.piece_on(square), board.color_on(square)) {
                let glyph = match piece {
                    Piece::Pawn => "♟",
                    Piece::Knight => "♞",
                    Piece::Bishop => "♝",
                    Piece::Rook => "♜",
                    Piece::Queen => "♛",
                    Piece::King => "♚",
                };
                ui.painter().text(
                    rect.center(),
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
            if value.pressure.contains(&square) {
                ui.painter().rect_stroke(
                    rect.shrink(2.0),
                    2.0,
                    egui::Stroke::new(2.0, Color32::from_rgb(232, 112, 112)),
                    egui::StrokeKind::Inside,
                );
            }
            if value.king == square {
                ui.painter().rect_stroke(
                    rect.shrink(4.0),
                    2.0,
                    egui::Stroke::new(2.0, Color32::from_rgb(230, 178, 65)),
                    egui::StrokeKind::Inside,
                );
            }
            ui.interact(rect, ui.id().with(("king_square", f, r)), Sense::hover())
                .on_hover_text(square.to_string());
        }
    }
    for i in 0..8 {
        ui.painter().text(
            origin + Vec2::new((i as f32 + 0.5) * cell, size + 12.0),
            egui::Align2::CENTER_CENTER,
            ((b'a' + i as u8) as char).to_string(),
            egui::FontId::proportional(13.0),
            Color32::LIGHT_GRAY,
        );
        ui.painter().text(
            origin + Vec2::new(-10.0, (i as f32 + 0.5) * cell),
            egui::Align2::CENTER_CENTER,
            (8 - i).to_string(),
            egui::FontId::proportional(13.0),
            Color32::LIGHT_GRAY,
        );
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    #[test]
    fn starting_kings_are_sheltered() {
        for c in [Color::White, Color::Black] {
            let s = assess(&Board::default(), c);
            assert_eq!(s.assessment, Assessment::Sheltered);
            assert_eq!(s.shield.len(), 3);
            assert!(s.pressure.is_empty());
        }
    }
    #[test]
    fn missing_shield_and_open_file_expose_king() {
        let b = Board::from_str("rnbqkbnr/ppppp2p/8/8/8/8/PPPPP2P/RNBQ1RK1 w kq - 0 1").unwrap();
        let s = assess(&b, Color::White);
        assert_eq!(s.assessment, Assessment::Exposed);
        assert!(s.open.contains(&6));
        assert!(s.missing.contains(&6));
    }
    #[test]
    fn endgame_king_does_not_need_pawn_shield() {
        let b = Board::from_str("7k/8/8/8/3K4/8/8/8 w - - 0 1").unwrap();
        assert_eq!(assess(&b, Color::White).assessment, Assessment::Endgame);
    }
    #[test]
    fn check_overrides_endgame_assessment() {
        let b = Board::from_str("4r2k/8/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        let s = assess(&b, Color::White);
        assert_eq!(s.assessment, Assessment::Check);
        assert!(s.pressure.contains(&Square::E1));
        assert_eq!(s.attackers, vec![Square::E8]);
    }
    #[test]
    fn corner_shield_uses_only_existing_files() {
        let b = Board::from_str("7k/8/8/8/8/8/6PP/7K w - - 0 1").unwrap();
        let s = assess(&b, Color::White);
        assert_eq!(s.shield.len(), 2);
        assert!(s.missing.is_empty());
    }
}
