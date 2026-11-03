use super::*;

const GOLD: Color32 = Color32::from_rgb(215,185,121);
const MUTED: Color32 = Color32::from_rgb(168,179,174);

impl ChessApp {
    fn prediction_console_text_button(ui:&mut egui::Ui,label:&str,kind:u8,width:f32,height:f32)->egui::Response {
        let response=ui.add_sized([width,height],egui::Button::new(""));
        let color=if response.enabled(){Color32::from_rgb(231,233,226)}else{Color32::from_gray(90)};
        let font=FontId::proportional(12.0);
        let text=ui.painter().layout_no_wrap(label.into(),font,color);
        let left=response.rect.center().x-(text.size().x+20.0)/2.0;
        Self::prediction_console_icon(ui,egui::Rect::from_center_size(egui::pos2(left+6.0,response.rect.center().y),Vec2::splat(14.0)),kind,color);
        ui.painter().galley(egui::pos2(left+20.0,response.rect.center().y-text.size().y/2.0),text,color);
        response.widget_info(||egui::WidgetInfo::labeled(egui::WidgetType::Button,response.enabled(),label));
        response
    }

    // Original line icons keep the console independent of installed symbol fonts.
    pub(super) fn prediction_console_icon(ui:&egui::Ui, rect:egui::Rect, kind:u8, color:Color32) {
        let c=rect.center(); let s=Stroke::new(1.4,color);
        let line=|a:Vec2,b:Vec2|{ui.painter().line_segment([c+a,c+b],s);};
        match kind {
            0 => { // Sparkles
                for (offset,r) in [(Vec2::new(-3.0,2.0),6.0),(Vec2::new(5.0,-5.0),3.0)] {
                    ui.painter().add(egui::Shape::closed_line(vec![c+offset+Vec2::new(0.0,-r),c+offset+Vec2::new(r*0.3,0.0),c+offset+Vec2::new(0.0,r),c+offset+Vec2::new(-r*0.3,0.0)],s));
                    line(offset-Vec2::new(r,0.0),offset+Vec2::new(r,0.0));
                }
            }
            1 => {for (y,x) in [(-5.0,2.0),(0.0,-3.0),(5.0,1.0)] {
                line(Vec2::new(-7.0,y),Vec2::new(7.0,y));
                ui.painter().circle_filled(c+Vec2::new(x,y),2.1,color);
            }}
            2 => {line(Vec2::new(-4.0,-6.0),Vec2::new(5.0,0.0));line(Vec2::new(5.0,0.0),Vec2::new(-4.0,6.0));line(Vec2::new(-4.0,6.0),Vec2::new(-4.0,-6.0));}
            3 => {line(Vec2::new(-4.0,-3.0),Vec2::new(0.0,2.0));line(Vec2::new(0.0,2.0),Vec2::new(4.0,-3.0));}
            4|5|6|7 => {
                let right=kind>=6;let x=if right {1.0}else{-1.0};
                line(Vec2::new(-x*3.0,-5.0),Vec2::new(x*2.0,0.0));
                line(Vec2::new(x*2.0,0.0),Vec2::new(-x*3.0,5.0));
                if kind==4||kind==7 {line(Vec2::new(x*6.0,-6.0),Vec2::new(x*6.0,6.0));}
            }
            8 => {line(Vec2::new(-5.0,5.0),Vec2::new(5.0,-5.0));line(Vec2::new(-1.0,-5.0),Vec2::new(5.0,-5.0));line(Vec2::new(5.0,-5.0),Vec2::new(5.0,1.0));}
            9 => {for x in [-3.0,3.0] {line(Vec2::new(x,-5.0),Vec2::new(x,5.0));}}
            _ => {for v in [Vec2::new(-1.0,-1.0),Vec2::new(1.0,1.0)] {
                let a=v*5.0;line(a,a-Vec2::new(v.x*4.0,0.0));line(a,a-Vec2::new(0.0,v.y*4.0));
            }}
        }
    }

