use super::*;

impl ChessApp {
    pub(super) fn notes_toggle_ui(&self, ui: &mut egui::Ui) {
        let id = egui::Id::new("notes_window_open");
        let mut open = ui.ctx().data(|d| d.get_temp::<bool>(id)).unwrap_or(false);
        let response = ui.add(egui::Button::new("").selected(open).min_size(Vec2::new(29.0, 22.0)))
            .on_hover_text(if open { "Close Notes" } else { "Open Notes" });
        let center = response.rect.center();
        let paper = egui::Rect::from_center_size(center, Vec2::new(11.0, 14.0));
        let stroke = ui.style().interact(&response).fg_stroke;
        ui.painter().rect_stroke(paper, 1.5, stroke, egui::StrokeKind::Inside);
        for offset in [-3.5, 0.0, 3.5] {
            ui.painter().line_segment([center + Vec2::new(-3.0, offset), center + Vec2::new(3.0, offset)], stroke);
        }
        if response.clicked() {
            open = !open;
            ui.ctx().data_mut(|d| d.insert_temp(id, open));
        }
        response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, open, "Notes"));
    }

    fn notes_tabs_ui(&mut self, ui: &mut egui::Ui) {
        let tab_id = egui::Id::new("notes_start_tab");
        let mut start = ui.ctx().data(|d| d.get_temp::<bool>(tab_id)).unwrap_or(false);
        ui.horizontal(|ui| {
            ui.selectable_value(&mut start, false, "Note").on_hover_text("Position Note");
            ui.selectable_value(&mut start, true, "Start").on_hover_text("Starting Note");
        });
        ui.ctx().data_mut(|d| d.insert_temp(tab_id, start));
        ui.separator();
        let Some(position) = self.review_index.or_else(|| self.review_positions.len().checked_sub(1)) else {
            ui.label(RichText::new("Play or open a game to add position notes.").weak());
            return;
        };
        let index = if start { 0 } else { position };
        if !start && let Some(values) = self.move_annotations.get(index).filter(|values| !values.is_empty()) {
            ui.label(values.iter().map(|nag| Self::annotation_symbol(*nag)).collect::<Vec<_>>().join(" "));
        }
        let empty = self.note_at(index).is_empty();
        let text = if empty { "No note for this position.".to_owned() } else { self.note_at(index).to_owned() };
        let mut edit = false;
        egui::ScrollArea::vertical().id_salt(("notes_text", index)).max_height(260.0).show(ui, |ui| {
            let rich = if empty { RichText::new(text).color(Color32::from_rgb(160, 173, 190)) }
                else { RichText::new(text).color(Color32::from_rgb(218, 225, 235)) };
            let response = ui.add(egui::Label::new(rich).wrap().selectable(false).sense(Sense::click()))
                .on_hover_text(if empty { "Double-click to add a note" } else { "Double-click to edit this note" })
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            edit = response.double_clicked();
        });
        if edit { self.edit_note(index); }
        self.all_notes_list(ui);
    }

    fn all_notes_list(&mut self, ui: &mut egui::Ui) {
        let notes: Vec<_> = self.review_positions.iter().enumerate()
            .filter_map(|(index, _)| {
                let text = self.note_at(index);
                (!text.trim().is_empty()).then(|| (index, text.to_owned()))
            }).collect();
        egui::CollapsingHeader::new(format!("All notes ({})", notes.len())).id_salt("all_game_notes")
            .show(ui, |ui| {
                if notes.is_empty() { ui.label(RichText::new("No notes in this game yet.").weak()); }
                egui::ScrollArea::vertical().id_salt("all_notes_scroll").max_height(280.0).show(ui, |ui| {
                    for (index, text) in &notes {
                        let selected = self.review_index.unwrap_or(self.review_moves.len()) == *index;
                        let card = Frame::new().fill(Color32::from_rgba_unmultiplied(33, 42, 55, 190))
                            .corner_radius(CornerRadius::same(8)).inner_margin(Margin::same(10)).show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                let title = if *index == 0 { "Starting note".to_owned() } else { self.graph_position_label(*index, true) };
                                ui.label(RichText::new(title).strong().color(Color32::from_rgb(231, 221, 199)));
                                ui.add(egui::Label::new(RichText::new(text).color(Color32::from_rgb(218, 225, 235))).wrap().selectable(false));
                            });
                        let response = ui.interact(card.response.rect, ui.id().with(("note_entry", index)), Sense::click())
                            .on_hover_text("Click to view this note; double-click to edit")
                            .on_hover_cursor(egui::CursorIcon::PointingHand);
                        if response.hovered() || selected {
                            ui.painter().rect_stroke(card.response.rect, 8.0, Stroke::new(1.0, Color32::from_rgba_unmultiplied(202, 168, 99, 110)), egui::StrokeKind::Inside);
                        }
                        if response.clicked() {
                            ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("notes_start_tab"), *index == 0));
                            if *index != 0 { self.review_to(*index); }
                        }
                        if response.double_clicked() { self.edit_note(*index); }
                    }
                });
            });
    }

    pub(super) fn notes_title(&self, index: usize) -> String {
        if index == 0 { return "Notes: Starting position".to_owned(); }
        self.review_moves.get(index - 1).map_or_else(|| "Notes".to_owned(), |san| {
            let description = Self::review_move_at(&self.review_positions, &self.review_moves, index)
                .and_then(|mv| self.review_positions.get(index - 1).map(|board| Self::move_hover_text(board, mv)));
            description.map_or_else(|| format!("Notes: {san}"), |text| format!("Notes: {san} ({text})"))
        })
    }

    pub(super) fn notes_header(ui: &mut egui::Ui, title: &str) -> bool {
        let mut close = false;
        ui.horizontal(|ui| {
            ui.add(egui::Label::new(RichText::new(title).size(16.0).strong()
                .color(Color32::from_rgb(231, 221, 199))).selectable(false).sense(Sense::hover()))
                .on_hover_cursor(egui::CursorIcon::Grab);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                close = ui.add(egui::Button::new(RichText::new("×").size(18.0)).frame(false))
                    .on_hover_text("Close Notes").clicked();
            });
        });
        close
    }

    pub(super) fn notes_window(&mut self, ctx: &egui::Context) {
        let id = egui::Id::new("notes_window_open");
        let mut open = ctx.data(|d| d.get_temp::<bool>(id)).unwrap_or(false);
        if !open { return; }
        let frame = Frame::window(&ctx.style())
            .fill(Color32::from_rgba_unmultiplied(17, 23, 33, 235))
            .inner_margin(Margin::same(14))
            .corner_radius(CornerRadius::same(12))
            .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(202, 168, 99, 100)));
        let start_tab = ctx.data(|d| d.get_temp::<bool>(egui::Id::new("notes_start_tab"))).unwrap_or(false);
        let title = self.review_index.or_else(|| self.review_positions.len().checked_sub(1))
            .map(|index| if start_tab { 0 } else { index })
            .map(|index| self.notes_title(index)).unwrap_or_else(|| "Notes".to_owned());
        let mut close = false;
        egui::Window::new(&title)
            .id(egui::Id::new("notes_window"))
            .open(&mut open).frame(frame)
            .default_pos(egui::pos2(80.0, 160.0))
            .default_width((ctx.screen_rect().width() - 32.0).clamp(280.0, 420.0))
            .min_width(280.0).resizable(true).collapsible(false).title_bar(false)
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
                close = Self::notes_header(ui, &title);
                self.notes_tabs_ui(ui);
            });
        if close { open = false; }
        ctx.data_mut(|d| d.insert_temp(id, open));
    }
}
