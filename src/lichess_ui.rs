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
    fn lichess_computer_setup(&mut self, ui: &mut egui::Ui, seeking: bool) {
        let gold = Color32::from_rgb(211, 173, 98);
        let dark = Color32::from_rgb(43, 43, 40);
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
                    if ui.add_sized([width, 38.0], egui::Button::new(RichText::new(*label).size(16.0))
                        .fill(Color32::TRANSPARENT).stroke(Stroke::new(if selected { 2.0 } else { 0.0 }, gold))).clicked() {
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
                    if ui.add_sized([strength_width, 38.0], egui::Button::new(RichText::new(level.to_string()).size(16.0)
                        .color(if selected { Color32::BLACK } else { Color32::LIGHT_GRAY }))
                        .fill(if selected { gold } else { dark })).clicked() { self.lichess.ai_level = level; }
                }
            });
            ui.add_space(18.0);
            ui.label(RichText::new("Side").size(16.0).strong());
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                for (key, label, color) in [("white", "White", Color::White), ("random", "Random side", Color::White), ("black", "Black", Color::Black)] {
                    let selected = self.lichess.color == key;
                    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 78.0), egui::Sense::click());
                    ui.painter().rect_filled(rect, 5.0, if selected { gold } else { dark });
                    let icon = egui::Rect::from_center_size(rect.center_top() + Vec2::new(0.0, 27.0), Vec2::splat(36.0));
                    Self::paint_piece(ui, icon, 36.0, self.piece_set, color, Piece::King);
                    if key == "random" {
                        Self::paint_piece(ui, icon.translate(Vec2::new(10.0, 0.0)), 36.0, self.piece_set, Color::Black, Piece::King);
                    }
                    ui.painter().text(rect.center_bottom() - Vec2::new(0.0, 15.0), egui::Align2::CENTER_CENTER,
                        label, egui::FontId::proportional(15.0), if selected { Color32::BLACK } else { Color32::LIGHT_GRAY });
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
            if ui.add_enabled(!seeking && valid, egui::Button::new(RichText::new("Play against computer").size(17.0).strong().color(Color32::BLACK))
                .fill(gold).corner_radius(6.0).min_size(Vec2::new(0.0, 42.0))).clicked() {
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

    fn start_lichess(&mut self) {
        self.enter_lichess();
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
                    if event["connected"].as_bool() == Some(true) && !self.lichess.active {
                        self.enter_lichess();
                    }
                    self.lichess.lobby = event;
                    if self.lichess.active {
                        self.fics_connected = self.lichess.connected();
                        self.fics_status =
                            self.lichess.lobby["status"].as_str().unwrap_or("").into();
                        self.engine_status = format!("Lichess · {}", self.fics_status);
                    }
                }
                "loading" => {
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
                    self.enter_lichess();
                    self.receive_lichess_game(event);
                }
                "parked" => {
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
            let center = if self.board_3d_active {
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

    fn lichess_menu(&mut self, ui: &mut egui::Ui) {
        Self::set_menu_item_font(ui);
        ui.set_min_width(280.0);
        ui.label(
            self.lichess.lobby["status"]
                .as_str()
                .unwrap_or("Sign in to play on Lichess"),
        );
        if ui.button("Lichess lobby and game controls…").clicked() {
            self.lichess.open = true;
            ui.close();
        }
        if !self.lichess.active && ui.button("Play on Lichess").clicked() {
            self.start_lichess();
            ui.close();
        }
        if self.lichess.active && !self.lichess.connected() && ui.button("Reconnect").clicked() {
            self.start_lichess();
            ui.close();
        }
        Self::lichess_account_link(ui);
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
                self.fics_resign_dialog_open = true;
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

    fn lichess_ui(&mut self, ctx: &egui::Context) {
        if !self.lichess.open {
            return;
        }
        let mut open = true;
        let mut close_after = false;
        egui::Window::new("Lichess lobby").title_bar(false).collapsible(false)
            .fixed_size(Vec2::new(760.0, 800.0)).resizable(false).anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .frame(Self::dialog_frame()).show(ctx, |ui| {
            ui.set_min_size(Vec2::new(760.0, 800.0));
            if Self::dialog_header(ui, "Lichess lobby") { open = false; }
            ui.spacing_mut().item_spacing = Vec2::new(10.0, 10.0);
            ui.spacing_mut().button_padding = Vec2::new(12.0, 7.0);
            let status = self.lichess.lobby["status"].as_str().unwrap_or("Sign in to play on Lichess");
            if status != "Connected to Lichess" { ui.label(status); }
            #[cfg(not(target_arch = "wasm32"))]
            ui.label("Lichess play is available in the browser app.");
            if !self.lichess.connected() {
                if ui.button("Sign in with Lichess").clicked() {
                    self.enter_lichess();
                    #[cfg(target_arch = "wasm32")]
                    lichess_sign_in();
                }
                if ui.button("Reconnect saved session").clicked() { self.start_lichess(); }
                ui.label("Sign-in lasts for this browser session. Engine assistance is disabled during online play.");
            } else {
                ui.horizontal(|ui| {
                    let green = Color32::from_rgb(136, 194, 151);
                    let (indicator, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                    ui.painter().circle_filled(indicator.center(), 4.0, green);
                    ui.colored_label(green, self.lichess.lobby["account"].as_str().unwrap_or("Connected"));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        Self::lichess_account_link(ui);
                    });
                });
                ui.add_space(2.0);
                egui::ScrollArea::vertical().id_salt("lichess_lobby_content").max_height(660.0).min_scrolled_height(660.0).auto_shrink([false, false]).show(ui, |ui| {
                Frame::new().fill(Color32::from_rgb(24, 28, 33)).corner_radius(8.0).inner_margin(16.0).show(ui, |ui| {


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
                        if ui.add_enabled(supported, egui::Button::new(RichText::new(label).size(15.0)).min_size(Vec2::new(ui.available_width(), 58.0))).on_hover_text(format!("Resume game {id}")).clicked() {
                            lichess_command(serde_json::json!({"type":"open", "id":id}));
                        }
                        if !supported { ui.hyperlink_to("Open game on Lichess", format!("https://lichess.org/{id}")); }
                    }
                    if self.fics_playing && self.lichess.active {
                        if self.lichess.game["speed"].as_str() == Some("correspondence") {
                            if ui.add_enabled(!self.fics_pending_move, egui::Button::new("Return to lobby · keep this game ongoing")).clicked() {
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
                        ui.horizontal(|ui| {
                            for (index, label) in [(0, "Quick pairing"), (1, "Create a game"), (2, "Correspondence")] {
                                if ui.selectable_label(self.lichess.lobby_tab == index, RichText::new(label).size(16.0)).clicked() {
                                    self.lichess.lobby_tab = index;
                                    self.lichess.correspondence = index == 2;
                                }
                            }
                        });
                        ui.separator();
                        if self.lichess.lobby_tab == 0 {
                            ui.horizontal_top(|ui| {
                                ui.vertical(|ui| {
                                    ui.set_width(460.0);
                                    egui::Grid::new("lichess_quick_pairing").spacing(Vec2::splat(10.0)).show(ui, |ui| {
                                        for (index, (minutes, increment, speed)) in [
                                            (10, 0, "Rapid"), (10, 5, "Rapid"), (15, 10, "Rapid"),
                                            (30, 0, "Classical"), (30, 20, "Classical"), (0, 0, "Custom"),
                                        ].into_iter().enumerate() {
                                            let label = if minutes == 0 { "Custom".to_owned() } else { format!("{minutes}+{increment}\n{speed}") };
                                            if ui.add_enabled(!seeking, egui::Button::new(RichText::new(label).size(23.0))
                                                .min_size(Vec2::new(145.0, 100.0)).fill(Color32::from_rgb(43, 43, 40))
                                                .corner_radius(7.0)).clicked() {
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
                                            if index % 3 == 2 { ui.end_row(); }
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
                                        if ui.add_sized([210.0, 46.0], egui::Button::new(RichText::new(label).size(15.0)).corner_radius(6.0)).clicked() {
                                            self.lichess.lobby_tab = tab;
                                            self.lichess.correspondence = false;
                                        }
                                    }
                                });
                            });
                        }
                        if seeking {
                            ui.label("Finding an opponent…");
                            if ui.button("Cancel matchmaking").clicked() { lichess_command(serde_json::json!({"type":"cancelSeek"})); }
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
                                    if ui.add_sized([84.0, 66.0], egui::Button::new(RichText::new(format!("{days}\n{}", if days == 1 { "day" } else { "days" })).size(18.0))
                                        .fill(if selected { Color32::from_rgb(58, 49, 32) } else { Color32::from_rgb(43, 43, 40) })
                                        .stroke(Stroke::new(1.0, if selected { Color32::from_rgb(211, 173, 98) } else { Color32::from_white_alpha(28) }))
                                        .corner_radius(6.0)).clicked() { self.lichess.days = days; }
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
                        ui.horizontal(|ui| {
                            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
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
                            ui.add_space(12.0);
                            ui.label("Play as");
                            egui::ComboBox::from_id_salt("lichess_color").selected_text(&self.lichess.color).show_ui(ui, |ui| {
                                for color in ["random", "white", "black"] { ui.selectable_value(&mut self.lichess.color, color.into(), color); }
                            });
                            });
                        });
                        }
                        if !seeking && self.lichess.lobby_tab <= 2 {
                            let rapid = self.lichess.correspondence || self.lichess.minutes * 60 + self.lichess.increment * 40 >= 480;
                            if ui.add_enabled(rapid, egui::Button::new(RichText::new(if self.lichess.lobby_tab == 2 { "Find a correspondence opponent" } else { "Find opponent" }).size(16.0).strong().color(Color32::from_rgb(24, 27, 31))).fill(Color32::from_rgb(211, 173, 98)).min_size(Vec2::new(0.0, 40.0))).clicked() {
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
                            if ui.add_enabled(!seeking && !self.lichess.opponent.trim().is_empty(), egui::Button::new("Challenge")).clicked() {
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
                                if ui.add_enabled(action == "decline" || (supported && !self.fics_playing), egui::Button::new(action)).clicked() {
                                    lichess_command(serde_json::json!({"type":"challengeAction", "id":challenge["id"], "action":action}));
                                }
                            }
                        });
                    }
                    let outgoing = self.lichess.lobby["outgoing"].as_array().cloned().unwrap_or_default();
                    for challenge in outgoing {
                        ui.horizontal(|ui| {
                            ui.label(format!("Challenge pending: {}", challenge["destUser"]["name"].as_str().unwrap_or("opponent")));
                            if ui.button("Cancel challenge").clicked() { lichess_command(serde_json::json!({"type":"challengeAction", "id":challenge["id"], "action":"cancel"})); }
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
            ui.separator();
            ui.horizontal(|ui| {
            if ui.button("Leave online play").clicked() {
                if self.lichess.active { self.stop_fics(); }
                close_after = true;
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if self.lichess.connected() && ui.add_enabled(!self.fics_playing, egui::Button::new("Sign out")).clicked() {
                    #[cfg(target_arch = "wasm32")]
                    lichess_sign_out();
                }
            });
            });
        });
        self.lichess.open = open && !close_after;
    }
}

#[cfg(test)]
mod lichess_tests {
    use super::*;
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
