use crate::rules::{Board, MoveGen};
use eframe::egui::{self, Color32, RichText, Sense, Vec2};
use serde::Deserialize;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodExplorer)]
    fn lookup(fen: &str, source: &str, speed: &str, ratings: &str) -> String;
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodExplorerRetry)]
    fn retry();
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = ironwoodExplorerToken)]
    fn set_token(token: &str);
}
#[derive(Clone, Default)]
struct State { root: String, path: Vec<Board>, labels: Vec<String>, source: usize, speed: String, rating: usize, token: String, connected: bool }
#[derive(Deserialize)]
struct Data { #[serde(default)] moves: Vec<Entry>, error: Option<String>, opening: Option<Opening> }
#[derive(Deserialize)]
struct Opening { name: String, #[serde(default)] eco: String }
#[derive(Deserialize)]
struct Entry { uci: String, san: String, white: u64, draws: u64, black: u64 }
fn id() -> egui::Id { egui::Id::new("opening_explorer_state") }
fn chesscodex_url(eco: &str) -> Option<String> {
    let bytes = eco.as_bytes();
    (bytes.len() == 3 && (b'A'..=b'E').contains(&bytes[0])
        && bytes[1].is_ascii_digit() && bytes[2].is_ascii_digit())
        .then(|| format!("https://chesscodex.org/eco/{eco}"))
}
pub fn preview(ctx: &egui::Context, root: Board) -> Option<Board> {
    if ctx.data_mut(|d| d.get_persisted::<usize>(egui::Id::new("analysis_details_tab"))) != Some(4) { return None; }
    ctx.data_mut(|d| d.get_temp::<State>(id())).filter(|s| s.root == root.to_string()).and_then(|s| s.path.last().copied())
}
pub fn panel(ui: &mut egui::Ui, root: Board) {
    let mut state = ui.ctx().data_mut(|d| d.get_temp::<State>(id())).unwrap_or_default();
    if state.root != root.to_string() { state.root = root.to_string(); state.path.clear(); state.labels.clear(); }
    let gold = Color32::from_rgb(211,173,98);
    ui.spacing_mut().item_spacing = Vec2::new(8.0,6.0);
    ui.label(RichText::new("Opening Explorer").size(22.0).strong());
    ui.label(RichText::new("Explore the moves played from this position.").size(13.0).weak());
    ui.add_space(4.0);
    #[cfg(target_arch = "wasm32")]
    egui::Frame::new().fill(ui.visuals().faint_bg_color).corner_radius(6.0)
        .inner_margin(10.0).show(ui, |ui| {
    ui.set_width(ui.available_width());
    egui::CollapsingHeader::new(if state.connected { "Lichess · token set" } else { "Connect to Lichess" })
        .id_salt("explorer_connection").default_open(!state.connected).show(ui, |ui| {
        ui.label("Opening Explorer requires a Lichess API token. Create a token with no permissions selected.");
        ui.hyperlink_to("Create a Lichess API token", "https://lichess.org/account/oauth/token/create");
        ui.add(egui::TextEdit::singleline(&mut state.token).password(true).hint_text("Paste token here").desired_width(ui.available_width()));
        ui.horizontal(|ui| {
            if ui.add_enabled(!state.token.trim().is_empty(),egui::Button::new("Connect")).clicked() {
                set_token(&state.token); state.token.clear(); state.connected = true;
            }
            if state.connected && ui.button("Disconnect").clicked() { set_token(""); state.connected = false; state.token.clear(); }
        });
        ui.label(RichText::new("Kept only for this app session and sent directly to Lichess. Reloading clears it.").small().weak());
    });
    });
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        for (index,label) in ["Masters","Lichess games"].iter().enumerate() {
            let selected = state.source == index;
            if ui.add(egui::Button::new(RichText::new(*label).size(15.0).color(if selected { gold } else { ui.visuals().text_color() }))
                .fill(if selected { Color32::from_rgb(65,54,33) } else { ui.visuals().faint_bg_color })
                .min_size(Vec2::new(105.0,30.0))).clicked() { state.source = index; }
        }
    });
    ui.horizontal_wrapped(|ui| {
        if !state.path.is_empty() {
            if ui.button("Back").clicked() { state.path.pop(); state.labels.pop(); }
            if ui.button("Back to game").clicked() { state.path.clear(); state.labels.clear(); }
        }
    });
    if state.source == 1 {
        ui.horizontal_wrapped(|ui| {
            egui::ComboBox::from_id_salt("explorer_speed").selected_text(if state.speed.is_empty() { "All speeds" } else { &state.speed }).show_ui(ui, |ui| {
                for speed in ["","bullet","blitz","rapid","classical","correspondence"] { ui.selectable_value(&mut state.speed,speed.into(),if speed.is_empty() { "All speeds" } else { speed }); }
            });
            let labels = ["All ratings","Below 1600","1600–2199","2200+"];
            egui::ComboBox::from_id_salt("explorer_rating").selected_text(labels[state.rating]).show_ui(ui, |ui| {
                for (i,label) in labels.iter().enumerate() { ui.selectable_value(&mut state.rating,i,*label); }
            });
        });
    }
    let board = state.path.last().copied().unwrap_or(root);
    if !state.labels.is_empty() { ui.label(RichText::new(format!("Preview: {}",state.labels.join(" "))).color(Color32::from_rgb(211,173,98))); }
    if board.is_chess960() { ui.label("Opening Explorer currently supports standard chess positions."); }
    else {
        #[cfg(target_arch = "wasm32")]
        let response = lookup(&board.to_string(),if state.source == 0 { "masters" } else { "lichess" },&state.speed,
            ["","0,1000,1200,1400","1600,1800,2000","2200,2500"][state.rating]);
        #[cfg(not(target_arch = "wasm32"))]
        let response = "{\"error\":\"Opening Explorer is available in the browser app.\"}".to_string();
        if response.is_empty() { ui.label("Loading opening statistics…"); }
        else if let Ok(data) = serde_json::from_str::<Data>(&response) {
            if let Some(error) = data.error {
                ui.colored_label(Color32::LIGHT_RED,error);
                #[cfg(target_arch = "wasm32")]
                if ui.button("Retry").clicked() { retry(); }
            } else {
                if let Some(opening) = data.opening {
                    ui.add_space(4.0);
                    ui.label(RichText::new(&opening.name).size(18.0).strong().color(gold));
                    if let Some(url) = chesscodex_url(&opening.eco) {
                        if ui.link(RichText::new(format!("{} · Opening guide on ChessCodex", opening.eco)).size(13.0))
                            .on_hover_text("Open information about this opening’s ECO category in a new tab.").clicked() {
                            ui.ctx().open_url(egui::OpenUrl::new_tab(url));
                        }
                    }
                }
                ui.add_space(4.0);
                if data.moves.is_empty() { ui.label("No games found for this position and these filters."); }
                let width = ui.available_width();
                let move_width = 56.0;
                let count_width = 70.0;
                let bar_width = (width-move_width-count_width-16.0).max(30.0);
                ui.horizontal(|ui| {
                    for (text,w) in [("Move",move_width),("Games",count_width),("Results",bar_width)] {
                        let (rect,_) = ui.allocate_exact_size(Vec2::new(w,22.0),Sense::hover());
                        ui.painter().text(rect.left_center(),egui::Align2::LEFT_CENTER,text,egui::FontId::proportional(13.0),ui.visuals().weak_text_color());
                    }
                });
                ui.spacing_mut().item_spacing.y = 3.0;
                for (row,entry) in data.moves.into_iter().enumerate() {
                    let total = entry.white.saturating_add(entry.draws).saturating_add(entry.black);
                    let legal = MoveGen::new_legal(&board).find(|mv| mv.to_string() == entry.uci);
                    ui.horizontal(|ui| {
                        let row_rect = egui::Rect::from_min_size(ui.cursor().min,Vec2::new(width,30.0));
                        if row%2 == 0 { ui.painter().rect_filled(row_rect,4.0,ui.visuals().faint_bg_color); }
                        if ui.add_enabled(legal.is_some(),egui::Button::new(RichText::new(&entry.san).size(14.0).color(gold)).min_size(Vec2::new(move_width,30.0))).clicked() {
                            if let Some(mv) = legal { state.path.push(board.make_move_new(mv)); state.labels.push(entry.san.clone()); }
                        }
                        let (count_rect,_) = ui.allocate_exact_size(Vec2::new(count_width,30.0),Sense::hover());
                        ui.painter().text(count_rect.left_center(),egui::Align2::LEFT_CENTER,total.to_string(),egui::FontId::monospace(13.0),ui.visuals().text_color());
                        let (slot,response) = ui.allocate_exact_size(Vec2::new(bar_width,30.0),Sense::hover());
                        let rect = slot.shrink2(Vec2::new(0.0,5.0));
                        let mut x = rect.left();
                        for (count,color) in [(entry.white,Color32::from_gray(225)),(entry.draws,Color32::from_gray(120)),(entry.black,Color32::from_gray(40))] {
                            let w = rect.width()*count as f32/total.max(1) as f32;
                            ui.painter().rect_filled(egui::Rect::from_min_size(egui::pos2(x,rect.top()),Vec2::new(w,rect.height())),0.0,color); x += w;
                        }
                        response.on_hover_text(format!("White: {:.1}% ({}) · Draw: {:.1}% ({}) · Black: {:.1}% ({})",entry.white as f64*100.0/total.max(1) as f64,entry.white,entry.draws as f64*100.0/total.max(1) as f64,entry.draws,entry.black as f64*100.0/total.max(1) as f64,entry.black));
                    });
                }
            }
        }
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
    }
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        for (label,color) in [("White",Color32::from_gray(225)),("Draw",Color32::from_gray(120)),("Black",Color32::from_gray(40))] {
            let (rect,_) = ui.allocate_exact_size(Vec2::splat(10.0),Sense::hover());
            ui.painter().rect_filled(rect,2.0,color);
            ui.label(RichText::new(label).size(12.0));
        }
    });
    ui.label(RichText::new("Hover for percentages. Click a move to preview.").size(12.0).weak());
    ui.label(RichText::new("Game results, not an engine evaluation.").size(12.0).weak());
    if ui.link(RichText::new("Opening data from Lichess").size(12.0)).clicked() {
        ui.ctx().open_url(egui::OpenUrl::new_tab("https://lichess.org"));
    }
    ui.ctx().data_mut(|d| d.insert_temp(id(),state));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preview_is_scoped_to_selected_tab_and_original_position() {
        let ctx = egui::Context::default();
        let root = Board::default();
        let mv = MoveGen::new_legal(&root).next().unwrap();
        let child = root.make_move_new(mv);
        ctx.data_mut(|d| {
            d.insert_persisted(egui::Id::new("analysis_details_tab"),4usize);
            d.insert_temp(id(),State { root: root.to_string(), path: vec![child], ..Default::default() });
        });
        assert_eq!(preview(&ctx,root),Some(child));
        assert_eq!(preview(&ctx,child),None);
        ctx.data_mut(|d| d.insert_persisted(egui::Id::new("analysis_details_tab"),0usize));
        assert_eq!(preview(&ctx,root),None);
        assert_eq!(root,Board::default());
    }
}
