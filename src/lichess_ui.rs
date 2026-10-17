// Included in app.rs so online play shares the existing board and fair-play guards.
#[derive(Default)]
struct LichessSession {
    active: bool,
    open: bool,
    lobby: serde_json::Value,
    game: serde_json::Value,
    applied_moves: Vec<String>,
    saved_id: String,
    minutes: i32,
    increment: i32,
    days: i32,
    correspondence: bool,
    rated: bool,
    color: String,
    opponent: String,
    chat_input: String,
    ai_level: i32,
    ai_variant: String,
    ai_fen: String,
    ai_time: usize,
    pending_preview: Option<ChessMove>,
    lobby_tab: usize,
    profile_windows: Vec<PlayerProfileWindow>,
    next_profile_window_id: u64,
    profile_action_error: String,
    profile_review_requested: bool,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct PlayerProfileWindow {
    id: u64,
    open: bool,
    #[serde(default)]
    detached: bool,
    username: String,
    control: usize,
    shown_username: String,
    range: usize,
    following_search: String,
    performance_control: Option<usize>,
}
impl PlayerProfileWindow {
    fn new(id: u64, username: &str) -> Self {
        Self {id,open:true,detached:false,username:username.into(),control:0,shown_username:String::new(),range:5,following_search:String::new(),performance_control:None}
    }
    fn load(&mut self, refresh: bool) {
        if !refresh {self.performance_control=None;}
        lichess_command(serde_json::json!({"type":"playerProfile","windowId":self.id.to_string(),"username":self.username.trim(),"refresh":refresh}));
    }
}

impl LichessSession {
    fn new() -> Self {
        Self {
            minutes: 10,
            increment: 5,
            days: 2,
            color: "random".into(),
            ai_level: 3,
            ai_variant: "standard".into(),
            ..Self::default()
        }
    }
    fn connected(&self) -> bool {
        self.lobby["connected"].as_bool().unwrap_or(false)
    }
}

fn lichess_command(value: serde_json::Value) {
    #[cfg(target_arch = "wasm32")]
    lichess_send_command(&value.to_string());
    #[cfg(not(target_arch = "wasm32"))]
    let _ = value;
}

// Validate the complete authoritative move history before replacing the displayed game.
#[cfg(test)]
fn lichess_replay(initial: &str, moves: &str) -> Result<(Board, Vec<ChessMove>), String> {
    lichess_replay_variant(initial, moves, false)
}

fn lichess_replay_variant(initial: &str, moves: &str, chess960: bool) -> Result<(Board, Vec<ChessMove>), String> {
    let start = if chess960 {
        Board::from_chess960_fen(if initial == "startpos" { "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1" } else { initial })?
    } else if initial == "startpos" || initial.is_empty() {
        Board::default()
    } else {
        Board::from_str(initial).map_err(|_| "Unsupported Lichess starting position")?
    };
    let mut board = start;
    let mut parsed = Vec::new();
    for uci in moves.split_whitespace() {
        // Lichess streams king-to-rook castling; Ironwood's chess crate uses king-to-destination.
        let normalized = if board.piece_on(
            ChessMove::from_str(uci)
                .map_err(|_| "Invalid Lichess move")?
                .get_source(),
        ) == Some(Piece::King) && !board.is_chess960()
        {
            match uci {
                "e1h1" => "e1g1",
                "e1a1" => "e1c1",
                "e8h8" => "e8g8",
                "e8a8" => "e8c8",
                _ => uci,
            }
        } else {
            uci
        };
        let mv = ChessMove::from_str(normalized).map_err(|_| "Invalid Lichess move")?;
        if !MoveGen::new_legal(&board).any(|legal| legal == mv) {
            return Err(format!(
                "Lichess move history could not be reconciled at {uci}"
            ));
        }
        board = board.make_move_new(mv);
        parsed.push(mv);
    }
    Ok((start, parsed))
}

fn lichess_player_name(player: &serde_json::Value, fallback: &str) -> String {
    player["name"]
        .as_str()
        .map(str::to_owned)
        .or_else(|| {
            player["aiLevel"]
                .as_u64()
                .map(|level| format!("Lichess AI level {level}"))
        })
        .unwrap_or_else(|| fallback.to_owned())
}

fn lichess_pgn(game: &serde_json::Value, moves: &[String], result: &str) -> String {
    let escape = |s: &str| {
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace(['\r', '\n'], " ")
    };
    let initial = game["initialFen"].as_str().unwrap_or("startpos");
    let custom = initial != "startpos" && !initial.is_empty();
    let mut pgn = format!(
        "[Event \"Lichess {} game\"]\n[Site \"https://lichess.org/{}\"]\n[White \"{}\"]\n[Black \"{}\"]\n[Result \"{result}\"]\n",
        if game["rated"].as_bool() == Some(true) {
            "rated"
        } else {
            "casual"
        },
        escape(game["id"].as_str().unwrap_or("")),
        escape(&lichess_player_name(&game["white"], "White")),
        escape(&lichess_player_name(&game["black"], "Black"))
    );
    if game["variant"]["key"].as_str() == Some("chess960") {
        pgn.push_str("[Variant \"Chess960\"]\n");
    }
    for (player, tag) in [("white", "WhiteElo"), ("black", "BlackElo")] {
        if let Some(elo) = game[player]["rating"].as_u64() {
            pgn.push_str(&format!("[{tag} \"{elo}\"]\n"));
        }
    }
    if let Some(date) = game["date"].as_str() {
        pgn.push_str(&format!("[Date \"{}\"]\n", escape(date)));
    }
    if let Some(clock) = game["clock"].as_object() {
        if let (Some(initial), Some(increment)) =
            (clock["initial"].as_u64(), clock["increment"].as_u64())
        {
            pgn.push_str(&format!(
                "[TimeControl \"{}+{}\"]\n",
                initial / 1000,
                increment / 1000
            ));
        }
    }
    if custom {
        pgn.push_str(&format!("[SetUp \"1\"]\n[FEN \"{}\"]\n", escape(initial)));
    }
    pgn.push('\n');
    let fields: Vec<&str> = initial.split_whitespace().collect();
    let start_ply = if custom {
        fields
            .get(5)
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(1)
            .saturating_sub(1)
            * 2
            + usize::from(fields.get(1) == Some(&"b"))
    } else {
        0
    };
    for (index, san) in moves.iter().enumerate() {
        let ply = start_ply + index;
        if ply % 2 == 0 {
            pgn.push_str(&format!("{}. ", ply / 2 + 1));
        } else if index == 0 {
            pgn.push_str(&format!("{}... ", ply / 2 + 1));
        }
        pgn.push_str(san);
        pgn.push(' ');
    }
    pgn.push_str(result);
    pgn
}

impl ChessApp {
    fn lichess_selection_border(ui: &egui::Ui, response: &egui::Response, selected: bool) {
        if selected {
            ui.painter().rect_stroke(response.rect.shrink(3.0), 4.0,
                Stroke::new(2.0, Color32::from_rgb(55, 43, 23)), egui::StrokeKind::Inside);
        }
    }

