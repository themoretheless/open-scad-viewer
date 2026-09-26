//! Headless offscreen rasterizer. Renders a [`Frame`] (grid, deep-selection
//! underlay, meshes, edges, lines, selection overlay, instanced groups) into
//! a caller-provided texture view or a fresh texture via
//! [`Rasterizer::render_to_rgba`], mirroring the browser renderer's draw
//! semantics: opaque draws write depth, transparent draws blend without
//! depth writes, morph source bound at vertex slot 1, rest morph weight 1.

use crate::pipeline::RasterPipelines;
use crate::shaders::MORPH_VERTEX_STRIDE;
use crate::uniform::{
    MORPH_BYTE_OFFSET, OBJECT_UNIFORM_BYTES, OBJECT_UNIFORM_FLOATS, STYLE_BYTE_OFFSET,
    ObjectUniform, SceneUniform, SCENE_UNIFORM_BYTES, SCENE_UNIFORM_FLOATS,
};
use gpu_compute::{GpuContext, pack_f32, pack_u32, wgpu};

/// A drawable mesh: interleaved position+normal vertices, u32 triangle
/// indices, an object uniform buffer and an optional morph source buffer
/// (positions only, stride 3 floats).
pub struct DrawMesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
    pub uniform_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    /// Positions blended from when `morph.x` drops below the rest weight 1;
    /// `None` binds a zero dummy (inert while the weight stays at rest).
    pub morph_source: Option<wgpu::Buffer>,
    pub transparent: bool,
}

/// Edges of one mesh: same vertex buffer as the mesh, a separate index buffer
/// with u32 line pairs, and its own object uniform (edge style lives in
/// `style.z` = edge alpha).
pub struct DrawEdges {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
    pub uniform_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub morph_source: Option<wgpu::Buffer>,
}

/// Per-vertex colored line segments (pos3 + color4 per vertex, line list).
pub struct LineBatch {
    pub vertex_buffer: wgpu::Buffer,
    pub vertex_count: u32,
}

/// Per-vertex colored overlay triangles (pos3 + color4 per vertex).
pub struct OverlayBatch {
    pub vertex_buffer: wgpu::Buffer,
    pub vertex_count: u32,
}

/// GPU instance records (56 floats per instance, [`ObjectUniform`] layout),
/// bound as a read-only storage buffer at group 1.
pub struct InstancePool {
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub count: u32,
}

/// Geometry shared by the instances of one [`InstancePool`] draw.
pub struct InstancedGeometry {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
    pub morph_source: Option<wgpu::Buffer>,
}

/// One frame's worth of draws, composited in the browser renderer's order:
/// deep-selection underlay, grid, opaque meshes, instanced meshes,
/// transparent meshes, edges, lines, selection overlay.
#[derive(Default)]
pub struct Frame<'a> {
    pub clear: wgpu::Color,
    /// Fullscreen ground grid; requires `SceneUniform.inverse_vp` and
    /// `options.y` (grid step) to be set.
    pub grid: bool,
    /// Deep-selection x-ray overlay (selected mesh + its edges, drawn through
    /// everything after the meshes: depth compare `Always`, alpha blend).
    pub deep: Option<(&'a DrawMesh, &'a DrawEdges)>,
    pub meshes: &'a [&'a DrawMesh],
    /// (geometry, pool, first_instance, instance_count, transparent)
    pub instances: &'a [(&'a InstancedGeometry, &'a InstancePool, u32, u32, bool)],
    pub edges: &'a [&'a DrawEdges],
    pub lines: Option<&'a LineBatch>,
    pub overlay: Option<&'a OverlayBatch>,
}

