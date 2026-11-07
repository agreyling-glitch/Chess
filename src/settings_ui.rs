#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum SettingsPage {
    #[default]
    Appearance,
    BoardTheme,
    ThemeAdjustments,
    Background,
    Moves,
    Engine,
    AnalysisDisplay,
    QuickAccess,
    Companion,
    Tablebases,
    StorageInfo,
    BackupRestore,
    LocalData,
}

impl SettingsPage {
    fn paint_icon(self, ui: &egui::Ui, center: egui::Pos2, color: Color32) {
        let painter = ui.painter();
        let stroke = Stroke::new(1.3, color);
        let point = |(x, y)| center + Vec2::new(x, y);
        let path = |points: &[(f32, f32)]| {
            painter.add(egui::Shape::line(
                points.iter().copied().map(point).collect(),
                stroke,
            ));
        };
        let rect = |x, y, width, height| {
            painter.rect_stroke(
                egui::Rect::from_min_size(point((x, y)), Vec2::new(width, height)),
                2.0,
                stroke,
                egui::StrokeKind::Inside,
            );
        };
        match self {
            Self::Appearance => {
                painter.circle_stroke(center, 3.0, stroke);
                for index in 0..8 {
                    let ray = Vec2::angled(index as f32 * std::f32::consts::TAU / 8.0);
                    painter.line_segment([center + ray * 5.0, center + ray * 7.0], stroke);
                }
            }
            Self::BoardTheme => {
                rect(-7.0, -7.0, 14.0, 14.0);
                path(&[(-7.0, 0.0), (7.0, 0.0)]);
                path(&[(0.0, -7.0), (0.0, 7.0)]);
            }
            Self::ThemeAdjustments => {
                for (y, x) in [(-5.0, -2.0), (0.0, 3.0), (5.0, 0.0)] {
                    path(&[(-7.0, y), (7.0, y)]);
                    painter.circle_filled(point((x, y)), 1.8, color);
                }
            }
            Self::Background => {
                rect(-7.0, -6.0, 14.0, 12.0);
                path(&[
                    (-6.0, 4.0),
                    (-2.0, 0.0),
                    (0.0, 2.0),
                    (3.0, -1.0),
                    (6.0, 3.0),
                ]);
                painter.circle_filled(point((-3.0, -3.0)), 1.0, color);
            }
            Self::Moves => {
                path(&[
                    (-7.0, -2.0),
                    (-4.0, -2.0),
                    (0.0, -6.0),
                    (0.0, 6.0),
                    (-4.0, 2.0),
                    (-7.0, 2.0),
                    (-7.0, -2.0),
                ]);
                path(&[(3.0, -3.0), (4.0, 0.0), (3.0, 3.0)]);
                path(&[(5.0, -5.0), (7.0, 0.0), (5.0, 5.0)]);
            }
            Self::Engine => {
                rect(-5.0, -5.0, 10.0, 10.0);
                rect(-2.0, -2.0, 4.0, 4.0);
                for offset in [-3.0, 0.0, 3.0] {
                    path(&[(offset, -7.0), (offset, -5.0)]);
                    path(&[(offset, 5.0), (offset, 7.0)]);
                    path(&[(-7.0, offset), (-5.0, offset)]);
                    path(&[(5.0, offset), (7.0, offset)]);
                }
            }
            Self::AnalysisDisplay => {
                path(&[(-7.0, -6.0), (-7.0, 6.0), (7.0, 6.0)]);
                path(&[(-4.0, 2.0), (-1.0, -2.0), (2.0, 0.0), (7.0, -5.0)]);
            }
            Self::QuickAccess => {
                path(&[
                    (1.0, -7.0),
                    (-6.0, 1.0),
                    (-1.0, 1.0),
                    (-1.0, 7.0),
                    (6.0, -1.0),
                    (1.0, -1.0),
                    (1.0, -7.0),
                ]);
            }
            Self::Companion => {
                rect(-7.0, -6.0, 14.0, 10.0);
                path(&[(0.0, 4.0), (0.0, 7.0)]);
                path(&[(-4.0, 7.0), (4.0, 7.0)]);
            }
            Self::Tablebases => {
                rect(-6.0, -7.0, 12.0, 14.0);
                for y in [-3.0, 2.0] {
                    path(&[(-5.0, y), (0.0, y + 1.0), (5.0, y)]);
                }
            }
            Self::StorageInfo => {
                painter.circle_stroke(center, 7.0, stroke);
                painter.circle_filled(point((0.0, -3.0)), 0.8, color);
                path(&[(0.0, -0.5), (0.0, 4.0)]);
            }
            Self::LocalData | Self::BackupRestore => {
                rect(-7.0, -5.0, 14.0, 12.0);
                path(&[(-7.0, -5.0), (-7.0, -7.0), (-2.0, -7.0), (0.0, -5.0)]);
            }
        }
    }

