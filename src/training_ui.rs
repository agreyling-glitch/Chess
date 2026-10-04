use super::*;

impl ChessApp {
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
        profiles.profiles.push(Profile {
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

    pub(super) fn start_training_game(&mut self) {
        self.settle_training();
        if self.training_error.is_some() {
            return;
        }
        let Some(profile) = self.training_profiles.selected() else {
            return;
        };
        let session = profile.session(Self::training_id(), Self::training_now());
        if self.fics_active {
            self.stop_fics();
        }
        self.new_game_training = true;
        self.compact_panel = CompactPanel::Moves;
        self.engine_enabled = true;
        self.engine_config = EngineConfig {
            limit_strength: true,
            elo: session.opponent_elo as u32,
            threads: Self::engine_thread_count(),
            move_time_ms: 1000,
            multipv: 1,
            ..EngineConfig::default()
        };
        self.realtime_analysis_due_at = None;
        let side = if session.side == TrainingSide::White {
            PlayerSide::White
        } else {
            PlayerSide::Black
        };
        self.reset_for_side(side);
        self.review_positions = vec![self.board];
        self.local_clock = Some(LocalClock {
            white: 600.0,
            black: 600.0,
            increment: 5.0,
            flagged_white: None,
            updated_at: Self::animation_time(),
        });
        let opponent = format!("Stockfish {} Elo", session.opponent_elo);
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
        egui::Frame::new()
            .fill(Color32::from_rgb(32, 37, 43))
            .corner_radius(8.0)
            .inner_margin(14.0)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(
                    RichText::new("TRAINING PROFILE")
                        .size(if roomy { 16.0 } else { 12.0 })
                        .strong()
                        .color(gold),
                );
                ui.add_space(7.0);
                let old_selected = self.training_profiles.selected_id.clone();
                if !self.training_profiles.profiles.is_empty() {
                    egui::ComboBox::from_id_salt("training_profile")
                        .selected_text(
                            self.training_profiles
                                .selected()
                                .map(|p| p.name.as_str())
                                .unwrap_or("Choose a profile"),
                        )
                        .width(ui.available_width())
                        .show_ui(ui, |ui| {
                            for p in &self.training_profiles.profiles {
                                ui.selectable_value(
                                    &mut self.training_profiles.selected_id,
                                    p.id.clone(),
                                    &p.name,
                                );
                            }
                        });
                    ui.add_space(7.0);
                }
                if old_selected != self.training_profiles.selected_id {
                    self.training_profile_rename = None;
                    if let Err(error) = self.write_training_profiles(&self.training_profiles) {
                        self.training_error = Some(error);
                    }
                }
                if let Some(profile) = self.training_profiles.selected().cloned() {
                    if self.training_profile_rename.is_none() && ui.button("Rename profile").clicked() {
                        self.training_profile_rename = Some(profile.name.clone());
                    }
                    if let Some(name) = &mut self.training_profile_rename {
                        let mut save = false;
                        let mut cancel = false;
                        ui.horizontal_wrapped(|ui| {
                            ui.add(egui::TextEdit::singleline(name).desired_width((ui.available_width() - 160.0).max(100.0)).char_limit(60)
                                .text_color(Color32::from_rgb(238, 238, 238)));
                            save = ui.add_enabled(!name.trim().is_empty() && self.training_error.is_none(), egui::Button::new("Save name")).clicked();
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
                ui.horizontal(|ui| {
                    let width = (ui.available_width() - 128.0).max(100.0);
                    ui.add_sized(
                        [width, 34.0],
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
                                && self.training_error.is_none(),
                            egui::Button::new(RichText::new("Create profile").size(if roomy { 16.0 } else { 14.0 }))
                                .min_size(Vec2::new(120.0, 34.0)),
                        )
                        .clicked()
                    {
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
                egui::Frame::new()
                    .fill(Color32::from_rgb(32, 37, 43))
                    .corner_radius(8.0)
                    .inner_margin(12.0)
                    .show(column, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(
                            RichText::new(label)
                                .size(if roomy { 16.0 } else { 12.0 })
                                .strong()
                                .color(if label == "OVERALL" { gold } else { muted }),
                        );
                        ui.label(RichText::new(rating.to_string()).size(30.0).strong());
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
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("NEXT: {}", side.label().to_uppercase()))
                    .size(if roomy { 16.0 } else { 14.0 })
                    .strong()
                    .color(gold),
            );
            ui.label(RichText::new(format!("vs Stockfish · {} Elo", target)).size(if roomy { 16.0 } else { 14.0 }));
        });
        ui.label(
            RichText::new("Standard chess  ·  10 minutes + 5 seconds per move")
                .size(if roomy { 16.0 } else { 13.0 })
                .color(muted),
        );
        ui.add_space(if roomy { 24.0 } else { 12.0 });
        ui.label(
            RichText::new("Play. Alternate. Improve.")
                .size(16.0)
                .strong(),
        );
        ui.label(RichText::new("Finish a game to update that color’s rating. Your next game switches sides, and Stockfish adjusts to your rating.").size(if roomy { 16.0 } else { 13.0 }).color(muted));
        ui.add_space(if roomy { 10.0 } else { 5.0 });
        ui.label(RichText::new("Review your game and spot patterns afterward. Hints and takebacks are paused during training.").size(if roomy { 16.0 } else { 13.0 }).color(muted));
        ui.add_space(if roomy { 12.0 } else { 6.0 });
        ui.label(
            RichText::new("Ironwood training Elo · starts at 1320 · not an official rating")
                .size(if roomy { 16.0 } else { 11.0 })
                .color(muted),
        );
        #[cfg(not(target_arch = "wasm32"))]
        ui.label("Native preview: profiles remain in memory for this session. Use the web app for saved profiles.");
    }

    pub(super) fn training_game_summary_ui(&mut self, ui: &mut egui::Ui) {
        let Some(session) = &self.training else {
            return;
        };
        ui.label(
            RichText::new(format!(
                "Training · {} · {}",
                session.profile_name,
                session.side.label()
            ))
            .strong(),
        );
        if let Some(after) = session.rating_after {
            ui.label(format!(
                "{} to {} ({:+}) · opponent {}",
                session.rating_before,
                after,
                after - session.rating_before,
                session.opponent_elo
            ));
        } else {
            ui.label(format!(
                "Your rating {} · Stockfish {}",
                session.rating_before, session.opponent_elo
            ));
        }
        if let Some(error) = &self.training_error {
            ui.colored_label(Color32::from_rgb(240, 150, 130), error);
        }
        ui.horizontal_wrapped(|ui| {
            if ui.button("Profile & progress").clicked() {
                self.training_dialog_open = true;
            }
            if !self.training_live() && ui.button("Next training game").clicked() {
                if let Some(session) = &self.training {
                    self.training_profiles.selected_id = session.profile_id.clone();
                }
                self.start_training_game();
            }
        });
        ui.separator();
    }

    fn training_rating_graph(ui: &mut egui::Ui, profile: &Profile) {
        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 160.0), Sense::hover());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 6.0, Color32::from_rgb(16, 18, 22));
        let mut white = training::START_RATING;
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
            - 20;
        let max = points
            .iter()
            .flat_map(|p| [p.0, p.1, p.2])
            .max()
            .unwrap_or(1320)
            + 20;
        let plot = rect.shrink2(Vec2::new(50.0, 16.0));
        let y = |rating: i32| {
            plot.bottom() - (rating - min) as f32 / (max - min) as f32 * plot.height()
        };
        for rating in [min, training::START_RATING, max] {
            painter.line_segment(
                [
                    egui::pos2(plot.left(), y(rating)),
                    egui::pos2(plot.right(), y(rating)),
                ],
                Stroke::new(1.0, Color32::from_gray(55)),
            );
            painter.text(
                egui::pos2(rect.left() + 4.0, y(rating)),
                Align2::LEFT_CENTER,
                rating.to_string(),
                FontId::proportional(15.0),
                Color32::LIGHT_GRAY,
            );
        }
        for (line, color) in [
            Color32::LIGHT_GRAY,
            Color32::from_rgb(102, 162, 226),
            Color32::from_rgb(211, 173, 98),
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
                painter.add(egui::Shape::line(data, Stroke::new(2.0, color)));
            }
        }
        ui.horizontal_wrapped(|ui| {
            ui.colored_label(Color32::LIGHT_GRAY, "White");
            ui.colored_label(Color32::from_rgb(102, 162, 226), "Black");
            ui.colored_label(Color32::from_rgb(211, 173, 98), "Overall (lower rating)");
            ui.label("Completed games");
        });
    }

    fn training_focus_ui(ui: &mut egui::Ui, stats: &AnalysisStats, analyzed_games: usize) {
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
            if let Some((i, phase)) = stats.phases.iter().enumerate().filter(|(_, p)| p.cp_moves >= 20 && p.mistakes + p.blunders > 0).max_by(|(_, a), (_, b)| a.mean_loss().total_cmp(&b.mean_loss())) {
                ui.label(RichText::new(["Opening play", "Middlegame decisions", "Endgame technique"][i]).size(22.0).strong());
                ui.label(["Review development, king safety, and early tactical errors.", "Review your largest errors and compare the engine's alternative lines.", "Practice converting advantages and defending simplified positions."][i]);
                ui.add_space(6.0);
                ui.label(RichText::new(format!("Highest average loss: {:.0} cp across {} numerical evaluations. A practice suggestion based on the analyzed moves.", phase.mean_loss(), phase.cp_moves)).color(muted));
            } else {
                ui.label(RichText::new("Build your analysis sample").size(22.0).strong());
                ui.label("Analyze completed games to find a recurring weakness. A phase needs at least 20 numerical evaluations and a mistake or blunder before a focus is suggested.");
            }
        });
        ui.add_space(16.0);
    }

