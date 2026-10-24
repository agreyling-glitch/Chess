use super::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodTrainingCompanionMode)]
    fn training_companion_mode() -> bool;
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodTrainingCompanionOpen)]
    fn training_companion_open() -> bool;
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodOpenTrainingCompanion)]
    fn open_training_companion() -> bool;
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodCloseTrainingCompanion)]
    fn close_training_companion();
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodPublishTrainingLive)]
    fn publish_training_live(live: bool);
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodReadTrainingLive)]
    fn read_training_live() -> bool;
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodTakeTrainingChanged)]
    fn take_training_changed() -> bool;
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodStartTrainingInMain)]
    fn start_training_in_main(id: &str) -> bool;
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodTakeTrainingStarts)]
    fn take_training_starts() -> String;
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodTrainingProfilesAction)]
    fn profile_action(command: &str, payload: &str) -> String;
}
#[cfg(not(target_arch = "wasm32"))]
fn profile_action(_: &str, _: &str) -> String { r#"{"error":"Profile transfer is available in the browser app."}"#.into() }

#[derive(Clone, Default)]
struct ProfileManager {
    selected: Vec<String>,
    message: String,
    busy: bool,
    delete_open: bool,
    delete_games: bool,
    engine: training::TrainingEngine,
    starting_level: u8,
    starting_elo: i32,
}

impl ChessApp {
    pub(super) fn companion_training_blocked(&self) -> bool {
        self.training.as_ref().is_some_and(|s| s.engine.is_companion()) && !crate::companion::enabled() && self.game_result() == "*"
    }
    fn maia3_training_choice(&self, config: &EngineConfig)->Option<training::TrainingEngine> {
        if config.opponent!=OpponentEngine::Companion || !config.label().to_ascii_lowercase().replace(['-',' '],"").contains("maia3") {return None;}
        // A fresh profile pop-out has the saved engine identity but no in-memory
        // option catalog yet. Recognize Maia3's published UCI contract there;
        // when discovered options exist, reject incompatible rating controls.
        if self.engine_uci_catalog.get(&Self::verification_engine_key(config)).is_some_and(|options|
            !["Elo","SelfElo","OppoElo"].iter().all(|name|options.iter().any(|o|o.name==*name && o.kind=="spin" && o.min.unwrap_or(0)<=0 && o.max.unwrap_or(5000)>=5000))) {return None;}
        Some(training::TrainingEngine::Maia3{identity:config.companion_identity.clone(),name:config.companion_name.clone(),options:config.companion_options.clone()})
    }
    fn training_engine_choices(&self, enabled: bool, catalog: &serde_json::Value) -> Vec<(training::TrainingEngine, String)> {
        let mut choices = vec![(training::TrainingEngine::Stockfish, "Built-in Stockfish · target Elo".into())];
        if !enabled { return choices; }
        let mut seen = std::collections::BTreeSet::new();
        for entry in catalog["engines"].as_array().into_iter().flatten() {
            let (Some(identity), Some(name), Some(label)) = (entry["identity"].as_str(), entry["engine_label"].as_str(), entry["label"].as_str()) else { continue; };
            if identity.is_empty() || !seen.insert(identity.to_owned()) { continue; }
            let mut config = EngineConfig { opponent: OpponentEngine::Companion, companion_identity: identity.into(), companion_name: name.into(), ..EngineConfig::default() };
            for saved in [&self.engine_config, &self.analysis_config] {
                if saved.opponent == OpponentEngine::Companion && saved.companion_identity == identity { config.companion_options = saved.companion_options.clone(); }
            }
            if let Some(options) = self.engine_uci_preferences.get(&Self::verification_engine_key(&config)) { config.companion_options = options.clone(); }
            if let Some(engine) = self.maia3_training_choice(&config) {
                choices.push((engine, format!("{label} · adaptive Elo")));
            } else {
                let normalized = name.to_ascii_lowercase();
                if normalized.contains("lc0") || normalized.contains("leela") {
                    choices.push((training::TrainingEngine::Lc0 { identity: identity.into(), name: name.into(), options: config.companion_options }, format!("{label} · node-based levels")));
                }
            }
        }
        choices
    }
    pub(super) fn training_engine_config(session: &Session) -> EngineConfig {
        let mut config = EngineConfig { limit_strength:true, elo:session.opponent_elo.max(1320) as u32,
            threads:Self::engine_thread_count(), move_time_ms:1000, multipv:1, ..EngineConfig::default() };
        if let training::TrainingEngine::Lc0 {identity,name,options} | training::TrainingEngine::Maia3 {identity,name,options} = &session.engine {
            config.opponent = OpponentEngine::Companion;
            config.companion_identity = identity.clone(); config.companion_name = name.clone();
            config.companion_options = options.clone(); config.limit_strength = false;
            config.search_limit = SearchLimit::Nodes; config.nodes = session.node_budget.max(1);
            if session.engine.is_maia3() {
                config.elo=session.opponent_elo.clamp(training::MAIA_MIN_ELO,training::MAIA_MAX_ELO) as u32;
                for name in ["Elo","SelfElo","OppoElo"] {config.companion_options.insert(name.into(),config.elo.to_string());}
            }
        }
        config
    }
    pub(super) fn training_companion_ui(&mut self, ui: &mut egui::Ui) {
        #[cfg(target_arch = "wasm32")]
        if training_companion_mode() {
            self.poll_training_companion(ui.ctx());
            self.training_dialog_open = true;
            self.training_dialog(ui.ctx());
        }
    }

    pub(super) fn poll_training_companion(&mut self, ctx: &egui::Context) {
        #[cfg(target_arch = "wasm32")]
        {
            if take_training_changed() {
                match Self::read_training_profiles() {
                    Ok(profiles) => self.training_profiles = profiles,
                    Err(error) => self.training_error = Some(error),
                }
            }
            if !training_companion_mode() {
                publish_training_live(self.training_live());
                if let Ok(starts) = serde_json::from_str::<Vec<String>>(&take_training_starts()) {
                    for id in starts {
                        if self.training_live() || self.profile_management_busy() { continue; }
                        if let Ok(mut profiles) = Self::read_training_profiles() {
                            if !profiles.profiles.iter().any(|profile| profile.id == id) { continue; }
                            profiles.selected_id = id;
                            self.training_profiles = profiles;
                            self.start_training_game();
                        }
                    }
                }
            }
            if training_companion_mode() || training_companion_open() { ctx.request_repaint_after(std::time::Duration::from_millis(100)); }
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = ctx;
    }

    pub(super) fn is_training_companion() -> bool {
        #[cfg(target_arch = "wasm32")]
        { return training_companion_mode(); }
        #[cfg(not(target_arch = "wasm32"))]
        false
    }
    fn training_sunken_section<R>(
        ui: &mut egui::Ui,
        contents: impl FnOnce(&mut egui::Ui) -> R,
    ) -> egui::InnerResponse<R> {
        let section = Frame::new()
            .fill(Color32::from_rgb(13, 17, 22))
            .stroke(Stroke::new(1.0, Color32::from_white_alpha(25)))
            .corner_radius(CornerRadius::same(6))
            .inner_margin(Margin::same(14))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                contents(ui)
            });
        Self::paint_sunken_border(ui, section.response.rect);
        section
    }

    pub(super) fn training_draw(&self) -> bool {
        let positions = &self.review_positions;
        let board = positions.last().unwrap_or(&self.board);
        if board.status() != BoardStatus::Ongoing {
            return false;
        }
        if positions
            .iter()
            .filter(|p| p.get_hash() == board.get_hash())
            .count()
            >= 3
        {
            return true;
        }
        if self.review_moves.len() >= 100 {
            let start = self.review_moves.len() - 100;
            if (start..self.review_moves.len()).all(|i| {
                let Some(mv) = Self::parse_san_move(&positions[i], &self.review_moves[i]) else {
                    return false;
                };
                positions[i].piece_on(mv.get_source()) != Some(Piece::Pawn)
                    && positions[i].piece_on(mv.get_dest()).is_none()
            }) {
                return true;
            }
        }
        use shakmaty::Position;
        board
            .to_string()
            .parse::<shakmaty::fen::Fen>()
            .ok()
            .and_then(|fen| {
                fen.into_position::<shakmaty::Chess>(shakmaty::CastlingMode::Standard)
                    .ok()
            })
            .is_some_and(|position| position.is_insufficient_material())
    }
    pub(super) fn training_live(&self) -> bool {
        #[cfg(target_arch = "wasm32")]
        if training_companion_mode() { return read_training_live(); }
        self.training.is_some() && self.game_result() == "*"
    }

    fn training_id() -> String {
        let mut bytes = [0u8; 16];
        if getrandom::fill(&mut bytes).is_ok() {
            bytes.iter().map(|byte| format!("{byte:02x}")).collect()
        } else {
            format!("training-{}", Self::animation_time().to_bits())
        }
    }

    fn training_now() -> String {
        #[cfg(target_arch = "wasm32")]
        {
            js_sys::Date::new_0()
                .to_iso_string()
                .as_string()
                .unwrap_or_default()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            format!(
                "{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            )
        }
    }

    pub(super) fn read_training_profiles() -> Result<Profiles, String> {
        #[cfg(target_arch = "wasm32")]
        {
            let storage = web_sys::window()
                .and_then(|w| w.local_storage().ok().flatten())
                .ok_or("Training profiles need browser storage. Enable storage and try again.")?;
            let json = storage
                .get_item(training::STORAGE_KEY)
                .map_err(|_| "Could not read training profiles.")?;
            match json {
                Some(json) => serde_json::from_str(&json).map_err(|_| "Training profiles could not be read. Restore a backup before starting new training games.".into()),
                None => Ok(Profiles::default()),
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            Ok(Profiles::default())
        }
    }

    fn write_training_profiles(&self, profiles: &Profiles) -> Result<(), String> {
        #[cfg(target_arch = "wasm32")]
        {
            let storage = web_sys::window()
                .and_then(|w| w.local_storage().ok().flatten())
                .ok_or("Browser storage is unavailable; training progress has not been saved.")?;
            let json = serde_json::to_string(profiles)
                .map_err(|_| "Could not serialize training profiles.")?;
            storage.set_item(training::STORAGE_KEY, &json)
                .map_err(|_| "Could not save training progress. Free browser storage or back up your games, then retry.".into())
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = profiles;
            Ok(())
        }
    }

    fn create_training_profile(&mut self) {
        let name: String = self.training_profile_name.trim().chars().take(60).collect();
        if name.is_empty() {
            return;
        }
        let mut profiles = self.training_profiles.clone();
        let id = Self::training_id();
        let draft = self.puzzle_context.data(|d| d.get_temp::<ProfileManager>(egui::Id::new("training_profile_manager"))).unwrap_or_default();
        if draft.engine.is_companion() && !crate::companion::enabled() {
            self.training_error = Some("Enable Companion in Version diagnostics to use this training profile.".into()); return;
        }
        let (min_elo,max_elo)=draft.engine.elo_bounds();
        profiles.profiles.push(Profile {
            engine: draft.engine,
            starting_level: draft.starting_level.clamp(1, training::NODE_LEVELS.len() as u8),
            starting_elo: draft.starting_elo.clamp(min_elo,max_elo),
            id: id.clone(),
            name,
            results: vec![],
        });
        profiles.selected_id = id;
        match self.write_training_profiles(&profiles) {
            Ok(()) => {
                self.training_profiles = profiles;
                self.training_profile_name.clear();
                self.training_error = None;
            }
            Err(error) => self.training_error = Some(error),
        }
    }

    fn training_analysis_stats(&self, side: TrainingSide) -> AnalysisStats {
        let color = if side == TrainingSide::White {
            Color::White
        } else {
            Color::Black
        };
        let mut stats = AnalysisStats::default();
        for index in 1..self.review_positions.len() {
            let board = &self.review_positions[index - 1];
            if board.side_to_move() != color {
                continue;
            }
            stats.total_player_moves += 1;
            let Some(loss) = self.move_quality_loss(index) else {
                continue;
            };
            // Opening is the first ten full moves; low non-pawn material
            // identifies endgames after that. These are transparent heuristics.
            let material: u32 = [
                (Piece::Knight, 3),
                (Piece::Bishop, 3),
                (Piece::Rook, 5),
                (Piece::Queen, 9),
            ]
            .iter()
            .map(|(piece, points)| board.pieces(*piece).popcnt() * points)
            .sum();
            let phase = if index <= 20 {
                0
            } else if material <= 26 {
                2
            } else {
                1
            };
            stats.phases[phase].add(self.move_centipawn_loss(index), loss);
            match self.move_mate_outcome(index) {
                Some(MateOutcome::Missed) => stats.missed_mates += 1,
                Some(MateOutcome::Allowed) => stats.allowed_mates += 1,
                _ => {}
            }
        }
        stats
    }

    pub(super) fn settle_training(&mut self) {
        let Some(session) = self.training.clone() else {
            return;
        };
        // Retained history from a deleted profile remains available for review.
        if session.rating_after.is_some() && !self.training_profiles.profiles.iter().any(|p| p.id == session.profile_id) { return; }
        let result = self.game_result();
        if result == "*" {
            return;
        }
        let stats = self.training_analysis_stats(session.side);
        if session.rating_after.is_some()
            && self
                .training_profiles
                .profiles
                .iter()
                .find(|p| p.id == session.profile_id)
                .and_then(|p| p.results.iter().find(|r| r.session.id == session.id))
                .is_some_and(|r| r.analysis == stats)
        {
            return;
        }
        #[cfg(target_arch = "wasm32")]
        let mut profiles = match Self::read_training_profiles() {
            Ok(p) => p,
            Err(error) => {
                self.training_error = Some(error);
                return;
            }
        };
        #[cfg(not(target_arch = "wasm32"))]
        let mut profiles = self.training_profiles.clone();
        let Some(profile) = profiles
            .profiles
            .iter_mut()
            .find(|p| p.id == session.profile_id)
        else {
            self.training_error = Some("This game's training profile is missing. Restore its profile backup to record the result.".into());
            return;
        };
        let timed_out = self
            .local_clock
            .as_ref()
            .is_some_and(|c| c.flagged_white == Some(session.side == TrainingSide::White));
        profile.settle(&session, &result, Self::training_now(), timed_out);
        let Some(entry) = profile
            .results
            .iter_mut()
            .find(|r| r.session.id == session.id)
        else {
            return;
        };
        entry.analysis = stats;
        let settled = entry.session.clone();
        match self.write_training_profiles(&profiles) {
            Ok(()) => {
                self.training_profiles = profiles;
                self.training = Some(settled);
                self.training_error = None;
                self.save_game();
            }
            Err(error) => self.training_error = Some(error),
        }
    }

    pub(super) fn profile_management_busy(&self) -> bool {
        self.puzzle_context.data(|d| d.get_temp::<ProfileManager>(egui::Id::new("training_profile_manager")))
            .is_some_and(|manager| manager.busy || manager.delete_open)
    }

    pub(super) fn start_training_game(&mut self) {
        #[cfg(target_arch = "wasm32")]
        if training_companion_mode() {
            if let Some(profile) = self.training_profiles.selected() {
                if !start_training_in_main(&profile.id) { self.training_error = Some("Keep the main Ironwood window open to start a training game.".into()); }
            }
            return;
        }
        if self.profile_management_busy() { return; }
        self.settle_training();
        if self.training_error.is_some() {
            return;
        }
        let Some(profile) = self.training_profiles.selected() else {
            return;
        };
        let session = profile.session(Self::training_id(), Self::training_now());
        if session.engine.is_companion() && !crate::companion::enabled() {
            self.training_error = Some("Enable Companion in Version diagnostics to use this training profile.".into()); return;
        }
        if self.fics_active {
            self.stop_fics();
        }
        self.new_game_training = true;
        self.compact_panel = CompactPanel::Moves;
        self.engine_enabled = true;
        self.engine_config = Self::training_engine_config(&session);
        #[cfg(target_arch = "wasm32")]
        {
            self.ensure_engine_provider(self.engine_config.clone());
            self.engine = None; self.engine_ready = false; self.engine_searching = false;
        }
        self.realtime_analysis_due_at = None;
        let side = if session.side == TrainingSide::White {
            PlayerSide::White
        } else {
            PlayerSide::Black
        };
        self.reset_for_side(side);
        self.review_positions = vec![self.board];
        self.local_clock = None;
        let opponent = session.opponent_label();
        self.review_white_player = if side == PlayerSide::White {
            session.profile_name.clone()
        } else {
            opponent.clone()
        };
        self.review_black_player = if side == PlayerSide::Black {
            session.profile_name.clone()
        } else {
            opponent
        };
        self.training = Some(session);
        self.new_game_dialog_open = false;
        self.training_dialog_open = false;
        self.resume_engine_after_ready = false;
        self.save_game();
        #[cfg(target_arch = "wasm32")]
        self.queue_live_engine_resume();
    }

    pub(super) fn training_setup_ui(&mut self, ui: &mut egui::Ui) {
        let manager_id = egui::Id::new("training_profile_manager");
        let mut manager = ui.ctx().data(|d| d.get_temp::<ProfileManager>(manager_id)).unwrap_or_default();
        let companion_enabled = crate::companion::enabled();
        if !companion_enabled {
            manager.engine = training::TrainingEngine::Stockfish;
        }
        if let training::TrainingEngine::Lc0 { identity, options, .. } | training::TrainingEngine::Maia3 { identity, options, .. } = &mut manager.engine {
            if *identity == self.analysis_config.companion_identity { *options = self.analysis_config.companion_options.clone(); }
        }
        if let Ok(status) = serde_json::from_str::<serde_json::Value>(&profile_action("poll", "")) {
            manager.busy = status["busy"].as_bool().unwrap_or(false);
            if let Some(message) = status["message"].as_str().or(status["error"].as_str()) { manager.message = message.into(); }
            if status["changed"].as_bool() == Some(true) {
                match Self::read_training_profiles() {
                    Ok(profiles) => {
                        self.training_profiles = profiles;
                        self.training_error = None;
                        manager.selected.retain(|id| self.training_profiles.profiles.iter().any(|p| &p.id == id));
                        if self.training.as_ref().is_some_and(|session| !self.training_profiles.profiles.iter().any(|p| p.id == session.profile_id)) {
                            self.training = None;
                        }
                        self.training_profile_rename = None;
                    }
                    Err(error) => self.training_error = Some(error),
                }
            }
        }
        if manager.busy { ui.ctx().request_repaint_after(std::time::Duration::from_millis(150)); }
        if !companion_enabled {
            manager.selected.retain(|id| self.training_profiles.profiles.iter().any(|p| &p.id == id && !p.engine.is_companion()));
            if self.training_profiles.selected().is_some_and(|p| p.engine.is_companion()) {
                self.training_profiles.selected_id = self.training_profiles.profiles.iter().find(|p| !p.engine.is_companion()).map(|p| p.id.clone()).unwrap_or_default();
            }
        }
        if self.training_error.as_deref() == Some("Enable Companion in Version diagnostics to use this training profile.")
            && (companion_enabled || self.training_profiles.selected().is_some_and(|p| !p.engine.is_companion())) {
            self.training_error = None;
        }
        let roomy = self.training_dialog_open && !self.new_game_dialog_open;
        let gold = Color32::from_rgb(211, 173, 98);
        let muted = Color32::from_rgb(164, 171, 180);
        ui.label(
            RichText::new("Your next step toward stronger chess")
                .size(23.0)
                .strong(),
        );
        ui.label(
            RichText::new("A personal rating. A matching opponent. Both sides of the board.")
                .size(if roomy { 16.0 } else { 14.0 })
                .color(muted),
        );
        ui.add_space(if roomy { 24.0 } else { 14.0 });
        Self::training_sunken_section(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(
                    RichText::new("TRAINING PROFILE")
                        .size(if roomy { 16.0 } else { 12.0 })
                        .strong()
                        .color(gold),
                );
                ui.add_space(7.0);
                let old_selected = self.training_profiles.selected_id.clone();
                ui.add_enabled_ui(!manager.busy && !manager.delete_open && !self.training_live(), |ui| {
                    let table = Frame::new().fill(Color32::from_rgb(19, 24, 30))
                        .corner_radius(6.0).inner_margin(10.0).show(ui, |ui| {
                    ui.style_mut().wrap_mode=Some(egui::TextWrapMode::Truncate);
                    // Reserve checkbox, column gutters, and the radio indicator/padding.
                    let usable = (ui.available_width() - 28.0 - if companion_enabled {50.0} else {40.0} - 48.0).max(180.0);
                    let name_width = usable * if companion_enabled {0.28} else {0.43};
                    let engine_width = usable * 0.24;
                    let stat_width = usable * if companion_enabled {0.16} else {0.19};
                    ui.spacing_mut().interact_size.y = 32.0;
                    egui::ScrollArea::vertical().id_salt("training_profiles_table_scroll").max_height(220.0).show_gold(ui, |ui| {
                        egui::Grid::new(("training_profiles_table", companion_enabled)).striped(true).num_columns(if companion_enabled {6} else {5}).min_col_width(0.0).spacing(Vec2::new(10.0, 8.0)).show(ui, |ui| {
                            let visible_count = self.training_profiles.profiles.iter().filter(|p| companion_enabled || !p.engine.is_companion()).count();
                            let mut all = visible_count > 0 && manager.selected.len() == visible_count;
                            if ui.checkbox(&mut all, "").on_hover_text("Select all profiles").changed() {
                                manager.selected = if all { self.training_profiles.profiles.iter().filter(|p| companion_enabled || !p.engine.is_companion()).map(|p| p.id.clone()).collect() } else { Vec::new() };
                            }
                            Self::game_moves_glass_label(ui, Vec2::new(name_width, 30.0), "Profile", true);
                            if companion_enabled { Self::game_moves_glass_label(ui, Vec2::new(engine_width, 30.0), "Engine", true); }
                            for (label, width) in [("White", stat_width), ("Black", stat_width), ("Games", stat_width)] {
                                Self::game_moves_glass_label(ui, Vec2::new(width, 30.0), label, true);
                            }
                            ui.end_row();
                            for profile in &self.training_profiles.profiles {
                                if profile.engine.is_companion() && !companion_enabled { continue; }
                                let mut selected = manager.selected.contains(&profile.id);
                                if ui.checkbox(&mut selected, "").changed() {
                                    if selected { manager.selected.push(profile.id.clone()); } else { manager.selected.retain(|id| id != &profile.id); }
                                }
                                let active = self.training_profiles.selected_id == profile.id;
                                if ui.add_sized([name_width, 34.0], egui::RadioButton::new(active,
                                    RichText::new(&profile.name).strong().color(if active { gold } else { Color32::from_rgb(228, 231, 234) })))
                                    .on_hover_text(format!("{} · {}\nUse this profile for training and puzzle focus",profile.name,profile.engine.label())).clicked() {
                                    self.training_profiles.selected_id = profile.id.clone();
                                }
                                if companion_enabled {
                                    let engine = if profile.engine.is_companion() { profile.engine.label() } else { "Stockfish 19" };
                                    ui.add_sized([engine_width, 34.0], egui::Label::new(engine).truncate()).on_hover_text(engine);
                                }
                                let difficulty = |side| if profile.engine.is_lc0() { format!("Level {}",profile.rating(side)) } else { format!("{} Elo",profile.rating(side)) };
                                for value in [difficulty(TrainingSide::White), difficulty(TrainingSide::Black), profile.results.len().to_string()] {
                                    ui.add_sized([stat_width, 34.0], egui::Label::new(RichText::new(value).size(18.0).color(Color32::from_rgb(235, 226, 205))));
                                }
                                ui.end_row();
                            }
                        });
                    });
                    });
                    Self::paint_sunken_border(ui, table.response.rect);
                });
                ui.add_space(4.0);
                ui.small("Choose a profile by its name. Check rows for rename, delete, or export.");
                if self.training_live() { ui.small("Finish your active training game to manage profiles."); }
                ui.horizontal_wrapped(|ui| {
                    let enabled = !manager.busy && !manager.delete_open && !self.training_live() && self.training_error.is_none();
                    if ui.add_enabled(enabled && manager.selected.len() == 1, egui::Button::new(RichText::new("Rename").color(gold)).min_size(Vec2::new(88.0, 34.0)).fill(Color32::from_rgb(34, 41, 47)).stroke(Stroke::new(1.0, Color32::from_rgb(103, 88, 59))).corner_radius(5.0))
                        .on_disabled_hover_text("Select one profile to rename; finish any active training game first.").clicked() {
                        self.training_profiles.selected_id = manager.selected[0].clone();
                        self.training_profile_rename = self.training_profiles.selected().map(|p| p.name.clone());
                    }
                    if ui.add_enabled(enabled && !manager.selected.is_empty(), egui::Button::new(RichText::new("Delete").color(Color32::from_rgb(225, 163, 151))).min_size(Vec2::new(88.0, 34.0)).fill(Color32::from_rgb(34, 41, 47)).stroke(Stroke::new(1.0, Color32::from_rgb(103, 88, 59))).corner_radius(5.0)).clicked() {
                        manager.delete_open = true; manager.delete_games = false;
                    }
                    if ui.add_enabled(enabled && !manager.selected.is_empty(), egui::Button::new(RichText::new("Export").color(gold)).min_size(Vec2::new(88.0, 34.0)).fill(Color32::from_rgb(34, 41, 47)).stroke(Stroke::new(1.0, Color32::from_rgb(103, 88, 59))).corner_radius(5.0)).clicked() {
                        profile_action("export", &serde_json::json!({"ids":manager.selected}).to_string());
                        manager.busy = true;
                    }
                    if ui.add_enabled(enabled, egui::Button::new(RichText::new("Import").color(gold)).min_size(Vec2::new(88.0, 34.0)).fill(Color32::from_rgb(34, 41, 47)).stroke(Stroke::new(1.0, Color32::from_rgb(103, 88, 59))).corner_radius(5.0)).clicked() {
                        profile_action("import", ""); manager.busy = true;
                    }
                });
                if manager.busy { ui.spinner(); }
                if !manager.message.is_empty() { ui.label(&manager.message); }
                if old_selected != self.training_profiles.selected_id {
                    self.training_profile_rename = None;
                    if let Err(error) = self.write_training_profiles(&self.training_profiles) {
                        self.training_error = Some(error);
                    }
                }
                if let Some(profile) = self.training_profiles.selected().cloned() {
                    let rename_enabled = !manager.busy && !manager.delete_open && !self.training_live();
                    if let Some(name) = &mut self.training_profile_rename {
                        let mut save = false;
                        let mut cancel = false;
                        ui.horizontal_wrapped(|ui| {
                            ui.add(egui::TextEdit::singleline(name).desired_width((ui.available_width() - 160.0).max(100.0)).char_limit(60)
                                .text_color(Color32::from_rgb(238, 238, 238)));
                            save = ui.add_enabled(!name.trim().is_empty() && self.training_error.is_none() && rename_enabled, egui::Button::new("Save name")).clicked();
                            cancel = ui.button("Cancel").clicked();
                        });
                        if save {
                            let renamed: String = name.trim().chars().take(60).collect();
                            let mut profiles = self.training_profiles.clone();
                            if let Some(p) = profiles.profiles.iter_mut().find(|p| p.id == profile.id) {
                                p.name = renamed;
                            }
                            match self.write_training_profiles(&profiles) {
                                Ok(()) => {
                                    self.training_profiles = profiles;
                                    self.training_profile_rename = None;
                                }
                                Err(error) => self.training_error = Some(error),
                            }
                        } else if cancel {
                            self.training_profile_rename = None;
                        }
                    }
                    ui.add_space(7.0);
                }
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(4.0);
                ui.label(RichText::new("Create a new profile").strong().color(gold));
                ui.add_enabled_ui(!manager.busy && !manager.delete_open && !self.training_live(), |ui| {
                    if companion_enabled {
                        let previous_engine=manager.engine.clone();
                        #[cfg(target_arch = "wasm32")]
                        let catalog: serde_json::Value = serde_json::from_str(&companion_engine_catalog()).unwrap_or_default();
                        #[cfg(not(target_arch = "wasm32"))]
                        let catalog = serde_json::Value::Null;
                        let choices = self.training_engine_choices(companion_enabled, &catalog);
                        let selected_label = choices.iter().find(|(engine, _)| engine == &manager.engine).map(|(_, label)| label.as_str()).unwrap_or(manager.engine.label()).to_owned();
                        ui.horizontal(|ui| {
                            egui::ComboBox::from_id_salt("training_profile_engine").selected_text(selected_label).show_ui(ui, |ui| {
                                for (engine, label) in choices { ui.selectable_value(&mut manager.engine, engine, label); }
                            });
                            #[cfg(target_arch = "wasm32")]
                            if Self::analysis_refresh_button(ui, catalog["busy"] != true, "Refresh training engines").clicked() { refresh_companion_engines(); }
                        });
                        if previous_engine != manager.engine {
                            if let training::TrainingEngine::Maia3 { options, .. } = &manager.engine {
                                manager.starting_elo = options.get("SelfElo").or_else(|| options.get("Elo")).and_then(|value| value.parse().ok()).unwrap_or(1500);
                            }
                        }
                        if catalog["busy"] == true { ui.small("Checking available training engines…"); }
                        ui.small("Choose a training engine. Each profile keeps its own engine options.");
                    }
                    if manager.engine.is_lc0() {
                        manager.starting_level = manager.starting_level.clamp(1, training::NODE_LEVELS.len() as u8);
                        ui.add(egui::Slider::new(&mut manager.starting_level, 1..=training::NODE_LEVELS.len() as u8).text("Starting level"));
                        ui.small(format!("{} nodes per move · levels are not calibrated Elo",training::NODE_LEVELS[manager.starting_level as usize - 1]));
                        ui.small("Create a new profile after replacing engine or network files so progress stays comparable.");
                    } else {
                        let (min,max)=manager.engine.elo_bounds();
                        manager.starting_elo = manager.starting_elo.clamp(min,max);
                        ui.add(egui::Slider::new(&mut manager.starting_elo, min..=max).text("Starting target Elo"));
                        if manager.engine.is_maia3() {ui.small("Maia3 adjusts its Elo after completed games. This tracks training difficulty, not an official rating.");}
                    }
                });
                ui.horizontal(|ui| {
                    let width = (ui.available_width() - 140.0 - ui.spacing().item_spacing.x - 2.0).max(80.0);
                    ui.add_sized(
                        [width, 40.0],
                        egui::TextEdit::singleline(&mut self.training_profile_name)
                            .hint_text(
                                RichText::new("New profile name")
                                    .color(Color32::from_rgb(164, 171, 180)),
                            )
                            .text_color(Color32::from_rgb(238, 238, 238))
                            .font(egui::TextStyle::Body)
                            .char_limit(60),
                    );
                    if ui
                        .add_enabled(
                            !self.training_profile_name.trim().is_empty()
                                && self.training_error.is_none() && !manager.busy && !self.training_live(),
                            egui::Button::new(RichText::new("Create profile").size(if roomy { 16.0 } else { 14.0 }).strong().color(Color32::from_rgb(25, 29, 35)))
                                .min_size(Vec2::new(140.0, 40.0))
                                .fill(gold).stroke(Stroke::new(1.0, Color32::from_rgb(241, 206, 131)))
                                .corner_radius(5.0).raised_gold(),
                        )
                        .clicked()
                    {
                        ui.ctx().data_mut(|d| d.insert_temp(manager_id, manager.clone()));
                        self.create_training_profile();
                    }
                });
                ui.add_space(if roomy { 10.0 } else { 5.0 });
                ui.label(
                    RichText::new("Your games and progress stay with this profile.")
                        .size(if roomy { 16.0 } else { 12.0 })
                        .color(muted),
                );
            });
        if manager.delete_open {
            let confirmation = egui::Modal::new(egui::Id::new("delete_training_profiles_confirmation"))
                .frame(Self::dialog_frame().inner_margin(Margin::same(24)))
                .show(ui.ctx(), |ui| {
                    ui.set_width((ui.ctx().content_rect().width() - 80.0).clamp(240.0, 440.0));
                    ui.label(RichText::new("PROFILE MANAGEMENT").size(11.0).strong().color(gold));
                    ui.add_space(6.0);
                    ui.label(RichText::new("Delete training profiles?").size(24.0).strong()
                        .color(Color32::from_rgb(240, 234, 220)));
                    ui.add_space(10.0);
                    ui.label(RichText::new("Their ratings and progress will be permanently removed.")
                        .size(14.0).color(muted));
                    ui.add_space(14.0);
                    Self::training_sunken_section(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(RichText::new("SELECTED PROFILES").size(11.0).strong().color(gold));
                        egui::ScrollArea::vertical().max_height(120.0).show(ui, |ui| {
                            for profile in self.training_profiles.profiles.iter().filter(|p| manager.selected.contains(&p.id)) {
                                ui.label(RichText::new(&profile.name).size(15.0).strong());
                            }
                        });
                    });
                    ui.add_space(14.0);
                    ui.checkbox(&mut manager.delete_games, RichText::new("Also delete their saved games").size(14.0));
                    ui.label(RichText::new(if manager.delete_games { "Saved games and their analysis will also be removed." } else { "Saved games will remain in Saved Games > Training." }).size(12.0).color(muted));
                    ui.add_space(12.0);
                    ui.label(RichText::new("This cannot be undone.").size(13.0).strong()
                        .color(Color32::from_rgb(235, 156, 139)));
                    ui.add_space(20.0);
                    ui.horizontal_wrapped(|ui| {
                        if ui.add(egui::Button::new(RichText::new("Cancel").size(14.0).strong()
                            .color(Color32::from_rgb(24, 27, 31)))
                            .min_size(Vec2::new(140.0, 42.0)).fill(gold).corner_radius(6.0).raised_gold()).clicked() { manager.delete_open = false; }
                        if ui.add_enabled(!manager.busy && !self.training_live(),
                            egui::Button::new(RichText::new("Delete profiles").size(14.0)
                                .color(Color32::from_rgb(255, 222, 215)))
                                .min_size(Vec2::new(164.0, 42.0)).corner_radius(6.0)
                                .stroke(Stroke::new(1.0, Color32::from_rgb(168, 89, 76)))
                                .fill(Color32::from_rgb(103, 48, 43))).clicked() {
                            profile_action("delete", &serde_json::json!({"ids":manager.selected,"deleteGames":manager.delete_games}).to_string());
                            manager.busy = true; manager.delete_open = false;
                        }
                    });
                });
            Self::paint_fics_window_border(&ui.ctx().layer_painter(confirmation.response.layer_id), confirmation.response.rect);
        }
        ui.ctx().data_mut(|d| d.insert_temp(manager_id, manager));
        if let Some(error) = &self.training_error {
            ui.colored_label(Color32::from_rgb(240, 150, 130), error);
            if ui.button("Retry storage").clicked() {
                match Self::read_training_profiles() {
                    Ok(p) => {
                        self.training_profiles = p;
                        self.training_error = None;
                        self.settle_training();
                    }
                    Err(error) => self.training_error = Some(error),
                }
            }
        }
        ui.add_space(if roomy { 24.0 } else { 14.0 });
        let profile = self.training_profiles.selected();
        let white = profile
            .map(|p| p.rating(TrainingSide::White))
            .unwrap_or(training::START_RATING);
        let black = profile
            .map(|p| p.rating(TrainingSide::Black))
            .unwrap_or(training::START_RATING);
        let overall = profile
            .map(|p| p.overall())
            .unwrap_or(training::START_RATING);
        ui.columns(3, |columns| {
            for (column, (label, rating, caption)) in columns.iter_mut().zip([
                ("WHITE", white, "First side"),
                ("BLACK", black, "Second side"),
                ("OVERALL", overall, "Lower of both"),
            ]) {
                Self::training_sunken_section(column, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(
                            RichText::new(label)
                                .size(if roomy { 16.0 } else { 12.0 })
                                .strong()
                                .color(if label == "OVERALL" { gold } else { muted }),
                        );
                        ui.label(RichText::new(format!("{}{}",if profile.is_some_and(|p| p.engine.is_lc0()) {"Level "} else {""},rating)).size(30.0).strong());
                        ui.label(RichText::new(caption).size(if roomy { 16.0 } else { 12.0 }).color(muted));
                    });
            }
        });
        ui.add_space(if roomy { 24.0 } else { 14.0 });
        let side = profile
            .map(|p| p.next_side())
            .unwrap_or(TrainingSide::White);
        let target = profile
            .map(|p| p.target(side))
            .unwrap_or(training::START_RATING);
        Self::training_sunken_section(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("NEXT: {}", side.label().to_uppercase()))
                    .size(if roomy { 16.0 } else { 14.0 })
                    .strong()
                    .color(gold),
            );
            ui.label(RichText::new(profile.map(|p| p.session(String::new(),String::new()).opponent_label()).unwrap_or_else(|| format!("Stockfish {target} Elo"))).size(if roomy { 16.0 } else { 14.0 }));
        });
        ui.label(
            RichText::new(if profile.is_some_and(|p| p.engine.is_lc0()) { "Standard chess · Unlimited time for you · lc0 uses the level's node budget" } else if profile.is_some_and(|p|p.engine.is_maia3()) { "Standard chess · Unlimited time for you · Maia3 predicts a human-style move at your target Elo" } else { "Standard chess  ·  Unlimited time for you  ·  Stockfish thinks up to 1 second per move" })
                .size(if roomy { 16.0 } else { 13.0 })
                .color(muted),
        );
        ui.add_space(if roomy { 24.0 } else { 12.0 });
        ui.label(
            RichText::new("Play. Alternate. Improve.")
                .size(16.0)
                .strong(),
        );
        ui.label(RichText::new(if profile.is_some_and(|p| p.engine.is_lc0()) { "Each color has its own level: a win raises it by one, a loss lowers it by one, and a draw keeps it unchanged. Your next game switches sides." } else if profile.is_some_and(|p|p.engine.is_maia3()) { "Each color progresses separately. Wins increase Maia3's target Elo, losses lower it, and draws keep it steady when ratings match. Your next game switches sides." } else { "Finish a game to update that color’s rating. Your next game switches sides, and Stockfish adjusts to your rating." }).size(if roomy { 16.0 } else { 13.0 }).color(muted));
        ui.add_space(if roomy { 10.0 } else { 5.0 });
        ui.label(RichText::new("Review your game and spot patterns afterward. Hints and takebacks are paused during training.").size(if roomy { 16.0 } else { 13.0 }).color(muted));
        ui.add_space(if roomy { 12.0 } else { 6.0 });
        ui.label(
            RichText::new(if profile.is_some_and(|p| p.engine.is_lc0()) { "Node-based difficulty levels · not Elo · even low levels can be strong with a powerful network" } else if profile.is_some_and(|p|p.engine.is_maia3()) { "Maia3 training difficulty · Elo guides human-move predictions, not a verified playing strength" } else { "Ironwood training Elo · not an official rating" })
                .size(if roomy { 16.0 } else { 11.0 })
                .color(muted),
        );
        #[cfg(not(target_arch = "wasm32"))]
        ui.label("Native preview: profiles remain in memory for this session. Use the web app for saved profiles.");
        });
    }

    pub(super) fn training_game_summary_ui(&mut self, ui: &mut egui::Ui) {
        let Some(session) = self.training.clone() else {
            return;
        };
        let gold = Color32::from_rgb(211, 173, 98);
        let muted = Color32::from_rgb(175, 182, 191);
        let finished = !self.training_live();
        let summary = Frame::new().fill(Color32::from_rgb(13, 17, 22))
            .stroke(Stroke::new(1.0, Color32::from_white_alpha(25)))
            .corner_radius(6.0).inner_margin(12.0).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.spacing_mut().item_spacing.y = 8.0;
        ui.label(RichText::new(if finished { "TRAINING RESULT" } else { "TRAINING GAME" }).size(10.0).strong().color(gold));
        ui.label(RichText::new(&session.profile_name).size(16.0).strong().color(Color32::from_rgb(238, 232, 219)));
        ui.label(RichText::new(format!("{} · {}", session.side.label(), session.opponent_label())).size(12.0).color(muted));
        if let Some(after) = session.rating_after {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(format!("{} to {}", session.rating_before, after)).size(20.0).strong().color(gold));
                let change = after - session.rating_before;
                ui.label(RichText::new(format!("{change:+}")).size(14.0).strong().color(
                    if change > 0 { Color32::from_rgb(136, 194, 151) }
                    else if change < 0 { Color32::from_rgb(240, 150, 130) } else { muted }));
            });
        } else {
            ui.label(RichText::new(format!("{} {}", session.rating_before, if session.engine.is_lc0() { "level" } else { "Elo" })).size(20.0).strong().color(gold));
        }
        if let Some(error) = &self.training_error {
            ui.colored_label(Color32::from_rgb(240, 150, 130), error);
        }
        ui.add_space(2.0);
        #[cfg(target_arch = "wasm32")]
        if !finished && session.engine.is_companion() && crate::companion::enabled() && self.engine_start_failed {
            ui.label("Check Game → Companion connection… before retrying.");
            if ui.button("Retry engine connection").clicked() {
                self.engine = None; self.engine_ready = false; self.engine_start_failed = false;
                self.queue_live_engine_resume();
            }
        }
            if finished && (!session.engine.is_companion() || crate::companion::enabled()) && ui.add_sized([ui.available_width(), 38.0], egui::Button::new(
                RichText::new("Next training game").size(13.0).strong().color(Color32::from_rgb(24, 27, 31)))
                .fill(gold).corner_radius(6.0).raised_gold()).clicked() {
                self.training_profiles.selected_id = session.profile_id.clone();
                self.start_training_game();
            }
            if ui.add_sized([ui.available_width(), 34.0], egui::Button::new(
                RichText::new("Profile & progress").size(13.0).strong().color(gold))
                .fill(Color32::from_rgb(43, 38, 29)).stroke(Stroke::new(1.0, gold.gamma_multiply(0.65)))
                .corner_radius(6.0)).clicked() {
                self.training_dialog_open = true;
            }
        });
        Self::paint_sunken_border(ui, summary.response.rect);
        ui.add_space(8.0);
    }

    fn training_rating_graph(ui: &mut egui::Ui, profile: &Profile) {
        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 160.0), Sense::hover());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 6.0, Color32::from_rgb(207, 177, 113));
        // Procedural paper fibers keep the gold grain subtle and stable between frames.
        let paper = rect.shrink(4.0);
        let mut grain = egui::Mesh::default();
        for row in 0..(paper.height() / 4.0).ceil() as u32 {
            for column in 0..(paper.width() / 4.0).ceil() as u32 {
                let hash = column.wrapping_mul(374761393)
                    .wrapping_add(row.wrapping_mul(668265263));
                let hash = (hash ^ (hash >> 13)).wrapping_mul(1274126177);
                let x = paper.left() + column as f32 * 4.0 + (hash & 3) as f32 * 0.5;
                let y = paper.top() + row as f32 * 4.0 + ((hash >> 2) & 3) as f32 * 0.5;
                let color = if hash & 16 == 0 {
                    Color32::from_rgba_unmultiplied(87, 56, 20, 20)
                } else {
                    Color32::from_rgba_unmultiplied(255, 241, 193, 32)
                };
                grain.add_colored_rect(
                    egui::Rect::from_min_size(egui::pos2(x, y), Vec2::new(1.0 + ((hash >> 5) & 3) as f32, 0.7)).intersect(paper),
                    color,
                );
            }
        }
        painter.add(egui::Shape::mesh(grain));
        let mut white = profile.initial_rating();
        let mut black = white;
        let mut points = vec![(white, black, white)];
        for entry in &profile.results {
            match entry.session.side {
                TrainingSide::White => white = entry.session.rating_after.unwrap_or(white),
                TrainingSide::Black => black = entry.session.rating_after.unwrap_or(black),
            }
            points.push((white, black, white.min(black)));
        }
        let min = points
            .iter()
            .flat_map(|p| [p.0, p.1, p.2])
            .min()
            .unwrap_or(1320)
            - if profile.engine.is_lc0() { 1 } else { 20 };
        let max = points
            .iter()
            .flat_map(|p| [p.0, p.1, p.2])
            .max()
            .unwrap_or(1320)
            + if profile.engine.is_lc0() { 1 } else { 20 };
        let plot = rect.shrink2(Vec2::new(50.0, 16.0));
        let y = |rating: i32| {
            plot.bottom() - (rating - min) as f32 / (max - min) as f32 * plot.height()
        };
        for rating in [min, profile.initial_rating(), max] {
            painter.line_segment(
                [
                    egui::pos2(plot.left(), y(rating)),
                    egui::pos2(plot.right(), y(rating)),
                ],
                Stroke::new(1.0, Color32::from_rgba_unmultiplied(83, 58, 24, 55)),
            );
            painter.text(
                egui::pos2(rect.left() + 4.0, y(rating)),
                Align2::LEFT_CENTER,
                rating.to_string(),
                FontId::proportional(15.0),
                Color32::from_rgb(65, 46, 23),
            );
        }
        for (line, color) in [
            Color32::from_rgb(255, 245, 215),
            Color32::from_rgb(36, 73, 105),
            Color32::from_rgb(128, 81, 26),
        ]
        .into_iter()
        .enumerate()
        {
            let data: Vec<_> = points
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    egui::pos2(
                        plot.left() + i as f32 / (points.len() - 1).max(1) as f32 * plot.width(),
                        y([p.0, p.1, p.2][line]),
                    )
                })
                .collect();
            if data.len() > 1 {
                if line == 0 {
                    painter.add(egui::Shape::line(data.clone(), Stroke::new(3.5, Color32::from_rgb(110, 83, 42))));
                }
                painter.add(egui::Shape::line(data, Stroke::new(2.0, color)));
            }
        }
        Self::paint_sunken_border(ui, rect);
        ui.horizontal_wrapped(|ui| {
            ui.colored_label(Color32::LIGHT_GRAY, "White");
            ui.colored_label(Color32::from_rgb(102, 162, 226), "Black");
            ui.colored_label(Color32::from_rgb(211, 173, 98), if profile.engine.is_lc0() { "Overall (lower level)" } else { "Overall (lower rating)" });
            ui.label("Completed games");
        });
    }

    fn training_focus_ui(ui: &mut egui::Ui, stats: &AnalysisStats, analyzed_games: usize, practice_enabled: bool) -> bool {
        let mut launched = false;
        let gold = Color32::from_rgb(211, 173, 98);
        let muted = Color32::from_rgb(175, 182, 191);
        let analyzed = stats.analyzed_moves();
        ui.label(format!("{analyzed} of {} player moves analyzed · {analyzed_games} {}", stats.total_player_moves, if analyzed_games == 1 { "game" } else { "games" }));
        let coverage = analyzed as f32 / stats.total_player_moves.max(1) as f32;
        ui.add(egui::ProgressBar::new(coverage).fill(Color32::from_rgb(102, 162, 226)).desired_width(ui.available_width()).text(format!("{:.0}% analysis coverage", coverage * 100.0)));
        ui.add_space(16.0);
        ui.label(RichText::new("Average move loss by phase").size(18.0).strong());
        ui.label(RichText::new("Lower is better · 100 centipawns (cp) = one pawn").size(16.0).color(muted));
        let scale = stats.phases.iter().filter(|p| p.cp_moves > 0).map(|p| p.mean_loss()).fold(100.0_f64, f64::max);
        for (name, phase) in ["Opening", "Middlegame", "Endgame"].iter().zip(&stats.phases) {
            ui.add_space(10.0);
            egui::Frame::new().fill(Color32::from_rgb(32, 37, 43)).corner_radius(8.0).inner_margin(14.0).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(*name).size(18.0).strong());
                    ui.label(RichText::new(if phase.cp_moves > 0 { format!("{:.0} cp average loss", phase.mean_loss()) } else { "No numerical evaluations yet".to_owned() }).size(17.0).color(gold));
                });
                let (rect, response) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 14.0), Sense::hover());
                ui.painter().rect_filled(rect, 4.0, Color32::from_rgb(48, 55, 63));
                if phase.cp_moves > 0 && phase.mean_loss() > 0.0 {
                    let bar = egui::Rect::from_min_size(rect.min, Vec2::new(rect.width() * (phase.mean_loss() / scale) as f32, rect.height()));
                    ui.painter().rect_filled(bar, 4.0, Color32::from_rgb(102, 162, 226));
                }
                response.on_hover_text(format!("Common scale: 0 to {scale:.0} cp. {} numerical evaluations; mate scores excluded.", phase.cp_moves));
                ui.add_space(5.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(format!("{} moves analyzed · {} numerical", phase.moves, phase.cp_moves)).color(muted));
                    ui.colored_label(gold, format!("{} mistakes", phase.mistakes));
                    ui.colored_label(Color32::from_rgb(236, 145, 132), format!("{} blunders", phase.blunders));
                });
            });
        }
        ui.add_space(18.0);
        egui::Frame::new().fill(Color32::from_rgb(42, 39, 31)).stroke(Stroke::new(1.0, gold)).corner_radius(8.0).inner_margin(16.0).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new("YOUR NEXT PRACTICE FOCUS").size(16.0).strong().color(gold));
            if let Some(i) = stats.practice_focus() {
                let phase = &stats.phases[i];
                ui.label(RichText::new(["Opening play", "Middlegame decisions", "Endgame technique"][i]).size(22.0).strong());
                ui.label(["Review development, king safety, and early tactical errors.", "Review your largest errors and compare the engine's alternative lines.", "Practice converting advantages and defending simplified positions."][i]);
                ui.add_space(6.0);
                ui.label(RichText::new(format!("Highest average loss: {:.0} cp across {} numerical evaluations. A practice suggestion based on the analyzed moves.", phase.mean_loss(), phase.cp_moves)).color(muted));
                launched = Self::phase_practice_ui(ui, i, practice_enabled);
            } else {
                ui.label(RichText::new("Build your analysis sample").size(22.0).strong());
                ui.label("Analyze completed games to find a recurring weakness. A phase needs at least 20 numerical evaluations and a mistake or blunder before a focus is suggested.");
            }
        });
        ui.add_space(16.0);
        launched
    }

    pub(super) fn training_dialog(&mut self, ctx: &egui::Context) {
        if !self.training_dialog_open {
            return;
        }
        let mut close = false;
        let mut detached = false;
        let maximize_id = egui::Id::new("training_profiles_maximized");
        let mut maximized = ctx.data(|data| data.get_temp::<bool>(maximize_id).unwrap_or(false));
        let mut window = egui::Window::new("Training profiles").id(egui::Id::new(("training_profiles", maximized)))
            .title_bar(false).drag_area(egui::WindowDrag::Anywhere).resizable(!maximized).movable(!maximized)
            .default_pos(egui::pos2(48.0,48.0))
            .default_size(Vec2::new(780.0, (ctx.content_rect().height()-96.0).clamp(240.0,800.0)))
            .min_size(Vec2::new(360.0,240.0))
            .max_width((ctx.content_rect().width()-40.0).max(280.0))
            .max_height((ctx.content_rect().height()-40.0).max(200.0))
            .frame(Self::dialog_frame().shadow(egui::epaint::Shadow {
                offset:[4,6],blur:14,spread:2,color:Color32::from_black_alpha(160),
            }));
        if Self::is_training_companion() || maximized {
            let bounds = ctx.content_rect().shrink(8.0);
            window=window.max_size(bounds.size()).fixed_rect(bounds).resizable(false).movable(false);
        }
        let response=window.show(ctx, |ui| {
            let grain=ui.painter().add(egui::Shape::Noop);
            ui.style_mut().text_styles.insert(egui::TextStyle::Body, FontId::proportional(16.0));
            ui.style_mut().text_styles.insert(egui::TextStyle::Button, FontId::proportional(16.0));
            ui.style_mut().text_styles.insert(egui::TextStyle::Small, FontId::proportional(15.0));
            ui.spacing_mut().item_spacing.y = 8.0;
            ui.horizontal(|ui| {
                ui.add_sized([(ui.available_width()-122.0).max(80.0),28.0],
                    egui::Label::new(RichText::new("Training profiles").size(21.0).strong()).selectable(false).truncate());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if Self::glossy_board_button(ui,"×",false,true).on_hover_text("Close Training Profiles").clicked() { close=true; }
                    if !Self::is_training_companion() && Self::glossy_board_button(ui, if maximized { "❐" } else { "□" }, false, true)
                        .on_hover_text(if maximized { "Restore Training Profiles window" } else { "Maximize Training Profiles window" }).clicked() {
                        maximized = !maximized;
                        ui.ctx().request_repaint();
                    }
                    #[cfg(target_arch="wasm32")]
                    if !training_companion_mode() && Self::glossy_board_button(ui,"↗",false,true)
                        .on_hover_text("Move Training Profiles to a separate window").clicked() { detached=open_training_companion(); }
                });
            });
            ui.add_space(10.0);
            let content_height=(ui.available_height()-8.0).max(100.0);
            egui::ScrollArea::vertical().id_salt("training_progress")
                .max_height(content_height).min_scrolled_height(content_height).auto_shrink([false, false])
                .show_gold(ui, |ui| {
                self.training_setup_ui(ui);
                let Some(profile) = self.training_profiles.selected().cloned() else { return; };
                ui.add_space(24.0);
                if ui.add_enabled(self.training_error.is_none() && !self.training_live() && !self.profile_management_busy(), egui::Button::new(RichText::new("Start next training game").size(20.0).strong().color(Color32::from_rgb(24, 27, 31)))
                    .min_size(Vec2::new(280.0, 48.0)).fill(Color32::from_rgb(211, 173, 98)).corner_radius(8.0).raised_gold()).clicked() { self.start_training_game(); }
                if self.training_live() { ui.label("Finish the current game to update progress and unlock the next color."); }
                let [wins, draws, losses] = profile.counts(None);
                ui.add_space(20.0);
                Self::training_sunken_section(ui, |ui| {
                ui.label(RichText::new("PROGRESS").strong().color(Color32::from_rgb(211, 173, 98)));
                ui.label(RichText::new(format!("{} completed · {} wins · {} draws · {} losses", profile.results.len(), wins, draws, losses)).strong());
                egui::Grid::new("training_color_stats").striped(true).spacing(Vec2::new(24.0, 12.0)).show(ui, |ui| {
                    for label in ["Color", if profile.engine.is_lc0() { "Level" } else { "Rating" }, "W / D / L", "Last 10 games"] { ui.strong(label); } ui.end_row();
                    for side in [TrainingSide::White, TrainingSide::Black] {
                        let c = profile.counts(Some(side));
                        ui.label(side.label()); ui.label(profile.rating(side).to_string());
                        ui.label(format!("{} / {} / {}", c[0], c[1], c[2])); ui.label(format!("{:+} {}", profile.recent_change(side),if profile.engine.is_lc0() { "levels" } else { "Elo" })); ui.end_row();
                    }
                });
                ui.add_space(24.0);
                ui.label(RichText::new(if profile.engine.is_lc0() { "Level progress" } else { "Rating progress" }).size(19.0).strong());
                Self::training_rating_graph(ui, &profile);
                ui.add_space(12.0);
                if let Some((score, lower, upper)) = profile.score_interval() {
                    ui.label(format!("Score rate {:.0}% · 95% descriptive range {:.0}–{:.0}% · n={}", score*100.0, lower*100.0, upper*100.0, profile.results.len()));
                    ui.label(RichText::new("Draws count as half a point. This broad bounded-score range assumes independent results; adaptive opponents and changing skill limit interpretation. It is not Elo uncertainty or proof of improvement.").small().weak());
                }
                });
                ui.add_space(16.0);
                Self::training_sunken_section(ui, |ui| {
                ui.label(RichText::new("Areas to focus on").size(19.0).strong().color(Color32::from_rgb(211, 173, 98)));
                let stats = profile.analysis_totals();
                let analyzed_games = profile.results.iter().filter(|r| r.analysis.analyzed_moves() > 0).count();
                if Self::training_focus_ui(ui, &stats, analyzed_games, !Self::is_training_companion() && !self.training_live() && !(self.fics_active && self.fics_playing)) { close = true; }
                if Self::is_training_companion() { ui.weak("Open Training Profiles in the main window to launch puzzle practice."); }
                if stats.missed_mates + stats.allowed_mates > 0 { ui.label(format!("Tactics: {} missed mates · {} moves allowing mate. Review those positions before your next session.", stats.missed_mates, stats.allowed_mates)); }
                let flags = profile.results.iter().filter(|r| r.timed_out).count();
                if flags > 0 { ui.label(format!("Clock management: {flags} games lost on time.")); }
                let white_n: usize = profile.counts(Some(TrainingSide::White)).iter().sum();
                let black_n: usize = profile.counts(Some(TrainingSide::Black)).iter().sum();
                if white_n >= 5 && black_n >= 5 && (profile.rating(TrainingSide::White)-profile.rating(TrainingSide::Black)).abs() >= if profile.engine.is_lc0() {2} else {50} {
                    ui.label(format!("Color balance: review your {} games; that rating trails by {} points.", if profile.rating(TrainingSide::White) < profile.rating(TrainingSide::Black) { "White" } else { "Black" }, (profile.rating(TrainingSide::White)-profile.rating(TrainingSide::Black)).abs()));
                }
                ui.add_space(12.0);
                ui.collapsing("How these statistics are calculated", |ui| {
                    ui.label("Opening means the first 10 moves; endgame means at most 26 points of non-pawn material. Average loss uses raw centipawns and excludes mate scores. Mistakes and blunders use Ironwood's move-quality scale. Partial analysis can bias the results.");
                });
                });
                ui.add_space(16.0);
                Self::training_sunken_section(ui, |ui| {
                ui.label(RichText::new("Recent games").size(19.0).strong().color(Color32::from_rgb(211, 173, 98)));
                for r in profile.results.iter().rev().take(20) {
                    ui.label(format!("{} · {} · {} vs {} · {} to {} ({:+})", r.session.completed_at.as_deref().unwrap_or("").get(..10).unwrap_or(""), r.session.side.label(), r.result, r.session.opponent_label(), r.session.rating_before, r.session.rating_after.unwrap_or(r.session.rating_before), r.session.rating_after.unwrap_or(r.session.rating_before)-r.session.rating_before));
                }
                ui.add_space(16.0);
                ui.label("Open Saved Games > Training to replay games, add notes, and analyze mistakes. Profiles and progress are included in Storage > Backup all data.");
                });
            });
            let paper=Self::paper_grain_texture(ui.ctx());
            let surface=ui.min_rect().expand(6.0);
            ui.painter().set(grain,egui::Shape::image(paper.id(),surface,
                egui::Rect::from_min_max(egui::Pos2::ZERO,egui::pos2(surface.width()/256.0,surface.height()/256.0)),Color32::WHITE));
        });
        ctx.data_mut(|data| data.insert_temp(maximize_id, maximized));
        if let Some(window)=response {
            Self::paint_fics_window_border(&ctx.layer_painter(window.response.layer_id),window.response.rect);
        }
        #[cfg(target_arch="wasm32")]
        if close && training_companion_mode() { close_training_companion(); }
        if close || detached {
            self.training_dialog_open = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn maia_training_applies_each_sessions_elo_and_stays_private() {
        let mut app=app();app.training=None;
        let mut profile=app.training_profiles.selected().unwrap().clone();
        profile.engine=training::TrainingEngine::Maia3{identity:"http://localhost:8790/maia".into(),name:"Maia3".into(),options:std::collections::BTreeMap::from([("Temperature".into(),"0.5".into()),("Elo".into(),"2000".into()),("SelfElo".into(),"2000".into()),("OppoElo".into(),"2000".into())])};
        profile.starting_elo=800;
        let session=profile.session("maia".into(),"now".into());let config=ChessApp::training_engine_config(&session);
        assert_eq!(config.opponent,OpponentEngine::Companion);assert!(!config.limit_strength);assert_eq!(config.nodes,1);assert_eq!(config.elo,800);
        for name in ["Elo","SelfElo","OppoElo"] {assert_eq!(config.companion_options[name],"800");}
        assert_eq!(config.companion_options["Temperature"],"0.5");assert_eq!(session.search_command(),"go nodes 1");
        profile.settle(&session,"1-0","later".into(),false);
        let black=profile.session("black".into(),"now".into());profile.settle(&black,"0-1","later".into(),false);
        let next=profile.session("next".into(),"now".into());let next_config=ChessApp::training_engine_config(&next);assert_eq!(next_config.companion_options["SelfElo"],"816");
        app.training_profiles.profiles=vec![profile];app.start_training_game();assert!(app.training.is_none());assert!(app.training_error.as_ref().unwrap().contains("Enable Companion"));
        app.training=Some(session);assert!(app.companion_training_blocked());
        app.training=None;
        app.puzzle_context.data_mut(|d|d.insert_temp(egui::Id::new("training_profile_manager"),ProfileManager{engine:next.engine,starting_elo:800,..Default::default()}));
        let before=app.training_profiles.profiles.len();app.training_profile_name="Must stay private".into();app.create_training_profile();assert_eq!(app.training_profiles.profiles.len(),before);
    }
    #[test]
    fn maia_training_profile_controls_are_hidden_when_companion_is_disabled() {
        let ctx=egui::Context::default();let cc=eframe::CreationContext::_new_kittest(ctx.clone());let mut app=ChessApp::new(&cc);
        app.training_profiles=training::Profiles{selected_id:"private".into(),profiles:vec![Profile{id:"private".into(),name:"Hidden Maia3 profile".into(),engine:training::TrainingEngine::Maia3{identity:"http://localhost:8790/maia".into(),name:"Maia3".into(),options:Default::default()},starting_elo:800,starting_level:1,results:vec![]}]};
        let mut texts=Vec::new();
        for _ in 0..3 {
            let output=run_test_ui(&ctx,egui::RawInput{screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,Vec2::new(1100.0,1100.0))),..Default::default()},|ui|app.training_setup_ui(ui));
            texts.extend(output.shapes.iter().filter_map(|shape|if let egui::Shape::Text(text)=&shape.shape{Some(text.galley.job.text.clone())}else{None}));
        }
        assert!(!texts.iter().any(|text|text.contains("Maia3")),"{texts:?}");
        assert!(app.training_profiles.selected_id.is_empty());assert_eq!(app.training_profiles.profiles.len(),1,"Hidden data must be retained");
    }
    #[test]
    fn training_choices_list_all_providers_without_switching_live_engines() {
        let mut app = app();
        let before = app.analysis_config.companion_identity.clone();
        let catalog = serde_json::json!({"engines":[
            {"identity":"windows/lc0","engine_label":"Lc0 v0.32.1","label":"Lc0 — Windows"},
            {"identity":"windows/maia","engine_label":"Maia3 79M (GPU)","label":"Maia3 — Windows"},
            {"identity":"other/lc0","engine_label":"Lc0","label":"Lc0 — Other"},
            {"identity":"windows/other","engine_label":"Other engine","label":"Other"},
            {"identity":"windows/lc0","engine_label":"Lc0","label":"Duplicate"}
        ]});
        app.engine_uci_preferences.insert("companion:windows/lc0".into(), std::collections::BTreeMap::from([("Backend".into(),"cuda".into())]));
        let choices = app.training_engine_choices(true, &catalog);
        assert_eq!(choices.len(),4);
        assert!(choices[0].0 == training::TrainingEngine::Stockfish);
        assert!(choices[2].0.is_maia3());
        assert!(choices[3].1.contains("Other"));
        let training::TrainingEngine::Lc0 {identity,options,..} = &choices[1].0 else {panic!("Expected lc0")};
        assert_eq!(identity,"windows/lc0");
        assert_eq!(options.get("Backend").map(String::as_str),Some("cuda"));
        assert_eq!(app.analysis_config.companion_identity,before);
        assert_eq!(app.training_engine_choices(false,&catalog).len(),1);
        assert_eq!(app.training_engine_choices(true,&serde_json::Value::Null).len(),1);
    }
    #[test]
    fn maia_training_choice_supports_popouts_and_rejects_incompatible_options() {
        let mut app=app();app.analysis_config.opponent=OpponentEngine::Companion;app.analysis_config.companion_name="Maia3 79M".into();app.analysis_config.companion_identity="http://localhost:8790/maia".into();
        assert!(app.maia3_training_choice(&app.analysis_config).unwrap().is_maia3(),"Fresh pop-outs must recognize the saved engine");
        let key=ChessApp::verification_engine_key(&app.analysis_config);
        app.engine_uci_catalog.insert(key.clone(),vec![]);assert!(app.maia3_training_choice(&app.analysis_config).is_none());
        let options=["Elo","SelfElo","OppoElo"].iter().map(|name|engine_uci_settings::parse_option(&format!("option name {name} type spin default 1500 min 0 max 5000")).unwrap()).collect();
        app.engine_uci_catalog.insert(key,options);assert!(app.maia3_training_choice(&app.analysis_config).unwrap().is_maia3());
        app.analysis_config.companion_name="Lc0".into();assert!(app.maia3_training_choice(&app.analysis_config).is_none());
        app.analysis_config.opponent=OpponentEngine::Stockfish;assert!(app.maia3_training_choice(&app.analysis_config).is_none());
    }
    #[test]
    fn lc0_session_uses_its_profile_options_and_cannot_start_with_preview_disabled() {
        let mut app = app();
        app.training = None;
        let mut profile=app.training_profiles.selected().unwrap().clone();
        profile.engine=training::TrainingEngine::Lc0 {identity:"http://localhost:8790/lc0".into(),name:"Lc0".into(),options:std::collections::BTreeMap::from([("Backend".into(),"cuda".into())])};
        profile.starting_level=4;
        let session=profile.session("lc0".into(),"now".into());
        let config=ChessApp::training_engine_config(&session);
        assert_eq!(config.opponent,OpponentEngine::Companion);
        assert!(!config.limit_strength);
        assert_eq!(config.nodes,64);
        assert_eq!(config.companion_options.get("Backend").map(String::as_str),Some("cuda"));
        assert_eq!(session.search_command(),"go nodes 64");
        app.training_profiles.profiles=vec![profile];
        app.start_training_game();
        assert!(app.training.is_none());
        assert!(app.training_error.as_ref().unwrap().contains("Enable Companion"));
    }
    fn app() -> ChessApp {
        let cc = eframe::CreationContext::_new_kittest(egui::Context::default());
        let mut app = ChessApp::new(&cc);
        app.training_profile_name = "Training Tester".into();
        app.create_training_profile();
        app.start_training_game();
        app
    }
    fn play(app: &mut ChessApp, san: &str) {
        let mv = ChessApp::parse_san_move(&app.board, san).unwrap();
        app.play(mv);
    }
    #[test]
    fn profile_transfer_blocks_starting_a_training_game() {
        let mut app = app();
        app.training = None;
        app.puzzle_context.data_mut(|d| d.insert_temp(egui::Id::new("training_profile_manager"), ProfileManager { busy: true, ..ProfileManager::default() }));
        app.start_training_game();
        assert!(app.training.is_none());
    }
    #[test]
    fn retained_history_after_profile_deletion_does_not_block_training_storage() {
        let mut app = app();
        for san in ["f3", "e5", "g4", "Qh4#"] { play(&mut app, san); }
        app.settle_training();
        assert!(app.training.as_ref().unwrap().rating_after.is_some());
        app.training_profiles.profiles.clear();
        app.training_profiles.selected_id.clear();
        app.settle_training();
        assert!(app.training_error.is_none());
        assert!(app.training_profiles.profiles.is_empty());
    }
    #[test]
    fn training_checkmate_settles_once_and_next_game_uses_black() {
        let mut app = app();
        assert_eq!(app.training.as_ref().unwrap().side, TrainingSide::White);
        assert!(app.engine_config.limit_strength);
        for san in ["f3", "e5", "g4", "Qh4#"] {
            play(&mut app, san);
        }
        assert_eq!(app.game_result(), "0-1");
        app.settle_training();
        app.settle_training();
        let profile = app.training_profiles.selected().unwrap();
        assert_eq!(profile.results.len(), 1);
        assert_eq!(profile.rating(TrainingSide::White), 1304);
        let saved: PersistedGame =
            serde_json::from_str(&serde_json::to_string(&app.persisted_game()).unwrap()).unwrap();
        assert_eq!(saved.training.unwrap().rating_after, Some(1304));
        app.start_training_game();
        assert_eq!(app.training.as_ref().unwrap().side, TrainingSide::Black);
        assert!(app.player_side == PlayerSide::Black);
        assert_eq!(app.engine_config.elo, 1320);
        assert!(app.local_clock.is_none());
        assert_eq!(app.engine_config.move_time_ms, 1000);
    }
    #[test]
    fn unfinished_game_stays_unrated_and_resignation_and_timeout_settle() {
        let mut app = app();
        play(&mut app, "e4");
        app.settle_training();
        assert!(app.training_profiles.selected().unwrap().results.is_empty());
        app.undo();
        assert_eq!(app.review_moves.len(), 1);
        app.review_to(0);
        assert!(app.review_index.is_none());
        app.local_resigned_white = Some(true);
        app.settle_training();
        assert_eq!(app.training_profiles.selected().unwrap().results.len(), 1);
        app.start_training_game();
        // Historical timed training results remain supported.
        app.local_clock = Some(LocalClock {
            white: 600.0, black: 0.0, increment: 5.0,
            flagged_white: Some(false), updated_at: ChessApp::animation_time(),
        });
        app.settle_training();
        assert_eq!(app.training_profiles.selected().unwrap().results.len(), 2);
        assert!(app.training_profiles.selected().unwrap().results[1].timed_out);
    }
    #[test]
    fn repetition_draw_stops_play_and_does_not_change_equal_rating() {
        let mut app = app();
        for san in ["Nf3", "Nf6", "Ng1", "Ng8", "Nf3", "Nf6", "Ng1", "Ng8"] {
            play(&mut app, san);
        }
        assert_eq!(app.game_result(), "1/2-1/2");
        let moves = app.review_moves.len();
        play(&mut app, "e4");
        assert_eq!(app.review_moves.len(), moves);
        app.settle_training();
        assert_eq!(
            app.training_profiles
                .selected()
                .unwrap()
                .rating(TrainingSide::White),
            1320
        );
        assert_eq!(
            app.training_profiles.selected().unwrap().counts(None),
            [0, 1, 0]
        );
    }
    #[test]
    fn postgame_analysis_updates_focus_data_without_changing_rating() {
        let mut app = app();
        for san in ["e4", "e5", "Nf3", "Nc6"] {
            play(&mut app, san);
        }
        app.local_resigned_white = Some(true);
        app.settle_training();
        app.game_analysis = vec![
            Some(PositionAnalysis {
                eval_cp: Some(0),
                ..PositionAnalysis::default()
            });
            5
        ];
        app.game_analysis[1].as_mut().unwrap().eval_cp = Some(-1000);
        app.settle_training();
        let profile = app.training_profiles.selected().unwrap();
        assert_eq!(profile.results.len(), 1);
        assert_eq!(profile.rating(TrainingSide::White), 1304);
        assert_eq!(profile.analysis_totals().total_player_moves, 2);
        assert_eq!(profile.analysis_totals().analyzed_moves(), 2);
        assert_eq!(profile.analysis_totals().phases[0].blunders, 1);
    }
    #[test]
    fn imports_clear_training_identity_and_do_not_rate_imported_results() {
        let mut app = app();
        app.load_pgn("[Result \"0-1\"]\n1. f3 e5 2. g4 Qh4# 0-1");
        assert!(app.training.is_none());
        app.settle_training();
        assert!(app.training_profiles.selected().unwrap().results.is_empty());
    }
}
