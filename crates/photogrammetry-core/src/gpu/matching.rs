//! GPU descriptor matching (feature `gpu`). One thread per candidate pair
//! computes the full 128-component squared distance; workgroup reductions keep
//! the CPU scan's ordering semantics: smaller distance wins, equal distances
//! keep the first index (the CPU scan uses strict `<`). NaN distances never
//! win, as on the CPU. GPU floating-point contraction (fma) means results are
//! NOT bit-identical to the CPU path — this mode is opt-in and qualified
//! separately; CPU remains the deterministic reference.

use super::wgpu;

use super::GpuContext;
use compute_core::{Binding, Kernel};

/// Metal (Apple GPUs) tunes toward a smaller, SIMD-group-aligned workgroup to
/// keep more threadgroups resident; every other backend — including Vulkan on
/// NVIDIA/"CUDA-class" hardware — keeps the larger default that hides memory
/// latency with more warps in flight. See `gpu_compute::tuned_workgroup_size`.
const WG_METAL: u32 = 128;
const WG_DEFAULT: u32 = 256;

// `WG` anchor convention: the compute-core runtime substitutes a per-backend
// tuned power of two (see `Kernel::with_workgroup_size`) before compilation.
const SHADER_TEMPLATE: &str = r#"
struct Params {
    rows: u32,
    cols: u32,
}
struct RowBest {
    j: u32,
    d1: f32,
    d2: f32,
}
struct ColBest {
    i: u32,
    d1: f32,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> da: array<f32>;
@group(0) @binding(2) var<storage, read> db: array<f32>;
@group(0) @binding(3) var<storage, read_write> rows_out: array<RowBest>;
@group(0) @binding(4) var<storage, read_write> cols_out: array<ColBest>;

const NONE: u32 = 0xFFFFFFFFu;
const INF: f32 = 3.402823466e+38;
const WG: u32 = 256;

var<workgroup> sh_d: array<f32, WG>;
var<workgroup> sh_i: array<u32, WG>;
var<workgroup> sh_s: array<f32, WG>;

// True when (a_d, a_i) outranks (b_d, b_i): smaller distance, then smaller index.
fn better(a_d: f32, a_i: u32, b_d: f32, b_i: u32) -> bool {
    return a_d < b_d || (a_d == b_d && a_i < b_i);
}

fn dist(row: u32, col: u32) -> f32 {
    var d = 0.0;
    let ra = row * 128u;
    let rb = col * 128u;
    for (var k = 0u; k < 128u; k++) {
        let v = da[ra + k] - db[rb + k];
        d += v * v;
    }
    return d;
}

@compute @workgroup_size(WG)
fn match_rows(@builtin(workgroup_id) wg: vec3<u32>, @builtin(local_invocation_id) lid: vec3<u32>) {
    let row = wg.x;
    var best_d = INF;
    var best_j = NONE;
    var second_d = INF;
    var col = lid.x;
    while (col < params.cols) {
        let d = dist(row, col);
        if (d < best_d) {
            second_d = best_d;
            best_d = d;
            best_j = col;
        } else if (d < second_d) {
            second_d = d;
        }
        col += WG;
    }
    sh_d[lid.x] = best_d;
    sh_i[lid.x] = best_j;
    sh_s[lid.x] = second_d;
    workgroupBarrier();
    var stride = WG / 2u;
    while (stride > 0u) {
        if (lid.x < stride) {
            let other = lid.x + stride;
            if (better(sh_d[other], sh_i[other], sh_d[lid.x], sh_i[lid.x])) {
                // other becomes best; old best competes for second
                sh_s[lid.x] = min(min(sh_s[lid.x], sh_s[other]), sh_d[lid.x]);
                sh_d[lid.x] = sh_d[other];
                sh_i[lid.x] = sh_i[other];
            } else {
                sh_s[lid.x] = min(min(sh_s[lid.x], sh_s[other]), sh_d[other]);
            }
        }
        workgroupBarrier();
        stride = stride / 2u;
    }
    if (lid.x == 0u) {
        rows_out[row] = RowBest(sh_i[0], sh_d[0], sh_s[0]);
    }
}

@compute @workgroup_size(WG)
fn match_cols(@builtin(workgroup_id) wg: vec3<u32>, @builtin(local_invocation_id) lid: vec3<u32>) {
    let col = wg.x;
    var best_d = INF;
    var best_i = NONE;
    var row = lid.x;
    while (row < params.rows) {
        let d = dist(row, col);
        if (d < best_d) {
            best_d = d;
            best_i = row;
        }
        row += WG;
    }
    sh_d[lid.x] = best_d;
    sh_i[lid.x] = best_i;
    workgroupBarrier();
    var stride = WG / 2u;
    while (stride > 0u) {
        if (lid.x < stride) {
            let other = lid.x + stride;
            if (better(sh_d[other], sh_i[other], sh_d[lid.x], sh_i[lid.x])) {
                sh_d[lid.x] = sh_d[other];
                sh_i[lid.x] = sh_i[other];
            }
        }
        workgroupBarrier();
        stride = stride / 2u;
    }
    if (lid.x == 0u) {
        cols_out[col] = ColBest(sh_i[0], sh_d[0]);
    }
}
"#;

#[derive(Clone, Copy, Debug)]
pub struct RowBest {
    pub j: usize,
    pub d1: f32,
    pub d2: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct ColBest {
    pub i: usize,
    pub d1: f32,
}

struct Buffers {
    row_capacity: usize,
    col_capacity: usize,
    params: wgpu::Buffer,
    descriptors_a: wgpu::Buffer,
    descriptors_b: wgpu::Buffer,
    rows_out: wgpu::Buffer,
    cols_out: wgpu::Buffer,
    read_rows: wgpu::Buffer,
    read_cols: wgpu::Buffer,
    bind: wgpu::BindGroup,
}

pub struct GpuMatcher {
    device: wgpu::Device,
    queue: wgpu::Queue,
    rows_kernel: Kernel,
    cols_kernel: Kernel,
    /// The workgroup size baked into the compiled pipelines, chosen per
    /// backend by `gpu_compute::tuned_workgroup_size`. Exposed for tests
    /// (hence `allow(dead_code)` on non-test builds).
    #[allow(dead_code)]
    workgroup_size: u32,
    buffers: std::cell::RefCell<Option<Buffers>>,
}

impl GpuMatcher {
    pub fn new(context: &GpuContext) -> Self {
        let workgroup_size =
            gpu_compute::tuned_workgroup_size(context.backend, WG_METAL, WG_DEFAULT);
        Self::with_workgroup_size(context, workgroup_size)
    }

