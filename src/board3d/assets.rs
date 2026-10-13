//! Complete, validated 3D packs. Browser packs are fetched only when requested;
//! native builds retain the same embedded artwork.

use super::{Model, Models, TextureSet, Theme, V3};
use egui::{Context, Ui};
use image::{ImageFormat, RgbImage};
use serde_json::Value;
use std::{io::Cursor, sync::OnceLock};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LoadState {
    #[default]
    Idle,
    Loading,
    Ready,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PackKey { Marble, Wood }

impl PackKey {
    fn for_theme(theme: Theme) -> Self {
        // Glass shares Marble geometry; Art Deco/Egyptian share its board and maps.
        if theme == Theme::Wood { Self::Wood } else { Self::Marble }
    }

    fn name(self) -> &'static str {
        match self { Self::Marble => "marble", Self::Wood => "wood" }
    }

    fn slot(self) -> &'static OnceLock<Pack> {
        match self { Self::Marble => &MARBLE, Self::Wood => &WOOD }
    }
}

pub(super) struct Pack {
    pub(super) models: Models,
    pub(super) textures: TextureSet,
}

static MARBLE: OnceLock<Pack> = OnceLock::new();
static WOOD: OnceLock<Pack> = OnceLock::new();

pub fn status(theme: Theme) -> LoadState {
    #[cfg(test)]
    if let Some((key, state)) = TEST_STATUS.with(|value| value.get()) {
        if key == PackKey::for_theme(theme) { return state; }
    }
    #[cfg(target_arch = "wasm32")]
    {
        let key = PackKey::for_theme(theme);
        if key.slot().get().is_some() { return LoadState::Ready; }
        REQUESTS.with(|requests| requests.borrow()[key as usize].state)
    }
    #[cfg(not(target_arch = "wasm32"))]
    { let _ = theme; LoadState::Ready }
}

#[cfg(test)]
thread_local! {
    static TEST_STATUS: std::cell::Cell<Option<(PackKey, LoadState)>> = const { std::cell::Cell::new(None) };
}

/// Exercise browser loading/fallback UI in native egui tests without downloading assets.
#[cfg(test)]
pub(crate) fn with_test_status<R>(theme: Theme, state: LoadState, run: impl FnOnce() -> R) -> R {
    struct Restore(Option<(PackKey, LoadState)>);
    impl Drop for Restore {
        fn drop(&mut self) { TEST_STATUS.with(|value| value.set(self.0)); }
    }
    let _restore = Restore(TEST_STATUS.with(|value| value.replace(Some((PackKey::for_theme(theme), state)))));
    run()
}

/// Call before entering any 3D renderer, export, or geometry consumer.
pub fn request(ctx: &Context, theme: Theme) { start_request(ctx, theme, false); }

/// Explicit selection/retry may retry a failed pack; ready/loading packs are reused.
pub fn retry(ctx: &Context, theme: Theme) { start_request(ctx, theme, true); }

pub fn status_ui(ui: &mut Ui, theme: Theme) {
    match status(theme) {
        LoadState::Idle | LoadState::Loading => {
            ui.label(egui::RichText::new("Loading 3D artwork… Using 2D meanwhile.").small().weak());
        },
        LoadState::Failed => {
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new("3D artwork unavailable. Using 2D.").small().weak());
                if ui.small_button("Retry").clicked() { retry(ui.ctx(), theme); }
            });
        },
        LoadState::Ready => {},
    }
}

#[cfg(any(target_arch = "wasm32", test))]
#[derive(Default)]
struct RequestState {
    state: LoadState,
    listeners: Vec<Context>,
}

#[cfg(any(target_arch = "wasm32", test))]
impl RequestState {
    fn begin(&mut self, ctx: &Context, retry: bool) -> bool {
        let start = self.state == LoadState::Idle || (retry && self.state == LoadState::Failed);
        if start { self.state = LoadState::Loading; }
        if self.state == LoadState::Loading && !self.listeners.contains(ctx) {
            self.listeners.push(ctx.clone());
        }
        start
    }

    fn finish(&mut self, ready: bool) {
        self.state = if ready { LoadState::Ready } else { LoadState::Failed };
        for ctx in self.listeners.drain(..) { ctx.request_repaint(); }
    }
}

