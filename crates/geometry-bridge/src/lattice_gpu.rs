//! GPU evaluation of the lattice implicit field (feature `gpu`). The BVH is
//! flattened to plain arrays; each grid point runs `LATTICE_WGSL` on the GPU
//! in f32. Points whose ray-parity walk overflows write NaN and are recomputed
//! by the CPU field closure. Extraction stays on the CPU reference path.
use crate::mesh_shell::{LATTICE_WGSL, Node, P};
use gpu_compute::{GpuContext, read_buffer, storage_entry, uniform_entry, wgpu};
use wgpu::util::DeviceExt;

type Segments = [(P, P, f64, f64)];

struct GpuLattice {
    device: wgpu::Device,
    queue: wgpu::Queue,
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::ComputePipeline,
}

/// Preorder flattening of the recursive BVH: min/max (6 f32), left/right
/// (i32, -1 for leaves), triangle window (start, count as u32 bits).
fn flatten_bvh(root: &Node) -> (Vec<f32>, Vec<f32>) {
    let mut nodes: Vec<f32> = Vec::new();
    let mut triangles: Vec<f32> = Vec::new();
    fn emit(node: &Node, nodes: &mut Vec<f32>, triangles: &mut Vec<f32>) -> u32 {
        let index = (nodes.len() / 10) as u32;
        nodes.resize(nodes.len() + 10, 0.);
        for k in 0..3 {
            nodes[index as usize * 10 + k] = node.min[k] as f32;
            nodes[index as usize * 10 + 3 + k] = node.max[k] as f32;
        }
        if let Some(children) = &node.children {
            let left = emit(&children[0], nodes, triangles);
            let right = emit(&children[1], nodes, triangles);
            nodes[index as usize * 10 + 6] = f32::from_bits(left);
            nodes[index as usize * 10 + 7] = f32::from_bits(right);
        } else {
            nodes[index as usize * 10 + 6] = f32::from_bits(u32::MAX);
            nodes[index as usize * 10 + 7] = f32::from_bits(u32::MAX);
            let start = (triangles.len() / 9) as u32;
            for t in &node.triangles {
                for point in t.p {
                    for k in 0..3 {
                        triangles.push(point[k] as f32);
                    }
                }
            }
            nodes[index as usize * 10 + 8] = f32::from_bits(start);
            nodes[index as usize * 10 + 9] = f32::from_bits(triangles.len() as u32 / 9 - start);
        }
        index
    }
    emit(root, &mut nodes, &mut triangles);
    (nodes, triangles)
}

impl GpuLattice {
    fn new(context: &GpuContext) -> Self {
        let device = &context.device;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("lattice"),
            source: wgpu::ShaderSource::Wgsl(LATTICE_WGSL.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("lattice"),
            entries: &[
                uniform_entry(0),
                storage_entry(1, true),
                storage_entry(2, true),
                storage_entry(3, true),
                storage_entry(4, false),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("lattice"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("lattice"),
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
        }
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
        let total = (cells[0] + 1) * (cells[1] + 1) * (cells[2] + 1);
        let device = &self.device;
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
            params.extend_from_slice(&v.to_ne_bytes());
        }
        for i in 0..3 {
            params.extend_from_slice(&(min[i] as f32).to_ne_bytes());
        }
        for i in 0..3 {
            // The shader maps grid indices by p = min + i * step.
            params.extend_from_slice(&(((max[i] - min[i]) / cells[i] as f64) as f32).to_ne_bytes());
        }
        for v in [
            skin as f32,
            blend as f32,
            wall_depth as f32,
            top_z as f32,
            0.,
            0.,
        ] {
            params.extend_from_slice(&v.to_ne_bytes());
        }
        let mk = |label: &str, bytes: &[u8], usage| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: if bytes.is_empty() { &[0u8; 4] } else { bytes },
                usage,
            })
        };
        let nodes_bytes = gpu_compute::pack_f32(nodes);
        let tris_bytes = gpu_compute::pack_f32(triangles);
        let mut seg_bytes = Vec::with_capacity(segments.len() * 32);
        for (a, d, length2, r) in segments {
            for v in [*a, *d].concat() {
                seg_bytes.extend_from_slice(&(v as f32).to_ne_bytes());
            }
            seg_bytes.extend_from_slice(&(*length2 as f32).to_ne_bytes());
            seg_bytes.extend_from_slice(&(*r as f32).to_ne_bytes());
        }
        let params_buf = mk("params", &params, wgpu::BufferUsages::UNIFORM);
        let nodes_buf = mk("nodes", &nodes_bytes, wgpu::BufferUsages::STORAGE);
        let tris_buf = mk("tris", &tris_bytes, wgpu::BufferUsages::STORAGE);
        let seg_buf = mk("segments", &seg_bytes, wgpu::BufferUsages::STORAGE);
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
            label: Some("lattice"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: nodes_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: tris_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: seg_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: out_buf.as_entire_binding(),
                },
            ],
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("lattice"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("lattice"),
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
                let (nodes, triangles) = flatten_bvh(all);
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
                .map_err(|e| polygon_core::Error {
                    code: e.code,
                    message: e.message,
                })
                .map(crate::mesh_from_triangles)
                .map(Some)
            })
            .transpose()
            .map(Option::flatten)
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
