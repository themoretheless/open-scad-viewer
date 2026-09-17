//! CUDA lattice field sampler (feature `cuda`): native PTX port of
//! `LATTICE_WGSL`. Mesh extraction and shader-marked NaN repair remain on CPU.
use crate::mesh_shell::{Node, P, flatten_lattice_bvh};
use gpu_compute::cuda::{CudaDevice, CudaFunction, CudaSlice, PushKernelArg, launch_1d};

type Segments = [(P, P, f64, f64)];

/// PTX generated from `lattice.cu` by `scripts/build-cuda-kernels.mjs`.
pub const LATTICE_PTX: &str = include_str!("lattice.ptx");
const BLOCK: u32 = 256;

struct CudaLattice {
    device: CudaDevice,
    kernel: CudaFunction,
    buffers: std::cell::RefCell<Option<Buffers>>,
}

struct Buffers {
    node_capacity: usize,
    triangle_capacity: usize,
    segment_capacity: usize,
    value_capacity: usize,
    nodes: CudaSlice<f32>,
    triangles: CudaSlice<f32>,
    segments: CudaSlice<f32>,
    values: CudaSlice<f32>,
}

impl CudaLattice {
    fn new() -> Option<Self> {
        let device = CudaDevice::new()?;
        let module = device.load_ptx(LATTICE_PTX)?;
        let kernel = module.load_function("lattice_sample").ok()?;
        Some(Self {
            device,
            kernel,
            buffers: std::cell::RefCell::new(None),
        })
    }

    fn ensure_buffers(
        &self,
        node_values: usize,
        triangle_values: usize,
        segment_values: usize,
        value_count: usize,
    ) -> Option<()> {
        let stale = match &*self.buffers.borrow() {
            Some(b) => {
                b.node_capacity < node_values
                    || b.triangle_capacity < triangle_values
                    || b.segment_capacity < segment_values
                    || b.value_capacity < value_count
            }
            None => true,
        };
        if !stale {
            return Some(());
        }
        let stream = &self.device.stream;
        let node_capacity = node_values.max(1);
        let triangle_capacity = triangle_values.max(1);
        let segment_capacity = segment_values.max(1);
        let value_capacity = value_count.max(1);
        let nodes = stream.alloc_zeros::<f32>(node_capacity).ok()?;
        let triangles = stream.alloc_zeros::<f32>(triangle_capacity).ok()?;
        let segments = stream.alloc_zeros::<f32>(segment_capacity).ok()?;
        let values = stream.alloc_zeros::<f32>(value_capacity).ok()?;
        *self.buffers.borrow_mut() = Some(Buffers {
            node_capacity,
            triangle_capacity,
            segment_capacity,
            value_capacity,
            nodes,
            triangles,
            segments,
            values,
        });
        Some(())
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
    ) -> Option<Vec<f32>> {
        let total = (cells[0] + 1) * (cells[1] + 1) * (cells[2] + 1);
        let mut segment_values = Vec::with_capacity(segments.len() * 8);
        for (a, d, length2, radius) in segments {
            segment_values.extend(a.iter().map(|&v| v as f32));
            segment_values.extend(d.iter().map(|&v| v as f32));
            segment_values.push(*length2 as f32);
            segment_values.push(*radius as f32);
        }
        self.ensure_buffers(nodes.len(), triangles.len(), segment_values.len(), total)?;
        let stream = &self.device.stream;
        let mut buffers = self.buffers.borrow_mut();
        let b = buffers.as_mut().expect("ensure_buffers was just called");
        if !nodes.is_empty() {
            let mut view = b.nodes.slice_mut(0..nodes.len());
            stream.memcpy_htod(nodes, &mut view).ok()?;
        }
        if !triangles.is_empty() {
            let mut view = b.triangles.slice_mut(0..triangles.len());
            stream.memcpy_htod(triangles, &mut view).ok()?;
        }
        if !segment_values.is_empty() {
            let mut view = b.segments.slice_mut(0..segment_values.len());
            stream.memcpy_htod(&segment_values, &mut view).ok()?;
        }
        let nodes_view = b.nodes.slice(0..nodes.len().max(1));
        let triangles_view = b.triangles.slice(0..triangles.len().max(1));
        let segments_view = b.segments.slice(0..segment_values.len().max(1));
        let mut values = b.values.slice_mut(0..total.max(1));
        let nx = cells[0] as u32;
        let ny = cells[1] as u32;
        let nz = cells[2] as u32;
        let n_segments = segments.len() as u32;
        let n_nodes = (nodes.len() / 10) as u32;
        let organic_u32 = organic as u32;
        let open_top_u32 = open_top as u32;
        let keep_core_u32 = keep_core as u32;
        let min_f32: [f32; 3] = std::array::from_fn(|i| min[i] as f32);
        let step: [f32; 3] = std::array::from_fn(|i| ((max[i] - min[i]) / cells[i] as f64) as f32);
        let skin = skin as f32;
        let blend = blend as f32;
        let wall_depth = wall_depth as f32;
        let top_z = top_z as f32;
        let mut launch = stream.launch_builder(&self.kernel);
        launch
            .arg(&nx)
            .arg(&ny)
            .arg(&nz)
            .arg(&n_segments)
            .arg(&n_nodes)
            .arg(&organic_u32)
            .arg(&open_top_u32)
            .arg(&keep_core_u32)
            .arg(&min_f32[0])
            .arg(&min_f32[1])
            .arg(&min_f32[2])
            .arg(&step[0])
            .arg(&step[1])
            .arg(&step[2])
            .arg(&skin)
            .arg(&blend)
            .arg(&wall_depth)
            .arg(&top_z)
            .arg(&nodes_view)
            .arg(&triangles_view)
            .arg(&segments_view)
            .arg(&mut values);
        unsafe { launch.launch(launch_1d(total as u32, BLOCK)) }.ok()?;
        let mut out = vec![0f32; total];
        if total > 0 {
            stream.memcpy_dtoh(&values, &mut out).ok()?;
        }
        Some(out)
    }
}

thread_local! {
    static SHARED: std::cell::LazyCell<Option<&'static CudaLattice>> =
        std::cell::LazyCell::new(|| {
            CudaLattice::new().map(|lattice| Box::leak(Box::new(lattice)) as &'static CudaLattice)
        });
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn try_cuda(
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
        let shared: &Option<&CudaLattice> = cell;
        shared
            .map(|lattice| {
                let (nodes, triangles) = flatten_lattice_bvh(all);
                let mut values = lattice.run(
                    &nodes, &triangles, segments, min, max, cells, skin, blend, organic, open_top,
                    wall_depth, keep_core, all.max[2],
                );
                let values = match &mut values {
                    Some(values) => values,
                    None => return Ok(None),
                };
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

#[cfg(test)]
pub(crate) fn try_cuda_available() -> bool {
    SHARED.with(|cell| {
        let shared: &Option<&CudaLattice> = cell;
        shared.is_some()
    })
}
