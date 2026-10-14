//! Browser-only, on-demand artwork for the three large raster piece sets.
//! Load state belongs to the egui context so a recreated app cannot inherit stale readiness.

use super::{Color, Piece, PieceSet};
use eframe::egui;

// This order is shared with window.ironwoodLoadPieceTheme.
const PIECES: [&str; 12] = ["wK", "wQ", "wR", "wB", "wN", "wP", "bK", "bQ", "bR", "bB", "bN", "bP"];

#[derive(Clone, Debug, PartialEq, Eq)]
enum LoadState {
    Loading,
    Ready,
    Failed,
}

pub(super) fn theme_key(set: PieceSet) -> Option<&'static str> {
    match set {
        PieceSet::SculptedMarble => Some("sculpted-marble"),
        PieceSet::HybridWood => Some("hybrid-wood"),
        PieceSet::HybridGlass => Some("hybrid-glass"),
        _ => None,
    }
}

fn state_id(theme: &str) -> egui::Id {
    egui::Id::new(("downloaded_piece_theme", theme))
}

fn state(ctx: &egui::Context, theme: &str) -> Option<LoadState> {
    ctx.data(|data| data.get_temp(state_id(theme)))
}

fn begin_request(ctx: &egui::Context, theme: &str, retry: bool) -> bool {
    ctx.data_mut(|data| {
        let id = state_id(theme);
        match data.get_temp::<LoadState>(id) {
            None => {},
            Some(LoadState::Failed) if retry => {},
            _ => return false,
        }
        data.insert_temp(id, LoadState::Loading);
        true
    })
}

fn uri(theme: &str, piece: &str) -> String {
    format!("bytes://piece-themes/{theme}/{piece}.png")
}

fn piece_name(color: Color, piece: Piece) -> &'static str {
    let side = if color == Color::White { 0 } else { 6 };
    let index = match piece {
        Piece::King => 0, Piece::Queen => 1, Piece::Rook => 2,
        Piece::Bishop => 3, Piece::Knight => 4, Piece::Pawn => 5,
    };
    PIECES[side + index]
}

fn finish_request(ctx: &egui::Context, theme: &str, result: Result<Vec<Vec<u8>>, String>) {
    let result = result.and_then(|images| {
        if images.len() != PIECES.len() || images.iter().any(|bytes| !bytes.starts_with(b"\x89PNG\r\n\x1a\n")) {
            return Err("Piece theme did not contain all twelve PNG images".to_owned());
        }
        // Register the complete set together. A partially fetched theme never replaces the fallback.
        for (piece, bytes) in PIECES.iter().zip(images) {
            ctx.include_bytes(uri(theme, piece), bytes);
        }
        Ok(())
    });
    let next = match result {
        Ok(()) => LoadState::Ready,
        Err(error) => {
            log::warn!("Could not load piece theme {theme}: {error}");
            LoadState::Failed
        }
    };
    ctx.data_mut(|data| data.insert_temp(state_id(theme), next));
    ctx.request_repaint();
}

fn request(ctx: &egui::Context, theme: &'static str, retry: bool) {
    if !begin_request(ctx, theme, retry) { return; }
    if retry {
        for piece in PIECES { ctx.forget_image(&uri(theme, piece)); }
    }
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::{JsCast, prelude::*};
        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(js_namespace = window, js_name = ironwoodLoadPieceTheme, catch)]
            fn load_piece_theme(theme: &str) -> Result<js_sys::Promise, JsValue>;
        }
        let ctx = ctx.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let result = async {
                let promise = load_piece_theme(theme)?;
                let value = wasm_bindgen_futures::JsFuture::from(promise).await?;
                let images = value.dyn_into::<js_sys::Array>()?;
                if images.length() != PIECES.len() as u32 {
                    return Err(JsValue::from_str("Incomplete piece theme"));
                }
                images.iter().map(|value| value.dyn_into::<js_sys::Uint8Array>().map(|bytes| bytes.to_vec())).collect::<Result<Vec<_>, _>>()
            }.await;
            finish_request(&ctx, theme, result.map_err(|error| format!("{error:?}")));
        });
    }
}

pub(super) fn image(ctx: &egui::Context, set: PieceSet, color: Color, piece: Piece) -> Option<egui::ImageSource<'static>> {
    let theme = theme_key(set)?;
    request(ctx, theme, false);
    if state(ctx, theme) != Some(LoadState::Ready) { return None; }
    let uri = uri(theme, piece_name(color, piece));
    match ctx.try_load_image(&uri, egui::load::SizeHint::default()) {
        Ok(egui::load::ImagePoll::Ready { .. }) => Some(egui::ImageSource::Uri(uri.into())),
        Ok(egui::load::ImagePoll::Pending { .. }) => None,
        Err(error) => {
            // A corrupt image must not replace a usable piece with egui's broken-image icon.
            finish_request(ctx, theme, Err(error.to_string()));
            None
        }
    }
}

pub(super) fn retry(ctx: &egui::Context, set: PieceSet) {
    if let Some(theme) = theme_key(set) { request(ctx, theme, true); }
}

