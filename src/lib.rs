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

/// Checks scoresheet moves without changing the current game or saving anything.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn validate_scoresheet_pgn(pgn: &str) -> Result<usize, JsValue> {
    app::ChessApp::parse_pgn_mainline(pgn)
        .map(|(_, moves)| moves.len())
        .map_err(|error| JsValue::from_str(&error))
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn preview_scoresheet(entries: &str) -> Result<String, JsValue> {
    let entries: Vec<String> = serde_json::from_str(entries).map_err(|error| JsValue::from_str(&error.to_string()))?;
    if entries.len() > 1000 { return Err(JsValue::from_str("A scoresheet supports up to 500 move pairs.")); }
    Ok(app::ChessApp::scoresheet_preview(&entries).to_string())
}

#[cfg(test)]
mod scoresheet_tests {
    use super::app::ChessApp;
    fn replay(values: &[&str]) -> serde_json::Value {
        ChessApp::scoresheet_preview(&values.iter().map(|value| value.to_string()).collect::<Vec<_>>())
    }
    #[test]
    fn scoresheet_distinguishes_partial_illegal_and_missing_moves() {
        assert_eq!(replay(&["e4", "e5", "Nf"])["statuses"], serde_json::json!(["valid", "valid", "incomplete"]));
        assert_eq!(replay(&["e4", "e4", "Nf3"])["statuses"], serde_json::json!(["valid", "invalid", "blocked"]));
        assert_eq!(replay(&["e4", "", "Nf3"])["statuses"], serde_json::json!(["valid", "missing", "blocked"]));
    }
    #[test]
    fn scoresheet_sample_accepts_lowercase_piece_notation() {
        let notation = "d4 d5 c4 e6 nc3 nf6 bg5 be7 e3 h6 bh4 o-o nf3 b6 cxd5 exd5 bd3 be6 o-o nbd7 rc1 c5 dxc5 bxc5 b3 nb6 ne5 a6 nc6 qc7 nxe7+ qxe7 qf3 g5 qg3 ng4 bxg5 qxg5 f4 qg7 f5 nxe3 qxe3 d4 qe2 dxc3 fxe6 kh8 rxf7 rxf7 exf7 qxf7 qe5+ qg7 qxc5 nd7 qxc3 ne5 bxa6 qa7+ kh1 re8 bb5 re7 a4 kh7 re1 qc7 qxc7 rxc7 h3 nf7 rd1";
        let values: Vec<_> = notation.split_whitespace().collect();
        let result = replay(&values);
        assert_eq!(result["moves"].as_array().unwrap().len(), 73);
        assert!(result["statuses"].as_array().unwrap().iter().all(|status| status == "valid"));
        assert_eq!(result["moves"][4], "Nc3");
        assert_eq!(result["highlights"][4], serde_json::json!({"from":"b1","to":"c3"}));
        assert_eq!(replay(&["d4", "d5", "c4", "e6", "nc"])["statuses"][4], "incomplete");
    }
    #[test]
    fn scoresheet_replays_corrections_and_canonical_castling() {
        let good = replay(&["e4", "e5", "Nf3", "Nc6", "Bc4", "Bc5", "0-0", ""]);
        assert_eq!(good["moves"][6], "O-O");
        assert_eq!(good["highlights"][6], serde_json::json!({"from":"e1","to":"g1"}));
        assert_eq!(good["positions"].as_array().unwrap().len(), 8);
        let changed = replay(&["d4", "e5", "Nf3", "Nc6"]);
        assert_ne!(changed["positions"][1], good["positions"][1]);
        assert_eq!(changed["moves"].as_array().unwrap().len(), 4);
    }
}
