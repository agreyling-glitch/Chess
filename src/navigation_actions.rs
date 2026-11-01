use super::*;

impl ChessApp {
    pub(super) fn navigation_play(&mut self, ui: &mut egui::Ui) {
        Self::set_menu_item_font(ui);
        if Self::navigation_action(ui, true, "New game against AI…").clicked() {
            self.navigation_new_game(false);
        }
        ui.collapsing("Online", |ui| {
            if Self::navigation_action(ui, true, "Lichess lobby…").clicked() {
                self.open_online_lobby(true);
            }
            if Self::navigation_action(ui, true, "FICS lobby…").clicked() {
                self.open_online_lobby(false);
            }
        });
        if Self::navigation_action(ui, true, "Self-play…").clicked() {
            self.navigation_new_game(true);
        }
        if Self::navigation_action(
            ui,
            !self.fics_active || self.fics_game_finished,
            "Save current game",
        )
        .clicked()
        {
            self.save_current_game();
            self.engine_status = "Game saved on this device".into();
        }
        ui.separator();
        if Self::navigation_action(ui, true, "Exit to home").clicked() {
            #[cfg(target_arch = "wasm32")]
            if let Some(window) = web_sys::window() {
                let _ = window.location().set_href("/");
            }
        }
    }
    pub(super) fn navigation_training(&mut self, ui: &mut egui::Ui) {
        Self::set_menu_item_font(ui);
        if Self::navigation_action(ui, true, "Train against AI…").clicked() {
            self.settle_training();
            self.training_dialog_open = true;
        }
        if Self::navigation_action(ui, self.navigation_puzzles_enabled(), "Lichess Puzzles…")
            .on_disabled_hover_text("Finish your training or online game before starting puzzles.")
            .clicked()
        {
            ui.ctx()
                .data_mut(|d| d.insert_temp(egui::Id::new("puzzles_open"), true));
        }
    }
    pub(super) fn navigation_library(&mut self, ui: &mut egui::Ui) {
        Self::set_menu_item_font(ui);
        if Self::navigation_action(ui, self.navigation_library_enabled(), "Saved games…").clicked()
        {
            self.navigation_open_library();
        }
        if Self::navigation_action(ui, true, "Import…").clicked() {
            if self.fics_active {
                self.stop_fics();
            }
            #[cfg(target_arch = "wasm32")]
            ensure_imported_index();
            self.import_input.clear();
            self.pgn_dialog_open = true;
            self.pgn_error = None;
        }
        if Self::navigation_action(ui, true, "Backup and Restore…").clicked() {
            #[cfg(target_arch = "wasm32")]
            open_history_backup();
        }
        ui.separator();
        if Self::navigation_action(ui, true, "Storage information…").clicked() {
            #[cfg(target_arch = "wasm32")]
            open_storage_info();
        }
        ui.separator();
    }
    pub(super) fn navigation_settings(&mut self, ui: &mut egui::Ui) {
        Self::set_menu_item_font(ui);
        if Self::navigation_action(ui, !self.engine_settings_locked(), "Engine settings…")
                        .on_disabled_hover_text("Finish active training and stop full-game analysis before changing engine settings")
                        .clicked() {
                        self.open_engine_settings();

                    }
        #[cfg(target_arch = "wasm32")]
        if crate::companion::enabled()
            && Self::navigation_action(ui, true, "Companion connection…").clicked()
        {
            manage_companion_connection();
        }
        ui.separator();
        if Self::navigation_action(ui, true, "Background…").clicked() {
            self.background_dialog_open = true;
        }
        ui.separator();
        ui.collapsing("Move notation", |ui| {
            Self::set_menu_item_font(ui);
            for (figurines, label) in [(false, "Letters (Qf3)"), (true, "Figurines (♕f3)")] {
                if ui
                    .selectable_value(&mut self.figurine_notation, figurines, label)
                    .changed()
                {
                    self.save_game();
                }
            }
        });
        ui.collapsing("Board", |ui| {
            Self::set_menu_item_font(ui);
            if Self::navigation_action(ui, true, "Theme…").clicked() {
                self.open_board_theme_dialog();
            }
            if Self::navigation_action(ui, true, "Theme adjustments…").clicked() {
                self.theme_adjustments_open = true;
            }
            ui.separator();
            if ui
                .checkbox(&mut self.show_highlighted_move, "Show highlighted move")
                .changed()
            {
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
        });
        ui.collapsing("Move feedback", |ui| {
            Self::set_menu_item_font(ui);
            if ui
                .checkbox(&mut self.highlight_checked_king, "Highlight king in check")
                .changed()
            {
                self.save_preferences();
            }
            if ui
                .checkbox(&mut self.animate_captures, "Capture dissolve")
                .changed()
            {
                self.capture_animation = None;
                self.save_preferences();
            }
            ui.label(
                RichText::new("Animations respect your system's reduced-motion setting.")
                    .small()
                    .weak(),
            );
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
        ui.separator();
        ui.collapsing("Local data", |ui| {
            if Self::navigation_action(ui, true, "Clear saved games…").clicked() {
                #[cfg(target_arch = "wasm32")]
                clear_saved_games();
            }
            if Self::navigation_action(ui, true, "Reset all local data…").clicked() {
                #[cfg(target_arch = "wasm32")]
                reset_local_data();
            }
        });
    }
    pub(super) fn navigation_tools(&mut self, ui: &mut egui::Ui) {
        Self::set_menu_item_font(ui);

        #[cfg(target_arch = "wasm32")]
        if Self::navigation_action(ui, true, "Enter a scoresheet…").clicked() {
            open_scoresheet();
        }
        if Self::navigation_action(ui, true, "Elo calculator…").clicked() {
            self.elo_calculator_open = true;
        }
        if Self::navigation_action(ui, true, "Position Editor…").clicked() {
            #[cfg(target_arch = "wasm32")]
            if let Err(error) = open_position_editor(&self.board.to_string()) {
                self.engine_status = format!("Could not open position editor: {error:?}");
            }
        }
    }
    pub(super) fn navigation_help(&mut self, ui: &mut egui::Ui) {
        Self::set_menu_item_font(ui);
        if Self::navigation_action(ui, true, "Contents…").clicked() {
            ui.ctx().open_url(egui::OpenUrl::new_tab(HELP_URL));
        }
        if Self::navigation_action(ui, true, "Companion setup…").clicked() {
            ui.ctx().open_url(egui::OpenUrl::new_tab(
                "https://ironwoodchess.com/downloads/companion/",
            ));
        }
        #[cfg(target_arch = "wasm32")]
        if Self::navigation_action(ui, true, "Companion connections…").clicked() {
            manage_companion_connection();
        }
        if Self::navigation_action(ui, true, "Join our Discord…").clicked() {
            ui.ctx().open_url(egui::OpenUrl::new_tab(DISCORD_URL));
        }
        #[cfg(target_arch = "wasm32")]
        if has_deferred_update() && Self::navigation_action(ui, true, "Update and Reload").clicked()
        {
            update_and_reload();
        }
        if Self::navigation_action(ui, true, "About Ironwood Chess…").clicked() {
            self.about_dialog_open = true;
            #[cfg(target_arch = "wasm32")]
            refresh_version_diagnostics();
        }
    }
}