    /// Builds the pipelines for an explicit workgroup size, bypassing backend
    /// auto-detection. `new` is the production entry point; this is what lets
    /// tests exercise both the Metal-tuned and default-tuned kernel variants
    /// on whatever adapter the test machine actually has.
    fn with_workgroup_size(context: &GpuContext, workgroup_size: u32) -> Self {
        let bindings = [
            Binding::Uniform,
            Binding::StorageRead,
            Binding::StorageRead,
            Binding::StorageReadWrite,
            Binding::StorageReadWrite,
        ];
        let rows_kernel = Kernel::with_workgroup_size(
            &context.device,
            "match_rows",
            SHADER_TEMPLATE,
            "match_rows",
            &bindings,
            workgroup_size,
        )
        .expect("match_rows kernel builds");
        let cols_kernel = Kernel::with_workgroup_size(
            &context.device,
            "match_cols",
            SHADER_TEMPLATE,
            "match_cols",
            &bindings,
            workgroup_size,
        )
        .expect("match_cols kernel builds");
        Self {
            device: context.device.clone(),
            queue: context.queue.clone(),
            rows_kernel,
            cols_kernel,
            workgroup_size,
            buffers: std::cell::RefCell::new(None),
        }
    }

    #[cfg(test)]
    fn workgroup_size(&self) -> u32 {
        self.workgroup_size
    }

