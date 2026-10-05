use crate::rules::{Board, MoveGen};
#[path = "training_ui.rs"]
mod training_ui;
#[path = "tactical_ui.rs"]
mod tactical_ui;
#[path = "notes_ui.rs"]
mod notes_ui;
#[path = "draw_ui.rs"]
mod draw_ui;

use crate::training::{self, Profiles, Profile, Session, Side as TrainingSide, AnalysisStats};
use chess::{BoardStatus, ChessMove, Color, File, Piece, Rank, Square};
use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontFamily, FontId, Frame, Layout, Margin,
    RichText, Sense, Stroke, Vec2, epaint::TextShape,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[cfg(target_arch = "wasm32")]
const DISCORD_URL: &str = "/discord";
#[cfg(not(target_arch = "wasm32"))]
const DISCORD_URL: &str = "https://ironwoodchess.com/discord";
#[cfg(target_arch = "wasm32")]
const HELP_URL: &str = "/help/";
#[cfg(not(target_arch = "wasm32"))]
const HELP_URL: &str = "https://ironwoodchess.com/help/";

trait GoldScrollAreaExt {
    fn show_gold<R>(
        self,
        ui: &mut egui::Ui,
        add_contents: impl FnOnce(&mut egui::Ui) -> R,
    ) -> egui::scroll_area::ScrollAreaOutput<R>;
}

impl GoldScrollAreaExt for egui::ScrollArea {
    fn show_gold<R>(
        self,
        ui: &mut egui::Ui,
        add_contents: impl FnOnce(&mut egui::Ui) -> R,
    ) -> egui::scroll_area::ScrollAreaOutput<R> {
        let original = ui.visuals().clone();
        ui.scope(|ui| {
            let gold = Color32::from_rgb(185, 150, 82);
            let visuals = ui.visuals_mut();
            for widget in [
                &mut visuals.widgets.inactive,
                &mut visuals.widgets.hovered,
                &mut visuals.widgets.active,
            ] {
                widget.bg_fill = gold;
            }
            self.show(ui, |ui| {
                *ui.visuals_mut() = original;
                add_contents(ui)
            })
        })
        .inner
    }
}

struct BatchPgnGame {
    text: String,
    selected: bool,
    white: String,
    black: String,
    result: String,
    date: String,
    site: String,
    time_control: String,
}

impl BatchPgnGame {
    fn new(text: String) -> Self {
        Self {
            white: ChessApp::pgn_tag(&text, "White").unwrap_or_else(|| "White".into()),
            black: ChessApp::pgn_tag(&text, "Black").unwrap_or_else(|| "Black".into()),
            result: ChessApp::pgn_tag(&text, "Result").unwrap_or_else(|| "*".into()),
            date: ChessApp::pgn_tag(&text, "Date").unwrap_or_default(),
            site: ChessApp::pgn_tag(&text, "Site").unwrap_or_default(),
            time_control: ChessApp::pgn_tag(&text, "TimeControl").unwrap_or_default(),
            text,
            selected: false,
        }
    }

    fn includes_player(&self, player: &str) -> bool {
        self.white == player || self.black == player
    }
}

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize, Default)]
struct BatchImportStatus {
    pending: usize,
    saved: usize,
    failed: usize,
    #[serde(rename = "firstError")]
    first_error: Option<String>,
}

#[derive(Deserialize, Default)]
struct VersionDiagnostics {
    status: String,
    environment: Option<String>,
    app_version: Option<String>,
    package_version: Option<String>,
    engine_version: Option<String>,
    server_app_version: Option<String>,
    server_package_version: Option<String>,
    server_engine_version: Option<String>,
    service_worker_version: Option<String>,
    service_worker_engine_version: Option<String>,
    service_worker_state: Option<String>,
    online: Option<bool>,
    cross_origin_isolated: Option<bool>,
    browser: Option<String>,
}

#[cfg(target_arch = "wasm32")]
use std::sync::mpsc::{self, Receiver};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, JsValue, closure::Closure};

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodOpenScoresheet)]
    fn open_scoresheet();
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodPollScoresheet)]
    fn poll_scoresheet() -> String;
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodFetchLichess)]
    fn fetch_lichess(input: &str);
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodPollLichess)]
    fn poll_lichess() -> String;
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodLichessStatus)]
    fn lichess_status() -> String;
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodCopyBoard)]
    fn copy_board_image(fen: &str, settings: &str);
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodBeginBoardPngCopy)]
    fn begin_board_png_copy() -> bool;
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodFinishBoardPngCopy)]
    fn finish_board_png_copy(png: &js_sys::Uint8Array, coordinates: &str);
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodFailBoardPngCopy)]
    fn fail_board_png_copy(message: &str);
    #[wasm_bindgen::prelude::wasm_bindgen(catch, js_namespace = window, js_name = ironwoodOpenPositionEditor)]
    fn open_position_editor(fen: &str) -> Result<(), JsValue>;
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodPollPositionEditor)]
    fn poll_position_editor() -> String;
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodPlayChessSound)]
    fn play_chess_sound(kind: &str);
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodStoreCurrentGame)]
    fn store_current_game(json: &str);
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodStoreObservedGame)]
    fn store_observed_game(json: &str, id: &str);
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodStoreImportedGame)]
    fn store_imported_game(json: &str);
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodStartNewStoredGame)]
    fn start_new_stored_game(category: &str);
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodIsImportedDuplicate)]
    fn is_imported_duplicate(pgn: &str) -> bool;
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodImportedIndexReady)]
    fn imported_index_ready() -> bool;
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodEnsureImportedIndex)]
    fn ensure_imported_index();
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodRefreshStorageStatus)]
    fn refresh_storage_status();
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodStorageStatus)]
    fn storage_status() -> String;
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodBeginBatchImport)]
    fn begin_batch_import_storage();
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodBatchImportStatus)]
    fn batch_import_storage_status() -> String;
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodEndBatchImport)]
    fn end_batch_import_storage();
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodOpenGameLibrary)]
    fn open_game_library();
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodBackupStorage)]
    fn backup_storage();
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodRestoreStorage)]
    fn restore_storage();
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodOpenStorageInfo)]
    fn open_storage_info();
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodClearSavedGames)]
    fn clear_saved_games();
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodResetLocalData)]
    fn reset_local_data();
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodVersionDiagnostics)]
    fn version_diagnostics() -> String;
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodRefreshVersionDiagnostics)]
    fn refresh_version_diagnostics();
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodFicsConnect)]
    fn fics_connect();
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodFicsConnectRegistered)]
    fn fics_connect_registered(username: &str, password: &str);
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodFicsDisconnect)]
    fn fics_disconnect();
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodFicsSend)]
    fn fics_send(command: &str);
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodFicsSendCommand)]
    fn fics_send_command(command: &str);
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodFicsPoll)]
    fn fics_poll() -> String;
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum FicsEvent {
    Status {
        message: String,
        connected: bool,
        #[serde(default)]
        registered: bool,
        #[serde(default)]
        transport_open: bool,
    },
    Line {
        message: String,
    },
    History {
        game: i32,
        moves: Vec<String>,
    },
    Board {
        fen: String,
        game: i32,
        white: String,
        black: String,
        side: String,
        last_move: String,
        #[serde(default)]
        last_san: String,
        white_time: i32,
        black_time: i32,
        #[serde(default)]
        move_number: i32,
        #[serde(default)]
        observing: bool,
    },
    End {
        message: String,
        #[serde(default)]
        observing: bool,
        #[serde(default)]
        game: Option<i32>,
    },
    Unobserved {
        #[serde(default)]
        game: Option<i32>,
    },
    Observationstopped {
        game: i32,
    },
}

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

#[derive(Clone, Copy)]
struct MoveAnimation {
    chess_move: ChessMove,
    piece: Piece,
    color: Color,
    started_at: f64,
}

#[derive(Clone, Copy)]
struct BestMoveAttempt {
    target_index: usize,
    best_move: ChessMove,
    attempted_move: Option<ChessMove>,
    revealed: bool,
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

#[derive(Deserialize)]
struct ImportedAnalysisFile {
    schema: String,
    engine: ImportedAnalysisEngine,
    game: ImportedAnalysisGame,
    positions: Vec<ImportedAnalysisPosition>,
    #[serde(default)]
    verification: Option<ImportedVerification>,
}

#[derive(Deserialize)]
struct ImportedVerification {
    nodes_per_position: u64,
    target_positions: Vec<usize>,
}

#[derive(Deserialize)]
struct ImportedAnalysisEngine {
    threads: u32,
    hash_mib: u32,
    full_game_quality: String,
    nodes_per_position: u64,
}

#[derive(Deserialize)]
struct ImportedAnalysisGame {
    white: String,
    black: String,
    moves: Vec<String>,
    #[serde(default)]
    date: Option<String>,
    #[serde(default)]
    site: Option<String>,
    #[serde(default)]
    result: Option<String>,
    #[serde(default)]
    source_pgn: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct BoardMark {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    style: String,
    color: char,
    from: String,
    to: String,
}

impl BoardMark {
    fn valid(&self) -> bool {
        (self.style.is_empty() || matches!(self.style.as_str(), "dashed" | "curve-left" | "curve-right" | "circle" | "dotted-square" | "dotted-circle"))
            && matches!(self.color, 'G' | 'R' | 'Y' | 'B')
            && Square::from_str(&self.from).is_ok() && Square::from_str(&self.to).is_ok()
    }
}

#[derive(Deserialize)]
struct ImportedAnalysisPosition {
    fen: String,
    evaluation_cp: Option<i32>,
    mate: Option<i32>,
    depth: Option<u32>,
    nodes: Option<u64>,
    best_move: Option<String>,
    principal_variation: Option<String>,
    #[serde(default)]
    note: String,
    #[serde(default)]
    annotations: Vec<u8>,
    #[serde(default)]
    board_marks: Vec<BoardMark>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MoveClassification {
    Best,
    Good,
    Inaccuracy,
    Mistake,
    Blunder,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum MateOutcome {
    Found,
    Missed,
    Allowed,
    Escaped,
}

impl MateOutcome {
    const ALL: [Self; 4] = [Self::Found, Self::Missed, Self::Allowed, Self::Escaped];

    fn color(self) -> Color32 {
        match self {
            Self::Found | Self::Escaped => Color32::from_rgb(112, 193, 133),
            Self::Missed => Color32::from_rgb(230, 163, 77),
            Self::Allowed => Color32::from_rgb(223, 104, 91),
        }
    }

    fn between(
        before: Option<i32>,
        after: Option<i32>,
        mover: Color,
        checkmate_after_move: bool,
    ) -> Option<Self> {
        let factor = if mover == Color::White { 1 } else { -1 };
        let before = before.map(|mate| mate * factor);
        let after = after.map(|mate| mate * factor);
        if checkmate_after_move {
            return (!before.is_some_and(|mate| mate > 0)).then_some(Self::Found);
        }
        if after.is_some_and(|mate| mate < 0) && !before.is_some_and(|mate| mate < 0) {
            Some(Self::Allowed)
        } else if before.is_some_and(|mate| mate > 0) && !after.is_some_and(|mate| mate > 0) {
            Some(Self::Missed)
        } else if before.is_some_and(|mate| mate < 0) && !after.is_some_and(|mate| mate < 0) {
            Some(Self::Escaped)
        } else if after.is_some_and(|mate| mate > 0) && !before.is_some_and(|mate| mate > 0) {
            Some(Self::Found)
        } else {
            None
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Found => "Forced mate found",
            Self::Missed => "Forced mate missed",
            Self::Allowed => "Forced mate allowed",
            Self::Escaped => "Forced mate escaped",
        }
    }

    fn explanation(self) -> &'static str {
        match self {
            Self::Found => "Stockfish now sees a forced mate for the player who moved.",
            Self::Missed => {
                "Stockfish saw a forced mate before this move but no longer sees it afterward."
            }
            Self::Allowed => "Stockfish now sees a forced mate for the opponent.",
            Self::Escaped => "Stockfish no longer sees the opponent's forced mate after this move.",
        }
    }

    fn classification(self) -> MoveClassification {
        match self {
            Self::Found | Self::Escaped => MoveClassification::Best,
            Self::Missed | Self::Allowed => MoveClassification::Blunder,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GamePhase {
    Opening,
    Middlegame,
    Endgame,
}

impl GamePhase {
    fn color(self) -> Color32 {
        match self {
            Self::Opening => Color32::from_rgb(97, 181, 211),
            Self::Middlegame => Color32::from_rgb(191, 153, 224),
            Self::Endgame => Color32::from_rgb(108, 192, 143),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Opening => "Opening",
            Self::Middlegame => "Middlegame",
            Self::Endgame => "Endgame",
        }
    }
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
const PREFERENCES_KEY: &str = "ironwood.chess.preferences.v1";
#[cfg(target_arch = "wasm32")]
const ENGINE_CACHE_KEY: &str = "ironwood.chess.engine.stockfish-19.ready";

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
struct EngineConfig {
    opponent: OpponentEngine,
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
enum OpponentEngine {
    #[default]
    #[serde(alias = "Lc0")]
    Stockfish,
}

impl OpponentEngine {
    fn label(self) -> &'static str {
        match self {
            Self::Stockfish => "Stockfish 19",
        }
    }
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
            opponent: OpponentEngine::Stockfish,
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
    System,
    Cburnett,
    #[default]
    #[serde(alias = "Minecraft")]
    Merida,
    RoyalRascals,
    UndeadCourt,
}

impl PieceSet {
    fn label(self) -> &'static str {
        match self {
            Self::System => "System (Unicode)",
            Self::Cburnett => "Cburnett",
            Self::Merida => "Merida",
            Self::RoyalRascals => "Royal Rascals",
            Self::UndeadCourt => "Undead Court",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
enum PlayerSide {
    #[default]
    White,
    Black,
}

impl PlayerSide {
    fn color(self) -> Color {
        match self {
            Self::White => Color::White,
            Self::Black => Color::Black,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum NewGameColor {
    #[default]
    White,
    Black,
    Random,
}

#[derive(Clone, Serialize, Deserialize)]
struct LocalClock {
    white: f64,
    black: f64,
    increment: f64,
    flagged_white: Option<bool>,
    #[serde(skip)]
    updated_at: f64,
}

impl LocalClock {
    fn advance(&mut self, now: f64, white_to_move: bool, running: bool) -> bool {
        let elapsed = (now - self.updated_at).max(0.0);
        self.updated_at = now;
        if !running || self.flagged_white.is_some() {
            return false;
        }
        let remaining = if white_to_move {
            &mut self.white
        } else {
            &mut self.black
        };
        *remaining = (*remaining - elapsed).max(0.0);
        if *remaining == 0.0 {
            self.flagged_white = Some(white_to_move);
            return true;
        }
        false
    }
}

struct FicsRunningGame {
    id: i32,
    label: String,
}
impl FicsRunningGame {
    fn parse(line: &str) -> Option<Self> {
        let line = line.trim().trim_start_matches("fics%").trim();
        let (players, rest) = line.split_once('[')?;
        let players: Vec<_> = players.split_whitespace().collect();
        if players.len() != 5 {
            return None;
        }
        let id = players[0].parse().ok()?;
        let flags: Vec<_> = rest.split_once(']')?.0.split_whitespace().collect();
        if flags.len() != 3
            || !matches!(
                flags[0],
                "br" | "bu" | "lr" | "lu" | "sr" | "su" | "ur" | "uu"
            )
        {
            return None;
        }
        let minutes: u32 = flags[1].parse().ok()?;
        let increment: u32 = flags[2].parse().ok()?;
        Some(Self {
            id,
            label: format!(
                "#{} · {} ({}) vs {} ({}) · {}+{}",
                id, players[2], players[1], players[4], players[3], minutes, increment
            ),
        })
    }
}

#[derive(Clone)]
struct ObservedGame {
    id: i32,
    storage_id: String,
    white: String,
    black: String,
    board: Board,
    positions: Vec<Board>,
    moves: Vec<String>,
    start_ply: usize,
    white_time: i32,
    black_time: i32,
    board_at: f64,
    flipped: bool,
    finished: bool,
    ended: bool,
    end_message: Option<String>,
    analysis: Option<ObservedAnalysis>,
    move_notes: Vec<String>,
    move_annotations: Vec<Vec<u8>>,
    board_marks: Vec<Vec<BoardMark>>,
}

#[derive(Clone)]
struct ObservedAnalysis {
    results: Vec<Option<PositionAnalysis>>,
    index: Option<usize>,
    paused: bool,
    verification_targets: Vec<usize>,
    verification_results: Vec<Option<PositionAnalysis>>,
    verification_nodes: u64,
    config: FullGameAnalysisConfig,
}

#[derive(Clone)]
struct FicsAd {
    id: String,
    player: String,
    rating: String,
    minutes: String,
    increment: String,
    category: String,
    rated: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FicsConsoleTone {
    Plain,
    Command,
    Chat,
    Game,
    Warning,
    Error,
}

const FICS_CONSOLE_COMMANDS: [(&str, &str); 15] = [
    ("help", "FICS help"),
    ("who a", "Available players"),
    ("finger ", "Player profile"),
    ("sought", "Open game ads"),
    ("games", "Live games"),
    ("observe ", "Watch a game"),
    ("unobserve", "Stop watching"),
    ("getgame", "Find a blitz game"),
    ("unseek", "Cancel game search"),
    ("+channel 53", "Listen to Guest Chat"),
    ("inchannel 53", "Channel listeners"),
    ("tell ", "Private message"),
    ("say ", "Message opponent"),
    ("variables", "Account settings"),
    ("history", "Recent games"),
];

impl FicsConsoleTone {
    fn for_line(line: &str) -> Self {
        let trimmed = line.trim_start();
        if trimmed.starts_with("> ") {
            return Self::Command;
        }
        let lower = trimmed.to_ascii_lowercase();
        if lower.contains("illegal move")
            || lower.contains("unknown command")
            || lower.contains("connection failed")
            || lower.contains("rejected")
            || lower.starts_with("error:")
        {
            return Self::Error;
        }
        if lower.contains("offers you")
            || lower.contains("offer from")
            || lower.contains("challenge from")
            || lower.contains("mute list")
            || lower.contains("warning:")
        {
            return Self::Warning;
        }
        if trimmed.starts_with("{Game ")
            || lower.starts_with("creating:")
            || lower.contains("game has started")
            || lower.contains("game is over")
        {
            return Self::Game;
        }
        let channel_message = trimmed
            .split_once(':')
            .and_then(|(prefix, _)| prefix.rsplit_once('('))
            .and_then(|(_, channel)| channel.strip_suffix(')'))
            .is_some_and(|channel| {
                !channel.is_empty() && channel.bytes().all(|byte| byte.is_ascii_digit())
            });
        if channel_message
            || lower.contains(" tells you:")
            || lower.contains(" says:")
            || lower.contains(" kibitz")
            || lower.contains(" whispers:")
            || lower.contains(" shouts:")
        {
            return Self::Chat;
        }
        Self::Plain
    }

    fn color(self) -> Color32 {
        match self {
            Self::Plain => Color32::from_rgb(194, 198, 200),
            Self::Command => Color32::from_rgb(142, 158, 166),
            Self::Chat => Color32::from_rgb(125, 181, 222),
            Self::Game => Color32::from_rgb(118, 193, 141),
            Self::Warning => Color32::from_rgb(221, 177, 96),
            Self::Error => Color32::from_rgb(227, 120, 110),
        }
    }
}

impl FicsAd {
    fn parse(line: &str) -> Option<Self> {
        let line = line.trim().strip_prefix("fics%").unwrap_or(line).trim();
        let parts: Vec<_> = line.split_whitespace().collect();
        if parts.len() < 7 || parts[0].len() > 4 || !parts[0].chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        if !parts[3].chars().all(|c| c.is_ascii_digit())
            || !parts[4].chars().all(|c| c.is_ascii_digit())
            || !matches!(parts[5], "rated" | "unrated")
        {
            return None;
        }
        Some(Self {
            id: parts[0].into(),
            rating: parts[1].into(),
            player: parts[2].into(),
            minutes: parts[3].into(),
            increment: parts[4].into(),
            rated: parts[5].into(),
            category: parts[6].into(),
        })
    }
}

fn fics_game_pgn(white: &str, black: &str, result: &str, moves: &[String]) -> String {
    let mut pgn = format!(
        "[Event \"FICS game\"]\n[Site \"freechess.org\"]\n[White \"{white}\"]\n[Black \"{black}\"]\n[Result \"{result}\"]\n\n"
    );
    for (index, san) in moves.iter().enumerate() {
        if index % 2 == 0 {
            pgn.push_str(&format!("{}. ", index / 2 + 1));
        }
        pgn.push_str(san);
        pgn.push(' ');
    }
    pgn.push_str(result);
    pgn
}

#[derive(Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
enum WorkspaceMode {
    #[default]
    #[serde(alias = "Expanded")]
    Compact,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum CompactPanel {
    #[default]
    Moves,
    Analysis,
}

#[derive(Serialize, Deserialize)]
struct PersistedGame {
    #[serde(default)]
    training: Option<Session>,
    #[serde(default)]
    local_clock: Option<LocalClock>,
    #[serde(default)]
    local_resigned_white: Option<bool>,
    #[serde(default)]
    local_start_ply: usize,
    board: String,
    #[serde(default)]
    final_board: String,
    #[serde(default)]
    result: String,
    history: Vec<String>,
    last_move: Option<String>,
    flipped: bool,
    #[serde(default, skip_serializing)]
    workspace_mode: WorkspaceMode,
    #[serde(default = "default_best_move_arrows", skip_serializing)]
    show_coordinates: bool,
    #[serde(default = "default_best_move_arrows", skip_serializing)]
    show_board_frame: bool,
    #[serde(default = "default_best_move_arrows", skip_serializing)]
    piece_shadows: bool,
    #[serde(default = "default_best_move_arrows", skip_serializing)]
    show_best_move_arrows: bool,
    #[serde(default = "default_best_move_arrows", skip_serializing)]
    show_move_hover_text: bool,
    #[serde(default, skip_serializing)]
    figurine_notation: bool,
    #[serde(default = "default_best_move_arrows", skip_serializing)]
    move_sounds: bool,
    #[serde(default = "default_best_move_arrows", skip_serializing)]
    animate_moves: bool,
    #[serde(default, skip_serializing)]
    piece_set: PieceSet,
    engine_enabled: bool,
    #[serde(default)]
    player_side: PlayerSide,
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
    #[serde(default)]
    move_notes: Vec<String>,
    #[serde(default)]
    move_annotations: Vec<Vec<u8>>,
    #[serde(default)]
    board_marks: Vec<Vec<BoardMark>>,
    #[serde(default)]
    game_analysis_running: bool,
    #[serde(default)]
    game_analysis_paused: bool,
    #[serde(default)]
    verification_targets: Vec<usize>,
    #[serde(default)]
    verification_results: Vec<Option<PositionAnalysis>>,
    #[serde(default)]
    verification_nodes: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
struct UserPreferences {
    workspace_mode: WorkspaceMode,
    show_coordinates: bool,
    show_highlighted_move: bool,
    show_radial_light: bool,
    show_board_frame: bool,
    piece_shadows: bool,
    show_best_move_arrows: bool,
    show_move_hover_text: bool,
    figurine_notation: bool,
    move_sounds: bool,
    animate_moves: bool,
    piece_set: PieceSet,
    board_3d_active: bool,
    board_3d_theme: crate::board3d::Theme,
    board_3d_appearance: u8,
    board_3d_appearance_customized: bool,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            workspace_mode: WorkspaceMode::default(),
            show_coordinates: true,
            show_highlighted_move: true,
            show_radial_light: true,
            show_board_frame: true,
            piece_shadows: true,
            show_best_move_arrows: true,
            show_move_hover_text: true,
            figurine_notation: false,
            move_sounds: true,
            animate_moves: true,
            piece_set: PieceSet::default(),
            board_3d_active: false,
            board_3d_theme: crate::board3d::Theme::Marble,
            board_3d_appearance: 85,
            board_3d_appearance_customized: false,
        }
    }
}

impl UserPreferences {
    fn from_game(game: &PersistedGame) -> Self {
        Self {
            workspace_mode: game.workspace_mode,
            show_coordinates: game.show_coordinates,
            show_highlighted_move: true,
            show_radial_light: true,
            show_board_frame: game.show_board_frame,
            piece_shadows: game.piece_shadows,
            show_best_move_arrows: game.show_best_move_arrows,
            show_move_hover_text: game.show_move_hover_text,
            figurine_notation: game.figurine_notation,
            move_sounds: game.move_sounds,
            animate_moves: game.animate_moves,
            piece_set: game.piece_set,
            board_3d_active: false,
            board_3d_theme: crate::board3d::Theme::Marble,
            board_3d_appearance: 85,
            board_3d_appearance_customized: false,
        }
    }
}

pub struct ChessApp {
    training: Option<Session>,
    training_profiles: Profiles,
    training_dialog_open: bool,
    training_dialog_maximized: bool,
    training_profile_name: String,
    training_profile_rename: Option<String>,
    training_error: Option<String>,
    new_game_training: bool,
    board: Board,
    board_3d_active: bool,
    board_3d_theme: crate::board3d::Theme,
    board_3d_distance: f32,
    board_3d_yaw: f32,
    board_3d_elevation: f32,
    board_3d_appearance: u8,
    board_3d_appearance_customized: bool,
    board_3d_rendered_distance: f32,
    board_3d_zoom_until: f64,
    board_3d_render_key: String,
    board_3d_texture: Option<egui::TextureHandle>,
    board_3d_gpu_format: Option<eframe::wgpu::TextureFormat>,
    board_3d_gpu_key: String,
    board_3d_gpu_scene: Option<crate::board3d::gpu::Scene>,
    board_3d_gpu_scene_id: u64,
    selected: Option<Square>,
    legal_targets: Vec<Square>,
    history: Vec<Board>,
    last_move: Option<ChessMove>,
    promotion: Option<(Square, Square)>,
    typed_move: String,
    typed_move_error: bool,
    typed_move_board: Option<Board>,
    flipped: bool,
    workspace_mode: WorkspaceMode,
    compact_panel: CompactPanel,
    show_coordinates: bool,
    show_highlighted_move: bool,
    show_radial_light: bool,
    show_board_frame: bool,
    piece_shadows: bool,
    show_best_move_arrows: bool,
    show_move_hover_text: bool,
    figurine_notation: bool,
    engine_analysis_dock_collapsed: bool,
    move_sounds: bool,
    animate_moves: bool,
    move_animation: Option<MoveAnimation>,
    piece_set: PieceSet,
    engine_enabled: bool,
    fics_active: bool,
    fics_available_was_open: bool,
    fics_available_open_this_frame: bool,
    fics_observe_was_open: bool,
    fics_observe_open_this_frame: bool,
    fics_running_games: Vec<FicsRunningGame>,
    fics_console_open: bool,
    fics_chats: crate::fics_chat::Chats,
    fics_console_input: String,
    fics_console_selected_text: String,
    fics_console_suggestions_open: bool,
    fics_console_suggestion_index: usize,
    fics_resign_dialog_open: bool,
    fics_connected: bool,
    fics_registered: bool,
    fics_playing: bool,
    fics_observing: bool,
    fics_observe_target: String,
    fics_observation_start_ply: Option<usize>,
    fics_observed_games: Vec<ObservedGame>,
    fics_game_finished: bool,
    fics_seeking: bool,
    fics_pending_move: bool,
    fics_applying_update: bool,
    fics_game_id: Option<i32>,
    fics_status: String,
    fics_log: Vec<String>,
    fics_ads: Vec<FicsAd>,
    fics_minutes: i32,
    fics_increment: i32,
    fics_player: String,
    fics_challenge_open: bool,
    fics_challenge_error: String,
    fics_sign_in_open: bool,
    fics_username: String,
    fics_password: String,
    fics_sign_in_error: String,
    fics_white_time: i32,
    fics_black_time: i32,
    fics_turn: Color,
    fics_board_at: f64,
    fics_previous_workspace_mode: WorkspaceMode,
    player_side: PlayerSide,
    new_game_dialog_open: bool,
    new_game_color: NewGameColor,
    new_game_online: bool,
    new_game_both_sides: bool,
    new_game_opponent: OpponentEngine,
    new_game_limit_strength: bool,
    new_game_elo: u32,
    new_game_time: usize,
    new_game_minutes: u32,
    new_game_increment: u32,
    new_game_position: usize,
    new_game_fen: String,
    local_clock: Option<LocalClock>,
    local_resigned_white: Option<bool>,
    local_start_ply: usize,
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
    elo_calculator_open: bool,
    elo_your_rating: i32,
    elo_opponent_rating: i32,
    elo_k_factor: i32,
    elo_system: usize,
    elo_result: usize,
    pgn_dialog_open: bool,
    batch_pgn_dialog_open: bool,
    batch_pgn_games: Vec<BatchPgnGame>,
    batch_pgn_players: Vec<String>,
    batch_pgn_player_filter: Option<String>,
    batch_pgn_player_search: String,
    batch_pgn_page: usize,
    batch_pgn_queue: std::collections::VecDeque<String>,
    batch_pgn_total: usize,
    batch_pgn_checked: usize,
    batch_pgn_imported: usize,
    batch_pgn_duplicates: usize,
    batch_pgn_failed: usize,
    batch_pgn_storage_failed_seen: usize,
    batch_pgn_first_error: Option<String>,
    batch_pgn_result_open: bool,
    batch_pgn_stopped: bool,
    pgn_analyze_after_import: bool,
    import_input: String,
    lichess_input: String,
    pgn_input: String,
    pgn_error: Option<String>,
    print_confirm_open: bool,
    expanded_graph_open: bool,
    expanded_graph_hover_index: Option<usize>,
    print_notice_until: Option<f64>,
    review_positions: Vec<Board>,
    review_moves: Vec<String>,
    review_index: Option<usize>,
    review_scroll_to_selected: bool,
    best_move_attempt: Option<BestMoveAttempt>,
    review_white_player: String,
    review_black_player: String,
    analysis_running: bool,
    realtime_analysis_due_at: Option<f64>,
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
    move_notes: Vec<String>,
    move_annotations: Vec<Vec<u8>>,
    board_marks: Vec<Vec<BoardMark>>,
    drawing_undo: Vec<(usize, Vec<BoardMark>)>,
    board_mark_mode: bool,
    board_mark_color: char,
    board_mark_drag: Option<(usize, Square, char)>,
    note_editor_index: Option<usize>,
    note_editor_text: String,
    game_analysis_index: Option<usize>,
    game_analysis_running: bool,
    game_analysis_paused: bool,
    verification_targets: Vec<usize>,
    verification_results: Vec<Option<PositionAnalysis>>,
    verification_nodes: u64,
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    resume_game_analysis_after_ready: bool,
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    engine_ready: bool,
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    show_engine_download: bool,
    resume_engine_after_ready: bool,
    pending_analysis_search: Option<String>,
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    ignore_next_bestmove: bool,
    #[cfg(target_arch = "wasm32")]
    engine: Option<EngineBridge>,
}

impl ChessApp {
    fn tick_local_clock(&mut self) {
        if self.training.is_some() && self.game_result() != "*" { return; }
        if self.fics_active || self.local_resigned_white.is_some() {
            return;
        }
        let live_board = self.review_positions.last().copied().unwrap_or(self.board);
        let Some(clock) = &mut self.local_clock else {
            return;
        };
        let now = Self::animation_time();
        let white = live_board.side_to_move() == Color::White;
        let running = live_board.status() == BoardStatus::Ongoing
            && (!self.engine_enabled || self.engine_ready);
        if clock.advance(now, white, running) {
            self.engine_searching = false;
            self.resume_engine_after_ready = false;
            self.engine_status =
                format!("{} ran out of time", if white { "White" } else { "Black" });
            #[cfg(target_arch = "wasm32")]
            if let Some(engine) = &self.engine {
                engine.command("stop");
            }
            self.save_game();
        }
    }

    fn receive_observed_board(
        &mut self,
        game: i32,
        white: String,
        black: String,
        server_board: Board,
        last_move: &str,
        last_san: &str,
        move_number: i32,
        white_time: i32,
        black_time: i32,
        flipped: bool,
    ) {
        let start_ply = (move_number.max(1) as usize - 1) * 2
            + usize::from(server_board.side_to_move() == Color::Black);
        let now = Self::animation_time();
        let first_observed = self.fics_observed_games.is_empty();
        let mut live_move = None;
        if let Some(observed) = self
            .fics_observed_games
            .iter_mut()
            .find(|item| item.id == game)
        {
            let previous = observed.board;
            if !Self::same_fics_position(&previous, &server_board) {
                if let Some(moves) =
                    Self::fics_transition(&previous, &server_board, last_move, last_san)
                {
                    let mut position = previous;
                    for mv in moves {
                        live_move = Some((position, mv));
                        observed.moves.push(Self::san_for_move(&position, mv));
                        position = position.make_move_new(mv);
                        observed.positions.push(position);
                    }
                } else {
                    observed.positions = vec![server_board];
                    observed.moves.clear();
                    observed.start_ply = start_ply;
                    self.fics_log.push(format!(
                        "Game #{game}: move history restarted after a missed update."
                    ));
                }
            }
            observed.board = server_board;
            observed.white_time = white_time;
            observed.black_time = black_time;
            observed.board_at = now;
            observed.finished = false;
            observed.ended = false;
            if let Some(last) = observed.positions.last_mut() {
                *last = server_board;
            }
        } else {
            if self.fics_observed_games.len() >= 10 {
                return;
            }
            self.fics_observed_games.push(ObservedGame {
                id: game,
                storage_id: format!("{game}-{now}"),
                white,
                black,
                board: server_board,
                positions: vec![server_board],
                moves: Vec::new(),
                start_ply,
                white_time,
                black_time,
                board_at: now,
                flipped,
                finished: false,
                ended: false,
                end_message: None,
                analysis: None,
                move_notes: Vec::new(),
                move_annotations: Vec::new(),
                board_marks: Vec::new(),
            });
        }
        if !self.fics_playing && (first_observed || self.fics_game_id == Some(game)) {
            self.show_observed_game(game);
            if let Some((previous, mv)) = live_move {
                self.live_move_effects(&previous, mv);
            }
        }
    }

    fn live_move_effects(&mut self, previous: &Board, mv: ChessMove) {
        let Some(piece) = previous.piece_on(mv.get_source()) else {
            return;
        };
        let castle = previous.castle_side(mv);
        let destination = castle
            .map(|kingside| {
                Square::make_square(
                    mv.get_source().get_rank(),
                    if kingside { File::G } else { File::C },
                )
            })
            .unwrap_or(mv.get_dest());
        self.move_animation =
            (self.animate_moves && destination != mv.get_source()).then(|| MoveAnimation {
                chess_move: ChessMove::new(mv.get_source(), destination, mv.get_promotion()),
                piece,
                color: previous.side_to_move(),
                started_at: Self::animation_time(),
            });
        #[cfg(target_arch = "wasm32")]
        if self.move_sounds {
            let capture = castle.is_none()
                && (previous.piece_on(mv.get_dest()).is_some()
                    || (piece == Piece::Pawn
                        && mv.get_source().get_file() != mv.get_dest().get_file()));
            let sound = if self.board.checkers().popcnt() > 0 {
                "check"
            } else if capture {
                "capture"
            } else {
                "move"
            };
            play_chess_sound(sound);
        }
    }

    fn show_observed_game(&mut self, game: i32) {
        if self.fics_playing {
            return;
        }
        let Some(observed) = self
            .fics_observed_games
            .iter()
            .find(|item| item.id == game)
            .cloned()
        else {
            return;
        };
        let switching = self.fics_game_id != Some(game);
        if switching {
            if let Some(id) = self.fics_game_id {
                self.save_observed_game(id);
            }
            if let Some(previous) = self
                .fics_observed_games
                .iter_mut()
                .find(|item| Some(item.id) == self.fics_game_id)
            {
                previous.flipped = self.flipped;
                previous.move_notes = self.move_notes.clone();
                previous.move_annotations = self.move_annotations.clone();
                previous.board_marks = self.board_marks.clone();
                previous.analysis = Some(ObservedAnalysis {
                    results: self.game_analysis.clone(),
                    index: self.game_analysis_index,
                    paused: self.game_analysis_running || self.game_analysis_paused,
                    verification_targets: self.verification_targets.clone(),
                    verification_results: self.verification_results.clone(),
                    verification_nodes: self.verification_nodes,
                    config: self.full_game_analysis_config.clone(),
                });
            }
            self.fics_observing = true;
            self.reset_for_side(if observed.flipped {
                PlayerSide::Black
            } else {
                PlayerSide::White
            });
            if let Some(analysis) = &observed.analysis {
                self.game_analysis = analysis.results.clone();
                self.game_analysis_index = analysis.index;
                self.game_analysis_paused = analysis.paused;
                self.verification_targets = analysis.verification_targets.clone();
                self.verification_results = analysis.verification_results.clone();
                self.verification_nodes = analysis.verification_nodes;
                self.full_game_analysis_config = analysis.config.clone();
            }
        }
        if switching {
            self.move_notes = observed.move_notes.clone();
            self.move_annotations = observed.move_annotations.clone();
            self.drawing_undo.clear();
            self.board_marks = observed.board_marks.clone();
            self.board_mark_drag = None;
        }
        self.board = observed.board;
        self.review_positions = observed.positions.clone();
        self.review_moves = observed.moves.clone();
        self.history = observed
            .positions
            .iter()
            .copied()
            .take(observed.positions.len().saturating_sub(1))
            .collect();
        self.last_move = Self::review_move_at(
            &self.review_positions,
            &self.review_moves,
            self.review_moves.len(),
        );
        self.review_index = None;
        self.review_scroll_to_selected = true;
        self.selected = None;
        self.legal_targets.clear();
        self.flipped = observed.flipped;
        self.fics_game_id = Some(game);
        self.fics_observing = !observed.finished;
        self.fics_game_finished = observed.finished;

        self.fics_white_time = observed.white_time;
        self.fics_black_time = observed.black_time;
        self.fics_turn = observed.board.side_to_move();
        self.fics_board_at = observed.board_at;
        self.fics_observation_start_ply = Some(observed.start_ply);
        self.review_white_player = observed.white;
        self.review_black_player = observed.black;
        self.fics_status = if observed.ended {
            format!("Observed game #{game} ended")
        } else if observed.finished {
            format!("No longer observing game #{game}")
        } else {
            format!("Observing game #{game}")
        };
        self.engine_status = format!("FICS · {}", self.fics_status);
    }

    fn receive_observed_history(&mut self, game: i32, sans: Vec<String>) {
        let Some(index) = self
            .fics_observed_games
            .iter()
            .position(|item| item.id == game)
        else {
            return;
        };
        let observed = &self.fics_observed_games[index];
        let mut positions = vec![Board::default()];
        let mut moves = Vec::with_capacity(sans.len() + observed.moves.len());
        for san in sans {
            let previous = *positions.last().unwrap();
            let Some(chess_move) = Self::parse_san_move(&previous, &san) else {
                self.fics_log.push(format!(
                    "Game #{game}: FICS move history could not be read."
                ));
                return;
            };
            moves.push(Self::san_for_move(&previous, chess_move));
            positions.push(previous.make_move_new(chess_move));
        }
        let live_index = observed.start_ply + observed.moves.len();
        if positions
            .get(live_index)
            .is_some_and(|board| Self::same_fics_position(board, &observed.board))
        {
            // The server can list a move before its live style-12 board reaches us.
            positions.truncate(live_index + 1);
            moves.truncate(live_index);
        } else if let Some(overlap) = observed
            .positions
            .iter()
            .position(|board| Self::same_fics_position(board, positions.last().unwrap()))
        {
            // The live board can also move ahead while FICS prepares the list.
            for san in observed.moves.iter().skip(overlap) {
                let previous = *positions.last().unwrap();
                let Some(chess_move) = Self::parse_san_move(&previous, san) else {
                    return;
                };
                moves.push(Self::san_for_move(&previous, chess_move));
                positions.push(previous.make_move_new(chess_move));
            }
        } else {
            self.fics_log.push(format!(
                "Game #{game}: FICS move history did not match the live board."
            ));
            return;
        }
        if !Self::same_fics_position(positions.last().unwrap(), &observed.board) {
            self.fics_log.push(format!(
                "Game #{game}: FICS move history did not match the live board."
            ));
            return;
        }
        let observed = &mut self.fics_observed_games[index];
        if observed.start_ply > 0 {
            let mut notes = vec![String::new(); observed.start_ply];
            notes.append(&mut observed.move_notes);
            observed.move_notes = notes;
            let mut annotations = vec![Vec::new(); observed.start_ply];
            annotations.append(&mut observed.move_annotations);
            observed.move_annotations = annotations;
            let mut marks = vec![Vec::new(); observed.start_ply];
            marks.append(&mut observed.board_marks);
            observed.board_marks = marks;
            if self.fics_game_id == Some(game) {
                self.move_notes = observed.move_notes.clone();
                self.move_annotations = observed.move_annotations.clone();
            self.drawing_undo.clear();
            self.board_marks = observed.board_marks.clone();
            self.board_mark_drag = None;
            }
        }
        observed.start_ply = 0;
        observed.positions = positions;
        observed.moves = moves;
        self.save_observed_game(game);
        if self.fics_game_id == Some(game) && !self.fics_playing {
            self.show_observed_game(game);
        }
    }

    fn remove_observed_game(&mut self, game: i32) {
        let Some(index) = self
            .fics_observed_games
            .iter()
            .position(|item| item.id == game)
        else {
            return;
        };
        self.fics_observed_games.remove(index);
        if self.fics_game_id == Some(game) && !self.fics_playing {
            self.fics_game_id = None;
            self.fics_observing = false;
            if let Some(next) = self
                .fics_observed_games
                .get(index.min(self.fics_observed_games.len().saturating_sub(1)))
            {
                self.show_observed_game(next.id);
            } else {
                self.fics_game_finished = false;
                self.fics_status = "Stopped observing".into();
            }
        }
    }

    fn start_fics(&mut self) {
        self.settle_training();
        self.training = None;
        self.new_game_training = false;
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(engine) = &self.engine {
                engine.command("stop");
            }
            fics_connect();
        }
        self.engine_enabled = false;
        self.engine_searching = false;
        self.analysis_running = false;
        self.fics_active = true;

        self.fics_console_open = false;
        self.fics_console_input.clear();
        self.fics_console_selected_text.clear();
        self.fics_console_suggestions_open = false;
        self.fics_resign_dialog_open = false;
        self.fics_connected = false;
        self.fics_registered = false;
        self.fics_playing = false;
        self.fics_observing = false;
        self.fics_observe_target.clear();
        self.fics_observation_start_ply = None;
        self.fics_observed_games.clear();
        self.fics_game_finished = false;
        self.fics_seeking = false;
        self.fics_pending_move = false;
        self.fics_status = "Connecting to FICS…".into();
        self.fics_log.clear();
        self.fics_chats = crate::fics_chat::Chats::default();
        self.fics_ads.clear();
        self.fics_challenge_open = false;
        self.fics_challenge_error.clear();
        self.fics_sign_in_open = false;
        self.fics_password.clear();
        self.fics_sign_in_error.clear();
        self.reset_for_side(PlayerSide::White);
        self.fics_previous_workspace_mode = self.workspace_mode;
        self.workspace_mode = WorkspaceMode::Compact;
        self.compact_panel = CompactPanel::Moves;
        self.review_white_player = "White".into();
        self.review_black_player = "Black".into();
        self.engine_status = "Connecting to FICS…".into();
    }

    fn stop_fics(&mut self) {
        #[cfg(target_arch = "wasm32")]
        fics_disconnect();
        self.fics_active = false;

        self.fics_console_open = false;
        self.fics_console_input.clear();
        self.fics_console_selected_text.clear();
        self.fics_console_suggestions_open = false;
        self.fics_resign_dialog_open = false;
        self.fics_connected = false;
        self.fics_registered = false;
        self.fics_playing = false;
        self.fics_observing = false;
        self.fics_observe_target.clear();
        self.fics_observation_start_ply = None;
        self.fics_observed_games.clear();
        self.fics_game_finished = false;
        self.fics_seeking = false;
        self.fics_pending_move = false;
        self.fics_game_id = None;
        self.fics_status = "Offline".into();
        self.fics_challenge_open = false;
        self.fics_challenge_error.clear();
        self.fics_sign_in_open = false;
        self.fics_password.clear();
        self.workspace_mode = self.fics_previous_workspace_mode;
        self.engine_status = "FICS disconnected · start a new game to play locally".into();
    }

    #[cfg(target_arch = "wasm32")]
    fn poll_fics(&mut self, ctx: &egui::Context) {
        if !self.fics_active {
            return;
        }
        for _ in 0..32 {
            let json = fics_poll();
            if json.is_empty() {
                break;
            }
            let Ok(event) = serde_json::from_str::<FicsEvent>(&json) else {
                continue;
            };
            match event {
                FicsEvent::Status {
                    message,
                    connected,
                    registered,
                    transport_open,
                } => {
                    if transport_open || (connected && !self.fics_connected) {
                        self.fics_console_open = true;
                    }
                    self.fics_connected = connected;
                    self.fics_registered = connected && registered;
                    if message.contains("rejected")
                        || message.contains("did not request a password")
                    {
                        self.fics_sign_in_open = true;
                        self.fics_sign_in_error = message.clone();
                        self.fics_password.clear();
                    }
                    self.fics_status = message;
                    if !connected {
                        self.fics_running_games.clear();
                        self.fics_ads.clear();
                        self.fics_playing = false;
                        self.fics_observing = false;
                        self.fics_game_id = None;
                        self.fics_observed_games.clear();
                        self.fics_seeking = false;
                        self.fics_pending_move = false;
                    }
                }
                FicsEvent::Line { message } => {
                    self.fics_chats.receive(&message, self.fics_console_open);
                    if let Some(game) = FicsRunningGame::parse(&message) {
                        if let Some(existing) =
                            self.fics_running_games.iter_mut().find(|g| g.id == game.id)
                        {
                            *existing = game;
                        } else {
                            self.fics_running_games.push(game);
                        }
                    }
                    if let Some(ad) = FicsAd::parse(&message) {
                        if let Some(existing) =
                            self.fics_ads.iter_mut().find(|item| item.id == ad.id)
                        {
                            *existing = ad;
                        } else {
                            self.fics_ads.push(ad);
                        }
                    }
                    if message.to_ascii_lowercase().contains("illegal move") {
                        self.fics_pending_move = false;
                    }
                    self.fics_log.push(message);
                    if self.fics_log.len() > 80 {
                        self.fics_log.remove(0);
                    }
                }
                FicsEvent::History { game, moves } => {
                    self.receive_observed_history(game, moves);
                }
                FicsEvent::End {
                    message,
                    observing,
                    game,
                } => {
                    if observing {
                        if let Some(game) = game {
                            if let Some(item) = self
                                .fics_observed_games
                                .iter_mut()
                                .find(|item| item.id == game)
                            {
                                item.finished = true;
                                item.ended = true;
                                item.end_message = Some(message.clone());
                            }
                            if self.fics_game_id == Some(game) && !self.fics_playing {
                                self.show_observed_game(game);
                            }
                            self.save_observed_game(game);
                        }
                        self.fics_log.push(message);
                        continue;
                    }
                    self.fics_playing = false;
                    self.fics_observing = false;
                    self.fics_game_finished = true;
                    self.fics_seeking = false;
                    self.fics_pending_move = false;
                    self.fics_status = message.clone();
                    self.fics_log.push(message.clone());
                    if !self.review_moves.is_empty() {
                        let result = ["1-0", "0-1", "1/2-1/2"]
                            .into_iter()
                            .find(|result| message.contains(result))
                            .unwrap_or("*");
                        self.pgn_input = fics_game_pgn(
                            &self.review_white_player,
                            &self.review_black_player,
                            result,
                            &self.review_moves,
                        );
                        self.review_index = Some(self.review_moves.len());
                        self.review_scroll_to_selected = true;
                        self.compact_panel = CompactPanel::Analysis;
                        self.engine_status =
                            "FICS game finished · ready for Stockfish analysis".into();
                        start_new_stored_game("mine");
                        self.save_game();
                    }
                }
                FicsEvent::Board {
                    fen,
                    game,
                    white,
                    black,
                    side,
                    last_move,
                    last_san,
                    white_time,
                    black_time,
                    move_number,
                    observing,
                } => {
                    let Ok(server_board) = Board::from_str(&fen) else {
                        if observing {
                            self.fics_status =
                                "This FICS game uses a board Ironwood cannot display".into();
                        }
                        continue;
                    };
                    if observing {
                        self.receive_observed_board(
                            game,
                            white,
                            black,
                            server_board,
                            &last_move,
                            &last_san,
                            move_number,
                            white_time,
                            black_time,
                            side == "black",
                        );
                        continue;
                    }
                    let is_new = self.fics_game_id != Some(game) || self.fics_observing;
                    if is_new {
                        self.fics_game_finished = false;
                        self.fics_observing = false;
                        let player_side = if side == "black" {
                            PlayerSide::Black
                        } else {
                            PlayerSide::White
                        };
                        self.reset_for_side(player_side);
                        self.fics_game_id = Some(game);

                        self.review_white_player = white;
                        self.review_black_player = black;
                        self.fics_observation_start_ply = None;
                    }
                    let anchor = self.review_positions.last().copied().unwrap_or(self.board);
                    if !is_new && !Self::same_fics_position(&anchor, &server_board) {
                        if let Some(moves) =
                            Self::fics_transition(&anchor, &server_board, &last_move, &last_san)
                        {
                            self.board = anchor;
                            self.fics_applying_update = true;
                            for mv in moves {
                                self.play(mv);
                            }
                            self.fics_applying_update = false;
                        } else {
                            self.fics_log.push(
                                "FICS move history could not be reconciled; earlier moves were kept."
                                    .into(),
                            );
                        }
                    }
                    self.board = server_board;
                    if let Some(last) = self.review_positions.last_mut()
                        && Self::same_fics_position(last, &server_board)
                    {
                        *last = server_board;
                    }
                    self.selected = None;
                    self.legal_targets.clear();
                    self.fics_playing = true;
                    self.fics_observing = false;
                    self.fics_seeking = false;
                    self.fics_pending_move = false;
                    self.fics_white_time = white_time;
                    self.fics_black_time = black_time;
                    self.fics_turn = self.board.side_to_move();
                    self.fics_board_at = Self::animation_time();
                    self.fics_status = if self.board.side_to_move() == self.player_side.color() {
                        "Your move".into()
                    } else {
                        "Opponent's move".into()
                    };
                    self.engine_status = format!("FICS · {}", self.fics_status);
                }
                FicsEvent::Unobserved { game } => {
                    if let Some(game) = game {
                        self.remove_observed_game(game);
                    } else {
                        let active = self.fics_game_id;
                        self.fics_observed_games.clear();
                        if !self.fics_playing && active.is_some() {
                            self.fics_observing = false;
                            self.fics_game_id = None;

                            self.fics_status = "Stopped observing".into();
                        }
                    }
                }
                FicsEvent::Observationstopped { game } => {
                    if let Some(item) = self
                        .fics_observed_games
                        .iter_mut()
                        .find(|item| item.id == game)
                    {
                        item.finished = true;
                    }
                    if self.fics_game_id == Some(game) && !self.fics_playing {
                        self.show_observed_game(game);
                    }
                }
            }
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }

    fn animation_time() -> f64 {
        #[cfg(target_arch = "wasm32")]
        {
            js_sys::Date::now() / 1_000.0
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64()
        }
    }

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
            if game.review_pgn.is_some() || (game.live_moves.is_empty() && game.training.is_none()) {
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
        let player_side = saved
            .as_ref()
            .map(|game| game.player_side)
            .unwrap_or_default();
        let review_white_player = saved.as_ref().and_then(|g| g.training.as_ref()).map(|t| if t.side == TrainingSide::White { t.profile_name.clone() } else { format!("Stockfish {} Elo", t.opponent_elo) }).or_else(|| review_pgn
            .and_then(|pgn| Self::pgn_tag(pgn, "White")))
            .unwrap_or_else(|| {
                if saved.as_ref().is_some_and(|game| !game.engine_enabled) {
                    "White".to_owned()
                } else if player_side == PlayerSide::White {
                    "You".to_owned()
                } else {
                    saved
                        .as_ref()
                        .map(|game| game.engine_config.opponent)
                        .unwrap_or_default()
                        .label()
                        .to_owned()
                }
            });
        let review_black_player = saved.as_ref().and_then(|g| g.training.as_ref()).map(|t| if t.side == TrainingSide::Black { t.profile_name.clone() } else { format!("Stockfish {} Elo", t.opponent_elo) }).or_else(|| review_pgn
            .and_then(|pgn| Self::pgn_tag(pgn, "Black")))
            .unwrap_or_else(|| {
                if saved.as_ref().is_some_and(|game| !game.engine_enabled) {
                    "Black".to_owned()
                } else if player_side == PlayerSide::Black {
                    "You".to_owned()
                } else {
                    saved
                        .as_ref()
                        .map(|game| game.engine_config.opponent)
                        .unwrap_or_default()
                        .label()
                        .to_owned()
                }
            });
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
        let mut game_analysis = saved
            .as_ref()
            .map(|game| game.game_analysis.clone())
            .filter(|analysis| analysis.len() == review_positions.len())
            .unwrap_or_default();
        let (mut game_analysis_index, mut game_analysis_running, mut game_analysis_paused) =
            Self::analysis_resume_state(
                &game_analysis,
                saved
                    .as_ref()
                    .is_some_and(|game| game.game_analysis_running),
                saved.as_ref().is_some_and(|game| game.game_analysis_paused),
            );
        let verification_targets = saved
            .as_ref()
            .map(|game| game.verification_targets.clone())
            .unwrap_or_default();
        let verification_results = saved
            .as_ref()
            .map(|game| game.verification_results.clone())
            .filter(|results| results.len() == review_positions.len())
            .unwrap_or_default();
        let verification_targets = if !game_analysis.is_empty()
            && game_analysis.iter().all(Option::is_some)
            && verification_results.len() == review_positions.len()
            && verification_targets
                .iter()
                .all(|index| *index < review_positions.len())
        {
            verification_targets
        } else {
            Vec::new()
        };
        let verification_nodes = saved
            .as_ref()
            .map(|game| game.verification_nodes)
            .unwrap_or_default();
        if !verification_targets.is_empty()
            && verification_targets
                .iter()
                .all(|index| verification_results[*index].is_some())
        {
            for index in &verification_targets {
                game_analysis[*index] = verification_results[*index].clone();
            }
        }
        if let Some(next) =
            Self::pending_verification_target(&verification_targets, &verification_results)
        {
            let was_running = saved
                .as_ref()
                .is_some_and(|game| game.game_analysis_running);
            let was_paused = saved.as_ref().is_some_and(|game| game.game_analysis_paused);
            if was_running || was_paused {
                game_analysis_index = Some(next);
                game_analysis_running = was_running && !was_paused;
                game_analysis_paused = was_paused;
            }
        }
        let flipped = saved.as_ref().is_some_and(|game| game.flipped);
        let preferences = Self::load_preferences().unwrap_or_else(|| {
            saved
                .as_ref()
                .map(UserPreferences::from_game)
                .unwrap_or_default()
        });
        let workspace_mode = preferences.workspace_mode;
        let show_coordinates = preferences.show_coordinates;
        let show_highlighted_move = preferences.show_highlighted_move;
        let show_radial_light = preferences.show_radial_light;
        let show_board_frame = preferences.show_board_frame;
        let piece_shadows = preferences.piece_shadows;
        let show_best_move_arrows = preferences.show_best_move_arrows;
        let show_move_hover_text = preferences.show_move_hover_text;
        let figurine_notation = preferences.figurine_notation;
        let move_sounds = preferences.move_sounds;
        let animate_moves = preferences.animate_moves;
        let piece_set = preferences.piece_set;
        let board_3d_appearance = if preferences.board_3d_appearance_customized {
            preferences.board_3d_appearance.min(100)
        } else {
            85
        };
        let engine_enabled = saved
            .as_ref()
            .map(|game| game.engine_enabled)
            .unwrap_or(true);
        let resume_engine_after_ready = !game_analysis_running
            && !game_analysis_paused
            && engine_enabled
            && review_index.is_none()
            && board.side_to_move() != player_side.color()
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

        #[allow(unused_mut)]
        let (training_profiles, training_error) = match Self::read_training_profiles() {
            Ok(profiles) => (profiles, None),
            Err(error) => (Profiles::default(), Some(error)),
        };
        let app = Self {
            training: saved.as_ref().and_then(|game| game.training.clone()),
            training_profiles,
            training_dialog_open: false,
            training_dialog_maximized: false,
            training_profile_name: String::new(),
            training_profile_rename: None,
            training_error,
            new_game_training: false,
            board,
            board_3d_active: preferences.board_3d_active,
            board_3d_theme: preferences.board_3d_theme,
            board_3d_distance: crate::board3d::View::default().distance,
            board_3d_yaw: 0.0,
            board_3d_elevation: crate::board3d::View::default().elevation,
            board_3d_appearance,
            board_3d_appearance_customized: preferences.board_3d_appearance_customized,
            board_3d_rendered_distance: crate::board3d::View::default().distance,
            board_3d_zoom_until: 0.0,
            board_3d_render_key: String::new(),
            board_3d_texture: None,
            board_3d_gpu_format: cc.wgpu_render_state.as_ref().map(|state| state.target_format),
            board_3d_gpu_key: String::new(),
            board_3d_gpu_scene: None,
            board_3d_gpu_scene_id: 0,
            selected: None,
            legal_targets: vec![],
            history,
            last_move,
            promotion: None,
            typed_move: String::new(),
            typed_move_error: false,
            typed_move_board: None,
            flipped,
            workspace_mode,
            compact_panel: CompactPanel::default(),
            show_coordinates,
            show_highlighted_move,
            show_radial_light,
            show_board_frame,
            piece_shadows,
            show_best_move_arrows,
            show_move_hover_text,
            figurine_notation,
            engine_analysis_dock_collapsed: false,
            move_sounds,
            animate_moves,
            move_animation: None,
            piece_set,
            engine_enabled,
            fics_active: false,
            fics_available_was_open: false,
            fics_available_open_this_frame: false,
            fics_observe_was_open: false,
            fics_observe_open_this_frame: false,
            fics_running_games: Vec::new(),
            fics_console_open: false,
            fics_chats: crate::fics_chat::Chats::default(),
            fics_console_input: String::new(),
            fics_console_selected_text: String::new(),
            fics_console_suggestions_open: false,
            fics_console_suggestion_index: 0,
            fics_resign_dialog_open: false,
            fics_connected: false,
            fics_registered: false,
            fics_playing: false,
            fics_observing: false,
            fics_observe_target: String::new(),
            fics_observation_start_ply: None,
            fics_observed_games: Vec::new(),
            fics_game_finished: false,
            fics_seeking: false,
            fics_pending_move: false,
            fics_applying_update: false,
            fics_game_id: None,
            fics_status: "Offline".into(),
            fics_log: Vec::new(),
            fics_ads: Vec::new(),
            fics_minutes: 5,
            fics_increment: 0,
            fics_player: String::new(),
            fics_challenge_open: false,
            fics_challenge_error: String::new(),
            fics_sign_in_open: false,
            fics_username: String::new(),
            fics_password: String::new(),
            fics_sign_in_error: String::new(),
            fics_white_time: 0,
            fics_black_time: 0,
            fics_turn: Color::White,
            fics_board_at: 0.0,
            fics_previous_workspace_mode: workspace_mode,
            player_side,
            new_game_dialog_open: false,
            new_game_color: NewGameColor::White,
            new_game_online: false,
            new_game_both_sides: false,
            new_game_opponent: engine_config.opponent,
            new_game_limit_strength: engine_config.limit_strength,
            new_game_elo: engine_config.elo.clamp(1320, 3190),
            new_game_time: 1,
            new_game_minutes: 5,
            new_game_increment: 0,
            new_game_position: 0,
            new_game_fen: Board::default().to_string(),
            local_start_ply: saved.as_ref().map(|game| game.local_start_ply).unwrap_or(0),
            local_resigned_white: saved.as_ref().and_then(|game| game.local_resigned_white),
            local_clock: saved
                .as_ref()
                .and_then(|game| game.local_clock.clone())
                .map(|mut clock| {
                    clock.updated_at = Self::animation_time();
                    clock
                }),
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
            elo_calculator_open: false,
            elo_your_rating: 1200,
            elo_opponent_rating: 1400,
            elo_k_factor: 20,
            elo_system: 0,
            elo_result: 0,
            pgn_dialog_open: false,
            batch_pgn_dialog_open: false,
            batch_pgn_games: Vec::new(),
            batch_pgn_players: Vec::new(),
            batch_pgn_player_filter: None,
            batch_pgn_player_search: String::new(),
            batch_pgn_page: 0,
            batch_pgn_queue: std::collections::VecDeque::new(),
            batch_pgn_total: 0,
            batch_pgn_checked: 0,
            batch_pgn_imported: 0,
            batch_pgn_duplicates: 0,
            batch_pgn_failed: 0,
            batch_pgn_storage_failed_seen: 0,
            batch_pgn_first_error: None,
            batch_pgn_result_open: false,
            batch_pgn_stopped: false,
            pgn_analyze_after_import: false,
            import_input: String::new(),
            lichess_input: String::new(),
            pgn_input: saved
                .as_ref()
                .and_then(|game| game.review_pgn.clone())
                .unwrap_or_default(),
            pgn_error: None,
            print_confirm_open: false,
            expanded_graph_open: false,
            expanded_graph_hover_index: None,
            print_notice_until: None,
            review_positions,
            review_moves,
            review_index,
            review_scroll_to_selected: review_index.is_some(),
            best_move_attempt: None,
            review_white_player,
            review_black_player,
            analysis_running: false,
            realtime_analysis_due_at: None,
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
            move_notes: saved
                .as_ref()
                .map(|game| game.move_notes.clone())
                .unwrap_or_default(),
            move_annotations: saved.as_ref().map(|game| game.move_annotations.clone()).unwrap_or_default(),
            board_marks: saved.as_ref().map(|game| game.board_marks.clone()).unwrap_or_default(),
            drawing_undo: Vec::new(),
            board_mark_mode: false,
            board_mark_color: 'G',
            board_mark_drag: None,
            note_editor_index: None,
            note_editor_text: String::new(),
            game_analysis,
            game_analysis_index,
            game_analysis_running,
            game_analysis_paused,
            verification_targets,
            verification_results,
            verification_nodes,
            resume_game_analysis_after_ready: game_analysis_running,
            engine_ready: false,
            show_engine_download,
            resume_engine_after_ready,
            pending_analysis_search: None,
            ignore_next_bestmove: false,
            #[cfg(target_arch = "wasm32")]
            engine: EngineBridge::new(cc.egui_ctx.clone(), &engine_config),
        };
        app
    }

    #[cfg(target_arch = "wasm32")]
    fn engine_thread_count() -> u32 {
        web_sys::window()
            .map(|window| window.navigator().hardware_concurrency() as u32)
            .unwrap_or(2)
            .clamp(1, ENGINE_MAX_THREADS)
    }

    fn version_diagnostics_snapshot() -> VersionDiagnostics {
        #[cfg(target_arch = "wasm32")]
        {
            serde_json::from_str(&version_diagnostics()).unwrap_or_else(|_| VersionDiagnostics {
                status: "Version diagnostics are unavailable".into(),
                ..Default::default()
            })
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            VersionDiagnostics {
                status: "Native development build".into(),
                environment: Some("Native development".into()),
                ..Default::default()
            }
        }
    }

    fn version_diagnostics_report(details: &VersionDiagnostics) -> String {
        format!(
            "Ironwood Chess diagnostics\nStatus: {}\nApp: v{} · build {}\nEnvironment: {}\nLoaded shell: {}\nServer shell: {}\nLoaded WASM package: {}\nServer WASM package: {}\nLoaded engine: {}\nServer engine: {}\nService worker: {}\nService-worker shell: {}\nService-worker engine: {}\nOnline: {}\nCross-origin isolated: {}\nBrowser: {}",
            details.status,
            env!("CARGO_PKG_VERSION"),
            env!("IRONWOOD_BUILD_ID"),
            Self::diagnostic_value(&details.environment),
            Self::diagnostic_value(&details.app_version),
            Self::diagnostic_value(&details.server_app_version),
            Self::diagnostic_value(&details.package_version),
            Self::diagnostic_value(&details.server_package_version),
            Self::diagnostic_value(&details.engine_version),
            Self::diagnostic_value(&details.server_engine_version),
            Self::diagnostic_value(&details.service_worker_state),
            Self::diagnostic_value(&details.service_worker_version),
            Self::diagnostic_value(&details.service_worker_engine_version),
            details
                .online
                .map_or("unavailable".into(), |v| v.to_string()),
            details
                .cross_origin_isolated
                .map_or("unavailable".into(), |v| v.to_string()),
            Self::diagnostic_value(&details.browser),
        )
    }

    fn diagnostic_value(value: &Option<String>) -> &str {
        value.as_deref().unwrap_or("Unavailable")
    }

    fn context_menu_frame(ctx: &egui::Context) -> Frame {
        Frame::popup(&ctx.style()).stroke(Stroke::new(1.5, Color32::from_rgb(211, 173, 98)))
    }

    fn style_context_menu(ui: &mut egui::Ui) {
        Self::set_menu_item_font(ui);
    }

    fn gold_menu_style(style: &mut egui::Style) {
        egui::containers::menu::menu_style(style);
        style.visuals.window_stroke = Stroke::new(1.5, Color32::from_rgb(211, 173, 98));
    }

    fn gold_menu_button<R>(
        ui: &mut egui::Ui,
        title: &'static str,
        contents: impl FnOnce(&mut egui::Ui) -> R,
    ) {
        egui::containers::menu::MenuButton::new(title)
            .config(egui::containers::menu::MenuConfig::new().style(Self::gold_menu_style))
            .ui(ui, contents);
    }

    fn set_menu_item_font(ui: &mut egui::Ui) {
        ui.style_mut()
            .text_styles
            .insert(egui::TextStyle::Button, FontId::proportional(15.0));
        ui.style_mut()
            .text_styles
            .insert(egui::TextStyle::Body, FontId::proportional(15.0));
        ui.style_mut().spacing.button_padding.y = 2.0;
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

    fn elo_expected_score(your_rating: i32, opponent_rating: i32) -> f64 {
        1.0 / (1.0 + 10.0_f64.powf((opponent_rating - your_rating) as f64 / 400.0))
    }

    fn dialog_frame() -> Frame {
        Frame::new()
            .fill(Color32::from_rgb(23, 26, 30))
            .stroke(Stroke::new(1.0, Color32::from_rgb(211, 173, 98)))
            .corner_radius(CornerRadius::same(12))
            .inner_margin(Margin::same(18))
    }

    fn dialog_header(ui: &mut egui::Ui, title: &str) -> bool {
        Self::dialog_header_with_maximize(ui, title, None)
    }

    fn dialog_header_with_maximize(ui: &mut egui::Ui, title: &str, maximized: Option<&mut bool>) -> bool {
        // Dialog chrome follows the dark Scoresheet palette in either app theme.
        *ui.visuals_mut() = egui::Visuals::dark();
        let mut close = false;
        ui.horizontal(|ui| {
            ui.label(RichText::new(title).size(21.0).strong().color(Color32::from_rgb(238, 238, 238)));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let gold = Color32::from_rgb(211, 173, 98);
                ui.spacing_mut().button_padding = Vec2::new(12.0, 8.0);
                close = ui.add(egui::Button::new(RichText::new("×").size(14.0)
                    .color(Color32::from_rgb(244, 221, 176)))
                    .fill(Color32::from_rgb(41, 40, 32))
                    .stroke(Stroke::new(1.0, gold))
                    .corner_radius(CornerRadius::same(6)))
                    .on_hover_text(format!("Close {title}")).clicked();
                if let Some(maximized) = maximized {
                    if ui.add(egui::Button::new(RichText::new(if *maximized { "❐" } else { "□" }).size(14.0)
                        .color(Color32::from_rgb(244, 221, 176)))
                        .fill(Color32::from_rgb(41, 40, 32))
                        .stroke(Stroke::new(1.0, gold))
                        .corner_radius(CornerRadius::same(6)))
                        .on_hover_text(if *maximized { "Restore window" } else { "Maximize window" }).clicked() {
                        *maximized = !*maximized;
                    }
                }
            });
        });
        ui.add_space(10.0);
        close
    }

    fn elo_calculator_dialog(&mut self, ctx: &egui::Context) {
        if !self.elo_calculator_open {
            return;
        }
        let mut close = false;
        let width = (ctx.screen_rect().width() - 64.0).clamp(280.0, 540.0);
        let response = egui::Modal::new(egui::Id::new("elo_calculator"))
            .frame(Self::dialog_frame())
            .show(ctx, |ui| {
                ui.set_width(width);
                close = Self::dialog_header(ui, "Elo calculator");
                ui.label("Estimate your expected score and rating change for one game.");
                ui.add_space(12.0);

                ui.columns(2, |columns| {
                    for (column, (label, rating)) in columns.iter_mut().zip([
                        ("Your rating", &mut self.elo_your_rating),
                        ("Opponent rating", &mut self.elo_opponent_rating),
                    ]) {
                        column.label(RichText::new(label).strong());
                        column.horizontal(|ui| {
                            if ui.button("−").on_hover_text("Decrease by 50").clicked() {
                                *rating -= 50;
                            }
                            ui.add(egui::DragValue::new(rating).range(100..=4000).speed(1.0));
                            if ui.button("+").on_hover_text("Increase by 50").clicked() {
                                *rating += 50;
                            }
                            ui.add_sized([88.0, 20.0],
                                egui::Slider::new(rating, 100..=4000).show_value(false));
                        });
                        *rating = (*rating).clamp(100, 4000);
                    }
                });
                ui.add_space(8.0);

                let expected = Self::elo_expected_score(self.elo_your_rating, self.elo_opponent_rating);
                let pct = (expected * 100.0).round() as i32;
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("You {pct}%")).strong()
                        .color(Color32::from_rgb(211, 173, 98)));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(format!("Opponent {}%", 100 - pct));
                    });
                });
                ui.add(egui::ProgressBar::new(expected as f32).desired_width(width));
                ui.add_space(12.0);
                ui.label(RichText::new("If you...").strong());
                let results = [("Win", 1.0), ("Draw", 0.5), ("Loss", 0.0)];
                ui.columns(3, |columns| {
                    for (index, (label, score)) in results.iter().enumerate() {
                        let change = (self.elo_k_factor as f64 * (score - expected)).round() as i32;
                        if columns[index].selectable_label(self.elo_result == index,
                            format!("{label}\n{change:+} points")).clicked() {
                            self.elo_result = index;
                        }
                    }
                });
                let score = results[self.elo_result].1;
                let change = (self.elo_k_factor as f64 * (score - expected)).round() as i32;
                ui.add_space(10.0);
                Frame::new()
                    .fill(Color32::from_rgb(43, 39, 29))
                    .stroke(Stroke::new(1.0, Color32::from_rgb(211, 173, 98)))
                    .corner_radius(6.0)
                    .inner_margin(Margin::same(12))
                    .show(ui, |ui| {
                        ui.set_min_width(width - 24.0);
                        ui.label("Estimated new rating");
                        ui.label(RichText::new(format!("{}  ({change:+})",
                            self.elo_your_rating + change)).size(25.0).strong()
                            .color(Color32::from_rgb(211, 173, 98)));
                    });
                ui.add_space(8.0);
                ui.collapsing("Advanced", |ui| {
                    let systems = ["FIDE / standard Elo", "Chess.com estimate", "Lichess estimate"];
                    egui::ComboBox::from_label("Rating system")
                        .selected_text(systems[self.elo_system])
                        .show_ui(ui, |ui| {
                            for (index, name) in systems.iter().enumerate() {
                                if ui.selectable_value(&mut self.elo_system, index, *name).changed() {
                                    self.elo_k_factor = [20, 32, 40][index];
                                }
                            }
                        });
                    ui.add(egui::Slider::new(&mut self.elo_k_factor, 5..=60).text("K-factor"));
                });
                if self.elo_system == 0 {
                    ui.label(RichText::new("Elo estimate: new rating = current rating + K × (result − expected score). Official FIDE changes may use additional rules.").small().weak());
                } else {
                    ui.label(RichText::new("Illustrative Elo estimate only. Chess.com and Lichess use Glicko-based systems, so actual changes depend on rating uncertainty and may differ.").small().weak());
                }
            });
        if close || response.should_close() {
            self.elo_calculator_open = false;
        }
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
                        ui.add(
                            egui::Image::new(egui::include_image!(
                                "../web/brand/ironwood-logo-256.webp"
                            ))
                            .fit_to_exact_size(Vec2::splat(128.0))
                            .corner_radius(CornerRadius::same(64)),
                        );
                        ui.add_space(12.0);
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
                    let diagnostics = Self::version_diagnostics_snapshot();
                    egui::CollapsingHeader::new("Version diagnostics")
                        .id_salt("about_version_diagnostics")
                        .show(ui, |ui| {
                            let is_match = diagnostics.status.starts_with("All loaded");
                            let status_color = if is_match {
                                Color32::from_rgb(104, 190, 128)
                            } else if diagnostics.status.contains("mismatch") {
                                Color32::from_rgb(230, 119, 112)
                            } else {
                                muted
                            };
                            ui.label(
                                RichText::new(&diagnostics.status)
                                    .strong()
                                    .color(status_color),
                            );
                            ui.add_space(6.0);
                            egui::Grid::new("about_version_grid")
                                .num_columns(2)
                                .spacing([16.0, 4.0])
                                .show(ui, |ui| {
                                    for (label, detail) in [
                                        (
                                            "Environment",
                                            Self::diagnostic_value(&diagnostics.environment),
                                        ),
                                        (
                                            "Loaded shell",
                                            Self::diagnostic_value(&diagnostics.app_version),
                                        ),
                                        (
                                            "Server shell",
                                            Self::diagnostic_value(&diagnostics.server_app_version),
                                        ),
                                        (
                                            "Loaded WASM",
                                            Self::diagnostic_value(&diagnostics.package_version),
                                        ),
                                        (
                                            "Server WASM",
                                            Self::diagnostic_value(
                                                &diagnostics.server_package_version,
                                            ),
                                        ),
                                        (
                                            "Loaded engine",
                                            Self::diagnostic_value(&diagnostics.engine_version),
                                        ),
                                        (
                                            "Server engine",
                                            Self::diagnostic_value(
                                                &diagnostics.server_engine_version,
                                            ),
                                        ),
                                        (
                                            "Service worker",
                                            Self::diagnostic_value(
                                                &diagnostics.service_worker_state,
                                            ),
                                        ),
                                        (
                                            "Worker shell",
                                            Self::diagnostic_value(
                                                &diagnostics.service_worker_version,
                                            ),
                                        ),
                                    ] {
                                        ui.label(RichText::new(label).color(muted));
                                        ui.label(RichText::new(detail).monospace().size(11.0));
                                        ui.end_row();
                                    }
                                });
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                if ui.button("Refresh").clicked() {
                                    #[cfg(target_arch = "wasm32")]
                                    refresh_version_diagnostics();
                                    ctx.request_repaint_after(std::time::Duration::from_millis(
                                        500,
                                    ));
                                }
                                if ui.button("Copy diagnostics").clicked() {
                                    ctx.copy_text(Self::version_diagnostics_report(&diagnostics));
                                }
                                ui.label(
                                    RichText::new("No games or personal data included.")
                                        .size(10.0)
                                        .color(muted),
                                );
                            });
                        });
                    ui.add_space(16.0);
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
                            RichText::new("Ironwood Chess: GPLv3 or later · Stockfish: GPLv3")
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
        #[cfg(target_arch = "wasm32")]
        {
            let pgn = poll_lichess();
            if !pgn.is_empty() {
                self.import_input = pgn;
                self.pgn_error = None;
            }
            let scoresheet = poll_scoresheet();
            if !scoresheet.is_empty() {
                self.import_input = scoresheet;
                self.pgn_error = None;
            }
        }
        let mut import = false;
        let mut cancel = false;
        let response = egui::Modal::new(egui::Id::new("import_pgn"))
            .frame(Self::dialog_frame())
            .show(ctx, |ui| {
                let gold = Color32::from_rgb(211, 173, 98);
                ui.set_width((ctx.screen_rect().width() - 72.0).clamp(240.0, 620.0));
                ui.spacing_mut().item_spacing.y = 8.0;
                cancel = Self::dialog_header(ui, "Import Games");
                ui.label(RichText::new("Bring your games into Ironwood for review and analysis.").weak());
                ui.add_space(8.0);
                egui::ScrollArea::vertical()
                    .id_salt("import_options")
                    .max_height((ctx.screen_rect().height() - 200.0).max(150.0))
                    .show_gold(ui, |ui| {
                #[cfg(target_arch = "wasm32")]
                Frame::new()
                    .fill(ui.visuals().faint_bg_color)
                    .stroke(Stroke::new(1.0, Color32::from_rgb(90, 77, 51)))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(Margin::same(14))
                    .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new("From Lichess").size(17.0).strong().color(gold));
                    ui.label(RichText::new("Your latest 20 completed games, or a single game link. No login needed.").size(13.0).weak());
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut self.lichess_input)
                            .hint_text("Lichess username or game URL")
                            .desired_width((ui.available_width() - 100.0).max(80.0)));
                        if ui.add_enabled(!self.lichess_input.trim().is_empty(), egui::Button::new("Fetch games")
                            .stroke(Stroke::new(1.0, gold))).clicked() {
                            ensure_imported_index();
                            fetch_lichess(&self.lichess_input);
                        }
                    });
                    let status = lichess_status();
                    if !status.is_empty() { ui.label(RichText::new(status).size(13.0).weak()); }
                    ctx.request_repaint_after(std::time::Duration::from_millis(200));
                });
                ui.add_space(4.0);
                #[cfg(target_arch = "wasm32")]
                Frame::new()
                    .fill(ui.visuals().faint_bg_color)
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(Margin::same(14))
                    .show(ui, |ui| {
                        ui.label(RichText::new("From a scoresheet").size(17.0).strong().color(gold));
                        ui.label(RichText::new("Enter moves beside your scoresheet photo, with a board preview and live validation.").size(13.0).weak());
                        if ui.button("Enter scoresheet").clicked() {
                            open_scoresheet();
                        }
                    });
                Frame::new()
                    .fill(ui.visuals().faint_bg_color)
                    .stroke(Stroke::new(1.0, Color32::from_rgb(90, 77, 51)))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(Margin::same(14))
                    .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new("Paste a game or collection").size(17.0).strong().color(gold));
                ui.label(RichText::new("Paste PGN or analysis JSON, or drag a .pgn or .json file onto the app.").size(13.0).weak());
                egui::ScrollArea::vertical()
                    .id_salt("import_game_text")
                    .max_height(180.0)
                    .auto_shrink([false, true])
                    .show_gold(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.import_input)
                                .desired_width(ui.available_width())
                                .desired_rows(7)
                                .hint_text("[Event \"Example\"]\n[White \"Player 1\"]\n[Black \"Player 2\"]\n\n1. e4 e5 2. Nf3 Nc6 ...")
                                .font(egui::TextStyle::Monospace),
                        );
                    });
                ui.label(RichText::new("Analysis JSON restores your saved analysis immediately.").size(12.0).weak());
                });
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
                        "Uses your Full game quality setting. You can pause or stop analysis.",
                    )
                    .size(13.0)
                    .color(ui.visuals().weak_text_color()),
                );
                });
                ui.add_space(10.0);
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Clear").clicked() {
                        self.import_input.clear();
                        self.pgn_error = None;
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        import = ui
                            .add_enabled(!self.import_input.trim().is_empty(), egui::Button::new(RichText::new("Continue").strong().color(Color32::from_rgb(30, 26, 20)))
                                .fill(gold).min_size(Vec2::new(110.0, 34.0)))
                            .clicked();
                    });
                });
        });
        if import {
            self.begin_import();
        } else if cancel || response.should_close() {
            self.pgn_dialog_open = false;
            self.pgn_error = None;
        }
    }

    fn fide_profile_url(pgn: &str, white: bool) -> Option<String> {
        let id = Self::pgn_tag(pgn, if white { "WhiteFideId" } else { "BlackFideId" })?;
        let id = id.trim();
        (id.bytes().all(|ch| ch.is_ascii_digit()) && id.bytes().any(|ch| ch != b'0'))
            .then(|| format!("https://ratings.fide.com/profile/{id}"))
    }

    fn player_name_ui(&self, ui: &mut egui::Ui, name: &str, white: bool) {
        let text = RichText::new(name).size(17.0).strong();
        if let Some(url) = Self::fide_profile_url(&self.pgn_input, white) {
            let response = ui.link(text.color(Color32::from_rgb(211, 173, 98)))
                .on_hover_text("Open FIDE profile in a new tab");
            if response.clicked() {
                ui.ctx().open_url(egui::OpenUrl::new_tab(url));
            }
        } else {
            ui.label(text);
        }
    }

    fn batch_pgn_dialog(&mut self, ctx: &egui::Context) {
        if !self.batch_pgn_dialog_open {
            return;
        }
        let mut import = false;
        let mut cancel = false;
        let selected = self
            .batch_pgn_games
            .iter()
            .filter(|game| game.selected)
            .count();
        let filtered_indices: Vec<usize> = self
            .batch_pgn_games
            .iter()
            .enumerate()
            .filter(|(_, game)| {
                self.batch_pgn_player_filter
                    .as_deref()
                    .is_none_or(|player| game.includes_player(player))
            })
            .map(|(index, _)| index)
            .collect();
        let response = egui::Modal::new(egui::Id::new("batch_pgn_import"))
            .frame(
                Frame::popup(&ctx.style_of(ctx.theme()))
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(Margin::same(24)),
            )
            .show(ctx, |ui| {
                ui.set_width((ctx.screen_rect().width() - 48.0).clamp(320.0, 680.0));
                let narrow = ui.available_width() < 520.0;
                let gold = Color32::from_rgb(211, 173, 98);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("Import {} games", self.batch_pgn_games.len()))
                            .size(27.0)
                            .strong(),
                    );
                    if !narrow {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(
                                RichText::new(format!("{selected} selected"))
                                    .size(15.0)
                                    .strong()
                                    .color(gold),
                            );
                        });
                    }
                });
                if narrow {
                    ui.label(
                        RichText::new(format!("{selected} selected"))
                            .size(15.0)
                            .strong()
                            .color(gold),
                    );
                }
                ui.add_space(5.0);
                ui.label(
                    RichText::new(
                        "Choose games for Imported Games. Analyze them individually after opening.",
                    )
                    .size(13.0)
                    .color(ui.visuals().weak_text_color()),
                );
                #[cfg(target_arch = "wasm32")]
                ui.label(
                    RichText::new(storage_status())
                        .size(12.0)
                        .color(ui.visuals().weak_text_color()),
                );
                #[cfg(target_arch = "wasm32")]
                if !imported_index_ready() {
                    ui.label(
                        RichText::new("Checking saved games for duplicates…")
                            .size(12.0)
                            .color(gold),
                    );
                    ctx.request_repaint_after(std::time::Duration::from_millis(250));
                }
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.label("Player");
                    egui::ComboBox::from_id_salt("batch_pgn_player")
                        .selected_text(
                            self.batch_pgn_player_filter
                                .as_deref()
                                .unwrap_or("All players"),
                        )
                        .width(260.0)
                        .show_ui(ui, |ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.batch_pgn_player_search)
                                    .hint_text("Find a player…"),
                            );
                            egui::ScrollArea::vertical()
                                .max_height(240.0)
                                .show(ui, |ui| {
                                    if ui
                                        .selectable_label(
                                            self.batch_pgn_player_filter.is_none(),
                                            "All players",
                                        )
                                        .clicked()
                                    {
                                        self.batch_pgn_player_filter = None;
                                        self.batch_pgn_player_search.clear();
                                        self.batch_pgn_page = 0;
                                        ui.close();
                                    }
                                    for player in &self.batch_pgn_players {
                                        if !player
                                            .to_lowercase()
                                            .contains(&self.batch_pgn_player_search.to_lowercase())
                                        {
                                            continue;
                                        }
                                        if ui
                                            .selectable_label(
                                                self.batch_pgn_player_filter.as_deref()
                                                    == Some(player.as_str()),
                                                player,
                                            )
                                            .clicked()
                                        {
                                            self.batch_pgn_player_filter = Some(player.clone());
                                            self.batch_pgn_player_search.clear();
                                            self.batch_pgn_page = 0;
                                            ui.close();
                                        }
                                    }
                                });
                        });
                    ui.label(format!("{} matching", filtered_indices.len()));
                });
                ui.add_space(10.0);
                let page_count = filtered_indices.len().div_ceil(25).max(1);
                self.batch_pgn_page = self.batch_pgn_page.min(page_count.saturating_sub(1));
                Frame::new()
                    .fill(Color32::from_rgb(31, 37, 42))
                    .corner_radius(CornerRadius::same(7))
                    .inner_margin(Margin::same(10))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Select")
                                    .size(13.0)
                                    .color(ui.visuals().weak_text_color()),
                            );
                            if ui.button("Select matching").clicked() {
                                for game in &mut self.batch_pgn_games {
                                    game.selected = self
                                        .batch_pgn_player_filter
                                        .as_deref()
                                        .is_none_or(|player| game.includes_player(player));
                                }
                            }
                            if ui.button("None").clicked() {
                                for game in &mut self.batch_pgn_games {
                                    game.selected = false;
                                }
                            }
                            if !narrow {
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if ui
                                        .add_enabled(
                                            self.batch_pgn_page + 1 < page_count,
                                            egui::Button::new("Next"),
                                        )
                                        .clicked()
                                    {
                                        self.batch_pgn_page += 1;
                                    }
                                    if ui
                                        .add_enabled(
                                            self.batch_pgn_page > 0,
                                            egui::Button::new("Previous"),
                                        )
                                        .clicked()
                                    {
                                        self.batch_pgn_page -= 1;
                                    }
                                    ui.label(
                                        RichText::new(format!(
                                            "Page {} of {}",
                                            self.batch_pgn_page + 1,
                                            page_count
                                        ))
                                        .size(13.0),
                                    );
                                });
                            }
                        });
                        if narrow {
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!(
                                        "Page {} of {}",
                                        self.batch_pgn_page + 1,
                                        page_count
                                    ))
                                    .size(13.0),
                                );
                                if ui
                                    .add_enabled(
                                        self.batch_pgn_page > 0,
                                        egui::Button::new("Previous"),
                                    )
                                    .clicked()
                                {
                                    self.batch_pgn_page -= 1;
                                }
                                if ui
                                    .add_enabled(
                                        self.batch_pgn_page + 1 < page_count,
                                        egui::Button::new("Next"),
                                    )
                                    .clicked()
                                {
                                    self.batch_pgn_page += 1;
                                }
                            });
                        }
                    });
                ui.add_space(10.0);
                egui::ScrollArea::vertical()
                    .id_salt("batch_pgn_games")
                    .max_height(360.0)
                    .show_gold(ui, |ui| {
                        for &index in filtered_indices
                            .iter()
                            .skip(self.batch_pgn_page * 25)
                            .take(25)
                        {
                            let game = &mut self.batch_pgn_games[index];
                            ui.group(|ui| {
                                ui.set_min_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    ui.checkbox(
                                        &mut game.selected,
                                        format!("{}. {} vs {}", index + 1, game.white, game.black),
                                    );
                                    ui.label(&game.result);
                                });
                                let details = [&game.date, &game.time_control, &game.site]
                                    .into_iter()
                                    .filter(|s| !s.is_empty() && *s != "?")
                                    .map(String::as_str)
                                    .collect::<Vec<_>>()
                                    .join(" · ");
                                if !details.is_empty() {
                                    ui.label(RichText::new(details).weak().size(12.0));
                                }
                            });
                        }
                    });
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    cancel = ui.button("Back to import").clicked();
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        #[cfg(target_arch = "wasm32")]
                        let ready = imported_index_ready();
                        #[cfg(not(target_arch = "wasm32"))]
                        let ready = true;
                        import = ui
                            .add_enabled(
                                selected > 0 && ready,
                                egui::Button::new(format!("Import selected ({selected})")),
                            )
                            .clicked();
                    });
                });
            });
        if import {
            self.batch_pgn_queue = self
                .batch_pgn_games
                .iter()
                .filter(|game| game.selected)
                .map(|game| game.text.clone())
                .collect();
            self.batch_pgn_imported = 0;
            self.batch_pgn_duplicates = 0;
            self.batch_pgn_total = self.batch_pgn_queue.len();
            self.batch_pgn_checked = 0;
            self.batch_pgn_failed = 0;
            self.batch_pgn_storage_failed_seen = 0;
            self.batch_pgn_first_error = None;
            self.batch_pgn_stopped = false;
            #[cfg(target_arch = "wasm32")]
            begin_batch_import_storage();
            self.batch_pgn_dialog_open = false;
            self.pgn_dialog_open = false;
            self.batch_pgn_games.clear();
            self.batch_pgn_players.clear();
            ctx.request_repaint();
        } else if cancel || response.should_close() {
            self.batch_pgn_dialog_open = false;
            self.batch_pgn_games.clear();
            self.batch_pgn_players.clear();
            self.batch_pgn_page = 0;
            self.import_input.clear();
            self.pgn_dialog_open = true;
        }
    }

    fn batch_pgn_progress_dialog(&mut self, ctx: &egui::Context) {
        if self.batch_pgn_total == 0 && !self.batch_pgn_result_open {
            return;
        }
        let mut dismiss = false;
        let mut cancel = false;
        egui::Modal::new(egui::Id::new("batch_pgn_progress"))
            .frame(
                Frame::popup(&ctx.style_of(ctx.theme()))
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(Margin::same(24)),
            )
            .show(ctx, |ui| {
                ui.set_width((ctx.screen_rect().width() - 48.0).clamp(320.0, 460.0));
                if self.batch_pgn_result_open {
                    let gold = Color32::from_rgb(211, 173, 98);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new("GAME LIBRARY")
                                .size(12.0)
                                .strong()
                                .color(gold),
                        );
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new(if self.batch_pgn_stopped {
                                "Import stopped"
                            } else {
                                "Import complete"
                            })
                            .size(27.0)
                            .strong(),
                        );
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new("Your selected games are ready in Saved Games.")
                                .color(ui.visuals().weak_text_color()),
                        );
                    });
                    ui.add_space(20.0);
                    Frame::new()
                        .fill(Color32::from_rgb(31, 37, 42))
                        .stroke(Stroke::new(1.0, Color32::from_rgb(90, 77, 51)))
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(Margin::same(16))
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.columns(3, |columns| {
                                columns[0].vertical_centered(|ui| {
                                    ui.label(
                                        RichText::new(self.batch_pgn_imported.to_string())
                                            .size(30.0)
                                            .strong()
                                            .color(gold),
                                    );
                                    ui.label("Games saved");
                                });
                                columns[1].vertical_centered(|ui| {
                                    ui.label(
                                        RichText::new(self.batch_pgn_duplicates.to_string())
                                            .size(30.0)
                                            .strong()
                                            .color(gold),
                                    );
                                    ui.label("Already saved");
                                });
                                columns[2].vertical_centered(|ui| {
                                    ui.label(
                                        RichText::new(self.batch_pgn_failed.to_string())
                                            .size(30.0)
                                            .strong()
                                            .color(if self.batch_pgn_failed == 0 {
                                                gold
                                            } else {
                                                Color32::from_rgb(232, 112, 112)
                                            }),
                                    );
                                    ui.label("Failed");
                                });
                            });
                        });
                    if let Some(error) = &self.batch_pgn_first_error {
                        ui.add_space(12.0);
                        ui.label(
                            RichText::new(format!("First error: {error}"))
                                .color(Color32::from_rgb(232, 112, 112)),
                        );
                    }
                    if self.batch_pgn_stopped && self.batch_pgn_checked < self.batch_pgn_total {
                        ui.add_space(8.0);
                        ui.label(format!(
                            "{} games were not attempted.",
                            self.batch_pgn_total - self.batch_pgn_checked
                        ));
                    }
                    ui.add_space(20.0);
                    ui.vertical_centered(|ui| {
                        dismiss = ui
                            .add_sized(
                                [168.0, 38.0],
                                egui::Button::new(RichText::new("Done").strong())
                                    .fill(Color32::from_rgb(101, 79, 40))
                                    .stroke(Stroke::new(1.0, gold)),
                            )
                            .clicked();
                    });
                } else {
                    let completed = self.batch_pgn_checked;
                    let accounted =
                        self.batch_pgn_imported + self.batch_pgn_duplicates + self.batch_pgn_failed;
                    let progress_total = if self.batch_pgn_stopped {
                        completed
                    } else {
                        self.batch_pgn_total
                    };
                    ui.heading("Importing games");
                    ui.add(
                        egui::ProgressBar::new(accounted as f32 / progress_total.max(1) as f32)
                            .show_percentage(),
                    );
                    ui.label(format!(
                        "{completed} of {} checked · {} saved · {} already saved · {} failed",
                        self.batch_pgn_total,
                        self.batch_pgn_imported,
                        self.batch_pgn_duplicates,
                        self.batch_pgn_failed
                    ));
                    if self.batch_pgn_stopped {
                        ui.label("Stopped · waiting for pending saves to finish…");
                    } else {
                        cancel = ui.button("Stop importing").clicked();
                    }
                }
            });
        if dismiss {
            self.batch_pgn_result_open = false;
            self.batch_pgn_total = 0;
        }
        if cancel {
            self.batch_pgn_queue.clear();
            self.batch_pgn_stopped = true;
            ctx.request_repaint();
        }
        if dismiss {
            #[cfg(target_arch = "wasm32")]
            end_batch_import_storage();
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

    fn position_score(analysis: &PositionAnalysis, board: &Board) -> Option<i32> {
        if let Some(mate) = analysis.mate {
            return Some(match mate.cmp(&0) {
                std::cmp::Ordering::Greater => 10_000,
                std::cmp::Ordering::Less => -10_000,
                std::cmp::Ordering::Equal if board.status() == BoardStatus::Checkmate => {
                    if board.side_to_move() == Color::White {
                        -10_000
                    } else {
                        10_000
                    }
                }
                std::cmp::Ordering::Equal => 0,
            });
        }
        analysis.eval_cp
    }

    fn move_centipawn_loss(&self, position_index: usize) -> Option<i32> {
        let before_index = position_index.checked_sub(1)?;
        let before = self.game_analysis.get(before_index)?.as_ref()?;
        let after = self.game_analysis.get(position_index)?.as_ref()?;
        if before.mate.is_some() || after.mate.is_some() {
            return None;
        }
        let before = before.eval_cp?;
        let after = after.eval_cp?;
        if self.move_matches_engine_best(position_index) {
            return Some(0);
        }
        let mover = self.review_positions.get(before_index)?.side_to_move();
        Some(match mover {
            Color::White => (before - after).max(0),
            Color::Black => (after - before).max(0),
        })
    }

    fn move_classification(&self, position_index: usize) -> Option<MoveClassification> {
        if let Some(outcome) = self.move_mate_outcome(position_index) {
            return Some(outcome.classification());
        }
        Some(Self::classify_loss(self.move_quality_loss(position_index)?))
    }

    fn move_matches_engine_best(&self, position_index: usize) -> bool {
        let Some(before_index) = position_index.checked_sub(1) else {
            return false;
        };
        let best = self
            .game_analysis
            .get(before_index)
            .and_then(Option::as_ref)
            .and_then(|analysis| analysis.best_move.as_deref())
            .and_then(Self::parse_uci_value);
        let played =
            Self::review_move_at(&self.review_positions, &self.review_moves, position_index);
        best.is_some() && best == played
    }

    fn effective_centipawn_loss(before: i32, after: i32, mover: Color) -> i32 {
        let factor = if mover == Color::White { 1.0 } else { -1.0 };
        let chance = |cp: i32| 1.0 / (1.0 + (-(cp as f32 * factor) / 220.0).exp());
        ((chance(before) - chance(after)).max(0.0) * 700.0).round() as i32
    }

    fn move_quality_loss(&self, position_index: usize) -> Option<i32> {
        if self.move_ends_in_checkmate(position_index) {
            self.game_analysis
                .get(position_index.checked_sub(1)?)?
                .as_ref()?;
            self.game_analysis.get(position_index)?.as_ref()?;
            return Some(0);
        }
        match self.move_mate_outcome(position_index) {
            Some(MateOutcome::Allowed | MateOutcome::Missed) => return Some(450),
            Some(MateOutcome::Found | MateOutcome::Escaped) => return Some(0),
            None => {}
        }
        if self.move_has_mate_score(position_index) {
            return Some(25);
        }
        let before_index = position_index.checked_sub(1)?;
        let before = self.game_analysis.get(before_index)?.as_ref()?.eval_cp?;
        let after = self.game_analysis.get(position_index)?.as_ref()?.eval_cp?;
        if self.move_matches_engine_best(position_index) {
            return Some(0);
        }
        Some(Self::effective_centipawn_loss(
            before,
            after,
            self.review_positions.get(before_index)?.side_to_move(),
        ))
    }

    fn move_mate_outcome(&self, position_index: usize) -> Option<MateOutcome> {
        let before_index = position_index.checked_sub(1)?;
        MateOutcome::between(
            self.game_analysis.get(before_index)?.as_ref()?.mate,
            self.game_analysis.get(position_index)?.as_ref()?.mate,
            self.review_positions.get(before_index)?.side_to_move(),
            self.move_ends_in_checkmate(position_index),
        )
    }

    fn move_ends_in_checkmate(&self, position_index: usize) -> bool {
        self.review_positions
            .get(position_index)
            .is_some_and(|board| board.status() == BoardStatus::Checkmate)
    }

    fn move_has_mate_score(&self, position_index: usize) -> bool {
        let Some(before_index) = position_index.checked_sub(1) else {
            return false;
        };
        let before = self
            .game_analysis
            .get(before_index)
            .and_then(Option::as_ref);
        let after = self
            .game_analysis
            .get(position_index)
            .and_then(Option::as_ref);
        matches!((before, after), (Some(before), Some(after)) if before.mate.is_some() || after.mate.is_some())
    }

    fn move_accuracy_loss(&self, position_index: usize) -> Option<i32> {
        self.move_quality_loss(position_index)
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
        let losses = (1..self.review_positions.len())
            .filter(|index| self.review_positions[index - 1].side_to_move() == color)
            .filter_map(|index| self.move_accuracy_loss(index))
            .collect::<Vec<_>>();
        Self::accuracy_from_losses(&losses)
    }

    fn analysis_resume_state(
        analysis: &[Option<PositionAnalysis>],
        was_running: bool,
        was_paused: bool,
    ) -> (Option<usize>, bool, bool) {
        let first_missing = analysis.iter().position(Option::is_none);
        let paused = first_missing.is_some() && was_paused;
        let running = first_missing.is_some() && was_running && !paused;
        (
            (running || paused).then_some(first_missing).flatten(),
            running,
            paused,
        )
    }

    fn accuracy_from_losses(losses: &[i32]) -> Option<f32> {
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

    fn game_phase_for_move(&self, position_index: usize) -> Option<GamePhase> {
        let board = self.review_positions.get(position_index.checked_sub(1)?)?;
        if position_index <= 20 {
            return Some(GamePhase::Opening);
        }
        let non_pawn_material = board.pieces(Piece::Knight).popcnt() * 3
            + board.pieces(Piece::Bishop).popcnt() * 3
            + board.pieces(Piece::Rook).popcnt() * 5
            + board.pieces(Piece::Queen).popcnt() * 9;
        Some(if non_pawn_material <= 26 {
            GamePhase::Endgame
        } else {
            GamePhase::Middlegame
        })
    }

    fn graph_phase_ranges(&self, count: usize) -> Vec<(GamePhase, usize, usize)> {
        let mut ranges: Vec<(GamePhase, usize, usize)> = Vec::new();
        for index in 0..count.min(self.review_positions.len()) {
            let Some(phase) = self.game_phase_for_move(index.max(1)) else { continue; };
            if let Some((previous, _, end)) = ranges.last_mut() && *previous == phase {
                *end = index;
            } else { ranges.push((phase, index, index)); }
        }
        ranges
    }

    fn phase_stats(&self, phase: GamePhase) -> Option<(usize, f32, Option<f32>)> {
        let indices = (1..self.review_positions.len())
            .filter(|index| self.game_phase_for_move(*index) == Some(phase))
            .collect::<Vec<_>>();
        let quality_losses = indices
            .iter()
            .filter_map(|index| self.move_quality_loss(*index))
            .collect::<Vec<_>>();
        let accuracy = Self::accuracy_from_losses(&quality_losses)?;
        let cp_losses = indices
            .iter()
            .filter_map(|index| self.move_centipawn_loss(*index))
            .collect::<Vec<_>>();
        let average = (!cp_losses.is_empty())
            .then(|| cp_losses.iter().sum::<i32>() as f32 / cp_losses.len() as f32);
        Some((quality_losses.len(), accuracy, average))
    }

    fn phase_stats_for_color(
        &self,
        phase: GamePhase,
        color: Color,
    ) -> Option<(usize, f32, Option<f32>)> {
        let indices = (1..self.review_positions.len())
            .filter(|index| self.game_phase_for_move(*index) == Some(phase))
            .filter(|index| self.review_positions[index - 1].side_to_move() == color)
            .collect::<Vec<_>>();
        let quality_losses = indices
            .iter()
            .filter_map(|index| self.move_quality_loss(*index))
            .collect::<Vec<_>>();
        let accuracy = Self::accuracy_from_losses(&quality_losses)?;
        let cp_losses = indices
            .iter()
            .filter_map(|index| self.move_centipawn_loss(*index))
            .collect::<Vec<_>>();
        let average = (!cp_losses.is_empty())
            .then(|| cp_losses.iter().sum::<i32>() as f32 / cp_losses.len() as f32);
        Some((quality_losses.len(), accuracy, average))
    }

    fn first_move_in_phase(&self, phase: GamePhase) -> Option<usize> {
        (1..self.review_positions.len())
            .find(|index| self.game_phase_for_move(*index) == Some(phase))
    }

    fn first_move_in_phase_for_color(&self, phase: GamePhase, color: Color) -> Option<usize> {
        (1..self.review_positions.len()).find(|index| {
            self.game_phase_for_move(*index) == Some(phase)
                && self.review_positions[index - 1].side_to_move() == color
        })
    }

    fn classification_count(&self, classification: MoveClassification) -> usize {
        (1..self.review_positions.len())
            .filter(|index| self.move_classification(*index) == Some(classification))
            .count()
    }

    fn classification_count_for_color(
        &self,
        classification: MoveClassification,
        color: Color,
    ) -> usize {
        (1..self.review_positions.len())
            .filter(|index| self.review_positions[index - 1].side_to_move() == color)
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

    fn next_classified_move_for_color(
        &self,
        classification: MoveClassification,
        color: Color,
        current: usize,
    ) -> Option<usize> {
        ((current + 1)..self.review_positions.len())
            .chain(1..=current.min(self.review_positions.len().saturating_sub(1)))
            .find(|index| {
                self.review_positions[*index - 1].side_to_move() == color
                    && self.move_classification(*index) == Some(classification)
            })
    }

    fn mate_outcome_count(&self, outcome: MateOutcome) -> usize {
        (1..self.review_positions.len())
            .filter(|index| self.move_mate_outcome(*index) == Some(outcome))
            .count()
    }

    fn next_mate_outcome(&self, outcome: MateOutcome, current: usize) -> Option<usize> {
        ((current + 1)..self.review_positions.len())
            .chain(1..=current.min(self.review_positions.len().saturating_sub(1)))
            .find(|index| self.move_mate_outcome(*index) == Some(outcome))
    }

    fn critical_move_indices(&self) -> Vec<usize> {
        (1..self.review_positions.len())
            .filter(|index| {
                matches!(
                    self.move_classification(*index),
                    Some(
                        MoveClassification::Inaccuracy
                            | MoveClassification::Mistake
                            | MoveClassification::Blunder
                    )
                )
            })
            .collect()
    }

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    fn critical_verification_targets(&self) -> Vec<usize> {
        let mut targets = Vec::new();
        for index in 1..self.review_positions.len() {
            if matches!(
                self.move_classification(index),
                Some(
                    MoveClassification::Inaccuracy
                        | MoveClassification::Mistake
                        | MoveClassification::Blunder
                )
            ) || self.move_mate_outcome(index).is_some()
            {
                targets.push(index - 1);
                targets.push(index);
            }
        }
        targets.sort_unstable();
        targets.dedup();
        targets
    }

    fn pending_verification_target(
        targets: &[usize],
        results: &[Option<PositionAnalysis>],
    ) -> Option<usize> {
        targets
            .iter()
            .copied()
            .find(|index| results.get(*index).is_some_and(Option::is_none))
    }

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    fn verification_pending_index(&self) -> Option<usize> {
        Self::pending_verification_target(&self.verification_targets, &self.verification_results)
    }

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    fn verification_completed_count(&self) -> usize {
        self.verification_targets
            .iter()
            .filter(|index| {
                self.verification_results
                    .get(**index)
                    .is_some_and(Option::is_some)
            })
            .count()
    }

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    fn verification_is_complete(&self) -> bool {
        !self.verification_targets.is_empty() && self.verification_pending_index().is_none()
    }

    fn clear_verification(&mut self) {
        self.verification_targets.clear();
        self.verification_results.clear();
        self.verification_nodes = 0;
    }

    fn turning_point_index(&self) -> Option<usize> {
        (1..self.review_positions.len())
            .filter(|index| {
                matches!(
                    self.move_classification(*index),
                    Some(
                        MoveClassification::Inaccuracy
                            | MoveClassification::Mistake
                            | MoveClassification::Blunder
                    )
                )
            })
            .filter_map(|index| self.move_quality_loss(index).map(|loss| (index, loss)))
            .max_by_key(|(_, loss)| *loss)
            .map(|(index, _)| index)
    }

    fn move_explanation(&self, position_index: usize) -> Option<String> {
        let root_index = position_index.checked_sub(1)?;
        let root = *self.review_positions.get(root_index)?;
        let before = self.game_analysis.get(root_index)?.as_ref()?;
        let after = self.game_analysis.get(position_index)?.as_ref()?;
        if let Some(outcome) = self.move_mate_outcome(position_index) {
            return Some(format!("{} {}", outcome.label(), outcome.explanation()));
        }
        if before.mate.is_some() || after.mate.is_some() {
            return None;
        }
        let mover_factor = if root.side_to_move() == Color::White {
            1
        } else {
            -1
        };
        let before_score = Self::position_score(before, &root).map(|score| score * mover_factor);
        let after_score = Self::position_score(after, self.review_positions.get(position_index)?)
            .map(|score| score * mover_factor);
        let explanation = if matches!((before_score, after_score), (Some(before), Some(after)) if before >= 100 && after <= -100)
        {
            "The position swings from an advantage to a disadvantage.".to_owned()
        } else {
            match self.move_classification(position_index)? {
                MoveClassification::Blunder => {
                    "This move causes a decisive evaluation swing.".to_owned()
                }
                MoveClassification::Mistake => {
                    "This move gives away a significant part of the position's value.".to_owned()
                }
                MoveClassification::Inaccuracy => {
                    "This move is playable, but it gives the opponent a noticeably better position."
                        .to_owned()
                }
                MoveClassification::Best | MoveClassification::Good => return None,
            }
        };
        let recommendation = before
            .best_move
            .as_deref()
            .and_then(Self::parse_uci_value)
            .map(|best_move| Self::san_for_move(&root, best_move));
        Some(match recommendation {
            Some(best_move) => format!(
                "{explanation} Stockfish prefers {}.",
                self.display_san(&best_move)
            ),
            None => explanation,
        })
    }

    fn analyzed_move_button(
        &self,
        ui: &mut egui::Ui,
        san: &str,
        position_index: usize,
        selected: bool,
        width: f32,
    ) -> egui::Response {
        let manual: String = self.move_annotations.get(position_index).into_iter().flatten().map(|nag| Self::annotation_symbol(*nag)).collect::<Vec<_>>().join(" ");
        let display_san = self.display_san(san);
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
        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 30.0), Sense::click());
        let visuals = ui.style().interact(&response);
        let fill = if selected {
            ui.visuals().selection.bg_fill
        } else {
            visuals.weak_bg_fill
        };
        if fill != Color32::TRANSPARENT {
            ui.painter().rect_filled(rect, visuals.corner_radius, fill);
        }
        ui.painter().rect_stroke(
            rect,
            visuals.corner_radius,
            classification
                .map(|quality| Stroke::new(1.5, quality.color()))
                .unwrap_or(visuals.bg_stroke),
            egui::StrokeKind::Inside,
        );

        let center_y = rect.center().y;
        let san_x = rect.left() + rect.width() * 0.27;
        let quality_x = rect.left() + rect.width() * 0.57;
        let score_x = rect.left() + rect.width() * 0.80;
        ui.painter().text(
            egui::pos2(san_x, if manual.is_empty() { center_y } else { rect.top() + 8.0 }),
            Align2::CENTER_CENTER,
            &display_san,
            FontId::proportional(13.0),
            ui.visuals().text_color(),
        );
        if !manual.is_empty() {
            ui.painter().text(
                egui::pos2(san_x, rect.bottom() - 4.0), Align2::CENTER_BOTTOM,
                &manual, FontId::proportional(10.0), Color32::from_rgb(211, 173, 98),
            );
        }
        if let Some(classification) = classification {
            ui.painter().text(
                egui::pos2(quality_x, center_y),
                Align2::CENTER_CENTER,
                classification.symbol(),
                FontId::proportional(13.0),
                classification.color(),
            );
        }
        if let Some(score) = evaluation_text.as_deref() {
            ui.painter().text(
                egui::pos2(score_x, center_y),
                Align2::CENTER_CENTER,
                score,
                FontId::proportional(12.5),
                ui.visuals().weak_text_color(),
            );
        }
        if !self.note_at(position_index).is_empty() {
            Self::paint_note_icon(
                ui.painter(),
                egui::Rect::from_center_size(
                    rect.left_center() + Vec2::new(10.0, 0.0),
                    Vec2::splat(11.0),
                ),
                Color32::from_rgb(211, 173, 98),
            );
        }
        let response = if !self.note_at(position_index).is_empty() {
            response.on_hover_text(format!("Your note: {}", self.note_at(position_index)))
        } else {
            response
        };
        if !self.show_move_hover_text {
            return response;
        }
        response.on_hover_ui(|ui| {
            ui.set_max_width(290.0);
            if let Some(description) = &move_description {
                ui.label(RichText::new(description).size(15.0).strong());
                ui.separator();
            }
            ui.label(RichText::new(format!("Move {position_index}: {display_san}")).strong());
            if let Some(classification) = classification {
                let detail = if let Some(outcome) = self.move_mate_outcome(position_index) {
                    outcome.label().to_owned()
                } else if let Some(loss) = self.move_centipawn_loss(position_index) {
                    format!("{} · {loss} centipawn loss", classification.label())
                } else {
                    classification.label().to_owned()
                };
                ui.label(RichText::new(detail).color(classification.color()));
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
                        ui.label(format!(
                            "Engine's best move: {}",
                            self.display_san(best_move)
                        ));
                    }
                    if !before.pv.is_empty() {
                        ui.separator();
                        ui.label(RichText::new("Engine line").small().weak());
                        ui.label(self.display_san_line(&before.pv));
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

    fn analysis_graph(&mut self, ui: &mut egui::Ui, expanded: bool) -> Option<usize> {
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
            Vec2::new(
                ui.available_width(),
                if expanded {
                    (ui.ctx().screen_rect().height() * 0.43).max(160.0)
                } else {
                    112.0
                },
            ),
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
        if !expanded {
            egui::Popup::context_menu(&response)
                .style(Self::gold_menu_style)
                .frame(Self::context_menu_frame(&response.ctx))
                .show(|ui| {
                    Self::style_context_menu(ui);
                    if ui.button("Expand analysis graph").clicked() {
                        self.expanded_graph_open = true;
                        ui.close();
                    }
                });
        }
        let plot = if expanded {
            egui::Rect::from_min_max(
                rect.min + Vec2::new(62.0, 34.0),
                rect.max - Vec2::new(24.0, 38.0),
            )
        } else {
            rect.shrink2(Vec2::new(8.0, 10.0))
        };
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
        if expanded {
            for (phase, start, end) in self.graph_phase_ranges(self.game_analysis.len()) {
                let left = plot.left() + plot.width() * (start as f32 - 0.5).max(0.0) / denominator;
                let right = plot.left() + plot.width() * (end as f32 + 0.5).min(denominator) / denominator;
                let band = egui::Rect::from_min_max(egui::pos2(left, plot.top()), egui::pos2(right, plot.bottom()));
                let color = phase.color();
                ui.painter().rect_filled(band, 0.0, color.gamma_multiply(0.10));
                ui.painter().rect_filled(egui::Rect::from_min_max(egui::pos2(left, plot.top() - 5.0), egui::pos2(right, plot.top() - 2.0)), 0.0, color);
                if start > 0 {
                    ui.painter().line_segment([band.left_top(), band.left_bottom()], Stroke::new(1.0, color.gamma_multiply(0.45)));
                }
                if band.width() >= 76.0 {
                    ui.painter().with_clip_rect(egui::Rect::from_min_max(egui::pos2(left, rect.top()), egui::pos2(right, plot.top())))
                        .text(egui::pos2(band.center().x, plot.top() - 17.0), Align2::CENTER_CENTER,
                            phase.label(), FontId::proportional(13.0), color);
                }
            }
            for cp in [-1000, -500, -200, -100, 0, 100, 200, 500, 1000] {
                let y = point_for(0, cp).y;
                ui.painter().line_segment(
                    [egui::pos2(plot.left(), y), egui::pos2(plot.right(), y)],
                    Stroke::new(
                        1.0,
                        Color32::from_white_alpha(if cp == 0 { 65 } else { 20 }),
                    ),
                );
                ui.painter().text(
                    egui::pos2(plot.left() - 10.0, y),
                    Align2::RIGHT_CENTER,
                    format!("{:+.0}", cp as f32 / 100.0),
                    FontId::proportional(15.0),
                    Color32::from_gray(190),
                );
            }
            let intervals = (plot.width() / 95.0).floor().max(1.0) as usize;
            let last = self.game_analysis.len().saturating_sub(1);
            for tick in 0..=intervals.min(last.max(1)) {
                let index = tick * last / intervals.min(last.max(1));
                let x = point_for(index, 0).x;
                ui.painter().line_segment(
                    [egui::pos2(x, plot.top()), egui::pos2(x, plot.bottom())],
                    Stroke::new(1.0, Color32::from_white_alpha(15)),
                );
                ui.painter().text(
                    egui::pos2(x, plot.bottom() + 18.0),
                    Align2::CENTER_CENTER,
                    self.graph_position_label(index, false),
                    FontId::proportional(14.0),
                    Color32::from_gray(190),
                );
            }
        }
        let mut previous = None;
        for (index, item) in self.game_analysis.iter().enumerate() {
            let Some(score) = item.as_ref().and_then(|analysis| {
                self.review_positions
                    .get(index)
                    .and_then(|board| Self::position_score(analysis, board))
            }) else {
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
                ui.painter().circle_filled(
                    point,
                    if expanded { 6.0 } else { 3.5 },
                    classification.color(),
                );
            }
            previous = Some(point);
        }
        for (index, _) in self
            .move_notes
            .iter()
            .enumerate()
            .filter(|(index, note)| !note.is_empty() && *index < self.review_positions.len())
        {
            let x = plot.left() + plot.width() * index as f32 / denominator;
            Self::paint_note_icon(
                ui.painter(),
                egui::Rect::from_center_size(
                    egui::pos2(x, plot.bottom() - 14.0),
                    Vec2::splat(if expanded { 17.0 } else { 11.0 }),
                ),
                Color32::from_rgb(145, 198, 224),
            );
        }
        if let Some(index) = self.review_index {
            let x = plot.left() + plot.width() * index as f32 / denominator;
            ui.painter().line_segment(
                [egui::pos2(x, plot.top()), egui::pos2(x, plot.bottom())],
                Stroke::new(1.5, Color32::from_rgb(232, 229, 214)),
            );
        }
        let hover_index = response.hover_pos().map(|pointer| {
            (((pointer.x - plot.left()) / plot.width()).clamp(0.0, 1.0) * denominator).round()
                as usize
        });
        if expanded {
            self.expanded_graph_hover_index = hover_index;
        }
        if let Some(index) = hover_index {
            let x = plot.left() + plot.width() * index as f32 / denominator;
            ui.painter().line_segment(
                [egui::pos2(x, plot.top()), egui::pos2(x, plot.bottom())],
                Stroke::new(1.0, Color32::from_rgb(211, 173, 98)),
            );
            if let Some(score) = self
                .game_analysis
                .get(index)
                .and_then(Option::as_ref)
                .and_then(|analysis| {
                    self.review_positions
                        .get(index)
                        .and_then(|board| Self::position_score(analysis, board))
                })
            {
                ui.painter().circle_filled(
                    point_for(index, score),
                    4.5,
                    Color32::from_rgb(211, 173, 98),
                );
            }
        }
        let response = if let Some(index) = hover_index {
            let show_details = |ui: &mut egui::Ui| {
                ui.set_max_width(290.0);
                let position = self.graph_position_label(index, true);
                ui.label(RichText::new(position).strong());
                if expanded && let Some(phase) = self.game_phase_for_move(index.max(1)) {
                    ui.label(RichText::new(phase.label()).color(phase.color()));
                }
                if !self.note_at(index).is_empty() {
                    ui.label(format!("Your note: {}", self.note_at(index)));
                }
                if let Some(analysis) = self.game_analysis.get(index).and_then(Option::as_ref) {
                    let evaluation = if let Some(mate) = analysis.mate {
                        format!("Mate {mate:+}")
                    } else if let Some(cp) = analysis.eval_cp {
                        format!("{:+.2}", cp as f32 / 100.0)
                    } else {
                        "No evaluation".to_owned()
                    };
                    ui.label(format!("White evaluation: {evaluation}"));
                    if index > 0
                        && let Some(classification) = self.move_classification(index)
                    {
                        ui.label(
                            RichText::new(format!(
                                "{} {}",
                                classification.symbol(),
                                classification.label()
                            ))
                            .color(classification.color()),
                        );
                    }
                }
                ui.label(
                    RichText::new("Click to jump to this position")
                        .small()
                        .weak(),
                );
            };
            if expanded {
                response.on_hover_ui_at_pointer(show_details)
            } else {
                response.on_hover_ui(show_details)
            }
        } else {
            response
        };
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

    fn paint_note_icon(painter: &egui::Painter, rect: egui::Rect, color: Color32) {
        let point = |x: f32, y: f32| rect.min + Vec2::new(x * rect.width(), y * rect.height());
        let stroke = Stroke::new(1.3, color);
        painter.add(egui::Shape::convex_polygon(
            vec![
                point(0.12, 0.88),
                point(0.23, 0.57),
                point(0.73, 0.07),
                point(0.93, 0.27),
                point(0.43, 0.77),
            ],
            Color32::TRANSPARENT,
            stroke,
        ));
        painter.line_segment([point(0.64, 0.16), point(0.84, 0.36)], stroke);
        painter.line_segment([point(0.23, 0.57), point(0.43, 0.77)], stroke);
    }

    fn parse_pgn_board_marks(text: &str, positions: &[Board]) -> Vec<Vec<BoardMark>> {
        let mut marks = vec![Vec::new(); positions.len()];
        Self::visit_pgn_comments(text, positions, |comment, index| {
            let Some(target) = marks.get_mut(index) else { return; };
            for (name, arrow) in [("cal", true), ("csl", false)] {
                // Multiple directives may occur in one comment.
                let prefix = format!("[%{name} ");
                for part in comment.split(&prefix).skip(1) {
                    let Some((value, _)) = part.split_once(']') else { continue; };
                    for encoded in value.split(',').map(str::trim) {
                        if !encoded.is_ascii() || encoded.len() != if arrow { 5 } else { 3 } { continue; }
                        let mark = BoardMark {
                            style: String::new(),
                            color: encoded.as_bytes()[0] as char,
                            from: encoded[1..3].to_owned(),
                            to: if arrow { encoded[3..5].to_owned() } else { encoded[1..3].to_owned() },
                        };
                        if mark.valid() && !target.contains(&mark) { target.push(mark); }
                    }
                }
            }
            for part in comment.split("[%iw_arrow_styles ").skip(1) {
                let Some((value, _)) = part.split_once(']') else { continue; };
                for item in value.split(',') {
                    let Some((encoded, style)) = item.trim().split_once(':') else { continue; };
                    if !encoded.is_ascii() || encoded.len() != 5 { continue; }
                    if let Some(mark) = target.iter_mut().find(|m| m.color == encoded.as_bytes()[0] as char && m.from == encoded[1..3] && m.to == encoded[3..5]) {
                        if matches!(style, "dashed" | "curve-left" | "curve-right" | "circle" | "dotted-square" | "dotted-circle") { mark.style = style.to_owned(); }
                    }
                }
            }
        });
        marks
    }

    fn pgn_board_marks(marks: Option<&Vec<BoardMark>>) -> String {
        let mut output = String::new();
        for (name, arrow) in [("cal", true), ("csl", false)] {
            let values: Vec<String> = marks.into_iter().flatten()
                .filter(|mark| mark.valid() && (mark.from != mark.to) == arrow)
                .map(|mark| format!("{}{}{}", mark.color, mark.from, if arrow { &mark.to } else { "" }))
                .collect();
            if !values.is_empty() { output.push_str(&format!(" {{ [%{name} {}] }}", values.join(","))); }
        }
        let styles: Vec<_> = marks.into_iter().flatten().filter(|m| m.valid() && !m.style.is_empty())
            .map(|m| format!("{}{}{}:{}", m.color, m.from, m.to, m.style)).collect();
        if !styles.is_empty() { output.push_str(&format!(" {{ [%iw_arrow_styles {}] }}", styles.join(","))); }
        output
    }

    fn toggle_board_mark(&mut self, index: usize, mark: BoardMark) {
        if index >= self.review_positions.len() || !mark.valid() { return; }
        self.board_marks.resize(self.review_positions.len(), Vec::new());
        self.record_drawing_undo(index);
        let marks = &mut self.board_marks[index];
        if let Some(existing) = marks.iter().position(|existing| existing == &mark) {
            marks.remove(existing);
        } else {
            marks.retain(|existing| existing.from != mark.from || existing.to != mark.to);
            marks.push(mark);
        }
        self.save_board_marks();
    }

    fn save_board_marks(&mut self) {
        if let Some(observed) = self.fics_observed_games.iter_mut().find(|game| Some(game.id) == self.fics_game_id) {
            observed.board_marks = self.board_marks.clone();
        }
        self.save_game();
    }

    fn board_mark_color(color: char) -> Color32 {
        match color {
            'R' => Color32::from_rgb(235, 88, 82), 'Y' => Color32::from_rgb(245, 205, 72),
            'B' => Color32::from_rgb(83, 151, 239), _ => Color32::from_rgb(74, 196, 116),
        }
    }

    fn paint_board_marks(&self, ui: &egui::Ui, rect: egui::Rect, index: usize) {
        if self.prediction_index != 0 { return; }
        let center = |square| {
            if self.board_3d_active { crate::board3d::square_center(square, rect, self.flipped, self.board_3d_view()) }
            else { Some(rect.left_top() + Self::square_screen_offset(square, self.flipped, rect.width() / 8.0)) }
        };
        let mut marks = self.board_marks.get(index).cloned().unwrap_or_default();
        if let Some((drag_index, from, color)) = self.board_mark_drag {
            if drag_index == index && let Some(pos) = ui.input(|input| input.pointer.latest_pos())
                && let Some(to) = self.board_mark_square(pos, rect)
            {
                let square_tool = self.board_mark_mode
                    && ui.ctx().data(|d| d.get_temp::<bool>(egui::Id::new("draw_window_open"))).unwrap_or(false)
                    && !ui.ctx().data(|d| d.get_temp::<bool>(egui::Id::new("draw_tool_arrow"))).unwrap_or(false)
                    && !ui.input(|i| i.modifiers.alt);
                marks.push(BoardMark { style: if ui.input(|i| i.modifiers.alt) { String::new() } else if square_tool { Self::drawing_shape_style(ui.ctx()) } else { Self::drawing_arrow_style(ui.ctx()) }, color, from: from.to_string(), to: if square_tool { from } else { to }.to_string() });
            }
        }
        let cell = rect.width() / if self.board_3d_active { 9.0 } else { 8.0 };
        for mark in marks.iter().filter(|mark| mark.valid()) {
            let from = Square::from_str(&mark.from).unwrap();
            let to = Square::from_str(&mark.to).unwrap();
            let (Some(source), Some(destination)) = (center(from), center(to)) else { continue; };
            let color = Self::board_mark_color(mark.color);
            if from == to {
                let stroke = Stroke::new((cell * 0.06).clamp(2.0, 5.0), color);
                let circle = mark.style.contains("circle");
                let points = if self.board_3d_active {
                    if circle { crate::board3d::circle_outline(from, rect, self.flipped, self.board_3d_view()) }
                    else { crate::board3d::square_outline(from, rect, self.flipped, self.board_3d_view()) }
                } else { Some(Self::shape_points(source, cell, circle)) };
                if let Some(points) = points { Self::paint_shape_outline(ui.painter(), points, stroke, mark.style.starts_with("dotted")); }
            } else {
                Self::paint_styled_arrow(ui.painter(), source, destination, cell, color, &mark.style);
            }
        }
    }

    fn square_screen_offset(square: Square, flipped: bool, cell: f32) -> Vec2 {
        let file = square.get_file().to_index() as f32;
        let rank = square.get_rank().to_index() as f32;
        Vec2::new((if flipped { 7.0 - file } else { file } + 0.5) * cell,
            (if flipped { rank } else { 7.0 - rank } + 0.5) * cell)
    }

    fn board_mark_square(&self, pos: egui::Pos2, rect: egui::Rect) -> Option<Square> {
        if !rect.contains(pos) { return None; }
        if self.board_3d_active {
            crate::board3d::board_square_at(pos, rect, self.flipped, self.board_3d_view())
        } else {
            let file = ((pos.x - rect.left()) / (rect.width() / 8.0)).floor() as usize;
            let rank = ((pos.y - rect.top()) / (rect.height() / 8.0)).floor() as usize;
            if file > 7 || rank > 7 { return None; }
            Some(Square::make_square(Rank::from_index(if self.flipped { rank } else { 7 - rank }),
                File::from_index(if self.flipped { 7 - file } else { file })))
        }
    }

    fn board_mark_input(&mut self, ui: &egui::Ui, rect: egui::Rect) {
        let index = self.review_index.unwrap_or(self.review_moves.len());
        let (pos, pressed, released, modifiers, escape) = ui.input(|input| (
            input.pointer.latest_pos(), input.pointer.button_pressed(egui::PointerButton::Primary),
            input.pointer.button_released(egui::PointerButton::Primary), input.modifiers, input.key_pressed(egui::Key::Escape)));
        if escape || self.prediction_index != 0 { self.board_mark_drag = None; return; }
        if pressed && (self.board_mark_mode || modifiers.alt) && ui.rect_contains_pointer(rect)
            && let Some(pos) = pos && let Some(square) = self.board_mark_square(pos, rect)
        {
            let color = if modifiers.alt { if modifiers.shift { 'R' } else if modifiers.ctrl { 'Y' } else { 'G' } } else { self.board_mark_color };
            self.board_mark_drag = Some((index, square, color));
        }
        if self.board_mark_drag.is_some_and(|(start_index, _, _)| start_index == index)
            && !modifiers.alt
            && ui.input(|input| input.pointer.button_pressed(egui::PointerButton::Secondary))
            && ui.ctx().data(|d| d.get_temp::<bool>(egui::Id::new("draw_tool_arrow"))).unwrap_or(false)
        {
            let style = Self::drawing_arrow_style(ui.ctx());
            let opposite = match style.as_str() {
                "" => Some("dashed"),
                "dashed" => Some(""),
                "curve-left" => Some("curve-right"),
                "curve-right" => Some("curve-left"),
                _ => None,
            };
            if let Some(opposite) = opposite {
                ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("drawing_arrow_style"), opposite.to_owned()));
                ui.ctx().request_repaint();
            }
        }
        if released && let Some((start_index, from, color)) = self.board_mark_drag.take()
            && ui.rect_contains_pointer(rect) && start_index == index && let Some(pos) = pos && let Some(to) = self.board_mark_square(pos, rect)
        {
            let tool_active = !modifiers.alt && ui.ctx().data(|d| d.get_temp::<bool>(egui::Id::new("draw_window_open"))).unwrap_or(false);
            let arrow = ui.ctx().data(|d| d.get_temp::<bool>(egui::Id::new("draw_tool_arrow"))).unwrap_or(false);
            if !tool_active || !arrow || from != to {
                let to = if tool_active && !arrow { from } else { to };
                self.toggle_board_mark(index, BoardMark { style: if !tool_active { String::new() } else if arrow { Self::drawing_arrow_style(ui.ctx()) } else { Self::drawing_shape_style(ui.ctx()) }, color, from: from.to_string(), to: to.to_string() });
            }
        }
        if self.board_mark_drag.is_some() { ui.ctx().request_repaint(); }
    }

    fn annotation_symbol(nag: u8) -> &'static str {
        match nag {
            1 => "!", 2 => "?", 3 => "!!", 4 => "??", 5 => "!?", 6 => "?!",
            10 => "=", 13 => "∞", 14 => "+=", 15 => "=+", 16 => "+/-", 17 => "-/+",
            18 => "+−", 19 => "−+", _ => "",
        }
    }

    fn pgn_nags(annotations: Option<&Vec<u8>>) -> String {
        annotations.into_iter().flatten().map(|nag| format!(" ${nag}")).collect()
    }

    fn annotation_menu(&mut self, ui: &mut egui::Ui, index: usize) {
        if index >= self.review_positions.len() { return; }
        ui.label(RichText::new("Your annotations · separate from Stockfish").weak());
        for (heading, choices) in [
            ("Move quality", vec![(1, "! Good move"), (3, "!! Brilliant move"), (5, "!? Interesting move"),
                (6, "?! Dubious move"), (2, "? Mistake"), (4, "?? Blunder")]),
            ("Position assessment", vec![(10, "= Equal"), (13, "∞ Unclear"),
                (14, "+= Slight White advantage"), (15, "=+ Slight Black advantage"),
                (16, "+/- Clear White advantage"), (17, "-/+ Clear Black advantage"),
                (18, "+− Decisive White advantage"), (19, "−+ Decisive Black advantage")]),
        ] {
            ui.separator();
            ui.label(RichText::new(heading).strong());
            for (nag, label) in choices {
                if index == 0 && nag <= 6 { continue; }
                let selected = self.move_annotations.get(index).is_some_and(|values| values.contains(&nag));
                if ui.selectable_label(selected, label).clicked() {
                    self.move_annotations.resize(self.review_positions.len(), Vec::new());
                    let values = &mut self.move_annotations[index];
                    values.retain(|value| if nag <= 6 { *value > 6 } else { !matches!(*value, 10 | 13..=19) });
                    if !selected { values.push(nag); }
                    self.save_manual_annotations();
                    ui.close();
                }
            }
        }
        ui.separator();
        if ui.add_enabled(self.move_annotations.get(index).is_some_and(|values| !values.is_empty()), egui::Button::new("Clear annotation")).clicked() {
            self.move_annotations[index].clear();
            self.save_manual_annotations();
            ui.close();
        }
    }

    fn save_manual_annotations(&mut self) {
        if let Some(observed) = self.fics_observed_games.iter_mut().find(|game| Some(game.id) == self.fics_game_id) {
            observed.move_annotations = self.move_annotations.clone();
        }
        self.save_game();
    }

    fn note_at(&self, index: usize) -> &str {
        self.move_notes.get(index).map(String::as_str).unwrap_or("")
    }

    fn edit_note(&mut self, index: usize) {
        self.note_editor_text = self.note_at(index).to_owned();
        self.note_editor_index = Some(index);
    }

    fn set_move_note(&mut self, index: usize, note: String) {
        if index >= self.review_positions.len() {
            return;
        }
        self.move_notes
            .resize(self.review_positions.len(), String::new());
        self.move_notes[index] = note.trim().to_owned();
        if let Some(observed) = self
            .fics_observed_games
            .iter_mut()
            .find(|game| Some(game.id) == self.fics_game_id)
        {
            observed.move_notes = self.move_notes.clone();
        }
        self.save_game();
    }

    fn move_note_dialog(&mut self, ctx: &egui::Context) {
        let Some(index) = self.note_editor_index else {
            return;
        };
        if index >= self.review_positions.len() {
            self.note_editor_index = None;
            return;
        }
        let mut close = false;
        let mut save = false;
        let mut delete = false;
        let response = egui::Modal::new(egui::Id::new("move_note_editor"))
            .frame(Self::dialog_frame())
            .show(ctx, |ui| {
                ui.set_width((ctx.screen_rect().width() - 64.0).clamp(240.0, 560.0));
                close = Self::notes_header(ui, &self.notes_title(index));
                ui.add_space(12.0);
                egui::ScrollArea::vertical()
                    .max_height((ctx.screen_rect().height() - 250.0).clamp(80.0, 300.0))
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.note_editor_text)
                                .desired_width(f32::INFINITY)
                                .desired_rows(8)
                                .font(FontId::proportional(17.0))
                                .hint_text("What did you notice about this position?"),
                        );
                    });
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    delete = ui
                        .add_enabled(
                            !self.note_at(index).is_empty(),
                            egui::Button::new("Delete note").min_size(Vec2::new(110.0, 40.0)),
                        )
                        .clicked();
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        save = ui
                            .add_sized(
                                [110.0, 40.0],
                                egui::Button::new(
                                    RichText::new("Save note")
                                        .size(17.0)
                                        .strong()
                                        .color(Color32::from_rgb(25, 29, 35)),
                                )
                                .fill(Color32::from_rgb(211, 173, 98)),
                            )
                            .clicked();
                    });
                });
            });
        if save || delete {
            self.set_move_note(
                index,
                if delete {
                    String::new()
                } else {
                    self.note_editor_text.clone()
                },
            );
        }
        if close || save || delete || response.should_close() {
            self.note_editor_index = None;
        }
    }

    fn graph_position_label(&self, index: usize, with_move: bool) -> String {
        if index == 0 {
            return "Start".into();
        }
        let start = Self::pgn_tag(&self.pgn_input, "FEN")
            .and_then(|fen| fen.split_whitespace().nth(5)?.parse::<usize>().ok())
            .map(|fullmove| {
                fullmove.saturating_sub(1) * 2
                    + usize::from(
                        self.review_positions
                            .first()
                            .is_some_and(|b| b.side_to_move() == Color::Black),
                    )
            })
            .unwrap_or(
                self.fics_observation_start_ply
                    .unwrap_or(self.local_start_ply),
            );
        let ply = start + index - 1;
        let number = format!("{}{}", ply / 2 + 1, if ply % 2 == 0 { "." } else { "..." });
        if with_move {
            format!(
                "{number} {}",
                self.review_moves
                    .get(index - 1)
                    .map(String::as_str)
                    .unwrap_or("")
            )
        } else {
            number
        }
    }

    fn expanded_graph_board_preview(&self, ui: &mut egui::Ui, width: f32) {
        let index = self.expanded_graph_hover_index
            .or(self.review_index)
            .unwrap_or(self.review_moves.len())
            .min(self.review_positions.len().saturating_sub(1));
        let Some(board) = self.review_positions.get(index) else { return; };
        Frame::new()
            .fill(Color32::from_rgb(20, 24, 28))
            .stroke(Stroke::new(1.5, Color32::from_rgb(211, 173, 98)))
            .corner_radius(6.0)
            .inner_margin(Margin::same(9))
            .show(ui, |ui| {
              ui.allocate_ui_with_layout(
                  Vec2::new(width - 18.0, width - 18.0 + 24.0),
                  Layout::top_down(Align::Min),
                  |ui| {
                let side = width - 18.0;
                ui.set_min_width(side);
                ui.label(RichText::new(self.graph_position_label(index, true)).small().weak());
                ui.add_space(3.0);
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(side), Sense::hover());
                let cell = side / 8.0;
                let last_move = Self::review_move_at(&self.review_positions, &self.review_moves, index);
                for screen_rank in 0..8 {
                    for screen_file in 0..8 {
                        let file_index = if self.flipped { 7 - screen_file } else { screen_file };
                        let rank_index = if self.flipped { screen_rank } else { 7 - screen_rank };
                        let square = Square::make_square(
                            Rank::from_index(rank_index), File::from_index(file_index),
                        );
                        let square_rect = egui::Rect::from_min_size(
                            rect.min + Vec2::new(screen_file as f32 * cell, screen_rank as f32 * cell),
                            Vec2::splat(cell),
                        );
                        let light = (file_index + rank_index) % 2 == 1;
                        let mut color = if light {
                            Color32::from_rgb(205, 214, 193)
                        } else {
                            Color32::from_rgb(76, 116, 92)
                        };
                        if self.show_highlighted_move && last_move.is_some_and(|chess_move| chess_move.get_source() == square || chess_move.get_dest() == square) {
                            color = Self::blend_color(color, Color32::from_rgb(211, 173, 98), 0.42);
                        }
                        ui.painter().rect_filled(square_rect, 0.0, color);
                        if let (Some(color), Some(piece)) = (board.color_on(square), board.piece_on(square)) {
                            Self::paint_piece(ui, square_rect, cell, self.piece_set, color, piece);
                        }
                    }
                }
                  },
              );
            });
    }

    fn expanded_analysis_graph(&mut self, ctx: &egui::Context) {
        if !self.expanded_graph_open {
            return;
        }
        self.expanded_graph_hover_index = None;
        let mut close = false;
        let response = egui::Modal::new(egui::Id::new("expanded_analysis_graph"))
            .frame(Self::dialog_frame())
            .show(ctx, |ui| {
                let size = ctx.screen_rect().size() - Vec2::splat(48.0);
                ui.set_min_size(size);
                ui.set_max_size(size);
                close = Self::dialog_header(ui, "Game Analysis");
                    ui.label(RichText::new(format!("{}  vs  {}",self.review_white_player,self.review_black_player)).size(20.0));
                    let completed = self.game_analysis.iter().filter(|value| value.is_some()).count();
                    ui.label(format!("Stockfish 19 · {completed}/{} positions analyzed · White perspective · Nonlinear pawn scale; mates reach the edges",
                        self.review_positions.len()));
                    ui.add_space(10.0);
                    if let Some(index) = self.analysis_graph(ui, true) { self.review_to(index); }
                let bottom_height = ui.available_height();
                let preview_width = (size.x * 0.19).clamp(150.0, 240.0)
                    .min((bottom_height - 43.0).max(100.0));
                let details_width = (ui.available_width() - preview_width - 16.0).max(320.0);
                ui.horizontal(|ui| {
                  ui.allocate_ui_with_layout(Vec2::new(details_width, bottom_height), Layout::top_down(Align::Min), |ui| {
                   ui.set_max_width(details_width);
                   egui::ScrollArea::vertical().auto_shrink([false,false]).show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for quality in [MoveClassification::Inaccuracy,MoveClassification::Mistake,MoveClassification::Blunder] {
                            ui.label(RichText::new(format!("● {}",quality.label())).color(quality.color()).size(16.0));
                        }
                        let (icon,_) = ui.allocate_exact_size(Vec2::splat(17.0),Sense::hover());
                        Self::paint_note_icon(ui.painter(),icon,Color32::from_rgb(145,198,224));
                        ui.label("Your notes · Gaps are unanalyzed positions. Click or drag to inspect.");
                    });
                    ui.horizontal_wrapped(|ui| {
                        for phase in [GamePhase::Opening, GamePhase::Middlegame, GamePhase::Endgame] {
                            let (swatch, _) = ui.allocate_exact_size(Vec2::new(12.0, 12.0), Sense::hover());
                            ui.painter().rect_filled(swatch, 2.0, phase.color());
                            ui.label(RichText::new(phase.label()).color(phase.color()));
                        }
                    }).response.on_hover_text("Approximate phases: first 10 full moves are opening; after that, endgame begins at 26 or fewer non-pawn material points. These match the phase summaries.");
                    ui.add_space(12.0);
                    let last = self.review_moves.len();
                    let mut index = self.review_index.unwrap_or(last).min(last);
                    ui.horizontal(|ui| {
                        if ui.add_enabled(index > 0,egui::Button::new("◀ Previous").min_size(Vec2::new(110.0,36.0))).clicked() { index -= 1; }
                        if ui.add_enabled(index < last,egui::Button::new("Next ▶").min_size(Vec2::new(110.0,36.0))).clicked() { index += 1; }
                        ui.add(egui::Slider::new(&mut index,0..=last).text("Position"));
                    });
                    if Some(index) != self.review_index { self.review_to(index); }
                    ui.add_space(8.0);
                    ui.label(RichText::new(self.graph_position_label(index,true)).size(22.0).strong());
                    if let Some(phase) = self.game_phase_for_move(index.max(1)) {
                        ui.label(RichText::new(format!("Game phase: {}", phase.label())).color(phase.color()).size(17.0));
                    }
                    if let Some(value) = self.game_analysis.get(index).and_then(Option::as_ref) {
                        let evaluation = value.mate.map(|mate| format!("Mate {mate:+}"))
                            .or_else(|| value.eval_cp.map(|cp| format!("{:+.2} pawns",cp as f32 / 100.0)))
                            .unwrap_or_else(|| "No evaluation".into());
                        ui.label(RichText::new(format!("White evaluation: {evaluation} · Depth {} · {} nodes",value.depth,value.nodes)).size(18.0));
                        if let Some(quality) = self.move_classification(index) {
                            ui.label(RichText::new(format!("{}{}",quality.label(),self.move_centipawn_loss(index)
                                .map(|loss| format!(" · {loss} centipawn loss")).unwrap_or_default())).size(17.0).color(quality.color()));
                        }
                        if let Some(outcome) = self.move_mate_outcome(index) { ui.label(RichText::new(outcome.label()).size(17.0)); }
                        if !value.pv.is_empty() { ui.label(RichText::new(format!("Best continuation: {}",value.pv)).size(17.0)); }
                    } else { ui.label(RichText::new("This position has not been analyzed yet.").size(18.0)); }
                    ui.add_space(14.0);
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Your note").size(20.0).strong());
                        if ui.button(if self.note_at(index).is_empty() { "Add note…" } else { "Edit note…" }).clicked() { self.edit_note(index); }
                    });
                    ui.label(RichText::new(if self.note_at(index).is_empty() { "No note for this position." } else { self.note_at(index) }).size(18.0));
                   });
                  });
                  ui.allocate_ui_with_layout(Vec2::new(preview_width, bottom_height), Layout::top_down(Align::Max), |ui| {
                      self.expanded_graph_board_preview(ui, preview_width);
                  });
                });
            });
        if close || response.should_close() {
            self.expanded_graph_open = false;
        }
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

    fn engine_setting_value(
        &mut self,
        ui: &mut egui::Ui,
        name: &str,
        analysis: bool,
        full_game: bool,
        clock: bool,
        available_threads: u32,
    ) {
        if name == "Analysis quality" {
            egui::ComboBox::from_id_salt("full_game_quality")
                .selected_text(self.full_game_analysis_draft.quality.label())
                .show_ui(ui, |ui| {
                    for quality in [
                        FullGameQuality::Quick,
                        FullGameQuality::Standard,
                        FullGameQuality::Deep,
                        FullGameQuality::Custom,
                    ] {
                        ui.selectable_value(
                            &mut self.full_game_analysis_draft.quality,
                            quality,
                            quality.label(),
                        );
                    }
                });
            return;
        }
        if name == "Nodes per position" {
            if self.full_game_analysis_draft.quality == FullGameQuality::Custom {
                ui.add(
                    egui::DragValue::new(&mut self.full_game_analysis_draft.custom_nodes)
                        .range(10_000..=50_000_000)
                        .speed(10_000),
                );
            } else {
                ui.label(
                    self.full_game_analysis_draft
                        .quality
                        .nodes(self.full_game_analysis_draft.custom_nodes)
                        .to_string(),
                );
            }
            return;
        }
        let draft = if analysis || full_game {
            &mut self.analysis_draft
        } else {
            &mut self.strength_draft
        };
        match name {
            "Strength" => {
                ui.label("Maximum · Skill 20 · Elo limiting off");
            }
            "Network" => {
                ui.label("Stockfish 19 · Full NNUE (fixed)");
            }
            "Strength mode" => {
                ui.selectable_value(&mut draft.limit_strength, false, "Skill level");
                ui.selectable_value(&mut draft.limit_strength, true, "Elo rating");
            }
            "Skill level" => {
                ui.add_enabled(
                    !draft.limit_strength,
                    egui::Slider::new(&mut draft.skill_level, 0..=20),
                );
            }
            "Target Elo" => {
                ui.add_enabled(
                    draft.limit_strength,
                    egui::Slider::new(&mut draft.elo, 1320..=3190),
                );
            }
            "Search limit" => {
                ui.add_enabled_ui(!clock, |ui| {
                    egui::ComboBox::from_id_salt("search_limit")
                        .selected_text(match draft.search_limit {
                            SearchLimit::Infinite => "Unlimited",
                            SearchLimit::Time => "Time",
                            SearchLimit::Depth => "Depth",
                            SearchLimit::Nodes => "Nodes",
                        })
                        .show_ui(ui, |ui| {
                            if analysis {
                                ui.selectable_value(
                                    &mut draft.search_limit,
                                    SearchLimit::Infinite,
                                    "Unlimited",
                                );
                            }
                            ui.selectable_value(&mut draft.search_limit, SearchLimit::Time, "Time");
                            ui.selectable_value(
                                &mut draft.search_limit,
                                SearchLimit::Depth,
                                "Depth",
                            );
                            ui.selectable_value(
                                &mut draft.search_limit,
                                SearchLimit::Nodes,
                                "Nodes",
                            );
                        });
                });
            }
            "Time per move" | "Time limit" => {
                ui.add_enabled(
                    !clock && draft.search_limit == SearchLimit::Time,
                    egui::Slider::new(&mut draft.move_time_ms, 100..=60_000)
                        .logarithmic(true)
                        .suffix(" ms"),
                );
            }
            "Search depth" => {
                ui.add_enabled(
                    !clock && draft.search_limit == SearchLimit::Depth,
                    egui::Slider::new(&mut draft.depth, 1..=40),
                );
            }
            "Search nodes" => {
                ui.add_enabled(
                    !clock && draft.search_limit == SearchLimit::Nodes,
                    egui::DragValue::new(&mut draft.nodes)
                        .range(1_000..=50_000_000)
                        .speed(1000),
                );
            }
            "Search threads" => {
                ui.add(egui::Slider::new(&mut draft.threads, 1..=available_threads));
            }
            "Hash memory" => {
                egui::ComboBox::from_id_salt("hash_memory")
                    .selected_text(format!("{} MiB", draft.hash_mib))
                    .show_ui(ui, |ui| {
                        for value in [16, 32, 64, 128, 256] {
                            ui.selectable_value(&mut draft.hash_mib, value, format!("{value} MiB"));
                        }
                    });
            }
            "Candidate lines" => {
                if analysis {
                    ui.add(egui::Slider::new(&mut draft.multipv, 1..=5));
                } else {
                    ui.label("1 (fixed)");
                }
            }
            _ => {}
        }
    }

    fn settings_tab_bar(ui: &mut egui::Ui, tabs: &[&str], selected: usize) -> usize {
        let mut chosen = selected;
        let border = ui.visuals().widgets.noninteractive.bg_stroke.color;
        let active_fill = ui.visuals().window_fill();
        let inactive_fill = ui.visuals().faint_bg_color;
        let width = ui.available_width();
        let start = ui.cursor().min;
        let bottom = start.y + 34.0;
        ui.painter().line_segment(
            [
                egui::pos2(start.x, bottom),
                egui::pos2(start.x + width, bottom),
            ],
            Stroke::new(1.0, border),
        );
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for (index, text) in tabs.iter().enumerate() {
                let active = index == selected;
                let text_width = ui
                    .painter()
                    .layout_no_wrap(
                        (*text).into(),
                        FontId::proportional(18.0),
                        ui.visuals().text_color(),
                    )
                    .size()
                    .x;
                let (slot, _) =
                    ui.allocate_exact_size(Vec2::new(text_width + 28.0, 34.0), Sense::hover());
                let rect = egui::Rect::from_min_max(
                    egui::pos2(slot.left(), slot.top() + if active { 0.0 } else { 4.0 }),
                    slot.right_bottom(),
                );
                let response = ui.put(
                    rect,
                    egui::Button::new(RichText::new(*text).size(18.0))
                        .fill(if active { active_fill } else { inactive_fill })
                        .stroke(Stroke::new(1.0, border))
                        .corner_radius(CornerRadius {
                            nw: 5,
                            ne: 5,
                            sw: 0,
                            se: 0,
                        }),
                );
                if response.clicked() {
                    chosen = index;
                }
                if active {
                    // Open the selected tab's lower edge into the content panel.
                    ui.painter().line_segment(
                        [
                            egui::pos2(rect.left() + 1.0, bottom),
                            egui::pos2(rect.right() - 1.0, bottom),
                        ],
                        Stroke::new(2.0, active_fill),
                    );
                    ui.painter().line_segment(
                        [
                            rect.left_top() + Vec2::new(5.0, 1.0),
                            rect.right_top() + Vec2::new(-5.0, 1.0),
                        ],
                        Stroke::new(2.0, Color32::from_rgb(211, 173, 98)),
                    );
                }
            }
        });
        chosen
    }

    fn strength_dialog(&mut self, ctx: &egui::Context) {
        if self.training_live() { self.strength_dialog_open = false; return; }
        if !self.strength_dialog_open {
            return;
        }
        if self.game_analysis_running || self.game_analysis_paused {
            self.strength_dialog_open = false;
            return;
        }

        let mut apply = false;
        let mut close = false;
        let mut reset_defaults = false;
        let available_threads = Self::engine_thread_count();
        let width = (ctx.screen_rect().width() - 48.0).clamp(300.0, 660.0);
        // Ten equal-height rows on normal displays; fit smaller windows without
        // allowing the selected engine/context to alter the dialog dimensions.
        let row_height = ((ctx.screen_rect().height() - 300.0) / 10.0).clamp(24.0, 36.0);
        let table_height = row_height * 10.0;
        let response =
            egui::Modal::new(egui::Id::new("stockfish_strength_settings"))
                .frame(Self::dialog_frame())
                .show(ctx, |ui| {
                    ui.set_width(width);
                    close = Self::dialog_header(ui, "Engine settings");
                    ui.add_space(4.0);
                    Self::settings_tab_bar(ui, &["Stockfish"], 0);
                    let context_index = match self.engine_settings_tab {
                        EngineSettingsTab::Play => 0,
                        EngineSettingsTab::Analysis => 1,
                        EngineSettingsTab::FullGame => 2,
                    };
                    let selected_context = Self::settings_tab_bar(
                        ui,
                        &["Play", "Realtime Analysis", "Full game Analysis"],
                        context_index,
                    );
                    self.engine_settings_tab = match selected_context {
                        1 => EngineSettingsTab::Analysis,
                        2 => EngineSettingsTab::FullGame,
                        _ => EngineSettingsTab::Play,
                    };
                    ui.add_space(4.0);
                    let analysis = self.engine_settings_tab == EngineSettingsTab::Analysis;
                    let full_game = self.engine_settings_tab == EngineSettingsTab::FullGame;
                    let clock =
                        !analysis && !full_game && !self.fics_active && self.local_clock.is_some();
                    let rows: &[&str] = if full_game {
                        &[
                            "Analysis quality",
                            "Nodes per position",
                            "Search threads",
                            "Hash memory",
                            "Candidate lines",
                            "Strength",
                            "Network",
                        ]
                    } else if analysis {
                        &[
                            "Strength",
                            "Search limit",
                            "Time limit",
                            "Search depth",
                            "Search nodes",
                            "Search threads",
                            "Hash memory",
                            "Candidate lines",
                            "Network",
                        ]
                    } else {
                        &[
                            "Strength mode",
                            "Skill level",
                            "Target Elo",
                            "Search limit",
                            "Time per move",
                            "Search depth",
                            "Search nodes",
                            "Search threads",
                            "Hash memory",
                            "Candidate lines",
                            "Network",
                        ]
                    };
                    ui.scope(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        ui.spacing_mut().slider_width = (width - 310.0).max(75.0);
                        egui_extras::TableBuilder::new(ui)
                            .id_salt(("engine_settings_table", analysis, full_game))
                            .striped(true)
                            .resizable(false)
                            .cell_layout(Layout::left_to_right(Align::Center))
                            .column(egui_extras::Column::exact((width * 0.34).min(210.0)))
                            .column(egui_extras::Column::remainder().clip(true))
                            .min_scrolled_height(table_height)
                            .max_scroll_height(table_height)
                            .auto_shrink([false, false])
                            .header(30.0, |mut header| {
                                header.col(|ui| {
                                    ui.strong("Setting");
                                });
                                header.col(|ui| {
                                    ui.strong("Value");
                                });
                            })
                            .body(|body| {
                                body.rows(row_height, rows.len(), |mut row| {
                                    let name = rows[row.index()];
                                    row.col(|ui| {
                                        ui.label(name);
                                    });
                                    row.col(|ui| {
                                        self.engine_setting_value(
                                            ui,
                                            name,
                                            analysis,
                                            full_game,
                                            clock,
                                            available_threads,
                                        );
                                    });
                                })
                            });
                    });
                    ui.add_space(10.0);
                    // Single-line help keeps the footer in the same place on every tab.
                    ui.add(egui::Label::new(if full_game {
                    "Threads and hash memory are shared with realtime analysis."
                } else if clock {
                    "The game clock controls search time; manual search limits are inactive."
                } else {
                    "Inactive settings are retained for their corresponding mode."
                }).truncate());
                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        reset_defaults = ui.button("Reset this context").clicked();
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            apply = ui
                                .add_sized([76.0, 30.0], egui::Button::new("Apply"))
                                .clicked();
                            if ui
                                .add_sized([76.0, 30.0], egui::Button::new("Cancel"))
                                .clicked()
                            {
                                close = true;
                            }
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
            let was_stockfish_search =
                self.engine_searching && self.engine_config.opponent == OpponentEngine::Stockfish;
            self.engine_config = self.strength_draft.clone();
            if self.review_index.is_none() {
                if self.player_side == PlayerSide::White {
                    self.review_black_player = self.engine_config.opponent.label().into();
                } else {
                    self.review_white_player = self.engine_config.opponent.label().into();
                }
            }
            self.analysis_draft.skill_level = 20;
            self.analysis_draft.limit_strength = false;
            self.analysis_draft.opponent = OpponentEngine::Stockfish;
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
            self.ignore_next_bestmove = was_stockfish_search;
            self.engine_status = "Engine settings updated".into();
            self.strength_dialog_open = false;
            #[cfg(target_arch = "wasm32")]
            self.request_engine_move();
        } else if close || response.should_close() {
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

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    fn game_result(&self) -> String {
        if !self.fics_active
            && let Some(white) = self.local_resigned_white
        {
            return if white { "0-1" } else { "1-0" }.into();
        }
        if !self.fics_active
            && let Some(white) = self
                .local_clock
                .as_ref()
                .and_then(|clock| clock.flagged_white)
        {
            return if white { "0-1" } else { "1-0" }.into();
        }
        if self.fics_active
            && !self.fics_playing
            && let Some(message) = self
                .fics_observed_games
                .iter()
                .find(|game| Some(game.id) == self.fics_game_id)
                .and_then(|game| game.end_message.as_deref())
            && let Some(result) = message.split_whitespace().last()
            && matches!(result, "1-0" | "0-1" | "1/2-1/2" | "*")
        {
            return result.to_owned();
        }
        if let Some(result) = Self::pgn_tag(&self.pgn_input, "Result")
            && matches!(result.as_str(), "1-0" | "0-1" | "1/2-1/2")
        {
            return result;
        }
        if self.training.is_some() && self.training_draw() { return "1/2-1/2".into(); }
        let board = self.review_positions.last().copied().unwrap_or(self.board);
        match board.status() {
            BoardStatus::Ongoing => "*".into(),
            BoardStatus::Stalemate => "1/2-1/2".into(),
            BoardStatus::Checkmate if board.side_to_move() == Color::White => "0-1".into(),
            BoardStatus::Checkmate => "1-0".into(),
        }
    }

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    // Export a saved record without loading it into the active board or starting an engine.
    pub fn export_saved_game(json: &str, format: &str) -> Result<String, String> {
        let raw: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
        let opponent = if raw["engine_config"]["opponent"] == "Lc0" {
            "Lc0 · Good Gyal"
        } else {
            "Stockfish 19"
        };
        let saved: PersistedGame = serde_json::from_value(raw).map_err(|e| e.to_string())?;
        let source = saved.review_pgn.as_deref().unwrap_or("");
        let (positions, moves) = if !source.is_empty() {
            Self::parse_pgn_mainline(source)?
        } else if !saved.live_positions.is_empty() {
            let positions = saved
                .live_positions
                .iter()
                .map(|fen| Board::from_str(fen))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| "Invalid saved position")?;
            (positions, saved.live_moves.clone())
        } else if !saved.live_moves.is_empty() {
            Self::positions_from_san_moves(&saved.live_moves)?
        } else {
            (
                vec![Board::from_str(&saved.board).map_err(|_| "Invalid saved board")?],
                Vec::new(),
            )
        };
        if format == "preview" {
            return Ok(serde_json::json!({
                "positions": positions.iter().map(ToString::to_string).collect::<Vec<_>>(),
                "moves": moves,
            }).to_string());
        }

        if positions.len() != moves.len() + 1 {
            return Err("Saved move timeline is incomplete".into());
        }
        for (index, san) in moves.iter().enumerate() {
            let mv = Self::parse_san_move(&positions[index], san).ok_or("Invalid saved move")?;
            if positions[index].make_move_new(mv) != positions[index + 1] {
                return Err("Saved move timeline does not match the moves".into());
            }
        }
        let player = |white: bool| {
            if let Some(t) = &saved.training {
                return if (t.side == TrainingSide::White) == white { t.profile_name.clone() } else { format!("Stockfish {} Elo", t.opponent_elo) };
            }
            Self::pgn_tag(source, if white { "White" } else { "Black" }).unwrap_or_else(|| {
                if !saved.engine_enabled {
                    if white { "White" } else { "Black" }.into()
                } else if (saved.player_side == PlayerSide::White) == white {
                    "You".into()
                } else {
                    opponent.into()
                }
            })
        };
        let result = if ["1-0", "0-1", "1/2-1/2", "*"].contains(&saved.result.as_str()) {
            saved.result.clone()
        } else {
            Self::pgn_tag(source, "Result").unwrap_or_else(|| "*".into())
        };
        let analysis = |index: usize| saved.game_analysis.get(index).and_then(Option::as_ref);
        let mut timeline = Vec::new();
        let mut quality_losses = Vec::new();
        for (index, board) in positions.iter().enumerate() {
            let value = analysis(index);
            let previous = index.checked_sub(1).and_then(analysis);
            let mover = index.checked_sub(1).map(|i| positions[i].side_to_move());
            let best = previous
                .and_then(|v| v.best_move.as_deref())
                .and_then(Self::parse_uci_value);
            let matches_best =
                best.is_some() && best == Self::review_move_at(&positions, &moves, index);
            let outcome = previous.zip(value).and_then(|(before, after)| {
                MateOutcome::between(
                    before.mate,
                    after.mate,
                    mover.unwrap(),
                    board.status() == BoardStatus::Checkmate,
                )
            });
            let cp = previous.zip(value).and_then(|(before, after)| {
                if before.mate.is_some() || after.mate.is_some() {
                    return None;
                }
                Some((before.eval_cp?, after.eval_cp?))
            });
            let loss = cp.map(|(before, after)| {
                if matches_best {
                    0
                } else if mover == Some(Color::White) {
                    (before - after).max(0)
                } else {
                    (after - before).max(0)
                }
            });
            let quality_loss = previous.zip(value).and_then(|(before, after)| {
                if board.status() == BoardStatus::Checkmate {
                    Some(0)
                } else if let Some(outcome) = outcome {
                    Some(match outcome {
                        MateOutcome::Allowed | MateOutcome::Missed => 450,
                        MateOutcome::Found | MateOutcome::Escaped => 0,
                    })
                } else if before.mate.is_some() || after.mate.is_some() {
                    Some(25)
                } else {
                    cp.map(|(a, b)| {
                        if matches_best {
                            0
                        } else {
                            Self::effective_centipawn_loss(a, b, mover.unwrap())
                        }
                    })
                }
            });
            let quality = outcome
                .map(MateOutcome::classification)
                .or_else(|| quality_loss.map(Self::classify_loss));
            quality_losses.push(quality_loss);
            timeline.push(serde_json::json!({
                "note": saved.move_notes.get(index).cloned().unwrap_or_default(),
                "annotations": saved.move_annotations.get(index).cloned().unwrap_or_default(),
                "board_marks": saved.board_marks.get(index).cloned().unwrap_or_default(),
                "index": index, "fen": board.to_string(), "move": index.checked_sub(1).and_then(|i| moves.get(i)),
                "side_to_move": if board.side_to_move() == Color::White { "white" } else { "black" },
                "evaluation_cp": value.and_then(|v| v.eval_cp), "mate": value.and_then(|v| v.mate),
                "depth": value.map(|v| v.depth), "nodes": value.map(|v| v.nodes),
                "best_move": value.and_then(|v| v.best_move.as_deref()), "principal_variation": value.map(|v| &v.pv),
                "centipawn_loss": loss, "classification": quality.map(MoveClassification::label),
                "mate_outcome": outcome.map(MateOutcome::label)
            }));
        }
        if format == "json" {
            let quality = saved.full_game_analysis_config.quality;
            let accuracy = |color| {
                Self::accuracy_from_losses(
                    &(1..positions.len())
                        .filter(|i| positions[i - 1].side_to_move() == color)
                        .filter_map(|i| quality_losses[i])
                        .collect::<Vec<_>>(),
                )
            };
            let average = |color| {
                let losses = (1..positions.len())
                    .filter(|i| positions[i - 1].side_to_move() == color)
                    .filter_map(|i| timeline[i]["centipawn_loss"].as_i64())
                    .collect::<Vec<_>>();
                (!losses.is_empty())
                    .then(|| losses.iter().sum::<i64>() as f64 / losses.len() as f64)
            };
            return serde_json::to_string_pretty(&serde_json::json!({
                "schema": "ironwood.analyzed-game/v1", "schema_version": 1, "application": "Ironwood Chess",
                "engine": { "name": "Stockfish", "version": 19, "network": "Full NNUE",
                    "threads": saved.analysis_config.threads, "hash_mib": saved.analysis_config.hash_mib,
                    "full_game_quality": quality.label(), "nodes_per_position": quality.nodes(saved.full_game_analysis_config.custom_nodes) },
                "game": { "white": player(true), "black": player(false), "result": result,
                    "date": Self::pgn_tag(source,"Date"), "site": Self::pgn_tag(source,"Site"),
                    "source_pgn": saved.review_pgn, "moves": moves },
                "verification": (!saved.verification_targets.is_empty() && saved.verification_targets.iter().all(|i|
                    saved.verification_results.get(*i).is_some_and(Option::is_some))).then(|| serde_json::json!({
                        "nodes_per_position": saved.verification_nodes, "target_positions": saved.verification_targets })),
                "summary": { "white_accuracy": accuracy(Color::White), "black_accuracy": accuracy(Color::Black),
                    "white_average_centipawn_loss": average(Color::White), "black_average_centipawn_loss": average(Color::Black) },
                "positions": timeline
            })).map_err(|e| e.to_string());
        }
        if format != "pgn" && format != "annotated" {
            return Err("Unknown export format".into());
        }
        let mut output = String::new();
        for (tag, value) in [
            (
                "Event",
                Self::pgn_tag(source, "Event").unwrap_or_else(|| "Ironwood Chess game".into()),
            ),
            (
                "Site",
                Self::pgn_tag(source, "Site").unwrap_or_else(|| "-".into()),
            ),
            (
                "Date",
                Self::pgn_tag(source, "Date").unwrap_or_else(|| "????.??.??".into()),
            ),
            (
                "Round",
                Self::pgn_tag(source, "Round").unwrap_or_else(|| "-".into()),
            ),
            ("White", player(true)),
            ("Black", player(false)),
            ("Result", result.clone()),
        ] {
            output.push_str(&format!(
                "[{tag} \"{}\"]\n",
                value.replace('\\', "\\\\").replace('"', "\\\"")
            ));
        }
        if format == "annotated" {
            output.push_str("[Annotator \"Ironwood Chess · Stockfish 19 NNUE\"]\n");
        }
        let initial = &positions[0];
        if initial.is_chess960() {
            output.push_str("[Variant \"Chess960\"]\n");
        }
        if *initial != Board::default() {
            output.push_str(&format!("[SetUp \"1\"]\n[FEN \"{initial}\"]\n"));
        }
        output.push('\n');
        if format == "annotated"
            && let Some(note) = saved.move_notes.first().filter(|note| !note.is_empty())
        {
            output.push_str(&Self::pgn_note(note));
        }
        if format == "annotated" { output.push_str(&Self::pgn_nags(saved.move_annotations.first())); output.push_str(&Self::pgn_board_marks(saved.board_marks.first())); output.push(' '); }
        let fen = Self::pgn_tag(source, "FEN")
            .or_else(|| saved.live_positions.first().cloned())
            .unwrap_or_else(|| initial.to_string());
        let fullmove = fen
            .split_whitespace()
            .nth(5)
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(1);
        let start =
            (fullmove.saturating_sub(1)) * 2 + usize::from(initial.side_to_move() == Color::Black);
        for (index, san) in moves.iter().enumerate() {
            let ply = start + index;
            if ply % 2 == 0 {
                output.push_str(&format!("{}. ", ply / 2 + 1));
            } else if index == 0 {
                output.push_str(&format!("{}... ", ply / 2 + 1));
            }
            output.push_str(san);
            if format == "annotated" { output.push_str(&Self::pgn_nags(saved.move_annotations.get(index + 1))); output.push_str(&Self::pgn_board_marks(saved.board_marks.get(index + 1))); }
            if format == "annotated"
                && let Some(value) = analysis(index + 1)
            {
                let mut notes = Vec::new();
                if let Some(mate) = value.mate {
                    notes.push(format!("[%eval #{mate}]"));
                } else if let Some(cp) = value.eval_cp {
                    notes.push(format!("[%eval {:+.2}]", cp as f64 / 100.0));
                }
                notes.push(format!("[%depth {}]", value.depth));
                notes.push(format!("[%nodes {}]", value.nodes));
                let entry = &timeline[index + 1];
                if let Some(loss) = entry["centipawn_loss"].as_i64() {
                    notes.push(format!("[%cpl {loss}]"));
                }
                if let Some(quality) = entry["classification"].as_str() {
                    notes.push(format!("Quality: {quality}"));
                }
                if let Some(outcome) = entry["mate_outcome"].as_str() {
                    notes.push(outcome.into());
                }
                if let Some(previous) = analysis(index) {
                    if let Some(best) = &previous.best_move {
                        notes.push(format!("Best move: {best}"));
                    }
                    if !previous.pv.is_empty() {
                        notes.push(format!("PV: {}", previous.pv.replace(['{', '}'], "")));
                    }
                }
                output.push_str(&format!(" {{ {} }}", notes.join("; ")));
            }
            if format == "annotated"
                && let Some(note) = saved
                    .move_notes
                    .get(index + 1)
                    .filter(|note| !note.is_empty())
            {
                output.push_str(&Self::pgn_note(note));
            }
            output.push(' ');
        }
        output.push_str(&result);
        output.push('\n');
        Ok(output)
    }

    fn annotated_pgn(&self) -> String {
        let mut output = String::new();
        let tags = [
            (
                "Event",
                Self::pgn_tag(&self.pgn_input, "Event")
                    .unwrap_or_else(|| "Ironwood Chess game".into()),
            ),
            (
                "Site",
                Self::pgn_tag(&self.pgn_input, "Site").unwrap_or_else(|| "-".into()),
            ),
            (
                "Date",
                Self::pgn_tag(&self.pgn_input, "Date").unwrap_or_else(|| "????.??.??".into()),
            ),
            (
                "Round",
                Self::pgn_tag(&self.pgn_input, "Round").unwrap_or_else(|| "-".into()),
            ),
            ("White", self.review_white_player.clone()),
            ("Black", self.review_black_player.clone()),
            ("Result", self.game_result().into()),
            ("Annotator", "Ironwood Chess · Stockfish 19 NNUE".into()),
        ];
        for (name, value) in tags {
            output.push_str(&format!("[{name} \"{}\"]\n", value.replace('"', "'")));
        }
        if self
            .review_positions
            .first()
            .is_some_and(Board::is_chess960)
        {
            output.push_str("[Variant \"Chess960\"]\n");
        }
        if let Some(initial) = self.review_positions.first()
            && *initial != Board::default()
        {
            output.push_str("[SetUp \"1\"]\n");
            output.push_str(&format!("[FEN \"{initial}\"]\n"));
        }
        output.push('\n');
        if let Some(note) = self.move_notes.first().filter(|note| !note.is_empty()) {
            output.push_str(&Self::pgn_note(note));
        }

        output.push_str(&Self::pgn_nags(self.move_annotations.first()));
        output.push_str(&Self::pgn_board_marks(self.board_marks.first()));
        output.push(' ');
        for (move_index, san) in self.review_moves.iter().enumerate() {
            let before = self.review_positions.get(move_index);
            let fullmove = before
                .and_then(|board| {
                    board
                        .to_string()
                        .split_whitespace()
                        .nth(5)?
                        .parse::<u32>()
                        .ok()
                })
                .unwrap_or((move_index / 2 + 1) as u32);
            if before.is_some_and(|board| board.side_to_move() == Color::Black) {
                output.push_str(&format!("{fullmove}... "));
            } else {
                output.push_str(&format!("{fullmove}. "));
            }
            output.push_str(san);
            output.push_str(&Self::pgn_nags(self.move_annotations.get(move_index + 1)));
            output.push_str(&Self::pgn_board_marks(self.board_marks.get(move_index + 1)));
            if let Some(analysis) = self
                .game_analysis
                .get(move_index + 1)
                .and_then(Option::as_ref)
            {
                let mut annotations = Vec::new();
                if let Some(mate) = analysis.mate {
                    annotations.push(format!("[%eval #{mate}]"));
                } else if let Some(cp) = analysis.eval_cp {
                    annotations.push(format!("[%eval {:+.2}]", cp as f32 / 100.0));
                }
                annotations.push(format!("[%depth {}]", analysis.depth));
                annotations.push(format!("[%nodes {}]", analysis.nodes));
                if let Some(loss) = self.move_centipawn_loss(move_index + 1) {
                    annotations.push(format!("[%cpl {loss}]"));
                }
                if let Some(classification) = self.move_classification(move_index + 1) {
                    annotations.push(format!("Quality: {}", classification.label()));
                }
                if let Some(outcome) = self.move_mate_outcome(move_index + 1) {
                    annotations.push(outcome.label().to_owned());
                }
                if let Some(best) = self
                    .game_analysis
                    .get(move_index)
                    .and_then(Option::as_ref)
                    .and_then(|value| value.best_move.as_deref())
                {
                    annotations.push(format!("Best move: {best}"));
                }
                if let Some(pv) = self
                    .game_analysis
                    .get(move_index)
                    .and_then(Option::as_ref)
                    .map(|value| value.pv.as_str())
                    .filter(|pv| !pv.is_empty())
                {
                    annotations.push(format!("PV: {pv}"));
                }
                output.push_str(" { ");
                output.push_str(&annotations.join("; "));
                output.push_str(" }");
            }
            if let Some(note) = self
                .move_notes
                .get(move_index + 1)
                .filter(|note| !note.is_empty())
            {
                output.push_str(&Self::pgn_note(note));
            }
            output.push(' ');
        }
        output.push_str(&self.game_result());
        output.push('\n');
        output
    }

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    fn html_escape(value: &str) -> String {
        value
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    }

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    fn report_board(
        board: Board,
        played: Option<ChessMove>,
        best: Option<ChessMove>,
        quality_color: &str,
        marker_id: usize,
    ) -> String {
        let mut html = String::from("<div class=\"board-wrap\"><div class=\"board\">");
        for rank_index in (0..8).rev() {
            for file_index in 0..8 {
                let square =
                    Square::make_square(Rank::from_index(rank_index), File::from_index(file_index));
                let piece = board.piece_on(square).and_then(|piece| {
                    let white = board.color_on(square)? == Color::White;
                    Some(match (white, piece) {
                        (true, Piece::Pawn) => '♙',
                        (true, Piece::Knight) => '♘',
                        (true, Piece::Bishop) => '♗',
                        (true, Piece::Rook) => '♖',
                        (true, Piece::Queen) => '♕',
                        (true, Piece::King) => '♔',
                        (false, Piece::Pawn) => '♟',
                        (false, Piece::Knight) => '♞',
                        (false, Piece::Bishop) => '♝',
                        (false, Piece::Rook) => '♜',
                        (false, Piece::Queen) => '♛',
                        (false, Piece::King) => '♚',
                    })
                });
                let shade = if (rank_index + file_index) % 2 == 1 {
                    "light"
                } else {
                    "dark"
                };
                let style = if played.is_some_and(|value| value.get_dest() == square) {
                    format!(" style=\"background:{quality_color}\"")
                } else if played.is_some_and(|value| value.get_source() == square) {
                    format!(" style=\"box-shadow:inset 0 0 0 3px {quality_color}\"")
                } else {
                    String::new()
                };
                html.push_str(&format!(
                    "<div class=\"sq {shade}\"{style}>{}</div>",
                    piece.unwrap_or(' ')
                ));
            }
        }
        html.push_str("</div>");
        if let Some(best) = best {
            let source = best.get_source();
            let destination = best.get_dest();
            let x1 = source.get_file().to_index() * 100 + 50;
            let y1 = (7 - source.get_rank().to_index()) * 100 + 50;
            let x2 = destination.get_file().to_index() * 100 + 50;
            let y2 = (7 - destination.get_rank().to_index()) * 100 + 50;
            html.push_str(&format!(
                "<svg class=\"best-arrow\" viewBox=\"0 0 800 800\" aria-label=\"Stockfish best move\"><defs><marker id=\"arrow-{marker_id}\" markerWidth=\"3.2\" markerHeight=\"3.2\" refX=\"2.35\" refY=\"1.6\" orient=\"auto\"><path d=\"M0,0 L3.2,1.6 L0,3.2 Z\" fill=\"#4d996a\" stroke=\"#111713\" stroke-width=\"0.28\" stroke-opacity=\"0.8\" stroke-linejoin=\"round\" paint-order=\"stroke fill\"/></marker></defs><line class=\"arrow-outline\" x1=\"{x1}\" y1=\"{y1}\" x2=\"{x2}\" y2=\"{y2}\" stroke=\"#111713\" stroke-width=\"19\" stroke-opacity=\"0.72\" stroke-linecap=\"round\"/><line class=\"arrow-fill\" x1=\"{x1}\" y1=\"{y1}\" x2=\"{x2}\" y2=\"{y2}\" stroke=\"#4d996a\" stroke-width=\"15\" stroke-opacity=\"0.94\" stroke-linecap=\"round\" marker-end=\"url(#arrow-{marker_id})\"/></svg>"
            ));
        }
        html.push_str("</div>");
        html
    }

    fn report_evaluation(analysis: Option<&PositionAnalysis>) -> String {
        analysis.map_or_else(
            || "-".to_owned(),
            |value| {
                value.mate.map_or_else(
                    || {
                        value
                            .eval_cp
                            .map_or_else(|| "-".into(), |cp| format!("{:+.2}", cp as f32 / 100.0))
                    },
                    |mate| format!("M{mate}"),
                )
            },
        )
    }

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    fn analysis_report_html(&self) -> String {
        let white = Self::html_escape(&self.review_white_player);
        let black = Self::html_escape(&self.review_black_player);
        let analyzed = self
            .game_analysis
            .iter()
            .filter(|entry| entry.is_some())
            .count();
        let total = self.review_positions.len();
        let quality_color = |classification: MoveClassification| match classification {
            MoveClassification::Best => "#a77a20",
            MoveClassification::Good => "#3f8752",
            MoveClassification::Inaccuracy => "#b88a12",
            MoveClassification::Mistake => "#c76525",
            MoveClassification::Blunder => "#b53535",
        };
        let move_cell = |move_index: usize| {
            let position_index = move_index + 1;
            let san = self
                .review_moves
                .get(move_index)
                .map(String::as_str)
                .unwrap_or("-");
            let san = self.display_san(san);
            let classification = self.move_classification(position_index);
            let symbol = classification
                .map(MoveClassification::symbol)
                .unwrap_or("·");
            let label = classification
                .map(MoveClassification::label)
                .unwrap_or("NOT ANALYZED");
            let color = classification.map(&quality_color).unwrap_or("#858b86");
            let evaluation = Self::report_evaluation(
                self.game_analysis
                    .get(position_index)
                    .and_then(Option::as_ref),
            );
            let cpl = classification
                .filter(|value| {
                    matches!(
                        value,
                        MoveClassification::Inaccuracy
                            | MoveClassification::Mistake
                            | MoveClassification::Blunder
                    )
                })
                .and_then(|_| self.move_centipawn_loss(position_index))
                .map_or_else(String::new, |value| {
                    format!("<span class=\"cpl\">{value} CPL</span>")
                });
            let cpl = if let Some(outcome) = self.move_mate_outcome(position_index) {
                format!("<span class=\"cpl\">{}</span>", outcome.label())
            } else {
                cpl
            };
            format!(
                "<div class=\"move\" title=\"{label}\"><span class=\"marker\" style=\"color:{color}\">{symbol}</span><span class=\"san\">{}</span><span class=\"eval\">{evaluation}</span>{cpl}</div>",
                Self::html_escape(&san)
            )
        };
        let mut move_rows = String::new();
        for move_index in (0..self.review_moves.len()).step_by(2) {
            let white_move = move_cell(move_index);
            let black_move = if move_index + 1 < self.review_moves.len() {
                move_cell(move_index + 1)
            } else {
                "<div class=\"move empty-move\">-</div>".into()
            };
            move_rows.push_str(&format!(
                "<div class=\"move-row\"><b>{}.</b>{white_move}{black_move}</div>",
                move_index / 2 + 1
            ));
        }
        let quality_counts = [
            MoveClassification::Best,
            MoveClassification::Good,
            MoveClassification::Inaccuracy,
            MoveClassification::Mistake,
            MoveClassification::Blunder,
        ]
        .map(|classification| {
            let count = (1..self.review_positions.len())
                .filter(|index| self.move_classification(*index) == Some(classification))
                .count();
            format!(
                "<span style=\"color:{}\">{} {} {count}</span>",
                quality_color(classification),
                classification.symbol(),
                classification.label()
            )
        })
        .join("");
        let verification_note = if self.verification_is_complete() {
            format!(
                "Selected critical positions rechecked at {} nodes",
                self.verification_nodes
            )
        } else if !self.verification_targets.is_empty() {
            format!(
                "Critical labels provisional · {}/{} recheck positions complete",
                self.verification_completed_count(),
                self.verification_targets.len()
            )
        } else {
            "Critical labels provisional · single-pass analysis".to_owned()
        };
        let quality_counts = format!("{quality_counts}<span>{verification_note}</span>");
        let mut user_notes = String::new();
        for (index, note) in self
            .move_notes
            .iter()
            .enumerate()
            .filter(|(_, note)| !note.is_empty())
        {
            user_notes.push_str(&format!("<section style=\"break-inside:avoid\"><h3>{}</h3><p style=\"white-space:pre-wrap;overflow-wrap:anywhere\">{}</p></section>",
                Self::html_escape(&self.graph_position_label(index,true)), Self::html_escape(note)));
        }
        if !user_notes.is_empty() {
            user_notes = format!("<h1>Your notes</h1>{user_notes}");
        }
        let mut critical = String::new();
        for index in 1..self.review_positions.len() {
            let Some(classification) = self.move_classification(index) else {
                continue;
            };
            if matches!(
                classification,
                MoveClassification::Best | MoveClassification::Good
            ) {
                continue;
            }
            let analysis = self.game_analysis.get(index).and_then(Option::as_ref);
            let before = self.game_analysis.get(index - 1).and_then(Option::as_ref);
            let move_number = (index + 1) / 2;
            let side = if index % 2 == 1 { "White" } else { "Black" };
            let san = Self::html_escape(
                &self.display_san(
                    self.review_moves
                        .get(index - 1)
                        .map(String::as_str)
                        .unwrap_or("-"),
                ),
            );
            let evaluation = Self::report_evaluation(analysis);
            let best = before
                .and_then(|value| value.best_move.as_deref())
                .unwrap_or("-");
            let pv = before
                .map(|value| value.pv.as_str())
                .filter(|value| !value.is_empty())
                .unwrap_or("-");
            let best = self.display_san(best);
            let pv = self.display_san_line(pv);
            let cpl = self.move_centipawn_loss(index).map_or_else(
                || {
                    self.move_mate_outcome(index)
                        .map_or_else(|| "-".into(), |outcome| outcome.label().into())
                },
                |value| value.to_string(),
            );
            let color = quality_color(classification);
            let decision_board = self.review_positions[index - 1];
            let played = self
                .review_moves
                .get(index - 1)
                .and_then(|value| Self::parse_san_move(&decision_board, value));
            let best_move = before
                .and_then(|value| value.best_move.as_deref())
                .and_then(Self::parse_uci_value);
            critical.push_str(&format!(
                "<section class=\"moment\"><div>{}</div><div class=\"notes\"><p class=\"kicker\" style=\"color:{color}\">{} {} - MOVE {move_number}</p><h2>{side} played {san}</h2><dl><dt>Evaluation</dt><dd>{evaluation}</dd><dt>Loss / mate</dt><dd>{cpl}</dd><dt>Best move</dt><dd>{}</dd><dt>Depth / nodes</dt><dd>{} / {}</dd></dl><h3>Engine line</h3><p class=\"pv\">{}</p></div></section>",
                Self::report_board(decision_board, played, best_move, color, index),
                classification.symbol(),
                classification.label(),
                Self::html_escape(&best),
                analysis.map_or(0, |value| value.depth),
                analysis.map_or(0, |value| value.nodes),
                Self::html_escape(&pv),
            ));
        }
        if critical.is_empty() {
            critical.push_str("<p class=\"empty\">No inaccuracies, mistakes, or blunders were found in the available analysis.</p>");
        }
        let generated = Self::html_escape(
            &Self::pgn_tag(&self.pgn_input, "Date").unwrap_or_else(|| "Undated game".into()),
        );
        let white_accuracy = self
            .analysis_accuracy(Color::White)
            .map_or_else(|| "-".into(), |value| format!("{value:.1}%"));
        let black_accuracy = self
            .analysis_accuracy(Color::Black)
            .map_or_else(|| "-".into(), |value| format!("{value:.1}%"));
        let white_cpl = self
            .analysis_average_centipawn_loss(Color::White)
            .map_or_else(|| "-".into(), |value| format!("{value:.1}"));
        let black_cpl = self
            .analysis_average_centipawn_loss(Color::Black)
            .map_or_else(|| "-".into(), |value| format!("{value:.1}"));
        let phase_summary = [
            GamePhase::Opening,
            GamePhase::Middlegame,
            GamePhase::Endgame,
        ]
        .into_iter()
        .filter_map(|phase| {
            let (moves, accuracy, average) = self.phase_stats(phase)?;
            let average = average.map_or_else(|| "—".into(), |value| format!("{value:.1}"));
            Some(format!(
                "<div class=\"phase\"><b>{}</b><strong>{accuracy:.1}%</strong><small>{average} average CPL · {moves} moves</small></div>",
                phase.label()
            ))
        })
        .collect::<String>();
        let turning_point = self.turning_point_index().map_or_else(String::new, |index| {
            let classification = self
                .move_classification(index)
                .unwrap_or(MoveClassification::Good);
            let color = quality_color(classification);
            let decision_board = self.review_positions[index - 1];
            let mover = decision_board.side_to_move();
            let side = if mover == Color::White { "White" } else { "Black" };
            let player = if mover == Color::White { &white } else { &black };
            let san = Self::html_escape(
                &self.display_san(
                    self.review_moves
                        .get(index - 1)
                        .map(String::as_str)
                        .unwrap_or("-"),
                ),
            );
            let move_label = if index % 2 == 1 {
                format!("{}. {san}", index.div_ceil(2))
            } else {
                format!("{}... {san}", index / 2)
            };
            let before = self.game_analysis.get(index - 1).and_then(Option::as_ref);
            let after = self.game_analysis.get(index).and_then(Option::as_ref);
            let best_move = before
                .and_then(|value| value.best_move.as_deref())
                .and_then(Self::parse_uci_value);
            let best = best_move
                .map(|chess_move| Self::san_for_move(&decision_board, chess_move))
                .unwrap_or_else(|| "-".to_owned());
            let best = self.display_san(&best);
            let played = self
                .review_moves
                .get(index - 1)
                .and_then(|value| Self::parse_san_move(&decision_board, value));
            let loss_text = self.move_mate_outcome(index)
                .map(|outcome| outcome.explanation().to_owned())
                .or_else(|| self.move_centipawn_loss(index).map(|loss| format!("Largest move-quality drop in the game · {loss} estimated CPL.")))
                .unwrap_or_default();
            format!(
                "<section class=\"turning-point\"><div>{}</div><div><p class=\"kicker\" style=\"color:{color}\">TURNING POINT · {} {}</p><h2>{side} — {player} played {move_label}</h2><p>{loss_text}</p><dl><dt>Before the move</dt><dd>{}</dd><dt>After the move</dt><dd>{}</dd><dt>Stockfish preferred</dt><dd>{}</dd></dl></div></section>",
                Self::report_board(decision_board, played, best_move, color, 10_000 + index),
                classification.symbol(),
                classification.label(),
                Self::report_evaluation(before),
                Self::report_evaluation(after),
                Self::html_escape(&best),
            )
        });
        format!(
            r#"<!doctype html><html><head><meta charset="utf-8"><title>Ironwood analysis - {white} vs {black}</title><style>
@page{{size:A4 portrait;margin:15mm}}*{{box-sizing:border-box}}body{{margin:0;color:#202521;background:#fff;font:10.5pt/1.45 Arial,sans-serif}}header{{padding:0 0 10mm;border-bottom:2px solid #c89e45}}.brand{{font-size:9pt;letter-spacing:.18em;color:#80601e}}h1{{margin:3mm 0 1mm;font:26pt Georgia,serif}}.meta{{color:#68706a}}.summary{{display:grid;grid-template-columns:repeat(2,1fr);gap:3mm;margin:8mm 0}}.metric{{padding:4mm;background:#f0f2ed;border-top:2px solid #c89e45}}.metric .side{{display:block;font-size:8pt;font-weight:bold;letter-spacing:.13em;color:#68706a}}.metric strong{{display:block;font-size:16pt;color:#9b7425}}.metric b{{display:block;font-size:11pt}}.metric small{{display:block;color:#68706a}}.phases{{display:grid;grid-template-columns:repeat(3,1fr);gap:3mm;margin:0 0 6mm}}.phase{{padding:3mm;background:#f0f2ed;border-left:2px solid #c89e45}}.phase b,.phase strong,.phase small{{display:block}}.phase strong{{font-size:14pt;color:#9b7425}}.phase small{{color:#68706a}}.turning-point{{display:grid;grid-template-columns:52mm 1fr;gap:7mm;padding:5mm;background:#f6f4ed;border:1px solid #d6c18f;break-inside:avoid;page-break-inside:avoid;margin:0 0 6mm}}.turning-point .board-wrap,.turning-point .board{{width:52mm;height:52mm}}.turning-point .sq{{font-size:16pt}}.turning-point h2{{font-size:15pt;margin-bottom:2mm}}.turning-point p{{margin:1mm 0 2mm}}.report-legend{{display:flex;flex-wrap:wrap;gap:3mm 6mm;margin:3mm 0 5mm;font-size:8.5pt;font-weight:bold}}.report-legend span{{white-space:nowrap}}.arrow-key{{color:#397a54;text-shadow:0 0 1px #111}}.move-list{{margin-bottom:7mm;border-top:1px solid #ccd1cc}}.move-row{{display:grid;grid-template-columns:9mm 1fr 1fr;gap:2mm;padding:1.2mm 0;border-bottom:1px solid #e5e7e4;break-inside:avoid}}.move-row>b{{color:#727872;text-align:right}}.move{{display:grid;grid-template-columns:7mm minmax(20mm,1fr) 13mm auto;gap:1mm;align-items:center;min-width:0}}.marker{{font-weight:bold}}.san{{font-weight:bold}}.eval{{color:#68706a;font-variant-numeric:tabular-nums}}.cpl{{color:#7d4b25;font-size:7.5pt;white-space:nowrap}}.critical-break{{break-before:page;page-break-before:always}}.moment{{display:grid;grid-template-columns:76mm 1fr;gap:8mm;padding:9mm 0;border-top:1px solid #d9ddd7;break-inside:avoid;page-break-inside:avoid}}.board-wrap{{position:relative;width:76mm;height:76mm}}.board{{width:76mm;height:76mm;display:grid;grid-template-columns:repeat(8,1fr);grid-template-rows:repeat(8,1fr);border:1px solid #425448}}.sq{{display:grid;place-items:center;min-width:0;min-height:0;font:23pt/1 "DejaVu Sans","Segoe UI Symbol",serif}}.light{{background:#dbe2d3}}.dark{{background:#557c66}}.best-arrow{{position:absolute;inset:0;width:100%;height:100%;pointer-events:none}}.kicker{{margin:0;font-weight:bold;letter-spacing:.08em}}h2{{margin:1mm 0 4mm;font:18pt Georgia,serif}}h3{{margin:4mm 0 1mm;font-size:9pt;text-transform:uppercase;letter-spacing:.1em;color:#737a74}}dl{{display:grid;grid-template-columns:32mm 1fr;margin:0}}dt,dd{{margin:0;padding:1.2mm 0;border-bottom:1px solid #e4e6e2}}dt{{color:#68706a}}dd{{font-weight:bold}}.pv{{font-family:Consolas,monospace;font-size:9pt}}.empty{{padding:12mm;background:#f0f2ed}}.print-guidance{{position:sticky;top:0;z-index:5;margin:0 0 5mm;padding:3mm 4mm;background:#fff3cf;border:1px solid #c89e45;color:#4d3b14;font-weight:bold}}footer{{margin-top:10mm;padding-top:4mm;border-top:1px solid #bbb;color:#737a74;font-size:8pt}}@media screen{{html.printing{{overflow:hidden;background:#2b2b2b}}html.printing body{{visibility:hidden}}}}@media print{{body{{print-color-adjust:exact;-webkit-print-color-adjust:exact}}.print-guidance{{display:none}}}}</style></head><body><div class="print-guidance">Print preview is opening. Edge may pause Ironwood until you finish printing or close this window.</div><header><div class="brand">IRONWOOD CHESS - STOCKFISH 19 FULL NNUE</div><h1>{white} vs {black}</h1><div class="meta">{generated} - {analyzed} of {total} positions analyzed - {}</div></header><section class="summary"><div class="metric"><span class="side">WHITE</span><strong>{white_accuracy}</strong><b>{white}</b><small>{white_cpl} average CPL</small></div><div class="metric"><span class="side">BLACK</span><strong>{black_accuracy}</strong><b>{black}</b><small>{black_cpl} average CPL</small></div></section><section class="phases">{phase_summary}</section>{turning_point}<div class="report-legend">{quality_counts}<span class="arrow-key">↗ Stockfish best move</span></div><h1>Game moves</h1><div class="move-list">{move_rows}</div><div class="critical-break"><h1>Critical positions</h1>{critical}</div>{user_notes}<footer>Generated locally by Ironwood Chess. Stockfish 19 full NNUE - {} threads - {} MiB hash - {} nodes per position.</footer><script>addEventListener('beforeprint',()=>document.documentElement.classList.add('printing'));addEventListener('afterprint',()=>close());addEventListener('load',()=>setTimeout(()=>print(),900));</script></body></html>"#,
            self.game_result(),
            self.analysis_config.threads,
            self.analysis_config.hash_mib,
            self.full_game_analysis_config
                .quality
                .nodes(self.full_game_analysis_config.custom_nodes),
        )
    }

    #[cfg(target_arch = "wasm32")]
    fn print_analysis_report(&self) -> Result<(), JsValue> {
        let html = self.analysis_report_html();
        let parts = js_sys::Array::new();
        parts.push(&JsValue::from_str(&html));
        let options = web_sys::BlobPropertyBag::new();
        options.set_type("text/html;charset=utf-8");
        let blob = web_sys::Blob::new_with_str_sequence_and_options(&parts, &options)?;
        let url = web_sys::Url::create_object_url_with_blob(&blob)?;
        let opened = web_sys::window()
            .ok_or_else(|| JsValue::from_str("Window unavailable"))?
            .open_with_url_and_target(&url, "_blank")?;
        if opened.is_none() {
            return Err(JsValue::from_str("The report window was blocked"));
        }
        Ok(())
    }

    pub(crate) fn scoresheet_preview(entries: &[String]) -> serde_json::Value {
        let mut board = Board::default();
        let mut positions = vec![board.to_string()];
        let mut moves = Vec::new();
        let mut highlights = Vec::new();
        let mut statuses = Vec::new();
        let mut stopped = false;
        for (index, entry) in entries.iter().enumerate() {
            let token = entry.trim().replace('0', "O");
            if stopped {
                statuses.push(if token.is_empty() { "empty" } else { "blocked" });
                continue;
            }
            if token.is_empty() {
                let gap = entries[index + 1..].iter().any(|value| !value.trim().is_empty());
                statuses.push(if gap { "missing" } else { "empty" });
                stopped = true;
            } else if let Some(mv) = Self::parse_san_move(&board, &token).or_else(|| {
                let mut candidates = MoveGen::new_legal(&board).filter(|mv| {
                    Self::san_for_move(&board, *mv).trim_end_matches(['+', '#'])
                        .eq_ignore_ascii_case(token.trim_end_matches(['+', '#']))
                });
                let found = candidates.next()?;
                candidates.next().is_none().then_some(found)
            }) {
                moves.push(Self::san_for_move(&board, mv));
                highlights.push(serde_json::json!({"from": mv.get_source().to_string(), "to": mv.get_dest().to_string()}));
                board = board.make_move_new(mv);
                positions.push(board.to_string());
                statuses.push("valid");
            } else {
                let incomplete = MoveGen::new_legal(&board).any(|mv| Self::san_for_move(&board, mv).to_ascii_lowercase().starts_with(&token.to_ascii_lowercase()));
                statuses.push(if incomplete { "incomplete" } else { "invalid" });
                stopped = true;
            }
        }
        serde_json::json!({"positions": positions, "moves": moves, "statuses": statuses, "highlights": highlights})
    }

    fn parse_san_move(board: &Board, san: &str) -> Option<ChessMove> {
        if let Ok(chess_move) = ChessMove::from_san(board, san) {
            return Some(chess_move);
        }
        let wanted = san.trim_end_matches(['+', '#']);
        let mut matches = MoveGen::new_legal(board).filter(|candidate| {
            Self::san_for_move(board, *candidate).trim_end_matches(['+', '#']) == wanted
        });
        let found = matches.next()?;
        matches.next().is_none().then_some(found)
    }

    pub(crate) fn parse_pgn_mainline(text: &str) -> Result<(Vec<Board>, Vec<String>), String> {
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
            let chess_move = Self::parse_san_move(&board, token)
                .ok_or_else(|| format!("Could not parse move {}: {token}", moves.len() + 1))?;
            board = board.make_move_new(chess_move);
            moves.push(token.to_owned());
            positions.push(board);
        }
        if moves.is_empty() {
            return Err("No legal mainline moves were found in the PGN.".to_owned());
        }
        Ok((positions, moves))
    }

    fn split_pgn_games(text: &str) -> Vec<String> {
        let mut games = Vec::new();
        let mut current = String::new();
        let mut seen_moves = false;
        let mut brace_depth = 0_i32;
        for line in text.lines() {
            let trimmed = line.trim();
            let tag = brace_depth == 0
                && trimmed.starts_with('[')
                && trimmed.ends_with(']')
                && trimmed.contains(" \"");
            if tag && seen_moves {
                if !current.trim().is_empty() {
                    games.push(current.trim().to_owned());
                }
                current.clear();
                seen_moves = false;
            }
            current.push_str(line);
            current.push('\n');
            if !tag
                && brace_depth == 0
                && !trimmed.is_empty()
                && !trimmed.starts_with(';')
                && !trimmed.starts_with('%')
            {
                seen_moves = true;
            }
            for ch in line.chars() {
                if ch == '{' {
                    brace_depth += 1;
                } else if ch == '}' {
                    brace_depth = (brace_depth - 1).max(0);
                }
            }
        }
        if !current.trim().is_empty() {
            games.push(current.trim().to_owned());
        }
        games
    }

    fn pgn_annotation_directive<'a>(comment: &'a str, name: &str) -> Option<&'a str> {
        let marker = format!("[%{name} ");
        let start = comment.find(&marker)? + marker.len();
        let end = comment[start..].find(']')? + start;
        Some(comment[start..end].trim())
    }

    fn parse_pgn_annotations(text: &str, positions: &[Board]) -> Vec<Option<PositionAnalysis>> {
        let mut analyses = vec![None; positions.len()];
        Self::visit_pgn_comments(text, positions, |comment, index| {
            Self::restore_pgn_comment(comment, index, &mut analyses)
        });
        analyses
    }

    fn parse_pgn_notes(text: &str, positions: &[Board]) -> Vec<String> {
        let mut notes = vec![String::new(); positions.len()];
        Self::visit_pgn_comments(text, positions, |comment, index| {
            if let Some(value) = Self::pgn_annotation_directive(comment, "note")
                && let Ok(note) = serde_json::from_str::<String>(value)
                && let Some(target) = notes.get_mut(index)
            {
                *target = note;
            }
        });
        notes
    }

    fn pgn_note(note: &str) -> String {
        let value = serde_json::to_string(note)
            .unwrap_or_default()
            .replace('{', "\\u007b")
            .replace('}', "\\u007d")
            .replace('[', "\\u005b")
            .replace(']', "\\u005d");
        format!(" {{ [%note {value}] }} ")
    }

    fn visit_pgn_comments(text: &str, positions: &[Board], visit: impl FnMut(&str, usize)) {
        Self::visit_pgn_content(text, positions, visit, |_, _| {});
    }

    fn parse_pgn_nags(text: &str, positions: &[Board]) -> Vec<Vec<u8>> {
        let mut annotations = vec![Vec::new(); positions.len()];
        Self::visit_pgn_content(text, positions, |_, _| {}, |token, index| {
            let nag = token.strip_prefix('$').and_then(|value| value.parse::<u8>().ok())
                .or_else(|| match token.trim_end_matches(|c| c != '!' && c != '?') {
                    value if value.ends_with("!!") => Some(3), value if value.ends_with("??") => Some(4),
                    value if value.ends_with("!?") => Some(5), value if value.ends_with("?!") => Some(6),
                    value if value.ends_with('!') => Some(1), value if value.ends_with('?') => Some(2), _ => None,
                });
            if let Some(nag) = nag && let Some(values) = annotations.get_mut(index) && !values.contains(&nag) {
                values.push(nag);
            }
        });
        annotations
    }

    fn visit_pgn_content(text: &str, positions: &[Board], mut visit: impl FnMut(&str, usize), mut visit_token: impl FnMut(&str, usize)) {
        let move_text = text
            .lines()
            .filter(|line| !line.trim_start().starts_with('['))
            .collect::<Vec<_>>()
            .join("\n");
        let mut board = positions.first().copied().unwrap_or_default();
        let mut move_count = 0_usize;
        let mut variation_depth = 0_u32;
        let mut line_comment = false;
        let mut comment = String::new();
        let mut token = String::new();
        let mut in_comment = false;

        let mut consume_token = |raw: &mut String, board: &mut Board, move_count: &mut usize| {
            if raw.is_empty() {
                raw.clear();
                return;
            }
            let mut value = raw.as_str();
            if let Some((_, suffix)) = value.rsplit_once('.') {
                value = suffix;
            }
            value = value.trim_matches(|character| matches!(character, '!' | '?'));
            if !value.is_empty()
                && !value.starts_with('$')
                && !matches!(value, "1-0" | "0-1" | "1/2-1/2" | "*")
                && let Some(chess_move) = Self::parse_san_move(board, value)
            {
                *board = board.make_move_new(chess_move);
                *move_count += 1;
            }
            visit_token(raw, *move_count);
            raw.clear();
        };

        for character in move_text.chars().chain(std::iter::once(' ')) {
            if line_comment {
                if character == '\n' {
                    line_comment = false;
                }
                continue;
            }
            if in_comment {
                if character == '}' {
                    in_comment = false;
                    visit(&comment, move_count);
                    comment.clear();
                } else {
                    comment.push(character);
                }
                continue;
            }
            match character {
                ';' if variation_depth == 0 => {
                    consume_token(&mut token, &mut board, &mut move_count);
                    line_comment = true;
                }
                '{' if variation_depth == 0 => {
                    consume_token(&mut token, &mut board, &mut move_count);
                    in_comment = true;
                }
                '(' => {
                    consume_token(&mut token, &mut board, &mut move_count);
                    variation_depth += 1;
                }
                ')' if variation_depth > 0 => variation_depth -= 1,
                value if value.is_whitespace() => {
                    consume_token(&mut token, &mut board, &mut move_count);
                }
                value if variation_depth == 0 => token.push(value),
                _ => {}
            }
        }
    }

    fn restore_pgn_comment(
        comment: &str,
        move_count: usize,
        analyses: &mut [Option<PositionAnalysis>],
    ) {
        if move_count == 0 || move_count >= analyses.len() {
            return;
        }
        let eval = Self::pgn_annotation_directive(comment, "eval");
        let depth = Self::pgn_annotation_directive(comment, "depth");
        let nodes = Self::pgn_annotation_directive(comment, "nodes");
        if eval.is_none() && depth.is_none() && nodes.is_none() {
            return;
        }

        let result = analyses[move_count].get_or_insert_with(PositionAnalysis::default);
        if let Some(value) = eval {
            if let Some(mate) = value.strip_prefix('#').and_then(|value| value.parse().ok()) {
                result.mate = Some(mate);
            } else if let Ok(pawns) = value.parse::<f32>() {
                result.eval_cp = Some((pawns * 100.0).round() as i32);
            }
        }
        if let Some(value) = depth.and_then(|value| value.parse().ok()) {
            result.depth = value;
        }
        if let Some(value) = nodes.and_then(|value| value.parse().ok()) {
            result.nodes = value;
        }

        let before = analyses[move_count - 1].get_or_insert_with(PositionAnalysis::default);
        for section in comment.split(';').map(str::trim) {
            if let Some(value) = section.strip_prefix("Best move: ") {
                before.best_move = Some(value.trim().to_owned());
            } else if let Some(value) = section.strip_prefix("PV: ") {
                before.pv = value.trim().to_owned();
            }
        }
    }

    fn positions_from_san_moves(moves: &[String]) -> Result<(Vec<Board>, Vec<String>), String> {
        let mut board = Board::default();
        let mut positions = vec![board];
        for (index, san) in moves.iter().enumerate() {
            let chess_move = Self::parse_san_move(&board, san)
                .ok_or_else(|| format!("Could not restore move {}: {san}", index + 1))?;
            board = board.make_move_new(chess_move);
            positions.push(board);
        }
        Ok((positions, moves.to_vec()))
    }

    fn load_pgn(&mut self, text: &str) -> bool {
        match Self::parse_pgn_mainline(text) {
            Ok((positions, moves)) => {
                self.settle_training();
                self.training = None;
                self.new_game_training = false;
                self.local_clock = None;
                self.local_resigned_white = None;
                self.local_start_ply = 0;
                let imported_analysis = Self::parse_pgn_annotations(text, &positions);
                let restored_analysis = imported_analysis.iter().any(Option::is_some);
                self.review_white_player =
                    Self::pgn_tag(text, "White").unwrap_or_else(|| "White".into());
                self.review_black_player =
                    Self::pgn_tag(text, "Black").unwrap_or_else(|| "Black".into());
                Self::set_page_title(true, &self.review_white_player, &self.review_black_player);
                #[cfg(target_arch = "wasm32")]
                if let Some(engine) = &self.engine {
                    engine.command("stop");
                }
                self.engine_searching = false;
                self.analysis_running = false;
                self.realtime_analysis_due_at = None;
                self.review_positions = positions;
                self.move_notes = Self::parse_pgn_notes(text, &self.review_positions);
                self.move_annotations = Self::parse_pgn_nags(text, &self.review_positions);
                self.drawing_undo.clear();
                self.board_marks = Self::parse_pgn_board_marks(text, &self.review_positions);
                self.note_editor_index = None;
                self.review_moves = moves;
                self.game_analysis = imported_analysis;
                self.game_analysis_index = None;
                self.game_analysis_running = false;
                self.game_analysis_paused = false;
                self.clear_verification();
                let final_index = self.review_positions.len() - 1;
                self.review_index = Some(final_index);
                self.review_scroll_to_selected = true;
                self.board = *self.review_positions.last().expect("review has positions");
                self.selected = None;
                self.legal_targets.clear();
                self.last_move =
                    Self::review_move_at(&self.review_positions, &self.review_moves, final_index);
                self.clear_analysis_result();
                self.engine_status = if restored_analysis {
                    "Annotated PGN loaded · analysis restored".into()
                } else {
                    "PGN loaded · ready to analyze".into()
                };
                self.pgn_input = text.to_owned();
                self.import_input.clear();
                self.pgn_error = None;
                self.pgn_dialog_open = false;
                #[cfg(target_arch = "wasm32")]
                start_new_stored_game("imported");
                self.save_game();
                restored_analysis
            }
            Err(error) => {
                self.pgn_error = Some(error);
                false
            }
        }
    }

    fn save_imported_pgn_without_opening(&self, text: &str) -> Result<(), String> {
        let (positions, moves) = Self::parse_pgn_mainline(text)?;
        #[cfg(target_arch = "wasm32")]
        {
            let final_board = *positions.last().expect("PGN has a position");
            let result = Self::pgn_tag(text, "Result")
                .filter(|value| matches!(value.as_str(), "1-0" | "0-1" | "1/2-1/2" | "*"))
                .unwrap_or_else(|| match final_board.status() {
                    BoardStatus::Stalemate => "1/2-1/2".into(),
                    BoardStatus::Checkmate if final_board.side_to_move() == Color::White => {
                        "0-1".into()
                    }
                    BoardStatus::Checkmate => "1-0".into(),
                    BoardStatus::Ongoing => "*".into(),
                });
            let last_move = Self::review_move_at(&positions, &moves, positions.len() - 1)
                .map(|chess_move| chess_move.to_string());
            let game = PersistedGame {
                training: None,
                local_clock: None,
                local_resigned_white: None,
                local_start_ply: 0,
                board: final_board.to_string(),
                final_board: final_board.to_string(),
                result,
                history: Vec::new(),
                last_move,
                flipped: false,
                workspace_mode: self.workspace_mode,
                show_coordinates: self.show_coordinates,
                show_board_frame: self.show_board_frame,
                piece_shadows: self.piece_shadows,
                show_best_move_arrows: self.show_best_move_arrows,
                show_move_hover_text: self.show_move_hover_text,
                figurine_notation: self.figurine_notation,
                move_sounds: self.move_sounds,
                animate_moves: self.animate_moves,
                piece_set: self.piece_set,
                engine_enabled: self.engine_enabled,
                player_side: self.player_side,
                engine_config: self.engine_config.clone(),
                analysis_config: self.analysis_config.clone(),
                full_game_analysis_config: self.full_game_analysis_config.clone(),
                review_pgn: Some(text.to_owned()),
                live_moves: Vec::new(),
                live_positions: Vec::new(),
                review_index: Some(positions.len() - 1),
                game_analysis: Self::parse_pgn_annotations(text, &positions),
                move_notes: Self::parse_pgn_notes(text, &positions),
                move_annotations: Self::parse_pgn_nags(text, &positions),
                board_marks: Self::parse_pgn_board_marks(text, &positions),
                game_analysis_running: false,
                game_analysis_paused: false,
                verification_targets: Vec::new(),
                verification_results: Vec::new(),
                verification_nodes: 0,
            };
            let json = serde_json::to_string(&game)
                .map_err(|error| format!("Could not prepare saved game: {error}"))?;
            store_imported_game(&json);
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = (positions, moves);
        Ok(())
    }

    fn load_import_text(&mut self) -> bool {
        let text = self.import_input.clone();
        if text.trim_start().starts_with('{') {
            self.load_analysis_json(&text)
        } else {
            self.load_pgn(&text)
        }
    }

    fn begin_import(&mut self) {
        #[cfg(target_arch = "wasm32")]
        ensure_imported_index();
        if !self.import_input.trim_start().starts_with('{') {
            let games = Self::split_pgn_games(&self.import_input);
            if games.len() > 1 {
                self.batch_pgn_games = games.into_iter().map(BatchPgnGame::new).collect();
                self.batch_pgn_players = self
                    .batch_pgn_games
                    .iter()
                    .flat_map(|game| [game.white.clone(), game.black.clone()])
                    .filter(|player| !player.is_empty() && player != "White" && player != "Black")
                    .collect();
                self.batch_pgn_players
                    .sort_by_key(|player| player.to_lowercase());
                self.batch_pgn_players.dedup();
                self.batch_pgn_player_filter = None;
                self.batch_pgn_player_search.clear();
                self.batch_pgn_page = 0;
                self.pgn_error = None;
                #[cfg(target_arch = "wasm32")]
                refresh_storage_status();
                self.import_input.clear();
                self.pgn_dialog_open = false;
                self.batch_pgn_dialog_open = true;
                return;
            }
            #[cfg(target_arch = "wasm32")]
            if !imported_index_ready() {
                self.pgn_error = Some(
                    "Still checking Saved Games for duplicates. Please try again shortly.".into(),
                );
                self.pgn_dialog_open = true;
                return;
            }
            #[cfg(target_arch = "wasm32")]
            if is_imported_duplicate(&self.import_input) {
                self.pgn_error = Some("This game is already in Saved Games.".into());
                self.pgn_dialog_open = true;
                return;
            }
        }
        #[cfg(target_arch = "wasm32")]
        self.import_game_with_options();
        #[cfg(not(target_arch = "wasm32"))]
        self.load_import_text();
    }

    #[cfg(target_arch = "wasm32")]
    fn import_game_with_options(&mut self) {
        let analyze_after_import = self.pgn_analyze_after_import;
        let restored_analysis = self.load_import_text();
        if analyze_after_import && !restored_analysis && self.pgn_error.is_none() {
            self.start_game_analysis();
        }
    }

    fn load_analysis_json(&mut self, text: &str) -> bool {
        let imported: ImportedAnalysisFile = match serde_json::from_str(text) {
            Ok(imported) => imported,
            Err(error) => {
                self.pgn_error = Some(format!("Could not read Ironwood JSON: {error}"));
                return false;
            }
        };
        if imported.schema != "ironwood.analyzed-game/v1" {
            self.pgn_error = Some(format!("Unsupported analysis schema: {}", imported.schema));
            return false;
        }
        if imported.positions.len() != imported.game.moves.len() + 1 {
            self.pgn_error =
                Some("The JSON position timeline does not match its number of moves.".into());
            return false;
        }
        let positions = match imported
            .positions
            .iter()
            .map(|position| Board::from_str(&position.fen))
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(positions) => positions,
            Err(_) => {
                self.pgn_error = Some("The JSON contains an invalid FEN position.".into());
                return false;
            }
        };
        for (index, san) in imported.game.moves.iter().enumerate() {
            let Some(chess_move) = Self::parse_san_move(&positions[index], san) else {
                self.pgn_error = Some(format!("The JSON contains an invalid move: {san}"));
                return false;
            };
            if positions[index].make_move_new(chess_move) != positions[index + 1] {
                self.pgn_error = Some(format!(
                    "The JSON timeline does not match the move at position {}.",
                    index + 1
                ));
                return false;
            }
        }

        #[cfg(target_arch = "wasm32")]
        if let Some(engine) = &self.engine {
            engine.command("stop");
        }
        self.engine_searching = false;
        self.analysis_running = false;
        self.realtime_analysis_due_at = None;
        self.game_analysis_running = false;
        self.game_analysis_paused = false;
        self.clear_verification();
        self.game_analysis_index = None;
        self.clear_analysis_result();
        let source_pgn = imported.game.source_pgn.unwrap_or_else(|| {
            let mut headers = String::new();
            if let Some(date) = imported.game.date.as_deref() {
                headers.push_str(&format!("[Date \"{}\"]\n", date.replace('"', "'")));
            }
            if let Some(site) = imported.game.site.as_deref() {
                headers.push_str(&format!("[Site \"{}\"]\n", site.replace('"', "'")));
            }
            if let Some(result) = imported.game.result.as_deref()
                && matches!(result, "1-0" | "0-1" | "1/2-1/2" | "*")
            {
                headers.push_str(&format!("[Result \"{result}\"]\n"));
            }
            headers
        });
        self.review_white_player = imported.game.white;
        self.review_black_player = imported.game.black;
        self.move_notes = imported
            .positions
            .iter()
            .map(|position| position.note.clone())
            .collect();
        self.move_annotations = imported.positions.iter().map(|position| position.annotations.clone()).collect();
        self.drawing_undo.clear();
        self.board_marks = imported.positions.iter().map(|position| position.board_marks.iter().filter(|mark| mark.valid()).cloned().collect()).collect();
        self.board_mark_drag = None;
        self.note_editor_index = None;
        self.review_moves = imported.game.moves;
        self.review_positions = positions;
        self.game_analysis = imported
            .positions
            .into_iter()
            .map(|position| {
                let analyzed = position.evaluation_cp.is_some()
                    || position.mate.is_some()
                    || position.depth.is_some()
                    || position.nodes.is_some()
                    || position.best_move.is_some()
                    || position.principal_variation.is_some();
                analyzed.then_some(PositionAnalysis {
                    eval_cp: position.evaluation_cp,
                    mate: position.mate,
                    depth: position.depth.unwrap_or_default(),
                    nodes: position.nodes.unwrap_or_default(),
                    best_move: position.best_move,
                    pv: position.principal_variation.unwrap_or_default(),
                })
            })
            .collect();
        let available_threads = Self::engine_thread_count();
        self.analysis_config.threads = imported.engine.threads.clamp(1, available_threads);
        self.analysis_config.hash_mib = imported.engine.hash_mib.clamp(16, 256);
        self.full_game_analysis_config.quality = match imported.engine.full_game_quality.as_str() {
            "Quick" => FullGameQuality::Quick,
            "Standard" => FullGameQuality::Standard,
            "Deep" => FullGameQuality::Deep,
            _ => FullGameQuality::Custom,
        };
        self.full_game_analysis_config.custom_nodes =
            imported.engine.nodes_per_position.clamp(10_000, 50_000_000);
        if let Some(verification) = imported.verification {
            let mut targets = verification.target_positions;
            targets.sort_unstable();
            targets.dedup();
            if !targets.is_empty()
                && targets.iter().all(|index| {
                    *index < self.game_analysis.len() && self.game_analysis[*index].is_some()
                })
            {
                self.verification_nodes = verification.nodes_per_position;
                self.verification_results = vec![None; self.game_analysis.len()];
                for index in &targets {
                    self.verification_results[*index] = self.game_analysis[*index].clone();
                }
                self.verification_targets = targets;
            }
        }
        let final_index = self.review_moves.len();
        self.review_index = Some(final_index);
        self.review_scroll_to_selected = true;
        self.board = self.review_positions[final_index];
        self.last_move =
            Self::review_move_at(&self.review_positions, &self.review_moves, final_index);
        self.selected = None;
        self.legal_targets.clear();
        self.pgn_input = source_pgn;
        self.pgn_input = self.annotated_pgn();
        self.import_input.clear();
        self.pgn_error = None;
        self.pgn_dialog_open = false;
        self.engine_status = "Ironwood analysis restored from JSON".into();
        Self::set_page_title(true, &self.review_white_player, &self.review_black_player);
        #[cfg(target_arch = "wasm32")]
        start_new_stored_game("imported");
        self.save_game();
        true
    }

    fn review_move_at(
        positions: &[Board],
        moves: &[String],
        position_index: usize,
    ) -> Option<ChessMove> {
        let move_index = position_index.checked_sub(1)?;
        Self::parse_san_move(positions.get(move_index)?, moves.get(move_index)?)
    }

    fn review_to(&mut self, index: usize) {
        if self.training_live() { return; }
        self.board_mark_drag = None;
        if self.review_positions.is_empty() {
            return;
        }
        self.realtime_analysis_due_at = None;
        self.best_move_attempt = None;
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
        self.schedule_realtime_analysis();
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
        if let Some(kingside) = board.castle_side(chess_move) {
            return if kingside { "O-O" } else { "O-O-O" }.to_owned();
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

    fn figurine_san(san: &str) -> String {
        let Some(first) = san.chars().next() else {
            return String::new();
        };
        let mut display = match first {
            'K' => format!("♔{}", &san[first.len_utf8()..]),
            'Q' => format!("♕{}", &san[first.len_utf8()..]),
            'R' => format!("♖{}", &san[first.len_utf8()..]),
            'B' => format!("♗{}", &san[first.len_utf8()..]),
            'N' => format!("♘{}", &san[first.len_utf8()..]),
            _ => san.to_owned(),
        };
        for (letter, figurine) in [('Q', '♕'), ('R', '♖'), ('B', '♗'), ('N', '♘')] {
            display = display.replace(&format!("={letter}"), &format!("={figurine}"));
        }
        display
    }

    fn display_san(&self, san: &str) -> String {
        if self.figurine_notation {
            Self::figurine_san(san)
        } else {
            san.to_owned()
        }
    }

    fn display_san_line(&self, line: &str) -> String {
        if self.figurine_notation {
            line.split_whitespace()
                .map(Self::figurine_san)
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            line.to_owned()
        }
    }

    fn move_hover_text(board: &Board, chess_move: ChessMove) -> String {
        let piece = board
            .piece_on(chess_move.get_source())
            .unwrap_or(Piece::Pawn);
        if let Some(kingside) = board.castle_side(chess_move) {
            return if kingside {
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

    fn load_stored_prediction(&mut self, root_index: usize) -> bool {
        let Some(root) = self.review_positions.get(root_index).copied() else {
            return false;
        };
        let Some(pv) = self
            .game_analysis
            .get(root_index)
            .and_then(Option::as_ref)
            .map(|analysis| analysis.pv.clone())
            .filter(|pv| !pv.is_empty())
        else {
            return false;
        };
        let mut board = root;
        let mut positions = vec![board];
        let mut moves = Vec::new();
        let mut chess_moves = Vec::new();
        for san in pv.split_whitespace().take(12) {
            let Some(chess_move) = Self::parse_san_move(&board, san) else {
                break;
            };
            moves.push(san.to_owned());
            chess_moves.push(chess_move);
            board = board.make_move_new(chess_move);
            positions.push(board);
        }
        if moves.is_empty() {
            return false;
        }
        self.prediction_positions = positions;
        self.prediction_moves = moves;
        self.prediction_chess_moves = chess_moves;
        self.prediction_index = 0;
        self.prediction_navigation_active = true;
        self.prediction_scroll_to_selected = true;
        self.engine_analysis_dock_collapsed = false;
        self.board = root;
        self.last_move = root_index.checked_sub(1).and_then(|index| {
            Self::review_move_at(&self.review_positions, &self.review_moves, index + 1)
        });
        self.selected = None;
        self.legal_targets.clear();
        true
    }

    fn actual_continuation(&self, position_index: usize, max_plies: usize) -> String {
        let start = position_index.saturating_sub(1);
        let mut line = String::new();
        for (offset, san) in self
            .review_moves
            .iter()
            .enumerate()
            .skip(start)
            .take(max_plies)
        {
            if !line.is_empty() {
                line.push(' ');
            }
            let move_number = offset / 2 + 1;
            if offset % 2 == 0 {
                line.push_str(&format!("{move_number}. {san}"));
            } else if offset == start {
                line.push_str(&format!("{move_number}... {san}"));
            } else {
                line.push_str(san);
            }
        }
        line
    }

    #[cfg(target_arch = "wasm32")]
    fn start_analysis(&mut self) {
        if self.training_live() { return; }
        if self.fics_active && !self.fics_game_finished {
            return;
        }
        if self.pgn_input.is_empty()
            && self.engine_enabled
            && self.review_index.is_none()
            && self.board.side_to_move() != self.player_side.color()
            && self.board.status() == BoardStatus::Ongoing
        {
            self.realtime_analysis_due_at = None;
            self.request_engine_move();
            return;
        }
        self.realtime_analysis_due_at = None;
        self.game_analysis_running = false;
        self.game_analysis_paused = false;
        self.resume_game_analysis_after_ready = false;
        self.game_analysis_index = None;
        self.save_game();
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
        engine.command(&format!(
            "setoption name UCI_Chess960 value {}",
            root.is_chess960()
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

    fn schedule_realtime_analysis(&mut self) {
        if self.training_live() { self.realtime_analysis_due_at = None; return; }
        let engine_move_pending = self.engine_enabled
            && self.review_index.is_none()
            && self.board.side_to_move() != self.player_side.color()
            && self.board.status() == BoardStatus::Ongoing;
        self.realtime_analysis_due_at = (!engine_move_pending
            && self.board.status() == BoardStatus::Ongoing)
            .then(|| Self::animation_time() + 0.75);
    }

    #[cfg(target_arch = "wasm32")]
    fn start_game_analysis(&mut self) {
        if self.training_live() { return; }
        if self.fics_active && !self.fics_game_finished {
            return;
        }
        if self.review_positions.is_empty() {
            return;
        }
        self.realtime_analysis_due_at = None;
        self.clear_verification();
        self.game_analysis = vec![None; self.review_positions.len()];
        self.game_analysis_running = true;
        self.game_analysis_paused = false;
        self.game_analysis_index = Some(0);
        self.resume_game_analysis_after_ready = !self.engine_ready;
        self.save_game();
        if self.engine_ready {
            self.start_game_analysis_position();
        }
    }

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    fn verification_budget(base_nodes: u64) -> u64 {
        base_nodes.saturating_mul(5).clamp(1_000_000, 50_000_000)
    }

    #[cfg(target_arch = "wasm32")]
    fn start_verification(&mut self) {
        if self.review_positions.is_empty()
            || self.game_analysis.len() != self.review_positions.len()
            || self.game_analysis.iter().any(Option::is_none)
        {
            return;
        }
        let targets = self.critical_verification_targets();
        if targets.is_empty() {
            return;
        }
        let base_nodes = self
            .full_game_analysis_config
            .quality
            .nodes(self.full_game_analysis_config.custom_nodes);
        let verification_nodes = Self::verification_budget(base_nodes);
        if verification_nodes <= base_nodes {
            return;
        }
        self.ignore_next_bestmove = self.analysis_running && self.engine_searching;
        self.realtime_analysis_due_at = None;
        self.resume_engine_after_ready = false;
        self.verification_nodes = verification_nodes;
        self.verification_targets = targets;
        self.verification_results = vec![None; self.review_positions.len()];
        self.resume_verification();
    }

    #[cfg(target_arch = "wasm32")]
    fn resume_verification(&mut self) {
        let Some(index) = self.verification_pending_index() else {
            return;
        };
        self.ignore_next_bestmove = self.analysis_running && self.engine_searching;
        self.realtime_analysis_due_at = None;
        self.game_analysis_index = Some(index);
        self.game_analysis_running = true;
        self.game_analysis_paused = false;
        self.resume_game_analysis_after_ready = !self.engine_ready;
        self.save_game();
        if self.engine_ready {
            self.start_game_analysis_position();
        }
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
        engine.command(&format!(
            "setoption name UCI_Chess960 value {}",
            root.is_chess960()
        ));
        engine.command(&format!("position fen {root}"));
        let nodes = if !self.verification_targets.is_empty() {
            self.verification_nodes
        } else {
            self.full_game_analysis_config
                .quality
                .nodes(self.full_game_analysis_config.custom_nodes)
        };
        self.pending_analysis_search = Some(format!("go nodes {nodes}"));
        self.analysis_running = true;
        self.engine_searching = false;
        self.engine_status = if !self.verification_targets.is_empty() {
            format!(
                "Preparing verification · {} of {} positions",
                self.verification_completed_count() + 1,
                self.verification_targets.len()
            )
        } else {
            format!(
                "Preparing game analysis · position {} of {}",
                index + 1,
                self.review_positions.len()
            )
        };
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
        self.resume_game_analysis_after_ready = false;
        self.engine_status = "Game analysis paused".into();
        self.save_game();
    }

    #[cfg(target_arch = "wasm32")]
    fn resume_game_analysis(&mut self) {
        if self.game_analysis_index.is_none() {
            return;
        }
        self.game_analysis_running = true;
        self.game_analysis_paused = false;
        self.resume_game_analysis_after_ready = !self.engine_ready;
        self.save_game();
        if self.engine_ready {
            self.start_game_analysis_position();
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn continue_game_analysis(&mut self) {
        let Some(index) = self.game_analysis.iter().position(Option::is_none) else {
            return;
        };
        self.game_analysis_index = Some(index);
        self.game_analysis_running = true;
        self.game_analysis_paused = false;
        self.resume_game_analysis_after_ready = !self.engine_ready;
        self.save_game();
        if self.engine_ready {
            self.start_game_analysis_position();
        }
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
        self.resume_game_analysis_after_ready = false;
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
        self.resume_game_analysis_after_ready = false;
        self.game_analysis_index = None;
        self.engine_status = "Analysis stopped".into();
        self.save_game();
        self.queue_live_engine_resume();
    }

    #[cfg(target_arch = "wasm32")]
    fn resume_live_engine_if_needed(&mut self) {
        if self.pgn_input.is_empty()
            && self.review_index.is_none()
            && self.board.side_to_move() != self.player_side.color()
            && self.board.status() == BoardStatus::Ongoing
        {
            self.request_engine_move();
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn queue_live_engine_resume(&mut self) {
        if self.pgn_input.is_empty()
            && self.review_index.is_none()
            && self.board.side_to_move() != self.player_side.color()
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
    fn load_preferences() -> Option<UserPreferences> {
        let storage = web_sys::window()?.local_storage().ok()??;
        let json = storage.get_item(PREFERENCES_KEY).ok()??;
        serde_json::from_str(&json).ok()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn load_preferences() -> Option<UserPreferences> {
        None
    }

    fn save_preferences(&self) {
        #[cfg(target_arch = "wasm32")]
        if let Some(storage) =
            web_sys::window().and_then(|window| window.local_storage().ok().flatten())
        {
            let preferences = UserPreferences {
                workspace_mode: self.workspace_mode,
                show_coordinates: self.show_coordinates,
                show_highlighted_move: self.show_highlighted_move,
                show_radial_light: self.show_radial_light,
                show_board_frame: self.show_board_frame,
                piece_shadows: self.piece_shadows,
                show_best_move_arrows: self.show_best_move_arrows,
                show_move_hover_text: self.show_move_hover_text,
                figurine_notation: self.figurine_notation,
                move_sounds: self.move_sounds,
                animate_moves: self.animate_moves,
                piece_set: self.piece_set,
                board_3d_active: self.board_3d_active,
                board_3d_theme: self.board_3d_theme,
                board_3d_appearance: self.board_3d_appearance,
                board_3d_appearance_customized: self.board_3d_appearance_customized,
            };
            if let Ok(json) = serde_json::to_string(&preferences) {
                let _ = storage.set_item(PREFERENCES_KEY, &json);
            }
        }
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

    fn persisted_game(&self) -> PersistedGame {
        PersistedGame {
            training: self.training.clone(),
            local_clock: self.local_clock.clone(),
            local_resigned_white: self.local_resigned_white,
            local_start_ply: self.local_start_ply,
            board: self.board.to_string(),
            final_board: self
                .review_positions
                .last()
                .unwrap_or(&self.board)
                .to_string(),
            result: self.game_result(),
            history: self.history.iter().map(ToString::to_string).collect(),
            last_move: self.last_move.map(|mv| mv.to_string()),
            flipped: self.flipped,
            workspace_mode: self.workspace_mode,
            show_coordinates: self.show_coordinates,
            show_board_frame: self.show_board_frame,
            piece_shadows: self.piece_shadows,
            show_best_move_arrows: self.show_best_move_arrows,
            show_move_hover_text: self.show_move_hover_text,
            figurine_notation: self.figurine_notation,
            move_sounds: self.move_sounds,
            animate_moves: self.animate_moves,
            piece_set: self.piece_set,
            engine_enabled: self.engine_enabled,
            player_side: self.player_side,
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
            move_notes: self.move_notes.clone(),
            move_annotations: self.move_annotations.clone(),
            board_marks: self.board_marks.clone(),
            game_analysis_running: self.game_analysis_running,
            game_analysis_paused: self.game_analysis_paused,
            verification_targets: self.verification_targets.clone(),
            verification_results: self.verification_results.clone(),
            verification_nodes: self.verification_nodes,
        }
    }

    fn observed_saved_game(&self, observed: &ObservedGame) -> PersistedGame {
        let mut game = self.persisted_game();
        game.training = None;
        game.move_notes = if self.fics_game_id == Some(observed.id) {
            self.move_notes.clone()
        } else {
            observed.move_notes.clone()
        };
        game.move_annotations = if self.fics_game_id == Some(observed.id) { self.move_annotations.clone() } else { observed.move_annotations.clone() };
        game.board_marks = if self.fics_game_id == Some(observed.id) { self.board_marks.clone() } else { observed.board_marks.clone() };
        let active = self.fics_game_id == Some(observed.id) && !self.fics_playing;
        let result = ["1-0", "0-1", "1/2-1/2"]
            .into_iter()
            .find(|result| {
                observed
                    .end_message
                    .as_deref()
                    .unwrap_or("")
                    .contains(result)
            })
            .unwrap_or("*");
        let mut pgn = fics_game_pgn(&observed.white, &observed.black, result, &observed.moves);
        if let Some(first) = observed.positions.first() {
            if observed.start_ply != 0 {
                pgn = format!("[SetUp \"1\"]\n[FEN \"{first}\"]\n{pgn}");
            }
        }
        game.local_clock = None;
        game.local_resigned_white = None;
        game.local_start_ply = observed.start_ply;
        game.board = observed.board.to_string();
        game.final_board = observed.board.to_string();
        game.result = result.into();
        game.history = observed
            .positions
            .iter()
            .take(observed.positions.len().saturating_sub(1))
            .map(ToString::to_string)
            .collect();
        game.last_move = None;
        game.flipped = observed.flipped;
        game.engine_enabled = false;
        game.review_pgn = Some(pgn);
        game.live_moves.clear();
        game.live_positions.clear();
        game.review_index = Some(observed.moves.len());
        game.game_analysis_running = false;
        if active {
            game.game_analysis_paused = self.game_analysis_running || self.game_analysis_paused;
        } else if let Some(analysis) = &observed.analysis {
            game.game_analysis = analysis.results.clone();
            game.game_analysis_paused = analysis.paused;
            game.verification_targets = analysis.verification_targets.clone();
            game.verification_results = analysis.verification_results.clone();
            game.verification_nodes = analysis.verification_nodes;
            game.full_game_analysis_config = analysis.config.clone();
        } else {
            game.game_analysis.clear();
            game.game_analysis_paused = false;
            game.verification_targets.clear();
            game.verification_results.clear();
            game.verification_nodes = 0;
        }
        game
    }

    fn save_observed_game(&self, id: i32) {
        #[cfg(target_arch = "wasm32")]
        if let Some(observed) = self
            .fics_observed_games
            .iter()
            .find(|game| game.id == id && game.ended)
        {
            if let Ok(json) = serde_json::to_string(&self.observed_saved_game(observed)) {
                store_observed_game(&json, &observed.storage_id);
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = id;
    }

    fn save_game(&self) {
        self.save_preferences();
        if !self.fics_playing {
            if let Some(id) = self
                .fics_game_id
                .filter(|id| self.fics_observed_games.iter().any(|game| game.id == *id))
            {
                self.save_observed_game(id);
                return;
            }
        }
        if self.fics_active && !self.fics_game_finished {
            return;
        }
        #[cfg(target_arch = "wasm32")]
        if let Some(storage) =
            web_sys::window().and_then(|window| window.local_storage().ok().flatten())
        {
            let game = self.persisted_game();
            if let Ok(json) = serde_json::to_string(&game) {
                let _ = storage.set_item(STORAGE_KEY, &json);
                store_current_game(&json);
            }
        }
    }

    fn reset_for_side(&mut self, player_side: PlayerSide) {
        self.settle_training();
        self.training = None;
        self.settle_training();
        self.training = None;
        self.new_game_training = false;
        self.local_clock = None;
        self.local_resigned_white = None;
        self.local_start_ply = 0;
        self.ignore_next_bestmove = self.engine_searching;
        self.pending_analysis_search = None;
        self.resume_game_analysis_after_ready = false;
        #[cfg(target_arch = "wasm32")]
        if let Some(engine) = &self.engine {
            engine.command("stop");
            engine.command("ucinewgame");
        }
        self.board = Board::default();
        self.player_side = player_side;
        self.flipped = player_side == PlayerSide::Black;
        self.selected = None;
        self.legal_targets.clear();
        self.history.clear();
        self.last_move = None;
        self.move_animation = None;
        self.promotion = None;
        self.engine_searching = false;
        self.analysis_running = false;
        self.realtime_analysis_due_at = None;
        self.review_positions.clear();
        self.review_moves.clear();
        self.move_notes.clear();
        self.move_annotations.clear();
        self.drawing_undo.clear();
        self.board_marks.clear();
        self.board_mark_drag = None;
        self.note_editor_index = None;
        self.game_analysis.clear();
        self.clear_verification();
        self.game_analysis_index = None;
        self.game_analysis_running = false;
        self.game_analysis_paused = false;
        self.review_index = None;
        self.review_scroll_to_selected = false;
        self.best_move_attempt = None;
        self.pgn_input.clear();
        self.review_white_player = if !self.engine_enabled {
            "White".into()
        } else if player_side == PlayerSide::White {
            "You".into()
        } else {
            self.engine_config.opponent.label().into()
        };
        self.review_black_player = if !self.engine_enabled {
            "Black".into()
        } else if player_side == PlayerSide::Black {
            "You".into()
        } else {
            self.engine_config.opponent.label().into()
        };
        self.clear_analysis_result();
        self.resume_engine_after_ready = false;
        let engine_should_move =
            self.engine_enabled && player_side == PlayerSide::Black && !self.fics_observing;
        self.engine_status = if engine_should_move {
            format!("Waiting for {}…", self.engine_config.opponent.label())
        } else {
            "Your move".into()
        };
        Self::set_page_title(false, "", "");
        #[cfg(target_arch = "wasm32")]
        if !self.fics_active {
            start_new_stored_game(if self.new_game_training { "training" } else { "mine" });
        }
        if !self.fics_observing && !self.new_game_training {
            self.save_game();
        }
        #[cfg(target_arch = "wasm32")]
        if engine_should_move {
            self.queue_live_engine_resume();
        }
    }

    fn play_from_current_position(&mut self) {
        if self.training_live() { return; }
        self.settle_training();
        self.training = None;
        self.new_game_training = false;
        #[cfg(target_arch = "wasm32")]
        start_new_stored_game("mine");
        self.local_clock = None;
        self.local_resigned_white = None;
        if self.fics_active {
            return;
        }
        self.best_move_attempt = None;
        self.move_animation = None;
        let branch_index = self
            .review_index
            .unwrap_or_else(|| self.review_positions.len().saturating_sub(1))
            .min(self.review_positions.len().saturating_sub(1));
        if let Some(branch_board) = self.review_positions.get(branch_index).copied() {
            self.review_positions.truncate(branch_index + 1);
            self.review_moves.truncate(branch_index);
            self.move_notes.truncate(branch_index + 1);
            self.move_annotations.truncate(branch_index + 1);
            self.drawing_undo.clear();
            self.board_marks.truncate(branch_index + 1);
            self.game_analysis.truncate(branch_index + 1);
            self.clear_verification();
            self.board = branch_board;
            self.history = self.review_positions[..branch_index].to_vec();
            self.last_move =
                Self::review_move_at(&self.review_positions, &self.review_moves, branch_index);
        }
        let engine_should_move =
            self.engine_enabled && self.board.side_to_move() != self.player_side.color();
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
        self.review_white_player = if !self.engine_enabled {
            "White".into()
        } else if self.player_side == PlayerSide::White {
            "You".into()
        } else {
            self.engine_config.opponent.label().into()
        };
        self.review_black_player = if !self.engine_enabled {
            "Black".into()
        } else if self.player_side == PlayerSide::Black {
            "You".into()
        } else {
            self.engine_config.opponent.label().into()
        };
        self.clear_analysis_result();
        self.resume_engine_after_ready = engine_should_move;
        self.engine_status = if engine_should_move {
            format!("Waiting for {}…", self.engine_config.opponent.label())
        } else {
            "Your move".into()
        };
        Self::set_page_title(false, "", "");
        #[cfg(target_arch = "wasm32")]
        start_new_stored_game("mine");
        self.save_game();
    }

    fn select(&mut self, square: Square) {
        if self.fics_active
            && (!self.fics_playing
                || self.fics_pending_move
                || self.board.side_to_move() != self.player_side.color())
        {
            return;
        }
        if self.review_index.is_some() && self.best_move_attempt.is_none() {
            return;
        }
        if self.promotion.is_some() || self.board.status() != BoardStatus::Ongoing {
            return;
        }
        if self.best_move_attempt.is_none()
            && self.engine_enabled
            && self.board.side_to_move() != self.player_side.color()
        {
            return;
        }
        if let Some(from) = self.selected {
            if self.legal_targets.contains(&square) {
                let promotion_needed = self.board.piece_on(from) == Some(Piece::Pawn)
                    && matches!(square.get_rank(), Rank::First | Rank::Eighth);
                if promotion_needed {
                    self.promotion = Some((from, square));
                } else if self.best_move_attempt.is_some() {
                    self.submit_best_move_attempt(ChessMove::new(from, square, None));
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

    fn keyboard_move_ui(&mut self, ctx: &egui::Context) {
        let blocked = ctx.wants_keyboard_input()
            || self.new_game_dialog_open || self.training_dialog_open || self.elo_calculator_open
            || self.pgn_dialog_open || self.strength_dialog_open || self.about_dialog_open
            || self.batch_pgn_dialog_open || self.batch_pgn_result_open || self.print_confirm_open
            || self.expanded_graph_open || self.fics_console_open || self.fics_challenge_open
            || self.fics_sign_in_open || self.fics_resign_dialog_open;
        let playable = !blocked && self.review_index.is_none() && self.best_move_attempt.is_none()
            && self.promotion.is_none() && self.game_result() == "*"
            && (!self.engine_enabled || self.board.side_to_move() == self.player_side.color())
            && (!self.fics_active || (self.fics_playing && !self.fics_pending_move
                && self.board.side_to_move() == self.player_side.color()));
        if !playable || self.typed_move_board.is_some_and(|board| board != self.board) {
            self.typed_move.clear();
            self.typed_move_error = false;
            self.typed_move_board = None;
        }
        if !playable { return; }
        let events = ctx.input(|input| if input.modifiers.command || input.modifiers.ctrl || input.modifiers.alt { vec![] } else { input.events.clone() });
        for event in events {
            match event {
                egui::Event::Text(text) => {
                    for ch in text.chars() {
                        if self.typed_move.len() >= 16 { break; }
                        if self.typed_move.is_empty() && !"abcdefghABCDEFGHNnRrQqKkOo0".contains(ch) { continue; }
                        if ch.is_ascii_alphanumeric() || "=+#-".contains(ch) {
                            self.typed_move.push(ch);
                            self.typed_move_board = Some(self.board);
                            self.typed_move_error = false;
                        }
                    }
                }
                egui::Event::Key { key: egui::Key::Backspace, pressed: true, .. } => {
                    self.typed_move.pop(); self.typed_move_error = false;
                }
                egui::Event::Key { key: egui::Key::Escape, pressed: true, .. } => {
                    self.typed_move.clear(); self.typed_move_error = false;
                }
                egui::Event::Key { key: egui::Key::Enter, pressed: true, .. } if !self.typed_move.is_empty() => {
                    if let Some(mv) = Self::parse_keyboard_move(&self.board, &self.typed_move) {
                        self.typed_move.clear(); self.typed_move_board = None;
                        self.play(mv);
                    } else { self.typed_move_error = true; }
                }
                _ => {}
            }
        }
        if self.typed_move.is_empty() { return; }
        egui::Area::new(egui::Id::new("typed_move_overlay"))
            .order(egui::Order::Foreground).anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                egui::Frame::new().fill(Color32::from_rgba_unmultiplied(23, 26, 30, 225))
                    .stroke(Stroke::new(1.0, Color32::from_rgb(211, 173, 98)))
                    .corner_radius(12.0).inner_margin(20.0).show(ui, |ui| {
                        ui.set_min_width(260.0);
                        ui.label(RichText::new("TYPE YOUR MOVE").size(14.0).color(Color32::from_rgb(211, 173, 98)));
                        ui.label(RichText::new(&self.typed_move).size(36.0).strong().color(Color32::WHITE));
                        if self.typed_move_error {
                            ui.label(RichText::new("Not a legal move. Edit and try again.").color(Color32::from_rgb(240, 150, 130)));
                        }
                        ui.label(RichText::new("Enter to play · Backspace to edit · Escape to cancel").size(14.0).color(Color32::LIGHT_GRAY));
                    });
            });
    }

    fn parse_keyboard_move(board: &Board, text: &str) -> Option<ChessMove> {
        let san = text.replace('0', "O").replace('o', "O");
        Self::parse_san_move(board, &san).or_else(|| {
            // Try the pawn/file spelling first so b4 remains a pawn move.
            let mut normalized = san.clone();
            if matches!(normalized.chars().next(), Some('n' | 'b' | 'r' | 'q' | 'k')) {
                normalized.replace_range(..1, &normalized[..1].to_ascii_uppercase());
            }
            if let Some(index) = normalized.find('=') {
                normalized[index + 1..].make_ascii_uppercase();
            }
            Self::parse_san_move(board, &normalized)
        }).or_else(|| {
            Self::parse_uci_value(text).filter(|mv| MoveGen::new_legal(board).any(|legal| legal == *mv))
        })
    }

    fn start_best_move_attempt(&mut self, target_index: usize) {
        if self.training_live() { return; }
        let Some(root_index) = target_index.checked_sub(1) else {
            return;
        };
        let Some(root) = self.review_positions.get(root_index).copied() else {
            return;
        };
        let Some(best_move) = self
            .game_analysis
            .get(root_index)
            .and_then(Option::as_ref)
            .and_then(|analysis| analysis.best_move.as_deref())
            .and_then(Self::parse_uci_value)
        else {
            return;
        };
        #[cfg(target_arch = "wasm32")]
        if let Some(engine) = &self.engine {
            engine.command("stop");
        }
        self.analysis_running = false;
        self.engine_searching = false;
        self.pending_analysis_search = None;
        self.clear_analysis_result();
        self.board = root;
        self.review_index = Some(root_index);
        self.review_scroll_to_selected = true;
        self.last_move =
            Self::review_move_at(&self.review_positions, &self.review_moves, root_index);
        self.selected = None;
        self.legal_targets.clear();
        self.promotion = None;
        self.best_move_attempt = Some(BestMoveAttempt {
            target_index,
            best_move,
            attempted_move: None,
            revealed: false,
        });
        self.engine_status = "Find the best move".into();
    }

    fn submit_best_move_attempt(&mut self, chess_move: ChessMove) {
        if !MoveGen::new_legal(&self.board).any(|candidate| candidate == chess_move) {
            return;
        }
        let Some(mut attempt) = self.best_move_attempt else {
            return;
        };
        self.board = self.board.make_move_new(chess_move);
        self.last_move = Some(chess_move);
        self.review_index = Some(attempt.target_index);
        self.review_scroll_to_selected = true;
        attempt.attempted_move = Some(chess_move);
        self.best_move_attempt = Some(attempt);
        self.selected = None;
        self.legal_targets.clear();
        self.engine_status = if chess_move == attempt.best_move {
            "Best move found".into()
        } else {
            "Try again or reveal Stockfish's move".into()
        };
    }

    fn reset_best_move_attempt(&mut self) {
        let Some(attempt) = self.best_move_attempt else {
            return;
        };
        self.start_best_move_attempt(attempt.target_index);
    }

    fn reveal_best_move_attempt(&mut self) {
        let Some(mut attempt) = self.best_move_attempt else {
            return;
        };
        let root_index = attempt.target_index.saturating_sub(1);
        let Some(root) = self.review_positions.get(root_index).copied() else {
            return;
        };
        attempt.revealed = true;
        self.best_move_attempt = Some(attempt);
        self.board = root;
        self.review_index = Some(root_index);
        self.last_move =
            Self::review_move_at(&self.review_positions, &self.review_moves, root_index);
        self.selected = None;
        self.legal_targets.clear();
        self.engine_status = "Best move revealed".into();
    }

    fn play(&mut self, mv: ChessMove) {
        if self.training.is_some() && self.game_result() != "*" { return; }
        if self.fics_active && !self.fics_applying_update {
            if self.fics_playing
                && !self.fics_pending_move
                && self.board.side_to_move() == self.player_side.color()
                && MoveGen::new_legal(&self.board).any(|candidate| candidate == mv)
            {
                #[cfg(target_arch = "wasm32")]
                fics_send(&mv.to_string());
                self.fics_pending_move = true;
                self.selected = None;
                self.legal_targets.clear();
                self.fics_status = "Waiting for FICS to confirm move…".into();
            }
            return;
        }
        self.tick_local_clock();
        if !self.fics_active && self.local_resigned_white.is_some() {
            return;
        }
        if !self.fics_active
            && self
                .local_clock
                .as_ref()
                .is_some_and(|clock| clock.flagged_white.is_some())
        {
            return;
        }
        if MoveGen::new_legal(&self.board).any(|candidate| candidate == mv) {
            #[cfg(target_arch = "wasm32")]
            let mut resume_engine_after_analysis = false;
            #[cfg(target_arch = "wasm32")]
            if self.analysis_running && !self.game_analysis_running {
                let search_in_flight = self.engine_searching;
                self.analysis_running = false;
                self.engine_searching = false;
                self.pending_analysis_search = None;
                self.ignore_next_bestmove = search_in_flight;
                if search_in_flight && let Some(engine) = &self.engine {
                    engine.command("stop");
                }
                resume_engine_after_analysis = true;
            }
            let san = Self::san_for_move(&self.board, mv);
            if self.review_positions.is_empty() {
                self.review_positions.push(self.board);
                if !self.fics_active {
                    self.review_white_player = if !self.engine_enabled {
                        "White".into()
                    } else if self.player_side == PlayerSide::White {
                        "You".into()
                    } else {
                        self.engine_config.opponent.label().into()
                    };
                    self.review_black_player = if !self.engine_enabled {
                        "Black".into()
                    } else if self.player_side == PlayerSide::Black {
                        "You".into()
                    } else {
                        self.engine_config.opponent.label().into()
                    };
                }
            }
            self.history.push(self.board);
            let previous = self.board;
            self.board = self.board.make_move_new(mv);
            if !self.fics_active
                && let Some(clock) = &mut self.local_clock
            {
                if previous.side_to_move() == Color::White {
                    clock.white += clock.increment;
                } else {
                    clock.black += clock.increment;
                }
                clock.updated_at = Self::animation_time();
            }
            self.live_move_effects(&previous, mv);
            self.review_moves.push(san);
            self.review_positions.push(self.board);
            self.clear_verification();
            if !self.game_analysis.is_empty() {
                self.game_analysis.resize(self.review_positions.len(), None);
            }
            self.review_scroll_to_selected = true;
            self.last_move = Some(mv);
            self.selected = None;
            self.legal_targets.clear();
            self.engine_status =
                if self.engine_enabled && self.board.side_to_move() != self.player_side.color() {
                    format!("Waiting for {}…", self.engine_config.opponent.label())
                } else {
                    "Your move".into()
                };
            if !self.fics_active {
                self.schedule_realtime_analysis();
            }
            if !self.fics_observing {
                self.save_game();
            }
            #[cfg(target_arch = "wasm32")]
            if resume_engine_after_analysis {
                self.queue_live_engine_resume();
            } else {
                self.request_engine_move();
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn request_engine_move(&mut self) {
        if self.training.is_some() && self.game_result() != "*" { return; }
        if let Some(session) = &self.training {
            self.engine_config.limit_strength = true;
            self.engine_config.elo = session.opponent_elo as u32;
        }
        if self.local_resigned_white.is_some() && !self.fics_active {
            return;
        }
        if self.fics_active
            || self
                .local_clock
                .as_ref()
                .is_some_and(|clock| clock.flagged_white.is_some())
        {
            return;
        }
        if self.engine_enabled
            && self.review_index.is_none()
            && self.board.side_to_move() != self.player_side.color()
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
                engine.command(&format!(
                    "setoption name UCI_Chess960 value {}",
                    self.board.is_chess960()
                ));
                engine.command(&format!("position fen {}", self.board));
                let search = if self.training_live() {
                    let remaining = self.local_clock.as_ref().map(|c| if self.board.side_to_move() == Color::White { c.white } else { c.black }).unwrap_or(1.0);
                    format!("go movetime {}", (remaining * 1000.0).clamp(1.0, 1000.0) as u64)
                } else if let Some(clock) = &self.local_clock {
                    format!(
                        "go wtime {} btime {} winc {} binc {}",
                        (clock.white * 1000.0) as u64,
                        (clock.black * 1000.0) as u64,
                        (clock.increment * 1000.0) as u64,
                        (clock.increment * 1000.0) as u64
                    )
                } else {
                    match self.engine_config.search_limit {
                        SearchLimit::Infinite => "go infinite".to_owned(),
                        SearchLimit::Time => {
                            format!("go movetime {}", self.engine_config.move_time_ms)
                        }
                        SearchLimit::Depth => format!("go depth {}", self.engine_config.depth),
                        SearchLimit::Nodes => format!("go nodes {}", self.engine_config.nodes),
                    }
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

    fn same_fics_position(a: &Board, b: &Board) -> bool {
        a.to_string()
            .split_whitespace()
            .take(3)
            .eq(b.to_string().split_whitespace().take(3))
    }

    fn fics_transition(
        start: &Board,
        target: &Board,
        last_move: &str,
        last_san: &str,
    ) -> Option<Vec<ChessMove>> {
        if Self::same_fics_position(start, target) {
            return Some(Vec::new());
        }
        let preferred =
            Self::parse_uci_value(last_move).or_else(|| Self::parse_san_move(start, last_san));
        if let Some(mv) = preferred
            && MoveGen::new_legal(start).any(|candidate| candidate == mv)
            && Self::same_fics_position(&start.make_move_new(mv), target)
        {
            return Some(vec![mv]);
        }
        for first in MoveGen::new_legal(start) {
            let after_first = start.make_move_new(first);
            if Self::same_fics_position(&after_first, target) {
                return Some(vec![first]);
            }
            let preferred_second = Self::parse_uci_value(last_move)
                .or_else(|| Self::parse_san_move(&after_first, last_san));
            if let Some(second) = preferred_second
                && MoveGen::new_legal(&after_first).any(|candidate| candidate == second)
                && Self::same_fics_position(&after_first.make_move_new(second), target)
            {
                return Some(vec![first, second]);
            }
            for second in MoveGen::new_legal(&after_first) {
                if Self::same_fics_position(&after_first.make_move_new(second), target) {
                    return Some(vec![first, second]);
                }
            }
        }
        None
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
                        self.engine_ready = true;
                        self.show_engine_download = false;
                        self.engine_status = "Stockfish 19 ready".into();
                        self.engine_progress = None;
                        if self.resume_game_analysis_after_ready && self.game_analysis_running {
                            self.resume_game_analysis_after_ready = false;
                            self.engine_status = "Resuming saved game analysis…".into();
                            self.start_game_analysis_position();
                        } else if self.resume_engine_after_ready {
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
                            self.engine_status = if !self.verification_targets.is_empty()
                                && self.game_analysis_running
                            {
                                format!(
                                    "Verifying critical position {} of {}",
                                    self.verification_completed_count() + 1,
                                    self.verification_targets.len()
                                )
                            } else if let Some(index) = self
                                .game_analysis_index
                                .filter(|_| self.game_analysis_running)
                            {
                                format!(
                                    "Analyzing game · position {} of {}",
                                    index + 1,
                                    self.review_positions.len()
                                )
                            } else {
                                "Real-time analysis…".into()
                            };
                        }
                    }
                    if self.analysis_running && self.engine_searching && line.starts_with("info ") {
                        self.update_analysis_from_info(&line);
                    }
                    if line.starts_with("bestmove ") {
                        if !self.analysis_running
                            && !self.fics_active
                            && self
                                .local_clock
                                .as_ref()
                                .is_some_and(|clock| clock.flagged_white.is_some())
                        {
                            self.engine_searching = false;
                            continue;
                        }
                        if self.ignore_next_bestmove {
                            self.ignore_next_bestmove = false;
                            continue;
                        }
                        let completed_analysis = self.analysis_running && self.engine_searching;
                        if completed_analysis {
                            self.pending_analysis_search = None;
                            if self.game_analysis_running {
                                let index = self.game_analysis_index.unwrap_or(0);
                                let terminal_board = self.review_positions.get(index).copied();
                                let terminal_mate = terminal_board
                                    .is_some_and(|board| board.status() == BoardStatus::Checkmate);
                                let terminal_draw = terminal_board
                                    .is_some_and(|board| board.status() == BoardStatus::Stalemate);
                                let result = PositionAnalysis {
                                    eval_cp: if terminal_mate {
                                        None
                                    } else {
                                        self.analysis_eval_cp.or(terminal_draw.then_some(0))
                                    },
                                    mate: if terminal_mate {
                                        Some(0)
                                    } else {
                                        self.analysis_mate
                                    },
                                    depth: self.analysis_depth,
                                    nodes: self.analysis_nodes,
                                    best_move: line
                                        .split_whitespace()
                                        .nth(1)
                                        .filter(|value| *value != "(none)")
                                        .map(str::to_owned),
                                    pv: self.analysis_pv.clone(),
                                };
                                if !self.verification_targets.is_empty() {
                                    if let Some(slot) = self.verification_results.get_mut(index) {
                                        *slot = Some(result);
                                    }
                                } else if let Some(slot) = self.game_analysis.get_mut(index) {
                                    *slot = Some(result);
                                }
                                self.save_game();
                                let next = if !self.verification_targets.is_empty() {
                                    self.verification_pending_index()
                                } else {
                                    self.game_analysis
                                        .iter()
                                        .enumerate()
                                        .skip(index + 1)
                                        .find(|(_, result)| result.is_none())
                                        .map(|(index, _)| index)
                                };
                                if let Some(next) = next {
                                    self.game_analysis_index = Some(next);
                                    self.engine_searching = false;
                                    self.start_game_analysis_position();
                                    continue;
                                }
                                self.game_analysis_running = false;
                                self.game_analysis_paused = false;
                                self.resume_game_analysis_after_ready = false;
                                self.game_analysis_index = None;
                                self.analysis_running = false;
                                self.clear_analysis_result();
                                if !self.verification_targets.is_empty() {
                                    for target in &self.verification_targets {
                                        if let Some(result) =
                                            self.verification_results[*target].clone()
                                        {
                                            self.game_analysis[*target] = Some(result);
                                        }
                                    }
                                    self.engine_status =
                                        "Critical-move verification complete".into();
                                } else {
                                    self.engine_status = "Game analysis complete".into();
                                    self.start_verification();
                                }
                                self.save_game();
                            } else {
                                self.analysis_running = false;
                                self.engine_status = "Real-time analysis complete".into();
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
                    if line.contains("Cross-origin isolation")
                        || line.contains("worker failed")
                        || line.starts_with("info string CRITICAL ERROR:")
                    {
                        self.engine_status = if let Some(index) = self.game_analysis_index {
                            format!(
                                "Analysis failed at position {} of {}: {line}. Completed results are saved.",
                                index + 1,
                                self.review_positions.len()
                            )
                        } else {
                            line
                        };
                        self.engine_progress = None;
                        self.pending_analysis_search = None;
                        self.analysis_running = false;
                        self.engine_searching = false;
                        self.game_analysis_running = false;
                        self.game_analysis_paused = false;
                        self.resume_game_analysis_after_ready = false;
                        self.game_analysis_index = None;
                        self.save_game();
                    }
                }
            }
        }
    }

    fn undo(&mut self) {
        if self.training.is_some() { return; }
        if self.local_resigned_white.is_some() {
            return;
        }
        if self.fics_active {
            return;
        }
        #[cfg(target_arch = "wasm32")]
        if let Some(engine) = &self.engine {
            engine.command("stop");
        }
        self.engine_searching = false;
        self.move_animation = None;
        if let Some(board) = self.history.pop() {
            self.board = board;
            self.review_moves.pop();
            self.review_positions.pop();
            self.move_notes.truncate(self.review_positions.len());
            self.move_annotations.truncate(self.review_positions.len());
            self.drawing_undo.clear();
            self.board_marks.truncate(self.review_positions.len());
            self.clear_verification();
            if !self.game_analysis.is_empty() {
                self.game_analysis.truncate(self.review_positions.len());
            }
            self.last_move = self.history.last().and_then(|previous| {
                self.review_moves
                    .last()
                    .and_then(|san| Self::parse_san_move(previous, san))
            });
            self.selected = None;
            self.legal_targets.clear();
            self.promotion = None;
            self.save_game();
        }
    }

    fn piece_glyph(&self, square: Square) -> &'static str {
        let Some(color) = self.board.color_on(square) else {
            return "";
        };
        let Some(piece) = self.board.piece_on(square) else {
            return "";
        };
        Self::piece_glyph_for(color, piece)
    }

    fn piece_glyph_for(color: Color, piece: Piece) -> &'static str {
        match (color, piece) {
            (Color::White, Piece::King) => "♔",
            (Color::White, Piece::Queen) => "♕",
            (Color::White, Piece::Rook) => "♖",
            (Color::White, Piece::Bishop) => "♗",
            (Color::White, Piece::Knight) => "♘",
            (Color::White, Piece::Pawn) => "♙",
            (Color::Black, Piece::King) => "♚",
            (Color::Black, Piece::Queen) => "♛",
            (Color::Black, Piece::Rook) => "♜",
            (Color::Black, Piece::Bishop) => "♝",
            (Color::Black, Piece::Knight) => "♞",
            (Color::Black, Piece::Pawn) => "♟",
        }
    }

    fn piece_image(&self, square: Square) -> Option<egui::ImageSource<'static>> {
        let color = self.board.color_on(square)?;
        let piece = self.board.piece_on(square)?;
        Self::piece_image_for(self.piece_set, color, piece)
    }

    fn piece_image_for(
        piece_set: PieceSet,
        color: Color,
        piece: Piece,
    ) -> Option<egui::ImageSource<'static>> {
        match (piece_set, color, piece) {
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
            (PieceSet::RoyalRascals, Color::White, Piece::King) => {
                Some(egui::include_image!("../web/pieces/royal-rascals/wK.svg"))
            }
            (PieceSet::RoyalRascals, Color::White, Piece::Queen) => {
                Some(egui::include_image!("../web/pieces/royal-rascals/wQ.svg"))
            }
            (PieceSet::RoyalRascals, Color::White, Piece::Rook) => {
                Some(egui::include_image!("../web/pieces/royal-rascals/wR.svg"))
            }
            (PieceSet::RoyalRascals, Color::White, Piece::Bishop) => {
                Some(egui::include_image!("../web/pieces/royal-rascals/wB.svg"))
            }
            (PieceSet::RoyalRascals, Color::White, Piece::Knight) => {
                Some(egui::include_image!("../web/pieces/royal-rascals/wN.svg"))
            }
            (PieceSet::RoyalRascals, Color::White, Piece::Pawn) => {
                Some(egui::include_image!("../web/pieces/royal-rascals/wP.svg"))
            }
            (PieceSet::RoyalRascals, Color::Black, Piece::King) => {
                Some(egui::include_image!("../web/pieces/royal-rascals/bK.svg"))
            }
            (PieceSet::RoyalRascals, Color::Black, Piece::Queen) => {
                Some(egui::include_image!("../web/pieces/royal-rascals/bQ.svg"))
            }
            (PieceSet::RoyalRascals, Color::Black, Piece::Rook) => {
                Some(egui::include_image!("../web/pieces/royal-rascals/bR.svg"))
            }
            (PieceSet::RoyalRascals, Color::Black, Piece::Bishop) => {
                Some(egui::include_image!("../web/pieces/royal-rascals/bB.svg"))
            }
            (PieceSet::RoyalRascals, Color::Black, Piece::Knight) => {
                Some(egui::include_image!("../web/pieces/royal-rascals/bN.svg"))
            }
            (PieceSet::RoyalRascals, Color::Black, Piece::Pawn) => {
                Some(egui::include_image!("../web/pieces/royal-rascals/bP.svg"))
            }
            (PieceSet::UndeadCourt, Color::White, Piece::King) => {
                Some(egui::include_image!("../web/pieces/undead-court/wK.svg"))
            }
            (PieceSet::UndeadCourt, Color::White, Piece::Queen) => {
                Some(egui::include_image!("../web/pieces/undead-court/wQ.svg"))
            }
            (PieceSet::UndeadCourt, Color::White, Piece::Rook) => {
                Some(egui::include_image!("../web/pieces/undead-court/wR.svg"))
            }
            (PieceSet::UndeadCourt, Color::White, Piece::Bishop) => {
                Some(egui::include_image!("../web/pieces/undead-court/wB.svg"))
            }
            (PieceSet::UndeadCourt, Color::White, Piece::Knight) => {
                Some(egui::include_image!("../web/pieces/undead-court/wN.svg"))
            }
            (PieceSet::UndeadCourt, Color::White, Piece::Pawn) => {
                Some(egui::include_image!("../web/pieces/undead-court/wP.svg"))
            }
            (PieceSet::UndeadCourt, Color::Black, Piece::King) => {
                Some(egui::include_image!("../web/pieces/undead-court/bK.svg"))
            }
            (PieceSet::UndeadCourt, Color::Black, Piece::Queen) => {
                Some(egui::include_image!("../web/pieces/undead-court/bQ.svg"))
            }
            (PieceSet::UndeadCourt, Color::Black, Piece::Rook) => {
                Some(egui::include_image!("../web/pieces/undead-court/bR.svg"))
            }
            (PieceSet::UndeadCourt, Color::Black, Piece::Bishop) => {
                Some(egui::include_image!("../web/pieces/undead-court/bB.svg"))
            }
            (PieceSet::UndeadCourt, Color::Black, Piece::Knight) => {
                Some(egui::include_image!("../web/pieces/undead-court/bN.svg"))
            }
            (PieceSet::UndeadCourt, Color::Black, Piece::Pawn) => {
                Some(egui::include_image!("../web/pieces/undead-court/bP.svg"))
            }
            _ => None,
        }
    }

    fn paint_piece_shadow(
        ui: &egui::Ui,
        rect: egui::Rect,
        cell: f32,
        piece_set: PieceSet,
        color: Color,
        piece: Piece,
    ) {
        let offset = Vec2::new(cell * 0.035, cell * 0.045);
        let softness = (cell * 0.015).max(0.6);
        for (jitter, alpha) in [
            (Vec2::ZERO, 45),
            (Vec2::new(-softness, 0.0), 18),
            (Vec2::new(softness, 0.0), 18),
            (Vec2::new(0.0, -softness), 18),
            (Vec2::new(0.0, softness), 18),
        ] {
            let shadow_rect = rect.translate(offset + jitter);
            let tint = Color32::from_black_alpha(alpha);
            if let Some(source) = Self::piece_image_for(piece_set, color, piece) {
                egui::Image::new(source)
                    .maintain_aspect_ratio(true)
                    .tint(tint)
                    .paint_at(ui, shadow_rect.shrink(cell * 0.06));
            } else {
                ui.painter().text(
                    shadow_rect.center(),
                    Align2::CENTER_CENTER,
                    Self::piece_glyph_for(color, piece),
                    FontId::proportional(cell * 0.74),
                    tint,
                );
            }
        }
    }

    fn paint_piece(
        ui: &egui::Ui,
        rect: egui::Rect,
        cell: f32,
        piece_set: PieceSet,
        color: Color,
        piece: Piece,
    ) {
        if let Some(source) = Self::piece_image_for(piece_set, color, piece) {
            egui::Image::new(source)
                .maintain_aspect_ratio(true)
                .paint_at(ui, rect.shrink(cell * 0.06));
            return;
        }
        let label = Self::piece_glyph_for(color, piece);
        let font = FontId::new(cell * 0.74, FontFamily::Proportional);
        let piece_color = match color {
            Color::White => Color32::from_rgb(250, 246, 224),
            Color::Black => Color32::from_rgb(24, 29, 32),
        };
        let (outline, cardinal, diagonal) = match color {
            Color::White => (Color32::from_rgb(20, 24, 27), 1.5, 1.1),
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
                rect.center() + offset,
                Align2::CENTER_CENTER,
                label,
                font.clone(),
                outline,
            );
        }
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            label,
            font,
            piece_color,
        );
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
        Self::paint_best_move_arrow(ui, source, destination, cell);
    }

    fn draw_best_move_arrow_3d(
        ui: &egui::Ui,
        board_rect: egui::Rect,
        chess_move: ChessMove,
        flipped: bool,
        view: crate::board3d::View,
    ) {
        let Some(source) =
            crate::board3d::square_center(chess_move.get_source(), board_rect, flipped, view)
        else {
            return;
        };
        let Some(destination) =
            crate::board3d::square_center(chess_move.get_dest(), board_rect, flipped, view)
        else {
            return;
        };
        Self::paint_best_move_arrow(ui, source, destination, board_rect.width() / 9.0);
    }

    fn paint_best_move_arrow(
        ui: &egui::Ui,
        source: egui::Pos2,
        destination: egui::Pos2,
        cell: f32,
    ) {
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

    #[cfg(target_arch = "wasm32")]
    fn board_3d_png(&self, board: &Board, ctx: &egui::Context) -> Result<Vec<u8>, wasm_bindgen::JsValue> {
        use image::ImageEncoder;

        let size = 1200.0;
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::splat(size));
        let view = self.board_3d_view();
        let image = crate::board3d::image_with_options(
            board,
            rect,
            self.flipped,
            view,
            None,
            &[],
            None,
            ctx.style().visuals.panel_fill,
            1.0,
            self.board_3d_appearance as f32 / 100.0,
            self.board_3d_theme,
            self.show_radial_light,
        );
        let mut rgba = Vec::with_capacity(image.pixels.len() * 4);
        for pixel in image.pixels {
            rgba.extend_from_slice(&pixel.to_array());
        }
        let mut png = Vec::new();
        image::codecs::png::PngEncoder::new(&mut png)
            .write_image(&rgba, image.size[0] as u32, image.size[1] as u32, image::ColorType::Rgba8.into())
            .map_err(|error| wasm_bindgen::JsValue::from_str(&error.to_string()))?;

        Ok(png)
    }

    fn board_3d_preview_rect(&self, rect: egui::Rect, view: crate::board3d::View) -> egui::Rect {
        let mut rendered_view = self.board_3d_view();
        rendered_view.distance = self.board_3d_rendered_distance;
        let a4 = Square::make_square(Rank::Fourth, File::A);
        let h4 = Square::make_square(Rank::Fourth, File::H);
        if let (Some(old_left), Some(old_right), Some(new_left), Some(new_right)) = (
            crate::board3d::square_center(a4, rect, self.flipped, rendered_view),
            crate::board3d::square_center(h4, rect, self.flipped, rendered_view),
            crate::board3d::square_center(a4, rect, self.flipped, view),
            crate::board3d::square_center(h4, rect, self.flipped, view),
        ) {
            let old_pivot = old_left + (old_right - old_left) * 0.5;
            let new_pivot = new_left + (new_right - new_left) * 0.5;
            let scale = (new_right - new_left).length()
                / (old_right - old_left).length();
            egui::Rect::from_min_max(
                new_pivot + (rect.min - old_pivot) * scale,
                new_pivot + (rect.max - old_pivot) * scale,
            )
        } else {
            rect
        }
    }

    fn toggle_board_dimension(&mut self, ctx: &egui::Context) {
        self.board_3d_active = !self.board_3d_active;
        self.save_preferences();
        self.move_animation = None;
        ctx.request_repaint();
    }

    fn board_3d_view(&self) -> crate::board3d::View {
        crate::board3d::View {
            yaw: self.board_3d_yaw,
            elevation: self.board_3d_elevation,
            distance: self.board_3d_distance,
        }
    }

    fn board_3d_view_rotated(&self) -> bool {
        let view = crate::board3d::View::default();
        self.board_3d_yaw != view.yaw || self.board_3d_elevation != view.elevation
    }

    fn reset_board_3d_view(&mut self, ctx: &egui::Context) {
        let view = crate::board3d::View::default();
        self.board_3d_yaw = view.yaw;
        self.board_3d_elevation = view.elevation;
        self.board_3d_distance = view.distance;
        self.board_3d_zoom_until = 0.0;
        ctx.request_repaint();
    }

    fn board_3d_ui(&mut self, ui: &mut egui::Ui, size: Vec2) -> egui::InnerResponse<()> {
        ui.allocate_ui_with_layout(size, Layout::top_down(Align::Min), |ui| {
            let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
            if self.board_mark_drag.is_none() && response.double_clicked_by(egui::PointerButton::Secondary) {
                self.reset_board_3d_view(ui.ctx());
            } else if self.board_mark_drag.is_none() && response.dragged_by(egui::PointerButton::Secondary) {
                let mut view = self.board_3d_view();
                view.orbit(ui.input(|input| input.pointer.delta()));
                self.board_3d_yaw = view.yaw;
                self.board_3d_elevation = view.elevation;
                // A scaled zoom preview cannot represent a changed camera angle.
                self.board_3d_zoom_until = 0.0;
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                ui.ctx().request_repaint();
            }
            if response.hovered() {
                let scroll = ui.input(|input| input.smooth_scroll_delta.y);
                if scroll != 0.0 {
                    let distance =
                        (self.board_3d_distance * (-scroll * 0.0003).exp()).clamp(14.0, 26.0);
                    if distance != self.board_3d_distance {
                        self.board_3d_distance = distance;
                        self.board_3d_zoom_until = Self::animation_time() + 0.15;
                    }
                }
            }
            let zooming = Self::animation_time() < self.board_3d_zoom_until;
            if zooming && self.board_3d_gpu_format.is_none() {
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(160));
            }
            let view = self.board_3d_view();
            let visible_last_move = self.last_move.filter(|_| self.show_highlighted_move);
            let background = ui.visuals().panel_fill;
            if let Some(target_format) = self.board_3d_gpu_format {
                let key = format!(
                    "{}:{}:{}:{}:{:?}:{:?}:{:?}:{:?}:{}:{:?}",
                    self.board, size.x.to_bits(), size.y.to_bits(),
                    self.flipped, self.selected, self.legal_targets, visible_last_move,
                    background, ui.ctx().pixels_per_point().to_bits(), self.board_3d_theme,
                );
                let key = format!("{key}:{}:{}:{}", view.yaw.to_bits(), view.elevation.to_bits(), self.show_radial_light);
                if self.board_3d_gpu_key != key || self.board_3d_gpu_scene.is_none() {
                    self.board_3d_gpu_scene_id = self.board_3d_gpu_scene_id.wrapping_add(1);
                    self.board_3d_gpu_scene = Some(crate::board3d::gpu::scene(
                        &self.board, rect, self.flipped, view, self.selected,
                        &self.legal_targets, visible_last_move, background,
                        ui.ctx().pixels_per_point(), target_format, self.board_3d_gpu_scene_id,
                        self.board_3d_theme, self.show_radial_light,
                    ));
                    self.board_3d_gpu_key = key;
                    self.board_3d_rendered_distance = view.distance;
                }
                if let Some(scene) = &self.board_3d_gpu_scene {
                    ui.painter().add(eframe::egui_wgpu::Callback::new_paint_callback(
                        rect, scene.clone().at_distance(
                            view.distance - self.board_3d_rendered_distance,
                            self.board_3d_appearance as f32 / 100.0,
                        ),
                    ));
                }
            } else {
                let (supersample, limit) = match self.board_3d_theme {
                    crate::board3d::Theme::Marble => (1.5, 2048.0),
                    crate::board3d::Theme::Wood => (1.65, 2304.0),
                    crate::board3d::Theme::Glass => (1.8, 2560.0),
                    crate::board3d::Theme::ArtDeco |
                    crate::board3d::Theme::Egyptian => (1.5, 2048.0),
                };
                let scale = (ui.ctx().pixels_per_point() * supersample).min(limit / size.x.max(size.y));
                let render_key = format!(
                    "{}:{}:{}:{}:{}:{:?}:{:?}:{:?}:{:?}:{}:{}:{:?}",
                    self.board,
                    size.x.to_bits(),
                    size.y.to_bits(),
                    view.distance.to_bits(),
                    self.flipped,
                    self.selected,
                    self.legal_targets,
                    visible_last_move,
                    background,
                    scale.to_bits(),
                    self.board_3d_appearance,
                    self.board_3d_theme,
                );
                let render_key = format!("{render_key}:{}:{}:{}", view.yaw.to_bits(), view.elevation.to_bits(), self.show_radial_light);
                if self.board_3d_render_key != render_key
                    && (!zooming || self.board_3d_texture.is_none())
                {
                    let image = crate::board3d::image_with_options(
                        &self.board,
                        rect,
                        self.flipped,
                        view,
                        self.selected,
                        &self.legal_targets,
                        visible_last_move,
                        background,
                        scale,
                        self.board_3d_appearance as f32 / 100.0,
                        self.board_3d_theme, self.show_radial_light,
                    );
                    if let Some(texture) = &mut self.board_3d_texture {
                        texture.set(image, egui::TextureOptions::LINEAR);
                    } else {
                        self.board_3d_texture = Some(ui.ctx().load_texture(
                            "board_3d_depth",
                            image,
                            egui::TextureOptions::LINEAR,
                        ));
                    }
                    self.board_3d_render_key = render_key;
                    self.board_3d_rendered_distance = view.distance;
                }
                if let Some(texture) = &self.board_3d_texture {
                    let preview_rect = if zooming { self.board_3d_preview_rect(rect, view) } else { rect };
                    ui.painter().with_clip_rect(rect).image(
                        texture.id(),
                        preview_rect,
                        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }
            }
            if self.show_coordinates && !self.board_3d_view_rotated() {
                let labels = crate::board3d::coordinate_labels(rect, self.flipped, view, self.board_3d_theme);
                let file_spacing = labels.get(0).zip(labels.get(1))
                    .map(|((_, a), (_, b))| a.distance(*b))
                    .unwrap_or(48.0);
                let font = FontId::proportional((file_spacing * 0.22).clamp(10.0, 18.0));
                let painter = ui.painter().with_clip_rect(rect.expand2(Vec2::new(14.0, 16.0)));
                for (label, position) in labels {
                    painter.text(
                        position + Vec2::new(1.0, 1.0),
                        Align2::CENTER_CENTER,
                        label,
                        font.clone(),
                        Color32::from_rgb(20, 22, 23),
                    );
                    painter.text(
                        position,
                        Align2::CENTER_CENTER,
                        label,
                        font.clone(),
                        Color32::from_rgb(220, 189, 128),
                    );
                }
            }
            if response.clicked() && !self.board_mark_mode && self.board_mark_drag.is_none() && !ui.input(|input| input.modifiers.alt)
                && let Some(pos) = response.interact_pointer_pos()
                && let Some(square) =
                    crate::board3d::square_at(pos, rect, self.flipped, view, &self.board, &self.legal_targets)
            {
                self.select(square);
            }
        })
    }

    fn piece_count(board: &Board, color: Color, piece: Piece) -> i32 {
        (*board.pieces(piece) & *board.color_combined(color)).popcnt() as i32
    }

    fn material_score(board: &Board, color: Color) -> i32 {
        [
            (Piece::Pawn, 1),
            (Piece::Knight, 3),
            (Piece::Bishop, 3),
            (Piece::Rook, 5),
            (Piece::Queen, 9),
        ]
        .into_iter()
        .map(|(piece, value)| Self::piece_count(board, color, piece) * value)
        .sum()
    }

    fn captured_pieces(positions: &[Board], victim: Color) -> Vec<Piece> {
        let mut captured = Vec::new();
        for pair in positions.windows(2) {
            let [before, after] = pair else { continue };
            if before.side_to_move() == victim {
                continue;
            }
            for piece in [
                Piece::Pawn,
                Piece::Knight,
                Piece::Bishop,
                Piece::Rook,
                Piece::Queen,
            ] {
                let removed = (Self::piece_count(before, victim, piece)
                    - Self::piece_count(after, victim, piece))
                .max(0);
                captured.extend(std::iter::repeat_n(piece, removed as usize));
            }
        }
        captured.sort_by_key(|piece| match piece {
            Piece::Pawn => 0,
            Piece::Knight => 1,
            Piece::Bishop => 2,
            Piece::Rook => 3,
            Piece::Queen => 4,
            Piece::King => 5,
        });
        captured
    }

    fn captured_material_ui(
        ui: &mut egui::Ui,
        width: f32,
        captured: &[Piece],
        victim: Color,
        piece_set: PieceSet,
        advantage: i32,
    ) {
        ui.allocate_ui_with_layout(
            Vec2::new(width, 20.0),
            Layout::left_to_right(Align::Center),
            |ui| {
                ui.spacing_mut().item_spacing.x = if captured.len() > 10 { 0.0 } else { 3.0 };
                if captured.is_empty() {
                    ui.label(RichText::new("No captures").size(11.0).weak());
                } else {
                    let icon_size = ((width - 44.0) / captured.len() as f32).clamp(10.0, 20.0);
                    for &piece in captured {
                        let (rect, response) =
                            ui.allocate_exact_size(Vec2::splat(icon_size), Sense::hover());
                        ui.painter().rect_filled(
                            rect,
                            3.0,
                            if victim == Color::Black {
                                Color32::from_rgb(207, 217, 207)
                            } else {
                                Color32::from_rgb(46, 60, 55)
                            },
                        );
                        Self::paint_piece(ui, rect, icon_size, piece_set, victim, piece);
                        response.on_hover_text(format!("Captured {victim:?} {piece:?}"));
                    }
                }
                if advantage > 0 {
                    ui.add_space(5.0);
                    ui.label(
                        RichText::new(format!("+{advantage}"))
                            .size(12.0)
                            .strong()
                            .color(Color32::from_rgb(211, 216, 209)),
                    );
                }
            },
        );
    }

    fn board_ui(&mut self, ui: &mut egui::Ui) {
        let available = ui.available_size();
        let show_players =
            !self.review_positions.is_empty() || (self.fics_active && self.fics_playing);
        let mut capture_positions = if self.review_positions.is_empty() {
            self.history.clone()
        } else {
            let index = self
                .review_index
                .unwrap_or(self.review_positions.len() - 1)
                .min(self.review_positions.len() - 1);
            self.review_positions[..=index].to_vec()
        };
        if capture_positions.last().copied() != Some(self.board) {
            capture_positions.push(self.board);
        }
        let captured_white = Self::captured_pieces(&capture_positions, Color::White);
        let captured_black = Self::captured_pieces(&capture_positions, Color::Black);
        let material_difference = Self::material_score(&self.board, Color::White)
            - Self::material_score(&self.board, Color::Black);
        let fics_clocks = (self.fics_active && (self.fics_playing || self.fics_observing))
            .then(|| self.fics_display_clocks())
            .or_else(|| {
                (!self.fics_active)
                    .then_some(self.local_clock.as_ref())
                    .flatten()
                    .map(|clock| (clock.white.ceil() as i32, clock.black.ceil() as i32))
            });
        let clock_turn = if self.fics_active {
            self.fics_turn
        } else {
            self.review_positions
                .last()
                .unwrap_or(&self.board)
                .side_to_move()
        };
        let game_result = self.game_result();
        let player_area_height = if show_players { 108.0 } else { 0.0 };
        let observe_tabs_height =
            if self.fics_active && !self.fics_playing && !self.fics_observed_games.is_empty() {
                36.0
            } else {
                0.0
            };
        let online_actions_height =
            if self.fics_active && (self.fics_playing || self.fics_observing) {
                44.0
            } else {
                0.0
            };
        let board_size = available
            .x
            .min(
                (available.y - player_area_height - observe_tabs_height - online_actions_height)
                    .max(180.0),
            )
            .max(180.0);
        let board_width = if self.board_3d_active {
            (board_size * 1.4).min(available.x)
        } else {
            board_size
        };
        let frame_width = if self.show_board_frame && !self.board_3d_active {
            (board_size * 0.035).clamp(14.0, 26.0) as i8
        } else {
            0
        };
        let cell = (board_size - 2.0 * frame_width as f32) / 8.0;
        let move_animation = self.move_animation.filter(|animation| {
            self.review_index.is_none()
                && self.prediction_index == 0
                && Self::animation_time() - animation.started_at < 0.24
        });
        if self.move_animation.is_some() && move_animation.is_none() {
            self.move_animation = None;
        }
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
            .best_move_attempt
            .filter(|attempt| attempt.revealed)
            .map(|attempt| attempt.best_move)
            .or_else(|| {
                self.best_move_attempt.is_none().then_some(())?;
                (self.show_best_move_arrows && !self.training_live())
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
                    .filter(|best_move| Some(*best_move) != self.last_move)
            });
        ui.allocate_ui_with_layout(
            Vec2::new(
                board_width,
                board_size + player_area_height + observe_tabs_height + online_actions_height,
            ),
            Layout::top_down(Align::Min),
            |ui| {
                if observe_tabs_height > 0.0 {
                    let mut selected = None;
                    let mut closed = None;
                    egui::ScrollArea::horizontal()
                        .id_salt("fics_observed_tabs")
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                for (index, game) in self.fics_observed_games.iter().enumerate() {
                                    let key = if index == 9 { 0 } else { index + 1 };
                                    ui.group(|ui| {
                                        ui.horizontal(|ui| {
                                            let label = format!("#{}  [{key}]", game.id);
                                            if ui
                                                .selectable_label(
                                                    self.fics_game_id == Some(game.id),
                                                    label,
                                                )
                                                .on_hover_text(format!(
                                                    "{} vs {} · Press {key} to switch",
                                                    game.white, game.black
                                                ))
                                                .clicked()
                                            {
                                                selected = Some(game.id);
                                            }
                                            if ui
                                                .small_button("×")
                                                .on_hover_text("Close observation")
                                                .clicked()
                                            {
                                                closed = Some(game.id);
                                            }
                                        });
                                    });
                                }
                            });
                        });
                    if let Some(game) = closed {
                        #[cfg(target_arch = "wasm32")]
                        fics_send_command(&format!("unobserve {game}"));
                        self.remove_observed_game(game);
                    } else if let Some(game) = selected {
                        self.show_observed_game(game);
                    }
                    ui.add_space(4.0);
                }
                if show_players {
                    let top_is_white = self.flipped;
                    let top_player = if self.flipped {
                        self.review_white_player.clone()
                    } else {
                        self.review_black_player.clone()
                    };
                    ui.allocate_ui_with_layout(
                        Vec2::new(board_width, 50.0),
                        Layout::top_down(Align::Min),
                        |ui| {
                            ui.allocate_ui_with_layout(
                                Vec2::new(board_width, 28.0),
                                Layout::left_to_right(Align::Center),
                                |ui| {
                                    ui.spacing_mut().item_spacing.x = 7.0;
                                    ui.label(
                                        RichText::new("●").color(Color32::from_rgb(76, 116, 92)),
                                    );
                                    self.player_name_ui(ui, &top_player, top_is_white);
                                    if let Some((white_time, black_time)) = fics_clocks {
                                        let (seconds, active) = if top_is_white {
                                            (white_time, clock_turn == Color::White)
                                        } else {
                                            (black_time, clock_turn == Color::Black)
                                        };
                                        Self::fics_board_clock_ui(ui, seconds, active);
                                    }
                                    Self::game_result_badge(ui, &game_result, top_is_white);
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        if Self::board_action_icon(ui, true, true)
                                            .on_hover_text("Flip board")
                                            .clicked()
                                        {
                                            self.flipped = !self.flipped;
                                            self.save_game();
                                        }
                                        if ui.add_sized(
                                            [29.0, 22.0],
                                            egui::Button::new(if self.board_3d_active { "2D" } else { "3D" }),
                                        )
                                        .on_hover_text(if self.board_3d_active { "Switch to 2D board" } else { "Switch to 3D board" })
                                        .clicked() {
                                            self.toggle_board_dimension(ui.ctx());
                                        }
                                        self.tactical_toggle_ui(ui);
                                        self.notes_toggle_ui(ui);
                                        self.draw_toggle_ui(ui);
                                        if self.board_3d_active && self.board_3d_view_rotated()
                                            && ui.add_sized([29.0, 22.0], egui::Button::new("↺"))
                                                .on_hover_text("Reset 3D view (or double right-click the board)")
                                                .clicked()
                                        {
                                            self.reset_board_3d_view(ui.ctx());
                                        }
                                    });
                                },
                            );
                            ui.add_space(2.0);
                            Self::captured_material_ui(
                                ui,
                                board_width,
                                if top_is_white { &captured_black } else { &captured_white },
                                if top_is_white { Color::Black } else { Color::White },
                                self.piece_set,
                                if top_is_white {
                                    material_difference.max(0)
                                } else {
                                    (-material_difference).max(0)
                                }
                            );
                        },
                    );
                    ui.add_space(4.0);
                }
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                let board_response = if self.board_3d_active {
                    self.board_3d_ui(ui, Vec2::new(board_width, board_size))
                } else {
                    egui::Frame::new()
                        .fill(Color32::from_rgb(31, 38, 33))
                        .inner_margin(egui::Margin::same(frame_width))
                        .show(ui, |ui| {
                            ui.allocate_ui_with_layout(
                                Vec2::splat(cell * 8.0),
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
                                                if self.show_highlighted_move && self.last_move.is_some_and(|m| {
                                                    m.get_source() == square
                                                        || m.get_dest() == square
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
                                                                let destination =
                                                                    self.last_move.is_some_and(
                                                                        |m| m.get_dest() == square,
                                                                    );
                                                                Self::blend_color(
                                                                    color,
                                                                    classification.color(),
                                                                    if destination {
                                                                        0.68
                                                                    } else {
                                                                        0.38
                                                                    },
                                                                )
                                                            }
                                                            None if light => {
                                                                Color32::from_rgb(222, 205, 111)
                                                            }
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
                                                let piece_color = match self.board.color_on(square)
                                                {
                                                    Some(Color::White) => {
                                                        Color32::from_rgb(250, 246, 224)
                                                    }
                                                    Some(Color::Black) => {
                                                        Color32::from_rgb(24, 29, 32)
                                                    }
                                                    None => Color32::from_rgba_unmultiplied(
                                                        30, 35, 33, 150,
                                                    ),
                                                };
                                                let font = FontId::new(
                                                    cell * 0.74,
                                                    FontFamily::Proportional,
                                                );
                                                let response = ui.add_sized(
                                                    Vec2::splat(cell),
                                                    egui::Button::new("")
                                                        .fill(color)
                                                        .stroke(Stroke::NONE)
                                                        .corner_radius(0.0),
                                                );
                                                if self.show_highlighted_move && self.prediction_index == 0
                                                    && self
                                                        .last_move
                                                        .is_some_and(|m| m.get_dest() == square)
                                                    && let Some(classification) =
                                                        highlighted_classification
                                                {
                                                    ui.painter().rect_stroke(
                                                        response.rect.shrink(2.0),
                                                        0.0,
                                                        Stroke::new(2.5, classification.color()),
                                                        egui::StrokeKind::Inside,
                                                    );
                                                }
                                                let center = response.rect.center();
                                                let animation_hides_piece = move_animation
                                                    .is_some_and(|animation| {
                                                        animation.chess_move.get_dest() == square
                                                    });
                                                if !animation_hides_piece {
                                                    if self.piece_shadows
                                                        && let (Some(color), Some(piece)) = (
                                                            self.board.color_on(square),
                                                            self.board.piece_on(square),
                                                        )
                                                    {
                                                        Self::paint_piece_shadow(
                                                            ui,
                                                            response.rect,
                                                            cell,
                                                            self.piece_set,
                                                            color,
                                                            piece,
                                                        );
                                                    }
                                                    if let Some(source) = piece_image {
                                                        egui::Image::new(source)
                                                            .maintain_aspect_ratio(true)
                                                            .paint_at(
                                                                ui,
                                                                response.rect.shrink(cell * 0.06),
                                                            );
                                                    } else if let Some(piece_side) =
                                                        self.board.color_on(square)
                                                    {
                                                        let (outline, cardinal, diagonal) =
                                                            match piece_side {
                                                                Color::White => (
                                                                    Color32::from_rgb(20, 24, 27),
                                                                    1.5,
                                                                    1.1,
                                                                ),
                                                                Color::Black => (
                                                                    Color32::from_rgba_unmultiplied(
                                                                        246, 244, 232, 210,
                                                                    ),
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
                                                }
                                                if self.show_coordinates && !self.show_board_frame {
                                                    let coordinate_color = if light {
                                                        Color32::from_rgb(64, 91, 72)
                                                    } else {
                                                        Color32::from_rgb(218, 225, 207)
                                                    };
                                                    let coordinate_font = FontId::proportional(
                                                        (cell * 0.14).clamp(10.0, 16.0),
                                                    );
                                                    if screen_file == 0 {
                                                        ui.painter().text(
                                                            response.rect.left_top()
                                                                + Vec2::new(4.0, 3.0),
                                                            Align2::LEFT_TOP,
                                                            (rank_index + 1).to_string(),
                                                            coordinate_font.clone(),
                                                            coordinate_color,
                                                        );
                                                    }
                                                    if screen_rank == 7 {
                                                        ui.painter().text(
                                                            response.rect.right_bottom()
                                                                - Vec2::new(4.0, 3.0),
                                                            Align2::RIGHT_BOTTOM,
                                                            ((b'a' + file_index as u8) as char)
                                                                .to_string(),
                                                            coordinate_font,
                                                            coordinate_color,
                                                        );
                                                    }
                                                }
                                                if response.clicked() && !self.board_mark_mode && self.board_mark_drag.is_none() && !ui.input(|input| input.modifiers.alt) {
                                                    self.select(square);
                                                }
                                            }
                                        });
                                    }
                                },
                            )
                        })
                        .inner
                };
                if !self.board_3d_active && self.show_board_frame && self.show_coordinates {
                    let rect = board_response.response.rect;
                    let inset = frame_width as f32 / 2.0;
                    let font = FontId::proportional((frame_width as f32 * 0.58).clamp(10.0, 15.0));
                    let gold = Color32::from_rgb(211, 173, 98);
                    for index in 0..8 {
                        let file = if self.flipped { 7 - index } else { index };
                        let rank = if self.flipped { index + 1 } else { 8 - index };
                        let x = rect.left() + (index as f32 + 0.5) * cell;
                        let y = rect.top() + (index as f32 + 0.5) * cell;
                        for edge_y in [rect.top() - inset, rect.bottom() + inset] {
                            ui.painter().text(
                                egui::pos2(x, edge_y),
                                Align2::CENTER_CENTER,
                                ((b'a' + file as u8) as char).to_string(),
                                font.clone(),
                                gold,
                            );
                        }
                        for edge_x in [rect.left() - inset, rect.right() + inset] {
                            ui.painter().text(
                                egui::pos2(edge_x, y),
                                Align2::CENTER_CENTER,
                                rank.to_string(),
                                font.clone(),
                                gold,
                            );
                        }
                    }
                }
                if let Some(animation) = move_animation.filter(|_| !self.board_3d_active) {
                    let board_rect = board_response.response.rect;
                    let square_center = |square: Square| {
                        let file = square.get_file().to_index() as f32;
                        let rank = square.get_rank().to_index() as f32;
                        let screen_file = if self.flipped { 7.0 - file } else { file };
                        let screen_rank = if self.flipped { rank } else { 7.0 - rank };
                        board_rect.left_top()
                            + Vec2::new((screen_file + 0.5) * cell, (screen_rank + 0.5) * cell)
                    };
                    let progress = ((Self::animation_time() - animation.started_at) / 0.24)
                        .clamp(0.0, 1.0) as f32;
                    let eased = 1.0 - (1.0 - progress).powi(3);
                    let source = square_center(animation.chess_move.get_source());
                    let destination = square_center(animation.chess_move.get_dest());
                    let center = source + (destination - source) * eased;
                    if self.piece_shadows {
                        Self::paint_piece_shadow(
                            ui,
                            egui::Rect::from_center_size(center, Vec2::splat(cell)),
                            cell,
                            self.piece_set,
                            animation.color,
                            animation.piece,
                        );
                    }
                    Self::paint_piece(
                        ui,
                        egui::Rect::from_center_size(center, Vec2::splat(cell)),
                        cell,
                        self.piece_set,
                        animation.color,
                        animation.piece,
                    );
                    ui.ctx().request_repaint();
                }
                self.board_mark_input(ui, board_response.response.rect);
                self.paint_tactical_map(ui, board_response.response.rect);
                self.paint_board_marks(ui, board_response.response.rect, self.review_index.unwrap_or(self.review_moves.len()));
                if let Some(best_move) = best_move_arrow {
                    if self.board_3d_active {
                        Self::draw_best_move_arrow_3d(
                            ui,
                            board_response.response.rect,
                            best_move,
                            self.flipped,
                            self.board_3d_view(),
                        );
                    } else {
                        Self::draw_best_move_arrow(
                            ui,
                            board_response.response.rect,
                            best_move,
                            self.flipped,
                        );
                    }
                }
                if show_players {
                    ui.add_space(4.0);
                    let bottom_is_white = !self.flipped;
                    let bottom_player = if self.flipped {
                        &self.review_black_player
                    } else {
                        &self.review_white_player
                    };
                    ui.allocate_ui_with_layout(
                        Vec2::new(board_width, 50.0),
                        Layout::top_down(Align::Min),
                        |ui| {
                            ui.allocate_ui_with_layout(
                                Vec2::new(board_width, 28.0),
                                Layout::left_to_right(Align::Center),
                                |ui| {
                                    ui.spacing_mut().item_spacing.x = 7.0;
                                    ui.label(
                                        RichText::new("●")
                                            .color(Color32::from_rgb(230, 178, 65)),
                                    );
                                    self.player_name_ui(ui, bottom_player, bottom_is_white);
                                    if let Some((white_time, black_time)) = fics_clocks {
                                        let (seconds, active) = if bottom_is_white {
                                            (white_time, clock_turn == Color::White)
                                        } else {
                                            (black_time, clock_turn == Color::Black)
                                        };
                                        Self::fics_board_clock_ui(ui, seconds, active);
                                    }
                                    Self::game_result_badge(ui, &game_result, bottom_is_white);
                                },
                            );
                            ui.add_space(2.0);
                            Self::captured_material_ui(
                                ui,
                                board_width,
                                if bottom_is_white { &captured_black } else { &captured_white },
                                if bottom_is_white { Color::Black } else { Color::White },
                                self.piece_set,
                                if bottom_is_white {
                                    material_difference.max(0)
                                } else {
                                    (-material_difference).max(0)
                                },
                            );
                        },
                    );
                }
                if self.fics_active && self.fics_playing {
                    ui.add_space(8.0);
                    ui.allocate_ui_with_layout(
                        Vec2::new(board_width, 36.0),
                        Layout::right_to_left(Align::Center),
                        |ui| {
                            ui.spacing_mut().item_spacing.x = 10.0;
                            let draw = ui.add(
                                egui::Button::new(RichText::new("     Offer draw").size(14.0))
                                    .min_size(Vec2::new(128.0, 36.0))
                                    .fill(Color32::from_rgb(44, 67, 58)),
                            );
                            let icon = draw.rect.left_center() + Vec2::new(17.0, 0.0);
                            let draw_color = Color32::from_rgb(210, 229, 216);
                            ui.painter()
                                .circle_stroke(icon, 7.0, Stroke::new(1.4, draw_color));
                            for offset in [-2.5, 2.5] {
                                ui.painter().line_segment(
                                    [
                                        icon + Vec2::new(-3.5, offset),
                                        icon + Vec2::new(3.5, offset),
                                    ],
                                    Stroke::new(1.4, draw_color),
                                );
                            }
                            if draw.clicked() {
                                #[cfg(target_arch = "wasm32")]
                                fics_send("draw");
                                self.fics_status = "Draw offer sent to FICS".into();
                            }
                        },
                    );
                } else if self.fics_active && self.fics_observing {
                    ui.add_space(8.0);
                    ui.allocate_ui_with_layout(
                        Vec2::new(board_width, 36.0),
                        Layout::right_to_left(Align::Center),
                        |ui| {
                            if ui
                                .add_sized([132.0, 36.0], egui::Button::new("Stop observing"))
                                .clicked()
                            {
                                #[cfg(target_arch = "wasm32")]
                                if let Some(game) = self.fics_game_id {
                                    #[cfg(target_arch = "wasm32")]
                                    fics_send_command(&format!("unobserve {game}"));
                                    self.remove_observed_game(game);
                                }
                            }
                        },
                    );
                }
            },
        );
    }

    fn engine_analysis_dock_ui(&mut self, ui: &mut egui::Ui) {
        if self.training_live() {
            ui.label(RichText::new("Training vs AI").size(19.0).strong().color(Color32::from_rgb(238, 238, 238)));
            ui.label(RichText::new("Play without hints or takebacks. Analyze the game after it finishes.").color(Color32::LIGHT_GRAY));
            return;
        }
        ui.horizontal(|ui| {
            let marker = if self.engine_analysis_dock_collapsed {
                "▶"
            } else {
                "▼"
            };
            if ui
                .button(format!("{marker}  Real-Time Analysis"))
                .on_hover_text("Collapse or expand live position analysis")
                .clicked()
            {
                self.engine_analysis_dock_collapsed = !self.engine_analysis_dock_collapsed;
            }

            ui.separator();
            if self.game_analysis_running || self.game_analysis_paused {
                let state = if !self.verification_targets.is_empty() {
                    if self.game_analysis_paused {
                        "Critical verification paused"
                    } else {
                        "Critical verification running"
                    }
                } else if self.game_analysis_paused {
                    "Full-game analysis paused"
                } else {
                    "Full-game analysis running"
                };
                ui.label(
                    RichText::new("Live analysis on hold").color(Color32::from_rgb(211, 173, 98)),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(state).weak());
                });
            } else {
                ui.label(
                    RichText::new(self.evaluation_text())
                        .size(20.0)
                        .strong()
                        .color(Color32::from_rgb(211, 173, 98)),
                );
                ui.label(RichText::new("White perspective").weak());

                if self.realtime_analysis_due_at.is_some() {
                    ui.separator();
                    ui.label(
                        RichText::new("Waiting for position to settle…")
                            .color(Color32::from_rgb(211, 173, 98)),
                    );
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(format!("{} nps", self.analysis_nps));
                    ui.separator();
                    ui.label(format!("{} nodes", self.analysis_nodes));
                    ui.separator();
                    ui.label(format!("Depth {}", self.analysis_depth));
                });
            }
        });

        if self.engine_analysis_dock_collapsed {
            return;
        }

        ui.separator();
        if self.game_analysis_running || self.game_analysis_paused {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(
                    RichText::new(
                        "Live position analysis is on hold while the game analysis pass is active.",
                    )
                    .size(14.0),
                );
            });
            return;
        }

        ui.horizontal(|ui| {
            self.evaluation_bar(ui);
            ui.vertical(|ui| {
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
                ui.label(RichText::new("Local Stockfish · private").small().weak());
            });

            if !self.prediction_moves.is_empty() {
                ui.separator();
                ui.vertical(|ui| {
                    ui.set_min_width(420.0_f32.min(ui.available_width()));
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Engine Move Prediction").size(16.0).strong());
                        if self.analysis_variations.len() > 1 {
                            let labels = self
                                .analysis_variations
                                .iter()
                                .enumerate()
                                .map(|(rank, variation)| variation.label(rank))
                                .collect::<Vec<_>>();
                            let mut selected = self.selected_variation.min(labels.len() - 1);
                            egui::ComboBox::from_id_salt("engine_prediction_variation_dock")
                                .selected_text(
                                    RichText::new(&labels[selected])
                                        .color(Color32::from_rgb(230, 178, 65)),
                                )
                                .show_ui(ui, |ui| {
                                    let baseline =
                                        self.analysis_variations[selected].comparison_value();
                                    for (variation_index, label) in labels.iter().enumerate() {
                                        let color = match (
                                            self.analysis_variations[variation_index]
                                                .comparison_value(),
                                            baseline,
                                        ) {
                                            (Some(value), Some(base)) if value > base => {
                                                Color32::from_rgb(106, 201, 126)
                                            }
                                            (Some(value), Some(base)) if value < base => {
                                                Color32::from_rgb(232, 112, 112)
                                            }
                                            _ => Color32::from_rgb(230, 178, 65),
                                        };
                                        ui.selectable_value(
                                            &mut selected,
                                            variation_index,
                                            RichText::new(label).color(color),
                                        );
                                    }
                                });
                            if selected != self.selected_variation {
                                self.prediction_index = 0;
                                self.prediction_navigation_active = true;
                                self.select_analysis_variation(selected);
                            }
                        }
                    });

                    let mut prediction_jump = None;
                    egui::ScrollArea::horizontal()
                        .id_salt("engine_prediction_moves_dock")
                        .show_gold(ui, |ui| {
                            ui.horizontal(|ui| {
                                for (move_index, san) in self.prediction_moves.iter().enumerate() {
                                    let selected = self.prediction_index == move_index + 1;
                                    let display_san = self.display_san(san);
                                    let mut response = ui.add_sized(
                                        [62.0, 28.0],
                                        egui::Button::selectable(selected, display_san),
                                    );
                                    if self.show_move_hover_text
                                        && let (Some(board), Some(chess_move)) = (
                                            self.prediction_positions.get(move_index),
                                            self.prediction_chess_moves.get(move_index),
                                        )
                                    {
                                        response = response.on_hover_text(Self::move_hover_text(
                                            board,
                                            *chess_move,
                                        ));
                                    }
                                    if response.clicked() {
                                        prediction_jump = Some(move_index + 1);
                                    }
                                    if selected && self.prediction_scroll_to_selected {
                                        response.scroll_to_me(Some(Align::Center));
                                    }
                                }
                            });
                        });
                    ui.horizontal(|ui| {
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
                                (self.prediction_index + 1).min(self.prediction_moves.len()),
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
                                .add_enabled(enabled, egui::Button::new(label))
                                .on_hover_text(hint)
                                .clicked()
                            {
                                prediction_jump = Some(destination);
                            }
                        }
                    });
                    self.prediction_scroll_to_selected = false;
                    if let Some(destination) = prediction_jump {
                        self.prediction_to(destination);
                    }
                });
            }
        });
    }

    fn fics_connection_menu(&mut self, ui: &mut egui::Ui) {
        ui.label(&self.fics_status);
        ui.separator();
        if self.fics_connected {
            if ui.button("Disconnect").clicked() {
                self.stop_fics();
                ui.close();
            }
        } else if self.fics_status.starts_with("Connecting") {
            if ui.button("Cancel connection").clicked() {
                self.stop_fics();
                ui.close();
            }
        } else if ui
            .button(if self.fics_active {
                "Reconnect as guest"
            } else {
                "Connect as guest"
            })
            .clicked()
        {
            self.start_fics();
            ui.close();
        }
        if ui.button("Open console").clicked() {
            self.fics_console_open = true;
            ui.close();
        }
    }

    fn fics_chat_tabs(&mut self, ui: &mut egui::Ui) -> bool {
        let mut selected = self.fics_chats.active.clone();
        let mut closed = None;
        egui::ScrollArea::horizontal().id_salt("fics_chat_tabs").show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.selectable_label(selected.is_none(), "Console").clicked() { selected = None; }
                for tab in &self.fics_chats.tabs {
                    let label = if tab.unread > 0 { format!("{} ({})", tab.title(), tab.unread) } else { tab.title() };
                    if ui.selectable_label(selected.as_deref() == Some(&tab.target), label).clicked() {
                        selected = Some(tab.target.clone());
                    }
                    if ui.small_button("×").on_hover_text("Close this chat tab. Channel subscriptions are unchanged.").clicked() {
                        closed = Some(tab.target.clone());
                    }
                }
                Self::gold_menu_button(ui, "+ New chat", |ui| {
                    ui.label("Player name or channel number (0–255)");
                    ui.text_edit_singleline(&mut self.fics_chats.new_target);
                    if ui.add_enabled(self.fics_connected, egui::Button::new("Open chat / join channel")).clicked() {
                        let recipient = crate::fics_chat::target(&self.fics_chats.new_target);
                        if let Some(recipient) = recipient {
                            if let Some(index) = self.fics_chats.ensure(&recipient) {
                                selected = Some(self.fics_chats.tabs[index].target.clone());
                                if recipient.bytes().all(|b| b.is_ascii_digit()) {
                                    let command = format!("+channel {recipient}");
                                    #[cfg(target_arch = "wasm32")]
                                    fics_send_command(&command);
                                    self.fics_log.push(format!("> {command}"));
                                }
                                self.fics_chats.new_target.clear();
                                self.fics_chats.error.clear();
                                ui.close();
                            } else {
                                self.fics_chats.error = "Close a chat tab before opening another (maximum 32).".into();
                            }
                        } else {
                            self.fics_chats.error = "Enter a 3–17 letter player name or a channel number from 0 to 255.".into();
                        }
                    }
                    if !self.fics_chats.error.is_empty() { ui.colored_label(Color32::LIGHT_RED, &self.fics_chats.error); }
                });
            });
        });
        if selected != self.fics_chats.active {
            self.fics_console_selected_text.clear();
            self.fics_console_suggestions_open = false;
            self.fics_chats.error.clear();
        }
        self.fics_chats.active = selected;
        if let Some(recipient) = closed {
            self.fics_chats.close(&recipient);
        }
        let Some(recipient) = self.fics_chats.active.clone() else {
            return false;
        };
        let Some(index) = self
            .fics_chats
            .tabs
            .iter()
            .position(|tab| tab.target == recipient)
        else {
            return false;
        };
        let tab = &mut self.fics_chats.tabs[index];
        tab.unread = 0;
        ui.separator();
        let messages_height = (ui.available_height() - 80.0).max(60.0);
        egui::ScrollArea::vertical()
            .id_salt(("fics_chat_messages", &recipient))
            .stick_to_bottom(true)
            .max_height(messages_height)
            .min_scrolled_height(messages_height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if tab.messages.is_empty() {
                    ui.weak("No messages yet.");
                }
                for message in &tab.messages {
                    ui.add(
                        egui::Label::new(RichText::new(message).font(FontId::monospace(13.0)))
                            .wrap()
                            .selectable(true),
                    );
                }
            });
        ui.separator();
        let mut send = false;
        let message_hint = format!("Message to {}", tab.title());
        ui.horizontal(|ui| {
            let response = ui.add_enabled(
                self.fics_connected,
                egui::TextEdit::singleline(&mut tab.draft)
                    .id_salt(("fics_chat_draft", &recipient))
                    .font(FontId::monospace(13.0))
                    .hint_text(message_hint)
                    .desired_width((ui.available_width() - 65.0).max(80.0)),
            );
            send = self.fics_connected
                && ((response.has_focus() || response.lost_focus())
                    && ui.input(|input| input.key_pressed(egui::Key::Enter)));
            send |= ui
                .add_enabled(self.fics_connected, egui::Button::new("Send"))
                .clicked();
            if send {
                response.request_focus();
            }
        });
        if send {
            let message = self.fics_chats.tabs[index].draft.clone();
            if let Some(command) = crate::fics_chat::command(&recipient, &message) {
                #[cfg(target_arch = "wasm32")]
                fics_send_command(&command);
                self.fics_log.push(format!("> {command}"));
                if self.fics_log.len() > 80 {
                    self.fics_log.remove(0);
                }
                self.fics_chats.submitted(&recipient, &message);
                self.fics_chats.tabs[index].draft.clear();
                self.fics_chats.error.clear();
            } else {
                self.fics_chats.error = "Use a nonempty message with standard English characters; recipient and message must fit within 256 characters.".into();
            }
        }
        if !self.fics_chats.error.is_empty() {
            ui.colored_label(Color32::LIGHT_RED, &self.fics_chats.error);
        }
        if !self.fics_registered {
            ui.weak("FICS may restrict guest chat. Check Console for server replies.");
        }
        true
    }

    fn fics_online_menu(&mut self, ui: &mut egui::Ui) {
        Self::set_menu_item_font(ui);
        ui.menu_button("Connection", |ui| self.fics_connection_menu(ui));
        ui.add_enabled_ui(!self.fics_playing, |ui| {
            ui.menu_button("FICS account", |ui| {
                ui.hyperlink_to("Create account", "https://www.freechess.org/Register/");
                if !self.fics_registered {
                    if ui.button("Sign in…").clicked() {
                        self.fics_sign_in_open = true;
                        ui.close();
                    }
                }
            });
        });
        ui.separator();
        ui.add_enabled_ui(self.fics_connected && !self.fics_playing, |ui| {
            egui::containers::menu::SubMenuButton::new("Available games")
                .config(
                    egui::containers::menu::MenuConfig::new()
                        .close_behavior(egui::containers::PopupCloseBehavior::CloseOnClickOutside),
                )
                .ui(ui, |ui| {
                    Self::set_menu_item_font(ui);
                    let screen = ui.ctx().screen_rect();
                    let list_width = (screen.width() - 40.0).clamp(180.0, 600.0);
                    let list_height = (screen.height() - 180.0).clamp(100.0, 480.0);
                    ui.set_width(list_width);
                    ui.spacing_mut().button_padding.y = 5.0;
                    self.fics_available_open_this_frame = true;
                    let refresh_clicked = ui.button("↻ Refresh available games").clicked();
                    if !self.fics_available_was_open || refresh_clicked {
                        self.fics_ads.clear();
                        #[cfg(target_arch = "wasm32")]
                        fics_send("sought");
                    }
                    ui.label(format!("{} available games", self.fics_ads.len()));
                    if self.fics_ads.is_empty() {
                        ui.label("Waiting for available games…");
                    }
                    egui::ScrollArea::vertical()
                        .id_salt("join_games_menu")
                        .max_height(list_height)
                        .min_scrolled_height(list_height)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            for ad in &self.fics_ads {
                                if ui
                                    .add(
                                        egui::Button::new(format!(
                                            "{} · {}+{} · {} · {} · {}",
                                            ad.player,
                                            ad.minutes,
                                            ad.increment,
                                            ad.category,
                                            ad.rated,
                                            ad.rating
                                        ))
                                        .min_size(egui::vec2(ui.available_width(), 30.0)),
                                    )
                                    .clicked()
                                {
                                    #[cfg(target_arch = "wasm32")]
                                    fics_send(&format!("play {}", ad.id));
                                    ui.close();
                                }
                            }
                        });
                });
            egui::containers::menu::SubMenuButton::new("Observe a game")
                .config(
                    egui::containers::menu::MenuConfig::new()
                        .close_behavior(egui::containers::PopupCloseBehavior::CloseOnClickOutside),
                )
                .ui(ui, |ui| {
                    Self::set_menu_item_font(ui);
                    // Submenus start with a small default area. Give the live list
                    // its own viewport so it cannot collapse to a few wrapped rows.
                    let screen = ui.ctx().screen_rect();
                    let list_width = (screen.width() - 40.0).clamp(180.0, 600.0);
                    let list_height = (screen.height() - 180.0).clamp(100.0, 480.0);
                    ui.set_width(list_width);
                    ui.spacing_mut().button_padding.y = 5.0;
                    self.fics_observe_open_this_frame = true;
                    let refresh_clicked = ui.button("↻ Refresh running games").clicked();
                    if !self.fics_observe_was_open || refresh_clicked {
                        self.fics_running_games.clear();
                        #[cfg(target_arch = "wasm32")]
                        fics_send_command("games /blsu");
                    }
                    ui.label(format!(
                        "{} public games · {} / 10 observed",
                        self.fics_running_games.len(),
                        self.fics_observed_games.len()
                    ));
                    if self.fics_running_games.is_empty() {
                        ui.label("Waiting for public games…");
                    }
                    let mut selected = None;
                    egui::ScrollArea::vertical()
                        .id_salt("observe_games_menu")
                        .max_height(list_height)
                        .min_scrolled_height(list_height)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            for game in &self.fics_running_games {
                                let existing =
                                    self.fics_observed_games.iter().any(|g| g.id == game.id);
                                if ui
                                    .add_enabled(
                                        existing || self.fics_observed_games.len() < 10,
                                        egui::Button::new(&game.label)
                                            .min_size(egui::vec2(ui.available_width(), 30.0)),
                                    )
                                    .clicked()
                                {
                                    selected = Some((game.id, existing));
                                    ui.close();
                                }
                            }
                        });
                    if let Some((id, existing)) = selected {
                        if existing {
                            self.show_observed_game(id);
                        } else {
                            #[cfg(target_arch = "wasm32")]
                            fics_send_command(&format!("observe {id}"));
                        }
                    }
                });
            ui.menu_button("Find an opponent", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Minutes");
                    ui.add(egui::DragValue::new(&mut self.fics_minutes).range(1..=60));
                    ui.label("Increment");
                    ui.add(egui::DragValue::new(&mut self.fics_increment).range(0..=60));
                });
                if self.fics_seeking {
                    if ui.button("Cancel search").clicked() {
                        #[cfg(target_arch = "wasm32")]
                        fics_send("unseek");
                        self.fics_seeking = false;
                    }
                } else if ui.button("Find unrated opponent").clicked() {
                    #[cfg(target_arch = "wasm32")]
                    fics_send(&format!(
                        "seek {} {} unrated",
                        self.fics_minutes, self.fics_increment
                    ));
                    self.fics_seeking = true;
                    ui.close();
                }
            });
            if ui.button("Challenge a specific player…").clicked() {
                self.fics_challenge_open = true;
                self.fics_challenge_error.clear();
                ui.close();
            }
            ui.menu_button("Challenge requests", |ui| {
                ui.horizontal(|ui| {
                    for (label, command) in [("Accept", "accept"), ("Decline", "decline")] {
                        #[cfg(not(target_arch = "wasm32"))]
                        let _ = command;
                        if ui.button(label).clicked() {
                            #[cfg(target_arch = "wasm32")]
                            fics_send(command);
                        }
                    }
                });
            });
        });
        ui.add_enabled_ui(self.fics_connected, |ui| {
            if self.fics_observing && ui.button("Stop observing current game").clicked() {
                if let Some(id) = self.fics_game_id {
                    #[cfg(target_arch = "wasm32")]
                    fics_send_command(&format!("unobserve {id}"));
                    self.remove_observed_game(id);
                }
                ui.close();
            }
        });
        if self.fics_connected && self.fics_playing {
            ui.menu_button("Game actions", |ui| {
                for (label, command) in [
                    ("Offer draw", "draw"),
                    ("Accept offer", "accept"),
                    ("Decline offer", "decline"),
                ] {
                    #[cfg(not(target_arch = "wasm32"))]
                    let _ = command;
                    if ui.button(label).clicked() {
                        #[cfg(target_arch = "wasm32")]
                        fics_send(command);
                        ui.close();
                    }
                }
                if ui.button("Resign…").clicked() {
                    self.fics_resign_dialog_open = true;
                    ui.close();
                }
            });
        }
        let unread: usize = self.fics_chats.tabs.iter().map(|tab| tab.unread).sum();
        let console_label = if unread > 0 {
            format!("FICS console & chats ({unread})")
        } else {
            "FICS console & chats".into()
        };
        if ui.button(console_label).clicked() {
            self.fics_console_open = true;
            ui.close();
        }
    }

    fn position_fen_for_copy(&self, index: usize) -> String {
        let board = self.review_positions.get(index).unwrap_or(&self.board);
        let fen = board.to_string();
        let mut fields: Vec<String> = fen.split_whitespace().map(str::to_owned).collect();
        if fields.len() == 6 {
            let start = self
                .fics_observation_start_ply
                .unwrap_or(self.local_start_ply);
            fields[5] = ((start + index) / 2 + 1).to_string();
            let mut halfmove = self
                .review_positions
                .first()
                .and_then(|b| {
                    b.to_string()
                        .split_whitespace()
                        .nth(4)?
                        .parse::<usize>()
                        .ok()
                })
                .unwrap_or(0);
            for san in self.review_moves.iter().take(index) {
                halfmove = if san.contains('x')
                    || san.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
                {
                    0
                } else {
                    halfmove + 1
                };
            }
            fields[4] = halfmove.to_string();
        }
        fields.join(" ")
    }

    fn clipboard_pgn(&self, count: usize) -> String {
        let count = count.min(self.review_moves.len());
        let result = if count == self.review_moves.len() {
            self.game_result()
        } else {
            "*".into()
        };
        let mut output = String::new();
        for (tag, value) in [
            ("Event", "Ironwood Chess game".to_owned()),
            ("White", self.review_white_player.clone()),
            ("Black", self.review_black_player.clone()),
            ("Result", result.clone()),
        ] {
            output.push_str(&format!("[{tag} \"{}\"]\n", value.replace('"', "'")));
        }
        if self
            .review_positions
            .first()
            .is_some_and(Board::is_chess960)
        {
            output.push_str("[Variant \"Chess960\"]\n");
        }
        if self
            .review_positions
            .first()
            .is_some_and(|b| *b != Board::default())
        {
            output.push_str(&format!(
                "[SetUp \"1\"]\n[FEN \"{}\"]\n",
                self.position_fen_for_copy(0)
            ));
        }
        output.push('\n');
        let start = self
            .fics_observation_start_ply
            .unwrap_or(self.local_start_ply);
        for (index, san) in self.review_moves.iter().take(count).enumerate() {
            let ply = start + index;
            if ply % 2 == 0 {
                output.push_str(&format!("{}. ", ply / 2 + 1));
            } else if index == 0 {
                output.push_str(&format!("{}... ", ply / 2 + 1));
            }
            output.push_str(san);
            output.push(' ');
        }
        output.push_str(&result);
        output
    }

    fn move_context_response(&mut self, response: &egui::Response, index: usize) {
        if response.double_clicked() { self.edit_note(index); }
        let selected = self.review_index.unwrap_or(self.review_moves.len()) == index;
        let keyboard_open = selected
            && !self.new_game_dialog_open
            && !response.ctx.wants_keyboard_input()
            && response
                .ctx
                .input_mut(|input| input.consume_key(egui::Modifiers::SHIFT, egui::Key::F10));
        let mut popup = egui::Popup::context_menu(response)
            .style(Self::gold_menu_style)
            .frame(Self::context_menu_frame(&response.ctx))
            .at_position(response.rect.right_bottom());
        if keyboard_open {
            popup = popup.open_memory(egui::SetOpenCommand::Bool(true));
        }
        popup.show(|ui| self.game_moves_context_menu(ui, index));
    }

    fn annotated_copy_marks(&self, index: usize) -> Vec<BoardMark> {
        let mut marks = self.board_marks.get(index).cloned().unwrap_or_default();
        let engine_move = if let Some(attempt) = self.best_move_attempt {
            (attempt.revealed && attempt.target_index.checked_sub(1) == Some(index)).then_some(attempt.best_move)
        } else if self.show_best_move_arrows && matches!(self.move_classification(index),
            Some(MoveClassification::Inaccuracy | MoveClassification::Mistake | MoveClassification::Blunder)) {
            index.checked_sub(1).and_then(|previous| self.game_analysis.get(previous))
                .and_then(Option::as_ref).and_then(|analysis| analysis.best_move.as_deref())
                .and_then(Self::parse_uci_value)
                .filter(|mv| Some(*mv) != Self::review_move_at(&self.review_positions, &self.review_moves, index))
        } else { None };
        if let Some(mv) = engine_move {
            let mark = BoardMark { style: String::new(), color: 'G', from: mv.get_source().to_string(), to: mv.get_dest().to_string() };
            if !marks.contains(&mark) { marks.push(mark); }
        }
        marks.retain(BoardMark::valid);
        marks
    }

    fn game_moves_context_menu(&mut self, ui: &mut egui::Ui, index: usize) {
        Self::style_context_menu(ui);
        ui.menu_button("Add annotation…", |ui| self.annotation_menu(ui, index));
        if ui
            .button(if self.note_at(index).is_empty() {
                "Add note…"
            } else {
                "Edit note…"
            })
            .clicked()
        {
            self.edit_note(index);
            ui.close();
        }
        if ui.button("Starting position note…").clicked() {
            self.edit_note(0);
            ui.close();
        }
        ui.separator();
        if ui
            .checkbox(&mut self.show_move_hover_text, "Show Hover Text")
            .changed()
        {
            self.save_game();
        }
        ui.separator();
        let playable = !self.fics_active && !self.training_live()
            && self
                .review_positions
                .get(index)
                .is_some_and(|board| board.status() == BoardStatus::Ongoing);
        ui.add_enabled_ui(playable, |ui| {
            ui.menu_button("Play from this position", |ui| {
                Self::set_menu_item_font(ui);
                for opponent in [OpponentEngine::Stockfish] {
                    if ui.button(opponent.label()).clicked() {
                        self.review_index = Some(index);
                        self.engine_config.opponent = opponent;
                        self.engine_enabled = true;
                        self.play_from_current_position();
                        ui.close();
                    }
                }
            });
        });
        ui.separator();
        for (label, annotated) in [("Copy Board to Clipboard", false), ("Copy Annotated Board to Clipboard", true)] {
        let copy_marks = self.annotated_copy_marks(index);
        let drawing_count = copy_marks.len();
        let response = ui.add_enabled(!annotated || drawing_count > 0, egui::Button::new(label));
        let response = if annotated {
            response.on_hover_text(if drawing_count == 0 {
                "This position has no personal drawings or displayed Stockfish recommendation arrow. Right-click the annotated move you want to copy.".to_owned()
            } else {
                format!("Copy {} with {drawing_count} drawing(s), including any displayed Stockfish recommendation.", self.graph_position_label(index, true))
            })
        } else { response };
        if response.clicked() {
            #[cfg(target_arch = "wasm32")]
            if self.board_3d_active {
                if begin_board_png_copy() {
                    let board = self.review_positions.get(index).unwrap_or(&self.board);
                    match self.board_3d_png(board, ui.ctx()) {
                        Ok(png) => {
                            let coordinates = if self.show_coordinates && !self.board_3d_view_rotated() {
                                let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::splat(1200.0));
                                let view = self.board_3d_view();
                                let labels = crate::board3d::coordinate_labels(rect, self.flipped, view, self.board_3d_theme);
                                let spacing = labels.get(0).zip(labels.get(1))
                                    .map(|((_, a), (_, b))| a.distance(*b))
                                    .unwrap_or(48.0);
                                serde_json::json!({
                                    "fontSize": (spacing * 0.22).clamp(10.0, 18.0),
                                    "labels": labels.iter().map(|(text, point)| serde_json::json!({
                                        "text": text.to_string(), "x": point.x, "y": point.y,
                                    })).collect::<Vec<_>>(),
                                }).to_string()
                            } else {
                                String::new()
                            };
                            let coordinates = if annotated {
                                let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::splat(1200.0));
                                let view = self.board_3d_view();
                                let drawings: Vec<_> = copy_marks.iter().filter_map(|mark| {
                                    let from = Square::from_str(&mark.from).ok()?;
                                    let to = Square::from_str(&mark.to).ok()?;
                                    let source = crate::board3d::square_center(from, rect, self.flipped, view)?;
                                    let dest = crate::board3d::square_center(to, rect, self.flipped, view)?;
                                    let outline = if mark.style.contains("circle") { crate::board3d::circle_outline(from, rect, self.flipped, view)? } else { crate::board3d::square_outline(from, rect, self.flipped, view)? };
                                    Some(serde_json::json!({ "color": mark.color.to_string(), "style": mark.style, "from": [source.x, source.y], "to": [dest.x, dest.y],
                                        "outline": if from == to { outline.iter().map(|p| vec![p.x, p.y]).collect::<Vec<_>>() } else { Vec::new() } }))
                                }).collect();
                                let mut settings: serde_json::Value = serde_json::from_str(&coordinates).unwrap_or_else(|_| serde_json::json!({"labels": []}));
                                settings["drawings"] = serde_json::json!(drawings);
                                settings["cell"] = serde_json::json!(1200.0 / 9.0);
                                settings.to_string()
                            } else { coordinates };
                            finish_board_png_copy(&js_sys::Uint8Array::from(png.as_slice()), &coordinates);
                        }
                        Err(error) => fail_board_png_copy(&format!("Could not render 3D board: {error:?}")),
                    }
                }
            } else {
                copy_board_image(
                    &self.position_fen_for_copy(index),
                    &serde_json::json!({
                        "flipped": self.flipped, "frame": self.show_board_frame,
                        "coordinates": self.show_coordinates, "shadows": self.piece_shadows,
                        "pieceSet": self.piece_set,
                        "boardMarks": if annotated { copy_marks.clone() } else { Vec::new() }
                    })
                    .to_string(),
                );
            }
            ui.close();
        }
        }
        if ui.button("Copy FEN to Clipboard").clicked() {
            ui.ctx().copy_text(self.position_fen_for_copy(index));
            ui.close();
        }
        if ui.button("Copy PGN (up to selected move)").clicked() {
            ui.ctx().copy_text(self.clipboard_pgn(index));
            ui.close();
        }
        if ui.button("Copy PGN (Full - all moves)").clicked() {
            ui.ctx()
                .copy_text(self.clipboard_pgn(self.review_moves.len()));
            ui.close();
        }
    }

    fn game_moves_status_ui(&mut self, ui: &mut egui::Ui) {
        if self.fics_active && !self.fics_game_finished {
            return;
        }
        let result = self.game_result();
        if result == "*" {
            let status = if self.board.side_to_move() == Color::White {
                "White to move"
            } else {
                "Black to move"
            };
            let response = ui
                .label(RichText::new(status).size(16.0).weak())
                .interact(Sense::click());
            let index = self.review_index.unwrap_or(self.review_moves.len());
            self.move_context_response(&response, index);
        } else {
            let final_board = self.review_positions.last().copied().unwrap_or(self.board);
            let (heading, detail, accent) = match result.as_str() {
                "1-0" => (
                    "WHITE WINS · 1–0".to_owned(),
                    if final_board.status() == BoardStatus::Checkmate {
                        format!("{} won by checkmate", self.review_white_player)
                    } else {
                        format!("{} won", self.review_white_player)
                    },
                    Color32::from_rgb(112, 193, 133),
                ),
                "0-1" => (
                    "BLACK WINS · 0–1".to_owned(),
                    if final_board.status() == BoardStatus::Checkmate {
                        format!("{} won by checkmate", self.review_black_player)
                    } else {
                        format!("{} won", self.review_black_player)
                    },
                    Color32::from_rgb(112, 193, 133),
                ),
                _ => (
                    "DRAW · ½–½".to_owned(),
                    if final_board.status() == BoardStatus::Stalemate {
                        "Draw by stalemate".to_owned()
                    } else {
                        "Drawn game".to_owned()
                    },
                    Color32::from_rgb(211, 173, 98),
                ),
            };
            let detail = if !self.fics_active && self.local_resigned_white.is_some() {
                "Game ended by resignation".to_owned()
            } else if !self.fics_active
                && let Some(white) = self
                    .local_clock
                    .as_ref()
                    .and_then(|clock| clock.flagged_white)
            {
                format!("{} ran out of time", if white { "White" } else { "Black" })
            } else if self.fics_active && !self.fics_playing {
                self.fics_observed_games
                    .iter()
                    .find(|game| Some(game.id) == self.fics_game_id)
                    .and_then(|game| game.end_message.as_deref())
                    .and_then(|message| message.split_once(") "))
                    .and_then(|(_, tail)| tail.rsplit_once('}'))
                    .map(|(reason, _)| reason.to_owned())
                    .unwrap_or(detail)
            } else {
                detail
            };
            Frame::new()
                .fill(Color32::from_rgba_unmultiplied(
                    accent.r(),
                    accent.g(),
                    accent.b(),
                    24,
                ))
                .stroke(Stroke::new(1.0, accent))
                .corner_radius(CornerRadius::same(6))
                .inner_margin(Margin::symmetric(10, 7))
                .show(ui, |ui| {
                    ui.label(RichText::new(heading).size(15.0).strong().color(accent));
                    ui.label(RichText::new(detail).weak());
                });
        }
        ui.separator();
    }

    fn fics_clock(seconds: i32) -> String {
        format!("{}:{:02}", seconds.max(0) / 60, seconds.max(0) % 60)
    }

    fn fics_display_clocks(&self) -> (i32, i32) {
        let elapsed = (Self::animation_time() - self.fics_board_at).max(0.0) as i32;
        (
            (self.fics_white_time
                - if self.fics_turn == Color::White {
                    elapsed
                } else {
                    0
                })
            .max(0),
            (self.fics_black_time
                - if self.fics_turn == Color::Black {
                    elapsed
                } else {
                    0
                })
            .max(0),
        )
    }

    fn fics_board_clock_ui(ui: &mut egui::Ui, seconds: i32, active: bool) {
        Frame::new()
            .fill(if active {
                Color32::from_rgb(85, 67, 37)
            } else {
                Color32::from_rgb(36, 44, 45)
            })
            .corner_radius(CornerRadius::same(4))
            .inner_margin(Margin::symmetric(7, 2))
            .show(ui, |ui| {
                ui.label(
                    RichText::new(Self::fics_clock(seconds))
                        .font(FontId::monospace(16.0))
                        .color(if active {
                            Color32::from_rgb(245, 215, 148)
                        } else {
                            Color32::from_rgb(185, 195, 190)
                        }),
                );
            });
    }

    fn game_result_badge(ui: &mut egui::Ui, result: &str, player_is_white: bool) {
        let outcome = match (result, player_is_white) {
            ("1-0", true) | ("0-1", false) => Some(("WIN", Color32::from_rgb(112, 193, 133))),
            ("1-0", false) | ("0-1", true) => Some(("LOSS", Color32::from_rgb(232, 112, 112))),
            ("1/2-1/2", _) => Some(("DRAW", Color32::from_rgb(211, 173, 98))),
            _ => None,
        };
        if let Some((label, accent)) = outcome {
            Frame::new()
                .fill(Color32::from_rgba_unmultiplied(
                    accent.r(),
                    accent.g(),
                    accent.b(),
                    24,
                ))
                .stroke(Stroke::new(1.0, accent))
                .corner_radius(CornerRadius::same(4))
                .inner_margin(Margin::symmetric(6, 2))
                .show(ui, |ui| {
                    ui.label(RichText::new(label).small().strong().color(accent));
                });
        }
    }

    fn board_action_icon(ui: &mut egui::Ui, flip: bool, enabled: bool) -> egui::Response {
        let response = ui.add_enabled(
            enabled,
            egui::Button::new("").min_size(Vec2::new(26.0, 22.0)),
        );
        let rect = response.rect.shrink(5.0);
        let stroke = ui.style().interact(&response).fg_stroke;
        let painter = ui.painter();
        if flip {
            let left = rect.left() + rect.width() * 0.32;
            let right = rect.right() - rect.width() * 0.32;
            painter.line_segment(
                [
                    egui::pos2(left, rect.bottom()),
                    egui::pos2(left, rect.top()),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    egui::pos2(left, rect.top()),
                    egui::pos2(rect.left(), rect.center().y),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    egui::pos2(left, rect.top()),
                    egui::pos2(rect.center().x, rect.center().y),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    egui::pos2(right, rect.top()),
                    egui::pos2(right, rect.bottom()),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    egui::pos2(right, rect.bottom()),
                    egui::pos2(rect.center().x, rect.center().y),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    egui::pos2(right, rect.bottom()),
                    egui::pos2(rect.right(), rect.center().y),
                ],
                stroke,
            );
        } else {
            let center_y = rect.center().y;
            painter.line_segment(
                [
                    egui::pos2(rect.left(), center_y),
                    egui::pos2(rect.right(), center_y),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    egui::pos2(rect.left(), center_y),
                    egui::pos2(rect.center().x, rect.top()),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    egui::pos2(rect.left(), center_y),
                    egui::pos2(rect.center().x, rect.bottom()),
                ],
                stroke,
            );
        }
        response
    }

    fn game_moves_header_ui(ui: &mut egui::Ui, min_move_width: f32) {
        ui.horizontal(|ui| {
            ui.add_sized(
                [30.0, 22.0],
                egui::Label::new(RichText::new("#").strong().weak()),
            );
            let move_width =
                ((ui.available_width() - ui.spacing().item_spacing.x) / 2.0).max(min_move_width);
            for heading in ["White", "Black"] {
                ui.add_sized(
                    [move_width, 22.0],
                    egui::Label::new(RichText::new(heading).strong().weak()),
                );
            }
        });
        ui.separator();
    }

    fn new_game_color_card(
        ui: &mut egui::Ui,
        piece_set: PieceSet,
        choice: NewGameColor,
        selected: bool,
    ) -> egui::Response {
        let size = Vec2::new(150.0, 124.0);
        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
        let accent = Color32::from_rgb(211, 173, 98);
        let border = if selected {
            Stroke::new(2.0, accent)
        } else {
            Stroke::new(1.0, Color32::from_white_alpha(36))
        };
        ui.painter().rect_filled(
            rect,
            CornerRadius::same(8),
            if selected {
                Color32::from_rgb(43, 39, 29)
            } else {
                Color32::from_rgb(24, 28, 33)
            },
        );
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(8),
            border,
            egui::StrokeKind::Inside,
        );

        match choice {
            NewGameColor::White | NewGameColor::Black => {
                let color = if choice == NewGameColor::White {
                    Color::White
                } else {
                    Color::Black
                };
                let king_rect = egui::Rect::from_center_size(
                    egui::pos2(rect.center().x, rect.top() + 40.0),
                    Vec2::splat(48.0),
                );
                Self::paint_piece(ui, king_rect, 48.0, piece_set, color, Piece::King);
            }
            NewGameColor::Random => {
                for (color, x) in [
                    (Color::White, rect.center().x - 19.0),
                    (Color::Black, rect.center().x + 19.0),
                ] {
                    let king_rect = egui::Rect::from_center_size(
                        egui::pos2(x, rect.top() + 40.0),
                        Vec2::splat(40.0),
                    );
                    Self::paint_piece(ui, king_rect, 40.0, piece_set, color, Piece::King);
                }
            }
        }

        let (title, subtitle) = match choice {
            NewGameColor::White => ("White", "Play as White"),
            NewGameColor::Black => ("Black", "Play as Black"),
            NewGameColor::Random => ("Random", "Let Ironwood choose"),
        };
        ui.painter().text(
            egui::pos2(rect.center().x, rect.top() + 73.0),
            Align2::CENTER_CENTER,
            title,
            FontId::proportional(14.0),
            if selected {
                accent
            } else {
                ui.visuals().text_color()
            },
        );
        ui.painter().text(
            egui::pos2(rect.center().x, rect.top() + 94.0),
            Align2::CENTER_CENTER,
            subtitle,
            FontId::proportional(12.0),
            ui.visuals().weak_text_color(),
        );
        if selected {
            ui.painter().text(
                egui::pos2(rect.center().x, rect.bottom() - 11.0),
                Align2::CENTER_CENTER,
                "SELECTED",
                FontId::proportional(9.5),
                accent,
            );
        }
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    }

    fn promotion_piece_card(
        ui: &mut egui::Ui,
        piece_set: PieceSet,
        color: Color,
        piece: Piece,
        name: &str,
    ) -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(Vec2::new(86.0, 108.0), Sense::click());
        let gold = Color32::from_rgb(211, 173, 98);
        let hovering = response.hovered();
        ui.painter().rect_filled(
            rect,
            CornerRadius::same(8),
            if hovering {
                Color32::from_rgb(48, 45, 36)
            } else {
                Color32::from_rgb(31, 36, 41)
            },
        );
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(8),
            Stroke::new(if hovering { 2.0 } else { 1.0 }, if hovering { gold } else { gold.gamma_multiply(0.45) }),
            egui::StrokeKind::Inside,
        );
        let piece_rect = egui::Rect::from_center_size(
            egui::pos2(rect.center().x, rect.top() + 41.0),
            Vec2::splat(62.0),
        );
        ui.painter().rect_filled(
            piece_rect,
            CornerRadius::same(6),
            if color == Color::White {
                Color32::from_rgb(64, 77, 67)
            } else {
                Color32::from_rgb(192, 203, 184)
            },
        );
        Self::paint_piece(ui, piece_rect, 62.0, piece_set, color, piece);
        ui.painter().text(
            egui::pos2(rect.center().x, rect.bottom() - 16.0),
            Align2::CENTER_CENTER,
            name,
            FontId::proportional(14.0),
            if hovering { gold } else { ui.visuals().text_color() },
        );
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    }

    fn start_battle_button(ui: &mut egui::Ui, piece_set: PieceSet, online: bool) -> egui::Response {
        let size = Vec2::new(310.0, 64.0);
        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
        let accent = Color32::from_rgb(211, 173, 98);
        let hovered = response.hovered();

        ui.painter().rect_filled(
            rect,
            CornerRadius::same(9),
            if hovered {
                Color32::from_rgb(58, 49, 29)
            } else {
                Color32::from_rgb(43, 39, 29)
            },
        );
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(9),
            Stroke::new(if hovered { 2.0 } else { 1.0 }, accent),
            egui::StrokeKind::Inside,
        );

        let piece_size = 46.0;
        let left_piece = egui::Rect::from_center_size(
            egui::pos2(rect.left() + 39.0, rect.center().y),
            Vec2::splat(piece_size),
        );
        let right_piece = egui::Rect::from_center_size(
            egui::pos2(rect.right() - 39.0, rect.center().y),
            Vec2::splat(piece_size),
        );
        Self::paint_piece(
            ui,
            left_piece,
            piece_size,
            piece_set,
            Color::White,
            Piece::Knight,
        );
        Self::paint_piece(
            ui,
            right_piece,
            piece_size,
            piece_set,
            Color::Black,
            Piece::Knight,
        );

        ui.painter().text(
            egui::pos2(rect.center().x, rect.center().y - 8.0),
            Align2::CENTER_CENTER,
            if online {
                "CONNECT TO FICS"
            } else {
                "START GAME"
            },
            FontId::proportional(18.0),
            Color32::WHITE,
        );
        ui.painter().text(
            egui::pos2(rect.center().x, rect.center().y + 13.0),
            Align2::CENTER_CENTER,
            if online {
                "PLAY ONLINE"
            } else {
                "BEGIN THE BATTLE"
            },
            FontId::proportional(10.0),
            accent,
        );

        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn graph_phase_ranges_cover_positions_and_follow_phase_summaries() {
        let context = eframe::CreationContext::_new_kittest(egui::Context::default());
        let mut app = ChessApp::new(&context);
        app.review_positions = vec![Board::default(); 26];
        let endgame = Board::from_str("4k3/8/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        for board in &mut app.review_positions[22..] { *board = endgame; }
        let ranges = app.graph_phase_ranges(26);
        assert!(ranges == vec![(GamePhase::Opening, 0, 20), (GamePhase::Middlegame, 21, 22), (GamePhase::Endgame, 23, 25)]);
        assert!(app.graph_phase_ranges(12) == vec![(GamePhase::Opening, 0, 11)]);
        assert!(app.graph_phase_ranges(0).is_empty());
        assert!(app.graph_phase_ranges(100) == ranges);
        app.review_positions[24] = Board::default();
        assert!(app.graph_phase_ranges(26).last() == Some(&(GamePhase::Middlegame, 25, 25)));
    }
    #[test]
    fn keyboard_moves_validate_notation_and_legality() {
        let board = Board::default();
        assert_eq!(ChessApp::parse_keyboard_move(&board, "e4"), Some(ChessMove::new(Square::E2, Square::E4, None)));
        assert_eq!(ChessApp::parse_keyboard_move(&board, "e2e4"), ChessApp::parse_keyboard_move(&board, "e4"));
        assert!(ChessApp::parse_keyboard_move(&board, "Nf3").is_some());
        assert_eq!(ChessApp::parse_keyboard_move(&board, "nc3"), ChessApp::parse_keyboard_move(&board, "Nc3"));
        assert!(ChessApp::parse_keyboard_move(&board, "nc3").is_some());
        assert_eq!(ChessApp::parse_keyboard_move(&board, "b4"), Some(ChessMove::new(Square::B2, Square::B4, None)));
        assert!(ChessApp::parse_keyboard_move(&board, "e5").is_none());
        assert!(ChessApp::parse_keyboard_move(&board, "e2e5").is_none());
        let castle = Board::from_str("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").unwrap();
        assert_eq!(ChessApp::parse_keyboard_move(&castle, "0-0"), ChessApp::parse_keyboard_move(&castle, "O-O"));
        assert!(ChessApp::parse_keyboard_move(&castle, "O-O").is_some());
        let promotion = Board::from_str("7k/P7/8/8/8/8/8/K7 w - - 0 1").unwrap();
        assert_eq!(ChessApp::parse_keyboard_move(&promotion, "a8=N").unwrap().get_promotion(), Some(Piece::Knight));
        assert_eq!(ChessApp::parse_keyboard_move(&promotion, "a8=n"), ChessApp::parse_keyboard_move(&promotion, "a8=N"));
    }

    #[test]
    fn board_visibility_preferences_default_on_and_preserve_off_values() {
        let defaults: super::UserPreferences = serde_json::from_str("{}").unwrap();
        assert!(defaults.show_highlighted_move && defaults.show_radial_light);
        let preferences: super::UserPreferences = serde_json::from_str(
            "{\"show_highlighted_move\":false,\"show_radial_light\":false}"
        ).unwrap();
        let restored: super::UserPreferences = serde_json::from_str(&serde_json::to_string(&preferences).unwrap()).unwrap();
        assert!(!restored.show_highlighted_move && !restored.show_radial_light);
    }
    #[test]
    fn fide_profile_links_use_the_correct_player_and_reject_invalid_ids() {
        let pgn = "[WhiteFideId \"1503014\"]\n[BlackFideId \"123456\"]\n";
        assert_eq!(super::ChessApp::fide_profile_url(pgn, true).as_deref(), Some("https://ratings.fide.com/profile/1503014"));
        assert_eq!(super::ChessApp::fide_profile_url(pgn, false).as_deref(), Some("https://ratings.fide.com/profile/123456"));
        for id in ["", "0", "000", "?", "-1", "12/34", "javascript:alert(1)"] {
            assert!(super::ChessApp::fide_profile_url(&format!("[WhiteFideId \"{id}\"]"), true).is_none());
        }
        assert!(super::ChessApp::fide_profile_url("", false).is_none());
    }
    #[test]
    fn elo_calculator_matches_standard_expected_score_and_changes() {
        let expected = ChessApp::elo_expected_score(1200, 1400);
        assert!((expected - 0.240_253).abs() < 0.000_001);
        assert_eq!((20.0 * (1.0 - expected)).round() as i32, 15);
        assert_eq!((20.0 * (0.5 - expected)).round() as i32, 5);
        assert_eq!((20.0 * (0.0 - expected)).round() as i32, -5);
        assert_eq!(ChessApp::elo_expected_score(1500, 1500), 0.5);
    }

    #[test]
    fn captured_material_follows_the_displayed_position_and_ignores_promotion() {
        use chess::{ChessMove, Color, Piece, Square};

        let (positions, _) = ChessApp::parse_pgn_mainline("1. e4 d5 2. exd5 Qxd5 *").unwrap();
        assert_eq!(ChessApp::captured_pieces(&positions[..4], Color::Black), vec![Piece::Pawn]);
        assert!(ChessApp::captured_pieces(&positions[..4], Color::White).is_empty());
        assert_eq!(ChessApp::material_score(&positions[3], Color::White)
            - ChessApp::material_score(&positions[3], Color::Black), 1);
        assert_eq!(ChessApp::captured_pieces(&positions, Color::White), vec![Piece::Pawn]);

        let before = Board::from_str("7k/P7/8/8/8/8/8/K7 w - - 0 1").unwrap();
        let after = before.make_move_new(ChessMove::new(Square::A7, Square::A8, Some(Piece::Queen)));
        assert!(ChessApp::captured_pieces(&[before, after], Color::White).is_empty());
    }

    #[test]
    fn annotated_copy_includes_stockfish_arrow_without_personal_drawings() {
        let context = eframe::CreationContext::_new_kittest(egui::Context::default());
        let mut app = ChessApp::new(&context);
        app.load_pgn("1. f3 e5 2. g4 Qh4# 0-1");
        app.show_best_move_arrows = true;
        app.game_analysis.resize(app.review_positions.len(), None);
        app.game_analysis[2] = Some(super::PositionAnalysis { eval_cp: Some(0), best_move: Some("e2e4".into()), ..Default::default() });
        app.game_analysis[3] = Some(super::PositionAnalysis { eval_cp: Some(-900), ..Default::default() });
        let marks = app.annotated_copy_marks(3);
        assert_eq!(marks, vec![super::BoardMark { style: String::new(), color: 'G', from: "e2".into(), to: "e4".into() }]);
        assert!(app.board_marks.iter().all(Vec::is_empty));
        app.show_best_move_arrows = false;
        assert!(app.annotated_copy_marks(3).is_empty());
        app.toggle_board_mark(3, super::BoardMark { style: String::new(), color: 'R', from: "g4".into(), to: "g4".into() });
        app.show_best_move_arrows = true;
        assert_eq!(app.annotated_copy_marks(3).len(), 2);
        assert!(app.annotated_copy_marks(1).is_empty());
    }

    #[test]
    fn board_drawings_toggle_roundtrip_and_follow_retained_positions() {
        let context = eframe::CreationContext::_new_kittest(egui::Context::default());
        let mut app = ChessApp::new(&context);
        app.load_pgn("1. e4 e5 2. Nf3 *");
        let arrow = super::BoardMark { style: String::new(), color: 'G', from: "e2".into(), to: "e4".into() };
        let square = super::BoardMark { style: String::new(), color: 'R', from: "f6".into(), to: "f6".into() };
        app.toggle_board_mark(0, arrow.clone());
        app.toggle_board_mark(1, square.clone());
        app.toggle_board_mark(3, arrow.clone());
        let marks = app.board_marks.clone();
        let saved = serde_json::to_string(&app.persisted_game()).unwrap();
        let restored: super::PersistedGame = serde_json::from_str(&saved).unwrap();
        assert_eq!(restored.board_marks, marks);
        for pgn in [app.annotated_pgn(), ChessApp::export_saved_game(&saved, "annotated").unwrap()] {
            let (positions, _) = ChessApp::parse_pgn_mainline(&pgn).unwrap();
            assert_eq!(ChessApp::parse_pgn_board_marks(&pgn, &positions), marks);
            let mut imported = ChessApp::new(&context);
            imported.load_pgn(&pgn);
            assert_eq!(imported.board_marks, marks);
        }
        let json = ChessApp::export_saved_game(&saved, "json").unwrap();
        let mut imported = ChessApp::new(&context);
        assert!(imported.load_analysis_json(&json));
        assert_eq!(imported.board_marks, marks);
        assert!(!ChessApp::export_saved_game(&saved, "pgn").unwrap().contains("[%cal"));
        app.toggle_board_mark(0, arrow.clone());
        assert!(app.board_marks[0].is_empty());
        app.toggle_board_mark(1, super::BoardMark { color: 'Y', ..square.clone() });
        assert_eq!(app.board_marks[1].len(), 1);
        assert_eq!(app.board_marks[1][0].color, 'Y');
        app.review_to(1);
        app.play_from_current_position();
        assert_eq!(app.board_marks.len(), 2);
        app.reset_for_side(super::PlayerSide::White);
        assert!(app.board_marks.is_empty());
        let pgn = "{ [%csl Ge4,Re9,Ze2,éa1] } 1. e4 { [%cal Ge2e4,Ga0e4] [%csl Yf6] } (1. d4 { [%csl Rd4] }) *";
        let (positions, _) = ChessApp::parse_pgn_mainline(pgn).unwrap();
        let parsed = ChessApp::parse_pgn_board_marks(pgn, &positions);
        assert_eq!(parsed[0].len(), 1);
        assert_eq!(parsed[1].len(), 2);
    }

    #[test]
    fn board_drawing_input_maps_flipped_squares_and_preserves_board() {
        let context = eframe::CreationContext::_new_kittest(egui::Context::default());
        let mut app = ChessApp::new(&context);
        app.load_pgn("1. e4 *");
        app.review_to(0);
        let board = app.board;
        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(400.0, 400.0));
        let source = rect.min + ChessApp::square_screen_offset(Square::E2, false, 50.0);
        let dest = rect.min + ChessApp::square_screen_offset(Square::E4, false, 50.0);
        let modifiers = egui::Modifiers { alt: true, shift: true, ..Default::default() };
        for (pos, pressed) in [(source, true), (dest, false)] {
            let input = egui::RawInput { modifiers, events: vec![egui::Event::PointerMoved(pos),
                egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers }], ..Default::default() };
            let _ = context.egui_ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| app.board_mark_input(ui, rect));
            });
        }
        assert_eq!(app.board_marks[0], vec![super::BoardMark { style: String::new(), color: 'R', from: "e2".into(), to: "e4".into() }]);
        assert_eq!(app.board, board);
        app.flipped = true;
        let pos = rect.min + ChessApp::square_screen_offset(Square::E2, true, 50.0);
        assert_eq!(app.board_mark_square(pos, rect), Some(Square::E2));
        assert!(app.board_mark_square(egui::pos2(-1.0, 0.0), rect).is_none());
    }

    #[test]
    fn manual_annotations_roundtrip_and_ignore_variations_and_comments() {
        let pgn = "[Result \"*\"]\n\n$10 1. e4! $14 { $4 } (1. d4?? $19) e5?! 2. Nf3 $3 $16 *";
        let (positions, _) = super::ChessApp::parse_pgn_mainline(pgn).unwrap();
        let nags = super::ChessApp::parse_pgn_nags(pgn, &positions);
        assert_eq!(nags, vec![vec![10], vec![1, 14], vec![6], vec![3, 16]]);
        let context = eframe::CreationContext::_new_kittest(egui::Context::default());
        let mut app = ChessApp::new(&context);
        app.load_pgn(pgn);
        assert_eq!(app.move_annotations, nags);
        let saved = serde_json::to_string(&app.persisted_game()).unwrap();
        let restored: super::PersistedGame = serde_json::from_str(&saved).unwrap();
        assert_eq!(restored.move_annotations, nags);
        let annotated = app.annotated_pgn();
        let (boards, _) = super::ChessApp::parse_pgn_mainline(&annotated).unwrap();
        assert_eq!(super::ChessApp::parse_pgn_nags(&annotated, &boards), nags);
        let exported = super::ChessApp::export_saved_game(&saved, "annotated").unwrap();
        assert_eq!(super::ChessApp::parse_pgn_nags(&exported, &boards), nags);
        let json = super::ChessApp::export_saved_game(&saved, "json").unwrap();
        let mut imported = ChessApp::new(&context);
        assert!(imported.load_analysis_json(&json));
        assert_eq!(imported.move_annotations, nags);
        let plain = super::ChessApp::export_saved_game(&saved, "pgn").unwrap();
        assert!(!plain.contains('$'));
        app.play_from_current_position();
        app.reset_for_side(super::PlayerSide::White);
        assert!(app.move_annotations.is_empty());
    }

    #[test]
    fn move_notes_roundtrip_without_becoming_engine_annotations() {
        let context = eframe::CreationContext::_new_kittest(egui::Context::default());
        let mut app = ChessApp::new(&context);
        app.load_pgn("[White \"Ada\"]\n[Black \"Ben\"]\n\n1. e4 e5 2. Nf3 *");
        assert!(app.pgn_error.is_none());
        let note =
            "Consider {a plan} [not an engine tag] \"carefully\"\n<script>not HTML</script> ♞";
        app.set_move_note(0, "Opening plan".into());
        app.set_move_note(2, note.into());
        let saved = serde_json::to_string(&app.persisted_game()).unwrap();
        let restored: super::PersistedGame = serde_json::from_str(&saved).unwrap();
        assert_eq!(restored.move_notes[2], note);
        let pgn = ChessApp::export_saved_game(&saved, "annotated").unwrap();
        let mut imported = ChessApp::new(&context);
        imported.load_pgn(&pgn);
        assert!(imported.pgn_error.is_none());
        assert_eq!(imported.note_at(0), "Opening plan");
        assert_eq!(imported.note_at(2), note);
        assert!(imported.game_analysis.iter().all(Option::is_none));
        let json = ChessApp::export_saved_game(&saved, "json").unwrap();
        assert!(imported.load_analysis_json(&json));
        assert_eq!(imported.note_at(2), note);
        let plain = ChessApp::export_saved_game(&saved, "pgn").unwrap();
        assert!(!plain.contains("note"));
        let report = app.analysis_report_html();
        assert!(report.contains("Opening plan"));
        assert!(report.contains("&lt;script&gt;not HTML&lt;/script&gt;"));
        assert!(!report.contains("<script>not HTML"));
        app.review_index = Some(1);
        app.play_from_current_position();
        assert_eq!(app.note_at(0), "Opening plan");
        assert_eq!(app.note_at(2), "");
        app.set_move_note(0, String::new());
        assert_eq!(app.note_at(0), "");
        app.reset_for_side(super::PlayerSide::White);
        assert!(app.move_notes.is_empty());
    }

    #[test]
    fn saved_exports_preserve_moves_analysis_and_custom_start() {
        let pgn = "[White \"Ada\"]\n[Black \"Ben\"]\n[Date \"2026.09.27\"]\n[Result \"*\"]\n\n1. e4 e5 2. Nf3 *";
        let record = serde_json::json!({ "board": super::START_FEN, "history": [], "last_move": null,
            "flipped": false, "engine_enabled": false, "review_pgn": pgn,
            "game_analysis": [null, {"eval_cp":32,"mate":null,"depth":18,"nodes":1000,"best_move":"e7e5","pv":"e5"},null,null] });
        let json = record.to_string();
        let plain = super::ChessApp::export_saved_game(&json, "pgn").unwrap();
        assert!(plain.contains("1. e4 e5 2. Nf3 *"));
        assert!(plain.contains("2026.09.27"));
        assert!(!plain.contains("[%eval"));
        let annotated = super::ChessApp::export_saved_game(&json, "annotated").unwrap();
        assert!(annotated.contains("[%eval +0.32]"));
        let (boards, moves) = super::ChessApp::parse_pgn_mainline(&annotated).unwrap();
        assert_eq!(moves.len(), 3);
        assert_eq!(
            super::ChessApp::parse_pgn_annotations(&annotated, &boards)[1]
                .as_ref()
                .unwrap()
                .eval_cp,
            Some(32)
        );
        let exported = super::ChessApp::export_saved_game(&json, "json").unwrap();
        let imported: super::ImportedAnalysisFile = serde_json::from_str(&exported).unwrap();
        assert_eq!(imported.positions.len(), 4);
        assert_eq!(imported.positions[1].evaluation_cp, Some(32));
        assert_eq!(imported.game.moves, moves);
        let mut custom = record;
        custom["review_pgn"] = serde_json::json!(
            "[SetUp \"1\"]\n[FEN \"7k/8/8/8/8/8/8/K7 b - - 0 23\"]\n[Result \"*\"]\n\n23... Kh7 *"
        );
        custom["game_analysis"] = serde_json::json!([]);
        let exported = super::ChessApp::export_saved_game(&custom.to_string(), "pgn").unwrap();
        assert!(exported.contains("23... Kh7 *"));
        assert_eq!(
            super::ChessApp::parse_pgn_mainline(&exported).unwrap().1,
            vec!["Kh7"]
        );
        assert!(super::ChessApp::export_saved_game(&json, "invalid").is_err());
    }

    #[test]
    fn removed_opponent_preferences_fall_back_to_stockfish() {
        let config: super::EngineConfig = serde_json::from_str(
            r#"{"opponent":"Lc0","lc0_difficulty":"Hard","lc0_move_time_ms":2400,"move_time_ms":900}"#
        ).unwrap();
        assert_eq!(config.opponent, super::OpponentEngine::Stockfish);
        assert_eq!(config.move_time_ms, 900);
        let saved = serde_json::to_string(&config).unwrap();
        assert!(!saved.contains("lc0"));
        assert!(!saved.contains("Lc0"));
    }

    #[test]
    fn running_games_menu_filters_private_and_unsupported_games() {
        let game = super::FicsRunningGame::parse(
            "fics% 2 2274 OldManII ++++ Peshkin [ bu 2 12] 2:34 - 1:47 (39-39) B: 3",
        )
        .unwrap();
        assert_eq!(game.id, 2);
        assert!(game.label.contains("OldManII"));
        assert!(game.label.contains("2+12"));
        for line in [
            "1 1878 Roberto 1881 baraka [psr 45 30]",
            "3 2000 A 1800 B [ zr 3 0]",
            "6 games displayed (of 23 in progress)",
            "25 (Exam. 0 Friar 0 Friar) [ uu 0 0]",
        ] {
            assert!(super::FicsRunningGame::parse(line).is_none());
        }
    }

    #[test]
    fn local_clock_debits_only_the_active_side_and_flags_once() {
        let mut clock = super::LocalClock {
            white: 60.0,
            black: 60.0,
            increment: 2.0,
            flagged_white: None,
            updated_at: 100.0,
        };
        assert!(!clock.advance(110.0, true, true));
        assert_eq!((clock.white, clock.black), (50.0, 60.0));
        assert!(!clock.advance(140.0, false, false));
        assert_eq!(clock.black, 60.0);
        assert!(!clock.advance(145.0, false, true));
        assert_eq!(clock.black, 55.0);
        assert!(clock.advance(200.0, false, true));
        assert_eq!(clock.flagged_white, Some(false));
        assert!(!clock.advance(210.0, true, true));
        assert_eq!(clock.white, 50.0);
    }

    #[test]
    fn chess960_san_and_pgn_replay_castling() {
        let pgn = "[Variant \"Chess960\"]\n[SetUp \"1\"]\n[FEN \"4k3/8/8/8/8/8/8/5KR1 w G - 0 1\"]\n\n1. O-O *";
        let (positions, moves) = super::ChessApp::parse_pgn_mainline(pgn).unwrap();
        assert_eq!(moves, vec!["O-O"]);
        assert_eq!(
            positions[1].piece_on(chess::Square::G1),
            Some(chess::Piece::King)
        );
        assert_eq!(
            positions[1].piece_on(chess::Square::F1),
            Some(chess::Piece::Rook)
        );
    }
    use super::*;

    #[test]
    fn parses_fics_sought_game_for_join_button() {
        let ad = FicsAd::parse("57 +++ GuestZNKK 1 0 unrated lightning 0-2000").unwrap();
        assert_eq!(ad.id, "57");
        assert_eq!(ad.player, "GuestZNKK");
        assert_eq!(ad.minutes, "1");
        assert_eq!(ad.increment, "0");
        assert!(FicsAd::parse("3 ads displayed.").is_none());
    }

    #[test]
    fn fics_console_colors_distinguish_messages_without_coloring_ascii_art() {
        assert_eq!(FicsConsoleTone::for_line("> who"), FicsConsoleTone::Command);
        assert_eq!(
            FicsConsoleTone::for_line("GuestABCD(53): hello (there)"),
            FicsConsoleTone::Chat
        );
        assert_eq!(
            FicsConsoleTone::for_line("GriffyJr(C)(1914)[17] kibitzes: nice move"),
            FicsConsoleTone::Chat
        );
        assert_eq!(
            FicsConsoleTone::for_line("{Game 12 (Alice vs. Bob) Alice checkmated} 0-1"),
            FicsConsoleTone::Game
        );
        assert_eq!(
            FicsConsoleTone::for_line("You have been added to the mute list"),
            FicsConsoleTone::Warning
        );
        assert_eq!(
            FicsConsoleTone::for_line("Illegal move"),
            FicsConsoleTone::Error
        );
        assert_eq!(
            FicsConsoleTone::for_line("     /\\___"),
            FicsConsoleTone::Plain
        );
    }

    #[test]
    fn finished_fics_game_reopens_with_moves_for_analysis() {
        let moves = vec!["e4".into(), "e5".into(), "Nf3".into(), "Nc6".into()];
        let pgn = fics_game_pgn("Alice", "Bob", "1-0", &moves);
        let (positions, restored_moves) = ChessApp::parse_pgn_mainline(&pgn).unwrap();
        assert_eq!(restored_moves, moves);
        assert_eq!(positions.len(), 5);
        assert_eq!(ChessApp::pgn_tag(&pgn, "Result").as_deref(), Some("1-0"));
        assert_eq!(ChessApp::pgn_tag(&pgn, "White").as_deref(), Some("Alice"));
    }

    #[test]
    fn fics_updates_recover_castling_and_a_missed_board_packet() {
        let start = Board::default();
        let e4 = ChessApp::parse_uci_value("e2e4").unwrap();
        let e5 = ChessApp::parse_uci_value("e7e5").unwrap();
        let after_two = start.make_move_new(e4).make_move_new(e5);
        assert_eq!(
            ChessApp::fics_transition(&start, &after_two, "e7e5", "e5"),
            Some(vec![e4, e5])
        );

        let castle_start = Board::from_str("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").unwrap();
        let castle = ChessApp::parse_san_move(&castle_start, "O-O").unwrap();
        let castle_end = castle_start.make_move_new(castle);
        assert_eq!(
            ChessApp::fics_transition(&castle_start, &castle_end, "", "O-O"),
            Some(vec![castle])
        );
    }

    #[test]
    fn imports_en_passant_and_promotion_san() {
        let game = "1.d4 Nf6 2.c4 e6 3.Nf3 Bb4+ 4.Nbd2 O-O 5.e3 b6 6.Bd3 Bb7 7.O-O d5 8.a3 Bxd2 9.Bxd2 Nbd7 10.cxd5 Bxd5 11.b4 c5 12.Rc1 cxd4 13.Nxd4 Ne5 14.Ba6 Ne4 15.Be1 Qg5 16.f4 Qg6 17.fxe5 Nc5 18.Bg3 Nxa6 19.Nf5 Rae8 20.Nd6 Re7 21.Rf4 h5 22.e4 Ba8 23.Bh4 Rd7 24.Rc3 Qh6 25.Qf1 Nc7 26.Rcf3 f5 27.exf6 Rxd6";
        assert!(ChessApp::parse_pgn_mainline(game).is_ok());
        let promotion_board = Board::from_str("7k/P7/8/8/8/8/8/K7 w - - 0 1").unwrap();
        assert_eq!(
            ChessApp::parse_san_move(&promotion_board, "a8=Q"),
            Some(ChessMove::new(Square::A7, Square::A8, Some(Piece::Queen)))
        );
    }

    #[test]
    fn merida_is_the_default_piece_set() {
        assert!(PieceSet::default() == PieceSet::Merida);
    }

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
    fn converts_san_piece_letters_to_figurines() {
        assert_eq!(ChessApp::figurine_san("Qf3"), "♕f3");
        assert_eq!(ChessApp::figurine_san("Nxe5+"), "♘xe5+");
        assert_eq!(ChessApp::figurine_san("O-O"), "O-O");
        assert_eq!(ChessApp::figurine_san("e8=Q+"), "e8=♕+");
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
    fn restores_full_game_analysis_at_first_unfinished_position() {
        let completed = Some(PositionAnalysis::default());
        let partial = [completed.clone(), None, completed.clone(), None];
        assert_eq!(
            ChessApp::analysis_resume_state(&partial, true, false),
            (Some(1), true, false)
        );
        assert_eq!(
            ChessApp::analysis_resume_state(&partial, false, true),
            (Some(1), false, true)
        );
        assert_eq!(
            ChessApp::analysis_resume_state(&[completed], true, false),
            (None, false, false)
        );
    }

    #[test]
    fn observation_switch_preserves_partial_analysis_and_verification() {
        let context = eframe::CreationContext::_new_kittest(egui::Context::default());
        let mut app = ChessApp::new(&context);
        app.receive_observed_board(
            1,
            "White".into(),
            "Black".into(),
            Board::default(),
            "",
            "",
            1,
            300,
            300,
            false,
        );
        app.fics_observed_games[0].finished = true;
        app.fics_game_finished = true;
        app.fics_observing = false;
        app.set_move_note(0, "First observed game".into());
        app.game_analysis = vec![
            Some(PositionAnalysis {
                eval_cp: Some(42),
                ..Default::default()
            }),
            None,
        ];
        app.game_analysis_index = Some(1);
        app.game_analysis_running = true;
        app.verification_targets = vec![1];
        app.verification_results = vec![None, None];
        app.verification_nodes = 1_250_000;
        app.receive_observed_board(
            2,
            "Other White".into(),
            "Other Black".into(),
            Board::default(),
            "",
            "",
            1,
            300,
            300,
            false,
        );
        app.show_observed_game(2);
        assert!(app.game_analysis.is_empty());
        assert!(!app.game_analysis_running);
        app.show_observed_game(1);
        assert_eq!(app.game_analysis[0].as_ref().unwrap().eval_cp, Some(42));
        assert_eq!(app.note_at(0), "First observed game");
        assert!(app.game_analysis[1].is_none());
        assert_eq!(app.game_analysis_index, Some(1));
        assert!(app.game_analysis_paused);
        assert!(!app.game_analysis_running);
        assert_eq!(app.verification_targets, vec![1]);
        assert_eq!(app.verification_nodes, 1_250_000);
        // Selecting an already selected finished game must not clear its results.
        app.show_observed_game(1);
        assert_eq!(app.game_analysis[0].as_ref().unwrap().eval_cp, Some(42));
    }

    #[test]
    fn observed_en_passant_game_exports_valid_engine_positions() {
        let (positions, moves) =
            ChessApp::parse_pgn_mainline(include_str!("../tests/fixtures/observed-en-passant.pgn"))
                .unwrap();
        assert_eq!(moves.len(), 104);
        assert_eq!(positions.len(), 105);
        assert_eq!(
            positions[97].to_string().split_whitespace().nth(3),
            Some("g3")
        );
        println!("Position 98 FEN: {}", positions[97]);
        for board in &positions {
            let _: shakmaty::Chess = shakmaty::fen::Fen::from_ascii(board.to_string().as_bytes())
                .unwrap()
                .into_position(shakmaty::CastlingMode::Standard)
                .unwrap();
        }
        assert_eq!(positions.last().unwrap().status(), BoardStatus::Checkmate);
    }

    #[test]
    fn observed_save_roundtrips_moves_results_and_paused_analysis() {
        let context = eframe::CreationContext::_new_kittest(egui::Context::default());
        let mut app = ChessApp::new(&context);
        app.receive_observed_board(
            1,
            "Ada".into(),
            "Ben".into(),
            Board::default(),
            "",
            "",
            1,
            300,
            300,
            false,
        );
        let (positions, moves) = ChessApp::parse_pgn_mainline("1. e4 e5 2. Nf3 Nc6 1-0").unwrap();
        let observed = &mut app.fics_observed_games[0];
        observed.board = *positions.last().unwrap();
        observed.positions = positions.clone();
        observed.moves = moves.clone();
        observed.ended = true;
        observed.finished = true;
        observed.end_message = Some("Ada wins 1-0".into());
        app.game_analysis = vec![Some(PositionAnalysis::default()), None, None, None, None];
        app.game_analysis_running = true;
        let snapshot = app.observed_saved_game(&app.fics_observed_games[0]);
        let restored: PersistedGame =
            serde_json::from_str(&serde_json::to_string(&snapshot).unwrap()).unwrap();
        let (restored_positions, restored_moves) =
            ChessApp::parse_pgn_mainline(restored.review_pgn.as_ref().unwrap()).unwrap();
        assert_eq!(restored_positions.len(), positions.len());
        assert_eq!(restored_moves, moves);
        assert_eq!(restored.result, "1-0");
        assert!(!restored.engine_enabled);
        assert_eq!(
            ChessApp::analysis_resume_state(
                &restored.game_analysis,
                restored.game_analysis_running,
                restored.game_analysis_paused
            ),
            (Some(1), false, true)
        );
        // A background game must never inherit the selected game's analysis.
        app.fics_game_id = Some(2);
        let background = app.observed_saved_game(&app.fics_observed_games[0]);
        assert!(background.game_analysis.is_empty());
        assert!(!background.game_analysis_paused);
        // A partial observation must replay from its actual starting position.
        app.fics_observed_games[0].positions = positions[2..].to_vec();
        app.fics_observed_games[0].moves = moves[2..].to_vec();
        app.fics_observed_games[0].start_ply = 2;
        let partial = app.observed_saved_game(&app.fics_observed_games[0]);
        let (partial_positions, _) =
            ChessApp::parse_pgn_mainline(partial.review_pgn.as_ref().unwrap()).unwrap();
        assert_eq!(partial_positions[0].to_string(), positions[2].to_string());
        assert_eq!(
            partial_positions.last().unwrap().to_string(),
            positions.last().unwrap().to_string()
        );
    }

    #[test]
    fn verification_resumes_at_first_unfinished_target() {
        let done = Some(PositionAnalysis::default());
        let results = [None, done.clone(), None, done, None];
        assert_eq!(
            ChessApp::pending_verification_target(&[1, 2, 3, 4], &results),
            Some(2)
        );
        assert_eq!(
            ChessApp::pending_verification_target(&[1, 3], &results),
            None
        );
        assert_eq!(ChessApp::verification_budget(250_000), 1_250_000);
        assert_eq!(ChessApp::verification_budget(1_000_000), 5_000_000);
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
    fn splits_tagged_pgn_collection_without_merging_games() {
        let collection = "[Event \"First\"]\n[White \"Ada\"]\n\n1. e4 e5 1-0\n\n[Event \"Second\"]\n[Black \"Ben\"]\n\n1. d4 d5 0-1\n";
        let games = ChessApp::split_pgn_games(collection);
        assert_eq!(games.len(), 2);
        assert_eq!(
            ChessApp::pgn_tag(&games[0], "White").as_deref(),
            Some("Ada")
        );
        assert_eq!(
            ChessApp::pgn_tag(&games[1], "Black").as_deref(),
            Some("Ben")
        );
        assert_eq!(
            ChessApp::parse_pgn_mainline(&games[0]).unwrap().1,
            ["e4", "e5"]
        );
        assert_eq!(
            ChessApp::parse_pgn_mainline(&games[1]).unwrap().1,
            ["d4", "d5"]
        );
    }

    #[test]
    fn restores_ironwood_analysis_from_annotated_pgn() {
        let pgn = r#"1. e4 { [%eval +0.25]; [%depth 18]; [%nodes 1000000]; [%cpl 12]; Quality: GOOD; Best move: e2e4; PV: e4 e5 Nf3 } 1... e5 { [%eval -0.10]; [%depth 17]; [%nodes 900000]; Best move: e7e5; PV: e5 Nf3 Nc6 }"#;
        let (positions, _) = ChessApp::parse_pgn_mainline(pgn).unwrap();
        let analysis = ChessApp::parse_pgn_annotations(pgn, &positions);
        assert_eq!(analysis.len(), 3);
        assert_eq!(analysis[1].as_ref().unwrap().eval_cp, Some(25));
        assert_eq!(analysis[1].as_ref().unwrap().depth, 18);
        assert_eq!(analysis[1].as_ref().unwrap().nodes, 1_000_000);
        assert_eq!(
            analysis[0].as_ref().unwrap().best_move.as_deref(),
            Some("e2e4")
        );
        assert_eq!(analysis[0].as_ref().unwrap().pv, "e4 e5 Nf3");
        assert_eq!(analysis[2].as_ref().unwrap().eval_cp, Some(-10));
        assert_eq!(
            analysis[1].as_ref().unwrap().best_move.as_deref(),
            Some("e7e5")
        );
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

    #[test]
    fn distinguishes_forced_mate_outcomes_for_both_sides() {
        assert_eq!(
            MateOutcome::between(None, Some(3), Color::White, false),
            Some(MateOutcome::Found)
        );
        assert_eq!(
            MateOutcome::between(Some(3), None, Color::White, false),
            Some(MateOutcome::Missed)
        );
        assert_eq!(
            MateOutcome::between(None, Some(-2), Color::White, false),
            Some(MateOutcome::Allowed)
        );
        assert_eq!(
            MateOutcome::between(Some(-2), None, Color::White, false),
            Some(MateOutcome::Escaped)
        );
        assert_eq!(
            MateOutcome::between(None, Some(-3), Color::Black, false),
            Some(MateOutcome::Found)
        );
        assert_eq!(
            MateOutcome::between(Some(-3), None, Color::Black, false),
            Some(MateOutcome::Missed)
        );
        assert_eq!(
            MateOutcome::between(None, Some(2), Color::Black, false),
            Some(MateOutcome::Allowed)
        );
        assert_eq!(
            MateOutcome::between(Some(2), None, Color::Black, false),
            Some(MateOutcome::Escaped)
        );
        assert_eq!(
            MateOutcome::between(Some(3), Some(2), Color::White, false),
            None
        );
        assert_eq!(
            MateOutcome::between(Some(-1), Some(0), Color::Black, true),
            None
        );
        assert_eq!(
            MateOutcome::between(None, Some(0), Color::Black, true),
            Some(MateOutcome::Found)
        );
    }

    #[test]
    fn fools_mate_finish_is_not_a_missed_mate() {
        let (positions, moves) = ChessApp::parse_pgn_mainline("1. f3 e5 2. g4 Qh4# 0-1").unwrap();
        assert_eq!(moves.last().map(String::as_str), Some("Qh4#"));
        assert_eq!(positions.last().unwrap().status(), BoardStatus::Checkmate);
        assert_eq!(positions[3].side_to_move(), Color::Black);
        assert_eq!(
            MateOutcome::between(Some(-1), Some(0), Color::Black, true),
            None
        );
    }

    #[test]
    fn graph_score_keeps_terminal_checkmate_on_winning_side() {
        let terminal = PositionAnalysis {
            mate: Some(0),
            ..Default::default()
        };
        let (fools_mate, _) = ChessApp::parse_pgn_mainline("1. f3 e5 2. g4 Qh4# 0-1").unwrap();
        assert_eq!(
            ChessApp::position_score(&terminal, fools_mate.last().unwrap()),
            Some(-10_000)
        );
        let (scholars_mate, _) =
            ChessApp::parse_pgn_mainline("1. e4 e5 2. Qh5 Nc6 3. Bc4 Nf6 4. Qxf7# 1-0").unwrap();
        assert_eq!(
            ChessApp::position_score(&terminal, scholars_mate.last().unwrap()),
            Some(10_000)
        );
    }

    #[test]
    fn move_quality_does_not_overpunish_a_still_winning_position() {
        let still_winning = ChessApp::effective_centipawn_loss(1_000, 470, Color::White);
        let genuine_swing = ChessApp::effective_centipawn_loss(0, -200, Color::White);
        assert!(still_winning < 100);
        assert!(genuine_swing > 100);
        assert_eq!(
            ChessApp::effective_centipawn_loss(-1_000, -470, Color::Black),
            still_winning
        );
    }
}

impl eframe::App for ChessApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.fics_available_was_open = self.fics_available_open_this_frame;
        self.fics_observe_was_open = self.fics_observe_open_this_frame;
        self.fics_available_open_this_frame = false;
        self.fics_observe_open_this_frame = false;
        #[cfg(target_arch = "wasm32")]
        {
            let fen = poll_position_editor();
            if !fen.is_empty() {
                self.new_game_training = false;
                self.new_game_fen = fen;
                self.new_game_position = 2;
                self.new_game_online = false;
                self.new_game_dialog_open = true;
            }
        }
        self.tick_local_clock();
        if self.local_clock.is_some() && !self.fics_active {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        if let Some(contents) = ctx.input(|input| {
            input.raw.dropped_files.iter().find_map(|file| {
                file.bytes
                    .as_ref()
                    .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
            })
        }) {
            self.import_input = contents;
            self.begin_import();
        }
        #[cfg(target_arch = "wasm32")]
        let storage_progress = if self.batch_pgn_total > 0 && !self.batch_pgn_result_open {
            serde_json::from_str::<BatchImportStatus>(&batch_import_storage_status())
                .unwrap_or_default()
        } else {
            BatchImportStatus::default()
        };
        #[cfg(target_arch = "wasm32")]
        if self.batch_pgn_total > 0 && !self.batch_pgn_result_open {
            self.batch_pgn_imported = storage_progress.saved;
            self.batch_pgn_failed += storage_progress
                .failed
                .saturating_sub(self.batch_pgn_storage_failed_seen);
            self.batch_pgn_storage_failed_seen = storage_progress.failed;
            if self.batch_pgn_first_error.is_none() {
                self.batch_pgn_first_error = storage_progress.first_error.clone();
            }
            if storage_progress.failed > 0 && !self.batch_pgn_stopped {
                self.batch_pgn_queue.clear();
                self.batch_pgn_stopped = true;
            }
        }
        #[cfg(target_arch = "wasm32")]
        let can_process_next = storage_progress.pending < 8;
        #[cfg(not(target_arch = "wasm32"))]
        let can_process_next = true;
        let mut processed_this_frame = false;
        if let Some(text) = can_process_next
            .then(|| self.batch_pgn_queue.pop_front())
            .flatten()
        {
            processed_this_frame = true;
            self.batch_pgn_checked += 1;
            #[cfg(target_arch = "wasm32")]
            let duplicate = is_imported_duplicate(&text);
            #[cfg(not(target_arch = "wasm32"))]
            let duplicate = false;
            if duplicate {
                self.batch_pgn_duplicates += 1;
            } else {
                match self.save_imported_pgn_without_opening(&text) {
                    Ok(()) => {
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            self.batch_pgn_imported += 1;
                        }
                    }
                    Err(error) => {
                        self.batch_pgn_failed += 1;
                        if self.batch_pgn_first_error.is_none() {
                            self.batch_pgn_first_error = Some(error);
                        }
                    }
                }
            }
        }
        #[cfg(target_arch = "wasm32")]
        let pending_saves = storage_progress.pending;
        #[cfg(not(target_arch = "wasm32"))]
        let pending_saves = 0;
        if self.batch_pgn_total > 0 && !self.batch_pgn_result_open {
            if self.batch_pgn_queue.is_empty() && pending_saves == 0 && !processed_this_frame {
                self.engine_status = format!(
                    "Imported {} games · open Saved Games to browse",
                    self.batch_pgn_imported
                );
                self.import_input.clear();
                self.batch_pgn_result_open = true;
            } else {
                ctx.request_repaint_after(std::time::Duration::from_millis(50));
            }
        }
        if let Some(index) = self.review_index.or_else(|| {
            (!self.review_positions.is_empty()).then(|| self.review_positions.len() - 1)
        }) && self.best_move_attempt.is_none()
            && !ctx.wants_keyboard_input()
            && !self.pgn_dialog_open
            && !self.strength_dialog_open
            && !self.about_dialog_open
            && !self.new_game_dialog_open
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
            && !self.new_game_dialog_open
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
        #[cfg(target_arch = "wasm32")]
        self.poll_fics(ctx);
        if self.fics_active && !self.fics_playing && !ctx.wants_keyboard_input() {
            let keys = [
                egui::Key::Num1,
                egui::Key::Num2,
                egui::Key::Num3,
                egui::Key::Num4,
                egui::Key::Num5,
                egui::Key::Num6,
                egui::Key::Num7,
                egui::Key::Num8,
                egui::Key::Num9,
                egui::Key::Num0,
            ];
            let selected = ctx.input(|input| keys.iter().position(|key| input.key_pressed(*key)));
            if let Some(game) =
                selected.and_then(|index| self.fics_observed_games.get(index).map(|game| game.id))
            {
                self.show_observed_game(game);
            }
        }
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.style_mut()
                .text_styles
                .insert(egui::TextStyle::Button, FontId::proportional(15.0));
            ui.style_mut()
                .text_styles
                .insert(egui::TextStyle::Body, FontId::proportional(15.0));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.heading("IRONWOOD");
                ui.label(RichText::new("CHESS").color(Color32::from_rgb(207, 172, 93)));
                ui.separator();
                Self::gold_menu_button(ui, "Game", |ui| {
                    Self::set_menu_item_font(ui);
                    if ui.button("New game").clicked() {
                        self.new_game_training = self.training.is_some();
                        self.new_game_online = self.fics_active;
                        self.new_game_both_sides = !self.engine_enabled && !self.fics_active;
                        self.new_game_opponent = self.engine_config.opponent;
                        self.new_game_limit_strength = self.engine_config.limit_strength;
                        self.new_game_elo = self.engine_config.elo.clamp(1320, 3190);
                        self.new_game_color = match self.player_side {
                            PlayerSide::White => NewGameColor::White,
                            PlayerSide::Black => NewGameColor::Black,
                        };
                        self.new_game_dialog_open = true;
                        ui.close();
                    }
                    if ui
                        .add_enabled(
                            !self.fics_active || self.fics_game_finished,
                            egui::Button::new("Save current game"),
                        )
                        .clicked()
                    {
                        self.save_game();
                        self.engine_status = "Game saved on this device".into();
                        ui.close();
                    }
                    if ui.button("Training profiles…").clicked() {
                        self.settle_training();
                        self.training_dialog_open = true;
                        ui.close();
                    }
                    if ui.button("Load saved game…").clicked() {
                        if self.fics_active {
                            self.stop_fics();
                        }
                        #[cfg(target_arch = "wasm32")]
                        open_game_library();
                        ui.close();
                    }
                    ui.separator();
                    let local_playing = !self.fics_active
                        && self.review_index.is_none()
                        && self.pgn_input.trim().is_empty()
                        && !self.history.is_empty()
                        && self.game_result() == "*";
                    let online_playing = self.fics_active
                        && self.fics_playing
                        && !self.fics_observing
                        && !self.fics_game_finished;
                    if ui
                        .add_enabled(
                            local_playing || online_playing,
                            egui::Button::new("Resign…"),
                        )
                        .clicked()
                    {
                        self.fics_resign_dialog_open = true;
                        ui.close();
                    }
                    if ui
                        .add_enabled(local_playing && self.training.is_none(), egui::Button::new("Take back"))
                        .on_disabled_hover_text(
                            "Available during local games after a move has been played",
                        )
                        .clicked()
                    {
                        let undo_reply = self.engine_enabled
                            && self.board.side_to_move() == self.player_side.color()
                            && self.history.len() >= 2;
                        self.undo();
                        if undo_reply {
                            self.undo();
                        }
                        ui.close();
                    }
                    if ui.button("Flip board").clicked() {
                        self.flipped = !self.flipped;
                        self.save_game();
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Import…").clicked() {
                        if self.fics_active {
                            self.stop_fics();
                        }
                        #[cfg(target_arch = "wasm32")]
                        ensure_imported_index();
                        self.import_input.clear();
                        self.pgn_dialog_open = true;
                        self.pgn_error = None;
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
                Self::gold_menu_button(ui, "View", |ui| {
                    Self::set_menu_item_font(ui);
                    if ui.button(if self.board_3d_active { "Show 2D board" } else { "Show 3D board" }).clicked() {
                        self.toggle_board_dimension(ui.ctx());
                        ui.close();
                    }
                    ui.separator();
                    ui.menu_button("Move notation", |ui| {
                        Self::set_menu_item_font(ui);
                        for (figurines, label) in
                            [(false, "Letters (Qf3)"), (true, "Figurines (♕f3)")]
                        {
                            if ui
                                .selectable_value(&mut self.figurine_notation, figurines, label)
                                .changed()
                            {
                                self.save_game();
                                ui.close();
                            }
                        }
                    });
                    ui.menu_button("Board", |ui| {
                        Self::set_menu_item_font(ui);
                        if self.board_3d_active {
                            ui.set_min_width(245.0);
                            ui.menu_button("3D theme", |ui| {
                                Self::set_menu_item_font(ui);
                                for (theme, label) in crate::board3d::Theme::ALL {
                                    if ui.selectable_value(&mut self.board_3d_theme, theme, label).changed() {
                                        self.save_preferences();
                                        ui.close();
                                    }
                                }
                            });
                            let appearance = ui.add(
                                egui::Slider::new(&mut self.board_3d_appearance, 0..=100)
                                    .text("3D appearance"),
                            ).on_hover_text("Adjusts gloss, brightness, and contrast together. 50 is the original look.");
                            if appearance.changed() {
                                self.board_3d_appearance_customized = true;
                                self.save_preferences();
                            }
                            if ui.checkbox(&mut self.show_radial_light, "Show radial light below board").changed() {
                                self.save_preferences();
                            }
                            ui.separator();
                        }
                        if ui.checkbox(&mut self.show_highlighted_move, "Show highlighted move").changed() {
                            self.save_preferences();
                        }
                        if ui
                            .checkbox(&mut self.piece_shadows, "Piece shadows")
                            .changed()
                        {
                            self.save_game();
                        }
                        if ui
                            .checkbox(&mut self.show_board_frame, "Show board frame")
                            .changed()
                        {
                            self.save_game();
                        }
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
                        ui.separator();
                        ui.menu_button("2D Piece set", |ui| {
                            Self::set_menu_item_font(ui);
                            for piece_set in [
                                PieceSet::System,
                                PieceSet::Cburnett,
                                PieceSet::Merida,
                                PieceSet::RoyalRascals,
                                PieceSet::UndeadCourt,
                            ] {
                                if ui
                                    .selectable_value(
                                        &mut self.piece_set,
                                        piece_set,
                                        piece_set.label(),
                                    )
                                    .changed()
                                {
                                    self.save_game();
                                    ui.close();
                                }
                            }
                        });
                    });
                    ui.menu_button("Move feedback", |ui| {
                        Self::set_menu_item_font(ui);
                        if ui
                            .checkbox(&mut self.animate_moves, "Animate live moves")
                            .changed()
                        {
                            self.move_animation = None;
                            self.save_game();
                        }
                        if ui
                            .checkbox(&mut self.move_sounds, "Play move sounds")
                            .changed()
                        {
                            self.save_game();
                            #[cfg(target_arch = "wasm32")]
                            if self.move_sounds {
                                play_chess_sound("move");
                            }
                        }
                    });
                });
                Self::gold_menu_button(ui, "Tools", |ui| {
                    Self::set_menu_item_font(ui);
                    ui.set_min_width(170.0);
                    if ui.button("Elo calculator…").clicked() {
                        self.elo_calculator_open = true;
                        ui.close();
                    }
                    if ui.button("Position Editor…").clicked() {
                        #[cfg(target_arch = "wasm32")]
                        if let Err(error) = open_position_editor(&self.board.to_string()) {
                            self.engine_status =
                                format!("Could not open position editor: {error:?}");
                        }
                        ui.close();
                    }
                });
                Self::gold_menu_button(ui, "Storage", |ui| {
                    Self::set_menu_item_font(ui);
                    if ui.button("Backup all data…").clicked() {
                        #[cfg(target_arch = "wasm32")]
                        backup_storage();
                        ui.close();
                    }
                    if ui.button("Restore backup…").clicked() {
                        #[cfg(target_arch = "wasm32")]
                        restore_storage();
                        ui.close();
                    }
                    if ui.button("Storage information…").clicked() {
                        #[cfg(target_arch = "wasm32")]
                        open_storage_info();
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Clear saved games…").clicked() {
                        #[cfg(target_arch = "wasm32")]
                        clear_saved_games();
                        ui.close();
                    }
                    if ui.button("Reset all local data…").clicked() {
                        #[cfg(target_arch = "wasm32")]
                        reset_local_data();
                        ui.close();
                    }
                });
                Self::gold_menu_button(ui, "Online Controls", |ui| self.fics_online_menu(ui));
                Self::gold_menu_button(ui, "Help", |ui| {
                    Self::set_menu_item_font(ui);
                    if ui.button("Contents…").clicked() {
                        ui.ctx().open_url(egui::OpenUrl::new_tab(HELP_URL));
                        ui.close();
                    }
                    if ui.button("Join our Discord…").clicked() {
                        ui.ctx().open_url(egui::OpenUrl::new_tab(DISCORD_URL));
                        ui.close();
                    }
                    if ui.button("About Ironwood Chess…").clicked() {
                        self.about_dialog_open = true;
                        #[cfg(target_arch = "wasm32")]
                        refresh_version_diagnostics();
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
                    Self::gold_menu_button(ui,
                        if self.fics_connected {
                            "FICS · Connected"
                        } else {
                            "FICS · Offline"
                        },
                        |ui| self.fics_connection_menu(ui),
                    );
                    ui.separator();
                    if self.fics_active && !self.fics_game_finished {
                        ui.label(
                            RichText::new(if self.fics_connected {
                                "● Connected"
                            } else {
                                "● Offline"
                            })
                            .color(if self.fics_connected {
                                Color32::from_rgb(102, 180, 125)
                            } else {
                                Color32::from_rgb(211, 173, 98)
                            }),
                        );
                        ui.separator();
                        ui.label(format!("FICS · {}", self.fics_status));
                        ui.separator();
                        ui.label(RichText::new("Online game · guest · unrated").weak());
                        return;
                    }
                    if self.fics_active {
                        ui.label(
                            RichText::new(if self.fics_connected {
                                "● FICS connected"
                            } else {
                                "● FICS offline"
                            })
                            .color(if self.fics_connected {
                                Color32::from_rgb(102, 180, 125)
                            } else {
                                Color32::from_rgb(211, 173, 98)
                            }),
                        );
                        ui.separator();
                    }
                    let status_lower = self.engine_status.to_ascii_lowercase();
                    let dock_state = if status_lower.contains("failed")
                        || status_lower.contains("error")
                        || status_lower.contains("cross-origin")
                    {
                        "Error"
                    } else if self.analysis_running {
                        "Analyzing"
                    } else if self.review_index.is_some() {
                        "Ready"
                    } else if !self.engine_enabled {
                        "Disabled"
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
                    ui.label(RichText::new(dock_state).strong())
                        .on_hover_text(&self.engine_status);
                    ui.separator();
                    ui.label(
                        if !self.engine_enabled
                            && !self.analysis_running
                            && self.review_index.is_none()
                        {
                            "Play Both Sides"
                        } else {
                            "Stockfish 19 · Full NNUE"
                        },
                    );
                    ui.separator();
                    let strength_label = if !self.engine_enabled
                        && !self.analysis_running
                        && self.review_index.is_none()
                    {
                        "Engine settings".to_owned()
                    } else if self.review_index.is_some() {
                        format!("Analysis · MultiPV {}", self.analysis_config.multipv)
                    } else {
                        format!("Play · {}", Self::strength_summary(&self.engine_config))
                    };
                    let engine_settings_locked =
                        self.game_analysis_running || self.game_analysis_paused || self.training_live();
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
                            ui.add_enabled(
                                !engine_settings_locked,
                                egui::Button::new(RichText::new(strength_label).color(gold))
                                    .frame(true),
                            )
                            .on_hover_text(if engine_settings_locked {
                                "Stop full-game analysis before changing engine settings"
                            } else {
                                "Open Play and Analysis engine settings"
                            })
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
                    let analyzing = self.analysis_running || self.review_index.is_some();
                    let config = if analyzing {
                        &self.analysis_config
                    } else {
                        &self.engine_config
                    };
                    let limit = if !analyzing && !self.fics_active && self.local_clock.is_some() {
                        let clock = self.local_clock.as_ref().unwrap();
                        format!(
                            "Game clock · White {} · Black {} · +{}s",
                            Self::fics_clock(clock.white.ceil() as i32),
                            Self::fics_clock(clock.black.ceil() as i32),
                            clock.increment
                        )
                    } else {
                        match config.search_limit {
                            SearchLimit::Infinite => "unlimited".to_owned(),
                            SearchLimit::Time => format!(
                                "{} ms/{}",
                                config.move_time_ms,
                                if analyzing { "position" } else { "move" }
                            ),
                            SearchLimit::Depth => format!("depth {}", config.depth),
                            SearchLimit::Nodes => format!(
                                "{} nodes/{}",
                                config.nodes,
                                if analyzing { "position" } else { "move" }
                            ),
                        }
                    };
                    if !analyzing && !self.engine_enabled {
                        ui.label("Manual moves · both sides");
                    } else {
                        ui.label(format!(
                            "{} threads · {} MiB hash · {}",
                            config.threads, config.hash_mib, limit
                        ));
                    }
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

        let show_moves = self.compact_panel == CompactPanel::Moves;
        let show_analysis = self.compact_panel == CompactPanel::Analysis;

        egui::SidePanel::right("game_panel")
            .default_width(340.0)
            .min_width(280.0)
            .max_width(640.0)
            .resizable(true)
            .show(ctx, |ui| {
                let panel_bottom = ui.max_rect().bottom();
                egui::ScrollArea::vertical()
                    .id_salt("game_panel_scroll")
                    .max_height((ui.available_height() - if show_analysis { 64.0 } else { 0.0 }).max(0.0))
                    .auto_shrink([false, false])
                    .show_gold(ui, |ui| {
                {
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.compact_panel, CompactPanel::Moves, RichText::new("Game Moves").size(19.0).strong());
                        if !self.fics_active || self.fics_game_finished {
                            ui.add_enabled_ui(!self.training_live(), |ui| {
                                ui.selectable_value(&mut self.compact_panel, CompactPanel::Analysis, RichText::new("Game Analysis").size(19.0).strong());
                            }).response.on_disabled_hover_text("Finish the training game to unlock analysis.");
                        }
                    });
                    ui.separator();
                }
                if show_moves {
                    ui.add_space(6.0);
                    self.training_game_summary_ui(ui);
                    self.game_moves_status_ui(ui);
                } else if show_analysis {
                    ui.add_space(18.0);
                }
                if let Some(index) = self.review_index.or_else(|| {
                    (!self.review_positions.is_empty()).then(|| self.review_positions.len() - 1)
                }) {
                    if show_moves {
                    let mut jump_to = None;
                    let scroll_to_selected = self.review_scroll_to_selected;
                    if self.fics_observation_start_ply.unwrap_or(0) > 0 {
                        ui.label(format!("Moves seen since joining: {}", self.review_moves.len()));
                    } else {
                        ui.label(format!("Position {index} of {}", self.review_moves.len()));
                    }
                    ui.add_space(8.0);
                    Frame::new()
                        .fill(Color32::from_rgb(11, 16, 29))
                        .stroke(Stroke::new(1.0, Color32::from_white_alpha(24)))
                        .corner_radius(CornerRadius::same(6))
                        .inner_margin(Margin::same(8))
                        .show(ui, |ui| {
                            Self::game_moves_header_ui(ui, 78.0);
                            // Leave room for navigation and the frame's bottom margin.
                            let moves_height = (panel_bottom - ui.cursor().top() - 56.0).max(0.0);
                            egui::ScrollArea::vertical()
                                .id_salt("game_moves")
                                .max_height(moves_height)
                                .min_scrolled_height(moves_height)
                                .auto_shrink([false, false])
                                .show_gold(ui, |ui| {
                                    let first_ply = self.fics_observation_start_ply.unwrap_or(self.local_start_ply);
                                    let end_pair = if self.review_moves.is_empty() {
                                        first_ply / 2
                                    } else {
                                        (first_ply + self.review_moves.len()).div_ceil(2)
                                    };
                                    for pair in first_ply / 2..end_pair {
                                        let white_ply = (pair * 2).checked_sub(first_ply);
                                        let black_ply = (pair * 2 + 1).checked_sub(first_ply);
                                        ui.horizontal(|ui| {
                                            let move_number = ui.add_sized(
                                                [30.0, 28.0],
                                                egui::Label::new(format!("{}.", pair + 1)),
                                            );
                                            if pair == first_ply / 2 && index == 0 && scroll_to_selected {
                                                move_number.scroll_to_me(Some(Align::Min));
                                            }
                                            let move_width = ((ui.available_width()
                                                - ui.spacing().item_spacing.x)
                                                / 2.0)
                                                .max(78.0);
                                            if let Some((white_ply, san)) = white_ply
                                                .and_then(|ply| self.review_moves.get(ply).map(|san| (ply, san)))
                                            {
                                                let selected = index == white_ply + 1;
                                                let response = self.analyzed_move_button(
                                                    ui,
                                                    san,
                                                    white_ply + 1,
                                                    selected,
                                                    move_width,
                                                );
                                                self.move_context_response(&response, white_ply + 1);
                                                if response.clicked() || response.secondary_clicked() {
                                                    jump_to = Some(white_ply + 1);
                                                }
                                                if selected && scroll_to_selected {
                                                    response.scroll_to_me(Some(Align::Center));
                                                }
                                            }
                                            if let Some((black_ply, san)) = black_ply
                                                .and_then(|ply| self.review_moves.get(ply).map(|san| (ply, san)))
                                            {
                                                let selected = index == black_ply + 1;
                                                let response = self.analyzed_move_button(
                                                    ui,
                                                    san,
                                                    black_ply + 1,
                                                    selected,
                                                    move_width,
                                                );
                                                self.move_context_response(&response, black_ply + 1);
                                                if response.clicked() || response.secondary_clicked() {
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
                    }
                    if show_analysis {
                    let completed = self
                        .game_analysis
                        .iter()
                        .filter(|item| item.is_some())
                        .count();
                    let total = self.review_positions.len().max(1);
                    let analysis_incomplete = completed < total || self.verification_pending_index().is_some();
                    let progress = if self.game_analysis_running
                        || self.game_analysis_paused
                        || (completed > 0 && analysis_incomplete)
                    {
                        let in_second_pass = !self.verification_targets.is_empty();
                        let second_pass_available = in_second_pass
                            || (completed == total
                                && !self.critical_verification_targets().is_empty()
                                && Self::verification_budget(self.full_game_analysis_config.quality.nodes(
                                    self.full_game_analysis_config.custom_nodes,
                                )) > self.full_game_analysis_config.quality.nodes(
                                    self.full_game_analysis_config.custom_nodes,
                                ));
                        let (fraction, progress_text) = if in_second_pass {
                            let done = self.verification_completed_count();
                            let pass_total = self.verification_targets.len();
                            (0.5 + 0.5 * done as f32 / pass_total as f32,
                                format!("Pass 2 · {done}/{pass_total}"))
                        } else if second_pass_available || completed < total {
                            (0.5 * completed as f32 / total as f32,
                                format!("Pass 1 · {completed}/{total}"))
                        } else {
                            (1.0, format!("{completed}/{total}"))
                        };
                        Some((fraction, progress_text))
                    } else {
                        None
                    };
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Game Analysis").size(18.0).strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            #[cfg(target_arch = "wasm32")]
                            {
                                if self.game_analysis_running {
                                    if ui.button("Stop").clicked() {
                                        self.stop_game_analysis();
                                    }
                                    if ui.button("Pause").clicked() {
                                        self.pause_game_analysis();
                                    }
                                } else if self.game_analysis_paused {
                                    if ui.button("Stop").clicked() {
                                        self.stop_game_analysis();
                                    }
                                    if ui.button("Resume").clicked() {
                                        self.resume_game_analysis();
                                    }
                                } else if analysis_incomplete {
                                    let label = if completed > 0 { "Continue" } else { "Analyze full game" };
                                    if ui.button(label).clicked() {
                                        if self.verification_pending_index().is_some() {
                                            self.resume_verification();
                                        } else if completed > 0 {
                                            self.continue_game_analysis();
                                        } else {
                                            self.start_game_analysis();
                                        }
                                    }
                                } else if ui.button("Analyze again").clicked() {
                                    self.start_game_analysis();
                                }
                            }
                            if let Some((fraction, progress_text)) = &progress {
                                ui.add(
                                    egui::ProgressBar::new(*fraction)
                                        .desired_width(ui.available_width().max(40.0))
                                        .desired_height(24.0)
                                        .text(progress_text),
                                );
                            }
                        });
                    });
                    if self.engine_status.to_ascii_lowercase().contains("failed")
                        || self.engine_status.to_ascii_lowercase().contains("error")
                    {
                        ui.colored_label(Color32::LIGHT_RED, &self.engine_status);
                    }
                    if let Some(destination) = self.analysis_graph(ui, false) {
                        self.review_to(destination);
                    }
                    if self.game_analysis.iter().all(Option::is_some)
                        && !self.game_analysis.is_empty()
                    {
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Analysis Summary").size(17.0).strong());
                            if !self.verification_targets.is_empty()
                                || !self.critical_verification_targets().is_empty()
                            {
                                let verified = self.verification_is_complete();
                                ui.label(RichText::new(if verified { "Critical pass complete" } else { "Provisional" })
                                    .size(12.0)
                                    .color(if verified {
                                        Color32::from_rgb(112, 193, 133)
                                    } else {
                                        Color32::from_rgb(211, 173, 98)
                                    }));
                            }
                        });
                        let white_label = self.review_white_player.clone();
                        let black_label = self.review_black_player.clone();
                        let current_phase = self.game_phase_for_move(index.max(1));
                        let mut phase_jump = None;
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
                                        ui.add_space(5.0);
                                        egui::CollapsingHeader::new("Game phases")
                                            .id_salt(if color == Color::White {
                                                "white_game_phases"
                                            } else {
                                                "black_game_phases"
                                            })
                                            .default_open(false)
                                            .show(ui, |ui| {
                                                for phase in [
                                                    GamePhase::Opening,
                                                    GamePhase::Middlegame,
                                                    GamePhase::Endgame,
                                                ] {
                                                    let stats = self.phase_stats_for_color(phase, color);
                                                    let phase_reached = self.first_move_in_phase(phase).is_some();
                                                    let response = Frame::new()
                                                        .fill(if current_phase == Some(phase) {
                                                            Color32::from_rgb(43, 39, 29)
                                                        } else {
                                                            Color32::from_rgb(24, 28, 33)
                                                        })
                                                        .corner_radius(CornerRadius::same(4))
                                                        .inner_margin(Margin::symmetric(6, 4))
                                                        .show(ui, |ui| {
                                                            ui.set_min_width(ui.available_width());
                                                            ui.label(RichText::new(phase.label()).size(12.0).strong());
                                                            if let Some((moves, accuracy, average)) = stats {
                                                                ui.label(RichText::new(format!("{accuracy:.1}%"))
                                                                    .size(15.0)
                                                                    .strong()
                                                                    .color(Color32::from_rgb(230, 178, 65)));
                                                                let average = average.map_or_else(|| "—".into(), |value| format!("{value:.1}"));
                                                                ui.label(RichText::new(format!("{average} CPL · {moves} moves"))
                                                                    .size(11.0).weak());
                                                            } else {
                                                                ui.label(RichText::new(if phase_reached { "No moves" } else { "Not reached" })
                                                                    .size(12.0).weak());
                                                            }
                                                        })
                                                        .response
                                                        .interact(Sense::click())
                                                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                                                        .on_hover_text(format!(
                                                            "Jump to this player's first move in the {}",
                                                            phase.label().to_lowercase()
                                                        ));
                                                    if stats.is_some() && response.clicked() {
                                                        phase_jump = self.first_move_in_phase_for_color(phase, color);
                                                    }
                                                    ui.add_space(3.0);
                                                }
                                            });
                                    });
                            }
                        });
                        if let Some(destination) = phase_jump {
                            self.review_to(destination);
                        }
                        if let Some(turning_point) = self.turning_point_index() {
                            let loss_text = self.move_mate_outcome(turning_point)
                                .map(|outcome| outcome.explanation().to_owned())
                                .or_else(|| self.move_centipawn_loss(turning_point)
                                    .map(|loss| format!("Largest move-quality drop · {loss} estimated CPL")))
                                .unwrap_or_default();
                            let classification = self
                                .move_classification(turning_point)
                                .unwrap_or(MoveClassification::Good);
                            let mover = self.review_positions[turning_point - 1].side_to_move();
                            let player = if mover == Color::White {
                                self.review_white_player.clone()
                            } else {
                                self.review_black_player.clone()
                            };
                            let san = self
                                .review_moves
                                .get(turning_point - 1)
                                .cloned()
                                .unwrap_or_else(|| "move".to_owned());
                            let move_label = if turning_point % 2 == 1 {
                                format!("{}. {san}", turning_point.div_ceil(2))
                            } else {
                                format!("{}... {san}", turning_point / 2)
                            };
                            ui.add_space(8.0);
                            let response = Frame::new()
                                .fill(Color32::from_rgb(24, 28, 33))
                                .stroke(Stroke::new(1.0, classification.color()))
                                .corner_radius(CornerRadius::same(6))
                                .inner_margin(Margin::same(9))
                                .show(ui, |ui| {
                                    ui.set_min_width(ui.available_width());
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new("Turning point")
                                                .size(15.0)
                                                .strong(),
                                        );
                                        ui.with_layout(
                                            Layout::right_to_left(Align::Center),
                                            |ui| {
                                                ui.label(
                                                    RichText::new(format!(
                                                        "{} {}",
                                                        classification.symbol(),
                                                        classification.label()
                                                    ))
                                                    .color(classification.color()),
                                                );
                                            },
                                        );
                                    });
                                    ui.label(
                                        RichText::new(format!("{player} played {move_label}"))
                                            .size(14.0)
                                            .strong(),
                                    );
                                    ui.label(
                                        RichText::new(loss_text)
                                        .size(13.0)
                                        .weak(),
                                    );
                                })
                                .response
                                .interact(Sense::click())
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .on_hover_text("Jump to this turning point");
                            if response.clicked() {
                                self.review_to(turning_point);
                            }
                        }
                        ui.add_space(7.0);
                        let mut summary_jump = None;
                        egui::CollapsingHeader::new(
                            RichText::new("Move quality").size(14.0).strong(),
                        )
                        .id_salt("move_quality_breakdown")
                        .default_open(false)
                        .show(ui, |ui| {
                        Frame::new()
                            .fill(Color32::from_rgb(24, 28, 33))
                            .corner_radius(CornerRadius::same(5))
                            .inner_margin(Margin::symmetric(8, 6))
                            .show(ui, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(RichText::new("All moves").size(13.0).strong());
                                    for classification in [
                                MoveClassification::Best,
                                MoveClassification::Good,
                                MoveClassification::Inaccuracy,
                                MoveClassification::Mistake,
                                MoveClassification::Blunder,
                                    ] {
                                        let count = self.classification_count(classification);
                                        if ui
                                            .add_enabled(
                                                count > 0,
                                                egui::Button::new(
                                                    RichText::new(format!(
                                                        "{} {count}",
                                                        classification.symbol()
                                                    ))
                                                    .size(12.0)
                                                    .color(classification.color()),
                                                )
                                                .stroke(Stroke::new(
                                                    1.0,
                                                    classification.color(),
                                                )),
                                            )
                                            .on_hover_text(format!(
                                                "All moves: {} {} — jump to the next matching move",
                                                count,
                                                classification.label().to_lowercase()
                                            ))
                                            .clicked()
                                        {
                                            summary_jump =
                                                self.next_classified_move(classification, index);
                                        }
                                    }
                                });
                            });
                        ui.add_space(8.0);
                        ui.label(RichText::new("By player").size(13.0).strong().weak());
                        for (color, player) in [
                            (Color::White, self.review_white_player.clone()),
                            (Color::Black, self.review_black_player.clone()),
                        ] {
                            Frame::new()
                                .fill(Color32::from_rgb(24, 28, 33))
                                .corner_radius(CornerRadius::same(5))
                                .inner_margin(Margin::symmetric(8, 6))
                                .show(ui, |ui| {
                                    ui.set_min_width(ui.available_width());
                                    ui.label(RichText::new(&player).size(14.0).strong());
                                    ui.add_space(4.0);
                                    ui.horizontal_wrapped(|ui| {
                                        ui.spacing_mut().item_spacing.x = 5.0;
                                        for classification in [
                                            MoveClassification::Best,
                                            MoveClassification::Good,
                                            MoveClassification::Inaccuracy,
                                            MoveClassification::Mistake,
                                            MoveClassification::Blunder,
                                        ] {
                                            let count = self.classification_count_for_color(
                                                classification,
                                                color,
                                            );
                                            if ui
                                                .add_enabled(
                                                    count > 0,
                                                    egui::Button::new(
                                                        RichText::new(format!(
                                                            "{} {count}",
                                                            classification.symbol()
                                                        ))
                                                        .size(12.0)
                                                        .color(classification.color()),
                                                    )
                                                    .stroke(Stroke::new(
                                                        1.0,
                                                        classification.color(),
                                                    )),
                                                )
                                                .on_hover_text(format!(
                                                    "{}: {} {} — jump to the next matching move",
                                                    player,
                                                    count,
                                                    classification.label().to_lowercase()
                                                ))
                                                .clicked()
                                            {
                                                summary_jump = self
                                                    .next_classified_move_for_color(
                                                        classification,
                                                        color,
                                                        index,
                                                    );
                                            }
                                        }
                                    });
                                });
                            ui.add_space(4.0);
                        }
                        if MateOutcome::ALL
                            .iter()
                            .any(|outcome| self.mate_outcome_count(*outcome) > 0)
                        {
                            ui.add_space(8.0);
                            ui.label(RichText::new("Forced mate").size(13.0).strong().weak());
                            Frame::new()
                                .fill(Color32::from_rgb(24, 28, 33))
                                .corner_radius(CornerRadius::same(5))
                                .inner_margin(Margin::symmetric(8, 6))
                                .show(ui, |ui| {
                                    ui.horizontal_wrapped(|ui| {
                                        for outcome in MateOutcome::ALL {
                                            let count = self.mate_outcome_count(outcome);
                                            if count == 0 {
                                                continue;
                                            }
                                            if ui
                                                .add(
                                                    egui::Button::new(
                                                        RichText::new(format!(
                                                            "{} · {count}",
                                                            outcome.label()
                                                        ))
                                                        .size(12.0)
                                                        .color(outcome.color()),
                                                    )
                                                    .stroke(Stroke::new(1.0, outcome.color())),
                                                )
                                                .on_hover_text(format!(
                                                    "Jump to the next move where {}",
                                                    outcome.label().to_lowercase()
                                                ))
                                                .clicked()
                                            {
                                                summary_jump = self.next_mate_outcome(outcome, index);
                                            }
                                        }
                                    });
                                });
                        }
                        })
                        .header_response
                        .on_hover_text("Quality uses estimated winning chances and Stockfish's recommended move. CPL is estimated from the position evaluations; a move matching Stockfish's recommendation has 0 CPL, even if a later search changes the score.");
                        let critical_moves = self.critical_move_indices();
                        if !critical_moves.is_empty() && self.best_move_attempt.is_some() {
                            ui.add_space(8.0);
                            Frame::new()
                                .fill(Color32::from_rgb(24, 28, 33))
                                .stroke(Stroke::new(
                                    1.0,
                                    Color32::from_rgb(211, 173, 98),
                                ))
                                .corner_radius(CornerRadius::same(6))
                                .inner_margin(Margin::same(9))
                                .show(ui, |ui| {
                                    ui.set_min_width(ui.available_width());
                                    if let Some(attempt) = self.best_move_attempt {
                                        let root_index = attempt.target_index.saturating_sub(1);
                                        let root = self.review_positions[root_index];
                                        let best_san = self.display_san(&Self::san_for_move(
                                            &root,
                                            attempt.best_move,
                                        ));
                                        ui.label(
                                            RichText::new("Find the best move")
                                                .size(17.0)
                                                .strong(),
                                        );
                                        ui.label(
                                            RichText::new(format!(
                                                "Position before move {} · play directly on the board",
                                                attempt.target_index
                                            ))
                                            .size(13.0)
                                            .weak(),
                                        );
                                        ui.add_space(6.0);
                                        if attempt.revealed {
                                            ui.label(
                                                RichText::new(format!("Best move: {best_san}"))
                                                    .size(16.0)
                                                    .strong()
                                                    .color(Color32::from_rgb(106, 201, 126)),
                                            );
                                            if let Some(line) = self
                                                .game_analysis
                                                .get(root_index)
                                                .and_then(Option::as_ref)
                                                .map(|analysis| analysis.pv.as_str())
                                                .filter(|line| !line.is_empty())
                                            {
                                                ui.label(
                                                    RichText::new(self.display_san_line(line))
                                                        .size(13.0)
                                                        .weak(),
                                                );
                                            }
                                            ui.horizontal(|ui| {
                                                if ui.button("Try again").clicked() {
                                                    self.reset_best_move_attempt();
                                                }
                                                if ui.button("Back to review").clicked() {
                                                    self.best_move_attempt = None;
                                                    summary_jump = Some(attempt.target_index);
                                                }
                                            });
                                        } else if let Some(attempted_move) = attempt.attempted_move {
                                            let attempted_san =
                                                self.display_san(&Self::san_for_move(
                                                    &root,
                                                    attempted_move,
                                                ));
                                            if attempted_move == attempt.best_move {
                                                ui.label(
                                                    RichText::new(format!(
                                                        "Best move found: {attempted_san}"
                                                    ))
                                                    .size(16.0)
                                                    .strong()
                                                    .color(Color32::from_rgb(106, 201, 126)),
                                                );
                                            } else {
                                                ui.label(
                                                    RichText::new(format!(
                                                        "You played {attempted_san}. Stockfish prefers another move."
                                                    ))
                                                    .size(15.0)
                                                    .color(Color32::from_rgb(224, 190, 92)),
                                                );
                                            }
                                            ui.horizontal(|ui| {
                                                if ui.button("Try again").clicked() {
                                                    self.reset_best_move_attempt();
                                                }
                                                if ui.button("Show solution").clicked() {
                                                    self.reveal_best_move_attempt();
                                                }
                                            });
                                        } else {
                                            ui.label(
                                                RichText::new(
                                                    "Stockfish's recommendation is hidden until you move or reveal it.",
                                                )
                                                .size(14.0),
                                            );
                                            if ui.button("Cancel exercise").clicked() {
                                                self.best_move_attempt = None;
                                                summary_jump = Some(attempt.target_index);
                                            }
                                        }
                                    }
                                });
                        }
                        if !critical_moves.is_empty()
                            && self.best_move_attempt.is_none()
                            && index == 0
                            && ui.button("Start critical review").clicked()
                        {
                            summary_jump = critical_moves.first().copied();
                        }
                        if self.best_move_attempt.is_none()
                            && index > 0
                            && let Some(classification) = self.move_classification(index)
                        {
                            let loss_text = self.move_mate_outcome(index)
                                .map(|outcome| outcome.label().to_owned())
                                .or_else(|| self.move_centipawn_loss(index).map(|loss| format!("{loss} CPL")))
                                .unwrap_or_default();
                            let root_index = index - 1;
                            let root = self.review_positions[root_index];
                            let before = self.game_analysis[root_index].clone();
                            let after = self.game_analysis[index].clone();
                            let played = self
                                .review_moves
                                .get(root_index)
                                .cloned()
                                .unwrap_or_else(|| "-".to_owned());
                            let best_move = before
                                .as_ref()
                                .and_then(|analysis| analysis.best_move.as_deref())
                                .and_then(Self::parse_uci_value);
                            let best_san = best_move
                                .map(|chess_move| Self::san_for_move(&root, chess_move))
                                .unwrap_or_else(|| "-".to_owned());
                            let actual_line = self.actual_continuation(index, 6);
                            let recommended_line = before
                                .as_ref()
                                .map(|analysis| analysis.pv.clone())
                                .unwrap_or_default();
                            let played_display = self.display_san(&played);
                            let best_san_display = self.display_san(&best_san);
                            let actual_line_display = self.display_san_line(&actual_line);
                            let recommended_line_display =
                                self.display_san_line(&recommended_line);
                            let review_position = critical_moves.iter().position(|move_index| *move_index == index);
                            let mut try_best_move = false;
                            let mut step_best_line = false;
                            ui.add_space(6.0);
                            Frame::new()
                                .fill(Color32::from_rgb(24, 28, 33))
                                .stroke(Stroke::new(1.0, classification.color()))
                                .corner_radius(CornerRadius::same(5))
                                .inner_margin(Margin::same(8))
                                .show(ui, |ui| {
                                    ui.set_min_width(ui.available_width());
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label(
                                            RichText::new("Move Analysis")
                                                .size(16.0)
                                                .strong(),
                                        );
                                        if let Some(position) = review_position {
                                            ui.label(
                                                RichText::new(format!(
                                                    "{} of {}",
                                                    position + 1,
                                                    critical_moves.len()
                                                ))
                                                .size(12.0)
                                                .color(Color32::from_rgb(211, 173, 98)),
                                            );
                                        }
                                        ui.label(
                                            RichText::new(format!(
                                                "{} {} · {loss_text}",
                                                classification.symbol(),
                                                classification.label()
                                            ))
                                            .strong()
                                            .color(classification.color()),
                                        );
                                    });
                                    ui.add_space(5.0);
                                    ui.horizontal_wrapped(|ui| {
                                        if let Some(position) = review_position {
                                            if ui
                                                .add_enabled(
                                                    position > 0,
                                                    egui::Button::new("◀ Previous"),
                                                )
                                                .clicked()
                                            {
                                                summary_jump = Some(critical_moves[position - 1]);
                                            }
                                            if ui
                                                .add_enabled(
                                                    position + 1 < critical_moves.len(),
                                                    egui::Button::new("Next ▶"),
                                                )
                                                .clicked()
                                            {
                                                summary_jump = Some(critical_moves[position + 1]);
                                            }
                                        }
                                        try_best_move = ui
                                            .add_enabled(
                                                best_move.is_some(),
                                                egui::Button::new("Try the best move"),
                                            )
                                            .clicked();
                                    });
                                    ui.add_space(5.0);
                                    ui.columns(2, |columns| {
                                        for (column, (label, analysis)) in columns.iter_mut().zip([
                                            ("Before", before.as_ref()),
                                            ("After", after.as_ref()),
                                        ]) {
                                            Frame::new()
                                                .fill(Color32::from_rgb(18, 21, 25))
                                                .corner_radius(CornerRadius::same(4))
                                                .inner_margin(Margin::same(7))
                                                .show(column, |ui| {
                                                    ui.label(
                                                        RichText::new(label).size(11.5).weak(),
                                                    );
                                                    ui.label(
                                                        RichText::new(Self::report_evaluation(
                                                            analysis,
                                                        ))
                                                        .size(17.0)
                                                        .strong()
                                                        .color(Color32::from_rgb(230, 178, 65)),
                                                    );
                                                });
                                        }
                                    });
                                    ui.add_space(7.0);
                                    ui.label(
                                        RichText::new(format!("Played: {played_display}"))
                                            .size(14.0)
                                            .strong(),
                                    );
                                    ui.label(
                                        RichText::new(format!(
                                            "Stockfish preferred: {best_san_display}"
                                        ))
                                            .size(14.0)
                                            .strong()
                                            .color(Color32::from_rgb(106, 201, 126)),
                                    );
                                    if let Some(explanation) = self.move_explanation(index) {
                                        ui.label(RichText::new(explanation).size(14.0));
                                    }
                                    if !actual_line.is_empty() {
                                        ui.add_space(7.0);
                                        ui.label(
                                            RichText::new("WHAT HAPPENED")
                                                .size(11.0)
                                                .strong()
                                                .weak(),
                                        );
                                        ui.label(
                                            RichText::new(&actual_line_display).size(13.0),
                                        );
                                    }
                                    if !recommended_line.is_empty() {
                                        ui.add_space(7.0);
                                        ui.label(
                                            RichText::new("STOCKFISH CONTINUATION")
                                                .size(11.0)
                                                .strong()
                                                .weak(),
                                        );
                                        ui.label(
                                            RichText::new(&recommended_line_display)
                                                .size(13.0)
                                                .weak(),
                                        );
                                    }
                                    ui.add_space(7.0);
                                    ui.horizontal_wrapped(|ui| {
                                        if review_position.is_none()
                                            && !critical_moves.is_empty()
                                            && ui.button("Start critical review").clicked()
                                        {
                                            summary_jump = critical_moves.first().copied();
                                        }
                                        step_best_line = ui
                                            .add_enabled(
                                                !recommended_line.is_empty(),
                                                egui::Button::new("Step through best line"),
                                            )
                                            .on_hover_text(
                                                "Load the saved engine line into move prediction",
                                            )
                                            .clicked();
                                    });
                                });
                            if try_best_move {
                                self.start_best_move_attempt(index);
                            } else if step_best_line {
                                self.load_stored_prediction(root_index);
                            }
                        }
                        if let Some(destination) = summary_jump {
                            self.review_to(destination);
                        }
                    }
                    ui.add_space(12.0);
                    }
                }
                if show_analysis { ui.add_space(18.0); }
                if self.review_positions.is_empty() && show_moves {
                    ui.label(RichText::new("Moves").strong());
                    if self.fics_active {
                        ui.label("FICS moves will appear when a game starts.");
                    } else {
                        ui.label(format!("{} half-moves played", self.history.len()));
                        ui.label(RichText::new("Saved locally").small().weak());
                    }
                } else if self.review_positions.is_empty() && show_analysis {
                    ui.label(RichText::new("Game Analysis").size(17.0).strong());
                    ui.label(RichText::new("Play or import a game to analyze it.").weak());
                }
                    });
                if show_analysis {
                    ui.add_space(8.0);
                    ui.separator();
                    let enabled = !self.review_moves.is_empty()
                        && self.game_analysis.iter().any(Option::is_some)
                        && !self.game_analysis_running;
                    let reason = if self.game_analysis_running {
                        "Pause or stop game analysis or verification before printing."
                    } else { "Analyze at least one position before printing." };
                    let gold = Color32::from_rgb(211, 173, 98);
                    ui.scope(|ui| {
                        ui.visuals_mut().widgets.hovered.weak_bg_fill = Color32::from_rgb(232, 195, 123);
                        ui.visuals_mut().widgets.active.weak_bg_fill = Color32::from_rgb(189, 151, 76);
                        if ui.add_enabled(enabled, egui::Button::new(
                            RichText::new("Print analysis report…").size(16.0).strong().color(Color32::from_rgb(25, 29, 35)))
                            .fill(gold)
                            .stroke(Stroke::new(1.0, Color32::from_rgb(241, 206, 131)))
                            .corner_radius(CornerRadius::same(5))
                            .min_size(Vec2::new(ui.available_width(), 44.0)))
                            .on_disabled_hover_text(reason)
                            .on_hover_text("Open a critical-positions report to print or save as PDF")
                            .clicked() {
                            self.print_confirm_open = true;
                        }
                    });
                }
            });

        let dock_height = if self.engine_analysis_dock_collapsed {
            40.0
        } else {
            190.0
        };
        if !self.fics_active || self.fics_game_finished {
            egui::TopBottomPanel::bottom("engine_analysis_dock")
                .exact_height(dock_height)
                .frame(
                    Frame::new()
                        .fill(Color32::from_rgb(18, 21, 25))
                        .stroke(Stroke::new(1.0, Color32::from_white_alpha(28)))
                        .inner_margin(Margin::same(10)),
                )
                .show(ctx, |ui| self.engine_analysis_dock_ui(ui));
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.centered_and_justified(|ui| self.board_ui(ui));
        });

        if let Some(until) = self.print_notice_until {
            if Self::animation_time() < until {
                let mut dismiss = false;
                egui::Area::new(egui::Id::new("print_report_notice"))
                    .order(egui::Order::Foreground)
                    .anchor(egui::Align2::CENTER_TOP, Vec2::new(0.0, 48.0))
                    .show(ctx, |ui| {
                        Frame::new()
                            .fill(Color32::from_rgb(43, 39, 29))
                            .stroke(Stroke::new(1.5, Color32::from_rgb(211, 173, 98)))
                            .corner_radius(CornerRadius::same(7))
                            .inner_margin(Margin::symmetric(14, 10))
                            .show(ui, |ui| {
                                ui.set_max_width(560.0);
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new("Print preview is opening")
                                            .strong()
                                            .color(Color32::from_rgb(230, 178, 65)),
                                    );
                                    ui.label(
                                        "Edge may pause Ironwood until you finish printing or close the print window.",
                                    );
                                    dismiss = ui
                                        .button("×")
                                        .on_hover_text("Dismiss this notice")
                                        .clicked();
                                });
                            });
                    });
                if dismiss {
                    self.print_notice_until = None;
                } else {
                    ctx.request_repaint_after(std::time::Duration::from_millis(250));
                }
            } else {
                self.print_notice_until = None;
            }
        }

        self.expanded_analysis_graph(ctx);
        self.draw_window(ctx);
        self.notes_window(ctx);
        self.move_note_dialog(ctx);
        self.tactical_window(ctx);

        if self.print_confirm_open {
            let mut open = true;
            let mut print = false;
            let width = (ctx.screen_rect().width() - 64.0).clamp(260.0, 520.0);
            egui::Window::new(RichText::new("Open print preview?").size(24.0).strong())
                .id(egui::Id::new("print_preview_confirmation"))
                .collapsible(false)
                .resizable(false)
                .frame(Self::dialog_frame())
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .title_bar(false)
                .show(ctx, |ui| {
                    ui.set_width(width);
                    if Self::dialog_header(ui, "Open print preview?") { open = false; }
                    ui.add_space(8.0);
                    ui.label(RichText::new("Print your analysis or save it as a PDF.").size(19.0).strong());
                    ui.add_space(12.0);
                    ui.label(RichText::new(
                        "Your browser may pause Ironwood while the print window is open. Finish printing or close that window to return to the game."
                    ).size(17.0));
                    ui.add_space(22.0);
                    if ui.add_sized([width, 48.0],
                        egui::Button::new(RichText::new("Open print preview").size(18.0).strong()
                            .color(Color32::from_rgb(25, 29, 35)))
                            .fill(Color32::from_rgb(211, 173, 98)))
                        .on_hover_text("Open the analysis report and system print window")
                        .clicked() {
                        print = true;
                    }
                });
            self.print_confirm_open = open && !print;
            if print {
                #[cfg(target_arch = "wasm32")]
                {
                    self.print_notice_until = Some(Self::animation_time() + 8.0);
                    self.engine_status = match self.print_analysis_report() {
                        Ok(()) => "Analysis report opened · choose Save as PDF".into(),
                        Err(_) => {
                            self.print_notice_until = None;
                            "Analysis report window was blocked".into()
                        }
                    };
                }
            }
        }

        if self.fics_challenge_open {
            let mut open = true;
            let mut sent = false;
            egui::Window::new("Challenge a player")
                .title_bar(false)
                .resizable(false)
                .collapsible(false)
                .default_width(390.0)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                 .frame(Self::dialog_frame())
                .show(ctx, |ui| {
                    ui.set_min_width(380.0);
                    if Self::dialog_header(ui, "Challenge a player") { open = false; }
                    Frame::new()
                        .inner_margin(Margin::symmetric(16, 14))
                        .show(ui, |ui| {
                            ui.label(RichText::new("FICS handle").size(13.0).strong());
                            ui.add_sized(
                                [ui.available_width(), 30.0],
                                egui::TextEdit::singleline(&mut self.fics_player)
                                    .hint_text("Player to challenge"),
                            );
                            ui.add_space(12.0);
                            ui.horizontal(|ui| {
                                ui.label("Minutes");
                                ui.add(egui::DragValue::new(&mut self.fics_minutes).range(1..=60));
                                ui.add_space(12.0);
                                ui.label("Increment");
                                ui.add(
                                    egui::DragValue::new(&mut self.fics_increment).range(0..=60),
                                );
                            });
                            ui.add_space(6.0);
                            ui.label(RichText::new("Unrated game").size(12.0).weak());
                            if !self.fics_challenge_error.is_empty() {
                                ui.add_space(8.0);
                                ui.colored_label(
                                    Color32::from_rgb(225, 137, 126),
                                    &self.fics_challenge_error,
                                );
                            }
                            ui.add_space(16.0);
                            if ui
                                .add_sized(
                                    [ui.available_width(), 36.0],
                                    egui::Button::new(
                                        RichText::new("Send challenge")
                                            .size(14.0)
                                            .strong()
                                            .color(Color32::from_rgb(25, 29, 35)),
                                    )
                                    .fill(Color32::from_rgb(211, 173, 98)),
                                )
                                .clicked()
                            {
                                if !self.fics_connected || self.fics_playing {
                                    self.fics_challenge_error =
                                        "Connect to FICS before sending a challenge.".into();
                                } else if self.fics_player.len() > 20
                                    || !self
                                        .fics_player
                                        .starts_with(|c: char| c.is_ascii_alphabetic())
                                    || !self
                                        .fics_player
                                        .chars()
                                        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
                                {
                                    self.fics_challenge_error =
                                        "Enter a valid FICS handle (up to 20 characters).".into();
                                } else {
                                    #[cfg(target_arch = "wasm32")]
                                    fics_send(&format!(
                                        "match {} unrated {} {}",
                                        self.fics_player, self.fics_minutes, self.fics_increment
                                    ));
                                    self.fics_challenge_error.clear();
                                    sent = true;
                                }
                            }
                        });
                });
            self.fics_challenge_open = open && !sent;
            if !open {
                self.fics_challenge_error.clear();
            }
        }

        if self.fics_sign_in_open {
            let mut open = true;
            let mut signed_in = false;
            egui::Window::new("FICS account")
                .title_bar(false)
                .resizable(false)
                .collapsible(false)
                .default_width(390.0)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                 .frame(Self::dialog_frame())
                .show(ctx, |ui| {
                    ui.set_min_width(380.0);
                    if Self::dialog_header(ui, "FICS account") { open = false; }
                    Frame::new()
                        .inner_margin(Margin::symmetric(16, 14))
                        .show(ui, |ui| {
                            ui.label(RichText::new("Handle").size(13.0).strong());
                            ui.add_sized(
                                [ui.available_width(), 30.0],
                                egui::TextEdit::singleline(&mut self.fics_username)
                                    .hint_text("Your FICS handle"),
                            );
                            ui.add_space(10.0);
                            ui.label(RichText::new("Password").size(13.0).strong());
                            ui.add_sized(
                                [ui.available_width(), 30.0],
                                egui::TextEdit::singleline(&mut self.fics_password)
                                    .password(true)
                                    .hint_text("Your FICS password"),
                            );
                            ui.add_space(14.0);
                            Frame::new()
                                .fill(Color32::from_rgb(48, 40, 27))
                                .stroke(Stroke::new(1.0, Color32::from_rgb(105, 82, 43)))
                                .corner_radius(CornerRadius::same(6))
                                .inner_margin(Margin::same(10))
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.label(
                                        RichText::new("FICS uses an unencrypted connection beyond Ironwood. Use a password unique to FICS.")
                                            .size(12.0)
                                            .color(Color32::from_rgb(226, 192, 128)),
                                    );
                                });
                            if !self.fics_sign_in_error.is_empty() {
                                ui.add_space(8.0);
                                ui.colored_label(Color32::from_rgb(225, 137, 126), &self.fics_sign_in_error);
                            }
                            ui.add_space(16.0);
                            if ui.add_sized(
                                [ui.available_width(), 36.0],
                                egui::Button::new(
                                    RichText::new("Sign in to FICS")
                                        .size(14.0)
                                        .strong()
                                        .color(Color32::from_rgb(25, 29, 35)),
                                )
                                .fill(Color32::from_rgb(211, 173, 98)),
                            ).clicked() {
                                if self.fics_username.len() < 3
                                    || self.fics_username.len() > 17
                                    || !self.fics_username.chars().all(|c| c.is_ascii_alphabetic())
                                    || self.fics_password.is_empty()
                                    || self.fics_password.len() > 128
                                    || !self.fics_password.bytes().all(|c| (33..=126).contains(&c))
                                {
                                    self.fics_sign_in_error = "Enter a 3–17 letter handle and a valid password.".into();
                                } else {
                                    #[cfg(target_arch = "wasm32")]
                                    fics_connect_registered(&self.fics_username, &self.fics_password);
                                    self.fics_active = true;
                                    self.engine_enabled = false;
                                    self.engine_searching = false;
                                    self.analysis_running = false;
                                    self.local_clock = None;
                                    self.local_resigned_white = None;
                                    #[cfg(target_arch = "wasm32")]
                                    if let Some(engine) = &self.engine { engine.command("stop"); }
                                    self.fics_password.clear();
                                    self.fics_sign_in_error.clear();
                                    self.fics_connected = false;
                                    self.fics_registered = false;
                                    self.fics_ads.clear();
                                    self.fics_log.clear();
                                    self.fics_chats = crate::fics_chat::Chats::default();
                                    self.fics_status = format!("Connecting as {}…", self.fics_username);
                                    signed_in = true;
                                }
                            }
                            ui.add_space(10.0);
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("New to FICS?").size(12.0).weak());
                                ui.hyperlink_to("Create an account", "https://www.freechess.org/Register/");
                            });
                        });
                });
            self.fics_sign_in_open = open && !signed_in;
            if !open {
                self.fics_password.clear();
                self.fics_sign_in_error.clear();
            }
        }

        if self.fics_console_open {
            let mut open = self.fics_console_open;
            egui::Window::new("FICS console")
                .title_bar(false)
                .resizable(true)
                .default_size(Vec2::new(560.0, 340.0))
                .min_size(Vec2::new(360.0, 180.0))
                .default_pos(egui::pos2(80.0, 90.0))
                 .frame(Self::dialog_frame())
                .show(ctx, |ui| {
                    if Self::dialog_header(ui, "FICS console") { open = false; }
                    ui.label(RichText::new(&self.fics_status).weak());
                    if self.fics_chat_tabs(ui) {
                        return;
                    }
                    ui.separator();
                    let messages_height = (ui.available_height() - 60.0).max(90.0);
                    egui::ScrollArea::both()
                        .id_salt("fics_console_messages")
                        .stick_to_bottom(true)
                        .max_height(messages_height)
                        .min_scrolled_height(messages_height)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            let transcript = self.fics_log.join("\n");
                            let mut transcript_view = transcript.as_str();
                            let mut layouter =
                                |ui: &egui::Ui, text: &dyn egui::TextBuffer, _: f32| {
                                    let mut job = egui::text::LayoutJob::default();
                                    job.wrap.max_width = f32::INFINITY;
                                    for (index, line) in text.as_str().split('\n').enumerate() {
                                        let format = egui::TextFormat {
                                            font_id: FontId::monospace(13.0),
                                            color: FicsConsoleTone::for_line(line).color(),
                                            ..Default::default()
                                        };
                                        if index > 0 {
                                            job.append("\n", 0.0, format.clone());
                                        }
                                        job.append(line, 0.0, format);
                                    }
                                    ui.fonts(|fonts| fonts.layout_job(job))
                                };
                            let output = egui::TextEdit::multiline(&mut transcript_view)
                                .id_salt("fics_console_transcript")
                                .font(FontId::monospace(13.0))
                                .frame(false)
                                .desired_width(ui.available_width())
                                .layouter(&mut layouter)
                                .show(ui);
                            if let Some(range) = output.state.cursor.char_range() {
                                let range = range.as_sorted_char_range();
                                if range.start < range.end {
                                    self.fics_console_selected_text = transcript
                                        .chars()
                                        .skip(range.start)
                                        .take(range.end - range.start)
                                        .collect();
                                } else if output.response.clicked() {
                                    self.fics_console_selected_text.clear();
                                }
                            }
                            egui::Popup::context_menu(&output.response)
                                .style(Self::gold_menu_style)
                                .frame(Self::context_menu_frame(&output.response.ctx))
                                .show(|ui| {
                                    Self::style_context_menu(ui);
                                    if ui
                                        .add_enabled(
                                            !self.fics_console_selected_text.is_empty(),
                                            egui::Button::new("Copy selected text"),
                                        )
                                        .clicked()
                                    {
                                        ui.ctx().copy_text(self.fics_console_selected_text.clone());
                                        ui.close();
                                    }
                                });
                        });
                    ui.separator();
                    let mut command_field_rect = None;
                    let mut command_list_left = None;
                    let mut scroll_to_suggestion = false;
                    ui.horizontal(|ui| {
                        let help = ui
                            .add_enabled(
                                self.fics_connected,
                                egui::Button::new(
                                    RichText::new("?")
                                        .color(Color32::from_rgb(226, 181, 83))
                                        .strong(),
                                )
                                .min_size(Vec2::new(24.0, 20.0)),
                            )
                            .on_hover_text("Show useful FICS commands");
                        command_list_left = Some(help.rect.left());
                        let response = ui.add_enabled(
                            self.fics_connected,
                            egui::TextEdit::singleline(&mut self.fics_console_input)
                                .font(FontId::monospace(13.0))
                                .hint_text("FICS command")
                                .desired_width(ui.available_width() - 65.0),
                        );
                        command_field_rect = Some(response.rect);
                        if help.clicked() {
                            self.fics_console_suggestions_open =
                                !self.fics_console_suggestions_open;
                            if self.fics_console_suggestions_open {
                                self.fics_console_suggestion_index = 0;
                                scroll_to_suggestion = true;
                                ui.memory_mut(|memory| memory.request_focus(response.id));
                            }
                        }
                        if self.fics_console_suggestions_open
                            && response.has_focus()
                            && ui.input(|input| input.key_pressed(egui::Key::ArrowDown))
                        {
                            self.fics_console_suggestion_index =
                                (self.fics_console_suggestion_index + 1)
                                    % FICS_CONSOLE_COMMANDS.len();
                            scroll_to_suggestion = true;
                        }
                        if self.fics_console_suggestions_open
                            && response.has_focus()
                            && ui.input(|input| input.key_pressed(egui::Key::ArrowUp))
                        {
                            self.fics_console_suggestion_index =
                                (self.fics_console_suggestion_index + FICS_CONSOLE_COMMANDS.len()
                                    - 1)
                                    % FICS_CONSOLE_COMMANDS.len();
                            scroll_to_suggestion = true;
                        }
                        if self.fics_console_suggestions_open
                            && ui.input(|input| input.key_pressed(egui::Key::Escape))
                        {
                            self.fics_console_suggestions_open = false;
                        }
                        let send_clicked = ui
                            .add_enabled(self.fics_connected, egui::Button::new("Send"))
                            .clicked();
                        let enter = (response.has_focus() || response.lost_focus())
                            && ui.input(|input| input.key_pressed(egui::Key::Enter));
                        if self.fics_console_suggestions_open && enter && !send_clicked {
                            self.fics_console_input = FICS_CONSOLE_COMMANDS
                                [self.fics_console_suggestion_index]
                                .0
                                .to_owned();
                            self.fics_console_suggestions_open = false;
                            ui.memory_mut(|memory| memory.request_focus(response.id));
                        } else if enter || send_clicked {
                            let command = self.fics_console_input.trim();
                            if !command.is_empty()
                                && command.len() <= 256
                                && command.bytes().all(|byte| (32..=126).contains(&byte))
                            {
                                #[cfg(target_arch = "wasm32")]
                                fics_send_command(command);
                                self.fics_log.push(format!("> {command}"));
                                if self.fics_log.len() > 80 {
                                    self.fics_log.remove(0);
                                }
                                self.fics_console_input.clear();
                                self.fics_console_suggestions_open = false;
                            }
                        }
                    });
                    if self.fics_console_suggestions_open && self.fics_connected {
                        if let Some(field_rect) = command_field_rect {
                            let below = ctx.available_rect().bottom() - field_rect.bottom();
                            let top = if below >= 180.0 {
                                field_rect.bottom()
                            } else {
                                field_rect.top() - 180.0
                            };
                            egui::Area::new(egui::Id::new("fics_command_suggestions"))
                                .order(egui::Order::Foreground)
                                .fixed_pos(egui::pos2(
                                    command_list_left.unwrap_or(field_rect.left()),
                                    top,
                                ))
                                .show(ctx, |ui| {
                                    Frame::new()
                                        .fill(Color32::from_rgb(28, 34, 39))
                                        .stroke(Stroke::new(1.0, Color32::from_white_alpha(36)))
                                        .inner_margin(Margin::same(6))
                                        .show(ui, |ui| {
                                            ui.set_width(field_rect.width().min(320.0));
                                            egui::ScrollArea::vertical()
                                                .id_salt("fics_command_suggestions_list")
                                                .min_scrolled_width(field_rect.width().min(320.0))
                                                .max_height(140.0)
                                                .auto_shrink([false, true])
                                                .show(ui, |ui| {
                                                    for (index, (command, description)) in
                                                        FICS_CONSOLE_COMMANDS.iter().enumerate()
                                                    {
                                                        let item = ui.selectable_label(
                                                            index
                                                                == self
                                                                    .fics_console_suggestion_index,
                                                            RichText::new(format!(
                                                                "{command:<15}{description}"
                                                            ))
                                                            .monospace(),
                                                        );
                                                        if scroll_to_suggestion
                                                            && index
                                                                == self
                                                                    .fics_console_suggestion_index
                                                        {
                                                            item.scroll_to_me(Some(Align::Center));
                                                        }
                                                        if item.clicked() {
                                                            self.fics_console_input =
                                                                (*command).to_owned();
                                                            self.fics_console_suggestions_open =
                                                                false;
                                                        }
                                                    }
                                                });
                                            ui.label(
                                                RichText::new(
                                                    "Enter inserts a command; Send runs it.",
                                                )
                                                .weak()
                                                .small(),
                                            );
                                        });
                                });
                        }
                    }
                    if !self.fics_connected {
                        ui.label(RichText::new("Connect to FICS to send commands.").weak());
                    }
                });
            self.fics_console_open = open;
        }

        if self.fics_resign_dialog_open
            && ((self.fics_active && self.fics_playing)
                || (!self.fics_active && self.game_result() == "*"))
        {
            egui::Window::new("Resign game")
                .collapsible(false)
                .resizable(false)
                .fixed_size(Vec2::new(400.0, 148.0))
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .frame(Frame::window(&ctx.style()).inner_margin(Margin::symmetric(20, 18)))
                .show(ctx, |ui| {
                    ui.set_min_width(360.0);
                    ui.label("Resigning ends this game and gives the win to your opponent.");
                    ui.add_space(18.0);
                    ui.allocate_ui_with_layout(
                        Vec2::new(ui.available_width(), 32.0),
                        Layout::right_to_left(Align::Center),
                        |ui| {
                            if ui
                                .add(
                                    egui::Button::new("Resign game")
                                        .min_size(Vec2::new(108.0, 32.0))
                                        .fill(Color32::from_rgb(116, 58, 52)),
                                )
                                .clicked()
                            {
                                if self.fics_active {
                                    #[cfg(target_arch = "wasm32")]
                                    fics_send("resign");
                                    self.fics_status = "Resignation sent to FICS…".into();
                                } else {
                                    self.local_resigned_white = Some(if self.engine_enabled {
                                        self.player_side == PlayerSide::White
                                    } else {
                                        self.board.side_to_move() == Color::White
                                    });
                                    #[cfg(target_arch = "wasm32")]
                                    self.stop_analysis();
                                    #[cfg(target_arch = "wasm32")]
                                    if let Some(engine) = &self.engine {
                                        engine.command("stop");
                                    }
                                    self.engine_searching = false;
                                    self.resume_engine_after_ready = false;
                                    self.selected = None;
                                    self.legal_targets.clear();
                                    self.promotion = None;
                                    self.engine_status = "Game ended by resignation".into();
                                    self.save_game();
                                }
                                self.fics_resign_dialog_open = false;
                            }
                            if ui
                                .add(
                                    egui::Button::new("Keep playing")
                                        .min_size(Vec2::new(108.0, 32.0)),
                                )
                                .clicked()
                            {
                                self.fics_resign_dialog_open = false;
                            }
                        },
                    );
                });
        }

        if self.new_game_dialog_open {
            let mut open = self.new_game_dialog_open;
            egui::Window::new("New game")
                .collapsible(false)
                .resizable(false)
                .fixed_size(Vec2::new(500.0, 540.0))
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .title_bar(false)
                .frame(Self::dialog_frame())
                .show(ctx, |ui| {
                    ui.set_min_size(Vec2::new(500.0, 540.0));
                    if Self::dialog_header(ui, "New game") { open = false; }
                    ui.horizontal_wrapped(|ui| {
                        if ui.selectable_label(!self.new_game_training && !self.new_game_online && !self.new_game_both_sides, RichText::new("You vs Engine").size(18.0)).clicked() {
                            self.new_game_training = false; self.new_game_online = false; self.new_game_both_sides = false;
                        }
                        if ui.selectable_label(!self.new_game_training && !self.new_game_online && self.new_game_both_sides, RichText::new("Play Both Sides").size(18.0)).clicked() {
                            self.new_game_training = false; self.new_game_online = false; self.new_game_both_sides = true;
                        }
                        if ui.selectable_label(self.new_game_training, RichText::new("Training vs AI").size(18.0)).clicked() {
                            self.new_game_training = true; self.new_game_online = false; self.new_game_both_sides = false;
                        }
                        if ui.selectable_label(self.new_game_online, RichText::new("Online").size(18.0)).clicked() { self.new_game_training = false; self.new_game_online = true; }
                    });
                    ui.separator();
                    ui.add_space(10.0);
                    egui::ScrollArea::vertical()
                        .id_salt("new_game_mode_content")
                        .max_height(if self.new_game_training { 440.0 } else { 400.0 }).min_scrolled_height(400.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                    if self.new_game_training {
                        self.training_setup_ui(ui);
                    } else if !self.new_game_online {
                    if !self.new_game_both_sides {
                    ui.horizontal(|ui| {
                        const CARDS_WIDTH: f32 = 466.0;
                        let side_padding = ((ui.available_width() - CARDS_WIDTH) * 0.5).max(0.0);
                        ui.add_space(side_padding);
                        for choice in [
                            NewGameColor::White,
                            NewGameColor::Black,
                            NewGameColor::Random,
                        ] {
                            let selected = self.new_game_color == choice;
                            if Self::new_game_color_card(ui, self.piece_set, choice, selected)
                                .clicked()
                            {
                                self.new_game_color = choice;
                            }
                        }
                    });
                    } else {
                        ui.label("Move White and Black yourself. No automatic engine moves.");
                    }
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                    let card_width = ui.available_width().min(420.0);
                    ui.add_space(((ui.available_width() - card_width) / 2.0).max(0.0));
                    ui.allocate_ui_with_layout(Vec2::new(card_width, 0.0), Layout::top_down(Align::Min), |ui| {
                    Frame::new()
                        .fill(Color32::from_rgb(29, 35, 41))
                        .stroke(Stroke::new(1.0, Color32::from_rgb(57, 65, 72)))
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(Margin::same(16))
                        .show(ui, |ui| {
                    ui.set_width(card_width - 32.0);
                    ui.spacing_mut().interact_size.y = 30.0;
                    ui.spacing_mut().button_padding = Vec2::new(10.0, 6.0);
                    egui::Grid::new("new_game_settings").spacing([24.0, 12.0]).show(ui, |ui| {
                        if !self.new_game_both_sides {
                            ui.label("Engine");
                            ui.label("Stockfish 19");
                            ui.end_row();
                            ui.label(RichText::new("Engine strength").size(14.0));
                            ui.vertical(|ui| {
                                ui.checkbox(&mut self.new_game_limit_strength, "Limit to Elo rating");
                                ui.add_enabled(self.new_game_limit_strength,
                                    egui::Slider::new(&mut self.new_game_elo, 1320..=3190).suffix(" Elo"));
                                if !self.new_game_limit_strength {
                                    ui.label(RichText::new("Uses your current skill-level setting").small().weak());
                                }
                            });
                            ui.end_row();
                        }

                        ui.label(RichText::new("Time control").size(14.0));
                        let before = self.new_game_time;
                        egui::ComboBox::from_id_salt("new_game_time")
                            .width(260.0)
                            .selected_text(["Bullet · 1+0", "Blitz · 5+0", "Rapid · 10+0", "Classical · 30+0", "Custom", "None"][self.new_game_time])
                            .show_ui(ui, |ui| {
                                for (index, label) in ["Bullet · 1+0", "Blitz · 5+0", "Rapid · 10+0", "Classical · 30+0", "Custom", "None"].iter().enumerate() {
                                    ui.selectable_value(&mut self.new_game_time, index, *label);
                                }
                            });
                        if before != self.new_game_time && self.new_game_time < 4 {
                            self.new_game_minutes = [1, 5, 10, 30][self.new_game_time];
                            self.new_game_increment = 0;
                        }
                        ui.end_row();
                        if self.new_game_time == 4 {
                            ui.label("Custom clock");
                            ui.horizontal(|ui| {
                                ui.add(egui::DragValue::new(&mut self.new_game_minutes).range(1..=180).suffix(" min"));
                                ui.label("+");
                                ui.add(egui::DragValue::new(&mut self.new_game_increment).range(0..=180).suffix(" sec / move"));
                            });
                            ui.end_row();
                        }
                        ui.label(RichText::new("Starting position").size(14.0));
                        egui::ComboBox::from_id_salt("new_game_position")
                            .width(260.0)
                            .selected_text(["Standard", "Chess960", "Custom FEN"][self.new_game_position])
                            .show_ui(ui, |ui| {
                                for (index, label) in ["Standard", "Chess960", "Custom FEN"].iter().enumerate() {
                                    ui.selectable_value(&mut self.new_game_position, index, *label);
                                }
                            });
                        ui.end_row();
                    });
                    if self.new_game_position == 1 {
                        ui.label(RichText::new("Random Chess960 position. Castle by moving your king onto its rook.").small().weak());
                    } else if self.new_game_position == 2 {
                        ui.add_space(14.0);
                        ui.separator();
                        ui.add_space(10.0);
                        let valid_fen = Board::from_str(self.new_game_fen.trim()).is_ok();
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Custom position").size(14.0).strong());
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(RichText::new(if valid_fen { "Valid position" } else { "Needs attention" })
                                    .size(12.0).color(if valid_fen { Color32::from_rgb(136, 194, 151) } else { Color32::from_rgb(235, 150, 120) }));
                            });
                        });
                        ui.add_space(6.0);
                        ui.add(egui::TextEdit::multiline(&mut self.new_game_fen)
                            .font(FontId::monospace(12.0))
                            .margin(Vec2::new(10.0, 9.0))
                            .desired_width(f32::INFINITY).desired_rows(2).hint_text("Paste a complete FEN"));
                        ui.add_space(10.0);
                        if ui.add(egui::Button::new(RichText::new("Edit board / Import screenshot").size(14.0).color(Color32::from_rgb(238, 204, 137)))
                            .fill(Color32::from_rgb(58, 49, 32))
                            .stroke(Stroke::new(1.0, Color32::from_rgb(142, 117, 63)))
                            .corner_radius(CornerRadius::same(5))
                            .min_size(Vec2::new(260.0, 34.0))).clicked() {
                            #[cfg(target_arch = "wasm32")]
                            if let Err(error) = open_position_editor(&self.new_game_fen) {
                                self.engine_status = format!("Could not open position editor: {error:?}");
                            }
                        }
                        ui.add_space(5.0);
                        ui.label(RichText::new("Arrange pieces manually, or paste an image of a board.").size(12.0).weak());
                        if !valid_fen {
                            ui.colored_label(Color32::from_rgb(235, 130, 120), "Enter a valid FEN with a legal board, turn and castling rights.");
                        }
                    }
                    });
                    });
                    });
                    } else {
                    ui.vertical_centered(|ui| {
                        let selected = true;
                        let fill = if selected {
                            Color32::from_rgb(43, 39, 29)
                        } else {
                            Color32::from_rgb(24, 28, 33)
                        };
                        let button = egui::Button::new(
                            RichText::new("FICS - Free Internet Chess Server")
                                .size(16.0),
                        )
                        .min_size(Vec2::new(466.0, 58.0))
                        .fill(fill)
                        .stroke(Stroke::new(
                            if selected { 2.0 } else { 1.0 },
                            if selected {
                                Color32::from_rgb(211, 173, 98)
                            } else {
                                Color32::from_white_alpha(36)
                            },
                        ));
                        ui.add(button);
                        ui.add_space(8.0);
                        ui.label(RichText::new("Play other players or observe live games.").weak());
                    });
                    }
                    });
                    ui.add_space(18.0);
                    ui.vertical_centered(|ui| {
                        let valid = if self.new_game_training { self.training_profiles.selected().is_some() && self.training_error.is_none() } else { self.new_game_online || self.new_game_position != 2 || Board::from_str(self.new_game_fen.trim()).is_ok() };
                        if ui.add_enabled_ui(valid, |ui| {
                            if self.new_game_training {
                                let label = self.training_profiles.selected().map(|p| format!("Start {} training", p.next_side().label())).unwrap_or_else(|| "Create a profile to start".to_owned());
                                ui.add_sized([ui.available_width(), 52.0], egui::Button::new(RichText::new(label).size(19.0).strong().color(Color32::from_rgb(24, 27, 31)))
                                    .fill(Color32::from_rgb(211, 173, 98)).corner_radius(8.0))
                            } else {
                                Self::start_battle_button(ui, self.piece_set, self.new_game_online)
                            }
                        }).inner
                        .clicked()
                        {
                            self.new_game_dialog_open = false;
                            if self.new_game_training {
                                self.start_training_game();
                            } else if self.new_game_online {
                                if self.fics_active {

                                    if self.fics_connected && !self.fics_playing {
                                        self.fics_ads.clear();
                                        #[cfg(target_arch = "wasm32")]
                                        fics_send("sought");
                                    }
                                } else {
                                    self.start_fics();
                                }
                            } else {
                                if self.fics_active {
                                    self.stop_fics();
                                }
                                let side = if self.new_game_both_sides { PlayerSide::White } else { match self.new_game_color {
                                    NewGameColor::White => PlayerSide::White,
                                    NewGameColor::Black => PlayerSide::Black,
                                    NewGameColor::Random => {
                                        if Self::animation_time().to_bits() & 1 == 0 {
                                            PlayerSide::White
                                        } else {
                                            PlayerSide::Black
                                        }
                                    }
                                } };
                                self.engine_enabled = !self.new_game_both_sides;
                                self.engine_config.opponent = self.new_game_opponent;
                                if !self.new_game_both_sides {
                                    self.engine_config.limit_strength = self.new_game_limit_strength;
                                    self.engine_config.elo = self.new_game_elo.clamp(1320, 3190);
                                    self.strength_draft = self.engine_config.clone();
                                }
                                self.reset_for_side(side);
                                self.board = match self.new_game_position {
                                    1 => { let mut bytes = [0u8; 2]; let _ = getrandom::fill(&mut bytes); Board::chess960(u16::from_le_bytes(bytes) % 960) },
                                    2 => Board::from_str(self.new_game_fen.trim()).expect("validated FEN"),
                                    _ => Board::default(),
                                };
                                self.review_positions = vec![self.board];
                                if self.new_game_position == 2 {
                                    let fullmove = self.new_game_fen.split_whitespace().nth(5).and_then(|value| value.parse::<usize>().ok()).unwrap_or(1).clamp(1, 100000);
                                    self.local_start_ply = (fullmove - 1) * 2 + usize::from(self.board.side_to_move() == Color::Black);
                                }
                                self.local_clock = (self.new_game_time != 5).then(|| LocalClock {
                                    white: f64::from(self.new_game_minutes * 60),
                                    black: f64::from(self.new_game_minutes * 60),
                                    increment: f64::from(self.new_game_increment),
                                    flagged_white: None,
                                    updated_at: Self::animation_time(),
                                });
                                self.resume_engine_after_ready = false;
                                #[cfg(target_arch = "wasm32")]
                                self.queue_live_engine_resume();
                                self.save_game();
                            }
                        }
                    });
                });
            self.new_game_dialog_open &= open;
        }

        if let Some((from, to)) = self.promotion {
            egui::Window::new("Choose promotion")
                .title_bar(false)
                .collapsible(false)
                .resizable(false)
                .movable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .frame(
                    Frame::window(&ctx.style())
                        .fill(Color32::from_rgb(24, 28, 33))
                        .stroke(Stroke::new(2.0, Color32::from_rgb(211, 173, 98)))
                        .corner_radius(CornerRadius::same(10))
                        .inner_margin(Margin::symmetric(18, 16)),
                )
                .show(ctx, |ui| {
                    ui.label(
                        RichText::new("Choose promotion")
                            .size(22.0)
                            .strong()
                            .color(Color32::from_rgb(211, 173, 98)),
                    );
                    ui.label(format!("Pawn reaches {to}. Choose a piece."));
                    ui.add_space(12.0);
                    let color = self.board.color_on(from).unwrap_or(self.board.side_to_move());
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        for (piece, name) in [
                            (Piece::Queen, "Queen"),
                            (Piece::Rook, "Rook"),
                            (Piece::Bishop, "Bishop"),
                            (Piece::Knight, "Knight"),
                        ] {
                            if Self::promotion_piece_card(ui, self.piece_set, color, piece, name).clicked() {
                                self.promotion = None;
                                let chess_move = ChessMove::new(from, to, Some(piece));
                                if self.best_move_attempt.is_some() {
                                    self.submit_best_move_attempt(chess_move);
                                } else {
                                    self.play(chess_move);
                                }
                            }
                        }
                    });
                });
        }

        self.settle_training();
        self.training_dialog(ctx);
        self.elo_calculator_dialog(ctx);
        self.about_dialog(ctx);
        self.strength_dialog(ctx);
        self.pgn_dialog(ctx);
        self.batch_pgn_dialog(ctx);
        self.batch_pgn_progress_dialog(ctx);
        self.keyboard_move_ui(ctx);

        // Moves are made while drawing the central panel, so arm the delayed
        // repaint after all UI interaction for this frame has completed.
        #[cfg(target_arch = "wasm32")]
        if let Some(due_at) = self.realtime_analysis_due_at {
            let remaining = due_at - Self::animation_time();
            if remaining <= 0.0 {
                let blocked_temporarily = self.engine_searching
                    || self.pgn_dialog_open
                    || self.strength_dialog_open
                    || self.about_dialog_open
                    || self.new_game_dialog_open;
                if self.game_analysis_running
                    || self.game_analysis_paused
                    || self.board.status() != BoardStatus::Ongoing
                {
                    self.realtime_analysis_due_at = None;
                } else if blocked_temporarily {
                    self.realtime_analysis_due_at = Some(Self::animation_time() + 0.25);
                    ctx.request_repaint_after(std::time::Duration::from_millis(250));
                } else {
                    self.start_analysis();
                }
            } else {
                ctx.request_repaint_after(std::time::Duration::from_secs_f64(remaining));
            }
        }

        #[cfg(target_arch = "wasm32")]
        if self.pgn_input.is_empty()
            && self.engine_ready
            && self.engine_enabled
            && self.review_index.is_none()
            && self.board.side_to_move() != self.player_side.color()
            && self.board.status() == BoardStatus::Ongoing
            && !self.engine_searching
            && !self.analysis_running
            && !self.game_analysis_running
            && !self.game_analysis_paused
        {
            self.request_engine_move();
        }
    }
}
