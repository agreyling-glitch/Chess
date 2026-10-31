use super::*;

// Timing categories are conservative review prompts, not a claim that a
// position's difficulty can be established from a single engine line.
fn timing_category(seconds: f64, quality: Option<MoveClassification>, easy: bool) -> usize {
    let good = matches!(quality, Some(MoveClassification::Best | MoveClassification::Good));
    let error = matches!(quality, Some(MoveClassification::Mistake | MoveClassification::Blunder));
    if seconds <= 5.0 && error { 2 }
    else if seconds > 20.0 && good && easy { 1 }
    else if seconds > 20.0 && good { 0 }
    else { 3 }
}

impl ChessApp {
    pub(super) fn time_quality_quadrant_ui(&self, ui: &mut egui::Ui, selected: usize) -> Option<usize> {
        ui.label(RichText::new("Time vs. move quality quadrant").size(17.0).strong());
        ui.add(egui::Label::new("Rushed errors (top left): slow down and add a quick blunder check: ‘What does their last move threaten?’\nThought hard, still wrong (top right): improve the thinking process with a candidate-move routine; avoid replaying the same line.\nTime well spent (bottom right): this is where the time should go.").wrap());
        ui.add(egui::Label::new("Review guides: 20 seconds and 80 estimated centipawns lost. These are fixed prompts, not time-control-adjusted judgments. Lower loss is better. Only moves with both recorded time and a numerical CPL estimate are plotted; mate transitions and missing data are omitted. Hover a point for details; click to review.").wrap());
        let colors = [Color32::from_rgb(177,176,165), Color32::from_rgb(229,66,71), Color32::from_rgb(44,128,219), Color32::from_rgb(238,162,0)];
        let labels = ["Quick and fine", "Quick and costly", "Slow and accurate", "Slow and still costly"];
        ui.horizontal_wrapped(|ui| {
            for n in 0..4 {
                ui.horizontal(|ui| {
                    let (rect,_) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
                    ui.painter().circle_filled(rect.center(),4.0,colors[n]);
                    ui.label(labels[n]);
                });
            }
        });
        let mut jump = None;
        for side in [Color::White, Color::Black] {
            let rows: Vec<_> = (1..self.review_positions.len())
                .filter(|&i| self.review_positions[i-1].side_to_move()==side)
                .filter_map(|i| Some((i, self.move_timings.get(i)?.elapsed.as_deref().and_then(Self::pgn_seconds)?, self.move_centipawn_loss(i)? as f64)))
                .collect();
            ui.add_space(12.0);
            let mut grain = None;
            let section = Frame::new().fill(Color32::from_rgb(13,17,22)).corner_radius(6.0).inner_margin(Margin::same(12)).show(ui,|ui| {
                ui.set_min_width(ui.available_width());
                grain = Some(ui.painter().add(egui::Shape::Noop));
                ui.label(RichText::new(format!("{} · {}",if side==Color::White {"White"} else {"Black"},if side==Color::White {&self.review_white_player} else {&self.review_black_player})).strong());
                if rows.is_empty() { ui.label("No moves with both recorded time and a numerical CPL estimate for this player."); return; }
                ui.label("Estimated centipawn loss (lower is better)");
                let xmax=(rows.iter().map(|r|r.1).fold(40.0_f64,f64::max)/10.0).ceil()*10.0;
                let ymax=(rows.iter().map(|r|r.2).fold(160.0_f64,f64::max)/40.0).ceil()*40.0;
                let (rect,_) = ui.allocate_exact_size(Vec2::new(ui.available_width().max(100.0),320.0),Sense::hover());
                let plot=egui::Rect::from_min_max(rect.min+Vec2::new(48.0,8.0),rect.max-Vec2::new(8.0,38.0));
                let point=|seconds:f64,loss:f64| egui::pos2(plot.left()+plot.width()*(seconds/xmax) as f32,plot.bottom()-plot.height()*(loss/ymax) as f32);
                let painter=ui.painter();
                let muted=Color32::from_gray(155);
                for tick in 0..=4 {
                    let x=point(xmax*tick as f64/4.0,0.0).x;
                    let y=point(0.0,ymax*tick as f64/4.0).y;
                    painter.line_segment([egui::pos2(x,plot.top()),egui::pos2(x,plot.bottom())],Stroke::new(1.0,Color32::from_white_alpha(24)));
                    painter.line_segment([egui::pos2(plot.left(),y),egui::pos2(plot.right(),y)],Stroke::new(1.0,Color32::from_white_alpha(24)));
                    painter.text(egui::pos2(x,plot.bottom()+6.0),Align2::CENTER_TOP,format!("{:.0}",xmax*tick as f64/4.0),FontId::proportional(10.0),muted);
                    painter.text(egui::pos2(plot.left()-6.0,y),Align2::RIGHT_CENTER,format!("{:.0}",ymax*tick as f64/4.0),FontId::proportional(10.0),muted);
                }
                for (a,b) in [(point(20.0,0.0),point(20.0,ymax)),(point(0.0,80.0),point(xmax,80.0))] {
                    let length=a.distance(b);
                    for n in 0..(length/10.0).ceil() as usize {
                        painter.line_segment([a.lerp(b,(n as f32*10.0/length).min(1.0)),a.lerp(b,((n as f32*10.0+5.0)/length).min(1.0))],Stroke::new(1.0,muted));
                    }
                }
                for (pos,align,label) in [
                    (plot.left_top()+Vec2::splat(6.0),Align2::LEFT_TOP,"Rushed errors"),
                    (plot.right_top()+Vec2::new(-6.0,6.0),Align2::RIGHT_TOP,"Thought hard, still wrong"),
                    (plot.left_bottom()+Vec2::new(6.0,-6.0),Align2::LEFT_BOTTOM,"Routine"),
                    (plot.right_bottom()-Vec2::splat(6.0),Align2::RIGHT_BOTTOM,"Time well spent"),
                ] { painter.text(pos,align,label,FontId::proportional(11.0),muted); }
                painter.text(egui::pos2(plot.center().x,rect.bottom()),Align2::CENTER_BOTTOM,"Seconds spent on the move",FontId::proportional(11.0),muted);
                let pointer=ui.input(|input|input.pointer.hover_pos());
                let nearest=pointer.filter(|p|plot.expand(7.0).contains(*p)).and_then(|p|rows.iter().enumerate().map(|(n,r)|(n,point(r.1,r.2).distance(p))).filter(|r|r.1<=10.0).min_by(|a,b|a.1.total_cmp(&b.1)).map(|r|r.0));
                for (n,&(i,seconds,loss)) in rows.iter().enumerate() {
                    let category=usize::from(seconds>20.0)*2+usize::from(loss>=80.0);
                    let p=point(seconds,loss);
                    painter.circle_filled(p,5.0,colors[category]);
                    painter.circle_stroke(p,5.0,Stroke::new(1.0,Color32::from_gray(225)));
                    if i==selected { painter.circle_stroke(p,8.0,Stroke::new(1.5,Color32::WHITE)); }
                    if nearest==Some(n) && ui.interact(egui::Rect::from_center_size(p,Vec2::splat(20.0)),ui.id().with(("time_quality",i)),Sense::click()).on_hover_text(format!("{}\n{seconds:.1}s spent · {loss:.0} estimated CPL\n{}\nClick to review",self.graph_position_label(i,true),labels[category])).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {jump=Some(i);}
                }
            });
            if let Some(shape)=grain {
                let paper=Self::paper_grain_texture(ui.ctx());
                let surface=section.response.rect.shrink(4.0);
                ui.painter().set(shape,egui::Shape::image(paper.id(),surface,egui::Rect::from_min_max(egui::Pos2::ZERO,egui::pos2(surface.width()/256.0,surface.height()/256.0)),Color32::from_white_alpha(75)));
            }
            Self::paint_sunken_border(ui,section.response.rect);
        }
        jump
    }

