//! A second UI context and native-resolution render output over one ChessApp.
//! No game snapshot, engine, or image stream is created for the detached panel.
use crate::app::ChessApp;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::prelude::*;
use egui::{Context, TextureId};

struct Shared {
    app: Rc<RefCell<ChessApp>>,
    main: Context,
    panel: Option<Context>,
}
thread_local! { static SHARED: RefCell<Option<Shared>> = const { RefCell::new(None) }; }

pub struct SharedApp(Rc<RefCell<ChessApp>>);
impl SharedApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let app=Rc::new(RefCell::new(ChessApp::new(cc)));
        SHARED.with(|slot|*slot.borrow_mut()=Some(Shared{app:app.clone(),main:cc.egui_ctx.clone(),panel:None}));
        Self(app)
    }
}
impl eframe::App for SharedApp {
    fn logic(&mut self,ctx:&Context,frame:&mut eframe::Frame){self.0.borrow_mut().logic(ctx,frame);}
    fn ui(&mut self,ui:&mut egui::Ui,frame:&mut eframe::Frame){
        self.0.borrow_mut().ui(ui,frame);
        SHARED.with(|slot| {
            if let Some(shared)=slot.borrow().as_ref() {
                if let Some(panel)=&shared.panel {sync_tabs(&shared.main,panel);}
            }
        });
    }
}

fn property(object:&js_sys::Object,name:&str,value:impl Into<JsValue>) {
    let _=js_sys::Reflect::set(object,&JsValue::from_str(name),&value.into());
}
fn texture_name(id:TextureId)->String {match id{TextureId::Managed(n)=>format!("m{n}"),TextureId::User(n)=>format!("u{n}")}}

fn sync_tabs(from:&Context,to:&Context) {
    for name in ["game_sidebar_tab","annotations_tab","tactical_map_tab"] {
        if let Some(value)=from.data(|d|d.get_temp::<usize>(egui::Id::new(name))) {
            to.data_mut(|d|d.insert_temp(egui::Id::new(name),value));
        }
    }
    for name in ["position_details_tab","explore_details_tab"] {
        if let Some(value)=from.data_mut(|d|d.get_persisted::<usize>(egui::Id::new(name))) {
            to.data_mut(|d|d.insert_persisted(egui::Id::new(name),value));
        }
    }
    fn copy<T: Clone + Send + Sync + 'static>(from:&Context,to:&Context,name:&str) {
        let id=egui::Id::new(name);
        let value=from.data(|data|data.get_temp::<T>(id));
        to.data_mut(|data|{data.remove::<T>(id);if let Some(value)=value {data.insert_temp(id,value);}});
    }
    for name in ["draw_tool_arrow","tactical_knight_reach","tactical_attack_defense","tactical_show_opponent","tactical_show_changes","tactical_destination_safety"] {copy::<bool>(from,to,name);}
    for name in ["drawing_arrow_style","drawing_shape_style","draw_ghost_piece","tactical_selection"] {copy::<String>(from,to,name);}
    copy::<[bool;3]>(from,to,"tactical_filters");
    copy::<chess::Square>(from,to,"tactical_knight_source");
    copy::<(crate::rules::Board,chess::Square)>(from,to,"tactical_activity_piece");
    copy::<(crate::rules::Board,chess::Square,chess::Square)>(from,to,"tactical_knight_lock");
    copy::<(crate::rules::Board,String)>(from,to,"tactical_opponent_selected");
    if let Some(pick)=from.data_mut(|data|data.remove_temp::<(chess::Square,bool)>(egui::Id::new("mobility_pick"))) {
        to.data_mut(|data|data.insert_temp(egui::Id::new("mobility_pick"),pick));
    }
    crate::opening_explorer::copy_view_state(from,to);
    crate::tablebase::copy_view_state(from,to);
}

#[wasm_bindgen]
pub fn begin_study_panel() {
    SHARED.with(|slot| {
        if let Some(shared)=slot.borrow_mut().as_mut() {
            let panel=Context::default();
            panel.set_theme(shared.main.theme());
            panel.set_style_of(shared.main.theme(),(*shared.main.style_of(shared.main.theme())).clone());
            panel.set_zoom_factor(shared.main.zoom_factor());
            egui_extras::install_image_loaders(&panel);
            sync_tabs(&shared.main,&panel);
            shared.panel=Some(panel);
            shared.main.request_repaint();
        }
    });
}

#[wasm_bindgen]
pub fn dock_study_panel() {
    SHARED.with(|slot| {
        if let Ok(shared)=slot.try_borrow() {
            if let Some(shared)=shared.as_ref() {
                if let Some(panel)=&shared.panel {sync_tabs(panel,&shared.main);}
                shared.main.request_repaint();
            }
        }
    });
}

