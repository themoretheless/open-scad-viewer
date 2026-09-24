//! Headless offscreen rasterizer. Renders mesh lists into a caller-provided
//! texture view (or a fresh texture via [`Rasterizer::render_to_rgba`]) and
//! mirrors the browser renderer's draw semantics: opaque draws write depth,
//! transparent draws blend without depth writes, morph source bound at slot 1.

use crate::pipeline::RasterPipelines;
use crate::shaders::MORPH_VERTEX_STRIDE;
use crate::uniform::{
    MORPH_BYTE_OFFSET, OBJECT_UNIFORM_BYTES, STYLE_BYTE_OFFSET, ObjectUniform, SceneUniform,
    SCENE_UNIFORM_BYTES,
};
use gpu_compute::{GpuContext, pack_f32, pack_u32, read_buffer, wgpu};

/// A drawable mesh: interleaved position+normal vertices, u32 triangle
/// indices, an object uniform buffer and an optional morph source buffer
/// (positions only, stride 3 floats).
pub struct DrawMesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
    pub uniform_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    /// Positions blended toward by `morph.x`; `None` binds a zero dummy.
    pub morph_source: Option<wgpu::Buffer>,
    pub transparent: bool,
}

pub struct Rasterizer {
    pub context: GpuContext,
    pub pipelines: RasterPipelines,
    scene_buffer: wgpu::Buffer,
    scene_bind_group: wgpu::BindGroup,
    morph_dummy: wgpu::Buffer,
    depth: Option<(wgpu::Texture, wgpu::TextureView, u32, u32)>,
}

impl Rasterizer {
    pub fn new(context: GpuContext, format: wgpu::TextureFormat) -> Self {
        let pipelines = RasterPipelines::new(&context.device, format);
        let scene_buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("raster scene uniform"),
            size: SCENE_UNIFORM_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let scene_bind_group = context.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("raster scene bind group"),
            layout: &pipelines.scene_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: scene_buffer.as_entire_binding(),
            }],
        });
        let morph_dummy = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("raster morph dummy"),
            size: MORPH_VERTEX_STRIDE,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self { context, pipelines, scene_buffer, scene_bind_group, morph_dummy, depth: None }
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.context.device
    }

    /// Uploads the full scene uniform.
    pub fn set_scene(&self, scene: &SceneUniform) {
        let mut floats = [0.0f32; 52];
        scene.write_f32(&mut floats);
        self.context.queue.write_buffer(&self.scene_buffer, 0, &pack_f32(&floats));
    }

    /// Creates a mesh with its own object uniform buffer and bind group.
    /// `vertices` is the interleaved position+normal layout (6 floats per
    /// vertex); `morph_source` is positions only (3 floats per vertex).
    pub fn create_mesh(
        &self,
        vertices: &[f32],
        indices: &[u32],
        uniform: &ObjectUniform,
        morph_source: Option<&[f32]>,
    ) -> DrawMesh {
        let device = &self.context.device;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("raster mesh vertices"),
            size: (vertices.len() * 4) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.context.queue.write_buffer(&vertex_buffer, 0, &pack_f32(vertices));
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("raster mesh indices"),
            size: (indices.len() * 4) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.context.queue.write_buffer(&index_buffer, 0, &pack_u32(indices));
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("raster object uniform"),
            size: OBJECT_UNIFORM_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut floats = [0.0f32; 44];
        uniform.write_f32(&mut floats);
        self.context.queue.write_buffer(&uniform_buffer, 0, &pack_f32(&floats));
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("raster object bind group"),
            layout: &self.pipelines.object_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });
        let morph = morph_source.map(|positions| {
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("raster morph source"),
                size: (positions.len() * 4) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.context.queue.write_buffer(&buffer, 0, &pack_f32(positions));
            buffer
        });
        DrawMesh {
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
            uniform_buffer,
            bind_group,
            morph_source: morph,
            transparent: uniform.style[0] < 1.0,
        }
    }

    /// Updates the 4-float style vector (alpha, selected, edge, hovered).
    pub fn set_style(&self, mesh: &DrawMesh, style: [f32; 4]) {
        self.context
            .queue
            .write_buffer(&mesh.uniform_buffer, STYLE_BYTE_OFFSET, &pack_f32(&style));
    }

    /// Updates the morph blend weight (`morph.x`, 0 = source, 1 = target).
    pub fn set_morph_weight(&self, mesh: &DrawMesh, weight: f32) {
        self.context
            .queue
            .write_buffer(&mesh.uniform_buffer, MORPH_BYTE_OFFSET, &pack_f32(&[weight, 0.0, 0.0, 0.0]));
    }

    fn ensure_depth(&mut self, width: u32, height: u32) {
        let recreate = self
            .depth
            .as_ref()
            .map(|(_, _, w, h)| *w != width || *h != height)
            .unwrap_or(true);
        if recreate {
            let texture = self.context.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("raster depth"),
                size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth24Plus,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            self.depth = Some((texture, view, width, height));
        }
    }

    /// Draws the meshes (opaque first, then transparent) into `target`.
    pub fn render(&mut self, target: &wgpu::TextureView, width: u32, height: u32, draws: &[&DrawMesh]) {
        self.ensure_depth(width, height);
        let mut encoder = self
            .context
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("raster") });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("raster meshes"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.05, g: 0.06, b: 0.08, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth.as_ref().unwrap().1,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            let (opaque, transparent): (Vec<&&DrawMesh>, Vec<&&DrawMesh>) =
                draws.iter().partition(|draw| !draw.transparent);
            let mut draw_list = opaque;
            draw_list.extend(transparent);
            for draw in draw_list {
                pass.set_pipeline(if draw.transparent {
                    &self.pipelines.mesh_transparent
                } else {
                    &self.pipelines.mesh_opaque
                });
                pass.set_bind_group(0, &self.scene_bind_group, &[]);
                pass.set_bind_group(1, &draw.bind_group, &[]);
                pass.set_vertex_buffer(0, draw.vertex_buffer.slice(..));
                match &draw.morph_source {
                    Some(source) => pass.set_vertex_buffer(1, source.slice(..)),
                    None => pass.set_vertex_buffer(1, self.morph_dummy.slice(..)),
                }
                pass.set_index_buffer(draw.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..draw.index_count, 0, 0..1);
            }
        }
        self.context.queue.submit([encoder.finish()]);
    }

    /// Renders into a fresh RGBA8 texture and returns the pixel bytes.
    pub fn render_to_rgba(
        &mut self,
        width: u32,
        height: u32,
        draws: &[&DrawMesh],
    ) -> Vec<u8> {
        let texture = self.context.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("raster target"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.render(&view, width, height, draws);

        let bytes_per_row = (width * 4).div_ceil(256) * 256;
        let readback = self.context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("raster readback"),
            size: (bytes_per_row * height) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = self.context.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("raster copy") });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        );
        self.context.queue.submit([encoder.finish()]);
        let mapped = read_buffer(&self.context.device, &readback, (bytes_per_row * height) as usize);
        let mut out = Vec::with_capacity((width * height * 4) as usize);
        for row in 0..height {
            let start = (row * bytes_per_row) as usize;
            out.extend_from_slice(&mapped[start..start + (width * 4) as usize]);
        }
        out
    }
}
