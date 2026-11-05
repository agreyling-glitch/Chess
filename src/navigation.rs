use super::*;

const RAIL_WIDTH: f32 = 56.0;
const DRAWER_WIDTH: f32 = 288.0;
const COLLAPSE_DELAY: f64 = 0.25;
const SHORTCUTS: [&str;8] = ["New game","Analysis","Saved games","Lichess puzzles","Save current game","Lichess Profiles","Board Theme","Study"];
const SHORTCUT_KEY: &str = "ironwood.quick-access.v1";

#[derive(Clone,Debug,PartialEq,serde::Serialize,serde::Deserialize)]
struct QuickAccess {
    enabled: [bool;8],
    order: [usize;8],
}
impl Default for QuickAccess {
    fn default()->Self {Self{enabled:[true,true,true,true,true,false,false,false],order:[0,1,2,3,4,5,6,7]}}
}
impl QuickAccess {
    fn parse(value:&str)->Option<Self> {
        let config:Self=serde_json::from_str(value).ok()?;
        let mut order=config.order;order.sort_unstable();
        (order==[0,1,2,3,4,5,6,7]).then_some(config)
    }
}
const SECTIONS: [&str; 7] = [
    "Play",
    "Analysis",
    "Training",
    "Game library",
    "Settings",
    "Tools",
    "Help",
];

#[derive(Default)]
pub(super) struct NavigationState {
    expanded: bool,
    pinned: bool,
    section: usize,
    last_inside: f64,
    keyboard: bool,
    suppress_hover: bool,
    touch_open: bool,
    shortcuts: QuickAccess,
    shortcuts_loaded: bool,
}

impl NavigationState {
    fn should_collapse(&self, now: f64, inside: bool, focused: bool) -> bool {
        self.expanded
            && !self.pinned
            && !self.touch_open
            && !inside
            && !focused
            && now - self.last_inside >= COLLAPSE_DELAY
    }
}

// Original geometric icons: no font glyph or external asset dependency.
fn icon(ui: &mut egui::Ui, index: usize, label: &str, selected: bool) -> egui::Response {
    let response = ui.add_sized(
        [40.0, 40.0],
        egui::Button::new("").frame(false).selected(selected),
    );
    let p = ui.painter();
    let c = response.rect.center();
    let color = if selected {
        Color32::from_rgb(223, 190, 119)
    } else {
        ui.visuals().text_color()
    };
    let stroke = Stroke::new(1.6, color);
    let point = |x: f32, y: f32| c + Vec2::new(x, y);
    let line =
        |a: (f32, f32), b: (f32, f32)| p.line_segment([point(a.0, a.1), point(b.0, b.1)], stroke);
    match index {
        0 => {
            p.add(egui::Shape::closed_line(
                vec![point(-5.0, -8.0), point(8.0, 0.0), point(-5.0, 8.0)],
                stroke,
            ));
        }
        1 => {
            p.circle_stroke(c, 9.0, stroke);
            p.circle_stroke(c, 4.0, stroke);
        }
        2 => {
            for x in [-8.0, -2.0, 5.0] {
                line((x, -8.0), (x, 8.0));
            }
            line((-10.0, 9.0), (10.0, 9.0));
        }
        3 => {
            for (y, x) in [(-7.0, -3.0), (0.0, 4.0), (7.0, -1.0)] {
                line((-9.0, y), (9.0, y));
                p.circle_filled(point(x, y), 2.7, color);
            }
        }
        4 => {
            line((-7.0, 8.0), (6.0, -5.0));
            line((1.0, -8.0), (6.0, -5.0));
            line((6.0, -5.0), (9.0, 0.0));
        }
        5 => {
            p.circle_stroke(c, 9.0, stroke);
            p.text(
                c,
                Align2::CENTER_CENTER,
                "?",
                FontId::proportional(16.0),
                color,
            );
        }
        6 => {
            line((-8.0, -8.0), (-8.0, 8.0));
            line((-3.0, -6.0), (5.0, 0.0));
            line((5.0, 0.0), (-3.0, 6.0));
        }
        7 => {
            line((-8.0, 0.0), (8.0, 0.0));
            line((0.0, -8.0), (0.0, 8.0));
        }
        8 => {
            p.rect_stroke(
                egui::Rect::from_center_size(c, Vec2::splat(16.0)),
                2.0,
                stroke,
                egui::StrokeKind::Inside,
            );
            line((-4.0, -7.0), (-4.0, -1.0));
            line((-4.0, -1.0), (4.0, -1.0));
            line((4.0, -1.0), (4.0, -7.0));
        }
        9 => {
            line((-9.0, 9.0), (-9.0, -8.0));
            line((-9.0, 9.0), (10.0, 9.0));
            line((-6.0, 4.0), (-1.0, -2.0));
            line((-1.0, -2.0), (3.0, 1.0));
            line((3.0, 1.0), (9.0, -7.0));
        }
        10 => {
            p.circle_stroke(point(0.0,-5.0),4.0,stroke);
            p.add(egui::Shape::line(vec![point(-8.0,8.0),point(-7.0,3.0),point(-3.0,0.0),point(3.0,0.0),point(7.0,3.0),point(8.0,8.0)],stroke));
        }
        11 => {
            p.rect_stroke(egui::Rect::from_center_size(c,Vec2::splat(18.0)),2,stroke,egui::StrokeKind::Inside);
            for row in 0..3 {for col in 0..3 {if (row+col)%2==0 {p.rect_filled(egui::Rect::from_min_size(point(-8.0+col as f32*5.4,-8.0+row as f32*5.4),Vec2::splat(5.4)),0,color);}}}
        }
        12 => {
            p.add(egui::Shape::closed_line(vec![point(0.0,-6.0),point(-9.0,-8.0),point(-9.0,6.0),point(0.0,8.0),point(9.0,6.0),point(9.0,-8.0)],stroke));
            line((0.0,-6.0),(0.0,8.0));
        }
        _ => {}
    }
    if selected {
        p.line_segment(
            [
                response.rect.left_top() + Vec2::new(1.0, 9.0),
                response.rect.left_bottom() - Vec2::new(-1.0, 9.0),
            ],
            Stroke::new(3.0, color),
        );
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            ui.is_enabled(),
            selected,
            label,
        )
    });
    response.on_hover_text(label)
}

