use super::*;

pub(super) const RAIL_WIDTH: f32 = 56.0;
pub(super) const DRAWER_WIDTH: f32 = 288.0;
const SETTINGS_SECTION: usize = 4;
const SHORTCUTS: [&str;8] = ["New game","Analysis","Saved games","Lichess puzzles","Save current game","Lichess Profiles","Board Theme","Study"];
// Keep the original indices so saved visibility and order still apply.
const QUICK_ACCESS_INDICES: [usize; 3] = [3, 5, 7];
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
    pub(super) expanded: bool,
    pub(super) section: usize,
    pub(super) game_setup_attached: bool,
    pub(super) training_setup_attached: bool,
    pending_switch: Option<ModeSwitch>,
    waiting_for_resignation: bool,
    pub(super) local_game_started: bool,
    shortcuts: QuickAccess,
    shortcuts_loaded: bool,
}

#[derive(Clone, Copy)]
pub(super) enum ModeSwitch {
    Setup { self_play: bool, attached: bool },
    OnlineLobby { lichess: bool },
    Analysis,
    Library,
    Training,
    Puzzles,
    Study,
    Import,
    Settings(SettingsPage),
    #[cfg(target_arch = "wasm32")]
    Home,
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
            line((1.0, -8.0), (-8.0, -8.0));
            line((-8.0, -8.0), (-8.0, 8.0));
            line((-8.0, 8.0), (1.0, 8.0));
            line((-3.0, 0.0), (8.0, 0.0));
            line((3.0, -5.0), (8.0, 0.0));
            line((8.0, 0.0), (3.0, 5.0));
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
        13 => {
            line((0.0, -9.0), (0.0, 3.0));
            line((-4.0, -1.0), (0.0, 3.0));
            line((0.0, 3.0), (4.0, -1.0));
            line((-8.0, 2.0), (-8.0, 8.0));
            line((-8.0, 8.0), (8.0, 8.0));
            line((8.0, 8.0), (8.0, 2.0));
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
    pub(super) fn playing_game_before_mode_switch(&self) -> bool {
        self.board_game_actions_available()
            && (self.fics_active
                || (!self.analysis_workspace_active(&self.puzzle_context)
                    && !self.training_live()
                    && !self.puzzle_mode()
                    && (self.navigation.local_game_started
                        || self.local_clock.is_some()
                        || !self.review_moves.is_empty())))
    }

    pub(super) fn request_mode_switch(&mut self, destination: ModeSwitch) {
        if self.navigation.waiting_for_resignation {
            return;
        }
        if self.playing_game_before_mode_switch() {
            self.navigation.pending_switch = Some(destination);
        } else {
            self.enter_mode(destination);
        }
    }

    fn enter_mode(&mut self, destination: ModeSwitch) {
        let ctx = self.puzzle_context.clone();
        self.navigation.expanded = false;
        match destination {
            ModeSwitch::Setup { self_play, attached } => {
                self.navigation.section = 0;
                self.navigation.expanded = attached;
                self.navigation_new_game(self_play);
                self.navigation.game_setup_attached = attached;
            }
            ModeSwitch::OnlineLobby { lichess } => self.open_online_lobby(lichess),
            ModeSwitch::Analysis => self.navigation_open_analysis(&ctx),
            ModeSwitch::Library => self.navigation_open_library(),
            ModeSwitch::Training => {
                self.settle_training();
                self.navigation.section = 2;
                self.navigation.expanded = true;
                self.navigation.training_setup_attached = true;
                self.training_dialog_open = true;
            }
            ModeSwitch::Puzzles => self.navigation_enter_puzzles(&ctx),
            ModeSwitch::Study => self.navigation_enter_study(&ctx),
            ModeSwitch::Import => {
                if self.fics_active {
                    self.stop_fics();
                }
                #[cfg(target_arch = "wasm32")]
                ensure_imported_index();
                self.import_input.clear();
                self.pgn_dialog_open = true;
                self.pgn_error = None;
            }
            ModeSwitch::Settings(page) => self.show_settings_page(page),
            #[cfg(target_arch = "wasm32")]
            ModeSwitch::Home => Self::exit_to_home(),
        }
    }

    fn open_navigation_settings(&mut self) {
        self.open_settings(SettingsPage::Appearance);
        self.navigation.expanded = false;
    }

    fn navigation_shortcut(&mut self,ui:&mut egui::Ui,index:usize)->egui::Response {
        let enabled=match index {
            3|7=>self.navigation_puzzles_enabled(),
            4=>!self.fics_active||self.fics_game_finished,_=>true,
        } || (matches!(index, 3 | 7) && self.playing_game_before_mode_switch());
        let selected = index == 7 && ui.ctx().data(|d| d.get_temp::<bool>(egui::Id::new("study_window_open"))).unwrap_or(false);
        let response=ui.add_enabled_ui(enabled,|ui|icon(ui,[7,9,2,1,8,10,11,12][index],SHORTCUTS[index],selected)).inner;
        if response.clicked() {
            match index {
                3 => self.request_mode_switch(ModeSwitch::Puzzles),
                4=>self.save_current_game(),
                5=>{self.open_lichess_history();}
                7 => self.request_mode_switch(ModeSwitch::Study),
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
        if gear.clicked() { self.open_settings(SettingsPage::QuickAccess); }
        gear.clicked()
    }

    pub(super) fn quick_access_settings(&mut self, ui: &mut egui::Ui) {
        let before = self.navigation.shortcuts.clone();
        Self::stable_panel_button_style(ui);
        ui.label("Show shortcuts · change order");ui.separator();
        let order: Vec<_> = self.navigation.shortcuts.order.into_iter().enumerate()
            .filter(|(_, index)| QUICK_ACCESS_INDICES.contains(index)).collect();
        let mut swap=None;
        for (position, &(_, index)) in order.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(Vec2::new(170.0,30.0),Layout::left_to_right(Align::Center),|ui| {
                    ui.set_min_width(170.0);
                    ui.checkbox(&mut self.navigation.shortcuts.enabled[index],SHORTCUTS[index]);
                });
                if ui.add_enabled(position>0,egui::Button::new("Up")).clicked(){swap=Some((position,position-1));}
                if ui.add_enabled(position+1<order.len(),egui::Button::new("Down")).clicked(){swap=Some((position,position+1));}
            });
        }
        if let Some((a,b))=swap {self.navigation.shortcuts.order.swap(order[a].0,order[b].0);}
        if before!=self.navigation.shortcuts {
            #[cfg(target_arch="wasm32")]
            if let Some(storage)=web_sys::window().and_then(|w|w.local_storage().ok().flatten()) {
                if let Ok(json)=serde_json::to_string(&self.navigation.shortcuts){let _=storage.set_item(SHORTCUT_KEY,&json);}
            }
        }
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
        let active_mode = self.active_mode_index(&ctx);
        let mut open_settings = false;
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
                        for (index, label) in SECTIONS.iter().enumerate() {
                            let response = icon(ui, [0, 9, 1, 2, 3, 4, 5][index], label, active_mode == index);
                            if response.clicked() {
                                if index == 1 {
                                    self.request_mode_switch(ModeSwitch::Analysis);
                                    continue;
                                }
                                if index == 3 {
                                    self.request_mode_switch(ModeSwitch::Library);
                                    continue;
                                }
                                if index == SETTINGS_SECTION {
                                    open_settings = true;
                                    continue;
                                }
                                self.navigation.expanded =
                                    !self.navigation.expanded || self.navigation.section != index;
                                self.navigation.section = index;
                            }
                            if index == 0 {
                                self.navigation_shortcut(ui, 4);
                            }
                        }
                        if icon(ui, 13, "Import games", false).clicked() {
                            self.request_mode_switch(ModeSwitch::Import);
                        }
                        #[cfg(target_arch = "wasm32")]
                        {
                            let exit = icon(ui, 6, "Exit to home", false);
                            if exit.clicked() {
                                self.request_mode_switch(ModeSwitch::Home);
                            }
                        }
                        ui.separator();
                        let config = self.navigation.shortcuts.clone();
                        for index in config.order {
                            if QUICK_ACCESS_INDICES.contains(&index) && config.enabled[index] {
                                self.navigation_shortcut(ui, index);
                            }
                        }
                    });
                ui.add_space((ui.available_height()-40.0).max(0.0));
                shortcut_popup_open=self.navigation_shortcut_settings(ui);
            })
            .response
            .rect;

        if open_settings {
            self.open_navigation_settings();
        }
        let attached = self.new_game_dialog_open && self.navigation.game_setup_attached;
        if (self.new_game_dialog_open && !attached) || self.settings.open || shortcut_popup_open {
            self.navigation.expanded = false;
        }
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
                        .stroke(Stroke::new(1.0, Color32::from_rgb(133, 109, 65)))
                        .inner_margin(Margin::same(12))
                        .show(ui, |ui| {
                            ui.set_width(width - 26.0);
                            ui.set_min_height((rail.height() - 24.0).max(120.0));
                            let surface = egui::Rect::from_min_size(ui.min_rect().min, Vec2::new(width - 26.0, (rail.height() - 24.0).max(120.0))).expand(12.0);
                            let grain = Self::paper_grain_texture(ui.ctx());
                            ui.painter().with_clip_rect(surface).image(grain.id(), surface,
                                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(surface.width() / 256.0, surface.height() / 256.0)),
                                Color32::from_white_alpha(90));
                            egui::ScrollArea::vertical()
                                .id_salt("navigation_drawer_scroll")
                                .max_height((rail.height() - 26.0).max(120.0))
                                .show(ui, |ui| {
                                    ui.style_mut().spacing.interact_size.y = 36.0;
                                    for (index, label) in SECTIONS.iter().enumerate() {
                                        if index != self.navigation.section {
                                            continue;
                                        }
                                        let response = ui.add_sized(
                                            [ui.available_width(), 36.0],
                                            egui::Button::new("")
                                                .left_text(*label)
                                                .frame(false)
                                                .selected(self.navigation.section == index),
                                        );
                                        if response.clicked() {
                                            if index == SETTINGS_SECTION {
                                                self.open_navigation_settings();
                                            } else {
                                                self.navigation.section = index;
                                            }
                                        }
                                        if self.navigation.section == index {
                                            ui.indent(index, |ui| {
                                                ui.vertical(|ui| match index {
                                                    0 => self.navigation_play(ui),
                                                    2 => self.navigation_training(ui),
                                                    1 | 3 | SETTINGS_SECTION => {},
                                                    5 => self.navigation_tools(ui),
                                                    _ => self.navigation_help(ui),
                                                });
                                            });
                                        }
                                    }
                                });
                        });
                });
            let training_attached = self.training_dialog_open && self.navigation.training_setup_attached;
            let setup = if attached {
                ctx.memory(|memory| memory.area_rect(egui::Id::new("new_game_window")))
            } else if training_attached {
                ctx.memory(|memory| memory.area_rect(egui::Id::new("training_setup_panel")))
            } else {
                None
            };
            let training_child_open = training_attached && (
                egui::Popup::is_any_open(&ctx)
                    || self.profile_management_busy()
            );
            if !training_child_open && ctx.input(|input| {
                input.key_pressed(egui::Key::Escape)
                    || (input.pointer.any_pressed()
                        && input.pointer.interact_pos().is_some_and(|pos| {
                            !rail.contains(pos) && !area.response.rect.contains(pos)
                                && !setup.is_some_and(|rect| rect.contains(pos))
                        }))
            }) {
                self.navigation.expanded = false;
            }
        }
        if ctx
            .data_mut(|d| d.remove_temp::<String>(egui::Id::new("navigation_action_selected")))
            .is_some()
            && !(self.new_game_dialog_open && self.navigation.game_setup_attached)
            && !(self.training_dialog_open && self.navigation.training_setup_attached)
        {
            self.navigation.expanded = false;
        }
        if self.navigation.waiting_for_resignation && !self.playing_game_before_mode_switch() {
            self.navigation.waiting_for_resignation = false;
            if let Some(destination) = self.navigation.pending_switch.take() {
                self.enter_mode(destination);
            }
        }
        if let Some(destination) = self.navigation.pending_switch {
            let waiting = self.navigation.waiting_for_resignation;
            let warning = egui::Modal::new(egui::Id::new("resign_before_mode_switch"))
                .frame(Self::dialog_frame())
                .show(&ctx, |ui| {
                    ui.set_width(ui.ctx().content_rect().width().min(424.0).max(80.0) - 40.0);
                    let gold = Color32::from_rgb(215, 185, 121);
                    ui.label(RichText::new("GAME IN PROGRESS").small().strong().color(gold));
                    ui.add_space(4.0);
                    ui.label(RichText::new("Finish your game first").size(22.0).strong());
                    ui.add_space(10.0);
                    Self::textured_sunken_section(ui, true, |ui| {
                        ui.label(if waiting {
                            "Waiting for the game server to confirm your resignation."
                        } else {
                            "Resign the current game before switching modes."
                        });
                    });
                    ui.add_space(14.0);
                    ui.horizontal_wrapped(|ui| {
                        let stay = ui.button("Return to game").clicked();
                        let switch = ui.add_enabled(
                            !waiting,
                            egui::Button::new(
                                RichText::new("Resign and switch modes").strong()
                                    .color(Color32::from_rgb(24, 27, 31)),
                            )
                            .fill(gold)
                            .corner_radius(6.0)
                            .min_size(Vec2::new(130.0, 36.0)),
                        ).clicked();
                        (stay, switch)
                    }).inner
                });
            let (stay, switch) = warning.inner;
            if switch {
                if self.playing_game_before_mode_switch() {
                    self.resign_current_game();
                }
                if self.playing_game_before_mode_switch() {
                    self.navigation.waiting_for_resignation = true;
                } else {
                    self.navigation.pending_switch = None;
                    self.enter_mode(destination);
                }
            } else if stay || warning.should_close() {
                self.navigation.pending_switch = None;
                self.navigation.waiting_for_resignation = false;
            }
        }
    }
    pub(super) fn navigation_new_game(&mut self, both_sides: bool) {
        self.navigation.game_setup_attached = false;
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
    fn drawer_hover_stays_closed_and_never_changes_board_space() {
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
                    false,
                    "Unexpected drawer state at frame {frame}, width {width}"
                );
            }
        }
    }

    #[test]
    fn touch_play_stays_open_after_finger_release_and_dismisses_outside() {
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
    fn keyboard_focus_does_not_open_navigation() {
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
                assert!(!app.navigation.expanded);
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

    fn training_panel_app(ctx: &egui::Context) -> ChessApp {
        let mut app = ChessApp::new(&eframe::CreationContext::_new_kittest(ctx.clone()));
        app.training_profiles = Profiles {
            selected_id: "panel-profile".into(),
            profiles: vec![Profile {
                id: "panel-profile".into(),
                name: "Panel profile".into(),
                engine: training::TrainingEngine::Stockfish,
                starting_level: 1,
                starting_elo: training::START_RATING,
                results: vec![],
            }],
        };
        app
    }

    fn training_panel_frame(
        app: &mut ChessApp,
        ctx: &egui::Context,
        screen: egui::Rect,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        run_test_ui(ctx, egui::RawInput {
            screen_rect: Some(screen),
            events,
            ..Default::default()
        }, |ui| {
            app.navigation_ui(ui);
            app.training_dialog(ctx);
        })
    }

    fn training_panel_click(
        app: &mut ChessApp,
        ctx: &egui::Context,
        screen: egui::Rect,
        pos: egui::Pos2,
    ) {
        for pressed in [true, false] {
            training_panel_frame(app, ctx, screen, vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ]);
        }
    }

    #[test]
    fn training_panel_fills_height_and_stays_on_screen_at_each_layout() {
        for (width, height) in [(1440.0, 1000.0), (900.0, 420.0), (680.0, 600.0), (390.0, 600.0)] {
            let ctx = egui::Context::default();
            let mut app = training_panel_app(&ctx);
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(width, height));
            app.request_mode_switch(ModeSwitch::Training);
            for _ in 0..5 {
                training_panel_frame(&mut app, &ctx, screen, vec![]);
            }
            let panel = ctx.memory(|m| m.area_rect(egui::Id::new("training_setup_panel"))).unwrap();
            let expected_left = if width >= 760.0 { RAIL_WIDTH + DRAWER_WIDTH }
                else if width >= 600.0 { RAIL_WIDTH } else { 0.0 };
            assert!((panel.left() - expected_left).abs() < 1.0, "Unexpected left edge at {width}×{height}: {panel:?}");
            assert!((panel.top() - screen.top()).abs() < 1.0, "Unexpected top at {width}×{height}: {panel:?}");
            assert!((panel.bottom() - screen.bottom()).abs() < 1.0, "Unexpected bottom at {width}×{height}: {panel:?}");
            assert!(panel.right() <= screen.right() + 1.0, "Panel exceeds screen at {width}×{height}: {panel:?}");
            assert!(app.navigation.expanded && app.training_dialog_open && app.navigation.training_setup_attached);
        }
    }

    #[test]
    fn training_menu_action_and_panel_clicks_keep_the_menu_open() {
        let ctx = egui::Context::default();
        let mut app = training_panel_app(&ctx);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(1440.0, 1000.0));
        app.navigation.expanded = true;
        app.navigation.section = 2;
        let mut output = training_panel_frame(&mut app, &ctx, screen, vec![]);
        for _ in 0..3 {
            output = training_panel_frame(&mut app, &ctx, screen, vec![]);
        }
        let action = output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == "Train against Engine…" => {
                Some(text.galley.rect.translate(text.pos.to_vec2()).center())
            }
            _ => None,
        }).expect("Training menu action should be visible");
        training_panel_click(&mut app, &ctx, screen, action);
        assert!(app.navigation.expanded && app.training_dialog_open && app.navigation.training_setup_attached);
        for _ in 0..3 {
            training_panel_frame(&mut app, &ctx, screen, vec![]);
        }
        let panel = ctx.memory(|m| m.area_rect(egui::Id::new("training_setup_panel"))).unwrap();
        training_panel_click(&mut app, &ctx, screen, panel.left_bottom() + Vec2::new(8.0, -8.0));
        assert!(app.navigation.expanded && app.training_dialog_open && app.navigation.training_setup_attached);
        assert!(app.training.is_none(), "Opening and inspecting training must not start a game");
    }

    #[test]
    fn closing_or_switching_the_training_menu_closes_its_panel_without_changing_profiles() {
        for switch_section in [false, true] {
            let ctx = egui::Context::default();
            let mut app = training_panel_app(&ctx);
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(1440.0, 1000.0));
            let profiles = serde_json::to_string(&app.training_profiles).unwrap();
            let board = app.board;
            app.request_mode_switch(ModeSwitch::Training);
            for _ in 0..3 {
                training_panel_frame(&mut app, &ctx, screen, vec![]);
            }
            if switch_section {
                app.navigation.section = 0;
            } else {
                app.navigation.expanded = false;
            }
            training_panel_frame(&mut app, &ctx, screen, vec![]);
            assert!(!app.training_dialog_open && !app.navigation.training_setup_attached);
            assert_eq!(app.board, board);
            assert_eq!(serde_json::to_string(&app.training_profiles).unwrap(), profiles);
            assert!(app.training.is_none());
        }
    }

    #[test]
    fn starting_training_from_attached_panel_closes_menu_and_preserves_profile_progress() {
        let ctx = egui::Context::default();
        let mut app = training_panel_app(&ctx);
        app.request_mode_switch(ModeSwitch::Training);
        app.start_training_game();
        assert!(!app.training_dialog_open && !app.navigation.training_setup_attached && !app.navigation.expanded);
        let session = app.training.as_ref().expect("Training should have started");
        assert_eq!(session.profile_id, "panel-profile");
        assert_eq!(session.side, TrainingSide::White);
        assert!(app.training_profiles.selected().unwrap().results.is_empty());
    }

    fn training_text_position(output: &egui::FullOutput, label: &str) -> egui::Pos2 {
        output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == label => {
                let rect = text.galley.rect.translate(text.pos.to_vec2());
                shape.clip_rect.contains(rect.center()).then_some(rect.center())
            }
            _ => None,
        }).unwrap_or_else(|| panic!("Expected visible training control: {label}"))
    }

    #[test]
    fn training_primary_tabs_keep_the_panel_in_bounds_without_mutating_game_or_profiles() {
        for (width, height) in [(1440.0, 1000.0), (900.0, 420.0), (680.0, 600.0), (390.0, 600.0)] {
            let ctx = egui::Context::default();
            let mut app = training_panel_app(&ctx);
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(width, height));
            let board = app.board;
            let profiles = serde_json::to_string(&app.training_profiles).unwrap();
            app.request_mode_switch(ModeSwitch::Training);
            for _ in 0..4 {
                training_panel_frame(&mut app, &ctx, screen, vec![]);
            }
            let initial_panel = ctx.memory(|m| m.area_rect(egui::Id::new("training_setup_panel"))).unwrap();
            for (label, profiles_tab, expected_content) in [
                ("Profiles", true, "TRAINING PROFILE"),
                ("Train", false, "NEXT TRAINING GAME"),
            ] {
                let output = training_panel_frame(&mut app, &ctx, screen, vec![]);
                training_panel_click(&mut app, &ctx, screen, training_text_position(&output, label));
                for _ in 0..3 {
                    training_panel_frame(&mut app, &ctx, screen, vec![]);
                }
                let output = training_panel_frame(&mut app, &ctx, screen, vec![]);
                assert_eq!(ctx.data(|d| d.get_temp::<bool>(egui::Id::new("training_profiles_tab"))), Some(profiles_tab));
                assert!(output.shapes.iter().any(|shape| matches!(&shape.shape,
                    egui::Shape::Text(text) if text.galley.job.text == expected_content)), "Expected {label} content at {width}×{height}");
                let panel = ctx.memory(|m| m.area_rect(egui::Id::new("training_setup_panel"))).unwrap();
                assert!((panel.left() - initial_panel.left()).abs() < 1.0, "{label} moved the panel at {width}×{height}: {panel:?}");
                assert!((panel.top() - screen.top()).abs() < 1.0, "{label} top exceeds screen at {width}×{height}: {panel:?}");
                assert!((panel.bottom() - screen.bottom()).abs() < 1.0, "{label} must remain full height at {width}×{height}: {panel:?}");
                assert!(panel.right() <= screen.right() + 1.0, "{label} exceeds screen width at {width}×{height}: {panel:?}");
                assert!(app.navigation.expanded && app.training_dialog_open && app.navigation.training_setup_attached);
                assert_eq!(app.board, board);
                assert_eq!(serde_json::to_string(&app.training_profiles).unwrap(), profiles);
                assert!(app.training.is_none());
            }
        }
    }

    #[test]
    fn escape_closes_the_profiles_tab_and_its_menu_without_changing_game_or_progress() {
        for width in [1440.0, 390.0] {
            let ctx = egui::Context::default();
            let mut app = training_panel_app(&ctx);
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(width, 800.0));
            let board = app.board;
            let profiles = serde_json::to_string(&app.training_profiles).unwrap();
            app.request_mode_switch(ModeSwitch::Training);
            for _ in 0..4 {
                training_panel_frame(&mut app, &ctx, screen, vec![]);
            }
            let output = training_panel_frame(&mut app, &ctx, screen, vec![]);
            training_panel_click(&mut app, &ctx, screen, training_text_position(&output, "Profiles"));
            assert_eq!(ctx.data(|d| d.get_temp::<bool>(egui::Id::new("training_profiles_tab"))), Some(true));
            training_panel_frame(&mut app, &ctx, screen, vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }]);
            assert!(!app.training_dialog_open && !app.navigation.training_setup_attached && !app.navigation.expanded);
            assert_eq!(app.board, board);
            assert_eq!(serde_json::to_string(&app.training_profiles).unwrap(), profiles);
            assert!(app.training.is_none());
        }
    }

    #[test]
    fn selecting_a_profile_updates_the_train_tab_without_starting_a_game() {
        let ctx = egui::Context::default();
        let mut app = training_panel_app(&ctx);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(1440.0, 1000.0));
        let mut alternate = app.training_profiles.profiles[0].clone();
        alternate.id = "alternate-profile".into();
        alternate.name = "Alternate profile".into();
        let completed = alternate.session("completed-game".into(), "before".into());
        alternate.settle(&completed, "1-0", "after".into(), false);
        app.training_profiles.profiles.push(alternate);
        let progress = serde_json::to_string(&app.training_profiles.profiles).unwrap();
        let board = app.board;
        app.request_mode_switch(ModeSwitch::Training);
        for _ in 0..4 {
            training_panel_frame(&mut app, &ctx, screen, vec![]);
        }
        let output = training_panel_frame(&mut app, &ctx, screen, vec![]);
        training_panel_click(&mut app, &ctx, screen, training_text_position(&output, "Profiles"));
        for _ in 0..3 {
            training_panel_frame(&mut app, &ctx, screen, vec![]);
        }
        let output = training_panel_frame(&mut app, &ctx, screen, vec![]);
        training_panel_click(&mut app, &ctx, screen, training_text_position(&output, "Alternate profile"));
        assert_eq!(app.training_profiles.selected_id, "alternate-profile");
        let output = training_panel_frame(&mut app, &ctx, screen, vec![]);
        training_panel_click(&mut app, &ctx, screen, training_text_position(&output, "Train"));
        for _ in 0..3 {
            training_panel_frame(&mut app, &ctx, screen, vec![]);
        }
        let output = training_panel_frame(&mut app, &ctx, screen, vec![]);
        for label in ["Alternate profile · Stockfish", "♟ Black", "NEXT TRAINING GAME"] {
            training_text_position(&output, label);
        }
        assert_eq!(app.board, board);
        assert_eq!(serde_json::to_string(&app.training_profiles.profiles).unwrap(), progress);
        assert!(app.training.is_none());
        assert!(app.navigation.expanded && app.training_dialog_open);
    }

    #[test]
    fn choosing_a_different_train_profile_discards_the_previous_profiles_rename_draft() {
        let ctx = egui::Context::default();
        let mut app = training_panel_app(&ctx);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(1440.0, 1000.0));
        let mut alternate = app.training_profiles.profiles[0].clone();
        alternate.id = "alternate-profile".into();
        alternate.name = "Alternate profile".into();
        app.training_profiles.profiles.push(alternate);
        let profiles = serde_json::to_string(&app.training_profiles.profiles).unwrap();
        let board = app.board;
        app.request_mode_switch(ModeSwitch::Training);
        for _ in 0..4 {
            training_panel_frame(&mut app, &ctx, screen, vec![]);
        }
        let output = training_panel_frame(&mut app, &ctx, screen, vec![]);
        training_panel_click(&mut app, &ctx, screen, training_text_position(&output, "Profiles"));
        app.training_profile_rename = Some("Draft name for Panel profile".into());
        let output = training_panel_frame(&mut app, &ctx, screen, vec![]);
        training_panel_click(&mut app, &ctx, screen, training_text_position(&output, "Train"));
        assert!(app.training_profile_rename.is_some(), "Switching tabs alone should retain the current profile's draft");
        for _ in 0..3 {
            training_panel_frame(&mut app, &ctx, screen, vec![]);
        }
        let output = training_panel_frame(&mut app, &ctx, screen, vec![]);
        training_panel_click(&mut app, &ctx, screen, training_text_position(&output, "Panel profile · Stockfish"));
        for _ in 0..3 {
            training_panel_frame(&mut app, &ctx, screen, vec![]);
        }
        let output = training_panel_frame(&mut app, &ctx, screen, vec![]);
        training_panel_click(&mut app, &ctx, screen, training_text_position(&output, "Alternate profile"));
        assert_eq!(app.training_profiles.selected_id, "alternate-profile");
        assert!(app.training_profile_rename.is_none(), "A profile switch must discard the previous profile's rename draft");
        let output = training_panel_frame(&mut app, &ctx, screen, vec![]);
        training_panel_click(&mut app, &ctx, screen, training_text_position(&output, "Profiles"));
        let output = training_panel_frame(&mut app, &ctx, screen, vec![]);
        assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape,
            egui::Shape::Text(text) if text.galley.job.text == "Save name")));
        assert_eq!(serde_json::to_string(&app.training_profiles.profiles).unwrap(), profiles);
        assert_eq!(app.board, board);
        assert!(app.training.is_none());
    }
}
