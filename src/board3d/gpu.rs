//! WGPU rendering of the projected chess scene inside egui's paint pass.
use super::*;
use eframe::{egui_wgpu, wgpu};
use egui_wgpu::CallbackTrait;
use std::sync::Arc;
use wgpu::util::DeviceExt;

const SCENE_SHADER: &str = include_str!("gpu_scene.wgsl");

#[derive(Clone)]
pub struct Scene {
    vertices: Arc<[u8]>,
    id: u64,
    glow_count: u32,
    board_count: u32,
    overlay_count: u32,
    shadow_count: u32,
    piece_count: u32,
    size: [u32; 2],
    background: Color32,
    target_format: wgpu::TextureFormat,
    distance_delta: f32,
    appearance: f32,
    theme: Theme,
}

impl Scene {
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
) -> Scene {
    let camera = Camera::new(flipped, view);
    let mut board_triangles = Vec::new();
    let model = &theme_models(theme).board;
    for face in model.indices.chunks_exact(3) {
        let points = std::array::from_fn(|i| model.positions[face[i] as usize]);
        let uv = std::array::from_fn(|i| model.texcoords[face[i] as usize]);
        let normals = std::array::from_fn(|i| model.normals[face[i] as usize]);
        push_triangle(
            &mut board_triangles,
            camera,
            rect,
            points,
            [230; 3],
            Some((uv, None)),
            Some(normals),
        );
    }
    let dynamic = dynamic_triangles(board, rect, flipped, view, selected, targets, last_move, theme);
    let mut vertices = Vec::with_capacity((board_triangles.len() + dynamic.len()) * 3 * 104);
    let glow_count = append_triangles(&mut vertices, &board_glow(camera, rect, theme), rect);
    let board_count = append_triangles(&mut vertices, &board_triangles, rect);
    let overlay_count = append_triangles(
        &mut vertices,
        dynamic
            .iter()
            .filter(|t| t.texture.is_none() && t.shadow_uv.is_none()),
        rect,
    );
    let shadow_count = append_triangles(
        &mut vertices,
        dynamic.iter().filter(|t| t.shadow_uv.is_some()),
        rect,
    );
    let piece_count = append_triangles(
        &mut vertices,
        dynamic.iter().filter(|t| t.texture.is_some()),
        rect,
    );
    // Render above display resolution and let the blit sampler soften edges.
    let (supersample, limit) = match theme {
        Theme::Marble => (1.5, 2048.0),
        Theme::Wood => (1.65, 2304.0),
        Theme::Glass => (2.0, 2816.0),
        Theme::ArtDeco | Theme::Egyptian => (1.5, 2048.0),
    };
    let scale = (pixels_per_point * supersample).min(limit / rect.width().max(rect.height()));
    Scene {
        vertices: vertices.into(),
        id,
        glow_count,
        board_count,
        overlay_count,
        shadow_count,
        piece_count,
        size: [
            (rect.width() * scale).ceil().max(1.0) as u32,
            (rect.height() * scale).ceil().max(1.0) as u32,
        ],
        background,
        target_format,
        distance_delta: 0.0,
        appearance: 0.5,
        theme,
    }
}

struct Gpu {
    theme: Theme,
    scene_pipeline: wgpu::RenderPipeline,
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
    rendered_scene_id: Option<u64>,
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
        device.on_uncaptured_error(Box::new(|error| log::error!("3D GPU error: {error:?}")));
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
            size: 16,
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
            bind_group_layouts: &[&textures_layout],
            push_constant_ranges: &[],
        });
        let vertex_attributes = wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x2, 2 => Float32x4, 3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4];
        let make_pipeline = |depth_write_enabled, blend: Option<wgpu::BlendState>| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("3D chess scene"),
                layout: Some(&scene_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("scene_vertex"),
                    compilation_options: Default::default(),
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: 104,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &vertex_attributes,
                    }],
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
                    depth_write_enabled,
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                multiview: None,
                cache: None,
            })
        };
        let scene_pipeline = make_pipeline(true, None);
        let translucent_pipeline = make_pipeline(false, Some(wgpu::BlendState::ALPHA_BLENDING));
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
            bind_group_layouts: &[&blit_layout],
            push_constant_ranges: &[],
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
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
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
            rendered_scene_id: None,
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
        let changed_scene = gpu.rendered_scene_id != Some(self.id);
        let delta_bits = self.distance_delta.to_bits();
        let appearance_bits = self.appearance.to_bits();
        if !changed_scene && gpu.rendered_distance_delta == Some(delta_bits)
            && gpu.rendered_appearance == Some(appearance_bits) {
            return Vec::new();
        }
        let mut adjustment = [0_u8; 16];
        adjustment[..4].copy_from_slice(&self.distance_delta.to_le_bytes());
        adjustment[4..8].copy_from_slice(&self.appearance.to_le_bytes());
        let theme_code = match self.theme {
            Theme::Marble => 0.0_f32, Theme::Wood => 1.0, Theme::Glass => 2.0,
            Theme::ArtDeco => 3.0,
            Theme::Egyptian => 4.0,
        };
        adjustment[8..12].copy_from_slice(&theme_code.to_le_bytes());
        queue.write_buffer(&gpu.distance_buffer, 0, &adjustment);
        if changed_scene {
            gpu.vertex_buffer = Some(device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("3D chess vertices"),
                    contents: &self.vertices,
                    usage: wgpu::BufferUsages::VERTEX,
                },
            ));
        }
        let c = self.background;
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("3D chess scene"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &gpu.color_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: c.r() as f64 / 255.0,
                        g: c.g() as f64 / 255.0,
                        b: c.b() as f64 / 255.0,
                        a: 1.0,
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
        let mut start = self.glow_count + self.board_count;
        pass.draw(start..start + self.overlay_count, 0..1);
        start += self.overlay_count;
        pass.draw(start..start + self.shadow_count, 0..1);
        start += self.shadow_count;
        pass.set_pipeline(&gpu.scene_pipeline);
        pass.draw(start..start + self.piece_count, 0..1);
        drop(pass);
        gpu.rendered_scene_id = Some(self.id);
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
                    let projected = initial.screen_center(rect)
                        + (old - initial.screen_center(rect)) * old_depth / (old_depth + delta);
                    assert!((new_depth - old_depth - delta).abs() < 0.0001);
                    assert!(new.distance(projected) < 0.001);
                }
            }
        }
    }
}
