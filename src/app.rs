use chess::{Board, BoardStatus, ChessMove, Color, File, MoveGen, Piece, Rank, Square};
use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontFamily, FontId, Frame, Layout, Margin,
    RichText, Sense, Stroke, Vec2, epaint::TextShape,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

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
}

impl BatchPgnGame {
    fn new(text: String) -> Self {
        Self {
            white: ChessApp::pgn_tag(&text, "White").unwrap_or_else(|| "White".into()),
            black: ChessApp::pgn_tag(&text, "Black").unwrap_or_else(|| "Black".into()),
            result: ChessApp::pgn_tag(&text, "Result").unwrap_or_else(|| "*".into()),
            date: ChessApp::pgn_tag(&text, "Date").unwrap_or_default(),
            site: ChessApp::pgn_tag(&text, "Site").unwrap_or_default(),
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
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodPlayChessSound)]
    fn play_chess_sound(kind: &str);
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodStoreCurrentGame)]
    fn store_current_game(json: &str);
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

#[derive(Deserialize)]
struct ImportedAnalysisPosition {
    fen: String,
    evaluation_cp: Option<i32>,
    mate: Option<i32>,
    depth: Option<u32>,
    nodes: Option<u64>,
    best_move: Option<String>,
    principal_variation: Option<String>,
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
    System,
    Cburnett,
    #[default]
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

#[derive(Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
enum WorkspaceMode {
    #[default]
    Compact,
    Expanded,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum CompactPanel {
    #[default]
    Moves,
    Analysis,
}

#[derive(Serialize, Deserialize)]
struct PersistedGame {
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
    show_best_move_arrows: bool,
    show_move_hover_text: bool,
    figurine_notation: bool,
    move_sounds: bool,
    animate_moves: bool,
    piece_set: PieceSet,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            workspace_mode: WorkspaceMode::default(),
            show_coordinates: true,
            show_best_move_arrows: true,
            show_move_hover_text: true,
            figurine_notation: false,
            move_sounds: true,
            animate_moves: true,
            piece_set: PieceSet::default(),
        }
    }
}

impl UserPreferences {
    fn from_game(game: &PersistedGame) -> Self {
        Self {
            workspace_mode: game.workspace_mode,
            show_coordinates: game.show_coordinates,
            show_best_move_arrows: game.show_best_move_arrows,
            show_move_hover_text: game.show_move_hover_text,
            figurine_notation: game.figurine_notation,
            move_sounds: game.move_sounds,
            animate_moves: game.animate_moves,
            piece_set: game.piece_set,
        }
    }
}

pub struct ChessApp {
    board: Board,
    selected: Option<Square>,
    legal_targets: Vec<Square>,
    history: Vec<Board>,
    last_move: Option<ChessMove>,
    promotion: Option<(Square, Square)>,
    flipped: bool,
    workspace_mode: WorkspaceMode,
    compact_panel: CompactPanel,
    show_coordinates: bool,
    show_best_move_arrows: bool,
    show_move_hover_text: bool,
    figurine_notation: bool,
    engine_analysis_dock_collapsed: bool,
    move_sounds: bool,
    animate_moves: bool,
    move_animation: Option<MoveAnimation>,
    piece_set: PieceSet,
    engine_enabled: bool,
    player_side: PlayerSide,
    new_game_dialog_open: bool,
    new_game_color: NewGameColor,
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
    pgn_input: String,
    pgn_error: Option<String>,
    print_confirm_open: bool,
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
        let player_side = saved
            .as_ref()
            .map(|game| game.player_side)
            .unwrap_or_default();
        let review_white_player = review_pgn
            .and_then(|pgn| Self::pgn_tag(pgn, "White"))
            .unwrap_or_else(|| {
                if player_side == PlayerSide::White {
                    "You".to_owned()
                } else {
                    "Stockfish 19".to_owned()
                }
            });
        let review_black_player = review_pgn
            .and_then(|pgn| Self::pgn_tag(pgn, "Black"))
            .unwrap_or_else(|| {
                if player_side == PlayerSide::Black {
                    "You".to_owned()
                } else {
                    "Stockfish 19".to_owned()
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
        let show_best_move_arrows = preferences.show_best_move_arrows;
        let show_move_hover_text = preferences.show_move_hover_text;
        let figurine_notation = preferences.figurine_notation;
        let move_sounds = preferences.move_sounds;
        let animate_moves = preferences.animate_moves;
        let piece_set = preferences.piece_set;
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

        Self {
            board,
            selected: None,
            legal_targets: vec![],
            history,
            last_move,
            promotion: None,
            flipped,
            workspace_mode,
            compact_panel: CompactPanel::default(),
            show_coordinates,
            show_best_move_arrows,
            show_move_hover_text,
            figurine_notation,
            engine_analysis_dock_collapsed: false,
            move_sounds,
            animate_moves,
            move_animation: None,
            piece_set,
            engine_enabled,
            player_side,
            new_game_dialog_open: false,
            new_game_color: NewGameColor::White,
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
            pgn_input: saved
                .as_ref()
                .and_then(|game| game.review_pgn.clone())
                .unwrap_or_default(),
            pgn_error: None,
            print_confirm_open: false,
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
        }
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
                ui.label(RichText::new("Import game(s)").size(30.0).strong());
                ui.label(
                    RichText::new("Paste PGN or Ironwood analysis JSON below, or drag a .pgn or .json file onto the app. JSON restores saved analysis without rerunning Stockfish.")
                        .color(ui.visuals().weak_text_color()),
                );
                ui.add_space(14.0);
                egui::ScrollArea::vertical()
                    .id_salt("import_game_text")
                    .max_height(300.0)
                    .auto_shrink([false, false])
                    .show_gold(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.import_input)
                                .desired_width(ui.available_width())
                                .desired_rows(14)
                                .hint_text("[Event \"Example\"]\n\n1. e4 e5 2. Nf3 Nc6 ...\n\nor { \"schema\": \"ironwood.analyzed-game/v1\", ... }")
                                .font(egui::TextStyle::Monospace),
                        );
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
                        "Uses the Full game quality setting for every position. You can pause or stop it.",
                    )
                    .size(13.0)
                    .color(ui.visuals().weak_text_color()),
                );
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button("Clear").clicked() {
                        self.import_input.clear();
                        self.pgn_error = None;
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        import = ui
                            .add_enabled(!self.import_input.trim().is_empty(), egui::Button::new("Continue"))
                            .clicked();
                        cancel = ui.button("Cancel").clicked();
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
                                let details = [&game.date, &game.site]
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
            egui::pos2(san_x, center_y),
            Align2::CENTER_CENTER,
            &display_san,
            FontId::proportional(13.0),
            ui.visuals().text_color(),
        );
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
        let hover_index = response.hover_pos().map(|pointer| {
            (((pointer.x - plot.left()) / plot.width()).clamp(0.0, 1.0) * denominator).round()
                as usize
        });
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
            response.on_hover_ui(|ui| {
                let position = if index == 0 {
                    "Starting position".to_owned()
                } else {
                    let san = self
                        .review_moves
                        .get(index - 1)
                        .map(String::as_str)
                        .unwrap_or("Position");
                    if index % 2 == 1 {
                        format!("{}. {san}", index.div_ceil(2))
                    } else {
                        format!("{}... {san}", index / 2)
                    }
                };
                ui.label(RichText::new(position).strong());
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
            })
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
        if self.game_analysis_running || self.game_analysis_paused {
            self.strength_dialog_open = false;
            return;
        }

