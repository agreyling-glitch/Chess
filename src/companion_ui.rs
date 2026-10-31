use super::*;

impl ChessApp {
    #[cfg(target_arch = "wasm32")]
    pub(super) fn import_location_selector(ui: &mut egui::Ui) -> (bool,bool) {
        let catalog:serde_json::Value=serde_json::from_str(&import_location_catalog()).unwrap_or_default();
        let mut selected=catalog["selected"].as_str().unwrap_or("browser").to_owned();let previous=selected.clone();
        ui.add_enabled_ui(catalog["busy"]!=true,|ui|{
            ui.spacing_mut().interact_size.y=28.0;
            ui.spacing_mut().button_padding=Vec2::new(6.0,4.0);
            let menu_style=ui.style().as_ref().clone();
            let background=ui.painter().add(egui::Shape::Noop);
            let mut visuals=ui.visuals().widgets.inactive;
            visuals.bg_fill=Color32::TRANSPARENT;visuals.weak_bg_fill=Color32::TRANSPARENT;visuals.bg_stroke=Stroke::NONE;visuals.expansion=0.0;
            ui.visuals_mut().widgets.inactive=visuals;ui.visuals_mut().widgets.hovered=visuals;
            ui.visuals_mut().widgets.active=visuals;ui.visuals_mut().widgets.open=visuals;
            let response=egui::ComboBox::from_id_salt("import_destination_location")
                .selected_text(catalog["label"].as_str().unwrap_or("Browser")).width(170.0).truncate()
                .show_ui(ui,|ui|{ui.set_style(menu_style);if let Some(locations)=catalog["locations"].as_array(){for location in locations {
                    let value=location["value"].as_str().unwrap_or("browser");let label=location["label"].as_str().unwrap_or("Browser");
                    ui.add_enabled_ui(location["enabled"]!=false,|ui|{ui.selectable_value(&mut selected,value.to_owned(),label);});
                }}});
            ui.painter().set(background,egui::Shape::Vec(vec![
                egui::Shape::rect_filled(response.response.rect,4,Color32::from_rgb(8,11,15)),
                egui::Shape::rect_stroke(response.response.rect,4,Stroke::new(1.0,Color32::from_rgb(83,93,97)),egui::StrokeKind::Inside),
            ]));
            response.response.on_hover_text("Import destination");
        });
        if selected!=previous {set_import_location(&selected);}
        let current:serde_json::Value=serde_json::from_str(&import_location_catalog()).unwrap_or_default();
        let archive=current["selected"].as_str().unwrap_or("browser")!="browser";
        (archive,current["ready"]==true)
    }
    #[cfg(target_arch = "wasm32")]
    fn companion_settings_icon(ui: &mut egui::Ui, enabled: bool, label: &str, icon: u8) -> egui::Response {
        let response = ui.add_enabled(enabled, egui::Button::new("").min_size(Vec2::splat(36.0)).raised_gray());
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, response.enabled(), label));
        let center = response.rect.center();
        let color = if response.enabled() { Color32::from_rgb(226, 231, 235) } else { Color32::from_gray(105) };
        let stroke = Stroke::new(2.0, color);
        if icon == 2 {
            let points: Vec<_> = (0..=24).map(|i| center + Vec2::angled(0.45 + i as f32 / 24.0 * 5.0) * 8.0).collect();
            let end = *points.last().unwrap();
            ui.painter().add(egui::Shape::line(points, stroke));
            ui.painter().add(egui::Shape::convex_polygon(vec![end + Vec2::new(-3.0, -2.0), end + Vec2::new(3.0, -2.0), end + Vec2::new(1.0, 3.0)], color, Stroke::NONE));
        } else if icon == 1 {
            for (x, y) in [(-7.0, -4.0), (0.0, 4.0), (7.0, -1.0)] {
                ui.painter().line_segment([center + Vec2::new(x, -9.0), center + Vec2::new(x, 9.0)], stroke);
                ui.painter().rect_filled(egui::Rect::from_center_size(center + Vec2::new(x, y), Vec2::new(6.0, 5.0)), 1.0, color);
            }
        } else {
            ui.painter().line_segment([center - Vec2::new(8.0, 0.0), center + Vec2::new(8.0, 0.0)], stroke);
            ui.painter().line_segment([center - Vec2::new(0.0, 8.0), center + Vec2::new(0.0, 8.0)], stroke);
        }
        response.on_hover_text(label)
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn analysis_engine_selector(&mut self, ui: &mut egui::Ui) {
        if !crate::companion::enabled() { return; }
        let catalog: serde_json::Value = serde_json::from_str(&companion_engine_catalog()).unwrap_or_default();
        let busy = catalog["busy"] == true;
        let allowed = !self.engine_settings_locked() && !(self.fics_active && !self.fics_game_finished) && !busy;
        let mut identity = catalog["pending_identity"].as_str().map(str::to_owned).unwrap_or_else(|| {
            if self.analysis_config.opponent == OpponentEngine::Companion {
                self.analysis_config.companion_identity.clone()
            } else { String::new() }
        });
        let previous = identity.clone();
        ui.add_enabled_ui(allowed, |ui| {
            let menu_style=ui.style().as_ref().clone();
            let background=ui.painter().add(egui::Shape::Noop);
            let mut visuals=ui.visuals().widgets.inactive;
            visuals.weak_bg_fill=Color32::TRANSPARENT;visuals.bg_stroke=Stroke::NONE;visuals.expansion=0.0;
            ui.visuals_mut().widgets.inactive=visuals;ui.visuals_mut().widgets.hovered=visuals;
            ui.visuals_mut().widgets.active=visuals;ui.visuals_mut().widgets.open=visuals;
            let selector=egui::ComboBox::from_id_salt("analysis_engine_provider")
                .selected_text(catalog["engines"].as_array().and_then(|engines| engines.iter().find(|engine| engine["identity"] == identity)).and_then(|engine| engine["label"].as_str()).unwrap_or(self.analysis_config.label())).width(260.0).truncate()
                .show_ui(ui, |ui| {
                    *ui.style_mut()=menu_style;
                    ui.set_min_width(320.0_f32.min(ui.ctx().content_rect().width()-24.0));
                    ui.selectable_value(&mut identity, String::new(), "Built-in Stockfish");
                    if let Some(engines) = catalog["engines"].as_array() {
                        for engine in engines {
                            if let (Some(id), Some(label)) = (engine["identity"].as_str(), engine["label"].as_str()) {
                                ui.selectable_value(&mut identity, id.to_owned(), label);
                            }
                        }
                    }
                });
            ui.painter().set(background,Self::glass_surface(selector.response.rect));
            selector.response.on_hover_text("Engine for the next full-game analysis");
        });
        if identity != previous {
            self.save_game();
            select_companion_engine(&identity);
            self.poll_companion_provider();
        }
        if Self::analysis_refresh_button(ui,!busy,"Refresh installed engines").on_disabled_hover_text("Checking engines…").clicked() { refresh_companion_engines(); }
        if catalog["has_engine_provider"] != true {
            let help = ui.add(egui::Button::new("?").raised_gold())
                .on_hover_text("Use other engines with Ironwood Companion");
            egui::Popup::from_toggle_button_response(&help).show(|ui| {
                ui.set_max_width(300.0_f32.min(ui.ctx().content_rect().width() - 24.0));
                ui.add(egui::Label::new("To use other engines in Ironwood, use Ironwood Companion and connect your engines. Open Companion connections, connect Companion, and enable ‘Use as an engine provider’. Then add your engines in Engine settings and refresh this list.").wrap());
                if ui.button("Companion connections…").clicked() {
                    manage_companion_connection();
                    ui.close();
                }
            });
        } else if let Some(status) = catalog["status"].as_str().filter(|s| !s.is_empty()) {
            ui.add(egui::Label::new(status).truncate()).on_hover_text(status);
        }
        if busy { ui.ctx().request_repaint_after(std::time::Duration::from_millis(50)); }
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn new_game_engine_selector(&mut self, ui: &mut egui::Ui, width: f32) {
        let catalog: serde_json::Value = serde_json::from_str(&companion_engine_catalog()).unwrap_or_default();
        let busy = catalog["busy"] == true;
        let mut identity = catalog["pending_identity"].as_str().map(str::to_owned).unwrap_or_else(|| {
            if self.new_game_opponent == OpponentEngine::Companion { self.engine_config.companion_identity.clone() } else { String::new() }
        });
        let previous = identity.clone();
        ui.vertical(|ui| {
            ui.set_width(width);
            ui.horizontal(|ui| {
            ui.add_enabled_ui(!busy && !self.engine_settings_locked(), |ui| {
                egui::ComboBox::from_id_salt("new_game_engine")
                    .width((width - 46.0).max(90.0)).truncate()
                    .selected_text(catalog["engines"].as_array().and_then(|engines| engines.iter().find(|engine| engine["identity"] == identity)).and_then(|engine| engine["label"].as_str()).unwrap_or(self.engine_config.label()))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut identity, String::new(), "Built-in Stockfish");
                        if let Some(engines) = catalog["engines"].as_array() {
                            for engine in engines {
                                if let (Some(id),Some(label)) = (engine["identity"].as_str(),engine["label"].as_str()) {
                                    ui.selectable_value(&mut identity,id.to_owned(),label);
                                }
                            }
                        }
                    });
            });
            if Self::companion_settings_icon(ui, !busy, "Refresh installed engines", 2).clicked() { refresh_companion_engines(); }
            });
            if let Some(status) = catalog["status"].as_str().filter(|status| !status.is_empty()) { ui.add(egui::Label::new(status).wrap()); }
        });
        if identity != previous {
            select_companion_engine(&identity);
            self.poll_companion_provider();
            ui.ctx().request_repaint();
        }
        if busy { ui.ctx().request_repaint_after(std::time::Duration::from_millis(50)); }
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn companion_engine_selector(&mut self, ui: &mut egui::Ui) {
        let catalog: serde_json::Value = serde_json::from_str(&companion_engine_catalog()).unwrap_or_default();
        let busy = catalog["busy"].as_bool().unwrap_or(false);
        let allowed = !self.engine_settings_locked() && !(self.fics_active && !self.fics_game_finished) && !busy;
        let mut identity = catalog["pending_identity"].as_str().map(str::to_owned).unwrap_or_else(|| {
            if self.analysis_config.opponent == OpponentEngine::Companion {
                self.analysis_config.companion_identity.clone()
            } else { String::new() }
        });
        let previous = identity.clone();
        ui.scope(|ui| {
            ui.style_mut().text_styles.insert(egui::TextStyle::Body, FontId::proportional(16.0));
            ui.style_mut().text_styles.insert(egui::TextStyle::Button, FontId::proportional(16.0));
            ui.spacing_mut().interact_size.y = 36.0;
            ui.spacing_mut().button_padding = Vec2::new(12.0, 8.0);
            ui.spacing_mut().item_spacing = Vec2::new(10.0, 8.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Engine").size(18.0).strong().color(Color32::from_rgb(211, 173, 98)));
            ui.add_enabled_ui(allowed, |ui| {
                egui::ComboBox::from_id_salt("settings_engine_provider")
                    .selected_text(catalog["engines"].as_array().and_then(|engines| engines.iter().find(|engine| engine["identity"] == identity)).and_then(|engine| engine["label"].as_str()).unwrap_or(self.analysis_config.label())).width(240.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut identity, String::new(), "Built-in Stockfish");
                        if let Some(engines) = catalog["engines"].as_array() {
                            for engine in engines {
                                if let (Some(id), Some(label)) = (engine["identity"].as_str(), engine["label"].as_str()) {
                                    ui.selectable_value(&mut identity, id.to_owned(), label);
                                }
                            }
                        }
                    });
            });
            if Self::companion_settings_icon(ui, allowed, "Add engine", 0).clicked() { add_companion_engine(); }
            if Self::companion_settings_icon(ui, allowed && !identity.is_empty(), "Manage engines", 1).clicked() { manage_companion_engines(&identity); }
            if Self::companion_settings_icon(ui, !busy, "Refresh installed engines", 2).clicked() { refresh_companion_engines(); }
        });
        });
        if identity != previous {
            select_companion_engine(&identity);
            self.poll_companion_provider();
            ui.ctx().request_repaint();
        }
        if let Some(status) = catalog["status"].as_str().filter(|s| !s.is_empty()) { ui.label(status); }
        if busy { ui.ctx().request_repaint_after(std::time::Duration::from_millis(50)); }
        // Until preparation succeeds, the drafts still belong to the previous engine.
        if catalog["pending_identity"].is_string() { ui.disable(); }
        ui.add_space(8.0);
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn ensure_engine_provider(&mut self, config: EngineConfig) {
        if self.engine.as_ref().is_some_and(|e| e.identity != Self::verification_engine_key(&config)
            || (e.remote && e.overrides != config.companion_options)) {
            self.engine = None;
            self.engine_ready = false;
            self.engine_searching = false;
            self.engine_start_failed = false;
            self.ignore_next_bestmove = false;
            self.pending_analysis_search = None;
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn poll_companion_provider(&mut self) {
        let allowed = !self.training_live() && !(self.fics_active && !self.fics_game_finished)
            && !self.game_analysis_running;
        let enabled = crate::companion::enabled();
        if self.companion_training_blocked() && self.training_live() {
            self.engine = None; self.engine_ready = false; self.engine_searching = false;
            self.resume_engine_after_ready = false;
            self.engine_status = "Companion training paused · enable Companion in Version diagnostics to resume".into();
            return;
        }
        if enabled && self.training_live() && self.training.as_ref().is_some_and(|s| s.engine.is_companion())
            && self.engine_status.starts_with("Companion training paused") {
            self.engine_status = "Companion training resumed".into();
            self.queue_live_engine_resume();
        }
        let disabling = !enabled && (self.analysis_config.opponent == OpponentEngine::Companion || self.engine_config.opponent == OpponentEngine::Companion || self.new_game_opponent == OpponentEngine::Companion || self.strength_draft.opponent == OpponentEngine::Companion || self.analysis_draft.opponent == OpponentEngine::Companion);
        companion_engine_allowed(allowed && enabled);
        if !enabled && !disabling { let _ = companion_engine_change(); return; }
        let raw = if disabling { r#"{"provider":"builtin"}"#.to_owned() } else { companion_engine_change() };
        if raw.is_empty() { return; }
        if !allowed && !disabling { self.engine_status = "Finish active training, online play, or game analysis before changing providers".into(); return; }
        let Ok(change) = serde_json::from_str::<serde_json::Value>(&raw) else { return; };
        let opponent = if change["provider"] == "companion" { OpponentEngine::Companion } else { OpponentEngine::Stockfish };
        let name = change["name"].as_str().unwrap_or("").to_owned();
        let identity = change["identity"].as_str().unwrap_or("").to_owned();
        let key = if opponent == OpponentEngine::Stockfish { "builtin:stockfish".into() } else {format!("companion:{identity}")};
        if let Ok(catalog) = serde_json::from_value::<Vec<engine_uci_settings::UciOption>>(change["uci_options"].clone()) {
            self.engine_uci_catalog.insert(key.clone(), catalog);
        }
        let options: std::collections::BTreeMap<String,String> = self.engine_uci_preferences.get(&key).cloned().unwrap_or_else(||serde_json::from_value(change["options"].clone()).unwrap_or_default());
        let changed = self.analysis_config.opponent != opponent || self.analysis_config.companion_identity != identity
            || self.analysis_config.companion_name != name || self.analysis_config.companion_options != options;
        self.engine = None; self.engine_ready = false; self.engine_start_failed = false;
        self.engine_searching = false; self.analysis_running = false;
        self.pending_analysis_search = None; self.ignore_next_bestmove = false;
        self.realtime_analysis_due_at = None; self.resume_engine_after_ready = false;
        self.game_analysis_paused = !changed && self.game_analysis_paused;
        self.resume_game_analysis_after_ready = false;
        for config in [&mut self.engine_config, &mut self.analysis_config, &mut self.strength_draft, &mut self.analysis_draft] {
            config.opponent = opponent; config.companion_name = name.clone();
            config.companion_identity = identity.clone(); config.companion_options = options.clone();
            if opponent == OpponentEngine::Stockfish { config.uci_options = options.clone(); }
        }
        self.new_game_opponent = opponent;
        if self.new_game_dialog_open {self.prepare_new_game_strength();}
        self.verification_draft = self.verify_after_analysis();
        if changed {
            self.full_game_analysis_config = self.engine_full_game_preferences.get(&Self::verification_engine_key(&self.analysis_config)).cloned().unwrap_or_else(|| FullGameAnalysisConfig::for_engine(opponent));
            self.full_game_analysis_draft = self.full_game_analysis_config.clone();
            self.clear_analysis_result();
            // Reviewing a saved run is independent from choosing the engine for
            // the next run. Retain that snapshot until analysis is started.
            if self.analysis_run.is_none() && !self.archive_analysis_active() {
            self.game_analysis = vec![None; self.review_positions.len()];
            self.verification_targets.clear(); self.verification_results.clear();
            self.verification_nodes = 0; self.verification_time_ms = 0;
            }
        }
        if self.review_index.is_none() && self.engine_enabled {
            if self.player_side == PlayerSide::White { self.review_black_player = self.engine_config.label().into(); }
            else { self.review_white_player = self.engine_config.label().into(); }
        }
        if self.strength_dialog_open { self.prepare_uci_settings(); }
        self.engine_status = format!("{} selected · start analysis or continue play", self.engine_config.label());
        self.save_game(); self.queue_live_engine_resume();
    }
}
