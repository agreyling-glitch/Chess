//! WGPU rendering of the projected chess scene inside egui's paint pass.
use super::*;
use eframe::{egui_wgpu, wgpu};
use egui_wgpu::CallbackTrait;
use std::sync::Arc;
use wgpu::util::DeviceExt;

const SCENE_SHADER: &str = include_str!("gpu_scene.wgsl");
const VERTEX_STRIDE: usize = 104;

#[derive(Clone, Copy, PartialEq, Eq)]
struct Highlights {
    checked: Option<Square>,
    selected: Option<Square>,
    targets: u64,
    last_move: Option<chess::ChessMove>,
}

impl Highlights {
    fn new(selected: Option<Square>, targets: &[Square], last_move: Option<chess::ChessMove>) -> Self {
        Self { checked: None, selected, targets: targets.iter().fold(0, |mask, square| mask | (1_u64 << square.to_index())), last_move }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct VertexUploads {
    geometry: bool,
    highlights: bool,
    squares: u64,
}

impl VertexUploads {
    fn any(&self) -> bool { self.geometry || self.highlights || self.squares != 0 }

    fn square(&self, index: usize) -> bool { self.squares & (1_u64 << index) != 0 }

    #[cfg(test)]
    fn bytes(&self, scene: &Scene) -> usize {
        (if self.geometry { scene.vertices.len() } else { 0 })
            + (if self.highlights { scene.highlight_vertices.len() } else { 0 })
            + scene.squares.iter().enumerate().filter(|(i, _)| self.square(*i))
                .map(|(_, square)| square.vertices.len()).sum::<usize>()
    }
}

#[derive(Clone)]
struct SquareGeometry {
    occupant: Option<(Piece, Color)>,
    revision: u64,
    // Shadow followed by the piece. Draw all squares' shadows before any
    // outlines/pieces, preserving the original single-buffer draw order.
    vertices: Arc<[u8]>,
    shadow_count: u32,
    piece_count: u32,
}

impl Default for SquareGeometry {
    fn default() -> Self {
        Self { occupant: None, revision: 0, vertices: Arc::from([]), shadow_count: 0, piece_count: 0 }
    }
}

fn square_from_index(index: usize) -> Square {
    Square::make_square(Rank::from_index(index / 8), File::from_index(index % 8))
}

fn occupant(board: &Board, square: Square) -> Option<(Piece, Color)> {
    board.piece_on(square).zip(board.color_on(square))
}

#[derive(Clone)]
pub struct Scene {
    current_check: Option<Square>,
    // Static glow/board geometry is shared by all positions at this view.
    vertices: Arc<[u8]>,
    squares: Arc<[SquareGeometry; 64]>,
    highlight_vertices: Arc<[u8]>,
    highlights: Highlights,
    highlight_revision: u64,
    projection_rect: Rect,
    projection_flipped: bool,
    projection_view: View,
    id: u64,
    glow_count: u32,
    board_count: u32,
    overlay_count: u32,
    size: [u32; 2],
    background: Color32,
    target_format: wgpu::TextureFormat,
    distance_delta: f32,
    appearance: f32,
    theme: Theme,
    adjustments: PieceAppearance,
}

impl Scene {
    /// Moves, captures and history navigation change only affected squares.
    /// Piece geometry retains the base projection so a move after zoom gets
    /// the same single shader distance adjustment as every retained piece.
    pub fn update_board(&mut self, board: &Board) {
        self.current_check = checked_king(board, self.adjustments.highlight_check);
        let changed = (0..64).fold(0_u64, |mask, index| {
            if self.squares[index].occupant != occupant(board, square_from_index(index)) {
                mask | (1_u64 << index)
            } else { mask }
        });
        if changed == 0 { return; }
        let camera = Camera::new(self.projection_flipped, self.projection_view);
        let squares = Arc::make_mut(&mut self.squares);
        for (index, geometry) in squares.iter_mut().enumerate() {
            if changed & (1_u64 << index) == 0 { continue; }
            let square = square_from_index(index);
            let occupant = occupant(board, square);
            let mut triangles = Vec::new();
            if let Some((piece, side)) = occupant {
                let model = theme_models(self.theme).get(piece);
                triangles.reserve(model.indices.len() / 3 + 2);
                append_piece_triangles_with_lighting(&mut triangles, square, piece, side, camera,
                    self.projection_rect, self.theme, self.adjustments, LightingMode::Shader);
            }
            let mut vertices = Vec::with_capacity(triangles.len() * 3 * VERTEX_STRIDE);
            let shadow_count = append_triangles(&mut vertices,
                triangles.iter().filter(|t| t.shadow_uv.is_some()), self.projection_rect);
            let piece_count = append_triangles(&mut vertices,
                triangles.iter().filter(|t| t.texture.is_some()), self.projection_rect);
            *geometry = SquareGeometry { occupant, revision: geometry.revision.wrapping_add(1),
                vertices: vertices.into(), shadow_count, piece_count };
        }
    }

    #[cfg(test)]
    pub(crate) fn cached_piece(&self, square: Square) -> Option<(Piece, Color)> {
        self.squares[square.to_index()].occupant
    }

    /// Selection changes keep the board/piece mesh and its GPU buffer intact.
    /// Retain the base projection: zoom is applied to both buffers by the shader.
    pub fn update_highlights(&mut self, selected: Option<Square>, targets: &[Square], last_move: Option<chess::ChessMove>) {
        let mut highlights = Highlights::new(selected, targets, last_move);
        highlights.checked = self.current_check;
        if self.highlights == highlights { return; }
        let mut triangles = highlight_triangles(self.projection_rect, self.projection_flipped, self.projection_view,
            selected, targets, last_move, self.theme);
        triangles.extend(check_triangles(self.projection_rect, self.projection_flipped, self.projection_view, self.current_check));
        let mut vertices = Vec::with_capacity(triangles.len() * 3 * VERTEX_STRIDE);
        self.overlay_count = append_triangles(&mut vertices, &triangles, self.projection_rect);
        self.highlight_vertices = vertices.into();
        self.highlights = highlights;
        self.highlight_revision = self.highlight_revision.wrapping_add(1);
    }

    fn vertex_uploads(&self, geometry_id: Option<u64>, highlight_revision: Option<u64>, square_revisions: &[u64; 64]) -> VertexUploads {
        let geometry = geometry_id != Some(self.id);
        let squares = if geometry { u64::MAX } else {
            self.squares.iter().enumerate().fold(0, |mask, (index, square)| {
                if square.revision != square_revisions[index] { mask | (1_u64 << index) } else { mask }
            })
        };
        VertexUploads { geometry, highlights: geometry || highlight_revision != Some(self.highlight_revision), squares }
    }

    pub fn at_distance(mut self, delta: f32, appearance: f32) -> Self {
        self.distance_delta = delta;
        self.appearance = appearance;
        self
    }
}

fn push_f32(out: &mut Vec<u8>, value: f32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn append_triangles<'a>(
    out: &mut Vec<u8>,
    triangles: impl IntoIterator<Item = &'a Triangle>,
    rect: Rect,
) -> u32 {
    let mut count = 0;
    for tri in triangles {
        count += 3;
        let mode = if tri.glow_uv.is_some() {
            5.0
        } else if tri.shadow_uv.is_some() {
            4.0
        } else if let Some((_, side)) = tri.texture {
            match side {
                None => 1.0,
                Some(Color::White) => 2.0,
                Some(Color::Black) => 3.0,
            }
        } else {
            0.0
        };
        for i in 0..3 {
            let p = tri.points[i] - rect.min;
            let uv = tri
                .glow_uv.or(tri.shadow_uv)
                .map(|uv| uv[i])
                .or_else(|| tri.texture.map(|(uv, _)| uv[i]))
                .unwrap_or([0.0; 2]);
            let normal = tri
                .normal_surface
                .as_ref()
                .map(|s| s.normals[i])
                .unwrap_or(V3::new(0.0, 1.0, 0.0));
            let tangent = tri
                .normal_surface
                .as_ref()
                .map(|s| s.tangent)
                .unwrap_or(V3::new(1.0, 0.0, 0.0));
            let bitangent = tri
                .normal_surface
                .as_ref()
                .map(|s| s.bitangent)
                .unwrap_or(V3::new(0.0, 0.0, 1.0));
            let world = tri
                .normal_surface
                .as_ref()
                .map(|s| s.center)
                .unwrap_or(V3::default());
            for value in [
                p.x / rect.width(),
                p.y / rect.height(),
                tri.depths[i],
                mode,
                uv[0],
                uv[1],
                tri.color.r() as f32 / 255.0,
                tri.color.g() as f32 / 255.0,
                tri.color.b() as f32 / 255.0,
                tri.color.a() as f32 / 255.0,
                normal.x,
                normal.y,
                normal.z,
                0.0,
                tangent.x,
                tangent.y,
                tangent.z,
                0.0,
                bitangent.x,
                bitangent.y,
                bitangent.z,
                0.0,
                world.x,
                world.y,
                world.z,
                0.0,
            ] {
                push_f32(out, value);
            }
        }
    }
    count
}

pub fn scene(
    board: &Board,
    rect: Rect,
    flipped: bool,
    view: View,
    selected: Option<Square>,
    targets: &[Square],
    last_move: Option<chess::ChessMove>,
    background: Color32,
    pixels_per_point: f32,
    target_format: wgpu::TextureFormat,
    id: u64,
    theme: Theme,
    show_radial_light: bool,
    adjustments: PieceAppearance,
) -> Scene {
    let camera = Camera::new(flipped, view);
    let mut board_triangles = Vec::new();
    let model = &theme_models(theme).board;
    for face in model.indices.chunks_exact(3) {
        let points = std::array::from_fn(|i| model.positions[face[i] as usize]);
        let uv = std::array::from_fn(|i| model.texcoords[face[i] as usize]);
        let normals = std::array::from_fn(|i| model.normals[face[i] as usize]);
        push_triangle_with_lighting(
            &mut board_triangles,
            camera,
            rect,
            points,
            [230; 3],
            Some((uv, None)),
            Some(normals),
            LightingMode::Shader,
        );
    }
    let mut overlays = highlight_triangles(rect, flipped, view, selected, targets, last_move, theme);
    overlays.extend(check_triangles(rect, flipped, view, checked_king(board, adjustments.highlight_check)));
    let mut vertices = Vec::with_capacity((board_triangles.len() + 2) * 3 * VERTEX_STRIDE);
    let mut highlight_vertices = Vec::with_capacity(overlays.len() * 3 * VERTEX_STRIDE);
    let glow_count = if show_radial_light { append_triangles(&mut vertices, &board_glow(camera, rect, theme), rect) } else { 0 };
    let board_count = append_triangles(&mut vertices, &board_triangles, rect);
    let overlay_count = append_triangles(&mut highlight_vertices, &overlays, rect);
    // Render above display resolution and let the blit sampler soften edges.
    let (supersample, limit) = match theme {
        Theme::Marble => (1.5, 2048.0),
        Theme::Wood => (1.65, 2304.0),
        Theme::Glass => (2.0, 2816.0),
        Theme::ArtDeco | Theme::Egyptian => (1.5, 2048.0),
    };
    let scale = (pixels_per_point * supersample).min(limit / rect.width().max(rect.height()));
    let mut scene = Scene {
        vertices: vertices.into(),
        squares: Arc::new(std::array::from_fn(|_| SquareGeometry::default())),
        highlight_vertices: highlight_vertices.into(),
        current_check: checked_king(board, adjustments.highlight_check),
        highlights: { let mut highlights = Highlights::new(selected, targets, last_move);
            highlights.checked = checked_king(board, adjustments.highlight_check); highlights },
        highlight_revision: 0,
        projection_rect: rect,
        projection_flipped: flipped,
        projection_view: view,
        id,
        glow_count,
        board_count,
        overlay_count,
        size: [
            (rect.width() * scale).ceil().max(1.0) as u32,
            (rect.height() * scale).ceil().max(1.0) as u32,
        ],
        background,
        target_format,
        distance_delta: 0.0,
        appearance: 0.5,
        adjustments,
        theme,
    };
    scene.update_board(board);
    scene
}

struct Gpu {
    theme: Theme,
    scene_pipeline: wgpu::RenderPipeline,
    outline_pipeline: wgpu::RenderPipeline,
    translucent_pipeline: wgpu::RenderPipeline,
    blit_pipeline: wgpu::RenderPipeline,
    textures_group: wgpu::BindGroup,
    color_texture: wgpu::Texture,
    color_view: wgpu::TextureView,
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
    blit_group: wgpu::BindGroup,
    blit_layout: wgpu::BindGroupLayout,
    distance_buffer: wgpu::Buffer,
    vertex_buffer: Option<wgpu::Buffer>,
    square_buffers: [Option<wgpu::Buffer>; 64],
    highlight_buffer: Option<wgpu::Buffer>,
    rendered_scene_id: Option<u64>,
    rendered_highlight_revision: Option<u64>,
    rendered_square_revisions: [u64; 64],
    rendered_distance_delta: Option<u32>,
    rendered_appearance: Option<u32>,
    size: [u32; 2],
}

fn uploaded_texture(device: &wgpu::Device, queue: &wgpu::Queue, image: &RgbImage) -> wgpu::Texture {
    let size = wgpu::Extent3d {
        width: image.width(),
        height: image.height(),
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("chess texture"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let mut rgba = Vec::with_capacity((image.width() * image.height() * 4) as usize);
    for pixel in image.pixels() {
        rgba.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255]);
    }
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(image.width() * 4),
            rows_per_image: Some(image.height()),
        },
        size,
    );
    texture
}

