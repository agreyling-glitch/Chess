use super::*;

impl ChessApp {
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
    pub(super) fn tactical_toggle_ui(&self, ui: &mut egui::Ui) {
        let id = egui::Id::new("tactical_enabled");
        let mut open = ui.ctx().data(|d| d.get_temp::<bool>(id)).unwrap_or(false);
        let response = ui.add_enabled(self.tactical_available(),
            egui::Button::new("").selected(open).min_size(Vec2::new(29.0, 22.0)))
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
        let width = (ctx.screen_rect().width() - 32.0).clamp(360.0, 420.0);
        let frame = Frame::window(&ctx.style())
            .fill(Color32::from_rgba_unmultiplied(17, 23, 33, 235))
            .inner_margin(Margin::same(14))
            .corner_radius(CornerRadius::same(12))
            .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(195, 155, 245, 100)));
        let mut close = false;
        egui::Window::new(RichText::new("Tactical Map").size(18.0).strong().color(Color32::from_rgb(231, 221, 199)))
            .id(egui::Id::new("tactical_window"))
            .open(&mut open)
            .frame(frame)
            .default_pos(egui::pos2(48.0, 100.0))
            .default_width(width)
            .min_width(360.0)
            .resizable(true)
            .collapsible(false)
            .title_bar(false)
            .show(ctx, |ui| self.tactical_map_contents(ui, &mut close));
        if close { open = false; }
        ctx.data_mut(|d| d.insert_temp(id, open));
    }

    fn tactical_map_contents(&self, ui: &mut egui::Ui, close: &mut bool) {
        let muted = Color32::from_rgb(160, 173, 190);
        ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
        let findings = self.tactical_findings(ui.ctx());
        let mut filters = ui.ctx().data(|d| d.get_temp::<[bool; 3]>(egui::Id::new("tactical_filters"))).unwrap_or([true; 3]);
        ui.horizontal(|ui| {
            ui.add(egui::Label::new(RichText::new("Tactical Map").size(16.0).strong()
                .color(Color32::from_rgb(231, 221, 199))).selectable(false).sense(Sense::hover()))
                .on_hover_cursor(egui::CursorIcon::Grab);
            let middle_width = (ui.available_width() - 30.0).max(190.0);
            ui.allocate_ui_with_layout(Vec2::new(middle_width, 26.0), Layout::left_to_right(Align::Center), |ui| {
                ui.add_space(((middle_width - 190.0) * 0.5).max(0.0));
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
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.add(egui::Button::new(RichText::new("×").size(18.0).color(muted)).frame(false))
                    .on_hover_text("Close Tactical Map").clicked() { *close = true; }
            });
        });
        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("tactical_filters"), filters));
        let count_id = egui::Id::new("tactical_attack_defense");
        let mut show_counts = ui.ctx().data(|d| d.get_temp::<bool>(count_id)).unwrap_or(false);
        if ui.checkbox(&mut show_counts, "Attack vs Defense").on_hover_text(
            "Red at top left: enemy attackers. Green at top right: friendly defenders. Counts include pinned pieces; they describe geometric control, not legal captures."
        ).changed() { ui.ctx().request_repaint(); }
        ui.ctx().data_mut(|d| d.insert_temp(count_id, show_counts));
        if show_counts {
            ui.label(RichText::new(if self.board_3d_active {
                "Switch to 2D to see attacker and defender counts."
            } else { "Red left: attackers · Green right: defenders (including pinned pieces)." }).size(12.0).color(muted));
        }
        let selection = egui::Id::new("tactical_selection");
        let mut selected = ui.ctx().data(|d| d.get_temp::<String>(selection)).unwrap_or_default();
        let visible: Vec<_> = findings.iter().filter(|f| filters[Self::tactical_filter(f.kind)]).collect();
        if !visible.iter().any(|f| f.text == selected) { selected.clear(); }
        ui.add_space(3.0);
        egui::ScrollArea::vertical().id_salt("tactical_list").max_height(320.0).auto_shrink([false, true]).show(ui, |ui| {
            for (index, f) in visible.iter().enumerate() {
                let active = selected == f.text;
                let color = Self::tactical_color(f.kind);
                let title = match f.kind {
                    crate::tactical::Kind::Danger => format!("Exposed piece · {}", f.squares[0]),
                    crate::tactical::Kind::Pin => format!("Absolute pin · {}", f.squares[1]),
                    crate::tactical::Kind::Fork => format!("Fork candidate · {} to {}", f.squares[0], f.squares[1]),
                };
                let description = f.text.split_once(": ").map_or(f.text.as_str(), |(_, body)| body)
                    .split(". ").next().unwrap_or(&f.text);
                let card = Frame::new()
                    .fill(if active { color.gamma_multiply(0.12) } else { Color32::from_rgba_unmultiplied(33, 42, 55, 210) })
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
                    });
                let response = ui.interact(card.response.rect, ui.id().with(("tactical_card", index)), egui::Sense::click())
                    .on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(&f.text);
                if response.hovered() {
                    ui.painter().rect_stroke(card.response.rect, 9.0, Stroke::new(1.0, color.gamma_multiply(0.65)), egui::StrokeKind::Inside);
                }
                if response.clicked() { selected = if active { String::new() } else { f.text.clone() }; }
            }
            if visible.is_empty() {
                Frame::new().fill(Color32::from_rgba_unmultiplied(33, 42, 55, 170))
                    .corner_radius(CornerRadius::same(9)).inner_margin(Margin::same(16)).show(ui, |ui| {
                        ui.label(RichText::new("No patterns to show").strong().color(Color32::from_rgb(218, 225, 235)));
                        ui.label(RichText::new("Try another position or enable more filters.").size(12.0).color(muted));
                    });
            }
        });
        ui.ctx().data_mut(|d| d.insert_temp(selection, selected));
    }
    fn tactical_filter(kind: crate::tactical::Kind) -> usize {
        match kind { crate::tactical::Kind::Danger => 0, crate::tactical::Kind::Pin => 1, crate::tactical::Kind::Fork => 2 }
    }
    fn tactical_color(kind: crate::tactical::Kind) -> Color32 {
        match kind {
            crate::tactical::Kind::Danger => Color32::from_rgb(240, 130, 105),
            crate::tactical::Kind::Pin => Color32::from_rgb(195, 155, 245),
            crate::tactical::Kind::Fork => Color32::from_rgb(100, 195, 230),
        }
    }
    pub(super) fn paint_tactical_map(&self, ui: &egui::Ui, rect: egui::Rect) {
        if !self.tactical_available() || !ui.ctx().data(|d| d.get_temp::<bool>(egui::Id::new("tactical_enabled"))).unwrap_or(false) { return; }
        if !self.board_3d_active && ui.ctx().data(|d| d.get_temp::<bool>(egui::Id::new("tactical_attack_defense"))).unwrap_or(false) {
            type Counts = (Board, Vec<(Square, usize, usize)>);
            let cache_id = egui::Id::new("tactical_counts_cache");
            let cached = ui.ctx().data(|d| d.get_temp::<Counts>(cache_id));
            let counts = if let Some((_, counts)) = cached.filter(|(board, _)| *board == self.board) {
                counts
            } else {
                let counts = crate::tactical::attack_defense_counts(&self.board);
                ui.ctx().data_mut(|d| d.insert_temp(cache_id, (self.board, counts.clone())));
                counts
            };
            let cell = rect.width() / 8.0;
            let radius = (cell * 0.13).clamp(5.0, 12.0).min(cell * 0.20);
            let inset = radius + 2.0;
            let painter = ui.painter().with_clip_rect(rect);
            for (square, attackers, defenders) in counts {
                let center = rect.min + Self::square_screen_offset(square, self.flipped, cell);
                for (count, x, color) in [(attackers, -cell * 0.5 + inset, Color32::from_rgb(185,43,43)),
                    (defenders, cell * 0.5 - inset, Color32::from_rgb(30,125,67))] {
                    let pos = center + Vec2::new(x, -cell * 0.5 + inset);
                    painter.circle_filled(pos, radius, color);
                    painter.circle_stroke(pos, radius, Stroke::new(1.0, Color32::from_white_alpha(180)));
                    painter.text(pos, Align2::CENTER_CENTER, count, FontId::proportional(radius * 1.35), Color32::WHITE);
                }
            }
        }
        let filters = ui.ctx().data(|d| d.get_temp::<[bool; 3]>(egui::Id::new("tactical_filters"))).unwrap_or([true; 3]);
        let selected = ui.ctx().data(|d| d.get_temp::<String>(egui::Id::new("tactical_selection"))).unwrap_or_default();
        let cell = rect.width() / if self.board_3d_active { 9.0 } else { 8.0 };
        let center = |square| {
            if self.board_3d_active { crate::board3d::square_center(square, rect, self.flipped, self.board_3d_view()) }
            else { Some(rect.left_top() + Self::square_screen_offset(square, self.flipped, cell)) }
        };
        for f in self.tactical_findings(ui.ctx()) {
            if !filters[Self::tactical_filter(f.kind)] { continue; }
            let active = f.text == selected;
            let color = Self::tactical_color(f.kind);
            let squares = if active { f.squares.clone() } else {
                match f.kind { crate::tactical::Kind::Danger => vec![f.squares[0]], crate::tactical::Kind::Pin => vec![f.squares[1]], crate::tactical::Kind::Fork => Vec::new() }
            };
            for square in squares {
                let stroke = Stroke::new(if active { 3.0 } else { 1.5 }, color);
                if self.board_3d_active {
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
}

#[cfg(test)]
mod tests {
    use super::*;
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
