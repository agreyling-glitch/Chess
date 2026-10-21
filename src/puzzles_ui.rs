use super::*;
use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodPuzzles)]
    fn bridge(command: &str, payload: &str) -> String;
}
#[cfg(not(target_arch = "wasm32"))]
fn bridge(command: &str, _: &str) -> String {
    if command == "history" { "[]".into() } else if command == "save" { String::new() }
    else { r#"{"error":"Online puzzles are available in the browser app."}"#.into() }
}

#[derive(Clone, Deserialize, Serialize)]
struct PuzzleData { game: PuzzleGame, puzzle: PuzzleInfo }
#[derive(Clone, Deserialize, Serialize)]
struct PuzzleGame { pgn: String }
#[derive(Clone, Deserialize, Serialize)]
struct PuzzleInfo {
    id: String,
    rating: u32,
    #[serde(rename = "initialPly")]
    initial_ply: usize,
    solution: Vec<String>,
    #[serde(default)]
    themes: Vec<String>,
}
#[derive(Clone, Deserialize, Serialize)]
struct Record { data: PuzzleData, solved: bool }
#[derive(Deserialize)]
struct Response { data: Option<PuzzleData>, error: Option<String> }
#[derive(Clone)]
struct Session {
    data: PuzzleData,
    start: Board,
    board: Board,
    moves: Vec<ChessMove>,
    played: Vec<ChessMove>,
    step: usize,
    selected: Option<Square>,
    promotion: Option<(Square, Square)>,
    recorded: bool,
    storage_started: bool,
    done: bool,
    message: String,
}
impl Session {
    fn new(data: PuzzleData) -> Result<Self, String> {
        if data.puzzle.id.is_empty() || data.puzzle.solution.is_empty() || data.puzzle.solution.len() > 100 {
            return Err("The puzzle has no valid solution.".into());
        }
        let (positions, _) = ChessApp::parse_pgn_mainline(&data.game.pgn)?;
        // Lichess initialPly is the zero-based ply of the opponent's setup move.
        let start = *positions.get(data.puzzle.initial_ply.checked_add(1).ok_or("Invalid puzzle ply")?)
            .ok_or("The source game does not contain the puzzle position.")?;
        let mut board = start;
        let mut moves = Vec::new();
        for value in &data.puzzle.solution {
            let mv = ChessMove::from_str(value).map_err(|_| "Invalid puzzle solution move")?;
            if !MoveGen::new_legal(&board).any(|legal| legal == mv) { return Err("The puzzle solution is not legal from its source position.".into()); }
            moves.push(mv);
            board = board.make_move_new(mv);
        }
        Ok(Self { data, start, board: start, moves, played: Vec::new(), step: 0, selected: None,
            promotion: None, recorded: false, storage_started: false, done: false, message: "Find the best move.".into() })
    }
    fn submit(&mut self, mv: ChessMove) -> Option<bool> {
        if self.done || !MoveGen::new_legal(&self.board).any(|legal| legal == mv) { return None; }
        self.selected = None;
        self.promotion = None;
        let after = self.board.make_move_new(mv);
        if self.moves.get(self.step) != Some(&mv) && after.status() != BoardStatus::Checkmate {
            self.message = "That is not the solution. Try again.".into();
            return Some(false);
        }
        self.board = after;
        self.played.push(mv);
        self.step += 1;
        if self.board.status() == BoardStatus::Checkmate { self.step = self.moves.len(); }
        if let Some(&reply) = self.moves.get(self.step) {
            self.board = self.board.make_move_new(reply);
            self.played.push(reply);
            self.step += 1;
            self.message = format!("Opponent replied {}. Find your next move.", reply);
        }
        self.done = self.step >= self.moves.len();
        if self.done { self.message = "Puzzle solved!".into(); return Some(true); }
        None
    }
    fn reveal(&mut self) {
        self.played = self.moves.clone();
        self.board = self.start;
        for mv in &self.moves { self.board = self.board.make_move_new(*mv); }
        self.done = true;
        self.selected = None;
        self.promotion = None;
        self.message = format!("Solution: {}", self.data.puzzle.solution.join(" "));
    }
    fn review_pgn(&self) -> String {
        let mut pgn = format!("[Event \"Lichess puzzle\"]\n[SetUp \"1\"]\n[FEN \"{}\"]\n[Result \"*\"]\n", self.start);
        pgn.push_str(&format!("[PuzzleId \"{}\"]\n[Site \"https://lichess.org/training/{}\"]\n\n", self.data.puzzle.id, self.data.puzzle.id));
        let mut board = self.start;
        for mv in &self.played {
            pgn.push_str(&ChessApp::san_for_move(&board, *mv));
            pgn.push(' ');
            board = board.make_move_new(*mv);
        }
        pgn.push('*');
        pgn
    }
}

#[derive(Clone)]
struct State {
    tab: usize,
    theme: usize,
    difficulty: usize,
    session: Option<Session>,
    history: Vec<Record>,
    loading: bool,
    error: String,
    storage_error: String,
    linked: bool,
    synced_board: Option<Board>,
}
impl Default for State {
    fn default() -> Self {
        let history = serde_json::from_str::<Vec<Record>>(&bridge("history", "")).unwrap_or_default();
        Self { tab: 0, theme: 0, difficulty: 2, session: None, history, loading: false, error: String::new(), storage_error: String::new(), linked: false, synced_board: None }
    }
}
impl State {
    fn record(&mut self, solved: bool) {
        let Some(session) = self.session.as_mut().filter(|s| !s.recorded) else { return; };
        session.recorded = true;
        self.history.retain(|r| r.data.puzzle.id != session.data.puzzle.id);
        self.history.push(Record { data: session.data.clone(), solved });
        if self.history.len() > 500 { self.history.remove(0); }
        self.storage_error = bridge("save", &serde_json::to_string(&self.history).unwrap_or_default());
    }
    fn theme_results(&self, theme: &str) -> (usize, usize) {
        let mut solved = 0;
        let mut review = 0;
        for record in &self.history {
            if theme == "mix" || record.data.puzzle.themes.iter().any(|tag| tag == theme) {
                if record.solved { solved += 1; } else { review += 1; }
            }
        }
        (solved, review)
    }

    fn request(&mut self, daily: bool, theme: &str, difficulty: &str) {
        self.error.clear();
        self.loading = true;
        bridge("request", &serde_json::json!({"daily":daily,"theme":theme,"difficulty":difficulty}).to_string());
    }
    fn poll(&mut self) {
        if !self.loading { return; }
        if let Ok(response) = serde_json::from_str::<Response>(&bridge("poll", "")) {
            if let Some(error) = response.error { self.error = error; self.loading = false; }
            else if let Some(data) = response.data {
                self.loading = false;
                match Session::new(data) {
                    Ok(session) => { self.session = Some(session); }
                    Err(error) => { self.error = error; }
                }
            }
        }
    }
}

const THEMES: &[(&str, &str)] = &[("Mixed", "mix"), ("Forks", "fork"), ("Pins", "pin"), ("Skewers", "skewer"),
    ("Discovered attacks", "discoveredAttack"), ("Hanging pieces", "hangingPiece"), ("Mate in one", "mateIn1"),
    ("Mate in two", "mateIn2"), ("Endgames", "endgame"), ("Opening", "opening"), ("Middlegame", "middlegame")];
const DIFFICULTIES: &[(&str, &str)] = &[("Easiest", "easiest"), ("Easier", "easier"), ("Normal", "normal"), ("Harder", "harder"), ("Hardest", "hardest")];

fn training_emphasis(stats: &crate::training::AnalysisStats, theme: &str) -> Option<&'static str> {
    if stats.practice_focus().is_some_and(|i| ["opening", "middlegame", "endgame"][i] == theme) {
        Some("Recommended phase")
    } else if stats.missed_mates > 0 && matches!(theme, "mateIn1" | "mateIn2") {
        Some("Mate practice")
    } else { None }
}