    fn lichess_computer_setup(&mut self, ui: &mut egui::Ui, seeking: bool) {
        let gold = Color32::from_rgb(211, 173, 98);
        ui.vertical_centered(|ui| {
            ui.spacing_mut().interact_size.y = 36.0;
            let title = match self.lichess.ai_variant.as_str() {
                "chess960" => "Chess960 — randomized back rank",
                "fromPosition" => "From Position — standard chess from a custom position",
                _ => "Standard — standard rules of chess",
            };
            egui::ComboBox::from_id_salt("lichess_ai_variant").width(ui.available_width() - 8.0)
                .selected_text(RichText::new(title).size(17.0)).show_ui(ui, |ui| {
                    for (key, label) in [("standard", "Standard"), ("chess960", "Chess960"), ("fromPosition", "From Position")] {
                        ui.selectable_value(&mut self.lichess.ai_variant, key.into(), label);
                    }
                });
            let custom = self.lichess.ai_variant == "fromPosition";
            if custom {
                ui.add_space(12.0);
                ui.add(egui::TextEdit::singleline(&mut self.lichess.ai_fen)
                    .desired_width(ui.available_width()).margin(Vec2::new(10.0, 10.0))
                    .hint_text("Paste the FEN text here"));
            }
            ui.add_space(18.0);
            let width = (ui.available_width() - 16.0) / 3.0;
            ui.horizontal(|ui| {
                for (index, label) in ["Unlimited", "Real time", "Correspondence"].iter().enumerate() {
                    let selected = self.lichess.ai_time == index;
                    let response = ui.add_sized([width, 38.0], egui::Button::new(egui::WidgetText::from(RichText::new(*label).size(16.0)).color(Color32::from_rgb(25, 29, 35))).raised_gold());
                    Self::lichess_selection_border(ui, &response, selected);
                    if response.clicked() {
                        self.lichess.ai_time = index;
                    }
                }
            });
            ui.add_space(12.0);
            match self.lichess.ai_time {
                1 => { ui.horizontal(|ui| {
                    ui.label("Time");
                    ui.add(egui::DragValue::new(&mut self.lichess.minutes).range(1..=180).suffix(" min"));
                    ui.label("Increment");
                    ui.add(egui::DragValue::new(&mut self.lichess.increment).range(0..=60).suffix(" sec"));
                }); }
                2 => { egui::ComboBox::from_id_salt("lichess_ai_days").selected_text(format!("{} days per move", self.lichess.days)).show_ui(ui, |ui| {
                    for days in [1, 2, 3, 5, 7, 10, 14] { ui.selectable_value(&mut self.lichess.days, days, format!("{days} days per move")); }
                }); }
                _ => { ui.label(RichText::new("Take all the time you need").size(15.0)); }
            }
            ui.add_space(18.0);
            ui.label(RichText::new("Strength").size(16.0).strong());
            ui.add_space(6.0);
            let strength_width = (ui.available_width() - 7.0 * 4.0) / 8.0;
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                for level in 1..=8 {
                    let selected = self.lichess.ai_level == level;
                    let response = ui.add_sized([strength_width, 38.0], egui::Button::new(egui::WidgetText::from(RichText::new(level.to_string()).size(16.0)
                        .color(if selected { Color32::BLACK } else { Color32::LIGHT_GRAY })).color(Color32::from_rgb(25, 29, 35)))
                        .raised_gold());
                    Self::lichess_selection_border(ui, &response, selected);
                    if response.clicked() { self.lichess.ai_level = level; }
                }
            });
            ui.add_space(18.0);
            ui.label(RichText::new("Side").size(16.0).strong());
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                for (key, label, color) in [("white", "White", Color::White), ("random", "Random side", Color::White), ("black", "Black", Color::Black)] {
                    let selected = self.lichess.color == key;
                    let response = ui.add_sized([width, 78.0], egui::Button::new("").raised_gold());
                    let rect = response.rect;
                    Self::lichess_selection_border(ui, &response, selected);
                    let icon = egui::Rect::from_center_size(rect.center_top() + Vec2::new(0.0, 27.0), Vec2::splat(36.0));
                    Self::paint_piece(ui, icon, 36.0, self.piece_set, color, Piece::King);
                    if key == "random" {
                        Self::paint_piece(ui, icon.translate(Vec2::new(10.0, 0.0)), 36.0, self.piece_set, Color::Black, Piece::King);
                    }
                    ui.painter().text(rect.center_bottom() - Vec2::new(0.0, 15.0), egui::Align2::CENTER_CENTER,
                        label, egui::FontId::proportional(15.0), Color32::from_rgb(25, 29, 35));
                    if response.clicked() { self.lichess.color = key.into(); }
                }
            });
            ui.add_space(18.0);
            ui.separator();
            ui.add_space(14.0);
            let valid = !custom || Board::from_str(self.lichess.ai_fen.trim()).is_ok_and(|b| !b.is_chess960());
            if custom && !valid && !self.lichess.ai_fen.trim().is_empty() {
                ui.colored_label(Color32::from_rgb(235, 130, 120), "Enter a valid standard-chess FEN.");
            }
            if ui.add_enabled(!seeking && valid, egui::Button::new(egui::WidgetText::from(RichText::new("Play against computer").size(17.0).strong().color(Color32::BLACK)).color(Color32::from_rgb(25, 29, 35)))
                .fill(gold).corner_radius(6.0).min_size(Vec2::new(0.0, 42.0)).raised_gold()).clicked() {
                let mut options = serde_json::json!({"level":self.lichess.ai_level, "color":self.lichess.color, "variant":self.lichess.ai_variant});
                if custom { options["fen"] = self.lichess.ai_fen.trim().into(); }
                if self.lichess.ai_time == 1 {
                    options["clock.limit"] = (self.lichess.minutes * 60).into();
                    options["clock.increment"] = self.lichess.increment.into();
                } else if self.lichess.ai_time == 2 { options["days"] = self.lichess.days.into(); }
                lichess_command(serde_json::json!({"type":"ai", "options":options}));
            }
        });
    }

    fn enter_lichess(&mut self) {
        if self.lichess.active {
            return;
        }
        if self.fics_active {
            self.stop_fics();
        }
        // Connecting to the lobby does not replace or lock the local game.
        self.fics_active = false;
        self.fics_previous_workspace_mode = self.workspace_mode;
        self.fics_playing = false;
        self.fics_observing = false;
        self.fics_game_finished = false;
        self.fics_pending_move = false;
        self.fics_game_id = None;
        self.fics_console_open = false;
        self.fics_connected = false;
        self.fics_registered = true;
        self.lichess.active = true;
        self.lichess.applied_moves.clear();
        self.lichess.saved_id.clear();
        self.lichess.game = serde_json::Value::Null;
        self.workspace_mode = WorkspaceMode::Compact;
        self.compact_panel = CompactPanel::Moves;
        self.fics_status = "Connecting to Lichess…".into();
        self.engine_status = self.fics_status.clone();
    }

    fn lichess_can_take_board(&self) -> bool {
        !self.training_live()
            && !(self.fics_active && !self.lichess.active && self.fics_playing && !self.fics_game_finished)
    }

    fn start_lichess(&mut self) {
        // Authentication and lobby access do not claim the board or disconnect FICS.
        self.lichess.open = true;
        if self.lichess.connected() {
            return;
        }
        #[cfg(target_arch = "wasm32")]
        lichess_connect();
    }

    #[cfg(target_arch = "wasm32")]
    fn poll_lichess_play(&mut self, ctx: &egui::Context) {
        for _ in 0..64 {
            let json = lichess_play_poll();
            if json.is_empty() {
                break;
            }
            let Ok(event) = serde_json::from_str::<serde_json::Value>(&json) else {
                continue;
            };
            match event["type"].as_str().unwrap_or("") {
                "lobby" => {
                    self.lichess.lobby = event;
                    if self.lichess.active {
                        self.fics_connected = self.lichess.connected();
                        self.fics_status =
                            self.lichess.lobby["status"].as_str().unwrap_or("").into();
                        self.engine_status = format!("Lichess · {}", self.fics_status);
                    }
                }
                "loading" => {
                    if !self.lichess_can_take_board() { continue; }
                    self.lichess.pending_preview = None;
                    self.enter_lichess();
                    self.fics_active = true;
                    self.settle_training();
                    self.training = None;
                    self.engine_enabled = false;
                    self.fics_game_finished = false;
                    self.fics_playing = false;
                    self.fics_pending_move = true;
                    self.analysis_running = false;
                    self.game_analysis_running = false;
                    self.engine_searching = false;
                    self.resume_engine_after_ready = false;
                    self.pending_analysis_search = None;
                    self.ignore_next_bestmove = true;
                    if let Some(engine) = &self.engine {
                        engine.command("stop");
                    }
                }
                "game" => {
                    if !self.lichess_can_take_board() { continue; }
                    self.enter_lichess();
                    self.receive_lichess_game(event);
                }
                "parked" => {
                    if !self.lichess.active { continue; }
                    self.fics_active = false;
                    self.fics_playing = false;
                    self.fics_game_finished = false;
                    self.fics_pending_move = false;
                    self.lichess.game = serde_json::Value::Null;
                    self.lichess.applied_moves.clear();
                    self.reset_for_side(PlayerSide::White);
                }
                "error" => {
                    if self.lichess.active {
                        self.fics_status = event["message"]
                            .as_str()
                            .unwrap_or("Lichess connection error")
                            .into();
                        self.engine_status = self.fics_status.clone();
                    }
                }
                "moveRejected" => {
                    if event["id"] == self.lichess.game["id"] {
                        self.lichess.pending_preview = None;
                        self.fics_pending_move = false;
                    }
                }
                _ => {}
            }
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(if self.lichess.active {
            100
        } else {
            500
        }));
    }

    fn receive_lichess_game(&mut self, event: serde_json::Value) {
        self.fics_active = true;
        self.settle_training();
        self.training = None;
        let moves_text = event["state"]["moves"].as_str().unwrap_or("");
        let initial = event["initialFen"].as_str().unwrap_or("startpos");
        let Ok((start, parsed)) = lichess_replay_variant(initial, moves_text, event["variant"]["key"].as_str() == Some("chess960")) else {
            self.lichess.pending_preview = None;
            self.fics_status =
                "Lichess move history could not be reconciled. Reconnect to reload the game."
                    .into();
            self.fics_playing = false;
            return;
        };
        let moves: Vec<String> = moves_text.split_whitespace().map(str::to_owned).collect();
        let new_game = self.lichess.game["id"] != event["id"];
        let rebuild = new_game || !moves.starts_with(&self.lichess.applied_moves);
        // A takeback or reconnect can replace any suffix of the history.
        self.engine_enabled = false;
        self.fics_game_finished = false;
        if rebuild {
            let side = if event["side"].as_str() == Some("black") {
                PlayerSide::Black
            } else {
                PlayerSide::White
            };
            self.reset_for_side(side);
            self.board = start;
            self.review_positions = vec![start];
            self.lichess.applied_moves.clear();
        }
        self.review_index = None;
        self.board = *self.review_positions.last().unwrap_or(&start);
        self.review_white_player = lichess_player_name(&event["white"], "White");
        self.review_black_player = lichess_player_name(&event["black"], "Black");
        self.fics_applying_update = true;
        for mv in parsed.into_iter().skip(self.lichess.applied_moves.len()) {
            self.play(mv);
        }
        self.fics_applying_update = false;
        self.lichess.applied_moves = moves;
        self.selected = None;
        self.legal_targets.clear();
        self.promotion = None;
        self.fics_pending_move = event["pendingMove"].as_bool().unwrap_or(false);
        if !self.fics_pending_move {
            if self.lichess.pending_preview.is_some()
                && self.last_move == self.lichess.pending_preview
            {
                // The piece is already previewed at its destination; make it solid in place.
                self.move_animation = None;
            }
            self.lichess.pending_preview = None;
        }
        self.fics_white_time = (event["state"]["wtime"].as_i64().unwrap_or(0) / 1000) as i32;
        self.fics_black_time = (event["state"]["btime"].as_i64().unwrap_or(0) / 1000) as i32;
        self.fics_turn = self.board.side_to_move();
        self.fics_board_at = Self::animation_time();
        let status = event["state"]["status"].as_str().unwrap_or("started");
        let playing = matches!(status, "created" | "started");
        self.fics_playing = playing;
        self.fics_game_finished = !playing;
        self.lichess.game = event.clone();
        if playing {
            self.fics_status = if self.board.side_to_move() == self.player_side.color() {
                "Your move"
            } else {
                "Opponent's move"
            }
            .into();
            Self::set_page_title(false, &self.review_white_player, &self.review_black_player);
        } else {
            let result = match event["state"]["winner"].as_str() {
                Some("white") => "1-0",
                Some("black") => "0-1",
                _ if matches!(
                    status,
                    "draw" | "stalemate" | "outoftime" | "timeout" | "insufficientMaterialClaim"
                ) =>
                {
                    "1/2-1/2"
                }
                _ => "*",
            };
            self.pgn_input = lichess_pgn(&event, &self.review_moves, result);
            self.review_index = Some(self.review_moves.len());
            Self::set_page_title(true, &self.review_white_player, &self.review_black_player);
            self.compact_panel = CompactPanel::Analysis;
            self.fics_status = format!("Game finished · {status} · {result}");
            let id = event["id"].as_str().unwrap_or("");
            if self.lichess.saved_id != id {
                #[cfg(target_arch = "wasm32")]
                start_new_stored_game("mine");
                self.lichess.saved_id = id.into();
                self.save_game();
            }
        }
        self.engine_status = format!("Lichess · {}", self.fics_status);
    }

    fn lichess_preview_board(&self) -> Option<Board> {
        if !self.lichess.active
            || !self.fics_playing
            || !self.fics_pending_move
            || self.review_index.is_some()
            || self.prediction_index != 0
        {
            return None;
        }
        let mv = self.lichess.pending_preview?;
        MoveGen::new_legal(&self.board)
            .any(|legal| legal == mv)
            .then(|| self.board.make_move_new(mv))
    }

    fn paint_lichess_preview(&self, ui: &mut egui::Ui, rect: egui::Rect, cell: f32) {
        let Some(preview) = self.lichess_preview_board() else {
            return;
        };
        for square in chess::ALL_SQUARES {
            if preview.piece_on(square) == self.board.piece_on(square)
                && preview.color_on(square) == self.board.color_on(square)
            {
                continue;
            }
            let (Some(piece), Some(color)) = (preview.piece_on(square), preview.color_on(square))
            else {
                continue;
            };
            let center = if self.board_3d_rendering() {
                crate::board3d::square_center(square, rect, self.flipped, self.board_3d_view())
            } else {
                let file = square.get_file().to_index() as f32;
                let rank = square.get_rank().to_index() as f32;
                Some(
                    rect.left_top()
                        + Vec2::new(
                            ((if self.flipped { 7.0 - file } else { file }) + 0.5) * cell,
                            ((if self.flipped { rank } else { 7.0 - rank }) + 0.5) * cell,
                        ),
                )
            };
            let Some(center) = center else {
                continue;
            };
            let piece_rect = egui::Rect::from_center_size(center, Vec2::splat(cell));
            let mut ghost_ui = ui.new_child(egui::UiBuilder::new().max_rect(piece_rect));
            ghost_ui.set_opacity(0.5);
            Self::paint_piece(&ghost_ui, piece_rect, cell, self.piece_set, color, piece);
        }
    }

    fn lichess_game_actions(&mut self, ui: &mut egui::Ui, include_resign: bool) -> bool {
        let mut clicked = false;
        let available = self.lichess.connected() && self.fics_playing && !self.fics_game_finished;
        ui.add_enabled_ui(available, |ui| {
            let action_button = |ui: &mut egui::Ui, label: &str, action: &str| {
                if !include_resign { return ui.button(label); }
                let (fill, border, text) = if action == "resign" {
                    (Color32::from_rgb(58, 32, 34), Color32::from_rgb(135, 77, 79), Color32::from_rgb(238, 183, 183))
                } else if action.starts_with("draw/") {
                    (Color32::from_rgb(35, 55, 46), Color32::from_rgb(75, 120, 96), Color32::from_rgb(192, 225, 205))
                } else {
                    (Color32::from_rgb(30, 35, 41), Color32::from_rgb(74, 83, 93), Color32::from_rgb(219, 223, 227))
                };
                ui.add(egui::Button::new(RichText::new(label).size(14.0).color(text))
                    .fill(fill).stroke(Stroke::new(1.0, border)).corner_radius(6.0)
                    .min_size(Vec2::new(0.0, 34.0)))
            };
            if include_resign {
                ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
                ui.spacing_mut().button_padding = Vec2::new(12.0, 7.0);
            }
            if include_resign && action_button(ui, "Resign…", "resign").clicked() {
                #[cfg(target_arch="wasm32")]
                if lobby_companion_mode() { lobby_action_in_main("resign"); }
                else { self.fics_resign_dialog_open = true; }
                #[cfg(not(target_arch="wasm32"))]
                { self.fics_resign_dialog_open = true; }
                clicked = true;
            }
            let draw_key = if self.player_side == PlayerSide::White { "bdraw" } else { "wdraw" };
            let takeback_key = if self.player_side == PlayerSide::White { "btakeback" } else { "wtakeback" };
            let draw = self.lichess.game["state"][draw_key].as_bool() == Some(true);
            let takeback = self.lichess.game["state"][takeback_key].as_bool() == Some(true);
            let mut actions = vec![
                ("Abort game", "abort"),
                (if draw { "Accept draw" } else { "Offer draw" }, "draw/yes"),
                (if takeback { "Accept takeback" } else { "Request takeback" }, "takeback/yes"),
                ("Claim draw", "claim-draw"),
            ];
            if draw { actions.push(("Decline draw", "draw/no")); }
            if takeback { actions.push(("Decline takeback", "takeback/no")); }
            if self.lichess.lobby["opponentGone"]["gone"].as_bool() == Some(true) {
                actions.push(("Claim victory", "claim-victory"));
            }
            for (label, action) in actions {
                if action_button(ui, label, action).clicked() {
                    lichess_command(serde_json::json!({"type":"action", "action":action}));
                    clicked = true;
                }
            }
        });
        clicked
    }

    fn lichess_chat_panel(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.label(RichText::new("Game chat").size(19.0).strong());
        let opponent = if self.player_side == PlayerSide::White {
            &self.review_black_player
        } else {
            &self.review_white_player
        };
        ui.label(RichText::new(format!("With {opponent}")).weak());
        ui.separator();
        let chat = self.lichess.lobby["chat"].as_array().cloned().unwrap_or_default();
        let height = (ui.available_height() - 90.0).max(60.0);
        egui::ScrollArea::vertical().id_salt("lichess_board_chat")
            .max_height(height).min_scrolled_height(height)
            .auto_shrink([false, false]).stick_to_bottom(true).show(ui, |ui| {
                if chat.is_empty() {
                    ui.add_space(14.0);
                    ui.label(RichText::new("No messages yet. Say hello to your opponent.").weak());
                }
                for line in chat {
                    let sender = line["user"].as_str().unwrap_or("");
                    Frame::new().fill(Color32::from_rgb(24, 28, 33))
                        .corner_radius(6.0).inner_margin(10.0).show(ui, |ui| {
                            ui.set_width((ui.available_width() - 4.0).max(40.0));
                            ui.label(RichText::new(sender).small().strong().color(Color32::from_rgb(211, 173, 98)));
                            ui.label(line["text"].as_str().unwrap_or(""));
                        });
                    ui.add_space(6.0);
                }
            });
        ui.separator();
        ui.add_enabled_ui(self.lichess.connected(), |ui| {
            let input = ui.add(egui::TextEdit::singleline(&mut self.lichess.chat_input)
                .char_limit(140).desired_width(f32::INFINITY)
                .margin(Vec2::new(10.0, 8.0)).hint_text("Message your opponent"));
            let enter = input.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            let send = ui.add_enabled(!self.lichess.chat_input.trim().is_empty(), egui::Button::new("Send"))
                .clicked();
            if (send || enter) && !self.lichess.chat_input.trim().is_empty() {
                lichess_command(serde_json::json!({"type":"action", "action":"chat", "text":self.lichess.chat_input.trim()}));
                self.lichess.chat_input.clear();
                input.request_focus();
            }
        });
        if !self.lichess.connected() { ui.label(RichText::new("Reconnect to send messages.").small().weak()); }
    }

    fn lichess_account_link(ui: &mut egui::Ui) {
        if ui.link("Lichess account").clicked() {
            ui.ctx().open_url(egui::OpenUrl::new_tab(
                "https://lichess.org/account/oauth/token",
            ));
        }
    }

    #[cfg(target_arch="wasm32")]
    fn poll_lobby_companion(&mut self, ctx: &egui::Context) {
        if let Ok(actions) = serde_json::from_str::<Vec<String>>(&take_lobby_actions()) {
            for action in actions {
                match action.as_str() {
                    "leave" => { if self.lichess.active { self.stop_fics(); } },
                    "profile" => self.open_lichess_history(),
                    "resign" if self.lichess.active && self.fics_playing => self.fics_resign_dialog_open = true,
                    _ => {},
                }
            }
        }
        if lobby_companion_open() {
            publish_lobby(&serde_json::json!({
                "lobby":self.lichess.lobby,"game":self.lichess.game,"active":self.lichess.active,
                "playing":self.fics_playing,"finished":self.fics_game_finished,"pending":self.fics_pending_move,
                "status":self.fics_status,"white":self.review_white_player,"black":self.review_black_player,
                "side":if self.player_side == PlayerSide::White { "white" } else { "black" },
            }).to_string());
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }

    #[cfg(target_arch="wasm32")]
    fn lichess_lobby_companion_ui(&mut self, ui: &mut egui::Ui) {
        let state = read_lobby();
        if let Ok(state) = serde_json::from_str::<serde_json::Value>(&state) {
            self.lichess.lobby = state["lobby"].clone();
            self.lichess.game = state["game"].clone();
            self.lichess.active = state["active"].as_bool().unwrap_or(false);
            self.fics_playing = state["playing"].as_bool().unwrap_or(false);
            self.fics_game_finished = state["finished"].as_bool().unwrap_or(false);
            self.fics_pending_move = state["pending"].as_bool().unwrap_or(false);
            self.fics_status = state["status"].as_str().unwrap_or_default().to_owned();
            self.review_white_player = state["white"].as_str().unwrap_or_default().to_owned();
            self.review_black_player = state["black"].as_str().unwrap_or_default().to_owned();
            self.player_side = if state["side"] == "black" { PlayerSide::Black } else { PlayerSide::White };
            self.lichess.open = true;
            self.lichess_ui(ui.ctx());
        } else { ui.label("Keep the main Ironwood window open to use the Lichess lobby."); }
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
    }

    fn lichess_ui(&mut self, ctx: &egui::Context) {
        self.lichess_history_ui(ctx);
        if !self.lichess.open {
            return;
        }
        let mut open = true;
        let mut close_after = false;
        let connected = self.lichess.connected();
        let width = (ctx.content_rect().width() - 64.0).clamp(280.0, if connected { 760.0 } else { 600.0 });
        let mut detached = false;
        let lobby_window = egui::Window::new("Lichess lobby").title_bar(false).collapsible(false)
            .drag_area(egui::WindowDrag::Anywhere).resizable(true)
            .default_pos(egui::pos2(48.0,48.0))
            .default_size(Vec2::new(width, 800.0))
            .min_size(Vec2::new(380.0,340.0))
            .max_width((ctx.content_rect().width()-40.0).max(280.0))
            .max_height((ctx.content_rect().height()-40.0).max(240.0))
            .frame(Self::dialog_frame().shadow(egui::epaint::Shadow {
                offset: [4,6], blur: 14, spread: 2, color: Color32::from_black_alpha(160),
            })).show(ctx, |ui| {
            let grain_background = ui.painter().add(egui::Shape::Noop);
            ui.horizontal(|ui| {
                ui.add_sized([(ui.available_width()-82.0).max(80.0),28.0],
                    egui::Label::new(RichText::new("Lichess lobby").size(21.0).strong()).selectable(false).truncate());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if Self::glossy_board_button(ui,"×",false,true).on_hover_text("Close Lichess lobby").clicked() { open=false; }
                    #[cfg(target_arch="wasm32")]
                    if !lobby_companion_mode() && Self::glossy_board_button(ui,"↗",false,true)
                        .on_hover_text("Move Lichess lobby to a separate window").clicked() { detached=open_lobby_companion(); }
                });
            });
            ui.spacing_mut().item_spacing = Vec2::new(10.0, 10.0);
            ui.spacing_mut().button_padding = Vec2::new(12.0, 7.0);
            let status = self.lichess.lobby["status"].as_str().unwrap_or("Sign in to play on Lichess").to_owned();
            if connected && status != "Connected to Lichess" { ui.label(&status); }
            #[cfg(not(target_arch = "wasm32"))]
            ui.label("Lichess play is available in the browser app.");
            if !self.lichess.connected() {
                let gold = Color32::from_rgb(211, 173, 98);
                let muted = Color32::from_rgb(175, 182, 191);
                ui.add_space(12.0);
                let sign_in_frame = Frame::new().fill(Color32::from_rgb(13, 17, 22))
                    .stroke(Stroke::new(1.0, Color32::from_white_alpha(25)))
                    .corner_radius(6.0).inner_margin(22.0).show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.label(RichText::new("PLAY ONLINE").size(12.0).strong().color(gold));
                ui.add_space(6.0);
                ui.label(RichText::new("Your next opponent awaits.").size(28.0).strong());
                ui.label(RichText::new("Connect for Opening Explorer and online play. Your FICS connection and current board stay in place until you open a Lichess game.").size(16.0).color(muted));
                ui.add_space(12.0);
                ui.horizontal_wrapped(|ui| {
                    for label in ["Rapid & classical", "Correspondence", "Friend challenges"] {
                        Frame::new().fill(Color32::from_rgb(34, 41, 47)).corner_radius(5.0)
                            .stroke(Stroke::new(1.0, gold.gamma_multiply(0.45))).inner_margin(7.0).show(ui, |ui| { ui.label(RichText::new(label).size(13.0).color(muted)); });
                    }
                });
                ui.add_space(18.0);
                if ui.add_sized([ui.available_width(), 46.0], egui::Button::new(egui::WidgetText::from(
                    RichText::new("Sign in with Lichess").size(17.0).strong().color(Color32::from_rgb(24, 27, 31))).color(Color32::from_rgb(25, 29, 35)))
                    .fill(gold).corner_radius(7.0).raised_gold()).clicked() {
                    self.enter_lichess();
                    #[cfg(target_arch = "wasm32")]
                    lichess_sign_in();
                }
                if ui.add_sized([ui.available_width(), 38.0], egui::Button::new(egui::WidgetText::from(RichText::new("Reconnect saved session").size(14.0)).color(Color32::from_rgb(25, 29, 35)))
                    .fill(Color32::TRANSPARENT).stroke(Stroke::new(1.0, Color32::from_rgb(80, 84, 88))).corner_radius(7.0).raised_gold()).clicked() { self.start_lichess(); }
                Self::lichess_account_link(ui);
                ui.add_space(6.0);
                ui.label(RichText::new(status).size(13.0).color(muted));
                });
                Self::paint_sunken_border(ui, sign_in_frame.response.rect);
                ui.add_space(12.0);
                ui.label(RichText::new("Sign-in lasts for this browser session.").size(13.0).color(muted));
                ui.label(RichText::new("Fair play · Engine assistance is disabled during online play.").size(13.0).color(muted));
            } else {
                let account_panel = Frame::new().fill(Color32::from_rgb(13,17,22)).corner_radius(6.0).inner_margin(12.0).show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    let green = Color32::from_rgb(136, 194, 151);
                    let (indicator, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                    ui.painter().circle_filled(indicator.center(), 4.0, green);
                    ui.label(RichText::new(self.lichess.lobby["account"].as_str().unwrap_or("Connected"))
                        .size(22.0).strong().color(green));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.add(egui::Button::new(egui::WidgetText::from("Player profile").color(Color32::from_rgb(25, 29, 35))).raised_gold()).clicked() {
                            #[cfg(target_arch="wasm32")]
                            if lobby_companion_mode() { lobby_action_in_main("profile"); }
                            else { self.open_lichess_history(); }
                            #[cfg(not(target_arch="wasm32"))]
                            self.open_lichess_history();
                        }
                        Self::lichess_account_link(ui);
                    });
                });
                });
                Self::paint_sunken_border(ui, account_panel.response.rect);
                ui.add_space(2.0);
                let content_height = (ui.available_height()-64.0).max(100.0);
                egui::ScrollArea::vertical().id_salt("lichess_lobby_content").max_height(content_height).min_scrolled_height(content_height).auto_shrink([false, false]).show(ui, |ui| {
                Self::import_sunken_section(ui, true, |ui| {


                    let games = self.lichess.lobby["games"].as_array().cloned().unwrap_or_default();
                    if !games.is_empty() { ui.separator(); ui.strong("Your ongoing games"); }
                    for game in games {
                        let id = game["gameId"].as_str().unwrap_or("");
                        let variant = game["variant"]["key"].as_str().unwrap_or("standard");
                        let supported = matches!(variant, "standard" | "fromPosition" | "chess960")
                            && game["compat"]["board"].as_bool() != Some(false)
                            && !matches!(game["speed"].as_str(), Some("bullet" | "ultraBullet"));
                        let mut details = vec![game["speed"].as_str().unwrap_or("").to_owned()];
                        if let Some(rated) = game["rated"].as_bool() { details.push(if rated { "Rated" } else { "Casual" }.into()); }
                        if let Some(color) = game["color"].as_str() { details.push(format!("You play {color}")); }
                        if let Some(turn) = game["isMyTurn"].as_bool() { details.push(if turn { "Your turn" } else { "Opponent's turn" }.into()); }
                        if let Some(days) = game["daysPerTurn"].as_u64() { details.push(format!("{days} days per move")); }
                        else if let (Some(initial), Some(increment)) = (game["clock"]["initial"].as_u64(), game["clock"]["increment"].as_u64()) { details.push(format!("{}+{increment}", initial / 60)); }
                        let mut label = format!("{}\n{}", game["opponent"]["username"].as_str().unwrap_or("Opponent"), details.join(" · "));
                        if let Some(started) = game["startedLabel"].as_str() { label.push_str(&format!("\nStarted {started}")); }
                        if ui.add_enabled(supported, egui::Button::new(egui::WidgetText::from(RichText::new(label).size(15.0)).color(Color32::from_rgb(25, 29, 35))).min_size(Vec2::new(ui.available_width(), 58.0)).raised_gold()).on_hover_text(format!("Resume game {id}")).clicked() {
                            lichess_command(serde_json::json!({"type":"open", "id":id}));
                        }
                        if !supported { ui.hyperlink_to("Open game on Lichess", format!("https://lichess.org/{id}")); }
                    }
                    if self.fics_playing && self.lichess.active {
                        if self.lichess.game["speed"].as_str() == Some("correspondence") {
                            if ui.add_enabled(!self.fics_pending_move, egui::Button::new(egui::WidgetText::from("Return to lobby · keep this game ongoing").color(Color32::from_rgb(25, 29, 35))).raised_gold()).clicked() {
                                lichess_command(serde_json::json!({"type":"leaveCorrespondence"}));
                            }
                            ui.label("Your move deadline continues. Resume this game from Your ongoing games.");
                        }
                        ui.separator();
                        ui.strong(format!("{} vs {}", self.review_white_player, self.review_black_player));
                        ui.label(&self.fics_status);
                        ui.label(if self.lichess.game["rated"].as_bool() == Some(true) { "Rated game" } else { "Casual game" });
                        ui.horizontal_wrapped(|ui| { self.lichess_game_actions(ui, true); });
                        ui.label(RichText::new("Chat with your opponent in the panel beside the board.").weak());
                    }
                    if !self.fics_playing {
                        let seeking = self.lichess.lobby["seeking"].as_bool().unwrap_or(false);
                        let mut selected = self.lichess.lobby_tab;
                        ui.horizontal_wrapped(|ui| {
                            for (index,label) in ["Quick pairing", "Create a game", "Correspondence"].iter().enumerate() {
                                if Self::fics_primary_tab(ui,label,self.lichess.lobby_tab == index).clicked() { selected = index; }
                            }
                        });
                        if selected != self.lichess.lobby_tab {
                            self.lichess.lobby_tab = selected;
                            self.lichess.correspondence = selected == 2;
                        }
                        ui.separator();
                        if self.lichess.lobby_tab == 0 {
                            ui.horizontal_wrapped(|ui| {
                                ui.vertical(|ui| {
                                    let grid_width = ui.available_width().min(460.0);
                                    ui.set_width(grid_width);
                                    let columns = if grid_width < 420.0 { 2 } else { 3 };
                                    let button_width = (grid_width - (columns-1) as f32 * 10.0) / columns as f32;
                                    egui::Grid::new("lichess_quick_pairing").spacing(Vec2::splat(10.0)).show(ui, |ui| {
                                        for (index, (minutes, increment, speed)) in [
                                            (10, 0, "Rapid"), (10, 5, "Rapid"), (15, 10, "Rapid"),
                                            (30, 0, "Classical"), (30, 20, "Classical"), (0, 0, "Custom"),
                                        ].into_iter().enumerate() {
                                            let label = if minutes == 0 { "Custom".to_owned() } else { format!("{minutes}+{increment}\n{speed}") };
                                            if ui.add_enabled(!seeking, egui::Button::new(egui::WidgetText::from(RichText::new(label).size(23.0).strong().color(Color32::from_rgb(25, 29, 35))).color(Color32::from_rgb(25, 29, 35)))
                                                .min_size(Vec2::new(button_width, 100.0)).fill(Color32::from_rgb(211, 173, 98))
                                                .corner_radius(7.0).raised_gold()).clicked() {
                                                if minutes == 0 { self.lichess.lobby_tab = 1; }
                                                else {
                                                    self.lichess.minutes = minutes;
                                                    self.lichess.increment = increment;
                                                    self.lichess.correspondence = false;
                                                    lichess_command(serde_json::json!({"type":"seek", "options":{
                                                        "time":minutes, "increment":increment, "color":"random", "rated":self.lichess.rated
                                                    }}));
                                                }
                                            }
                                            if (index+1) % columns == 0 { ui.end_row(); }
                                        }
                                    });
                                    ui.add_space(8.0);
                                    ui.checkbox(&mut self.lichess.rated, "Rated games");
                                    ui.label(RichText::new("Quick pairing supports rapid and classical games.").size(15.0).color(Color32::from_rgb(185, 190, 195)));
                                });
                                ui.vertical(|ui| {
                                    ui.set_width(210.0);
                                    ui.add_space(36.0);
                                    for (tab, label) in [(1, "Create a game"), (3, "Challenge a friend"), (4, "Play against computer")] {
                                        if ui.add_sized([210.0, 46.0], egui::Button::new(egui::WidgetText::from(RichText::new(label).size(15.0)).color(Color32::from_rgb(25, 29, 35))).corner_radius(6.0).raised_gold()).clicked() {
                                            self.lichess.lobby_tab = tab;
                                            self.lichess.correspondence = false;
                                        }
                                    }
                                });
                            });
                        }
                        if seeking {
                            ui.label("Finding an opponent…");
                            if ui.add(egui::Button::new(egui::WidgetText::from("Cancel matchmaking").color(Color32::from_rgb(25, 29, 35))).raised_gold()).clicked() { lichess_command(serde_json::json!({"type":"cancelSeek"})); }
                        }
                        if self.lichess.lobby_tab == 4 { self.lichess_computer_setup(ui, seeking); }
                        if self.lichess.lobby_tab != 0 && self.lichess.lobby_tab != 4 {
                        if self.lichess.lobby_tab > 2 {
                            ui.label(RichText::new(if self.lichess.lobby_tab == 3 { "Challenge a friend" } else { "Play against computer" }).size(20.0).strong());
                            ui.checkbox(&mut self.lichess.correspondence, "Correspondence");
                        }
                        if self.lichess.lobby_tab == 2 {
                            ui.add_space(8.0);
                            ui.label(RichText::new("Play at your own pace").size(22.0).strong());
                            ui.label(RichText::new("Make a move, then come back later. You don't need to be online together.").weak());
                            ui.add_space(14.0);
                            ui.label(RichText::new("Time for each move").size(15.0).strong());
                            ui.horizontal_wrapped(|ui| {
                                for days in [1, 2, 3, 5, 7, 10, 14] {
                                    let selected = self.lichess.days == days;
                                    let response = ui.add_sized([84.0, 66.0], egui::Button::new(egui::WidgetText::from(RichText::new(format!("{days}\n{}", if days == 1 { "day" } else { "days" })).size(18.0)).color(Color32::from_rgb(25, 29, 35)))
                                        .fill(if selected { Color32::from_rgb(58, 49, 32) } else { Color32::from_rgb(43, 43, 40) })
                                        .stroke(Stroke::new(1.0, if selected { Color32::from_rgb(211, 173, 98) } else { Color32::from_white_alpha(28) }))
                                        .corner_radius(6.0).raised_gold());
                                    Self::lichess_selection_border(ui, &response, selected);
                                    if response.clicked() { self.lichess.days = days; }
                                }
                            });
                            ui.add_space(10.0);
                            ui.label(RichText::new("Game mode").size(15.0).strong());
                            ui.horizontal(|ui| {
                                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                                    ui.selectable_value(&mut self.lichess.rated, false, RichText::new("Casual").size(15.0));
                                    ui.selectable_value(&mut self.lichess.rated, true, RichText::new("Rated").size(15.0));
                                });
                            });
                            ui.add_space(8.0);
                            ui.label(RichText::new(format!("Up to {} hours for each reply. The deadline keeps running while you're away.", self.lichess.days * 24)).size(14.0).color(Color32::from_rgb(185, 190, 195)));
                            ui.label(RichText::new("Resume from Your ongoing games in this lobby. Colors are assigned automatically.").size(14.0).color(Color32::from_rgb(185, 190, 195)));
                            ui.add_space(14.0);
                        } else {
                        ui.add_space(8.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(18.0, 12.0);
                            ui.spacing_mut().interact_size.y = 34.0;
                            ui.spacing_mut().button_padding.y = 7.0;
                            if self.lichess.correspondence {
                                egui::ComboBox::from_id_salt("lichess_days").selected_text(format!("{} days per move", self.lichess.days)).show_ui(ui, |ui| {
                                    for days in [1, 2, 3, 5, 7, 10, 14] { ui.selectable_value(&mut self.lichess.days, days, days.to_string()); }
                                });
                            } else {
                                ui.add_sized([80.0, 34.0], egui::DragValue::new(&mut self.lichess.minutes).range(1..=180).suffix(" min"));
                                ui.add_sized([130.0, 34.0], egui::DragValue::new(&mut self.lichess.increment).range(0..=60).suffix(" sec increment"));
                            }
                            ui.checkbox(&mut self.lichess.rated, "Rated");
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;
                            ui.label("Play as");
                            egui::ComboBox::from_id_salt("lichess_color").selected_text(&self.lichess.color).show_ui(ui, |ui| {
                                for color in ["random", "white", "black"] { ui.selectable_value(&mut self.lichess.color, color.into(), color); }
                            });
                            });
                        });
                        ui.add_space(12.0);
                        }
                        if !seeking && self.lichess.lobby_tab <= 2 {
                            let rapid = self.lichess.correspondence || self.lichess.minutes * 60 + self.lichess.increment * 40 >= 480;
                            if ui.add_enabled(rapid, egui::Button::new(egui::WidgetText::from(RichText::new(if self.lichess.lobby_tab == 2 { "Find a correspondence opponent" } else { "Find opponent" }).size(16.0).strong().color(Color32::from_rgb(24, 27, 31))).color(Color32::from_rgb(25, 29, 35))).fill(Color32::from_rgb(211, 173, 98)).min_size(Vec2::new(0.0, 40.0)).raised_gold()).clicked() {
                                let options = if self.lichess.correspondence { serde_json::json!({"days":self.lichess.days, "rated":self.lichess.rated}) }
                                else { serde_json::json!({"time":self.lichess.minutes, "increment":self.lichess.increment, "color":self.lichess.color, "rated":self.lichess.rated}) };
                                lichess_command(serde_json::json!({"type":"seek", "options":options}));
                            }
                            if !rapid { ui.label("Matchmaking requires rapid or slower. Use a direct challenge for blitz."); }
                        }
                        if self.lichess.lobby_tab == 3 {
                        ui.add_space(6.0);
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.add(egui::TextEdit::singleline(&mut self.lichess.opponent).desired_width(300.0).margin(Vec2::new(10.0, 8.0)).hint_text("Lichess username"));
                            if ui.add_enabled(!seeking && !self.lichess.opponent.trim().is_empty(), egui::Button::new(egui::WidgetText::from("Challenge").color(Color32::from_rgb(25, 29, 35))).raised_gold()).clicked() {
                                let options = if self.lichess.correspondence { serde_json::json!({"days":self.lichess.days, "rated":self.lichess.rated, "color":self.lichess.color}) }
                                else { serde_json::json!({"clock.limit":self.lichess.minutes * 60, "clock.increment":self.lichess.increment, "color":self.lichess.color, "rated":self.lichess.rated}) };
                                lichess_command(serde_json::json!({"type":"challenge", "username":self.lichess.opponent.trim(), "options":options}));
                            }
                        });
                        ui.small("Direct challenges expire after 20 seconds if not accepted.");
                        }

                        }
                    }
                    let challenges = self.lichess.lobby["challenges"].as_array().cloned().unwrap_or_default();
                    for challenge in challenges {
                        ui.separator();
                        let variant = challenge["variant"]["key"].as_str().unwrap_or("");
                        let speed = challenge["speed"].as_str().unwrap_or("");
                        ui.label(format!("{} challenges you · {} · {} · {}", challenge["challenger"]["name"].as_str().unwrap_or("Player"), variant, speed,
                            if challenge["rated"].as_bool() == Some(true) { "rated" } else { "casual" }));
                        if let (Some(limit), Some(increment)) = (challenge["clock"]["limit"].as_i64(), challenge["clock"]["increment"].as_i64()) {
                            ui.label(format!("{}+{} · your color: {}", limit / 60, increment, match challenge["color"].as_str() { Some("white") => "black", Some("black") => "white", _ => "random" }));
                        } else if let Some(days) = challenge["daysPerTurn"].as_i64() { ui.label(format!("{days} days per move")); }
                        let supported = matches!(variant, "standard" | "fromPosition" | "chess960") && !matches!(speed, "bullet" | "ultraBullet");
                        ui.horizontal(|ui| {
                            for action in ["accept", "decline"] {
                                if ui.add_enabled(action == "decline" || (supported && !self.fics_playing), egui::Button::new(egui::WidgetText::from(action).color(Color32::from_rgb(25, 29, 35))).raised_gold()).clicked() {
                                    lichess_command(serde_json::json!({"type":"challengeAction", "id":challenge["id"], "action":action}));
                                }
                            }
                        });
                    }
                    let outgoing = self.lichess.lobby["outgoing"].as_array().cloned().unwrap_or_default();
                    for challenge in outgoing {
                        ui.horizontal(|ui| {
                            ui.label(format!("Challenge pending: {}", challenge["destUser"]["name"].as_str().unwrap_or("opponent")));
                            if ui.add(egui::Button::new(egui::WidgetText::from("Cancel challenge").color(Color32::from_rgb(25, 29, 35))).raised_gold()).clicked() { lichess_command(serde_json::json!({"type":"challengeAction", "id":challenge["id"], "action":"cancel"})); }
                        });
                    }
                    if let Some(id) = self.lichess.game["id"].as_str() {
                        ui.add_space(8.0);
                        if ui.link("Open game on Lichess").clicked() { ui.ctx().open_url(egui::OpenUrl::new_tab(format!("https://lichess.org/{id}"))); }
                    }
                });
                });
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
            if ui.add(egui::Button::new(egui::WidgetText::from("Leave online play").color(Color32::from_rgb(25, 29, 35))).raised_gold()).clicked() {
                #[cfg(target_arch="wasm32")]
                if lobby_companion_mode() { lobby_action_in_main("leave"); }
                else if self.lichess.active { self.stop_fics(); }
                #[cfg(not(target_arch="wasm32"))]
                if self.lichess.active { self.stop_fics(); }
                close_after = true;
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if self.lichess.connected() && ui.add_enabled(!self.fics_playing, egui::Button::new(egui::WidgetText::from("Sign out").color(Color32::from_rgb(25, 29, 35))).raised_gold()).clicked() {
                    #[cfg(target_arch = "wasm32")]
                    lichess_sign_out();
                }
            });
            });
            let paper = Self::paper_grain_texture(ctx);
            let surface = ui.min_rect().expand(6.0);
            ui.painter().set(grain_background, egui::Shape::image(paper.id(), surface,
                egui::Rect::from_min_max(egui::Pos2::ZERO,
                    egui::pos2(surface.width() / 256.0, surface.height() / 256.0)), Color32::WHITE));
        });
        if let Some(window) = lobby_window {
            Self::paint_fics_window_border(&ctx.layer_painter(window.response.layer_id), window.response.rect);
        }
        self.lichess.open = open && !close_after && !detached;
        #[cfg(target_arch="wasm32")]
        if lobby_companion_mode() && (!open || close_after) { close_lobby_companion(); }
    }
}

