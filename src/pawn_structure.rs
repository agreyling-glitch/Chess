use crate::rules::Board;
use chess::{Color, Piece, Square};
use egui::{Color32, RichText, Sense, Vec2};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Structure {
    pub pawns: Vec<Square>,
    pub isolated: Vec<Square>,
    pub passed: Vec<Square>,
    pub chains: Vec<Vec<Square>>,
}

pub fn structure(board: &Board, color: Color) -> Structure {
    let pawns: Vec<_> = (board.pieces(Piece::Pawn) & board.color_combined(color))
        .into_iter()
        .collect();
    let enemies: Vec<_> = (board.pieces(Piece::Pawn) & board.color_combined(!color))
        .into_iter()
        .collect();
    let file = |s: Square| s.get_file().to_index() as i32;
    let rank = |s: Square| s.get_rank().to_index() as i32;
    let isolated = pawns
        .iter()
        .copied()
        .filter(|&p| !pawns.iter().any(|&q| (file(p) - file(q)).abs() == 1))
        .collect();
    let passed = pawns
        .iter()
        .copied()
        .filter(|&p| {
            !enemies.iter().any(|&q| {
                (file(p) - file(q)).abs() <= 1
                    && if color == Color::White {
                        rank(q) > rank(p)
                    } else {
                        rank(q) < rank(p)
                    }
            })
        })
        .collect();
    // A chain is a connected group of friendly pawns linked by diagonal protection.
    let mut remaining = pawns.clone();
    let mut chains = Vec::new();
    while let Some(seed) = remaining.pop() {
        let mut group = vec![seed];
        let mut cursor = 0;
        while cursor < group.len() {
            let p = group[cursor];
            let mut i = 0;
            while i < remaining.len() {
                let q = remaining[i];
                if (file(p) - file(q)).abs() == 1 && (rank(p) - rank(q)).abs() == 1 {
                    group.push(remaining.remove(i));
                } else {
                    i += 1;
                }
            }
            cursor += 1;
        }
        if group.len() > 1 {
            group.sort_by_key(|s| s.to_index());
            chains.push(group);
        }
    }
    chains.sort_by_key(|g| g[0].to_index());
    Structure {
        pawns,
        isolated,
        passed,
        chains,
    }
}

fn squares(values: &[Square]) -> String {
    if values.is_empty() {
        "None".into()
    } else {
        values
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    }
}

pub fn panel(
    ui: &mut egui::Ui,
    positions: &[Board],
    labels: &[String],
    current: usize,
) -> Option<usize> {
    ui.scope(|ui| panel_content(ui, positions, labels, current))
        .inner
}