impl ChessApp {
    pub(super) fn puzzle_mode(&self) -> bool {
        self.puzzle_context.data(|d| d.get_temp::<bool>(egui::Id::new("puzzles_open"))).unwrap_or(false)
    }

    pub(super) fn exit_puzzle_mode(&mut self) {
        self.puzzle_context.data_mut(|data| {
            data.insert_temp(egui::Id::new("puzzles_open"), false);
            let id = egui::Id::new("puzzles_state");
            if let Some(mut state) = data.get_temp::<State>(id) {
                state.session = None;
                state.loading = false;
                state.linked = false;
                state.synced_board = None;
                state.error.clear();
                data.insert_temp(id, state);
            }
        });
    }

    pub(super) fn puzzle_navigation_locked(&self) -> bool {
        self.puzzle_mode() && self.puzzle_context.data(|d| {
            d.get_temp::<State>(egui::Id::new("puzzles_state"))
                .is_none_or(|state| state.loading || state.session.is_none_or(|session| !session.done))
        })
    }

    pub(super) fn play_puzzle_move(&mut self, mv: ChessMove) {
        let ctx = self.puzzle_context.clone();
        let id = egui::Id::new("puzzles_state");
        let mut state = ctx.data(|d| d.get_temp::<State>(id)).unwrap_or_default();
        if !state.loading {
            if let Some(session) = state.session.as_mut().filter(|session| !session.done) {
                let result = session.submit(mv);
                let _ = self.sync_puzzle_position(session);
                state.synced_board = Some(session.board);
                if let Some(solved) = result { state.record(solved); }
            }
        }
        self.selected = None;
        self.legal_targets.clear();
        self.promotion = None;
        ctx.data_mut(|d| d.insert_temp(id, state));
    }