#[cfg(test)]
mod lichess_tests {
    use super::*;
    #[test]
    fn lichess_lobby_opens_with_or_without_an_active_game() {
        let ctx = egui::Context::default();
        let mut app = ChessApp::new(&eframe::CreationContext::_new_kittest(ctx));
        app.lichess.lobby = serde_json::json!({"connected": true, "account": "Player"});
        let lobby = app.lichess.lobby.clone();
        for active in [false, true] {
            app.lichess.active = active;
            app.lichess.open = false;
            app.start_lichess();
            assert!(app.lichess.open);
            assert_eq!(app.lichess.lobby, lobby);
            assert_eq!(app.lichess.active, active);
        }
        app.lichess.lobby = serde_json::Value::Null;
        app.lichess.open = false;
        app.start_lichess();
        assert!(app.lichess.open);
    }

    #[test]
    fn lichess_account_connection_preserves_fics_board_and_connection() {
        let mut app = app();
        app.fics_active = true;
        app.fics_connected = true;
        app.fics_playing = true;
        app.fics_game_finished = false;
        app.fics_game_id = Some(42);
        app.board = app.board.make_move_new(ChessMove::from_str("e2e4").unwrap());
        let board = app.board;
        app.start_lichess();
        assert!(app.fics_active && app.fics_connected && app.fics_playing);
        assert_eq!(app.fics_game_id, Some(42));
        assert_eq!(app.board, board);
        assert!(!app.lichess.active);
        assert!(!app.lichess_can_take_board());
        app.fics_game_finished = true;
        assert!(app.lichess_can_take_board());
    }