#[cfg(target_arch = "wasm32")]
thread_local! {
    static REQUESTS: std::cell::RefCell<[RequestState; 2]> = Default::default();
}

fn start_request(ctx: &Context, theme: Theme, retry: bool) {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::{prelude::*, JsCast};
        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(js_namespace = window, js_name = ironwoodLoadBoardTheme, catch)]
            fn load_board_theme(theme: &str) -> Result<js_sys::Promise, JsValue>;
        }
        let key = PackKey::for_theme(theme);
        if key.slot().get().is_some() { return; }
        if !REQUESTS.with(|requests| requests.borrow_mut()[key as usize].begin(ctx, retry)) { return; }
        wasm_bindgen_futures::spawn_local(async move {
            let result = async {
                let promise = load_board_theme(key.name())?;
                let value = wasm_bindgen_futures::JsFuture::from(promise).await?;
                let files = value.dyn_into::<js_sys::Array>()?;
                if files.length() != 14 { return Err(JsValue::from_str("Incomplete 3D theme pack")); }
                files.iter().map(|value| value.dyn_into::<js_sys::Uint8Array>().map(|bytes| bytes.to_vec())).collect::<Result<Vec<_>, _>>()
            }.await;
            let decoded = result.map_err(|error| format!("{error:?}"))
                .and_then(|files| decode_pack(key, &files.iter().map(Vec::as_slice).collect::<Vec<_>>()));
            let ready = match decoded {
                Ok(pack) => {
                    // Models and every texture become visible together, once. Never publish
                    // a fallback under the requested theme's GPU/CPU cache key.
                    let _ = key.slot().set(pack);
                    true
                },
                Err(error) => { log::warn!("Could not load {} 3D artwork: {error}", key.name()); false },
            };
            REQUESTS.with(|requests| requests.borrow_mut()[key as usize].finish(ready));
        });
    }
    #[cfg(not(target_arch = "wasm32"))]
    { let _ = (ctx, theme, retry); }
}

pub(super) fn pack(theme: Theme) -> &'static Pack {
    let key = PackKey::for_theme(theme);
    #[cfg(target_arch = "wasm32")]
    { key.slot().get().expect("3D artwork must be ready before rendering") }
    #[cfg(not(target_arch = "wasm32"))]
    { key.slot().get_or_init(|| decode_pack(key, &embedded(key)).expect("bundled 3D artwork")) }
}

// Exactly the order returned by window.ironwoodLoadBoardTheme: seven GLBs,
// then white/black diffuse, board diffuse/normal/roughness, white/black normal.
#[cfg(not(target_arch = "wasm32"))]
fn embedded(key: PackKey) -> [&'static [u8]; 14] {
    match key {
        PackKey::Wood => [
            include_bytes!("../../assets/3d/omies-chess-set/board.glb"),
            include_bytes!("../../assets/3d/omies-chess-set/pawn.glb"),
            include_bytes!("../../assets/3d/omies-chess-set/rook.glb"),
            include_bytes!("../../assets/3d/omies-chess-set/knight.glb"),
            include_bytes!("../../assets/3d/omies-chess-set/bishop.glb"),
            include_bytes!("../../assets/3d/omies-chess-set/queen.glb"),
            include_bytes!("../../assets/3d/omies-chess-set/king.glb"),
            include_bytes!("../../assets/3d/omies-chess-set/pieces_white_diff.jpg"),
            include_bytes!("../../assets/3d/omies-chess-set/pieces_black_diff.jpg"),
            include_bytes!("../../assets/3d/omies-chess-set/board_diff.jpg"),
            include_bytes!("../../assets/3d/omies-chess-set/board_nor.jpg"),
            include_bytes!("../../assets/3d/omies-chess-set/board_rough.jpg"),
            include_bytes!("../../assets/3d/omies-chess-set/pieces_white_nor.jpg"),
            include_bytes!("../../assets/3d/omies-chess-set/pieces_black_nor.jpg"),
        ],
        PackKey::Marble => [
            include_bytes!("../../assets/3d/polyhaven-chess-set/board.glb"),
            include_bytes!("../../assets/3d/polyhaven-chess-set/pawn.glb"),
            include_bytes!("../../assets/3d/polyhaven-chess-set/rook.glb"),
            include_bytes!("../../assets/3d/polyhaven-chess-set/knight.glb"),
            include_bytes!("../../assets/3d/polyhaven-chess-set/bishop.glb"),
            include_bytes!("../../assets/3d/polyhaven-chess-set/queen.glb"),
            include_bytes!("../../assets/3d/polyhaven-chess-set/king.glb"),
            include_bytes!("../../assets/3d/polyhaven-chess-set/chess_set_pieces_white_diff_1k.jpg"),
            include_bytes!("../../assets/3d/polyhaven-chess-set/chess_set_pieces_black_diff_1k.jpg"),
            include_bytes!("../../assets/3d/polyhaven-chess-set/chess_set_board_diff_1k.jpg"),
            include_bytes!("../../assets/3d/polyhaven-chess-set/chess_set_board_nor_gl_1k.jpg"),
            include_bytes!("../../assets/3d/polyhaven-chess-set/chess_set_board_arm_1k.jpg"),
            include_bytes!("../../assets/3d/polyhaven-chess-set/chess_set_pieces_white_nor_gl_1k.jpg"),
            include_bytes!("../../assets/3d/polyhaven-chess-set/chess_set_pieces_black_nor_gl_1k.jpg"),
        ],
    }
}

