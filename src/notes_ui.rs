use super::*;

impl ChessApp {
    fn notes_tabs_ui(&mut self, ui: &mut egui::Ui) {
        if let Some(index) = self.note_editor_index {
            if index >= self.review_positions.len() { self.note_editor_index = None; }
            else {
                ui.label(RichText::new(self.notes_title(index)).strong());
                ui.add(egui::TextEdit::multiline(&mut self.note_editor_text)
                    .desired_width(f32::INFINITY).desired_rows(8)
                    .font(FontId::proportional(17.0))
                    .hint_text("What did you notice about this position?"));
                ui.horizontal_wrapped(|ui| {
                    if ui.button("Save note").clicked() {
                        self.set_move_note(index, self.note_editor_text.clone());
                        self.note_editor_index = None;
                    }
                    if ui.button("Cancel").clicked() { self.note_editor_index = None; }
                    if ui.add_enabled(!self.note_at(index).is_empty(), egui::Button::new("Delete note")).clicked() {
                        self.set_move_note(index, String::new());
                        self.note_editor_index = None;
                    }
                });
                return;
            }
        }
        let tab_id = egui::Id::new("notes_start_tab");
        let mut start = ui.ctx().data(|d| d.get_temp::<bool>(tab_id)).unwrap_or(false);
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(&mut start, false, "Position note");
            ui.selectable_value(&mut start, true, "Starting note");
        });
        ui.ctx().data_mut(|d| d.insert_temp(tab_id, start));
        ui.add_space(8.0);
        let Some(position) = self.review_index.or_else(|| self.review_positions.len().checked_sub(1)) else {
            ui.label(RichText::new("Play or open a game to add position notes.").weak());
            return;
        };
        let index = if start { 0 } else { position };
        if !start && let Some(values) = self.move_annotations.get(index).filter(|values| !values.is_empty()) {
            ui.label(values.iter().map(|nag| crate::nags::label(*nag)).collect::<Vec<_>>().join(" "));
        }
        let empty = self.note_at(index).is_empty();
        let text = if empty { "No note for this position.".to_owned() } else { self.note_at(index).to_owned() };
        let mut edit = false;
        Self::notes_section(ui, |ui| {
        ui.label(RichText::new(if start { "STARTING POSITION" } else { "POSITION NOTE" }).size(11.0).strong().color(Color32::from_rgb(211,173,98)));
        egui::ScrollArea::vertical().id_salt(("notes_text", index)).max_height(260.0).show(ui, |ui| {
            let rich = if empty { RichText::new(text).color(Color32::from_rgb(160, 173, 190)) }
                else { RichText::new(text).color(Color32::from_rgb(218, 225, 235)) };
            let response = ui.add(egui::Label::new(rich).wrap().selectable(false).sense(Sense::click()))
                .on_hover_text(if empty { "Double-click to add a note" } else { "Double-click to edit this note" })
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            edit = response.double_clicked();
        });
        ui.add_space(8.0);
        edit |= ui.add(egui::Button::new(RichText::new(if empty { "Add note" } else { "Edit note" })
            .color(Color32::from_rgb(25,29,35))).raised_gold()).clicked();
        });
        if edit { self.edit_note(index, ui.ctx()); }
        ui.add_space(8.0);
        Self::notes_section(ui, |ui| self.all_notes_list(ui));
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
                        let card = Frame::new().fill(Color32::from_rgb(25,31,37))
                            .corner_radius(CornerRadius::same(8)).inner_margin(Margin::same(10)).show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                let title = if *index == 0 { "Starting note".to_owned() } else { self.graph_position_label(*index, true) };
                                ui.label(RichText::new(title).strong().color(Color32::from_rgb(225,190,117)));
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
                        if response.double_clicked() { self.edit_note(*index, ui.ctx()); }
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

    pub(super) fn notes_section<R>(ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui) -> R) -> R {
        let section = Frame::new().fill(Color32::from_rgb(13,17,22))
            .stroke(Stroke::new(1.0, Color32::from_white_alpha(25)))
            .corner_radius(6.0).inner_margin(12.0).show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                content(ui)
            });
        Self::paint_sunken_border(ui, section.response.rect);
        section.inner
    }

    pub(super) fn annotations_panel(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.label(RichText::new("Annotations").size(19.0).strong());
        ui.separator();
        let content_height = (ui.available_height() - 24.0).max(0.0);
        Self::notes_section(ui, |ui| {
            ui.set_min_height(content_height);
            let tab_id = egui::Id::new("annotations_tab");
            let mut tab = ui.ctx().data(|d| d.get_temp::<usize>(tab_id)).unwrap_or(0);
            let previous = tab;
            let background = ui.painter().add(egui::Shape::Noop);
            let tabs = ui.horizontal_wrapped(|ui| {
                ui.selectable_value(&mut tab, 0, "Drawings");
                ui.selectable_value(&mut tab, 1, "Notes");
                ui.selectable_value(&mut tab, 2, "Chess Symbols");
            });
            ui.painter().set(background, Self::glass_surface(tabs.response.rect.expand(2.0)));
            ui.ctx().data_mut(|d| d.insert_temp(tab_id, tab));
            if tab != previous {
                self.board_mark_mode = tab == 0;
                self.board_mark_drag = None;
                self.selected = None;
                self.legal_targets.clear();
            }
            ui.separator();
            egui::ScrollArea::vertical().id_salt(("annotations_content", tab))
                .max_height(ui.available_height().max(0.0))
                .auto_shrink([false, false]).show_gold(ui, |ui| {
                    if tab == 0 { self.drawings_contents(ui); }
                    else if tab == 1 { self.notes_tabs_ui(ui); }
                    else {
                        if let Some(index) = self.review_index.or_else(|| self.review_positions.len().checked_sub(1)) {
                            self.chess_symbols_ui(ui, index, false);
                        } else {
                            ui.label("Play or open a game to add chess symbols.");
                        }
                    }
                });
        });
    }
}
