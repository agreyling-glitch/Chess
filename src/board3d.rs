//! Poly Haven chess geometry and the shared CPU/GPU scene description.
//! The CPU rasterizer remains available for PNG export and renderer fallback.
pub mod gpu;
use chess::{Board, Color, File, Piece, Rank, Square};
use egui::{Color32, ColorImage, Pos2, Rect, Vec2};
use image::{ImageFormat, RgbImage};
use serde_json::Value;
use std::cell::RefCell;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PieceAppearance {
    pub size: u8,
    pub height: u8,
    pub custom_colors: bool,
    pub white: [u8; 3],
    pub black: [u8; 3],
    pub finish: u8,
    pub outline: bool,
    pub inner_shading: u8,
    pub gradient: bool,
    pub gradient_direction: u8,
    pub white_end: [u8;3],
    pub black_end: [u8;3],
    pub shadow_strength: u8,
    pub shadow_softness: u8,
    pub shadows: bool,
    pub board_appearance: Option<u8>,
    pub radial_light: Option<bool>,
}
impl Default for PieceAppearance {
    fn default() -> Self { Self { size:100,height:100,custom_colors:false,white:[235,232,225],black:[62,73,68],finish:0,outline:false,inner_shading:0,gradient:false,gradient_direction:0,white_end:[168,143,91],black_end:[12,20,32],shadow_strength:100,shadow_softness:50,shadows:true,board_appearance:None,radial_light:None } }
}
impl PieceAppearance {
    pub fn normalized(mut self) -> Self {
        self.gradient_direction=self.gradient_direction.min(3);
        self.inner_shading = self.inner_shading.min(100);
        self.board_appearance = self.board_appearance.map(|value| value.min(100));
        self.size = self.size.clamp(65,115); self.height = self.height.clamp(75,125);
        self.finish = self.finish.min(5); self.shadow_strength = self.shadow_strength.min(150); self.shadow_softness = self.shadow_softness.min(100); self
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    #[serde(alias = "bauhaus", alias = "scifi")]
    Marble,
    Wood,
    Glass,
    ArtDeco,
    Egyptian,
}

impl Theme {
    pub const ALL: [(Self, &'static str); 5] = [
        (Self::Marble, "Marble"), (Self::Wood, "Wood"), (Self::Glass, "Glass"),
        (Self::ArtDeco, "Art Deco"),
        (Self::Egyptian, "Egyptian"),
    ];

    fn is_polyy(self) -> bool {
        matches!(self, Self::ArtDeco | Self::Egyptian)
    }
}

#[derive(Clone, Copy, Default)]
struct V3 {
    x: f32,
    y: f32,
    z: f32,
}
impl V3 {
    fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
    fn add(self, b: Self) -> Self {
        Self::new(self.x + b.x, self.y + b.y, self.z + b.z)
    }
    fn sub(self, b: Self) -> Self {
        Self::new(self.x - b.x, self.y - b.y, self.z - b.z)
    }
    fn mul(self, s: f32) -> Self {
        Self::new(self.x * s, self.y * s, self.z * s)
    }
    fn dot(self, b: Self) -> f32 {
        self.x * b.x + self.y * b.y + self.z * b.z
    }
    fn cross(self, b: Self) -> Self {
        Self::new(
            self.y * b.z - self.z * b.y,
            self.z * b.x - self.x * b.z,
            self.x * b.y - self.y * b.x,
        )
    }
    fn unit(self) -> Self {
        self.mul(self.dot(self).sqrt().recip())
    }
}

#[derive(Clone)]
struct Model {
    positions: Vec<V3>,
    normals: Vec<V3>,
    texcoords: Vec<[f32; 2]>,
    indices: Vec<u16>,
}
impl Model {
    fn with_dimensions(mut self, footprint: f32, height: f32) -> Self {
        let current_height = self
            .positions
            .iter()
            .map(|point| point.y)
            .fold(0.0, f32::max);
        let horizontal_span = |axis: fn(&V3) -> f32| {
            let min = self
                .positions
                .iter()
                .map(axis)
                .fold(f32::INFINITY, f32::min);
            let max = self
                .positions
                .iter()
                .map(axis)
                .fold(f32::NEG_INFINITY, f32::max);
            max - min
        };
        let current_footprint =
            horizontal_span(|point| point.x).max(horizontal_span(|point| point.z));
        let horizontal_scale = footprint / current_footprint;
        let vertical_scale = height / current_height;
        for point in &mut self.positions {
            point.x *= horizontal_scale;
            point.y *= vertical_scale;
            point.z *= horizontal_scale;
        }
        for normal in &mut self.normals {
            *normal = V3::new(
                normal.x / horizontal_scale,
                normal.y / vertical_scale,
                normal.z / horizontal_scale,
            )
            .unit();
        }
        self
    }
}
struct Models {
    board: Model,
    pawn: Model,
    rook: Model,
    knight: Model,
    bishop: Model,
    queen: Model,
    king: Model,
}
static MODELS: OnceLock<Models> = OnceLock::new();
static WOOD_MODELS: OnceLock<Models> = OnceLock::new();
struct BoardBase {
    key: (usize, usize, bool, u32, u32, u32, Color32, u32, u32, Theme, bool),
    image: ColorImage,
    depth: Vec<f32>,
}
thread_local! {
    static BOARD_BASE: RefCell<Option<BoardBase>> = const { RefCell::new(None) };
}
static TEXTURES: OnceLock<(
    RgbImage,
    RgbImage,
    RgbImage,
    RgbImage,
    RgbImage,
    RgbImage,
    RgbImage,
)> = OnceLock::new();
static WOOD_TEXTURES: OnceLock<(
    RgbImage, RgbImage, RgbImage, RgbImage, RgbImage, RgbImage, RgbImage,
)> = OnceLock::new();
fn textures() -> &'static (
    RgbImage,
    RgbImage,
    RgbImage,
    RgbImage,
    RgbImage,
    RgbImage,
    RgbImage,
) {
    TEXTURES.get_or_init(|| {
        let decode = |bytes| {
            image::load_from_memory_with_format(bytes, ImageFormat::Jpeg)
                .expect("bundled chess-piece texture")
                .to_rgb8()
        };
        (
            decode(include_bytes!(
                "../assets/3d/polyhaven-chess-set/chess_set_pieces_white_diff_1k.jpg"
            )),
            decode(include_bytes!(
                "../assets/3d/polyhaven-chess-set/chess_set_pieces_black_diff_1k.jpg"
            )),
            decode(include_bytes!(
                "../assets/3d/polyhaven-chess-set/chess_set_board_diff_1k.jpg"
            )),
            decode(include_bytes!(
                "../assets/3d/polyhaven-chess-set/chess_set_board_nor_gl_1k.jpg"
            )),
            decode(include_bytes!(
                "../assets/3d/polyhaven-chess-set/chess_set_board_arm_1k.jpg"
            )),
            decode(include_bytes!(
                "../assets/3d/polyhaven-chess-set/chess_set_pieces_white_nor_gl_1k.jpg"
            )),
            decode(include_bytes!(
                "../assets/3d/polyhaven-chess-set/chess_set_pieces_black_nor_gl_1k.jpg"
            )),
        )
    })
}
fn theme_textures(theme: Theme) -> &'static (
    RgbImage, RgbImage, RgbImage, RgbImage, RgbImage, RgbImage, RgbImage,
) {
    if theme.is_polyy() { return polyy_textures(theme); }
    if theme != Theme::Wood { return textures(); }
    WOOD_TEXTURES.get_or_init(|| {
        let decode = |bytes| image::load_from_memory_with_format(bytes, ImageFormat::Jpeg)
            .expect("bundled Omie texture").to_rgb8();
        (
            decode(include_bytes!("../assets/3d/omies-chess-set/pieces_white_diff.jpg")),
            decode(include_bytes!("../assets/3d/omies-chess-set/pieces_black_diff.jpg")),
            decode(include_bytes!("../assets/3d/omies-chess-set/board_diff.jpg")),
            decode(include_bytes!("../assets/3d/omies-chess-set/board_nor.jpg")),
            decode(include_bytes!("../assets/3d/omies-chess-set/board_rough.jpg")),
            decode(include_bytes!("../assets/3d/omies-chess-set/pieces_white_nor.jpg")),
            decode(include_bytes!("../assets/3d/omies-chess-set/pieces_black_nor.jpg")),
        )
    })
}
type TextureSet = (RgbImage, RgbImage, RgbImage, RgbImage, RgbImage, RgbImage, RgbImage);
static DECO_TEXTURES: OnceLock<TextureSet> = OnceLock::new();
static EGYPT_TEXTURES: OnceLock<TextureSet> = OnceLock::new();

fn polyy_textures(theme: Theme) -> &'static TextureSet {
    let (slot, light, dark) = match theme {
        Theme::ArtDeco => (&DECO_TEXTURES, include_bytes!("../assets/3d/polyy-chess-pack-1/deco/light.jpg").as_slice(), include_bytes!("../assets/3d/polyy-chess-pack-1/deco/dark.jpg").as_slice()),
        Theme::Egyptian => (&EGYPT_TEXTURES, include_bytes!("../assets/3d/polyy-chess-pack-1/egypt/light.jpg").as_slice(), include_bytes!("../assets/3d/polyy-chess-pack-1/egypt/dark.jpg").as_slice()),
        _ => unreachable!(),
    };
    slot.get_or_init(|| {
        let base = textures();
        let decode = |data| image::load_from_memory_with_format(data, ImageFormat::Jpeg)
            .expect("Polyy chess atlas").to_rgb8();
        (decode(light), decode(dark), base.2.clone(), base.3.clone(),
         base.4.clone(), base.5.clone(), base.6.clone())
    })
}

static DECO_MODELS: OnceLock<Models> = OnceLock::new();
static EGYPT_MODELS: OnceLock<Models> = OnceLock::new();