    const GROUPS: [(&'static str, &'static [Self]); 3] = [
        (
            "Preferences",
            &[
                Self::Appearance,
                Self::BoardTheme,
                Self::ThemeAdjustments,
                Self::Background,
                Self::Moves,
                Self::Engine,
                Self::AnalysisDisplay,
                Self::QuickAccess,
            ],
        ),
        ("Connections", &[Self::Companion, Self::Tablebases]),
        ("Storage", &[Self::StorageInfo, Self::LocalData, Self::BackupRestore]),
    ];

    fn details(self) -> (&'static str, &'static str) {
        match self {
            Self::Appearance => (
                "Appearance",
                "Board view 2D 3D highlighted move frame coordinates piece shadows best-move arrows",
            ),
            Self::BoardTheme => ("Board theme", "2D pieces 3D boards gallery previews"),
            Self::ThemeAdjustments => (
                "Theme adjustments",
                "Piece style proportions size colors texture outlines shadows lighting contrast radial light",
            ),
            Self::Background => ("Background", "Image upload library gradient opacity"),
            Self::Moves => (
                "Moves & sound",
                "Move notation letters figurines hover text highlight king check capture dissolve animate live moves play move sounds volume board tap octave pitch decay reduced motion",
            ),
            Self::Engine => (
                "Engine & analysis",
                "Stockfish play realtime full game UCI options strength skill Elo threads hash memory network search time depth nodes candidate lines budget verification second pass",
            ),
            Self::AnalysisDisplay => (
                "Analysis display",
                "Prediction show best next move compare saved line balance bar location left right analysis panel",
            ),
            Self::QuickAccess => ("Quick access", "Navigation shortcuts show hide reorder"),
            Self::Companion => (
                "Companion",
                "Connections endpoint address token engines archives setup Windows download",
            ),
            Self::Tablebases => (
                "Tablebases",
                "Endgame Syzygy source Lichess online local Companion coverage",
            ),
            Self::BackupRestore => ("Backup and Restore", "Browser Companion backups restore archive folder"),
            Self::StorageInfo => ("Information", "Storage information browser usage capacity archive database statistics"),
            Self::LocalData => (
                "Local data",
                "Storage clear saved games reset all local data",
            ),
        }
    }

    fn available(self) -> bool {
        match self {
            Self::Companion => cfg!(target_arch = "wasm32") && crate::companion::enabled(),
            Self::StorageInfo | Self::LocalData | Self::BackupRestore => cfg!(target_arch = "wasm32"),
            _ => true,
        }
    }

    fn matches(self, query: &str) -> bool {
        let (title, keywords) = self.details();
        let searchable = format!("{title} {keywords}").to_lowercase();
        self.available()
            && query
                .split_whitespace()
                .all(|word| searchable.contains(word))
    }
}

#[derive(Default)]
struct SettingsPanel {
    open: bool,
    page: SettingsPage,
    query: String,
}

impl ChessApp {
    fn open_settings(&mut self, page: SettingsPage) {
        self.request_mode_switch(navigation::ModeSwitch::Settings(page));
    }

    fn show_settings_page(&mut self, page: SettingsPage) {
        self.settings.open = true;
        self.settings.query.clear();
        self.engine_settings_draft_ready = false;
        if page == SettingsPage::Engine && !self.engine_settings_locked() {
            self.prepare_engine_settings();
        }
        self.settings.page = page;
    }