    #[test]
    fn lichess_lobby_preserves_local_game_until_online_game_starts() {
        let mut app = app();
        app.engine_enabled = true;
        app.board = app.board.make_move_new(ChessMove::from_str("e2e4").unwrap());
        let local_board = app.board;
        app.enter_lichess();
        assert_eq!(app.board, local_board);
        assert!(app.engine_enabled);
        assert!(!app.fics_active);
        app.stop_fics();
        assert_eq!(app.board, local_board);
        app.enter_lichess();
        app.receive_lichess_game(event("", "started"));
        assert!(app.fics_active);
        assert!(!app.engine_enabled);
        assert_eq!(app.board, Board::default());
    }
    #[test]
    fn lichess_ghost_waits_for_confirmation_without_changing_the_game() {
        let mut app = app();
        app.enter_lichess();
        app.lichess.lobby = serde_json::json!({"connected":true});
        app.receive_lichess_game(event("", "started"));
        let mv = ChessMove::from_str("e2e4").unwrap();
        app.play(mv);
        assert_eq!(app.board, Board::default());
        assert!(app.review_moves.is_empty());
        assert_eq!(
            app.lichess_preview_board().unwrap().piece_on(Square::E4),
            Some(Piece::Pawn)
        );
        let mut heartbeat = event("", "started");
        heartbeat["pendingMove"] = true.into();
        app.receive_lichess_game(heartbeat);
        assert!(app.lichess_preview_board().is_some());
        app.receive_lichess_game(event("e2e4", "started"));
        assert_eq!(app.review_moves, ["e4"]);
        assert!(app.lichess_preview_board().is_none());
        assert!(app.move_animation.is_none());
    }
    #[test]
    fn lichess_ghost_previews_special_moves_and_clears_on_reset() {
        for (fen, uci, expected_square, expected_piece) in [
            (
                "4k3/P7/8/8/8/8/8/4K3 w - - 0 1",
                "a7a8n",
                Square::A8,
                Piece::Knight,
            ),
            (
                "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1",
                "e1g1",
                Square::F1,
                Piece::Rook,
            ),
            (
                "4k3/8/8/3p4/4P3/8/8/4K3 w - - 0 1",
                "e4d5",
                Square::D5,
                Piece::Pawn,
            ),
        ] {
            let mut app = app();
            app.enter_lichess();
            let mut initial = event("", "started");
            initial["initialFen"] = fen.into();
            app.receive_lichess_game(initial);
            app.lichess.lobby = serde_json::json!({"connected":true});
            let official = app.board;
            app.play(ChessMove::from_str(uci).unwrap());
            assert_eq!(app.board, official);
            let preview = app.lichess_preview_board().unwrap();
            assert_eq!(preview.piece_on(expected_square), Some(expected_piece));
            assert_eq!(preview.color_on(expected_square), Some(Color::White));
            app.stop_fics();
            assert!(app.lichess.pending_preview.is_none());
            assert!(app.lichess_preview_board().is_none());
        }
    }
    fn app() -> ChessApp {
        ChessApp::new(&eframe::CreationContext::_new_kittest(
            egui::Context::default(),
        ))
    }
    fn event(moves: &str, status: &str) -> serde_json::Value {
        serde_json::json!({"id":"abcd1234", "initialFen":"startpos", "side":"white", "rated":true,
            "white":{"name":"Alice", "rating":1500}, "black":{"name":"Bob", "rating":1600},
            "state":{"moves":moves, "status":status, "wtime":598000, "btime":595000, "winner":"white"}})
    }
    #[test]
    fn lichess_castling_en_passant_and_promotion_replay() {
        let (_, castle) =
            lichess_replay("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1", "e1h1 e8a8").unwrap();
        assert_eq!(
            castle.iter().map(ToString::to_string).collect::<Vec<_>>(),
            ["e1g1", "e8c8"]
        );
        let (_, ep) = lichess_replay("startpos", "e2e4 a7a6 e4e5 d7d5 e5d6").unwrap();
        assert_eq!(ep.len(), 5);
        let (_, promotion) = lichess_replay("4k3/P7/8/8/8/8/8/4K3 w - - 0 1", "a7a8n").unwrap();
        assert_eq!(promotion[0].get_promotion(), Some(Piece::Knight));
        assert!(lichess_replay("startpos", "e2e5").is_err());
    }
    #[test]
    fn lichess_takeback_and_full_reconnect_replace_the_authoritative_history() {
        let mut app = app();
        app.enter_lichess();
        app.receive_lichess_game(event("e2e4 e7e5 g1f3", "started"));
        assert_eq!(app.review_moves, ["e4", "e5", "Nf3"]);
        app.receive_lichess_game(event("e2e4 e7e5", "started"));
        assert_eq!(app.review_moves, ["e4", "e5"]);
        app.receive_lichess_game(event("e2e4 e7e5 f1c4 b8c6", "started"));
        assert_eq!(app.review_moves, ["e4", "e5", "Bc4", "Nc6"]);
        assert!(!app.engine_enabled);
        assert!(app.fics_playing);
        let board = app.board;
        app.receive_lichess_game(event("e2e5", "started"));
        assert_eq!(app.board, board);
        assert!(!app.fics_playing);
    }
    #[test]
    fn lichess_finished_game_keeps_server_result_and_can_reopen_for_review() {
        let mut app = app();
        app.enter_lichess();
        app.receive_lichess_game(event("e2e4 e7e5 g1f3", "resign"));
        assert!(app.fics_game_finished);
        assert!(!app.fics_playing);
        assert_eq!(app.game_result(), "1-0");
        let (positions, moves) = ChessApp::parse_pgn_mainline(&app.pgn_input).unwrap();
        assert_eq!(moves, app.review_moves);
        assert_eq!(positions.last(), Some(&app.board));
        assert_eq!(
            ChessApp::pgn_tag(&app.pgn_input, "Site").as_deref(),
            Some("https://lichess.org/abcd1234")
        );
        app.stop_fics();
        assert_eq!(app.board, *positions.last().unwrap());
    }
    #[test]
    fn lichess_disconnect_discards_unfinished_online_position() {
        let mut app = app();
        app.enter_lichess();
        app.receive_lichess_game(event("e2e4 e7e5", "started"));
        app.stop_fics();
        assert!(!app.lichess.active);
        assert!(!app.fics_active);
        assert_eq!(app.board, Board::default());
        assert!(app.review_moves.is_empty());
    }
    #[test]
    fn lichess_custom_position_pgn_uses_black_to_move_and_correct_move_number() {
        let initial = "4k3/8/8/8/8/8/8/4K3 b - - 0 37";
        let mut game = event("e8d7", "draw");
        game["initialFen"] = initial.into();
        let pgn = lichess_pgn(&game, &["Kd7".into()], "1/2-1/2");
        assert!(pgn.contains("37... Kd7"));
        let (positions, moves) = ChessApp::parse_pgn_mainline(&pgn).unwrap();
        assert_eq!(moves, ["Kd7"]);
        assert_eq!(positions[0], Board::from_str(initial).unwrap());
    }
    #[test]
    fn lichess_chess960_replay_preserves_king_to_rook_castling() {
        let (start, moves) = lichess_replay_variant("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1", "e1h1 e8a8", true).unwrap();
        assert!(start.is_chess960());
        assert_eq!(moves[0].to_string(), "e1h1");
        let end = start.make_move_new(moves[0]).make_move_new(moves[1]);
        assert_eq!(end.piece_on(chess::Square::G1), Some(Piece::King));
        assert_eq!(end.piece_on(chess::Square::C8), Some(Piece::King));
    }

}


fn lichess_profile_summary(lobby: &serde_json::Value) -> Option<(String, String)> {
    let profile = lobby.get("profile")?.as_object()?;
    let username = lobby["account"].as_str()?;
    let count = profile.get("count");
    let number = |key: &str| count.and_then(|c| c[key].as_u64());
    let mut parts = vec![username.to_owned()];
    if let Some(all) = number("all") { parts.push(format!("{all} games")); }
    if let (Some(wins), Some(draws), Some(losses)) = (number("win"), number("draw"), number("loss")) {
        parts.push(format!("{wins}W / {draws}D / {losses}L"));
    }
    let mut details = vec!["Lichess profile · overall record includes casual games.".to_owned()];
    if let Some(rated) = number("rated") { details.push(format!("Rated games: {rated}")); }
    for (key,label) in [("bullet","Bullet"),("blitz","Blitz"),("rapid","Rapid"),("classical","Classical"),("correspondence","Correspondence"),("puzzle","Puzzles")] {
        let perf = profile.get("perfs").and_then(|p| p.get(key));
        let games = perf.and_then(|p| p["games"].as_u64()).unwrap_or(0);
        let rating = if games > 0 {
            perf.and_then(|p| p["rating"].as_u64()).map(|rating| format!("{rating}{}", if perf.is_some_and(|p| p["prov"].as_bool() == Some(true)) { "?" } else { "" })).unwrap_or_else(|| "—".into())
        } else { "—".into() };
        if key != "puzzle" { parts.push(format!("{label} {rating}")); }
        details.push(format!("{label}: {rating} · {games} rated games/attempts"));
    }
    if let Some(seconds) = profile.get("playTime").and_then(|p| p["total"].as_u64()) {
        details.push(format!("Playing time: {}h {}m", seconds / 3600, seconds % 3600 / 60));
    }
    details.push("? = provisional rating · — = no rating established. Profile fetched at sign-in/reconnect.".into());
    Some((parts.join(" · "), details.join("\n")))
}