fn panel_content(
    ui: &mut egui::Ui,
    positions: &[Board],
    labels: &[String],
    current: usize,
) -> Option<usize> {
    ui.spacing_mut().item_spacing = Vec2::new(8.0, 10.0);
    ui.style_mut()
        .text_styles
        .insert(egui::TextStyle::Body, egui::FontId::proportional(15.0));
    ui.style_mut()
        .text_styles
        .insert(egui::TextStyle::Button, egui::FontId::proportional(15.0));
    let mut jump = None;
    let Some(board) = positions.get(current) else {
        return None;
    };
    ui.label(
        RichText::new(
            labels
                .get(current)
                .map(String::as_str)
                .unwrap_or("Position"),
        )
        .size(18.0)
        .strong(),
    );
    ui.add_space(4.0);
    let sides = [
        structure(board, Color::White),
        structure(board, Color::Black),
    ];
    let id = ui.id().with("pawn_feature");
    let mut feature = ui.data_mut(|data| data.get_persisted::<usize>(id).unwrap_or(0));
    ui.horizontal_wrapped(|ui| {
        for (i, name) in ["Chains", "Isolated", "Passed"].iter().enumerate() {
            ui.selectable_value(&mut feature, i, *name);
        }
    });
    ui.data_mut(|data| data.insert_persisted(id, feature));
    ui.label(
        RichText::new(match feature {
            0 => "Chains: friendly pawns connected by diagonal protection.",
            1 => "Isolated: no friendly pawn on either adjacent file.",
            _ => "Passed: no enemy pawn ahead on the same or adjacent files.",
        })
        .size(14.0)
        .color(Color32::from_rgb(185, 193, 201)),
    );
    ui.add_space(6.0);
    let width = ui.available_width();
    let gap = 18.0;
    let board_width = (width * 0.42).min(280.0);
    let detail_width = (width - board_width - gap).max(100.0);
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        ui.allocate_ui_with_layout(
            Vec2::new(board_width, 0.0),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.set_width(board_width);
                pawn_board(ui, &sides, feature);
            },
        );
        ui.allocate_ui_with_layout(
            Vec2::new(detail_width, 0.0),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.set_width(detail_width);
                ui.scope(|ui| {
                    ui.style_mut()
                        .text_styles
                        .insert(egui::TextStyle::Body, egui::FontId::proportional(13.0));
                    pawn_details(ui, &sides);
                });
            },
        );
    });
    ui.add_space(4.0);
    ui.add(
        egui::Label::new(
            RichText::new("White at bottom · Gold highlights the selected feature")
                .size(14.0)
                .color(Color32::from_rgb(185, 193, 201)),
        )
        .extend(),
    );
    ui.add_space(8.0);
    ui.separator();
    ui.label(RichText::new("Structure evolution").size(18.0).strong());
    ui.label(
        RichText::new("Select a row to inspect the position. Only pawn changes appear.")
            .size(14.0)
            .color(Color32::from_rgb(185, 193, 201)),
    );
    ui.label(
        RichText::new("C = chains   ·   I = isolated   ·   P = passed")
            .size(14.0)
            .color(Color32::from_rgb(211, 173, 98)),
    );
    let width = (ui.available_width() - ui.spacing().scroll.allocated_width()).max(160.0);
    let move_width = width * 0.34;
    let count_width = (width - move_width) / 6.0;
    let (header, _) = ui.allocate_exact_size(Vec2::new(width, 48.0), Sense::hover());
    ui.painter()
        .rect_filled(header, 5.0, Color32::from_rgb(32, 38, 45));
    let text = |ui: &egui::Ui, point, value: &str, strong: bool| {
        ui.painter().text(
            point,
            egui::Align2::CENTER_CENTER,
            value,
            egui::FontId::proportional(if strong { 15.0 } else { 14.0 }),
            if strong {
                Color32::WHITE
            } else {
                Color32::from_rgb(195, 203, 210)
            },
        );
    };
    text(
        ui,
        header.min + Vec2::new(move_width / 2.0, 24.0),
        "Position",
        true,
    );
    for (side, name) in ["White", "Black"].iter().enumerate() {
        text(
            ui,
            header.min + Vec2::new(move_width + (side as f32 * 3.0 + 1.5) * count_width, 13.0),
            name,
            true,
        );
        for (column, name) in ["C", "I", "P"].iter().enumerate() {
            text(
                ui,
                header.min
                    + Vec2::new(
                        move_width + (side as f32 * 3.0 + column as f32 + 0.5) * count_width,
                        35.0,
                    ),
                name,
                false,
            );
        }
    }
    // Keep ten complete rows visible; additional structural changes scroll.
    const ROW_HEIGHT: f32 = 30.0;
    const VISIBLE_ROWS: f32 = 10.0;
    egui::ScrollArea::vertical()
        .id_salt("pawn_timeline")
        .max_height(ROW_HEIGHT * VISIBLE_ROWS)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let mut previous = None;
            let mut row = 0;
            for (index, board) in positions.iter().enumerate() {
                let values = [
                    structure(board, Color::White),
                    structure(board, Color::Black),
                ];
                if previous.as_ref() == Some(&values) {
                    continue;
                }
                let (rect, response) =
                    ui.allocate_exact_size(Vec2::new(width, ROW_HEIGHT), Sense::click());
                let selected = index == current;
                ui.painter().rect_filled(
                    rect,
                    3.0,
                    if selected {
                        Color32::from_rgb(86, 67, 35)
                    } else if response.hovered() {
                        Color32::from_rgb(43, 50, 59)
                    } else if row % 2 == 0 {
                        Color32::from_rgb(24, 29, 35)
                    } else {
                        Color32::from_rgb(18, 23, 29)
                    },
                );
                ui.painter().text(
                    rect.min + Vec2::new(8.0, ROW_HEIGHT * 0.5),
                    egui::Align2::LEFT_CENTER,
                    labels.get(index).map(String::as_str).unwrap_or("Position"),
                    egui::FontId::proportional(14.0),
                    if selected {
                        Color32::from_rgb(241, 206, 131)
                    } else {
                        Color32::WHITE
                    },
                );
                for (side, value) in values.iter().enumerate() {
                    for (column, count) in
                        [value.chains.len(), value.isolated.len(), value.passed.len()]
                            .iter()
                            .enumerate()
                    {
                        text(
                            ui,
                            rect.min
                                + Vec2::new(
                                    move_width
                                        + (side as f32 * 3.0 + column as f32 + 0.5) * count_width,
                                    ROW_HEIGHT * 0.5,
                                ),
                            &count.to_string(),
                            false,
                        );
                    }
                }
                for x in [move_width, move_width + count_width * 3.0] {
                    ui.painter().line_segment(
                        [
                            rect.min + Vec2::new(x, 0.0),
                            rect.min + Vec2::new(x, ROW_HEIGHT),
                        ],
                        egui::Stroke::new(1.0, Color32::from_white_alpha(18)),
                    );
                }
                if response.clicked() {
                    jump = Some(index);
                }
                row += 1;
                previous = Some(values);
            }
        });
    jump
}

