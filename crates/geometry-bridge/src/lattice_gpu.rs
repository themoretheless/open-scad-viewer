//! GPU evaluation of the lattice implicit field (feature `gpu`). The BVH is
//! flattened to plain arrays; each grid point runs `LATTICE_WGSL` on the GPU
//! in f32, dispatched through the compute-core runtime. Points whose
//! ray-parity walk overflows write NaN and are recomputed by the CPU field
//! closure. Extraction stays on the CPU reference path.
use crate::mesh_shell::{LATTICE_WGSL, Node, P, flatten_lattice_bvh};
use compute_core::gpu_compute::wgpu;
use compute_core::gpu_compute::{BackendReport, GpuContext, pack_f32};
use compute_core::{Binding, Kernel, read_f32};
use wgpu::{BindGroup, Buffer, BufferUsages, Device};

const WG_METAL: u32 = 128;
const WG_DEFAULT: u32 = 256;

type Segments = [(P, P, f64, f64)];

fn mk(device: &Device, label: &str, size: u64, usage: BufferUsages) -> Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: size.max(4),
        usage,
        mapped_at_creation: false,
    })
}

struct GpuLattice {
    device: Device,
    queue: wgpu::Queue,
    kernel: Kernel,
    buffers: std::cell::RefCell<Option<Buffers>>,
}

struct Buffers {
    node_capacity: usize,
    triangle_capacity: usize,
    segment_capacity: usize,
    value_capacity: usize,
    params: Buffer,
    nodes: Buffer,
    triangles: Buffer,
    segments: Buffer,
    values: Buffer,
    bind: BindGroup,
}

impl GpuLattice {
    fn new(context: &GpuContext) -> Self {
        let kernel = Kernel::tuned(
            context,
            "lattice",
            LATTICE_WGSL,
            "main",
            &[
                Binding::Uniform,
                Binding::StorageRead,
                Binding::StorageRead,
                Binding::StorageRead,
                Binding::StorageReadWrite,
            ],
            WG_METAL,
            WG_DEFAULT,
        )
        .expect("lattice kernel builds");
        Self {
            device: context.device.clone(),
            queue: context.queue.clone(),
            kernel,
            buffers: std::cell::RefCell::new(None),
        }
    }

