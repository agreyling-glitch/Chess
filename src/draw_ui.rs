use super::*;

impl ChessApp {
    pub(super) fn record_drawing_undo(&mut self, position: usize) {
        self.drawing_undo.push((position, self.board_marks.get(position).cloned().unwrap_or_default()));
        if self.drawing_undo.len() > 128 { self.drawing_undo.remove(0); }
    }
    fn undo_drawing(&mut self, position: usize) {
        if let Some(index) = self.drawing_undo.iter().rposition(|(p, _)| *p == position) {
            let (_, marks) = self.drawing_undo.remove(index);
            self.board_marks.resize(self.review_positions.len(), Vec::new());
            self.board_marks[position] = marks;
            self.board_mark_drag = None;
            self.save_board_marks();
        }
    }
    fn clear_drawings(&mut self, position: usize) {
        if self.board_marks.get(position).is_some_and(|m| !m.is_empty()) {
            self.record_drawing_undo(position);
            self.board_marks[position].clear();
            self.board_mark_drag = None;
            self.save_board_marks();
        }
    }
    fn delete_drawing(&mut self, position: usize, index: usize) {
        if self.board_marks.get(position).is_some_and(|m| index < m.len()) {
            self.record_drawing_undo(position);
            self.board_marks[position].remove(index);
            self.board_mark_drag = None;
            self.save_board_marks();
        }
    }

    pub(super) fn drawing_shape_style(ctx: &egui::Context) -> String {
        ctx.data(|d| d.get_temp::<String>(egui::Id::new("drawing_shape_style"))).unwrap_or_default()
    }

    pub(super) fn shape_points(center: egui::Pos2, size: f32, circle: bool) -> Vec<egui::Pos2> {
        if circle {
            (0..64).map(|i| {
                let angle = i as f32 * std::f32::consts::TAU / 64.0;
                center + Vec2::new(angle.cos(), angle.sin()) * size * 0.43
            }).collect()
        } else {
            let rect = egui::Rect::from_center_size(center, Vec2::splat(size * 0.86));
            vec![rect.left_top(), rect.right_top(), rect.right_bottom(), rect.left_bottom()]
        }
    }

    pub(super) fn paint_shape_outline(painter: &egui::Painter, points: Vec<egui::Pos2>, stroke: Stroke, dotted: bool) {
        if points.len() < 2 { return; }
        if !dotted { painter.add(egui::Shape::closed_line(points, stroke)); return; }
        let mut next = 0.0_f32;
        let spacing = stroke.width * 2.8;
        for i in 0..points.len() {
            let start = points[i];
            let delta = points[(i + 1) % points.len()] - start;
            let length = delta.length();
            if length > 0.0 {
                while next < length {
                    painter.circle_filled(start + delta * (next / length), stroke.width * 0.5, stroke.color);
                    next += spacing;
                }
                next -= length;
            }
        }
    }

    pub(super) fn drawing_arrow_style(ctx: &egui::Context) -> String {
        ctx.data(|d| d.get_temp::<String>(egui::Id::new("drawing_arrow_style"))).unwrap_or_default()
    }

    pub(super) fn paint_styled_arrow(painter: &egui::Painter, source: egui::Pos2, destination: egui::Pos2, cell: f32, color: Color32, style: &str) {
        let direction = (destination - source).normalized();
        let normal = Vec2::new(-direction.y, direction.x);
        let bend = match style { "curve-left" => 0.35, "curve-right" => -0.35, _ => 0.0 };
        let control = source + (destination - source) * 0.5 + normal * (destination - source).length() * bend;
        let tangent = (destination - control).normalized();
        let perpendicular = Vec2::new(-tangent.y, tangent.x);
        let tip = destination - tangent * cell * 0.12;
        let base = tip - tangent * cell * 0.28;
        let points: Vec<_> = (0..=40).map(|i| {
            let t = i as f32 / 40.0;
            if bend == 0.0 { source + (base - source) * t }
            else { egui::pos2((1.0-t).powi(2)*source.x + 2.0*(1.0-t)*t*control.x + t*t*base.x,
                (1.0-t).powi(2)*source.y + 2.0*(1.0-t)*t*control.y + t*t*base.y) }
        }).collect();
        let stroke = Stroke::new((cell * 0.07).clamp(2.0, 7.0), color);
        if style == "dashed" {
            let mut distance = 0.0_f32;
            let dash = (cell * 0.20).max(4.0);
            for pair in points.windows(2) {
                let delta = pair[1] - pair[0];
                let length = delta.length();
                let mut used = 0.0;
                while used < length {
                    let phase = distance % (dash * 1.65);
                    let on = phase < dash;
                    let remaining = if on { dash - phase } else { dash * 1.65 - phase };
                    let step = remaining.max(0.001).min(length - used);
                    if on { painter.line_segment([pair[0] + delta * (used / length), pair[0] + delta * ((used + step) / length)], stroke); }
                    distance += step; used += step;
                }
            }
        } else { painter.add(egui::Shape::line(points, stroke)); }
        painter.add(egui::Shape::convex_polygon(vec![tip, base + perpendicular * cell * 0.14, base - perpendicular * cell * 0.14], color, Stroke::NONE));
    }