#[wasm_bindgen]
pub fn render_study_panel(input:&str) -> Result<JsValue,JsValue> {
    let request:serde_json::Value=serde_json::from_str(input).map_err(|e|JsValue::from_str(&e.to_string()))?;
    SHARED.with(|slot| {
        let mut holder=slot.try_borrow_mut().map_err(|_|JsValue::from_str("Study panel is busy"))?;
        let Some(shared)=holder.as_mut() else {return Ok(JsValue::NULL);};
        let Some(panel)=shared.panel.as_ref() else {return Ok(JsValue::NULL);};
        let finite=|name:&str,default:f64,min:f64,max:f64|request[name].as_f64().filter(|v|v.is_finite()).unwrap_or(default).clamp(min,max) as f32;
        let width=finite("width",800.0,100.0,8192.0);
        let height=finite("height",800.0,100.0,8192.0);
        let dpr=finite("dpr",1.0,0.5,4.0);
        let zoom=finite("zoom",1.0,0.75,2.0);
        panel.set_zoom_factor(zoom);
        let mut raw=egui::RawInput{screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(width/zoom,height/zoom))),
            time:request["time"].as_f64(),focused:request["focused"].as_bool().unwrap_or(true),
            max_texture_side:Some(finite("textureLimit",8192.0,2048.0,16384.0) as usize),..Default::default()};
        raw.viewports.entry(egui::ViewportId::ROOT).or_default().native_pixels_per_point=Some(dpr);
        crate::sidebar_popout::append_input(&mut raw,&request["events"].to_string());
        for event in &mut raw.events {
            match event {
                egui::Event::PointerMoved(pos)|egui::Event::PointerButton{pos,..}=>{pos.x/=zoom;pos.y/=zoom;},
                egui::Event::MouseWheel{unit:egui::MouseWheelUnit::Point,delta,..}=>*delta/=zoom,
                _=>{},
            }
        }
        let mut app=shared.app.try_borrow_mut().map_err(|_|JsValue::from_str("Game is updating"))?;
        app.advance_game(&shared.main);
        let mut output=panel.run_ui(raw,|root| {
            egui::CentralPanel::default().frame(egui::Frame::new().fill(egui::Color32::from_rgb(17,20,24)).inner_margin(8))
                .show(root,|ui|app.detached_sidebar_ui(ui));
        });
        sync_tabs(panel,&shared.main);
        shared.main.request_repaint();
        let result=js_sys::Object::new();
        property(&result,"ready",true);
        property(&result,"pixelsPerPoint",output.pixels_per_point);
        property(&result,"width",width/zoom);property(&result,"height",height/zoom);
        property(&result,"cursor",format!("{:?}",output.platform_output.cursor_icon));
        property(&result,"editingText",output.platform_output.ime.is_some());
        let textures=js_sys::Array::new();
        for (id,deltas) in &output.textures_delta.set {
          for delta in deltas {
            let texture=js_sys::Object::new();
            let egui::ImageData::Color(image)=&delta.image;
            let bytes:Vec<u8>=image.pixels.iter().flat_map(|color|color.to_array()).collect();
            property(&texture,"id",texture_name(*id));
            property(&texture,"width",image.size[0] as u32);property(&texture,"height",image.size[1] as u32);
            property(&texture,"pixels",js_sys::Uint8Array::from(bytes.as_slice()));
            if let Some([x,y])=delta.pos {property(&texture,"x",x as u32);property(&texture,"y",y as u32);}
            property(&texture,"nearest",delta.options.magnification==egui::TextureFilter::Nearest);
            property(&texture,"nearestMin",delta.options.minification==egui::TextureFilter::Nearest);
            property(&texture,"wrap",format!("{:?}",delta.options.wrap_mode));
            textures.push(&texture);
          }
        }
        property(&result,"textures",textures);
        let free=js_sys::Array::new();for id in &output.textures_delta.free {free.push(&JsValue::from_str(&texture_name(*id)));}
        property(&result,"free",free);
        let meshes=js_sys::Array::new();
        for primitive in panel.tessellate(std::mem::take(&mut output.shapes),output.pixels_per_point) {
            if let egui::epaint::Primitive::Mesh(mesh)=primitive.primitive {
                let vertices:Vec<f32>=mesh.vertices.iter().flat_map(|v| {
                    let [r,g,b,a]=v.color.to_array();
                    [v.pos.x,v.pos.y,v.uv.x,v.uv.y,r as f32/255.0,g as f32/255.0,b as f32/255.0,a as f32/255.0]
                }).collect();
                let draw=js_sys::Object::new();
                property(&draw,"texture",texture_name(mesh.texture_id));
                property(&draw,"vertices",js_sys::Float32Array::from(vertices.as_slice()));
                property(&draw,"indices",js_sys::Uint32Array::from(mesh.indices.as_slice()));
                property(&draw,"clip",js_sys::Float32Array::from([primitive.clip_rect.left(),primitive.clip_rect.top(),primitive.clip_rect.right(),primitive.clip_rect.bottom()].as_slice()));
                meshes.push(&draw);
            }
        }
        property(&result,"meshes",meshes);
        let commands=js_sys::Array::new();
        for command in &output.platform_output.commands {
            let value=js_sys::Object::new();
            match command {
                egui::OutputCommand::CopyText(text)=>{property(&value,"type","copy");property(&value,"text",text.as_str());},
                egui::OutputCommand::OpenUrl(url)=>{property(&value,"type","url");property(&value,"url",url.url.as_str());property(&value,"newTab",url.new_tab);},
                egui::OutputCommand::CopyImage(image)=>{
                    property(&value,"type","copyImage");property(&value,"width",image.size[0] as u32);property(&value,"height",image.size[1] as u32);
                    let bytes:Vec<u8>=image.pixels.iter().flat_map(|color|color.to_srgba_unmultiplied()).collect();
                    property(&value,"pixels",js_sys::Uint8Array::from(bytes.as_slice()));
                },
            }
            commands.push(&value);
        }
        property(&result,"commands",commands);
        output.textures_delta.clear();
        Ok(result.into())
    })
}