impl ChessApp {
    fn navigation_shortcut(&mut self,ui:&mut egui::Ui,index:usize)->egui::Response {
        let enabled=match index {
            1=>!self.training_live()&&!(self.fics_active&&self.fics_playing),
            2=>self.navigation_library_enabled(),3|7=>self.navigation_puzzles_enabled(),
            4=>!self.fics_active||self.fics_game_finished,_=>true,
        };
        let selected=match index {0=>self.new_game_dialog_open,7=>ui.ctx().data(|d|d.get_temp::<bool>(egui::Id::new("study_window_open"))).unwrap_or(false),_=>false};
        let response=ui.add_enabled_ui(enabled,|ui|icon(ui,[7,9,2,1,8,10,11,12][index],SHORTCUTS[index],selected)).inner;
        if response.clicked() {
            let ctx=ui.ctx().clone();
            match index {
                0=>{self.navigation.section=0;self.navigation_new_game(false);}
                1=>{self.navigation.section=1;self.navigation_open_analysis(&ctx);}
                2=>{self.navigation.section=3;self.navigation_open_library();}
                3=>{self.navigation.section=2;ctx.data_mut(|d|{d.insert_temp(egui::Id::new("puzzles_open"),true);d.insert_temp(egui::Id::new("navigation_action_selected"),"Lichess puzzles".to_owned());});}
                4=>self.save_current_game(),
                5=>{self.open_lichess_history();}
                6=>self.open_board_theme_dialog(),
                7=>{self.navigation_enter_study(&ctx);ctx.data_mut(|d|d.insert_temp(egui::Id::new("navigation_action_selected"),"Study".to_owned()));}
                _=>{}
            }
        }
        response
    }