    /// Nearest/second-nearest per row and nearest per column over the full
    /// descriptor grid. Mirrors the CPU scan's tie-breaking; float contraction
    /// may shift borderline distances, so GPU results are not bit-identical.
    pub fn match_descriptors(
        &self,
        da: &[[f32; 128]],
        db: &[[f32; 128]],
    ) -> (Vec<RowBest>, Vec<ColBest>) {
        let rows = da.len() as u32;
        let cols = db.len() as u32;
        let packed_a = pack_descriptors(da);
        let packed_b = pack_descriptors(db);
        self.ensure_buffers(da.len(), db.len());
        let row_bytes = (rows as u64) * 12;
        let col_bytes = (cols as u64) * 8;
        let buffers = self.buffers.borrow();
        let buffers = buffers.as_ref().expect("ensure_buffers was just called");
        self.queue.write_buffer(
            &buffers.params,
            0,
            &[rows.to_ne_bytes(), cols.to_ne_bytes()].concat(),
        );
        if !packed_a.is_empty() {
            self.queue
                .write_buffer(&buffers.descriptors_a, 0, &packed_a);
        }
        if !packed_b.is_empty() {
            self.queue
                .write_buffer(&buffers.descriptors_b, 0, &packed_b);
        }

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("match"),
            });
        if rows > 0 && cols > 0 {
            self.rows_kernel
                .record_dispatch(&mut encoder, &buffers.bind, rows);
            self.cols_kernel
                .record_dispatch(&mut encoder, &buffers.bind, cols);
        }
        encoder.copy_buffer_to_buffer(
            &buffers.rows_out,
            0,
            &buffers.read_rows,
            0,
            row_bytes.max(12),
        );
        encoder.copy_buffer_to_buffer(
            &buffers.cols_out,
            0,
            &buffers.read_cols,
            0,
            col_bytes.max(8),
        );
        self.queue.submit([encoder.finish()]);

        let rows_raw = super::read_buffer(&self.device, &buffers.read_rows, row_bytes as usize);
        buffers.read_rows.unmap();
        let cols_raw = super::read_buffer(&self.device, &buffers.read_cols, col_bytes as usize);
        buffers.read_cols.unmap();

        let rows_out_vec = rows_raw
            .chunks_exact(12)
            .map(|c| RowBest {
                j: match u32::from_ne_bytes(c[0..4].try_into().unwrap()) {
                    u32::MAX => usize::MAX,
                    raw => raw as usize,
                },
                d1: f32::from_ne_bytes(c[4..8].try_into().unwrap()),
                d2: f32::from_ne_bytes(c[8..12].try_into().unwrap()),
            })
            .collect();
        let cols_out_vec = cols_raw
            .chunks_exact(8)
            .map(|c| ColBest {
                i: match u32::from_ne_bytes(c[0..4].try_into().unwrap()) {
                    u32::MAX => usize::MAX,
                    raw => raw as usize,
                },
                d1: f32::from_ne_bytes(c[4..8].try_into().unwrap()),
            })
            .collect();
        (rows_out_vec, cols_out_vec)
    }

    fn ensure_buffers(&self, row_count: usize, col_count: usize) {
        let stale = match &*self.buffers.borrow() {
            Some(buffers) => buffers.row_capacity < row_count || buffers.col_capacity < col_count,
            None => true,
        };
        if !stale {
            return;
        }
        let device = &self.device;
        let row_capacity = row_count.max(1);
        let col_capacity = col_count.max(1);
        let params = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("match_params"),
            size: 8,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let descriptors_a = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("match_da"),
            size: (row_capacity * 128 * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let descriptors_b = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("match_db"),
            size: (col_capacity * 128 * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let rows_out = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("match_rows_out"),
            size: (row_capacity * 12) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let cols_out = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("match_cols_out"),
            size: (col_capacity * 8) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let read_rows = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("match_read_rows"),
            size: (row_capacity * 12) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let read_cols = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("match_read_cols"),
            size: (col_capacity * 8) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("match_descriptors"),
            layout: self.rows_kernel.bind_group_layout(),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: descriptors_a.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: descriptors_b.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: rows_out.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: cols_out.as_entire_binding(),
                },
            ],
        });
        *self.buffers.borrow_mut() = Some(Buffers {
            row_capacity,
            col_capacity,
            params,
            descriptors_a,
            descriptors_b,
            rows_out,
            cols_out,
            read_rows,
            read_cols,
            bind,
        });
    }
}