impl ChessApp {
    #[cfg(target_arch = "wasm32")]
    fn lichess_profile_companion_ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        if let Ok(lobby) = serde_json::from_str(&read_profile_state()) { self.lichess.lobby = lobby; }
        let initialized = egui::Id::new("profile_companion_initialized");
        if !ctx.data(|data| data.get_temp::<bool>(initialized).unwrap_or(false)) {
            if let Ok(window) = serde_json::from_str::<PlayerProfileWindow>(&profile_companion_initial()) {
                self.lichess.next_profile_window_id = (window.id + 1) * 1000000;
                self.lichess.profile_windows.push(window);
                ctx.data_mut(|data| data.insert_temp(initialized, true));
            }
        }
        if read_profile_state().is_empty() { ui.label("Keep the main Ironwood window open to use this player profile."); }
        let mut windows = std::mem::take(&mut self.lichess.profile_windows);
        if let Some(window) = windows.first_mut() {
            let data = lichess_profile_window_data(&self.lichess.lobby, window.id);
            let title = lichess_profile_window_title(&data["playerProfile"]);
            let available = ui.available_size();
            Frame::new().fill(Color32::from_rgb(23,26,30))
                .stroke(Stroke::new(2.0, Color32::from_rgb(70,255,120)))
                .corner_radius(6.0).inner_margin(12).show(ui, |ui| {
                    ui.set_min_size((available - Vec2::splat(28.0)).max(Vec2::ZERO));
                    let background = ui.painter().add(egui::Shape::Noop);
                    ui.horizontal(|ui| {
                        ui.add_sized([(ui.available_width()-40.0).max(0.0),28.0],
                            egui::Label::new(RichText::new(&title).size(18.0).strong()).truncate());
                        if Self::glossy_board_button(ui, "×", false, true)
                            .on_hover_text("Close player profile window").clicked() { close_profile_companion(); }
                    });
                    ui.add_space(10.0);
                    egui::ScrollArea::vertical().id_salt("profile_companion_scroll")
                        .auto_shrink([false,false]).show(ui, |ui| {
                            self.lichess_profile_contents(ui, window, &data);
                        });
                    let surface = ui.min_rect();
                    let paper = Self::paper_grain_texture(&ctx);
                    ui.painter().set(background, egui::Shape::image(paper.id(),surface,
                        egui::Rect::from_min_max(egui::Pos2::ZERO,
                            egui::pos2(surface.width()/256.0,surface.height()/256.0)),Color32::WHITE));
                });
        }
        for window in windows.iter_mut().skip(1) { self.lichess_profile_window(&ctx, window); }
        windows.append(&mut self.lichess.profile_windows);
        self.lichess.profile_windows = windows;
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }

    fn open_lichess_history(&mut self) {
        let username=self.lichess.lobby["account"].as_str().unwrap_or_default().to_owned();
        self.open_lichess_profile_window(&username);
    }
    fn open_lichess_profile_window(&mut self, username: &str) {
        self.lichess.next_profile_window_id+=1;
        let mut window=PlayerProfileWindow::new(self.lichess.next_profile_window_id,username);
        window.load(false);
        self.lichess.profile_windows.push(window);
        lichess_command(serde_json::json!({"type":"following"}));
    }
    fn lichess_history_ui(&mut self, ctx: &egui::Context) {
        let mut windows=std::mem::take(&mut self.lichess.profile_windows);
        for window in &mut windows {
            self.lichess_profile_window(ctx,window);
        }
        if self.lichess.profile_review_requested {
            for window in &mut windows {window.open=false;}
            self.lichess.profile_review_requested=false;
        }
        for window in &windows {
            if !window.open && !window.detached { lichess_command(serde_json::json!({"type":"closeProfileWindow","windowId":window.id.to_string()})); }
        }
        windows.retain(|window|window.open);
        windows.append(&mut self.lichess.profile_windows);
        self.lichess.profile_windows=windows;
    }
    fn lichess_profile_window(&mut self, ctx: &egui::Context, window: &mut PlayerProfileWindow) {
        let data=lichess_profile_window_data(&self.lichess.lobby,window.id);
        let mut open = true;
        let mut detached = false;
        let title = lichess_profile_window_title(&data["playerProfile"]);
        let profile_window = egui::Window::new(&title).id(egui::Id::new(("lichess_player_profile_window",window.id)))
            .title_bar(false).drag_area(egui::WindowDrag::Anywhere)
            .frame(Frame::window(&ctx.style_of(ctx.theme())).stroke(Stroke::new(1.0,Color32::from_rgb(211,173,98)))
                .corner_radius(12)
                .shadow(egui::epaint::Shadow{offset:[4,6],blur:14,spread:2,color:Color32::from_black_alpha(160)}))
            .default_pos(egui::pos2(32.0+(window.id%8) as f32*28.0,40.0+(window.id%8) as f32*28.0))
            .default_width(1020.0).default_height(750.0)
            .min_size(Vec2::new(360.0,240.0)).resizable(true).vscroll(true)
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
            .max_width((ctx.content_rect().width() - 32.0).max(280.0))
            .max_height((ctx.content_rect().height() - 32.0).max(280.0))
            .show(ctx, |ui| {
            let grain_background = ui.painter().add(egui::Shape::Noop);
            ui.horizontal(|ui| {
                ui.add_sized([(ui.available_width() - 82.0).max(80.0), 28.0],
                    egui::Label::new(RichText::new(&title).size(18.0).strong()).selectable(false).truncate());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if Self::glossy_board_button(ui, "×", false, true)
                        .on_hover_text("Close player profile").clicked() { open = false; }
                    #[cfg(target_arch = "wasm32")]
                    if !profile_companion_mode() && Self::glossy_board_button(ui, "↗", false, true)
                        .on_hover_text("Move player profile to a separate window").clicked() {
                        publish_profile_state(&self.lichess.lobby.to_string());
                        if let Ok(state) = serde_json::to_string(window) {
                            detached = open_profile_companion(&state);
                        }
                    }
                });
            });

            ui.add_space(10.0);
            Frame::new().inner_margin(egui::Margin::symmetric(12, 0)).show(ui, |ui| {
                self.lichess_profile_contents(ui, window, &data);
            });
            let paper = Self::paper_grain_texture(ctx);
            let surface = ui.min_rect().expand(6.0);
            ui.painter().set(grain_background, egui::Shape::image(paper.id(), surface,
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(surface.width()/256.0, surface.height()/256.0)), Color32::WHITE));
        });
        if let Some(window) = profile_window {
            Self::paint_fics_window_border(&ctx.layer_painter(window.response.layer_id), window.response.rect);
        }
        window.detached = detached;
        window.open = open && !detached;
    }

    fn lichess_profile_contents(&mut self, ui: &mut egui::Ui, window: &mut PlayerProfileWindow, data: &serde_json::Value) {
            if !data["connected"].as_bool().unwrap_or(false) { ui.label("Reconnect to view a player profile."); return; }
            let loading = data["profileLoading"].as_bool().unwrap_or(false);
            Self::lichess_profile_section(ui, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(10.0, 10.0);
            ui.label(RichText::new("FIND A PLAYER").small().strong().color(Color32::from_rgb(211,173,98)));
            ui.horizontal_wrapped(|ui| {
                let input = ui.add(egui::TextEdit::singleline(&mut window.username).desired_width(220.0).margin(Vec2::new(10.0,7.0)).hint_text("Lichess username"));
                let enter = input.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if ui.add_enabled(!loading, Self::lichess_profile_button("Load player")).clicked() || (enter && !loading) {
                    window.load(false);
                }
                if ui.add_enabled(!loading, Self::lichess_profile_button("My account")).clicked() {
                    window.username=data["account"].as_str().unwrap_or_default().to_owned(); window.load(false);
                }
                if ui.add(Self::lichess_profile_button("Load in new window")).clicked() { self.open_lichess_profile_window(&window.username); }
                if ui.add_enabled(!loading, Self::lichess_profile_button("Refresh")).clicked() {
                    let username=window.username.clone();
                    window.username=data["profileUsername"].as_str().unwrap_or(&username).to_owned(); window.load(true);
                }
                if loading { ui.spinner(); ui.label("Loading player profile…"); }
            });
            });
            ui.add_space(8.0);
            for key in ["profileError", "historyError"] {
                if let Some(error) = data[key].as_str().filter(|s| !s.is_empty()) {
                    ui.colored_label(Color32::from_rgb(230,150,120), error);
                }
            }
            if !self.lichess.profile_action_error.is_empty() { ui.colored_label(Color32::from_rgb(230,150,120),&self.lichess.profile_action_error); }
            self.lichess_following_ui(ui, loading, window, &data);
            ui.scope(|ui| {
                let profile = data["playerProfile"].clone();
                if profile.is_null() { if !loading { ui.label("Load a Lichess username to view their profile."); } return; }
                let gold = Color32::from_rgb(211,173,98);
                ui.horizontal_wrapped(|ui| {
                    if let Some(title) = profile["title"].as_str() { ui.label(RichText::new(title).size(28.0).strong().color(gold)); }
                    ui.label(RichText::new(profile["username"].as_str().unwrap_or("Player")).size(28.0));
                    ui.add(egui::Hyperlink::from_label_and_url("View on Lichess",format!("https://lichess.org/@/{}",profile["username"].as_str().unwrap_or_default())).open_in_new_tab(true));
                    let username = profile["username"].as_str().unwrap_or_default();
                    let own = data["account"].as_str().is_some_and(|account|account.eq_ignore_ascii_case(username));
                    if !own {
                        let following = profile["following"].as_bool().or_else(||data["following"].as_array().map(|users|users.iter().any(|user|user["username"].as_str().is_some_and(|name|name.eq_ignore_ascii_case(username)))));
                        let busy = data["followBusy"].as_bool().unwrap_or(false) || data["followingLoading"].as_bool().unwrap_or(false);
                        let allowed = !busy && !loading && following.is_some() && (following == Some(true) || profile["followable"].as_bool() != Some(false));
                        if ui.add_enabled(allowed,egui::Button::new(if following==Some(true) {"Unfollow"} else {"Follow"}))
                            .on_hover_text("Updates your Lichess following list.").clicked() {
                            lichess_command(serde_json::json!({"type":"setFollowing","username":username,"follow":following!=Some(true)}));
                        }
                        if busy { ui.spinner(); }
                    }

                });
                ui.separator();
                self.lichess_profile_ratings(ui, &profile, window);
                self.lichess_profile_performance(ui, window, &data);
                ui.add_space(10.0);
                if ui.available_width() >= 720.0 {
                    let panel_gap = 12.0;
                    let row_width = ui.available_width();
                    let graph_width = (row_width-panel_gap)*0.64;
                    let bio_width = row_width-panel_gap-graph_width;
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x = panel_gap;
                        let graph_panel = ui.allocate_ui_with_layout(Vec2::new(graph_width,0.0),Layout::top_down(Align::Min),|ui| {
                            ui.set_width(graph_width);
                            Self::lichess_profile_section(ui, |ui| self.lichess_profile_graph(ui, window, &data));
                        });
                        ui.vertical(|ui| {
                            ui.set_width(bio_width);
                            Self::lichess_profile_section(ui, |ui| {
                                ui.set_min_height((graph_panel.response.rect.height()-24.0).max(0.0));
                                lichess_profile_bio(ui,&profile);
                            });
                        });
                    });
                } else {
                    Self::lichess_profile_section(ui, |ui| self.lichess_profile_graph(ui, window, &data));
                    ui.add_space(10.0);
                    Self::lichess_profile_section(ui, |ui| lichess_profile_bio(ui,&profile));
                }
                ui.add_space(10.0);
                self.lichess_profile_activity(ui,window,&data);
                ui.separator();
                Self::lichess_profile_record(ui, &profile);
                ui.add_space(10.0);
                egui::CollapsingHeader::new(RichText::new("Recent games").heading())
                    .id_salt(("lichess_recent_games",window.id)).default_open(true).show(ui, |ui| {
                        ui.small("Latest 12 completed games · all time controls");
                        let games = data["recentGames"].as_array().cloned().unwrap_or_default();
                        if games.is_empty() && !loading { ui.label("No recent games available."); }
                        for game in &games { self.lichess_profile_game(ui, game); }
                    });
            });
    }

    fn lichess_following_ui(&mut self, ui: &mut egui::Ui, profile_loading: bool, window: &mut PlayerProfileWindow, data: &serde_json::Value) {
        let loading = data["followingLoading"].as_bool().unwrap_or(false);
        let busy = data["followBusy"].as_bool().unwrap_or(false);
        if let Some(error) = data["followError"].as_str().filter(|error|!error.is_empty()) {
            ui.colored_label(Color32::from_rgb(230,150,120),error);
        }
        if data["followPermissionNeeded"].as_bool()==Some(true)
            && ui.button("Sign in again for follow access").clicked() {
            #[cfg(target_arch="wasm32")]
            lichess_sign_in();
        }
        let users = data["following"].as_array().cloned();
        egui::CollapsingHeader::new(users.as_ref().map(|users|format!("Following ({})",users.len())).unwrap_or_else(||"Following".into()))
            .id_salt("lichess_following_list").show(ui,|ui| {
            Self::lichess_profile_section(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(10.0,10.0);
                ui.horizontal_wrapped(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut window.following_search).desired_width(220.0).margin(Vec2::new(10.0,7.0)).hint_text("Find followed player"));
                    if ui.add_enabled(!loading && !busy,Self::lichess_profile_button("Refresh following")).clicked() {
                        lichess_command(serde_json::json!({"type":"following","refresh":true}));
                    }
                    if loading { ui.spinner(); }
                });
                let query = window.following_search.trim().to_lowercase();
                if let Some(users) = &users {
                    if users.is_empty() { ui.label("You are not following any players."); }
                    egui::ScrollArea::vertical().id_salt("lichess_following_scroll").max_height(180.0).show(ui,|ui| {
                        let mut matches = 0;
                        for user in users {
                            let username=user["username"].as_str().unwrap_or_default();
                            let label=format!("{} {}{}",user["title"].as_str().unwrap_or(""),username,user["realName"].as_str().filter(|name|!name.is_empty()).map(|name|format!(" · {name}")).unwrap_or_default());
                            if !label.to_lowercase().contains(&query) {continue;}
                            matches+=1;
                            if ui.add_enabled(!profile_loading,Self::lichess_profile_button(label.trim())).clicked() {
                                window.username=username.to_owned();
                                window.load(false);
                            }
                        }
                        if matches==0 && !users.is_empty() {ui.label("No followed players match.");}
                    });
                } else if !loading { ui.label("Refresh to load the players you follow on Lichess."); }
            });
        });
    }

    fn lichess_profile_ratings(&mut self, ui: &mut egui::Ui, profile: &serde_json::Value, window: &mut PlayerProfileWindow) {
        let gold = Color32::from_rgb(211,173,98);
        let gap = 10.0;
        let columns = ((ui.available_width()+gap)/184.0).floor().clamp(1.0,6.0);
        let card_width = ((ui.available_width()-gap*(columns-1.0))/columns).floor()-1.0;
        // Explicit rows cannot bounce between layouts on fractional pixel boundaries.
        let chips=[(0,"bullet","Bullet"),(1,"blitz","Blitz"),(2,"rapid","Rapid"),(3,"classical","Classical"),(4,"correspondence","Correspondence"),(5,"puzzle","Puzzles")];
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(gap);
            for row in chips.chunks(columns as usize) {
            ui.horizontal_top(|ui| {
            for &(i,key,label) in row {
                let perf = &profile["perfs"][key];
                let count = perf["games"].as_u64().unwrap_or(0);
                let rating = if count > 0 { format!("{}{}",perf["rating"].as_u64().unwrap_or(0),if perf["prov"].as_bool()==Some(true) {"?"} else {""}) } else { "—".into() };
                let (card_rect, card_response) = ui.allocate_exact_size(Vec2::new(card_width,64.0), if i<5 {Sense::click()} else {Sense::hover()});
                let mut clicked=card_response.clicked();
                let mut card_ui = ui.new_child(egui::UiBuilder::new().max_rect(card_rect).layout(Layout::top_down(Align::Min)));
                card_ui.set_clip_rect(ui.clip_rect().intersect(card_rect));
                    let card = Frame::new().fill(Color32::from_rgb(13,17,22)).inner_margin(10.0).corner_radius(6.0).show(&mut card_ui, |ui| {
                        ui.set_width(card_width-20.0);
                        ui.set_min_height(44.0);
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                        if i < 5 {
                            clicked |= ui.selectable_label(window.control==i,RichText::new(format!("{label}  {rating}")).color(gold)).on_hover_text("Show rating history and detailed performance statistics").clicked();
                        } else {
                            ui.allocate_ui_with_layout(Vec2::new(ui.available_width(),ui.spacing().interact_size.y), Layout::left_to_right(Align::Center), |ui| {
                                ui.label(RichText::new(format!("{label}  {rating}")).color(gold)).on_hover_text("Detailed performance statistics are available for game time controls.");
                            });
                        }
                        let count_text = format!("{count} {}",if key=="puzzle" {"attempts"} else {"rated games"});
                        ui.label(RichText::new(count_text).size(14.0));
                    });
                    Self::paint_sunken_border(ui, card.response.rect);
                    if i<5 {
                        card_response.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Show detailed performance statistics");
                        if clicked {
                            window.control=i;window.performance_control=Some(i);
                            lichess_command(serde_json::json!({"type":"performance","windowId":window.id.to_string(),"username":profile["username"].as_str().unwrap_or(window.username.trim()),"perf":key}));
                        }
                    }
            }
            });
            }
        });
    }

    fn lichess_profile_activity(&mut self, ui: &mut egui::Ui, window: &PlayerProfileWindow, data: &serde_json::Value) {
        egui::CollapsingHeader::new(RichText::new("Activity").heading())
            .id_salt(("lichess_player_activity",window.id)).default_open(false).show(ui, |ui| {
                self.lichess_profile_activity_contents(ui,window,data);
            });
    }

    fn lichess_profile_activity_contents(&mut self, ui: &mut egui::Ui, window: &PlayerProfileWindow, data: &serde_json::Value) {
            Self::lichess_profile_section(ui, |ui| {
                let activity=&data["activity"];
                let request=|refresh|lichess_command(serde_json::json!({"type":"activity","windowId":window.id.to_string(),"username":data["profileUsername"].as_str().unwrap_or(window.username.trim()),"refresh":refresh}));
                if activity.is_null() {request(false);}
                ui.horizontal_wrapped(|ui| {
                    ui.small("Recent daily game and puzzle activity");
                    if ui.add_enabled(activity["loading"].as_bool()!=Some(true),egui::Button::new("Refresh activity")).clicked() {request(true);}
                    if activity.is_null() || activity["loading"].as_bool()==Some(true) {ui.spinner();}
                });
                if let Some(error)=activity["error"].as_str().filter(|s|!s.is_empty()) {ui.colored_label(Color32::LIGHT_RED,error);}
                let mut displayed=0;
                for day in activity["days"].as_array().into_iter().flatten() {
                    let games=day["games"].as_object();
                    let puzzles=&day["puzzles"]["score"];
                    if games.is_none_or(|games|games.is_empty()) && !puzzles.is_object() {continue;}
                    if displayed>0 {ui.separator();}
                    displayed+=1;
                    ui.label(RichText::new(day["interval"]["start"].as_u64().map(profile_date).unwrap_or_else(||"Date unavailable".into())).strong().color(Color32::from_rgb(211,173,98)));
                    for (control,score) in games.into_iter().flatten() {
                        let label=match control.as_str() {"bullet"=>"Bullet","blitz"=>"Blitz","rapid"=>"Rapid","classical"=>"Classical","correspondence"=>"Correspondence","ultraBullet"=>"UltraBullet",other=>other};
                        ui.horizontal_wrapped(|ui| {
                            ui.label(format!("{label}: {} wins · {} losses · {} draws",score["win"].as_u64().unwrap_or(0),score["loss"].as_u64().unwrap_or(0),score["draw"].as_u64().unwrap_or(0)));
                            Self::lichess_activity_rating(ui,&score["rp"]);
                        });
                    }
                    if puzzles.is_object() {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(format!("Puzzles: {} solved · {} missed",puzzles["win"].as_u64().unwrap_or(0),puzzles["loss"].as_u64().unwrap_or(0)));
                            Self::lichess_activity_rating(ui,&puzzles["rp"]);
                        });
                    }
                }
                if displayed==0 && activity["days"].is_array() {ui.label("No recent game or puzzle activity available.");}
            });
    }

    fn lichess_activity_rating(ui: &mut egui::Ui, rating: &serde_json::Value) {
        if let (Some(before),Some(after))=(rating["before"].as_i64(),rating["after"].as_i64()) {
            let change=after-before;
            ui.colored_label(if change>0 {Color32::from_rgb(116,204,151)} else if change<0 {Color32::from_rgb(226,132,126)} else {Color32::GRAY},format!("Rating {before} to {after} ({change:+})"));
        }
    }

    fn lichess_profile_performance(&mut self, ui: &mut egui::Ui, window: &mut PlayerProfileWindow, data: &serde_json::Value) {
        let Some(selected)=window.performance_control else {return;};
        let (key,label)=[("bullet","Bullet"),("blitz","Blitz"),("rapid","Rapid"),("classical","Classical"),("correspondence","Correspondence")][selected.min(4)];
        let entry=&data["performance"][key];
        ui.add_space(10.0);
        Self::lichess_profile_section(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(format!("{label} · Performance details")).strong().color(Color32::from_rgb(211,173,98)));
                if ui.add_enabled(entry["loading"].as_bool()!=Some(true),egui::Button::new("Refresh")).clicked() {
                    lichess_command(serde_json::json!({"type":"performance","windowId":window.id.to_string(),"username":data["profileUsername"].as_str().unwrap_or(window.username.trim()),"perf":key,"refresh":true}));
                }
                if ui.button("Close details").clicked() {window.performance_control=None;}
            });
            if entry["loading"].as_bool()==Some(true) {ui.horizontal(|ui| {ui.spinner();ui.label("Loading performance and notable games…");});}
            if let Some(error)=entry["error"].as_str().filter(|s|!s.is_empty()) {ui.colored_label(Color32::LIGHT_RED,error);}
            let stats=&entry["data"];
            if stats.is_null() {return;}
            let stat=&stats["stat"];
            let number=|v:&serde_json::Value|v.as_f64().map(|n|format!("{n:.0}")).unwrap_or_else(||"—".into());
            ui.horizontal_wrapped(|ui| {
                for (name,value) in [("Rating",&stats["perf"]["glicko"]["rating"]),("Rank",&stats["rank"]),("Percentile",&stats["percentile"]),("Average opponent",&stat["count"]["opAvg"])] {
                    ui.label(format!("{name}: {}{}",number(value),if name=="Percentile" && value.is_number() {"%"} else {""}));
                    ui.add_space(10.0);
                }
            });
            for (name,k) in [("Highest rating","highest"),("Lowest rating","lowest")] {
                if !stat[k].is_null() {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(format!("{name}: {} · {}",number(&stat[k]["int"]),stat[k]["at"].as_str().unwrap_or("Date unavailable").split('T').next().unwrap_or_default()));
                        if let Some(id)=stat[k]["gameId"].as_str() {ui.add(egui::Hyperlink::from_label_and_url("Game on Lichess",format!("https://lichess.org/{id}")).open_in_new_tab(true));}
                    });
                }
            }
            ui.horizontal_wrapped(|ui| {
                for (name,k) in [("Games","all"),("Rated","rated"),("Wins","win"),("Losses","loss"),("Draws","draw"),("Tournament games","tour"),("Disconnects","disconnects")] {
                    ui.label(format!("{name}: {}",number(&stat["count"][k])));ui.add_space(8.0);
                }
            });
            if let Some(seconds)=stat["count"]["seconds"].as_u64() {ui.label(format!("Time playing: {}h {}m",seconds/3600,seconds%3600/60));}
            ui.horizontal_wrapped(|ui| {
                for (name,k) in [("Win streak","win"),("Loss streak","loss")] {
                    ui.label(format!("{name}: {} current · {} longest",number(&stat["resultStreak"][k]["cur"]["v"]),number(&stat["resultStreak"][k]["max"]["v"])));
                }
            });
            for (heading,k) in [("Best wins","bestWins"),("Worst losses","worstLosses")] {
                ui.add_space(8.0);ui.label(RichText::new(heading).strong());
                let results=stat[k]["results"].as_array();
                if results.is_none_or(|r|r.is_empty()) {ui.small("No games available.");}
                for result in results.into_iter().flatten().take(5) {
                    let id=result["gameId"].as_str().unwrap_or_default();
                    let game=&entry["games"][id];
                    if game.is_object() {
                        let mut game=game.clone();
                        if game["lastFen"].as_str().is_none() {
                            let cache_id=egui::Id::new(("lichess_performance_final_fen",id));
                            let cached=ui.ctx().data(|d|d.get_temp::<String>(cache_id));
                            let fen=cached.or_else(||game["pgn"].as_str().and_then(|pgn|Self::parse_pgn_mainline(pgn).ok())
                                .and_then(|(positions,_)|positions.last().map(ToString::to_string)));
                            if let Some(fen)=fen {
                                ui.ctx().data_mut(|d|d.insert_temp(cache_id,fen.clone()));
                                game["lastFen"]=fen.into();
                            }
                        }
                        self.lichess_profile_game(ui,&game);
                    } else {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(format!("{} · opponent {} · {}",result["opId"]["name"].as_str().unwrap_or("Anonymous"),number(&result["opRating"]),result["at"].as_str().unwrap_or_default().split('T').next().unwrap_or_default()));
                            if !id.is_empty() {ui.add(egui::Hyperlink::from_label_and_url("Open game on Lichess",format!("https://lichess.org/{id}")).open_in_new_tab(true));}
                        });
                        if let Some(error)=entry["gameErrors"][id].as_str() {ui.colored_label(Color32::LIGHT_RED,error);}
                    }
                }
            }
        });
    }

    fn lichess_profile_graph(&mut self, ui: &mut egui::Ui, window: &mut PlayerProfileWindow, data: &serde_json::Value) {
        let loading = data["historyLoading"].as_bool().unwrap_or(false);
            let controls = ["Bullet", "Blitz", "Rapid", "Classical", "Correspondence"];
            let shown = data["historyUsername"].as_str().unwrap_or_default();
            if !loading && data["ratingHistory"].is_array() && window.shown_username != shown {
                window.shown_username = shown.to_owned();
                window.control = controls.iter().enumerate()
                    .max_by_key(|(_, name)| lichess_history_points(&data, name).len())
                    .map(|(index, _)| index).unwrap_or(0);
            }

            ui.horizontal_wrapped(|ui| { for (i, name) in controls.iter().enumerate() {
                ui.selectable_value(&mut window.control, i, format!("{name}"));
            }});

            if loading { ui.spinner(); ui.label("Loading rating history…"); }
            if let Some(error) = data["historyError"].as_str().filter(|s| !s.is_empty()) { ui.label(error); }
            let name = controls[window.control.min(4)];
            ui.horizontal_wrapped(|ui| {
                for (i, label) in ["1M", "3M", "6M", "YTD", "1Y", "ALL"].iter().enumerate() {
                    ui.selectable_value(&mut window.range, i, *label);
                }
            });
            let all_points = lichess_history_points(&data, name);
            let today = profile_today_day();
            let cutoff = match window.range {
                0 => today - 30.0, 1 => today - 90.0, 2 => today - 180.0,
                3 => profile_year_start(today), 4 => today - 365.0, _ => f64::NEG_INFINITY,
            };
            let points: Vec<_> = all_points.into_iter().filter(|point| point.0 >= cutoff).collect();
            if points.is_empty() { if !loading { ui.label(format!("No {name} rating history in this period. Try ALL or another time control.")); } return; }
            let first = &points[0]; let last = &points[points.len()-1];
            ui.label(format!("Latest {:.0} · Change {:+.0}", last.1, last.1-first.1));
            ui.add_space(6.0);
            let (rect, response) = ui.allocate_exact_size(Vec2::new(ui.available_width().max(180.0), 250.0), Sense::hover());
            ui.painter().rect_filled(rect,6.0,Color32::from_rgb(13,17,22));
            let paper=Self::paper_grain_texture(ui.ctx());
            let surface=rect.shrink(4.0);
            ui.painter().image(paper.id(),surface,
                egui::Rect::from_min_max(egui::Pos2::ZERO,egui::pos2(surface.width()/256.0,surface.height()/256.0)),
                Color32::from_white_alpha(110));
            Self::paint_sunken_border(ui,rect);
            let plot = rect.shrink2(Vec2::new(44.0,22.0));
            let low = points.iter().map(|p| p.1).fold(f32::INFINITY,f32::min)-25.0;
            let high = points.iter().map(|p| p.1).fold(f32::NEG_INFINITY,f32::max)+25.0;
            let position = |p: &(f64,f32,String)| egui::pos2(plot.left()+((p.0-first.0)/(last.0-first.0).max(1.0)) as f32*plot.width(),plot.bottom()-(p.1-low)/(high-low)*plot.height());
            let painter=ui.painter(); let gold=Color32::from_rgb(211,173,98);
            for step in 0..=4 {
                let y=plot.bottom()-plot.height()*step as f32/4.0;
                painter.line_segment([egui::pos2(plot.left(),y),egui::pos2(plot.right(),y)],Stroke::new(1.0,Color32::from_gray(55)));
                painter.text(egui::pos2(plot.left()-6.0,y),egui::Align2::RIGHT_CENTER,format!("{:.0}",low+(high-low)*step as f32/4.0),egui::FontId::proportional(11.0),Color32::GRAY);
            }
            for pair in points.windows(2) { painter.line_segment([position(&pair[0]),position(&pair[1])],Stroke::new(2.0,gold)); }
            painter.circle_filled(position(last),3.0,gold);
            painter.text(plot.left_bottom()+Vec2::new(0.0,6.0),egui::Align2::LEFT_TOP,&first.2,egui::FontId::proportional(11.0),Color32::GRAY);
            painter.text(plot.right_bottom()+Vec2::new(0.0,6.0),egui::Align2::RIGHT_TOP,&last.2,egui::FontId::proportional(11.0),Color32::GRAY);
            if let Some(pointer)=response.hover_pos() {
                if let Some(point)=points.iter().min_by(|a,b| (position(a).x-pointer.x).abs().total_cmp(&(position(b).x-pointer.x).abs())) {
                    painter.circle_filled(position(point),4.0,gold);
                    response.on_hover_text(format!("{} · {:.0}",point.2,point.1));
                }
            }

        ui.small("Daily rating history · hover to inspect · ? = provisional rating");
    }

    fn lichess_profile_record(ui: &mut egui::Ui, profile: &serde_json::Value) {
        Self::lichess_profile_section(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Game record").strong().size(15.0));
                ui.label(RichText::new("All time · rated and casual").small().color(Color32::from_rgb(150,160,169)));
            });
            ui.add_space(8.0);
            let gap = 8.0;
            let columns = ((ui.available_width()+gap)/118.0).floor().clamp(1.0,5.0);
            let width = (ui.available_width()-gap*(columns-1.0))/columns;
            let gold=Color32::from_rgb(225,186,111);
            let green=Color32::from_rgb(116,204,151);
            let red=Color32::from_rgb(226,132,126);
            let blue=Color32::from_rgb(134,179,218);
            ui.spacing_mut().item_spacing = Vec2::splat(gap);
            let metrics=[("all","Games",gold),("rated","Rated",Color32::from_rgb(207,214,221)),("win","Wins",green),("loss","Losses",red),("draw","Draws",blue)];
            // Explicit rows avoid fractional-width auto-wrap oscillation during a resize.
            for row in metrics.chunks(columns as usize) {
                ui.horizontal_top(|ui| {
                for &(key,label,color) in row {
                    ui.allocate_ui_with_layout(Vec2::new(width.floor()-1.0,76.0),Layout::top_down(Align::Min),|ui| {
                        let tile=Frame::new().fill(Color32::from_rgb(23,29,34)).stroke(Stroke::new(1.0,Color32::from_rgb(45,54,62)))
                            .corner_radius(5.0).inner_margin(12.0).show(ui,|ui| {
                            ui.set_width((width.floor()-27.0).max(60.0));
                            ui.style_mut().wrap_mode=Some(egui::TextWrapMode::Extend);
                            ui.label(RichText::new(label).size(12.0).color(Color32::from_rgb(172,183,193)));
                            ui.add_space(2.0);
                            ui.label(RichText::new(profile["count"][key].as_u64().map(profile_record_number).unwrap_or_else(||"—".into())).size(24.0).strong().color(color));
                        });
                        let accent=egui::Rect::from_min_size(tile.response.rect.left_top()+Vec2::new(8.0,1.0),Vec2::new((tile.response.rect.width()-16.0).max(0.0),2.0));
                        ui.painter().rect_filled(accent,1.0,color.gamma_multiply(0.8));
                    });
                }
            });
            }
            if let (Some(wins),Some(losses),Some(draws))=(profile["count"]["win"].as_u64(),profile["count"]["loss"].as_u64(),profile["count"]["draw"].as_u64()) {
                let total=wins as f64+losses as f64+draws as f64;
                if total>0.0 {
                    ui.add_space(4.0);
                    let (bar,response)=ui.allocate_exact_size(Vec2::new(ui.available_width(),6.0),Sense::hover());
                    let mut x=bar.left();
                    for (count,color) in [(wins,green),(losses,red),(draws,blue)] {
                        let right=x+bar.width()*(count as f64/total) as f32;
                        ui.painter().rect_filled(egui::Rect::from_min_max(egui::pos2(x,bar.top()),egui::pos2(right,bar.bottom())),1.0,color);
                        x=right;
                    }
                    response.on_hover_text(format!("Wins {:.1}% · Losses {:.1}% · Draws {:.1}%",wins as f64/total*100.0,losses as f64/total*100.0,draws as f64/total*100.0));
                }
            }
        });
    }

    fn lichess_profile_button(label: &str) -> RaisedGoldButton<'_> {
        egui::Button::new(RichText::new(label).color(Color32::from_rgb(25,29,35)))
            .min_size(Vec2::new(0.0,32.0)).raised_gold()
    }

    fn lichess_profile_section(ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui)) {
        let section = Frame::new().fill(Color32::from_rgb(13,17,22))
            .corner_radius(6.0).inner_margin(12.0).show(ui, |ui| {
                ui.set_width(ui.available_width());
                content(ui);
            });
        Self::paint_sunken_border(ui, section.response.rect);
    }

    fn review_lichess_profile_game(&mut self, pgn: &str) {
        #[cfg(target_arch = "wasm32")]
        if profile_companion_mode() {
            if !review_profile_in_main(pgn) { self.lichess.profile_action_error = "Keep the main Ironwood window open to review this game.".into(); }
            return;
        }
        if (self.fics_active && !self.fics_game_finished) || self.training.is_some() || self.puzzle_navigation_locked() {
            self.lichess.profile_action_error="Finish active play or training before opening another game.".into(); return;
        }
        if Self::parse_pgn_mainline(pgn).is_err() {self.lichess.profile_action_error="Could not read the game moves.".into();return;}
        self.save_game();
        self.exit_puzzle_mode();
        self.transient_profile_review=true;
        self.load_pgn_with_storage(pgn,false);
        self.engine_enabled=false;
        self.review_to(0);
        self.engine_status="Lichess game review · not saved".into();
        self.lichess.profile_action_error.clear();
        self.lichess.profile_review_requested=true;
    }

    fn lichess_profile_game(&mut self, ui: &mut egui::Ui, game: &serde_json::Value) {
        Self::lichess_profile_section(ui, |ui| {
            let pgn=game["pgn"].as_str().filter(|pgn|!pgn.is_empty());
            ui.horizontal_top(|ui| {
                let controls_width = 144.0;
                let details_width = (ui.available_width()-controls_width-ui.spacing().item_spacing.x).max(120.0);
                ui.allocate_ui_with_layout(Vec2::new(details_width,0.0),egui::Layout::left_to_right(egui::Align::Min), |ui| {
                    ui.set_min_width(details_width);
                    let side = 112.0;
                    let (rect,preview_response) = ui.allocate_exact_size(Vec2::splat(side),Sense::click());
                    let preview_response=preview_response.on_hover_text("Watch moves in the enlarged board preview").on_hover_cursor(egui::CursorIcon::ZoomIn);
                    if preview_response.clicked() {
                        self.lichess.profile_action_error.clear();
                        match lichess_recent_game_preview(game) {
                            Ok(payload) => {
                                #[cfg(target_arch="wasm32")]
                                if let Err(error)=show_profile_game_preview(&payload.to_string()) {self.lichess.profile_action_error=format!("Could not open preview: {error:?}");}
                                #[cfg(not(target_arch="wasm32"))]
                                let _=payload;
                            }
                            Err(error)=>self.lichess.profile_action_error=error,
                        }
                    }
                    if let Some(board) = game["lastFen"].as_str().filter(|_|ui.is_rect_visible(rect)).and_then(|fen|Board::from_str(fen).ok()) {
                        let cell = side/8.0;
                        for square in chess::ALL_SQUARES {
                            let file=square.get_file().to_index(); let rank=square.get_rank().to_index();
                            let tile=egui::Rect::from_min_size(rect.min+Vec2::new(file as f32*cell,(7-rank) as f32*cell),Vec2::splat(cell));
                            ui.painter().rect_filled(tile,0.0,if (file+rank)%2==0 {Color32::from_rgb(173,133,92)} else {Color32::from_rgb(235,211,174)});
                            if let (Some(piece),Some(color))=(board.piece_on(square),board.color_on(square)) {
                                Self::paint_piece(ui,tile,cell,PieceSet::Merida,color,piece);
                            }
                        }
                    } else if ui.is_rect_visible(rect) { ui.painter().text(rect.center(),egui::Align2::CENTER_CENTER,"Position unavailable",egui::FontId::proportional(11.0),Color32::GRAY); }
                    if preview_response.hovered() || preview_response.has_focus() {
                        let gold=Color32::from_rgb(211,173,98);
                        ui.painter().rect_stroke(rect.expand(3.0),2.0,Stroke::new(2.0,gold),egui::StrokeKind::Outside);
                        let badge=egui::Rect::from_min_size(rect.right_top()+Vec2::new(-30.0,4.0),Vec2::splat(26.0));
                        ui.painter().rect_filled(badge,4.0,Color32::from_black_alpha(210));
                        let center=badge.center()-Vec2::splat(2.0);
                        ui.painter().circle_stroke(center,5.0,Stroke::new(2.0,gold));
                        ui.painter().line_segment([center+Vec2::splat(4.0),center+Vec2::splat(9.0)],Stroke::new(2.0,gold));
                    }
                    ui.vertical(|ui| {
                        ui.set_max_width(ui.available_width());
                        let time = if let Some(seconds)=game["clock"]["initial"].as_u64() { format!("{}+{}",seconds as f64/60.0,game["clock"]["increment"].as_u64().unwrap_or(0)) }
                            else { game["daysPerTurn"].as_u64().map(|d|format!("{d} days/move")).unwrap_or_else(||"Unlimited".into()) };
                        ui.add(egui::Label::new(RichText::new(format!("{time} · {} · {}",game["speed"].as_str().unwrap_or("Chess"),if game["rated"].as_bool()==Some(true) {"Rated"} else {"Casual"})).strong()).truncate());
                        if let Some(ms)=game["createdAt"].as_u64() { ui.small(profile_date(ms)); }
                        for color in ["white","black"] {
                                let player=&game["players"][color]; let user=&player["user"];
                                let label=format!("{} {}  {}{}",user["title"].as_str().unwrap_or(""),user["name"].as_str().unwrap_or("Anonymous"),player["rating"].as_u64().map(|r|r.to_string()).unwrap_or_else(||"—".into()),player["ratingDiff"].as_i64().map(|d|format!(" ({d:+})")).unwrap_or_default());
                                let label=format!("{} · {}",if color=="white" {"White"} else {"Black"},label.trim());
                                ui.add(egui::Label::new(&label).truncate()).on_hover_text(&label);
                        }
                        let result=match game["winner"].as_str() { Some("white")=>"1–0 · White wins",Some("black")=>"0–1 · Black wins",_ if matches!(game["status"].as_str(),Some("draw"|"stalemate"))=>"½–½ · Draw",_=>"No decisive result" };
                        let result=format!("{result} · {}",game["status"].as_str().unwrap_or("Finished"));
                        ui.add(egui::Label::new(&result).truncate()).on_hover_text(&result);
                        if let Some(opening)=game["opening"]["name"].as_str() { ui.add(egui::Label::new(RichText::new(opening).strong()).truncate()).on_hover_text(opening); }
                        if let Some(moves)=game["moves"].as_str() { let summary=format!("{} moves · {}",moves.split_whitespace().count().div_ceil(2),moves.split_whitespace().take(10).collect::<Vec<_>>().join(" ")); ui.add(egui::Label::new(RichText::new(&summary).small()).truncate()).on_hover_text(&summary); }
                    });
                });
                let id=game["id"].as_str().unwrap_or_default();
                ui.allocate_ui_with_layout(Vec2::new(controls_width,0.0),egui::Layout::top_down(egui::Align::Max), |ui| {
                        ui.set_min_width(ui.available_width());
                        if !id.is_empty() { ui.add(egui::Hyperlink::from_label_and_url("Open game on Lichess",format!("https://lichess.org/{id}")).open_in_new_tab(true)); }
                        let allowed=pgn.is_some() && !(self.fics_active && !self.fics_game_finished) && self.training.is_none() && !self.puzzle_navigation_locked();
                        if ui.add_enabled(allowed,Self::lichess_profile_button("Review"))
                            .on_hover_text("Review this game without adding it to Saved Games").on_disabled_hover_text("Finish active play or training before reviewing another game; refresh this profile if move data is missing.").clicked() {
                            self.review_lichess_profile_game(pgn.unwrap());
                        }
                });
            });
        });
        ui.add_space(6.0);
    }

}
fn lichess_rating_point(value: &serde_json::Value) -> Option<(f64,f32,String)> {
    let p=value.as_array()?;
    let year=p.first()?.as_i64()?; let month=p.get(1)?.as_i64()?+1; let day=p.get(2)?.as_i64()?;
    let rating=p.get(3)?.as_u64()? as f32;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) { return None; }
    // Gregorian day index keeps gaps between games proportional. API months are zero-based.
    let y=year-if month<=2 {1} else {0}; let era=y.div_euclid(400); let yoe=y-era*400;
    let m=month+if month>2 {-3} else {9};
    let days=era*146097+yoe*365+yoe/4-yoe/100+(153*m+2)/5+day-1;
    Some((days as f64,rating,format!("{year:04}-{month:02}-{day:02}")))
}

