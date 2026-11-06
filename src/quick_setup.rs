use super::*;

impl ChessApp {
    fn quick_strength(&mut self, ui: &mut egui::Ui, min: u32, max: u32, toggle: bool) {
        let gold = Color32::from_rgb(215, 185, 121);
        self.new_game_elo = self.new_game_elo.clamp(min, max);
        ui.horizontal(|ui| {
            ui.visuals_mut().selection.bg_fill = gold;
            if toggle {
                ui.checkbox(&mut self.new_game_limit_strength, "Limit strength");
            } else {
                self.new_game_limit_strength = true;
                ui.label("Target strength");
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("{} Elo target", self.new_game_elo))
                        .small()
                        .color(gold),
                );
            });
        });
        let track_width = (ui.available_width() - 100.0).max(80.0);
        ui.add_enabled_ui(self.new_game_limit_strength, |ui| {
            ui.horizontal(|ui| {
                ui.scope(|ui| {
                    ui.spacing_mut().slider_width = track_width;
                    ui.spacing_mut().interact_size.y = 22.0;
                    ui.visuals_mut().selection.bg_fill = gold;
                    let widgets = &mut ui.visuals_mut().widgets;
                    for visual in [
                        &mut widgets.inactive,
                        &mut widgets.hovered,
                        &mut widgets.active,
                    ] {
                        visual.bg_fill = Color32::from_rgb(28, 36, 41);
                        visual.fg_stroke = Stroke::new(3.0, gold);
                    }
                    ui.add(
                        egui::Slider::new(&mut self.new_game_elo, min..=max)
                            .show_value(false)
                            .trailing_fill(true)
                            .handle_shape(egui::style::HandleShape::Circle),
                    );
                });
                ui.add_sized(
                    [92.0, 36.0],
                    egui::DragValue::new(&mut self.new_game_elo)
                        .range(min..=max)
                        .suffix(" Elo"),
                );
            });
        });
        ui.allocate_ui(Vec2::new(track_width, 14.0), |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(min.to_string()).small().weak());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(max.to_string()).small().weak());
                });
            });
        });
    }
    fn quick_choice(ui: &mut egui::Ui, label: &str, selected: bool, size: Vec2) -> egui::Response {
        let gold = Color32::from_rgb(215, 185, 121);
        ui.add_sized(
            size,
            egui::Button::new(RichText::new(label).color(if selected {
                gold
            } else {
                Color32::from_rgb(237, 236, 228)
            }))
            .selected(selected)
            .corner_radius(CornerRadius::same(6))
            .fill(if selected {
                Color32::from_rgb(57, 53, 41)
            } else {
                Color32::from_rgb(28, 36, 41)
            })
            .stroke(Stroke::new(
                1.0,
                if selected {
                    gold
                } else {
                    Color32::from_rgb(53, 64, 71)
                },
            )),
        )
    }
    fn quick_position_valid(&self) -> bool {
        (self.new_game_position != 2 || Board::from_str(self.new_game_fen.trim()).is_ok())
            && (!self.quick_position_is_chess960() || self.quick_engine_supports_chess960())
    }

    pub(super) fn quick_position_is_chess960(&self) -> bool {
        self.new_game_position == 1 || (self.new_game_position == 2
            && Board::from_str(self.new_game_fen.trim()).is_ok_and(|board| board.is_chess960()))
    }

    fn quick_engine_supports_chess960(&self) -> bool {
        self.new_game_both_sides || self.new_game_opponent == OpponentEngine::Stockfish
            || self.engine_supports_chess960(self.new_game_engine_draft.as_ref().unwrap_or(&self.engine_config))
    }

    fn quick_time_label(&self) -> String {
        if self.new_game_time == 5 {
            "Untimed".into()
        } else {
            format!("{} + {}", self.new_game_minutes, self.new_game_increment)
        }
    }

    fn quick_summary(&self) -> String {
        let side = if self.new_game_both_sides {
            "Both sides"
        } else {
            match self.new_game_color {
                NewGameColor::White => "White",
                NewGameColor::Black => "Black",
                NewGameColor::Random => "Random side",
            }
        };
        let strength = if self.new_game_both_sides {
            String::new()
        } else if self.new_game_limit_strength
            && (self.new_game_opponent == OpponentEngine::Stockfish
                || self
                    .companion_elo_control(
                        self.new_game_engine_draft
                            .as_ref()
                            .unwrap_or(&self.engine_config),
                    )
                    .is_some())
        {
            format!(" · {} Elo target", self.new_game_elo)
        } else {
            " · Engine settings".into()
        };
        format!(
            "{side}{strength} · {} · {}",
            self.quick_time_label(),
            ["Standard", "Chess960", "Custom FEN"][self.new_game_position.min(2)]
        )
    }

    pub(super) fn new_game_dialog(&mut self, ctx: &egui::Context) {
        if !self.new_game_dialog_open {
            return;
        }
        if self.new_game_online {
            self.open_online_lobby(self.new_game_lichess);
            return;
        }
        self.new_game_training = false;
        let screen = ctx.content_rect();
        let mobile = screen.width() < 600.0;
        let left = screen.left() + if mobile { 0.0 } else { 56.0 };
        let width = (screen.right() - left).min(560.0);
        let height = screen.height();
        let mut close = false;
        let mut start = false;
        let area = egui::Area::new(egui::Id::new("new_game_window"))
            .order(egui::Order::Foreground)
            .fixed_pos(egui::pos2(left, screen.top()))
            .default_size(Vec2::new(width, height))
            .movable(false);
        let response = egui::Modal::new(egui::Id::new("new_game_window"))
            .area(area)
            .frame(Frame::NONE)
            .backdrop_color(Color32::from_black_alpha(24))
            .show(ctx, |ui| {
                Frame::new()
                    .fill(Color32::from_rgb(21, 27, 31))
                    .stroke(Stroke::new(1.0, Color32::from_rgb(66, 57, 35)))
                    .corner_radius(CornerRadius::same(10))
                    .inner_margin(Margin::same(22))
                    .show(ui, |ui| {
                        ui.set_width((width - 46.0).max(100.0));
                        ui.spacing_mut().item_spacing = Vec2::new(8.0, 10.0);
                        ui.spacing_mut().interact_size.y = 40.0;
                        ui.spacing_mut().button_padding = Vec2::new(12.0, 8.0);
                        for visual in [&mut ui.visuals_mut().widgets.inactive] {
                            visual.bg_fill = Color32::from_rgb(28, 36, 41);
                            visual.weak_bg_fill = visual.bg_fill;
                            visual.bg_stroke = Stroke::new(1.0, Color32::from_rgb(53, 64, 71));
                            visual.corner_radius = CornerRadius::same(6);
                        }
                        ui.visuals_mut().widgets.hovered.bg_fill = Color32::from_rgb(39, 47, 49);
                        ui.visuals_mut().widgets.hovered.weak_bg_fill =
                            Color32::from_rgb(39, 47, 49);
                        ui.visuals_mut().widgets.hovered.bg_stroke =
                            Stroke::new(1.0, Color32::from_rgb(215, 185, 121));
                        ui.visuals_mut().widgets.hovered.corner_radius = CornerRadius::same(6);
                        ui.visuals_mut().widgets.active.corner_radius = CornerRadius::same(6);
                        ui.style_mut()
                            .text_styles
                            .insert(egui::TextStyle::Body, FontId::proportional(14.0));
                        ui.style_mut()
                            .text_styles
                            .insert(egui::TextStyle::Button, FontId::proportional(14.0));
                        ui.style_mut()
                            .text_styles
                            .insert(egui::TextStyle::Small, FontId::proportional(12.0));
                        ui.visuals_mut().selection.bg_fill = Color32::from_rgb(66, 57, 35);
                        ui.visuals_mut().widgets.noninteractive.fg_stroke.color =
                            Color32::from_rgb(229, 231, 226);
                        ui.visuals_mut().selection.stroke =
                            Stroke::new(1.0, Color32::from_rgb(223, 190, 119));
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("IRONWOOD CHESS / PLAY")
                                    .small()
                                    .color(Color32::from_rgb(215, 185, 121)),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                close |= ui
                                    .add(egui::Button::new("×").frame(false))
                                    .on_hover_text("Close new game")
                                    .clicked();
                            });
                        });
                        ui.label(
                            RichText::new(if self.new_game_both_sides {
                                "Self-play"
                            } else {
                                "New game against AI"
                            })
                            .size(22.0),
                        );
                        ui.label(
                            RichText::new("Set your challenge. Take your seat.")
                                .color(Color32::from_rgb(174, 183, 181)),
                        );
                        ui.add_space(6.0);
                        ui.separator();
                        egui::ScrollArea::vertical()
                            .id_salt("quick_setup_fields")
                            // Keep the app's floating privacy control clear of Start on phones.
                            .max_height((height - 270.0).max(60.0))
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                self.quick_setup_fields(ui);
                            });
                        ui.separator();
                        ui.add(
                            egui::Label::new(RichText::new(self.quick_summary()).small().weak())
                                .wrap(),
                        );
                        let valid = self.quick_position_valid();
                        #[cfg(target_arch = "wasm32")]
                        let valid = valid
                            && (self.new_game_both_sides
                                || serde_json::from_str::<serde_json::Value>(
                                    &companion_engine_catalog(),
                                )
                                .map(|catalog| !catalog["pending_identity"].is_string())
                                .unwrap_or(true));
                        ui.horizontal(|ui| {
                            close |= ui.add(egui::Button::new("Cancel").frame(false)).clicked();
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                start = ui
                                    .add_enabled(
                                        valid,
                                        egui::Button::new(
                                            RichText::new("▶  Start game")
                                                .color(Color32::from_rgb(24, 27, 31)),
                                        )
                                        .fill(Color32::from_rgb(215, 185, 121))
                                        .corner_radius(CornerRadius::same(6))
                                        .min_size(Vec2::new(130.0, 40.0)),
                                    )
                                    .clicked();
                            });
                        });
                    });
            });
        // The backdrop consumes outside clicks rather than passing them to the board.
        close |= response.should_close();
        if close {
            self.new_game_dialog_open = false;
            self.new_game_engine_draft = None;
            if self.new_game_engine_selection.is_some() {
                self.new_game_engine_selection = Some(false);
            }
            self.new_game_opponent = self.engine_config.opponent;
        } else if start {
            self.start_quick_game();
        }
    }

    fn quick_setup_fields(&mut self, ui: &mut egui::Ui) {
        let gold = Color32::from_rgb(215, 185, 121);
        ui.spacing_mut().slider_width = (ui.available_width() - 100.0).max(100.0);
        if !self.new_game_both_sides {
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(20.0), egui::Sense::hover());
                let painter = ui.painter();
                let chip = rect.shrink(4.0);
                painter.rect_stroke(chip, 2.0, Stroke::new(1.4, gold), egui::StrokeKind::Inside);
                painter.rect_stroke(
                    chip.shrink(3.0),
                    0.0,
                    Stroke::new(1.0, gold),
                    egui::StrokeKind::Inside,
                );
                for offset in [-3.0, 3.0] {
                    let c = rect.center();
                    for sign in [-1.0, 1.0] {
                        painter.line_segment(
                            [
                                c + Vec2::new(offset, sign * 6.0),
                                c + Vec2::new(offset, sign * 10.0),
                            ],
                            Stroke::new(1.4, gold),
                        );
                        painter.line_segment(
                            [
                                c + Vec2::new(sign * 6.0, offset),
                                c + Vec2::new(sign * 10.0, offset),
                            ],
                            Stroke::new(1.4, gold),
                        );
                    }
                }
                ui.label(RichText::new("Opponent").strong().color(gold));
            });
            #[cfg(target_arch = "wasm32")]
            self.new_game_engine_selector(ui, ui.available_width());
            #[cfg(not(target_arch = "wasm32"))]
            ui.label(if self.new_game_opponent == self.engine_config.opponent {
                self.engine_config.label()
            } else {
                self.new_game_opponent.label()
            });
            if self.new_game_opponent == OpponentEngine::Companion {
                if let Some(control) = self.companion_elo_control(
                    self.new_game_engine_draft
                        .as_ref()
                        .unwrap_or(&self.engine_config),
                ) {
                    self.quick_strength(ui, control.min, control.max, control.has_toggle());
                } else {
                    ui.add(
                        egui::Label::new(
                            RichText::new(
                                "This engine has no Elo target. It uses its configured strength.",
                            )
                            .small()
                            .weak(),
                        )
                        .wrap(),
                    );
                }
            } else {
                #[cfg(not(target_arch = "wasm32"))]
                ui.label(
                    RichText::new("Built-in · Runs on this device")
                        .small()
                        .weak(),
                );
                self.quick_strength(ui, 1320, 3190, true);
                if !self.new_game_limit_strength {
                    ui.label(
                        RichText::new("Uses your current skill-level setting")
                            .small()
                            .weak(),
                    );
                }
            }
            ui.add_space(6.0);
            ui.separator();
        }
        ui.label(RichText::new("Your game").strong().color(gold));
        if !self.new_game_both_sides {
            ui.label("Play as");
            let button_width = ((ui.available_width() - 16.0) / 3.0).max(40.0);
            ui.horizontal(|ui| {
                for (side, label) in [
                    (NewGameColor::White, "White"),
                    (NewGameColor::Random, "Random"),
                    (NewGameColor::Black, "Black"),
                ] {
                    let selected = self.new_game_color == side;
                    let response =
                        Self::quick_choice(ui, "", selected, Vec2::new(button_width, 64.0));
                    let color = if selected {
                        gold
                    } else {
                        Color32::from_rgb(237, 236, 228)
                    };
                    let symbol = match side {
                        NewGameColor::White => "♔",
                        NewGameColor::Black => "♚",
                        NewGameColor::Random => "",
                    };
                    let center = response.rect.center();
                    if side == NewGameColor::Random {
                        let origin = egui::pos2(center.x, center.y - 11.0);
                        let stroke = Stroke::new(1.6, color);
                        for direction in [-1.0, 1.0] {
                            let points = [
                                (-10.0, -5.0 * direction),
                                (-4.0, -5.0 * direction),
                                (4.0, 5.0 * direction),
                                (10.0, 5.0 * direction),
                            ];
                            ui.painter().add(egui::Shape::line(
                                points
                                    .iter()
                                    .map(|(x, y)| origin + Vec2::new(*x, *y))
                                    .collect(),
                                stroke,
                            ));
                            ui.painter().add(egui::Shape::line(
                                vec![
                                    origin + Vec2::new(6.0, 5.0 * direction - 4.0),
                                    origin + Vec2::new(10.0, 5.0 * direction),
                                    origin + Vec2::new(6.0, 5.0 * direction + 4.0),
                                ],
                                stroke,
                            ));
                        }
                    }
                    ui.painter().text(
                        egui::pos2(center.x, center.y - 11.0),
                        Align2::CENTER_CENTER,
                        symbol,
                        FontId::proportional(28.0),
                        color,
                    );
                    ui.painter().text(
                        egui::pos2(center.x, center.y + 17.0),
                        Align2::CENTER_CENTER,
                        label,
                        FontId::proportional(14.0),
                        color,
                    );
                    response.widget_info(|| {
                        egui::WidgetInfo::selected(egui::WidgetType::Button, true, selected, label)
                    });
                    if response.clicked() {
                        self.new_game_color = side;
                    }
                }
            });
        } else {
            ui.label("Move White and Black yourself.");
        }
        ui.add_space(6.0);
        ui.label("Time control");
        let custom_selected = self.new_game_time != 5
            && ![(5, 3), (10, 5), (15, 10)]
                .contains(&(self.new_game_minutes, self.new_game_increment));
        let mut custom_clicked = false;
        ui.horizontal_wrapped(|ui| {
            for (minutes, increment) in [(5, 3), (10, 5), (15, 10)] {
                let selected = self.new_game_time != 5
                    && self.new_game_minutes == minutes
                    && self.new_game_increment == increment;
                if Self::quick_choice(
                    ui,
                    &format!("{minutes} + {increment}"),
                    selected,
                    Vec2::new(72.0, 40.0),
                )
                .clicked()
                {
                    self.new_game_time = 4;
                    self.new_game_minutes = minutes;
                    self.new_game_increment = increment;
                }
            }
            if Self::quick_choice(
                ui,
                "Untimed",
                self.new_game_time == 5,
                Vec2::new(80.0, 40.0),
            )
            .clicked()
            {
                self.new_game_time = 5;
            }
            custom_clicked =
                Self::quick_choice(ui, "Custom…", custom_selected, Vec2::new(86.0, 40.0)).clicked();
            if custom_clicked {
                self.new_game_time = 4;
            }
        });
        egui::CollapsingHeader::new("Custom clock")
            .id_salt("quick_custom_clock")
            .default_open(false)
            .open(if custom_clicked { Some(true) } else { None })
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    let changed = ui
                        .add(
                            egui::DragValue::new(&mut self.new_game_minutes)
                                .range(1..=180)
                                .suffix(" min"),
                        )
                        .changed()
                        | ui.add(
                            egui::DragValue::new(&mut self.new_game_increment)
                                .range(0..=180)
                                .suffix(" sec / move"),
                        )
                        .changed();
                    if changed {
                        self.new_game_time = 4;
                    }
                });
            });
        ui.separator();
        let chess960_allowed = self.quick_engine_supports_chess960();
        egui::CollapsingHeader::new(format!(
            "Starting position · {}",
            ["Standard", "Chess960", "Custom FEN"][self.new_game_position.min(2)]
        ))
        .id_salt("quick_position")
        .default_open(self.new_game_position != 0)
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                for (index, label) in ["Standard", "Chess960", "Custom FEN"].iter().enumerate() {
                    ui.add_enabled_ui(index != 1 || chess960_allowed, |ui| {
                        ui.selectable_value(&mut self.new_game_position, index, *label)
                            .on_disabled_hover_text("Select a Chess960-compatible engine.");
                    });
                }
            });
            if self.quick_position_is_chess960() && !chess960_allowed {
                ui.label("Select a Chess960-compatible engine to start this position.");
            }
            if self.new_game_position == 1 {
                ui.add(
                    egui::Label::new(
                        "Random Chess960 position. Castle by moving your king onto its rook.",
                    )
                    .wrap(),
                );
            }
            if self.new_game_position == 2 {
                ui.add(
                    egui::TextEdit::multiline(&mut self.new_game_fen)
                        .desired_width(f32::INFINITY)
                        .desired_rows(3)
                        .hint_text("Paste a complete FEN"),
                );
                let valid = Board::from_str(self.new_game_fen.trim()).is_ok();
                ui.label(if valid {
                    "Valid position"
                } else {
                    "Enter a valid FEN before starting."
                });
                if ui.button("Edit board / Import screenshot").clicked() {
                    #[cfg(target_arch = "wasm32")]
                    if let Err(error) = open_position_editor(&self.new_game_fen) {
                        self.engine_status = format!("Could not open position editor: {error:?}");
                    }
                }
            }
        });
    }

    fn start_quick_game(&mut self) {
        if !self.quick_position_valid() {
            return;
        }
        self.new_game_dialog_open = false;
        self.exit_puzzle_mode();
        if self.fics_active {
            self.stop_fics();
        }
        let side = if self.new_game_both_sides {
            PlayerSide::White
        } else {
            match self.new_game_color {
                NewGameColor::White => PlayerSide::White,
                NewGameColor::Black => PlayerSide::Black,
                NewGameColor::Random => {
                    if Self::animation_time().to_bits() & 1 == 0 {
                        PlayerSide::White
                    } else {
                        PlayerSide::Black
                    }
                }
            }
        };
        self.engine_enabled = !self.new_game_both_sides;
        if let Some(draft) = self.new_game_engine_draft.take() {
            self.engine_config = draft;
        }
        self.engine_config.opponent = self.new_game_opponent;
        if !self.new_game_both_sides {
            self.apply_new_game_strength();
        }
        self.reset_for_side(side);
        self.board = match self.new_game_position {
            1 => {
                let mut bytes = [0u8; 2];
                let _ = getrandom::fill(&mut bytes);
                Board::chess960(u16::from_le_bytes(bytes) % 960)
            }
            2 => Board::from_str(self.new_game_fen.trim()).expect("validated FEN"),
            _ => Board::default(),
        };
        self.review_positions = vec![self.board];
        if self.new_game_position == 2 {
            let fullmove = self
                .new_game_fen
                .split_whitespace()
                .nth(5)
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(1)
                .clamp(1, 100000);
            self.local_start_ply =
                (fullmove - 1) * 2 + usize::from(self.board.side_to_move() == Color::Black);
        }
        self.local_clock = (self.new_game_time != 5).then(|| LocalClock {
            white: f64::from(self.new_game_minutes * 60),
            black: f64::from(self.new_game_minutes * 60),
            increment: f64::from(self.new_game_increment),
            flagged_white: None,
            turn_elapsed: 0.0,
            updated_at: Self::animation_time(),
        });
        self.resume_engine_after_ready = false;
        #[cfg(target_arch = "wasm32")]
        self.queue_live_engine_resume();
        self.save_game();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chess960_start_requires_compatible_opponent() {
        let context = eframe::CreationContext::_new_kittest(egui::Context::default());
        let mut app = ChessApp::new(&context);
        app.new_game_opponent = OpponentEngine::Companion;
        app.engine_config.opponent = OpponentEngine::Companion;
        app.engine_config.companion_identity = "local/engine".into();
        app.new_game_position = 1;
        assert!(!app.quick_position_valid());

        app.new_game_position = 2;
        app.new_game_fen = Board::chess960(0).to_string();
        assert!(!app.quick_position_valid());
        app.new_game_both_sides = true;
        assert!(app.quick_position_valid());

        app.new_game_both_sides = false;
        let key = ChessApp::verification_engine_key(&app.engine_config);
        app.engine_uci_catalog.insert(key, vec![
            engine_uci_settings::parse_option("option name UCI_Chess960 type check default false").unwrap(),
        ]);
        assert!(app.quick_position_valid());
    }

    #[test]
    fn quick_setup_height_tracks_content_not_desktop_height() {
        let mut heights = Vec::new();
        for height in [1000.0, 1600.0] {
            let ctx = egui::Context::default();
            let mut app = ChessApp::new(&eframe::CreationContext::_new_kittest(ctx.clone()));
            app.navigation_new_game(false);
            app.new_game_opponent = OpponentEngine::Companion;
            for frame in 0..5 {
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            Vec2::new(1280.0, height),
                        )),
                        time: Some(frame as f64),
                        ..Default::default()
                    },
                    |_| app.new_game_dialog(&ctx),
                );
                output.textures_delta.clear();
            }
            let rect = ctx
                .memory(|m| m.area_rect(egui::Id::new("new_game_window")))
                .unwrap();
            assert!(
                rect.height() < 750.0,
                "Panel should fit its contents: {rect:?}"
            );
            heights.push(rect.height());
        }
        assert!(
            (heights[0] - heights[1]).abs() < 1.0,
            "Panel grew with the viewport: {heights:?}"
        );
    }

    #[test]
    fn quick_setup_fits_screen_and_escape_preserves_game() {
        for size in [
            Vec2::new(320.0, 640.0),
            Vec2::new(390.0, 844.0),
            Vec2::new(1280.0, 900.0),
        ] {
            let ctx = egui::Context::default();
            let mut app = ChessApp::new(&eframe::CreationContext::_new_kittest(ctx.clone()));
            let board = app.board;
            app.navigation_new_game(false);
            app.new_game_position = 2;
            app.new_game_fen = "Invalid position".into();
            for frame in 0..4 {
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                        time: Some(frame as f64),
                        ..Default::default()
                    },
                    |_| app.new_game_dialog(&ctx),
                );
                output.textures_delta.clear();
            }
            let rect = ctx
                .memory(|m| m.area_rect(egui::Id::new("new_game_window")))
                .unwrap();
            assert!(
                (rect.left() - if size.x < 600.0 { 0.0 } else { 56.0 }).abs() < 1.0,
                "{rect:?}"
            );
            assert!(
                rect.right() <= size.x + 1.0 && rect.bottom() <= size.y + 1.0,
                "Panel overflow at {size:?}: {rect:?}"
            );
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                    events: vec![egui::Event::Key {
                        key: egui::Key::Escape,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    }],
                    ..Default::default()
                },
                |_| app.new_game_dialog(&ctx),
            );
            output.textures_delta.clear();
            assert!(!app.new_game_dialog_open);
            assert_eq!(app.board, board);
            assert_eq!(app.new_game_fen, "Invalid position");
        }
    }

    #[test]
    fn outside_click_dismisses_without_clicking_the_underlying_board() {
        let ctx = egui::Context::default();
        let mut app = ChessApp::new(&eframe::CreationContext::_new_kittest(ctx.clone()));
        app.navigation_new_game(false);
        let original = app.engine_config.clone();
        app.new_game_engine_selection = Some(true);
        app.new_game_engine_draft.as_mut().unwrap().companion_name = "Draft only".into();
        let mut underlying_clicks = 0;
        for frame in 0..5 {
            let pos = egui::pos2(760.0, 50.0);
            let events = if frame == 2 || frame == 3 {
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: frame == 2,
                        modifiers: egui::Modifiers::NONE,
                    },
                ]
            } else {
                vec![]
            };
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(1000.0, 800.0),
                    )),
                    time: Some(frame as f64 * 0.1),
                    events,
                    ..Default::default()
                },
                |ui| {
                    if ui
                        .put(
                            egui::Rect::from_min_size(
                                egui::pos2(700.0, 20.0),
                                Vec2::new(160.0, 70.0),
                            ),
                            egui::Button::new("Underlying board"),
                        )
                        .clicked()
                    {
                        underlying_clicks += 1;
                    }
                    app.new_game_dialog(&ctx);
                },
            );
            output.textures_delta.clear();
        }
        assert!(!app.new_game_dialog_open);
        assert_eq!(underlying_clicks, 0);
        assert_eq!(app.engine_config.companion_name, original.companion_name);
        assert!(app.new_game_engine_draft.is_none());
        assert_eq!(app.new_game_engine_selection, Some(false));
    }

    #[test]
    fn quick_setup_validates_before_start_and_applies_position_clock_and_side() {
        let ctx = egui::Context::default();
        let mut app = ChessApp::new(&eframe::CreationContext::_new_kittest(ctx));
        let original = app.board;
        app.new_game_dialog_open = true;
        app.new_game_position = 2;
        app.new_game_fen = "bad FEN".into();
        app.start_quick_game();
        assert!(app.new_game_dialog_open);
        assert_eq!(app.board, original);
        app.new_game_fen = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 12".into();
        app.new_game_color = NewGameColor::Black;
        let previous_elo = app.engine_config.elo;
        let mut draft = app.engine_config.clone();
        draft.elo = 2050;
        draft.limit_strength = true;
        app.new_game_engine_draft = Some(draft);
        app.prepare_new_game_strength();
        assert_eq!(app.engine_config.elo, previous_elo);
        app.new_game_time = 4;
        app.new_game_minutes = 10;
        app.new_game_increment = 5;
        app.start_quick_game();
        assert!(!app.new_game_dialog_open);
        assert_eq!(app.engine_config.elo, 2050);
        assert_eq!(app.board, Board::from_str(&app.new_game_fen).unwrap());
        assert!(matches!(app.player_side, PlayerSide::Black));
        assert_eq!(app.local_start_ply, 23);
        let clock = app.local_clock.as_ref().unwrap();
        assert_eq!(clock.white, 600.0);
        assert_eq!(clock.increment, 5.0);
    }
}