    fn settings_panel(&mut self, ctx: &egui::Context, status_dock_top: f32) {
        let open_companion = cfg!(target_arch = "wasm32")
            && ctx.data_mut(|data| data.remove_temp::<bool>(egui::Id::new("open_companion_settings"))) == Some(true)
            && SettingsPage::Companion.available();
        if open_companion {
            self.open_settings(SettingsPage::Companion);
        }
        if !self.settings.open {
            #[cfg(target_arch = "wasm32")]
            {
                layout_companion_settings(false, 0.0, 0.0, 0.0, 0.0);
                layout_storage_info(false, 0.0, 0.0, 0.0, 0.0);
                layout_history_backup(false, 0.0, 0.0, 0.0, 0.0);
            }
            return;
        }
        let screen = ctx.content_rect();
        let left = screen.left() + if screen.width() < 600.0 { 0.0 } else { navigation::RAIL_WIDTH };
        let size = Vec2::new((screen.right() - left).min(1000.0), status_dock_top - screen.top());
        let mut close = false;
        let mut embedded_pane = None;
        let id = egui::Id::new("ironwood_settings");
        let frame = Self::dialog_frame()
            .stroke(Stroke::new(1.0, Color32::from_rgb(133, 109, 65)))
            .corner_radius(CornerRadius::ZERO);
        let content_size = size - frame.total_margin().sum();
        let area = egui::Area::new(id)
            .order(egui::Order::Foreground)
            .fixed_pos(egui::pos2(left, screen.top()))
            .default_size(size)
            .movable(false);
        let window = egui::Modal::new(id)
            .area(area)
            .frame(frame)
            .backdrop_color(Color32::from_black_alpha(24))
            .show(ctx, |ui| {
                ui.set_min_size(content_size);
                ui.set_max_size(content_size);
                close = Self::dialog_header(ui, "Settings");
                ui.separator();
                let narrow = ui.available_width() < 560.0;
                if narrow {
                    self.settings_navigation(ui, true);
                    ui.separator();
                    embedded_pane = self.settings_content_pane(ui);
                } else {
                    ui.horizontal_top(|ui| {
                        let height = ui.available_height();
                        ui.allocate_ui_with_layout(
                            Vec2::new(188.0, height),
                            Layout::top_down(Align::Min),
                            |ui| {
                                ui.set_width(188.0);
                                let surface = ui.available_rect_before_wrap();
                                let grain = Self::paper_grain_texture(ui.ctx());
                                let uv = egui::Rect::from_min_size(
                                    egui::Pos2::ZERO, surface.size() / 256.0,
                                );
                                for _ in 0..2 {
                                    ui.painter().image(grain.id(), surface, uv, Color32::WHITE);
                                }
                                self.settings_navigation(ui, false);
                            },
                        );
                        ui.separator();
                        ui.vertical(|ui| {
                            ui.set_width(ui.available_width());
                            embedded_pane = self.settings_content_pane(ui);
                        });
                    });
                }
            });
        if close || window.should_close() {
            self.settings.open = false;
            self.engine_settings_draft_ready = false;
        }
        #[cfg(target_arch = "wasm32")]
        {
            let zoom = ctx.zoom_factor();
            for (page, layout) in [
                (SettingsPage::Companion, layout_companion_settings as fn(bool, f32, f32, f32, f32)),
                (SettingsPage::StorageInfo, layout_storage_info),
                (SettingsPage::BackupRestore, layout_history_backup),
            ] {
                if let Some((_, rect)) = embedded_pane.filter(|(active, _)| self.settings.open && *active == page) {
                    layout(true, rect.left() * zoom, rect.top() * zoom, rect.width() * zoom, rect.height() * zoom);
                } else {
                    layout(false, 0.0, 0.0, 0.0, 0.0);
                }
            }
        }
    }

    fn settings_content_pane(&mut self, ui: &mut egui::Ui) -> Option<(SettingsPage, egui::Rect)> {
        Self::textured_sunken_section(ui, true, |ui| {
            ui.set_min_height(ui.available_height());
            let page = self.settings.page;
            if matches!(page, SettingsPage::Companion | SettingsPage::StorageInfo | SettingsPage::BackupRestore)
                && page.matches(&self.settings.query.to_lowercase())
            {
                ui.heading(match page {
                    SettingsPage::Companion => "Companion Connections",
                    SettingsPage::StorageInfo => "Storage information",
                    _ => "Backup and Restore",
                });
                ui.add_space(8.0);
                Some((page, ui.available_rect_before_wrap().intersect(ui.clip_rect())))
            } else {
                self.settings_page(ui);
                None
            }
        }).inner
    }