#[cfg(test)]
mod rating_history_tests {
    use super::*;
    #[test]
    fn history_dates_use_zero_based_months_and_real_calendar_spacing() {
        let a=lichess_rating_point(&serde_json::json!([2024,1,28,1500])).unwrap();
        let b=lichess_rating_point(&serde_json::json!([2024,2,1,1550])).unwrap();
        assert_eq!(a.2,"2024-02-28"); assert_eq!(b.0-a.0,2.0);
        assert!(lichess_rating_point(&serde_json::json!([2024,12,1,1500])).is_none());
        assert!(lichess_rating_point(&serde_json::json!([2024,0])).is_none());
    }
}

fn lichess_history_points(lobby: &serde_json::Value, name: &str) -> Vec<(f64, f32, String)> {
    lobby["ratingHistory"].as_array()
        .and_then(|series| series.iter().find(|series| series["name"].as_str().is_some_and(|key| key.eq_ignore_ascii_case(name))))
        .and_then(|series| series["points"].as_array())
        .map(|points| points.iter().filter_map(lichess_rating_point).collect()).unwrap_or_default()
}

#[cfg(test)]
mod rating_history_api_tests {
    use super::*;
    #[test]
    fn lowercase_api_time_controls_populate_display_tabs() {
        let lobby = serde_json::json!({"ratingHistory":[
            {"name":"blitz","points":[[2026,9,14,2997]]},
            {"name":"rapid","points":[]},
            {"name":"correspondence","points":[[2023,6,21,1258]]}
        ]});
        assert_eq!(lichess_history_points(&lobby,"Blitz")[0].1,2997.0);
        assert_eq!(lichess_history_points(&lobby,"Correspondence").len(),1);
        assert!(lichess_history_points(&lobby,"Rapid").is_empty());
    }
}