        let mut apply = false;
        let mut close = false;
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

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Engine settings").size(32.0).strong());
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        close = ui
                            .add_sized(
                                [32.0, 32.0],
                                egui::Button::new(RichText::new("×").size(22.0)),
                            )
                            .on_hover_text("Close engine settings")
                            .clicked();
                    });
                });
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
                    .show_gold(ui, |ui| {
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
        if let Some(result) = Self::pgn_tag(&self.pgn_input, "Result")
            && matches!(result.as_str(), "1-0" | "0-1" | "1/2-1/2")
        {
            return result;
        }
        let board = self.review_positions.last().copied().unwrap_or(self.board);
        match board.status() {
            BoardStatus::Ongoing => "*".into(),
            BoardStatus::Stalemate => "1/2-1/2".into(),
            BoardStatus::Checkmate if board.side_to_move() == Color::White => "0-1".into(),
            BoardStatus::Checkmate => "1-0".into(),
        }
    }

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
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
        if let Some(initial) = self.review_positions.first()
            && *initial != Board::default()
        {
            output.push_str("[SetUp \"1\"]\n");
            output.push_str(&format!("[FEN \"{initial}\"]\n"));
        }
        output.push('\n');

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
            output.push(' ');
        }
        output.push_str(&self.game_result());
        output.push('\n');
        output
    }

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    fn analysis_json(&self) -> Result<String, serde_json::Error> {
        let positions = self.review_positions.iter().enumerate().map(|(index, board)| {
            let analysis = self.game_analysis.get(index).and_then(Option::as_ref);
            serde_json::json!({
                "index": index,
                "fen": board.to_string(),
                "move": index.checked_sub(1).and_then(|move_index| self.review_moves.get(move_index)),
                "side_to_move": if board.side_to_move() == Color::White { "white" } else { "black" },
                "evaluation_cp": analysis.and_then(|value| value.eval_cp),
                "mate": analysis.and_then(|value| value.mate),
                "depth": analysis.map(|value| value.depth),
                "nodes": analysis.map(|value| value.nodes),
                "best_move": analysis.and_then(|value| value.best_move.as_deref()),
                "principal_variation": analysis.map(|value| value.pv.as_str()),
                "centipawn_loss": (index > 0).then(|| self.move_centipawn_loss(index)).flatten(),
                "mate_outcome": (index > 0).then(|| self.move_mate_outcome(index).map(MateOutcome::label)).flatten(),
                "classification": (index > 0).then(|| self.move_classification(index).map(MoveClassification::label)).flatten(),
            })
        }).collect::<Vec<_>>();
        let quality = self.full_game_analysis_config.quality;
        serde_json::to_string_pretty(&serde_json::json!({
            "schema": "ironwood.analyzed-game/v1",
            "schema_version": 1,
            "application": "Ironwood Chess",
            "engine": {
                "name": "Stockfish",
                "version": 19,
                "network": "Full NNUE",
                "threads": self.analysis_config.threads,
                "hash_mib": self.analysis_config.hash_mib,
                "full_game_quality": quality.label(),
                "nodes_per_position": quality.nodes(self.full_game_analysis_config.custom_nodes),
            },
            "game": {
                "white": self.review_white_player,
                "black": self.review_black_player,
                "result": self.game_result(),
                "date": Self::pgn_tag(&self.pgn_input, "Date"),
                "site": Self::pgn_tag(&self.pgn_input, "Site"),
                "source_pgn": (!self.pgn_input.is_empty()).then_some(self.pgn_input.as_str()),
                "moves": self.review_moves,
            },
            "summary": {
                "white_accuracy": self.analysis_accuracy(Color::White),
                "black_accuracy": self.analysis_accuracy(Color::Black),
                "white_average_centipawn_loss": self.analysis_average_centipawn_loss(Color::White),
                "black_average_centipawn_loss": self.analysis_average_centipawn_loss(Color::Black),
            },
            "verification": self.verification_is_complete().then(|| serde_json::json!({
                "nodes_per_position": self.verification_nodes,
                "target_positions": self.verification_targets,
            })),
            "positions": positions,
        }))
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
                let shade = if (rank_index + file_index) % 2 == 0 {
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
@page{{size:A4 portrait;margin:15mm}}*{{box-sizing:border-box}}body{{margin:0;color:#202521;background:#fff;font:10.5pt/1.45 Arial,sans-serif}}header{{padding:0 0 10mm;border-bottom:2px solid #c89e45}}.brand{{font-size:9pt;letter-spacing:.18em;color:#80601e}}h1{{margin:3mm 0 1mm;font:26pt Georgia,serif}}.meta{{color:#68706a}}.summary{{display:grid;grid-template-columns:repeat(2,1fr);gap:3mm;margin:8mm 0}}.metric{{padding:4mm;background:#f0f2ed;border-top:2px solid #c89e45}}.metric .side{{display:block;font-size:8pt;font-weight:bold;letter-spacing:.13em;color:#68706a}}.metric strong{{display:block;font-size:16pt;color:#9b7425}}.metric b{{display:block;font-size:11pt}}.metric small{{display:block;color:#68706a}}.phases{{display:grid;grid-template-columns:repeat(3,1fr);gap:3mm;margin:0 0 6mm}}.phase{{padding:3mm;background:#f0f2ed;border-left:2px solid #c89e45}}.phase b,.phase strong,.phase small{{display:block}}.phase strong{{font-size:14pt;color:#9b7425}}.phase small{{color:#68706a}}.turning-point{{display:grid;grid-template-columns:52mm 1fr;gap:7mm;padding:5mm;background:#f6f4ed;border:1px solid #d6c18f;break-inside:avoid;page-break-inside:avoid;margin:0 0 6mm}}.turning-point .board-wrap,.turning-point .board{{width:52mm;height:52mm}}.turning-point .sq{{font-size:16pt}}.turning-point h2{{font-size:15pt;margin-bottom:2mm}}.turning-point p{{margin:1mm 0 2mm}}.report-legend{{display:flex;flex-wrap:wrap;gap:3mm 6mm;margin:3mm 0 5mm;font-size:8.5pt;font-weight:bold}}.report-legend span{{white-space:nowrap}}.arrow-key{{color:#397a54;text-shadow:0 0 1px #111}}.move-list{{margin-bottom:7mm;border-top:1px solid #ccd1cc}}.move-row{{display:grid;grid-template-columns:9mm 1fr 1fr;gap:2mm;padding:1.2mm 0;border-bottom:1px solid #e5e7e4;break-inside:avoid}}.move-row>b{{color:#727872;text-align:right}}.move{{display:grid;grid-template-columns:7mm minmax(20mm,1fr) 13mm auto;gap:1mm;align-items:center;min-width:0}}.marker{{font-weight:bold}}.san{{font-weight:bold}}.eval{{color:#68706a;font-variant-numeric:tabular-nums}}.cpl{{color:#7d4b25;font-size:7.5pt;white-space:nowrap}}.critical-break{{break-before:page;page-break-before:always}}.moment{{display:grid;grid-template-columns:76mm 1fr;gap:8mm;padding:9mm 0;border-top:1px solid #d9ddd7;break-inside:avoid;page-break-inside:avoid}}.board-wrap{{position:relative;width:76mm;height:76mm}}.board{{width:76mm;height:76mm;display:grid;grid-template-columns:repeat(8,1fr);border:1px solid #425448}}.sq{{display:grid;place-items:center;font:23pt/1 "DejaVu Sans","Segoe UI Symbol",serif}}.light{{background:#dbe2d3}}.dark{{background:#557c66}}.best-arrow{{position:absolute;inset:0;width:100%;height:100%;pointer-events:none}}.kicker{{margin:0;font-weight:bold;letter-spacing:.08em}}h2{{margin:1mm 0 4mm;font:18pt Georgia,serif}}h3{{margin:4mm 0 1mm;font-size:9pt;text-transform:uppercase;letter-spacing:.1em;color:#737a74}}dl{{display:grid;grid-template-columns:32mm 1fr;margin:0}}dt,dd{{margin:0;padding:1.2mm 0;border-bottom:1px solid #e4e6e2}}dt{{color:#68706a}}dd{{font-weight:bold}}.pv{{font-family:Consolas,monospace;font-size:9pt}}.empty{{padding:12mm;background:#f0f2ed}}.print-guidance{{position:sticky;top:0;z-index:5;margin:0 0 5mm;padding:3mm 4mm;background:#fff3cf;border:1px solid #c89e45;color:#4d3b14;font-weight:bold}}footer{{margin-top:10mm;padding-top:4mm;border-top:1px solid #bbb;color:#737a74;font-size:8pt}}@media screen{{html.printing{{overflow:hidden;background:#2b2b2b}}html.printing body{{visibility:hidden}}}}@media print{{body{{print-color-adjust:exact;-webkit-print-color-adjust:exact}}.print-guidance{{display:none}}}}</style></head><body><div class="print-guidance">Print preview is opening. Edge may pause Ironwood until you finish printing or close this window.</div><header><div class="brand">IRONWOOD CHESS - STOCKFISH 19 FULL NNUE</div><h1>{white} vs {black}</h1><div class="meta">{generated} - {analyzed} of {total} positions analyzed - {}</div></header><section class="summary"><div class="metric"><span class="side">WHITE</span><strong>{white_accuracy}</strong><b>{white}</b><small>{white_cpl} average CPL</small></div><div class="metric"><span class="side">BLACK</span><strong>{black_accuracy}</strong><b>{black}</b><small>{black_cpl} average CPL</small></div></section><section class="phases">{phase_summary}</section>{turning_point}<div class="report-legend">{quality_counts}<span class="arrow-key">↗ Stockfish best move</span></div><h1>Game moves</h1><div class="move-list">{move_rows}</div><div class="critical-break"><h1>Critical positions</h1>{critical}</div><footer>Generated locally by Ironwood Chess. Stockfish 19 full NNUE - {} threads - {} MiB hash - {} nodes per position.</footer><script>addEventListener('beforeprint',()=>document.documentElement.classList.add('printing'));addEventListener('afterprint',()=>close());addEventListener('load',()=>setTimeout(()=>print(),900));</script></body></html>"#,
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

    #[cfg(target_arch = "wasm32")]
    fn download_text(filename: &str, mime_type: &str, contents: &str) -> Result<(), JsValue> {
        let parts = js_sys::Array::new();
        parts.push(&JsValue::from_str(contents));
        let options = web_sys::BlobPropertyBag::new();
        options.set_type(mime_type);
        let blob = web_sys::Blob::new_with_str_sequence_and_options(&parts, &options)?;
        let url = web_sys::Url::create_object_url_with_blob(&blob)?;
        let document = web_sys::window()
            .and_then(|window| window.document())
            .ok_or_else(|| JsValue::from_str("Document unavailable"))?;
        let anchor = document
            .create_element("a")?
            .dyn_into::<web_sys::HtmlAnchorElement>()?;
        anchor.set_href(&url);
        anchor.set_download(filename);
        anchor.click();
        web_sys::Url::revoke_object_url(&url)
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
        let move_text = text
            .lines()
            .filter(|line| !line.trim_start().starts_with('['))
            .collect::<Vec<_>>()
            .join(" ");
        let mut analyses = vec![None; positions.len()];
        let mut board = positions.first().copied().unwrap_or_default();
        let mut move_count = 0_usize;
        let mut variation_depth = 0_u32;
        let mut line_comment = false;
        let mut comment = String::new();
        let mut token = String::new();
        let mut in_comment = false;

        let consume_token = |raw: &mut String, board: &mut Board, move_count: &mut usize| {
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
                    Self::restore_pgn_comment(&comment, move_count, &mut analyses);
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
        analyses
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
                board: final_board.to_string(),
                final_board: final_board.to_string(),
                result,
                history: Vec::new(),
                last_move,
                flipped: false,
                workspace_mode: self.workspace_mode,
                show_coordinates: self.show_coordinates,
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
                show_best_move_arrows: self.show_best_move_arrows,
                show_move_hover_text: self.show_move_hover_text,
                figurine_notation: self.figurine_notation,
                move_sounds: self.move_sounds,
                animate_moves: self.animate_moves,
                piece_set: self.piece_set,
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

    fn save_game(&self) {
        self.save_preferences();
        #[cfg(target_arch = "wasm32")]
        if let Some(storage) =
            web_sys::window().and_then(|window| window.local_storage().ok().flatten())
        {
            let game = PersistedGame {
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
                game_analysis_running: self.game_analysis_running,
                game_analysis_paused: self.game_analysis_paused,
                verification_targets: self.verification_targets.clone(),
                verification_results: self.verification_results.clone(),
                verification_nodes: self.verification_nodes,
            };
            if let Ok(json) = serde_json::to_string(&game) {
                let _ = storage.set_item(STORAGE_KEY, &json);
                store_current_game(&json);
            }
        }
    }

    fn reset_for_side(&mut self, player_side: PlayerSide) {
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
        self.game_analysis.clear();
        self.clear_verification();
        self.game_analysis_index = None;
        self.game_analysis_running = false;
        self.game_analysis_paused = false;
        self.review_index = None;
        self.review_scroll_to_selected = false;
        self.best_move_attempt = None;
        self.pgn_input.clear();
        self.review_white_player = if player_side == PlayerSide::White {
            "You".into()
        } else {
            "Stockfish 19".into()
        };
        self.review_black_player = if player_side == PlayerSide::Black {
            "You".into()
        } else {
            "Stockfish 19".into()
        };
        self.clear_analysis_result();
        self.resume_engine_after_ready = false;
        let engine_should_move = self.engine_enabled && player_side == PlayerSide::Black;
        self.engine_status = if engine_should_move {
            "Waiting for Stockfish…".into()
        } else {
            "Your move".into()
        };
        Self::set_page_title(false, "", "");
        #[cfg(target_arch = "wasm32")]
        start_new_stored_game("mine");
        self.save_game();
        #[cfg(target_arch = "wasm32")]
        if engine_should_move {
            self.queue_live_engine_resume();
        }
    }

    fn play_from_current_position(&mut self) {
        self.best_move_attempt = None;
        self.move_animation = None;
        let branch_index = self
            .review_index
            .unwrap_or_else(|| self.review_positions.len().saturating_sub(1))
            .min(self.review_positions.len().saturating_sub(1));
        if let Some(branch_board) = self.review_positions.get(branch_index).copied() {
            self.review_positions.truncate(branch_index + 1);
            self.review_moves.truncate(branch_index);
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
        self.review_white_player = if self.player_side == PlayerSide::White {
            "You".into()
        } else {
            "Stockfish 19".into()
        };
        self.review_black_player = if self.player_side == PlayerSide::Black {
            "You".into()
        } else {
            "Stockfish 19".into()
        };
        self.clear_analysis_result();
        self.resume_engine_after_ready = engine_should_move;
        self.engine_status = if engine_should_move {
            "Waiting for Stockfish…".into()
        } else {
            "Your move".into()
        };
        Self::set_page_title(false, "", "");
        #[cfg(target_arch = "wasm32")]
        start_new_stored_game("mine");
        self.save_game();
    }

    fn select(&mut self, square: Square) {
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

    fn start_best_move_attempt(&mut self, target_index: usize) {
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
            let moving_piece = self
                .board
                .piece_on(mv.get_source())
                .expect("legal move has piece");
            let moving_color = self.board.side_to_move();
            #[cfg(target_arch = "wasm32")]
            let is_capture = self.board.piece_on(mv.get_dest()).is_some()
                || (self.board.piece_on(mv.get_source()) == Some(Piece::Pawn)
                    && mv.get_source().get_file() != mv.get_dest().get_file());
            let san = Self::san_for_move(&self.board, mv);
            if self.review_positions.is_empty() {
                self.review_positions.push(self.board);
                self.review_white_player = if self.player_side == PlayerSide::White {
                    "You".into()
                } else {
                    "Stockfish 19".into()
                };
                self.review_black_player = if self.player_side == PlayerSide::Black {
                    "You".into()
                } else {
                    "Stockfish 19".into()
                };
            }
            self.history.push(self.board);
            self.board = self.board.make_move_new(mv);
            self.move_animation = self.animate_moves.then(|| MoveAnimation {
                chess_move: mv,
                piece: moving_piece,
                color: moving_color,
                started_at: Self::animation_time(),
            });
            #[cfg(target_arch = "wasm32")]
            if self.move_sounds {
                let sound = if self.board.checkers().popcnt() > 0 {
                    "check"
                } else if is_capture {
                    "capture"
                } else {
                    "move"
                };
                play_chess_sound(sound);
            }
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
                    "Waiting for Stockfish…".into()
                } else {
                    "Your move".into()
                };
            self.schedule_realtime_analysis();
            self.save_game();
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
                    if line.contains("Cross-origin isolation") || line.contains("worker failed") {
                        self.engine_status = line;
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
        let game_result = self.game_result();
        let player_area_height = if show_players { 64.0 } else { 0.0 };
        let board_size = available
            .x
            .min((available.y - player_area_height).max(180.0))
            .min(760.0)
            .max(180.0);
        let cell = board_size / 8.0;
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
                self.show_best_move_arrows
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
            Vec2::new(board_size, board_size + player_area_height),
            Layout::top_down(Align::Min),
            |ui| {
                if show_players {
                    let top_player = if self.flipped {
                        self.review_white_player.clone()
                    } else {
                        self.review_black_player.clone()
                    };
                    ui.allocate_ui_with_layout(
                        Vec2::new(board_size, 28.0),
                        Layout::left_to_right(Align::Center),
                        |ui| {
                            ui.spacing_mut().item_spacing.x = 7.0;
                            ui.label(RichText::new("●").color(Color32::from_rgb(76, 116, 92)));
                            ui.label(RichText::new(&top_player).size(17.0).strong());
                            Self::game_result_badge(ui, &game_result, self.flipped);
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if Self::board_action_icon(ui, true, true)
                                    .on_hover_text("Flip board")
                                    .clicked()
                                {
                                    self.flipped = !self.flipped;
                                    self.save_game();
                                }
                                if Self::board_action_icon(ui, false, !self.history.is_empty())
                                    .on_hover_text("Undo last move")
                                    .clicked()
                                {
                                    self.undo();
                                }
                            });
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
                                    let animation_hides_piece =
                                        move_animation.is_some_and(|animation| {
                                            animation.chess_move.get_dest() == square
                                        });
                                    if !animation_hides_piece {
                                        if let Some(source) = piece_image {
                                            egui::Image::new(source)
                                                .maintain_aspect_ratio(true)
                                                .paint_at(ui, response.rect.shrink(cell * 0.06));
                                        } else if let Some(piece_side) = self.board.color_on(square)
                                        {
                                            let (outline, cardinal, diagonal) = match piece_side {
                                                Color::White => {
                                                    (Color32::from_rgb(20, 24, 27), 1.5, 1.1)
                                                }
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
                if let Some(animation) = move_animation {
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
                            ui.spacing_mut().item_spacing.x = 7.0;
                            ui.label(RichText::new("●").color(Color32::from_rgb(230, 178, 65)));
                            ui.label(RichText::new(bottom_player).size(17.0).strong());
                            Self::game_result_badge(ui, &game_result, !self.flipped);
                        },
                    );
                }
            },
        );
    }

    fn engine_analysis_dock_ui(&mut self, ui: &mut egui::Ui) {
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

    fn game_moves_status_ui(&mut self, ui: &mut egui::Ui) {
        let result = self.game_result();
        if result == "*" {
            let status = if self.board.side_to_move() == Color::White {
                "White to move"
            } else {
                "Black to move"
            };
            ui.label(RichText::new(status).size(16.0).weak());
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
        ui.separator();
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

    fn game_moves_column_ui(&mut self, ui: &mut egui::Ui) {
        ui.add_space(14.0);
        ui.label(RichText::new("Game Moves").size(19.0).strong());
        self.game_moves_status_ui(ui);
        let Some(index) = self.review_index.or_else(|| {
            (!self.review_positions.is_empty()).then(|| self.review_positions.len() - 1)
        }) else {
            ui.label(RichText::new("Moves will appear here as the game develops.").weak());
            return;
        };

        ui.label(RichText::new(format!("Position {index} of {}", self.review_moves.len())).weak());
        ui.add_space(8.0);

        let mut jump_to = None;
        let scroll_to_selected = self.review_scroll_to_selected;
        let moves_height = (ui.available_height() - 62.0).max(160.0);
        Frame::new()
            .fill(Color32::from_rgb(11, 16, 29))
            .stroke(Stroke::new(1.0, Color32::from_white_alpha(24)))
            .corner_radius(CornerRadius::same(6))
            .inner_margin(Margin::same(8))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("game_moves_column")
                    .max_height(moves_height)
                    .min_scrolled_height(moves_height)
                    .auto_shrink([false, false])
                    .show_gold(ui, |ui| {
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
                                let move_width =
                                    ((ui.available_width() - ui.spacing().item_spacing.x) / 2.0)
                                        .max(62.0);
                                if let Some(san) = self.review_moves.get(white_ply).cloned() {
                                    let selected = index == white_ply + 1;
                                    let response = self.analyzed_move_button(
                                        ui,
                                        &san,
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
                                if let Some(san) = self.review_moves.get(black_ply).cloned() {
                                    let selected = index == black_ply + 1;
                                    let response = self.analyzed_move_button(
                                        ui,
                                        &san,
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
                    for (label, destination, enabled, hint) in [
                        ("|◀", 0, index > 0, "First position (Home)"),
                        (
                            "◀",
                            index.saturating_sub(1),
                            index > 0,
                            "Previous move (Left arrow)",
                        ),
                        (
                            "▶",
                            (index + 1).min(self.review_moves.len()),
                            index < self.review_moves.len(),
                            "Next move (Right arrow)",
                        ),
                        (
                            "▶|",
                            self.review_moves.len(),
                            index < self.review_moves.len(),
                            "Last position (End)",
                        ),
                    ] {
                        if ui
                            .add_enabled(
                                enabled,
                                egui::Button::new(label).min_size(Vec2::new(button_width, 30.0)),
                            )
                            .on_hover_text(hint)
                            .clicked()
                        {
                            jump_to = Some(destination);
                        }
                    }
                });
            });

        self.review_scroll_to_selected = false;
        if let Some(destination) = jump_to {
            self.review_to(destination);
        }
    }

    fn new_game_color_card(
        ui: &mut egui::Ui,
        piece_set: PieceSet,
        choice: NewGameColor,
        selected: bool,
    ) -> egui::Response {
        let size = Vec2::new(150.0, 176.0);
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
                    egui::pos2(rect.center().x, rect.top() + 58.0),
                    Vec2::splat(78.0),
                );
                Self::paint_piece(ui, king_rect, 78.0, piece_set, color, Piece::King);
            }
            NewGameColor::Random => {
                for (color, x) in [
                    (Color::White, rect.center().x - 25.0),
                    (Color::Black, rect.center().x + 25.0),
                ] {
                    let king_rect = egui::Rect::from_center_size(
                        egui::pos2(x, rect.top() + 60.0),
                        Vec2::splat(58.0),
                    );
                    Self::paint_piece(ui, king_rect, 58.0, piece_set, color, Piece::King);
                }
            }
        }

        let (title, subtitle) = match choice {
            NewGameColor::White => ("WHITE", "You make the first move"),
            NewGameColor::Black => ("BLACK", "Stockfish opens the game"),
            NewGameColor::Random => ("RANDOM", "Let Ironwood choose"),
        };
        ui.painter().text(
            egui::pos2(rect.center().x, rect.top() + 113.0),
            Align2::CENTER_CENTER,
            title,
            FontId::proportional(17.0),
            if selected {
                accent
            } else {
                ui.visuals().text_color()
            },
        );
        ui.painter().text(
            egui::pos2(rect.center().x, rect.top() + 141.0),
            Align2::CENTER_CENTER,
            subtitle,
            FontId::proportional(12.5),
            ui.visuals().weak_text_color(),
        );
        if selected {
            ui.painter().text(
                egui::pos2(rect.center().x, rect.bottom() - 14.0),
                Align2::CENTER_CENTER,
                "SELECTED",
                FontId::proportional(10.5),
                accent,
            );
        }
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    }

    fn start_battle_button(ui: &mut egui::Ui, piece_set: PieceSet) -> egui::Response {
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
            "START GAME",
            FontId::proportional(18.0),
            Color32::WHITE,
        );
        ui.painter().text(
            egui::pos2(rect.center().x, rect.center().y + 13.0),
            Align2::CENTER_CENTER,
            "BEGIN THE BATTLE",
            FontId::proportional(10.0),
            accent,
        );

        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
                ui.menu_button("Game", |ui| {
                    Self::set_menu_item_font(ui);
                    let analysis_available = !self.review_moves.is_empty()
                        && self.game_analysis.iter().any(Option::is_some);
                    let export_enabled = analysis_available && !self.game_analysis_running;
                    let export_disabled_reason = if self.game_analysis_running {
                        "Pause or stop game analysis or verification before exporting."
                    } else {
                        "Analyze at least one position before exporting."
                    };
                    if ui.button("New game").clicked() {
                        self.new_game_color = match self.player_side {
                            PlayerSide::White => NewGameColor::White,
                            PlayerSide::Black => NewGameColor::Black,
                        };
                        self.new_game_dialog_open = true;
                        ui.close();
                    }
                    if ui.button("Save current game").clicked() {
                        self.save_game();
                        self.engine_status = "Game saved on this device".into();
                        ui.close();
                    }
                    if ui.button("Load saved game…").clicked() {
                        #[cfg(target_arch = "wasm32")]
                        open_game_library();
                        ui.close();
                    }
                    ui.separator();
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
                    if ui.button("Import PGN or JSON…").clicked() {
                        #[cfg(target_arch = "wasm32")]
                        ensure_imported_index();
                        self.import_input.clear();
                        self.pgn_dialog_open = true;
                        self.pgn_error = None;
                        ui.close();
                    }
                    if ui
                        .add_enabled(export_enabled, egui::Button::new("Export annotated PGN"))
                        .on_disabled_hover_text(export_disabled_reason)
                        .clicked()
                    {
                        #[cfg(target_arch = "wasm32")]
                        {
                            let pgn = self.annotated_pgn();
                            self.engine_status = match Self::download_text(
                                "ironwood-analyzed-game.pgn",
                                "application/x-chess-pgn;charset=utf-8",
                                &pgn,
                            ) {
                                Ok(()) => "Annotated PGN exported".into(),
                                Err(_) => "Annotated PGN export failed".into(),
                            };
                        }
                        ui.close();
                    }
                    if ui
                        .add_enabled(export_enabled, egui::Button::new("Export analysis JSON"))
                        .on_disabled_hover_text(export_disabled_reason)
                        .clicked()
                    {
                        #[cfg(target_arch = "wasm32")]
                        {
                            self.engine_status = match self.analysis_json() {
                                Ok(json) => match Self::download_text(
                                    "ironwood-analyzed-game.json",
                                    "application/json;charset=utf-8",
                                    &json,
                                ) {
                                    Ok(()) => "Analysis JSON exported".into(),
                                    Err(_) => "Analysis JSON export failed".into(),
                                },
                                Err(_) => "Analysis JSON export failed".into(),
                            };
                        }
                        ui.close();
                    }
                    if ui
                        .add_enabled(export_enabled, egui::Button::new("Print analysis report…"))
                        .on_disabled_hover_text(export_disabled_reason)
                        .on_hover_text("Open a critical-positions report that can be saved as PDF")
                        .clicked()
                    {
                        self.print_confirm_open = true;
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
                    Self::set_menu_item_font(ui);
                    ui.menu_button("Workspace", |ui| {
                        Self::set_menu_item_font(ui);
                        for (mode, label) in [
                            (WorkspaceMode::Compact, "Compact"),
                            (WorkspaceMode::Expanded, "Expanded"),
                        ] {
                            if ui
                                .selectable_value(&mut self.workspace_mode, mode, label)
                                .changed()
                            {
                                self.save_game();
                                ui.close();
                            }
                        }
                        ui.separator();
                        ui.label(RichText::new("Expanded needs a wide window").small().weak());
                    });
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
                        ui.menu_button("Piece set", |ui| {
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
                            .checkbox(&mut self.show_move_hover_text, "Show hover text")
                            .changed()
                        {
                            self.save_game();
                        }
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
                ui.menu_button("Storage", |ui| {
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
                ui.menu_button("Help", |ui| {
                    Self::set_menu_item_font(ui);
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
                    let engine_settings_locked =
                        self.game_analysis_running || self.game_analysis_paused;
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

        let separate_moves_panel = self.workspace_mode == WorkspaceMode::Expanded
            && ctx.available_rect().width() >= 1180.0;
        let compact = !separate_moves_panel;
        let show_moves = compact && self.compact_panel == CompactPanel::Moves;
        let show_analysis = !compact || self.compact_panel == CompactPanel::Analysis;

        egui::SidePanel::right("game_panel")
            .default_width(340.0)
            .min_width(280.0)
            .max_width(640.0)
            .resizable(true)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("game_panel_scroll")
                    .auto_shrink([false, false])
                    .show_gold(ui, |ui| {
                if compact {
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.compact_panel, CompactPanel::Moves, "Game Moves");
                        ui.selectable_value(&mut self.compact_panel, CompactPanel::Analysis, "Game Analysis");
                    });
                    ui.separator();
                }
                if show_moves {
                    ui.add_space(6.0);
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
                                .show_gold(ui, |ui| {
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
                    if let Some(destination) = self.analysis_graph(ui) {
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
                    if self.review_index.is_some()
                        && ui.button("Play from this position").clicked()
                    {
                        self.play_from_current_position();
                    }
                    }
                }
                ui.add_space(18.0);
                if self.review_positions.is_empty() && show_moves {
                    ui.label(RichText::new("Moves").strong());
                    ui.label(format!("{} half-moves played", self.history.len()));
                    ui.label(RichText::new("Saved locally").small().weak());
                } else if self.review_positions.is_empty() && show_analysis {
                    ui.label(RichText::new("Game Analysis").size(17.0).strong());
                    ui.label(RichText::new("Play or import a game to analyze it.").weak());
                }
                    });
            });

        let dock_height = if self.engine_analysis_dock_collapsed {
            40.0
        } else {
            190.0
        };
        egui::TopBottomPanel::bottom("engine_analysis_dock")
            .exact_height(dock_height)
            .frame(
                Frame::new()
                    .fill(Color32::from_rgb(18, 21, 25))
                    .stroke(Stroke::new(1.0, Color32::from_white_alpha(28)))
                    .inner_margin(Margin::same(10)),
            )
            .show(ctx, |ui| self.engine_analysis_dock_ui(ui));

        if separate_moves_panel {
            egui::SidePanel::right("game_moves_panel")
                .default_width(270.0)
                .min_width(220.0)
                .max_width(420.0)
                .resizable(true)
                .frame(
                    Frame::new()
                        .fill(Color32::from_rgb(18, 21, 25))
                        .stroke(Stroke::new(1.0, Color32::from_white_alpha(28)))
                        .inner_margin(Margin::symmetric(10, 0)),
                )
                .show(ctx, |ui| self.game_moves_column_ui(ui));
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

        if self.print_confirm_open {
            let mut open = true;
            let mut print = false;
            let mut cancel = false;
            egui::Window::new("Open print preview?")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.set_max_width(460.0);
                    ui.label(
                        "Edge may pause Ironwood while the system print window is open. Finish printing or close that window to return to the game.",
                    );
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            cancel = true;
                        }
                        if ui
                            .button("Open print preview")
                            .on_hover_text("Open the analysis report and system print window")
                            .clicked()
                        {
                            print = true;
                        }
                    });
                });
            self.print_confirm_open = open && !cancel && !print;
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

        if self.new_game_dialog_open {
            let mut open = self.new_game_dialog_open;
            egui::Window::new("New game")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .open(&mut open)
                .frame(Frame::window(&ctx.style()).inner_margin(Margin {
                    left: 22,
                    right: 22,
                    top: 14,
                    bottom: 24,
                }))
                .show(ctx, |ui| {
                    ui.set_min_width(500.0);
                    ui.vertical_centered(|ui| {
                        ui.heading(RichText::new("Choose your side").size(25.0));
                        ui.label(
                            RichText::new(format!(
                                "Playing with the {} piece set",
                                self.piece_set.label()
                            ))
                            .weak(),
                        );
                    });
                    ui.add_space(14.0);
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
                    ui.add_space(24.0);
                    ui.vertical_centered(|ui| {
                        if Self::start_battle_button(ui, self.piece_set).clicked() {
                            let side = match self.new_game_color {
                                NewGameColor::White => PlayerSide::White,
                                NewGameColor::Black => PlayerSide::Black,
                                NewGameColor::Random => {
                                    if Self::animation_time().to_bits() & 1 == 0 {
                                        PlayerSide::White
                                    } else {
                                        PlayerSide::Black
                                    }
                                }
                            };
                            self.new_game_dialog_open = false;
                            self.reset_for_side(side);
                        }
                    });
                });
            self.new_game_dialog_open &= open;
        }

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

        self.about_dialog(ctx);
        self.strength_dialog(ctx);
        self.pgn_dialog(ctx);
        self.batch_pgn_dialog(ctx);
        self.batch_pgn_progress_dialog(ctx);

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
