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
            promotion: None, recorded: false, done: false, message: "Find the best move.".into() })
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
        let mut pgn = format!("[Event \"Lichess puzzle\"]\n[SetUp \"1\"]\n[FEN \"{}\"]\n[Result \"*\"]\n\n", self.start);
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
    collapsed: bool,
}
impl Default for State {
    fn default() -> Self {
        let history = serde_json::from_str::<Vec<Record>>(&bridge("history", "")).unwrap_or_default();
        Self { tab: 0, theme: 0, difficulty: 2, session: None, history, loading: false, error: String::new(), storage_error: String::new(), linked: false, synced_board: None, collapsed: false }
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

impl ChessApp {
    pub(super) fn phase_practice_ui(ui: &mut egui::Ui, phase: usize, enabled: bool) -> bool {
        let mut launched = false;
        let state_id = egui::Id::new("puzzles_state");
        let mut state = ui.ctx().data(|d| d.get_temp::<State>(state_id)).unwrap_or_default();
        ui.horizontal_wrapped(|ui| {
            ui.label("Puzzle difficulty");
            egui::ComboBox::from_id_salt("phase_puzzle_difficulty")
                .selected_text(DIFFICULTIES[state.difficulty].0).show_ui(ui, |ui| {
                    for (i, (name, _)) in DIFFICULTIES.iter().enumerate() {
                        ui.selectable_value(&mut state.difficulty, i, *name);
                    }
                });
            if ui.add_enabled(enabled && !state.loading, egui::Button::new("Practice this phase"))
                .on_disabled_hover_text("Finish active training or online play, and wait for any puzzle request to complete.").clicked() {
                let theme = ["opening", "middlegame", "endgame"][phase];
                state.theme = THEMES.iter().position(|(_, key)| *key == theme).unwrap();
                state.tab = 1;
                state.collapsed = false;
                state.session = None;
                state.linked = false;
                state.synced_board = None;
                state.request(false, theme, DIFFICULTIES[state.difficulty].1);
                launched = true;
                ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("puzzles_open"), true));
            }
        });
        ui.small("Difficulty is relative to Lichess's anonymous baseline of 1500, not your Ironwood rating.");
        ui.ctx().data_mut(|d| d.insert_temp(state_id, state));
        launched
    }