    fn replace_drawing(&mut self, position: usize, index: usize, mark: BoardMark) -> Result<(), &'static str> {
        if !mark.valid() { return Err("Choose valid board squares."); }
        let Some(marks) = self.board_marks.get(position) else { return Err("Position unavailable."); };
        if index >= marks.len() { return Err("Drawing unavailable."); }
        let shape = marks[index].from == marks[index].to;
        if shape != (mark.from == mark.to) { return Err("An arrow needs two different squares."); }
        if marks.iter().enumerate().any(|(i, m)| i != index && m.from == mark.from && m.to == mark.to) {
            return Err("A drawing already uses those squares.");
        }
        if marks[index] != mark {
            self.record_drawing_undo(position);
            self.board_marks[position][index] = mark;
            self.board_mark_drag = None;
            self.save_board_marks();
        }
        Ok(())
    }

    fn drawing_square_picker(ui: &mut egui::Ui, id: &str, value: &mut String, excluded: Option<&str>) -> bool {
        let before = value.clone();
        egui::ComboBox::from_id_salt(id).width(48.0).height(220.0).selected_text(value.to_ascii_uppercase()).show_ui(ui, |ui| {
            for square in chess::ALL_SQUARES {
                let text = square.to_string();
                if excluded == Some(text.as_str()) { continue; }
                ui.selectable_value(value, text.clone(), text.to_ascii_uppercase());
            }
        });
        *value != before
    }

    fn drawn_shapes_list(&mut self, ui: &mut egui::Ui) {
        let position = self.review_index.unwrap_or(self.review_moves.len());
        let marks = self.board_marks.get(position).cloned().unwrap_or_default();
        let mut delete_index = None;
        egui::CollapsingHeader::new(format!("Drawings ({})", marks.len())).id_salt("drawn_shapes_list").show(ui, |ui| {
            if marks.is_empty() { ui.label(RichText::new("No drawings for this position.").weak()); }
            // The window's outer scroll area gives content an unbounded layout
            // height. Use its visible viewport to size this final list instead.
            let list_height = (ui.clip_rect().bottom() - ui.cursor().top()).max(0.0);
            egui::ScrollArea::vertical().id_salt("drawn_shapes_scroll")
                .max_height(list_height).min_scrolled_height(0.0).auto_shrink([false, false])
                .show(ui, |ui| {
                for (index, original) in marks.iter().enumerate() {
                    ui.push_id((position, index), |ui| {
                        let mut mark = original.clone();
                        let shape = mark.from == mark.to;
                        let name = if shape {
                            match mark.style.as_str() { "circle" => "Circle", "dotted-circle" => "Dotted circle", "dotted-square" => "Dotted square", _ => "Square" }
                        } else {
                            match mark.style.as_str() { "dashed" => "Dashed arrow", "curve-left" => "Curve left", "curve-right" => "Curve right", _ => "Arrow" }
                        };
                        Frame::new().fill(Color32::from_rgba_unmultiplied(33, 42, 55, 190))
                            .corner_radius(CornerRadius::same(8)).inner_margin(Margin::same(8)).show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(format!("{} · {name}", index + 1)).strong().color(Self::board_mark_color(mark.color)));
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        if ui.small_button("Delete").on_hover_text("Delete this drawing").clicked() {
                                            delete_index = Some(index);
                                        }
                                    });
                                });
                                ui.horizontal_wrapped(|ui| {
                                    let color_name = |c| match c { 'R' => "Red", 'Y' => "Yellow", 'B' => "Blue", _ => "Green" };
                                    egui::ComboBox::from_id_salt("color").width(64.0).selected_text(RichText::new(color_name(mark.color)).color(Self::board_mark_color(mark.color)))
                                        .show_ui(ui, |ui| {
                                            for color in ['G', 'R', 'Y', 'B'] {
                                                ui.selectable_value(&mut mark.color, color, RichText::new(color_name(color)).color(Self::board_mark_color(color)));
                                            }
                                        });
                                    ui.label(if shape { "Square" } else { "Start" });
                                    let end = mark.to.clone();
                                    if Self::drawing_square_picker(ui, "start", &mut mark.from, if shape { None } else { Some(&end) }) && shape {
                                        mark.to = mark.from.clone();
                                    }
                                    if !shape {
                                        ui.label("End");
                                        Self::drawing_square_picker(ui, "end", &mut mark.to, Some(&mark.from));
                                    }
                                });
                                if mark != *original {
                                    if let Err(error) = self.replace_drawing(position, index, mark) {
                                        ui.ctx().data_mut(|d| d.insert_temp(ui.id().with("edit_error"), error.to_owned()));
                                    } else { ui.ctx().data_mut(|d| d.remove::<String>(ui.id().with("edit_error"))); }
                                }
                                if let Some(error) = ui.ctx().data(|d| d.get_temp::<String>(ui.id().with("edit_error"))) {
                                    ui.label(RichText::new(error).small().color(Color32::from_rgb(240, 130, 105)));
                                }
                            });
                    });
                }
            });
        });
        if let Some(index) = delete_index { self.delete_drawing(position, index); }
    }

    pub(super) fn draw_toggle_ui(&mut self, ui: &mut egui::Ui) {
        let id = egui::Id::new("draw_window_open");
        let mut open = ui.ctx().data(|d| d.get_temp::<bool>(id)).unwrap_or(false);
        let response = ui.add(egui::Button::new("").selected(open).min_size(Vec2::new(29.0, 22.0)))
            .on_hover_text(if open { "Close drawing tools" } else { "Draw on board" });
        let center = response.rect.center();
        let stroke = ui.style().interact(&response).fg_stroke;
        let a = center + Vec2::new(-5.0, 5.0);
        let b = center + Vec2::new(5.0, -5.0);
        let side = Vec2::new(1.5, 1.5);
        ui.painter().add(egui::Shape::closed_line(vec![a, b - side, b + side, a + side * 2.0], stroke));
        ui.painter().line_segment([center + Vec2::new(-6.0, 7.0), center + Vec2::new(6.0, 7.0)], stroke);
        if response.clicked() {
            open = !open;
            self.board_mark_mode = open;
            self.board_mark_drag = None;
            self.selected = None;
            self.legal_targets.clear();
            ui.ctx().data_mut(|d| d.insert_temp(id, open));
        }
        response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, open, "Draw on board"));
    }

    pub(super) fn draw_window(&mut self, ctx: &egui::Context) {
        let id = egui::Id::new("draw_window_open");
        let mut open = ctx.data(|d| d.get_temp::<bool>(id)).unwrap_or(false);
        if !open { return; }
        let position = self.review_index.unwrap_or(self.review_moves.len());
        if !ctx.wants_keyboard_input() && ctx.input_mut(|input| input.consume_key(egui::Modifiers::CTRL, egui::Key::Z)) {
            self.undo_drawing(position);
        }
        let frame = Frame::window(&ctx.style())
            .fill(Color32::from_rgba_unmultiplied(17, 23, 33, 235))
            .inner_margin(Margin::same(14)).corner_radius(CornerRadius::same(12))
            .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(100, 195, 230, 100)));
        let mut close = false;
        egui::Window::new("Draw on board").id(egui::Id::new("draw_window"))
            .open(&mut open).frame(frame).default_pos(egui::pos2(112.0, 220.0))
            .default_width(320.0).default_height(280.0)
            .min_width(320.0).max_width(320.0).min_height(120.0)
            .vscroll(true).resizable([false, true]).collapsible(false).title_bar(false)
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(8.0, 10.0);
                ui.horizontal(|ui| {
                    // Leave pointer interaction to the floating window's move handler.
                    ui.add(egui::Label::new(RichText::new("Draw on board").size(16.0).strong()
                        .color(Color32::from_rgb(231, 221, 199))).selectable(false).sense(Sense::hover()))
                        .on_hover_cursor(egui::CursorIcon::Grab);
                    let (switch_rect, switch) = ui.allocate_exact_size(Vec2::new(38.0, 22.0), Sense::click());
                    if switch.clicked() {
                        self.board_mark_mode = !self.board_mark_mode;
                        self.board_mark_drag = None;
                        self.selected = None;
                        self.legal_targets.clear();
                    }
                    let enabled = self.board_mark_mode;
                    let switch = switch.on_hover_text(if enabled { "Drawing Enabled" } else { "Drawing disabled" })
                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                    switch.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, enabled, "Drawing enabled"));
                    let amount = ui.ctx().animate_bool(switch.id, enabled);
                    let track = switch_rect.shrink2(Vec2::new(0.0, 2.0));
                    ui.painter().rect_filled(track, 9.0, if enabled { Color32::from_rgb(74, 151, 111) } else { Color32::from_rgb(66, 75, 90) });
                    ui.painter().circle_filled(egui::pos2(track.left() + 9.0 + amount * (track.width() - 18.0), track.center().y),
                        7.0, Color32::from_rgb(237, 241, 246));
                    let index = self.review_index.unwrap_or(self.review_moves.len());
                    if ui.add_enabled(self.board_marks.get(index).is_some_and(|m| !m.is_empty()), egui::Button::new("Clear"))
                        .on_hover_text("Clear drawings for this position").clicked() {
                        self.clear_drawings(index);
                    }
                    let undo = ui.add_enabled(self.drawing_undo.iter().any(|(p, _)| *p == index),
                        egui::Button::new("").min_size(Vec2::new(29.0, 22.0))).on_hover_text("Undo drawing change (Ctrl+Z)");
                    let c = undo.rect.center();
                    let stroke = ui.style().interact(&undo).fg_stroke;
                    ui.painter().add(egui::Shape::line(vec![c + Vec2::new(4.0, 5.0), c + Vec2::new(6.0, 1.0), c + Vec2::new(4.0, -4.0), c + Vec2::new(-5.0, -4.0)], stroke));
                    ui.painter().line_segment([c + Vec2::new(-1.0, -8.0), c + Vec2::new(-5.0, -4.0)], stroke);
                    ui.painter().line_segment([c + Vec2::new(-1.0, 0.0), c + Vec2::new(-5.0, -4.0)], stroke);
                    undo.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Undo drawing"));
                    if undo.clicked() { self.undo_drawing(index); }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        close = ui.add(egui::Button::new(RichText::new("×").size(18.0)).frame(false)).on_hover_text("Close drawing tools").clicked();
                    });
                });
                ui.horizontal(|ui| {
                    ui.add_space(((ui.available_width() - 240.0) * 0.5).max(0.0));
                    for (color, name) in [('G', "Green"), ('R', "Red"), ('Y', "Yellow"), ('B', "Blue")] {
                        let response = ui.add(egui::Button::new("").selected(self.board_mark_color == color).min_size(Vec2::new(54.0, 42.0)))
                            .on_hover_text(name);
                        ui.painter().circle_filled(response.rect.center(), 10.0, Self::board_mark_color(color));
                        if self.board_mark_color == color { ui.painter().circle_stroke(response.rect.center(), 13.0, Stroke::new(1.5, Color32::WHITE)); }
                        if response.clicked() { self.board_mark_color = color; self.board_mark_drag = None; }
                        response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, self.board_mark_color == color, name));
                    }
                });
                let tool_id = egui::Id::new("draw_tool_arrow");
                let mut arrow = ctx.data(|d| d.get_temp::<bool>(tool_id)).unwrap_or(false);
                let mut shape_style = Self::drawing_shape_style(ctx);
                ui.horizontal(|ui| {
                    ui.add_space(((ui.available_width() - 240.0) * 0.5).max(0.0));
                    for (value, label) in [("", "Solid square"), ("dotted-square", "Dotted square"), ("circle", "Solid circle"), ("dotted-circle", "Dotted circle")] {
                        let response = ui.add(egui::Button::new("").selected(!arrow && shape_style == value).min_size(Vec2::new(54.0, 42.0))).on_hover_text(label);
                        let points = Self::shape_points(response.rect.center(), 26.0, value.contains("circle"));
                        Self::paint_shape_outline(ui.painter(), points, Stroke::new(2.0, Self::board_mark_color(self.board_mark_color)), value.starts_with("dotted"));
                        if response.clicked() { shape_style = value.to_owned(); arrow = false; self.board_mark_mode = true; self.board_mark_drag = None; }
                        response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, !arrow && shape_style == value, label));
                    }
                });
                ctx.data_mut(|d| d.insert_temp(egui::Id::new("drawing_shape_style"), shape_style));
                let mut style = Self::drawing_arrow_style(ctx);
                ui.horizontal(|ui| {
                    ui.add_space(((ui.available_width() - 240.0) * 0.5).max(0.0));
                    for (value, label) in [("", "Solid"), ("dashed", "Dashed"), ("curve-left", "Curve left"), ("curve-right", "Curve right")] {
                        let response = ui.add(egui::Button::new("").selected(arrow && style == value).min_size(Vec2::new(54.0, 42.0))).on_hover_text(label);
                        let rect = response.rect.shrink(8.0);
                        Self::paint_styled_arrow(ui.painter(), rect.left_bottom(), rect.right_top(), 22.0, Self::board_mark_color(self.board_mark_color), value);
                        if response.clicked() { style = value.to_owned(); arrow = true; self.board_mark_mode = true; self.board_mark_drag = None; }
                        response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, arrow && style == value, label));
                    }
                });
                ctx.data_mut(|d| d.insert_temp(egui::Id::new("drawing_arrow_style"), style));
                ctx.data_mut(|d| d.insert_temp(tool_id, arrow));
                self.drawn_shapes_list(ui);
            });
        if close { open = false; }
        if !open { self.board_mark_mode = false; self.board_mark_drag = None; }
        ctx.data_mut(|d| d.insert_temp(id, open));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn undo_restores_edits_deletion_clear_and_keeps_positions_separate() {
        let context = eframe::CreationContext::_new_kittest(egui::Context::default());
        let mut app = ChessApp::new(&context);
        app.load_pgn("1. e4 *");
        let mark = BoardMark { color: 'G', from: "e2".into(), to: "e4".into(), style: "dashed".into() };
        app.toggle_board_mark(1, mark.clone());
        app.toggle_board_mark(0, mark.clone());
        app.replace_drawing(1, 0, BoardMark { color: 'R', ..mark.clone() }).unwrap();
        app.undo_drawing(1);
        assert_eq!(app.board_marks[1], vec![mark.clone()]);
        app.delete_drawing(1, 0);
        app.undo_drawing(1);
        assert_eq!(app.board_marks[1], vec![mark.clone()]);
        app.clear_drawings(1);
        app.undo_drawing(1);
        assert_eq!(app.board_marks[1], vec![mark.clone()]);
        app.toggle_board_mark(1, mark.clone());
        app.undo_drawing(1);
        assert_eq!(app.board_marks[1], vec![mark.clone()]);
        app.undo_drawing(1);
        assert!(app.board_marks[1].is_empty());
        assert_eq!(app.board_marks[0], vec![mark]);
        app.load_pgn("1. d4 *");
        assert!(app.drawing_undo.is_empty());
        app.undo_drawing(0);
        assert!(app.board_marks[0].is_empty());
    }

    #[test]
    fn right_click_reverses_curve_during_drag_and_release_saves_it() {
        let ctx = egui::Context::default();
        let context = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = ChessApp::new(&context);
        app.load_pgn("1. e4 *");
        app.review_to(0);
        app.board_mark_mode = true;
        ctx.data_mut(|d| {
            d.insert_temp(egui::Id::new("draw_window_open"), true);
            d.insert_temp(egui::Id::new("draw_tool_arrow"), true);
            d.insert_temp(egui::Id::new("drawing_arrow_style"), "curve-left".to_owned());
        });
        let rect = egui::Rect::from_min_size(egui::pos2(20.0, 20.0), Vec2::splat(400.0));
        let start = rect.left_top() + ChessApp::square_screen_offset(Square::E2, false, 50.0);
        let end = rect.left_top() + ChessApp::square_screen_offset(Square::E4, false, 50.0);
        for (pos, button, pressed) in [(start, egui::PointerButton::Primary, true),
            (end, egui::PointerButton::Secondary, true), (end, egui::PointerButton::Secondary, false),
            (end, egui::PointerButton::Primary, false)] {
            let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::splat(500.0))),
                events: vec![egui::Event::PointerMoved(pos), egui::Event::PointerButton { pos, button, pressed, modifiers: egui::Modifiers::default() }], ..Default::default() };
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| app.board_mark_input(ui, rect));
            });
            if button == egui::PointerButton::Secondary {
                assert_eq!(ChessApp::drawing_arrow_style(&ctx), "curve-right");
                assert!(app.board_mark_drag.is_some());
                assert!(app.board_marks.iter().all(|marks| marks.is_empty()));
            }
        }
        assert_eq!(app.board_marks[0][0].style, "curve-right");
        assert_eq!(app.board_marks[0][0].from, "e2");
        assert_eq!(app.board_marks[0][0].to, "e4");
    }

    #[test]
    fn right_click_switches_solid_and_dashed_during_drag() {
        let ctx = egui::Context::default();
        let context = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = ChessApp::new(&context);
        app.load_pgn("1. e4 *");
        app.review_to(0);
        app.board_mark_mode = true;
        ctx.data_mut(|d| {
            d.insert_temp(egui::Id::new("draw_window_open"), true);
            d.insert_temp(egui::Id::new("draw_tool_arrow"), true);
            d.insert_temp(egui::Id::new("drawing_arrow_style"), "".to_owned());
        });
        let rect = egui::Rect::from_min_size(egui::pos2(20.0, 20.0), Vec2::splat(400.0));
        let start = rect.left_top() + ChessApp::square_screen_offset(Square::E2, false, 50.0);
        let end = rect.left_top() + ChessApp::square_screen_offset(Square::E4, false, 50.0);
        for (pos, button, pressed) in [(start, egui::PointerButton::Primary, true),
            (end, egui::PointerButton::Secondary, true), (end, egui::PointerButton::Secondary, false),
            (end, egui::PointerButton::Primary, false)] {
            let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::splat(500.0))),
                events: vec![egui::Event::PointerMoved(pos), egui::Event::PointerButton { pos, button, pressed, modifiers: egui::Modifiers::default() }], ..Default::default() };
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| app.board_mark_input(ui, rect));
            });
            if button == egui::PointerButton::Secondary {
                assert_eq!(ChessApp::drawing_arrow_style(&ctx), "dashed");
                assert!(app.board_mark_drag.is_some());
                assert!(app.board_marks.iter().all(|marks| marks.is_empty()));
            }
        }
        assert_eq!(app.board_marks[0][0].style, "dashed");
        assert_eq!(app.board_marks[0][0].from, "e2");
        assert_eq!(app.board_marks[0][0].to, "e4");
    }

    #[test]
    fn drawing_edits_preserve_style_and_reject_collisions_or_invalid_endpoints() {
        let context = eframe::CreationContext::_new_kittest(egui::Context::default());
        let mut app = ChessApp::new(&context);
        app.load_pgn("1. e4 *");
        let arrow = BoardMark { color: 'G', from: "e2".into(), to: "e4".into(), style: "dashed".into() };
        let circle = BoardMark { color: 'B', from: "c3".into(), to: "c3".into(), style: "dotted-circle".into() };
        app.toggle_board_mark(1, arrow.clone()); app.toggle_board_mark(1, circle.clone());
        let edited = BoardMark { color: 'R', from: "d2".into(), to: "d4".into(), ..arrow };
        assert!(app.replace_drawing(1, 0, edited.clone()).is_ok());
        let moved_circle = BoardMark { color: 'Y', from: "f6".into(), to: "f6".into(), ..circle };
        assert!(app.replace_drawing(1, 1, moved_circle.clone()).is_ok());
        assert_eq!(app.board_marks[1], vec![edited.clone(), moved_circle]);
        let before = app.board_marks[1].clone();
        assert!(app.replace_drawing(1, 0, BoardMark { to: "d2".into(), ..edited.clone() }).is_err());
        assert!(app.replace_drawing(1, 0, BoardMark { from: "z9".into(), ..edited.clone() }).is_err());
        app.toggle_board_mark(1, BoardMark { color: 'B', from: "h2".into(), to: "h4".into(), style: String::new() });
        assert!(app.replace_drawing(1, 0, BoardMark { from: "h2".into(), to: "h4".into(), ..edited }).is_err());
        assert_eq!(app.board_marks[1][..2], before);
        let pgn = format!("1. e4 {} *", ChessApp::pgn_board_marks(Some(&app.board_marks[1])));
        let restored = ChessApp::parse_pgn_board_marks(&pgn, &app.review_positions);
        assert_eq!(restored[1].len(), app.board_marks[1].len());
        assert!(app.board_marks[1].iter().all(|mark| restored[1].contains(mark)));
    }

    #[test]
    fn arrow_styles_roundtrip_and_replace_a_style_on_the_same_route() {
        let context = eframe::CreationContext::_new_kittest(egui::Context::default());
        let mut app = ChessApp::new(&context);
        app.load_pgn("1. e4 *");
        for style in ["dashed", "curve-left", "curve-right"] {
            let mark = BoardMark { color: 'B', from: "e2".into(), to: "e4".into(), style: style.into() };
            app.toggle_board_mark(1, mark.clone());
            assert_eq!(app.board_marks[1], vec![mark.clone()]);
            let json = serde_json::to_string(&mark).unwrap();
            assert_eq!(serde_json::from_str::<BoardMark>(&json).unwrap(), mark);
            let pgn = format!("1. e4 {} *", ChessApp::pgn_board_marks(Some(&vec![mark.clone()])));
            assert_eq!(ChessApp::parse_pgn_board_marks(&pgn, &app.review_positions)[1], vec![mark.clone()]);
        }
        let legacy: BoardMark = serde_json::from_str(r#"{"color":"G","from":"e2","to":"e4"}"#).unwrap();
        assert!(legacy.style.is_empty());
        let mark = app.board_marks[1][0].clone();
        app.toggle_board_mark(1, mark);
        assert!(app.board_marks[1].is_empty());
    }

    #[test]
    fn shape_styles_roundtrip_and_project_inside_their_square() {
        let context = eframe::CreationContext::_new_kittest(egui::Context::default());
        let mut app = ChessApp::new(&context);
        app.load_pgn("1. e4 *");
        for style in ["", "circle", "dotted-square", "dotted-circle"] {
            let mark = BoardMark { color: 'Y', from: "e4".into(), to: "e4".into(), style: style.into() };
            app.toggle_board_mark(1, mark.clone());
            assert_eq!(app.board_marks[1], vec![mark.clone()]);
            assert_eq!(serde_json::from_str::<BoardMark>(&serde_json::to_string(&mark).unwrap()).unwrap(), mark);
            let pgn = format!("1. e4 {} *", ChessApp::pgn_board_marks(Some(&vec![mark.clone()])));
            assert_eq!(ChessApp::parse_pgn_board_marks(&pgn, &app.review_positions)[1], vec![mark]);
        }
        let rect = egui::Rect::from_min_size(egui::pos2(20.0, 30.0), Vec2::new(1200.0, 900.0));
        for flipped in [false, true] {
            for distance in [14.0, 20.0, 26.0] {
                for yaw in [0.0, 0.8] {
                    let view = crate::board3d::View { distance, yaw, ..Default::default() };
                    let points = crate::board3d::circle_outline(Square::E4, rect, flipped, view).unwrap();
                    assert_eq!(points.len(), 64);
                    assert!(points.into_iter().all(|p| crate::board3d::board_square_at(p, rect, flipped, view) == Some(Square::E4)));
                }
            }
        }
    }

    #[test]
    fn selected_tools_create_highlights_or_arrows_without_moving_pieces() {
        let context = eframe::CreationContext::_new_kittest(egui::Context::default());
        let mut app = ChessApp::new(&context);
        app.load_pgn("1. e4 *"); app.review_to(0);
        let original = app.board;
        app.board_mark_mode = true;
        context.egui_ctx.data_mut(|d| d.insert_temp(egui::Id::new("draw_window_open"), true));
        let rect = egui::Rect::from_min_size(egui::pos2(10.0, 10.0), Vec2::splat(400.0));
        for arrow in [false, true] {
            context.egui_ctx.data_mut(|d| d.insert_temp(egui::Id::new("draw_tool_arrow"), arrow));
            for (square, pressed) in [(Square::E2, true), (Square::E4, false)] {
                let pos = rect.min + ChessApp::square_screen_offset(square, false, 50.0);
                let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::splat(500.0))),
                    events: vec![egui::Event::PointerMoved(pos), egui::Event::PointerButton {pos, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::default()}], ..Default::default() };
                let _ = context.egui_ctx.run(input, |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| app.board_mark_input(ui, rect));
                });
            }
        }
        assert!(app.board_marks[0].iter().any(|m| m.from == "e2" && m.to == "e2"));
        assert!(app.board_marks[0].iter().any(|m| m.from == "e2" && m.to == "e4"));
        assert_eq!(app.board, original);
    }
}