    fn ensure_buffers(
        &self,
        node_values: usize,
        triangle_values: usize,
        segment_count: usize,
        value_count: usize,
    ) {
        let stale = match &*self.buffers.borrow() {
            Some(b) => {
                b.node_capacity < node_values
                    || b.triangle_capacity < triangle_values
                    || b.segment_capacity < segment_count
                    || b.value_capacity < value_count
            }
            None => true,
        };
        if !stale {
            return;
        }
        let device = &self.device;
        let node_capacity = node_values.max(1);
        let triangle_capacity = triangle_values.max(1);
        let segment_capacity = segment_count.max(1);
        let value_capacity = value_count.max(1);
        let params = mk(
            device,
            "lattice_params",
            80,
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        );
        let nodes = mk(
            device,
            "lattice_nodes",
            (node_capacity * 4) as u64,
            BufferUsages::STORAGE | BufferUsages::COPY_DST,
        );
        let triangles = mk(
            device,
            "lattice_tris",
            (triangle_capacity * 4) as u64,
            BufferUsages::STORAGE | BufferUsages::COPY_DST,
        );
        let segments = mk(
            device,
            "lattice_segments",
            (segment_capacity * 32) as u64,
            BufferUsages::STORAGE | BufferUsages::COPY_DST,
        );
        let values = mk(
            device,
            "lattice_values",
            (value_capacity * 4) as u64,
            BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        );
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lattice"),
            layout: self.kernel.bind_group_layout(),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: nodes.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: triangles.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: segments.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: values.as_entire_binding(),
                },
            ],
        });
        *self.buffers.borrow_mut() = Some(Buffers {
            node_capacity,
            triangle_capacity,
            segment_capacity,
            value_capacity,
            params,
            nodes,
            triangles,
            segments,
            values,
            bind,
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn run(
        &self,
        nodes: &[f32],
        triangles: &[f32],
        segments: &Segments,
        min: P,
        max: P,
        cells: [usize; 3],
        skin: f64,
        blend: f64,
        organic: bool,
        open_top: bool,
        wall_depth: f64,
        keep_core: bool,
        top_z: f64,
    ) -> Vec<f32> {
        let total = ((cells[0] + 1) * (cells[1] + 1) * (cells[2] + 1)) as u32;
        let mut params = Vec::with_capacity(80);
        for v in [
            cells[0] as u32,
            cells[1] as u32,
            cells[2] as u32,
            segments.len() as u32,
            (nodes.len() / 10) as u32,
            organic as u32,
            open_top as u32,
            keep_core as u32,
        ] {
            params.extend_from_slice(&v.to_le_bytes());
        }
        for i in 0..3 {
            params.extend_from_slice(&(min[i] as f32).to_le_bytes());
        }
        for i in 0..3 {
            // The shader maps grid indices by p = min + i * step.
            params.extend_from_slice(&(((max[i] - min[i]) / cells[i] as f64) as f32).to_le_bytes());
        }
        for v in [
            skin as f32,
            blend as f32,
            wall_depth as f32,
            top_z as f32,
            0.,
            0.,
        ] {
            params.extend_from_slice(&v.to_le_bytes());
        }
        let nodes_bytes = pack_f32(nodes);
        let tris_bytes = pack_f32(triangles);
        let mut seg_bytes = Vec::with_capacity(segments.len() * 32);
        for (a, d, length2, r) in segments {
            for v in [*a, *d].concat() {
                seg_bytes.extend_from_slice(&(v as f32).to_le_bytes());
            }
            seg_bytes.extend_from_slice(&(*length2 as f32).to_le_bytes());
            seg_bytes.extend_from_slice(&(*r as f32).to_le_bytes());
        }
        self.ensure_buffers(nodes.len(), triangles.len(), segments.len(), total as usize);
        let buffers = self.buffers.borrow();
        let b = buffers.as_ref().expect("ensure_buffers was just called");
        self.queue.write_buffer(&b.params, 0, &params);
        if !nodes_bytes.is_empty() {
            self.queue.write_buffer(&b.nodes, 0, &nodes_bytes);
        }
        if !tris_bytes.is_empty() {
            self.queue.write_buffer(&b.triangles, 0, &tris_bytes);
        }
        if !seg_bytes.is_empty() {
            self.queue.write_buffer(&b.segments, 0, &seg_bytes);
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
    static SHARED: std::cell::LazyCell<Option<&'static (GpuContext, GpuLattice)>> =
        std::cell::LazyCell::new(|| GpuContext::new().map(|c| {
            let lattice = GpuLattice::new(&c);
            Box::leak(Box::new((c, lattice))) as &'static (GpuContext, GpuLattice)
        }));
}

/// Samples the lattice field on the GPU and extracts the mesh on the CPU.
/// Points the shader marks NaN (stack/overflow guards) are recomputed exactly
/// by the CPU field closure. None without an adapter.
#[allow(clippy::too_many_arguments)]
pub(crate) fn try_gpu(
    all: &Node,
    segments: &Segments,
    min: P,
    max: P,
    cells: [usize; 3],
    skin: f64,
    organic: bool,
    open_top: bool,
    wall_depth: f64,
    keep_core: bool,
    blend: f64,
    field: &impl Fn(P) -> f64,
) -> Result<Option<polygon_core::Mesh>, polygon_core::Error> {
    SHARED.with(|cell| {
        let shared: &Option<&(GpuContext, GpuLattice)> = cell;
        shared
            .map(|(_, lattice)| {
                let (nodes, triangles) = flatten_lattice_bvh(all);
                let mut values = lattice.run(
                    &nodes, &triangles, segments, min, max, cells, skin, blend, organic, open_top,
                    wall_depth, keep_core, all.max[2],
                );
                // CPU recompute of shader-marked points.
                let delta: P = std::array::from_fn(|k| (max[k] - min[k]) / cells[k] as f64);
                let mut index = 0usize;
                for z in 0..=cells[2] {
                    for y in 0..=cells[1] {
                        for x in 0..=cells[0] {
                            if !values[index].is_finite() {
                                let p: P = std::array::from_fn(|k| {
                                    min[k] + delta[k] * [x, y, z][k] as f64
                                });
                                values[index] = field(p) as f32;
                            }
                            index += 1;
                        }
                    }
                }
                let cursor = std::cell::Cell::new(0usize);
                sdf_core::polygonize_with(
                    |_| {
                        let i = cursor.get();
                        cursor.set(i + 1);
                        values[i] as f64
                    },
                    &sdf_core::Grid { min, max, cells },
                )
                .map(crate::mesh_from_triangles)
                .map(Some)
            })
            .transpose()
            .map(Option::flatten)
    })
}

/// Portable wgpu backend currently used for lattice field sampling.
pub fn backend_label() -> Option<&'static str> {
    backend_report().map(|report| report.label)
}

pub fn backend_report() -> Option<BackendReport> {
    SHARED.with(|cell| {
        let shared: &Option<&(GpuContext, GpuLattice)> = cell;
        shared.map(|(context, _)| context.backend_report())
    })
}

/// Test hook: whether a GPU adapter is available (the accelerated path then
/// actually engages instead of falling back).
#[cfg(test)]
pub(crate) fn try_gpu_available() -> bool {
    SHARED.with(|cell| {
        let shared: &Option<&(GpuContext, GpuLattice)> = cell;
        shared.is_some()
    })
}
