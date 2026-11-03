use super::*;

#[test]
fn batch_export_validates_timeline_and_preserves_notes_and_previous_identity() {
    let json=ChessApp::prepare_archived_pgn("[White \"Batch\"]\n[Black \"Test\"]\n\n1. e4 *").unwrap();
    let mut saved:serde_json::Value=serde_json::from_str(&json).unwrap();
    saved["move_notes"]=serde_json::json!(["Keep this annotation"]);
    saved["game_analysis"]=serde_json::json!([
        {"eval_cp":32,"mate":null,"depth":12,"nodes":1000,"best_move":"e2e4","pv":"e2e4 e7e5 g1f3"},null]);
    let normalized=ChessApp::export_saved_game(&saved.to_string(),"batch-normalize").unwrap();
    let result:serde_json::Value=serde_json::from_str(&normalized).unwrap();
    assert_eq!(result["move_notes"][0],"Keep this annotation");
    assert_eq!(result["game_analysis"][0]["pv"],"e4 e5 Nf3");
    let positions:serde_json::Value=serde_json::from_str(&ChessApp::export_saved_game(&json,"batch-positions").unwrap()).unwrap();
    assert_eq!(positions["positions"].as_array().unwrap().len(),2);
    saved["game_analysis"][0]["best_move"]=serde_json::json!("e2e5");
    assert!(ChessApp::export_saved_game(&saved.to_string(),"batch-normalize").is_err());
    saved["game_analysis"]=serde_json::json!([]);
    assert!(ChessApp::export_saved_game(&saved.to_string(),"batch-normalize").is_err());
}

#[test]
fn batch_second_pass_targets_both_sides_of_critical_moves() {
    let json=ChessApp::prepare_archived_pgn("[Event \"Critical\"]\n\n1. e4 *").unwrap();
    let mut saved:serde_json::Value=serde_json::from_str(&json).unwrap();
    saved["game_analysis"]=serde_json::json!([
        {"eval_cp":150,"mate":null,"depth":12,"nodes":1000,"best_move":"d2d4","pv":"d2d4"},
        {"eval_cp":-150,"mate":null,"depth":12,"nodes":1000,"best_move":"e7e5","pv":"e7e5"}]);
    assert_eq!(ChessApp::export_saved_game(&saved.to_string(),"batch-targets").unwrap(),"[0,1]");
    saved["game_analysis"][0]["best_move"]=serde_json::json!("e2e4");
    assert_eq!(ChessApp::export_saved_game(&saved.to_string(),"batch-targets").unwrap(),"[]");
    saved["game_analysis"][1]["eval_cp"]=serde_json::Value::Null;
    saved["game_analysis"][1]["mate"]=serde_json::json!(-3);
    assert_eq!(ChessApp::export_saved_game(&saved.to_string(),"batch-targets").unwrap(),"[0,1]");
}