    fn navigation_shortcut_settings(&mut self,ui:&mut egui::Ui)->bool {
        let gear=ui.add_sized([40.0,40.0],egui::Button::new("").frame(false)).on_hover_text("Choose and reorder shortcuts");
        gear.widget_info(||egui::WidgetInfo::labeled(egui::WidgetType::Button,true,"Choose and reorder shortcuts"));
        let c=gear.rect.center();let color=Color32::from_rgb(215,185,121);let stroke=Stroke::new(1.5,color);
        ui.painter().circle_stroke(c,6.0,stroke);ui.painter().circle_stroke(c,2.5,stroke);
        for n in 0..8 {let v=Vec2::angled(n as f32*std::f32::consts::TAU/8.0);ui.painter().line_segment([c+v*6.0,c+v*9.0],stroke);}
        let before=self.navigation.shortcuts.clone();
        let popup=egui::Popup::menu(&gear).close_behavior(egui::containers::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
            Self::stable_panel_button_style(ui);
            ui.label("Show shortcuts · change order");ui.separator();
            let order=self.navigation.shortcuts.order;
            let mut swap=None;
            for (position,&index) in order.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.allocate_ui_with_layout(Vec2::new(170.0,30.0),Layout::left_to_right(Align::Center),|ui| {
                        ui.set_min_width(170.0);
                        ui.checkbox(&mut self.navigation.shortcuts.enabled[index],SHORTCUTS[index]);
                    });
                    if ui.add_enabled(position>0,egui::Button::new("Up")).clicked(){swap=Some((position,position-1));}
                    if ui.add_enabled(position<7,egui::Button::new("Down")).clicked(){swap=Some((position,position+1));}
                });
            }
            if let Some((a,b))=swap {self.navigation.shortcuts.order.swap(a,b);}
        });
        if before!=self.navigation.shortcuts {
            #[cfg(target_arch="wasm32")]
            if let Some(storage)=web_sys::window().and_then(|w|w.local_storage().ok().flatten()) {
                if let Ok(json)=serde_json::to_string(&self.navigation.shortcuts){let _=storage.set_item(SHORTCUT_KEY,&json);}
            }
        }
        popup.is_some()
    }
    pub(super) fn navigation_action(
        ui: &mut egui::Ui,
        enabled: bool,
        label: &str,
    ) -> egui::Response {
        let response = ui.add_enabled(
            enabled,
            egui::Button::new("")
                .left_text(label)
                .frame(false)
                .wrap()
                .min_size(Vec2::new(ui.available_width(), 36.0)),
        );
        if response.clicked() {
            ui.ctx().data_mut(|d| {
                d.insert_temp(
                    egui::Id::new("navigation_action_selected"),
                    label.to_owned(),
                )
            });
        }
        response
    }

    fn navigation_game_mode(&self) -> &'static str {
        if self.puzzle_navigation_locked() {
            "Solving puzzles"
        } else if self.training_live() {
            "Training against AI"
        } else if self.study.is_some() && self.puzzle_context.data(|d|
            d.get_temp::<bool>(egui::Id::new("study_window_open"))).unwrap_or(false) {
            "Studying"
        } else if self.lichess.active {
            if self.fics_playing {
                "Playing on Lichess"
            } else {
                "Viewing Lichess game"
            }
        } else if self.fics_active {
            if self.fics_playing {
                "Playing on FICS"
            } else {
                "Viewing FICS game"
            }
        } else if self.review_index.is_some() {
            "Reviewing a game"
        } else if self.engine_enabled {
            "Playing against AI"
        } else {
            "Self-play"
        }
    }

    pub(super) fn navigation_enter_study(&mut self, ctx: &egui::Context) {
        self.navigation.section = 2;
        ctx.data_mut(|d| {
            d.insert_temp(egui::Id::new("puzzles_open"), false);
            d.insert_temp(egui::Id::new("study_window_open"), true);
        });
    }

    pub(super) fn navigation_enter_puzzles(&mut self,ctx:&egui::Context) {
        self.navigation.section=2;
        ctx.data_mut(|d|{d.insert_temp(egui::Id::new("study_window_open"),false);d.insert_temp(egui::Id::new("puzzles_open"),true);});
    }

    pub(super) fn navigation_ui(&mut self, root_ui: &mut egui::Ui) {
        if !self.navigation.shortcuts_loaded {
            #[cfg(target_arch="wasm32")]
            if let Some(config)=web_sys::window().and_then(|w|w.local_storage().ok().flatten())
                .and_then(|s|s.get_item(SHORTCUT_KEY).ok().flatten()).and_then(|s|QuickAccess::parse(&s)) {
                self.navigation.shortcuts=config;
            }
            self.navigation.shortcuts_loaded=true;
        }
        let ctx = root_ui.ctx().clone();
        let now = ctx.input(|i| i.time);
        // The release frame produces the click after the last finger has lifted.
        let touch = ctx.input(|i| {
            i.any_touches()
                || i.events
                    .iter()
                    .any(|event| matches!(event, egui::Event::Touch { .. }))
        });
        let mut focused = false;
        let mut rail_toggle = false;
        let mut shortcut_popup_open=false;
        let rail = egui::Panel::left("navigation_rail")
            .resizable(false)
            .exact_size(RAIL_WIDTH)
            .frame(
                Frame::new()
                    .fill(Color32::from_rgb(19, 24, 28))
                    .inner_margin(Margin::same(8)),
            )
            .show(root_ui, |ui| {
                let surface = ui.max_rect().expand(8.0);
                let grain = Self::paper_grain_texture(ui.ctx());
                let uv = egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(surface.width() / 256.0, surface.height() / 256.0));
                // Two layers match the heavier grain used on the app's outer surfaces.
                for _ in 0..2 {
                    ui.painter().with_clip_rect(surface).image(grain.id(), surface, uv, Color32::WHITE);
                }
                ui.visuals_mut().selection.bg_fill = Color32::from_rgb(66, 57, 35);
                ui.visuals_mut().selection.stroke =
                    Stroke::new(1.0, Color32::from_rgb(223, 190, 119));
                egui::ScrollArea::vertical()
                    .id_salt("navigation_rail_scroll")
                    .max_height((ui.available_height()-48.0).max(40.0))
                    .show(ui, |ui| {
                        let toggle = icon(
                            ui,
                            6,
                            "Expand or collapse navigation",
                            self.navigation.expanded,
                        );
                        focused |= toggle.has_focus();
                        rail_toggle = toggle.clicked();
                        ui.separator();
                        for (index, label) in SECTIONS.iter().enumerate() {
                            let response = icon(ui, [0, 9, 1, 2, 3, 4, 5][index], label, self.navigation.section == index);
                            focused |= response.has_focus();
                            if response.clicked() {
                                self.navigation.section = index;
                                self.navigation.expanded = true;
                                self.navigation.suppress_hover = false;
                                self.navigation.touch_open = touch;
                                self.navigation.last_inside = now;
                            }
                        }
                        ui.separator();
                        let config=self.navigation.shortcuts.clone();
                        for index in config.order {
                            if config.enabled[index] {
                                focused |= self.navigation_shortcut(ui,index).has_focus();
                            }
                        }
                    });
                ui.add_space((ui.available_height()-40.0).max(0.0));
                shortcut_popup_open=self.navigation_shortcut_settings(ui);
            })
            .response
            .rect;

        if ctx.input(|i| {
            i.events
                .iter()
                .any(|event| matches!(event, egui::Event::Key { pressed: true, .. }))
        }) {
            self.navigation.keyboard = true;
        }
        if ctx.input(|i| i.pointer.any_pressed()) {
            self.navigation.keyboard = false;
        }
        let pointer = ctx.input(|i| i.pointer.hover_pos());
        let in_rail = !touch && pointer.is_some_and(|p| rail.contains(p));
        if !in_rail {
            self.navigation.suppress_hover = false;
        }
        if rail_toggle {
            self.navigation.expanded = !self.navigation.expanded;
            self.navigation.touch_open = touch && self.navigation.expanded;
            self.navigation.last_inside = now;
            self.navigation.suppress_hover = !self.navigation.expanded;
            if !self.navigation.expanded {
                self.navigation.pinned = false;
            }
        } else if (in_rail && !self.navigation.suppress_hover && !shortcut_popup_open)
            || (focused && self.navigation.keyboard)
        {
            self.navigation.expanded = true;
            if in_rail {
                self.navigation.touch_open = false;
            }
        }
        if self.new_game_dialog_open {
            self.navigation.expanded = false;
        }
        if shortcut_popup_open { self.navigation.expanded=false; }
        let mut inside = in_rail;
        if self.navigation.expanded {
            let width = DRAWER_WIDTH.min((ctx.content_rect().width() - RAIL_WIDTH).max(180.0));
            let area = egui::Area::new(egui::Id::new("navigation_drawer"))
                .order(egui::Order::Foreground)
                .fixed_pos(rail.right_top())
                .default_size(Vec2::new(width, rail.height()))
                .movable(false)
                .show(&ctx, |ui| {
                    ui.visuals_mut().selection.bg_fill = Color32::from_rgb(66, 57, 35);
                    ui.visuals_mut().selection.stroke =
                        Stroke::new(1.0, Color32::from_rgb(223, 190, 119));
                    Frame::new()
                        .fill(Color32::from_rgb(19, 24, 28))
                        .stroke(Stroke::new(1.0, Color32::from_rgb(65, 69, 65)))
                        .inner_margin(Margin::same(12))
                        .show(ui, |ui| {
                            ui.set_width(width - 24.0);
                            ui.set_min_height((rail.height() - 24.0).max(120.0));
                            let surface = egui::Rect::from_min_size(ui.min_rect().min, Vec2::new(width - 24.0, (rail.height() - 24.0).max(120.0))).expand(12.0);
                            let grain = Self::paper_grain_texture(ui.ctx());
                            ui.painter().with_clip_rect(surface).image(grain.id(), surface,
                                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(surface.width() / 256.0, surface.height() / 256.0)),
                                Color32::from_white_alpha(90));
                            ui.horizontal(|ui| {
                                ui.strong("Navigation");
                                let pin = ui
                                    .selectable_label(self.navigation.pinned, "Pin")
                                    .on_hover_text("Keep navigation expanded");
                                focused |= pin.has_focus();
                                if pin.clicked() {
                                    self.navigation.pinned = !self.navigation.pinned;
                                }
                                let close = ui.button("Close");
                                focused |= close.has_focus();
                                if close.clicked() {
                                    self.navigation.expanded = false;
                                    self.navigation.pinned = false;
                                    self.navigation.suppress_hover = true;
                                }
                            });
                            ui.separator();
                            egui::ScrollArea::vertical()
                                .id_salt("navigation_drawer_scroll")
                                .max_height((rail.height() - 80.0).max(120.0))
                                .show(ui, |ui| {
                                    ui.style_mut().spacing.interact_size.y = 36.0;
                                    for (index, label) in SECTIONS.iter().enumerate() {
                                        let response = ui.add_sized(
                                            [ui.available_width(), 36.0],
                                            egui::Button::new("")
                                                .left_text(*label)
                                                .frame(false)
                                                .selected(self.navigation.section == index),
                                        );
                                        focused |= response.has_focus();
                                        if response.clicked() {
                                            self.navigation.section = index;
                                        }
                                        if self.navigation.section == index {
                                            ui.indent(index, |ui| {
                                                ui.vertical(|ui| match index {
                                                    0 => self.navigation_play(ui),
                                                    1 => self.navigation_analysis(ui),
                                                    2 => self.navigation_training(ui),
                                                    3 => self.navigation_library(ui),
                                                    4 => self.navigation_settings(ui),
                                                    5 => self.navigation_tools(ui),
                                                    _ => self.navigation_help(ui),
                                                });
                                            });
                                        }
                                    }
                                });
                        });
                });
            inside |= !touch && pointer.is_some_and(|p| area.response.rect.contains(p));
            // Descendant controls retain keyboard focus while the pointer is elsewhere.
            focused |= self.navigation.keyboard
                && ctx.memory(|m| m.focused()).is_some_and(|id| {
                    ctx.read_response(id)
                        .is_some_and(|r| area.response.rect.contains(r.rect.center()))
                });
            if ctx.input(|i| i.key_pressed(egui::Key::Escape))
                || (!self.navigation.pinned
                    && ctx.input(|i| i.pointer.any_pressed())
                    && !pointer.is_some_and(|p| rail.contains(p) || area.response.rect.contains(p)))
            {
                self.navigation.expanded = false;
                self.navigation.pinned = false;
                self.navigation.suppress_hover = true;
                if let Some(id) = ctx.memory(|m| m.focused()) {
                    ctx.memory_mut(|m| m.surrender_focus(id));
                }
            }
        }
        if inside || (focused && self.navigation.keyboard) {
            self.navigation.last_inside = now;
        }
        if self
            .navigation
            .should_collapse(now, inside, focused && self.navigation.keyboard)
        {
            self.navigation.expanded = false;
        }
        if ctx
            .data_mut(|d| d.remove_temp::<String>(egui::Id::new("navigation_action_selected")))
            .is_some()
        {
            self.navigation.expanded = false;
            self.navigation.suppress_hover = true;
            self.navigation.touch_open = false;
        }
        if self.navigation.expanded
            && !inside
            && !self.navigation.pinned
            && !self.navigation.touch_open
        {
            ctx.request_repaint_after(std::time::Duration::from_millis(260));
        }
    }

    pub(super) fn navigation_new_game(&mut self, both_sides: bool) {
        self.puzzle_context.data_mut(|d| {
            d.insert_temp(
                egui::Id::new("navigation_action_selected"),
                "New game".to_owned(),
            )
        });
        self.new_game_training = false;
        self.new_game_online = false;
        self.new_game_lichess = false;
        self.new_game_both_sides = both_sides;
        self.new_game_engine_draft = Some(self.engine_config.clone());
        self.new_game_opponent = self.engine_config.opponent;
        self.prepare_new_game_strength();
        self.new_game_color = match self.player_side {
            PlayerSide::White => NewGameColor::White,
            PlayerSide::Black => NewGameColor::Black,
        };
        self.new_game_dialog_open = true;
    }

    pub(super) fn navigation_puzzles_enabled(&self) -> bool {
        !self.training_live() && !(self.fics_active && self.fics_playing)
    }
    pub(super) fn navigation_library_enabled(&self) -> bool {
        !(self.lichess.active
            && self.lichess.game["speed"].as_str() == Some("correspondence")
            && self.fics_pending_move)
    }
    pub(super) fn navigation_open_library(&mut self) {
        if !self.navigation_library_enabled() {
            return;
        }
        self.puzzle_context.data_mut(|d| {
            d.insert_temp(
                egui::Id::new("navigation_action_selected"),
                "Saved games".to_owned(),
            )
        });
        if self.lichess.active
            && self.lichess.game["speed"].as_str() == Some("correspondence")
            && self.fics_playing
        {
            lichess_command(serde_json::json!({"type":"leaveCorrespondence"}));
        } else if self.fics_active && (!self.lichess.active || !self.fics_game_finished) {
            self.stop_fics();
        }
        #[cfg(target_arch = "wasm32")]
        open_game_library();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shortcut_defaults_and_saved_custom_order_are_preserved() {
        let mut config=QuickAccess::default();
        assert_eq!(config.enabled,[true,true,true,true,true,false,false,false]);
        config.enabled[5]=true;config.enabled[0]=false;config.order.swap(0,7);
        let json=serde_json::to_string(&config).unwrap();
        assert_eq!(QuickAccess::parse(&json),Some(config.clone()));
        config.order[0]=config.order[1];
        assert!(QuickAccess::parse(&serde_json::to_string(&config).unwrap()).is_none());
        assert!(QuickAccess::parse("{}").is_none());
        assert!(QuickAccess::parse("not JSON").is_none());
    }

    #[test]
    fn opening_a_study_selects_training_and_reports_study_mode() {
        let ctx = egui::Context::default();
        let mut app = ChessApp::new(&eframe::CreationContext::_new_kittest(ctx.clone()));
        app.open_study("[Event \"Study\"]\n[StudyName \"Test study\"]\n[ChapterName \"First chapter\"]\n[ChapterURL \"https://lichess.org/study/abcdefgh/12345678\"]\n\n1. e4 e5 *").unwrap();
        app.navigation_enter_study(&ctx);
        assert_eq!(app.navigation.section, 2);
        assert_eq!(app.navigation_game_mode(), "Studying");
        assert_eq!(ctx.data(|d|d.get_temp::<bool>(egui::Id::new("study_window_open"))), Some(true));
        assert_eq!(ctx.data(|d|d.get_temp::<bool>(egui::Id::new("puzzles_open"))), Some(false));
    }

    #[test]
    fn hover_exit_has_grace_period_and_respects_pin_and_focus() {
        let mut state = NavigationState {
            expanded: true,
            last_inside: 1.0,
            ..Default::default()
        };
        assert!(!state.should_collapse(1.1, false, false));
        assert!(state.should_collapse(1.3, false, false));
        assert!(!state.should_collapse(1.3, true, false));
        assert!(!state.should_collapse(1.3, false, true));
        state.pinned = true;
        assert!(!state.should_collapse(10.0, false, false));
        state.pinned = false;
        state.touch_open = true;
        assert!(!state.should_collapse(10.0, false, false));
    }

    #[test]
    fn drawer_hover_and_pin_never_change_board_space() {
        for width in [320.0, 768.0, 1440.0] {
            let ctx = egui::Context::default();
            let mut app = ChessApp::new(&eframe::CreationContext::_new_kittest(ctx.clone()));
            let mut baseline = None;
            for frame in 0..7 {
                let pos = if (1..=3).contains(&frame) {
                    egui::pos2(25.0, 80.0)
                } else {
                    egui::pos2(-10.0, 650.0)
                };
                if frame == 3 {
                    app.navigation.pinned = true;
                }
                if frame == 5 {
                    app.navigation.pinned = false;
                }
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            Vec2::new(width, 720.0),
                        )),
                        time: Some(frame as f64),
                        events: vec![egui::Event::PointerMoved(pos)],
                        ..Default::default()
                    },
                    |ui| {
                        app.navigation_ui(ui);
                        let remaining = ui.available_rect_before_wrap();
                        if let Some(rect) = baseline {
                            assert_eq!(
                                remaining, rect,
                                "Drawer moved the workspace at width {width}"
                            );
                        } else {
                            baseline = Some(remaining);
                        }
                    },
                );
                output.textures_delta.clear();
                assert_eq!(
                    app.navigation.expanded,
                    (1..=4).contains(&frame),
                    "Unexpected drawer state at frame {frame}, width {width}"
                );
            }
        }
    }

    #[test]
    fn touch_toggle_stays_open_after_finger_release_and_dismisses_outside() {
        let ctx = egui::Context::default();
        let mut app = ChessApp::new(&eframe::CreationContext::_new_kittest(ctx.clone()));
        let pos = egui::pos2(25.0, 25.0);
        for frame in 0..5 {
            let mut events = Vec::new();
            if frame == 1 || frame == 2 {
                events.push(egui::Event::Touch {
                    device_id: egui::TouchDeviceId(1),
                    id: egui::TouchId(1),
                    phase: if frame == 1 {
                        egui::TouchPhase::Start
                    } else {
                        egui::TouchPhase::End
                    },
                    pos,
                    force: None,
                });
                events.push(egui::Event::PointerMoved(pos));
                events.push(egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: frame == 1,
                    modifiers: egui::Modifiers::NONE,
                });
                if frame == 2 {
                    events.push(egui::Event::PointerGone);
                }
            }
            if frame == 4 {
                let outside = egui::pos2(700.0, 600.0);
                events.push(egui::Event::PointerMoved(outside));
                events.push(egui::Event::PointerButton {
                    pos: outside,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                });
            }
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(900.0, 720.0),
                    )),
                    time: Some(if frame < 3 {
                        frame as f64 * 0.1
                    } else {
                        frame as f64
                    }),
                    events,
                    ..Default::default()
                },
                |ui| app.navigation_ui(ui),
            );
            output.textures_delta.clear();
            if frame == 2 || frame == 3 {
                assert!(
                    app.navigation.expanded,
                    "Touch drawer closed at frame {frame}"
                );
            }
            if frame == 4 {
                assert!(!app.navigation.expanded);
            }
        }
    }

    #[test]
    fn keyboard_focus_opens_navigation_and_escape_keeps_it_closed() {
        let ctx = egui::Context::default();
        let mut app = ChessApp::new(&eframe::CreationContext::_new_kittest(ctx.clone()));
        for frame in 0..6 {
            let key = match frame {
                1 => Some(egui::Key::Tab),
                3 => Some(egui::Key::Escape),
                _ => None,
            };
            let events = key
                .into_iter()
                .map(|key| egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                })
                .collect();
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(900.0, 720.0),
                    )),
                    time: Some(frame as f64),
                    events,
                    ..Default::default()
                },
                |ui| app.navigation_ui(ui),
            );
            output.textures_delta.clear();
            if frame == 2 {
                assert!(app.navigation.expanded);
            }
            if frame >= 3 {
                assert!(
                    !app.navigation.expanded,
                    "Escape reopened the drawer at frame {frame}"
                );
            }
        }
    }

    #[test]
    fn changing_navigation_does_not_change_game_mode_or_bypass_action_locks() {
        let ctx = egui::Context::default();
        let mut app = ChessApp::new(&eframe::CreationContext::_new_kittest(ctx));
        app.lichess.active = true;
        app.fics_playing = true;
        app.fics_active = true;
        app.lichess.game = serde_json::json!({"speed":"correspondence"});
        app.fics_pending_move = true;
        app.navigation.section = 3;
        assert_eq!(app.navigation_game_mode(), "Playing on Lichess");
        assert!(!app.navigation_library_enabled());
        assert!(!app.navigation_puzzles_enabled());
        app.navigation_open_library();
        assert!(app.lichess.active && app.fics_active);
        app.fics_pending_move = false;
        assert!(app.navigation_library_enabled());
    }
}