    pub(super) fn training_dialog(&mut self, ctx: &egui::Context) {
        if !self.training_dialog_open {
            return;
        }
        let mut close = false;
        egui::Modal::new(egui::Id::new("training_profiles")).frame(Self::dialog_frame()).show(ctx, |ui| {
            ui.style_mut().text_styles.insert(egui::TextStyle::Body, FontId::proportional(16.0));
            ui.style_mut().text_styles.insert(egui::TextStyle::Button, FontId::proportional(16.0));
            ui.style_mut().text_styles.insert(egui::TextStyle::Small, FontId::proportional(15.0));
            ui.spacing_mut().item_spacing.y = 8.0;
            let available_width = (ctx.screen_rect().width() - 64.0).max(260.0);
            ui.set_width(if self.training_dialog_maximized { available_width } else { available_width.min(740.0) });
            let window_height = if self.training_dialog_maximized {
                ctx.screen_rect().height() - 40.0
            } else {
                (ctx.screen_rect().height() * 0.85).min(ctx.screen_rect().height() - 64.0)
            }.max(200.0);
            ui.set_min_height(window_height - 36.0);
            close = Self::dialog_header_with_maximize(ui, "Training profiles", Some(&mut self.training_dialog_maximized));
            let content_height = (window_height - 84.0).max(120.0);
            egui::ScrollArea::vertical().id_salt("training_progress")
                .max_height(content_height).min_scrolled_height(content_height).auto_shrink([false, false])
                .show_gold(ui, |ui| {
                self.training_setup_ui(ui);
                let Some(profile) = self.training_profiles.selected().cloned() else { return; };
                ui.add_space(24.0);
                if ui.add_enabled(self.training_error.is_none() && !self.training_live(), egui::Button::new(RichText::new("Start next training game").size(20.0).strong().color(Color32::from_rgb(24, 27, 31)))
                    .min_size(Vec2::new(280.0, 48.0)).fill(Color32::from_rgb(211, 173, 98)).corner_radius(8.0)).clicked() { self.start_training_game(); }
                if self.training_live() { ui.label("Finish the current game to update progress and unlock the next color."); }
                let [wins, draws, losses] = profile.counts(None);
                ui.add_space(20.0); ui.separator(); ui.add_space(16.0);
                ui.label(RichText::new(format!("{} completed · {} wins · {} draws · {} losses", profile.results.len(), wins, draws, losses)).strong());
                egui::Grid::new("training_color_stats").striped(true).spacing(Vec2::new(24.0, 12.0)).show(ui, |ui| {
                    for label in ["Color", "Rating", "W / D / L", "Last 10 games"] { ui.strong(label); } ui.end_row();
                    for side in [TrainingSide::White, TrainingSide::Black] {
                        let c = profile.counts(Some(side));
                        ui.label(side.label()); ui.label(profile.rating(side).to_string());
                        ui.label(format!("{} / {} / {}", c[0], c[1], c[2])); ui.label(format!("{:+} Elo", profile.recent_change(side))); ui.end_row();
                    }
                });
                ui.add_space(24.0);
                ui.label(RichText::new("Rating progress").size(19.0).strong());
                Self::training_rating_graph(ui, &profile);
                ui.add_space(12.0);
                if let Some((score, lower, upper)) = profile.score_interval() {
                    ui.label(format!("Score rate {:.0}% · 95% descriptive range {:.0}–{:.0}% · n={}", score*100.0, lower*100.0, upper*100.0, profile.results.len()));
                    ui.label(RichText::new("Draws count as half a point. This broad bounded-score range assumes independent results; adaptive opponents and changing skill limit interpretation. It is not Elo uncertainty or proof of improvement.").small().weak());
                }
                ui.add_space(20.0); ui.separator(); ui.add_space(16.0);
                ui.label(RichText::new("Areas to focus on").size(19.0).strong());
                let stats = profile.analysis_totals();
                let analyzed_games = profile.results.iter().filter(|r| r.analysis.analyzed_moves() > 0).count();
                Self::training_focus_ui(ui, &stats, analyzed_games);
                if stats.missed_mates + stats.allowed_mates > 0 { ui.label(format!("Tactics: {} missed mates · {} moves allowing mate. Review those positions before your next session.", stats.missed_mates, stats.allowed_mates)); }
                let flags = profile.results.iter().filter(|r| r.timed_out).count();
                if flags > 0 { ui.label(format!("Clock management: {flags} games lost on time.")); }
                let white_n: usize = profile.counts(Some(TrainingSide::White)).iter().sum();
                let black_n: usize = profile.counts(Some(TrainingSide::Black)).iter().sum();
                if white_n >= 5 && black_n >= 5 && (profile.rating(TrainingSide::White)-profile.rating(TrainingSide::Black)).abs() >= 50 {
                    ui.label(format!("Color balance: review your {} games; that rating trails by {} points.", if profile.rating(TrainingSide::White) < profile.rating(TrainingSide::Black) { "White" } else { "Black" }, (profile.rating(TrainingSide::White)-profile.rating(TrainingSide::Black)).abs()));
                }
                ui.add_space(12.0);
                ui.collapsing("How these statistics are calculated", |ui| {
                    ui.label("Opening means the first 10 moves; endgame means at most 26 points of non-pawn material. Average loss uses raw centipawns and excludes mate scores. Mistakes and blunders use Ironwood's move-quality scale. Partial analysis can bias the results.");
                });
                ui.add_space(20.0); ui.separator(); ui.add_space(16.0); ui.label(RichText::new("Recent games").size(19.0).strong());
                for r in profile.results.iter().rev().take(20) {
                    ui.label(format!("{} · {} · {} vs {} · {} to {} ({:+})", r.session.completed_at.as_deref().unwrap_or("").get(..10).unwrap_or(""), r.session.side.label(), r.result, r.session.opponent_elo, r.session.rating_before, r.session.rating_after.unwrap_or(r.session.rating_before), r.session.rating_after.unwrap_or(r.session.rating_before)-r.session.rating_before));
                }
                ui.add_space(16.0);
                ui.label("Open Saved Games > Training to replay games, add notes, and analyze mistakes. Profiles and progress are included in Storage > Backup all data.");
            });
        });
        if close {
            self.training_dialog_open = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        assert_eq!(app.local_clock.as_ref().unwrap().white, 600.0);
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
        app.local_clock.as_mut().unwrap().flagged_white = Some(false);
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