#[cfg(target_arch = "wasm32")]
pub(super) fn status_ui(ui: &mut egui::Ui, set: PieceSet) {
    let Some(theme) = theme_key(set) else { return; };
    match state(ui.ctx(), theme) {
        Some(LoadState::Loading) => { ui.label(egui::RichText::new("Loading pieces… Using Merida meanwhile.").small().weak()); },
        Some(LoadState::Failed) => {
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new("Pieces unavailable. Using Merida.").small().weak());
                if ui.small_button("Retry").clicked() { retry(ui.ctx(), set); }
            });
        },
        _ => {},
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decoded_image(ctx: &egui::Context, set: PieceSet, color: Color, piece: Piece) -> Option<egui::ImageSource<'static>> {
        // The native egui decoder uses a background thread; the browser decoder is synchronous.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            let source = image(ctx, set, color, piece);
            if source.is_some() || state(ctx, theme_key(set).unwrap()) == Some(LoadState::Failed) { return source; }
            assert!(std::time::Instant::now() < deadline, "PNG decoder did not finish");
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    #[test]
    fn piece_theme_requests_are_shared_only_within_the_same_context() {
        let ctx = egui::Context::default();
        assert!(begin_request(&ctx, "hybrid-wood", false));
        assert!(!begin_request(&ctx.clone(), "hybrid-wood", false));
        assert!(!begin_request(&ctx, "hybrid-wood", true));
        assert!(begin_request(&egui::Context::default(), "hybrid-wood", false));
        assert!(begin_request(&ctx, "hybrid-glass", false));
    }

    #[test]
    fn failed_piece_theme_keeps_fallback_until_explicit_retry() {
        let ctx = egui::Context::default();
        assert!(image(&ctx, PieceSet::HybridWood, Color::White, Piece::King).is_none());
        finish_request(&ctx, "hybrid-wood", Err("offline".into()));
        for _ in 0..3 {
            assert!(image(&ctx, PieceSet::HybridWood, Color::Black, Piece::Pawn).is_none());
            assert_eq!(state(&ctx, "hybrid-wood"), Some(LoadState::Failed));
        }
        retry(&ctx, PieceSet::HybridWood);
        assert_eq!(state(&ctx, "hybrid-wood"), Some(LoadState::Loading));
    }

    #[test]
    fn piece_theme_rejects_partial_and_corrupt_payloads() {
        let ctx = egui::Context::default();
        egui_extras::install_image_loaders(&ctx);
        finish_request(&ctx, "hybrid-glass", Ok(vec![b"\x89PNG\r\n\x1a\n".to_vec(); 11]));
        assert_eq!(state(&ctx, "hybrid-glass"), Some(LoadState::Failed));
        finish_request(&ctx, "hybrid-glass", Ok(vec![b"\x89PNG\r\n\x1a\n".to_vec(); 12]));
        assert!(decoded_image(&ctx, PieceSet::HybridGlass, Color::White, Piece::King).is_none());
        assert_eq!(state(&ctx, "hybrid-glass"), Some(LoadState::Failed));
    }

    #[test]
    fn piece_theme_registers_all_images_with_correct_side_and_piece_names() {
        let ctx = egui::Context::default();
        egui_extras::install_image_loaders(&ctx);
        // A tiny valid PNG exercises the same egui decoder used by the board.
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(1, 1).write_to(&mut png, image::ImageFormat::Png).unwrap();
        finish_request(&ctx, "sculpted-marble", Ok(vec![png.into_inner(); 12]));
        for color in [Color::White, Color::Black] {
            for (piece, name) in [(Piece::King, "K"), (Piece::Queen, "Q"), (Piece::Rook, "R"), (Piece::Bishop, "B"), (Piece::Knight, "N"), (Piece::Pawn, "P")] {
                let source = decoded_image(&ctx, PieceSet::SculptedMarble, color, piece).expect("loaded piece");
                let side = if color == Color::White { "w" } else { "b" };
                let expected = format!("bytes://piece-themes/sculpted-marble/{side}{name}.png");
                assert_eq!(source.uri(), Some(expected.as_str()));
            }
        }
        assert!(!begin_request(&ctx, "sculpted-marble", true));
    }

    #[test]
    fn offscreen_piece_theme_preview_does_not_load_its_artwork() {
        let ctx = egui::Context::default();
        egui_extras::install_image_loaders(&ctx);
        let (source, _) = super::super::ChessApp::piece_image_for(&ctx, PieceSet::SculptedMarble, Color::White, Piece::King).unwrap();
        let image_uri = source.uri().unwrap().to_owned();
        let _ = crate::app::run_test_ui(&ctx, egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400.0, 400.0))), ..Default::default() }, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.add_space(1_000.0);
                let rect = super::super::ChessApp::theme_2d_preview(ui, PieceSet::SculptedMarble, 200.0);
                assert!(!ui.is_rect_visible(rect));
            });
        });
        assert!(ctx.try_load_bytes(&image_uri).is_err(), "offscreen preview must not register or decode its artwork");
        assert_eq!(state(&ctx, "sculpted-marble"), None);
    }
}