fn decode_pack(key: PackKey, files: &[&[u8]]) -> Result<Pack, String> {
    if files.len() != 14 { return Err("Expected all fourteen 3D files".into()); }
    let mut parsed = Vec::with_capacity(7);
    for (index, bytes) in files[..7].iter().enumerate() {
        parsed.push(parse_glb_checked(bytes, index != 0).map_err(|error| format!("Model {index}: {error}"))?);
    }
    let [mut board, pawn, rook, knight, bishop, queen, king]: [Model; 7] = parsed.try_into().map_err(|_| "Incomplete models")?;
    if key == PackKey::Wood {
        if board.indices.len() > u16::MAX as usize - 600 { return Err("Wood board mesh is too large".into()); }
        let min = board.positions.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
        let max = board.positions.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);
        // Omie's 414-pixel checker grid occupies part of the 512-pixel top face.
        let scale = 8.0 / ((max - min) * (414.0 / 512.0));
        let top = board.positions.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max) * scale;
        for point in &mut board.positions { *point = point.mul(scale); point.y += 0.055 - top; }
        super::align_wood_board_grid(&mut board);
    } else {
        let scale = 1.0 / 0.057_888;
        let top = board.positions.iter().map(|p| p.y).fold(0.0, f32::max) * scale;
        for point in &mut board.positions { *point = point.mul(scale); point.y += 0.055 - top; }
    }
    let models = Models {
        board, pawn: pawn.with_dimensions(0.51, 0.90), rook: rook.with_dimensions(0.58, 0.98),
        knight: knight.with_dimensions(0.65, 1.15), bishop: bishop.with_dimensions(0.63, 1.30),
        queen: queen.with_dimensions(0.63, 1.43), king: king.with_dimensions(0.66, 1.52),
    };
    let mut textures = Vec::with_capacity(7);
    for (index, bytes) in files[7..].iter().enumerate() {
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(2048);
        limits.max_image_height = Some(2048);
        limits.max_alloc = Some(32 * 1024 * 1024);
        let mut reader = image::ImageReader::with_format(Cursor::new(bytes), ImageFormat::Jpeg);
        reader.limits(limits);
        textures.push(reader.decode().map_err(|error| format!("Texture {index}: {error}"))?.to_rgb8());
    }
    let [white, black, board_diff, board_normal, board_rough, white_normal, black_normal]: [RgbImage; 7] = textures.try_into().map_err(|_| "Incomplete textures")?;
    Ok(Pack { models, textures: (white, black, board_diff, board_normal, board_rough, white_normal, black_normal) })
}

fn number(value: &Value) -> Result<usize, String> {
    value.as_u64().and_then(|v| usize::try_from(v).ok()).ok_or_else(|| "Missing or invalid GLB integer".into())
}

fn optional_number(value: &Value, fallback: usize) -> Result<usize, String> {
    if value.is_null() { Ok(fallback) } else { number(value) }
}

