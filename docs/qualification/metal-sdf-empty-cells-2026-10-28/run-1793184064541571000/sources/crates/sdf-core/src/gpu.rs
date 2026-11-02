//! GPU grid sampler (feature `gpu`): runs the shared SDF_WGSL shader over the
//! flattened field tree, dispatched through the compute-core runtime. f32
//! arithmetic — see `math_core::Acceleration`.
use crate::Grid;
use crate::flat::FlatField;
use compute_core::gpu_compute::wgpu;
use compute_core::gpu_compute::{BackendReport, GpuContext, f32_le_bytes, pack_u32, u32_le_bytes};
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
    #[cfg(test)]
    disable_small_updates: bool,
    buffers: std::cell::RefCell<Option<Buffers>>,
}

struct Buffers {
    kind_capacity: usize,
    param_capacity: usize,
    aux_capacity: usize,
    triangle_capacity: usize,
    value_capacity: usize,
    small_input: Option<(Buffer, [usize; 5])>,
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
            #[cfg(test)]
            disable_small_updates: false,
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
        let sizes = [
            40,
            kind_capacity * 4,
            param_capacity * 4,
            aux_capacity * 4,
            triangle_capacity * 4,
        ];
        let alignment = device.limits().min_storage_buffer_offset_alignment as usize;
        let mut offsets = [0usize; 5];
        for i in 1..5 {
            offsets[i] = (offsets[i - 1] + sizes[i - 1]).next_multiple_of(alignment);
        }
        let total = offsets[4] + sizes[4];
        let allowed = {
            #[cfg(test)]
            {
                !self.disable_small_updates
            }
            #[cfg(not(test))]
            {
                true
            }
        };
        // Qualified on Apple Silicon Metal; other backends keep their old path.
        let small_input = (allowed && gpu_compute::shared_memory_uploads(device) && total <= 4096)
            .then(|| {
                (
                    mk(
                        device,
                        "sdf small inputs",
                        total as u64,
                        BufferUsages::UNIFORM | BufferUsages::STORAGE | BufferUsages::COPY_DST,
                    ),
                    offsets,
                )
            });
        let (params, kinds, node_params, aux, triangles) = if let Some((buffer, _)) = &small_input {
            (
                buffer.clone(),
                buffer.clone(),
                buffer.clone(),
                buffer.clone(),
                buffer.clone(),
            )
        } else {
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
            (params, kinds, node_params, aux, triangles)
        };
        let values = mk(
            device,
            "sdf_values",
            (value_capacity * 4) as u64,
            BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        );
        fn binding(buffer: &Buffer, offset: usize, size: usize) -> wgpu::BindingResource<'_> {
            wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer,
                offset: offset as u64,
                size: std::num::NonZeroU64::new(size as u64),
            })
        }
        let bind_offsets = small_input.as_ref().map_or([0; 5], |(_, offsets)| *offsets);
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sdf_grid"),
            layout: self.kernel.bind_group_layout(),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: binding(&params, bind_offsets[0], sizes[0]),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: binding(&kinds, bind_offsets[1], sizes[1]),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: binding(&node_params, bind_offsets[2], sizes[2]),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: binding(&aux, bind_offsets[3], sizes[3]),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: binding(&triangles, bind_offsets[4], sizes[4]),
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
            small_input,
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
        let kinds = u32_le_bytes(&flat.kinds);
        let node_params = f32_le_bytes(&flat.params);
        let aux = u32_le_bytes(&flat.aux);
        let tris = f32_le_bytes(&flat.triangles);
        self.ensure_buffers(
            flat.kinds.len(),
            flat.params.len(),
            flat.aux.len(),
            flat.triangles.len(),
            total as usize,
        );
        let buffers = self.buffers.borrow();
        let b = buffers.as_ref().expect("ensure_buffers was just called");
        if let Some((buffer, offsets)) = &b.small_input {
            let zero = [0u8; 4096];
            gpu_compute::write_small_parts(
                &self.queue,
                buffer,
                0,
                [
                    params.as_slice(),
                    &zero[..offsets[1] - params.len()],
                    &kinds,
                    &zero[..offsets[2] - offsets[1] - kinds.len()],
                    &node_params,
                    &zero[..offsets[3] - offsets[2] - node_params.len()],
                    &aux,
                    &zero[..offsets[4] - offsets[3] - aux.len()],
                    &tris,
                ]
                .into_iter(),
            )
            .expect("validated small SDF upload");
        } else {
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
    #[cfg(test)]
    if let Some(result) = TEST_SDF.with(|s| s.borrow().as_ref().map(|s| s.run(flat, grid))) {
        return Some(result);
    }
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

#[cfg(test)]
thread_local! { static TEST_SDF: std::cell::RefCell<Option<std::rc::Rc<GpuSdf>>> = const { std::cell::RefCell::new(None) }; }
#[cfg(test)]
mod small_batch_tests;
