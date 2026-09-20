use chess::{Board, BoardStatus, ChessMove, Color, File, MoveGen, Piece, Rank, Square};
use eframe::egui::{
    self, Align, Align2, Color32, FontFamily, FontId, Layout, RichText, Stroke, Vec2,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[cfg(target_arch = "wasm32")]
use std::sync::mpsc::{self, Receiver};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, JsValue, closure::Closure};

#[cfg(target_arch = "wasm32")]
struct EngineBridge {
    worker: web_sys::Worker,
    messages: Receiver<EngineEvent>,
    _handler: Closure<dyn FnMut(web_sys::MessageEvent)>,
}

#[cfg(target_arch = "wasm32")]
enum EngineEvent {
    Line(String),
    Progress { loaded: f64, total: f64 },
}

#[cfg(target_arch = "wasm32")]
impl EngineBridge {
    fn new(ctx: egui::Context) -> Option<Self> {
        let worker = web_sys::Worker::new("/engine-worker.js").ok()?;
        let (tx, messages) = mpsc::channel();
        let handler = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
            let data = event.data();
            let line = js_sys::Reflect::get(&data, &JsValue::from_str("line"))
                .ok()
                .and_then(|v| v.as_string());
            let error = js_sys::Reflect::get(&data, &JsValue::from_str("message"))
                .ok()
                .and_then(|v| v.as_string());
            if let Some(message) = line.or(error) {
                let _ = tx.send(EngineEvent::Line(message));
            } else {
                let loaded = js_sys::Reflect::get(&data, &JsValue::from_str("loaded"))
                    .ok()
                    .and_then(|v| v.as_f64());
                let total = js_sys::Reflect::get(&data, &JsValue::from_str("total"))
                    .ok()
                    .and_then(|v| v.as_f64());
                if let (Some(loaded), Some(total)) = (loaded, total) {
                    let _ = tx.send(EngineEvent::Progress { loaded, total });
                }
            }
            ctx.request_repaint();
        }) as Box<dyn FnMut(_)>);
        worker.set_onmessage(Some(handler.as_ref().unchecked_ref()));
        let init = js_sys::Object::new();
        js_sys::Reflect::set(&init, &"type".into(), &"init".into()).ok()?;
        let threads = web_sys::window()?
            .navigator()
            .hardware_concurrency()
            .clamp(1.0, 8.0);
        js_sys::Reflect::set(&init, &"threads".into(), &threads.into()).ok()?;
        js_sys::Reflect::set(&init, &"hash".into(), &64.into()).ok()?;
        worker.post_message(&init).ok()?;
        Some(Self {
            worker,
            messages,
            _handler: handler,
        })
    }

    fn command(&self, command: &str) {
        let message = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&message, &"type".into(), &"command".into());
        let _ = js_sys::Reflect::set(&message, &"command".into(), &command.into());
        let _ = self.worker.post_message(&message);
    }
}

const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
#[cfg(target_arch = "wasm32")]
const STORAGE_KEY: &str = "ironwood.chess.game.v1";
#[cfg(target_arch = "wasm32")]
const ENGINE_CACHE_KEY: &str = "ironwood.chess.engine.stockfish-19.ready";

#[derive(Serialize, Deserialize)]
struct PersistedGame {
    board: String,
    history: Vec<String>,
    last_move: Option<String>,
    flipped: bool,
    engine_enabled: bool,
}

pub struct ChessApp {
    board: Board,
    selected: Option<Square>,
    legal_targets: Vec<Square>,
    history: Vec<Board>,
    last_move: Option<ChessMove>,
    promotion: Option<(Square, Square)>,
    flipped: bool,
    engine_enabled: bool,
    engine_searching: bool,
    engine_status: String,
    engine_progress: Option<(f32, String)>,
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    show_engine_download: bool,
    resume_engine_after_ready: bool,
    #[cfg(target_arch = "wasm32")]
    engine: Option<EngineBridge>,
}