    fn sync_puzzle_position(&mut self, session: &Session) -> Result<(), String> {
        self.load_pgn(&session.review_pgn());
        if let Some(error) = self.pgn_error.as_ref() { return Err(error.clone()); }
        self.flipped = session.start.side_to_move() == Color::Black;
        self.engine_status = "Puzzle position synced · ready to analyze".into();
        Ok(())
    }
    pub(super) fn puzzles_window(&mut self, ctx: &egui::Context) {
        let open_id = egui::Id::new("puzzles_open");
        let mut open = ctx.data(|d| d.get_temp::<bool>(open_id)).unwrap_or(false);
        if !open { return; }
        let state_id = egui::Id::new("puzzles_state");
        let mut state = ctx.data(|d| d.get_temp::<State>(state_id)).unwrap_or_default();
        state.poll();
        if state.loading { ctx.request_repaint_after(std::time::Duration::from_millis(150)); }
        let mut close = false;
        let mut load_position = None;
        egui::Window::new("Puzzles").id(egui::Id::new("puzzles_window")).open(&mut open).title_bar(false)
            .default_width(650.0).resizable(true)
            .frame(Frame::window(&ctx.style()).fill(Color32::from_rgb(17, 20, 24)).inner_margin(16.0))
            .show(ctx, |ui| {
                ui.style_mut().text_styles.insert(egui::TextStyle::Body, FontId::proportional(16.0));
                ui.style_mut().text_styles.insert(egui::TextStyle::Button, FontId::proportional(16.0));
                ui.style_mut().text_styles.insert(egui::TextStyle::Small, FontId::proportional(14.0));
                ui.spacing_mut().item_spacing = Vec2::new(12.0, 12.0);
                ui.spacing_mut().button_padding = Vec2::new(12.0, 8.0);
                ui.horizontal(|ui| {
                    let (_, toggle) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::click());
                    let openness = ui.ctx().animate_bool(egui::Id::new("puzzles_expanded"), !state.collapsed);
                    let mut icon = toggle.clone();
                    icon.rect = egui::Rect::from_center_size(toggle.rect.center(), Vec2::splat(ui.spacing().icon_width));
                    egui::collapsing_header::paint_default_icon(ui, openness, &icon);
                    if toggle.on_hover_text(if state.collapsed { "Expand puzzles" } else { "Collapse puzzles" }).clicked() {
                        state.collapsed = !state.collapsed;
                    }
                    if ui.add(egui::Label::new(RichText::new("Puzzles").heading()).sense(Sense::click())).clicked() {
                        state.collapsed = !state.collapsed;
                    }
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
                if state.collapsed { return; }
                ui.separator();
                ui.label(if state.linked { "Linked to main board · solve here to update both boards." } else { "Practice on a separate board. Your current game stays in place." });
                ui.horizontal_wrapped(|ui| {
                    for (tab, label) in ["Daily puzzle", "Practice", "Review mistakes", "Progress"].iter().enumerate() {
                        ui.selectable_value(&mut state.tab, tab, *label);
                    }
                });
                ui.separator();
                if state.tab == 0 {
                    if ui.add_enabled(!state.loading, egui::Button::new("Load daily puzzle")).clicked() { state.request(true, "mix", "normal"); }
                } else if state.tab == 1 {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().interact_size.y = 36.0;
                        egui::ComboBox::from_id_salt("puzzle_theme").selected_text(THEMES[state.theme].0).show_ui(ui, |ui| {
                            for (i, (name, _)) in THEMES.iter().enumerate() { ui.selectable_value(&mut state.theme, i, *name); }
                        });
                        egui::ComboBox::from_id_salt("puzzle_difficulty").selected_text(DIFFICULTIES[state.difficulty].0).show_ui(ui, |ui| {
                            for (i, (name, _)) in DIFFICULTIES.iter().enumerate() { ui.selectable_value(&mut state.difficulty, i, *name); }
                        });
                        if ui.add_enabled(!state.loading, egui::Button::new("New puzzle")).clicked() {
                            state.request(false, THEMES[state.theme].1, DIFFICULTIES[state.difficulty].1);
                        }
                    });
                    ui.small("Difficulty is relative to Lichess's anonymous baseline of 1500, not your Ironwood rating.");
                } else if state.tab == 2 {
                    ui.label("Missed or assisted puzzles saved on this device. A clean retry removes them from this list.");
                    let mut retry = None;
                    egui::ScrollArea::vertical().id_salt("puzzle_mistakes").max_height(120.0).show(ui, |ui| {
                        for record in state.history.iter().rev().filter(|r| !r.solved) {
                            if ui.button(format!("Retry {} · {}", record.data.puzzle.id, record.data.puzzle.rating)).clicked() { retry = Some(record.data.clone()); }
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
                            Frame::new().fill(Color32::from_rgb(27, 33, 41))
                                .stroke(Stroke::new(1.0, Color32::from_rgb(49, 59, 71)))
                                .corner_radius(8.0).inner_margin(14.0).show(ui, |ui| {
                                    ui.set_min_width(145.0);
                                    ui.label(egui::RichText::new(value.to_string()).size(30.0).strong().color(color));
                                    ui.label(label);
                                });
                        }
                    });
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
                    let size = ui.available_width().min((ctx.screen_rect().height() - 420.0).max(240.0)).min(520.0);
                    let cell = size / 8.0;
                    let flipped = session.start.side_to_move() == Color::Black;
                    let mut clicked = None;
                    ui.spacing_mut().item_spacing = Vec2::ZERO;
                    for row in 0..8 {
                        ui.horizontal(|ui| {
                            for col in 0..8 {
                                let rank = if flipped { row } else { 7 - row };
                                let file = if flipped { 7 - col } else { col };
                                let square = Square::make_square(Rank::from_index(rank), File::from_index(file));
                                let mut color = if (file + rank) % 2 == 1 { Color32::from_rgb(207, 216, 190) } else { Color32::from_rgb(76, 111, 89) };
                                if session.selected == Some(square) { color = Color32::from_rgb(211, 173, 98); }
                                let (rect, response) = ui.allocate_exact_size(Vec2::splat(cell), Sense::click());
                                ui.painter().rect_filled(rect, 0.0, color);
                                if let Some((piece, side)) = session.board.piece_on(square).zip(session.board.color_on(square)) {
                                    Self::paint_piece(ui, rect, cell, PieceSet::Merida, side, piece);
                                }
                                if session.selected.is_some_and(|source| MoveGen::new_legal(&session.board).any(|mv| mv.get_source() == source && mv.get_dest() == square)) {
                                    ui.painter().circle_filled(rect.center(), cell * 0.07, Color32::from_black_alpha(110));
                                }
                                if col == 0 { ui.painter().text(rect.left_top() + Vec2::splat(3.0), Align2::LEFT_TOP, rank + 1, FontId::proportional(11.0), Color32::BLACK); }
                                if row == 7 { ui.painter().text(rect.right_bottom() - Vec2::splat(3.0), Align2::RIGHT_BOTTOM, (b'a' + file as u8) as char, FontId::proportional(11.0), Color32::BLACK); }
                                if response.clicked() && !session.done && !state.loading { clicked = Some(square); }
                            }
                        });
                    }
                    ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
                    if let Some(square) = clicked {
                        if let Some(source) = session.selected {
                            let legal: Vec<_> = MoveGen::new_legal(&session.board).filter(|mv| mv.get_source() == source && mv.get_dest() == square).collect();
                            if legal.iter().any(|mv| mv.get_promotion().is_some()) { session.promotion = Some((source, square)); }
                            else if let Some(&mv) = legal.first() { result = session.submit(mv); }
                            else { session.selected = session.board.color_on(square).filter(|c| *c == session.board.side_to_move()).map(|_| square); session.promotion = None; }
                        } else if session.board.color_on(square) == Some(session.board.side_to_move()) { session.selected = Some(square); }
                    }
                    if let Some((source, target)) = session.promotion {
                        ui.horizontal(|ui| {
                            ui.label("Promote to:");
                            for (piece, name) in [(Piece::Queen, "Queen"), (Piece::Rook, "Rook"), (Piece::Bishop, "Bishop"), (Piece::Knight, "Knight")] {
                                if ui.button(name).clicked() { result = session.submit(ChessMove::new(source, target, Some(piece))); }
                            }
                        });
                    }
                    ui.label(&session.message);
                    ui.horizontal(|ui| {
                        if !session.done {
                            if ui.add_enabled(!state.loading, egui::Button::new("Hint")).clicked() {
                                if let Some(mv) = session.moves.get(session.step) { session.message = format!("Look at the piece on {}.", mv.get_source()); result = Some(false); }
                            }
                            if ui.add_enabled(!state.loading, egui::Button::new("Show solution")).clicked() { session.reveal(); result = Some(false); }
                        }
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let online_playing = self.fics_active && self.fics_playing
                                && !self.fics_observing && !self.fics_game_finished;
                            let enabled = !state.loading && !self.training_live() && !online_playing;
                            if ui.add_enabled(enabled, egui::Button::new("Load into IRONWOOD"))
                                .on_hover_text(if online_playing { "Finish the active online game before loading a puzzle" }
                                    else { "Open the displayed puzzle position on the main board for analysis" })
                                .clicked() { load_position = Some(session.board); }
                        });
                    });
                }
                if let Some(solved) = result { state.record(solved); }
            });
        if close { open = false; }
        if let Some(board) = load_position {
            self.save_game();
            if self.fics_active { self.stop_fics(); }
            if let Some(session) = state.session.as_ref() { let _ = self.sync_puzzle_position(session); }
            if let Some(error) = self.pgn_error.as_ref() {
                state.error = format!("Could not load puzzle: {error}");
            } else {
                self.flipped = board.side_to_move() == Color::Black;
                self.engine_status = "Puzzle position loaded · ready to analyze".into();
                state.linked = true;
                state.synced_board = Some(board);
            }
        }
        if state.linked && !self.training_live() && !(self.fics_active && self.fics_playing && !self.fics_game_finished) {
            if let Some(board) = state.session.as_ref().map(|session| session.board)
                .filter(|board| Some(*board) != state.synced_board) {
                match self.sync_puzzle_position(state.session.as_ref().unwrap()) {
                    Ok(()) => state.synced_board = Some(board),
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
    fn solving_and_revealing_sync_exact_positions_to_main_board() {
        let mut session = Session::new(example()).unwrap();
        let mut app = ChessApp::new(&eframe::CreationContext::_new_kittest(egui::Context::default()));
        app.sync_puzzle_position(&session).unwrap();
        session.submit(ChessMove::from_str("f5f3").unwrap());
        app.sync_puzzle_position(&session).unwrap();
        assert_eq!(app.board, session.board);
        assert_eq!(app.board.king_square(Color::White), Square::F3);
        assert_eq!(app.review_moves, vec!["Qxf3+", "Kxf3"]);
        assert_eq!(app.review_positions.len(), 3);
        app.review_index = Some(0);
        assert_eq!(app.review_positions[0], session.start);
        session.reveal();
        app.sync_puzzle_position(&session).unwrap();
        assert_eq!(app.board, session.board);
        assert_eq!(app.review_positions.len(), session.played.len() + 1);
        assert_eq!(app.review_moves.len(), session.played.len());
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