fn pawn_board(ui: &mut egui::Ui, sides: &[Structure; 2], feature: usize) {
    let size = (ui.available_width() - 24.0).min(256.0).max(80.0);
    let left = ((ui.available_width() - size) * 0.5).max(0.0);
    let (outer, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), size + 22.0), Sense::hover());
    let board_min = outer.min + Vec2::new(left, 0.0);
    let rect = egui::Rect::from_min_size(board_min, Vec2::splat(size));
    let cell = size / 8.0;
    for rank in 0..8 {
        for file in 0..8 {
            let square =
                Square::make_square(chess::Rank::from_index(rank), chess::File::from_index(file));
            let tile = egui::Rect::from_min_size(
                rect.min + Vec2::new(file as f32 * cell, (7 - rank) as f32 * cell),
                Vec2::splat(cell),
            );
            ui.painter().rect_filled(
                tile,
                0.0,
                if (rank + file) % 2 == 0 {
                    Color32::from_rgb(70, 77, 83)
                } else {
                    Color32::from_rgb(115, 121, 126)
                },
            );
            for (side, value) in sides.iter().enumerate() {
                if !value.pawns.contains(&square) {
                    continue;
                }
                let highlighted = match feature {
                    0 => value.chains.iter().any(|g| g.contains(&square)),
                    1 => value.isolated.contains(&square),
                    _ => value.passed.contains(&square),
                };
                if highlighted {
                    ui.painter().rect_filled(
                        tile.shrink(1.0),
                        2.0,
                        Color32::from_rgb(151, 112, 35),
                    );
                }
                ui.painter().text(
                    tile.center(),
                    egui::Align2::CENTER_CENTER,
                    if side == 0 { "♙" } else { "♟" },
                    egui::FontId::proportional(cell * 0.85),
                    if side == 0 {
                        Color32::WHITE
                    } else {
                        Color32::BLACK
                    },
                );
            }
            ui.interact(tile, ui.id().with((rank, file)), Sense::hover())
                .on_hover_text(square.to_string());
        }
    }
    for i in 0..8 {
        ui.painter().text(
            rect.min + Vec2::new((i as f32 + 0.5) * cell, size + 12.0),
            egui::Align2::CENTER_CENTER,
            ((b'a' + i as u8) as char).to_string(),
            egui::FontId::proportional(13.0),
            Color32::LIGHT_GRAY,
        );
        ui.painter().text(
            rect.min + Vec2::new(-10.0, (i as f32 + 0.5) * cell),
            egui::Align2::CENTER_CENTER,
            (8 - i).to_string(),
            egui::FontId::proportional(13.0),
            Color32::LIGHT_GRAY,
        );
    }
    ui.add_space(4.0);
}