pub struct Rasterizer {
    pub context: GpuContext,
    pub pipelines: RasterPipelines,
    scene_buffer: wgpu::Buffer,
    scene_bind_group: wgpu::BindGroup,
    morph_dummy: wgpu::Buffer,
    grid_quad: wgpu::Buffer,
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
        // Fullscreen triangle-pair quad for the grid shader (vec2 corners).
        let grid_quad = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("raster grid quad"),
            size: 6 * 8,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        context.queue.write_buffer(
            &grid_quad,
            0,
            &pack_f32(&[-1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0, -1.0, 1.0, 1.0, -1.0, 1.0]),
        );
        Self { context, pipelines, scene_buffer, scene_bind_group, morph_dummy, grid_quad, depth: None }
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.context.device
    }

    /// Uploads the full scene uniform.
    pub fn set_scene(&self, scene: &SceneUniform) {
        let mut floats = [0.0f32; SCENE_UNIFORM_FLOATS];
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
        let (vertex_buffer, index_buffer, index_count) = self.upload_geometry(vertices, indices);
        let (uniform_buffer, bind_group) = self.upload_object_uniform(uniform);
        DrawMesh {
            vertex_buffer,
            index_buffer,
            index_count,
            uniform_buffer,
            bind_group,
            morph_source: morph_source.map(|positions| self.upload_morph_source(positions)),
            transparent: uniform.style[0] < 1.0,
        }
    }

    /// Creates an edge draw over the same interleaved vertex layout; edge
    /// indices are u32 pairs into `vertices`.
    pub fn create_edges(
        &self,
        vertices: &[f32],
        edge_indices: &[u32],
        uniform: &ObjectUniform,
        morph_source: Option<&[f32]>,
    ) -> DrawEdges {
        let (vertex_buffer, index_buffer, index_count) = self.upload_geometry(vertices, edge_indices);
        let (uniform_buffer, bind_group) = self.upload_object_uniform(uniform);
        DrawEdges {
            vertex_buffer,
            index_buffer,
            index_count,
            uniform_buffer,
            bind_group,
            morph_source: morph_source.map(|positions| self.upload_morph_source(positions)),
        }
    }

    /// Line segments, 7 floats per vertex (pos3 + color4).
    pub fn create_line_batch(&self, vertices: &[f32]) -> LineBatch {
        LineBatch {
            vertex_buffer: self.upload_vertex_data(vertices, "raster lines"),
            vertex_count: (vertices.len() / 7) as u32,
        }
    }

    /// Overlay triangles, 7 floats per vertex (pos3 + color4).
    pub fn create_overlay_batch(&self, vertices: &[f32]) -> OverlayBatch {
        OverlayBatch {
            vertex_buffer: self.upload_vertex_data(vertices, "raster overlay"),
            vertex_count: (vertices.len() / 7) as u32,
        }
    }

    /// Uploads per-instance records (one [`ObjectUniform`] per instance).
    pub fn create_instance_pool(&self, records: &[ObjectUniform]) -> InstancePool {
        let mut floats = vec![0.0f32; records.len() * OBJECT_UNIFORM_FLOATS];
        for (i, record) in records.iter().enumerate() {
            let slot = &mut floats[i * OBJECT_UNIFORM_FLOATS..(i + 1) * OBJECT_UNIFORM_FLOATS];
            let slot: &mut [f32; OBJECT_UNIFORM_FLOATS] = slot.try_into().expect("instance record slot");
            record.write_f32(slot);
        }
        let buffer = self.context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("raster instance pool"),
            size: (floats.len() * 4).max(4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.context.queue.write_buffer(&buffer, 0, &pack_f32(&floats));
        let bind_group = self.context.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("raster instance bind group"),
            layout: &self.pipelines.instance_bgl,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() }],
        });
        InstancePool { buffer, bind_group, count: records.len() as u32 }
    }

    /// Geometry shared by instances (same interleaved layout as [`create_mesh`]).
    pub fn create_instanced_geometry(
        &self,
        vertices: &[f32],
        indices: &[u32],
        morph_source: Option<&[f32]>,
    ) -> InstancedGeometry {
        let (vertex_buffer, index_buffer, index_count) = self.upload_geometry(vertices, indices);
        InstancedGeometry {
            vertex_buffer,
            index_buffer,
            index_count,
            morph_source: morph_source.map(|positions| self.upload_morph_source(positions)),
        }
    }

    /// Updates the 4-float style vector (alpha, selected, edge, hovered).
    pub fn set_style(&self, mesh: &DrawMesh, style: [f32; 4]) {
        self.write_uniform_field(&mesh.uniform_buffer, STYLE_BYTE_OFFSET, &style);
    }

    /// Edge variant of [`set_style`] (edge alpha lives in `style.z`).
    pub fn set_edge_style(&self, edges: &DrawEdges, style: [f32; 4]) {
        self.write_uniform_field(&edges.uniform_buffer, STYLE_BYTE_OFFSET, &style);
    }

    /// Updates the morph blend weight (`morph.x`, 0 = source, 1 = target/rest).
    pub fn set_morph_weight(&self, mesh: &DrawMesh, weight: f32) {
        self.write_uniform_field(&mesh.uniform_buffer, MORPH_BYTE_OFFSET, &[weight, 0.0, 0.0, 0.0]);
    }

    /// Edge variant of [`set_morph_weight`].
    pub fn set_edge_morph_weight(&self, edges: &DrawEdges, weight: f32) {
        self.write_uniform_field(&edges.uniform_buffer, MORPH_BYTE_OFFSET, &[weight, 0.0, 0.0, 0.0]);
    }

    fn write_uniform_field(&self, buffer: &wgpu::Buffer, byte_offset: u64, floats: &[f32; 4]) {
        self.context.queue.write_buffer(buffer, byte_offset, &pack_f32(floats));
    }

    fn upload_vertex_data(&self, floats: &[f32], label: &str) -> wgpu::Buffer {
        let buffer = self.context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: (floats.len() * 4).max(4) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.context.queue.write_buffer(&buffer, 0, &pack_f32(floats));
        buffer
    }

    fn upload_geometry(&self, vertices: &[f32], indices: &[u32]) -> (wgpu::Buffer, wgpu::Buffer, u32) {
        let vertex_buffer = self.upload_vertex_data(vertices, "raster mesh vertices");
        let index_buffer = self.context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("raster mesh indices"),
            size: (indices.len() * 4).max(4) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.context.queue.write_buffer(&index_buffer, 0, &pack_u32(indices));
        (vertex_buffer, index_buffer, indices.len() as u32)
    }

    fn upload_morph_source(&self, positions: &[f32]) -> wgpu::Buffer {
        self.upload_vertex_data(positions, "raster morph source")
    }

    fn upload_object_uniform(&self, uniform: &ObjectUniform) -> (wgpu::Buffer, wgpu::BindGroup) {
        let uniform_buffer = self.context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("raster object uniform"),
            size: OBJECT_UNIFORM_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut floats = [0.0f32; OBJECT_UNIFORM_FLOATS];
        uniform.write_f32(&mut floats);
        self.context.queue.write_buffer(&uniform_buffer, 0, &pack_f32(&floats));
        let bind_group = self.context.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("raster object bind group"),
            layout: &self.pipelines.object_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });
        (uniform_buffer, bind_group)
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

    /// Draws the frame into `target` (opaque first, then transparent).
    pub fn render(&mut self, target: &wgpu::TextureView, width: u32, height: u32, frame: &Frame) {
        self.ensure_depth(width, height);
        let mut encoder = self
            .context
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("raster") });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("raster frame"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(frame.clear), store: wgpu::StoreOp::Store },
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
            pass.set_bind_group(0, &self.scene_bind_group, &[]);

            if frame.grid {
                pass.set_pipeline(&self.pipelines.grid);
                pass.set_vertex_buffer(0, self.grid_quad.slice(..));
                pass.draw(0..6, 0..1);
            }

            for draw in frame.meshes.iter().filter(|d| !d.transparent) {
                self.draw_mesh_with(&mut pass, &self.pipelines.mesh_opaque, &draw.vertex_buffer,
                    draw.index_count, &draw.bind_group, draw.morph_source.as_ref(), &draw.index_buffer);
            }

            for (geometry, pool, start, count, transparent) in frame.instances {
                let pipeline = if *transparent {
                    &self.pipelines.instanced_mesh_transparent
                } else {
                    &self.pipelines.instanced_mesh_opaque
                };
                pass.set_pipeline(pipeline);
                pass.set_bind_group(1, &pool.bind_group, &[]);
                pass.set_vertex_buffer(0, geometry.vertex_buffer.slice(..));
                match &geometry.morph_source {
                    Some(source) => pass.set_vertex_buffer(1, source.slice(..)),
                    None => pass.set_vertex_buffer(1, self.morph_dummy.slice(..)),
                }
                pass.set_index_buffer(geometry.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..geometry.index_count, 0, *start..*start + *count);
            }

            for draw in frame.meshes.iter().filter(|d| d.transparent) {
                self.draw_mesh_with(&mut pass, &self.pipelines.mesh_transparent, &draw.vertex_buffer,
                    draw.index_count, &draw.bind_group, draw.morph_source.as_ref(), &draw.index_buffer);
            }

            if let Some((deep_mesh, deep_edges)) = frame.deep {
                // Deep selection is an x-ray overlay: depth compare Always with
                // alpha blend, drawn after the meshes so it bleeds through.
                self.draw_mesh_with(&mut pass, &self.pipelines.deep_mesh, &deep_mesh.vertex_buffer,
                    deep_mesh.index_count, &deep_mesh.bind_group, deep_mesh.morph_source.as_ref(),
                    &deep_mesh.index_buffer);
                self.draw_edges_with(&mut pass, &self.pipelines.deep_edge, deep_edges,
                    &deep_edges.index_buffer);
            }

            for edges in frame.edges {
                self.draw_edges_with(&mut pass, &self.pipelines.edge, edges, &edges.index_buffer);
            }

            if let Some(lines) = frame.lines {
                pass.set_pipeline(&self.pipelines.line);
                pass.set_vertex_buffer(0, lines.vertex_buffer.slice(..));
                pass.draw(0..lines.vertex_count, 0..1);
            }

            if let Some(overlay) = frame.overlay {
                pass.set_pipeline(&self.pipelines.selection_overlay);
                pass.set_vertex_buffer(0, overlay.vertex_buffer.slice(..));
                pass.draw(0..overlay.vertex_count, 0..1);
            }
        }
        self.context.queue.submit([encoder.finish()]);
    }

    fn draw_mesh_with<'p>(
        &self,
        pass: &mut wgpu::RenderPass<'p>,
        pipeline: &'p wgpu::RenderPipeline,
        vertex_buffer: &'p wgpu::Buffer,
        index_count: u32,
        bind_group: &'p wgpu::BindGroup,
        morph_source: Option<&'p wgpu::Buffer>,
        index_buffer: &'p wgpu::Buffer,
    ) {
        pass.set_pipeline(pipeline);
        pass.set_bind_group(1, bind_group, &[]);
        pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        match morph_source {
            Some(source) => pass.set_vertex_buffer(1, source.slice(..)),
            None => pass.set_vertex_buffer(1, self.morph_dummy.slice(..)),
        }
        pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..index_count, 0, 0..1);
    }

    fn draw_edges_with<'p>(
        &self,
        pass: &mut wgpu::RenderPass<'p>,
        pipeline: &'p wgpu::RenderPipeline,
        edges: &'p DrawEdges,
        index_buffer: &'p wgpu::Buffer,
    ) {
        pass.set_pipeline(pipeline);
        pass.set_bind_group(1, &edges.bind_group, &[]);
        pass.set_vertex_buffer(0, edges.vertex_buffer.slice(..));
        match &edges.morph_source {
            Some(source) => pass.set_vertex_buffer(1, source.slice(..)),
            None => pass.set_vertex_buffer(1, self.morph_dummy.slice(..)),
        }
        pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..edges.index_count, 0, 0..1);
    }

    /// Renders a frame into a fresh RGBA8 texture and returns the pixel bytes.
    pub fn render_to_rgba(&mut self, width: u32, height: u32, frame: &Frame) -> Vec<u8> {
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
        self.render(&view, width, height, frame);

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
        let mapped = gpu_compute::read_buffer(&self.context.device, &readback, (bytes_per_row * height) as usize);
        let mut out = Vec::with_capacity((width * height * 4) as usize);
        for row in 0..height {
            let start = (row * bytes_per_row) as usize;
            out.extend_from_slice(&mapped[start..start + (width * 4) as usize]);
        }
        out
    }
}

/// Writes RGBA bytes as a binary PPM (P6) — dependency-free snapshot for
/// eyeballing headless frames.
pub fn write_ppm(path: &std::path::Path, width: u32, height: u32, rgba: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let mut file = std::fs::File::create(path)?;
    write!(file, "P6\n{} {}\n255\n", width, height)?;
    // Pack RGB once and issue a single write instead of a 3-byte write per pixel.
    let mut rgb = Vec::with_capacity(rgba.len() / 4 * 3);
    for px in rgba.chunks_exact(4) {
        rgb.extend_from_slice(&px[..3]);
    }
    file.write_all(&rgb)?;
    Ok(())
}
