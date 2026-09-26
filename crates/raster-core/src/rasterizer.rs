//! Headless offscreen rasterizer. Renders a [`Frame`] (grid, deep-selection
//! underlay, meshes, edges, lines, selection overlay, instanced groups) into
//! a caller-provided texture view or a fresh texture via
//! [`Rasterizer::render_to_rgba`], mirroring the browser renderer's draw
//! semantics: opaque draws write depth, transparent draws blend without
//! depth writes, morph source bound at vertex slot 1, rest morph weight 1.

use crate::pipeline::RasterPipelines;
use crate::shaders::MORPH_VERTEX_STRIDE;
use crate::uniform::{SCENE_UNIFORM_BYTES, SCENE_UNIFORM_FLOATS, SceneUniform};
use gpu_compute::{GpuContext, pack_f32, wgpu};

pub use crate::resources::{
    DrawEdges, DrawMesh, InstancePool, InstancedGeometry, LineBatch, OverlayBatch,
};

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
    format: wgpu::TextureFormat,
    pub context: GpuContext,
    pub pipelines: RasterPipelines,
    scene_buffer: wgpu::Buffer,
    scene_bind_group: wgpu::BindGroup,
    morph_dummy: wgpu::Buffer,
    grid_quad: wgpu::Buffer,
    depth: Option<(wgpu::Texture, wgpu::TextureView, u32, u32)>,
    /// 1×1 dummy depth map + comparison sampler bound where mesh-surface and
    /// grid shaders sample the key-light shadow map; `shadow_params` stays
    /// disabled natively, so the dummy is never actually read.
    shadow_bind_group: wgpu::BindGroup,
    /// Mirrors `SceneUniform.options.x`; gates the section-cap pass.
    section_enabled: bool,
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
        let scene_bind_group = context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
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
            &pack_f32(&[
                -1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0, -1.0, 1.0, 1.0, -1.0, 1.0,
            ]),
        );
        // Shadow sampling shaders always have something bound: natively a 1×1
        // depth dummy + comparison sampler, with shadow_params disabled.
        let shadow_dummy = context.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("raster shadow dummy"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let shadow_sampler = context.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("raster shadow sampler"),
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        let shadow_bind_group = context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("raster shadow bind group"),
                layout: &pipelines.shadow_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(
                            &shadow_dummy.create_view(&Default::default()),
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&shadow_sampler),
                    },
                ],
            });
        Self {
            format,
            context,
            pipelines,
            scene_buffer,
            scene_bind_group,
            morph_dummy,
            grid_quad,
            depth: None,
            shadow_bind_group,
            section_enabled: false,
        }
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.context.device
    }

    /// Uploads the full scene uniform.
    pub fn set_scene(&mut self, scene: &SceneUniform) {
        let mut floats = [0.0f32; SCENE_UNIFORM_FLOATS];
        scene.write_f32(&mut floats);
        self.context
            .queue
            .write_buffer(&self.scene_buffer, 0, &pack_f32(&floats));
        self.section_enabled = scene.options[0] > 0.5;
    }

    fn ensure_depth(&mut self, width: u32, height: u32) {
        let recreate = self
            .depth
            .as_ref()
            .map(|(_, _, w, h)| *w != width || *h != height)
            .unwrap_or(true);
        if recreate {
            let texture = self
                .context
                .device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("raster depth"),
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
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
        let mut encoder =
            self.context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("raster"),
                });
        self.record(&mut encoder, target, width, height, frame);
        self.context.queue.submit([encoder.finish()]);
    }

    /// Records rendering into the caller's encoder without submitting. The
    /// encoder and target must belong to this rasterizer's device. This allows
    /// compute, copies and rendering to share an ordered command buffer.
    /// Uniform queue writes apply before submission, not between recorded frames.
    pub fn record(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        width: u32,
        height: u32,
        frame: &Frame,
    ) {
        self.ensure_depth(width, height);
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("raster frame"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(frame.clear),
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
            pass.set_bind_group(0, &self.scene_bind_group, &[]);

            if frame.grid {
                pass.set_pipeline(&self.pipelines.grid);
                pass.set_bind_group(1, &self.shadow_bind_group, &[]);
                pass.set_vertex_buffer(0, self.grid_quad.slice(..));
                pass.draw(0..6, 0..1);
            }

            for draw in frame.meshes.iter().filter(|d| !d.transparent) {
                self.draw_mesh_with(
                    &mut pass,
                    &self.pipelines.mesh_opaque,
                    &draw.vertex_buffer,
                    draw.index_count,
                    &draw.bind_group,
                    draw.morph_source.as_ref(),
                    &draw.index_buffer,
                );
            }

            if self.section_enabled {
                // Section cap: redraw the opaque meshes with the inverted-clip,
                // front-culled cap pipeline. The interior back faces of closed
                // solids survive and read as a filled, unlit cut surface.
                for draw in frame.meshes.iter().filter(|d| !d.transparent) {
                    self.draw_mesh_with(
                        &mut pass,
                        &self.pipelines.section_cap,
                        &draw.vertex_buffer,
                        draw.index_count,
                        &draw.bind_group,
                        draw.morph_source.as_ref(),
                        &draw.index_buffer,
                    );
                }
            }

            for (geometry, pool, start, count, transparent) in frame.instances {
                let pipeline = if *transparent {
                    &self.pipelines.instanced_mesh_transparent
                } else {
                    &self.pipelines.instanced_mesh_opaque
                };
                pass.set_pipeline(pipeline);
                pass.set_bind_group(1, &pool.bind_group, &[]);
                pass.set_bind_group(2, &self.shadow_bind_group, &[]);
                pass.set_vertex_buffer(0, geometry.vertex_buffer.slice(..));
                match &geometry.morph_source {
                    Some(source) => pass.set_vertex_buffer(1, source.slice(..)),
                    None => pass.set_vertex_buffer(1, self.morph_dummy.slice(..)),
                }
                pass.set_index_buffer(geometry.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..geometry.index_count, 0, *start..*start + *count);
            }

            for draw in frame.meshes.iter().filter(|d| d.transparent) {
                self.draw_mesh_with(
                    &mut pass,
                    &self.pipelines.mesh_transparent,
                    &draw.vertex_buffer,
                    draw.index_count,
                    &draw.bind_group,
                    draw.morph_source.as_ref(),
                    &draw.index_buffer,
                );
            }

            if let Some((deep_mesh, deep_edges)) = frame.deep {
                // Deep selection is an x-ray overlay: depth compare Always with
                // alpha blend, drawn after the meshes so it bleeds through.
                self.draw_mesh_with(
                    &mut pass,
                    &self.pipelines.deep_mesh,
                    &deep_mesh.vertex_buffer,
                    deep_mesh.index_count,
                    &deep_mesh.bind_group,
                    deep_mesh.morph_source.as_ref(),
                    &deep_mesh.index_buffer,
                );
                self.draw_edges_with(
                    &mut pass,
                    &self.pipelines.deep_edge,
                    deep_edges,
                    &deep_edges.index_buffer,
                );
            }

            for edges in frame.edges {
                self.draw_edges_with(&mut pass, &self.pipelines.edge, edges, &edges.index_buffer);
            }

            if let Some(lines) = frame.lines {
                pass.set_pipeline(&self.pipelines.line);
                pass.set_bind_group(1, &self.shadow_bind_group, &[]);
                pass.set_vertex_buffer(0, lines.vertex_buffer.slice(..));
                pass.draw(0..lines.vertex_count, 0..1);
            }

            if let Some(overlay) = frame.overlay {
                pass.set_pipeline(&self.pipelines.selection_overlay);
                pass.set_bind_group(1, &self.shadow_bind_group, &[]);
                pass.set_vertex_buffer(0, overlay.vertex_buffer.slice(..));
                pass.draw(0..overlay.vertex_count, 0..1);
            }
        }
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
        // Mesh-surface shaders sample the shadow map at group(2); the dummy
        // binding is inert while shadow_params stays disabled, and pipelines
        // whose shader does not declare group(2) simply ignore it.
        pass.set_bind_group(2, &self.shadow_bind_group, &[]);
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
        pass.set_bind_group(2, &self.shadow_bind_group, &[]);
        pass.set_vertex_buffer(0, edges.vertex_buffer.slice(..));
        match &edges.morph_source {
            Some(source) => pass.set_vertex_buffer(1, source.slice(..)),
            None => pass.set_vertex_buffer(1, self.morph_dummy.slice(..)),
        }
        pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..edges.index_count, 0, 0..1);
    }

    /// Compatibility convenience. Use `try_render_to_rgba` to recover errors.
    pub fn render_to_rgba(&mut self, width: u32, height: u32, frame: &Frame) -> Vec<u8> {
        self.try_render_to_rgba(width, height, frame)
            .expect("RGBA readback failed")
    }

    pub fn try_render_to_rgba(
        &mut self,
        width: u32,
        height: u32,
        frame: &Frame,
    ) -> Result<Vec<u8>, gpu_compute::ReadbackError> {
        self.render_rgba_async(width, height, frame)?
            .wait(std::time::Duration::from_secs(30))
    }

    /// Records the frame and its readback into one submission, without waiting.
    pub fn render_rgba_async(
        &mut self,
        width: u32,
        height: u32,
        frame: &Frame,
    ) -> Result<crate::readback::RgbaReadback, gpu_compute::ReadbackError> {
        if self.format != wgpu::TextureFormat::Rgba8Unorm {
            return Err(gpu_compute::ReadbackError::Device(
                "RGBA output requires an Rgba8Unorm rasterizer".into(),
            ));
        }
        let texture = crate::readback::RgbaReadback::target(&self.context.device, width, height)?;
        let view = texture.create_view(&Default::default());
        let mut encoder = self
            .context
            .device
            .create_command_encoder(&Default::default());
        self.record(&mut encoder, &view, width, height, frame);
        let mut ticket = crate::readback::RgbaReadback::record(
            &self.context.device,
            &mut encoder,
            &texture,
            width,
            height,
        )?;
        ticket.submitted(self.context.queue.submit([encoder.finish()]));
        Ok(ticket)
    }
}

/// Writes RGBA bytes as a binary PPM (P6) — dependency-free snapshot for
/// eyeballing headless frames.
pub fn write_ppm(
    path: &std::path::Path,
    width: u32,
    height: u32,
    rgba: &[u8],
) -> std::io::Result<()> {
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