impl ChessApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut style = (*cc.egui_ctx.style()).clone();
        style.visuals.panel_fill = Color32::from_rgb(17, 20, 24);
        style.visuals.window_fill = Color32::from_rgb(25, 29, 35);
        cc.egui_ctx.set_style(style);
        let saved = Self::load_game();
        let board = saved
            .as_ref()
            .and_then(|game| Board::from_str(&game.board).ok())
            .unwrap_or_else(|| Board::from_str(START_FEN).expect("valid initial board"));
        let history = saved
            .as_ref()
            .map(|game| {
                game.history
                    .iter()
                    .filter_map(|fen| Board::from_str(fen).ok())
                    .collect()
            })
            .unwrap_or_default();
        let last_move = saved
            .as_ref()
            .and_then(|game| game.last_move.as_deref())
            .and_then(Self::parse_uci_value);
        let flipped = saved.as_ref().is_some_and(|game| game.flipped);
        let engine_enabled = saved
            .as_ref()
            .map(|game| game.engine_enabled)
            .unwrap_or(true);
        let resume_engine_after_ready = engine_enabled
            && board.side_to_move() == Color::Black
            && board.status() == BoardStatus::Ongoing;
        let show_engine_download = !Self::engine_was_ready();

        Self {
            board,
            selected: None,
            legal_targets: vec![],
            history,
            last_move,
            promotion: None,
            flipped,
            engine_enabled,
            engine_searching: false,
            engine_status: if show_engine_download {
                "Downloading Stockfish 19…".into()
            } else {
                "Initializing Stockfish 19…".into()
            },
            engine_progress: show_engine_download.then(|| (0.0, "Starting download…".into())),
            show_engine_download,
            resume_engine_after_ready,
            #[cfg(target_arch = "wasm32")]
            engine: EngineBridge::new(cc.egui_ctx.clone()),
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn load_game() -> Option<PersistedGame> {
        let storage = web_sys::window()?.local_storage().ok()??;
        let json = storage.get_item(STORAGE_KEY).ok()??;
        serde_json::from_str(&json).ok()
    }

    #[cfg(target_arch = "wasm32")]
    fn engine_was_ready() -> bool {
        web_sys::window()
            .and_then(|window| window.local_storage().ok().flatten())
            .and_then(|storage| storage.get_item(ENGINE_CACHE_KEY).ok().flatten())
            .is_some()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn engine_was_ready() -> bool {
        false
    }

    #[cfg(target_arch = "wasm32")]
    fn remember_engine_ready() {
        if let Some(storage) =
            web_sys::window().and_then(|window| window.local_storage().ok().flatten())
        {
            let _ = storage.set_item(ENGINE_CACHE_KEY, "1");
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn load_game() -> Option<PersistedGame> {
        None
    }

    fn save_game(&self) {
        #[cfg(target_arch = "wasm32")]
        if let Some(storage) =
            web_sys::window().and_then(|window| window.local_storage().ok().flatten())
        {
            let game = PersistedGame {
                board: self.board.to_string(),
                history: self.history.iter().map(ToString::to_string).collect(),
                last_move: self.last_move.map(|mv| mv.to_string()),
                flipped: self.flipped,
                engine_enabled: self.engine_enabled,
            };
            if let Ok(json) = serde_json::to_string(&game) {
                let _ = storage.set_item(STORAGE_KEY, &json);
            }
        }
    }

    fn reset(&mut self) {
        #[cfg(target_arch = "wasm32")]
        if let Some(engine) = &self.engine {
            engine.command("stop");
            engine.command("ucinewgame");
        }
        self.board = Board::default();
        self.selected = None;
        self.legal_targets.clear();
        self.history.clear();
        self.last_move = None;
        self.promotion = None;
        self.engine_searching = false;
        self.resume_engine_after_ready = false;
        self.engine_status = "Your move".into();
        self.save_game();
    }

    fn select(&mut self, square: Square) {
        if self.promotion.is_some() || self.board.status() != BoardStatus::Ongoing {
            return;
        }
        if self.engine_enabled && self.board.side_to_move() == Color::Black {
            return;
        }
        if let Some(from) = self.selected {
            if self.legal_targets.contains(&square) {
                let promotion_needed = self.board.piece_on(from) == Some(Piece::Pawn)
                    && matches!(square.get_rank(), Rank::First | Rank::Eighth);
                if promotion_needed {
                    self.promotion = Some((from, square));
                } else {
                    self.play(ChessMove::new(from, square, None));
                }
                return;
            }
        }
        if self.board.color_on(square) == Some(self.board.side_to_move()) {
            self.selected = Some(square);
            self.legal_targets = MoveGen::new_legal(&self.board)
                .filter(|m| m.get_source() == square)
                .map(|m| m.get_dest())
                .collect();
        } else {
            self.selected = None;
            self.legal_targets.clear();
        }
    }

    fn play(&mut self, mv: ChessMove) {
        if MoveGen::new_legal(&self.board).any(|candidate| candidate == mv) {
            self.history.push(self.board);
            self.board = self.board.make_move_new(mv);
            self.last_move = Some(mv);
            self.selected = None;
            self.legal_targets.clear();
            self.engine_status = if self.engine_enabled && self.board.side_to_move() == Color::Black
            {
                "Waiting for Stockfish…".into()
            } else {
                "Your move".into()
            };
            self.save_game();
            #[cfg(target_arch = "wasm32")]
            self.request_engine_move();
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn request_engine_move(&mut self) {
        if self.engine_enabled
            && self.board.side_to_move() == Color::Black
            && self.board.status() == BoardStatus::Ongoing
        {
            if let Some(engine) = &self.engine {
                self.engine_searching = true;
                engine.command(&format!("position fen {}", self.board));
                engine.command("go movetime 650");
            }
        }
    }

    fn parse_uci_value(value: &str) -> Option<ChessMove> {
        if value.len() < 4 {
            return None;
        }
        let from = Square::from_str(&value[0..2]).ok()?;
        let to = Square::from_str(&value[2..4]).ok()?;
        let promotion = value.as_bytes().get(4).and_then(|p| match p {
            b'q' => Some(Piece::Queen),
            b'r' => Some(Piece::Rook),
            b'b' => Some(Piece::Bishop),
            b'n' => Some(Piece::Knight),
            _ => None,
        });
        Some(ChessMove::new(from, to, promotion))
    }

    #[cfg(any(target_arch = "wasm32", test))]
    fn parse_uci_move(text: &str) -> Option<ChessMove> {
        let value = text.split_whitespace().nth(1)?;
        Self::parse_uci_value(value)
    }

    #[cfg(target_arch = "wasm32")]
    fn poll_engine(&mut self) {
        let lines: Vec<_> = self
            .engine
            .as_ref()
            .map(|engine| engine.messages.try_iter().collect())
            .unwrap_or_default();
        for event in lines {
            match event {
                EngineEvent::Progress { loaded, total } if total > 0.0 => {
                    if !self.show_engine_download {
                        continue;
                    }
                    let fraction = (loaded / total).clamp(0.0, 1.0) as f32;
                    self.engine_progress = Some((
                        fraction,
                        format!(
                            "{:.1} / {:.1} MiB",
                            loaded / 1_048_576.0,
                            total / 1_048_576.0
                        ),
                    ));
                    self.engine_status = if fraction >= 1.0 {
                        "Compiling and initializing NNUE…".into()
                    } else {
                        "Downloading Stockfish 19…".into()
                    };
                }
                EngineEvent::Progress { .. } => {}
                EngineEvent::Line(line) => {
                    if line == "uciok" {
                        Self::remember_engine_ready();
                        self.show_engine_download = false;
                        self.engine_status = "Stockfish 19 ready".into();
                        self.engine_progress = None;
                        if self.resume_engine_after_ready {
                            self.resume_engine_after_ready = false;
                            self.request_engine_move();
                        }
                    }
                    if line.starts_with("bestmove ") {
                        if self.engine_searching && self.engine_enabled {
                            if let Some(mv) = Self::parse_uci_move(&line) {
                                self.play(mv);
                            }
                        }
                        self.engine_searching = false;
                        self.engine_status = "Your move".into();
                    }
                    if line.contains("Cross-origin isolation") || line.contains("worker failed") {
                        self.engine_status = line;
                        self.engine_progress = None;
                    }
                }
            }
        }
    }

    fn undo(&mut self) {
        #[cfg(target_arch = "wasm32")]
        if let Some(engine) = &self.engine {
            engine.command("stop");
        }
        self.engine_searching = false;
        if let Some(board) = self.history.pop() {
            self.board = board;
            self.last_move = None;
            self.selected = None;
            self.legal_targets.clear();
            self.promotion = None;
            self.save_game();
        }
    }

    fn piece_glyph(&self, square: Square) -> &'static str {
        match (self.board.color_on(square), self.board.piece_on(square)) {
            (Some(Color::White), Some(Piece::King)) => "♔",
            (Some(Color::White), Some(Piece::Queen)) => "♕",
            (Some(Color::White), Some(Piece::Rook)) => "♖",
            (Some(Color::White), Some(Piece::Bishop)) => "♗",
            (Some(Color::White), Some(Piece::Knight)) => "♘",
            (Some(Color::White), Some(Piece::Pawn)) => "♙",
            (Some(Color::Black), Some(Piece::King)) => "♚",
            (Some(Color::Black), Some(Piece::Queen)) => "♛",
            (Some(Color::Black), Some(Piece::Rook)) => "♜",
            (Some(Color::Black), Some(Piece::Bishop)) => "♝",
            (Some(Color::Black), Some(Piece::Knight)) => "♞",
            (Some(Color::Black), Some(Piece::Pawn)) => "♟",
            _ => "",
        }
    }

    fn board_ui(&mut self, ui: &mut egui::Ui) {
        let available = ui.available_size();
        let board_size = available.x.min(available.y).min(760.0).max(280.0);
        let cell = board_size / 8.0;
        ui.allocate_ui_with_layout(
            Vec2::splat(board_size),
            Layout::top_down(Align::Min),
            |ui| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                for screen_rank in 0..8 {
                    ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                        ui.spacing_mut().item_spacing = Vec2::ZERO;
                        for screen_file in 0..8 {
                            let file_index = if self.flipped {
                                7 - screen_file
                            } else {
                                screen_file
                            };
                            let rank_index = if self.flipped {
                                screen_rank
                            } else {
                                7 - screen_rank
                            };
                            let square = Square::make_square(
                                Rank::from_index(rank_index),
                                File::from_index(file_index),
                            );
                            let light = (file_index + rank_index) % 2 == 1;
                            let mut color = if light {
                                Color32::from_rgb(205, 214, 193)
                            } else {
                                Color32::from_rgb(76, 116, 92)
                            };
                            if self
                                .last_move
                                .is_some_and(|m| m.get_source() == square || m.get_dest() == square)
                            {
                                color = if light {
                                    Color32::from_rgb(222, 205, 111)
                                } else {
                                    Color32::from_rgb(165, 159, 67)
                                };
                            }
                            if self.selected == Some(square) {
                                color = Color32::from_rgb(230, 178, 65);
                            }
                            let glyph = self.piece_glyph(square);
                            let label = if glyph.is_empty() && self.legal_targets.contains(&square)
                            {
                                "•"
                            } else {
                                glyph
                            };
                            let piece_color = match self.board.color_on(square) {
                                Some(Color::White) => Color32::from_rgb(250, 246, 224),
                                Some(Color::Black) => Color32::from_rgb(24, 29, 32),
                                None => Color32::from_rgba_unmultiplied(30, 35, 33, 150),
                            };
                            let font = FontId::new(cell * 0.74, FontFamily::Proportional);
                            let response = ui.add_sized(
                                Vec2::splat(cell),
                                egui::Button::new("")
                                    .fill(color)
                                    .stroke(Stroke::NONE)
                                    .corner_radius(0.0),
                            );
                            let center = response.rect.center();
                            if self.board.color_on(square) == Some(Color::White) {
                                let outline = Color32::from_rgb(20, 24, 27);
                                for offset in [
                                    Vec2::new(-1.5, 0.0),
                                    Vec2::new(1.5, 0.0),
                                    Vec2::new(0.0, -1.5),
                                    Vec2::new(0.0, 1.5),
                                    Vec2::new(-1.1, -1.1),
                                    Vec2::new(1.1, -1.1),
                                    Vec2::new(-1.1, 1.1),
                                    Vec2::new(1.1, 1.1),
                                ] {
                                    ui.painter().text(
                                        center + offset,
                                        Align2::CENTER_CENTER,
                                        label,
                                        font.clone(),
                                        outline,
                                    );
                                }
                            }
                            ui.painter().text(
                                center,
                                Align2::CENTER_CENTER,
                                label,
                                font,
                                piece_color,
                            );
                            if response.clicked() {
                                self.select(square);
                            }
                        }
                    });
                }
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_normal_and_promotion_uci_moves() {
        assert_eq!(
            ChessApp::parse_uci_move("bestmove e2e4").unwrap(),
            ChessMove::new(Square::E2, Square::E4, None)
        );
        assert_eq!(
            ChessApp::parse_uci_move("bestmove a7a8q ponder h7h6").unwrap(),
            ChessMove::new(Square::A7, Square::A8, Some(Piece::Queen))
        );
    }
}

impl eframe::App for ChessApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(target_arch = "wasm32")]
        self.poll_engine();
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.heading("IRONWOOD");
                ui.label(RichText::new("CHESS").color(Color32::from_rgb(207, 172, 93)));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("Exit").clicked() {
                        #[cfg(target_arch = "wasm32")]
                        if let Some(window) = web_sys::window() {
                            let _ = window.location().set_href("/");
                        }
                    }
                    if ui.button("New game").clicked() {
                        self.reset();
                    }
                    if ui.button("Undo").clicked() {
                        self.undo();
                    }
                    if ui.button("Flip board").clicked() {
                        self.flipped = !self.flipped;
                        self.save_game();
                    }
                });
            });
            ui.add_space(8.0);
        });

        egui::SidePanel::right("game_panel")
            .default_width(280.0)
            .show(ctx, |ui| {
                ui.add_space(18.0);
                ui.heading(match self.board.status() {
                    BoardStatus::Ongoing => {
                        if self.board.side_to_move() == Color::White {
                            "White to move"
                        } else {
                            "Black to move"
                        }
                    }
                    BoardStatus::Stalemate => "Draw — stalemate",
                    BoardStatus::Checkmate => {
                        if self.board.side_to_move() == Color::White {
                            "Black wins"
                        } else {
                            "White wins"
                        }
                    }
                });
                ui.separator();
                if ui
                    .checkbox(&mut self.engine_enabled, "Play against Stockfish")
                    .changed()
                {
                    self.save_game();
                    #[cfg(target_arch = "wasm32")]
                    if self.engine_enabled {
                        self.request_engine_move();
                    } else {
                        if let Some(engine) = &self.engine {
                            engine.command("stop");
                        }
                        self.engine_searching = false;
                    }
                }
                ui.label(&self.engine_status);
                if let Some((progress, detail)) = &self.engine_progress {
                    ui.add(
                        egui::ProgressBar::new(*progress)
                            .show_percentage()
                            .animate(true),
                    );
                    ui.label(RichText::new(detail).small().weak());
                }
                ui.add_space(12.0);
                ui.label(RichText::new("Engine").strong());
                ui.label("Stockfish 19 · full NNUE");
                ui.label("Worker + shared-memory threads");
                ui.add_space(18.0);
                ui.label(RichText::new("Moves").strong());
                ui.label(format!("{} half-moves played", self.history.len()));
                ui.label(RichText::new("Saved locally").small().weak());
                ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
                    ui.label(
                        RichText::new("2D foundation • animation-ready game state")
                            .small()
                            .weak(),
                    );
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.centered_and_justified(|ui| self.board_ui(ui));
        });

        if let Some((from, to)) = self.promotion {
            egui::Window::new("Choose promotion")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        for (piece, name) in [
                            (Piece::Queen, "Queen"),
                            (Piece::Rook, "Rook"),
                            (Piece::Bishop, "Bishop"),
                            (Piece::Knight, "Knight"),
                        ] {
                            if ui.button(name).clicked() {
                                self.promotion = None;
                                self.play(ChessMove::new(from, to, Some(piece)));
                            }
                        }
                    });
                });
        }
    }
}
