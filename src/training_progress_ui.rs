use super::*;

// Windows refer to completed games, not just games with available analysis.
fn loss_history(phases: &[training::PhaseStats]) -> Vec<(Option<f64>, Option<f64>)> {
    phases.iter().enumerate().map(|(i, phase)| {
        if phase.cp_moves == 0 { return (None, None); }
        let samples: Vec<_> = phases[i.saturating_sub(9)..=i].iter()
            .filter(|p| p.cp_moves > 0).map(|p| p.mean_loss()).collect();
        (Some(phase.mean_loss()), Some(samples.iter().sum::<f64>() / samples.len() as f64))
    }).collect()
}

impl ChessApp {
    pub(super) fn training_progress_charts(ui: &mut egui::Ui, profile: &training::Profile) {
        let gold = Color32::from_rgb(215,185,121);
        let muted = Color32::from_rgb(164,176,179);
        ui.add_space(18.0);
        ui.label(RichText::new("Progress over time").size(18.0).color(gold));
        let id = egui::Id::new(("training_focus_chart_range", &profile.id));
        let mut range = ui.ctx().data(|d| d.get_temp::<usize>(id).unwrap_or(50));
        ui.horizontal_wrapped(|ui| {
            for (value,label) in [(20,"Last 20 games"),(50,"Last 50 games"),(0,"All games")] {
                if Self::compact_panel_button(ui,label,range==value).clicked() {range=value;}
            }
        });
        ui.ctx().data_mut(|d| d.insert_temp(id,range));
        ui.label(RichText::new("Gold: 10-game rolling average · dots: individual games · lower is better").size(12.0).color(muted));
        ui.label(RichText::new("Available numerical samples within each 10-game window; missing samples remain gaps. Mate scores excluded.").size(12.0).color(muted));
        let count=profile.results.len();
        if count==0 { ui.label("Complete and analyze training games to build your progress history."); return; }
        let start=if range==0 {0} else {count.saturating_sub(range)};
        let histories:Vec<_>=(0..3).map(|phase|loss_history(&profile.results.iter().map(|r|r.analysis.phases[phase].clone()).collect::<Vec<_>>())).collect();
        let maximum=histories.iter().flat_map(|h| h[start..].iter()).flat_map(|(a,b)| a.iter().chain(b.iter())).copied().fold(0.0_f64,f64::max);
        let ceiling=((maximum*1.1/50.0).ceil()*50.0).max(100.0);
        let shown=&profile.results[start..];
        let analyzed:usize=shown.iter().map(|r|r.analysis.analyzed_moves()).sum();
        let total:usize=shown.iter().map(|r|r.analysis.total_player_moves).sum();
        let analyzed_games=shown.iter().filter(|r|r.analysis.analyzed_moves()>0).count();
        ui.label(RichText::new(format!("{} games shown · {} with analysis · known moves: {} / {} ({:.0}%)",shown.len(),analyzed_games,analyzed,total,100.0*analyzed as f64/total.max(1) as f64)).size(12.0).color(muted));
        if shown.iter().any(|r|r.analysis.total_player_moves==0) {ui.label(RichText::new("Games without recorded move totals are excluded from move coverage.").size(12.0).color(muted));}
        for (phase,name) in ["Opening","Middlegame","Endgame"].iter().enumerate() {
            ui.add_space(10.0);
            Self::training_sunken_section(ui,|ui| {
                ui.label(RichText::new(format!("{name} · average loss (cp)")).color(gold));
                let (rect,response)=ui.allocate_exact_size(Vec2::new(ui.available_width(),160.0),Sense::hover());
                let painter=ui.painter_at(rect);
                let plot=egui::Rect::from_min_max(rect.min+Vec2::new(46.0,12.0),rect.max-Vec2::new(9.0,29.0));
                let x=|i:usize| if count-start==1 {plot.center().x} else {plot.left()+(i-start) as f32/(count-start-1) as f32*plot.width()};
                let y=|v:f64| plot.bottom()-(v/ceiling) as f32*plot.height();
                for tick in [0.0,ceiling/2.0,ceiling] {
                    painter.line_segment([egui::pos2(plot.left(),y(tick)),egui::pos2(plot.right(),y(tick))],Stroke::new(1.0,Color32::from_rgb(44,55,60)));
                    painter.text(egui::pos2(plot.left()-7.0,y(tick)),Align2::RIGHT_CENTER,format!("{tick:.0}"),FontId::proportional(11.0),muted);
                }
                let history=&histories[phase];
                let mut previous=None;
                for (i,(value,trend)) in history.iter().enumerate().skip(start) {
                    if let Some(v)=value {painter.circle_filled(egui::pos2(x(i),y(*v)),2.6,Color32::from_rgba_unmultiplied(215,185,121,95));}
                    if let Some(v)=trend {
                        let point=egui::pos2(x(i),y(*v));
                        if let Some(last)=previous {painter.line_segment([last,point],Stroke::new(2.0,gold));}
                        painter.circle_filled(point,2.2,gold); previous=Some(point);
                    } else {previous=None;}
                }
                if !history[start..].iter().any(|p|p.0.is_some()) {
                    painter.text(plot.center(),Align2::CENTER_CENTER,"No numerical analysis for this phase",FontId::proportional(12.0),muted);
                }
                painter.text(egui::pos2(plot.left(),plot.bottom()+8.0),Align2::LEFT_TOP,format!("Game {}",start+1),FontId::proportional(11.0),muted);
                if count-start>1 {painter.text(egui::pos2(plot.right(),plot.bottom()+8.0),Align2::RIGHT_TOP,format!("Game {count}"),FontId::proportional(11.0),muted);}
                if let Some(pointer)=response.hover_pos() {
                    let i=if count-start==1 {start} else {start+(((pointer.x-plot.left())/plot.width()).clamp(0.0,1.0)*(count-start-1) as f32).round() as usize};
                    painter.line_segment([egui::pos2(x(i),plot.top()),egui::pos2(x(i),plot.bottom())],Stroke::new(1.0,muted));
                    let entry=&profile.results[i]; let stats=&entry.analysis; let p=&stats.phases[phase];
                    let samples=profile.results[i.saturating_sub(9)..=i].iter().filter(|r|r.analysis.phases[phase].cp_moves>0).count();
                    response.on_hover_text(format!("Game {} · {} · {}\n{}\n10-game average: {} ({} sampled games)\nPhase: {} analyzed moves · {} numerical\nGame coverage: {} / {} player moves ({:.0}%)",i+1,entry.session.completed_at.as_deref().unwrap_or("Date unknown"),entry.session.side.label(),history[i].0.map(|v|format!("{v:.1} cp average loss")).unwrap_or_else(||"No numerical analysis in this phase".into()),history[i].1.map(|v|format!("{v:.1} cp")).unwrap_or_else(||"No point".into()),samples,p.moves,p.cp_moves,stats.analyzed_moves(),stats.total_player_moves,100.0*stats.analyzed_moves() as f64/stats.total_player_moves.max(1) as f64));
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn phase(value:Option<u64>)->training::PhaseStats {
        training::PhaseStats{loss_cp:value.unwrap_or(0),cp_moves:usize::from(value.is_some()),..Default::default()}
    }
    #[test]
    fn missing_analysis_is_a_gap_and_real_zero_is_a_sample() {
        let h=loss_history(&[phase(Some(0)),phase(None),phase(Some(100))]);
        assert_eq!(h,vec![(Some(0.0),Some(0.0)),(None,None),(Some(100.0),Some(50.0))]);
    }
    #[test]
    fn rolling_window_counts_completed_games_and_uses_per_game_means() {
        let mut phases=vec![phase(Some(1000))];phases.extend((0..9).map(|_|phase(None)));phases.push(phase(Some(20)));
        assert_eq!(loss_history(&phases)[10],(Some(20.0),Some(20.0)));
        phases=vec![training::PhaseStats{cp_moves:10,loss_cp:100,..Default::default()},phase(Some(30))];
        assert_eq!(loss_history(&phases)[1],(Some(30.0),Some(20.0)));
        assert!(loss_history(&[]).is_empty());
    }
}
