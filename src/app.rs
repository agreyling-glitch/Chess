use chess::{Board, BoardStatus, ChessMove, Color, File, MoveGen, Piece, Rank, Square};
use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontFamily, FontId, Frame, Layout, Margin,
    RichText, Sense, Stroke, TextFormat, Vec2, epaint::TextShape, text::LayoutJob,
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

#[derive(Clone, Default)]
struct AnalysisVariation {
    eval_cp: Option<i32>,
    mate: Option<i32>,
    depth: u32,
    positions: Vec<Board>,
    moves: Vec<String>,
    chess_moves: Vec<ChessMove>,
}

impl AnalysisVariation {
    fn label(&self, rank: usize) -> String {
        let score = if let Some(mate) = self.mate {
            format!("M{mate}")
        } else if let Some(cp) = self.eval_cp {
            format!("{:+.2}", cp as f32 / 100.0)
        } else {
            "—".to_owned()
        };
        let first_move = self.moves.first().map(String::as_str).unwrap_or("…");
        format!(
            "{}. {score} · {first_move} · depth {}",
            rank + 1,
            self.depth
        )
    }

    fn comparison_value(&self) -> Option<i64> {
        if let Some(mate) = self.mate {
            let distance = i64::from(mate).abs().min(999_999);
            Some(if mate > 0 {
                10_000_000 - distance
            } else {
                -10_000_000 + distance
            })
        } else {
            self.eval_cp.map(i64::from)
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
struct PositionAnalysis {
    eval_cp: Option<i32>,
    mate: Option<i32>,
    depth: u32,
    nodes: u64,
    best_move: Option<String>,
    pv: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MoveClassification {
    Best,
    Good,
    Inaccuracy,
    Mistake,
    Blunder,
}

impl MoveClassification {
    fn label(self) -> &'static str {
        match self {
            Self::Best => "BEST",
            Self::Good => "GOOD",
            Self::Inaccuracy => "INACCURACY",
            Self::Mistake => "MISTAKE",
            Self::Blunder => "BLUNDER",
        }
    }

    fn color(self) -> Color32 {
        match self {
            Self::Best => Color32::from_rgb(211, 173, 98),
            Self::Good => Color32::from_rgb(106, 201, 126),
            Self::Inaccuracy => Color32::from_rgb(224, 190, 92),
            Self::Mistake => Color32::from_rgb(224, 139, 76),
            Self::Blunder => Color32::from_rgb(205, 76, 76),
        }
    }

    fn symbol(self) -> &'static str {
        match self {
            Self::Best => "★",
            Self::Good => "+",
            Self::Inaccuracy => "?!",
            Self::Mistake => "?",
            Self::Blunder => "??",
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl EngineBridge {
    fn new(ctx: egui::Context, config: &EngineConfig) -> Option<Self> {
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
        js_sys::Reflect::set(&init, &"threads".into(), &config.threads.into()).ok()?;
        js_sys::Reflect::set(&init, &"hash".into(), &config.hash_mib.into()).ok()?;
        js_sys::Reflect::set(&init, &"skillLevel".into(), &config.skill_level.into()).ok()?;
        js_sys::Reflect::set(
            &init,
            &"limitStrength".into(),
            &config.limit_strength.into(),
        )
        .ok()?;
        js_sys::Reflect::set(&init, &"elo".into(), &config.elo.into()).ok()?;
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
const ENGINE_MAX_THREADS: u32 = 8;
fn default_best_move_arrows() -> bool {
    true
}
#[cfg(target_arch = "wasm32")]
const STORAGE_KEY: &str = "ironwood.chess.game.v1";
#[cfg(target_arch = "wasm32")]
const ENGINE_CACHE_KEY: &str = "ironwood.chess.engine.stockfish-19.ready";

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
struct EngineConfig {
    skill_level: u32,
    limit_strength: bool,
    elo: u32,
    search_limit: SearchLimit,
    move_time_ms: u32,
    depth: u32,
    nodes: u64,
    threads: u32,
    hash_mib: u32,
    multipv: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
enum SearchLimit {
    Infinite,
    #[default]
    Time,
    Depth,
    Nodes,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            skill_level: 20,
            limit_strength: false,
            elo: 1800,
            search_limit: SearchLimit::Time,
            move_time_ms: 650,
            depth: 18,
            nodes: 250_000,
            threads: 0,
            hash_mib: 64,
            multipv: 3,
        }
    }
}

impl EngineConfig {
    fn analysis_default() -> Self {
        Self {
            search_limit: SearchLimit::Nodes,
            nodes: 1_000_000,
            threads: 0,
            ..Self::default()
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EngineSettingsTab {
    Play,
    Analysis,
    FullGame,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
enum FullGameQuality {
    Quick,
    #[default]
    Standard,
    Deep,
    Custom,
}

impl FullGameQuality {
    fn nodes(self, custom_nodes: u64) -> u64 {
        match self {
            Self::Quick => 100_000,
            Self::Standard => 1_000_000,
            Self::Deep => 5_000_000,
            Self::Custom => custom_nodes.clamp(10_000, 50_000_000),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Quick => "Quick",
            Self::Standard => "Standard",
            Self::Deep => "Deep",
            Self::Custom => "Custom",
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
struct FullGameAnalysisConfig {
    quality: FullGameQuality,
    custom_nodes: u64,
}

impl Default for FullGameAnalysisConfig {
    fn default() -> Self {
        Self {
            quality: FullGameQuality::Standard,
            custom_nodes: 1_000_000,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
enum PieceSet {
    #[default]
    System,
    Cburnett,
    Merida,
}

impl PieceSet {
    fn label(self) -> &'static str {
        match self {
            Self::System => "System (Unicode)",
            Self::Cburnett => "Cburnett",
            Self::Merida => "Merida",
        }
    }
}

#[derive(Serialize, Deserialize)]
struct PersistedGame {
    board: String,
    history: Vec<String>,
    last_move: Option<String>,
    flipped: bool,
    #[serde(default)]
    show_coordinates: bool,
    #[serde(default = "default_best_move_arrows")]
    show_best_move_arrows: bool,
    #[serde(default = "default_best_move_arrows")]
    show_move_hover_text: bool,
    #[serde(default)]
    piece_set: PieceSet,
    engine_enabled: bool,
    #[serde(default)]
    engine_config: EngineConfig,
    #[serde(default = "EngineConfig::analysis_default")]
    analysis_config: EngineConfig,
    #[serde(default)]
    full_game_analysis_config: FullGameAnalysisConfig,
    #[serde(default)]
    review_pgn: Option<String>,
    #[serde(default)]
    live_moves: Vec<String>,
    #[serde(default)]
    live_positions: Vec<String>,
    #[serde(default)]
    review_index: Option<usize>,
    #[serde(default)]
    game_analysis: Vec<Option<PositionAnalysis>>,
}

pub struct ChessApp {
    board: Board,
    selected: Option<Square>,
    legal_targets: Vec<Square>,
    history: Vec<Board>,
    last_move: Option<ChessMove>,
    promotion: Option<(Square, Square)>,
    flipped: bool,
    show_coordinates: bool,
    show_best_move_arrows: bool,
    show_move_hover_text: bool,
    piece_set: PieceSet,
    engine_enabled: bool,
    engine_searching: bool,
    engine_status: String,
    engine_progress: Option<(f32, String)>,
    engine_config: EngineConfig,
    analysis_config: EngineConfig,
    full_game_analysis_config: FullGameAnalysisConfig,
    strength_draft: EngineConfig,
    analysis_draft: EngineConfig,
    full_game_analysis_draft: FullGameAnalysisConfig,
    engine_settings_tab: EngineSettingsTab,
    strength_dialog_open: bool,
    about_dialog_open: bool,
    pgn_dialog_open: bool,
    pgn_analyze_after_import: bool,
    pgn_input: String,
    pgn_error: Option<String>,
    review_positions: Vec<Board>,
    review_moves: Vec<String>,
    review_index: Option<usize>,
    review_scroll_to_selected: bool,
    review_white_player: String,
    review_black_player: String,
    analysis_running: bool,
    analysis_eval_cp: Option<i32>,
    analysis_mate: Option<i32>,
    analysis_depth: u32,
    analysis_nodes: u64,
    analysis_nps: u64,
    analysis_pv: String,
    analysis_variations: Vec<AnalysisVariation>,
    selected_variation: usize,
    prediction_positions: Vec<Board>,
    prediction_moves: Vec<String>,
    prediction_chess_moves: Vec<ChessMove>,
    prediction_index: usize,
    prediction_navigation_active: bool,
    prediction_scroll_to_selected: bool,
    game_analysis: Vec<Option<PositionAnalysis>>,
    game_analysis_index: Option<usize>,
    game_analysis_running: bool,
    game_analysis_paused: bool,
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    show_engine_download: bool,
    resume_engine_after_ready: bool,
    pending_analysis_search: Option<String>,
    #[cfg(target_arch = "wasm32")]
    engine: Option<EngineBridge>,
}

impl ChessApp {
    fn paint_arc_text(
        ui: &egui::Ui,
        center: egui::Pos2,
        radius: f32,
        text: &str,
        start_angle: f32,
        end_angle: f32,
        color: Color32,
    ) {
        let characters: Vec<char> = text.chars().collect();
        let denominator = (characters.len().saturating_sub(1)).max(1) as f32;
        let font = FontId::new(10.5, FontFamily::Proportional);
        for (index, character) in characters.into_iter().enumerate() {
            let progress = index as f32 / denominator;
            let angle = start_angle + (end_angle - start_angle) * progress;
            let point = center + Vec2::angled(angle) * radius;
            let galley =
                ui.fonts(|fonts| fonts.layout_no_wrap(character.to_string(), font.clone(), color));
            let position = point - galley.rect.center().to_vec2();
            let tangent = if end_angle >= start_angle {
                angle + std::f32::consts::FRAC_PI_2
            } else {
                angle - std::f32::consts::FRAC_PI_2
            };
            ui.painter().add(
                TextShape::new(position, galley, color)
                    .with_override_text_color(color)
                    .with_angle_and_anchor(tangent, Align2::CENTER_CENTER),
            );
        }
    }

    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        let mut style = (*cc.egui_ctx.style()).clone();
        style.visuals.panel_fill = Color32::from_rgb(17, 20, 24);
        style.visuals.window_fill = Color32::from_rgb(25, 29, 35);
        cc.egui_ctx.set_style(style);
        let saved = Self::load_game();
        let mut board = saved
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
        let mut last_move = saved
            .as_ref()
            .and_then(|game| game.last_move.as_deref())
            .and_then(Self::parse_uci_value);
        let restored_pgn_review = saved
            .as_ref()
            .and_then(|game| game.review_pgn.as_deref())
            .and_then(|pgn| Self::parse_pgn_mainline(pgn).ok());
        let restored_live_review = saved.as_ref().and_then(|game| {
            if game.review_pgn.is_some() || game.live_moves.is_empty() {
                return None;
            }
            let positions = game
                .live_positions
                .iter()
                .map(|fen| Board::from_str(fen))
                .collect::<Result<Vec<_>, _>>()
                .ok()
                .filter(|positions| positions.len() == game.live_moves.len() + 1);
            positions
                .map(|positions| (positions, game.live_moves.clone()))
                .or_else(|| Self::positions_from_san_moves(&game.live_moves).ok())
        });
        let restored_review = restored_pgn_review.or(restored_live_review);
        let review_pgn = saved.as_ref().and_then(|game| game.review_pgn.as_deref());
        let review_white_player = review_pgn
            .and_then(|pgn| Self::pgn_tag(pgn, "White"))
            .unwrap_or_else(|| "You".to_owned());
        let review_black_player = review_pgn
            .and_then(|pgn| Self::pgn_tag(pgn, "Black"))
            .unwrap_or_else(|| "Stockfish 19".to_owned());
        let (review_positions, review_moves, review_index) =
            if let Some((positions, moves)) = restored_review {
                let saved_index = saved.as_ref().and_then(|game| game.review_index);
                let index = saved_index.unwrap_or(moves.len()).min(moves.len());
                if saved_index.is_some() || review_pgn.is_some() {
                    board = positions[index];
                    last_move = Self::review_move_at(&positions, &moves, index);
                }
                (positions, moves, saved_index.or(review_pgn.map(|_| index)))
            } else {
                (Vec::new(), Vec::new(), None)
            };
        let game_analysis = saved
            .as_ref()
            .map(|game| game.game_analysis.clone())
            .filter(|analysis| analysis.len() == review_positions.len())
            .unwrap_or_default();
        let flipped = saved.as_ref().is_some_and(|game| game.flipped);
        let show_coordinates = saved.as_ref().is_some_and(|game| game.show_coordinates);
        let show_best_move_arrows = saved
            .as_ref()
            .map(|game| game.show_best_move_arrows)
            .unwrap_or(true);
        let show_move_hover_text = saved
            .as_ref()
            .map(|game| game.show_move_hover_text)
            .unwrap_or(true);
        let piece_set = saved
            .as_ref()
            .map(|game| game.piece_set)
            .unwrap_or_default();
        let engine_enabled = saved
            .as_ref()
            .map(|game| game.engine_enabled)
            .unwrap_or(true);
        let resume_engine_after_ready = engine_enabled
            && review_index.is_none()
            && board.side_to_move() == Color::Black
            && board.status() == BoardStatus::Ongoing;
        let show_engine_download = !Self::engine_was_ready();
        let available_threads = Self::engine_thread_count();
        let mut engine_config = saved
            .as_ref()
            .map(|game| game.engine_config.clone())
            .unwrap_or_default();
        if engine_config.threads == 0 {
            engine_config.threads = available_threads;
        }
        engine_config.threads = engine_config.threads.clamp(1, available_threads);
        let mut analysis_config = saved
            .as_ref()
            .map(|game| game.analysis_config.clone())
            .unwrap_or_else(EngineConfig::analysis_default);
        if analysis_config.threads == 0 {
            analysis_config.threads = available_threads;
        }
        analysis_config.threads = analysis_config.threads.clamp(1, available_threads);
        let full_game_analysis_config = saved
            .as_ref()
            .map(|game| game.full_game_analysis_config.clone())
            .unwrap_or_default();
        let strength_draft = engine_config.clone();
        let analysis_draft = analysis_config.clone();
        let full_game_analysis_draft = full_game_analysis_config.clone();
        Self::set_page_title(
            review_index.is_some(),
            &review_white_player,
            &review_black_player,
        );

        Self {
            board,
            selected: None,
            legal_targets: vec![],
            history,
            last_move,
            promotion: None,
            flipped,
            show_coordinates,
            show_best_move_arrows,
            show_move_hover_text,
            piece_set,
            engine_enabled,
            engine_searching: false,
            engine_status: if show_engine_download {
                "Downloading Stockfish 19…".into()
            } else {
                "Initializing Stockfish 19…".into()
            },
            engine_progress: show_engine_download.then(|| (0.0, "Starting download…".into())),
            engine_config: engine_config.clone(),
            analysis_config,
            full_game_analysis_config,
            strength_draft,
            analysis_draft,
            full_game_analysis_draft,
            engine_settings_tab: EngineSettingsTab::Play,
            strength_dialog_open: false,
            about_dialog_open: false,
            pgn_dialog_open: false,
            pgn_analyze_after_import: false,
            pgn_input: saved
                .as_ref()
                .and_then(|game| game.review_pgn.clone())
                .unwrap_or_default(),
            pgn_error: None,
            review_positions,
            review_moves,
            review_index,
            review_scroll_to_selected: review_index.is_some(),
            review_white_player,
            review_black_player,
            analysis_running: false,
            analysis_eval_cp: None,
            analysis_mate: None,
            analysis_depth: 0,
            analysis_nodes: 0,
            analysis_nps: 0,
            analysis_pv: String::new(),
            analysis_variations: Vec::new(),
            selected_variation: 0,
            prediction_positions: Vec::new(),
            prediction_moves: Vec::new(),
            prediction_chess_moves: Vec::new(),
            prediction_index: 0,
            prediction_navigation_active: false,
            prediction_scroll_to_selected: false,
            game_analysis,
            game_analysis_index: None,
            game_analysis_running: false,
            game_analysis_paused: false,
            show_engine_download,
            resume_engine_after_ready,
            pending_analysis_search: None,
            #[cfg(target_arch = "wasm32")]
            engine: EngineBridge::new(cc.egui_ctx.clone(), &engine_config),
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn engine_thread_count() -> u32 {
        web_sys::window()
            .map(|window| window.navigator().hardware_concurrency() as u32)
            .unwrap_or(2)
            .clamp(1, ENGINE_MAX_THREADS)
    }

    #[cfg(target_arch = "wasm32")]
    fn set_page_title(analyzing: bool, white: &str, black: &str) {
        if let Some(document) = web_sys::window().and_then(|window| window.document()) {
            let title = if analyzing {
                format!("Analyze — {white} vs {black} — Ironwood Chess")
            } else {
                "Play — Ironwood Chess".to_owned()
            };
            document.set_title(&title);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn set_page_title(_analyzing: bool, _white: &str, _black: &str) {}

    #[cfg(not(target_arch = "wasm32"))]
    fn engine_thread_count() -> u32 {
        std::thread::available_parallelism()
            .map(|count| count.get() as u32)
            .unwrap_or(1)
            .clamp(1, ENGINE_MAX_THREADS)
    }

    fn about_dialog(&mut self, ctx: &egui::Context) {
        if !self.about_dialog_open {
            return;
        }

        let mut close = false;
        let width = (ctx.screen_rect().width() - 48.0).clamp(280.0, 500.0);
        let response = egui::Modal::new(egui::Id::new("about_ironwood_chess"))
            .frame(
                Frame::popup(&ctx.style_of(ctx.theme()))
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(Margin::same(24)),
            )
            .show(ctx, |ui| {
                ui.set_width(width);
                ui.vertical(|ui| {
                    ui.set_width(width);
                    let muted = ui.visuals().weak_text_color();
                    let gold = Color32::from_rgb(211, 173, 98);

                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("IRONWOOD CHESS")
                                .size(12.0)
                                .strong()
                                .extra_letter_spacing(2.0)
                                .color(gold),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            Frame::new()
                                .fill(ui.visuals().widgets.inactive.weak_bg_fill)
                                .corner_radius(CornerRadius::same(5))
                                .inner_margin(Margin::symmetric(9, 4))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(format!(
                                            "v{}  ·  build {}",
                                            env!("CARGO_PKG_VERSION"),
                                            env!("IRONWOOD_BUILD_ID")
                                        ))
                                        .size(12.0),
                                    );
                                });
                        });
                    });

                    ui.add_space(18.0);
                    ui.vertical_centered(|ui| {
                        let (rect, _) =
                            ui.allocate_exact_size(Vec2::new(180.0, 150.0), Sense::hover());
                        let painter = ui.painter();
                        let center = rect.center();
                        painter.circle_stroke(
                            center,
                            68.0,
                            Stroke::new(1.0, gold.gamma_multiply(0.45)),
                        );
                        painter.circle_stroke(
                            center,
                            53.0,
                            Stroke::new(1.0, Color32::WHITE.gamma_multiply(0.12)),
                        );
                        painter.text(
                            center,
                            Align2::CENTER_CENTER,
                            "♞",
                            FontId::new(82.0, FontFamily::Proportional),
                            Color32::from_rgb(243, 240, 228),
                        );
                        Self::paint_arc_text(ui, center, 64.0, "STOCKFISH 19", 3.86, 5.56, gold);
                        Self::paint_arc_text(ui, center, 64.0, "FULL NNUE", 2.30, 0.84, gold);
                        ui.label(
                            RichText::new("Chess without compromise.")
                                .size(25.0)
                                .strong(),
                        );
                        ui.label(
                            RichText::new("Private, unrestricted engine play in your browser.")
                                .size(13.0)
                                .color(muted),
                        );
                    });

                    ui.add_space(20.0);
                    ui.separator();
                    ui.add_space(14.0);
                    ui.label(
                        RichText::new("BUILT FOR THE BROWSER")
                            .size(11.0)
                            .strong()
                            .extra_letter_spacing(1.5)
                            .color(muted),
                    );
                    ui.add_space(10.0);
                    ui.columns(3, |columns| {
                        for (index, column) in columns.iter_mut().enumerate() {
                            let (title, detail) = [
                                ("Rust", "Game and UI"),
                                ("WebGPU", "Rendering"),
                                ("WASM", "Local engine"),
                            ][index];
                            column.vertical_centered(|ui| {
                                Frame::new()
                                    .fill(Color32::WHITE)
                                    .stroke(Stroke::new(1.0, Color32::from_gray(220)))
                                    .corner_radius(CornerRadius::same(7))
                                    .inner_margin(Margin::same(7))
                                    .show(ui, |ui| {
                                        let (rect, _) = ui.allocate_exact_size(
                                            Vec2::new((ui.available_width() - 2.0).max(62.0), 54.0),
                                            Sense::hover(),
                                        );
                                        match index {
                                            0 => {
                                                let logo_rect = egui::Rect::from_center_size(
                                                    rect.center(),
                                                    Vec2::splat(52.0),
                                                );
                                                egui::Image::new(egui::include_image!(
                                                    "../web/brand/rust-logo.svg"
                                                ))
                                                .paint_at(ui, logo_rect);
                                            }
                                            1 => {
                                                let badge = rect.shrink2(Vec2::new(5.0, 9.0));
                                                ui.painter().rect_filled(
                                                    badge,
                                                    5.0,
                                                    Color32::from_rgb(55, 101, 168),
                                                );
                                                ui.painter().text(
                                                    badge.center(),
                                                    Align2::CENTER_CENTER,
                                                    "WebGPU",
                                                    FontId::new(16.0, FontFamily::Proportional),
                                                    Color32::WHITE,
                                                );
                                            }
                                            _ => {
                                                let badge = egui::Rect::from_center_size(
                                                    rect.center(),
                                                    Vec2::splat(44.0),
                                                );
                                                ui.painter().rect_filled(
                                                    badge,
                                                    2.0,
                                                    Color32::from_rgb(101, 78, 163),
                                                );
                                                ui.painter().circle_filled(
                                                    badge.center_top(),
                                                    5.5,
                                                    Color32::WHITE,
                                                );
                                                ui.painter().text(
                                                    badge.center() + Vec2::new(0.0, 2.0),
                                                    Align2::CENTER_CENTER,
                                                    "WA",
                                                    FontId::new(17.0, FontFamily::Monospace),
                                                    Color32::WHITE,
                                                );
                                            }
                                        }
                                    });
                                ui.add_space(8.0);
                                ui.label(RichText::new(title).size(14.0).strong());
                                ui.label(RichText::new(detail).size(11.0).color(muted));
                            });
                        }
                    });

                    ui.add_space(20.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.hyperlink_to("ironwoodchess.com", "https://ironwoodchess.com/");
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.hyperlink_to("Open-source notices", "/open-source-notices.html");
                        });
                    });
                    ui.add_space(14.0);
                    ui.separator();
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Stockfish is licensed under GPLv3.")
                                .size(11.0)
                                .color(muted),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            close = ui
                                .add_sized([76.0, 30.0], egui::Button::new("OK"))
                                .clicked();
                        });
                    });
                });
            });

        if close || response.should_close() {
            self.about_dialog_open = false;
        }
    }

    fn pgn_dialog(&mut self, ctx: &egui::Context) {
        if !self.pgn_dialog_open {
            return;
        }
        let mut import = false;
        let mut cancel = false;
        let response = egui::Modal::new(egui::Id::new("import_pgn"))
            .frame(
                Frame::popup(&ctx.style_of(ctx.theme()))
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(Margin::same(24)),
            )
            .show(ctx, |ui| {
                ui.set_width((ctx.screen_rect().width() - 48.0).clamp(320.0, 620.0));
                ui.label(RichText::new("Import PGN").size(30.0).strong());
                ui.label(
                    RichText::new("Paste a PGN below, or drag a .pgn file onto the app. Comments and side variations are ignored for this first mainline review.")
                        .color(ui.visuals().weak_text_color()),
                );
                ui.add_space(14.0);
                ui.add_sized(
                    [ui.available_width(), 300.0],
                    egui::TextEdit::multiline(&mut self.pgn_input)
                        .hint_text("[Event \"Example\"]\n\n1. e4 e5 2. Nf3 Nc6 ...")
                        .font(egui::TextStyle::Monospace),
                );
                if let Some(error) = &self.pgn_error {
                    ui.label(RichText::new(error).color(Color32::from_rgb(232, 112, 112)));
                }
                ui.add_space(8.0);
                ui.checkbox(
                    &mut self.pgn_analyze_after_import,
                    "Analyze the full game after import",
                );
                ui.label(
                    RichText::new(
                        "Uses the Full game quality setting for every position. You can pause or stop it.",
                    )
                    .size(13.0)
                    .color(ui.visuals().weak_text_color()),
                );
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button("Clear").clicked() {
                        self.pgn_input.clear();
                        self.pgn_error = None;
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        import = ui
                            .add_enabled(!self.pgn_input.trim().is_empty(), egui::Button::new("Import game"))
                            .clicked();
                        cancel = ui.button("Cancel").clicked();
                    });
                });
        });
        if import {
            #[cfg(target_arch = "wasm32")]
            let analyze_after_import = self.pgn_analyze_after_import;
            self.load_pgn();
            #[cfg(target_arch = "wasm32")]
            if analyze_after_import && self.pgn_error.is_none() {
                self.start_game_analysis();
            }
        } else if cancel || response.should_close() {
            self.pgn_dialog_open = false;
            self.pgn_error = None;
        }
    }

    fn evaluation_text(&self) -> String {
        if let Some(mate) = self.analysis_mate {
            format!("M{mate}")
        } else if let Some(cp) = self.analysis_eval_cp {
            format!("{:+.2}", cp as f32 / 100.0)
        } else {
            "—".to_owned()
        }
    }

    fn position_score(analysis: &PositionAnalysis) -> Option<i32> {
        analysis.eval_cp.or_else(|| {
            analysis
                .mate
                .map(|mate| if mate > 0 { 10_000 } else { -10_000 })
        })
    }

    fn move_centipawn_loss(&self, position_index: usize) -> Option<i32> {
        let before = Self::position_score(self.game_analysis.get(position_index - 1)?.as_ref()?)?;
        let after = Self::position_score(self.game_analysis.get(position_index)?.as_ref()?)?;
        let mover = self
            .review_positions
            .get(position_index - 1)?
            .side_to_move();
        Some(match mover {
            Color::White => (before - after).max(0),
            Color::Black => (after - before).max(0),
        })
    }

    fn move_classification(&self, position_index: usize) -> Option<MoveClassification> {
        Some(Self::classify_loss(
            self.move_centipawn_loss(position_index)?,
        ))
    }

    fn classify_loss(loss: i32) -> MoveClassification {
        match loss {
            0..=10 => MoveClassification::Best,
            11..=50 => MoveClassification::Good,
            51..=100 => MoveClassification::Inaccuracy,
            101..=200 => MoveClassification::Mistake,
            _ => MoveClassification::Blunder,
        }
    }

    fn analysis_accuracy(&self, color: Color) -> Option<f32> {
        let losses = self.analysis_losses(color);
        if losses.is_empty() {
            return None;
        }
        let average = losses.iter().sum::<i32>() as f32 / losses.len() as f32;
        Some((100.0 * (-average / 225.0).exp()).clamp(0.0, 100.0))
    }

    fn analysis_losses(&self, color: Color) -> Vec<i32> {
        (1..self.review_positions.len())
            .filter(|index| self.review_positions[index - 1].side_to_move() == color)
            .filter_map(|index| self.move_centipawn_loss(index))
            .collect()
    }

    fn analysis_average_centipawn_loss(&self, color: Color) -> Option<f32> {
        let losses = self.analysis_losses(color);
        (!losses.is_empty()).then(|| losses.iter().sum::<i32>() as f32 / losses.len() as f32)
    }

    fn classification_count(&self, classification: MoveClassification) -> usize {
        (1..self.review_positions.len())
            .filter(|index| self.move_classification(*index) == Some(classification))
            .count()
    }

    fn next_classified_move(
        &self,
        classification: MoveClassification,
        current: usize,
    ) -> Option<usize> {
        ((current + 1)..self.review_positions.len())
            .chain(1..=current.min(self.review_positions.len().saturating_sub(1)))
            .find(|index| self.move_classification(*index) == Some(classification))
    }

    fn analyzed_move_button(
        &self,
        ui: &mut egui::Ui,
        san: &str,
        position_index: usize,
        selected: bool,
        width: f32,
    ) -> egui::Response {
        let classification = self.move_classification(position_index);
        let evaluation = self
            .game_analysis
            .get(position_index)
            .and_then(Option::as_ref);
        let move_description = position_index
            .checked_sub(1)
            .and_then(|index| self.review_positions.get(index).copied())
            .and_then(|board| {
                Self::review_move_at(&self.review_positions, &self.review_moves, position_index)
                    .map(|chess_move| Self::move_hover_text(&board, chess_move))
            });
        let evaluation_text = evaluation.map(|analysis| {
            if let Some(mate) = analysis.mate {
                format!("M{mate}")
            } else if let Some(cp) = analysis.eval_cp {
                format!("{:+.1}", cp as f32 / 100.0)
            } else {
                "—".to_owned()
            }
        });
        let font_id = FontId::proportional(13.0);
        let mut label = LayoutJob::default();
        label.append(
            san,
            0.0,
            TextFormat {
                font_id: font_id.clone(),
                color: ui.visuals().text_color(),
                ..Default::default()
            },
        );
        if let Some(classification) = classification {
            label.append(
                &format!(" {}", classification.symbol()),
                0.0,
                TextFormat {
                    font_id: font_id.clone(),
                    color: classification.color(),
                    ..Default::default()
                },
            );
        }
        if let Some(score) = evaluation_text.as_deref() {
            label.append(
                &format!(" {score}"),
                0.0,
                TextFormat {
                    font_id,
                    color: ui.visuals().weak_text_color(),
                    ..Default::default()
                },
            );
        }
        let mut button = egui::Button::selectable(selected, label);
        if let Some(classification) = classification {
            button = button.stroke(Stroke::new(1.5, classification.color()));
        }
        let response = ui.add_sized([width, 30.0], button);
        if !self.show_move_hover_text {
            return response;
        }
        response.on_hover_ui(|ui| {
            ui.set_max_width(290.0);
            if let Some(description) = &move_description {
                ui.label(RichText::new(description).size(15.0).strong());
                ui.separator();
            }
            ui.label(RichText::new(format!("Move {position_index}: {san}")).strong());
            if let Some(classification) = classification {
                let loss = self.move_centipawn_loss(position_index).unwrap_or_default();
                ui.label(
                    RichText::new(format!(
                        "{} · {loss} centipawn loss",
                        classification.label()
                    ))
                    .color(classification.color()),
                );
            }
            if let Some(analysis) = evaluation {
                ui.label(format!(
                    "Evaluation after move: {}",
                    evaluation_text.as_deref().unwrap_or("—")
                ));
                if let Some(before) = position_index
                    .checked_sub(1)
                    .and_then(|before| self.game_analysis.get(before))
                    .and_then(Option::as_ref)
                {
                    if let Some(best_move) = &before.best_move {
                        ui.label(format!("Engine's best move: {best_move}"));
                    }
                    if !before.pv.is_empty() {
                        ui.separator();
                        ui.label(RichText::new("Engine line").small().weak());
                        ui.label(&before.pv);
                    }
                    ui.label(
                        RichText::new(format!("Depth {} · {} nodes", before.depth, before.nodes))
                            .small()
                            .weak(),
                    );
                } else {
                    ui.label(
                        RichText::new(format!(
                            "Depth {} · {} nodes",
                            analysis.depth, analysis.nodes
                        ))
                        .small()
                        .weak(),
                    );
                }
            } else {
                ui.label(RichText::new("Not analyzed yet").weak());
            }
        })
    }

    fn blend_color(base: Color32, accent: Color32, amount: f32) -> Color32 {
        let blend = |base: u8, accent: u8| {
            (base as f32 + (accent as f32 - base as f32) * amount)
                .round()
                .clamp(0.0, 255.0) as u8
        };
        Color32::from_rgb(
            blend(base.r(), accent.r()),
            blend(base.g(), accent.g()),
            blend(base.b(), accent.b()),
        )
    }

    fn analysis_graph(&self, ui: &mut egui::Ui) -> Option<usize> {
        if self
            .game_analysis
            .iter()
            .filter(|item| item.is_some())
            .count()
            < 2
        {
            return None;
        }
        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), 112.0),
            Sense::click_and_drag(),
        );
        ui.painter()
            .rect_filled(rect, 6.0, Color32::from_rgb(13, 17, 22));
        ui.painter().rect_stroke(
            rect,
            6.0,
            Stroke::new(1.0, Color32::from_white_alpha(25)),
            egui::StrokeKind::Inside,
        );
        let plot = rect.shrink2(Vec2::new(8.0, 10.0));
        let center_y = plot.center().y;
        ui.painter().line_segment(
            [
                egui::pos2(plot.left(), center_y),
                egui::pos2(plot.right(), center_y),
            ],
            Stroke::new(1.0, Color32::from_white_alpha(28)),
        );
        let denominator = self.game_analysis.len().saturating_sub(1).max(1) as f32;
        let point_for = |index: usize, score: i32| {
            let x = plot.left() + plot.width() * index as f32 / denominator;
            let normalized = (score as f32 / 450.0).tanh();
            egui::pos2(x, center_y - normalized * plot.height() * 0.46)
        };
        let mut previous = None;
        for (index, item) in self.game_analysis.iter().enumerate() {
            let Some(score) = item.as_ref().and_then(Self::position_score) else {
                previous = None;
                continue;
            };
            let point = point_for(index, score);
            if let Some(previous) = previous {
                ui.painter().line_segment(
                    [previous, point],
                    Stroke::new(2.0, Color32::from_rgb(211, 173, 98)),
                );
            }
            if index > 0
                && let Some(classification) = self.move_classification(index)
                && matches!(
                    classification,
                    MoveClassification::Inaccuracy
                        | MoveClassification::Mistake
                        | MoveClassification::Blunder
                )
            {
                ui.painter()
                    .circle_filled(point, 3.5, classification.color());
            }
            previous = Some(point);
        }
        if let Some(index) = self.review_index {
            let x = plot.left() + plot.width() * index as f32 / denominator;
            ui.painter().line_segment(
                [egui::pos2(x, plot.top()), egui::pos2(x, plot.bottom())],
                Stroke::new(1.5, Color32::from_rgb(232, 229, 214)),
            );
        }
        let pointer = if response.dragged() || response.clicked() {
            response.interact_pointer_pos()
        } else {
            None
        }?;
        Some(
            (((pointer.x - plot.left()) / plot.width()).clamp(0.0, 1.0) * denominator).round()
                as usize,
        )
    }

    fn evaluation_bar(&self, ui: &mut egui::Ui) {
        let size = Vec2::new(38.0, 220.0_f32.min(ui.available_height().max(100.0)));
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let white_fraction = if let Some(mate) = self.analysis_mate {
            if mate > 0 { 0.97 } else { 0.03 }
        } else if let Some(cp) = self.analysis_eval_cp {
            0.5 + 0.47 * ((cp as f32) / 500.0).tanh()
        } else {
            0.5
        };
        let split = rect.bottom() - rect.height() * white_fraction;
        ui.painter()
            .rect_filled(rect, 4.0, Color32::from_rgb(31, 35, 38));
        ui.painter().rect_filled(
            egui::Rect::from_min_max(egui::pos2(rect.left(), split), rect.right_bottom()),
            4.0,
            Color32::from_rgb(232, 229, 214),
        );
        ui.painter().rect_stroke(
            rect,
            4.0,
            Stroke::new(1.0, Color32::from_white_alpha(55)),
            egui::StrokeKind::Inside,
        );
        let score_rect = egui::Rect::from_center_size(rect.center(), Vec2::new(34.0, 20.0));
        ui.painter().rect_filled(
            score_rect,
            4.0,
            Color32::from_rgba_unmultiplied(15, 18, 21, 232),
        );
        ui.painter().rect_stroke(
            score_rect,
            4.0,
            Stroke::new(1.0, Color32::from_rgb(211, 173, 98)),
            egui::StrokeKind::Inside,
        );
        ui.painter().text(
            score_rect.center(),
            Align2::CENTER_CENTER,
            self.evaluation_text(),
            FontId::proportional(11.0),
            Color32::WHITE,
        );
    }

    fn strength_summary(config: &EngineConfig) -> String {
        if config.limit_strength {
            format!("Limited strength · {} Elo", config.elo)
        } else if config.skill_level < 20 {
            format!("Skill level {} of 20", config.skill_level)
        } else {
            "Maximum strength · Skill 20".to_owned()
        }
    }

    fn strength_dialog(&mut self, ctx: &egui::Context) {
        if !self.strength_dialog_open {
            return;
        }

        let mut apply = false;
        let mut cancel = false;
        let mut reset_defaults = false;
        let available_threads = Self::engine_thread_count();
        let width = (ctx.screen_rect().width() - 48.0).clamp(300.0, 560.0);
        let response = egui::Modal::new(egui::Id::new("stockfish_strength_settings"))
            .frame(
                Frame::popup(&ctx.style_of(ctx.theme()))
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(Margin::same(24)),
            )
            .show(ctx, |ui| {
                ui.set_width(width);
                let muted = ui.visuals().weak_text_color();
                let gold = Color32::from_rgb(211, 173, 98);

                ui.label(RichText::new("Engine settings").size(32.0).strong());
                ui.label(
                    RichText::new("Keep playing strength separate from full-strength analysis.")
                        .size(14.0)
                        .color(muted),
                );

                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    ui.selectable_value(
                        &mut self.engine_settings_tab,
                        EngineSettingsTab::Play,
                        RichText::new("Play").size(15.0).strong(),
                    );
                    ui.selectable_value(
                        &mut self.engine_settings_tab,
                        EngineSettingsTab::Analysis,
                        RichText::new("Analysis").size(15.0).strong(),
                    );
                    ui.selectable_value(
                        &mut self.engine_settings_tab,
                        EngineSettingsTab::FullGame,
                        RichText::new("Full game").size(15.0).strong(),
                    );
                });

                let editing_analysis = self.engine_settings_tab == EngineSettingsTab::Analysis;
                let editing_full_game = self.engine_settings_tab == EngineSettingsTab::FullGame;
                let shared_analysis_threads = self.analysis_draft.threads;
                let shared_analysis_hash_mib = self.analysis_draft.hash_mib;
                let draft = if editing_analysis {
                    &mut self.analysis_draft
                } else {
                    &mut self.strength_draft
                };

                ui.add_space(12.0);
                ui.separator();
                let body_height = (ctx.screen_rect().height() - 250.0).clamp(300.0, 650.0);
                egui::ScrollArea::vertical()
                    .id_salt("engine_settings_scroll")
                    .max_height(body_height)
                    .min_scrolled_height(body_height)
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
                    .show(ui, |ui| {
                        // Reserve a gutter so the floating scrollbar never covers sliders or selectors.
                        ui.set_width((width - 30.0).max(270.0));
                        ui.add_space(14.0);
                if editing_full_game {
                    ui.label(
                        RichText::new("ANALYSIS QUALITY")
                            .size(12.5)
                            .strong()
                            .color(muted),
                    );
                    ui.add_space(10.0);
                    ui.horizontal_wrapped(|ui| {
                        for (quality, description) in [
                            (FullGameQuality::Quick, "100K"),
                            (FullGameQuality::Standard, "1M"),
                            (FullGameQuality::Deep, "5M"),
                            (FullGameQuality::Custom, "Custom"),
                        ] {
                            ui.selectable_value(
                                &mut self.full_game_analysis_draft.quality,
                                quality,
                                RichText::new(format!("{} · {description}", quality.label()))
                                    .size(14.0),
                            );
                        }
                    });
                    ui.add_space(12.0);
                    let nodes = self
                        .full_game_analysis_draft
                        .quality
                        .nodes(self.full_game_analysis_draft.custom_nodes);
                    Frame::new()
                        .fill(ui.visuals().widgets.inactive.weak_bg_fill)
                        .corner_radius(CornerRadius::same(7))
                        .inner_margin(Margin::same(14))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Work per position").size(14.0).strong());
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.label(
                                        RichText::new(format!("{} nodes", nodes))
                                            .size(16.0)
                                            .strong()
                                            .color(gold),
                                    );
                                });
                            });
                            if self.full_game_analysis_draft.quality
                                == FullGameQuality::Custom
                            {
                                ui.add_space(8.0);
                                ui.scope(|ui| {
                                    ui.style_mut().spacing.slider_width = ui.available_width();
                                    ui.style_mut().spacing.slider_rail_height = 6.0;
                                    ui.visuals_mut().selection.bg_fill = gold;
                                    ui.visuals_mut().widgets.inactive.bg_fill =
                                        Color32::from_rgb(25, 29, 32);
                                    ui.visuals_mut().widgets.inactive.fg_stroke =
                                        Stroke::new(1.5, gold);
                                    ui.add(
                                        egui::Slider::new(
                                            &mut self.full_game_analysis_draft.custom_nodes,
                                            10_000..=50_000_000,
                                        )
                                        .logarithmic(true)
                                        .show_value(false)
                                        .trailing_fill(true),
                                    );
                                });
                            }
                        });
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new(match self.full_game_analysis_draft.quality {
                            FullGameQuality::Quick => "Fast pass for spotting major swings and blunders.",
                            FullGameQuality::Standard => "Recommended balance of accuracy and completion time.",
                            FullGameQuality::Deep => "More reliable evaluations, with a substantially longer run time.",
                            FullGameQuality::Custom => "Choose a repeatable node budget for every position.",
                        })
                        .size(13.0)
                        .color(muted),
                    );
                    ui.add_space(22.0);
                    ui.label(
                        RichText::new("SHARED RESOURCES")
                            .size(12.5)
                            .strong()
                            .color(muted),
                    );
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(format!(
                            "{} threads · {} MiB hash · 1 candidate line",
                            shared_analysis_threads, shared_analysis_hash_mib
                        ))
                        .size(14.0)
                        .color(gold),
                    );
                    ui.label(
                        RichText::new(
                            "Threads and hash memory are shared with the Analysis tab. Full-game analysis uses MultiPV 1 so every position completes efficiently.",
                        )
                        .size(13.0)
                        .color(muted),
                    );
                } else {
                if !editing_analysis {
                ui.label(
                    RichText::new("CHOOSE A STRENGTH MODE")
                        .size(12.5)
                        .strong()
                        .color(muted),
                );
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.selectable_value(
                        &mut draft.limit_strength,
                        false,
                        RichText::new("Skill level").size(14.0),
                    );
                    ui.selectable_value(
                        &mut draft.limit_strength,
                        true,
                        RichText::new("Elo rating").size(14.0),
                    );
                });
                ui.add_space(10.0);
                Frame::new()
                    .fill(ui.visuals().widgets.inactive.weak_bg_fill)
                    .corner_radius(CornerRadius::same(7))
                    .inner_margin(Margin::same(14))
                    .show(ui, |ui| {
                        if draft.limit_strength {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Target Elo").size(14.0).strong());
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.label(
                                        RichText::new(format!("{} Elo", draft.elo))
                                            .size(17.0)
                                            .strong()
                                            .color(gold),
                                    );
                                });
                            });
                            ui.scope(|ui| {
                                ui.style_mut().spacing.slider_width = ui.available_width();
                                ui.style_mut().spacing.slider_rail_height = 6.0;
                                ui.visuals_mut().selection.bg_fill = gold;
                                ui.visuals_mut().widgets.inactive.bg_fill =
                                    Color32::from_rgb(25, 29, 32);
                                ui.visuals_mut().widgets.inactive.fg_stroke =
                                    Stroke::new(1.5, gold);
                                ui.add(
                                    egui::Slider::new(&mut draft.elo, 1320..=3190)
                                        .show_value(false)
                                        .trailing_fill(true),
                                );
                            });
                        } else {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Skill level").size(14.0).strong());
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.label(
                                        RichText::new(format!(
                                            "{} / 20",
                                            draft.skill_level
                                        ))
                                        .size(17.0)
                                        .strong()
                                        .color(gold),
                                    );
                                });
                            });
                            ui.scope(|ui| {
                                ui.style_mut().spacing.slider_width = ui.available_width();
                                ui.style_mut().spacing.slider_rail_height = 6.0;
                                ui.visuals_mut().selection.bg_fill = gold;
                                ui.visuals_mut().widgets.inactive.bg_fill =
                                    Color32::from_rgb(25, 29, 32);
                                ui.visuals_mut().widgets.inactive.fg_stroke =
                                    Stroke::new(1.5, gold);
                                ui.add(
                                    egui::Slider::new(
                                        &mut draft.skill_level,
                                        0..=20,
                                    )
                                    .show_value(false)
                                    .trailing_fill(true),
                                );
                            });
                        }
                    });
                ui.add_space(8.0);
                ui.label(
                    RichText::new(if draft.limit_strength {
                        "Choose an approximate playing rating from 1320 to 3190 Elo."
                    } else {
                        "Skill 20 is unrestricted. Lower levels deliberately make Stockfish easier to play."
                    })
                    .size(13.0)
                    .color(muted),
                );
                } else {
                    Frame::new()
                        .fill(ui.visuals().widgets.inactive.weak_bg_fill)
                        .corner_radius(CornerRadius::same(7))
                        .inner_margin(Margin::same(14))
                        .show(ui, |ui| {
                            ui.label(RichText::new("Unrestricted analysis").size(15.0).strong());
                            ui.label(
                                RichText::new("Skill 20 · Elo limiting off · Full NNUE")
                                    .size(13.5)
                                    .color(gold),
                            );
                            ui.label(
                                RichText::new("Analysis always uses Stockfish at full playing strength.")
                                    .size(13.0)
                                    .color(muted),
                            );
                        });
                }

                ui.add_space(22.0);
                ui.label(
                    RichText::new("SEARCH LIMIT")
                        .size(12.5)
                        .strong()
                        .color(muted),
                );
                ui.add_space(10.0);
                ui.horizontal_wrapped(|ui| {
                    if editing_analysis {
                        ui.selectable_value(
                            &mut draft.search_limit,
                            SearchLimit::Infinite,
                            RichText::new("Unlimited").size(14.0),
                        );
                    }
                    ui.selectable_value(
                        &mut draft.search_limit,
                        SearchLimit::Time,
                        RichText::new("Time").size(14.0),
                    );
                    ui.selectable_value(
                        &mut draft.search_limit,
                        SearchLimit::Depth,
                        RichText::new("Fixed depth").size(14.0),
                    );
                    ui.selectable_value(
                        &mut draft.search_limit,
                        SearchLimit::Nodes,
                        RichText::new("Fixed nodes").size(14.0),
                    );
                });
                ui.add_space(8.0);
                let (limit_label, limit_value) = match draft.search_limit {
                    SearchLimit::Infinite => ("Search limit", "Unlimited".to_owned()),
                    SearchLimit::Time => (
                        if editing_analysis { "Time limit" } else { "Time per move" },
                        format!("{} ms", draft.move_time_ms),
                    ),
                    SearchLimit::Depth => (
                        "Search depth",
                        format!("Depth {}", draft.depth),
                    ),
                    SearchLimit::Nodes => (
                        "Nodes per move",
                        format!("{} nodes", draft.nodes),
                    ),
                };
                ui.horizontal(|ui| {
                    ui.label(RichText::new(limit_label).size(14.0).strong());
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new(limit_value).size(14.0).color(gold));
                    });
                });
                ui.scope(|ui| {
                    ui.style_mut().spacing.slider_width = ui.available_width();
                    ui.style_mut().spacing.slider_rail_height = 6.0;
                    ui.visuals_mut().selection.bg_fill = gold;
                    ui.visuals_mut().widgets.inactive.bg_fill = Color32::from_rgb(25, 29, 32);
                    ui.visuals_mut().widgets.inactive.fg_stroke = Stroke::new(1.5, gold);
                    match draft.search_limit {
                        SearchLimit::Infinite => {
                            ui.label(RichText::new("Runs until you stop analysis.").size(13.0));
                        }
                        SearchLimit::Time => {
                            ui.add(
                                egui::Slider::new(
                                    &mut draft.move_time_ms,
                                    100..=60_000,
                                )
                                .logarithmic(true)
                                .show_value(false)
                                .trailing_fill(true),
                            );
                        }
                        SearchLimit::Depth => {
                            ui.add(
                                egui::Slider::new(&mut draft.depth, 1..=40)
                                    .show_value(false)
                                    .trailing_fill(true),
                            );
                        }
                        SearchLimit::Nodes => {
                            ui.add(
                                egui::Slider::new(
                                    &mut draft.nodes,
                                    1_000..=50_000_000,
                                )
                                .logarithmic(true)
                                .show_value(false)
                                .trailing_fill(true),
                            );
                        }
                    }
                });
                ui.add_space(7.0);
                ui.label(
                    RichText::new(match draft.search_limit {
                        SearchLimit::Infinite => {
                            "Best for deep analysis; the search continues until explicitly stopped."
                        }
                        SearchLimit::Time => {
                            "Best for normal play; achieved depth varies by device and position."
                        }
                        SearchLimit::Depth => {
                            "Stops at the chosen depth, but the amount of work can vary by position."
                        }
                        SearchLimit::Nodes => {
                            "Best for comparable analysis. Use one thread for the most repeatable results."
                        }
                    })
                    .size(13.0)
                    .color(muted),
                );

                ui.add_space(18.0);
                ui.label(
                    RichText::new("SEARCH RESOURCES")
                        .size(12.5)
                        .strong()
                        .color(muted),
                );
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Search threads").size(14.0).strong());
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(draft.threads.to_string()).size(14.0),
                        );
                    });
                });
                ui.scope(|ui| {
                    ui.style_mut().spacing.slider_width = ui.available_width();
                    ui.style_mut().spacing.slider_rail_height = 6.0;
                    ui.visuals_mut().selection.bg_fill = gold;
                    ui.visuals_mut().widgets.inactive.bg_fill = Color32::from_rgb(25, 29, 32);
                    ui.visuals_mut().widgets.inactive.fg_stroke = Stroke::new(1.5, gold);
                    ui.add(
                        egui::Slider::new(
                            &mut draft.threads,
                            1..=available_threads,
                        )
                        .show_value(false)
                        .trailing_fill(true),
                    );
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Hash memory").size(14.0).strong());
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        egui::ComboBox::from_id_salt(if editing_analysis {
                            "analysis_hash_memory"
                        } else {
                            "play_hash_memory"
                        })
                            .selected_text(format!("{} MiB", draft.hash_mib))
                            .show_ui(ui, |ui| {
                                for value in [16, 32, 64, 128, 256] {
                                    ui.selectable_value(
                                        &mut draft.hash_mib,
                                        value,
                                        format!("{value} MiB"),
                                    );
                                }
                            });
                    });
                });
                ui.add_space(7.0);
                ui.label(
                    RichText::new(
                        "More threads can finish a search sooner. Hash memory improves search efficiency.",
                    )
                    .size(13.0)
                    .color(muted),
                );

                if editing_analysis {
                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Candidate lines (MultiPV)").size(14.0).strong());
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            egui::ComboBox::from_id_salt("analysis_multipv")
                                .selected_text(draft.multipv.to_string())
                                .show_ui(ui, |ui| {
                                    for value in 1..=5 {
                                        ui.selectable_value(&mut draft.multipv, value, value.to_string());
                                    }
                                });
                        });
                    });
                    ui.label(
                        RichText::new("Show more candidate variations when the analysis view is active.")
                            .size(13.0)
                            .color(muted),
                    );
                }

                }

                        ui.add_space(14.0);
                    });

                ui.add_space(18.0);
                ui.separator();
                ui.add_space(10.0);
                ui.label(
                    RichText::new(if editing_full_game {
                        format!(
                            "Full game · {} · {} nodes per position",
                            self.full_game_analysis_draft.quality.label(),
                            self.full_game_analysis_draft
                                .quality
                                .nodes(self.full_game_analysis_draft.custom_nodes)
                        )
                    } else if editing_analysis {
                        format!("Analysis · {} candidate line{}", draft.multipv, if draft.multipv == 1 { "" } else { "s" })
                    } else {
                        Self::strength_summary(draft)
                    })
                        .size(13.5)
                        .color(gold),
                );
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    reset_defaults = ui
                        .add(egui::Button::new("Reset to defaults").frame(false))
                        .on_hover_text("Restore recommended settings; changes apply only after Apply")
                        .clicked();
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        apply = ui.add_sized([76.0, 30.0], egui::Button::new("Apply")).clicked();
                        cancel = ui.button("Cancel").clicked();
                    });
                });
            });

        if reset_defaults {
            match self.engine_settings_tab {
                EngineSettingsTab::Play => {
                    self.strength_draft = EngineConfig::default();
                    self.strength_draft.threads = available_threads;
                }
                EngineSettingsTab::Analysis => {
                    self.analysis_draft = EngineConfig::analysis_default();
                    self.analysis_draft.threads = available_threads;
                }
                EngineSettingsTab::FullGame => {
                    self.full_game_analysis_draft = FullGameAnalysisConfig::default();
                }
            }
        }

        if apply {
            self.engine_config = self.strength_draft.clone();
            self.analysis_draft.skill_level = 20;
            self.analysis_draft.limit_strength = false;
            self.analysis_config = self.analysis_draft.clone();
            self.full_game_analysis_config = self.full_game_analysis_draft.clone();
            self.save_game();
            #[cfg(target_arch = "wasm32")]
            if let Some(engine) = &self.engine {
                engine.command("stop");
                engine.command(&format!(
                    "setoption name Threads value {}",
                    self.engine_config.threads
                ));
                engine.command(&format!(
                    "setoption name Hash value {}",
                    self.engine_config.hash_mib
                ));
                engine.command(&format!(
                    "setoption name Skill Level value {}",
                    self.engine_config.skill_level
                ));
                engine.command(&format!(
                    "setoption name UCI_LimitStrength value {}",
                    self.engine_config.limit_strength
                ));
                engine.command(&format!(
                    "setoption name UCI_Elo value {}",
                    self.engine_config.elo
                ));
            }
            self.engine_searching = false;
            self.analysis_running = false;
            self.engine_status = "Engine settings updated".into();
            self.strength_dialog_open = false;
            #[cfg(target_arch = "wasm32")]
            self.request_engine_move();
        } else if cancel || response.should_close() {
            self.strength_dialog_open = false;
        }
    }

    fn clear_analysis_result(&mut self) {
        self.pending_analysis_search = None;
        self.analysis_eval_cp = None;
        self.analysis_mate = None;
        self.analysis_depth = 0;
        self.analysis_nodes = 0;
        self.analysis_nps = 0;
        self.analysis_pv.clear();
        self.analysis_variations.clear();
        self.selected_variation = 0;
        self.prediction_positions.clear();
        self.prediction_moves.clear();
        self.prediction_chess_moves.clear();
        self.prediction_index = 0;
        self.prediction_navigation_active = false;
        self.prediction_scroll_to_selected = false;
    }

    fn pgn_tag(text: &str, tag: &str) -> Option<String> {
        let prefix = format!("[{tag} \"");
        text.lines().find_map(|line| {
            line.trim()
                .strip_prefix(&prefix)
                .and_then(|value| value.strip_suffix("\"]"))
                .map(str::to_owned)
        })
    }

    fn parse_pgn_mainline(text: &str) -> Result<(Vec<Board>, Vec<String>), String> {
        let initial_fen = Self::pgn_tag(text, "FEN");
        let mut board = initial_fen
            .as_deref()
            .map(Board::from_str)
            .transpose()
            .map_err(|_| "The PGN contains an invalid FEN header.".to_owned())?
            .unwrap_or_default();

        let mut cleaned = String::with_capacity(text.len());
        let mut brace_depth = 0_u32;
        let mut variation_depth = 0_u32;
        let mut line_comment = false;
        for character in text.chars() {
            if line_comment {
                if character == '\n' {
                    line_comment = false;
                    cleaned.push(' ');
                }
                continue;
            }
            match character {
                ';' if brace_depth == 0 && variation_depth == 0 => line_comment = true,
                '{' if variation_depth == 0 => brace_depth += 1,
                '}' if brace_depth > 0 => brace_depth -= 1,
                '(' if brace_depth == 0 => variation_depth += 1,
                ')' if variation_depth > 0 => variation_depth -= 1,
                _ if brace_depth == 0 && variation_depth == 0 => cleaned.push(character),
                _ => {}
            }
        }

        let move_text = cleaned
            .lines()
            .filter(|line| !line.trim_start().starts_with('['))
            .collect::<Vec<_>>()
            .join(" ");
        let mut positions = vec![board];
        let mut moves = Vec::new();
        for raw in move_text.split_whitespace() {
            let mut token = raw;
            if let Some((_, suffix)) = token.rsplit_once('.') {
                token = suffix;
            }
            token = token.trim_matches(|character| matches!(character, '!' | '?'));
            if token.is_empty()
                || token.starts_with('$')
                || matches!(token, "1-0" | "0-1" | "1/2-1/2" | "*")
            {
                continue;
            }
            let chess_move = ChessMove::from_san(&board, token)
                .map_err(|_| format!("Could not parse move {}: {token}", moves.len() + 1))?;
            board = board.make_move_new(chess_move);
            moves.push(token.to_owned());
            positions.push(board);
        }
        if moves.is_empty() {
            return Err("No legal mainline moves were found in the PGN.".to_owned());
        }
        Ok((positions, moves))
    }

    fn positions_from_san_moves(moves: &[String]) -> Result<(Vec<Board>, Vec<String>), String> {
        let mut board = Board::default();
        let mut positions = vec![board];
        for (index, san) in moves.iter().enumerate() {
            let chess_move = ChessMove::from_san(&board, san)
                .map_err(|_| format!("Could not restore move {}: {san}", index + 1))?;
            board = board.make_move_new(chess_move);
            positions.push(board);
        }
        Ok((positions, moves.to_vec()))
    }

    fn load_pgn(&mut self) {
        match Self::parse_pgn_mainline(&self.pgn_input) {
            Ok((positions, moves)) => {
                self.review_white_player =
                    Self::pgn_tag(&self.pgn_input, "White").unwrap_or_else(|| "White".into());
                self.review_black_player =
                    Self::pgn_tag(&self.pgn_input, "Black").unwrap_or_else(|| "Black".into());
                Self::set_page_title(true, &self.review_white_player, &self.review_black_player);
                #[cfg(target_arch = "wasm32")]
                if let Some(engine) = &self.engine {
                    engine.command("stop");
                }
                self.engine_searching = false;
                self.analysis_running = false;
                self.review_positions = positions;
                self.review_moves = moves;
                self.game_analysis.clear();
                self.game_analysis_index = None;
                self.game_analysis_running = false;
                self.game_analysis_paused = false;
                let final_index = self.review_positions.len() - 1;
                self.review_index = Some(final_index);
                self.review_scroll_to_selected = true;
                self.board = *self.review_positions.last().expect("review has positions");
                self.selected = None;
                self.legal_targets.clear();
                self.last_move =
                    Self::review_move_at(&self.review_positions, &self.review_moves, final_index);
                self.clear_analysis_result();
                self.engine_status = "PGN loaded · ready to analyze".into();
                self.pgn_error = None;
                self.pgn_dialog_open = false;
                self.save_game();
            }
            Err(error) => self.pgn_error = Some(error),
        }
    }

    fn review_move_at(
        positions: &[Board],
        moves: &[String],
        position_index: usize,
    ) -> Option<ChessMove> {
        let move_index = position_index.checked_sub(1)?;
        ChessMove::from_san(positions.get(move_index)?, moves.get(move_index)?).ok()
    }

    fn review_to(&mut self, index: usize) {
        if self.review_positions.is_empty() {
            return;
        }
        let index = index.min(self.review_positions.len() - 1);
        if self.game_analysis_running || self.game_analysis_paused {
            self.review_index = Some(index);
            self.review_scroll_to_selected = true;
            self.board = self.review_positions[index];
            self.last_move =
                Self::review_move_at(&self.review_positions, &self.review_moves, index);
            self.selected = None;
            self.legal_targets.clear();
            self.save_game();
            return;
        }
        #[cfg(target_arch = "wasm32")]
        if let Some(engine) = &self.engine {
            engine.command("stop");
        }
        self.analysis_running = false;
        self.engine_searching = false;
        let live_position = self.pgn_input.is_empty() && index + 1 == self.review_positions.len();
        self.review_index = (!live_position).then_some(index);
        self.review_scroll_to_selected = true;
        self.board = self.review_positions[index];
        self.last_move = Self::review_move_at(&self.review_positions, &self.review_moves, index);
        self.selected = None;
        self.legal_targets.clear();
        self.clear_analysis_result();
        self.engine_status = if live_position {
            "Your move".into()
        } else {
            format!("Reviewing position {index} of {}", self.review_moves.len())
        };
        Self::set_page_title(
            !live_position,
            &self.review_white_player,
            &self.review_black_player,
        );
        self.save_game();
        #[cfg(target_arch = "wasm32")]
        if live_position {
            self.queue_live_engine_resume();
        }
    }

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    fn san_for_move(board: &Board, chess_move: ChessMove) -> String {
        let source = chess_move.get_source();
        let destination = chess_move.get_dest();
        let piece = board.piece_on(source).unwrap_or(Piece::Pawn);
        if piece == Piece::King
            && source
                .get_file()
                .to_index()
                .abs_diff(destination.get_file().to_index())
                == 2
        {
            return if destination.get_file() == File::G {
                "O-O"
            } else {
                "O-O-O"
            }
            .to_owned();
        }
        let capture = board.piece_on(destination).is_some()
            || (piece == Piece::Pawn && source.get_file() != destination.get_file());
        let mut san = String::new();
        if piece != Piece::Pawn {
            san.push(match piece {
                Piece::Knight => 'N',
                Piece::Bishop => 'B',
                Piece::Rook => 'R',
                Piece::Queen => 'Q',
                Piece::King => 'K',
                Piece::Pawn => unreachable!(),
            });
            let alternatives: Vec<_> = MoveGen::new_legal(board)
                .filter(|candidate| {
                    *candidate != chess_move
                        && candidate.get_dest() == destination
                        && board.piece_on(candidate.get_source()) == Some(piece)
                })
                .collect();
            if !alternatives.is_empty() {
                let same_file = alternatives
                    .iter()
                    .any(|candidate| candidate.get_source().get_file() == source.get_file());
                let same_rank = alternatives
                    .iter()
                    .any(|candidate| candidate.get_source().get_rank() == source.get_rank());
                if !same_file {
                    san.push((b'a' + source.get_file().to_index() as u8) as char);
                } else if !same_rank {
                    san.push((b'1' + source.get_rank().to_index() as u8) as char);
                } else {
                    san.push((b'a' + source.get_file().to_index() as u8) as char);
                    san.push((b'1' + source.get_rank().to_index() as u8) as char);
                }
            }
        } else if capture {
            san.push((b'a' + source.get_file().to_index() as u8) as char);
        }
        if capture {
            san.push('x');
        }
        san.push_str(&destination.to_string());
        if let Some(promotion) = chess_move.get_promotion() {
            san.push('=');
            san.push(match promotion {
                Piece::Knight => 'N',
                Piece::Bishop => 'B',
                Piece::Rook => 'R',
                Piece::Queen => 'Q',
                _ => '?',
            });
        }
        let next = board.make_move_new(chess_move);
        if next.status() == BoardStatus::Checkmate {
            san.push('#');
        } else if next.checkers().popcnt() > 0 {
            san.push('+');
        }
        san
    }

    fn move_hover_text(board: &Board, chess_move: ChessMove) -> String {
        let piece = board
            .piece_on(chess_move.get_source())
            .unwrap_or(Piece::Pawn);
        if piece == Piece::King
            && chess_move
                .get_source()
                .get_file()
                .to_index()
                .abs_diff(chess_move.get_dest().get_file().to_index())
                == 2
        {
            return if chess_move.get_dest().get_file() == File::G {
                "Castle kingside".to_owned()
            } else {
                "Castle queenside".to_owned()
            };
        }
        let piece_name = match piece {
            Piece::Pawn => "Pawn",
            Piece::Knight => "Knight",
            Piece::Bishop => "Bishop",
            Piece::Rook => "Rook",
            Piece::Queen => "Queen",
            Piece::King => "King",
        };
        let destination = chess_move.get_dest().to_string().to_ascii_uppercase();
        if let Some(promotion) = chess_move.get_promotion() {
            let promotion_name = match promotion {
                Piece::Knight => "Knight",
                Piece::Bishop => "Bishop",
                Piece::Rook => "Rook",
                Piece::Queen => "Queen",
                _ => "piece",
            };
            format!("{piece_name} to {destination}, promote to {promotion_name}")
        } else {
            format!("{piece_name} to {destination}")
        }
    }

    fn prediction_line(
        board: Board,
        uci_moves: &[&str],
    ) -> (Vec<Board>, Vec<String>, Vec<ChessMove>) {
        let mut board = board;
        let mut positions = vec![board];
        let mut san = Vec::new();
        let mut chess_moves = Vec::new();
        for value in uci_moves.iter().take(12) {
            let Some(chess_move) = Self::parse_uci_value(value) else {
                break;
            };
            if !MoveGen::new_legal(&board).any(|candidate| candidate == chess_move) {
                break;
            }
            san.push(Self::san_for_move(&board, chess_move));
            board = board.make_move_new(chess_move);
            chess_moves.push(chess_move);
            positions.push(board);
        }
        (positions, san, chess_moves)
    }

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    fn update_analysis_from_info(&mut self, line: &str) {
        let fields: Vec<_> = line.split_whitespace().collect();
        let value_after = |name: &str| {
            fields
                .iter()
                .position(|field| *field == name)
                .and_then(|index| fields.get(index + 1).copied())
        };
        let variation_index = value_after("multipv")
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(1)
            .saturating_sub(1) as usize;
        let depth = value_after("depth")
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        let root = self
            .game_analysis_index
            .filter(|_| self.game_analysis_running)
            .or(self.review_index)
            .and_then(|index| self.review_positions.get(index).copied())
            .unwrap_or(self.board);
        let perspective = if root.side_to_move() == Color::White {
            1
        } else {
            -1
        };
        let mut eval_cp = None;
        let mut mate = None;
        if let Some(score_index) = fields.iter().position(|field| *field == "score") {
            match (fields.get(score_index + 1), fields.get(score_index + 2)) {
                (Some(&"cp"), Some(value)) => {
                    if let Ok(value) = value.parse::<i32>() {
                        eval_cp = Some(value * perspective);
                    }
                }
                (Some(&"mate"), Some(value)) => {
                    if let Ok(value) = value.parse::<i32>() {
                        mate = Some(value * perspective);
                    }
                }
                _ => {}
            }
        }
        if variation_index == 0 {
            self.analysis_depth = depth;
            if let Some(value) = value_after("nodes").and_then(|value| value.parse().ok()) {
                self.analysis_nodes = value;
            }
            if let Some(value) = value_after("nps").and_then(|value| value.parse().ok()) {
                self.analysis_nps = value;
            }
            self.analysis_eval_cp = eval_cp;
            self.analysis_mate = mate;
        }
        if let Some(pv_index) = fields.iter().position(|field| *field == "pv") {
            let (positions, moves, chess_moves) =
                Self::prediction_line(root, &fields[pv_index + 1..]);
            self.analysis_variations
                .resize_with(variation_index + 1, AnalysisVariation::default);
            self.analysis_variations[variation_index] = AnalysisVariation {
                eval_cp,
                mate,
                depth,
                positions,
                moves,
                chess_moves,
            };
            if variation_index == self.selected_variation {
                self.select_analysis_variation(variation_index);
            }
        }
    }

    fn select_analysis_variation(&mut self, index: usize) {
        let Some(variation) = self.analysis_variations.get(index) else {
            return;
        };
        self.selected_variation = index;
        self.analysis_pv = variation.moves.join(" ");
        self.prediction_positions.clone_from(&variation.positions);
        self.prediction_moves.clone_from(&variation.moves);
        self.prediction_chess_moves
            .clone_from(&variation.chess_moves);
        self.prediction_index = self.prediction_index.min(self.prediction_moves.len());
        self.prediction_scroll_to_selected = true;
        if self.prediction_index > 0 {
            self.board = self.prediction_positions[self.prediction_index];
            self.last_move = self
                .prediction_chess_moves
                .get(self.prediction_index - 1)
                .copied();
        }
    }

    fn prediction_to(&mut self, index: usize) {
        if self.prediction_positions.is_empty() {
            return;
        }
        self.prediction_index = index.min(self.prediction_moves.len());
        self.prediction_navigation_active = true;
        self.prediction_scroll_to_selected = true;
        self.board = self.prediction_positions[self.prediction_index];
        self.last_move = self
            .prediction_index
            .checked_sub(1)
            .and_then(|move_index| self.prediction_chess_moves.get(move_index).copied())
            .or_else(|| {
                self.review_index.and_then(|review_index| {
                    Self::review_move_at(&self.review_positions, &self.review_moves, review_index)
                })
            });
        self.selected = None;
        self.legal_targets.clear();
    }

    #[cfg(target_arch = "wasm32")]
    fn start_analysis(&mut self) {
        self.game_analysis_running = false;
        self.game_analysis_paused = false;
        self.game_analysis_index = None;
        let root = self
            .review_index
            .and_then(|index| self.review_positions.get(index).copied())
            .unwrap_or(self.board);
        self.clear_analysis_result();
        self.board = root;
        self.last_move = self.review_index.and_then(|index| {
            Self::review_move_at(&self.review_positions, &self.review_moves, index)
        });
        let Some(engine) = &self.engine else { return };
        engine.command("stop");
        engine.command(&format!(
            "setoption name Threads value {}",
            self.analysis_config.threads
        ));
        engine.command(&format!(
            "setoption name Hash value {}",
            self.analysis_config.hash_mib
        ));
        engine.command("setoption name Skill Level value 20");
        engine.command("setoption name UCI_LimitStrength value false");
        engine.command(&format!(
            "setoption name MultiPV value {}",
            self.analysis_config.multipv
        ));
        engine.command(&format!("position fen {root}"));
        let search = match self.analysis_config.search_limit {
            SearchLimit::Infinite => "go infinite".to_owned(),
            SearchLimit::Time => format!("go movetime {}", self.analysis_config.move_time_ms),
            SearchLimit::Depth => format!("go depth {}", self.analysis_config.depth),
            SearchLimit::Nodes => format!("go nodes {}", self.analysis_config.nodes),
        };
        self.analysis_running = true;
        self.engine_searching = false;
        self.pending_analysis_search = Some(search);
        self.engine_status = "Preparing analysis…".into();
        engine.command("isready");
    }

    #[cfg(target_arch = "wasm32")]
    fn start_game_analysis(&mut self) {
        if self.review_positions.is_empty() {
            return;
        }
        self.game_analysis = vec![None; self.review_positions.len()];
        self.game_analysis_running = true;
        self.game_analysis_paused = false;
        self.game_analysis_index = Some(0);
        self.save_game();
        self.start_game_analysis_position();
    }

    #[cfg(target_arch = "wasm32")]
    fn start_game_analysis_position(&mut self) {
        let Some(index) = self.game_analysis_index else {
            return;
        };
        let Some(root) = self.review_positions.get(index).copied() else {
            return;
        };
        self.clear_analysis_result();
        let Some(engine) = &self.engine else { return };
        engine.command("stop");
        engine.command(&format!(
            "setoption name Threads value {}",
            self.analysis_config.threads
        ));
        engine.command(&format!(
            "setoption name Hash value {}",
            self.analysis_config.hash_mib
        ));
        engine.command("setoption name Skill Level value 20");
        engine.command("setoption name UCI_LimitStrength value false");
        engine.command("setoption name MultiPV value 1");
        engine.command(&format!("position fen {root}"));
        self.pending_analysis_search = Some(format!(
            "go nodes {}",
            self.full_game_analysis_config
                .quality
                .nodes(self.full_game_analysis_config.custom_nodes)
        ));
        self.analysis_running = true;
        self.engine_searching = false;
        self.engine_status = format!(
            "Preparing game analysis · position {} of {}",
            index + 1,
            self.review_positions.len()
        );
        engine.command("isready");
    }

    #[cfg(target_arch = "wasm32")]
    fn pause_game_analysis(&mut self) {
        if let Some(engine) = &self.engine {
            engine.command("stop");
        }
        self.pending_analysis_search = None;
        self.analysis_running = false;
        self.engine_searching = false;
        self.game_analysis_running = false;
        self.game_analysis_paused = true;
        self.engine_status = "Game analysis paused".into();
    }

    #[cfg(target_arch = "wasm32")]
    fn resume_game_analysis(&mut self) {
        if self.game_analysis_index.is_none() {
            return;
        }
        self.game_analysis_running = true;
        self.game_analysis_paused = false;
        self.start_game_analysis_position();
    }

    #[cfg(target_arch = "wasm32")]
    fn continue_game_analysis(&mut self) {
        let Some(index) = self.game_analysis.iter().position(Option::is_none) else {
            return;
        };
        self.game_analysis_index = Some(index);
        self.game_analysis_running = true;
        self.game_analysis_paused = false;
        self.start_game_analysis_position();
    }

    #[cfg(target_arch = "wasm32")]
    fn stop_game_analysis(&mut self) {
        if let Some(engine) = &self.engine {
            engine.command("stop");
        }
        self.pending_analysis_search = None;
        self.analysis_running = false;
        self.engine_searching = false;
        self.game_analysis_running = false;
        self.game_analysis_paused = false;
        self.game_analysis_index = None;
        self.clear_analysis_result();
        self.engine_status = "Game analysis stopped · partial results saved".into();
        self.save_game();
        self.queue_live_engine_resume();
    }

    #[cfg(target_arch = "wasm32")]
    fn stop_analysis(&mut self) {
        if let Some(engine) = &self.engine {
            engine.command("stop");
        }
        self.pending_analysis_search = None;
        self.analysis_running = false;
        self.engine_searching = false;
        self.game_analysis_running = false;
        self.game_analysis_paused = false;
        self.game_analysis_index = None;
        self.engine_status = "Analysis stopped".into();
        self.queue_live_engine_resume();
    }

    #[cfg(target_arch = "wasm32")]
    fn resume_live_engine_if_needed(&mut self) {
        if self.pgn_input.is_empty()
            && self.review_index.is_none()
            && self.board.side_to_move() == Color::Black
            && self.board.status() == BoardStatus::Ongoing
        {
            self.request_engine_move();
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn queue_live_engine_resume(&mut self) {
        if self.pgn_input.is_empty()
            && self.review_index.is_none()
            && self.board.side_to_move() == Color::Black
            && self.board.status() == BoardStatus::Ongoing
        {
            self.resume_engine_after_ready = true;
            if let Some(engine) = &self.engine {
                engine.command("isready");
            }
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
                show_coordinates: self.show_coordinates,
                show_best_move_arrows: self.show_best_move_arrows,
                show_move_hover_text: self.show_move_hover_text,
                piece_set: self.piece_set,
                engine_enabled: self.engine_enabled,
                engine_config: self.engine_config.clone(),
                analysis_config: self.analysis_config.clone(),
                full_game_analysis_config: self.full_game_analysis_config.clone(),
                review_pgn: (!self.pgn_input.is_empty()).then(|| self.pgn_input.clone()),
                live_moves: if self.pgn_input.is_empty() {
                    self.review_moves.clone()
                } else {
                    Vec::new()
                },
                live_positions: if self.pgn_input.is_empty() {
                    self.review_positions
                        .iter()
                        .map(ToString::to_string)
                        .collect()
                } else {
                    Vec::new()
                },
                review_index: self.review_index,
                game_analysis: self.game_analysis.clone(),
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
        self.analysis_running = false;
        self.review_positions.clear();
        self.review_moves.clear();
        self.game_analysis.clear();
        self.game_analysis_index = None;
        self.game_analysis_running = false;
        self.game_analysis_paused = false;
        self.review_index = None;
        self.review_scroll_to_selected = false;
        self.pgn_input.clear();
        self.review_white_player = "You".into();
        self.review_black_player = "Stockfish 19".into();
        self.clear_analysis_result();
        self.resume_engine_after_ready = false;
        self.engine_status = "Your move".into();
        Self::set_page_title(false, "", "");
        self.save_game();
    }

    fn play_from_current_position(&mut self) {
        let branch_index = self
            .review_index
            .unwrap_or_else(|| self.review_positions.len().saturating_sub(1))
            .min(self.review_positions.len().saturating_sub(1));
        if let Some(branch_board) = self.review_positions.get(branch_index).copied() {
            self.review_positions.truncate(branch_index + 1);
            self.review_moves.truncate(branch_index);
            self.game_analysis.truncate(branch_index + 1);
            self.board = branch_board;
            self.history = self.review_positions[..branch_index].to_vec();
            self.last_move =
                Self::review_move_at(&self.review_positions, &self.review_moves, branch_index);
        }
        let engine_should_move = self.engine_enabled && self.board.side_to_move() == Color::Black;
        #[cfg(target_arch = "wasm32")]
        if let Some(engine) = &self.engine {
            engine.command("stop");
            engine.command("ucinewgame");
            if engine_should_move {
                engine.command("isready");
            }
        }
        self.selected = None;
        self.legal_targets.clear();
        self.promotion = None;
        self.engine_searching = false;
        self.analysis_running = false;
        self.game_analysis_index = None;
        self.game_analysis_running = false;
        self.game_analysis_paused = false;
        self.review_index = None;
        self.review_scroll_to_selected = false;
        self.pgn_input.clear();
        self.review_white_player = "You".into();
        self.review_black_player = "Stockfish 19".into();
        self.clear_analysis_result();
        self.resume_engine_after_ready = engine_should_move;
        self.engine_status = if engine_should_move {
            "Waiting for Stockfish…".into()
        } else {
            "Your move".into()
        };
        Self::set_page_title(false, "", "");
        self.save_game();
    }

    fn select(&mut self, square: Square) {
        if self.review_index.is_some() {
            return;
        }
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
            let san = Self::san_for_move(&self.board, mv);
            if self.review_positions.is_empty() {
                self.review_positions.push(self.board);
                self.review_white_player = "You".into();
                self.review_black_player = "Stockfish 19".into();
            }
            self.history.push(self.board);
            self.board = self.board.make_move_new(mv);
            self.review_moves.push(san);
            self.review_positions.push(self.board);
            if !self.game_analysis.is_empty() {
                self.game_analysis.resize(self.review_positions.len(), None);
            }
            self.review_scroll_to_selected = true;
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
            && self.review_index.is_none()
            && self.board.side_to_move() == Color::Black
            && self.board.status() == BoardStatus::Ongoing
        {
            if let Some(engine) = &self.engine {
                self.engine_searching = true;
                self.analysis_running = false;
                engine.command(&format!(
                    "setoption name Threads value {}",
                    self.engine_config.threads
                ));
                engine.command(&format!(
                    "setoption name Hash value {}",
                    self.engine_config.hash_mib
                ));
                engine.command(&format!(
                    "setoption name Skill Level value {}",
                    self.engine_config.skill_level
                ));
                engine.command(&format!(
                    "setoption name UCI_LimitStrength value {}",
                    self.engine_config.limit_strength
                ));
                engine.command(&format!(
                    "setoption name UCI_Elo value {}",
                    self.engine_config.elo
                ));
                engine.command("setoption name MultiPV value 1");
                engine.command(&format!("position fen {}", self.board));
                let search = match self.engine_config.search_limit {
                    SearchLimit::Infinite => "go infinite".to_owned(),
                    SearchLimit::Time => {
                        format!("go movetime {}", self.engine_config.move_time_ms)
                    }
                    SearchLimit::Depth => format!("go depth {}", self.engine_config.depth),
                    SearchLimit::Nodes => format!("go nodes {}", self.engine_config.nodes),
                };
                engine.command(&search);
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
                    if line == "readyok" && self.resume_engine_after_ready {
                        self.resume_engine_after_ready = false;
                        self.request_engine_move();
                    }
                    if line == "readyok"
                        && self.analysis_running
                        && let Some(search) = self.pending_analysis_search.take()
                    {
                        if let Some(engine) = &self.engine {
                            engine.command(&search);
                            self.engine_searching = true;
                            self.engine_status = if let Some(index) = self
                                .game_analysis_index
                                .filter(|_| self.game_analysis_running)
                            {
                                format!(
                                    "Analyzing game · position {} of {}",
                                    index + 1,
                                    self.review_positions.len()
                                )
                            } else {
                                "Analyzing position…".into()
                            };
                        }
                    }
                    if self.analysis_running && self.engine_searching && line.starts_with("info ") {
                        self.update_analysis_from_info(&line);
                    }
                    if line.starts_with("bestmove ") {
                        let completed_analysis = self.analysis_running && self.engine_searching;
                        if completed_analysis {
                            self.pending_analysis_search = None;
                            if self.game_analysis_running {
                                let index = self.game_analysis_index.unwrap_or(0);
                                let terminal_board = self.review_positions.get(index).copied();
                                let terminal_mate = terminal_board.and_then(|board| {
                                    (board.status() == BoardStatus::Checkmate).then_some(
                                        if board.side_to_move() == Color::White {
                                            -1
                                        } else {
                                            1
                                        },
                                    )
                                });
                                let terminal_draw = terminal_board
                                    .is_some_and(|board| board.status() == BoardStatus::Stalemate);
                                if let Some(slot) = self.game_analysis.get_mut(index) {
                                    *slot = Some(PositionAnalysis {
                                        eval_cp: self
                                            .analysis_eval_cp
                                            .or(terminal_draw.then_some(0)),
                                        mate: self.analysis_mate.or(terminal_mate),
                                        depth: self.analysis_depth,
                                        nodes: self.analysis_nodes,
                                        best_move: line
                                            .split_whitespace()
                                            .nth(1)
                                            .filter(|value| *value != "(none)")
                                            .map(str::to_owned),
                                        pv: self.analysis_pv.clone(),
                                    });
                                }
                                self.save_game();
                                let next = index + 1;
                                if next < self.review_positions.len() {
                                    self.game_analysis_index = Some(next);
                                    self.engine_searching = false;
                                    self.start_game_analysis_position();
                                    continue;
                                }
                                self.game_analysis_running = false;
                                self.game_analysis_paused = false;
                                self.game_analysis_index = None;
                                self.analysis_running = false;
                                self.clear_analysis_result();
                                self.engine_status = "Game analysis complete".into();
                            } else {
                                self.analysis_running = false;
                                self.engine_status = "Analysis complete".into();
                            }
                        } else if self.engine_searching && self.engine_enabled {
                            if let Some(mv) = Self::parse_uci_move(&line) {
                                self.play(mv);
                            }
                            self.engine_status = "Your move".into();
                        }
                        self.engine_searching = false;
                        if completed_analysis {
                            self.resume_live_engine_if_needed();
                        }
                    }
                    if line.contains("Cross-origin isolation") || line.contains("worker failed") {
                        self.engine_status = line;
                        self.engine_progress = None;
                        self.pending_analysis_search = None;
                        self.analysis_running = false;
                        self.engine_searching = false;
                        self.game_analysis_running = false;
                        self.game_analysis_paused = false;
                        self.game_analysis_index = None;
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
            self.review_moves.pop();
            self.review_positions.pop();
            if !self.game_analysis.is_empty() {
                self.game_analysis.truncate(self.review_positions.len());
            }
            self.last_move = self.history.last().and_then(|previous| {
                self.review_moves
                    .last()
                    .and_then(|san| ChessMove::from_san(previous, san).ok())
            });
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

    fn piece_image(&self, square: Square) -> Option<egui::ImageSource<'static>> {
        let color = self.board.color_on(square)?;
        let piece = self.board.piece_on(square)?;
        match (self.piece_set, color, piece) {
            (PieceSet::Cburnett, Color::White, Piece::King) => {
                Some(egui::include_image!("../web/pieces/cburnett/wK.svg"))
            }
            (PieceSet::Cburnett, Color::White, Piece::Queen) => {
                Some(egui::include_image!("../web/pieces/cburnett/wQ.svg"))
            }
            (PieceSet::Cburnett, Color::White, Piece::Rook) => {
                Some(egui::include_image!("../web/pieces/cburnett/wR.svg"))
            }
            (PieceSet::Cburnett, Color::White, Piece::Bishop) => {
                Some(egui::include_image!("../web/pieces/cburnett/wB.svg"))
            }
            (PieceSet::Cburnett, Color::White, Piece::Knight) => {
                Some(egui::include_image!("../web/pieces/cburnett/wN.svg"))
            }
            (PieceSet::Cburnett, Color::White, Piece::Pawn) => {
                Some(egui::include_image!("../web/pieces/cburnett/wP.svg"))
            }
            (PieceSet::Cburnett, Color::Black, Piece::King) => {
                Some(egui::include_image!("../web/pieces/cburnett/bK.svg"))
            }
            (PieceSet::Cburnett, Color::Black, Piece::Queen) => {
                Some(egui::include_image!("../web/pieces/cburnett/bQ.svg"))
            }
            (PieceSet::Cburnett, Color::Black, Piece::Rook) => {
                Some(egui::include_image!("../web/pieces/cburnett/bR.svg"))
            }
            (PieceSet::Cburnett, Color::Black, Piece::Bishop) => {
                Some(egui::include_image!("../web/pieces/cburnett/bB.svg"))
            }
            (PieceSet::Cburnett, Color::Black, Piece::Knight) => {
                Some(egui::include_image!("../web/pieces/cburnett/bN.svg"))
            }
            (PieceSet::Cburnett, Color::Black, Piece::Pawn) => {
                Some(egui::include_image!("../web/pieces/cburnett/bP.svg"))
            }
            (PieceSet::Merida, Color::White, Piece::King) => {
                Some(egui::include_image!("../web/pieces/merida/wK.svg"))
            }
            (PieceSet::Merida, Color::White, Piece::Queen) => {
                Some(egui::include_image!("../web/pieces/merida/wQ.svg"))
            }
            (PieceSet::Merida, Color::White, Piece::Rook) => {
                Some(egui::include_image!("../web/pieces/merida/wR.svg"))
            }
            (PieceSet::Merida, Color::White, Piece::Bishop) => {
                Some(egui::include_image!("../web/pieces/merida/wB.svg"))
            }
            (PieceSet::Merida, Color::White, Piece::Knight) => {
                Some(egui::include_image!("../web/pieces/merida/wN.svg"))
            }
            (PieceSet::Merida, Color::White, Piece::Pawn) => {
                Some(egui::include_image!("../web/pieces/merida/wP.svg"))
            }
            (PieceSet::Merida, Color::Black, Piece::King) => {
                Some(egui::include_image!("../web/pieces/merida/bK.svg"))
            }
            (PieceSet::Merida, Color::Black, Piece::Queen) => {
                Some(egui::include_image!("../web/pieces/merida/bQ.svg"))
            }
            (PieceSet::Merida, Color::Black, Piece::Rook) => {
                Some(egui::include_image!("../web/pieces/merida/bR.svg"))
            }
            (PieceSet::Merida, Color::Black, Piece::Bishop) => {
                Some(egui::include_image!("../web/pieces/merida/bB.svg"))
            }
            (PieceSet::Merida, Color::Black, Piece::Knight) => {
                Some(egui::include_image!("../web/pieces/merida/bN.svg"))
            }
            (PieceSet::Merida, Color::Black, Piece::Pawn) => {
                Some(egui::include_image!("../web/pieces/merida/bP.svg"))
            }
            _ => None,
        }
    }

    fn draw_best_move_arrow(
        ui: &egui::Ui,
        board_rect: egui::Rect,
        chess_move: ChessMove,
        flipped: bool,
    ) {
        let cell = board_rect.width() / 8.0;
        let center = |square: Square| {
            let file = square.get_file().to_index() as f32;
            let rank = square.get_rank().to_index() as f32;
            let screen_file = if flipped { 7.0 - file } else { file };
            let screen_rank = if flipped { rank } else { 7.0 - rank };
            board_rect.left_top()
                + Vec2::new((screen_file + 0.5) * cell, (screen_rank + 0.5) * cell)
        };
        let source = center(chess_move.get_source());
        let destination = center(chess_move.get_dest());
        let direction = (destination - source).normalized();
        let perpendicular = Vec2::new(-direction.y, direction.x);
        let start = source + direction * cell * 0.20;
        let tip = destination - direction * cell * 0.13;
        let head_length = (cell * 0.22).clamp(9.0, 20.0);
        let head_width = (cell * 0.11).clamp(5.0, 11.0);
        let base = tip - direction * head_length;
        let color = Color32::from_rgba_unmultiplied(112, 214, 139, 215);
        ui.painter().line_segment(
            [start, base + direction * 2.0],
            Stroke::new((cell * 0.055).clamp(3.0, 6.0), color),
        );
        ui.painter().add(egui::Shape::convex_polygon(
            vec![
                tip,
                base + perpendicular * head_width,
                base - perpendicular * head_width,
            ],
            color,
            Stroke::NONE,
        ));
        ui.painter().circle_stroke(
            source,
            (cell * 0.10).clamp(5.0, 9.0),
            Stroke::new(2.0, color),
        );
    }

    fn board_ui(&mut self, ui: &mut egui::Ui) {
        let available = ui.available_size();
        let show_players = !self.review_positions.is_empty();
        let player_area_height = if show_players { 64.0 } else { 0.0 };
        let board_size = available
            .x
            .min((available.y - player_area_height).max(180.0))
            .min(760.0)
            .max(180.0);
        let cell = board_size / 8.0;
        let highlighted_position_index = (self.prediction_index == 0)
            .then(|| {
                self.review_index.or_else(|| {
                    (!self.review_positions.is_empty())
                        .then(|| self.review_positions.len().saturating_sub(1))
                })
            })
            .flatten();
        let highlighted_classification =
            highlighted_position_index.and_then(|index| self.move_classification(index));
        let best_move_arrow = self
            .show_best_move_arrows
            .then_some(())
            .and(highlighted_position_index)
            .filter(|_| {
                matches!(
                    highlighted_classification,
                    Some(
                        MoveClassification::Inaccuracy
                            | MoveClassification::Mistake
                            | MoveClassification::Blunder
                    )
                )
            })
            .and_then(|index| index.checked_sub(1))
            .and_then(|index| self.game_analysis.get(index))
            .and_then(Option::as_ref)
            .and_then(|analysis| analysis.best_move.as_deref())
            .and_then(Self::parse_uci_value)
            .filter(|best_move| Some(*best_move) != self.last_move);
        ui.allocate_ui_with_layout(
            Vec2::new(board_size, board_size + player_area_height),
            Layout::top_down(Align::Min),
            |ui| {
                if show_players {
                    let top_player = if self.flipped {
                        &self.review_white_player
                    } else {
                        &self.review_black_player
                    };
                    ui.allocate_ui_with_layout(
                        Vec2::new(board_size, 28.0),
                        Layout::left_to_right(Align::Center),
                        |ui| {
                            ui.label(RichText::new("●").color(Color32::from_rgb(76, 116, 92)));
                            ui.label(RichText::new(top_player).size(17.0).strong());
                        },
                    );
                    ui.add_space(4.0);
                }
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                let board_response = ui.allocate_ui_with_layout(
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
                                    if self.last_move.is_some_and(|m| {
                                        m.get_source() == square || m.get_dest() == square
                                    }) {
                                        color = if self.prediction_index > 0 {
                                            if light {
                                                Color32::from_rgb(181, 187, 184)
                                            } else {
                                                Color32::from_rgb(94, 104, 101)
                                            }
                                        } else {
                                            match highlighted_classification {
                                                Some(classification) => {
                                                    let destination = self
                                                        .last_move
                                                        .is_some_and(|m| m.get_dest() == square);
                                                    Self::blend_color(
                                                        color,
                                                        classification.color(),
                                                        if destination { 0.68 } else { 0.38 },
                                                    )
                                                }
                                                None if light => Color32::from_rgb(222, 205, 111),
                                                None => Color32::from_rgb(165, 159, 67),
                                            }
                                        };
                                    }
                                    if self.selected == Some(square) {
                                        color = Color32::from_rgb(230, 178, 65);
                                    }
                                    let glyph = self.piece_glyph(square);
                                    let piece_image = self.piece_image(square);
                                    let uses_image = piece_image.is_some();
                                    let label = if glyph.is_empty()
                                        && piece_image.is_none()
                                        && self.legal_targets.contains(&square)
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
                                    if self.prediction_index == 0
                                        && self.last_move.is_some_and(|m| m.get_dest() == square)
                                        && let Some(classification) = highlighted_classification
                                    {
                                        ui.painter().rect_stroke(
                                            response.rect.shrink(2.0),
                                            0.0,
                                            Stroke::new(2.5, classification.color()),
                                            egui::StrokeKind::Inside,
                                        );
                                    }
                                    let center = response.rect.center();
                                    if let Some(source) = piece_image {
                                        egui::Image::new(source)
                                            .maintain_aspect_ratio(true)
                                            .paint_at(ui, response.rect.shrink(cell * 0.06));
                                    } else if let Some(piece_side) = self.board.color_on(square) {
                                        let (outline, cardinal, diagonal) = match piece_side {
                                            Color::White => {
                                                (Color32::from_rgb(20, 24, 27), 1.5, 1.1)
                                            }
                                            Color::Black => (
                                                Color32::from_rgba_unmultiplied(246, 244, 232, 210),
                                                0.9,
                                                0.65,
                                            ),
                                        };
                                        for offset in [
                                            Vec2::new(-cardinal, 0.0),
                                            Vec2::new(cardinal, 0.0),
                                            Vec2::new(0.0, -cardinal),
                                            Vec2::new(0.0, cardinal),
                                            Vec2::new(-diagonal, -diagonal),
                                            Vec2::new(diagonal, -diagonal),
                                            Vec2::new(-diagonal, diagonal),
                                            Vec2::new(diagonal, diagonal),
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
                                    if !uses_image {
                                        ui.painter().text(
                                            center,
                                            Align2::CENTER_CENTER,
                                            label,
                                            font,
                                            piece_color,
                                        );
                                    }
                                    if self.show_coordinates {
                                        let coordinate_color = if light {
                                            Color32::from_rgb(64, 91, 72)
                                        } else {
                                            Color32::from_rgb(218, 225, 207)
                                        };
                                        let coordinate_font =
                                            FontId::proportional((cell * 0.14).clamp(10.0, 16.0));
                                        if screen_file == 0 {
                                            ui.painter().text(
                                                response.rect.left_top() + Vec2::new(4.0, 3.0),
                                                Align2::LEFT_TOP,
                                                (rank_index + 1).to_string(),
                                                coordinate_font.clone(),
                                                coordinate_color,
                                            );
                                        }
                                        if screen_rank == 7 {
                                            ui.painter().text(
                                                response.rect.right_bottom() - Vec2::new(4.0, 3.0),
                                                Align2::RIGHT_BOTTOM,
                                                ((b'a' + file_index as u8) as char).to_string(),
                                                coordinate_font,
                                                coordinate_color,
                                            );
                                        }
                                    }
                                    if response.clicked() {
                                        self.select(square);
                                    }
                                }
                            });
                        }
                    },
                );
                if let Some(best_move) = best_move_arrow {
                    Self::draw_best_move_arrow(
                        ui,
                        board_response.response.rect,
                        best_move,
                        self.flipped,
                    );
                }
                if show_players {
                    ui.add_space(4.0);
                    let bottom_player = if self.flipped {
                        &self.review_black_player
                    } else {
                        &self.review_white_player
                    };
                    ui.allocate_ui_with_layout(
                        Vec2::new(board_size, 28.0),
                        Layout::left_to_right(Align::Center),
                        |ui| {
                            ui.label(RichText::new("●").color(Color32::from_rgb(230, 178, 65)));
                            ui.label(RichText::new(bottom_player).size(17.0).strong());
                        },
                    );
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

    #[test]
    fn engine_defaults_are_unrestricted_and_time_limited() {
        let config = EngineConfig::default();
        assert_eq!(config.skill_level, 20);
        assert!(!config.limit_strength);
        assert_eq!(config.search_limit, SearchLimit::Time);
        assert_eq!(config.move_time_ms, 650);
        assert_eq!(config.depth, 18);
        assert_eq!(config.nodes, 250_000);
        assert_eq!(config.hash_mib, 64);
    }

    #[test]
    fn analysis_defaults_are_unrestricted_and_repeatable() {
        let config = EngineConfig::analysis_default();
        assert_eq!(config.skill_level, 20);
        assert!(!config.limit_strength);
        assert_eq!(config.search_limit, SearchLimit::Nodes);
        assert_eq!(config.nodes, 1_000_000);
        assert_eq!(config.multipv, 3);
    }

    #[test]
    fn parses_pgn_mainline_with_comments_and_variations() {
        let pgn = r#"
[Event "Test"]
[Result "1-0"]

1. e4 {King pawn} e5 2. Nf3 (2. Bc4) Nc6 3. Bb5 a6 1-0
"#;
        let (positions, moves) = ChessApp::parse_pgn_mainline(pgn).unwrap();
        assert_eq!(moves, ["e4", "e5", "Nf3", "Nc6", "Bb5", "a6"]);
        assert_eq!(positions.len(), 7);
    }

    #[test]
    fn restores_live_game_positions_from_san_moves() {
        let moves = vec!["e4".to_owned(), "e5".to_owned(), "Nf3".to_owned()];
        let (positions, restored_moves) = ChessApp::positions_from_san_moves(&moves).unwrap();
        assert_eq!(restored_moves, moves);
        assert_eq!(positions.len(), 4);
        assert_eq!(positions.last().unwrap().side_to_move(), Color::Black);
    }

    #[test]
    fn describes_moves_for_hover_text() {
        let board = Board::default();
        let knight = ChessMove::from_san(&board, "Nf3").unwrap();
        assert_eq!(ChessApp::move_hover_text(&board, knight), "Knight to F3");

        let castle_board = Board::from_str("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").unwrap();
        let castle = ChessMove::from_san(&castle_board, "O-O").unwrap();
        assert_eq!(
            ChessApp::move_hover_text(&castle_board, castle),
            "Castle kingside"
        );
    }

    #[test]
    fn reads_player_names_from_pgn_tags() {
        let pgn = r#"
[White "Carlsen, Magnus"]
[Black "Firouzja, Alireza"]

1. d4 Nf6
"#;
        assert_eq!(
            ChessApp::pgn_tag(pgn, "White").as_deref(),
            Some("Carlsen, Magnus")
        );
        assert_eq!(
            ChessApp::pgn_tag(pgn, "Black").as_deref(),
            Some("Firouzja, Alireza")
        );
    }

    #[test]
    fn renders_principal_variation_as_san() {
        let (positions, san, moves) =
            ChessApp::prediction_line(Board::default(), &["e2e4", "e7e5", "g1f3"]);
        assert_eq!(san, ["e4", "e5", "Nf3"]);
        assert_eq!(moves.len(), 3);
        assert_eq!(positions.len(), 4);
    }

    #[test]
    fn orders_evaluations_and_mate_scores_for_variation_colors() {
        let centipawns = |value| AnalysisVariation {
            eval_cp: Some(value),
            ..Default::default()
        };
        let mate = |value| AnalysisVariation {
            mate: Some(value),
            ..Default::default()
        };
        assert!(centipawns(40).comparison_value() > centipawns(15).comparison_value());
        assert!(mate(2).comparison_value() > mate(5).comparison_value());
        assert!(mate(-5).comparison_value() > mate(-2).comparison_value());
        assert!(mate(8).comparison_value() > centipawns(5_000).comparison_value());
    }

    #[test]
    fn classifies_centipawn_loss_boundaries() {
        assert!(matches!(
            ChessApp::classify_loss(10),
            MoveClassification::Best
        ));
        assert!(matches!(
            ChessApp::classify_loss(50),
            MoveClassification::Good
        ));
        assert!(matches!(
            ChessApp::classify_loss(100),
            MoveClassification::Inaccuracy
        ));
        assert!(matches!(
            ChessApp::classify_loss(200),
            MoveClassification::Mistake
        ));
        assert!(matches!(
            ChessApp::classify_loss(201),
            MoveClassification::Blunder
        ));
    }
}

impl eframe::App for ChessApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(contents) = ctx.input(|input| {
            input.raw.dropped_files.iter().find_map(|file| {
                file.bytes
                    .as_ref()
                    .and_then(|bytes| String::from_utf8(bytes.to_vec()).ok())
            })
        }) {
            self.pgn_input = contents;
            self.load_pgn();
        }
        if let Some(index) = self.review_index.or_else(|| {
            (!self.review_positions.is_empty()).then(|| self.review_positions.len() - 1)
        }) && !ctx.wants_keyboard_input()
            && !self.pgn_dialog_open
            && !self.strength_dialog_open
            && !self.about_dialog_open
        {
            if self.prediction_navigation_active && !self.prediction_moves.is_empty() {
                let destination = ctx.input(|input| {
                    if input.key_pressed(egui::Key::ArrowLeft) {
                        Some(self.prediction_index.saturating_sub(1))
                    } else if input.key_pressed(egui::Key::ArrowRight) {
                        Some((self.prediction_index + 1).min(self.prediction_moves.len()))
                    } else if input.key_pressed(egui::Key::Home) {
                        Some(0)
                    } else if input.key_pressed(egui::Key::End) {
                        Some(self.prediction_moves.len())
                    } else {
                        None
                    }
                });
                if let Some(destination) = destination
                    && destination != self.prediction_index
                {
                    self.prediction_to(destination);
                }
            } else {
                let destination = ctx.input(|input| {
                    if input.key_pressed(egui::Key::ArrowLeft) {
                        Some(index.saturating_sub(1))
                    } else if input.key_pressed(egui::Key::ArrowRight) {
                        Some((index + 1).min(self.review_moves.len()))
                    } else if input.key_pressed(egui::Key::Home) {
                        Some(0)
                    } else if input.key_pressed(egui::Key::End) {
                        Some(self.review_moves.len())
                    } else {
                        None
                    }
                });
                if let Some(destination) = destination
                    && destination != index
                {
                    self.review_to(destination);
                }
            }
        }
        #[cfg(target_arch = "wasm32")]
        if !self.review_positions.is_empty()
            && !ctx.wants_keyboard_input()
            && !self.pgn_dialog_open
            && !self.strength_dialog_open
            && !self.about_dialog_open
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Space))
        {
            if self.game_analysis_running {
                self.pause_game_analysis();
            } else if self.game_analysis_paused {
                self.resume_game_analysis();
            } else if self.analysis_running {
                self.stop_analysis();
            } else {
                self.start_analysis();
            }
        }
        #[cfg(target_arch = "wasm32")]
        self.poll_engine();
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.heading("IRONWOOD");
                ui.label(RichText::new("CHESS").color(Color32::from_rgb(207, 172, 93)));
                ui.separator();
                ui.menu_button("Game", |ui| {
                    if ui.button("Import PGN…").clicked() {
                        self.pgn_dialog_open = true;
                        self.pgn_error = None;
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("New game").clicked() {
                        self.reset();
                        ui.close();
                    }
                    if ui
                        .add_enabled(!self.history.is_empty(), egui::Button::new("Undo"))
                        .clicked()
                    {
                        self.undo();
                        ui.close();
                    }
                    if ui.button("Flip board").clicked() {
                        self.flipped = !self.flipped;
                        self.save_game();
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Exit to home").clicked() {
                        #[cfg(target_arch = "wasm32")]
                        if let Some(window) = web_sys::window() {
                            let _ = window.location().set_href("/");
                        }
                        ui.close();
                    }
                });
                ui.menu_button("View", |ui| {
                    if ui
                        .checkbox(&mut self.show_coordinates, "Show coordinates")
                        .changed()
                    {
                        self.save_game();
                    }
                    if ui
                        .checkbox(&mut self.show_best_move_arrows, "Show best-move arrows")
                        .changed()
                    {
                        self.save_game();
                    }
                    if ui
                        .checkbox(&mut self.show_move_hover_text, "Show move hover text")
                        .changed()
                    {
                        self.save_game();
                    }
                    ui.menu_button("Piece set", |ui| {
                        for piece_set in [PieceSet::System, PieceSet::Cburnett, PieceSet::Merida] {
                            if ui
                                .selectable_value(&mut self.piece_set, piece_set, piece_set.label())
                                .changed()
                            {
                                self.save_game();
                                ui.close();
                            }
                        }
                    });
                });
                ui.menu_button("Help", |ui| {
                    if ui.button("About Ironwood Chess…").clicked() {
                        self.about_dialog_open = true;
                        ui.close();
                    }
                });
            });
            ui.add_space(8.0);
        });

        egui::TopBottomPanel::bottom("engine_status_dock")
            .resizable(false)
            .frame(
                Frame::new()
                    .fill(Color32::from_rgb(14, 17, 20))
                    .stroke(Stroke::new(1.0, Color32::from_white_alpha(22)))
                    .inner_margin(Margin::symmetric(12, 7)),
            )
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    let status_lower = self.engine_status.to_ascii_lowercase();
                    let dock_state = if self.analysis_running {
                        "Analyzing"
                    } else if self.review_index.is_some() {
                        "Ready"
                    } else if !self.engine_enabled {
                        "Disabled"
                    } else if status_lower.contains("failed")
                        || status_lower.contains("error")
                        || status_lower.contains("cross-origin")
                    {
                        "Error"
                    } else if status_lower.contains("download") {
                        "Downloading"
                    } else if status_lower.contains("initializ")
                        || status_lower.contains("compiling")
                    {
                        "Initializing"
                    } else if self.engine_searching || status_lower.contains("waiting") {
                        "Thinking"
                    } else {
                        "Ready"
                    };
                    let active = dock_state == "Ready";
                    let (indicator, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                    ui.painter().circle_filled(
                        indicator.center(),
                        4.0,
                        if active {
                            Color32::from_rgb(102, 180, 125)
                        } else {
                            Color32::from_rgb(211, 173, 98)
                        },
                    );
                    ui.label(RichText::new(dock_state).strong());
                    ui.separator();
                    ui.label("Stockfish 19 · Full NNUE");
                    ui.separator();
                    let strength_label = if self.review_index.is_some() {
                        format!("Analysis · MultiPV {}", self.analysis_config.multipv)
                    } else {
                        format!("Play · {}", Self::strength_summary(&self.engine_config))
                    };
                    let strength_clicked = ui
                        .scope(|ui| {
                            let gold = Color32::from_rgb(211, 173, 98);
                            ui.style_mut().spacing.button_padding = Vec2::new(7.0, 2.0);
                            ui.visuals_mut().widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
                            ui.visuals_mut().widgets.inactive.bg_stroke =
                                Stroke::new(1.0, gold.gamma_multiply(0.35));
                            ui.visuals_mut().widgets.hovered.weak_bg_fill =
                                gold.gamma_multiply(0.12);
                            ui.visuals_mut().widgets.hovered.bg_stroke = Stroke::new(1.5, gold);
                            ui.visuals_mut().widgets.active.weak_bg_fill = gold.gamma_multiply(0.2);
                            ui.visuals_mut().widgets.active.bg_stroke = Stroke::new(1.5, gold);
                            ui.add(
                                egui::Button::new(RichText::new(strength_label).color(gold))
                                    .frame(true),
                            )
                            .on_hover_text("Open Play and Analysis engine settings")
                            .clicked()
                        })
                        .inner;
                    if strength_clicked {
                        self.strength_draft = self.engine_config.clone();
                        self.analysis_draft = self.analysis_config.clone();
                        self.full_game_analysis_draft = self.full_game_analysis_config.clone();
                        self.engine_settings_tab = if self.review_index.is_some() {
                            EngineSettingsTab::Analysis
                        } else {
                            EngineSettingsTab::Play
                        };
                        self.strength_dialog_open = true;
                    }
                    ui.separator();
                    ui.label(format!(
                        "{} threads · {} MiB hash · {}",
                        if self.review_index.is_some() {
                            self.analysis_config.threads
                        } else {
                            self.engine_config.threads
                        },
                        if self.review_index.is_some() {
                            self.analysis_config.hash_mib
                        } else {
                            self.engine_config.hash_mib
                        },
                        match if self.review_index.is_some() {
                            self.analysis_config.search_limit
                        } else {
                            self.engine_config.search_limit
                        } {
                            SearchLimit::Infinite => "unlimited".to_owned(),
                            SearchLimit::Time => {
                                format!("{} ms/move", self.engine_config.move_time_ms)
                            }
                            SearchLimit::Depth => {
                                format!("depth {}", self.engine_config.depth)
                            }
                            SearchLimit::Nodes => {
                                format!("{} nodes/move", self.engine_config.nodes)
                            }
                        }
                    ));
                    ui.separator();
                    ui.label(RichText::new("Local · private").weak());
                });
                if let Some((progress, detail)) = &self.engine_progress {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.add_sized(
                            [160.0, 8.0],
                            egui::ProgressBar::new(*progress).animate(true),
                        );
                        ui.label(RichText::new(detail).small().weak());
                    });
                }
            });

        egui::SidePanel::right("game_panel")
            .default_width(340.0)
            .min_width(280.0)
            .max_width(640.0)
            .resizable(true)
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
                if self.review_index.is_none()
                    && ui
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
                if let Some(index) = self.review_index.or_else(|| {
                    (!self.review_positions.is_empty()).then(|| self.review_positions.len() - 1)
                }) {
                    let mut jump_to = None;
                    let scroll_to_selected = self.review_scroll_to_selected;
                    ui.label(RichText::new("Game Moves").size(17.0).strong());
                    ui.label(format!("Position {index} of {}", self.review_moves.len()));
                    ui.add_space(8.0);
                    Frame::new()
                        .fill(Color32::from_rgb(11, 16, 29))
                        .stroke(Stroke::new(1.0, Color32::from_white_alpha(24)))
                        .corner_radius(CornerRadius::same(6))
                        .inner_margin(Margin::same(8))
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical()
                                .id_salt("game_moves")
                                .max_height(210.0)
                                .min_scrolled_height(160.0)
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    for pair in 0..self.review_moves.len().div_ceil(2) {
                                        let white_ply = pair * 2;
                                        let black_ply = white_ply + 1;
                                        ui.horizontal(|ui| {
                                            let move_number = ui.add_sized(
                                                [30.0, 28.0],
                                                egui::Label::new(format!("{}.", pair + 1)),
                                            );
                                            if pair == 0 && index == 0 && scroll_to_selected {
                                                move_number.scroll_to_me(Some(Align::Min));
                                            }
                                            let move_width = ((ui.available_width()
                                                - ui.spacing().item_spacing.x)
                                                / 2.0)
                                                .max(78.0);
                                            if let Some(san) = self.review_moves.get(white_ply) {
                                                let selected = index == white_ply + 1;
                                                let response = self.analyzed_move_button(
                                                    ui,
                                                    san,
                                                    white_ply + 1,
                                                    selected,
                                                    move_width,
                                                );
                                                if response.clicked() {
                                                    jump_to = Some(white_ply + 1);
                                                }
                                                if selected && scroll_to_selected {
                                                    response.scroll_to_me(Some(Align::Center));
                                                }
                                            }
                                            if let Some(san) = self.review_moves.get(black_ply) {
                                                let selected = index == black_ply + 1;
                                                let response = self.analyzed_move_button(
                                                    ui,
                                                    san,
                                                    black_ply + 1,
                                                    selected,
                                                    move_width,
                                                );
                                                if response.clicked() {
                                                    jump_to = Some(black_ply + 1);
                                                }
                                                if selected && scroll_to_selected {
                                                    response.scroll_to_me(Some(Align::Center));
                                                }
                                            }
                                        });
                                    }
                                });
                            ui.separator();
                            ui.horizontal(|ui| {
                                let gaps = ui.spacing().item_spacing.x * 3.0;
                                let button_width = ((ui.available_width() - gaps) / 4.0).max(32.0);
                                if ui
                                    .add_enabled(
                                        index > 0,
                                        egui::Button::new("|◀")
                                            .min_size(Vec2::new(button_width, 30.0)),
                                    )
                                    .on_hover_text("First position (Home)")
                                    .clicked()
                                {
                                    jump_to = Some(0);
                                }
                                if ui
                                    .add_enabled(
                                        index > 0,
                                        egui::Button::new("◀")
                                            .min_size(Vec2::new(button_width, 30.0)),
                                    )
                                    .on_hover_text("Previous move (Left arrow)")
                                    .clicked()
                                {
                                    jump_to = Some(index - 1);
                                }
                                if ui
                                    .add_enabled(
                                        index < self.review_moves.len(),
                                        egui::Button::new("▶")
                                            .min_size(Vec2::new(button_width, 30.0)),
                                    )
                                    .on_hover_text("Next move (Right arrow)")
                                    .clicked()
                                {
                                    jump_to = Some(index + 1);
                                }
                                if ui
                                    .add_enabled(
                                        index < self.review_moves.len(),
                                        egui::Button::new("▶|")
                                            .min_size(Vec2::new(button_width, 30.0)),
                                    )
                                    .on_hover_text("Last position (End)")
                                    .clicked()
                                {
                                    jump_to = Some(self.review_moves.len());
                                }
                            });
                        });
                    self.review_scroll_to_selected = false;
                    if let Some(destination) = jump_to {
                        self.review_to(destination);
                    }
                    ui.add_space(12.0);
                    ui.label(RichText::new("Game Analysis").size(18.0).strong());
                    let completed = self
                        .game_analysis
                        .iter()
                        .filter(|item| item.is_some())
                        .count();
                    let total = self.review_positions.len().max(1);
                    if self.game_analysis_running || self.game_analysis_paused || completed > 0 {
                        let fraction = completed as f32 / total as f32;
                        ui.add(
                            egui::ProgressBar::new(fraction)
                                .desired_width(ui.available_width())
                                .desired_height(24.0)
                                .text(format!(
                                    "{completed} / {total} positions · {:.0}%",
                                    fraction * 100.0
                                )),
                        );
                        ui.add_space(5.0);
                    }
                    #[cfg(target_arch = "wasm32")]
                    ui.horizontal(|ui| {
                        if self.game_analysis_running {
                            if ui.button("Pause").clicked() {
                                self.pause_game_analysis();
                            }
                            if ui.button("Stop").clicked() {
                                self.stop_game_analysis();
                            }
                            ui.label(
                                RichText::new(format!(
                                    "Analyzing position {}",
                                    self.game_analysis_index.unwrap_or(completed) + 1
                                ))
                                .size(14.0)
                                .color(Color32::from_rgb(211, 173, 98)),
                            );
                        } else if self.game_analysis_paused {
                            if ui.button("Resume").clicked() {
                                self.resume_game_analysis();
                            }
                            if ui.button("Stop").clicked() {
                                self.stop_game_analysis();
                            }
                            ui.label(
                                RichText::new("Paused · completed positions are saved")
                                    .size(14.0),
                            );
                        } else if completed < total {
                            let label = if completed > 0 {
                                "Continue analysis"
                            } else {
                                "Analyze full game"
                            };
                            if ui.button(label).clicked() {
                                if completed > 0 {
                                    self.continue_game_analysis();
                                } else {
                                    self.start_game_analysis();
                                }
                            }
                        } else if ui.button("Analyze again").clicked() {
                            self.start_game_analysis();
                        }
                    });
                    if let Some(destination) = self.analysis_graph(ui) {
                        self.review_to(destination);
                    }
                    if self.game_analysis.iter().all(Option::is_some)
                        && !self.game_analysis.is_empty()
                    {
                        ui.add_space(8.0);
                        ui.label(RichText::new("Analysis Summary").size(17.0).strong());
                        let white_label = self.review_white_player.clone();
                        let black_label = self.review_black_player.clone();
                        ui.columns(2, |columns| {
                            for (column, (label, color)) in columns.iter_mut().zip([
                                (white_label.as_str(), Color::White),
                                (black_label.as_str(), Color::Black),
                            ]) {
                                let accuracy = self.analysis_accuracy(color).unwrap_or(0.0);
                                let acpl = self
                                    .analysis_average_centipawn_loss(color)
                                    .unwrap_or(0.0);
                                Frame::new()
                                    .fill(Color32::from_rgb(28, 32, 37))
                                    .stroke(Stroke::new(1.0, Color32::from_white_alpha(20)))
                                    .corner_radius(CornerRadius::same(5))
                                    .inner_margin(Margin::same(9))
                                    .show(column, |ui| {
                                        ui.set_min_width(ui.available_width());
                                        ui.label(
                                            RichText::new(format!("{accuracy:.1}%"))
                                                .size(19.0)
                                                .strong()
                                                .color(Color32::from_rgb(230, 178, 65)),
                                        );
                                        ui.label(RichText::new(label).size(14.0).strong());
                                        ui.label(
                                            RichText::new(format!("{acpl:.1} average CPL"))
                                                .size(14.0)
                                                .color(ui.visuals().text_color()),
                                        );
                                    });
                            }
                        });
                        ui.label(
                            RichText::new("Ironwood accuracy · consistent node budget")
                                .size(13.0)
                                .weak(),
                        );
                        ui.add_space(7.0);
                        ui.label(RichText::new("Move quality").size(14.0).strong());
                        let mut summary_jump = None;
                        for classifications in [
                            &[
                                MoveClassification::Best,
                                MoveClassification::Good,
                                MoveClassification::Inaccuracy,
                            ][..],
                            &[
                                MoveClassification::Mistake,
                                MoveClassification::Blunder,
                            ][..],
                        ] {
                            ui.horizontal(|ui| {
                                for &classification in classifications {
                                    let count = self.classification_count(classification);
                                    let response = ui.add_enabled(
                                        count > 0,
                                        egui::Button::new(
                                            RichText::new(format!(
                                                "{} {} {count}",
                                                classification.symbol(),
                                                classification.label()
                                            ))
                                            .color(classification.color()),
                                        )
                                        .stroke(Stroke::new(1.0, classification.color())),
                                    );
                                    if response
                                        .on_hover_text("Jump to the next matching move")
                                        .clicked()
                                    {
                                        summary_jump =
                                            self.next_classified_move(classification, index);
                                    }
                                }
                            });
                        }
                        if let Some(destination) = summary_jump {
                            self.review_to(destination);
                        }
                        if index > 0
                            && let (Some(loss), Some(classification)) = (
                                self.move_centipawn_loss(index),
                                self.move_classification(index),
                            )
                        {
                            ui.add_space(6.0);
                            Frame::new()
                                .fill(Color32::from_rgb(24, 28, 33))
                                .stroke(Stroke::new(1.0, classification.color()))
                                .corner_radius(CornerRadius::same(5))
                                .inner_margin(Margin::same(8))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(classification.label())
                                                .strong()
                                                .color(classification.color()),
                                        );
                                        ui.with_layout(
                                            Layout::right_to_left(Align::Center),
                                            |ui| {
                                                ui.label(format!("{loss} CPL"));
                                            },
                                        );
                                    });
                                    if let Some(before) =
                                        self.game_analysis.get(index - 1).and_then(Option::as_ref)
                                    {
                                        if let Some(best_move) = &before.best_move {
                                            ui.label(format!("Best move: {best_move}"));
                                        }
                                        if !before.pv.is_empty() {
                                            ui.label(RichText::new(&before.pv).size(13.0).weak());
                                        }
                                        ui.label(
                                            RichText::new(format!(
                                                "Depth {} · {} nodes",
                                                before.depth, before.nodes
                                            ))
                                            .size(13.0)
                                            .weak(),
                                        );
                                    }
                                });
                        }
                    }
                    ui.add_space(12.0);
                    if self.game_analysis_running || self.game_analysis_paused {
                        Frame::new()
                            .fill(Color32::from_rgb(24, 28, 33))
                            .corner_radius(CornerRadius::same(5))
                            .inner_margin(Margin::same(9))
                            .show(ui, |ui| {
                                ui.label(RichText::new("Position analysis is on hold").strong());
                                ui.label(
                                    RichText::new(
                                        "Manual evaluation and move prediction return when the full-game pass finishes or stops.",
                                    )
                                    .size(14.0)
                                    .color(ui.visuals().weak_text_color()),
                                );
                            });
                    } else {
                    ui.label(RichText::new("Engine Analysis").size(17.0).strong());
                    ui.horizontal(|ui| {
                        self.evaluation_bar(ui);
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(self.evaluation_text())
                                    .size(26.0)
                                    .strong()
                                    .color(Color32::from_rgb(211, 173, 98)),
                            );
                            ui.label("White perspective");
                            ui.add_space(8.0);
                            #[cfg(target_arch = "wasm32")]
                            if self.analysis_running {
                                if ui
                                    .button("Stop analysis")
                                    .on_hover_text("Stop Stockfish analysis (Space)")
                                    .clicked()
                                {
                                    self.stop_analysis();
                                }
                            } else if ui
                                .button("Analyze position")
                                .on_hover_text("Analyze this position (Space)")
                                .clicked()
                            {
                                self.start_analysis();
                            }
                            ui.label(format!("Depth {}", self.analysis_depth));
                            ui.label(format!("{} nodes", self.analysis_nodes));
                            ui.label(format!("{} nps", self.analysis_nps));
                        });
                    });
                    if !self.prediction_moves.is_empty() {
                        ui.add_space(12.0);
                        ui.label(RichText::new("Engine Move Prediction").size(17.0).strong());
                        if self.analysis_variations.len() > 1 {
                            let labels = self
                                .analysis_variations
                                .iter()
                                .enumerate()
                                .map(|(rank, variation)| variation.label(rank))
                                .collect::<Vec<_>>();
                            let mut selected = self.selected_variation.min(labels.len() - 1);
                            let baseline = self.analysis_variations[selected].comparison_value();
                            let comparison_color = |variation: &AnalysisVariation| match (
                                variation.comparison_value(),
                                baseline,
                            ) {
                                (Some(value), Some(baseline)) if value > baseline => {
                                    Color32::from_rgb(106, 201, 126)
                                }
                                (Some(value), Some(baseline)) if value < baseline => {
                                    Color32::from_rgb(232, 112, 112)
                                }
                                _ => Color32::from_rgb(230, 178, 65),
                            };
                            egui::ComboBox::from_id_salt("engine_prediction_variation")
                                .width(ui.available_width())
                                .selected_text(
                                    RichText::new(&labels[selected])
                                        .color(Color32::from_rgb(230, 178, 65)),
                                )
                                .show_ui(ui, |ui| {
                                    for (index, label) in labels.iter().enumerate() {
                                        ui.selectable_value(
                                            &mut selected,
                                            index,
                                            RichText::new(label).color(comparison_color(
                                                &self.analysis_variations[index],
                                            )),
                                        );
                                    }
                                });
                            if selected != self.selected_variation {
                                self.prediction_index = 0;
                                self.prediction_navigation_active = true;
                                self.select_analysis_variation(selected);
                            }
                            ui.add_space(4.0);
                        }
                        let root = self
                            .review_positions
                            .get(index)
                            .copied()
                            .unwrap_or(self.board);
                        let start_fullmove = root
                            .to_string()
                            .split_whitespace()
                            .nth(5)
                            .and_then(|value| value.parse::<usize>().ok())
                            .unwrap_or(1);
                        let starting_offset = usize::from(root.side_to_move() == Color::Black);
                        let mut rows: Vec<(usize, Option<usize>, Option<usize>)> = Vec::new();
                        for move_index in 0..self.prediction_moves.len() {
                            let absolute = starting_offset + move_index;
                            let row_index = absolute / 2;
                            while rows.len() <= row_index {
                                rows.push((start_fullmove + rows.len(), None, None));
                            }
                            if absolute % 2 == 0 {
                                rows[row_index].1 = Some(move_index);
                            } else {
                                rows[row_index].2 = Some(move_index);
                            }
                        }
                        let mut prediction_jump = None;
                        let scroll_prediction_to_selected = self.prediction_scroll_to_selected;
                        Frame::new()
                            .fill(Color32::from_rgb(20, 23, 27))
                            .stroke(Stroke::new(1.0, Color32::from_white_alpha(28)))
                            .corner_radius(CornerRadius::same(6))
                            .inner_margin(Margin::same(8))
                            .show(ui, |ui| {
                                egui::ScrollArea::vertical()
                                    .id_salt("engine_prediction_moves")
                                    .max_height(150.0)
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        for (row_offset, (move_number, white, black)) in
                                            rows.into_iter().enumerate()
                                        {
                                            ui.horizontal(|ui| {
                                                let number_response = ui.add_sized(
                                                    [30.0, 28.0],
                                                    egui::Label::new(format!("{move_number}.")),
                                                );
                                                if row_offset == 0
                                                    && self.prediction_index == 0
                                                    && scroll_prediction_to_selected
                                                {
                                                    number_response.scroll_to_me(Some(Align::Min));
                                                }
                                                for move_index in [white, black] {
                                                    if let Some(move_index) = move_index {
                                                        let selected =
                                                            self.prediction_index == move_index + 1;
                                                        let button = egui::Button::new(
                                                            &self.prediction_moves[move_index],
                                                        )
                                                        .fill(if selected {
                                                            Color32::from_rgb(91, 98, 102)
                                                        } else {
                                                            Color32::TRANSPARENT
                                                        })
                                                        .frame(selected);
                                                        let mut response =
                                                            ui.add_sized([82.0, 28.0], button);
                                                        if self.show_move_hover_text
                                                            && let (Some(board), Some(chess_move)) = (
                                                                self.prediction_positions
                                                                    .get(move_index),
                                                                self.prediction_chess_moves
                                                                    .get(move_index),
                                                            )
                                                        {
                                                            response = response.on_hover_text(
                                                                Self::move_hover_text(
                                                                    board,
                                                                    *chess_move,
                                                                ),
                                                            );
                                                        }
                                                        if response.clicked() {
                                                            prediction_jump = Some(move_index + 1);
                                                        }
                                                        if selected && scroll_prediction_to_selected
                                                        {
                                                            response
                                                                .scroll_to_me(Some(Align::Center));
                                                        }
                                                    } else {
                                                        ui.allocate_space(Vec2::new(82.0, 28.0));
                                                    }
                                                }
                                            });
                                        }
                                    });
                                ui.separator();
                                ui.horizontal(|ui| {
                                    let gaps = ui.spacing().item_spacing.x * 3.0;
                                    let width = ((ui.available_width() - gaps) / 4.0).max(32.0);
                                    for (label, destination, enabled, hint) in [
                                        ("|◀", 0, self.prediction_index > 0, "First prediction"),
                                        (
                                            "◀",
                                            self.prediction_index.saturating_sub(1),
                                            self.prediction_index > 0,
                                            "Previous predicted move",
                                        ),
                                        (
                                            "▶",
                                            (self.prediction_index + 1)
                                                .min(self.prediction_moves.len()),
                                            self.prediction_index < self.prediction_moves.len(),
                                            "Next predicted move",
                                        ),
                                        (
                                            "▶|",
                                            self.prediction_moves.len(),
                                            self.prediction_index < self.prediction_moves.len(),
                                            "Last prediction",
                                        ),
                                    ] {
                                        if ui
                                            .add_enabled(
                                                enabled,
                                                egui::Button::new(label)
                                                    .min_size(Vec2::new(width, 30.0)),
                                            )
                                            .on_hover_text(hint)
                                            .clicked()
                                        {
                                            prediction_jump = Some(destination);
                                        }
                                    }
                                });
                            });
                        self.prediction_scroll_to_selected = false;
                        if let Some(destination) = prediction_jump {
                            self.prediction_to(destination);
                        }
                    }
                    }
                    ui.add_space(12.0);
                    if self.review_index.is_some()
                        && ui.button("Play from this position").clicked()
                    {
                        self.play_from_current_position();
                    }
                }
                ui.add_space(18.0);
                if self.review_positions.is_empty() {
                    ui.label(RichText::new("Moves").strong());
                    ui.label(format!("{} half-moves played", self.history.len()));
                    ui.label(RichText::new("Saved locally").small().weak());
                }
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

        self.about_dialog(ctx);
        self.strength_dialog(ctx);
        self.pgn_dialog(ctx);
    }
}
