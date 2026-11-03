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

#[derive(Clone,Deserialize,Serialize)]
pub(super) struct PuzzleSnapshot {
    data: PuzzleData,
    played: Vec<String>,
    done: bool,
    recorded: bool,
}
impl PuzzleSnapshot {
    fn from_session(session:&Session)->Self {
        Self{data:session.data.clone(),played:session.played.iter().map(ToString::to_string).collect(),done:session.done,recorded:session.recorded}
    }
    fn restore(&self)->Result<Session,String> {
        let mut session=Session::new(self.data.clone())?;
        if self.played.len()>session.moves.len(){return Err("Invalid saved puzzle progress.".into());}
        for (i,value) in self.played.iter().enumerate() {
            let mv=ChessMove::from_str(value).map_err(|_|"Invalid saved puzzle move")?;
            if !MoveGen::new_legal(&session.board).any(|legal|legal==mv) {return Err("Illegal saved puzzle move.".into());}
            let next=session.board.make_move_new(mv);
            if session.moves.get(i)!=Some(&mv)&&next.status()!=BoardStatus::Checkmate{return Err("Saved puzzle move does not match its solution.".into());}
            session.board=next;session.played.push(mv);
        }
        session.step=session.played.len();
        session.done=self.done||session.step==session.moves.len();
        session.recorded=self.recorded;session.storage_started=true;
        session.message=if session.done{"Saved puzzle · solution complete."}else{"Saved puzzle · find the best move."}.into();
        Ok(session)
    }
}
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
    restore_board: Option<Board>,
}
impl Default for State {
    fn default() -> Self {
        let history = serde_json::from_str::<Vec<Record>>(&bridge("history", "")).unwrap_or_default();
        Self { tab: 0, theme: 0, difficulty: 2, session: None, history, loading: false, error: String::new(), storage_error: String::new(), linked: false, synced_board: None,restore_board:None }
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
                    Ok(mut session) => {
                        if let Some(target)=self.restore_board.take() {
                            while session.board!=target&&session.played.len()<session.moves.len() {
                                let mv=session.moves[session.played.len()];session.board=session.board.make_move_new(mv);session.played.push(mv);
                            }
                            if session.board!=target {self.error="Saved position does not match this puzzle. The saved annotations were kept.".into();return;}
                            session.step=session.played.len();session.done=session.step==session.moves.len();session.storage_started=true;
                            self.synced_board=Some(target);self.linked=true;
                        }
                        self.session = Some(session);
                    }
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
    pub(super) fn puzzle_snapshot(&self)->Option<PuzzleSnapshot> {
        if !self.puzzle_mode(){return None;}
        self.puzzle_context.data(|d| {
            let snapshot=d.get_temp::<PuzzleSnapshot>(egui::Id::new("puzzle_save_snapshot"))?;
            if let Some(session)=d.get_temp::<State>(egui::Id::new("puzzles_state")).and_then(|state|state.session)
                .filter(|session|session.data.puzzle.id==snapshot.data.puzzle.id&&session.played.len()>=snapshot.played.len()) {
                Some(PuzzleSnapshot::from_session(&session))
            } else {Some(snapshot)}
        })
    }

    pub(super) fn restore_saved_puzzle(&mut self,snapshot:Option<&PuzzleSnapshot>) {
        let id=Self::pgn_tag(&self.pgn_input,"PuzzleId");
        if snapshot.is_none()&&id.is_none(){return;}
        self.study=None;
        let ctx=self.puzzle_context.clone();let mut state=State::default();
        if let Some(snapshot)=snapshot {
            match snapshot.restore() {
                Ok(session)=>{self.board=session.board;self.review_index=None;state.synced_board=Some(session.board);state.linked=true;
                    ctx.data_mut(|d|{d.insert_temp(egui::Id::new("puzzle_save_snapshot"),snapshot.clone());});state.session=Some(session);}
                Err(error)=>state.error=error,
            }
        } else if let Some(id)=id {
            if let Some(record)=state.history.iter().find(|record|record.data.puzzle.id==id) {
                let data=record.data.clone();state.restore_board=Some(self.board);state.loading=true;
                // Reuse the same response validation path for older saved records.
                let mut session=Session::new(data).ok();
                if let Some(session)=session.as_mut() {
                    while session.board!=self.board&&session.played.len()<session.moves.len(){let mv=session.moves[session.played.len()];session.board=session.board.make_move_new(mv);session.played.push(mv);}
                    if session.board==self.board {
                        session.step=session.played.len();session.done=session.step==session.moves.len();session.storage_started=true;
                        state.loading=false;state.restore_board=None;state.synced_board=Some(self.board);state.linked=true;
                        ctx.data_mut(|d|{d.insert_temp(egui::Id::new("puzzle_save_snapshot"),PuzzleSnapshot::from_session(session));});
                        state.session=session.clone().into();
                    }
                }
                if state.loading {bridge("request",&serde_json::json!({"id":id}).to_string());}
            } else {state.loading=true;state.restore_board=Some(self.board);bridge("request",&serde_json::json!({"id":id}).to_string());}
        }
        self.engine_enabled=false;self.analysis_running=false;self.realtime_analysis_due_at=None;
        self.game_analysis_running=false;self.game_analysis_paused=false;self.resume_game_analysis_after_ready=false;
        self.navigation_enter_puzzles(&ctx);
        ctx.data_mut(|d|{d.insert_temp(egui::Id::new("puzzles_state"),state);});
    }
    pub(super) fn puzzle_mode(&self) -> bool {
        self.puzzle_context.data(|d| d.get_temp::<bool>(egui::Id::new("puzzles_open"))).unwrap_or(false)
    }

    pub(super) fn exit_puzzle_mode(&mut self) {
        self.puzzle_context.data_mut(|data| {
            data.insert_temp(egui::Id::new("puzzles_open"), false);
            data.remove::<PuzzleSnapshot>(egui::Id::new("puzzle_save_snapshot"));
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
        self.study=None;
        // Keep local annotations when advancing within this same puzzle attempt.
        let annotations=if session.storage_started {
            self.review_positions.iter().enumerate().map(|(i,board)|(*board,
                self.move_notes.get(i).cloned().unwrap_or_default(),
                self.board_marks.get(i).cloned().unwrap_or_default(),
                self.move_annotations.get(i).cloned().unwrap_or_default())).collect::<Vec<_>>()
        }else{Vec::new()};
        if !session.storage_started {
            #[cfg(target_arch = "wasm32")]
            start_new_stored_game("puzzles");
            session.storage_started = true;
        }
        // Board synchronization updates the same attempt rather than importing a new game per move.
        self.load_pgn_with_storage(&session.review_pgn(), false);
        if let Some(error) = self.pgn_error.as_ref() { return Err(error.clone()); }
        for (i,board) in self.review_positions.iter().enumerate() {
            if let Some((_,note,marks,nags))=annotations.get(i).filter(|(previous,_,_,_)|previous==board) {
                if let Some(value)=self.move_notes.get_mut(i){*value=note.clone();}
                if let Some(value)=self.board_marks.get_mut(i){*value=marks.clone();}
                if let Some(value)=self.move_annotations.get_mut(i){*value=nags.clone();}
            }
        }
        self.flipped = session.start.side_to_move() == Color::Black;
        self.review_index = None;
        self.engine_enabled = false;
        self.local_clock = None;
        self.engine_status = "Puzzle: find the best move".into();
        self.puzzle_context.data_mut(|d|{d.insert_temp(egui::Id::new("puzzle_save_snapshot"),PuzzleSnapshot::from_session(session));});
        self.save_game();
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
        if let Some(session)=state.session.as_ref() {
            ctx.data_mut(|d|{d.insert_temp(egui::Id::new("puzzle_save_snapshot"),PuzzleSnapshot::from_session(session));});
        }
        if state.loading { ctx.request_repaint_after(std::time::Duration::from_millis(150)); }
        let mut close = false;
        ui.horizontal(|ui| {
            ui.label(RichText::new("Puzzles").size(19.0).strong());
            ui.with_layout(Layout::right_to_left(Align::Center),|ui| {close=ui.button("Close").on_hover_text("Close puzzles").clicked();});
        });
        ui.add_space(8.0);
        if self.workspace_annotations_tabs(ui,false) {
            if state.session.is_some(){self.annotations_panel(ui);}
            else {ui.label("Load a puzzle to add drawings, notes, and chess symbols.");}
        } else {
        egui::ScrollArea::vertical().id_salt("puzzle_panel_scroll")
            .auto_shrink([false, false]).show_gold(ui, |ui| {
                Self::stable_panel_button_style(ui);
                ui.spacing_mut().item_spacing = Vec2::new(8.0, 12.0);
                ui.spacing_mut().button_padding = Vec2::new(12.0, 7.0);
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let gold = Color32::from_rgb(215,185,121);
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(32.0), Sense::hover());
                    ui.painter().circle_stroke(rect.center(), 11.0, Stroke::new(1.5,gold));
                    ui.painter().circle_stroke(rect.center(), 5.0, Stroke::new(1.5,gold));
                    ui.vertical(|ui| {
                        ui.label(RichText::new("TRAINING").size(10.0).color(gold));
                        ui.label(RichText::new("Lichess puzzles").size(21.0).strong());
                    });
                });
                ui.label(RichText::new("Find the best continuation. Play your moves on the board.").weak());
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::splat(6.0);
                    for (tab, label) in ["Daily puzzle", "Practice", "Review mistakes", "Progress"].iter().enumerate() {
                        if Self::compact_panel_button(ui,label,state.tab == tab).clicked() { state.tab = tab; }
                    }
                });
                ui.add_space(4.0);
                Self::grained_outer_section(ui, |ui| {
                if state.tab == 0 {
                    ui.label(RichText::new("Your daily challenge").size(17.0).strong());
                    ui.label(RichText::new("A fresh puzzle from Lichess every day.").weak());
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
                    ui.scope(|ui| {
                        ui.set_min_width(ui.available_width());
                        ui.spacing_mut().item_spacing.y = 8.0;
                    ui.label(RichText::new("Build your practice").size(17.0).strong());
                    ui.label(RichText::new("Choose a theme and difficulty, then start a puzzle.").weak());
                    ui.label(RichText::new("PUZZLE TYPE").size(11.0).color(gold));
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
                            let fill = if selected { Color32::from_rgb(48,46,37) }
                                else { Color32::from_rgb(30,39,44) };
                            let text = if selected { gold } else { accent };
                            let chip = egui::Button::new(RichText::new(name).color(text))
                                .fill(fill)
                                .stroke(Stroke::new(1.0, if selected { gold } else { Color32::from_rgb(57,69,74) }))
                                .corner_radius(5.0)
                                .min_size(Vec2::new(0.0, 36.0));
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
                            .integer().show_value(false).step_by(1.0).trailing_fill(true));
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
                } else if state.tab == 2 {
                    let count = state.history.iter().filter(|r| !r.solved).count();
                    ui.heading(format!("Review mistakes · {count}"));
                    ui.label("Retry missed or assisted puzzles. A clean solve clears a puzzle from this list.");
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
                    ui.add_space(8.0);
                    let accent = if session.done { Color32::from_rgb(136,205,158) } else { Color32::from_rgb(215,185,121) };
                    Frame::new().fill(Color32::from_rgb(22,30,32))
                        .stroke(Stroke::new(1.0,accent.gamma_multiply(0.5)))
                        .corner_radius(6.0).inner_margin(14.0).show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.label(RichText::new(if session.done { "PUZZLE COMPLETE" } else { "ON THE BOARD" }).size(11.0).color(accent));
                    ui.horizontal_wrapped(|ui| {
                        ui.strong(format!("Puzzle {} · {} to move", session.data.puzzle.id, if session.board.side_to_move() == Color::White { "White" } else { "Black" }));
                        if session.done { ui.label(format!("Rating {} · {}", session.data.puzzle.rating, session.data.puzzle.themes.join(", "))); }
                    });
                    ui.label(RichText::new(&session.message).size(17.0));
                    ui.horizontal_wrapped(|ui| {
                        if !session.done {
                            if ui.add_enabled(!state.loading, egui::Button::new(RichText::new("Hint").color(accent))
                                .fill(Color32::from_rgb(30,39,44))
                                .stroke(Stroke::new(1.0, Color32::from_rgb(57,69,74)))
                                .corner_radius(5.0).min_size(Vec2::new(100.0, 36.0))).clicked() {
                                if let Some(mv) = session.moves.get(session.step) { session.message = format!("Look at the piece on {}.", mv.get_source()); result = Some(false); }
                            }
                            if ui.add_enabled(!state.loading, egui::Button::new(RichText::new("Show solution").color(accent))
                                .fill(Color32::from_rgb(30,39,44))
                                .stroke(Stroke::new(1.0, Color32::from_rgb(57,69,74)))
                                .corner_radius(5.0).min_size(Vec2::new(110.0, 36.0))).clicked() { session.reveal(); result = Some(false); }
                        }

                    });
                    });
                }
                if let Some(solved) = result { state.record(solved); }
                if state.tab == 2 {
                    ui.add_space(8.0);
                    ui.separator();
                    ui.strong("Choose a puzzle to retry");
                    let mut retry = None;
                    // Use the panel's full-height scroll area, rather than trapping the
                    // review queue in a second, fixed-height scrolling viewport.
                    for record in state.history.iter().rev().filter(|r| !r.solved) {
                        let active = state.session.as_ref()
                            .is_some_and(|session| session.data.puzzle.id == record.data.puzzle.id);
                        let gold = Color32::from_rgb(211, 173, 98);
                        Frame::new().fill(Color32::from_rgb(27, 33, 41))
                            .stroke(Stroke::new(1.0, if active { gold } else { Color32::from_rgb(49, 59, 71) }))
                            .corner_radius(8.0).inner_margin(12.0).show(ui, |ui| {
                                ui.set_min_width(ui.available_width());
                                ui.horizontal_wrapped(|ui| {
                                    ui.strong(format!("Puzzle {}", record.data.puzzle.id));
                                    ui.colored_label(gold, format!("Rating {}", record.data.puzzle.rating));
                                    if active { ui.small("On the board"); }
                                });
                                ui.add(egui::Label::new(record.data.puzzle.themes.join(" · ")).wrap());
                                if ui.add_enabled(!state.loading, egui::Button::new(
                                    RichText::new("Retry puzzle").strong().color(Color32::from_rgb(30, 26, 20)))
                                    .fill(gold).corner_radius(6.0)
                                    .min_size(Vec2::new(130.0, 36.0)).raised_gold()).clicked() {
                                    retry = Some(record.data.clone());
                                }
                            });
                    }
                    if !state.history.iter().any(|r| !r.solved) {
                        ui.colored_label(Color32::from_rgb(136, 205, 158), "All caught up!");
                        ui.label("Missed puzzles and puzzles solved with help will appear here.");
                    }
                    if let Some(data) = retry {
                        match Session::new(data) {
                            Ok(session) => state.session = Some(session),
                            Err(error) => state.error = error,
                        }
                    }
                }
                });
                ui.add_space(4.0);
                if ui.link("Puzzle data by Lichess · licensing").on_hover_text("Open Lichess puzzle data and licensing in a new tab").clicked() {
                    ui.ctx().open_url(egui::OpenUrl::new_tab("https://database.lichess.org/#puzzles"));
                }
            });
        }
        if close { open = false; self.board_mark_mode=false; self.board_mark_drag=None; }
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
    fn unsolved_saved_puzzle_restores_mode_and_preserves_annotations() {
        let ctx=egui::Context::default();
        let mut app=ChessApp::new(&eframe::CreationContext::_new_kittest(ctx.clone()));
        app.open_study("[Event \"Study\"]\n[StudyName \"Before puzzle\"]\n[ChapterName \"First\"]\n[ChapterURL \"https://lichess.org/study/abcdefgh/12345678\"]\n\n1. e4 e5 *").unwrap();
        app.navigation_enter_puzzles(&ctx);
        let mut session=Session::new(example()).unwrap();
        app.sync_puzzle_position(&mut session).unwrap();
        app.move_notes[0]="Look for the fork".into();app.move_annotations[0]=vec![1];
        app.board_marks[0]=vec![BoardMark{style:String::new(),color:'G',from:"f5".into(),to:"f3".into()}];
        let saved:PersistedGame=serde_json::from_str(&serde_json::to_string(&app.persisted_game()).unwrap()).unwrap();
        assert!(saved.puzzle.is_some());
        assert!(saved.study.is_none());
        let other=egui::Context::default();
        let mut restored=ChessApp::new(&eframe::CreationContext::_new_kittest(other));
        restored.load_pgn_with_storage(saved.review_pgn.as_deref().unwrap(),false);
        restored.move_notes=saved.move_notes.clone();restored.board_marks=saved.board_marks.clone();restored.move_annotations=saved.move_annotations.clone();
        restored.restore_saved_puzzle(saved.puzzle.as_ref());
        assert!(restored.puzzle_mode());assert!(restored.puzzle_navigation_locked());assert_eq!(restored.board,session.board);
        assert_eq!(restored.move_notes[0],"Look for the fork");assert_eq!(restored.board_marks[0].len(),1);assert_eq!(restored.move_annotations[0],vec![1]);
        assert!(!restored.engine_enabled);
        restored.exit_puzzle_mode();assert!(restored.puzzle_snapshot().is_none());
    }

    #[test]
    fn saved_puzzle_progress_is_validated_before_restoring() {
        let mut session=Session::new(example()).unwrap();session.submit(ChessMove::from_str("f5f3").unwrap());
        let snapshot=PuzzleSnapshot::from_session(&session);
        let restored=snapshot.restore().unwrap();assert_eq!(restored.board,session.board);assert_eq!(restored.step,session.step);
        let mut invalid=snapshot;invalid.played=vec!["a1a8".into()];assert!(invalid.restore().is_err());
    }

    #[test]
    fn advancing_a_puzzle_keeps_notes_and_marks_on_existing_positions() {
        let ctx=egui::Context::default();
        let mut app=ChessApp::new(&eframe::CreationContext::_new_kittest(ctx));
        let mut session=Session::new(example()).unwrap();
        app.sync_puzzle_position(&mut session).unwrap();
        app.move_notes[0]="Keep this puzzle note".into();
        app.move_annotations[0]=vec![1];
        session.reveal();
        app.sync_puzzle_position(&mut session).unwrap();
        assert_eq!(app.move_notes[0],"Keep this puzzle note");
        assert_eq!(app.move_annotations[0],vec![1]);
        let mut next=Session::new(example()).unwrap();
        app.sync_puzzle_position(&mut next).unwrap();
        assert_ne!(app.move_notes[0],"Keep this puzzle note");
    }

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