fn pawn_count(ui: &mut egui::Ui, title: &str, count: usize) {
    ui.allocate_ui_with_layout(
        Vec2::new(92.0, 16.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.add_sized(
                [58.0, 16.0],
                egui::Label::new(RichText::new(title).size(13.0).strong()).halign(egui::Align::Min),
            );
            ui.add_sized(
                [22.0, 16.0],
                egui::Label::new(RichText::new(count.to_string()).size(13.0).monospace().strong())
                    .halign(egui::Align::Max),
            );
        },
    );
}

fn pawn_details(ui: &mut egui::Ui, sides: &[Structure; 2]) {
    for (name, value) in ["White", "Black"].iter().zip(sides.iter()) {
        egui::Frame::new()
            .fill(Color32::from_rgb(25, 30, 36))
            .corner_radius(6.0)
            .inner_margin(8.0)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 3.0;
                ui.set_width(ui.available_width());
                ui.label(RichText::new(*name).size(14.0).strong());
                for (title, count, detail) in [
                    (
                        "Chains",
                        value.chains.len(),
                        if value.chains.is_empty() {
                            "None".into()
                        } else {
                            value
                                .chains
                                .iter()
                                .map(|g| squares(g))
                                .collect::<Vec<_>>()
                                .join(" / ")
                        },
                    ),
                    ("Isolated", value.isolated.len(), squares(&value.isolated)),
                    ("Passed", value.passed.len(), squares(&value.passed)),
                ] {
                    if ui.available_width() < 220.0 {
                        pawn_count(ui, title, count);
                        ui.add(egui::Label::new(RichText::new(detail).size(13.0)).wrap());
                    } else {
                        ui.horizontal_top(|ui| {
                            pawn_count(ui, title, count);
                            ui.add(egui::Label::new(RichText::new(detail).size(13.0)).wrap());
                        });
                    }
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    #[test]
    fn starting_pawns_have_no_special_features() {
        for color in [Color::White, Color::Black] {
            let s = structure(&Board::default(), color);
            assert_eq!(s.pawns.len(), 8);
            assert!(s.chains.is_empty() && s.isolated.is_empty() && s.passed.is_empty());
        }
    }
    #[test]
    fn chains_isolation_and_passers_are_color_symmetric() {
        let board = Board::from_str("7k/8/1p6/p7/6P1/5P2/2P5/7K w - - 0 1").unwrap();
        let white = structure(&board, Color::White);
        assert_eq!(white.chains, vec![vec![Square::F3, Square::G4]]);
        assert_eq!(white.isolated, vec![Square::C2]);
        assert_eq!(white.passed, vec![Square::F3, Square::G4]);
        let black = structure(&board, Color::Black);
        assert_eq!(black.chains, vec![vec![Square::A5, Square::B6]]);
        assert!(black.isolated.is_empty());
        assert_eq!(black.passed, vec![Square::A5]);
    }
    #[test]
    fn enemy_pawns_behind_do_not_block_passers_and_doubled_pawns_can_be_isolated() {
        let board = Board::from_str("7k/8/8/2P5/2P5/2p5/8/7K w - - 0 1").unwrap();
        let white = structure(&board, Color::White);
        assert_eq!(white.passed, vec![Square::C4, Square::C5]);
        assert_eq!(white.isolated, vec![Square::C4, Square::C5]);
        assert_eq!(structure(&board, Color::Black).passed, vec![Square::C3]);
    }
}