    fn prediction_console_style(ui:&mut egui::Ui) {
        Self::stable_panel_button_style(ui);
        ui.visuals_mut().override_text_color=Some(Color32::from_rgb(231,233,226));
        ui.spacing_mut().item_spacing=Vec2::new(8.0,8.0);
        ui.spacing_mut().button_padding=Vec2::new(12.0,8.0);
        ui.spacing_mut().interact_size.y=38.0;
        ui.style_mut().wrap_mode=Some(egui::TextWrapMode::Wrap);
        let widgets=&mut ui.visuals_mut().widgets;
        for (widget,fill,border) in [(&mut widgets.inactive,Color32::from_rgb(32,42,46),Color32::from_rgb(58,71,75)),
            (&mut widgets.hovered,Color32::from_rgb(42,53,57),Color32::from_rgb(152,131,87)),
            (&mut widgets.active,Color32::from_rgb(53,51,38),GOLD),
            (&mut widgets.open,Color32::from_rgb(53,51,38),GOLD)] {
            widget.bg_fill=fill;widget.weak_bg_fill=fill;widget.bg_stroke=Stroke::new(1.0,border);widget.corner_radius=CornerRadius::same(5);
        }
        ui.visuals_mut().selection.bg_fill=Color32::from_rgb(55,52,38);
        ui.visuals_mut().selection.stroke=Stroke::new(1.0,GOLD);
    }