    pub(super) fn phase_practice_ui(ui: &mut egui::Ui, phase: usize, enabled: bool) -> bool {
        let mut launched = false;
        let state_id = egui::Id::new("puzzles_state");
        let mut state = ui.ctx().data(|d| d.get_temp::<State>(state_id)).unwrap_or_default();
        ui.add_space(14.0);
        let controls = egui::Frame::new().fill(Color32::from_rgb(13, 17, 22))
            .corner_radius(6.0).inner_margin(16.0).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 10.0;
            ui.label(RichText::new("CHOOSE YOUR PUZZLE DIFFICULTY").size(12.0).strong()
                .color(Color32::from_rgb(211, 173, 98)));
            ui.scope(|ui| {
            ui.spacing_mut().interact_size.y = 38.0;
            ui.visuals_mut().widgets.inactive.bg_fill = Color32::from_rgb(42, 49, 57);
            ui.visuals_mut().widgets.inactive.weak_bg_fill = Color32::from_rgb(42, 49, 57);
            egui::ComboBox::from_id_salt("phase_puzzle_difficulty")
                .width(ui.available_width())
                .selected_text(RichText::new(DIFFICULTIES[state.difficulty].0).size(17.0).strong()
                    .color(Color32::from_rgb(240, 234, 220))).show_ui(ui, |ui| {
                    for (i, (name, _)) in DIFFICULTIES.iter().enumerate() {
                        ui.selectable_value(&mut state.difficulty, i, *name);
                    }
                });
            });
            let button_width = ui.available_width();
            if ui.add_enabled(enabled && !state.loading, egui::Button::new(
                RichText::new("Practice this phase").size(17.0).strong().color(Color32::from_rgb(24, 27, 31)))
                .min_size(Vec2::new(button_width, 46.0))
                .fill(Color32::from_rgb(211, 173, 98)).corner_radius(6.0).raised_gold())
                .on_disabled_hover_text("Finish active training or online play, and wait for any puzzle request to complete.").clicked() {
                let theme = ["opening", "middlegame", "endgame"][phase];
                state.theme = THEMES.iter().position(|(_, key)| *key == theme).unwrap();
                state.tab = 1;
                state.session = None;
                state.linked = false;
                state.synced_board = None;
                state.request(false, theme, DIFFICULTIES[state.difficulty].1);
                launched = true;
                ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("puzzles_open"), true));
            }
            ui.label(RichText::new("Difficulty uses Lichess's anonymous baseline of 1500, rather than your Ironwood rating.")
                .size(12.0).color(Color32::from_rgb(160, 169, 180)));
        });
        Self::paint_sunken_border(ui, controls.response.rect);
        ui.ctx().data_mut(|d| d.insert_temp(state_id, state));
        launched
    }

    fn sync_puzzle_position(&mut self, session: &mut Session) -> Result<(), String> {
        self.transient_profile_review = false;
        if !session.storage_started {
            #[cfg(target_arch = "wasm32")]
            start_new_stored_game("puzzles");
            session.storage_started = true;
        }
        // Board synchronization updates the same attempt rather than importing a new game per move.
        self.load_pgn_with_storage(&session.review_pgn(), false);
        if let Some(error) = self.pgn_error.as_ref() { return Err(error.clone()); }
        self.flipped = session.start.side_to_move() == Color::Black;
        self.review_index = None;
        self.engine_enabled = false;
        self.local_clock = None;
        self.engine_status = "Puzzle: find the best move".into();
        Ok(())
    }
    pub(super) fn puzzles_panel(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let open_id = egui::Id::new("puzzles_open");
        let mut open = ctx.data(|d| d.get_temp::<bool>(open_id)).unwrap_or(false);
        if !open { return; }
        let state_id = egui::Id::new("puzzles_state");
        let mut state = ctx.data(|d| d.get_temp::<State>(state_id)).unwrap_or_default();
        state.poll();
        if state.loading { ctx.request_repaint_after(std::time::Duration::from_millis(150)); }
        let mut close = false;
        egui::ScrollArea::vertical().id_salt("puzzle_panel_scroll")
            .auto_shrink([false, false]).show_gold(ui, |ui| {
                ui.style_mut().text_styles.insert(egui::TextStyle::Body, FontId::proportional(16.0));
                ui.style_mut().text_styles.insert(egui::TextStyle::Button, FontId::proportional(16.0));
                ui.style_mut().text_styles.insert(egui::TextStyle::Small, FontId::proportional(14.0));
                ui.spacing_mut().item_spacing = Vec2::new(12.0, 12.0);
                ui.spacing_mut().button_padding = Vec2::new(12.0, 8.0);
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Puzzles").size(19.0).strong());
                    ui.label("(");
                    if ui.link("by Lichess").on_hover_text("Open Lichess puzzle data and licensing in a new tab").clicked() {
                        ui.ctx().open_url(egui::OpenUrl::new_tab("https://database.lichess.org/#puzzles"));
                    }
                    ui.label(")");
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let (rect, response) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::click());
                        let inset = rect.shrink(6.0);
                        let stroke = ui.style().interact(&response).fg_stroke;
                        ui.painter().line_segment([inset.left_top(), inset.right_bottom()], stroke);
                        ui.painter().line_segment([inset.right_top(), inset.left_bottom()], stroke);
                        close = response.on_hover_text("Close puzzles").clicked();
                    });
                });
                ui.separator();
                let body_rect = egui::Rect::from_min_max(
                    egui::pos2(ui.clip_rect().left(), ui.cursor().top()),
                    ui.clip_rect().right_bottom(),
                );
                ui.painter().rect_filled(body_rect, 0.0, Color32::from_rgb(17, 20, 24));
                let body = Frame::new().fill(Color32::from_rgb(17, 20, 24))
                    .stroke(Stroke::new(1.0, Color32::from_rgb(12, 14, 17)))
                    .corner_radius(6.0).inner_margin(8.0).show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.set_min_height((body_rect.height() - 18.0).max(0.0));
                ui.label("Find the best moves on the live board.");
                let tabs_background = ui.painter().add(egui::Shape::Noop);
                let tabs = ui.horizontal_wrapped(|ui| {
                    for (tab, label) in ["Daily puzzle", "Practice", "Review mistakes", "Progress"].iter().enumerate() {
                        ui.selectable_value(&mut state.tab, tab, *label);
                    }
                });
                ui.painter().set(tabs_background, Self::glass_surface(tabs.response.rect.expand(2.0)));
                ui.separator();
                if state.tab == 0 {
                    if ui.add_enabled(!state.loading, egui::Button::new(RichText::new("Load daily puzzle").strong().color(Color32::from_rgb(30, 26, 20)))
                        .fill(Color32::from_rgb(211, 173, 98))
                        .stroke(Stroke::new(1.0, Color32::from_rgb(241, 206, 131)))
                        .corner_radius(6.0).min_size(Vec2::new(150.0, 40.0)).raised_gold()).clicked() { state.request(true, "mix", "normal"); }
                } else if state.tab == 1 {
                    let gold = Color32::from_rgb(211, 173, 98);
                    let ink = Color32::from_rgb(30, 26, 20);
                    let amber = Color32::from_rgb(232, 188, 114);
                    let training_stats = self.training_profiles.selected()
                        .map(|profile| profile.analysis_totals()).unwrap_or_default();
                    let card = Frame::new().fill(Color32::from_rgb(27, 33, 41))
                        .stroke(Stroke::new(1.0, Color32::from_rgb(49, 59, 71)))
                        .corner_radius(8.0).inner_margin(14.0).show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                    Self::game_moves_glass_label(ui, Vec2::new(ui.available_width(), 28.0), "Puzzle type", true);
                    for (group, indices) in [
                        ("", &[0][..]),
                        ("Tactics", &[1, 2, 3, 4, 5][..]),
                        ("Checkmates", &[6, 7][..]),
                        ("Game phases", &[9, 10, 8][..]),
                    ] {
                        if !group.is_empty() { ui.label(RichText::new(group).small().strong()); }
                        ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
                        for &i in indices {
                            let (name, theme) = THEMES[i];
                            let selected = state.theme == i;
                            let emphasis = training_emphasis(&training_stats, theme);
                            let accent = match emphasis {
                                Some("Recommended phase") => gold,
                                Some(_) => amber,
                                None => Color32::from_rgb(164, 171, 180),
                            };
                            let fill = if selected {
                                if emphasis.is_some() { accent } else { Color32::from_rgb(0, 104, 139) }
                            } else { Color32::from_rgb(27, 33, 41) };
                            let text = if selected && emphasis.is_some() { ink }
                                else if selected { Color32::WHITE } else { accent };
                            let chip = egui::Button::new(RichText::new(name).color(text))
                                .fill(fill)
                                .stroke(Stroke::new(if selected { 2.0 } else { 1.0 }, accent))
                                .corner_radius(16.0)
                                .min_size(Vec2::new(0.0, 32.0));
                            if ui.add(chip).on_hover_text(emphasis.unwrap_or("Practice this puzzle type")).clicked() {
                                state.theme = i;
                            }
                        }
                    });
                    }
                    ui.separator();
                    if let Some(profile) = self.training_profiles.selected() {
                        ui.small(format!("Training profile: {}", profile.name));
                        if let Some(i) = training_stats.practice_focus() {
                            ui.colored_label(gold, format!("Recommended phase: {}", ["Opening", "Middlegame", "Endgames"][i]));
                        }
                        if training_stats.missed_mates > 0 {
                            ui.colored_label(amber, "Mate practice: missed mates in analyzed training games");
                        }
                    }
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.strong("Difficulty").on_hover_text("Difficulty is relative to Lichess's anonymous baseline of 1500, not your Ironwood rating.");
                        ui.label(RichText::new(DIFFICULTIES[state.difficulty].0).color(gold));
                    });
                    ui.scope(|ui| {
                        ui.spacing_mut().slider_width = (ui.available_width() - 12.0).max(120.0);
                        ui.visuals_mut().selection.bg_fill = gold;
                        ui.visuals_mut().widgets.active.bg_fill = gold;
                        ui.visuals_mut().widgets.hovered.bg_fill = gold;
                        ui.add(egui::Slider::new(&mut state.difficulty, 0..=DIFFICULTIES.len() - 1)
                            .integer().show_value(false).step_by(1.0));
                    });
                    ui.horizontal(|ui| {
                        ui.small("Easiest");
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.small("Hardest");
                        });
                    });
                    ui.add_space(4.0);
                    if ui.add_enabled(!state.loading, egui::Button::new(
                        RichText::new("New puzzle").strong().color(ink))
                        .fill(gold).stroke(Stroke::new(1.0, Color32::from_rgb(241, 206, 131)))
                        .corner_radius(6.0).min_size(Vec2::new(150.0, 40.0)).raised_gold()).clicked() {
                        state.request(false, THEMES[state.theme].1, DIFFICULTIES[state.difficulty].1);
                    }
                    });
                    Self::paint_sunken_border(ui, card.response.rect);
                } else if state.tab == 2 {
                    ui.label("Missed or assisted puzzles saved on this device. A clean retry removes them from this list.");
                    let mut retry = None;
                    egui::ScrollArea::vertical().id_salt("puzzle_mistakes").max_height(120.0).show(ui, |ui| {
                        for record in state.history.iter().rev().filter(|r| !r.solved) {
                            if ui.add_enabled(!state.loading, egui::Button::new(
                                RichText::new(format!("Retry {} · {}", record.data.puzzle.id, record.data.puzzle.rating))
                                    .strong().color(Color32::from_rgb(30, 26, 20)))
                                .fill(Color32::from_rgb(211, 173, 98))
                                .stroke(Stroke::new(1.0, Color32::from_rgb(241, 206, 131)))
                                .corner_radius(6.0).min_size(Vec2::new(150.0, 40.0)).raised_gold()).clicked() { retry = Some(record.data.clone()); }
                        }
                        if !state.history.iter().any(|r| !r.solved) { ui.label("No missed puzzles saved yet."); }
                    });
                    if let Some(data) = retry { match Session::new(data) { Ok(session) => state.session = Some(session), Err(error) => state.error = error } }
                } else {
                    let solved = state.history.iter().filter(|r| r.solved).count();
                    ui.heading("Your progress");
                    ui.horizontal_wrapped(|ui| {
                        for (value, label, color) in [
                            (state.history.len(), "Puzzles tracked", Color32::from_rgb(126, 193, 224)),
                            (solved, "Solved cleanly", Color32::from_rgb(136, 205, 158)),
                            (state.history.len() - solved, "To review", Color32::from_rgb(232, 188, 114)),
                        ] {
                            let card = Frame::new().fill(Color32::from_rgb(27, 33, 41))
                                .stroke(Stroke::new(1.0, Color32::from_rgb(49, 59, 71)))
                                .corner_radius(8.0).inner_margin(14.0).show(ui, |ui| {
                                    ui.set_min_width(145.0);
                                    ui.label(egui::RichText::new(value.to_string()).size(30.0).strong().color(color));
                                    ui.label(label);
                                });
                    Self::paint_sunken_border(ui, card.response.rect);
                        }
                    });
                    ui.add_space(4.0);
                    ui.strong("Results by puzzle type");
                    let profile = self.training_profiles.selected();
                    let training_stats = profile.map(|p| p.analysis_totals()).unwrap_or_default();
                    let gold = Color32::from_rgb(211, 173, 98);
                    let amber = Color32::from_rgb(232, 188, 114);
                    if let Some(profile) = profile {
                        ui.small(format!("Training profile: {}", profile.name));
                        if let Some(i) = training_stats.practice_focus() {
                            ui.colored_label(gold, format!("Recommended phase: {}", ["Opening", "Middlegame", "Endgames"][i]));
                        } else {
                            ui.small("No phase focus yet: analyze at least 20 numerical move evaluations in a phase with a mistake or blunder.");
                        }
                        if training_stats.missed_mates > 0 {
                            ui.colored_label(amber, format!("Mate practice: {} missed mates in analyzed training games", training_stats.missed_mates));
                        }
                    } else {
                        ui.small("Select a training profile to highlight recommended practice areas.");
                    }

                    let card = Frame::new().fill(Color32::from_rgb(27, 33, 41))
                        .stroke(Stroke::new(1.0, Color32::from_rgb(49, 59, 71)))
                        .corner_radius(8.0).inner_margin(14.0).show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                    egui::Grid::new("puzzle_theme_progress").num_columns(4)
                        .striped(true).spacing(Vec2::new(16.0, 8.0)).show(ui, |ui| {
                            let header_background = ui.painter().add(egui::Shape::Noop);
                            let mut header_rect = ui.strong("Type").rect;
                            for label in ["Solved cleanly", "To review", "Total tracked"] {
                                header_rect = header_rect.union(ui.strong(label).rect);
                            }
                            ui.painter().set(header_background, Self::glass_surface(header_rect.expand(3.0)));
                            ui.end_row();
                            for &(name, theme) in THEMES {
                                let (solved, review) = state.theme_results(theme);
                                let emphasis = training_emphasis(&training_stats, theme);
                                let color = match emphasis {
                                    Some("Recommended phase") => gold,
                                    Some(_) => amber,
                                    None => ui.visuals().text_color(),
                                };
                                let label = if theme == "mix" { "Mixed (all puzzles)" } else { name };
                                ui.label(RichText::new(label).color(color))
                                    .on_hover_text(emphasis.unwrap_or("No specific training recommendation for this type."));
                                ui.colored_label(color, solved.to_string());
                                ui.colored_label(color, review.to_string());
                                ui.colored_label(color, (solved + review).to_string());
                                ui.end_row();
                            }
                        });
                        });
                    Self::paint_sunken_border(ui, card.response.rect);
                    ui.small("Types use each puzzle's Lichess tags, including puzzles fetched through Mixed. A puzzle can count toward several types; Mixed counts each puzzle once.");
                    ui.add_space(4.0);
                    ui.strong("How results are counted");
                    ui.label("A wrong move, hint, or revealed solution adds a puzzle to Review mistakes. Solve it cleanly on a retry to update its result.");
                    ui.small("Saved on this device: the latest result for up to 500 puzzles. Results are not submitted to Lichess.");
                }
                if state.loading { ui.spinner(); ui.label("Loading from Lichess..."); }
                if !state.error.is_empty() { ui.colored_label(Color32::LIGHT_RED, &state.error); }
                if !state.storage_error.is_empty() { ui.colored_label(Color32::LIGHT_RED, &state.storage_error); }
                let mut result = None;
                if let Some(session) = state.session.as_mut().filter(|_| state.tab != 3) {
                    ui.separator();
                    ui.horizontal_wrapped(|ui| {
                        ui.strong(format!("Puzzle {} · {} to move", session.data.puzzle.id, if session.board.side_to_move() == Color::White { "White" } else { "Black" }));
                        if session.done { ui.label(format!("Rating {} · {}", session.data.puzzle.rating, session.data.puzzle.themes.join(", "))); }
                    });
                    ui.label(&session.message);
                    ui.horizontal(|ui| {
                        if !session.done {
                            if ui.add_enabled(!state.loading, egui::Button::new(RichText::new("Hint").strong().color(Color32::from_rgb(30, 26, 20)))
                                .fill(Color32::from_rgb(211, 173, 98))
                                .stroke(Stroke::new(1.0, Color32::from_rgb(241, 206, 131)))
                                .corner_radius(6.0).min_size(Vec2::new(110.0, 40.0)).raised_gold()).clicked() {
                                if let Some(mv) = session.moves.get(session.step) { session.message = format!("Look at the piece on {}.", mv.get_source()); result = Some(false); }
                            }
                            if ui.add_enabled(!state.loading, egui::Button::new(RichText::new("Show solution").strong().color(Color32::from_rgb(30, 26, 20)))
                                .fill(Color32::from_rgb(211, 173, 98))
                                .stroke(Stroke::new(1.0, Color32::from_rgb(241, 206, 131)))
                                .corner_radius(6.0).min_size(Vec2::new(110.0, 40.0)).raised_gold()).clicked() { session.reveal(); result = Some(false); }
                        }

                    });
                }
                if let Some(solved) = result { state.record(solved); }
                });
                Self::paint_sunken_border(ui, body.response.rect);
            });
        if close { open = false; }
        if let Some(session) = state.session.as_mut() {
            if state.synced_board != Some(session.board) {
                if !state.linked { self.save_game(); }
                if self.fics_active { self.stop_fics(); }
                match self.sync_puzzle_position(session) {
                    Ok(()) => { state.linked = true; state.synced_board = Some(session.board); }
                    Err(error) => state.error = format!("Could not sync puzzle: {error}"),
                }
            }
        }
        ctx.data_mut(|d| { d.insert_temp(open_id, open); d.insert_temp(state_id, state); });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn starting_a_normal_game_exits_puzzles_and_routes_moves_to_the_game() {
        for side in [PlayerSide::White, PlayerSide::Black] {
            let context = egui::Context::default();
            let mut app = ChessApp::new(&eframe::CreationContext::_new_kittest(context.clone()));
            let mut state = State::default();
            state.session = Some(Session::new(example()).unwrap());
            state.record(false);
            state.loading = true;
            state.linked = true;
            context.data_mut(|data| {
                data.insert_temp(egui::Id::new("puzzles_open"), true);
                data.insert_temp(egui::Id::new("puzzles_state"), state);
            });
            app.reset_for_side(side);
            assert!(!app.puzzle_mode());
            assert!(!app.puzzle_navigation_locked());
            let state = context.data(|data| data.get_temp::<State>(egui::Id::new("puzzles_state"))).unwrap();
            assert!(state.session.is_none());
            assert!(!state.loading && !state.linked);
            assert_eq!(state.history.len(), 1);
            app.engine_enabled = false;
            app.play(ChessMove::new(Square::E2, Square::E4, None));
            assert_eq!(app.board.piece_on(Square::E4), Some(Piece::Pawn));
            assert_eq!(app.review_moves, vec!["e4"]);
        }
    }

    #[test]
    fn training_colors_follow_supported_profile_evidence() {
        let mut stats = crate::training::AnalysisStats::default();
        assert_eq!(training_emphasis(&stats, "opening"), None);
        stats.phases[0].cp_moves = 19;
        stats.phases[0].loss_cp = 5000;
        stats.phases[0].mistakes = 1;
        assert_eq!(training_emphasis(&stats, "opening"), None);
        stats.phases[0].cp_moves = 20;
        assert_eq!(training_emphasis(&stats, "opening"), Some("Recommended phase"));
        assert_eq!(training_emphasis(&stats, "fork"), None);
        assert_eq!(training_emphasis(&stats, "mix"), None);
        stats.missed_mates = 1;
        assert_eq!(training_emphasis(&stats, "mateIn1"), Some("Mate practice"));
        assert_eq!(training_emphasis(&stats, "mateIn2"), Some("Mate practice"));
    }
    #[test]
    fn theme_progress_includes_all_tags_and_zero_result_types() {
        let mut state = State::default();
        let mut data = example();
        data.puzzle.themes = vec!["fork".into(), "middlegame".into()];
        state.history.push(Record { data: data.clone(), solved: true });
        data.puzzle.themes = vec!["fork".into()];
        state.history.push(Record { data, solved: false });
        assert_eq!(state.theme_results("mix"), (1, 1));
        assert_eq!(state.theme_results("fork"), (1, 1));
        assert_eq!(state.theme_results("middlegame"), (1, 0));
        assert_eq!(state.theme_results("pin"), (0, 0));
    }
    #[test]
    fn live_board_attempts_validate_and_apply_automatic_replies() {
        let context = egui::Context::default();
        let mut app = ChessApp::new(&eframe::CreationContext::_new_kittest(context.clone()));
        let mut session = Session::new(example()).unwrap();
        let start = session.start;
        app.sync_puzzle_position(&mut session).unwrap();
        context.data_mut(|d| {
            d.insert_temp(egui::Id::new("puzzles_open"), true);
            d.insert_temp(egui::Id::new("puzzles_state"), State { session: Some(session), ..State::default() });
        });
        let wrong = MoveGen::new_legal(&start).find(|mv| mv.to_string() != "f5f3").unwrap();
        app.play(wrong);
        assert_eq!(app.board, start);
        app.select(Square::F5);
        assert_eq!(app.selected, Some(Square::F5));
        assert!(app.puzzle_mode());
        app.select(Square::F3);
        assert_eq!(app.board.king_square(Color::White), Square::F3);
        assert_eq!(app.review_moves, vec!["Qxf3+", "Kxf3"]);
        assert!(app.review_index.is_none());
        app.review_to(0);
        assert_eq!(app.board.king_square(Color::White), Square::F3);
        app.play(ChessMove::from_str("c6e5").unwrap());
        app.play(ChessMove::from_str("e5c4").unwrap());
        app.review_to(0);
        assert_eq!(app.board, start);
        app.review_to(app.review_moves.len());
        assert_eq!(app.review_index, Some(app.review_moves.len()));
        let state = context.data(|d| d.get_temp::<State>(egui::Id::new("puzzles_state"))).unwrap();
        assert!(state.session.unwrap().done);
        assert_eq!(state.history.len(), 1);
        assert!(!state.history[0].solved);
    }
    #[test]
    fn solving_and_revealing_sync_exact_positions_to_main_board() {
        let mut session = Session::new(example()).unwrap();
        let mut app = ChessApp::new(&eframe::CreationContext::_new_kittest(egui::Context::default()));
        app.sync_puzzle_position(&mut session).unwrap();
        session.submit(ChessMove::from_str("f5f3").unwrap());
        app.sync_puzzle_position(&mut session).unwrap();
        assert_eq!(app.board, session.board);
        assert_eq!(app.board.king_square(Color::White), Square::F3);
        assert_eq!(app.review_moves, vec!["Qxf3+", "Kxf3"]);
        assert_eq!(app.review_positions.len(), 3);
        app.review_index = Some(0);
        assert_eq!(app.review_positions[0], session.start);
        session.reveal();
        app.sync_puzzle_position(&mut session).unwrap();
        assert_eq!(app.board, session.board);
        assert_eq!(app.review_positions.len(), session.played.len() + 1);
        assert_eq!(app.review_moves.len(), session.played.len());
        assert!(session.storage_started);
        assert_eq!(ChessApp::pgn_tag(&app.pgn_input,"PuzzleId"),Some(session.data.puzzle.id.clone()));
        assert!(!Session::new(example()).unwrap().storage_started);
        assert_eq!(ChessApp::pgn_tag(&app.annotated_pgn(),"PuzzleId"),Some(session.data.puzzle.id.clone()));
    }
    #[test]
    fn puzzle_position_loads_into_main_board_without_solution_moves() {
        let board = Session::new(example()).unwrap().board;
        let mut app = ChessApp::new(&eframe::CreationContext::_new_kittest(egui::Context::default()));
        app.load_pgn(&format!("[Event \"Lichess puzzle\"]\n[SetUp \"1\"]\n[FEN \"{board}\"]\n[Result \"*\"]\n\n*"));
        assert!(app.pgn_error.is_none());
        assert_eq!(app.board, board);
        assert_eq!(app.review_positions, vec![board]);
        assert_eq!(app.review_index, Some(0));
        assert!(app.review_moves.is_empty());
        assert!(!app.engine_searching);
        assert!(ChessApp::parse_pgn_mainline("*").is_err());
    }
    fn example() -> PuzzleData {
        serde_json::from_str(r#"{"game":{"pgn":"d4 d5 Nf3 e5 dxe5 f6 exf6 Nxf6 g3 Bd6 Bg2 Bg4 O-O Nc6 c4 Qd7 Qc2 dxc4 Rd1 O-O-O Bf4 Bh3 Bxd6 cxd6 Qxc4 Bxg2 Kxg2 Kb8 Nc3 Qf5 e4"},"puzzle":{"id":"QYQYF","rating":1703,"initialPly":30,"solution":["f5f3","g2f3","c6e5","f3g2","e5c4"],"themes":["fork"]}}"#).unwrap()
    }
    #[test]
    fn source_ply_and_automatic_replies_match_lichess_example() {
        let mut session = Session::new(example()).unwrap();
        assert_eq!(session.board.side_to_move(), Color::Black);
        assert_eq!(session.board.piece_on(Square::E4), Some(Piece::Pawn));
        assert_eq!(session.submit(ChessMove::from_str("f5f3").unwrap()), None);
        assert_eq!(session.board.king_square(Color::White), Square::F3);
        assert_eq!(session.step, 2);
        assert_eq!(session.submit(ChessMove::from_str("c6e5").unwrap()), None);
        assert_eq!(session.submit(ChessMove::from_str("e5c4").unwrap()), Some(true));
        assert!(session.done);
    }
    #[test]
    fn wrong_moves_do_not_change_position_and_results_are_recorded_once() {
        let mut state = State::default();
        state.session = Some(Session::new(example()).unwrap());
        let session = state.session.as_mut().unwrap();
        let before = session.board;
        let wrong = MoveGen::new_legal(&before).find(|mv| *mv != session.moves[0]).unwrap();
        assert_eq!(session.submit(wrong), Some(false));
        assert_eq!(session.board, before);
        assert_eq!(session.step, 0);
        assert!(session.played.is_empty());
        state.record(false);
        state.record(true);
        assert_eq!(state.history.len(), 1);
        assert!(!state.history[0].solved);
        state.session = Some(Session::new(example()).unwrap());
        state.record(true);
        assert_eq!(state.history.len(), 1);
        assert!(state.history[0].solved);
    }
    #[test]
    fn malformed_solutions_are_rejected_and_reveal_finishes_without_playing_game() {
        let mut data = example();
        data.puzzle.solution[0] = "a1a8".into();
        assert!(Session::new(data).is_err());
        let mut data = example(); data.puzzle.initial_ply = usize::MAX;
        assert!(Session::new(data).is_err());
        let mut session = Session::new(example()).unwrap();
        session.reveal();
        assert!(session.done);
        assert_ne!(session.board, session.start);
        assert_eq!(session.submit(session.moves[0]), None);
    }
}
