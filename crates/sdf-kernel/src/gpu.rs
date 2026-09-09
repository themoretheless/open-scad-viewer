//! GPU grid sampler (feature `gpu`): runs the shared SDF_WGSL shader over the
//! flattened field tree. f32 arithmetic — see `Acceleration` in lib.rs.
use crate::flat::FlatField;
use crate::Grid;
use gpu_compute::{read_buffer, storage_entry, uniform_entry, wgpu, GpuContext};
use wgpu::util::DeviceExt;

struct GpuSdf {
    device: wgpu::Device,
    queue: wgpu::Queue,
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::ComputePipeline,
}

impl GpuSdf {
    fn new(context: &GpuContext) -> Self {
        let device = &context.device;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sdf_grid"),
            source: wgpu::ShaderSource::Wgsl(crate::SDF_WGSL.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sdf_grid"),
            entries: &[
                uniform_entry(0),
                storage_entry(1, true),
                storage_entry(2, true),
                storage_entry(3, true),
                storage_entry(4, true),
                storage_entry(5, false),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sdf_grid"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("sdf_grid"),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        Self { device: device.clone(), queue: context.queue.clone(), layout, pipeline }
    }

    fn run(&self, flat: &FlatField, grid: &Grid) -> Vec<f32> {
        let [nx, ny, nz] = grid.cells;
        let total = (nx + 1) * (ny + 1) * (nz + 1);
        let device = &self.device;
        let mut params = Vec::with_capacity(40);
        for v in [nx as u32, ny as u32, nz as u32, flat.kinds.len() as u32] {
            params.extend_from_slice(&v.to_ne_bytes());
        }
        for i in 0..3 {
            params.extend_from_slice(&(grid.min[i] as f32).to_ne_bytes());
        }
        for i in 0..3 {
            let step = (grid.max[i] - grid.min[i]) / grid.cells[i] as f64;
            params.extend_from_slice(&(step as f32).to_ne_bytes());
        }
        let mut kinds = Vec::with_capacity(flat.kinds.len() * 4);
        for v in &flat.kinds {
            kinds.extend_from_slice(&v.to_ne_bytes());
        }
        let mut node_params = Vec::with_capacity(flat.params.len() * 4);
        for v in &flat.params {
            node_params.extend_from_slice(&v.to_ne_bytes());
        }
        let mut aux = Vec::with_capacity(flat.aux.len() * 4);
        for v in &flat.aux {
            aux.extend_from_slice(&v.to_ne_bytes());
        }
        let mut tris = Vec::with_capacity(flat.triangles.len() * 4);
        for v in &flat.triangles {
            tris.extend_from_slice(&v.to_ne_bytes());
        }
        let mk = |label: &str, bytes: &[u8], usage| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytes,
                usage,
            })
        };
        let params_buf = mk("params", &params, wgpu::BufferUsages::UNIFORM);
        let kinds_buf = mk("kinds", &kinds, wgpu::BufferUsages::STORAGE);
        let node_buf = mk("nodes", &node_params, wgpu::BufferUsages::STORAGE);
        let aux_buf = mk("aux", if aux.is_empty() { &[0u8; 4] } else { &aux }, wgpu::BufferUsages::STORAGE);
        let tris_buf = mk("tris", if tris.is_empty() { &[0u8; 4] } else { &tris }, wgpu::BufferUsages::STORAGE);
        let value_bytes = (total * 4) as u64;
        let out_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("values"),
            size: value_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let read_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("values_read"),
            size: value_bytes,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sdf_grid"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: params_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: kinds_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: node_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: aux_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 4, resource: tris_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 5, resource: out_buf.as_entire_binding() },
            ],
        });
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("sdf") });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("sdf"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.dispatch_workgroups((total as u32).div_ceil(256), 1, 1);
        }
        encoder.copy_buffer_to_buffer(&out_buf, 0, &read_buf, 0, value_bytes);
        self.queue.submit([encoder.finish()]);
        let raw = read_buffer(device, &read_buf, total * 4);
        raw.chunks_exact(4)
            .take(total)
            .map(|c| f32::from_ne_bytes(c.try_into().unwrap()))
            .collect()
    }
}

thread_local! {
    // Process-lifetime device and pipeline; see photogrammetry gpu::matching.
    static SHARED: std::cell::LazyCell<Option<&'static (GpuContext, GpuSdf)>> =
        std::cell::LazyCell::new(|| GpuContext::new().map(|c| {
            let sdf = GpuSdf::new(&c);
            Box::leak(Box::new((c, sdf))) as &'static (GpuContext, GpuSdf)
        }));
}

/// Samples the whole grid on the GPU; None without an adapter (CPU fallback).
pub(crate) fn sample_grid_gpu(flat: &FlatField, grid: &Grid) -> Option<Vec<f32>> {
    SHARED.with(|cell| {
        let shared: &Option<&(GpuContext, GpuSdf)> = cell;
        shared.map(|(_, sdf)| sdf.run(flat, grid))
    })
}