    pub(super) fn engine_prediction_contents_ui(&mut self,ui:&mut egui::Ui) {
        Self::prediction_console_style(ui);
        if self.training_live() {
            ui.heading("Training vs AI");ui.label("Play without hints or takebacks. Analyze the game after it finishes.");return;
        }
        ui.horizontal(|ui| {
            let (rect,_)=ui.allocate_exact_size(Vec2::splat(24.0),Sense::hover());
            Self::prediction_console_icon(ui,rect,0,GOLD);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y=2.0;
                ui.label(RichText::new("ENGINE PREDICTION").size(11.0).color(GOLD));
                ui.label(RichText::new("Find the next move").size(20.0));
            });
            ui.with_layout(Layout::right_to_left(Align::Center),|ui| {
                let settings=Self::analysis_toolbar_icon_button(ui,!self.engine_settings_locked());
                Self::prediction_console_icon(ui,settings.rect,1,MUTED);
                settings.widget_info(||egui::WidgetInfo::labeled(egui::WidgetType::Button,settings.enabled(),"Prediction engine settings"));
                if settings.on_hover_text("Prediction engine settings").clicked(){self.open_engine_settings();}
            });
        });
        let side=if self.board.side_to_move()==Color::White {"White"}else{"Black"};
        ui.add_space(8.0);
        Self::grained_outer_section(ui,|ui| {
            Self::prediction_console_style(ui);
            let best=self.analysis_variations.iter().filter_map(|line| {
                let i=line.positions.iter().position(|p|*p==self.board)?;
                Some((line.comparison_value()?*if self.board.side_to_move()==Color::White{1}else{-1},line.moves.get(i)?.clone()))
            }).max_by_key(|(score,_)|*score).map(|(_,san)|san);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y=3.0;
                    ui.label(RichText::new(self.evaluation_text()).size(34.0).color(GOLD));
                    ui.label(RichText::new("White perspective").size(12.0).color(MUTED));
                });
                ui.with_layout(Layout::right_to_left(Align::Center),|ui| {
                    ui.with_layout(Layout::top_down(Align::Max),|ui| {
                        ui.label(RichText::new(best.as_deref().map(|san|self.display_san(san)).unwrap_or_else(||"Waiting for engine…".into())).color(GOLD));
                        ui.label(RichText::new(format!("Best move for {side}")).size(12.0).color(MUTED));
                    });
                });
            });
            let fraction=self.analysis_eval_cp.map(|cp|0.5+(cp as f32/400.0).atan()/std::f32::consts::PI).unwrap_or(0.5);
            let (track,_)=ui.allocate_exact_size(Vec2::new(ui.available_width(),5.0),Sense::hover());
            ui.painter().rect_filled(track,3,Color32::from_rgb(53,65,60));
            ui.painter().rect_filled(egui::Rect::from_min_size(track.min,Vec2::new(track.width()*fraction,5.0)),3,GOLD);
            ui.add_space(4.0);
            if self.game_analysis_running||self.game_analysis_paused {
                ui.label(RichText::new("Live analysis on hold").color(GOLD));
                ui.label("Pause or finish the full-game analysis pass to resume position analysis.");
            } else {self.prediction_analyze_control_ui(ui);}
        });
        ui.add_space(8.0);
        let enabled=self.show_prediction_best_move;
        Frame::new().fill(if enabled{Color32::from_rgb(30,43,37)}else{Color32::from_rgb(27,32,34)})
            .stroke(Stroke::new(1.0,if enabled{Color32::from_rgb(71,107,80)}else{Color32::from_rgb(58,71,75)}))
            .corner_radius(6).inner_margin(Margin::symmetric(12,10)).show(ui,|ui| {
                ui.spacing_mut().interact_size.y=22.0;
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    if ui.checkbox(&mut self.show_prediction_best_move,"Show best next move")
                        .on_hover_text("Show the best move in green and alternative moves with evaluation scores on the board.").changed(){self.save_game();}
                    ui.with_layout(Layout::right_to_left(Align::Center),|ui| {
                        let (r,_)=ui.allocate_exact_size(Vec2::splat(18.0),Sense::hover());Self::prediction_console_icon(ui,r,8,MUTED);
                    });
                });
            });
        ui.add_space(8.0);
        let mut jump=None;
        let panel=Frame::new().fill(Color32::from_rgb(14,21,25)).corner_radius(7).inner_margin(14).show(ui,|ui| {
            let layer=ui.painter().add(egui::Shape::Noop);
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {ui.label("Predicted line");ui.with_layout(Layout::right_to_left(Align::Center),|ui|{ui.label(RichText::new(format!("{} candidates",self.analysis_variations.len())).size(12.0).color(MUTED));});});
            let labels=self.analysis_variations.iter().enumerate().map(|(rank,v)|v.label(rank)).collect::<Vec<_>>();
            let mut selected=self.selected_variation.min(labels.len().saturating_sub(1));
            let selector=ui.add_enabled_ui(labels.len()>1,|ui|egui::ComboBox::from_id_salt("engine_prediction_variation_dock")
                .width(ui.available_width()).truncate().selected_text(RichText::new(labels.get(selected).map(String::as_str).unwrap_or("Waiting for engine…")).color(GOLD))
                .show_ui(ui,|ui|{for (i,label) in labels.iter().enumerate(){ui.selectable_value(&mut selected,i,label);}})).inner;
            #[cfg(test)]
            ui.ctx().data_mut(|d|d.insert_temp(egui::Id::new("prediction_selector_bounds"),selector.response.rect));
            #[cfg(not(test))]
            let _=selector;
            if selected!=self.selected_variation {self.prediction_index=0;self.prediction_navigation_active=true;self.select_analysis_variation(selected);}
            ui.add_space(4.0);
            let number_width=24.0;
            let gap=8.0;
            let col=((ui.available_width()-number_width-2.0*gap)/2.0).max(40.0);
            ui.horizontal(|ui|{ui.spacing_mut().item_spacing.x=gap;for (label,width) in [("#",number_width),("White",col),("Black",col)] {ui.add_sized([width,22.0],egui::Label::new(RichText::new(label).size(12.0).color(MUTED)));}});
            ui.separator();
            egui::ScrollArea::vertical().id_salt("prediction_move_rows").max_height(228.0).min_scrolled_height(228.0).auto_shrink([false,false]).show_gold(ui,|ui| {
                if self.prediction_moves.is_empty(){ui.label(RichText::new("Analyze this position to see its continuation.").size(12.0).color(MUTED));}
                let col=((ui.available_width()-number_width-2.0*gap)/2.0).max(40.0);
                egui::Grid::new("prediction_move_table").num_columns(3).min_col_width(0.0).spacing(Vec2::new(gap,2.0)).show(ui,|ui| {
                    let black_first=self.prediction_positions.first().is_some_and(|b|b.side_to_move()==Color::Black);
                    for (i,san) in self.prediction_moves.iter().enumerate() {
                        let ply=i+usize::from(black_first);
                        if ply%2==0||i==0 {
                            ui.add_sized([number_width,34.0],egui::Label::new(RichText::new(format!("{}.",ply/2+1)).color(MUTED)));
                            if black_first&&i==0 {ui.add_sized([col,34.0],egui::Label::new("…"));}
                        }
                        let selected=self.prediction_index==i+1;
                        let mut response=ui.add_sized([col,34.0],egui::Button::selectable(selected,self.display_san(san)).frame(selected));
                        if self.show_move_hover_text {if let (Some(board),Some(mv))=(self.prediction_positions.get(i),self.prediction_chess_moves.get(i)) {response=response.on_hover_text(Self::move_hover_text(board,*mv));}}
                        if response.clicked(){jump=Some(i+1);}
                        if selected&&self.prediction_scroll_to_selected {response.scroll_to_me(Some(Align::Center));}
                        if ply%2==1||i+1==self.prediction_moves.len(){ui.end_row();}
                    }
                });
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x=5.0;
                let width=((ui.available_width()-20.0-28.0)/5.0).max(20.0);
                for (n,(kind,dest,enabled,label)) in [(4,0,self.prediction_index>0,"Start of line"),(5,self.prediction_index.saturating_sub(1),self.prediction_index>0,"Previous move"),
                    (6,(self.prediction_index+1).min(self.prediction_moves.len()),self.prediction_index<self.prediction_moves.len(),"Next move"),(7,self.prediction_moves.len(),self.prediction_index<self.prediction_moves.len(),"End of line")].into_iter().enumerate() {
                    if n==2 {
                        let playing=self.playback_target==Some(true);
                        let response=ui.add_enabled_ui(!self.prediction_moves.is_empty(),|ui|Self::prediction_console_text_button(ui,if playing{"Pause"}else{"Play"},if playing{9}else{2},width+28.0,34.0)).inner;
                        self.playback_speed_menu(&response);
                        if response.on_hover_text("Autoplay · right-click to choose speed").clicked(){self.toggle_playback(true,ui.input(|i|i.time));}
                    }
                    let response=ui.add_enabled(enabled,egui::Button::new("").min_size(Vec2::new(width,34.0)));
                    Self::prediction_console_icon(ui,response.rect,kind,if enabled{Color32::from_rgb(231,233,226)}else{Color32::from_gray(90)});
                    response.widget_info(||egui::WidgetInfo::labeled(egui::WidgetType::Button,enabled,label));
                    if response.on_hover_text(label).clicked(){jump=Some(dest);}
                }
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new(if self.prediction_index==0{"Line start".into()}else{format!("Move {} of {}",self.prediction_index,self.prediction_moves.len())}).size(12.0).color(MUTED));
                ui.with_layout(Layout::right_to_left(Align::Center),|ui| {
                    if ui.add_enabled_ui(!self.prediction_moves.is_empty(),|ui|Self::prediction_console_text_button(ui,"Expand",10,86.0,30.0)).inner.on_hover_text("Show each predicted move on a mini board").clicked(){self.open_prediction_window();}
                });
            });
            (layer,ui.min_rect())
        });
        let texture=Self::paper_grain_texture(ui.ctx());let surface=panel.response.rect.shrink(4.0);
        ui.painter().set(panel.inner.0,egui::Shape::image(texture.id(),surface,egui::Rect::from_min_max(egui::Pos2::ZERO,egui::pos2(surface.width()/256.0,surface.height()/256.0)),Color32::WHITE));
        Self::paint_sunken_border(ui,panel.response.rect);
        #[cfg(test)]
        ui.ctx().data_mut(|d|d.insert_temp(egui::Id::new("prediction_panel_bounds"),panel.response.rect));
        self.prediction_scroll_to_selected=false;
        if let Some(destination)=jump {self.prediction_to(destination);}
        if self.prediction_navigation_active {self.wheel_move_navigation(ui,panel.response.rect);}
    }
}