fn lichess_profile_bio(ui: &mut egui::Ui, profile: &serde_json::Value) {
    let bio=&profile["profile"];
    if let Some(name)=bio["realName"].as_str() { ui.strong(name); }
    if let Some(text)=bio["bio"].as_str().filter(|s|!s.is_empty()) { profile_link_text(ui,text); } else { ui.weak("No biography provided."); }
    ui.add_space(8.0);
    if let Some(rating)=bio["fideRating"].as_u64() {
        let url=profile["fideId"].as_u64().map(|id|format!("https://ratings.fide.com/profile/{id}")).unwrap_or_else(||"https://ratings.fide.com/".into());
        ui.add(egui::Hyperlink::from_label_and_url(format!("FIDE rating: {rating} ↗"),url).open_in_new_tab(true));
    }
    if let Some(flag)=bio["flag"].as_str() { ui.label(format!("Country / flag: {flag}")); }
    if let Some(ms)=profile["createdAt"].as_u64() { ui.label(format!("Member since {}",profile_date(ms))); }
    if let Some(ms)=profile["seenAt"].as_u64() { ui.label(format!("Last active {}",profile_date(ms))); }
    if let Some(seconds)=profile["playTime"]["total"].as_u64() { ui.label(format!("Time spent playing: {} days, {} hours",seconds/86400,seconds%86400/3600)); }
    if let Some(links)=bio["links"].as_str().filter(|s|!s.is_empty()) { ui.add_space(6.0); ui.strong("Links"); profile_link_text(ui,links); }
}
fn profile_link_text(ui: &mut egui::Ui, text: &str) {
    for line in text.lines() {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x=3.0;
            for word in line.split_whitespace() {
                if let Some(url)=profile_word_url(word) {ui.add(egui::Hyperlink::from_label_and_url(word,url).open_in_new_tab(true));}
                else {ui.label(word);}
            }
        });
    }
}
fn profile_word_url(word: &str) -> Option<String> {
    let word=word.trim_matches(|c:char|matches!(c,','|'.'|';'|'!'|'?'|'('|')'|'['|']'));
    if word.starts_with("https://") || word.starts_with("http://") {Some(word.into())}
    else if word.starts_with("www.") {Some(format!("https://{word}"))}
    else if !word.starts_with('@') && word.split_once('@').is_some_and(|(name,host)|!name.is_empty() && host.contains('.')) {Some(format!("mailto:{word}"))}
    else {
        let host=word.split('/').next().unwrap_or_default();
        let tld=host.rsplit('.').next().unwrap_or_default();
        if host.contains('.') && host.chars().all(|c|c.is_ascii_alphanumeric() || matches!(c,'.'|'-')) && tld.len()>=2 && tld.chars().all(|c|c.is_ascii_alphabetic()) {Some(format!("https://{word}"))} else {None}
    }
}
fn lichess_recent_game_preview(game: &serde_json::Value) -> Result<serde_json::Value,String> {
    let pgn=game["pgn"].as_str().ok_or("Move data is missing. Refresh the player profile.")?;
    let (positions,moves)=ChessApp::parse_pgn_mainline(pgn)?;
    let white=ChessApp::pgn_tag(pgn,"White").unwrap_or_else(||"White".into());
    let black=ChessApp::pgn_tag(pgn,"Black").unwrap_or_else(||"Black".into());
    Ok(serde_json::json!({"title":format!("{white} vs {black}"),"white":white,"black":black,"result":ChessApp::pgn_tag(pgn,"Result"),"playedAt":ChessApp::pgn_tag(pgn,"Date"),"finalFen":positions.last().map(ToString::to_string),"preview":{"positions":positions.iter().map(ToString::to_string).collect::<Vec<_>>(),"moves":moves}}))
}

fn profile_today_day() -> f64 {
    #[cfg(target_arch="wasm32")]
    let ms=js_sys::Date::now();
    #[cfg(not(target_arch="wasm32"))]
    let ms=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as f64;
    (ms/86400000.0).floor()+719468.0
}
fn profile_date(ms: u64) -> String {
    let (year,month,day)=profile_civil_date((ms/86400000) as i64+719468);
    format!("{year:04}-{month:02}-{day:02}")
}
fn profile_civil_date(days: i64) -> (i64,i64,i64) {
    let era=days.div_euclid(146097); let doe=days-era*146097;
    let yoe=(doe-doe/1460+doe/36524-doe/146096)/365;
    let y=yoe+era*400; let doy=doe-(365*yoe+yoe/4-yoe/100);
    let mp=(5*doy+2)/153; let d=doy-(153*mp+2)/5+1;
    let m=mp+if mp<10 {3} else {-9};
    (y+if m<=2 {1} else {0},m,d)
}
fn profile_year_start(today: f64) -> f64 {
    let (year,_,_)=profile_civil_date(today as i64);
    lichess_rating_point(&serde_json::json!([year,0,1,0])).unwrap().0
}

#[cfg(test)]
mod player_profile_date_tests {
    use super::*;
    #[test]
    fn unix_dates_and_year_filter_align_with_history_calendar() {
        assert_eq!(profile_date(0), "1970-01-01");
        let day=lichess_rating_point(&serde_json::json!([2024,1,29,1500])).unwrap().0;
        assert_eq!(profile_civil_date(day as i64),(2024,2,29));
        assert_eq!(profile_civil_date(profile_year_start(day) as i64),(2024,1,1));
    }
}

#[cfg(test)]
mod player_profile_layout_tests {
    use super::*;
    #[test]
    fn profile_without_history_still_displays_bio_record_and_recent_games_at_both_widths() {
        for width in [600.0, 1200.0] {
            let ctx=egui::Context::default();
            let mut app=ChessApp::new(&eframe::CreationContext::_new_kittest(ctx.clone()));
            app.lichess.profile_windows.push(PlayerProfileWindow::new(0,""));
            app.lichess.lobby=serde_json::json!({"connected":true,"profileUsername":"Example","historyUsername":"Example","ratingHistory":[],
                "playerProfile":{"username":"Example","title":"GM","profile":{"realName":"Example Player","bio":"A player biography"},"count":{"all":42,"rated":40,"win":20,"loss":12,"draw":10}},
                "recentGames":[{"id":"abcd1234","speed":"blitz","rated":true,"status":"resign","winner":"black","clock":{"initial":180,"increment":0},"lastFen":"8/8/1N1k4/1Pp3p1/2P2nP1/3bp1KP/8/8 w - - 0 1","opening":{"name":"English Defense"},"players":{"white":{"user":{"name":"WhitePlayer"},"rating":2800},"black":{"user":{"name":"Example"},"rating":2997}}}]
            });
            let mut output=None;
            for _ in 0..3 {
                output=Some(run_test_ui(&ctx,egui::RawInput{screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,Vec2::new(width,1200.0))),..Default::default()},|_|app.lichess_history_ui(&ctx)));
            }
            let output=output.unwrap();
            let mut texts:Vec<String>=output.shapes.iter().filter_map(|shape|if let egui::Shape::Text(text)=&shape.shape {Some(text.galley.job.text.clone())} else {None}).collect();
            // Narrow profiles legitimately scroll; verify the games heading is reachable.
            for _ in 0..4 {
                let output=run_test_ui(&ctx,egui::RawInput{screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,Vec2::new(width,1200.0))),events:vec![
                    egui::Event::PointerMoved(egui::pos2(width*0.5,300.0)),
                    egui::Event::MouseWheel{phase:egui::TouchPhase::Move,unit:egui::MouseWheelUnit::Point,delta:Vec2::new(0.0,-220.0),modifiers:egui::Modifiers::default()}
                ],..Default::default()},|_|app.lichess_history_ui(&ctx));
                texts.extend(output.shapes.iter().filter_map(|shape|if let egui::Shape::Text(text)=&shape.shape {Some(text.galley.job.text.clone())} else {None}));
            }
            assert!(texts.iter().any(|text|text=="biography"),"Biography absent at {width}");
            assert!(texts.iter().any(|text|text=="Recent games"),"Games heading absent at {width}: {texts:?}");
            // The card can be below the scroll viewport; inspect it independently.
            let card_ctx=egui::Context::default();
            let game=app.lichess.lobby["recentGames"][0].clone();
            let card=run_test_ui(&card_ctx,egui::RawInput{screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,Vec2::new(width,600.0))),..Default::default()},|ui|app.lichess_profile_game(ui,&game));
            assert!(card.shapes.iter().any(|shape|matches!(&shape.shape,egui::Shape::Text(text) if text.galley.job.text=="English Defense")),"Recent-game card absent at {width}");
            assert!(texts.iter().any(|text|text=="Wins"),"Record absent at {width}");
        }
    }
}