fn pack_descriptors(data: &[[f32; 128]]) -> Vec<u8> {
    let flat: Vec<f32> = data.iter().flatten().copied().collect();
    super::pack_f32(&flat)
}

thread_local! {
    // The device and pipelines are process-lifetime resources; leaking avoids
    // dropping wgpu objects inside a thread-local destructor.
    static SHARED: std::cell::LazyCell<Option<&'static (GpuContext, GpuMatcher)>> =
        std::cell::LazyCell::new(|| GpuContext::new().map(|c| {
            let matcher = GpuMatcher::new(&c);
            Box::leak(Box::new((c, matcher))) as &'static (GpuContext, GpuMatcher)
        }));
}

/// Matches one descriptor pair-set through a thread-shared GPU context.
/// `None` when no GPU adapter is available; callers fall back to the CPU scan.
pub fn match_pair(da: &[[f32; 128]], db: &[[f32; 128]]) -> Option<(Vec<RowBest>, Vec<ColBest>)> {
    SHARED.with(|cell| {
        let shared: &Option<&(GpuContext, GpuMatcher)> = cell;
        shared.map(|(_, matcher)| matcher.match_descriptors(da, db))
    })
}

pub fn match_pair_accelerated(
    da: &[[f32; 128]],
    db: &[[f32; 128]],
    acceleration: crate::Acceleration,
) -> Option<(Vec<RowBest>, Vec<ColBest>)> {
    #[cfg(feature = "cuda")]
    if acceleration == crate::Acceleration::Cuda
        && let Some(result) = cuda::match_pair_cuda(da, db)
    {
        return Some(result);
    }
    match_pair(da, db)
}

#[cfg(feature = "cuda")]
mod cuda {
    use super::{ColBest, RowBest};
    use gpu_compute::cuda::{CudaDevice, CudaFunction, CudaSlice, LaunchConfig, PushKernelArg};

    const MATCHING_PTX: &str = include_str!("matching.ptx");
    const BLOCK: u32 = 256;

    struct Buffers {
        row_capacity: usize,
        col_capacity: usize,
        descriptors_a: CudaSlice<f32>,
        descriptors_b: CudaSlice<f32>,
        row_j: CudaSlice<u32>,
        row_d1: CudaSlice<f32>,
        row_d2: CudaSlice<f32>,
        col_i: CudaSlice<u32>,
        col_d1: CudaSlice<f32>,
    }

    struct CudaMatcher {
        device: CudaDevice,
        rows_kernel: CudaFunction,
        cols_kernel: CudaFunction,
        buffers: std::cell::RefCell<Option<Buffers>>,
    }

    impl CudaMatcher {
        fn new() -> Option<Self> {
            let device = CudaDevice::new()?;
            let module = device.load_ptx(MATCHING_PTX)?;
            let rows_kernel = module.load_function("match_descriptor_rows").ok()?;
            let cols_kernel = module.load_function("match_descriptor_cols").ok()?;
            Some(Self {
                device,
                rows_kernel,
                cols_kernel,
                buffers: std::cell::RefCell::new(None),
            })
        }

