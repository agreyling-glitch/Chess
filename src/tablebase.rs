use crate::opening_explorer::sunken_section;
use crate::rules::{Board, MoveGen};
use eframe::egui::{self, Color32, RichText, Stroke, Vec2};
use serde::Deserialize;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodTablebase)]
    fn lookup(fen: &str, retry: bool, begin: bool) -> String;
}
#[derive(Clone, Default)]
struct State {
    root: String,
    path: Vec<Board>,
    clocks: Vec<u32>,
    root_clock: Option<u32>,
    requested: String,
    pending_lookup: bool,
    nodes: Vec<Node>,
    selected_node: Option<usize>,
    root_collapsed: bool,
    hovered_move: Option<(Board, chess::ChessMove)>,
}
#[derive(Clone)]
struct Node {
    parent: Option<usize>,
    board: Board,
    clock: u32,
    uci: String,
    san: String,
    result: String,
    collapsed: bool,
}
impl State {
    fn select_node(&mut self, selected: Option<usize>) {
        self.hovered_move = None;
        self.selected_node = selected;
        self.root_collapsed = false;
        let mut chain = Vec::new();
        let mut cursor = selected;
        while let Some(index) = cursor {
            chain.push(index);
            cursor = self.nodes[index].parent;
            if let Some(parent) = cursor {
                self.nodes[parent].collapsed = false;
            }
        }
        chain.reverse();
        self.path = chain.iter().map(|&i| self.nodes[i].board).collect();
        self.clocks = chain.iter().map(|&i| self.nodes[i].clock).collect();
        self.requested.clear();
        self.pending_lookup = false;
    }
    fn branch(&mut self, board: Board, clock: u32, entry: &Entry) {
        let parent = self.selected_node;
        if let Some(index) = self
            .nodes
            .iter()
            .position(|n| n.parent == parent && n.uci == entry.uci)
        {
            self.select_node(Some(index));
            self.pending_lookup = true;
        } else if self.nodes.len() < 256 {
            let index = self.nodes.len();
            self.nodes.push(Node {
                parent,
                board,
                clock,
                uci: entry.uci.clone(),
                san: entry.san.clone(),
                result: outcome(&entry.category, true).to_owned(),
                collapsed: false,
            });
            self.select_node(Some(index));
            self.pending_lookup = true;
        }
    }
    fn tree_rows(&self) -> Vec<(usize, usize)> {
        if self.root_collapsed {
            return Vec::new();
        }
        let mut rows = Vec::new();
        let mut stack: Vec<_> = self
            .nodes
            .iter()
            .enumerate()
            .rev()
            .filter(|(_, n)| n.parent.is_none())
            .map(|(i, _)| (i, 1))
            .collect();
        while let Some((index, depth)) = stack.pop() {
            rows.push((index, depth));
            if self.nodes[index].collapsed {
                continue;
            }
            stack.extend(
                self.nodes
                    .iter()
                    .enumerate()
                    .rev()
                    .filter(|(_, n)| n.parent == Some(index))
                    .map(|(i, _)| (i, depth + 1)),
            );
        }
        rows
    }
}
fn disclosure(ui: &mut egui::Ui, collapsed: bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(16.0, 22.0), egui::Sense::click());
    let center = rect.center();
    let points = if collapsed {
        vec![
            center + Vec2::new(-2.0, -4.0),
            center + Vec2::new(3.0, 0.0),
            center + Vec2::new(-2.0, 4.0),
        ]
    } else {
        vec![
            center + Vec2::new(-4.0, -2.0),
            center + Vec2::new(4.0, -2.0),
            center + Vec2::new(0.0, 3.0),
        ]
    };
    ui.painter().add(egui::Shape::convex_polygon(
        points,
        if response.hovered() {
            GOLD
        } else {
            ui.visuals().text_color()
        },
        Stroke::NONE,
    ));
    response
        .on_hover_text(if collapsed {
            "Expand continuations"
        } else {
            "Collapse continuations"
        })
        .clicked()
}
fn exploration_tree(ui: &mut egui::Ui, state: &mut State, root: Board, root_fen: &str, display_san: &impl Fn(&str) -> String) -> bool {
    if state.nodes.is_empty() {
        return false;
    }
    let mut lookup_selected = false;
    sunken_section(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(RichText::new("Exploration tree").size(15.0).strong());
            if ui
                .small_button("Clear")
                .on_hover_text("Clear explored branches and return to the game position")
                .clicked()
            {
                state.select_node(None);
                state.nodes.clear();
            }
        });
        ui.label(
            RichText::new("Select a position to revisit it. New moves create branches.")
                .size(13.0)
                .weak(),
        );
        let fullmove = root_fen
            .split_whitespace()
            .nth(5)
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(1);
        let black_root = root.side_to_move() == chess::Color::Black;
        egui::ScrollArea::both()
            .id_salt("tablebase_tree")
            .max_height(180.0)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.horizontal(|ui| {
                    if disclosure(ui, state.root_collapsed) {
                        state.root_collapsed = !state.root_collapsed;
                    }
                    if ui
                        .selectable_label(
                            state.selected_node.is_none(),
                            RichText::new("Game position").color(GOLD),
                        )
                        .clicked()
                    {
                        state.select_node(None);
                        lookup_selected = true;
                    }
                });
                for (index, depth) in state.tree_rows() {
                    let node = &state.nodes[index];
                    let has_children = state.nodes.iter().any(|n| n.parent == Some(index));
                    let mut toggle = false;
                    let ply = depth - 1 + usize::from(black_root);
                    let label = format!(
                        "{}{} {}",
                        fullmove + ply / 2,
                        if ply % 2 == 0 { "." } else { "..." },
                        display_san(&node.san)
                    );
                    let selected = state.selected_node == Some(index);
                    let mut clicked = false;
                    ui.horizontal(|ui| {
                        let indent = depth as f32 * 16.0;
                        let (rect, _) =
                            ui.allocate_exact_size(Vec2::new(indent, 22.0), egui::Sense::hover());
                        let joint = egui::pos2(rect.right() - 8.0, rect.center().y);
                        let stroke = Stroke::new(1.0, Color32::from_rgb(98, 88, 66));
                        ui.painter()
                            .line_segment([egui::pos2(joint.x, rect.top()), joint], stroke);
                        ui.painter()
                            .line_segment([joint, egui::pos2(rect.right(), joint.y)], stroke);
                        if has_children {
                            toggle = disclosure(ui, node.collapsed);
                        } else {
                            ui.allocate_exact_size(Vec2::new(16.0, 22.0), egui::Sense::hover());
                        }
                        clicked = ui
                            .selectable_label(
                                selected,
                                RichText::new(label).size(14.0).color(if selected {
                                    GOLD
                                } else {
                                    ui.visuals().text_color()
                                }),
                            )
                            .clicked();
                        let result = if node.result.starts_with("Draw under") {
                            "Draw · 50-move rule"
                        } else {
                            &node.result
                        };
                        ui.label(
                            RichText::new(result)
                                .size(12.0)
                                .color(result_color(&node.result)),
                        )
                        .on_hover_text("Outcome for the player who made this move");
                    });
                    if toggle {
                        state.nodes[index].collapsed = !state.nodes[index].collapsed;
                    }
                    if clicked {
                        state.select_node(Some(index));
                        lookup_selected = true;
                    }
                }
            });
        if state.nodes.len() >= 256 {
            ui.small("Exploration limit reached. Clear the tree to start again.");
        }
    });
    lookup_selected
}
#[derive(Deserialize)]
struct Data {
    source: Option<String>,
    error: Option<String>,
    category: Option<String>,
    dtz: Option<i32>,
    dtm: Option<i32>,
    #[serde(default)]
    moves: Vec<Entry>,
}
#[derive(Deserialize)]
struct Entry {
    uci: String,
    san: String,
    category: String,
    dtz: Option<i32>,
    dtm: Option<i32>,
    #[serde(default)]
    zeroing: bool,
}
impl Entry {
    fn distances(&self, board: Board, mv: Option<chess::ChessMove>) -> String {
        let mut labels = Vec::new();
        if self.zeroing {
            labels.push(if mv.is_some_and(|mv| board.piece_on(mv.get_source()) == Some(chess::Piece::Pawn)) {
                "Pawn move".to_owned()
            } else {
                "Capture".to_owned()
            });
        } else if let Some(dtz) = self.dtz {
            labels.push(format!("DTZ {}", dtz.unsigned_abs()));
        }
        if let Some(dtm) = self.dtm {
            labels.push(format!("DTM {}", dtm.unsigned_abs()));
        }
        labels.join("   ")
    }
}
fn id() -> egui::Id {
    egui::Id::new("tablebase_state")
}
fn position_fen(board: Board, clock: u32, root_fen: &str, depth: usize) -> String {
    let mut fields: Vec<String> = board.to_string().split_whitespace().map(str::to_owned).collect();
    let root_fields: Vec<_> = root_fen.split_whitespace().collect();
    let start_move = root_fields.get(5).and_then(|s| s.parse::<usize>().ok()).unwrap_or(1);
    let black_root = root_fields.get(1) == Some(&"b");
    fields[4] = clock.to_string();
    fields[5] = (start_move + (depth + usize::from(black_root)) / 2).to_string();
    fields.join(" ")
}
fn lichess_position_url(fen: &str) -> String {
    format!("https://lichess.org/analysis/standard/{}#explorer", fen.replace(' ', "_"))
}
fn outcome(category: &str, child: bool) -> &str {
    match (category, child) {
        ("win", false) | ("loss", true) => "Win",
        ("loss", false) | ("win", true) => "Loss",
        ("draw", _) => "Draw",
        ("cursed-win", false) | ("blessed-loss", true) => "Draw under 50-move rule (cursed win)",
        ("blessed-loss", false) | ("cursed-win", true) => "Draw under 50-move rule (blessed loss)",
        _ => "Uncertain — see Lichess",
    }
}
const GOLD: Color32 = Color32::from_rgb(211, 173, 98);
fn result_color(label: &str) -> Color32 {
    if label == "Win" {
        Color32::from_rgb(137, 203, 156)
    } else if label == "Loss" {
        Color32::from_rgb(230, 139, 133)
    } else {
        Color32::from_rgb(189, 191, 203)
    }
}
pub fn preview(ctx: &egui::Context, root: Board) -> Option<Board> {
    if ctx.data_mut(|d| d.get_persisted::<usize>(egui::Id::new("analysis_details_tab"))) != Some(5)
    {
        return None;
    }
    ctx.data_mut(|d| d.get_temp::<State>(id()))
        .filter(|s| {
            s.root
                .split_whitespace()
                .take(4)
                .eq(root.to_string().split_whitespace().take(4))
        })
        .and_then(|s| s.path.last().copied())
}
pub fn hover_arrow(ctx: &egui::Context, board: Board) -> Option<chess::ChessMove> {
    if ctx.data_mut(|d| d.get_persisted::<usize>(egui::Id::new("analysis_details_tab"))) != Some(5) {
        return None;
    }
    ctx.data_mut(|d| d.get_temp::<State>(id()))
        .and_then(|s| s.hovered_move)
        .filter(|(position, _)| *position == board)
        .map(|(_, mv)| mv)
}
pub fn panel(ui: &mut egui::Ui, root: Board, root_fen: &str, display_san: impl Fn(&str) -> String) {
    let mut state = ui
        .ctx()
        .data_mut(|d| d.get_temp::<State>(id()))
        .unwrap_or_default();
    state.hovered_move = None;
    if state.root != root_fen {
        state = State {
            root: root_fen.to_owned(),
            ..Default::default()
        };
    }
    ui.spacing_mut().item_spacing = Vec2::new(8.0, 7.0);
    ui.label(
        RichText::new("ENDGAME TABLEBASES")
            .size(11.0)
            .color(GOLD)
            .strong(),
    );
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("Syzygy").size(25.0).strong());
        ui.label(RichText::new("7-piece coverage").size(12.0).weak());
    });
    ui.label(
        RichText::new("Perfect-play outcomes. Explore each continuation on the board.")
            .size(13.0)
            .weak(),
    );
    ui.add_space(3.0);
    let tree_request = sunken_section(ui, |ui| {
    ui.label(RichText::new("EXPLORATION").size(11.0).strong().color(GOLD));
    ui.horizontal_wrapped(|ui| {
        if ui
            .add_enabled(!state.path.is_empty(), egui::Button::new("Back"))
            .clicked()
        {
            let parent = state.selected_node.and_then(|i| state.nodes[i].parent);
            state.select_node(parent);
            state.pending_lookup = !state.nodes.is_empty();
        }
        if ui.button("Reset preview").clicked() {
            state.select_node(None);
            state.pending_lookup = !state.nodes.is_empty();
        }
        ui.label(
            RichText::new(if state.path.is_empty() {
                "Game position".to_owned()
            } else {
                format!("Preview · {} plies", state.path.len())
            })
            .size(12.0)
            .color(GOLD),
        );
    });
    exploration_tree(ui, &mut state, root, root_fen, &display_san)
    });
    ui.add_space(10.0);
    let continuation_request = std::mem::take(&mut state.pending_lookup);
    let board = state.path.last().copied().unwrap_or(root);
    let root_clock = state.root_clock.get_or_insert_with(|| {
        root_fen
            .split_whitespace()
            .nth(4)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0)
    });
    let mut refresh = false;
    sunken_section(ui, |ui| {
    egui::CollapsingHeader::new("Position settings & API details").id_salt("tablebase_settings").show(ui, |ui| {
    ui.horizontal_wrapped(|ui| {
        ui.label("50-move clock at root (plies):");
        if ui
            .add_enabled(
                state.path.is_empty(),
                egui::DragValue::new(root_clock).range(0..=10000),
            )
            .changed()
        {
            state.requested.clear();
            state.nodes.clear();
            state.selected_node = None;
        }
    });
    ui.label(RichText::new(
        "Check this clock for imported setup positions; recorded moves determine the default.",
    ).size(13.0));
    ui.label(RichText::new("Standard chess, up to seven pieces including kings, without castling rights. Lookup sends the position to Lichess; internet required, no sign-in needed.").size(13.0));
    if ui.link("Open this position on Lichess").on_hover_text("Send the selected position’s FEN to Lichess analysis in a new tab").clicked() {
        let clock = state.clocks.last().copied().unwrap_or(*root_clock);
        let fen = position_fen(board, clock, root_fen, state.path.len());
        ui.ctx().open_url(egui::OpenUrl::new_tab(lichess_position_url(&fen)));
    }
    ui.add_space(3.0);
    refresh = ui.button("Refresh tablebase result")
        .on_hover_text("Bypass Ironwood’s cache and request a fresh result from Lichess.")
        .clicked();
    });
    });
    ui.add_space(10.0);
    let clock = state.clocks.last().copied().unwrap_or(*root_clock);
    let fen = position_fen(board, clock, root_fen, state.path.len());
    let fields: Vec<_> = fen.split_whitespace().collect();
    let eligible = !board.is_chess960()
        && fields[0]
            .chars()
            .filter(|c| c.is_ascii_alphabetic())
            .count()
            <= 7
        && fields[2] == "-";
    if !eligible {
        sunken_section(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new("Outside tablebase coverage")
                    .strong()
                    .color(GOLD),
            );
            ui.label(
                "Choose a standard position with seven pieces or fewer and no castling rights.",
            );
        });
    } else {
        let manual_request = state.nodes.is_empty() && ui
            .add_sized(
                [ui.available_width(), 34.0],
                egui::Button::new(
                    RichText::new("Look up this position")
                    .strong()
                    .color(Color32::from_rgb(27, 25, 20)),
                )
                .fill(GOLD)
                .corner_radius(6),
            )
            .on_hover_text("Use Ironwood’s cached result when available; otherwise request this position from Lichess.")
            .clicked();
        let request = manual_request || tree_request || continuation_request;
        if request || refresh {
            state.requested = fen.clone();
        }
        if state.requested == fen {
            #[cfg(target_arch = "wasm32")]
            let response = lookup(&fen, refresh, request || refresh);
            #[cfg(not(target_arch = "wasm32"))]
            let response =
                "{\"error\":\"Tablebases are available in the browser app.\"}".to_owned();
            if response.is_empty() {
                sunken_section(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Probing endgame tablebases…");
                    });
                });
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(200));
            } else if let Ok(data) = serde_json::from_str::<Data>(&response) {
                if let Some(error) = data.error {
                    sunken_section(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(RichText::new("Lookup unavailable").strong().color(GOLD));
                        ui.label(error);
                        ui.small("Open Position settings & API details and use Refresh tablebase result to try again.");
                    });
                } else {
                    let result = outcome(data.category.as_deref().unwrap_or("unknown"), false);
                    sunken_section(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal_wrapped(|ui| {
                        let (label, color, help) = if data.source.as_deref() == Some("cache") {
                            ("Cached result", Color32::from_rgb(151, 183, 220), "Reused from Ironwood’s in-memory cache. No new API request. Refresh requests a fresh result.")
                        } else {
                            ("Lichess API", GOLD, "Fetched directly from the Lichess API for this lookup. This indicator stays the same while viewing the result.")
                        };
                        let (dot, _) = ui.allocate_exact_size(Vec2::splat(12.0), egui::Sense::hover());
                        ui.painter().circle_filled(dot.center(), 3.5, color);
                        ui.label(RichText::new(label).size(13.0).color(color)).on_hover_text(help);
                    });
                    ui.label(RichText::new(format!("{} TO MOVE", if board.side_to_move() == chess::Color::White { "WHITE" } else { "BLACK" })).size(11.0).weak());
                    ui.label(RichText::new(result).size(if result.len() > 12 { 18.0 } else { 29.0 }).strong().color(result_color(result)));
                    if let Some(dtz) = data.dtz.filter(|value| *value != 0) {
                        ui.label(RichText::new(format!("DTZ  {} plies", dtz.abs())).size(14.0).color(GOLD))
                            .on_hover_text("Distance to a pawn move, capture or mate; may be rounded. This is not generally distance to mate.");
                    }
                    if let Some(dtm) = data.dtm {
                        ui.label(format!("DTM  {} plies to mate", dtm.abs()));
                    }
                    ui.label(RichText::new("Tablebase outcome · assumes best play").size(13.0));
                    });
                    ui.add_space(5.0);
                    sunken_section(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Continuations").size(16.0).strong());
                        ui.label(
                            RichText::new(format!("{} legal moves", data.moves.len()))
                                .size(12.0)
                                .weak(),
                        );
                    });
                    ui.label(
                        RichText::new("Hover for an arrow · click to explore · outcomes for the player making the move")
                            .size(11.0)
                            .weak(),
                    );
                    egui::ScrollArea::vertical()
                        .id_salt("tablebase_moves")
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            for (rank, entry) in data.moves.into_iter().enumerate() {
                                let legal = MoveGen::new_legal(&board)
                                    .find(|mv| mv.to_string() == entry.uci);
                                let label = outcome(&entry.category, true);
                                let short = if label.starts_with("Draw under") {
                                    "Draw · 50-move rule"
                                } else if label.starts_with("Uncertain") {
                                    "Uncertain"
                                } else {
                                    label
                                };
                                let width = ui.available_width();
                                let distances = entry.distances(board, legal);
                                let (rect, response) = ui.allocate_exact_size(
                                    Vec2::new(width, 28.0),
                                    egui::Sense::click(),
                                );
                                let fill = if response.hovered() && legal.is_some() {
                                    Color32::from_rgb(54, 49, 38)
                                } else if rank % 2 == 0 {
                                    Color32::from_rgb(37, 39, 42)
                                } else {
                                    Color32::from_rgb(29, 31, 34)
                                };
                                ui.painter().rect_filled(rect, 5.0, fill);
                                if rank == 0 {
                                    ui.painter().rect_filled(
                                        egui::Rect::from_min_size(
                                            rect.min,
                                            Vec2::new(3.0, rect.height()),
                                        ),
                                        2.0,
                                        GOLD,
                                    );
                                }
                                let text_y = rect.center().y;
                                ui.painter().text(
                                    egui::pos2(rect.left() + 12.0, text_y),
                                    egui::Align2::LEFT_CENTER,
                                    format!("{:02}", rank + 1),
                                    egui::FontId::proportional(11.0),
                                    ui.visuals().weak_text_color(),
                                );
                                ui.painter().text(
                                    egui::pos2(rect.left() + 40.0, text_y),
                                    egui::Align2::LEFT_CENTER,
                                    display_san(&entry.san),
                                    egui::FontId::proportional(16.0),
                                    if rank == 0 {
                                        GOLD
                                    } else {
                                        ui.visuals().text_color()
                                    },
                                );
                                let mut badge_right = rect.right() - 8.0;
                                for text in distances.rsplit("   ").filter(|text| !text.is_empty()) {
                                    let galley = ui.painter().layout_no_wrap(
                                        text.to_owned(),
                                        egui::FontId::proportional(11.0),
                                        Color32::from_rgb(230, 232, 236),
                                    );
                                    let badge = egui::Rect::from_min_size(
                                        egui::pos2(badge_right - galley.size().x - 10.0, text_y - 10.0),
                                        Vec2::new(galley.size().x + 10.0, 20.0),
                                    );
                                    ui.painter().rect_filled(badge, 3.0, Color32::from_rgb(65, 68, 73));
                                    ui.painter().galley(
                                        egui::pos2(badge.left() + 5.0, text_y - galley.size().y / 2.0),
                                        galley,
                                        Color32::from_rgb(230, 232, 236),
                                    );
                                    badge_right = badge.left() - 5.0;
                                }
                                ui.painter().text(
                                    egui::pos2(badge_right - 8.0, text_y),
                                    egui::Align2::RIGHT_CENTER,
                                    short,
                                    egui::FontId::proportional(12.0),
                                    result_color(label),
                                );
                                let response = response.on_hover_text(format!(
                                    "{} · {}\nHover to show the move arrow; click to explore. Moves are ranked by Lichess.\nDTZ: distance to a pawn move, capture or mate (half-moves); may be rounded.\nDTM: distance to mate (half-moves). Distances describe the position after this move. Missing values are unavailable.",
                                    display_san(&entry.san), label
                                ));
                                if response.hovered() {
                                    state.hovered_move = legal.map(|mv| (board, mv));
                                }
                                if response.clicked() && legal.is_some() {
                                    if let Some(mv) = legal {
                                        let zeroing = board.piece_on(mv.get_source())
                                            == Some(chess::Piece::Pawn)
                                            || board.piece_on(mv.get_dest()).is_some();
                                        let next_clock =
                                            if zeroing { 0 } else { clock.saturating_add(1) };
                                        state.branch(board.make_move_new(mv), next_clock, &entry);
                                        ui.ctx().request_repaint();
                                    }
                                }
                            }
                        });
                    ui.label(RichText::new("Ranked by Lichess · DTZ favors progress toward a pawn move, capture or mate.").size(13.0));
                    });
                }
            } else {
                ui.label("Unexpected tablebase response. Retry when ready.");
            }
        }
    }
    ui.ctx().data_mut(|d| d.insert_temp(id(), state));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    #[test]
    fn continuation_distances_handle_signed_missing_and_zeroing_values() {
        let board = Board::from_str("7k/8/5KQ1/8/8/8/8/8 w - - 9 60").unwrap();
        let mut entry: Entry = serde_json::from_str(r#"{"uci":"g6g4","san":"Qg4","category":"loss","dtz":-2,"dtm":-6}"#).unwrap();
        assert_eq!(entry.distances(board, None), "DTZ 2   DTM 6");
        entry.zeroing = true;
        assert_eq!(entry.distances(board, None), "Capture   DTM 6");
        entry.zeroing = false;
        entry.dtm = Some(0);
        assert_eq!(entry.distances(board, None), "DTZ 2   DTM 0");
        let missing: Entry = serde_json::from_str(r#"{"uci":"g6g4","san":"Qg4","category":"draw","dtz":null,"dtm":null}"#).unwrap();
        assert!(missing.distances(board, None).is_empty());
    }
    #[test]
    fn hover_arrow_is_scoped_to_tablebase_tab_and_displayed_position() {
        let ctx = egui::Context::default();
        let board = Board::from_str("7k/8/5KQ1/8/8/8/8/8 w - - 9 60").unwrap();
        let mv = MoveGen::new_legal(&board).find(|mv| mv.to_string() == "g6g4").unwrap();
        let mut state = State { hovered_move: Some((board, mv)), ..Default::default() };
        ctx.data_mut(|d| d.insert_temp(id(), state.clone()));
        assert_eq!(hover_arrow(&ctx, board), None);
        ctx.data_mut(|d| d.insert_persisted(egui::Id::new("analysis_details_tab"), 5usize));
        assert_eq!(hover_arrow(&ctx, board), Some(mv));
        assert_eq!(hover_arrow(&ctx, board.make_move_new(mv)), None);
        state.select_node(None);
        ctx.data_mut(|d| d.insert_temp(id(), state));
        assert_eq!(hover_arrow(&ctx, board), None);
    }
    #[test]
    fn lichess_link_carries_selected_fen_and_move_counters() {
        let fen = "7k/8/5KQ1/8/8/8/8/8 w - - 9 60";
        let root = Board::from_str(fen).unwrap();
        assert_eq!(position_fen(root, 9, fen, 0), fen);
        let mv = MoveGen::new_legal(&root).find(|mv| mv.to_string() == "g6g4").unwrap();
        let next = root.make_move_new(mv);
        let selected = position_fen(next, 10, fen, 1);
        assert_eq!(selected, "7k/8/5K2/8/6Q1/8/8/8 b - - 10 60");
        assert_eq!(lichess_position_url(&selected), "https://lichess.org/analysis/standard/7k/8/5K2/8/6Q1/8/8/8_b_-_-_10_60#explorer");
        assert!(position_fen(root, 11, fen, 2).ends_with("11 61"));
        assert!(position_fen(root, 10, "7k/8/5KQ1/8/8/8/8/8 b - - 9 60", 1).ends_with("10 61"));
    }
    fn explore(state: &mut State, board: Board, uci: &str, san: &str, clock: u32) -> Board {
        let mv = MoveGen::new_legal(&board)
            .find(|mv| mv.to_string() == uci)
            .unwrap();
        let next = board.make_move_new(mv);
        state.branch(
            next,
            clock,
            &Entry {
                uci: uci.to_owned(),
                san: san.to_owned(),
                category: "loss".to_owned(),
                dtz: None,
                dtm: None,
                zeroing: false,
            },
        );
        next
    }
    #[test]
    fn exploration_preserves_sibling_branches_and_restores_path_and_clocks() {
        let root = Board::from_str("7k/8/5KQ1/8/8/8/8/8 w - - 9 60").unwrap();
        let mut state = State::default();
        let first = explore(&mut state, root, "g6g4", "Qg4", 10);
        let reply = explore(&mut state, first, "h8h7", "Kh7", 11);
        state.select_node(None);
        explore(&mut state, root, "g6g5", "Qg5", 10);
        assert_eq!(state.tree_rows(), vec![(0, 1), (1, 2), (2, 1)]);
        state.nodes[0].collapsed = true;
        assert_eq!(state.tree_rows(), vec![(0, 1), (2, 1)]);
        assert_eq!(state.nodes.len(), 3);
        state.root_collapsed = true;
        assert!(state.tree_rows().is_empty());
        state.select_node(Some(1));
        assert!(!state.root_collapsed);
        assert!(!state.nodes[0].collapsed);
        assert_eq!(state.tree_rows(), vec![(0, 1), (1, 2), (2, 1)]);
        assert_eq!(state.path, vec![first, reply]);
        assert_eq!(state.clocks, vec![10, 11]);
        state.select_node(None);
        explore(&mut state, root, "g6g4", "Qg4", 10);
        assert_eq!(state.nodes.len(), 3);
        assert_eq!(state.selected_node, Some(0));
        assert_eq!(state.nodes[1].parent, Some(0));
        assert!(state.requested.is_empty());
        assert!(state.pending_lookup);
        state.select_node(None);
        assert!(!state.pending_lookup);
    }
    #[test]
    fn child_outcomes_are_inverted_without_claiming_cursed_wins() {
        assert_eq!(outcome("loss", true), "Win");
        assert_eq!(outcome("win", true), "Loss");
        assert_eq!(
            outcome("blessed-loss", true),
            "Draw under 50-move rule (cursed win)"
        );
        assert_eq!(outcome("maybe-win", false), "Uncertain — see Lichess");
    }
}
