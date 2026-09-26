//! GPU grid sampler (feature `gpu`): runs the shared SDF_WGSL shader over the
//! flattened field tree, dispatched through the compute-core runtime. f32
//! arithmetic — see `math_core::Acceleration`.
use crate::Grid;
use crate::flat::FlatField;
use compute_core::gpu_compute::wgpu;
use compute_core::gpu_compute::{BackendReport, GpuContext, pack_f32, pack_u32};
use compute_core::{Binding, Kernel, read_f32};
use wgpu::{BindGroup, Buffer, BufferUsages, Device};

const WG_METAL: u32 = 256;
const WG_DEFAULT: u32 = 256;

fn mk(device: &Device, label: &str, size: u64, usage: BufferUsages) -> Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: size.max(4),
        usage,
        mapped_at_creation: false,
    })
}

struct GpuSdf {
    device: Device,
    queue: wgpu::Queue,
    kernel: Kernel,
    buffers: std::cell::RefCell<Option<Buffers>>,
}

struct Buffers {
    kind_capacity: usize,
    param_capacity: usize,
    aux_capacity: usize,
    triangle_capacity: usize,
    value_capacity: usize,
    params: Buffer,
    kinds: Buffer,
    node_params: Buffer,
    aux: Buffer,
    triangles: Buffer,
    values: Buffer,
    bind: BindGroup,
}

impl GpuSdf {
    fn new(context: &GpuContext) -> Self {
        let kernel = Kernel::tuned(
            context,
            "sdf_grid",
            crate::SDF_WGSL,
            "main",
            &[
                Binding::Uniform,
                Binding::StorageRead,
                Binding::StorageRead,
                Binding::StorageRead,
                Binding::StorageRead,
                Binding::StorageReadWrite,
            ],
            WG_METAL,
            WG_DEFAULT,
        )
        .expect("sdf_grid kernel builds");
        Self {
            device: context.device.clone(),
            queue: context.queue.clone(),
            kernel,
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
        let params = mk(
            device,
            "sdf_params",
            40,
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        );
        let kinds = mk(
            device,
            "sdf_kinds",
            (kind_capacity * 4) as u64,
            BufferUsages::STORAGE | BufferUsages::COPY_DST,
        );
        let node_params = mk(
            device,
            "sdf_nodes",
            (param_capacity * 4) as u64,
            BufferUsages::STORAGE | BufferUsages::COPY_DST,
        );
        let aux = mk(
            device,
            "sdf_aux",
            (aux_capacity * 4) as u64,
            BufferUsages::STORAGE | BufferUsages::COPY_DST,
        );
        let triangles = mk(
            device,
            "sdf_tris",
            (triangle_capacity * 4) as u64,
            BufferUsages::STORAGE | BufferUsages::COPY_DST,
        );
        let values = mk(
            device,
            "sdf_values",
            (value_capacity * 4) as u64,
            BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        );
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sdf_grid"),
            layout: self.kernel.bind_group_layout(),
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
            bind,
        });
    }

    fn run(&self, flat: &FlatField, grid: &Grid) -> Vec<f32> {
        let [nx, ny, nz] = grid.cells;
        let total = ((nx + 1) * (ny + 1) * (nz + 1)) as u32;
        let mut params = pack_u32(&[nx as u32, ny as u32, nz as u32, flat.kinds.len() as u32]);
        for i in 0..3 {
            params.extend_from_slice(&(grid.min[i] as f32).to_le_bytes());
        }
        for i in 0..3 {
            let step = (grid.max[i] - grid.min[i]) / grid.cells[i] as f64;
            params.extend_from_slice(&(step as f32).to_le_bytes());
        }
        let kinds = pack_u32(&flat.kinds);
        let node_params = pack_f32(&flat.params);
        let aux = pack_u32(&flat.aux);
        let tris = pack_f32(&flat.triangles);
        self.ensure_buffers(
            flat.kinds.len(),
            flat.params.len(),
            flat.aux.len(),
            flat.triangles.len(),
            total as usize,
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
        self.kernel.dispatch_bind_group(
            &self.device,
            &self.queue,
            &b.bind,
            self.kernel.workgroup_count(total),
        );
        read_f32(&self.device, &self.queue, &b.values, total as usize)
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