fn polyy_models(theme: Theme) -> &'static Models {
    macro_rules! set_models {
        ($slot:expr, $dir:literal) => {
            $slot.get_or_init(|| Models {
                board: models().board.clone(),
                pawn: parse_glb(include_bytes!(concat!("../assets/3d/polyy-chess-pack-1/", $dir, "/pawn.glb")), true).with_dimensions(0.51, 0.90),
                rook: parse_glb(include_bytes!(concat!("../assets/3d/polyy-chess-pack-1/", $dir, "/rook.glb")), true).with_dimensions(0.58, 0.98),
                knight: parse_glb(include_bytes!(concat!("../assets/3d/polyy-chess-pack-1/", $dir, "/knight.glb")), true).with_dimensions(0.65, 1.15),
                bishop: parse_glb(include_bytes!(concat!("../assets/3d/polyy-chess-pack-1/", $dir, "/bishop.glb")), true).with_dimensions(0.63, 1.30),
                queen: parse_glb(include_bytes!(concat!("../assets/3d/polyy-chess-pack-1/", $dir, "/queen.glb")), true).with_dimensions(0.63, 1.43),
                king: parse_glb(include_bytes!(concat!("../assets/3d/polyy-chess-pack-1/", $dir, "/king.glb")), true).with_dimensions(0.66, 1.52),
            })
        };
    }
    match theme {
        Theme::ArtDeco => set_models!(DECO_MODELS, "deco"),
        Theme::Egyptian => set_models!(EGYPT_MODELS, "egypt"),
        _ => unreachable!(),
    }
}
fn theme_models(theme: Theme) -> &'static Models {
    if theme.is_polyy() { return polyy_models(theme); }
    if theme != Theme::Wood { return models(); }
    WOOD_MODELS.get_or_init(|| {
        let mut board = parse_glb(include_bytes!("../assets/3d/omies-chess-set/board.glb"), false);
        let span = board.positions.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max)
            - board.positions.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
        // The checker grid occupies pixels 49..463 of the board atlas's
        // 512-pixel top-face region. Scale that 414-pixel grid to eight move
        // units, rather than scaling the full board (including its frame).
        let grid_fraction = 414.0 / 512.0;
        let scale = 8.0 / (span * grid_fraction);
        let top = board.positions.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max) * scale;
        for point in &mut board.positions { *point = point.mul(scale); point.y += 0.055 - top; }
        align_wood_board_grid(&mut board);
        Models {
            board,
            pawn: parse_glb(include_bytes!("../assets/3d/omies-chess-set/pawn.glb"), true).with_dimensions(0.51, 0.90),
            rook: parse_glb(include_bytes!("../assets/3d/omies-chess-set/rook.glb"), true).with_dimensions(0.58, 0.98),
            knight: parse_glb(include_bytes!("../assets/3d/omies-chess-set/knight.glb"), true).with_dimensions(0.65, 1.15),
            bishop: parse_glb(include_bytes!("../assets/3d/omies-chess-set/bishop.glb"), true).with_dimensions(0.63, 1.30),
            queen: parse_glb(include_bytes!("../assets/3d/omies-chess-set/queen.glb"), true).with_dimensions(0.63, 1.43),
            king: parse_glb(include_bytes!("../assets/3d/omies-chess-set/king.glb"), true).with_dimensions(0.66, 1.52),
        }
    })
}

fn align_wood_board_grid(board: &mut Model) {
    // The atlas is a photograph-like texture: its eight checker columns and
    // rows are not perfectly uniform. Give each square its own top-face quad
    // so every visible seam lands on the move/highlight grid exactly.
    let x_pixels: [f32; 11] = [0.0, 47.0, 96.0, 148.0, 200.0, 252.0, 304.0, 357.0, 409.0, 461.0, 512.0];
    let z_pixels: [f32; 11] = [0.0, 47.0, 99.0, 151.0, 203.0, 256.0, 308.0, 361.0, 413.0, 461.0, 512.0];
    let min_x = board.positions.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
    let max_x = board.positions.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);
    let min_z = board.positions.iter().map(|p| p.z).fold(f32::INFINITY, f32::min);
    let max_z = board.positions.iter().map(|p| p.z).fold(f32::NEG_INFINITY, f32::max);
    let top = board.positions.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);
    let xs = [min_x, -4.0, -3.0, -2.0, -1.0, 0.0, 1.0, 2.0, 3.0, 4.0, max_x];
    let zs = [max_z, 4.0, 3.0, 2.0, 1.0, 0.0, -1.0, -2.0, -3.0, -4.0, min_z];
    let mut aligned = Model { positions: Vec::new(), normals: Vec::new(), texcoords: Vec::new(), indices: Vec::new() };
    let mut vertex = |point: V3, normal: V3, uv: [f32; 2]| {
        aligned.indices.push(aligned.positions.len() as u16);
        aligned.positions.push(point);
        aligned.normals.push(normal);
        aligned.texcoords.push(uv);
    };
    for face in board.indices.chunks_exact(3) {
        if face.iter().all(|&i| board.normals[i as usize].y > 0.9) { continue; }
        for &i in face {
            let i = i as usize;
            vertex(board.positions[i], board.normals[i], board.texcoords[i]);
        }
    }
    for row in 0..10 {
        for column in 0..10 {
            let corners = [
                (xs[column], zs[row], x_pixels[column], z_pixels[row]),
                (xs[column + 1], zs[row], x_pixels[column + 1], z_pixels[row]),
                (xs[column + 1], zs[row + 1], x_pixels[column + 1], z_pixels[row + 1]),
                (xs[column], zs[row + 1], x_pixels[column], z_pixels[row + 1]),
            ];
            for index in [0, 1, 2, 0, 2, 3] {
                let (x, z, u, v) = corners[index];
                vertex(V3::new(x, top, z), V3::new(0.0, 1.0, 0.0), [u / 1024.0, v / 1024.0]);
            }
        }
    }
    *board = aligned;
}
fn models() -> &'static Models {
    MODELS.get_or_init(|| Models {
        board: {
            let mut board = parse_glb(
                include_bytes!("../assets/3d/polyhaven-chess-set/board.glb"),
                false,
            );
            // Poly Haven's playing grid has a 0.057888-unit square pitch.
            // Match that pitch to our one-unit move and picking coordinates;
            // scaling the outer mesh to 8.56 instead misplaces edge pieces.
            let scale = 1.0 / 0.057_888;
            let top = board.positions.iter().map(|p| p.y).fold(0.0, f32::max) * scale;
            for point in &mut board.positions {
                *point = point.mul(scale);
                point.y += 0.055 - top;
            }
            board
        },
        pawn: parse_glb(
            include_bytes!("../assets/3d/polyhaven-chess-set/pawn.glb"),
            true,
        )
        .with_dimensions(0.51, 0.90),
        rook: parse_glb(
            include_bytes!("../assets/3d/polyhaven-chess-set/rook.glb"),
            true,
        )
        .with_dimensions(0.58, 0.98),
        knight: parse_glb(
            include_bytes!("../assets/3d/polyhaven-chess-set/knight.glb"),
            true,
        )
        .with_dimensions(0.65, 1.15),
        bishop: parse_glb(
            include_bytes!("../assets/3d/polyhaven-chess-set/bishop.glb"),
            true,
        )
        .with_dimensions(0.63, 1.30),
        queen: parse_glb(
            include_bytes!("../assets/3d/polyhaven-chess-set/queen.glb"),
            true,
        )
        .with_dimensions(0.63, 1.43),
        king: parse_glb(
            include_bytes!("../assets/3d/polyhaven-chess-set/king.glb"),
            true,
        )
        .with_dimensions(0.66, 1.52),
    })
}
impl Models {
    fn get(&self, piece: Piece) -> &Model {
        match piece {
            Piece::Pawn => &self.pawn,
            Piece::Rook => &self.rook,
            Piece::Knight => &self.knight,
            Piece::Bishop => &self.bishop,
            Piece::Queen => &self.queen,
            Piece::King => &self.king,
        }
    }
}

fn word(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}
fn parse_glb(bytes: &[u8], normalize: bool) -> Model {
    assert_eq!(&bytes[..4], b"glTF");
    let json_len = word(bytes, 12) as usize;
    let gltf: Value = serde_json::from_slice(&bytes[20..20 + json_len]).unwrap();
    let bin_at = 20 + json_len + 8;
    let primitive = &gltf["meshes"][0]["primitives"][0];
    let position_accessor =
        &gltf["accessors"][primitive["attributes"]["POSITION"].as_u64().unwrap() as usize];
    let index_accessor = &gltf["accessors"][primitive["indices"].as_u64().unwrap() as usize];
    let uv_accessor =
        &gltf["accessors"][primitive["attributes"]["TEXCOORD_0"].as_u64().unwrap() as usize];
    let normal_accessor =
        &gltf["accessors"][primitive["attributes"]["NORMAL"].as_u64().unwrap() as usize];
    let view =
        |accessor: &Value| &gltf["bufferViews"][accessor["bufferView"].as_u64().unwrap() as usize];
    let pos_at = bin_at
        + view(position_accessor)["byteOffset"].as_u64().unwrap_or(0) as usize
        + position_accessor["byteOffset"].as_u64().unwrap_or(0) as usize;
    let idx_at = bin_at
        + view(index_accessor)["byteOffset"].as_u64().unwrap_or(0) as usize
        + index_accessor["byteOffset"].as_u64().unwrap_or(0) as usize;
    let uv_at = bin_at
        + view(uv_accessor)["byteOffset"].as_u64().unwrap_or(0) as usize
        + uv_accessor["byteOffset"].as_u64().unwrap_or(0) as usize;
    let normal_at = bin_at
        + view(normal_accessor)["byteOffset"].as_u64().unwrap_or(0) as usize
        + normal_accessor["byteOffset"].as_u64().unwrap_or(0) as usize;
    let count = position_accessor["count"].as_u64().unwrap() as usize;
    let mut raw = Vec::with_capacity(count);
    let mut texcoords = Vec::with_capacity(count);
    let mut normals = Vec::with_capacity(count);
    for n in 0..count {
        let at = pos_at + n * 12;
        raw.push(V3::new(
            f32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()),
            f32::from_le_bytes(bytes[at + 4..at + 8].try_into().unwrap()),
            f32::from_le_bytes(bytes[at + 8..at + 12].try_into().unwrap()),
        ));
        let uv = uv_at + n * 8;
        texcoords.push([
            f32::from_le_bytes(bytes[uv..uv + 4].try_into().unwrap()),
            f32::from_le_bytes(bytes[uv + 4..uv + 8].try_into().unwrap()),
        ]);
        let normal = normal_at + n * 12;
        normals.push(V3::new(
            f32::from_le_bytes(bytes[normal..normal + 4].try_into().unwrap()),
            f32::from_le_bytes(bytes[normal + 4..normal + 8].try_into().unwrap()),
            f32::from_le_bytes(bytes[normal + 8..normal + 12].try_into().unwrap()),
        ));
    }
    let (mut min, mut max) = (
        V3::new(f32::MAX, f32::MAX, f32::MAX),
        V3::new(f32::MIN, f32::MIN, f32::MIN),
    );
    for p in &raw {
        min.x = min.x.min(p.x);
        min.y = min.y.min(p.y);
        min.z = min.z.min(p.z);
        max.x = max.x.max(p.x);
        max.y = max.y.max(p.y);
        max.z = max.z.max(p.z);
    }
    let scale = if normalize {
        (0.76 / (max.x - min.x).max(max.z - min.z)).min(1.3 / (max.y - min.y))
    } else {
        1.0
    };
    let positions = raw
        .into_iter()
        .map(|p| {
            V3::new(
                (p.x - (min.x + max.x) * 0.5) * scale,
                (p.y - min.y) * scale,
                (p.z - (min.z + max.z) * 0.5) * scale,
            )
        })
        .collect();
    assert_eq!(index_accessor["componentType"].as_u64(), Some(5123));
    let indices = (0..index_accessor["count"].as_u64().unwrap() as usize)
        .map(|n| {
            u16::from_le_bytes(
                bytes[idx_at + n * 2..idx_at + n * 2 + 2]
                    .try_into()
                    .unwrap(),
            )
        })
        .collect();
    Model {
        positions,
        normals,
        texcoords,
        indices,
    }
}

