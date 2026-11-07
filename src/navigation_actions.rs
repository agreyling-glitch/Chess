use super::*;
use super::navigation::ModeSwitch;

impl ChessApp {
    pub(super) fn navigation_open_analysis(&mut self, ctx: &egui::Context) {
        if self.training_live() || (self.fics_active && self.fics_playing && !self.fics_game_finished) { return; }
            ctx.data_mut(|d| {
                d.insert_temp(egui::Id::new("analysis_workspace"), true);
                d.insert_temp(egui::Id::new("navigation_action_selected"), "Analysis".to_owned());
                d.insert_temp(egui::Id::new("puzzles_open"), false);
                d.insert_temp(egui::Id::new("study_window_open"), false);
                d.insert_temp(egui::Id::new("game_sidebar_tab"), 2usize);
                let mut panels = d.get_temp::<[bool; 8]>(egui::Id::new("enabled_sidebar_panels")).unwrap_or([true; 8]);
                #[cfg(target_arch = "wasm32")]
                if let Some(saved) = web_sys::window().and_then(|w| w.local_storage().ok().flatten())
                    .and_then(|s| s.get_item("ironwood.sidebar-panels.v1").ok().flatten())
                    .and_then(|s| serde_json::from_str::<[bool; 8]>(&s).ok()) { panels = saved; }
                panels[2] = true;
                d.insert_temp(egui::Id::new("enabled_sidebar_panels"), panels);
                #[cfg(target_arch = "wasm32")]
                if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
                    let _ = storage.set_item("ironwood.sidebar-panels.v1", &serde_json::to_string(&panels).unwrap());
                }
            });
    }

    pub(super) fn navigation_play(&mut self, ui: &mut egui::Ui) {
        Self::set_menu_item_font(ui);
        let setup_button = |ui: &mut egui::Ui, label: &str, selected| {
            ui.add(egui::Button::new("")
                .left_text(label)
                .frame(selected)
                .selected(selected)
                .wrap()
                .min_size(Vec2::new(ui.available_width(), 36.0)))
                .clicked() && !selected
        };
        let setup_open = self.new_game_dialog_open && self.navigation.game_setup_attached;
        if setup_button(ui, "New game against Engine…", setup_open && !self.new_game_both_sides) {
            self.request_mode_switch(ModeSwitch::Setup { self_play: false, attached: true });
        }
        egui::CollapsingHeader::new("Online").default_open(true).show(ui, |ui| {
            if Self::navigation_action(ui, true, "Lichess lobby…").clicked() {
                self.request_mode_switch(ModeSwitch::OnlineLobby { lichess: true });
            }
            if Self::navigation_action(ui, true, "FICS lobby…").clicked() {
                self.request_mode_switch(ModeSwitch::OnlineLobby { lichess: false });
            }
        });
        if setup_button(ui, "Self-play…", setup_open && self.new_game_both_sides) {
            self.request_mode_switch(ModeSwitch::Setup { self_play: true, attached: true });
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn exit_to_home() {
        if let Some(window) = web_sys::window() {
            let _ = window.location().set_href("/");
        }
    }
    pub(super) fn navigation_training(&mut self, ui: &mut egui::Ui) {
        Self::set_menu_item_font(ui);
        if Self::navigation_action(ui, true, "Train against AI…").clicked() {
            self.request_mode_switch(ModeSwitch::Training);
        }
        if Self::navigation_action(ui, self.navigation_puzzles_enabled() || self.playing_game_before_mode_switch(), "Lichess Puzzles…")
            .on_disabled_hover_text("Finish your training or online game before starting puzzles.")
            .clicked()
        {
            self.request_mode_switch(ModeSwitch::Puzzles);
        }
        if Self::navigation_action(ui, self.navigation_puzzles_enabled() || self.playing_game_before_mode_switch(), "Study…")
            .on_disabled_hover_text("Finish your training or online game before opening Study.").clicked() {
            self.request_mode_switch(ModeSwitch::Study);
        }
    }
    pub(super) fn board_display_settings(&mut self, ui: &mut egui::Ui) {
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
    }
    pub(super) fn move_feedback_settings(&mut self, ui: &mut egui::Ui) {
        for (figurines, label) in [(false, "Letters (Qf3)"), (true, "Figurines (♕f3)")] {
            if ui
                .selectable_value(&mut self.figurine_notation, figurines, label)
                .changed()
            {
                self.save_game();
            }
        }
        ui.separator();
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
                self.play_move_sound("move");
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
        if ui.add(egui::Slider::new(&mut self.move_sound_volume, 0..=100)
            .text("Volume").suffix("%")).changed()
        {
            self.save_preferences();
        }
        ui.horizontal_wrapped(|ui| {
            ui.label("Move sound");
            let previous = self.move_sound_style;
            egui::ComboBox::from_id_salt("move_sound_style")
                .selected_text(self.move_sound_style.label())
                .show_ui(ui, |ui| {
                    for style in MoveSoundStyle::ALL {
                        ui.selectable_value(&mut self.move_sound_style, style, style.label());
                    }
                });
            if self.move_sound_style != previous {
                self.save_preferences();
            }
            if ui.button("Preview").on_hover_text("Play a sample move sound").clicked() {
                self.play_move_sound("move");
            }
        });
        if self.move_sound_style == MoveSoundStyle::BoardTap {
            let octave_changed = ui.add(
                egui::Slider::new(&mut self.board_tap_octave, -2.0..=1.0)
                    .step_by(0.1)
                    .text("Octave"),
            ).changed();
            let decay_changed = ui.add(
                egui::Slider::new(&mut self.board_tap_decay, 25..=300)
                    .suffix("%")
                    .text("Decay"),
            ).changed();
            if octave_changed || decay_changed {
                self.save_preferences();
            }
        }
        }
    }
    #[cfg(target_arch = "wasm32")]
    pub(super) fn local_data_settings(&mut self, ui: &mut egui::Ui) {
        if Self::navigation_action(ui, true, "Clear saved games…").clicked() {
            #[cfg(target_arch = "wasm32")]
            clear_saved_games();
        }
        if Self::navigation_action(ui, true, "Reset all local data…").clicked() {
            #[cfg(target_arch = "wasm32")]
            reset_local_data();
        }
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
        #[cfg(target_arch = "wasm32")]
        if Self::navigation_action(ui, true, "Report a bug…").clicked() {
            open_bug_report(
                wasm_bindgen::JsValue::UNDEFINED,
                &self.clipboard_pgn(self.review_moves.len()),
            );
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
