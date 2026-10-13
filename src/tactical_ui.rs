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
        egui::Window::new(RichText::new("Tactical Map").size(18.0).strong().color(Color32::from_rgb(231, 221, 199)))
            .id(egui::Id::new("tactical_window"))
            .open(&mut open)
            .frame(frame)
            .default_pos(egui::pos2(48.0, 100.0))
            .default_width(width)
            .min_width(360.0)
            .resizable(true)
            .collapsible(true)
            .title_bar(true)
            .show(ctx, |ui| self.tactical_map_contents(ui));
        ctx.data_mut(|d| d.insert_temp(id, open));
    }

    fn tactical_map_contents(&self, ui: &mut egui::Ui) {
        let muted = Color32::from_rgb(160, 173, 190);
        ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
        let findings = self.tactical_findings(ui.ctx());
        let mut filters = ui.ctx().data(|d| d.get_temp::<[bool; 3]>(egui::Id::new("tactical_filters"))).unwrap_or([true; 3]);
        let count_id = egui::Id::new("tactical_attack_defense");
        let mut show_counts = ui.ctx().data(|d| d.get_temp::<bool>(count_id)).unwrap_or(false);
        if ui.checkbox(&mut show_counts, "Attack vs Defense").on_hover_text(
            "Red: geometric attackers. Green: geometric defenders/legal recapturers. Hover a piece for capture-specific legal recaptures. A dash means no legal capture to recapture."
        ).changed() { ui.ctx().request_repaint(); }
        ui.ctx().data_mut(|d| d.insert_temp(count_id, show_counts));
        if show_counts {
            if self.board_3d_rendering() {
                ui.label(RichText::new("Switch to 2D to see attacker and defender counts.").size(12.0).color(muted));
            } else {
                ui.label(RichText::new("Red: geometric attackers. Green: geometric defenders / legal recapturers.").size(12.0).color(muted));
                ui.label(RichText::new("2/-: no legal enemy capture, so recaptures do not apply.\n2/0: no defender can legally recapture. 2/1: one can.\nHover a piece for capture-specific details.").size(11.0).color(muted));
            }
        }
        let threats_id = egui::Id::new("tactical_show_opponent");
        let mut show_threats = ui.ctx().data(|d| d.get_temp::<bool>(threats_id)).unwrap_or(false);
        ui.checkbox(&mut show_threats, "Opponent threats");
        ui.ctx().data_mut(|d| d.insert_temp(threats_id, show_threats));
        if show_threats {
            ui.label(RichText::new(format!("{} candidates if you leave the position unchanged. Click a row to trace its move and attacks; click again to clear. Captures and forks may be unsound.",
                if self.board.side_to_move() == Color::White { "Black" } else { "White" })).size(12.0).color(muted));
            let selection_id = egui::Id::new("tactical_opponent_selected");
            let mut selected = ui.ctx().data(|d| d.get_temp::<(Board, String)>(selection_id))
                .filter(|(board, _)| *board == self.board).map(|(_, key)| key).unwrap_or_default();
            match self.tactical_opponent_threats(ui.ctx()) {
                None => { ui.label("The side to move is in check. Resolve check before exploring a hypothetical opponent turn."); selected.clear(); }
                Some(threats) => {
                    if !threats.iter().any(|t| t.key == selected) { selected.clear(); }
                    egui::ScrollArea::vertical().id_salt("opponent_threats_list").max_height(180.0).show(ui, |ui| {
                        if threats.is_empty() { ui.label("No candidate checks, captures, or forks."); }
                        for threat in threats {
                            if ui.selectable_label(selected == threat.key, &threat.text).clicked() {
                                selected = if selected == threat.key { String::new() } else { threat.key };
                            }
                        }
                    });
                }
            }
            ui.ctx().data_mut(|d| d.insert_temp(selection_id, (self.board, selected)));
        }
        let changes_id = egui::Id::new("tactical_show_changes");
        let mut show_changes = ui.ctx().data(|d| d.get_temp::<bool>(changes_id)).unwrap_or(true);
        ui.checkbox(&mut show_changes, "What changed?");
        ui.ctx().data_mut(|d| d.insert_temp(changes_id, show_changes));
        if show_changes {
            match self.tactical_changes(ui.ctx()) {
                None => { ui.label(RichText::new("Step forward in a game to compare with the previous position.").size(12.0).color(muted)); }
                Some(changes) => {
                    ui.label(RichText::new("Blue: opened attacks. Gold: lost support. Red: new threats. Geometric control, including pinned pieces.").size(12.0).color(muted));
                    egui::ScrollArea::vertical().id_salt("tactical_changes_list").max_height(180.0).show(ui, |ui| {
                        if changes.is_empty() { ui.label("No opened attack lines, lost defenders, or new threats after this move."); }
                        for change in changes {
                            ui.label(RichText::new(change.text).size(12.0).color(Self::tactical_change_color(change.kind)));
                        }
                    });
                }
            }
        }
        ui.add_space(6.0);
        ui.separator();
        ui.add_space(4.0);
        ui.label(RichText::new("Positional activity").size(14.0).strong().color(Color32::from_rgb(231, 221, 199)));
        ui.label(RichText::new("Click any piece on the main board to inspect its activity rating.").size(12.0).color(muted));
        if let Some((board, square)) = ui.ctx().data(|d| d.get_temp::<(Board, Square)>(egui::Id::new("tactical_activity_piece")))
            && board == self.board && let Some(activity) = crate::tactical::positional_activity(&board, square) {
            Frame::new().fill(Color32::from_rgb(24, 31, 42))
                .stroke(Stroke::new(1.0, Color32::from_white_alpha(30)))
                .corner_radius(CornerRadius::same(8)).inner_margin(Margin::same(10)).show(ui, |ui| {
                    ui.label(RichText::new(format!("{}: {} · {}/100", activity.title, activity.rating, activity.score)).strong());
                    for reason in activity.reasons { ui.label(format!("- {reason}")); }
                    ui.label(RichText::new("Positional heuristic, not material value or tactical safety. Geometric control includes pinned pieces.").size(11.0).color(muted));
                });
        }
        ui.add_space(4.0);
        ui.separator();
        let reach_id = egui::Id::new("tactical_knight_reach");
        let mut reach = ui.ctx().data(|d| d.get_temp::<bool>(reach_id)).unwrap_or(false);
        ui.checkbox(&mut reach, "Knight reach · two hops");
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
                ui.label("Knight:");
                for square in &knights { ui.selectable_value(&mut source, Some(*square), square.to_string()); }
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
            ui.checkbox(&mut safety, "Destination safety");
            ui.ctx().data_mut(|d| d.insert_temp(safety_id, safety));
            ui.label(RichText::new(if self.review_index.is_some() {
                "Click a ghost to lock/unlock its route."
            } else { "Live game: board clicks select and move pieces. Hover ghosts to preview; route locking is available in review." }).size(12.0).color(muted));
            ui.label(RichText::new("Safety: red attackers, green legal recapturers, amber unavailable defenders, gray geometric support. Hover for capture-specific recaptures.").size(12.0).color(muted));
            ui.label(RichText::new(if self.board_3d_rendering() {
                "Switch to 2D for knight reach."
            } else if knights.is_empty() { "No knights on the board." }
            else { "Click any knight on the board or choose its square above. Ghosts: first hop; outlines: next hop. Hover a ghost to focus. Gold: potential major-piece fork. Off-turn knights show geometric reach, including pinned moves. Other pieces stay put; replies can change reach." }).size(12.0).color(muted));
        }
        ui.add_space(4.0);
        ui.separator();
        ui.label(RichText::new("Patterns").size(14.0).strong().color(Color32::from_rgb(231, 221, 199)));
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
    pub(super) fn paint_tactical_map(&self, ui: &mut egui::Ui, rect: egui::Rect) {
        if !self.tactical_available() || !ui.ctx().data(|d| d.get_temp::<bool>(egui::Id::new("tactical_enabled"))).unwrap_or(false) { return; }
        if !self.board_3d_rendering() && ui.ctx().data(|d| d.get_temp::<bool>(egui::Id::new("tactical_knight_reach"))).unwrap_or(false)
            && let Some(source) = ui.ctx().data(|d| d.get_temp::<Square>(egui::Id::new("tactical_knight_source"))) {
            let cell = rect.width() / 8.0;
            let center = |s| rect.min + Self::square_screen_offset(s, self.flipped, cell);
            let routes = crate::tactical::knight_routes(&self.board, source);
            let hover = ui.ctx().pointer_hover_pos().filter(|&p| ui.rect_contains_pointer(rect) && rect.contains(p));
            let hovered_first = routes.iter().find(|(s, _)| hover.is_some_and(|p|
                egui::Rect::from_center_size(center(*s), Vec2::splat(cell)).contains(p))).map(|(s, _)| *s);
            let locked = ui.ctx().data(|d| d.get_temp::<(Board, Square, Square)>(egui::Id::new("tactical_knight_lock")))
                .filter(|(board, knight, first)| *board == self.board && *knight == source && routes.iter().any(|(s, _)| s == first))
                .map(|(_, _, first)| first);
            let focused = locked.or(hovered_first);
            let cyan = Color32::from_rgb(100, 195, 230);
            let painter = ui.painter().with_clip_rect(rect);
            painter.rect_stroke(egui::Rect::from_center_size(center(source), Vec2::splat(cell * 0.94)), 4.0,
                Stroke::new(2.5, cyan), egui::StrokeKind::Inside);
            for (first, next) in &routes {
                if focused.is_some_and(|s| s != *first) { continue; }
                for &(target, fork) in next {
                    let color = if fork { Color32::from_rgb(230, 178, 65) } else { cyan.gamma_multiply(if focused.is_some() { 0.8 } else { 0.35 }) };
                    painter.rect_stroke(egui::Rect::from_center_size(center(target), Vec2::splat(cell * 0.76)), 3.0,
                        Stroke::new(if fork { 2.5 } else { 1.5 }, color), egui::StrokeKind::Inside);
                    if focused.is_some() { painter.arrow(center(*first), (center(target) - center(*first)) * 0.82, Stroke::new(1.2, color)); }
                }
                if focused.is_some() { painter.arrow(center(source), (center(*first) - center(source)) * 0.8, Stroke::new(2.0, cyan)); }
            }
            if ui.ctx().data(|d| d.get_temp::<bool>(egui::Id::new("tactical_destination_safety"))).unwrap_or(true) {
                let destination = hover.and_then(|pos| {
                    let square = self.board_mark_square(pos, rect)?;
                    if let Some(first) = hovered_first { return Some((first, first)); }
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
                Self::paint_piece(&ghost, piece_rect, cell, self.piece_set, side, Piece::Knight);
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
            let color = Self::tactical_color(f.kind);
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
    // Paint last on the board layer, above annotations and tactical/engine arrows.
    pub(super) fn paint_tactical_bubbles(&self, ui: &mut egui::Ui, rect: egui::Rect) {
        if !self.tactical_piece_active(ui.ctx()) { return; }
        if !self.board_3d_rendering() && ui.ctx().data(|d| d.get_temp::<bool>(egui::Id::new("tactical_attack_defense"))).unwrap_or(false) {
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
            for (square, safety) in counts {
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