        fn ensure_buffers(&self, rows: usize, cols: usize) -> Option<()> {
            let stale = match &*self.buffers.borrow() {
                Some(b) => b.row_capacity < rows || b.col_capacity < cols,
                None => true,
            };
            if !stale {
                return Some(());
            }
            let row_capacity = rows.max(1);
            let col_capacity = cols.max(1);
            let stream = &self.device.stream;
            let descriptors_a = stream.alloc_zeros::<f32>(row_capacity * 128).ok()?;
            let descriptors_b = stream.alloc_zeros::<f32>(col_capacity * 128).ok()?;
            let row_j = stream.alloc_zeros::<u32>(row_capacity).ok()?;
            let row_d1 = stream.alloc_zeros::<f32>(row_capacity).ok()?;
            let row_d2 = stream.alloc_zeros::<f32>(row_capacity).ok()?;
            let col_i = stream.alloc_zeros::<u32>(col_capacity).ok()?;
            let col_d1 = stream.alloc_zeros::<f32>(col_capacity).ok()?;
            *self.buffers.borrow_mut() = Some(Buffers {
                row_capacity,
                col_capacity,
                descriptors_a,
                descriptors_b,
                row_j,
                row_d1,
                row_d2,
                col_i,
                col_d1,
            });
            Some(())
        }

        fn run(
            &self,
            da: &[[f32; 128]],
            db: &[[f32; 128]],
        ) -> Option<(Vec<RowBest>, Vec<ColBest>)> {
            let rows = da.len();
            let cols = db.len();
            if rows == 0 || cols == 0 {
                return Some((
                    vec![
                        RowBest {
                            j: usize::MAX,
                            d1: f32::INFINITY,
                            d2: f32::INFINITY,
                        };
                        rows
                    ],
                    vec![
                        ColBest {
                            i: usize::MAX,
                            d1: f32::INFINITY,
                        };
                        cols
                    ],
                ));
            }
            self.ensure_buffers(rows, cols)?;
            let stream = &self.device.stream;
            let mut buffers = self.buffers.borrow_mut();
            let b = buffers.as_mut().expect("ensure_buffers was just called");
            let flat_a: Vec<f32> = da.iter().flatten().copied().collect();
            let flat_b: Vec<f32> = db.iter().flatten().copied().collect();
            {
                let mut view = b.descriptors_a.slice_mut(0..flat_a.len());
                stream.memcpy_htod(&flat_a, &mut view).ok()?;
            }
            {
                let mut view = b.descriptors_b.slice_mut(0..flat_b.len());
                stream.memcpy_htod(&flat_b, &mut view).ok()?;
            }
            let rows_u32 = rows as u32;
            let cols_u32 = cols as u32;
            let a_view = b.descriptors_a.slice(0..flat_a.len().max(1));
            let b_view = b.descriptors_b.slice(0..flat_b.len().max(1));
            let mut row_j_view = b.row_j.slice_mut(0..rows.max(1));
            let mut row_d1_view = b.row_d1.slice_mut(0..rows.max(1));
            let mut row_d2_view = b.row_d2.slice_mut(0..rows.max(1));
            let mut row_launch = stream.launch_builder(&self.rows_kernel);
            row_launch
                .arg(&rows_u32)
                .arg(&cols_u32)
                .arg(&a_view)
                .arg(&b_view)
                .arg(&mut row_j_view)
                .arg(&mut row_d1_view)
                .arg(&mut row_d2_view);
            unsafe { row_launch.launch(launch_1d_blocks(rows_u32)) }.ok()?;
            let mut col_i_view = b.col_i.slice_mut(0..cols.max(1));
            let mut col_d1_view = b.col_d1.slice_mut(0..cols.max(1));
            let mut col_launch = stream.launch_builder(&self.cols_kernel);
            col_launch
                .arg(&rows_u32)
                .arg(&cols_u32)
                .arg(&a_view)
                .arg(&b_view)
                .arg(&mut col_i_view)
                .arg(&mut col_d1_view);
            unsafe { col_launch.launch(launch_1d_blocks(cols_u32)) }.ok()?;

            let mut row_j = vec![0u32; rows];
            let mut row_d1 = vec![0f32; rows];
            let mut row_d2 = vec![0f32; rows];
            let mut col_i = vec![0u32; cols];
            let mut col_d1 = vec![0f32; cols];
            stream.memcpy_dtoh(&row_j_view, &mut row_j).ok()?;
            stream.memcpy_dtoh(&row_d1_view, &mut row_d1).ok()?;
            stream.memcpy_dtoh(&row_d2_view, &mut row_d2).ok()?;
            stream.memcpy_dtoh(&col_i_view, &mut col_i).ok()?;
            stream.memcpy_dtoh(&col_d1_view, &mut col_d1).ok()?;

            Some((
                (0..rows)
                    .map(|i| RowBest {
                        j: if row_j[i] == u32::MAX {
                            usize::MAX
                        } else {
                            row_j[i] as usize
                        },
                        d1: row_d1[i],
                        d2: row_d2[i],
                    })
                    .collect(),
                (0..cols)
                    .map(|i| ColBest {
                        i: if col_i[i] == u32::MAX {
                            usize::MAX
                        } else {
                            col_i[i] as usize
                        },
                        d1: col_d1[i],
                    })
                    .collect(),
            ))
        }
    }

