use super::*;

impl ChessApp {
    fn tactical_safety_text(safety: &crate::tactical::PinAwareSafety) -> String {
        let list = |squares: &[Square]| if squares.is_empty() { "none".to_owned() }
            else { squares.iter().map(ToString::to_string).collect::<Vec<_>>().join(", ") };
        let mut text = format!("Geometric attackers: {}\nGeometric defenders: {}\nLegal recapturers (across candidate captures): {}\nDefenders unable to recapture: {}",
            list(&safety.attackers), list(&safety.defenders), list(&safety.recapturers), list(&safety.blocked));
        if safety.captures.is_empty() { text.push_str("\nNo legal enemy capture on this square; recapture count is not applicable."); }
        for (attacker, defenders) in &safety.captures {
            text.push_str(&format!("\nIf {attacker} captures: legal recaptures from {}", list(defenders)));
        }
        text.push_str("\nRecaptures must leave their own king safe. Pins, opened lines, and king moves are checked. Not a trade evaluation; other replies can change this.");
        text
    }
    pub(super) fn tactical_piece_active(&self, ctx: &egui::Context) -> bool {
        self.tactical_available() && ctx.data(|d| d.get_temp::<bool>(egui::Id::new("tactical_enabled")).unwrap_or(false))
    }
    pub(super) fn tactical_knight_active(&self, ctx: &egui::Context) -> bool {
        self.tactical_available() && ctx.data(|d|
            d.get_temp::<bool>(egui::Id::new("tactical_enabled")).unwrap_or(false)
                && d.get_temp::<bool>(egui::Id::new("tactical_knight_reach")).unwrap_or(false))
    }
    pub(super) fn tactical_knight_selection(&self, ctx: &egui::Context, square: Square) -> bool {
        if !self.tactical_piece_active(ctx) { return false; }
        let source = ctx.data(|d| d.get_temp::<Square>(egui::Id::new("tactical_knight_source")));
        let lock_id = egui::Id::new("tactical_knight_lock");
        if self.review_index.is_some() && self.tactical_knight_active(ctx) && let Some(source) = source
            && crate::tactical::knight_routes(&self.board, source).iter().any(|(first, _)| *first == square) {
            let route = (self.board, source, square);
            ctx.data_mut(|d| {
                if d.get_temp::<(Board, Square, Square)>(lock_id) == Some(route) { d.remove::<(Board, Square, Square)>(lock_id); }
                else { d.insert_temp(lock_id, route); }
            });
            ctx.request_repaint();
            return true;
        }
        if self.board.piece_on(square) == Some(Piece::Knight) {
            ctx.data_mut(|d| {
                d.insert_temp(egui::Id::new("tactical_knight_source"), square);
                d.remove::<(Board, Square, Square)>(lock_id);
            });
            ctx.request_repaint();
        }
        if self.board.piece_on(square).is_some() {
            ctx.data_mut(|d| d.insert_temp(egui::Id::new("tactical_activity_piece"), (self.board, square)));
            ctx.request_repaint();
        }
        // Inspection consumes review clicks; live-game clicks must reach move selection.
        self.review_index.is_some()
    }
    fn tactical_available(&self) -> bool {
        !self.training_live() && !(self.fics_active && self.fics_playing)
            && self.best_move_attempt.is_none() && self.prediction_index == 0
    }
    fn tactical_findings(&self, ctx: &egui::Context) -> Vec<crate::tactical::Finding> {
        let index = self.review_index.unwrap_or(self.review_moves.len());
        let previous = index.checked_sub(1).and_then(|i| self.review_positions.get(i)).copied();
        type Cache = (Board, Option<Board>, Vec<crate::tactical::Finding>);
        let id = egui::Id::new("tactical_cache");
        if let Some((board, before, findings)) = ctx.data(|d| d.get_temp::<Cache>(id))
            && board == self.board && before == previous { return findings; }
        let findings = crate::tactical::detect(&self.board, previous.as_ref());
        ctx.data_mut(|d| d.insert_temp(id, (self.board, previous, findings.clone())));
        findings
    }
    fn tactical_changes(&self, ctx: &egui::Context) -> Option<Vec<crate::tactical::Change>> {
        let index = self.review_index.unwrap_or(self.review_moves.len());
        if self.review_positions.get(index) != Some(&self.board) { return None; }
        let before = *self.review_positions.get(index.checked_sub(1)?)?;
        type Cache = (Board, Board, Vec<crate::tactical::Change>);
        let id = egui::Id::new("tactical_changes_cache");
        if let Some((_, _, changes)) = ctx.data(|d| d.get_temp::<Cache>(id))
            .filter(|(old, current, _)| *old == before && *current == self.board) { return Some(changes); }
        let changes = crate::tactical::changes_after_move(&before, &self.board);
        ctx.data_mut(|d| d.insert_temp(id, (before, self.board, changes.clone())));
        Some(changes)
    }
    fn tactical_change_color(kind: crate::tactical::ChangeKind) -> Color32 {
        match kind {
            crate::tactical::ChangeKind::OpenedLine => Color32::from_rgb(100, 195, 230),
            crate::tactical::ChangeKind::LostDefender => Color32::from_rgb(230, 178, 65),
            crate::tactical::ChangeKind::NewThreat => Color32::from_rgb(240, 130, 105),
        }
    }
    fn tactical_opponent_threats(&self, ctx: &egui::Context) -> Option<Vec<crate::tactical::OpponentThreat>> {
        type Cache = (Board, Option<Vec<crate::tactical::OpponentThreat>>);
        let id = egui::Id::new("tactical_opponent_cache");
        if let Some((_, threats)) = ctx.data(|d| d.get_temp::<Cache>(id)).filter(|(board, _)| *board == self.board) { return threats; }
        let threats = crate::tactical::opponent_threats(&self.board);
        ctx.data_mut(|d| d.insert_temp(id, (self.board, threats.clone())));
        threats
    }
    pub(super) fn tactical_toggle_ui(&self, ui: &mut egui::Ui) {
        let id = egui::Id::new("tactical_enabled");
        let mut open = ui.ctx().data(|d| d.get_temp::<bool>(id)).unwrap_or(false);
        let response = Self::glossy_board_button(ui, "", open, self.tactical_available())
            .on_hover_text(if open { "Close Tactical Map" } else { "Open Tactical Map" })
            .on_disabled_hover_text("Tactical Map is available when reviewing or analyzing a position.");
        let center = response.rect.center();
        let stroke = ui.style().interact(&response).fg_stroke;
        ui.painter().circle_stroke(center, 7.0, stroke);
        ui.painter().circle_stroke(center, 3.8, stroke);
        ui.painter().circle_filled(center, 1.4, stroke.color);
        if response.clicked() {
            open = !open;
            ui.ctx().data_mut(|d| d.insert_temp(id, open));
        }
        response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, self.tactical_available(), open, "Tactical Map"));
    }

    pub(super) fn tactical_window(&self, ctx: &egui::Context) {
        if !self.tactical_available() { return; }
        let id = egui::Id::new("tactical_enabled");
        let mut open = ctx.data(|d| d.get_temp::<bool>(id)).unwrap_or(false);
        if !open { return; }
        let width = (ctx.content_rect().width() - 32.0).clamp(360.0, 420.0);
        let frame = Frame::window(&ctx.style_of(ctx.theme()))
            .fill(Color32::from_rgb(24, 28, 33))
            .inner_margin(Margin::same(14))
            .corner_radius(CornerRadius::same(12))
            .stroke(Stroke::new(1.0, Color32::from_rgb(211, 173, 98)));
        egui::Window::new(RichText::new("Tactical Map").size(18.0).strong().color(Color32::from_rgb(211, 173, 98)))
            .id(egui::Id::new("tactical_window"))
            .frame(frame)
            .default_pos(egui::pos2(48.0, 100.0))
            .default_width(width)
            .default_height(600.0)
            .min_width(360.0)
            .min_height(240.0)
            .resizable(true)
            .collapsible(false)
            .title_bar(false)
            .show(ctx, |ui| {
                let grain=ui.painter().add(egui::Shape::Noop);
                if Self::dialog_header(ui, "Tactical Map") { open = false; }
                ui.add_space(6.0);
                egui::ScrollArea::vertical().id_salt("tactical_window_content")
                    .max_height(ui.available_height().max(0.0))
                    .auto_shrink([false, false]).show_gold(ui, |ui| self.tactical_map_contents(ui));
                let paper=Self::paper_grain_texture(ui.ctx());
                let surface=ui.min_rect().expand(6.0);
                ui.painter().set(grain,egui::Shape::image(paper.id(),surface,
                    egui::Rect::from_min_max(egui::Pos2::ZERO,egui::pos2(surface.width()/256.0,surface.height()/256.0)),Color32::WHITE));
            });
        ctx.data_mut(|d| d.insert_temp(id, open));
    }

    fn tactical_legend_badge(ui: &mut egui::Ui, label: &str, color: Color32) {
        Frame::new().fill(color.gamma_multiply(0.1)).stroke(Stroke::new(1.0, color.gamma_multiply(0.4)))
            .corner_radius(4.0).inner_margin(Margin::symmetric(7, 4)).show(ui, |ui| {
                ui.label(RichText::new(label).size(11.0).strong().color(color));
            });
    }

    fn tactical_sunken_section<R>(ui: &mut egui::Ui, contents: impl FnOnce(&mut egui::Ui) -> R) -> egui::InnerResponse<R> {
        let section = Frame::new().fill(Color32::from_rgb(13, 17, 22))
            .stroke(Stroke::new(1.0, Color32::from_white_alpha(25)))
            .corner_radius(6.0).inner_margin(12.0).show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                contents(ui)
            });
        Self::paint_sunken_border(ui, section.response.rect);
        section
    }

    fn tactical_map_contents(&self, ui: &mut egui::Ui) {
        let muted = Color32::from_rgb(160, 173, 190);
        ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
        let tab_id = egui::Id::new("tactical_map_tab");
        let current_tab = ui.ctx().data(|d| d.get_temp::<usize>(tab_id)).unwrap_or(0);
        let tab = Self::engine_settings_tabs(ui, &["Insights", "Activity", "Patterns"], current_tab);
        ui.ctx().data_mut(|d| d.insert_temp(tab_id, tab));
        ui.add_space(6.0);
        let mut filters = ui.ctx().data(|d| d.get_temp::<[bool; 3]>(egui::Id::new("tactical_filters"))).unwrap_or([true; 3]);
        if tab == 0 {
        let layout_id = egui::Id::new("tactical_insights_list_layout").with(ui.available_width().to_bits());
        let (overhead, previous_demand) = ui.ctx().data(|d| d.get_temp::<(f32, [f32; 2])>(layout_id)).unwrap_or((260.0, [180.0; 2]));
        let enabled = [
            ui.ctx().data(|d| d.get_temp::<bool>(egui::Id::new("tactical_show_opponent"))).unwrap_or(false),
            ui.ctx().data(|d| d.get_temp::<bool>(egui::Id::new("tactical_show_changes"))).unwrap_or(true),
        ];
        let start_y = ui.cursor().top();
        let budget = (ui.clip_rect().bottom() - start_y - overhead - 4.0).max(0.0);
        let mut list_limits = [0.0; 2];
        match enabled {
            [true, true] => {
                let half = budget * 0.5;
                list_limits = if previous_demand[0] < half {
                    [previous_demand[0], budget - previous_demand[0]]
                } else if previous_demand[1] < half {
                    [budget - previous_demand[1], previous_demand[1]]
                } else { [half, half] };
            }
            [true, false] => list_limits[0] = budget,
            [false, true] => list_limits[1] = budget,
            _ => {}
        }
        let mut list_heights = [0.0; 2];
        let mut list_demand = previous_demand;
        ui.label(RichText::new("POSITION INSIGHTS").strong().color(Color32::from_rgb(211, 173, 98)));
        ui.add_space(3.0);
        Self::tactical_sunken_section(ui, |ui| {
        let count_id = egui::Id::new("tactical_attack_defense");
        let mut show_counts = ui.ctx().data(|d| d.get_temp::<bool>(count_id)).unwrap_or(false);
        if ui.checkbox(&mut show_counts, RichText::new("Attack vs Defense").size(15.0).strong()).on_hover_text(
            "Red: geometric attackers. Green: geometric defenders/legal recapturers. Hover a piece for capture-specific legal recaptures. A dash means no legal capture to recapture."
        ).changed() { ui.ctx().request_repaint(); }
        ui.ctx().data_mut(|d| d.insert_temp(count_id, show_counts));
        if show_counts {
                ui.horizontal_wrapped(|ui| {
                    Self::tactical_legend_badge(ui, "Attackers", Color32::from_rgb(240, 130, 105));
                    Self::tactical_legend_badge(ui, "Defenders / recapturers", Color32::from_rgb(136, 194, 151));
                });
                ui.label(RichText::new("Hover a piece for capture-specific details.").size(12.0).color(muted));
                ui.collapsing("How to read the counts", |ui| {
                    ui.label("Counts show geometric control, including pinned pieces. Recapturers must be legal.");
                    for (count, meaning) in [("2/-", "No legal enemy capture; recaptures do not apply."), ("2/0", "No defender can legally recapture."), ("2/1", "One defender can legally recapture.")] {
                        ui.horizontal_wrapped(|ui| { ui.strong(count); ui.label(meaning); });
                    }
                });
        }
        });
        ui.add_space(6.0);
        Self::tactical_sunken_section(ui, |ui| {
        let threats_id = egui::Id::new("tactical_show_opponent");
        let mut show_threats = ui.ctx().data(|d| d.get_temp::<bool>(threats_id)).unwrap_or(false);
        ui.checkbox(&mut show_threats, RichText::new("Opponent threats").size(15.0).strong());
        ui.ctx().data_mut(|d| d.insert_temp(threats_id, show_threats));
        if show_threats {
            ui.label(RichText::new(format!("{} candidates · position unchanged",
                if self.board.side_to_move() == Color::White { "Black" } else { "White" })).size(12.0).color(muted));
            ui.label(RichText::new("Click to trace; click again to clear. Captures and forks may be unsound.").size(11.0).color(muted));
            let selection_id = egui::Id::new("tactical_opponent_selected");
            let mut selected = ui.ctx().data(|d| d.get_temp::<(Board, String)>(selection_id))
                .filter(|(board, _)| *board == self.board).map(|(_, key)| key).unwrap_or_default();
            match self.tactical_opponent_threats(ui.ctx()) {
                None => { ui.label("The side to move is in check. Resolve check before exploring a hypothetical opponent turn."); selected.clear(); }
                Some(threats) => {
                    if !threats.iter().any(|t| t.key == selected) { selected.clear(); }
                    let list_output = egui::ScrollArea::vertical().id_salt("opponent_threats_list")
                        .max_height(list_limits[0].max(48.0)).min_scrolled_height(0.0).show_gold(ui, |ui| {
                        if threats.is_empty() { ui.label("No candidate checks, captures, or forks."); }
                        for (index, threat) in threats.into_iter().enumerate() {
                            let active = selected == threat.key;
                            let card = Frame::new().fill(if active { Color32::from_rgb(52, 44, 29) } else { Color32::from_rgb(25, 31, 38) })
                                .stroke(Stroke::new(1.0, if active { Color32::from_rgb(211, 173, 98) } else { Color32::from_white_alpha(22) }))
                                .corner_radius(5.0).inner_margin(9.0).show(ui, |ui| {
                                    ui.set_min_width(ui.available_width());
                                    let (mv, detail) = threat.text.split_once(": ").unwrap_or((&threat.text, ""));
                                    ui.label(RichText::new(mv).strong().size(14.0).color(Color32::from_rgb(241, 206, 131)));
                                    if !detail.is_empty() { ui.add(egui::Label::new(RichText::new(detail).size(12.0).color(Color32::from_rgb(218, 225, 235))).wrap()); }
                                });
                            let response = ui.interact(card.response.rect, ui.id().with(("opponent_threat_card", index)), Sense::click())
                                .on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(&threat.text);
                            if response.hovered() { ui.painter().rect_stroke(card.response.rect, 5, Stroke::new(1.0, Color32::from_rgb(211, 173, 98)), egui::StrokeKind::Inside); }
                            if response.clicked() { selected = if active { String::new() } else { threat.key }; }
                        }
                    });
                    list_heights[0] = list_output.inner_rect.height();
                    list_demand[0] = list_output.content_size.y;
                }
            }
            ui.ctx().data_mut(|d| d.insert_temp(selection_id, (self.board, selected)));
        }
        });
        ui.add_space(6.0);
        Self::tactical_sunken_section(ui, |ui| {
        let changes_id = egui::Id::new("tactical_show_changes");
        let mut show_changes = ui.ctx().data(|d| d.get_temp::<bool>(changes_id)).unwrap_or(true);
        ui.checkbox(&mut show_changes, RichText::new("What changed?").size(15.0).strong());
        ui.ctx().data_mut(|d| d.insert_temp(changes_id, show_changes));
        if show_changes {
            match self.tactical_changes(ui.ctx()) {
                None => { ui.label(RichText::new("Step forward in a game to compare with the previous position.").size(12.0).color(muted)); }
                Some(changes) => {
                    ui.horizontal_wrapped(|ui| {
                        for (label, color) in [("Opened attacks", Color32::from_rgb(102, 162, 226)), ("Lost support", Color32::from_rgb(211, 173, 98)), ("New threats", Color32::from_rgb(240, 130, 105))] {
                            Self::tactical_legend_badge(ui, label, color);
                        }
                    });
                    ui.label(RichText::new("Geometric control includes pinned pieces.").size(11.0).color(muted));
                    let list_output = egui::ScrollArea::vertical().id_salt("tactical_changes_list")
                        .max_height(list_limits[1].max(48.0)).min_scrolled_height(0.0).show_gold(ui, |ui| {
                        if changes.is_empty() { ui.label("No opened attack lines, lost defenders, or new threats after this move."); }
                        for change in changes {
                            let color = Self::tactical_change_color(change.kind);
                            let row = Frame::new().fill(Color32::from_rgb(25, 31, 38)).corner_radius(5.0).inner_margin(9.0).show(ui, |ui| {
                                ui.set_min_width(ui.available_width());
                                ui.add(egui::Label::new(RichText::new(change.text).size(12.0).color(color)).wrap());
                            });
                            ui.painter().line_segment([row.response.rect.left_top() + Vec2::new(2.0, 5.0), row.response.rect.left_bottom() + Vec2::new(2.0, -5.0)], Stroke::new(2.0, color));
                        }
                    });
                    list_heights[1] = list_output.inner_rect.height();
                    list_demand[1] = list_output.content_size.y;
                }
            }
        }
        });
        let measured_overhead = (ui.cursor().top() - start_y - list_heights.iter().sum::<f32>()).max(0.0);
        if (measured_overhead - overhead).abs() > 0.5 || list_demand.iter().zip(previous_demand).any(|(a, b)| (*a - b).abs() > 0.5) {
            ui.ctx().data_mut(|d| d.insert_temp(layout_id, (measured_overhead, list_demand)));
            ui.ctx().request_repaint();
        }
        }
        if tab == 1 {
        Self::tactical_sunken_section(ui, |ui| {
        ui.label(RichText::new("PIECE ACTIVITY").strong().color(Color32::from_rgb(211, 173, 98)));
        ui.label(RichText::new("Choose a piece on the board").size(17.0).strong());
        ui.label(RichText::new("Inspect its reach, influence, and position.").size(12.0).color(muted));
        if let Some((board, square)) = ui.ctx().data(|d| d.get_temp::<(Board, Square)>(egui::Id::new("tactical_activity_piece")))
            && board == self.board && let Some(activity) = crate::tactical::positional_activity(&board, square) {
            Frame::new().fill(Color32::from_rgb(24, 31, 42))
                .stroke(Stroke::new(1.0, Color32::from_white_alpha(30)))
                .corner_radius(CornerRadius::same(8)).inner_margin(Margin::same(10)).show(ui, |ui| {
                    ui.label(RichText::new(&activity.title).size(17.0).strong().color(Color32::from_rgb(241, 206, 131)));
                    ui.label(RichText::new(activity.rating).size(14.0).color(Color32::from_rgb(226, 231, 235)));
                    Self::full_analysis_progress(ui, activity.score as f32 / 100.0, &format!("{} / 100 activity", activity.score));
                    ui.collapsing("Why this rating?", |ui| {
                        for reason in activity.reasons { ui.label(format!("• {reason}")); }
                    });
                    ui.label(RichText::new("Positional heuristic · not material value or tactical safety.").size(11.0).color(muted))
                        .on_hover_text("Geometric control includes pinned pieces.");
                });
        }
        });
        ui.add_space(6.0);
        Self::tactical_sunken_section(ui, |ui| {
        ui.label(RichText::new("KNIGHT EXPLORER").strong().color(Color32::from_rgb(211, 173, 98)));
        let reach_id = egui::Id::new("tactical_knight_reach");
        let mut reach = ui.ctx().data(|d| d.get_temp::<bool>(reach_id)).unwrap_or(false);
        ui.checkbox(&mut reach, RichText::new("Explore two-hop routes").size(15.0).strong());
        ui.ctx().data_mut(|d| d.insert_temp(reach_id, reach));
        if reach {
            let knight_id = egui::Id::new("tactical_knight_source");
            let mut source = ui.ctx().data(|d| d.get_temp::<Square>(knight_id));
            let knights: Vec<_> = chess::ALL_SQUARES.into_iter().filter(|&s|
                self.board.piece_on(s) == Some(Piece::Knight)).collect();
            if !source.is_some_and(|s| knights.contains(&s)) {
                source = self.selected.filter(|s| knights.contains(s)).or_else(|| knights.first().copied());
            }
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Knight").small().color(muted));
                for square in &knights {
                    let active = source == Some(*square);
                    if ui.add(egui::Button::new(RichText::new(square.to_string()).strong().color(Color32::from_rgb(241, 206, 131)))
                        .selected(active).min_size(Vec2::new(44.0, 32.0)).corner_radius(5.0)
                        .stroke(Stroke::new(1.0, if active { Color32::from_rgb(211, 173, 98) } else { Color32::from_white_alpha(35) }))).clicked() { source = Some(*square); }
                }
            });
            ui.ctx().data_mut(|d| {
                d.remove::<Square>(knight_id);
                if let Some(square) = source { d.insert_temp(knight_id, square); }
            });
            let lock_id = egui::Id::new("tactical_knight_lock");
            if let Some((board, knight, first)) = ui.ctx().data(|d| d.get_temp::<(Board, Square, Square)>(lock_id)) {
                if board != self.board || Some(knight) != source {
                    ui.ctx().data_mut(|d| d.remove::<(Board, Square, Square)>(lock_id));
                } else {
                    ui.horizontal(|ui| {
                        ui.label(format!("Route locked: {knight} to {first}"));
                        if ui.button("Unlock route").clicked() {
                            ui.ctx().data_mut(|d| d.remove::<(Board, Square, Square)>(lock_id));
                        }
                    });
                }
            }
            let safety_id = egui::Id::new("tactical_destination_safety");
            let mut safety = ui.ctx().data(|d| d.get_temp::<bool>(safety_id)).unwrap_or(true);
            ui.checkbox(&mut safety, RichText::new("Show destination safety").strong());
            ui.ctx().data_mut(|d| d.insert_temp(safety_id, safety));
            ui.label(RichText::new(if self.review_index.is_some() {
                "Click a ghost to lock/unlock its route."
            } else { "Live game: board clicks select and move pieces. Hover ghosts to preview; route locking is available in review." }).size(12.0).color(muted));
            if safety {
                ui.horizontal_wrapped(|ui| {
                    for (label, color) in [("Attackers", Color32::from_rgb(240, 130, 105)), ("Legal recapturers", Color32::from_rgb(136, 194, 151)), ("Unavailable", Color32::from_rgb(211, 173, 98)), ("Geometric support", muted)] {
                        Self::tactical_legend_badge(ui, label, color);
                    }
                });
            }
            if knights.is_empty() { ui.label(RichText::new("No knights in this position.").color(muted)); }
            else {
                ui.horizontal_wrapped(|ui| {
                    Self::tactical_legend_badge(ui, "Ghost · first hop", Color32::from_rgb(181, 195, 215));
                    Self::tactical_legend_badge(ui, "Outline · second hop", muted);
                    Self::tactical_legend_badge(ui, "Gold · possible fork", Color32::from_rgb(211, 173, 98));
                });
                ui.collapsing("How to explore routes", |ui| {
                    ui.label("Click a knight or choose its square above. Hover a ghost to focus its route and inspect capture-specific recaptures.");
                    ui.label("Off-turn knights show geometric reach, including pinned moves. Other pieces stay put; replies can change reach.");
                });
            }
        }
        });
        }
        if tab == 2 {
        let findings = self.tactical_findings(ui.ctx());
        ui.label(RichText::new("Patterns").size(14.0).strong().color(Color32::from_rgb(211, 173, 98)));
        ui.horizontal_wrapped(|ui| {
            for (index, kind, label) in [
                (0, crate::tactical::Kind::Danger, "Danger"),
                (1, crate::tactical::Kind::Pin, "Pins"),
                (2, crate::tactical::Kind::Fork, "Forks"),
            ] {
                let count = findings.iter().filter(|f| f.kind == kind).count();
                let color = Self::tactical_color(kind);
                let button = egui::Button::new(RichText::new(format!("{label} {count}")).size(11.0)
                    .color(if filters[index] { color } else { muted }))
                    .fill(if filters[index] { color.gamma_multiply(0.14) } else { Color32::from_rgba_unmultiplied(35, 43, 56, 170) })
                    .stroke(Stroke::new(1.0, if filters[index] { color.gamma_multiply(0.55) } else { Color32::from_gray(55) }))
                    .corner_radius(CornerRadius::same(8))
                    .min_size(Vec2::new(58.0, 26.0));
                if ui.add_sized([58.0, 26.0], button).on_hover_text(format!("Show or hide {label}" )).clicked() { filters[index] = !filters[index]; }
            }
        });
        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("tactical_filters"), filters));
        let selection = egui::Id::new("tactical_selection");
        let mut selected = ui.ctx().data(|d| d.get_temp::<String>(selection)).unwrap_or_default();
        let visible: Vec<_> = findings.iter().filter(|f| filters[Self::tactical_filter(f.kind)]).collect();
        if !visible.iter().any(|f| f.text == selected) { selected.clear(); }
        ui.add_space(3.0);
        egui::ScrollArea::vertical().id_salt("tactical_list").max_height(320.0).auto_shrink([false, true]).show(ui, |ui| {
            for (index, f) in visible.iter().enumerate() {
                let active = selected == f.text;
                let color = Self::tactical_finding_color(f);
                let title = match f.kind {
                    crate::tactical::Kind::Danger => format!("Exposed piece · {}", f.squares[0]),
                    crate::tactical::Kind::Pin => format!("Absolute pin · {}", f.squares[1]),
                    crate::tactical::Kind::Fork => format!("{} · {} to {}", f.fork.as_ref().map_or("Fork candidate", |fork| fork.label()), f.squares[0], f.squares[1]),
                };
                let description = f.text.split_once(": ").map_or(f.text.as_str(), |(_, body)| body)
                    .split(". ").next().unwrap_or(&f.text);
                let card = Frame::new()
                    .fill(if active { color.gamma_multiply(0.12) } else { Color32::from_rgb(13, 17, 22) })
                    .stroke(Stroke::new(1.0, if active { color.gamma_multiply(0.75) } else { Color32::from_rgba_unmultiplied(100, 119, 143, 65) }))
                    .corner_radius(CornerRadius::same(9)).inner_margin(Margin::same(11))
                    .show(ui, |ui| {
                        ui.set_width((ui.available_width()).max(0.0));
                        ui.horizontal(|ui| {
                            let (marker, _) = ui.allocate_exact_size(Vec2::splat(12.0), egui::Sense::hover());
                            ui.painter().circle_stroke(marker.center(), 3.5, Stroke::new(1.2, color));
                            if active { ui.painter().circle_filled(marker.center(), 2.0, color); }
                            ui.label(RichText::new(title).size(13.0).strong().color(color));
                        });
                        ui.add(egui::Label::new(RichText::new(description).size(13.0).color(Color32::from_rgb(218, 225, 235))).wrap());
                        if let Some(fork) = &f.fork {
                            ui.label(RichText::new(fork.badges()).size(12.0).color(color));
                            if !fork.capture_replies.is_empty() {
                                ui.label(RichText::new(format!("Legal capture replies: {}", fork.capture_replies.iter()
                                    .map(|reply| format!("{} × {}", reply.get_source(), reply.get_dest())).collect::<Vec<_>>().join(", "))).size(12.0));
                            }
                        }
                    });
                Self::paint_sunken_border(ui, card.response.rect);
                let response = ui.interact(card.response.rect, ui.id().with(("tactical_card", index)), egui::Sense::click())
                    .on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(&f.text);
                if response.hovered() {
                    ui.painter().rect_stroke(card.response.rect, 9.0, Stroke::new(1.0, color.gamma_multiply(0.65)), egui::StrokeKind::Inside);
                }
                if response.clicked() { selected = if active { String::new() } else { f.text.clone() }; }
            }
            if visible.is_empty() {
                Self::tactical_sunken_section(ui, |ui| {
                        ui.label(RichText::new("No patterns to show").strong().color(Color32::from_rgb(218, 225, 235)));
                        ui.label(RichText::new("Try another position or enable more filters.").size(12.0).color(muted));
                    });
            }
        });
        ui.ctx().data_mut(|d| d.insert_temp(selection, selected));
        }
    }
    fn tactical_filter(kind: crate::tactical::Kind) -> usize {
        match kind { crate::tactical::Kind::Danger => 0, crate::tactical::Kind::Pin => 1, crate::tactical::Kind::Fork => 2 }
    }
    fn tactical_finding_color(finding: &crate::tactical::Finding) -> Color32 {
        if let Some(fork) = &finding.fork {
            if !fork.capture_replies.is_empty() { return Color32::from_rgb(135, 173, 185); }
            if fork.strong() { return Color32::from_rgb(245, 139, 88); }
            if fork.gives_check { return Color32::from_rgb(230, 178, 65); }
        }
        Self::tactical_color(finding.kind)
    }

    fn tactical_color(kind: crate::tactical::Kind) -> Color32 {
        match kind {
            crate::tactical::Kind::Danger => Color32::from_rgb(240, 130, 105),
            crate::tactical::Kind::Pin => Color32::from_rgb(195, 155, 245),
            crate::tactical::Kind::Fork => Color32::from_rgb(100, 195, 230),
        }
    }
    fn knight_3d_ghosts(&self, ctx: &egui::Context, rect: egui::Rect, source: Square) -> Vec<(Square, egui::TextureHandle, egui::Rect)> {
        type Cached = (String, Vec<(Square, egui::TextureHandle, egui::Rect)>);
        let Some(side) = self.board.color_on(source).filter(|_| self.board.piece_on(source) == Some(Piece::Knight)) else { return Vec::new(); };
        let view = self.board_3d_view();
        let adjustments = self.render_piece_appearance();
        let key = format!("{}:{:?}:{:?}:{:?}:{:?}:{:?}:{}:{}:{}", self.board, source, self.board_3d_theme,
            (view.yaw, view.elevation, view.distance), adjustments, rect, self.flipped,
            self.board_3d_appearance, ctx.pixels_per_point());
        let id = egui::Id::new("knight_reach_3d_ghosts");
        if let Some((_, ghosts)) = ctx.data(|d| d.get_temp::<Cached>(id)).filter(|cached| cached.0 == key) { return ghosts; }
        let ghosts = crate::tactical::knight_routes(&self.board, source).into_iter().filter_map(|(first, _)| {
            let center = crate::board3d::square_center(first, rect, self.flipped, view)?;
            let (image, offset, size) = crate::board3d::drag_piece_image(first, Piece::Knight, side, rect,
                self.flipped, view, self.board_3d_appearance as f32 / 100.0, self.board_3d_theme,
                adjustments, ctx.pixels_per_point().min(1.5));
            Some((first, ctx.load_texture(format!("knight_reach_{first}"), image, egui::TextureOptions::LINEAR),
                egui::Rect::from_min_size(center + offset, size)))
        }).collect::<Vec<_>>();
        ctx.data_mut(|d| d.insert_temp(id, (key, ghosts.clone())));
        ghosts
    }

    pub(super) fn tactical_knight_3d_hit(&self, ctx: &egui::Context, pos: egui::Pos2, rect: egui::Rect) -> Option<Square> {
        if !self.board_3d_rendering() || !self.tactical_knight_active(ctx) || !rect.contains(pos) { return None; }
        let source = ctx.data(|d| d.get_temp::<Square>(egui::Id::new("tactical_knight_source")))?;
        self.knight_3d_ghosts(ctx, rect, source).iter().filter(|(_, _, bounds)| bounds.contains(pos))
            .min_by(|a, b| a.2.center().distance_sq(pos).total_cmp(&b.2.center().distance_sq(pos)))
            .map(|(square, _, _)| *square)
    }

    pub(super) fn paint_tactical_map(&self, ui: &mut egui::Ui, rect: egui::Rect) {
        if !self.tactical_available() || !ui.ctx().data(|d| d.get_temp::<bool>(egui::Id::new("tactical_enabled"))).unwrap_or(false) { return; }
        if ui.ctx().data(|d| d.get_temp::<bool>(egui::Id::new("tactical_knight_reach"))).unwrap_or(false)
            && let Some(source) = ui.ctx().data(|d| d.get_temp::<Square>(egui::Id::new("tactical_knight_source"))) {
            let is_3d = self.board_3d_rendering();
            let cell = rect.width() / if is_3d { 10.0 } else { 8.0 };
            let center = |s| if is_3d {
                crate::board3d::square_center(s, rect, self.flipped, self.board_3d_view()).unwrap_or(rect.center())
            } else { rect.min + Self::square_screen_offset(s, self.flipped, cell) };
            let routes = crate::tactical::knight_routes(&self.board, source);
            let hover = ui.ctx().pointer_hover_pos().filter(|&p| ui.rect_contains_pointer(rect) && rect.contains(p));
            let hovered_square = hover.and_then(|pos| self.tactical_knight_3d_hit(ui.ctx(), pos, rect)
                .or_else(|| self.board_mark_square(pos, rect)));
            let hovered_first = routes.iter().find(|(s, _)| Some(*s) == hovered_square).map(|(s, _)| *s);
            let locked = ui.ctx().data(|d| d.get_temp::<(Board, Square, Square)>(egui::Id::new("tactical_knight_lock")))
                .filter(|(board, knight, first)| *board == self.board && *knight == source && routes.iter().any(|(s, _)| s == first))
                .map(|(_, _, first)| first);
            let focused = locked.or(hovered_first);
            let cyan = Color32::from_rgb(100, 195, 230);
            let painter = ui.painter().with_clip_rect(rect);
            let outline = |square, scale, stroke| {
                if is_3d {
                    if let Some(points) = crate::board3d::square_outline(square, rect, self.flipped, self.board_3d_view()) {
                        let middle = center(square);
                        painter.add(egui::Shape::closed_line(points.into_iter().map(|p| middle + (p - middle) * scale).collect(), stroke));
                    }
                } else {
                    painter.rect_stroke(egui::Rect::from_center_size(center(square), Vec2::splat(cell * scale)), 3.0, stroke, egui::StrokeKind::Inside);
                }
            };
            outline(source, 0.94, Stroke::new(2.5, cyan));
            for (first, next) in &routes {
                if focused.is_some_and(|s| s != *first) { continue; }
                for &(target, fork) in next {
                    let color = if fork { Color32::from_rgb(230, 178, 65) } else { cyan.gamma_multiply(if focused.is_some() { 0.8 } else { 0.35 }) };
                    outline(target, 0.76, Stroke::new(if fork { 2.5 } else { 1.5 }, color));
                    if focused.is_some() { painter.arrow(center(*first), (center(target) - center(*first)) * 0.82, Stroke::new(1.2, color)); }
                }
                if focused.is_some() { painter.arrow(center(source), (center(*first) - center(source)) * 0.8, Stroke::new(2.0, cyan)); }
            }
            if ui.ctx().data(|d| d.get_temp::<bool>(egui::Id::new("tactical_destination_safety"))).unwrap_or(true) {
                let destination = hover.and_then(|pos| {
                    if let Some(first) = hovered_first { return Some((first, first)); }
                    let square = self.board_mark_square(pos, rect)?;
                    if let Some(first) = focused {
                        return routes.iter().find(|(s, _)| *s == first)
                            .filter(|(_, next)| next.iter().any(|(s, _)| *s == square)).map(|_| (first, square));
                    }
                    routes.iter().find(|(_, next)| next.iter().any(|(s, _)| *s == square)).map(|(first, _)| (*first, square))
                }).or_else(|| locked.map(|first| (first, first)));
                if let Some((first, target)) = destination
                    && let Some(safety) = crate::tactical::knight_pin_safety(&self.board, source, first, target) {
                    for (squares, color) in [(&safety.attackers, Color32::from_rgb(240, 100, 90)),
                        (&safety.defenders, Color32::from_gray(170)), (&safety.recapturers, Color32::from_rgb(90, 210, 135)),
                        (&safety.blocked, Color32::from_rgb(230, 178, 65))] {
                        for &square in squares {
                            painter.circle_stroke(center(square), cell * 0.39, Stroke::new(2.2, color));
                            painter.line_segment([center(square), center(target)], Stroke::new(1.8, color));
                        }
                    }
                    ui.interact(egui::Rect::from_center_size(center(target), Vec2::splat(cell)), ui.id().with("knight_safety_tip"), Sense::hover())
                        .on_hover_text(format!("{source} to {first}{}\n{}",
                            if first == target { String::new() } else { format!(" to {target}") },
                            Self::tactical_safety_text(&safety)));
                    painter.text(center(target) + Vec2::new(0.0, cell * 0.36), Align2::CENTER_CENTER,
                        format!("{} G / {} R", safety.defenders.len(), if safety.captures.is_empty() { "-".to_owned() } else { safety.recapturers.len().to_string() }), FontId::proportional((cell * 0.15).clamp(10.0, 15.0)), Color32::WHITE);
                }
            }
            for (first, _) in routes {
                let piece_rect = egui::Rect::from_center_size(center(first), Vec2::splat(cell));
                let mut ghost = ui.new_child(egui::UiBuilder::new().max_rect(piece_rect));
                ghost.set_clip_rect(rect);
                ghost.set_opacity(if focused == Some(first) { 0.8 } else { 0.45 });
                let side = self.board.color_on(source).unwrap();
                if is_3d {
                    if let Some((_, texture, bounds)) = self.knight_3d_ghosts(ui.ctx(), rect, source).iter().find(|(square, _, _)| *square == first) {
                        egui::Image::new((texture.id(), bounds.size())).paint_at(&ghost, *bounds);
                    }
                } else { Self::paint_piece(&ghost, piece_rect, cell, self.piece_set, side, Piece::Knight); }
                if self.board.color_on(first) == Some(!side) {
                    painter.circle_stroke(center(first), cell * 0.42, Stroke::new(2.0, Color32::from_rgb(240, 130, 105)));
                }
            }
        }
        let filters = ui.ctx().data(|d| d.get_temp::<[bool; 3]>(egui::Id::new("tactical_filters"))).unwrap_or([true; 3]);
        let selected = ui.ctx().data(|d| d.get_temp::<String>(egui::Id::new("tactical_selection"))).unwrap_or_default();
        let cell = rect.width() / if self.board_3d_rendering() { 9.0 } else { 8.0 };
        let center = |square| {
            if self.board_3d_rendering() { crate::board3d::square_center(square, rect, self.flipped, self.board_3d_view()) }
            else { Some(rect.left_top() + Self::square_screen_offset(square, self.flipped, cell)) }
        };
        if ui.ctx().data(|d| d.get_temp::<bool>(egui::Id::new("tactical_show_opponent"))).unwrap_or(false)
            && let Some((board, key)) = ui.ctx().data(|d| d.get_temp::<(Board, String)>(egui::Id::new("tactical_opponent_selected")))
            && board == self.board && let Some(threats) = self.tactical_opponent_threats(ui.ctx())
            && let Some(threat) = threats.iter().find(|t| t.key == key) {
            let painter = ui.painter().with_clip_rect(rect);
            let color = Color32::from_rgb(245, 115, 155);
            if let (Some(from), Some(to)) = (center(threat.source), center(threat.destination)) {
                painter.circle_stroke(from, cell * 0.35, Stroke::new(2.5, color));
                painter.circle_stroke(to, cell * 0.39, Stroke::new(2.5, color));
                painter.arrow(from, (to - from) * 0.87, Stroke::new(3.0, color));
            }
            for &(source, target) in &threat.lines {
                if let (Some(from), Some(to)) = (center(source), center(target)) {
                    painter.arrow(from, (to - from) * 0.87, Stroke::new(2.0, Color32::from_rgb(255, 195, 110)));
                    painter.circle_stroke(to, cell * 0.30, Stroke::new(2.0, color));
                }
            }
        }
        if ui.ctx().data(|d| d.get_temp::<bool>(egui::Id::new("tactical_show_changes"))).unwrap_or(true)
            && let Some(changes) = self.tactical_changes(ui.ctx()) {
            let painter = ui.painter().with_clip_rect(rect);
            for change in changes {
                let color = Self::tactical_change_color(change.kind);
                if self.board_3d_rendering() {
                    if let Some(points) = crate::board3d::square_outline(change.target, rect, self.flipped, self.board_3d_view()) {
                        painter.add(egui::Shape::closed_line(points, Stroke::new(2.5, color)));
                    }
                } else if let Some(pos) = center(change.target) {
                    painter.rect_stroke(egui::Rect::from_center_size(pos, Vec2::splat(cell * 0.84)), 4.0,
                        Stroke::new(2.5, color), egui::StrokeKind::Inside);
                }
                for source in change.related {
                    if let (Some(from), Some(to)) = (center(source), center(change.target)) {
                        if change.kind == crate::tactical::ChangeKind::LostDefender {
                            // Previous defender squares may now be empty or occupied by a capturer.
                            painter.circle_stroke(from, cell * 0.16, Stroke::new(1.5, color));
                            for step in (0..12).step_by(2) {
                                painter.line_segment([from.lerp(to, step as f32 / 12.0), from.lerp(to, (step + 1) as f32 / 12.0)], Stroke::new(1.5, color));
                            }
                        } else {
                            painter.arrow(from, (to - from) * 0.87, Stroke::new(if change.kind == crate::tactical::ChangeKind::OpenedLine { 2.5 } else { 1.5 }, color));
                        }
                    }
                }
            }
        }
        for f in self.tactical_findings(ui.ctx()) {
            if !filters[Self::tactical_filter(f.kind)] { continue; }
            let active = f.text == selected;
            let color = Self::tactical_finding_color(&f);
            let squares = if active { f.squares.clone() } else {
                match f.kind { crate::tactical::Kind::Danger => vec![f.squares[0]], crate::tactical::Kind::Pin => vec![f.squares[1]], crate::tactical::Kind::Fork => Vec::new() }
            };
            for square in squares {
                let stroke = Stroke::new(if active { 3.0 } else { 1.5 }, color);
                if self.board_3d_rendering() {
                    if let Some(points) = crate::board3d::square_outline(square, rect, self.flipped, self.board_3d_view()) {
                        ui.painter().add(egui::Shape::closed_line(points, stroke));
                    }
                } else if let Some(pos) = center(square) {
                    ui.painter().rect_stroke(egui::Rect::from_center_size(pos, Vec2::splat(cell * 0.90)), 3.0, stroke, egui::StrokeKind::Inside);
                }
            }
            if active && f.squares.len() > 1 {
                let origin = if f.kind == crate::tactical::Kind::Fork { 1 } else { 0 };
                if let Some(from) = center(f.squares[origin]) {
                    for &target in &f.squares[origin + 1..] {
                        if let Some(to) = center(target) { ui.painter().arrow(from, (to - from) * 0.88, Stroke::new(2.5, color)); }
                    }
                    if origin == 1 && let Some(source) = center(f.squares[0]) {
                        ui.painter().line_segment([source, from], Stroke::new(1.5, color));
                    }
                }
            }
        }
    }
    fn paint_surface_count(&self, ui: &egui::Ui, rect: egui::Rect, square: Square,
        offset: Vec2, text: &str, color: Color32) -> Option<egui::Rect> {
        let project = |point| crate::board3d::square_surface_point(square, point, rect, self.flipped, self.board_3d_view());
        let points = (0..40).map(|i| {
            let angle = i as f32 * std::f32::consts::TAU / 40.0;
            project(offset + Vec2::new(angle.cos(), angle.sin()) * 0.11)
        }).collect::<Option<Vec<_>>>()?;
        let bounds = egui::Rect::from_points(&points);
        let painter = ui.painter().with_clip_rect(rect);
        painter.add(egui::Shape::convex_polygon(points, color, Stroke::new(0.8, Color32::from_white_alpha(180))));
        // Vector digits are projected onto the same plane as the badge. Screen-space
        // text would stay upright and appear to float when the camera rotates.
        let count = text.chars().count().max(1) as f32;
        let advance = (0.18 / count).min(0.074);
        let glyph_width = advance * 0.72;
        let total_width = advance * (count - 1.0) + glyph_width;
        let stroke = Stroke::new((bounds.width().min(bounds.height()) * 0.09).clamp(0.7, 1.6), Color32::WHITE);
        for (index, ch) in text.chars().enumerate() {
            let left = offset.x - total_width * 0.5 + index as f32 * advance;
            let point = |x, y| project(Vec2::new(left + x * glyph_width, offset.y + (y - 1.0) * 0.065));
            let line = |a: (f32, f32), b: (f32, f32)| {
                if let (Some(a), Some(b)) = (point(a.0, a.1), point(b.0, b.1)) { painter.line_segment([a, b], stroke); }
            };
            let mask = match ch { '0'=>63, '1'=>6, '2'=>91, '3'=>79, '4'=>102, '5'=>109,
                '6'=>125, '7'=>7, '8'=>127, '9'=>111, '-'=>64, _=>0 };
            for (bit, a, b) in [(1,(0.0,0.0),(1.0,0.0)), (2,(1.0,0.0),(1.0,1.0)),
                (4,(1.0,1.0),(1.0,2.0)), (8,(0.0,2.0),(1.0,2.0)),
                (16,(0.0,1.0),(0.0,2.0)), (32,(0.0,0.0),(0.0,1.0)), (64,(0.0,1.0),(1.0,1.0))] {
                if mask & bit != 0 { line(a, b); }
            }
            if ch == '/' { line((0.0, 2.0), (1.0, 0.0)); }
        }
        Some(bounds)
    }

    // Paint last on the board layer, above annotations and tactical/engine arrows.
    pub(super) fn paint_tactical_bubbles(&self, ui: &mut egui::Ui, rect: egui::Rect) {
        if !self.tactical_piece_active(ui.ctx()) { return; }
        if ui.ctx().data(|d| d.get_temp::<bool>(egui::Id::new("tactical_attack_defense"))).unwrap_or(false) {
            type Counts = (Board, Vec<(Square, crate::tactical::PinAwareSafety)>);
            let cache_id = egui::Id::new("tactical_pin_counts_cache");
            let cached = ui.ctx().data(|d| d.get_temp::<Counts>(cache_id));
            let counts = if let Some((_, counts)) = cached.filter(|(board, _)| *board == self.board) {
                counts
            } else {
                let counts: Vec<_> = chess::ALL_SQUARES.into_iter().filter_map(|s| crate::tactical::square_pin_safety(&self.board, s).map(|safety| (s, safety))).collect();
                ui.ctx().data_mut(|d| d.insert_temp(cache_id, (self.board, counts.clone())));
                counts
            };
            let cell = rect.width() / 8.0;
            let radius = (cell * 0.13).clamp(5.0, 12.0).min(cell * 0.20);
            let inset = radius + 2.0;
            let painter = ui.painter().with_clip_rect(rect);
            let hovered_3d = self.board_3d_rendering().then(|| ui.ctx().pointer_hover_pos()
                .filter(|_| ui.rect_contains_pointer(rect))).flatten()
                .and_then(|pos| crate::board3d::square_at(pos, rect, self.flipped, self.board_3d_view(), &self.board, &[])
                    .map(|square| (pos, square)));
            for (square, safety) in counts {
                if self.board_3d_rendering() {
                    let mut hover = false;
                    for (text, x, color) in [(safety.attackers.len().to_string(), -0.28, Color32::from_rgb(185,43,43)),
                        (format!("{}/{}", safety.defenders.len(), if safety.captures.is_empty() { "-".to_owned() } else { safety.recapturers.len().to_string() }), 0.28, Color32::from_rgb(30,125,67))] {
                        if let Some(bounds) = self.paint_surface_count(ui, rect, square, Vec2::new(x, 0.28), &text, color) {
                            hover |= ui.rect_contains_pointer(bounds);
                            ui.interact(bounds, ui.id().with(("pin_safety_3d", square.to_index(), x < 0.0)), Sense::hover())
                                .on_hover_text(format!("{square}\n{}", Self::tactical_safety_text(&safety)));
                        }
                    }
                    if !hover { // Inspecting the piece itself keeps the existing detailed tooltip.
                        if let Some((pos, _)) = hovered_3d.filter(|(_, hovered)| *hovered == square) {
                            ui.interact(egui::Rect::from_center_size(pos, Vec2::splat(2.0)), ui.id().with(("pin_piece_3d", square.to_index())), Sense::hover())
                                .on_hover_text(format!("{square}\n{}", Self::tactical_safety_text(&safety)));
                        }
                    }
                    continue;
                }
                let center = rect.min + Self::square_screen_offset(square, self.flipped, cell);
                for (count, x, color) in [(safety.attackers.len().to_string(), -cell * 0.5 + inset, Color32::from_rgb(185,43,43)),
                    (format!("{}/{}", safety.defenders.len(), if safety.captures.is_empty() { "-".to_owned() } else { safety.recapturers.len().to_string() }), cell * 0.5 - inset, Color32::from_rgb(30,125,67))] {
                    let pos = center + Vec2::new(x, -cell * 0.5 + inset);
                    painter.circle_filled(pos, radius, color);
                    painter.circle_stroke(pos, radius, Stroke::new(1.0, Color32::from_white_alpha(180)));
                    painter.text(pos, Align2::CENTER_CENTER, count, FontId::proportional(radius * 0.95), Color32::WHITE);
                }
                ui.interact(egui::Rect::from_center_size(center, Vec2::splat(cell)), ui.id().with(("pin_safety", square.to_index())), Sense::hover())
                    .on_hover_text(format!("{square}\n{}", Self::tactical_safety_text(&safety)));
            }
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn three_d_count_badges_stay_on_their_square_when_camera_changes() {
        let ctx = egui::Context::default();
        let context = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = ChessApp::new(&context);
        app.board_3d_active = true;
        ctx.data_mut(|d| {
            d.insert_temp(egui::Id::new("tactical_enabled"), true);
            d.insert_temp(egui::Id::new("tactical_attack_defense"), true);
        });
        let board = app.board;
        let rect = egui::Rect::from_min_size(egui::pos2(20.0, 20.0), Vec2::new(700.0, 500.0));
        for (flipped, yaw, distance) in [(false, 0.0, 20.0), (true, 0.7, 17.0), (false, -0.9, 25.0)] {
            app.flipped = flipped;
            app.board_3d_yaw = yaw;
            app.board_3d_distance = distance;
            let mut bounds = None;
            let output = crate::app::run_test_ui(&ctx, egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    bounds = app.paint_surface_count(ui, rect, Square::E4, Vec2::new(0.28, 0.28), "2/-", Color32::GREEN);
                    app.paint_tactical_bubbles(ui, rect);
                });
            });
            let bounds = bounds.unwrap();
            assert!(bounds.is_finite() && bounds.width() > 1.0 && bounds.height() > 1.0);
            assert_eq!(crate::board3d::board_square_at(bounds.center(), rect, flipped, app.board_3d_view()), Some(Square::E4));
            assert!(!output.shapes.is_empty());
            assert_eq!(app.board, board);
        }
    }

    #[test]
    fn knight_reach_3d_ghosts_follow_camera_and_lock_without_moving() {
        let ctx = egui::Context::default();
        let context = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = ChessApp::new(&context);
        app.board_3d_active = true;
        app.review_index = Some(0);
        ctx.data_mut(|d| {
            d.insert_temp(egui::Id::new("tactical_enabled"), true);
            d.insert_temp(egui::Id::new("tactical_knight_reach"), true);
            d.insert_temp(egui::Id::new("tactical_knight_source"), Square::G1);
        });
        let rect = egui::Rect::from_min_size(egui::pos2(20.0, 20.0), Vec2::new(700.0, 500.0));
        let original = app.board;
        let mut previous_bounds = None;
        for (flipped, yaw) in [(false, 0.0), (true, 0.6), (false, -0.7)] {
            app.flipped = flipped;
            app.board_3d_yaw = yaw;
            let ghosts = app.knight_3d_ghosts(&ctx, rect, Square::G1);
            assert_eq!(ghosts.len(), 2);
            let cached = app.knight_3d_ghosts(&ctx, rect, Square::G1);
            assert_eq!(ghosts[0].1.id(), cached[0].1.id());
            let bounds = ghosts.iter().find(|(square, _, _)| *square == Square::F3).unwrap().2;
            assert!(bounds.is_finite() && bounds.width() > 1.0);
            assert_eq!(app.tactical_knight_3d_hit(&ctx, bounds.center(), rect), Some(Square::F3));
            if let Some(previous) = previous_bounds { assert_ne!(bounds, previous); }
            previous_bounds = Some(bounds);
            assert!(app.tactical_knight_selection(&ctx, Square::F3));
            assert_eq!(app.board, original);
            let _ = crate::app::run_test_ui(&ctx, egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| app.paint_tactical_map(ui, rect));
            });
        }
    }

    #[test]
    fn live_ai_moves_work_with_map_and_knight_preview_open() {
        for (source, target) in [(Square::E2, Square::E4), (Square::G1, Square::F3)] {
            let context = eframe::CreationContext::_new_kittest(egui::Context::default());
            let mut app = ChessApp::new(&context);
            app.review_index = None;
            app.engine_enabled = true;
            app.player_side = PlayerSide::White;
            context.egui_ctx.data_mut(|d| {
                d.insert_temp(egui::Id::new("tactical_enabled"), true);
                d.insert_temp(egui::Id::new("tactical_knight_reach"), true);
                d.insert_persisted(egui::Id::new("analysis_details_tab"), 2usize);
            });
            let piece = app.board.piece_on(source);
            assert!(!app.select_mobility_piece(&context.egui_ctx, source));
            assert_eq!(context.egui_ctx.data(|d| d.get_temp::<(Board, Square)>(egui::Id::new("tactical_activity_piece"))), Some((app.board, source)));
            app.select(source);
            assert_eq!(app.selected, Some(source));
            assert!(app.legal_targets.contains(&target));
            assert!(!app.select_mobility_piece(&context.egui_ctx, target));
            app.select(target);
            assert_eq!(app.board.piece_on(source), None);
            assert_eq!(app.board.piece_on(target), piece);
            assert_eq!(app.board.side_to_move(), Color::Black);
            assert!(context.egui_ctx.data(|d| d.get_temp::<(Board, Square, Square)>(egui::Id::new("tactical_knight_lock"))).is_none());
        }
    }
    #[test]
    fn knight_mode_selects_either_color_without_moving() {
        let context = eframe::CreationContext::_new_kittest(egui::Context::default());
        let mut app = ChessApp::new(&context);
        app.review_index = Some(0);
        context.egui_ctx.data_mut(|d| {
            d.insert_temp(egui::Id::new("tactical_enabled"), true);
            d.insert_temp(egui::Id::new("tactical_knight_reach"), true);
        });
        for square in [Square::G1, Square::B8] {
            assert!(app.tactical_knight_selection(&context.egui_ctx, square));
            assert_eq!(context.egui_ctx.data(|d| d.get_temp::<Square>(egui::Id::new("tactical_knight_source"))), Some(square));
        }
        let lock_id = egui::Id::new("tactical_knight_lock");
        app.tactical_knight_selection(&context.egui_ctx, Square::A6);
        assert_eq!(context.egui_ctx.data(|d| d.get_temp::<(Board, Square, Square)>(lock_id)), Some((app.board, Square::B8, Square::A6)));
        app.tactical_knight_selection(&context.egui_ctx, Square::A6);
        assert!(context.egui_ctx.data(|d| d.get_temp::<(Board, Square, Square)>(lock_id)).is_none());
        app.tactical_knight_selection(&context.egui_ctx, Square::A6);
        app.tactical_knight_selection(&context.egui_ctx, Square::G1);
        assert!(context.egui_ctx.data(|d| d.get_temp::<(Board, Square, Square)>(lock_id)).is_none());
        assert_eq!(app.board, Board::default());
    }
    #[test]
    fn map_selects_any_piece_for_activity_without_knight_mode() {
        let context = eframe::CreationContext::_new_kittest(egui::Context::default());
        let mut app = ChessApp::new(&context);
        app.review_index = Some(0);
        context.egui_ctx.data_mut(|d| d.insert_temp(egui::Id::new("tactical_enabled"), true));
        for square in [Square::E2, Square::D8, Square::E1] {
            assert!(app.tactical_knight_selection(&context.egui_ctx, square));
            assert_eq!(context.egui_ctx.data(|d| d.get_temp::<(Board, Square)>(egui::Id::new("tactical_activity_piece"))), Some((app.board, square)));
        }
        assert_eq!(app.board, Board::default());
    }
    #[test]
    fn online_play_and_prediction_hide_the_map_and_review_invalidates_cache() {
        let context = eframe::CreationContext::_new_kittest(egui::Context::default());
        let mut app = ChessApp::new(&context);
        assert!(app.tactical_available());
        app.fics_active = true; app.fics_playing = true;
        assert!(!app.tactical_available());
        app.fics_active = false; app.prediction_index = 1;
        assert!(!app.tactical_available());
        app.prediction_index = 0;
        app.board = Board::from_str("k3r3/8/8/8/8/8/4R3/4K3 w - - 0 1").unwrap();
        assert!(app.tactical_findings(&context.egui_ctx).iter().any(|f| f.kind == crate::tactical::Kind::Pin));
        app.board = Board::default();
        assert!(app.tactical_findings(&context.egui_ctx).is_empty());
    }
}