#[derive(Clone, Copy)]
struct Camera {
    eye: V3,
    right: V3,
    up: V3,
    forward: V3,
    focal: f32,
}
// Shared with GPU zoom so annotations and scene geometry use the same pivot.
const SCREEN_CENTER_LIFT: f32 = 0.08;

#[derive(Clone, Copy)]
pub struct View {
    pub yaw: f32,
    pub elevation: f32,
    pub distance: f32,
}
impl Default for View {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            elevation: 0.85,
            distance: 17.0,
        }
    }
}
impl View {
    /// Orbit in logical screen pixels, avoiding the vertical camera pole.
    pub fn orbit(&mut self, delta: Vec2) {
        self.yaw = (self.yaw - delta.x * 0.008).rem_euclid(std::f32::consts::TAU);
        self.elevation = (self.elevation + delta.y * 0.008).clamp(0.20, 1.45);
    }
}

impl Camera {
    fn new(flipped: bool, view: View) -> Self {
        let yaw = view.yaw + if flipped { std::f32::consts::PI } else { 0.0 };
        let horizontal = view.distance * view.elevation.cos();
        let eye = V3::new(
            horizontal * yaw.sin(),
            0.2 + view.distance * view.elevation.sin(),
            horizontal * yaw.cos(),
        );
        let forward = V3::new(0.0, 0.2, 0.0).sub(eye).unit();
        let right = forward.cross(V3::new(0.0, 1.0, 0.0)).unit();
        let up = right.cross(forward).unit();
        Self {
            eye,
            right,
            up,
            forward,
            focal: 2.9,
        }
    }
    fn screen_center(self, rect: Rect) -> Pos2 {
        // Lift the projected board within its egui canvas.
        rect.center() - Vec2::new(0.0, rect.height() * SCREEN_CENTER_LIFT)
    }
    fn project(self, p: V3, rect: Rect) -> Option<(Pos2, f32)> {
        let delta = p.sub(self.eye);
        let depth = delta.dot(self.forward);
        if depth <= 0.1 {
            return None;
        }
        let factor = self.focal / depth * rect.width() * 0.5;
        Some((
            self.screen_center(rect)
                + Vec2::new(delta.dot(self.right) * factor, -delta.dot(self.up) * factor),
            depth,
        ))
    }
    fn square_at(self, pos: Pos2, rect: Rect) -> Option<Square> {
        let center = self.screen_center(rect);
        let nx = (pos.x - center.x) / (rect.width() * 0.5 * self.focal);
        let ny = -(pos.y - center.y) / (rect.width() * 0.5 * self.focal);
        let ray = self.forward.add(self.right.mul(nx)).add(self.up.mul(ny));
        if ray.y.abs() < 0.0001 {
            return None;
        }
        let t = (0.075 - self.eye.y) / ray.y;
        if t <= 0.0 {
            return None;
        }
        let hit = self.eye.add(ray.mul(t));
        let file = (hit.x + 4.0).floor() as i32;
        let row = (hit.z + 4.0).floor() as i32;
        if !(0..8).contains(&file) || !(0..8).contains(&row) {
            return None;
        }
        Some(Square::make_square(
            Rank::from_index((7 - row) as usize),
            File::from_index(file as usize),
        ))
    }
}

struct Triangle {
    points: [Pos2; 3],
    depths: [f32; 3],
    color: Color32,
    texture: Option<([[f32; 2]; 3], Option<Color>)>,
    vertex_lighting: Option<[(f32, f32); 3]>,
    normal_surface: Option<NormalSurface>,
    shadow_uv: Option<[[f32; 2]; 3]>,
    glow_uv: Option<[[f32; 2]; 3]>,
}

struct NormalSurface {
    normals: [V3; 3],
    tangent: V3,
    bitangent: V3,
    center: V3,
}

fn studio_lighting(normal: V3, point: V3, camera: Camera, side: Option<Color>) -> (f32, f32) {
    studio_lighting_for_theme(normal, point, camera, side, Theme::Marble)
}

fn studio_lighting_for_theme(
    normal: V3, point: V3, camera: Camera, side: Option<Color>, theme: Theme,
) -> (f32, f32) {
    // The Poly Haven glTF has no lights. Recreate its preview's soft studio
    // key/fill and glossy marble highlights within this CPU rasterizer.
    let wood_board = theme == Theme::Wood && side.is_none();
    let key = if wood_board {
        V3::new(0.0, 1.0, 0.0)
    } else if theme == Theme::Wood {
        V3::new(-0.25, 1.0, 0.05).unit()
    } else {
        V3::new(-0.55, 1.0, 0.75).unit()
    };
    let fill = V3::new(0.8, 0.55, -0.35).unit();
    // A broad overhead reflection keeps the board's light pool centered
    // across ranks, independent of the camera's near edge or orbit.
    let view = if wood_board {
        V3::new(-point.x, 18.0, -point.z).unit()
    } else { camera.eye.sub(point).unit() };
    let black = side == Some(Color::Black);
    let center_light = if theme == Theme::Wood {
        (1.04 - 0.003 * (point.x * point.x + point.z * point.z)).clamp(0.96, 1.04)
    } else { 1.0 };
    let (ambient, key_power, fill_power) = if theme == Theme::Wood {
        (if black { 0.62 } else { 0.50 }, 0.25, 0.20)
    } else {
        (if black { 0.39 } else { 0.32 }, 0.43, if black { 0.29 } else { 0.17 })
    };
    let diffuse = ((ambient
        + key_power * normal.dot(key).max(0.0)
        + fill_power * normal.dot(fill).max(0.0)
        + 0.08 * normal.y.max(0.0)) * center_light)
    .clamp(0.32, if black { if theme == Theme::Wood { 1.12 } else { 1.08 } } else { 0.92 });
    let key_reflection = normal.dot(key.add(view).unit()).max(0.0);
    let fill_reflection = normal.dot(fill.add(view).unit()).max(0.0);
    let reflected = if wood_board {
        12.0 * key_reflection.powf(8.0) + 10.0 * key_reflection.powf(24.0)
    } else if theme == Theme::Wood {
        22.0 * key_reflection.powf(20.0)
            + 45.0 * key_reflection.powf(48.0)
            + 5.0 * fill_reflection.powf(32.0)
    } else {
        28.0 * key_reflection.powf(12.0)
            + 36.0 * key_reflection.powf(48.0)
            + 18.0 * fill_reflection.powf(16.0)
    };
    (
        diffuse,
        reflected * center_light,
    )
}

fn push_triangle(
    triangles: &mut Vec<Triangle>,
    camera: Camera,
    rect: Rect,
    points: [V3; 3],
    base: [u8; 3],
    texture: Option<([[f32; 2]; 3], Option<Color>)>,
    vertex_normals: Option<[V3; 3]>,
) {
    let normal = points[1]
        .sub(points[0])
        .cross(points[2].sub(points[0]))
        .unit();
    let light = V3::new(-0.5, 1.0, 0.6).unit();
    let shade = (0.47 + 0.53 * normal.dot(light).abs()).clamp(0.0, 1.0);
    let side = texture.and_then(|(_, side)| side);
    let vertex_lighting = vertex_normals.map(|normals| {
        std::array::from_fn(|i| studio_lighting(normals[i], points[i], camera, side))
    });
    let normal_surface = if let (Some((uv, _)), Some(normals)) = (texture, vertex_normals) {
        let edge1 = points[1].sub(points[0]);
        let edge2 = points[2].sub(points[0]);
        let du1 = uv[1][0] - uv[0][0];
        let dv1 = uv[1][1] - uv[0][1];
        let du2 = uv[2][0] - uv[0][0];
        let dv2 = uv[2][1] - uv[0][1];
        let determinant = du1 * dv2 - dv1 * du2;
        (determinant.abs() > 0.000001).then(|| NormalSurface {
            normals,
            tangent: edge1.mul(dv2).sub(edge2.mul(dv1)).mul(1.0 / determinant),
            bitangent: edge2.mul(du1).sub(edge1.mul(du2)).mul(1.0 / determinant),
            center: points[0].add(points[1]).add(points[2]).mul(1.0 / 3.0),
        })
    } else {
        None
    };
    let Some((a, da)) = camera.project(points[0], rect) else {
        return;
    };
    let Some((b, db)) = camera.project(points[1], rect) else {
        return;
    };
    let Some((c, dc)) = camera.project(points[2], rect) else {
        return;
    };
    triangles.push(Triangle {
        points: [a, b, c],
        depths: [da, db, dc],
        color: Color32::from_rgb(
            (base[0] as f32
                * if vertex_lighting.is_some() {
                    1.0
                } else {
                    shade
                }) as u8,
            (base[1] as f32
                * if vertex_lighting.is_some() {
                    1.0
                } else {
                    shade
                }) as u8,
            (base[2] as f32
                * if vertex_lighting.is_some() {
                    1.0
                } else {
                    shade
                }) as u8,
        ),
        texture,
        vertex_lighting,
        normal_surface,
        shadow_uv: None,
        glow_uv: None,
    });
}
fn quad(triangles: &mut Vec<Triangle>, camera: Camera, rect: Rect, pts: [V3; 4], color: [u8; 3]) {
    push_triangle(
        triangles,
        camera,
        rect,
        [pts[0], pts[1], pts[2]],
        color,
        None,
        None,
    );
    push_triangle(
        triangles,
        camera,
        rect,
        [pts[0], pts[2], pts[3]],
        color,
        None,
        None,
    );
}

fn overlay_quad(
    triangles: &mut Vec<Triangle>,
    camera: Camera,
    rect: Rect,
    pts: [V3; 4],
    color: [u8; 3],
) {
    let start = triangles.len();
    quad(triangles, camera, rect, pts, color);
    for triangle in &mut triangles[start..] {
        triangle.color = Color32::from_rgba_unmultiplied(color[0], color[1], color[2], 135);
    }
}

