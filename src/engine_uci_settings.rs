use super::*;

#[derive(Clone, serde::Deserialize)]
pub(super) struct UciOption {
    pub name: String,
    pub kind: String,
    pub default: String,
    pub min: Option<i64>,
    pub max: Option<i64>,
    #[serde(default)]
    pub choices: Vec<String>,
}
pub(super) fn parse_option(line: &str) -> Option<UciOption> {
    let (name, rest) = line.strip_prefix("option name ")?.split_once(" type ")?;
    if name.is_empty() || name.len() > 256 {
        return None;
    }
    let mut words = rest.split_whitespace();
    let kind = words.next()?.to_owned();
    if !matches!(
        kind.as_str(),
        "spin" | "check" | "combo" | "button" | "string"
    ) {
        return None;
    }
    let mut option = UciOption {
        name: name.into(),
        kind,
        default: String::new(),
        min: None,
        max: None,
        choices: vec![],
    };
    let words: Vec<_> = words.collect();
    let mut i = 0;
    while i < words.len() {
        let key = words[i];
        i += 1;
        let start = i;
        while i < words.len() && !matches!(words[i], "default" | "min" | "max" | "var") {
            i += 1;
        }
        let value = words[start..i].join(" ");
        match key {
            "default" => option.default = value,
            "min" => option.min = value.parse().ok(),
            "max" => option.max = value.parse().ok(),
            "var" => option.choices.push(value),
            _ => {}
        }
    }
    if option.default == "<empty>" {
        option.default.clear();
    }
    Some(option)
}
impl UciOption {
    fn editable(&self, remote: bool) -> bool {
        if !remote && self.kind == "string" {
            return false;
        }
        if matches!(self.name.as_str(), "Ponder" | "UCI_Chess960" | "MultiPV") {
            return false;
        }
        remote
            || !matches!(
                self.name.as_str(),
                "Threads"
                    | "Hash"
                    | "Skill Level"
                    | "UCI_LimitStrength"
                    | "UCI_Elo"
                    | "EvalFile"
                    | "EvalFileSmall"
                    | "SyzygyPath"
            )
    }
    fn valid(&self, value: &str) -> bool {
        if value.len() > 4096 || value.contains(['\r', '\n', '\0']) {
            return false;
        }
        match self.kind.as_str() {
            "spin" => value.parse::<i64>().is_ok_and(|n| {
                self.min.is_none_or(|min| n >= min) && self.max.is_none_or(|max| n <= max)
            }),
            "check" => matches!(value, "true" | "false"),
            "combo" => self.choices.iter().any(|c| c == value),
            "string" => true,
            _ => false,
        }
    }
}
// Discover rating controls from the selected engine, rather than its display name.
#[derive(Clone)]
pub(super) struct EloControl {
    options: Vec<UciOption>,
    toggle: Option<UciOption>,
    pub min: u32,
    pub max: u32,
    pub value: u32,
    pub limited: bool,
}
impl EloControl {
    fn discover(options: &[UciOption], values: &std::collections::BTreeMap<String,String>) -> Option<Self> {
        let primary=options.iter().find(|o|o.name=="Elo" && o.kind=="spin")
            .or_else(||options.iter().find(|o|o.name=="UCI_Elo" && o.kind=="spin"))?;
        let mut ratings=vec![primary.clone()];
        if primary.name=="Elo" {
            for name in ["SelfElo","OppoElo"] {
                if let Some(option)=options.iter().find(|o|o.name==name && o.kind=="spin") {ratings.push(option.clone());}
            }
        }
        let min=ratings.iter().map(|o|o.min.unwrap_or(0).max(0)).max()?;
        let max=ratings.iter().map(|o|o.max.unwrap_or(5000).min(u32::MAX as i64)).min()?;
        if min>max {return None;}
        let toggle=(primary.name=="UCI_Elo").then(||options.iter().find(|o|o.name=="UCI_LimitStrength" && o.kind=="check").cloned()).flatten();
        let current=if primary.name=="Elo" {values.get("SelfElo").or_else(||values.get(&primary.name))} else {values.get(&primary.name)};
        let value=current.unwrap_or(&primary.default).parse::<i64>().unwrap_or(1500).clamp(min,max) as u32;
        let limited=toggle.as_ref().is_none_or(|o|values.get(&o.name).unwrap_or(&o.default)=="true");
        Some(Self{options:ratings,toggle,min:min as u32,max:max as u32,value,limited})
    }
    fn write(&self, values:&mut std::collections::BTreeMap<String,String>, elo:u32, limited:bool) {
        let elo=elo.clamp(self.min,self.max).to_string();
        for option in &self.options {values.insert(option.name.clone(),elo.clone());}
        if let Some(option)=&self.toggle {values.insert(option.name.clone(),limited.to_string());}
    }
    pub(super) fn ui(&self, ui:&mut egui::Ui, elo:&mut u32, limited:&mut bool)->bool {
        ui.scope(|ui| {
            ui.spacing_mut().slider_width=(ui.available_width()-if self.toggle.is_some() {240.0} else {80.0}).clamp(60.0,280.0);
            let mut changed=false;
            if self.toggle.is_some() {changed|=ui.checkbox(limited,"Limit to Elo rating").changed();}
            else {*limited=true;}
            *elo=(*elo).clamp(self.min,self.max);
            changed|=ui.add_enabled(*limited,egui::Slider::new(elo,self.min..=self.max).suffix(" Elo")).changed();
            changed
        }).inner
    }
}