    fn launch_1d_blocks(blocks: u32) -> LaunchConfig {
        LaunchConfig {
            grid_dim: (blocks.max(1), 1, 1),
            block_dim: (BLOCK, 1, 1),
            shared_mem_bytes: 0,
        }
    }

    thread_local! {
        static SHARED: std::cell::LazyCell<Option<&'static CudaMatcher>> =
            std::cell::LazyCell::new(|| {
                CudaMatcher::new().map(|matcher| Box::leak(Box::new(matcher)) as &'static CudaMatcher)
            });
    }

    pub(super) fn match_pair_cuda(
        da: &[[f32; 128]],
        db: &[[f32; 128]],
    ) -> Option<(Vec<RowBest>, Vec<ColBest>)> {
        SHARED.with(|cell| {
            let shared: &Option<&CudaMatcher> = cell;
            shared.and_then(|matcher| matcher.run(da, db))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn descriptors(seed: u64, n: usize) -> Vec<[f32; 128]> {
        let mut state = seed;
        (0..n)
            .map(|_| {
                let mut d = [0f32; 128];
                for v in &mut d {
                    state = state
                        .wrapping_mul(6364136223846793005)
                        .wrapping_add(1442695040888963407);
                    *v = ((state >> 33) as f32 / 2147483648.0).fract();
                }
                d
            })
            .collect()
    }

    fn cpu_reference(
        da: &[[f32; 128]],
        db: &[[f32; 128]],
    ) -> (Vec<(usize, f32, f32)>, Vec<(usize, f32)>) {
        let mut best_a = vec![(usize::MAX, f32::INFINITY, f32::INFINITY); da.len()];
        let mut best_b = vec![(usize::MAX, f32::INFINITY); db.len()];
        for (i, x) in da.iter().enumerate() {
            for (j, y) in db.iter().enumerate() {
                let mut d = 0f32;
                for k in 0..128 {
                    let v = x[k] - y[k];
                    d += v * v;
                }
                if d < best_a[i].1 {
                    best_a[i].2 = best_a[i].1;
                    best_a[i].1 = d;
                    best_a[i].0 = j;
                } else if d < best_a[i].2 {
                    best_a[i].2 = d;
                }
                if d < best_b[j].1 {
                    best_b[j] = (i, d);
                }
            }
        }
        (best_a, best_b)
    }

    #[test]
    fn gpu_matching_agrees_with_cpu_on_synthetic_descriptors() {
        let Some((da, db)) = match_pair(&descriptors(7, 300), &descriptors(11, 280))
            .map(|_| (descriptors(7, 300), descriptors(11, 280)))
        else {
            eprintln!("no GPU adapter; skipping");
            return;
        };
        let (rows, cols) = match_pair(&da, &db).expect("adapter was available");
        let (ref_a, ref_b) = cpu_reference(&da, &db);
        let mut row_exact = 0;
        let mut row_close = 0;
        for (gpu, cpu) in rows.iter().zip(&ref_a) {
            if gpu.j == cpu.0
                && gpu.d1.to_bits() == cpu.1.to_bits()
                && gpu.d2.to_bits() == cpu.2.to_bits()
            {
                row_exact += 1;
            }
            // GPU float contraction may shift borderline distances; the selected
            // indices must agree and distances must stay within f32 contraction noise.
            if gpu.j == cpu.0 && (gpu.d1 - cpu.1).abs() <= 1e-4 * cpu.1.max(1.) {
                row_close += 1;
            }
        }
        let col_exact = cols
            .iter()
            .zip(&ref_b)
            .filter(|(gpu, cpu)| gpu.i == cpu.0 && (gpu.d1 - cpu.1).abs() <= 1e-4 * cpu.1.max(1.))
            .count();
        assert_eq!(row_close, ref_a.len(), "row selections must agree with CPU");
        assert_eq!(
            col_exact,
            ref_b.len(),
            "column selections must agree with CPU"
        );
        eprintln!("bit-exact rows: {row_exact}/{}", ref_a.len());
    }

    #[test]
    fn gpu_matcher_picks_the_backend_tuned_workgroup_size() {
        let Some(context) = GpuContext::new() else {
            eprintln!("no GPU adapter; skipping");
            return;
        };
        let matcher = GpuMatcher::new(&context);
        let expected = gpu_compute::tuned_workgroup_size(context.backend, WG_METAL, WG_DEFAULT);
        assert_eq!(matcher.workgroup_size(), expected);
        if context.backend == wgpu::Backend::Metal {
            assert_eq!(matcher.workgroup_size(), WG_METAL);
        } else {
            assert_eq!(matcher.workgroup_size(), WG_DEFAULT);
        }
    }

    /// Both the Metal-tuned (128) and default (256) workgroup variants must
    /// produce results that agree with the CPU reference, regardless of
    /// which one the test machine's actual backend happens to select — this
    /// is what protects the templated shader from a size-specific bug.
    #[test]
    fn gpu_matching_agrees_with_cpu_for_both_workgroup_tunings() {
        let Some(context) = GpuContext::new() else {
            eprintln!("no GPU adapter; skipping");
            return;
        };
        let da = descriptors(23, 300);
        let db = descriptors(29, 280);
        let (ref_a, ref_b) = cpu_reference(&da, &db);
        for &wg in &[WG_METAL, WG_DEFAULT] {
            let matcher = GpuMatcher::with_workgroup_size(&context, wg);
            let (rows, cols) = matcher.match_descriptors(&da, &db);
            let row_close = rows
                .iter()
                .zip(&ref_a)
                .filter(|(gpu, cpu)| {
                    gpu.j == cpu.0 && (gpu.d1 - cpu.1).abs() <= 1e-4 * cpu.1.max(1.)
                })
                .count();
            let col_close = cols
                .iter()
                .zip(&ref_b)
                .filter(|(gpu, cpu)| {
                    gpu.i == cpu.0 && (gpu.d1 - cpu.1).abs() <= 1e-4 * cpu.1.max(1.)
                })
                .count();
            assert_eq!(
                row_close,
                ref_a.len(),
                "WG={wg}: row selections must agree with CPU"
            );
            assert_eq!(
                col_close,
                ref_b.len(),
                "WG={wg}: column selections must agree with CPU"
            );
        }
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn cuda_matching_agrees_with_cpu_on_synthetic_descriptors() {
        let da = descriptors(31, 300);
        let db = descriptors(37, 280);
        let Some((rows, cols)) = match_pair_accelerated(&da, &db, crate::Acceleration::Cuda) else {
            eprintln!("no CUDA/wgpu adapter; skipping");
            return;
        };
        let (ref_a, ref_b) = cpu_reference(&da, &db);
        let row_close = rows
            .iter()
            .zip(&ref_a)
            .filter(|(gpu, cpu)| gpu.j == cpu.0 && (gpu.d1 - cpu.1).abs() <= 1e-4 * cpu.1.max(1.))
            .count();
        let col_close = cols
            .iter()
            .zip(&ref_b)
            .filter(|(gpu, cpu)| gpu.i == cpu.0 && (gpu.d1 - cpu.1).abs() <= 1e-4 * cpu.1.max(1.))
            .count();
        assert_eq!(row_close, ref_a.len(), "row selections must agree with CPU");
        assert_eq!(
            col_close,
            ref_b.len(),
            "column selections must agree with CPU"
        );
    }
}