fn contact_shadow(
    triangles: &mut Vec<Triangle>,
    camera: Camera,
    rect: Rect,
    center: V3,
    radius: f32,
) {
    let x = center.x + 0.055;
    let z = center.z - 0.06;
    let start = triangles.len();
    quad(
        triangles,
        camera,
        rect,
        [
            V3::new(x - radius, 0.064, z - radius),
            V3::new(x + radius, 0.064, z - radius),
            V3::new(x + radius, 0.064, z + radius),
            V3::new(x - radius, 0.064, z + radius),
        ],
        [0, 0, 0],
    );
    if triangles.len() == start + 2 {
        triangles[start].shadow_uv = Some([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]);
        triangles[start + 1].shadow_uv = Some([[0.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
    }
}

// A light pool on a plane below the board follows camera orbit and zoom.
fn board_glow(camera: Camera, rect: Rect, theme: Theme) -> Vec<Triangle> {
    let model = &theme_models(theme).board;
    let y = model.positions.iter().map(|p| p.y).fold(f32::INFINITY, f32::min) - 0.08;
    let tint = match theme {
        Theme::Wood | Theme::ArtDeco | Theme::Egyptian => [232, 164, 80],
        Theme::Marble | Theme::Glass => [76, 170, 220],
    };
    let mut triangles = Vec::new();
    let radius = 7.8;
    quad(&mut triangles, camera, rect, [
        V3::new(-radius, y, -radius), V3::new(radius, y, -radius),
        V3::new(radius, y, radius), V3::new(-radius, y, radius),
    ], tint);
    if triangles.len() == 2 {
        triangles[0].glow_uv = Some([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]);
        triangles[1].glow_uv = Some([[0.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
        for triangle in &mut triangles { triangle.color = Color32::from_rgb(tint[0], tint[1], tint[2]); }
    }
    triangles
}

fn piece_yaw(piece: Piece, side: Color, square: Square) -> f32 {
    let kind = match piece {
        Piece::Knight => 1,
        Piece::Bishop => 2,
        Piece::King => 3,
        Piece::Pawn => 4,
        Piece::Rook => 5,
        Piece::Queen => 6,
    };
    // A square gives each moved piece a repeatable new facing, so stepping
    // backward and forward through game history does not make it jump around.
    let index = square.get_rank().to_index() * 8 + square.get_file().to_index();
    let offset = (index * 37 + kind * 17 + usize::from(side == Color::Black) * 13) % 64;
    let yaw = offset as f32 * std::f32::consts::TAU / 64.0;
    yaw + if piece == Piece::Knight && side == Color::Black {
        std::f32::consts::PI
    } else {
        0.0
    }
}

fn turn_y(point: V3, sin: f32, cos: f32) -> V3 {
    V3::new(point.x * cos - point.z * sin, point.y, point.x * sin + point.z * cos)
}

fn texture_rgb(texture: &RgbImage, uv: [f32; 2]) -> [f32; 3] {
    let width = texture.width() as usize;
    let height = texture.height() as usize;
    let x = uv[0].clamp(0.0, 1.0) * (width - 1) as f32;
    let y = uv[1].clamp(0.0, 1.0) * (height - 1) as f32;
    let (x0, y0) = (x as usize, y as usize);
    let (x1, y1) = ((x0 + 1).min(width - 1), (y0 + 1).min(height - 1));
    let data = texture.as_raw();
    let pixel = |px: usize, py: usize, channel: usize| data[(py * width + px) * 3 + channel] as f32;
    let (fx, fy) = (x.fract(), y.fract());
    std::array::from_fn(|channel| {
        let top = pixel(x0, y0, channel) * (1.0 - fx) + pixel(x1, y0, channel) * fx;
        let bottom = pixel(x0, y1, channel) * (1.0 - fx) + pixel(x1, y1, channel) * fx;
        top * (1.0 - fy) + bottom * fy
    })
}

fn texture_brightness(texture: &RgbImage, uv: [f32; 2]) -> f32 {
    let width = texture.width() as usize;
    let height = texture.height() as usize;
    let x = uv[0].clamp(0.0, 1.0) * (width - 1) as f32;
    let y = uv[1].clamp(0.0, 1.0) * (height - 1) as f32;
    let (x0, y0) = (x as usize, y as usize);
    let (x1, y1) = ((x0 + 1).min(width - 1), (y0 + 1).min(height - 1));
    let pixel = |px, py| {
        let at = (py * width + px) * 3;
        let data = texture.as_raw();
        data[at] as f32 * 0.2126 + data[at + 1] as f32 * 0.7152 + data[at + 2] as f32 * 0.0722
    };
    let (fx, fy) = (x.fract(), y.fract());
    let top = pixel(x0, y0) * (1.0 - fx) + pixel(x1, y0) * fx;
    let bottom = pixel(x0, y1) * (1.0 - fx) + pixel(x1, y1) * fx;
    top * (1.0 - fy) + bottom * fy
}

fn normal_mapped_lighting(
    surface: &NormalSurface,
    uv: [f32; 2],
    weights: [f32; 3],
    camera: Camera,
    map: &RgbImage,
    strength: f32,
    side: Option<Color>,
    roughness_map: Option<&RgbImage>,
    theme: Theme,
) -> (f32, f32) {
    let base = surface.normals[0]
        .mul(weights[0])
        .add(surface.normals[1].mul(weights[1]))
        .add(surface.normals[2].mul(weights[2]))
        .unit();
    let tangent = surface
        .tangent
        .sub(base.mul(surface.tangent.dot(base)))
        .unit();
    let handedness = if base.cross(tangent).dot(surface.bitangent) < 0.0 {
        -1.0
    } else {
        1.0
    };
    let bitangent = base.cross(tangent).mul(handedness);
    let mapped = texture_rgb(map, uv);
    let normal = base
        .mul((mapped[2] / 127.5 - 1.0).max(0.1))
        .add(tangent.mul((mapped[0] / 127.5 - 1.0) * strength))
        .add(bitangent.mul((mapped[1] / 127.5 - 1.0) * strength))
        .unit();
    let (diffuse, specular) = studio_lighting_for_theme(normal, surface.center, camera, side, theme);
    let gloss = roughness_map.map_or(1.0, |map| {
        // The glTF ARM map stores roughness in its green channel.
        let roughness = texture_rgb(map, uv)[1] / 255.0;
        if theme == Theme::Wood {
            (0.55 + 0.8 * (1.0 - roughness)).clamp(0.55, 1.35)
        } else {
            (0.42 + 1.18 * (1.0 - roughness)).clamp(0.42, 1.35)
        }
    });
    (diffuse, specular * gloss)
}
pub fn square_at(
    pos: Pos2,
    rect: Rect,
    flipped: bool,
    view: View,
    board: &Board,
    legal_targets: &[Square],
) -> Option<Square> {
    let camera = Camera::new(flipped, view);
    let board_square = camera.square_at(pos, rect);
    if board_square.is_some_and(|square| legal_targets.contains(&square)) {
        return board_square;
    }
    let mut nearest = None;
    let radius = rect.width() * 0.043;
    for rank in 0..8 {
        for file in 0..8 {
            let square = Square::make_square(Rank::from_index(rank), File::from_index(file));
            let Some(piece) = board.piece_on(square) else {
                continue;
            };
            let height = match piece {
                Piece::Pawn => 0.43,
                Piece::King | Piece::Queen => 0.68,
                _ => 0.52,
            };
            let center = V3::new(file as f32 - 3.5, height, 3.5 - rank as f32);
            if let Some((screen, _)) = camera.project(center, rect) {
                let distance = screen.distance(pos);
                if distance < radius && nearest.is_none_or(|(best, _)| distance < best) {
                    nearest = Some((distance, square));
                }
            }
        }
    }
    nearest
        .map(|(_, square)| square)
        .or(board_square)
}

pub fn board_square_at(pos: Pos2, rect: Rect, flipped: bool, view: View) -> Option<Square> {
    Camera::new(flipped, view).square_at(pos, rect)
}

pub fn square_outline(square: Square, rect: Rect, flipped: bool, view: View) -> Option<Vec<Pos2>> {
    let x = square.get_file().to_index() as f32 - 3.5;
    let z = 3.5 - square.get_rank().to_index() as f32;
    let camera = Camera::new(flipped, view);
    [(-0.43, -0.43), (0.43, -0.43), (0.43, 0.43), (-0.43, 0.43)]
        .into_iter().map(|(dx, dz)| camera.project(V3::new(x + dx, 0.075, z + dz), rect).map(|(point, _)| point))
        .collect()
}

pub fn circle_outline(square: Square, rect: Rect, flipped: bool, view: View) -> Option<Vec<Pos2>> {
    let x = square.get_file().to_index() as f32 - 3.5;
    let z = 3.5 - square.get_rank().to_index() as f32;
    let camera = Camera::new(flipped, view);
    (0..64).map(|i| {
        let angle = i as f32 * std::f32::consts::TAU / 64.0;
        camera.project(V3::new(x + angle.cos() * 0.43, 0.075, z + angle.sin() * 0.43), rect).map(|(point, _)| point)
    }).collect()
}

pub fn square_center(square: Square, rect: Rect, flipped: bool, view: View) -> Option<Pos2> {
    let file = square.get_file().to_index() as f32;
    let rank = square.get_rank().to_index() as f32;
    Camera::new(flipped, view)
        .project(V3::new(file - 3.5, 0.075, 3.5 - rank), rect)
        .map(|(point, _)| point)
}

pub fn image(
    board: &Board,
    rect: Rect,
    flipped: bool,
    view: View,
    selected: Option<Square>,
    targets: &[Square],
    last_move: Option<chess::ChessMove>,
    background: Color32,
    scale: f32,
    appearance: f32,
) -> ColorImage {
    image_with_theme(board, rect, flipped, view, selected, targets, last_move,
        background, scale, appearance, Theme::Marble)
}

pub fn image_with_theme(
    board: &Board, rect: Rect, flipped: bool, view: View,
    selected: Option<Square>, targets: &[Square], last_move: Option<chess::ChessMove>,
    background: Color32, scale: f32, appearance: f32, theme: Theme,
) -> ColorImage {
    image_with_options(board, rect, flipped, view, selected, targets, last_move, background, scale, appearance, theme, true)
}

pub fn image_with_options(
    board: &Board, rect: Rect, flipped: bool, view: View,
    selected: Option<Square>, targets: &[Square], last_move: Option<chess::ChessMove>,
    background: Color32, scale: f32, appearance: f32, theme: Theme, show_radial_light: bool,
) -> ColorImage {
    image_with_adjustments(board,rect,flipped,view,selected,targets,last_move,background,scale,appearance,theme,show_radial_light,PieceAppearance::default())
}
pub fn image_with_adjustments(
    board: &Board, rect: Rect, flipped: bool, view: View,
    selected: Option<Square>, targets: &[Square], last_move: Option<chess::ChessMove>,
    background: Color32, scale: f32, appearance: f32, theme: Theme, show_radial_light: bool, adjustments: PieceAppearance,
) -> ColorImage {
    let adjustments = adjustments.normalized();
    let camera = Camera::new(flipped, view);
    let width = (rect.width() * scale).ceil().max(1.0) as usize;
    let height = (rect.height() * scale).ceil().max(1.0) as usize;
    let key = (
        width,
        height,
        flipped,
        view.yaw.to_bits(),
        view.elevation.to_bits(),
        view.distance.to_bits(),
        background,
        scale.to_bits(),
        appearance.to_bits(),
        theme,
        show_radial_light,
    );
    let (mut image, mut depth_buffer) = BOARD_BASE.with(|base| {
        let mut base = base.borrow_mut();
        if base.as_ref().is_none_or(|cached| cached.key != key) {
            let mut image = ColorImage::filled([width, height], background);
            let mut depth = vec![0.0_f32; width * height];
            let mut board_triangles = if show_radial_light { board_glow(camera, rect, theme) } else { Vec::new() };
            let board_model = &theme_models(theme).board;
            for face in board_model.indices.chunks_exact(3) {
                let points = std::array::from_fn(|i| board_model.positions[face[i] as usize]);
                let uv = std::array::from_fn(|i| board_model.texcoords[face[i] as usize]);
                let normals = std::array::from_fn(|i| board_model.normals[face[i] as usize]);
                push_triangle(
                    &mut board_triangles,
                    camera,
                    rect,
                    points,
                    [230, 230, 230],
                    Some((uv, None)),
                    Some(normals),
                );
            }
            rasterize(&mut image, &mut depth, board_triangles, camera, rect, scale, appearance, theme, PieceAppearance::default());
            *base = Some(BoardBase {
                key,
                image,
                depth,
            });
        }
        let cached = base.as_ref().unwrap();
        (cached.image.clone(), cached.depth.clone())
    });
    let triangles = dynamic_triangles(board, rect, flipped, view, selected, targets, last_move, theme, adjustments);
    rasterize(&mut image, &mut depth_buffer, triangles, camera, rect, scale, appearance, theme, adjustments);
    image
}

fn dynamic_triangles(
    board: &Board, rect: Rect, flipped: bool, view: View,
    selected: Option<Square>, targets: &[Square], last_move: Option<chess::ChessMove>, theme: Theme, adjustments: PieceAppearance,
) -> Vec<Triangle> {
    let camera = Camera::new(flipped, view);
    let mut triangles = Vec::with_capacity(40000);
    let mut piece_triangles = Vec::with_capacity(40000);
    for rank in 0..8 {
        for file in 0..8 {
            let square = Square::make_square(Rank::from_index(rank), File::from_index(file));
            let x = file as f32 - 4.0;
            let z = 3.0 - rank as f32;
            let mut color = None;
            if last_move.is_some_and(|m| m.get_source() == square || m.get_dest() == square) {
                color = Some([180, 165, 78]);
            }
            if targets.contains(&square) {
                color = Some([125, 190, 123]);
            }
            if selected == Some(square) {
                color = Some([230, 178, 65]);
            }
            if let Some(color) = color {
                // The Wood atlas's painted checker seams sit slightly away
                // from the mesh grid. Keep the tint full-size and register it
                // to the painted squares rather than shrinking its footprint.
                let (highlight_x, highlight_z) = if theme == Theme::Wood {
                    (x + 0.04, z - 0.035)
                } else { (x, z) };
                overlay_quad(
                    &mut triangles,
                    camera,
                    rect,
                    [
                        V3::new(highlight_x, 0.061, highlight_z),
                        V3::new(highlight_x + 1.0, 0.061, highlight_z),
                        V3::new(highlight_x + 1.0, 0.061, highlight_z + 1.0),
                        V3::new(highlight_x, 0.061, highlight_z + 1.0),
                    ],
                    color,
                );
            }
            if let (Some(piece), Some(side)) = (board.piece_on(square), board.color_on(square)) {
                let model = theme_models(theme).get(piece);
                let center = V3::new(file as f32 - 3.5, 0.075, 3.5 - rank as f32);
                let yaw = piece_yaw(piece, side, square);
                let (sin, cos) = yaw.sin_cos();
                let shadow_center = if theme == Theme::Wood {
                    center.add(V3::new(-0.055, 0.0, 0.06))
                } else { center };
                if adjustments.shadows && adjustments.shadow_strength > 0 { contact_shadow(
                    &mut piece_triangles,
                    camera,
                    rect,
                    shadow_center,
                    (if piece == Piece::Pawn { 0.45 } else { 0.54 }) * adjustments.size as f32 / 100.0 * (0.75 + adjustments.shadow_softness as f32 / 200.0),
                ); }
                let color = if side == Color::White {
                    [235, 232, 225]
                } else {
                    [62, 73, 68]
                };
                for face in model.indices.chunks_exact(3) {
                    let mut pts = [V3::default(); 3];
                    for (n, &i) in face.iter().enumerate() {
                        let p = turn_y(model.positions[i as usize], sin, cos);
                        pts[n] = center.add(V3::new(p.x * adjustments.size as f32 / 100.0, p.y * adjustments.size as f32 / 100.0 * adjustments.height as f32 / 100.0, p.z * adjustments.size as f32 / 100.0));
                    }
                    let uv = [
                        model.texcoords[face[0] as usize],
                        model.texcoords[face[1] as usize],
                        model.texcoords[face[2] as usize],
                    ];
                    let normals = std::array::from_fn(|i| {
                        { let n = turn_y(model.normals[face[i] as usize], sin, cos); V3::new(n.x,n.y*100.0/adjustments.height as f32,n.z).unit() }
                    });
                    push_triangle(
                        &mut piece_triangles,
                        camera,
                        rect,
                        pts,
                        color,
                        Some((uv, Some(side))),
                        Some(normals),
                    );
                }
            }
        }
    }
    triangles.extend(piece_triangles);
    triangles
}

fn appearance_color(color: Color32, appearance: f32) -> Color32 {
    if appearance == 0.5 {
        return color;
    }
    let strength = ((appearance - 0.5) * 2.0).clamp(-1.0, 1.0);
    let contrast = 1.0 + 0.2 * strength;
    let brightness = 0.08 * strength;
    let adjust = |channel: u8| {
        (((channel as f32 / 255.0 - 0.42) * contrast + 0.42 + brightness)
            .clamp(0.0, 1.0) * 255.0) as u8
    };
    Color32::from_rgb(adjust(color.r()), adjust(color.g()), adjust(color.b()))
}

pub fn coordinate_labels(rect: Rect, flipped: bool, view: View, theme: Theme) -> Vec<(char, Pos2)> {
    let camera = Camera::new(flipped, view);
    let board = &theme_models(theme).board;
    let front_z = board.positions.iter().map(|p| p.z).fold(
        if flipped { f32::INFINITY } else { f32::NEG_INFINITY },
        |edge, z| if flipped { edge.min(z) } else { edge.max(z) },
    );
    let left_x = board.positions.iter().map(|p| p.x).fold(
        if flipped { f32::NEG_INFINITY } else { f32::INFINITY },
        |edge, x| if flipped { edge.max(x) } else { edge.min(x) },
    );
    let bottom_y = board.positions.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
    let file_spacing = camera.project(V3::new(-2.5, bottom_y, front_z), rect)
        .zip(camera.project(V3::new(-3.5, bottom_y, front_z), rect))
        .map(|((a, _), (b, _))| a.distance(b))
        .unwrap_or(48.0);
    let font_size = (file_spacing * 0.22).clamp(10.0, 18.0);
    let file_margin = font_size * 0.5 + 11.0;
    let rank_margin = font_size * 0.5 + 3.0;
    let mut labels = Vec::with_capacity(16);
    for file in 0..8 {
        if let Some((point, _)) = camera.project(V3::new(file as f32 - 3.5, bottom_y, front_z), rect) {
            // At close zoom the frame can extend past the canvas. Keep the
            // label in the small strip below it rather than losing it.
            let y = (point.y + file_margin).min(rect.max.y + 8.0);
            labels.push(((b'a' + file as u8) as char, Pos2::new(point.x, y)));
        }
    }
    for rank in 0..8 {
        if let Some((point, _)) = camera.project(V3::new(left_x, 0.07, 3.5 - rank as f32), rect) {
            let x = (point.x - rank_margin).max(rect.min.x - 2.0);
            labels.push(((b'1' + rank as u8) as char, Pos2::new(x, point.y)));
        }
    }
    labels
}

fn rasterize(
    image: &mut ColorImage,
    depth_buffer: &mut [f32],
    triangles: Vec<Triangle>,
    camera: Camera,
    rect: Rect,
    scale: f32,
    appearance: f32,
    theme: Theme,
    adjustments: PieceAppearance,
) {
    let [width, height] = image.size;
    let mut silhouettes = if adjustments.outline { vec![None; width*height] } else { Vec::new() };
    for tri in triangles {
        let p = tri.points.map(|point| (point - rect.min) * scale);
        let min_x = p
            .iter()
            .map(|point| point.x)
            .fold(f32::INFINITY, f32::min)
            .floor()
            .max(0.0) as usize;
        let max_x = p
            .iter()
            .map(|point| point.x)
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil()
            .min(width as f32) as usize;
        let min_y = p
            .iter()
            .map(|point| point.y)
            .fold(f32::INFINITY, f32::min)
            .floor()
            .max(0.0) as usize;
        let max_y = p
            .iter()
            .map(|point| point.y)
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil()
            .min(height as f32) as usize;
        let denominator =
            (p[1].y - p[2].y) * (p[0].x - p[2].x) + (p[2].x - p[1].x) * (p[0].y - p[2].y);
        if denominator.abs() < 0.00001 {
            continue;
        }
        let reciprocal = tri.depths.map(f32::recip);
        for y in min_y..max_y {
            let py = y as f32 + 0.5;
            for x in min_x..max_x {
                let px = x as f32 + 0.5;
                let w0 = ((p[1].y - p[2].y) * (px - p[2].x) + (p[2].x - p[1].x) * (py - p[2].y))
                    / denominator;
                let w1 = ((p[2].y - p[0].y) * (px - p[2].x) + (p[0].x - p[2].x) * (py - p[2].y))
                    / denominator;
                let w2 = 1.0 - w0 - w1;
                if w0 < -0.00001 || w1 < -0.00001 || w2 < -0.00001 {
                    continue;
                }
                let near = w0 * reciprocal[0] + w1 * reciprocal[1] + w2 * reciprocal[2];
                let index = y * width + x;
                if near > depth_buffer[index] {
                    image.pixels[index] = if let Some(uv) = tri.glow_uv {
                        let u = (w0 * reciprocal[0] * uv[0][0] + w1 * reciprocal[1] * uv[1][0]
                            + w2 * reciprocal[2] * uv[2][0]) / near;
                        let v = (w0 * reciprocal[0] * uv[0][1] + w1 * reciprocal[1] * uv[1][1]
                            + w2 * reciprocal[2] * uv[2][1]) / near;
                        let radius_sq = (2.0 * u - 1.0).powi(2) + (2.0 * v - 1.0).powi(2);
                        let alpha = 0.55 * (1.0 - radius_sq).max(0.0).powi(2);
                        let previous = image.pixels[index];
                        Color32::from_rgb(
                            (previous.r() as f32 * (1.0 - alpha) + tri.color.r() as f32 * alpha) as u8,
                            (previous.g() as f32 * (1.0 - alpha) + tri.color.g() as f32 * alpha) as u8,
                            (previous.b() as f32 * (1.0 - alpha) + tri.color.b() as f32 * alpha) as u8,
                        )
                    } else if let Some(uv) = tri.shadow_uv {
                        let u = (w0 * reciprocal[0] * uv[0][0]
                            + w1 * reciprocal[1] * uv[1][0]
                            + w2 * reciprocal[2] * uv[2][0])
                            / near;
                        let v = (w0 * reciprocal[0] * uv[0][1]
                            + w1 * reciprocal[1] * uv[1][1]
                            + w2 * reciprocal[2] * uv[2][1])
                            / near;
                        let radius_sq = (2.0 * u - 1.0).powi(2) + (2.0 * v - 1.0).powi(2);
                        let alpha = (0.34 * adjustments.shadow_strength as f32 / 100.0 * (1.0 - radius_sq).max(0.0).powf(1.0 + (100.0-adjustments.shadow_softness as f32)/50.0)).min(0.85);
                        if alpha <= 0.001 {
                            continue;
                        }
                        let previous = image.pixels[index];
                        Color32::from_rgb(
                            (previous.r() as f32 * (1.0 - alpha)) as u8,
                            (previous.g() as f32 * (1.0 - alpha)) as u8,
                            (previous.b() as f32 * (1.0 - alpha)) as u8,
                        )
                    } else if let Some((uv, side)) = tri.texture {
                        // Divide by interpolated reciprocal depth to keep the texture attached
                        // to the model under perspective projection.
                        let u = (w0 * reciprocal[0] * uv[0][0]
                            + w1 * reciprocal[1] * uv[1][0]
                            + w2 * reciprocal[2] * uv[2][0])
                            / near;
                        let v = (w0 * reciprocal[0] * uv[0][1]
                            + w1 * reciprocal[1] * uv[1][1]
                            + w2 * reciprocal[2] * uv[2][1])
                            / near;
                        let (diffuse, specular) =
                            tri.vertex_lighting.map_or((1.0, 0.0), |lighting| {
                                let interpolate = |component: usize| {
                                    let values = if component == 0 {
                                        [lighting[0].0, lighting[1].0, lighting[2].0]
                                    } else {
                                        [lighting[0].1, lighting[1].1, lighting[2].1]
                                    };
                                    (w0 * reciprocal[0] * values[0]
                                        + w1 * reciprocal[1] * values[1]
                                        + w2 * reciprocal[2] * values[2])
                                        / near
                                };
                                (interpolate(0), interpolate(1))
                            });
                        let gloss = 1.0 + 0.75 * ((appearance - 0.5) * 2.0).clamp(-1.0, 1.0);
                        let surface_color = if let Some(side) = side {
                            let maps = theme_textures(theme);
                            let texture = if side == Color::White {
                                &maps.0
                            } else {
                                &maps.1
                            };
                            let normal_map = if side == Color::White {
                                &maps.5
                            } else {
                                &maps.6
                            };
                            let (diffuse, specular) = if theme.is_polyy() { (diffuse, specular) } else { tri.normal_surface.as_ref().map_or(
                                (diffuse, specular),
                                |surface| {
                                    normal_mapped_lighting(
                                        surface,
                                        [u, v],
                                        [
                                            w0 * reciprocal[0] / near,
                                            w1 * reciprocal[1] / near,
                                            w2 * reciprocal[2] / near,
                                        ],
                                        camera,
                                        normal_map,
                                        0.55,
                                        Some(side),
                                        None,
                                        theme,
                                    )
                                },
                            ) };
                            let specular = specular * gloss;
                            if theme.is_polyy() {
                                let paint = texture_rgb(texture, [u, v]);
                                Color32::from_rgb(
                                    (paint[0] * diffuse + specular * 0.6).min(250.0) as u8,
                                    (paint[1] * diffuse + specular * 0.6).min(250.0) as u8,
                                    (paint[2] * diffuse + specular * 0.6).min(250.0) as u8,
                                )
                            } else if theme == Theme::Wood {
                                let wood = texture_rgb(texture, [u, v]);
                                let (scale, lift) = if side == Color::White { ([0.99, 0.98, 0.96], 6.0) } else { ([0.72, 0.74, 0.76], 3.0) };
                                Color32::from_rgb(
                                    ((wood[0] * scale[0] + lift) * diffuse + specular * 0.35).min(245.0) as u8,
                                    ((wood[1] * scale[1] + lift) * diffuse + specular * 0.35).min(245.0) as u8,
                                    ((wood[2] * scale[2] + lift) * diffuse + specular * 0.35).min(245.0) as u8,
                                )
                            } else if theme == Theme::Glass {
                                let base = tri.normal_surface.as_ref().map_or(V3::new(0.0, 1.0, 0.0), |surface| {
                                    surface.normals[0].mul(w0).add(surface.normals[1].mul(w1)).add(surface.normals[2].mul(w2)).unit()
                                });
                                let view = camera.eye.sub(tri.normal_surface.as_ref().map_or(V3::default(), |s| s.center)).unit();
                                let rim = (1.0 - base.dot(view).abs()).powf(2.5);
                                let tint = if side == Color::White { [151.0, 202.0, 219.0] } else { [168.0, 51.0, 112.0] };
                                let rim_color = if side == Color::White { [120.0, 130.0, 135.0] } else { [128.0, 61.0, 105.0] };
                                Color32::from_rgb(
                                    (tint[0] * (0.42 + 0.25 * diffuse) + rim_color[0] * rim + specular * 1.5).min(250.0) as u8,
                                    (tint[1] * (0.42 + 0.25 * diffuse) + rim_color[1] * rim + specular * 1.5).min(250.0) as u8,
                                    (tint[2] * (0.42 + 0.25 * diffuse) + rim_color[2] * rim + specular * 1.5).min(250.0) as u8,
                                )
                            } else {
                            let reference = if side == Color::White { 210.0 } else { 48.0 };
                            let detail =
                                (texture_brightness(texture, [u, v]) / reference).clamp(0.0, 1.35);
                            let brightness = (0.78 + 0.22 * detail) * diffuse;
                            let ceiling = if side == Color::White { 245.0 } else { 230.0 };
                            Color32::from_rgb(
                                (tri.color.r() as f32 * brightness + specular).min(ceiling) as u8,
                                (tri.color.g() as f32 * brightness + specular * 0.97).min(ceiling)
                                    as u8,
                                (tri.color.b() as f32 * brightness + specular * 0.93).min(ceiling)
                                    as u8,
                            )
                            }
                        } else {
                            let maps = theme_textures(theme);
                            let marble = texture_rgb(&maps.2, [u, v]);
                            let (diffuse, specular) = tri.normal_surface.as_ref().map_or(
                                (diffuse, specular),
                                |surface| {
                                    normal_mapped_lighting(
                                        surface,
                                        [u, v],
                                        [
                                            w0 * reciprocal[0] / near,
                                            w1 * reciprocal[1] / near,
                                            w2 * reciprocal[2] / near,
                                        ],
                                        camera,
                                        &maps.3,
                                        0.7,
                                        None,
                                        Some(&maps.4),
                                        theme,
                                    )
                                },
                            );
                            let specular = specular * gloss;
                            let color = if theme == Theme::Wood {
                                // Keep the wood grain and square contrast while letting the
                                // colored pieces stand out against a nearly neutral board.
                                let gray = marble[0] * 0.2126 + marble[1] * 0.7152 + marble[2] * 0.0722;
                                [
                                    (gray * 0.88 + marble[0] * 0.12) * 0.72,
                                    (gray * 0.88 + marble[1] * 0.12) * 0.72,
                                    (gray * 0.88 + marble[2] * 0.12) * 0.72,
                                ]
                            } else if theme == Theme::Glass {
                                [marble[0] * 0.40 + 20.0, marble[1] * 0.52 + 35.0, marble[2] * 0.63 + 48.0]
                            } else { marble };
                            Color32::from_rgb(
                                (color[0] * diffuse + specular).min(230.0) as u8,
                                (color[1] * diffuse + specular).min(230.0) as u8,
                                (color[2] * diffuse + specular).min(230.0) as u8,
                            )
                        };
                        let surface_color = if let Some(side) = side {
                            if adjustments.custom_colors || adjustments.finish != 0 || adjustments.outline {
                                let base = if adjustments.custom_colors { if side == Color::White { adjustments.white } else { adjustments.black } } else { [surface_color.r(),surface_color.g(),surface_color.b()] };
                                let factor = match adjustments.finish { 1 => 0.0, 2 => 0.35, 3 => 1.5, 4 => 2.2, 5 => 1.8, _ => 1.0 };
                                let shade = if adjustments.custom_colors { diffuse } else { 1.0 };
                                let channel = |c: u8| ((c as f32 - if adjustments.custom_colors { 0.0 } else { specular }).max(0.0) * shade + specular * factor).clamp(0.0,255.0) as u8;
                                Color32::from_rgb(channel(base[0]),channel(base[1]),channel(base[2]))
                            } else { surface_color }
                        } else { surface_color };
                        let adjusted = appearance_color(surface_color, appearance);
                        if side.is_some() && adjustments.finish == 5 {
                            let previous = image.pixels[index];
                            Color32::from_rgb((adjusted.r() as f32*0.78+previous.r() as f32*0.22) as u8,(adjusted.g() as f32*0.78+previous.g() as f32*0.22) as u8,(adjusted.b() as f32*0.78+previous.b() as f32*0.22) as u8)
                        } else { adjusted }
                    } else {
                        let alpha = tri.color.a() as f32 / 255.0;
                        let previous = image.pixels[index];
                        Color32::from_rgb(
                            (previous.r() as f32 * (1.0 - alpha) + tri.color.r() as f32 * alpha)
                                as u8,
                            (previous.g() as f32 * (1.0 - alpha) + tri.color.g() as f32 * alpha)
                                as u8,
                            (previous.b() as f32 * (1.0 - alpha) + tri.color.b() as f32 * alpha)
                                as u8,
                        )
                    };
                    if adjustments.outline { silhouettes[index] = tri.texture.and_then(|(_,side)| side); }
                    depth_buffer[index] = near;
                }
            }
        }
    }
    if adjustments.outline {
        paint_silhouette_outline(image, &silhouettes, depth_buffer, adjustments);
    }

}

// A screen-space dilation touches only pixels outside the visible pieces.
fn paint_silhouette_outline(image: &mut ColorImage, mask: &[Option<Color>], depths: &[f32], settings: PieceAppearance) {
    let [width,height] = image.size;
    for y in 1..height.saturating_sub(1) { for x in 1..width.saturating_sub(1) {
        let index = y*width+x;
        if mask[index].is_some() { continue; }
        for neighbor in [index-1,index+1,index-width,index+width] {
            if let Some(side) = mask[neighbor] {
                if depths[neighbor] < depths[index] { continue; }
                let base = if settings.custom_colors { if side == Color::White { settings.white } else { settings.black } } else if side == Color::White { [240;3] } else { [25;3] };
                image.pixels[index] = if base.iter().map(|v| *v as u16).sum::<u16>() < 380 { Color32::from_gray(235) } else { Color32::from_gray(10) };
                break;
            }
        }
    }}
}

#[cfg(test)]
mod tests {
    #[test]
    fn silhouette_outline_preserves_interior_and_respects_occlusion() {
        let original = Color32::from_rgb(110,80,50);
        let mut image = ColorImage::new([7,7], vec![original;49]);
        let mut mask = vec![None;49];
        let mut depths = vec![1.0;49];
        for y in 2..5 { for x in 2..5 { mask[y*7+x] = Some(Color::White); depths[y*7+x] = 2.0; }}
        depths[1*7+3] = 3.0; // A foreground surface blocks the outline.
        paint_silhouette_outline(&mut image,&mask,&depths,PieceAppearance::default());
        for y in 2..5 { for x in 2..5 { assert_eq!(image.pixels[y*7+x],original); }}
        assert_eq!(image.pixels[1*7+3],original);
        assert_eq!(image.pixels[0],original);
        assert_ne!(image.pixels[3*7+1],original);
    }
    #[test]
    fn appearance_adjustments_update_cached_rendering() {
        let board = Board::default();
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::splat(220.0));
        let render = |settings| image_with_adjustments(&board, rect, false, View::default(),
            None, &[], None, Color32::from_rgb(24,27,31), 1.0, 0.5, Theme::Wood, true, settings);
        let defaults = PieceAppearance::default();
        let baseline = render(defaults);
        for changed in [PieceAppearance { size:75, ..defaults }, PieceAppearance { height:125, ..defaults },
            PieceAppearance { custom_colors:true, white:[230,40,80], ..defaults },
            PieceAppearance { outline:true, ..defaults }, PieceAppearance { finish:5, ..defaults },
            PieceAppearance { shadow_strength:0, ..defaults }] {
            assert_ne!(baseline, render(changed));
            assert_eq!(baseline, render(defaults));
        }
    }
    #[test]
    fn radial_light_toggle_updates_cached_rendering() {
        let board = Board::default();
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::splat(220.0));
        let render = |glow| image_with_options(&board, rect, false, View::default(),
            None, &[], None, Color32::from_rgb(24, 27, 31), 1.0, 0.5, Theme::Wood, glow);
        let on = render(true);
        let off = render(false);
        assert_ne!(on, off);
        assert_eq!(on, render(true));
    }
    #[test]
    fn wood_board_lighting_is_balanced_across_ranks_and_camera_views() {
        let normal = V3::new(0.0, 1.0, 0.0);
        let first = Camera::new(false, View::default());
        let mut orbit = View::default();
        orbit.orbit(Vec2::new(120.0, 30.0));
        for camera in [first, Camera::new(true, View::default()), Camera::new(false, orbit)] {
            for x in [-3.5, -0.5, 0.5, 3.5] {
                for z in [0.5, 1.5, 2.5, 3.5] {
                    let near = studio_lighting_for_theme(normal, V3::new(x, 0.06, z), camera, None, Theme::Wood);
                    let far = studio_lighting_for_theme(normal, V3::new(x, 0.06, -z), camera, None, Theme::Wood);
                    assert!((near.0 - far.0).abs() < 0.0001);
                    assert!((near.1 - far.1).abs() < 0.0001);
                    let initial = studio_lighting_for_theme(normal, V3::new(x, 0.06, z), first, None, Theme::Wood);
                    assert!((near.1 - initial.1).abs() < 0.0001);
                }
            }
        }
    }
    use super::*;

    #[test]
    fn orbit_keeps_projection_and_picking_aligned() {
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::splat(600.0));
        let board: Board = "7k/8/8/8/8/8/8/K7 w - - 0 1".parse().unwrap();
        for flipped in [false, true] {
            let mut view = View::default();
            for _ in 0..16 {
                view.orbit(Vec2::new(60.0, 0.0));
                for square in [Square::D4, Square::E5, Square::B3, Square::G6] {
                    let pos = square_center(square, rect, flipped, view).unwrap();
                    assert_eq!(square_at(pos, rect, flipped, view, &board, &[]), Some(square));
                }
            }
            for delta in [Vec2::new(10000.0, -10000.0), Vec2::new(-10000.0, 10000.0)] {
                view.orbit(delta);
                let camera = Camera::new(flipped, view);
                assert!(camera.right.x.is_finite() && camera.up.y.is_finite());
            }
        }
    }

    #[test]
    fn underboard_glow_renders_outside_board_with_theme_tint() {
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::splat(600.0));
        let background = Color32::from_rgb(24, 27, 31);
        for theme in Theme::ALL.map(|(theme, _)| theme) {
            let camera = Camera::new(false, View::default());
            let glow = board_glow(camera, rect, theme);
            assert_eq!(glow.len(), 2);
            let mut halo = ColorImage::filled([600, 600], background);
            let mut depth = vec![0.0; 600 * 600];
            rasterize(&mut halo, &mut depth, glow, camera, rect, 1.0, 0.5, theme, PieceAppearance::default());
            assert!(halo.pixels.iter().any(|p| p != &background));
            let rendered = image_with_theme(&Board::default(), rect, false, View::default(),
                None, &[], None, background, 1.0, 0.5, theme);
            // The board occludes the center, while a visible halo remains outside it.
            assert!(rendered.pixels.iter().zip(&halo.pixels).any(|(p, h)| p == h && h != &background));
            if std::env::var_os("IRONWOOD_GLOW_PREVIEW").is_some() {
                let mut output = RgbImage::new(600, 600);
                for (pixel, color) in output.pixels_mut().zip(rendered.pixels) {
                    *pixel = image::Rgb([color.r(), color.g(), color.b()]);
                }
                output.save(format!("target/underlight-{theme:?}.png")).unwrap();
            }
        }
    }
    #[test]
    fn bundled_gltf_models_load() {
        for model in [
            &models().board,
            &models().pawn,
            &models().rook,
            &models().knight,
            &models().bishop,
            &models().queen,
            &models().king,
        ] {
            assert!(!model.positions.is_empty());
            assert_eq!(model.normals.len(), model.positions.len());
            assert!(model
                .normals
                .iter()
                .all(|n| n.x.is_finite() && n.y.is_finite() && n.z.is_finite()));
            assert_eq!(model.texcoords.len(), model.positions.len());
            assert!(model
                .texcoords
                .iter()
                .all(|uv| uv.iter().all(|v| v.is_finite())));
            assert!(model
                .indices
                .iter()
                .all(|&i| (i as usize) < model.positions.len()));
        }
        assert_eq!(textures().0.dimensions(), (1024, 1024));
        assert_eq!(textures().1.dimensions(), (1024, 1024));
        assert_eq!(textures().2.dimensions(), (1024, 1024));
        assert_eq!(textures().3.dimensions(), (1024, 1024));
        assert_eq!(textures().4.dimensions(), (1024, 1024));
        assert_eq!(textures().5.dimensions(), (1024, 1024));
        assert_eq!(textures().6.dimensions(), (1024, 1024));
    }
    #[test]
    fn wood_models_and_maps_load() {
        let models = theme_models(Theme::Wood);
        let board_span = models.board.positions.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max)
            - models.board.positions.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
        assert!((board_span * 414.0 / 512.0 - 8.0).abs() < 0.01);
        for x in -4..=4 {
            for z in -4..=4 {
                assert!(models.board.positions.iter().any(|p|
                    (p.x - x as f32).abs() < 0.001 && (p.z - z as f32).abs() < 0.001
                    && (p.y - 0.055).abs() < 0.001));
            }
        }
        for model in [&models.board, &models.pawn, &models.rook, &models.knight,
            &models.bishop, &models.queen, &models.king] {
            assert!(!model.positions.is_empty());
            assert_eq!(model.normals.len(), model.positions.len());
            assert_eq!(model.texcoords.len(), model.positions.len());
            assert!(model.indices.iter().all(|&index| (index as usize) < model.positions.len()));
        }
        assert_eq!(theme_textures(Theme::Wood).2.dimensions(), (1024, 1024));
    }
    #[test]
    fn polyy_sets_load_with_aligned_atlases() {
        for theme in [Theme::ArtDeco, Theme::Egyptian] {
            let models = theme_models(theme);
            for model in [&models.pawn, &models.rook, &models.knight,
                &models.bishop, &models.queen, &models.king] {
                assert!(!model.indices.is_empty(), "{theme:?}");
                assert!(model.indices.iter().all(|&index| (index as usize) < model.positions.len()), "{theme:?}");
                assert!(model.texcoords.iter().all(|uv| uv[0].is_finite() && uv[1].is_finite()
                    && (0.0..=1.0).contains(&uv[0]) && (0.0..=1.0).contains(&uv[1])), "{theme:?}");
            }
            assert_eq!(theme_textures(theme).0.dimensions(), (1536, 1024));
            assert_eq!(theme_textures(theme).1.dimensions(), (1536, 1024));
        }
    }
    #[test]
    fn marble_grid_matches_piece_and_pick_coordinates() {
        let board = &models().board;
        for edge in [-4.0_f32, -3.0, 0.0, 3.0, 4.0] {
            assert!(board.positions.iter().any(|point| {
                (point.x - edge).abs() < 0.002 && (point.y - 0.055).abs() < 0.002
            }));
        }
    }
    #[test]
    fn coordinate_labels_follow_board_flip() {
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(840.0, 600.0));
        let normal = coordinate_labels(rect, false, View::default(), Theme::Marble);
        let flipped = coordinate_labels(rect, true, View::default(), Theme::Marble);
        assert_eq!(normal.len(), 16);
        assert_eq!(flipped.len(), 16);
        assert_eq!(normal[0].0, 'a');
        assert_eq!(normal[7].0, 'h');
        assert!(normal[0].1.x < normal[7].1.x);
        assert!(flipped[0].1.x > flipped[7].1.x);
        let label_area = rect.expand2(egui::Vec2::new(2.0, 12.0));
        assert!(normal.iter().all(|(_, point)| label_area.contains(*point)));
        assert!(flipped.iter().all(|(_, point)| label_area.contains(*point)));
    }
    #[test]
    fn coordinate_labels_stay_outside_squares_at_zoom_limits() {
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(840.0, 600.0));
        for theme in [Theme::Marble, Theme::Wood, Theme::Glass] {
          let board = &theme_models(theme).board;
          for flipped in [false, true] {
            for distance in [17.0, 26.0] {
                let view = View { distance, ..View::default() };
                let labels = coordinate_labels(rect, flipped, view, theme);
                let camera = Camera::new(flipped, view);
                let front_z = board.positions.iter().map(|p| p.z).fold(
                    if flipped { f32::INFINITY } else { f32::NEG_INFINITY },
                    |edge, z| if flipped { edge.min(z) } else { edge.max(z) },
                );
                let left_x = board.positions.iter().map(|p| p.x).fold(
                    if flipped { f32::NEG_INFINITY } else { f32::INFINITY },
                    |edge, x| if flipped { edge.max(x) } else { edge.min(x) },
                );
                let spacing = labels[0].1.distance(labels[1].1);
                let half_font = (spacing * 0.22).clamp(10.0, 18.0) * 0.5;
                for file in 0..8 {
                    let bottom_y = board.positions.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
                    let (edge, _) = camera.project(V3::new(file as f32 - 3.5, bottom_y, front_z), rect).unwrap();
                    assert!(
                        labels[file].1.y - edge.y > half_font
                            || (labels[file].1.y - (rect.max.y + 8.0)).abs() < 0.01,
                        "{theme:?} {flipped} {distance}",
                    );
                }
                for rank in 0..8 {
                    let (edge, _) = camera.project(V3::new(left_x, 0.07, 3.5 - rank as f32), rect).unwrap();
                    assert!(edge.x - labels[rank + 8].1.x > half_font, "{theme:?} {flipped} {distance}");
                }
            }
          }
        }
    }
    #[test]
    fn promotion_square_is_pickable_and_all_promoted_models_render() {
        let before: Board = "7k/P7/8/8/8/8/8/K7 w - - 0 1".parse().unwrap();
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(840.0, 600.0));
        let view = View::default();
        let destination = square_center(Square::A8, rect, false, view).unwrap();
        assert_eq!(square_at(destination, rect, false, view, &before, &[Square::A8]), Some(Square::A8));
        let black_before: Board = "7k/8/8/8/8/8/p7/7K b - - 0 1".parse().unwrap();
        let black_destination = square_center(Square::A1, rect, true, view).unwrap();
        assert_eq!(
            square_at(black_destination, rect, true, view, &black_before, &[Square::A1]),
            Some(Square::A1)
        );

        let mut images = Vec::new();
        for piece in [Piece::Queen, Piece::Rook, Piece::Bishop, Piece::Knight] {
            let chess_move = chess::ChessMove::new(Square::A7, Square::A8, Some(piece));
            assert!(chess::MoveGen::new_legal(&before).any(|candidate| candidate == chess_move));
            let after = before.make_move_new(chess_move);
            assert_eq!(after.piece_on(Square::A8), Some(piece));
            let rendered = image(
                &after,
                Rect::from_min_size(Pos2::ZERO, Vec2::new(320.0, 240.0)),
                false,
                view,
                None,
                &[],
                Some(chess_move),
                Color32::BLACK,
                1.0,
                0.85,
            );
            images.push(rendered.pixels);
        }
        for first in 0..images.len() {
            for second in first + 1..images.len() {
                assert!(images[first] != images[second]);
            }
        }
    }
    #[test]
    fn default_board_highlights_leave_capture_headroom() {
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::splat(600.0));
        let rendered = image(
            &Board::default(),
            rect,
            false,
            View::default(),
            None,
            &[],
            None,
            Color32::BLACK,
            1.0,
            0.5,
        );
        assert!(
            rendered
                .pixels
                .iter()
                .flat_map(|pixel| [pixel.r(), pixel.g(), pixel.b()])
                .max()
                .unwrap()
                <= 245
        );
    }
    #[test]
    fn king_is_taller_than_queen() {
        let height = |model: &Model| {
            model
                .positions
                .iter()
                .map(|point| point.y)
                .fold(0.0, f32::max)
        };
        assert!(height(&models().king) > height(&models().queen));
    }
    #[test]
    fn major_pieces_are_taller_and_broader_than_pawns() {
        let height = |model: &Model| model.positions.iter().map(|p| p.y).fold(0.0, f32::max);
        let footprint = |model: &Model| {
            let span = |axis: fn(&V3) -> f32| {
                let min = model
                    .positions
                    .iter()
                    .map(axis)
                    .fold(f32::INFINITY, f32::min);
                let max = model
                    .positions
                    .iter()
                    .map(axis)
                    .fold(f32::NEG_INFINITY, f32::max);
                max - min
            };
            span(|p| p.x).max(span(|p| p.z))
        };
        assert!(height(&models().pawn) < height(&models().rook));
        assert!(height(&models().bishop) < height(&models().queen));
        assert!(height(&models().queen) < height(&models().king));
        assert!(footprint(&models().pawn) < footprint(&models().queen));
    }
    #[test]
    fn center_pick_hits_board() {
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::splat(600.0));
        assert!(square_at(
            rect.center(),
            rect,
            false,
            View::default(),
            &Board::default(),
            &[]
        )
        .is_some());
    }
    #[test]
    fn default_camera_fits_and_lifts_marble_board() {
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::splat(600.0));
        let camera = Camera::new(false, View::default());
        let corners = [
            V3::new(-4.778, 0.055, 4.778),
            V3::new(4.778, 0.055, 4.778),
            V3::new(-4.778, 0.055, -4.778),
            V3::new(4.778, 0.055, -4.778),
        ];
        let projected = corners.map(|point| camera.project(point, rect).unwrap().0);
        let left = projected.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
        let right = projected
            .iter()
            .map(|p| p.x)
            .fold(f32::NEG_INFINITY, f32::max);
        let bottom = projected
            .iter()
            .map(|p| p.y)
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(left >= 0.0 && right <= rect.right());
        assert!(right - left > 580.0);
        assert!(bottom < rect.center().y + 200.0);
    }
    #[test]
    fn fixed_view_picks_a_piece_and_its_move_square() {
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::splat(600.0));
        let view = View::default();
        let camera = Camera::new(false, view);
        let board = Board::default();
        let e2 = Square::make_square(Rank::Second, File::E);
        let e4 = Square::make_square(Rank::Fourth, File::E);
        let pawn = camera.project(V3::new(0.5, 0.43, 2.5), rect).unwrap().0;
        let target = camera.project(V3::new(0.5, 0.075, 0.5), rect).unwrap().0;
        assert_eq!(square_at(pawn, rect, false, view, &board, &[]), Some(e2));
        assert_eq!(square_at(target, rect, false, view, &board, &[e4]), Some(e4));
    }
    #[test]
    fn wide_canvas_fits_board_and_picks_squares() {
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(840.0, 600.0));
        let view = View::default();
        let camera = Camera::new(false, view);
        let board = Board::default();
        for x in [-4.778, 4.778] {
            for z in [-4.778, 4.778] {
                let point = camera.project(V3::new(x, 0.055, z), rect).unwrap().0;
                assert!(rect.contains(point), "board corner outside canvas: {point:?}");
            }
        }
        let e4 = Square::make_square(Rank::Fourth, File::E);
        let target = camera.project(V3::new(0.5, 0.075, 0.5), rect).unwrap().0;
        assert_eq!(square_at(target, rect, false, view, &board, &[e4]), Some(e4));
    }
    #[test]
    fn arrow_anchor_tracks_square_when_board_flips_or_zooms() {
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(840.0, 600.0));
        let square = Square::make_square(Rank::Second, File::E);
        let normal = square_center(square, rect, false, View::default()).unwrap();
        let flipped = square_center(square, rect, true, View::default()).unwrap();
        assert!(normal.x > rect.center().x);
        assert!(flipped.x < rect.center().x);
        let mut zoomed = View::default();
        zoomed.distance = 14.0;
        let close = square_center(square, rect, false, zoomed).unwrap();
        assert!((close - rect.center()).length() > (normal - rect.center()).length());
    }
    #[test]
    fn cached_board_keeps_move_highlights_independent() {
        let board = Board::default();
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(210.0, 150.0));
        let render = |selected| {
            image(
                &board,
                rect,
                false,
                View::default(),
                selected,
                &[],
                None,
                Color32::BLACK,
                1.0,
                0.5,
            )
            .pixels
        };
        let plain = render(None);
        let highlighted = render(Some(Square::make_square(Rank::Fourth, File::E)));
        assert_ne!(plain, highlighted);
        assert_eq!(plain, render(None));
    }
    #[test]
    fn facing_changes_for_all_piece_types_when_they_move() {
        let b1 = Square::make_square(Rank::First, File::B);
        let c3 = Square::make_square(Rank::Third, File::C);
        for piece in [Piece::Pawn, Piece::Rook, Piece::Knight, Piece::Bishop, Piece::Queen, Piece::King] {
            for side in [Color::White, Color::Black] {
                let yaw = piece_yaw(piece, side, b1);
                assert_ne!(yaw, piece_yaw(piece, side, c3));
                assert_eq!(yaw, piece_yaw(piece, side, b1));
            }
        }
    }
    #[test]
    fn adjacent_piece_shadow_does_not_hide_move_highlight() {
        let board: Board = "4k3/8/8/8/8/8/8/1N2K3 w - - 0 1".parse().unwrap();
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::splat(600.0));
        let view = View::default();
        let render = |selected| {
            image(
                &board,
                rect,
                false,
                view,
                selected,
                &[],
                None,
                Color32::BLACK,
                1.0,
                0.5,
            )
        };
        let plain = render(None);
        let highlighted = render(Some(Square::make_square(Rank::First, File::C)));
        // This is just inside c1, where the shadow from the knight on b1
        // crosses the square boundary.
        let point = Camera::new(false, view)
            .project(V3::new(-1.96, 0.064, 3.5), rect)
            .unwrap()
            .0;
        let index = point.y as usize * plain.size[0] + point.x as usize;
        assert_ne!(plain.pixels[index], highlighted.pixels[index]);
    }
}