fn targets(
    device: &wgpu::Device,
    size: [u32; 2],
) -> (
    wgpu::Texture,
    wgpu::TextureView,
    wgpu::Texture,
    wgpu::TextureView,
) {
    let extent = wgpu::Extent3d {
        width: size[0],
        height: size[1],
        depth_or_array_layers: 1,
    };
    let color = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("3D chess color"),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("3D chess depth"),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth24Plus,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let color_view = color.create_view(&Default::default());
    let depth_view = depth.create_view(&Default::default());
    (color, color_view, depth, depth_view)
}

impl Gpu {
    fn new(device: &wgpu::Device, queue: &wgpu::Queue, scene: &Scene) -> Self {
        device.on_uncaptured_error(std::sync::Arc::new(|error| log::error!("3D GPU error: {error:?}")));
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("3D chess shader"),
            source: wgpu::ShaderSource::Wgsl(SCENE_SHADER.into()),
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let mut texture_entries = (0..8)
            .map(|binding| wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: if binding == 7 {
                    wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering)
                } else {
                    wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    }
                },
                count: None,
            })
            .collect::<Vec<_>>();
        texture_entries.push(wgpu::BindGroupLayoutEntry {
            binding: 8,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        });
        let textures_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("3D chess textures"),
            entries: &texture_entries,
        });
        let source = theme_textures(scene.theme);
        let maps = [
            &source.0, &source.1, &source.2, &source.3, &source.4, &source.5, &source.6,
        ];
        let uploaded: Vec<_> = maps
            .iter()
            .map(|map| uploaded_texture(device, queue, map))
            .collect();
        let views: Vec<_> = uploaded
            .iter()
            .map(|texture| texture.create_view(&Default::default()))
            .collect();
        let mut entries: Vec<_> = views
            .iter()
            .enumerate()
            .map(|(binding, view)| wgpu::BindGroupEntry {
                binding: binding as u32,
                resource: wgpu::BindingResource::TextureView(view),
            })
            .collect();
        entries.push(wgpu::BindGroupEntry {
            binding: 7,
            resource: wgpu::BindingResource::Sampler(&sampler),
        });
        let distance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("3D chess camera distance"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        entries.push(wgpu::BindGroupEntry {
            binding: 8,
            resource: distance_buffer.as_entire_binding(),
        });
        let textures_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("3D chess maps"),
            layout: &textures_layout,
            entries: &entries,
        });
        let scene_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&textures_layout)],
            immediate_size: 0,
        });
        let vertex_attributes = wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x2, 2 => Float32x4, 3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4];
        let make_pipeline = |depth_write_enabled, blend: Option<wgpu::BlendState>, vertex_entry| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("3D chess scene"),
                layout: Some(&scene_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some(vertex_entry),
                    compilation_options: Default::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: VERTEX_STRIDE as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &vertex_attributes,
                    })],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("scene_fragment"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth24Plus,
                    depth_write_enabled: Some(depth_write_enabled),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            })
        };
        let outline_pipeline = make_pipeline(false, None, "outline_vertex");
        let scene_pipeline = make_pipeline(true, None, "scene_vertex");
        let translucent_pipeline = make_pipeline(false, Some(wgpu::BlendState::ALPHA_BLENDING), "scene_vertex");
        let blit_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("3D chess blit layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let blit_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&blit_layout)],
            immediate_size: 0,
        });
        let blit_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("3D chess blit"),
            layout: Some(&blit_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("blit_vertex"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                // Egui prefers an unorm surface. Only an sRGB surface needs
                // encoded scene colors converted back to linear on output.
                entry_point: Some(if scene.target_format.is_srgb() {
                    "blit_fragment_srgb"
                } else {
                    "blit_fragment"
                }),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: scene.target_format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        let (color_texture, color_view, depth_texture, depth_view) = targets(device, scene.size);
        let blit_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &blit_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&color_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        Self {
            theme: scene.theme,
            scene_pipeline,
            outline_pipeline,
            translucent_pipeline,
            blit_pipeline,
            textures_group,
            color_texture,
            color_view,
            depth_texture,
            depth_view,
            blit_group,
            blit_layout,
            distance_buffer,
            vertex_buffer: None,
            square_buffers: std::array::from_fn(|_| None),
            highlight_buffer: None,
            rendered_scene_id: None,
            rendered_highlight_revision: None,
            rendered_square_revisions: [0; 64],
            rendered_distance_delta: None,
            rendered_appearance: None,
            size: scene.size,
        }
    }
    fn resize(&mut self, device: &wgpu::Device, size: [u32; 2]) {
        if self.size == size {
            return;
        }
        (
            self.color_texture,
            self.color_view,
            self.depth_texture,
            self.depth_view,
        ) = targets(device, size);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        self.blit_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.blit_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.color_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        self.size = size;
        self.rendered_scene_id = None;
        self.rendered_highlight_revision = None;
        self.rendered_distance_delta = None;
        self.rendered_appearance = None;
    }
}