    fn settings_navigation(&mut self, ui: &mut egui::Ui, narrow: bool) {
        let search = ui.add(
            egui::TextEdit::singleline(&mut self.settings.query)
                .hint_text("Search settings")
                .margin(Margin { left: 4, right: 26, top: 2, bottom: 2 })
                .desired_width(ui.available_width()),
        );
        if !self.settings.query.is_empty() {
            let clear_rect = egui::Rect::from_center_size(
                egui::pos2(search.rect.right() - 13.0, search.rect.center().y),
                Vec2::new(24.0, search.rect.height()),
            );
            let clear = ui.put(clear_rect, egui::Button::new("×").frame(false));
            clear.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Clear search")
            });
            if clear.on_hover_text("Clear search").clicked() {
                self.settings.query.clear();
                search.request_focus();
            }
        }
        let query = self.settings.query.to_lowercase();
        let first_match = SettingsPage::GROUPS
            .iter()
            .flat_map(|(_, pages)| pages.iter())
            .copied()
            .find(|page| page.matches(&query));
        if let Some(page) = first_match
            && !self.settings.page.matches(&query)
        {
            self.select_settings_page(page);
        }
        if narrow {
            if first_match.is_some() {
                egui::ComboBox::from_id_salt("settings_categories")
                    .selected_text(self.settings.page.details().0)
                    .show_ui(ui, |ui| self.settings_categories(ui, &query));
            }
        } else {
            egui::ScrollArea::vertical()
                .id_salt("settings_categories_scroll")
                .show(ui, |ui| self.settings_categories(ui, &query));
        }
    }

    fn settings_categories(&mut self, ui: &mut egui::Ui, query: &str) {
        for (group, pages) in SettingsPage::GROUPS {
            let mut matches = pages
                .iter()
                .copied()
                .filter(|page| page.matches(query))
                .peekable();
            if matches.peek().is_none() {
                continue;
            }
            ui.add_space(10.0);
            ui.label(
                RichText::new(group)
                    .size(16.0)
                    .strong()
                    .color(Color32::from_rgb(223, 190, 119)),
            );
            for page in matches {
                let selected = self.settings.page == page;
                let icon_id = egui::Id::new("category_icon");
                let icon_slot = egui::Atom::custom(icon_id, Vec2::splat(16.0));
                let row = egui::Button::new("")
                    .left_text((page.details().0, icon_slot))
                    .min_size(Vec2::new(ui.available_width(), 30.0))
                    .frame(false)
                    .selected(selected)
                    .atom_ui(ui);
                let color = if selected {
                    ui.visuals().selection.stroke.color
                } else {
                    ui.visuals().text_color()
                };
                if let Some(icon_rect) = row.rect(icon_id) {
                    page.paint_icon(ui, icon_rect.center(), color);
                }
                if row.response.clicked() {
                    self.select_settings_page(page);
                }
            }
        }
    }

    fn select_settings_page(&mut self, page: SettingsPage) {
        if self.settings.page != page {
            self.engine_settings_draft_ready = false;
            if page == SettingsPage::Engine && !self.engine_settings_locked() {
                self.prepare_engine_settings();
            }
            self.settings.page = page;
        }
    }

    fn settings_page(&mut self, ui: &mut egui::Ui) {
        let page = self.settings.page;
        if !page.matches(&self.settings.query.to_lowercase()) {
            ui.label("No matching settings");
            return;
        }
        ui.heading(page.details().0);
        ui.add_space(8.0);
        let ctx = ui.ctx().clone();
        match page {
            SettingsPage::Background => self.background_settings(&ctx, Some(ui)),
            SettingsPage::BoardTheme => self.board_theme_settings(&ctx, Some(ui)),
            SettingsPage::Engine => {
                if self.engine_settings_locked() {
                    self.engine_settings_draft_ready = false;
                    ui.label("Finish active training and stop full-game analysis before changing engine settings.");
                } else {
                    if !self.engine_settings_draft_ready {
                        self.prepare_engine_settings();
                    }
                    self.engine_settings(ui);
                }
            }
            _ => {
                egui::ScrollArea::vertical().id_salt(page.details().0).show_gold(ui, |ui| match page {
                    SettingsPage::Appearance => {
                        ui.label("Choose what appears on the board.");
                        ui.horizontal(|ui| {
                            ui.label("Board view");
                            let is_3d = self.board_3d_active;
                            let flat = ui.selectable_label(!is_3d, "2D").clicked();
                            let spatial = ui.selectable_label(is_3d, "3D").clicked();
                            if (flat && is_3d) || (spatial && !is_3d) {
                                self.toggle_board_dimension(ui.ctx());
                            }
                        });
                        self.board_display_settings(ui);
                    }
                    SettingsPage::ThemeAdjustments => self.theme_adjustments_settings(ui),
                    SettingsPage::Moves => {
                        self.move_feedback_settings(ui);
                        if ui.checkbox(&mut self.show_move_hover_text, "Show move hover text").changed() {
                            self.save_preferences();
                        }
                    }
                    SettingsPage::AnalysisDisplay => {
                        let prediction = ui.checkbox(&mut self.show_prediction_best_move, "Show best next move").changed();
                        let comparison = ui.checkbox(&mut self.show_saved_comparison, "Compare saved line").changed();
                        let before = self.balance_bar_location;
                        ui.horizontal_wrapped(|ui| {
                            ui.label("Balance Bar Location");
                            for (location, label) in [
                                (BalanceBarLocation::Left, "Left"),
                                (BalanceBarLocation::Right, "Right"),
                                (BalanceBarLocation::AnalysisPanel, "Analysis Panel"),
                            ] {
                                ui.selectable_value(&mut self.balance_bar_location, location, label);
                            }
                        });
                        if prediction || comparison || before != self.balance_bar_location {
                            self.save_preferences();
                        }
                    }
                    SettingsPage::QuickAccess => self.quick_access_settings(ui),
                    SettingsPage::Tablebases => crate::tablebase::source_settings(ui),
                    SettingsPage::LocalData => {
                        #[cfg(target_arch = "wasm32")]
                        self.local_data_settings(ui);
                    }
                    SettingsPage::Background | SettingsPage::BoardTheme | SettingsPage::Engine | SettingsPage::Companion | SettingsPage::StorageInfo | SettingsPage::BackupRestore => unreachable!(),
                });
            }
        }
    }
}