impl ChessApp {
    pub(super) fn companion_elo_control(&self,config:&EngineConfig)->Option<EloControl> {
        if config.opponent!=OpponentEngine::Companion {return None;}
        EloControl::discover(self.engine_uci_catalog.get(&Self::verification_engine_key(config))?,&config.companion_options)
    }
    pub(super) fn prepare_new_game_strength(&mut self) {
        if let Some(control)=self.companion_elo_control(&self.engine_config) {
            self.new_game_elo=control.value;
            self.new_game_limit_strength=control.limited;
        } else {
            self.new_game_elo=self.engine_config.elo.clamp(1320,3190);
            self.new_game_limit_strength=self.engine_config.limit_strength;
        }
    }
    pub(super) fn apply_new_game_strength(&mut self) {
        if self.engine_config.opponent==OpponentEngine::Stockfish {
            self.engine_config.limit_strength=self.new_game_limit_strength;
            self.engine_config.elo=self.new_game_elo.clamp(1320,3190);
        } else if let Some(control)=self.companion_elo_control(&self.engine_config) {
            let key=Self::verification_engine_key(&self.engine_config);
            // UCI options are remembered by engine identity, just as in Engine options.
            for config in [&mut self.engine_config,&mut self.analysis_config,&mut self.strength_draft,&mut self.analysis_draft] {
                if Self::verification_engine_key(config)==key {control.write(&mut config.companion_options,self.new_game_elo,self.new_game_limit_strength);}
            }
            self.engine_uci_preferences.insert(key,self.engine_config.companion_options.clone());
        }
        self.strength_draft=self.engine_config.clone();
    }
    pub(super) fn companion_play_elo_ui(&mut self,ui:&mut egui::Ui) {
        let mut config=self.strength_draft.clone();
        config.companion_options=self.uci_draft_values.clone();
        if let Some(control)=self.companion_elo_control(&config) {
            let (mut elo,mut limited)=(control.value,control.limited);
            if control.ui(ui,&mut elo,&mut limited) {control.write(&mut self.uci_draft_values,elo,limited);}
        } else {
            ui.label(if self.engine_uci_catalog.contains_key(&Self::verification_engine_key(&config)) {"This engine does not advertise an Elo setting."} else {"Connect the engine to load its Elo settings."});
        }
    }
    pub(super) fn prepare_uci_settings(&mut self) {
        self.uci_settings_error.clear();
        let key = Self::verification_engine_key(&self.analysis_config);
        self.uci_draft_values = self
            .engine_uci_preferences
            .get(&key)
            .cloned()
            .unwrap_or_else(|| {
                if self.analysis_config.opponent == OpponentEngine::Companion {
                    self.analysis_config.companion_options.clone()
                } else {
                    self.analysis_config.uci_options.clone()
                }
            });
        #[cfg(target_arch = "wasm32")]
        if !self.engine_uci_catalog.contains_key(&key) && self.engine.is_none() {
            self.engine = EngineBridge::new(self.puzzle_context.clone(), &self.analysis_config);
            self.engine_ready = false;
            self.engine_start_failed = self.engine.is_none();
            self.engine_status = "Connecting to engine…".into();
        }
    }
    pub(super) fn engine_uci_options_ui(&mut self, ui: &mut egui::Ui, width: f32, row_height: f32, table_height: f32) {
        let key = Self::verification_engine_key(&self.analysis_draft);
        let remote = self.analysis_draft.opponent == OpponentEngine::Companion;
        let options = self
            .engine_uci_catalog
            .get(&key)
            .cloned()
            .unwrap_or_default();
        if options.is_empty() {
            ui.label(if self.engine_start_failed { "Could not load the engine's UCI options." } else { "Loading the engine's advertised UCI options…" });
            #[cfg(target_arch = "wasm32")]
            if self.engine_start_failed || (remote && self.engine.is_none()) {
                ui.label(self.engine_status.clone());
                ui.horizontal(|ui| {
                    if remote && ui.add(egui::Button::new("Manage connection").raised_gray()).clicked() {
                        manage_companion_connection();
                    }
                    if ui.add(egui::Button::new("Retry connection").raised_gray()).clicked() {
                        self.engine = None;
                        self.engine_start_failed = false;
                        self.prepare_uci_settings();
                    }
                });
            }
        }
        egui_extras::TableBuilder::new(ui)
            .id_salt("engine_uci_options_table")
            .striped(true).resizable(false)
            .cell_layout(Layout::left_to_right(Align::Center))
            .column(egui_extras::Column::exact((width * 0.34).min(210.0)))
            .column(egui_extras::Column::remainder().clip(true))
            .min_scrolled_height(table_height).max_scroll_height(table_height)
            .auto_shrink([false, false])
            .header(30.0, |mut header| {
                for label in ["Setting", "Value"] {
                    header.col(|ui| { Self::game_moves_glass_label(ui, Vec2::new(ui.available_width(), 28.0), label, true); });
                }
            })
            .body(|body| body.rows(row_height, options.len(), |mut row| {
                let option = &options[row.index()];
                row.col(|ui| { ui.add(egui::Label::new(&option.name).truncate()).on_hover_text(&option.name); });
                row.col(|ui| { ui.push_id(&option.name, |ui| {
                    if !option.editable(remote) {
                        ui.add(egui::Label::new("Managed by Ironwood").truncate())
                            .on_hover_text("Managed by Ironwood's search controls or bundled files.");
                    } else if option.kind == "button" {
                        #[cfg(target_arch = "wasm32")]
                        if ui
                            .add_enabled(
                                self.engine_ready
                                    && !self.engine_searching
                                    && self
                                        .engine
                                        .as_ref()
                                        .is_some_and(|engine| engine.identity == key),
                                egui::Button::new("Run action").raised_gray(),
                            )
                            .on_hover_text(
                                "Runs once on the idle engine; actions are not saved as settings.",
                            )
                            .clicked()
                        {
                            if let Some(engine) = &self.engine {
                                engine.command(&format!("setoption name {}", option.name));
                            }
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        ui.label("Engine action");
                    } else {
                        let mut value = self
                            .uci_draft_values
                            .get(&option.name)
                            .cloned()
                            .unwrap_or_else(|| option.default.clone());
                        let changed = match option.kind.as_str() {
                            "check" => {
                                let mut checked = value == "true";
                                let changed = ui.checkbox(&mut checked, "Enabled").changed();
                                value = checked.to_string();
                                changed
                            }
                            "spin" => {
                                let mut number = value.parse::<i64>().unwrap_or_default();
                                let changed = ui
                                    .add(egui::DragValue::new(&mut number).range(
                                        option.min.unwrap_or(i32::MIN as i64)
                                            ..=option.max.unwrap_or(i32::MAX as i64),
                                    ))
                                    .changed();
                                value = number.to_string();
                                changed
                            }
                            "combo" => {
                                let previous = value.clone();
                                egui::ComboBox::from_id_salt("value")
                                    .selected_text(&value)
                                    .show_ui(ui, |ui| {
                                        for choice in &option.choices {
                                            ui.selectable_value(&mut value, choice.clone(), choice);
                                        }
                                    });
                                previous != value
                            }
                            "string" => ui
                                .add(
                                    egui::TextEdit::singleline(&mut value)
                                        .desired_width(ui.available_width())
                                        .char_limit(4096),
                                )
                                .changed(),
                            _ => false,
                        };
                        if changed {
                            self.uci_draft_values.insert(option.name.clone(), value);
                        }
                    }
                }); });
            }));
        if !self.uci_settings_error.is_empty() {
            ui.colored_label(Color32::from_rgb(224, 139, 76), &self.uci_settings_error);
        }
    }
    pub(super) fn apply_uci_settings(&mut self) -> bool {
        let key = Self::verification_engine_key(&self.analysis_config);
        let remote = self.analysis_config.opponent == OpponentEngine::Companion;
        let Some(options) = self.engine_uci_catalog.get(&key) else {
            return true;
        };
        let editable = |name: &str| {
            options
                .iter()
                .find(|o| o.name == name && o.editable(remote) && o.kind != "button")
        };
        let mut values = self.uci_draft_values.clone();
        values.retain(|name, _| editable(name).is_some());
        if values.len() > 64 {
            self.uci_settings_error = "Change at most 64 UCI options per engine.".into();
            self.engine_settings_tab = EngineSettingsTab::Uci;
            return false;
        }
        for (name, value) in &values {
            if !editable(name).is_some_and(|option| option.valid(value)) {
                self.uci_settings_error =
                    format!("Enter a valid value for {name} within its advertised range.");
                self.engine_settings_tab = EngineSettingsTab::Uci;
                return false;
            }
        }
        let mut previous = if remote {
            self.analysis_config.companion_options.clone()
        } else {
            self.analysis_config.uci_options.clone()
        };
        previous.retain(|name, _| editable(name).is_some());
        let changed = previous != values;
        self.engine_uci_preferences
            .insert(key.clone(), values.clone());
        for config in [
            &mut self.engine_config,
            &mut self.analysis_config,
            &mut self.strength_draft,
            &mut self.analysis_draft,
        ] {
            if Self::verification_engine_key(config) == key {
                if remote {
                    config.companion_options = values.clone();
                } else {
                    config.uci_options = values.clone();
                }
            }
        }
        if changed {
            #[cfg(target_arch = "wasm32")]
            {
                self.engine = None;
                self.engine_ready = false;
                self.engine_start_failed = false;
            }
            self.engine_searching = false;
            self.analysis_running = false;
            self.ignore_next_bestmove = false;
            self.resume_engine_after_ready = false;
            self.clear_analysis_result();
            self.game_analysis = vec![None; self.review_positions.len()];
            self.clear_verification();
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn maia_options()->Vec<UciOption> {
        ["option name Elo type spin default 1500 min 0 max 5000",
         "option name SelfElo type spin default 1500 min 0 max 5000",
         "option name OppoElo type spin default 1500 min 0 max 5000"]
         .into_iter().map(|line|parse_option(line).unwrap()).collect()
    }
    #[test]
    fn companion_elo_uses_advertised_ranges_and_supports_standard_limiters() {
        let options=maia_options();let mut values=std::collections::BTreeMap::new();
        let control=EloControl::discover(&options,&values).unwrap();
        assert_eq!((control.min,control.max,control.value,control.limited),(0,5000,1500,true));
        control.write(&mut values,7000,false);
        for name in ["Elo","SelfElo","OppoElo"] {assert_eq!(values[name],"5000");}
        assert!(!values.contains_key("UCI_LimitStrength"));
        values.insert("SelfElo".into(),"1200".into());
        assert_eq!(EloControl::discover(&options,&values).unwrap().value,1200);
        let standard=[parse_option("option name UCI_Elo type spin default 1800 min 1320 max 3190").unwrap(),parse_option("option name UCI_LimitStrength type check default false").unwrap()];
        let control=EloControl::discover(&standard,&values).unwrap();assert!(!control.limited);
        control.write(&mut values,800,true);assert_eq!(values["UCI_Elo"],"1320");assert_eq!(values["UCI_LimitStrength"],"true");
        assert!(EloControl::discover(&[parse_option("option name Threads type spin default 1 min 1 max 8").unwrap()],&values).is_none());
        assert!(EloControl::discover(&[parse_option("option name Elo type spin default 1 min 100 max 10").unwrap()],&values).is_none());
    }
    #[test]
    fn new_game_companion_rating_is_saved_and_applied_without_stockfish_clamping() {
        let ctx=eframe::CreationContext::_new_kittest(egui::Context::default());let mut app=ChessApp::new(&ctx);
        app.engine_config=EngineConfig{opponent:OpponentEngine::Companion,companion_name:"Custom renamed predictor".into(),companion_identity:"local/maia".into(),..Default::default()};
        app.engine_config.companion_options.insert("Temperature".into(),"0.5".into());
        app.analysis_config=app.engine_config.clone();
        let key=ChessApp::verification_engine_key(&app.engine_config);app.engine_uci_catalog.insert(key.clone(),maia_options());
        app.prepare_new_game_strength();assert_eq!(app.new_game_elo,1500);assert!(app.new_game_limit_strength);
        let stockfish_elo=app.engine_config.elo;
        app.new_game_elo=900;app.apply_new_game_strength();
        assert_eq!(app.engine_config.elo,stockfish_elo);
        for config in [&app.engine_config,&app.analysis_config,&app.strength_draft] {
            for name in ["Elo","SelfElo","OppoElo"] {assert_eq!(config.companion_options[name],"900");}
            assert_eq!(config.companion_options["Temperature"],"0.5");
        }
        app.prepare_new_game_strength();assert_eq!(app.new_game_elo,900);
        let prefs:UserPreferences=serde_json::from_str(&serde_json::to_string(&app.preferences_snapshot()).unwrap()).unwrap();assert_eq!(prefs.engine_uci_preferences[&key]["Elo"],"900");
        app.engine_config.companion_identity="local/lc0".into();app.engine_config.companion_options.clear();
        assert!(app.companion_elo_control(&app.engine_config).is_none());
        app.new_game_elo=1000;app.apply_new_game_strength();assert!(app.engine_config.companion_options.is_empty());
        app.engine_config.opponent=OpponentEngine::Stockfish;app.apply_new_game_strength();assert_eq!(app.engine_config.elo,1320);
    }
    #[test]
    fn play_rating_edits_reach_the_engine_override_map_only_after_apply() {
        let ctx=eframe::CreationContext::_new_kittest(egui::Context::default());let mut app=ChessApp::new(&ctx);
        app.engine_config=EngineConfig{opponent:OpponentEngine::Companion,companion_name:"Maia3".into(),companion_identity:"local/maia".into(),..Default::default()};
        app.analysis_config=app.engine_config.clone();app.strength_draft=app.engine_config.clone();app.analysis_draft=app.engine_config.clone();
        let key=ChessApp::verification_engine_key(&app.engine_config);app.engine_uci_catalog.insert(key,maia_options());
        app.prepare_uci_settings();let control=app.companion_elo_control(&app.strength_draft).unwrap();
        control.write(&mut app.uci_draft_values,2100,true);
        assert!(app.engine_config.companion_options.is_empty());
        assert!(app.apply_uci_settings());assert_eq!(app.engine_config.companion_options["Elo"],"2100");
        assert_eq!(app.strength_draft.companion_options["SelfElo"],"2100");
    }
    #[test]
    fn advertised_uci_types_validate_ranges_choices_and_text() {
        let threads = parse_option("option name Threads type spin default 2 min 1 max 8").unwrap();
        assert!(threads.valid("4"));
        assert!(!threads.valid("9"));
        assert!(!threads.valid("1\nquit"));
        let backend = parse_option(
            "option name Backend type combo default cuda var cuda var cpu accelerated",
        )
        .unwrap();
        assert!(backend.valid("cpu accelerated"));
        assert!(!backend.valid("missing"));
        let string =
            parse_option("option name BackendOptions type string default <empty>").unwrap();
        assert_eq!(string.default, "");
        assert!(string.valid("gpu=0"));
        assert!(!string.valid("gpu=0\nquit"));
        assert!(!string.editable(false));
        assert!(string.editable(true));
        assert!(
            !parse_option("option name UCI_Chess960 type check default false")
                .unwrap()
                .editable(true)
        );
    }
    #[test]
    fn uci_edits_are_validated_and_remembered_by_engine_identity() {
        let context = eframe::CreationContext::_new_kittest(egui::Context::default());
        let mut app = ChessApp::new(&context);
        app.analysis_config.opponent = OpponentEngine::Companion;
        app.analysis_config.companion_identity = "http://localhost:8790/lc0".into();
        app.analysis_draft = app.analysis_config.clone();
        let key = ChessApp::verification_engine_key(&app.analysis_config);
        app.engine_uci_catalog.insert(
            key.clone(),
            vec![
                parse_option("option name Threads type spin default 2 min 1 max 8").unwrap(),
                parse_option("option name Backend type combo default cuda var cuda var cpu")
                    .unwrap(),
                parse_option("option name BackendOptions type string default <empty>").unwrap(),
            ],
        );
        app.prepare_uci_settings();
        app.uci_draft_values.insert("Threads".into(), "99".into());
        assert!(!app.apply_uci_settings());
        assert!(!app.engine_uci_preferences.contains_key(&key));
        app.uci_draft_values.insert("Threads".into(), "4".into());
        app.uci_draft_values.insert("Backend".into(), "cuda".into());
        app.uci_draft_values
            .insert("BackendOptions".into(), "gpu=0".into());
        assert!(app.apply_uci_settings());
        assert_eq!(app.analysis_config.companion_options["Threads"], "4");
        let prefs: UserPreferences =
            serde_json::from_str(&serde_json::to_string(&app.preferences_snapshot()).unwrap())
                .unwrap();
        assert_eq!(
            prefs.engine_uci_preferences[&key]["BackendOptions"],
            "gpu=0"
        );
        assert!(
            !prefs
                .engine_uci_preferences
                .contains_key("builtin:stockfish")
        );
    }
}
