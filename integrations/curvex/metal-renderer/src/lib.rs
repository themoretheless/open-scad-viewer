//! GPU-resident, immutable 2D geometry. The application owns revision lifetimes.
//! Camera and material changes must not mutate this buffer allocation.
pub mod cache;
#[cfg(feature = "egui-integration")]
pub mod egui_integration;
use wgpu::util::DeviceExt;

/// Packed position and premultiplied sRGBA vertex, independent of UI types.
#[derive(Clone, Copy, Debug)]
pub struct Vertex {
    pub position: [f32; 2],
    pub color: [u8; 4],
}

pub struct ResidentMesh {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    index_count: u32,
    allocated_bytes: u64,
}

impl ResidentMesh {
    /// Validate before allocating/uploading. Caller enforces the total cache budget.
    pub fn upload(
        device: &wgpu::Device,
        vertices: &[Vertex],
        indices: &[u32],
        byte_budget: u64,
    ) -> Result<Self, &'static str> {
        if vertices.is_empty() || indices.is_empty() || !indices.len().is_multiple_of(3) {
            return Err("nonempty triangle mesh required");
        }
        let index_count = u32::try_from(indices.len()).map_err(|_| "index count overflow")?;
        if vertices
            .iter()
            .any(|v| v.position.iter().any(|x| !x.is_finite()))
        {
            return Err("nonfinite vertex");
        }
        if indices.iter().any(|&i| i as usize >= vertices.len()) {
            return Err("index outside vertex buffer");
        }
        let vertex_bytes = (vertices.len() as u64)
            .checked_mul(12)
            .ok_or("vertex size overflow")?;
        let index_bytes = (indices.len() as u64)
            .checked_mul(4)
            .ok_or("index size overflow")?;
        let total = vertex_bytes
            .checked_add(index_bytes)
            .ok_or("mesh size overflow")?;
        if total > byte_budget
            || vertex_bytes > device.limits().max_buffer_size
            || index_bytes > device.limits().max_buffer_size
        {
            return Err("GPU geometry budget exceeded");
        }
        // Explicit byte encoding avoids layout/unsafe assumptions across UI crates.
        let mut vb = Vec::with_capacity(vertex_bytes as usize);
        for v in vertices {
            for p in v.position {
                vb.extend_from_slice(&p.to_le_bytes());
            }
            vb.extend_from_slice(&v.color);
        }
        let mut ib = Vec::with_capacity(index_bytes as usize);
        for i in indices {
            ib.extend_from_slice(&i.to_le_bytes());
        }
        Ok(Self {
            vertices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("2D resident vertices"),
                contents: &vb,
                usage: wgpu::BufferUsages::VERTEX,
            }),
            indices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("2D resident indices"),
                contents: &ib,
                usage: wgpu::BufferUsages::INDEX,
            }),
            index_count,
            allocated_bytes: total,
        })
    }
}

/// The arithmetic order matches Curvex's screen-space camera conversion.
#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub screen: [f32; 2],
    pub origin: [f32; 2],
    pub offset: [f32; 2],
    pub zoom: f32,
    pub dithering: bool,
}
impl Camera {
    fn bytes(self) -> Result<[u8; 32], &'static str> {
        let values = [
            self.screen[0],
            self.screen[1],
            self.origin[0],
            self.origin[1],
            self.offset[0],
            self.offset[1],
            self.zoom,
            if self.dithering { 1.0 } else { 0.0 },
        ];
        if values.iter().any(|v| !v.is_finite())
            || self.screen.iter().any(|v| *v <= 0.0)
            || self.zoom <= 0.0
        {
            return Err("invalid camera");
        }
        let mut bytes = [0; 32];
        for (dst, src) in bytes.as_chunks_mut::<4>().0.iter_mut().zip(values) {
            dst.copy_from_slice(&src.to_le_bytes());
        }
        Ok(bytes)
    }
}

/// One camera per render batch. Separate batches must have separate uniform
/// bindings when encoded before a submission (queue writes are not draw-local).
pub struct CameraBinding {
    buffer: wgpu::Buffer,
    group: wgpu::BindGroup,
}
impl CameraBinding {
    pub fn update(&self, queue: &wgpu::Queue, camera: Camera) -> Result<(), &'static str> {
        queue.write_buffer(&self.buffer, 0, &camera.bytes()?);
        Ok(())
    }
}

/// Solid/per-vertex-color triangle pipeline. Texture paint is handled by the
/// host renderer. Uses premultiplied gamma colors in a non-sRGB attachment.
pub struct RetainedPipeline {
    pipeline: wgpu::RenderPipeline,
    camera_layout: wgpu::BindGroupLayout,
}
impl RetainedPipeline {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        samples: u32,
    ) -> Result<Self, &'static str> {
        if !matches!(
            format,
            wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Bgra8Unorm
        ) {
            return Err("retained pipeline requires gamma framebuffer");
        }
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("2D camera layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: std::num::NonZeroU64::new(32),
                },
                count: None,
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("2D retained layout"),
            bind_group_layouts: &[Some(&camera_layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("2D retained shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("retained.wgsl").into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("2D retained pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 12,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Unorm8x4],
                }],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: samples,
                ..Default::default()
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::OneMinusDstAlpha,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        Ok(Self {
            pipeline,
            camera_layout,
        })
    }
    pub fn camera(
        &self,
        device: &wgpu::Device,
        camera: Camera,
    ) -> Result<CameraBinding, &'static str> {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("2D camera"),
            contents: &camera.bytes()?,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("2D camera binding"),
            layout: &self.camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
        Ok(CameraBinding { buffer, group })
    }
    pub fn draw(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        mesh: &ResidentMesh,
        camera: &CameraBinding,
    ) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &camera.group, &[]);
        pass.set_vertex_buffer(0, mesh.vertices.slice(..));
        pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..mesh.index_count, 0, 0..1);
    }
}