    pub(super) fn clock_burn_down_ui(&self, ui: &mut egui::Ui, selected: usize) -> Option<usize> {
        ui.label(RichText::new("Clock burn-down").size(17.0).strong());
        ui.add(egui::Label::new("The dashed even-pace line is your budget. Falling below it after the opening can signal that you have overspent early. A steep drop after a long think is how time trouble starts. The red line marks 60 seconds: aim to reach move 30 or so with a cushion above it.").wrap());
        let control = Self::simple_time_control(&self.pgn_input);
        let colors = [Color32::from_rgb(44,128,219), Color32::from_rgb(243,112,48)];
        ui.horizontal_wrapped(|ui| {
            for (style, color, label) in [
                (0, colors[0], format!("White · {}", self.review_white_player)),
                (1, colors[1], format!("Black · {} (dotted)", self.review_black_player)),
                (2, Color32::from_gray(155), "Even pace (dashed)".into()),
                (0, Color32::from_rgb(229,66,71), "60s reference".into()),
            ] {
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(24.0, 14.0), Sense::hover());
                    let y = rect.center().y;
                    if style == 1 {
                        for x in [2.0, 8.0, 14.0, 20.0] {
                            ui.painter().circle_filled(egui::pos2(rect.left() + x, y), 1.3, color);
                        }
                    } else if style == 2 {
                        for x in [0.0, 12.0] {
                            ui.painter().line_segment([egui::pos2(rect.left() + x, y), egui::pos2(rect.left() + x + 7.0, y)], Stroke::new(1.4, color));
                        }
                    } else {
                        ui.painter().line_segment([egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)], Stroke::new(2.0, color));
                    }
                    ui.colored_label(color, label);
                });
            }
        });
        if let Some((initial,increment)) = control {
            ui.add(egui::Label::new(format!("Time control: {initial:.0}s + {increment:.1}s per move. Budget: spread the starting clock evenly to retain {:.0}s at move 40; the per-move spending allowance also includes the increment. Actual clocks include increments and can rise. The 60s line is a review guide, not a universal danger threshold.", initial.min(60.0))).wrap());
        } else {
            ui.add(egui::Label::new("Starting time / increment unavailable: the even-pace budget is omitted. Actual recorded clocks may rise when increments are added. The 60s line is a review guide.").wrap());
        }
        let rows: Vec<_> = (1..self.review_positions.len()).filter_map(|i| {
            self.move_timings.get(i)?.remaining.as_deref().and_then(Self::pgn_seconds)
                .map(|seconds| (i, usize::from(self.review_positions[i-1].side_to_move() == Color::Black), seconds))
        }).collect();
        if rows.is_empty() { ui.label("No recorded remaining-clock data for this game."); return None; }
        let last = self.review_positions.len().saturating_sub(1).div_ceil(2).max(40) as f32;
        let maximum = rows.iter().map(|r|r.2).fold(control.map_or(60.0,|c|c.0.max(60.0)), f64::max);
        let maximum = (maximum / 4.0).ceil() * 4.0;
        ui.add_space(12.0);
        let mut grain = None;
        let section = Frame::new()
            .fill(Color32::from_rgb(13, 17, 22))
            .corner_radius(6.0)
            .inner_margin(Margin::same(12))
            .show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        grain = Some(ui.painter().add(egui::Shape::Noop));
        ui.label("Seconds remaining");
        let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width().max(100.0), 320.0), Sense::hover());
        let plot = egui::Rect::from_min_max(rect.min+Vec2::new(48.0,8.0),rect.max-Vec2::new(8.0,38.0));
        let point = |mv:f32,seconds:f64| egui::pos2(plot.left()+plot.width()*mv/last,plot.bottom()-plot.height()*(seconds/maximum) as f32);
        let painter=ui.painter();
        let muted=Color32::from_gray(155);
        for tick in 0..=4 {
            let y=point(0.0,maximum*tick as f64/4.0).y;
            painter.line_segment([egui::pos2(plot.left(),y),egui::pos2(plot.right(),y)],Stroke::new(1.0,Color32::from_white_alpha(28)));
            painter.text(egui::pos2(plot.left()-6.0,y),Align2::RIGHT_CENTER,format!("{:.0}",maximum*tick as f64/4.0),FontId::proportional(10.0),muted);
        }
        let stride=((last*35.0/plot.width()).ceil() as usize).max(1);
        for mv in (0..=last as usize).step_by(stride) {
            painter.text(egui::pos2(point(mv as f32,0.0).x,plot.bottom()+6.0),Align2::CENTER_TOP,mv.to_string(),FontId::proportional(10.0),muted);
        }
        painter.text(egui::pos2(plot.center().x,rect.bottom()),Align2::CENTER_BOTTOM,"Move number",FontId::proportional(11.0),muted);
        painter.line_segment([point(0.0,60.0),point(last,60.0)],Stroke::new(1.0,Color32::from_rgb(229,66,71)));
        if let Some((initial,_))=control {
            let end=point(40.0,initial.min(60.0));
            let start=point(0.0,initial);
            let length=start.distance(end);
            for n in 0..(length/12.0).ceil() as usize {
                let a=(n as f32*12.0/length).min(1.0);
                let b=((n as f32*12.0+7.0)/length).min(1.0);
                painter.line_segment([start.lerp(end,a),start.lerp(end,b)],Stroke::new(1.4,muted));
            }
        }
        let mut jump=None;
        for side in 0..2 {
            let mut previous=control.map(|c|(0usize,point(0.0,c.0)));
            for &(i,_,seconds) in rows.iter().filter(|r|r.1==side) {
                let p=point(i.div_ceil(2) as f32,seconds);
                if let Some((before,start))=previous.filter(|(before,_)| *before==0 && i<=2 || i==*before+2) {
                    let _=before;
                    if side==0 { painter.line_segment([start,p],Stroke::new(2.0,colors[side])); }
                    else {
                        let length=start.distance(p);
                        for n in 0..(length/6.0).ceil() as usize { painter.circle_filled(start.lerp(p,(n as f32*6.0/length).min(1.0)),1.3,colors[side]); }
                    }
                }
                previous=Some((i,p));
                painter.circle_filled(p,2.0,colors[side]);
                if selected==i { painter.circle_stroke(p,5.0,Stroke::new(1.0,Color32::WHITE)); }
                let hit=egui::Rect::from_center_size(p,Vec2::splat(14.0));
                if ui.interact(hit,ui.id().with(("clock_burn",i)),Sense::click()).on_hover_text(format!("{} · {}\n{seconds:.1}s remaining\nClick to review",if side==0 {&self.review_white_player}else{&self.review_black_player},self.graph_position_label(i,true))).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {jump=Some(i);}
            }
        }
        jump
        });
        if let Some(shape) = grain {
            let paper = Self::paper_grain_texture(ui.ctx());
            let surface = section.response.rect.shrink(4.0);
            ui.painter().set(shape, egui::Shape::image(paper.id(), surface,
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(surface.width() / 256.0, surface.height() / 256.0)),
                Color32::from_white_alpha(75)));
        }
        Self::paint_sunken_border(ui, section.response.rect);
        section.inner
    }

    pub(super) fn time_management_ui(&self, ui: &mut egui::Ui, selected: usize) -> Option<usize> {
        ui.label(RichText::new("Time per move").size(17.0).strong());
        ui.add(egui::Label::new("Look for orange bars (long thinks on likely easy moves) and red bars (quick moves that became mistakes). Spending time on easy recaptures and rushing tactical positions can signal a reallocation problem. Before spending more than about 20 seconds, ask: ‘Is this a critical moment?’ Look for a pawn break, a forced sequence, or a tactic that could swing the game.").wrap());
        let colors = [Color32::from_rgb(44, 128, 219), Color32::from_rgb(243, 112, 48), Color32::from_rgb(229, 66, 71), Color32::from_rgb(177, 176, 165)];
        let labels = ["Deep think, good move", "Long think, likely easy move", "Rushed, then erred", "Routine / unclassified"];
        ui.horizontal_wrapped(|ui| {
            for category in 0..4 { ui.colored_label(colors[category], format!("■ {}", labels[category])); }
        });
        ui.add(egui::Label::new("Review flags: long = over 20 seconds; quick = 5 seconds or less. Likely easy = a forced legal move, or an engine-approved recapture with no check on either side. Red requires an analyzed mistake or blunder. Missing times stay blank. Hover for clock details; click a bar to review the move.").wrap());
        let mut jump = None;
        for side in [Color::White, Color::Black] {
            let indices: Vec<_> = (1..self.review_positions.len()).filter(|&i| self.review_positions[i-1].side_to_move() == side).collect();
            let rows: Vec<_> = indices.iter().map(|&i| {
                let timing = self.move_timings.get(i);
                let spent = timing.and_then(|t| t.elapsed.as_deref()).and_then(Self::pgn_seconds);
                let remaining = timing.and_then(|t| t.remaining.as_deref()).and_then(Self::pgn_seconds);
                let root = &self.review_positions[i-1];
                let played = Self::review_move_at(&self.review_positions, &self.review_moves, i);
                let previous = i.checked_sub(1).filter(|&p| p > 0).and_then(|p| Self::review_move_at(&self.review_positions, &self.review_moves, p));
                let recapture = played.zip(previous).is_some_and(|(m,p)| m.get_dest() == p.get_dest() && root.piece_on(m.get_dest()).is_some());
                let easy = MoveGen::new_legal(root).count() == 1 || (recapture && root.checkers().popcnt() == 0 && self.review_positions[i].checkers().popcnt() == 0 && self.move_matches_engine_best(i));
                let category = timing_category(spent.unwrap_or(0.0), self.move_classification(i), easy);
                (i, spent, remaining, category)
            }).collect();
            ui.add_space(12.0);
            let mut grain = None;
            let section = Frame::new()
                .fill(Color32::from_rgb(13, 17, 22))
                .corner_radius(6.0)
                .inner_margin(Margin::same(12))
                .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            grain = Some(ui.painter().add(egui::Shape::Noop));
            ui.label(RichText::new(format!("{} · {}", if side == Color::White { "White" } else { "Black" }, if side == Color::White { &self.review_white_player } else { &self.review_black_player })).strong());
            if rows.iter().all(|r| r.1.is_none() && r.2.is_none()) {
                ui.label("No recorded move times or clocks for this player.");
                return;
            }
            for clock in [false] {
                ui.label(if clock { "Time remaining after move (seconds)" } else { "Time spent (seconds)" });
                if rows.iter().all(|r| if clock { r.2.is_none() } else { r.1.is_none() }) {
                    ui.label("No recorded data for this chart.");
                    continue;
                }
                let maximum = rows.iter().filter_map(|r| if clock { r.2 } else { r.1 }).fold(1.0_f64, f64::max);
                let maximum = (maximum / 4.0).ceil() * 4.0;
                let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width().max(80.0), if clock { 135.0 } else { 220.0 }), Sense::hover());
                let plot = egui::Rect::from_min_max(rect.min + Vec2::new(48.0, 6.0), rect.max - Vec2::new(4.0, 35.0));
                let painter = ui.painter();
                let muted = Color32::from_gray(155);
                for tick in 0..=4 {
                    let y = plot.bottom() - plot.height() * tick as f32 / 4.0;
                    painter.line_segment([egui::pos2(plot.left(), y), egui::pos2(plot.right(), y)], Stroke::new(1.0, Color32::from_white_alpha(28)));
                    painter.text(egui::pos2(plot.left()-6.0,y), Align2::RIGHT_CENTER, format!("{:.0}", maximum * tick as f64 / 4.0), FontId::proportional(10.0), muted);
                }
                let slot = plot.width() / rows.len().max(1) as f32;
                let stride = (32.0 / slot.max(1.0)).ceil() as usize;
                for (n, &(i, spent, remaining, category)) in rows.iter().enumerate() {
                    let x = plot.left() + (n as f32 + 0.5) * slot;
                    if n % stride.max(1) == 0 {
                        painter.text(egui::pos2(x,plot.bottom()+5.0), Align2::CENTER_TOP, i.div_ceil(2).to_string(), FontId::proportional(10.0), muted);
                    }
                    let value = if clock { remaining } else { spent };
                    let Some(value) = value else { continue; };
                    let top = plot.bottom() - (value / maximum) as f32 * plot.height();
                    let bar = egui::Rect::from_min_max(egui::pos2(x-slot*0.4,top.min(plot.bottom()-1.0)), egui::pos2(x+slot*0.4,plot.bottom()));
                    painter.rect_filled(bar, 1.0, if clock { Color32::from_rgb(112,160,133) } else { colors[category] });
                    if i == selected { painter.line_segment([egui::pos2(x,plot.top()),egui::pos2(x,plot.bottom())], Stroke::new(1.0,Color32::WHITE)); }
                    let hit = egui::Rect::from_min_max(egui::pos2(x-slot/2.0,plot.top()),egui::pos2(x+slot/2.0,plot.bottom()));
                    let detail = format!("{}\nTime spent: {}\nTime remaining: {}\n{}{}\nClick to review", self.graph_position_label(i,true), spent.map_or("Not recorded".into(), |v| format!("{v:.1}s")), remaining.map_or("Not recorded".into(), |v| format!("{v:.1}s")), if spent.is_some() { labels[category] } else { "Move time not recorded" }, if remaining.is_some_and(|v|v<=20.0) { " · Low clock (≤20s after move)" } else { "" });
                    if ui.interact(hit, ui.id().with(("move_timing", side == Color::White, clock, i)), Sense::click()).on_hover_text(detail).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { jump = Some(i); }
                }
                painter.text(egui::pos2(plot.center().x,rect.bottom()), Align2::CENTER_BOTTOM, "Move number", FontId::proportional(11.0), muted);
            }
            });
            if let Some(shape) = grain {
                let paper = Self::paper_grain_texture(ui.ctx());
                let surface = section.response.rect.shrink(4.0);
                ui.painter().set(shape, egui::Shape::image(paper.id(), surface,
                    egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(surface.width() / 256.0, surface.height() / 256.0)),
                    Color32::from_white_alpha(75)));
            }
            Self::paint_sunken_border(ui, section.response.rect);
        }
        jump
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn flags_require_quality_and_difficulty_evidence() {
        assert_eq!(timing_category(3.0,Some(MoveClassification::Blunder),false),2);
        assert_eq!(timing_category(21.0,Some(MoveClassification::Best),true),1);
        assert_eq!(timing_category(21.0,Some(MoveClassification::Good),false),0);
        assert_eq!(timing_category(30.0,None,true),3);
        assert_eq!(timing_category(6.0,Some(MoveClassification::Mistake),false),3);
        assert_eq!(timing_category(20.0,Some(MoveClassification::Best),true),3);
    }
}