fn u32_at(bytes: &[u8], offset: usize) -> Result<u32, String> {
    let slice = offset.checked_add(4).and_then(|end| bytes.get(offset..end)).ok_or("Truncated GLB")?;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

struct Accessor<'a> { bytes: &'a [u8], count: usize, stride: usize }

fn accessor<'a>(document: &Value, binary: &'a [u8], index: &Value, components: usize, kind: &str, component_type: usize) -> Result<Accessor<'a>, String> {
    let value = &document["accessors"][number(index)?];
    if value["type"].as_str() != Some(kind) || number(&value["componentType"])? != component_type ||
        !value["sparse"].is_null() || value["normalized"].as_bool().unwrap_or(false) {
        return Err("Unsupported GLB accessor".into());
    }
    let count = number(&value["count"])?;
    if count == 0 || count > 1_000_000 { return Err("Invalid GLB element count".into()); }
    let view = &document["bufferViews"][number(&value["bufferView"])?];
    if optional_number(&view["buffer"], 0)? != 0 { return Err("External GLB buffer is unsupported".into()); }
    let view_at = optional_number(&view["byteOffset"], 0)?;
    let view_len = number(&view["byteLength"])?;
    let view_bytes = view_at.checked_add(view_len).and_then(|end| binary.get(view_at..end)).ok_or("Invalid GLB buffer range")?;
    let width = components * if component_type == 5123 { 2 } else { 4 };
    let stride = optional_number(&view["byteStride"], width)?;
    if stride < width || stride > 252 { return Err("Invalid GLB stride".into()); }
    let offset = optional_number(&value["byteOffset"], 0)?;
    let length = (count - 1).checked_mul(stride).and_then(|n| n.checked_add(width)).ok_or("GLB accessor overflow")?;
    let bytes = offset.checked_add(length).and_then(|end| view_bytes.get(offset..end)).ok_or("Truncated GLB accessor")?;
    Ok(Accessor { bytes, count, stride })
}

fn float(bytes: &[u8], offset: usize) -> Result<f32, String> {
    let value = f32::from_bits(u32_at(bytes, offset)?);
    if !value.is_finite() || value.abs() > 1_000_000.0 { return Err("Invalid GLB coordinate".into()); }
    Ok(value)
}

