#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod board3d;
mod fics_chat;
mod rules;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub async fn start() -> Result<(), JsValue> {
    eframe::WebLogger::init(log::LevelFilter::Info).ok();
    let options = eframe::WebOptions::default();
    let canvas = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.get_element_by_id("chess_canvas"))
        .ok_or_else(|| JsValue::from_str("missing #chess_canvas"))?
        .dyn_into::<web_sys::HtmlCanvasElement>()?;
    eframe::WebRunner::new()
        .start(
            canvas,
            options,
            Box::new(|cc| Ok(Box::new(app::ChessApp::new(cc)))),
        )
        .await
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run_native() -> eframe::Result {
    env_logger::init();
    eframe::run_native(
        "Ironwood Chess",
        eframe::NativeOptions::default(),
        Box::new(|cc| Ok(Box::new(app::ChessApp::new(cc)))),
    )
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn export_saved_game(json: &str, format: &str) -> Result<String, JsValue> {
    app::ChessApp::export_saved_game(json, format).map_err(|error| JsValue::from_str(&error))
}