impl CallbackTrait for Scene {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen: &egui_wgpu::ScreenDescriptor,
        encoder: &mut wgpu::CommandEncoder,
        resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        if resources.get::<Gpu>().is_none_or(|gpu| gpu.theme != self.theme) {
            resources.insert(Gpu::new(device, queue, self));
        }
        let gpu = resources.get_mut::<Gpu>().unwrap();
        gpu.resize(device, self.size);
        let uploads = self.vertex_uploads(gpu.rendered_scene_id, gpu.rendered_highlight_revision, &gpu.rendered_square_revisions);
        let delta_bits = self.distance_delta.to_bits();
        let appearance_bits = self.appearance.to_bits();
        if !uploads.any() && gpu.rendered_distance_delta == Some(delta_bits)
            && gpu.rendered_appearance == Some(appearance_bits) {
            return Vec::new();
        }
        let mut adjustment = [0_u8; 64];
        adjustment[..4].copy_from_slice(&self.distance_delta.to_le_bytes());
        adjustment[4..8].copy_from_slice(&self.appearance.to_le_bytes());
        let theme_code = match self.theme {
            Theme::Marble => 0.0_f32, Theme::Wood => 1.0, Theme::Glass => 2.0,
            Theme::ArtDeco => 3.0,
            Theme::Egyptian => 4.0,
        };
        adjustment[8..12].copy_from_slice(&theme_code.to_le_bytes());
        adjustment[12..16].copy_from_slice(&(2.0 * SCREEN_CENTER_LIFT).to_le_bytes());
        let a = self.adjustments;
        let extra = [a.white[0] as f32/255.0,a.white[1] as f32/255.0,a.white[2] as f32/255.0,if a.custom_colors { 1.0 } else { 0.0 },
            a.black[0] as f32/255.0,a.black[1] as f32/255.0,a.black[2] as f32/255.0,if a.outline { 1.0 } else { 0.0 },
            a.finish as f32,a.shadow_strength as f32/100.0,a.shadow_softness as f32/100.0,2.5/self.size[0].max(self.size[1]) as f32];
        for (i,value) in extra.iter().enumerate() { adjustment[16+i*4..20+i*4].copy_from_slice(&value.to_le_bytes()); }
        queue.write_buffer(&gpu.distance_buffer, 0, &adjustment);
        if uploads.geometry {
            gpu.vertex_buffer = Some(device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("3D chess board"),
                    contents: &self.vertices,
                    usage: wgpu::BufferUsages::VERTEX,
                },
            ));
        }
        for (index, square) in self.squares.iter().enumerate() {
            if uploads.square(index) {
                // Empty squares must release a captured/moved piece and still
                // trigger a redraw, without creating a zero-size GPU buffer.
                gpu.square_buffers[index] = (!square.vertices.is_empty()).then(|| device.create_buffer_init(
                    &wgpu::util::BufferInitDescriptor {
                        label: Some("3D chess square vertices"),
                        contents: &square.vertices,
                        usage: wgpu::BufferUsages::VERTEX,
                    },
                ));
            }
        }
        if uploads.highlights {
            gpu.highlight_buffer = (!self.highlight_vertices.is_empty()).then(|| device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("3D chess highlights"),
                    contents: &self.highlight_vertices,
                    usage: wgpu::BufferUsages::VERTEX,
                },
            ));
        }
        let c = self.background;
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("3D chess scene"),
            multiview_mask: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &gpu.color_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: c.r() as f64 / 255.0,
                        g: c.g() as f64 / 255.0,
                        b: c.b() as f64 / 255.0,
                        a: c.a() as f64 / 255.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &gpu.depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_bind_group(0, &gpu.textures_group, &[]);
        pass.set_vertex_buffer(0, gpu.vertex_buffer.as_ref().unwrap().slice(..));
        pass.set_pipeline(&gpu.translucent_pipeline);
        pass.draw(0..self.glow_count, 0..1);
        pass.set_pipeline(&gpu.scene_pipeline);
        pass.draw(self.glow_count..self.glow_count + self.board_count, 0..1);
        pass.set_pipeline(&gpu.translucent_pipeline);
        if let Some(highlights) = &gpu.highlight_buffer {
            pass.set_vertex_buffer(0, highlights.slice(..));
            pass.draw(0..self.overlay_count, 0..1);
        }
        for (index, square) in self.squares.iter().enumerate() {
            if square.shadow_count != 0 {
                pass.set_vertex_buffer(0, gpu.square_buffers[index].as_ref().unwrap().slice(..));
                pass.draw(0..square.shadow_count, 0..1);
            }
        }
        if self.adjustments.outline {
            pass.set_pipeline(&gpu.outline_pipeline);
            // Preserve the old instance-major ordering: every piece in one
            // outline direction before drawing the next direction. Outlines
            // do not write depth, so interleaving by piece can change pixels.
            for instance in 0..8 {
                for (index, square) in self.squares.iter().enumerate() {
                    if square.piece_count != 0 {
                        pass.set_vertex_buffer(0, gpu.square_buffers[index].as_ref().unwrap().slice(..));
                        pass.draw(square.shadow_count..square.shadow_count + square.piece_count, instance..instance + 1);
                    }
                }
            }
        }
        pass.set_pipeline(&gpu.scene_pipeline);
        for (index, square) in self.squares.iter().enumerate() {
            if square.piece_count != 0 {
                pass.set_vertex_buffer(0, gpu.square_buffers[index].as_ref().unwrap().slice(..));
                pass.draw(square.shadow_count..square.shadow_count + square.piece_count, 0..1);
            }
        }
        drop(pass);
        gpu.rendered_scene_id = Some(self.id);
        gpu.rendered_highlight_revision = Some(self.highlight_revision);
        gpu.rendered_square_revisions = std::array::from_fn(|index| self.squares[index].revision);
        gpu.rendered_distance_delta = Some(delta_bits);
        gpu.rendered_appearance = Some(appearance_bits);
        Vec::new()
    }
    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        pass: &mut wgpu::RenderPass<'static>,
        resources: &egui_wgpu::CallbackResources,
    ) {
        if let Some(gpu) = resources.get::<Gpu>() {
            pass.set_pipeline(&gpu.blit_pipeline);
            pass.set_bind_group(0, &gpu.blit_group, &[]);
            pass.draw(0..6, 0..1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn test_scene(theme: Theme, flipped: bool, view: View, selected: Option<Square>, targets: &[Square], last_move: Option<chess::ChessMove>) -> Scene {
        scene(&Board::default(), Rect::from_min_size(Pos2::new(37.0, 29.0), Vec2::new(840.0, 600.0)),
            flipped, view, selected, targets, last_move, Color32::BLACK, 1.0,
            wgpu::TextureFormat::Rgba8Unorm, 7, theme, true, PieceAppearance::default())
    }

    #[test]
    fn shader_lighting_skips_cpu_values_without_changing_uploaded_vertices() {
        let rect = Rect::from_min_size(Pos2::new(17.0, 23.0), Vec2::new(640.0, 480.0));
        let camera = Camera::new(true, View { yaw: 0.31, elevation: 0.72, distance: 19.0 });
        let points = [V3::new(-1.0, 0.1, -0.5), V3::new(1.0, 0.3, -0.5), V3::new(0.0, 1.2, 0.5)];
        let normals = [V3::new(0.1, 1.0, 0.2).unit(), V3::new(0.2, 0.9, 0.1).unit(), V3::new(-0.1, 1.0, 0.0).unit()];
        let uv = [[0.0, 0.0], [1.0, 0.0], [0.5, 1.0]];
        let base = [213, 177, 89];
        for texture in [None, Some((uv, None)), Some((uv, Some(Color::White))),
            Some((uv, Some(Color::Black))), Some(([[0.0; 2]; 3], Some(Color::White)))] {
            for vertex_normals in [None, Some(normals)] {
                let mut cpu = Vec::new();
                let mut shader = Vec::new();
                push_triangle(&mut cpu, camera, rect, points, base, texture, vertex_normals);
                push_triangle_with_lighting(&mut shader, camera, rect, points, base, texture, vertex_normals, LightingMode::Shader);
                assert_eq!((cpu.len(), shader.len()), (1, 1));
                let expected_lighting = vertex_normals.map(|normals| std::array::from_fn(|i|
                    studio_lighting(normals[i], points[i], camera, texture.and_then(|(_, side)| side))));
                assert_eq!(cpu[0].vertex_lighting, expected_lighting,
                    "previews and PNG export must retain the original CPU lighting");
                assert!(shader[0].vertex_lighting.is_none());
                if vertex_normals.is_some() {
                    assert_eq!(shader[0].color, Color32::from_rgb(base[0], base[1], base[2]),
                        "normal-bearing surfaces must not acquire flat shading when CPU lighting is skipped");
                }
                let mut cpu_bytes = Vec::new();
                let mut shader_bytes = Vec::new();
                append_triangles(&mut cpu_bytes, &cpu, rect);
                append_triangles(&mut shader_bytes, &shader, rect);
                assert_eq!(cpu_bytes, shader_bytes,
                    "positions, color, normals, tangent basis and texture coordinates must be unchanged");
            }
        }
    }

    // Frozen pre-split dynamic scene builder. Keep this independent of the new
    // highlight/piece helpers so moving work between them cannot change pixels.
    fn original_dynamic_triangles(board: &Board, rect: Rect, flipped: bool, view: View,
        selected: Option<Square>, targets: &[Square], last_move: Option<chess::ChessMove>,
        theme: Theme, adjustments: PieceAppearance) -> Vec<Triangle> {
        let camera = Camera::new(flipped, view);
        let mut triangles = Vec::with_capacity(40000);
        let mut piece_triangles = Vec::with_capacity(40000);
        for rank in 0..8 {
            for file in 0..8 {
                let square = Square::make_square(Rank::from_index(rank), File::from_index(file));
                let x = file as f32 - 4.0;
                let z = 3.0 - rank as f32;
                let mut color = None;
                if last_move.is_some_and(|m| m.get_source() == square || m.get_dest() == square) { color = Some([180, 165, 78]); }
                if targets.contains(&square) { color = Some([125, 190, 123]); }
                if selected == Some(square) { color = Some([230, 178, 65]); }
                if let Some(color) = color {
                    let (highlight_x, highlight_z) = (x, z);
                    overlay_quad(&mut triangles, camera, rect, [
                        V3::new(highlight_x, 0.061, highlight_z),
                        V3::new(highlight_x + 1.0, 0.061, highlight_z),
                        V3::new(highlight_x + 1.0, 0.061, highlight_z + 1.0),
                        V3::new(highlight_x, 0.061, highlight_z + 1.0),
                    ], color);
                }
                if let (Some(piece), Some(side)) = (board.piece_on(square), board.color_on(square)) {
                    let model = theme_models(theme).get(piece);
                    let center = V3::new(file as f32 - 3.5, 0.075, 3.5 - rank as f32);
                    let (sin, cos) = piece_yaw(piece, side, square).sin_cos();
                    let shadow_center = if theme == Theme::Wood { center.add(V3::new(-0.055, 0.0, 0.06)) } else { center };
                    if adjustments.shadows && adjustments.shadow_strength > 0 {
                        contact_shadow(&mut piece_triangles, camera, rect, shadow_center,
                            (if piece == Piece::Pawn { 0.45 } else { 0.54 }) * adjustments.size as f32 / 100.0 * (0.75 + adjustments.shadow_softness as f32 / 200.0));
                    }
                    let color = if side == Color::White { [235, 232, 225] } else { [62, 73, 68] };
                    for face in model.indices.chunks_exact(3) {
                        let mut pts = [V3::default(); 3];
                        for (n, &i) in face.iter().enumerate() {
                            let p = turn_y(model.positions[i as usize], sin, cos);
                            pts[n] = center.add(V3::new(p.x * adjustments.size as f32 / 100.0,
                                p.y * adjustments.size as f32 / 100.0 * adjustments.height as f32 / 100.0,
                                p.z * adjustments.size as f32 / 100.0));
                        }
                        let uv = [model.texcoords[face[0] as usize], model.texcoords[face[1] as usize], model.texcoords[face[2] as usize]];
                        let normals = std::array::from_fn(|i| {
                            let n = turn_y(model.normals[face[i] as usize], sin, cos);
                            V3::new(n.x, n.y * 100.0 / adjustments.height as f32, n.z).unit()
                        });
                        push_triangle(&mut piece_triangles, camera, rect, pts, color, Some((uv, Some(side))), Some(normals));
                    }
                }
            }
        }
        triangles.extend(piece_triangles);
        triangles
    }

    // Reference the original single-buffer draw sequence using the frozen
    // builder above, not either refactored geometry/highlight helper.
    fn original_vertex_stream(board: &Board, rect: Rect, flipped: bool, view: View,
        selected: Option<Square>, targets: &[Square], last_move: Option<chess::ChessMove>,
        theme: Theme, radial_light: bool, adjustments: PieceAppearance) -> (Vec<u8>, [u32; 5]) {
        let camera = Camera::new(flipped, view);
        let model = &theme_models(theme).board;
        let mut board_triangles = Vec::new();
        for face in model.indices.chunks_exact(3) {
            push_triangle(&mut board_triangles, camera, rect,
                std::array::from_fn(|i| model.positions[face[i] as usize]), [230; 3],
                Some((std::array::from_fn(|i| model.texcoords[face[i] as usize]), None)),
                Some(std::array::from_fn(|i| model.normals[face[i] as usize])));
        }
        let dynamic = original_dynamic_triangles(board, rect, flipped, view, selected, targets, last_move, theme, adjustments);
        let mut bytes = Vec::new();
        let glow = if radial_light { append_triangles(&mut bytes, &board_glow(camera, rect, theme), rect) } else { 0 };
        let board = append_triangles(&mut bytes, &board_triangles, rect);
        let overlays = append_triangles(&mut bytes, dynamic.iter().filter(|t| t.texture.is_none() && t.shadow_uv.is_none()), rect);
        let shadows = append_triangles(&mut bytes, dynamic.iter().filter(|t| t.shadow_uv.is_some()), rect);
        let pieces = append_triangles(&mut bytes, dynamic.iter().filter(|t| t.texture.is_some()), rect);
        (bytes, [glow, board, overlays, shadows, pieces])
    }

    fn split_vertex_stream(scene: &Scene) -> Vec<u8> {
        let mut bytes = scene.vertices.to_vec();
        bytes.extend_from_slice(&scene.highlight_vertices);
        for square in scene.squares.iter() {
            bytes.extend_from_slice(&square.vertices[..square.shadow_count as usize * VERTEX_STRIDE]);
        }
        for square in scene.squares.iter() {
            bytes.extend_from_slice(&square.vertices[square.shadow_count as usize * VERTEX_STRIDE..]);
        }
        bytes
    }

    fn square_revisions(scene: &Scene) -> [u64; 64] {
        std::array::from_fn(|index| scene.squares[index].revision)
    }

    fn all_uploads(scene: &Scene) -> VertexUploads {
        scene.vertex_uploads(None, None, &[0; 64])
    }

    fn move_cases() -> Vec<(&'static str, Board, Board, Vec<Square>)> {
        use chess::ChessMove;
        let start = Board::default();
        let e4 = start.make_move_new(ChessMove::new(Square::E2, Square::E4, None));
        let d5 = e4.make_move_new(ChessMove::new(Square::D7, Square::D5, None));
        let castles = Board::from_str("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").unwrap();
        let en_passant = Board::from_str("7k/8/8/3pP3/8/8/8/K7 w - d6 0 1").unwrap();
        let promotion = Board::from_str("1r5k/P7/8/8/8/8/8/K7 w - - 0 1").unwrap();
        let chess960 = crate::rules::Board::from_str("4k3/8/8/8/8/8/8/5KR1 w G - 0 1").unwrap();
        let chess960_castled = chess960.make_move_new(ChessMove::new(Square::F1, Square::G1, None));
        let imported = Board::from_str("7k/8/8/8/8/8/8/K7 w - - 0 1").unwrap();
        let edited = Board::from_str("7k/8/2b5/8/3Q4/8/8/K7 b - - 0 1").unwrap();
        vec![
            ("move", start, e4, vec![Square::E2, Square::E4]),
            ("capture", d5, d5.make_move_new(ChessMove::new(Square::E4, Square::D5, None)), vec![Square::E4, Square::D5]),
            ("castle", castles, castles.make_move_new(ChessMove::new(Square::E1, Square::G1, None)), vec![Square::E1, Square::F1, Square::G1, Square::H1]),
            ("en passant", en_passant, en_passant.make_move_new(ChessMove::new(Square::E5, Square::D6, None)), vec![Square::D5, Square::E5, Square::D6]),
            ("promotion capture", promotion, promotion.make_move_new(ChessMove::new(Square::A7, Square::B8, Some(Piece::Knight))), vec![Square::A7, Square::B8]),
            ("Chess960 occupied-square swap", *chess960, *chess960_castled, vec![Square::F1, Square::G1]),
            ("undo", e4, start, vec![Square::E2, Square::E4]),
            ("imported FEN", imported, edited, vec![Square::C6, Square::D4]),
        ]
    }

    #[test]
    fn board_updates_reuse_unaffected_squares_and_match_original_geometry_for_every_theme() {
        for (theme, _) in Theme::ALL {
            for (label, before, after, changed) in move_cases() {
                let flipped = matches!(theme, Theme::Wood | Theme::Egyptian);
                let view = View { yaw: 0.41, elevation: 0.71, distance: 19.0 };
                let rect = Rect::from_min_size(Pos2::new(37.0, 29.0), Vec2::new(840.0, 600.0));
                let adjustments = PieceAppearance { size: 93, height: 117, outline: true, ..Default::default() };
                let mut rendered = scene(&before, rect, flipped, view, None, &[], None, Color32::BLACK, 1.0,
                    wgpu::TextureFormat::Rgba8Unorm, 7, theme, true, adjustments);
                // An existing paint callback can retain the prior snapshot
                // while update_board uses copy-on-write for changed squares.
                let previous = rendered.clone();
                let revisions = square_revisions(&previous);
                rendered.update_board(&after);
                assert_eq!(rendered.id, previous.id);
                assert!(Arc::ptr_eq(&rendered.vertices, &previous.vertices));
                assert!(Arc::ptr_eq(&rendered.highlight_vertices, &previous.highlight_vertices));
                let expected_mask = changed.iter().fold(0, |mask, square| mask | (1_u64 << square.to_index()));
                let uploads = rendered.vertex_uploads(Some(previous.id), Some(previous.highlight_revision), &revisions);
                assert_eq!(uploads, VertexUploads { geometry: false, highlights: false, squares: expected_mask }, "{theme:?}: {label}");
                assert!(uploads.any());
                for index in 0..64 {
                    let square = square_from_index(index);
                    let old = &previous.squares[index];
                    let new = &rendered.squares[index];
                    assert_eq!(old.occupant, occupant(&before, square), "retained snapshot changed");
                    assert_eq!(new.occupant, occupant(&after, square));
                    if uploads.square(index) {
                        assert_eq!(new.revision, old.revision + 1);
                        if new.occupant.is_none() {
                            assert!(new.vertices.is_empty(), "removed piece/shadow must release its buffer");
                            assert_eq!((new.shadow_count, new.piece_count), (0, 0));
                        }
                    } else {
                        assert_eq!(new.revision, old.revision);
                        assert!(Arc::ptr_eq(&new.vertices, &old.vertices), "{theme:?}: {label} rebuilt unaffected {square}");
                    }
                }
                let (expected, _) = original_vertex_stream(&after, rect, flipped, view, None, &[], None, theme, true, adjustments);
                assert!(split_vertex_stream(&rendered) == expected, "{theme:?}: {label} changed the original rendering stream");
                assert!(uploads.bytes(&rendered) < all_uploads(&rendered).bytes(&rendered));
            }
        }
    }

    #[test]
    fn board_metadata_is_a_noop_but_new_resources_refresh_every_square() {
        let mut rendered = test_scene(Theme::Wood, false, View::default(), None, &[], None);
        let original = rendered.clone();
        let revisions = square_revisions(&original);
        let without_rights = Board::from_str("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b - - 0 1").unwrap();
        for board in [Board::default(), Board::default().null_move().unwrap(), without_rights] {
            rendered.update_board(&board);
            assert!(Arc::ptr_eq(&rendered.squares, &original.squares), "unchanged visible pieces must allocate nothing");
            let uploads = rendered.vertex_uploads(Some(rendered.id), Some(rendered.highlight_revision), &revisions);
            assert!(!uploads.any());
            assert_eq!(uploads.bytes(&rendered), 0);
        }
        for scene_id in [None, Some(rendered.id + 1)] {
            let uploads = rendered.vertex_uploads(scene_id, Some(rendered.highlight_revision), &revisions);
            assert_eq!(uploads, VertexUploads { geometry: true, highlights: true, squares: u64::MAX });
            assert_eq!(uploads.bytes(&rendered), split_vertex_stream(&rendered).len());
        }
        let e4 = Board::default().make_move_new(chess::ChessMove::new(Square::E2, Square::E4, None));
        rendered.update_board(&e4);
        let uploaded = square_revisions(&rendered);
        assert!(!rendered.vertex_uploads(Some(rendered.id), Some(rendered.highlight_revision), &uploaded).any());
    }

    #[test]
    fn removing_the_last_extra_piece_redraws_without_uploading_empty_buffers() {
        let before = Board::from_str("7k/8/8/8/8/8/P7/K7 w - - 0 1").unwrap();
        let after = Board::from_str("7k/8/8/8/8/8/8/K7 w - - 0 1").unwrap();
        let mut rendered = test_scene(Theme::Marble, false, View::default(), None, &[], None);
        rendered.update_board(&before);
        let revisions = square_revisions(&rendered);
        rendered.update_board(&after);
        let uploads = rendered.vertex_uploads(Some(rendered.id), Some(rendered.highlight_revision), &revisions);
        assert_eq!(uploads, VertexUploads { geometry: false, highlights: false, squares: 1_u64 << Square::A2.to_index() });
        assert!(uploads.any(), "clearing geometry must invalidate the offscreen color target");
        assert_eq!(uploads.bytes(&rendered), 0);
        assert!(rendered.squares[Square::A2.to_index()].vertices.is_empty());
    }

    #[test]
    fn split_buffers_preserve_original_vertex_bytes_and_draw_order_for_every_theme() {
        let rect = Rect::from_min_size(Pos2::new(37.0, 29.0), Vec2::new(840.0, 600.0));
        let last_move = Some(chess::ChessMove::new(Square::E2, Square::E4, None));
        for (theme, _) in Theme::ALL {
            for flipped in [false, true] {
                let view = if flipped { View { yaw: 0.47, elevation: 0.65, distance: 22.0 } } else { View::default() };
                let board = if flipped { Board::default().make_move_new(last_move.unwrap()) } else { Board::default() };
                let adjustments = PieceAppearance { size: 93, height: 117, outline: true, shadows: !flipped, ..Default::default() };
                let selected = Some(Square::E2);
                let targets = &[Square::E2, Square::E3, Square::E4];
                let rendered = scene(&board, rect, flipped, view, selected, targets, last_move, Color32::BLACK, 1.0,
                    wgpu::TextureFormat::Rgba8Unorm, 3, theme, !flipped, adjustments);
                let (expected, counts) = original_vertex_stream(&board, rect, flipped, view, selected, targets, last_move, theme, !flipped, adjustments);
                assert_eq!([rendered.glow_count, rendered.board_count, rendered.overlay_count,
                    rendered.squares.iter().map(|s| s.shadow_count).sum(), rendered.squares.iter().map(|s| s.piece_count).sum()], counts);
                assert!(split_vertex_stream(&rendered) == expected, "changed vertex bytes or draw order for {theme:?}, flipped={flipped}");
                assert_eq!(rendered.highlight_vertices.len(), rendered.overlay_count as usize * VERTEX_STRIDE);
                assert_eq!(rendered.vertices.len(), (rendered.glow_count + rendered.board_count) as usize * VERTEX_STRIDE);
                for square in rendered.squares.iter() {
                    assert_eq!(square.vertices.len(), (square.shadow_count + square.piece_count) as usize * VERTEX_STRIDE);
                }
            }
        }
    }

    #[test]
    fn highlight_updates_reuse_geometry_refresh_equal_counts_and_clear_empty_buffers() {
        for (theme, _) in Theme::ALL {
            let mut rendered = test_scene(theme, false, View::default(), Some(Square::E2), &[Square::E3, Square::E4], None);
            let geometry = rendered.vertices.clone();
            let squares = rendered.squares.clone();
            let square_revisions = square_revisions(&rendered);
            let highlights = rendered.highlight_vertices.clone();
            let original_count = rendered.overlay_count;
            let revision = rendered.highlight_revision;
            // Targets represent a set; reordered or duplicated inputs allocate
            // no new byte storage and do not request a GPU upload.
            rendered.update_highlights(Some(Square::E2), &[Square::E4, Square::E3, Square::E3], None);
            assert_eq!(rendered.highlight_revision, revision);
            assert!(Arc::ptr_eq(&rendered.highlight_vertices, &highlights));
            assert_eq!(rendered.vertex_uploads(Some(rendered.id), Some(revision), &square_revisions), VertexUploads { geometry: false, highlights: false, squares: 0 });

            rendered.update_highlights(Some(Square::D2), &[Square::D3, Square::D4], None);
            assert!(Arc::ptr_eq(&rendered.vertices, &geometry));
            assert!(Arc::ptr_eq(&rendered.squares, &squares));
            assert_eq!(rendered.overlay_count, original_count);
            assert_ne!(rendered.highlight_revision, revision);
            assert!(rendered.highlight_vertices.as_ref() != highlights.as_ref());
            assert_eq!(rendered.vertex_uploads(Some(rendered.id), Some(revision), &square_revisions), VertexUploads { geometry: false, highlights: true, squares: 0 });
            let mut expected = Vec::new();
            append_triangles(&mut expected, &highlight_triangles(rendered.projection_rect, false, View::default(), Some(Square::D2), &[Square::D3, Square::D4], None, theme), rendered.projection_rect);
            assert!(rendered.highlight_vertices.as_ref() == expected);

            let revision = rendered.highlight_revision;
            rendered.update_highlights(None, &[], None);
            assert!(Arc::ptr_eq(&rendered.vertices, &geometry));
            assert!(rendered.highlight_vertices.is_empty());
            assert_eq!(rendered.overlay_count, 0);
            assert_eq!(rendered.vertex_uploads(Some(rendered.id), Some(revision), &square_revisions), VertexUploads { geometry: false, highlights: true, squares: 0 }, "clearing must redraw the board");
            assert_eq!(rendered.vertex_uploads(None, Some(rendered.highlight_revision), &square_revisions), VertexUploads { geometry: true, highlights: true, squares: u64::MAX }, "new geometry/resources must refresh all buffers");
            let revision = rendered.highlight_revision;
            rendered.update_highlights(None, &[], None);
            assert_eq!(rendered.highlight_revision, revision);
        }
    }

    #[test]
    fn highlight_precedence_matches_board_squares_without_theme_offsets() {
        let rect = Rect::from_min_size(Pos2::new(37.0, 29.0), Vec2::new(840.0, 600.0));
        for (theme, _) in Theme::ALL {
            for flipped in [false, true] {
                let camera = Camera::new(flipped, View::default());
                let triangles = highlight_triangles(rect, flipped, View::default(), Some(Square::E2), &[Square::E2, Square::E3],
                    Some(chess::ChessMove::new(Square::E2, Square::E4, None)), theme);
                assert_eq!(triangles.len(), 6);
                for (index, (square, color)) in [(Square::E2, [230, 178, 65]), (Square::E3, [125, 190, 123]), (Square::E4, [180, 165, 78])].into_iter().enumerate() {
                    let color = Color32::from_rgba_unmultiplied(color[0], color[1], color[2], 135);
                    assert_eq!(triangles[index * 2].color, color);
                    assert_eq!(triangles[index * 2 + 1].color, color);
                    let x = square.get_file().to_index() as f32 - 4.0;
                    let z = 3.0 - square.get_rank().to_index() as f32;
                    let expected = camera.project(V3::new(x, 0.061, z), rect).unwrap().0;
                    assert!(triangles[index * 2].points[0].distance(expected) < 0.001);
                }
            }
        }
    }

    #[test]
    fn checked_king_highlight_updates_and_clears_with_board_and_toggle() {
        let board = Board::from_str("k3r3/8/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        assert_eq!(checked_king(&board, true), Some(Square::E1));
        assert_eq!(checked_king(&board, false), None);
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(700.0, 500.0));
        let mut rendered = scene(&board, rect, false, View::default(), None, &[], None,
            Color32::BLACK, 1.0, wgpu::TextureFormat::Rgba8Unorm, 1, Theme::Wood, false,
            PieceAppearance { highlight_check: true, ..Default::default() });
        assert_eq!(rendered.highlights.checked, Some(Square::E1));
        assert_eq!(rendered.overlay_count, 6);
        let base = rendered.vertices.clone();
        let revision = rendered.highlight_revision;
        let escaped = board.make_move_new(chess::ChessMove::new(Square::E1, Square::D1, None));
        rendered.update_board(&escaped);
        rendered.update_highlights(None, &[], None);
        assert!(rendered.highlights.checked.is_none());
        assert_eq!(rendered.overlay_count, 0);
        assert!(rendered.highlight_revision > revision);
        assert!(Arc::ptr_eq(&base, &rendered.vertices));
        rendered.update_board(&board);
        rendered.update_highlights(None, &[], None);
        assert_eq!(rendered.overlay_count, 6);
        let disabled = scene(&board, rect, false, View::default(), None, &[], None,
            Color32::BLACK, 1.0, wgpu::TextureFormat::Rgba8Unorm, 2, Theme::Wood, false, PieceAppearance::default());
        assert_eq!(disabled.overlay_count, 0);
    }

    fn vertex_float(bytes: &[u8], vertex: usize, component: usize) -> f32 {
        let at = vertex * VERTEX_STRIDE + component * 4;
        f32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
    }

    #[test]
    fn changed_highlights_after_zoom_use_the_original_projection() {
        for (theme, _) in Theme::ALL {
            let flipped = theme == Theme::Wood;
            let view = View { yaw: 0.37, elevation: 0.72, distance: 17.0 };
            let mut rendered = test_scene(theme, flipped, view, None, &[], None).at_distance(7.0, 0.85);
            let geometry = rendered.vertices.clone();
            rendered.update_highlights(Some(Square::D2), &[Square::D3, Square::D4], None);
            let mut expected = Vec::new();
            let target_view = View { distance: 24.0, ..view };
            append_triangles(&mut expected, &highlight_triangles(rendered.projection_rect, flipped, target_view,
                Some(Square::D2), &[Square::D3, Square::D4], None, theme), rendered.projection_rect);
            assert!(Arc::ptr_eq(&rendered.vertices, &geometry));
            assert_eq!(rendered.highlight_vertices.len(), expected.len());
            for vertex in 0..rendered.overlay_count as usize {
                let x = vertex_float(&rendered.highlight_vertices, vertex, 0);
                let y = vertex_float(&rendered.highlight_vertices, vertex, 1);
                let depth = vertex_float(&rendered.highlight_vertices, vertex, 2);
                let ndc = Vec2::new(x * 2.0 - 1.0, 1.0 - y * 2.0);
                let pivot = Vec2::new(0.0, 2.0 * SCREEN_CENTER_LIFT);
                let zoomed = ((ndc - pivot) * depth + pivot * (depth + rendered.distance_delta)) / (depth + rendered.distance_delta);
                let expected = Vec2::new(vertex_float(&expected, vertex, 0) * 2.0 - 1.0, 1.0 - vertex_float(&expected, vertex, 1) * 2.0);
                assert!((zoomed - expected).length() < 0.00001, "highlights must receive zoom exactly once for {theme:?}");
            }
        }
    }

    #[test]
    fn changed_pieces_after_zoom_share_the_original_projection() {
        let moved = Board::default().make_move_new(chess::ChessMove::new(Square::E2, Square::E4, None));
        for (theme, _) in Theme::ALL {
            let flipped = theme == Theme::Wood;
            let view = View { yaw: 0.37, elevation: 0.72, distance: 17.0 };
            let mut rendered = test_scene(theme, flipped, view, None, &[], None).at_distance(7.0, 0.85);
            let previous = rendered.clone();
            rendered.update_board(&moved);
            assert!(Arc::ptr_eq(&rendered.vertices, &previous.vertices));
            assert!(Arc::ptr_eq(&rendered.squares[Square::A1.to_index()].vertices, &previous.squares[Square::A1.to_index()].vertices));
            let target_view = View { distance: 24.0, ..view };
            let expected = scene(&moved, rendered.projection_rect, flipped, target_view, None, &[], None,
                Color32::BLACK, 1.0, wgpu::TextureFormat::Rgba8Unorm, 8, theme, true, PieceAppearance::default());
            for square in [Square::E4, Square::A1] {
                let actual = &rendered.squares[square.to_index()].vertices;
                let expected = &expected.squares[square.to_index()].vertices;
                assert_eq!(actual.len(), expected.len());
                for vertex in 0..actual.len() / VERTEX_STRIDE {
                    let x = vertex_float(actual, vertex, 0);
                    let y = vertex_float(actual, vertex, 1);
                    let depth = vertex_float(actual, vertex, 2);
                    let ndc = Vec2::new(x * 2.0 - 1.0, 1.0 - y * 2.0);
                    let pivot = Vec2::new(0.0, 2.0 * SCREEN_CENTER_LIFT);
                    let zoomed = ((ndc - pivot) * depth + pivot * (depth + rendered.distance_delta)) / (depth + rendered.distance_delta);
                    let target = Vec2::new(vertex_float(expected, vertex, 0) * 2.0 - 1.0, 1.0 - vertex_float(expected, vertex, 1) * 2.0);
                    assert!((zoomed - target).length() < 0.00001, "{theme:?}: {square} must receive zoom exactly once");
                    assert!((depth + rendered.distance_delta - vertex_float(expected, vertex, 2)).abs() < 0.00001);
                }
            }
        }
    }

    #[test]
    #[ignore = "Run with --release --ignored --nocapture to compare warm CPU scene construction and planned vertex uploads"]
    fn benchmark_full_scene_rebuild_versus_highlight_updates() {
        use std::{hint::black_box, time::Instant};
        const SAMPLES: usize = 31;
        for (theme, _) in Theme::ALL {
            let mut cached = test_scene(theme, false, View::default(), Some(Square::E2), &[Square::E3, Square::E4], None);
            let mut full_times = Vec::with_capacity(SAMPLES);
            let mut update_times = Vec::with_capacity(SAMPLES);
            let mut full_bytes = 0;
            let mut update_bytes = 0;
            for sample in 0..SAMPLES {
                let (selected, targets) = if sample % 2 == 0 { (Square::D2, [Square::D3, Square::D4]) } else { (Square::E2, [Square::E3, Square::E4]) };
                let start = Instant::now();
                let rebuilt = test_scene(theme, false, View::default(), Some(selected), &targets, None);
                black_box(&rebuilt);
                full_times.push(start.elapsed().as_secs_f64() * 1_000_000.0);
                full_bytes = all_uploads(&rebuilt).bytes(&rebuilt);

                let previous_revision = cached.highlight_revision;
                let start = Instant::now();
                cached.update_highlights(Some(selected), &targets, None);
                black_box(&cached);
                update_times.push(start.elapsed().as_secs_f64() * 1_000_000.0);
                let uploads = cached.vertex_uploads(Some(cached.id), Some(previous_revision), &square_revisions(&cached));
                assert_eq!(uploads, VertexUploads { geometry: false, highlights: true, squares: 0 });
                update_bytes = uploads.bytes(&cached);
            }
            full_times.sort_by(f64::total_cmp);
            update_times.sort_by(f64::total_cmp);
            println!("{theme:?}: warm native CPU median, {SAMPLES} samples: full scene {:.1} us, highlights {:.1} us; planned vertex uploads {full_bytes} -> {update_bytes} bytes. Excludes asset loading and GPU timing; not FPS.",
                full_times[SAMPLES / 2], update_times[SAMPLES / 2]);
        }
    }

    #[test]
    #[ignore = "Run with --release --ignored --nocapture to compare warm CPU scene reconstruction and changed-square uploads"]
    fn benchmark_full_scene_rebuild_versus_board_updates() {
        use std::{hint::black_box, time::Instant};
        const SAMPLES: usize = 31;
        let initial = Board::default();
        let chess_move = chess::ChessMove::new(Square::E2, Square::E4, None);
        let moved = initial.make_move_new(chess_move);
        for (theme, _) in Theme::ALL {
            let mut cached = test_scene(theme, false, View::default(), None, &[], None);
            let mut full_times = Vec::with_capacity(SAMPLES);
            let mut update_times = Vec::with_capacity(SAMPLES);
            let mut full_bytes = 0;
            let mut update_bytes = 0;
            for sample in 0..SAMPLES {
                let (board, last_move) = if sample % 2 == 0 { (&moved, Some(chess_move)) } else { (&initial, None) };
                let start = Instant::now();
                let rebuilt = scene(board, cached.projection_rect, false, View::default(), None, &[], last_move,
                    Color32::BLACK, 1.0, wgpu::TextureFormat::Rgba8Unorm, 8, theme, true, PieceAppearance::default());
                black_box(&rebuilt);
                full_times.push(start.elapsed().as_secs_f64() * 1_000_000.0);
                full_bytes = all_uploads(&rebuilt).bytes(&rebuilt);

                // Match a retained egui paint callback: unchanged square
                // storage survives copy-on-write without copying its bytes.
                let painted = cached.clone();
                let previous_revisions = square_revisions(&painted);
                let start = Instant::now();
                cached.update_board(board);
                cached.update_highlights(None, &[], last_move);
                black_box(&cached);
                update_times.push(start.elapsed().as_secs_f64() * 1_000_000.0);
                let uploads = cached.vertex_uploads(Some(painted.id), Some(painted.highlight_revision), &previous_revisions);
                assert_eq!(uploads.squares, (1_u64 << Square::E2.to_index()) | (1_u64 << Square::E4.to_index()));
                assert!(!uploads.geometry);
                update_bytes = uploads.bytes(&cached);
            }
            full_times.sort_by(f64::total_cmp);
            update_times.sort_by(f64::total_cmp);
            println!("{theme:?}: warm native CPU median, {SAMPLES} alternating e2-e4/undo samples: full scene {:.1} us, changed squares {:.1} us; planned e2-e4 vertex uploads {full_bytes} -> {update_bytes} bytes. Excludes asset loading, GPU command encoding and GPU timing; not FPS.",
                full_times[SAMPLES / 2], update_times[SAMPLES / 2]);
        }
    }

    #[test]
    fn shader_validates() {
        let module = naga::front::wgsl::parse_str(super::SCENE_SHADER).expect("valid WGSL syntax");
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .expect("valid GPU shader");
    }

    #[test]
    fn distance_adjustment_matches_camera_projection() {
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(1200.0, 900.0));
        for flipped in [false, true] {
            let initial = Camera::new(flipped, View::default());
            for distance in [14.0, 20.0, 26.0] {
                let target = Camera::new(
                    flipped,
                    View {
                        distance,
                        ..View::default()
                    },
                );
                let delta = distance - View::default().distance;
                for point in [V3::new(-3.5, 0.075, 3.5), V3::new(3.5, 1.5, -3.5)] {
                    let (old, old_depth) = initial.project(point, rect).unwrap();
                    let (new, new_depth) = target.project(point, rect).unwrap();
                    // Exercise the vertex shader's clip-space expression, rather
                    // than assuming its zoom pivot matches the CPU camera.
                    let ndc = Vec2::new(old.x / rect.width() * 2.0 - 1.0, 1.0 - old.y / rect.height() * 2.0);
                    let pivot = Vec2::new(0.0, 2.0 * SCREEN_CENTER_LIFT);
                    let clip_xy = (ndc - pivot) * old_depth + pivot * (old_depth + delta);
                    let zoomed_ndc = clip_xy / (old_depth + delta);
                    let projected = Pos2::new((zoomed_ndc.x + 1.0) * rect.width() * 0.5,
                        (1.0 - zoomed_ndc.y) * rect.height() * 0.5);
                    assert!((new_depth - old_depth - delta).abs() < 0.0001);
                    assert!(new.distance(projected) < 0.001);
                }
            }
        }
    }
}