/// The app uses the single indexed primitive emitted by its existing asset converters.
/// Every downloaded length/index/float is checked before allocation or slicing.
pub(super) fn parse_glb_checked(bytes: &[u8], normalize: bool) -> Result<Model, String> {
    if bytes.get(..4) != Some(b"glTF") || u32_at(bytes, 4)? != 2 || u32_at(bytes, 8)? as usize != bytes.len() || bytes.len() > 16 * 1024 * 1024 {
        return Err("Invalid GLB header".into());
    }
    if bytes.get(16..20) != Some(b"JSON") { return Err("Missing GLB JSON chunk".into()); }
    let json_len = u32_at(bytes, 12)? as usize;
    let json_end = 20usize.checked_add(json_len).ok_or("GLB JSON length overflow")?;
    let json = bytes.get(20..json_end).ok_or("Truncated GLB JSON")?;
    let document: Value = serde_json::from_slice(json).map_err(|error| error.to_string())?;
    let binary_len = u32_at(bytes, json_end)? as usize;
    let binary_at = json_end.checked_add(8).ok_or("GLB binary length overflow")?;
    if bytes.get(json_end + 4..binary_at) != Some(b"BIN\0") { return Err("Missing GLB binary chunk".into()); }
    let binary = binary_at.checked_add(binary_len).and_then(|end| bytes.get(binary_at..end)).ok_or("Truncated GLB binary")?;
    if number(&document["buffers"][0]["byteLength"])? > binary.len() { return Err("Truncated GLB buffer".into()); }
    let primitive = &document["meshes"][0]["primitives"][0];
    if optional_number(&primitive["mode"], 4)? != 4 { return Err("GLB mesh must contain triangles".into()); }
    let positions = accessor(&document, binary, &primitive["attributes"]["POSITION"], 3, "VEC3", 5126)?;
    let normals = accessor(&document, binary, &primitive["attributes"]["NORMAL"], 3, "VEC3", 5126)?;
    let uvs = accessor(&document, binary, &primitive["attributes"]["TEXCOORD_0"], 2, "VEC2", 5126)?;
    let indices = accessor(&document, binary, &primitive["indices"], 1, "SCALAR", 5123)?;
    if positions.count > u16::MAX as usize || positions.count != normals.count || positions.count != uvs.count || indices.count % 3 != 0 {
        return Err("Inconsistent GLB vertex/index counts".into());
    }
    let mut model = Model { positions: Vec::with_capacity(positions.count), normals: Vec::with_capacity(positions.count), texcoords: Vec::with_capacity(positions.count), indices: Vec::with_capacity(indices.count) };
    for n in 0..positions.count {
        let at = n * positions.stride;
        model.positions.push(V3::new(float(positions.bytes, at)?, float(positions.bytes, at + 4)?, float(positions.bytes, at + 8)?));
        let at = n * normals.stride;
        let normal = V3::new(float(normals.bytes, at)?, float(normals.bytes, at + 4)?, float(normals.bytes, at + 8)?);
        if normal.dot(normal) <= 1e-12 { return Err("Invalid GLB normal".into()); }
        model.normals.push(normal);
        let at = n * uvs.stride;
        model.texcoords.push([float(uvs.bytes, at)?, float(uvs.bytes, at + 4)?]);
    }
    for n in 0..indices.count {
        let at = n * indices.stride;
        let index = u16::from_le_bytes([indices.bytes[at], indices.bytes[at + 1]]);
        if index as usize >= positions.count { return Err("GLB index is outside vertex array".into()); }
        model.indices.push(index);
    }
    let mut min = V3::new(f32::MAX, f32::MAX, f32::MAX);
    let mut max = V3::new(f32::MIN, f32::MIN, f32::MIN);
    for point in &model.positions {
        min.x = min.x.min(point.x); min.y = min.y.min(point.y); min.z = min.z.min(point.z);
        max.x = max.x.max(point.x); max.y = max.y.max(point.y); max.z = max.z.max(point.z);
    }
    if max.x - min.x <= 1e-6 || max.y - min.y <= 1e-6 || max.z - min.z <= 1e-6 { return Err("Degenerate GLB dimensions".into()); }
    let scale = if normalize { (0.76 / (max.x - min.x).max(max.z - min.z)).min(1.3 / (max.y - min.y)) } else { 1.0 };
    for point in &mut model.positions {
        *point = V3::new((point.x - (min.x + max.x) * 0.5) * scale, (point.y - min.y) * scale, (point.z - (min.z + max.z) * 0.5) * scale);
    }
    Ok(model)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edited_document(bytes: &[u8], edit: impl FnOnce(&mut Value)) -> Vec<u8> {
        let json_len = u32_at(bytes, 12).unwrap() as usize;
        let mut document: Value = serde_json::from_slice(&bytes[20..20 + json_len]).unwrap();
        edit(&mut document);
        let mut json = serde_json::to_vec(&document).unwrap();
        while json.len() % 4 != 0 { json.push(b' '); }
        let binary = &bytes[20 + json_len..];
        let mut output = b"glTF".to_vec();
        output.extend(2_u32.to_le_bytes());
        output.extend(((20 + json.len() + binary.len()) as u32).to_le_bytes());
        output.extend((json.len() as u32).to_le_bytes());
        output.extend(b"JSON");
        output.extend(json);
        output.extend(binary);
        output
    }

    #[test]
    fn board_asset_packs_decode_every_model_and_texture_in_contract_order() {
        for (key, vertex_counts) in [
            (PackKey::Wood, [4416, 3156, 3072, 6504, 6810, 4644]),
            (PackKey::Marble, [1028, 2016, 1705, 1838, 1827, 2643]),
        ] {
            let pack = decode_pack(key, &embedded(key)).unwrap_or_else(|error| panic!("{}: {error}", key.name()));
            for (model, (count, height)) in [&pack.models.pawn, &pack.models.rook, &pack.models.knight, &pack.models.bishop, &pack.models.queen, &pack.models.king]
                .into_iter().zip(vertex_counts.into_iter().zip([0.90, 0.98, 1.15, 1.30, 1.43, 1.52])) {
                assert_eq!(model.positions.len(), count);
                let actual_height = model.positions.iter().map(|point| point.y).fold(0.0, f32::max);
                assert!((actual_height - height).abs() < 0.001);
                assert!(model.indices.iter().all(|&index| (index as usize) < count));
            }
            let maps = [&pack.textures.0, &pack.textures.1, &pack.textures.2, &pack.textures.3, &pack.textures.4, &pack.textures.5, &pack.textures.6];
            for (map, bytes) in maps.into_iter().zip(&embedded(key)[7..]) {
                let expected = image::load_from_memory_with_format(bytes, ImageFormat::Jpeg).unwrap().to_rgb8();
                assert_eq!(map, &expected);
            }
        }
    }

    #[test]
    fn board_asset_shared_dependency_mapping_keeps_glass_and_polyy_on_marble() {
        for theme in [Theme::Marble, Theme::Glass, Theme::ArtDeco, Theme::Egyptian] {
            assert_eq!(PackKey::for_theme(theme), PackKey::Marble);
        }
        assert_eq!(PackKey::for_theme(Theme::Wood), PackKey::Wood);
    }

    #[test]
    fn board_asset_requests_share_loads_latch_failure_and_allow_explicit_retry() {
        let ctx = Context::default();
        let second_ctx = Context::default();
        let mut request = RequestState::default();
        assert!(request.begin(&ctx, false));
        assert!(!request.begin(&ctx.clone(), false));
        assert!(!request.begin(&second_ctx, true));
        assert_eq!(request.listeners.len(), 2);
        request.finish(false);
        assert_eq!(request.state, LoadState::Failed);
        assert!(request.listeners.is_empty());
        assert!(!request.begin(&ctx, false));
        assert!(request.begin(&ctx, true));
        request.finish(true);
        assert_eq!(request.state, LoadState::Ready);
        assert!(!request.begin(&ctx, true));
    }

    #[test]
    fn board_asset_incomplete_or_corrupt_pack_is_not_installable() {
        for key in [PackKey::Wood, PackKey::Marble] {
            let mut files = embedded(key);
            assert!(decode_pack(key, &files[..13]).is_err());
            files[0] = b"glTF";
            assert!(decode_pack(key, &files).is_err());
            let mut files = embedded(key);
            files[7] = b"not a JPEG";
            assert!(decode_pack(key, &files).is_err());
        }
    }

    #[test]
    fn board_asset_glb_rejects_truncation_offsets_counts_and_invalid_indices() {
        let source = embedded(PackKey::Wood)[0];
        for length in (0..source.len()).step_by(17) {
            assert!(parse_glb_checked(&source[..length], false).is_err());
        }
        for broken in [
            edited_document(source, |doc| doc["accessors"][0]["count"] = Value::from(u64::MAX)),
            edited_document(source, |doc| doc["accessors"][0]["bufferView"] = Value::from(u64::MAX)),
            edited_document(source, |doc| doc["bufferViews"][0]["byteOffset"] = Value::from(u64::MAX)),
            edited_document(source, |doc| doc["bufferViews"][0]["byteLength"] = Value::from(1)),
            edited_document(source, |doc| doc["accessors"][3]["componentType"] = Value::from(5125)),
            edited_document(source, |doc| doc["accessors"][1]["count"] = Value::from(1)),
        ] {
            assert!(parse_glb_checked(&broken, false).is_err());
        }
        let json_len = u32_at(source, 12).unwrap() as usize;
        let doc: Value = serde_json::from_slice(&source[20..20 + json_len]).unwrap();
        let binary_at = 28 + json_len;
        let index_at = binary_at + number(&doc["bufferViews"][3]["byteOffset"]).unwrap();
        let mut broken = source.to_vec();
        broken[index_at..index_at + 2].copy_from_slice(&u16::MAX.to_le_bytes());
        assert!(parse_glb_checked(&broken, false).is_err());
        let mut broken = source.to_vec();
        broken[binary_at..binary_at + 4].copy_from_slice(&f32::NAN.to_le_bytes());
        assert!(parse_glb_checked(&broken, false).is_err());
    }
}
