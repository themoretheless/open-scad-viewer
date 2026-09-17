//! GPU grid sampler (feature `gpu`): runs the shared SDF_WGSL shader over the
//! flattened field tree. f32 arithmetic — see `math_core::Acceleration`.
use crate::Grid;
use crate::flat::FlatField;
use gpu_compute::{BackendReport, GpuContext, read_buffer, storage_entry, uniform_entry, wgpu};

struct GpuSdf {
    device: wgpu::Device,
    queue: wgpu::Queue,
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::ComputePipeline,
    buffers: std::cell::RefCell<Option<Buffers>>,
}

struct Buffers {
    kind_capacity: usize,
    param_capacity: usize,
    aux_capacity: usize,
    triangle_capacity: usize,
    value_capacity: usize,
    params: wgpu::Buffer,
    kinds: wgpu::Buffer,
    node_params: wgpu::Buffer,
    aux: wgpu::Buffer,
    triangles: wgpu::Buffer,
    values: wgpu::Buffer,
    values_read: wgpu::Buffer,
    bind: wgpu::BindGroup,
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
        Self {
            device: device.clone(),
            queue: context.queue.clone(),
            layout,
            pipeline,
            buffers: std::cell::RefCell::new(None),
        }
    }

    fn ensure_buffers(
        &self,
        kind_count: usize,
        param_count: usize,
        aux_count: usize,
        triangle_count: usize,
        value_count: usize,
    ) {
        let stale = match &*self.buffers.borrow() {
            Some(b) => {
                b.kind_capacity < kind_count
                    || b.param_capacity < param_count
                    || b.aux_capacity < aux_count
                    || b.triangle_capacity < triangle_count
                    || b.value_capacity < value_count
            }
            None => true,
        };
        if !stale {
            return;
        }
        let device = &self.device;
        let kind_capacity = kind_count.max(1);
        let param_capacity = param_count.max(1);
        let aux_capacity = aux_count.max(1);
        let triangle_capacity = triangle_count.max(1);
        let value_capacity = value_count.max(1);
        let mk = |label: &str, size: u64, usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage,
                mapped_at_creation: false,
            })
        };
        let params = mk(
            "sdf_params",
            40,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );
        let kinds = mk(
            "sdf_kinds",
            (kind_capacity * 4) as u64,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let node_params = mk(
            "sdf_nodes",
            (param_capacity * 4) as u64,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let aux = mk(
            "sdf_aux",
            (aux_capacity * 4) as u64,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let triangles = mk(
            "sdf_tris",
            (triangle_capacity * 4) as u64,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let values = mk(
            "sdf_values",
            (value_capacity * 4) as u64,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        );
        let values_read = mk(
            "sdf_values_read",
            (value_capacity * 4) as u64,
            wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        );
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sdf_grid"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: kinds.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: node_params.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: aux.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: triangles.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: values.as_entire_binding(),
                },
            ],
        });
        *self.buffers.borrow_mut() = Some(Buffers {
            kind_capacity,
            param_capacity,
            aux_capacity,
            triangle_capacity,
            value_capacity,
            params,
            kinds,
            node_params,
            aux,
            triangles,
            values,
            values_read,
            bind,
        });
    }

    fn run(&self, flat: &FlatField, grid: &Grid) -> Vec<f32> {
        let [nx, ny, nz] = grid.cells;
        let total = (nx + 1) * (ny + 1) * (nz + 1);
        let mut params =
            gpu_compute::pack_u32(&[nx as u32, ny as u32, nz as u32, flat.kinds.len() as u32]);
        for i in 0..3 {
            params.extend_from_slice(&(grid.min[i] as f32).to_le_bytes());
        }
        for i in 0..3 {
            let step = (grid.max[i] - grid.min[i]) / grid.cells[i] as f64;
            params.extend_from_slice(&(step as f32).to_le_bytes());
        }
        let kinds = gpu_compute::pack_u32(&flat.kinds);
        let node_params = gpu_compute::pack_f32(&flat.params);
        let aux = gpu_compute::pack_u32(&flat.aux);
        let tris = gpu_compute::pack_f32(&flat.triangles);
        self.ensure_buffers(
            flat.kinds.len(),
            flat.params.len(),
            flat.aux.len(),
            flat.triangles.len(),
            total,
        );
        let buffers = self.buffers.borrow();
        let b = buffers.as_ref().expect("ensure_buffers was just called");
        self.queue.write_buffer(&b.params, 0, &params);
        if !kinds.is_empty() {
            self.queue.write_buffer(&b.kinds, 0, &kinds);
        }
        if !node_params.is_empty() {
            self.queue.write_buffer(&b.node_params, 0, &node_params);
        }
        if !aux.is_empty() {
            self.queue.write_buffer(&b.aux, 0, &aux);
        }
        if !tris.is_empty() {
            self.queue.write_buffer(&b.triangles, 0, &tris);
        }
        let value_bytes = (total * 4) as u64;
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("sdf") });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("sdf"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &b.bind, &[]);
            pass.dispatch_workgroups((total as u32).div_ceil(256), 1, 1);
        }
        encoder.copy_buffer_to_buffer(&b.values, 0, &b.values_read, 0, value_bytes);
        self.queue.submit([encoder.finish()]);
        let raw = read_buffer(&self.device, &b.values_read, total * 4);
        b.values_read.unmap();
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

pub(crate) fn backend_report() -> Option<BackendReport> {
    SHARED.with(|cell| {
        let shared: &Option<&(GpuContext, GpuSdf)> = cell;
        shared.map(|(context, _)| context.backend_report())
    })
}