#[cfg(test)]
mod player_profile_resize_tests {
    use super::*;
    fn profile() -> serde_json::Value {
        serde_json::json!({"username":"cutemouse83","title":"GM","profile":{"bio":"A biography with words that must wrap in the available column. ".repeat(12)},"count":{"all":19642,"rated":19598,"win":12270,"loss":4246,"draw":3126},"perfs":{
            "bullet":{"rating":1756,"games":2,"prov":true},"blitz":{"rating":2997,"games":19590},"rapid":{"games":0},"classical":{"games":0},"correspondence":{"rating":1258,"games":1,"prov":true},"puzzle":{"rating":2300,"games":41,"prov":true}
        }})
    }
    #[test]
    fn rating_chips_wrap_as_complete_cards_without_vertical_letter_stacking() {
        for width in [360.0,551.9,552.0,600.0,735.9,736.0,960.0] {
            let ctx=egui::Context::default();
            let mut app=ChessApp::new(&eframe::CreationContext::_new_kittest(ctx.clone()));
            let data=profile();
            let output=run_test_ui(&ctx,egui::RawInput{screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,Vec2::new(width,1000.0))),..Default::default()},|ui|app.lichess_profile_ratings(ui,&data,&mut PlayerProfileWindow::new(0,"")));
            let labels:Vec<_>=output.shapes.iter().filter_map(|shape|if let egui::Shape::Text(text)=&shape.shape {Some(text)} else {None})
                .filter(|text| ["Bullet","Blitz","Rapid","Classical","Correspondence","Puzzles"].iter().any(|name|text.galley.job.text.starts_with(name))).collect();
            assert_eq!(labels.len(),6);
            for label in &labels {
                assert_eq!(label.galley.rows.len(),1,"Chip text wrapped at {width}: {}",label.galley.job.text);
                assert!(label.pos.x+label.galley.size().x<=width,"Chip outside viewport at {width}");
            }
            assert!(labels[5].pos.y>labels[0].pos.y,"Last chip should wrap at {width}");
        }
    }
    #[test]
    fn profile_header_can_drag_the_window() {
        let ctx=egui::Context::default();
        let mut app=ChessApp::new(&eframe::CreationContext::_new_kittest(ctx.clone()));
        app.lichess.profile_windows.push(PlayerProfileWindow::new(0,""));
        app.lichess.lobby=serde_json::json!({"connected":true,"playerProfile":profile(),"recentGames":[]});
        let id=egui::Id::new(("lichess_player_profile_window",0u64));
        let mut time=0.0;
        let mut frame=|events| {
            time+=0.1;
            run_test_ui(&ctx,egui::RawInput{time:Some(time),events,screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,Vec2::new(1600.0,1200.0))),..Default::default()},|_|app.lichess_history_ui(&ctx));
        };
        for _ in 0..4 { frame(vec![]); }
        let before=ctx.memory(|m|m.area_rect(id)).unwrap();
        let start=before.min+Vec2::new(180.0,22.0);
        let end=start+Vec2::new(120.0,90.0);
        frame(vec![egui::Event::PointerMoved(start)]);
        frame(vec![egui::Event::PointerButton{pos:start,button:egui::PointerButton::Primary,pressed:true,modifiers:Default::default()}]);
        frame(vec![egui::Event::PointerMoved(end)]);
        frame(vec![egui::Event::PointerButton{pos:end,button:egui::PointerButton::Primary,pressed:false,modifiers:Default::default()}]);
        frame(vec![]);
        let after=ctx.memory(|m|m.area_rect(id)).unwrap();
        assert!(after.left()>before.left()+80.0 && after.top()>before.top()+50.0,"Header did not drag window: {before:?} -> {after:?}");
    }

    #[test]
    fn resized_profile_stays_small_after_drag_release_and_idle_frames() {
        let ctx=egui::Context::default();
        let mut app=ChessApp::new(&eframe::CreationContext::_new_kittest(ctx.clone()));
        app.lichess.profile_windows.push(PlayerProfileWindow::new(0,""));
        app.lichess.lobby=serde_json::json!({"connected":true,"playerProfile":profile(),"historyUsername":"cutemouse83","ratingHistory":[],"recentGames":[]});
        let id=egui::Id::new(("lichess_player_profile_window",0u64));
        let mut time=0.0;
        let mut frame=|events| {
            time+=0.1;
            run_test_ui(&ctx,egui::RawInput{time:Some(time),events,screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,Vec2::new(1400.0,1100.0))),..Default::default()},|_|app.lichess_history_ui(&ctx));
        };
        for _ in 0..4 {frame(vec![]);}
        let original=ctx.memory(|m|m.area_rect(id)).unwrap();
        let handle=original.right_bottom()-Vec2::splat(2.0);
        let target=handle-Vec2::new(420.0,300.0);
        frame(vec![egui::Event::PointerMoved(handle)]);
        frame(vec![egui::Event::PointerButton{pos:handle,button:egui::PointerButton::Primary,pressed:true,modifiers:Default::default()}]);
        frame(vec![egui::Event::PointerMoved(target)]);
        frame(vec![egui::Event::PointerButton{pos:target,button:egui::PointerButton::Primary,pressed:false,modifiers:Default::default()}]);
        for _ in 0..3 {frame(vec![]);}
        let resized=ctx.memory(|m|m.area_rect(id)).unwrap();
        assert!(resized.width()<original.width()-200.0,"Window failed to shrink: {original:?} -> {resized:?}");
        assert!(resized.height()<original.height()-150.0,"Window height failed to shrink: {original:?} -> {resized:?}");
        for _ in 0..10 {frame(vec![]);}
        let idle=ctx.memory(|m|m.area_rect(id)).unwrap();
        assert!((idle.width()-resized.width()).abs()<1.0,"Window regrew horizontally");
        assert!((idle.height()-resized.height()).abs()<1.0,"Window regrew vertically");
    }
}

fn lichess_profile_window_title(profile: &serde_json::Value) -> String {
    let Some(username)=profile["username"].as_str() else {return "Lichess player profile".into();};
    let id=profile["id"].as_str().unwrap_or(username);
    let name=profile["profile"]["realName"].as_str().filter(|name|!name.is_empty()).unwrap_or(username);
    format!("Lichess player profile · {name} [{id}]")
}

#[cfg(test)]
mod player_profile_follow_tests {
    use super::*;
    #[test]
    fn window_title_includes_real_name_and_id_with_username_fallback() {
        assert_eq!(lichess_profile_window_title(&serde_json::json!({"id":"cutemouse83","username":"cutemouse83","profile":{"realName":"Michael Roiz"}})),"Lichess player profile · Michael Roiz [cutemouse83]");
        assert_eq!(lichess_profile_window_title(&serde_json::json!({"id":"alice","username":"Alice"})),"Lichess player profile · Alice [alice]");
        assert_eq!(lichess_profile_window_title(&serde_json::Value::Null),"Lichess player profile");
    }
}

fn lichess_profile_window_data(lobby: &serde_json::Value, id: u64) -> serde_json::Value {
    let mut data=if lobby["profileWindows"].is_object() {lobby["profileWindows"][id.to_string()].clone()} else {lobby.clone()};
    if !data.is_object() {data=serde_json::json!({});}
    for key in ["connected","account","following","followingLoading","followBusy","followError","followPermissionNeeded"] {
        data[key]=lobby[key].clone();
    }
    data
}

#[cfg(test)]
mod player_profile_comparison_tests {
    use super::*;
    #[test]
    fn record_counters_wrap_without_stacking_digits_or_label_letters() {
        for width in [360.0,560.0,960.0] {
            let ctx=egui::Context::default();
            let profile=serde_json::json!({"count":{"all":19642,"rated":19598,"win":12270,"loss":4246,"draw":3126}});
            let output=run_test_ui(&ctx,egui::RawInput{screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,Vec2::new(width,700.0))),..Default::default()},|ui|ChessApp::lichess_profile_record(ui,&profile));
            let texts:Vec<_>=output.shapes.iter().filter_map(|shape|if let egui::Shape::Text(text)=&shape.shape {Some(text)} else {None}).collect();
            for label in ["19,642","19,598","12,270","4,246","3,126","Games","Rated","Wins","Losses","Draws"] {
                let text=texts.iter().find(|text|text.galley.job.text==label).unwrap();
                assert_eq!(text.galley.rows.len(),1,"Stacked {label} at {width}");
                assert!(text.pos.x+text.galley.size().x<=width,"Counter outside viewport");
            }
        }
    }
    #[test]
    fn comparison_windows_keep_separate_geometry_and_graph_filters() {
        let ctx=egui::Context::default();
        let mut app=ChessApp::new(&eframe::CreationContext::_new_kittest(ctx.clone()));
        let mut first=PlayerProfileWindow::new(1,"Alice"); first.control=1;first.range=0;first.shown_username="Alice".into();
        let mut second=PlayerProfileWindow::new(2,"Bob"); second.control=2;second.range=5;second.shown_username="Bob".into();
        app.lichess.profile_windows=vec![first,second];
        app.lichess.lobby=serde_json::json!({"connected":true,"account":"Alice","profileWindows":{
            "1":{"playerProfile":{"id":"alice","username":"Alice"},"historyUsername":"Alice","ratingHistory":[]},
            "2":{"playerProfile":{"id":"bob","username":"Bob"},"historyUsername":"Bob","ratingHistory":[]}
        }});
        for _ in 0..3 {run_test_ui(&ctx,egui::RawInput{screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,Vec2::new(1600.0,1200.0))),..Default::default()},|_|app.lichess_history_ui(&ctx));}
        let a=ctx.memory(|m|m.area_rect(egui::Id::new(("lichess_player_profile_window",1u64)))).unwrap();
        let b=ctx.memory(|m|m.area_rect(egui::Id::new(("lichess_player_profile_window",2u64)))).unwrap();
        assert_ne!(a.min,b.min);
        assert_eq!((app.lichess.profile_windows[0].control,app.lichess.profile_windows[0].range),(1,0));
        assert_eq!((app.lichess.profile_windows[1].control,app.lichess.profile_windows[1].range),(2,5));
        assert_eq!(lichess_profile_window_data(&app.lichess.lobby,1)["playerProfile"]["username"],"Alice");
        assert_eq!(lichess_profile_window_data(&app.lichess.lobby,2)["playerProfile"]["username"],"Bob");
    }
}

fn profile_record_number(value: u64) -> String {
    let digits=value.to_string();
    let mut output=String::new();
    for (index,digit) in digits.chars().enumerate() {
        if index>0 && (digits.len()-index)%3==0 {output.push(',');}
        output.push(digit);
    }
    output
}

#[cfg(test)]
mod player_profile_review_tests {
    use super::*;
    const PGN:&str="[White \"Alice\"]\n[Black \"Bob\"]\n[Result \"*\"]\n\n1. e4 e5 2. Nf3 *";
    #[test]
    fn preview_contains_every_position_without_changing_the_board() {
        let ctx=egui::Context::default();let app=ChessApp::new(&eframe::CreationContext::_new_kittest(ctx));
        let board=app.board;
        let preview=lichess_recent_game_preview(&serde_json::json!({"pgn":PGN})).unwrap();
        assert_eq!(preview["title"],"Alice vs Bob");
        assert_eq!(preview["white"],"Alice");assert_eq!(preview["black"],"Bob");
        assert_eq!(preview["preview"]["positions"].as_array().unwrap().len(),4);
        assert_eq!(preview["preview"]["moves"].as_array().unwrap().len(),3);
        assert_eq!(app.board,board);
    }
    #[test]
    fn full_board_review_is_transient_until_a_normal_game_is_started() {
        let ctx=egui::Context::default();let mut app=ChessApp::new(&eframe::CreationContext::_new_kittest(ctx));
        app.review_lichess_profile_game(PGN);
        assert!(app.transient_profile_review);assert_eq!(app.review_index,Some(0));assert_eq!(app.review_moves.len(),3);
        app.review_to(3);app.save_game();assert!(app.transient_profile_review);
        app.load_pgn("not a valid PGN with chess moves");assert!(app.transient_profile_review);
        app.reset_for_side(PlayerSide::White);assert!(!app.transient_profile_review);
    }
    #[test]
    fn explicitly_saving_profile_review_preserves_full_game_at_first_move() {
        let ctx=egui::Context::default();let mut app=ChessApp::new(&eframe::CreationContext::_new_kittest(ctx));
        app.review_lichess_profile_game(PGN);
        assert!(app.transient_profile_review);
        assert_eq!(app.review_index,Some(0));
        let before=app.persisted_game();
        app.save_current_game();
        assert!(!app.transient_profile_review);
        let saved=app.persisted_game();
        assert_eq!(saved.review_pgn,before.review_pgn);
        assert!(saved.review_pgn.as_ref().is_some_and(|pgn|pgn.contains("1. e4")));
        assert_eq!(app.review_moves.len(),3);
        assert_eq!(app.review_index,Some(0));
    }
    #[test]
    fn active_online_play_cannot_be_replaced_by_profile_review() {
        let ctx=egui::Context::default();let mut app=ChessApp::new(&eframe::CreationContext::_new_kittest(ctx));
        app.fics_active=true;app.fics_game_finished=false;
        let board=app.board;app.review_lichess_profile_game(PGN);
        assert_eq!(app.board,board);assert!(!app.transient_profile_review);assert!(!app.lichess.profile_action_error.is_empty());
    }
    #[test]
    fn profile_urls_support_sites_email_and_punctuation_without_executable_schemes() {
        assert_eq!(profile_word_url("www.chessbasic.com,"),Some("https://www.chessbasic.com".into()));
        assert_eq!(profile_word_url("https://x.com/RoizMichael83"),Some("https://x.com/RoizMichael83".into()));
        assert_eq!(profile_word_url("name@example.com"),Some("mailto:name@example.com".into()));
        assert_eq!(profile_word_url("chessbasic.com"),Some("https://chessbasic.com".into()));
        assert_eq!(profile_word_url("javascript:alert(1)"),None);
    }
}

#[cfg(test)]
mod player_profile_reflow_tests {
    use super::*;
    #[test]
    fn repeated_resize_frames_keep_record_rows_and_recent_game_height_stable() {
        let ctx=egui::Context::default();
        let mut app=ChessApp::new(&eframe::CreationContext::_new_kittest(ctx.clone()));
        let profile=serde_json::json!({"count":{"all":19642,"rated":19598,"win":12270,"loss":4246,"draw":3126}});
        let game=serde_json::json!({"id":"abcd1234","speed":"blitz","rated":true,"status":"resign","winner":"black","createdAt":1792003006187i64,"clock":{"initial":180,"increment":0},"lastFen":"8/8/1N1k4/1Pp3p1/2P2nP1/3bp1KP/8/8 w - - 0 1","opening":{"name":"A long opening name with several words and variations"},"moves":"e4 e5 Nf3 Nc6 Bb5 a6","players":{"white":{"user":{"name":"LongPlayerNameWhite","title":"GM"},"rating":2800},"black":{"user":{"name":"LongPlayerNameBlack","title":"GM"},"rating":2997}}});
        let mut card_height:Option<f32>=None;
        for width in [360.0,479.5,480.0,560.0,600.0,960.0,560.0,360.0] {
            let mut record_height:Option<f32>=None;
            for _ in 0..4 {
                run_test_ui(&ctx,egui::RawInput{screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,Vec2::new(width,1200.0))),..Default::default()},|ui| {
                    let top=ui.cursor().top(); ChessApp::lichess_profile_record(ui,&profile);
                    let height=ui.cursor().top()-top;
                    if let Some(previous)=record_height {assert!((height-previous).abs()<0.1,"Record oscillated at width {width}");}
                    record_height=Some(height);
                    let top=ui.cursor().top(); app.lichess_profile_game(ui,&game);
                    let height=ui.cursor().top()-top;
                    if let Some(previous)=card_height {assert!((height-previous).abs()<0.1,"Game card changed height at width {width}");}
                    card_height=Some(height);
                });
            }
        }
    }
}

#[cfg(test)]
mod player_profile_performance_tests {
    use super::*;
    #[test]
    fn details_show_statistics_and_existing_game_controls_without_changing_board() {
        for width in [360.0,960.0] {
            let ctx=egui::Context::default();
            let mut app=ChessApp::new(&eframe::CreationContext::_new_kittest(ctx.clone()));
            let original=app.board;
            let mut window=PlayerProfileWindow::new(7,"Bob");window.performance_control=Some(1);
            let game=serde_json::json!({"id":"best1234","pgn":"[White \"Alice\"]\n[Black \"Bob\"]\n[Result \"1-0\"]\n\n1. e4 e5 2. Nf3 1-0","speed":"blitz","status":"resign","winner":"white","players":{"white":{"user":{"name":"Alice"}},"black":{"user":{"name":"Bob"}}}});
            let data=serde_json::json!({"profileUsername":"Bob","performance":{"blitz":{"loading":false,"data":{"perf":{"glicko":{"rating":2000}},"rank":10,"percentile":99,"stat":{"highest":{"int":2200,"at":"2026-10-01T00:00:00Z"},"count":{"all":50,"win":30,"loss":15,"draw":5,"seconds":3600},"resultStreak":{"win":{"cur":{"v":2},"max":{"v":5}}},"bestWins":{"results":[{"gameId":"best1234"}]},"worstLosses":{"results":[{"gameId":"best1234"}]}}},"games":{"best1234":game}}}});
            let output=run_test_ui(&ctx,egui::RawInput{screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,Vec2::new(width,2200.0))),..Default::default()},|ui|app.lichess_profile_performance(ui,&mut window,&data));
            let texts=output.shapes.iter().filter_map(|shape|if let egui::Shape::Text(text)=&shape.shape {Some(text.galley.job.text.as_str())}else{None}).collect::<Vec<_>>();
            for expected in ["Blitz · Performance details","Rating: 2000","Rank: 10","Percentile: 99%","Best wins","Worst losses","Open game on Lichess","Review"] {
                assert!(texts.contains(&expected),"Missing {expected} at {width}: {texts:?}");
            }
            assert_eq!(app.board,original);assert!(!app.transient_profile_review);
            assert!(ctx.data(|d|d.get_temp::<String>(egui::Id::new(("lichess_performance_final_fen","best1234")))).is_some());
        }
    }
    #[test]
    fn loading_another_player_closes_previous_performance_details() {
        let mut window=PlayerProfileWindow::new(3,"Alice");window.performance_control=Some(2);
        window.username="Bob".into();window.load(false);
        assert_eq!(window.performance_control,None);
    }
}

#[cfg(test)]
mod player_profile_activity_tests {
    use super::*;
    #[test]
    fn activity_displays_daily_results_puzzles_and_rating_changes() {
        let ctx=egui::Context::default();
        let mut app=ChessApp::new(&eframe::CreationContext::_new_kittest(ctx.clone()));
        let window=PlayerProfileWindow::new(1,"Alice");
        let data=serde_json::json!({"profileUsername":"Alice","activity":{"loading":false,"days":[{"interval":{"start":0},"games":{"blitz":{"win":3,"loss":2,"draw":1,"rp":{"before":2000,"after":2010}}},"puzzles":{"score":{"win":2,"loss":1,"rp":{"before":1500,"after":1490}}}}]}});
        let mut seen=Vec::new();
        for frame in 0..3 {
            let output=run_test_ui(&ctx,egui::RawInput{time:Some(frame as f64),..Default::default()},|ui| {
                app.lichess_profile_activity_contents(ui,&window,&data);
            });
            seen.extend(output.shapes.iter().filter_map(|shape|if let egui::Shape::Text(text)=&shape.shape {Some(text.galley.job.text.clone())}else{None}));
        }
        for expected in ["1970-01-01","Blitz: 3 wins · 2 losses · 1 draws","Puzzles: 2 solved · 1 missed","Rating 2000 to 2010 (+10)","Rating 1500 to 1490 (-10)"] {
            assert!(seen.iter().any(|text|text==expected),"Missing {expected}: {seen:?}");
        }
    }
}
