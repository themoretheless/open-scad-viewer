//! CUDA nearest-neighbor search (feature `cuda`): runs the PTX port of
//! `NEAREST_NEIGHBOR_WGSL` (`nearest_neighbor.cu` → `nearest_neighbor.ptx`)
//! through the CUDA driver API. f32 arithmetic — see [`crate::Acceleration`].
//!
//! Device buffers are cached per query/target capacity (grow-only) so
//! repeated calls at a stable size amortize allocation; only the elements
//! actually used for the current call are copied to/from the device.
use crate::V3;
use gpu_compute::cuda::{
    CudaDevice, CudaDeviceReport, CudaFunction, CudaSlice, PushKernelArg, launch_1d,
};

/// PTX generated from `nearest_neighbor.cu` by `scripts/build-cuda-kernels.mjs`.
pub const NEAREST_NEIGHBOR_PTX: &str = include_str!("nearest_neighbor.ptx");
/// PTX generated from `nearest_two.cu` by `scripts/build-cuda-kernels.mjs`.
pub const NEAREST_TWO_PTX: &str = include_str!("nearest_two.ptx");
/// PTX generated from `distance_pairs.cu` by `scripts/build-cuda-kernels.mjs`.
pub const DISTANCE_PAIRS_PTX: &str = include_str!("distance_pairs.ptx");
/// PTX generated from `distance_pair_sum.cu` by `scripts/build-cuda-kernels.mjs`.
pub const DISTANCE_PAIR_SUM_PTX: &str = include_str!("distance_pair_sum.ptx");
/// PTX generated from `chamfer.cu` by `scripts/build-cuda-kernels.mjs`.
pub const CHAMFER_PTX: &str = include_str!("chamfer.ptx");
/// PTX generated from `point_bounds.cu` by `scripts/build-cuda-kernels.mjs`.
pub const POINT_BOUNDS_PTX: &str = include_str!("point_bounds.ptx");
const BLOCK: u32 = 256;

struct Buffers {
    query_capacity: usize,
    target_capacity: usize,
    queries: CudaSlice<f32>,
    targets: CudaSlice<f32>,
    out_index: CudaSlice<u32>,
    out_dist: CudaSlice<f32>,
}

struct CudaNearestNeighbor {
    device: CudaDevice,
    kernel: CudaFunction,
    buffers: std::cell::RefCell<Option<Buffers>>,
}

impl CudaNearestNeighbor {
    fn new() -> Option<Self> {
        let device = CudaDevice::new()?;
        let module = device.load_ptx(NEAREST_NEIGHBOR_PTX)?;
        let kernel = module.load_function("nearest_neighbor").ok()?;
        Some(Self {
            device,
            kernel,
            buffers: std::cell::RefCell::new(None),
        })
    }

    /// (Re)allocates the device slices when either capacity is exceeded; a
    /// no-op otherwise, so repeated calls at a stable or shrinking size
    /// reuse the same device allocations.
    fn ensure_buffers(&self, query_count: usize, target_count: usize) -> Option<()> {
        let stale = match &*self.buffers.borrow() {
            Some(b) => b.query_capacity < query_count || b.target_capacity < target_count,
            None => true,
        };
        if !stale {
            return Some(());
        }
        let query_capacity = query_count.max(1);
        let target_capacity = target_count.max(1);
        let stream = &self.device.stream;
        let queries = stream.alloc_zeros::<f32>(query_capacity * 3).ok()?;
        let targets = stream.alloc_zeros::<f32>(target_capacity * 3).ok()?;
        let out_index = stream.alloc_zeros::<u32>(query_capacity).ok()?;
        let out_dist = stream.alloc_zeros::<f32>(query_capacity).ok()?;
        *self.buffers.borrow_mut() = Some(Buffers {
            query_capacity,
            target_capacity,
            queries,
            targets,
            out_index,
            out_dist,
        });
        Some(())
    }

    fn run(&self, queries: &[V3], targets: &[V3]) -> Option<Vec<(u32, f64)>> {
        let query_count = queries.len();
        let target_count = targets.len();
        self.ensure_buffers(query_count, target_count)?;
        let stream = &self.device.stream;
        let flat_q: Vec<f32> = queries.iter().flatten().map(|&v| v as f32).collect();
        let flat_t: Vec<f32> = targets.iter().flatten().map(|&v| v as f32).collect();
        let mut buffers = self.buffers.borrow_mut();
        let b = buffers.as_mut().expect("ensure_buffers was just called");
        if !flat_q.is_empty() {
            let mut view = b.queries.slice_mut(0..flat_q.len());
            stream.memcpy_htod(&flat_q, &mut view).ok()?;
        }
        if !flat_t.is_empty() {
            let mut view = b.targets.slice_mut(0..flat_t.len());
            stream.memcpy_htod(&flat_t, &mut view).ok()?;
        }
        let query_count_u32 = query_count as u32;
        let target_count_u32 = target_count as u32;
        let q_elems = (query_count * 3).max(1);
        let t_elems = (target_count * 3).max(1);
        let out_elems = query_count.max(1);
        let queries_view = b.queries.slice(0..q_elems);
        let targets_view = b.targets.slice(0..t_elems);
        let mut out_index_view = b.out_index.slice_mut(0..out_elems);
        let mut out_dist_view = b.out_dist.slice_mut(0..out_elems);
        let mut launch = stream.launch_builder(&self.kernel);
        launch
            .arg(&query_count_u32)
            .arg(&target_count_u32)
            .arg(&queries_view)
            .arg(&targets_view)
            .arg(&mut out_index_view)
            .arg(&mut out_dist_view);
        unsafe { launch.launch(launch_1d(query_count_u32, BLOCK)) }.ok()?;
        if query_count == 0 {
            return Some(Vec::new());
        }
        let mut out_index = vec![0u32; query_count];
        let mut out_dist = vec![0f32; query_count];
        stream.memcpy_dtoh(&out_index_view, &mut out_index).ok()?;
        stream.memcpy_dtoh(&out_dist_view, &mut out_dist).ok()?;
        Some(
            out_index
                .into_iter()
                .zip(out_dist)
                .map(|(i, d)| (i, d as f64))
                .collect(),
        )
    }
}

thread_local! {
    // Process-lifetime context, module and stream; see `sdf-core`'s `cuda.rs`.
    static SHARED: std::cell::LazyCell<Option<&'static CudaNearestNeighbor>> =
        std::cell::LazyCell::new(|| {
            CudaNearestNeighbor::new().map(|nn| Box::leak(Box::new(nn)) as &'static CudaNearestNeighbor)
        });
}

/// True when a CUDA device and the kernel module are available on this thread.
pub fn available() -> bool {
    SHARED.with(|cell| {
        let shared: &Option<&CudaNearestNeighbor> = cell;
        shared.is_some()
    })
}

/// Device name for diagnostics/benchmarks; `None` without a CUDA device.
pub fn device_name() -> Option<String> {
    device_report().map(|report| report.name)
}

pub fn device_report() -> Option<CudaDeviceReport> {
    SHARED.with(|cell| {
        let shared: &Option<&CudaNearestNeighbor> = cell;
        shared.map(|nn| nn.device.report())
    })
}

/// Finds each query's nearest target on the CUDA device; `None` without a
/// device or on any driver error (the caller then tries the wgpu path and
/// the CPU reference).
pub(crate) fn nearest_neighbor_cuda(queries: &[V3], targets: &[V3]) -> Option<Vec<(u32, f64)>> {
    SHARED.with(|cell| {
        let shared: &Option<&CudaNearestNeighbor> = cell;
        shared.and_then(|nn| nn.run(queries, targets))
    })
}

struct CudaNearestTwo {
    device: CudaDevice,
    kernel: CudaFunction,
    buffers: std::cell::RefCell<Option<NearestTwoBuffers>>,
}

struct NearestTwoBuffers {
    query_capacity: usize,
    target_capacity: usize,
    queries: CudaSlice<f32>,
    targets: CudaSlice<f32>,
    out_i0: CudaSlice<u32>,
    out_d0: CudaSlice<f32>,
    out_i1: CudaSlice<u32>,
    out_d1: CudaSlice<f32>,
}

impl CudaNearestTwo {
    fn new() -> Option<Self> {
        let device = CudaDevice::new()?;
        let module = device.load_ptx(NEAREST_TWO_PTX)?;
        let kernel = module.load_function("nearest_two").ok()?;
        Some(Self {
            device,
            kernel,
            buffers: std::cell::RefCell::new(None),
        })
    }

    fn ensure_buffers(&self, query_count: usize, target_count: usize) -> Option<()> {
        let stale = match &*self.buffers.borrow() {
            Some(b) => b.query_capacity < query_count || b.target_capacity < target_count,
            None => true,
        };
        if !stale {
            return Some(());
        }
        let query_capacity = query_count.max(1);
        let target_capacity = target_count.max(1);
        let stream = &self.device.stream;
        let queries = stream.alloc_zeros::<f32>(query_capacity * 3).ok()?;
        let targets = stream.alloc_zeros::<f32>(target_capacity * 3).ok()?;
        let out_i0 = stream.alloc_zeros::<u32>(query_capacity).ok()?;
        let out_d0 = stream.alloc_zeros::<f32>(query_capacity).ok()?;
        let out_i1 = stream.alloc_zeros::<u32>(query_capacity).ok()?;
        let out_d1 = stream.alloc_zeros::<f32>(query_capacity).ok()?;
        *self.buffers.borrow_mut() = Some(NearestTwoBuffers {
            query_capacity,
            target_capacity,
            queries,
            targets,
            out_i0,
            out_d0,
            out_i1,
            out_d1,
        });
        Some(())
    }

    fn run(&self, queries: &[V3], targets: &[V3]) -> Option<Vec<crate::TwoNearest>> {
        let query_count = queries.len();
        let target_count = targets.len();
        let stream = &self.device.stream;
        let flat_q: Vec<f32> = queries.iter().flatten().map(|&v| v as f32).collect();
        let flat_t: Vec<f32> = targets.iter().flatten().map(|&v| v as f32).collect();
        self.ensure_buffers(query_count, target_count)?;
        let mut buffers = self.buffers.borrow_mut();
        let b = buffers.as_mut().expect("ensure_buffers was just called");
        if !flat_q.is_empty() {
            let mut view = b.queries.slice_mut(0..flat_q.len());
            stream.memcpy_htod(&flat_q, &mut view).ok()?;
        }
        if !flat_t.is_empty() {
            let mut view = b.targets.slice_mut(0..flat_t.len());
            stream.memcpy_htod(&flat_t, &mut view).ok()?;
        }
        let query_count_u32 = query_count as u32;
        let target_count_u32 = target_count as u32;
        let q_view = b.queries.slice(0..(query_count * 3).max(1));
        let t_view = b.targets.slice(0..(target_count * 3).max(1));
        let mut i0_view = b.out_i0.slice_mut(0..query_count.max(1));
        let mut d0_view = b.out_d0.slice_mut(0..query_count.max(1));
        let mut i1_view = b.out_i1.slice_mut(0..query_count.max(1));
        let mut d1_view = b.out_d1.slice_mut(0..query_count.max(1));
        let mut launch = stream.launch_builder(&self.kernel);
        launch
            .arg(&query_count_u32)
            .arg(&target_count_u32)
            .arg(&q_view)
            .arg(&t_view)
            .arg(&mut i0_view)
            .arg(&mut d0_view)
            .arg(&mut i1_view)
            .arg(&mut d1_view);
        unsafe { launch.launch(launch_1d(query_count_u32, BLOCK)) }.ok()?;
        let mut i0 = vec![0u32; query_count];
        let mut d0 = vec![0f32; query_count];
        let mut i1 = vec![0u32; query_count];
        let mut d1 = vec![0f32; query_count];
        if query_count > 0 {
            stream.memcpy_dtoh(&i0_view, &mut i0).ok()?;
            stream.memcpy_dtoh(&d0_view, &mut d0).ok()?;
            stream.memcpy_dtoh(&i1_view, &mut i1).ok()?;
            stream.memcpy_dtoh(&d1_view, &mut d1).ok()?;
        }
        Some(
            (0..query_count)
                .map(|idx| [(i0[idx], d0[idx] as f64), (i1[idx], d1[idx] as f64)])
                .collect(),
        )
    }
}

thread_local! {
    static SHARED_NEAREST_TWO: std::cell::LazyCell<Option<&'static CudaNearestTwo>> =
        std::cell::LazyCell::new(|| {
            CudaNearestTwo::new().map(|nn| Box::leak(Box::new(nn)) as &'static CudaNearestTwo)
        });
}

pub(crate) fn nearest_two_cuda(queries: &[V3], targets: &[V3]) -> Option<Vec<crate::TwoNearest>> {
    SHARED_NEAREST_TWO.with(|cell| {
        let shared: &Option<&CudaNearestTwo> = cell;
        shared.and_then(|nn| nn.run(queries, targets))
    })
}

struct CudaDistancePairs {
    device: CudaDevice,
    kernel: CudaFunction,
    buffers: std::cell::RefCell<Option<DistancePairBuffers>>,
}

struct DistancePairBuffers {
    capacity: usize,
    a: CudaSlice<f32>,
    b: CudaSlice<f32>,
    out: CudaSlice<f32>,
}

impl CudaDistancePairs {
    fn new() -> Option<Self> {
        let device = CudaDevice::new()?;
        let module = device.load_ptx(DISTANCE_PAIRS_PTX)?;
        let kernel = module.load_function("squared_distance_pairs").ok()?;
        Some(Self {
            device,
            kernel,
            buffers: std::cell::RefCell::new(None),
        })
    }

    fn ensure_buffers(&self, pair_count: usize) -> Option<()> {
        let stale = match &*self.buffers.borrow() {
            Some(buffers) => buffers.capacity < pair_count,
            None => true,
        };
        if !stale {
            return Some(());
        }
        let capacity = pair_count.max(1);
        let stream = &self.device.stream;
        let a = stream.alloc_zeros::<f32>(capacity * 3).ok()?;
        let b = stream.alloc_zeros::<f32>(capacity * 3).ok()?;
        let out = stream.alloc_zeros::<f32>(capacity).ok()?;
        *self.buffers.borrow_mut() = Some(DistancePairBuffers {
            capacity,
            a,
            b,
            out,
        });
        Some(())
    }

    fn run(&self, a: &[V3], b: &[V3]) -> Option<Vec<f64>> {
        if a.len() != b.len() {
            return None;
        }
        let pair_count = a.len();
        let stream = &self.device.stream;
        let flat_a: Vec<f32> = a.iter().flatten().map(|&v| v as f32).collect();
        let flat_b: Vec<f32> = b.iter().flatten().map(|&v| v as f32).collect();
        self.ensure_buffers(pair_count)?;
        let mut buffers = self.buffers.borrow_mut();
        let buffers = buffers.as_mut().expect("ensure_buffers was just called");
        {
            let mut a_view = buffers.a.slice_mut(0..(pair_count * 3).max(1));
            stream.memcpy_htod(&flat_a, &mut a_view).ok()?;
        }
        {
            let mut b_view = buffers.b.slice_mut(0..(pair_count * 3).max(1));
            stream.memcpy_htod(&flat_b, &mut b_view).ok()?;
        }
        let pair_count_u32 = pair_count as u32;
        let a_view = buffers.a.slice(0..(pair_count * 3).max(1));
        let b_view = buffers.b.slice(0..(pair_count * 3).max(1));
        let mut out_view = buffers.out.slice_mut(0..pair_count.max(1));
        let mut launch = stream.launch_builder(&self.kernel);
        launch
            .arg(&pair_count_u32)
            .arg(&a_view)
            .arg(&b_view)
            .arg(&mut out_view);
        unsafe { launch.launch(launch_1d(pair_count_u32, BLOCK)) }.ok()?;
        let mut host = vec![0f32; pair_count];
        stream.memcpy_dtoh(&out_view, &mut host).ok()?;
        Some(host.into_iter().map(|value| value as f64).collect())
    }
}

thread_local! {
    static SHARED_DISTANCE_PAIRS: std::cell::LazyCell<Option<&'static CudaDistancePairs>> =
        std::cell::LazyCell::new(|| {
            CudaDistancePairs::new().map(|kernel| Box::leak(Box::new(kernel)) as &'static CudaDistancePairs)
        });
}

/// One-to-one squared distances on the CUDA device; `None` without a device.
pub(crate) fn squared_distance_pairs_cuda(a: &[V3], b: &[V3]) -> Option<Vec<f64>> {
    SHARED_DISTANCE_PAIRS.with(|cell| {
        let shared: &Option<&CudaDistancePairs> = cell;
        shared.and_then(|kernel| kernel.run(a, b))
    })
}

struct CudaDistancePairSum {
    device: CudaDevice,
    kernel: CudaFunction,
    buffers: std::cell::RefCell<Option<DistancePairSumBuffers>>,
}

struct DistancePairSumBuffers {
    pair_capacity: usize,
    partial_capacity: usize,
    a: CudaSlice<f32>,
    b: CudaSlice<f32>,
    out: CudaSlice<f32>,
}

impl CudaDistancePairSum {
    fn new() -> Option<Self> {
        let device = CudaDevice::new()?;
        let module = device.load_ptx(DISTANCE_PAIR_SUM_PTX)?;
        let kernel = module.load_function("squared_distance_pair_sum").ok()?;
        Some(Self {
            device,
            kernel,
            buffers: std::cell::RefCell::new(None),
        })
    }

    fn ensure_buffers(&self, pair_count: usize, partial_count: usize) -> Option<()> {
        let stale = match &*self.buffers.borrow() {
            Some(b) => b.pair_capacity < pair_count || b.partial_capacity < partial_count,
            None => true,
        };
        if !stale {
            return Some(());
        }
        let pair_capacity = pair_count.max(1);
        let partial_capacity = partial_count.max(1);
        let stream = &self.device.stream;
        let a = stream.alloc_zeros::<f32>(pair_capacity * 3).ok()?;
        let b = stream.alloc_zeros::<f32>(pair_capacity * 3).ok()?;
        let out = stream.alloc_zeros::<f32>(partial_capacity).ok()?;
        *self.buffers.borrow_mut() = Some(DistancePairSumBuffers {
            pair_capacity,
            partial_capacity,
            a,
            b,
            out,
        });
        Some(())
    }

    fn run(&self, a: &[V3], b: &[V3]) -> Option<f64> {
        if a.len() != b.len() {
            return None;
        }
        let pair_count = a.len();
        let partial_count = pair_count.div_ceil(BLOCK as usize).max(1);
        let stream = &self.device.stream;
        let flat_a: Vec<f32> = a.iter().flatten().map(|&v| v as f32).collect();
        let flat_b: Vec<f32> = b.iter().flatten().map(|&v| v as f32).collect();
        self.ensure_buffers(pair_count, partial_count)?;
        let mut buffers = self.buffers.borrow_mut();
        let buffers = buffers.as_mut().expect("ensure_buffers was just called");
        {
            let mut a_view = buffers.a.slice_mut(0..(pair_count * 3).max(1));
            stream.memcpy_htod(&flat_a, &mut a_view).ok()?;
        }
        {
            let mut b_view = buffers.b.slice_mut(0..(pair_count * 3).max(1));
            stream.memcpy_htod(&flat_b, &mut b_view).ok()?;
        }
        let pair_count_u32 = pair_count as u32;
        let a_view = buffers.a.slice(0..(pair_count * 3).max(1));
        let b_view = buffers.b.slice(0..(pair_count * 3).max(1));
        let mut out_view = buffers.out.slice_mut(0..partial_count);
        let mut launch = stream.launch_builder(&self.kernel);
        launch
            .arg(&pair_count_u32)
            .arg(&a_view)
            .arg(&b_view)
            .arg(&mut out_view);
        unsafe { launch.launch(launch_1d(pair_count_u32, BLOCK)) }.ok()?;
        let mut partials = vec![0f32; partial_count];
        stream.memcpy_dtoh(&out_view, &mut partials).ok()?;
        Some(partials.into_iter().map(|value| value as f64).sum())
    }
}

thread_local! {
    static SHARED_DISTANCE_PAIR_SUM: std::cell::LazyCell<Option<&'static CudaDistancePairSum>> =
        std::cell::LazyCell::new(|| {
            CudaDistancePairSum::new().map(|kernel| Box::leak(Box::new(kernel)) as &'static CudaDistancePairSum)
        });
}

/// Sum of one-to-one squared distances on the CUDA device; `None` without a device.
pub(crate) fn squared_distance_pair_sum_cuda(a: &[V3], b: &[V3]) -> Option<f64> {
    SHARED_DISTANCE_PAIR_SUM.with(|cell| {
        let shared: &Option<&CudaDistancePairSum> = cell;
        shared.and_then(|kernel| kernel.run(a, b))
    })
}

struct CudaChamfer {
    device: CudaDevice,
    kernel: CudaFunction,
    buffers: std::cell::RefCell<Option<ChamferBuffers>>,
}

struct ChamferBuffers {
    query_capacity: usize,
    target_capacity: usize,
    partial_capacity: usize,
    queries: CudaSlice<f32>,
    targets: CudaSlice<f32>,
    out_sum: CudaSlice<f32>,
    out_max: CudaSlice<f32>,
}

impl CudaChamfer {
    fn new() -> Option<Self> {
        let device = CudaDevice::new()?;
        let module = device.load_ptx(CHAMFER_PTX)?;
        let kernel = module.load_function("directed_chamfer_reduce").ok()?;
        Some(Self {
            device,
            kernel,
            buffers: std::cell::RefCell::new(None),
        })
    }

    fn ensure_buffers(
        &self,
        query_count: usize,
        target_count: usize,
        partial_count: usize,
    ) -> Option<()> {
        let stale = match &*self.buffers.borrow() {
            Some(b) => {
                b.query_capacity < query_count
                    || b.target_capacity < target_count
                    || b.partial_capacity < partial_count
            }
            None => true,
        };
        if !stale {
            return Some(());
        }
        let query_capacity = query_count.max(1);
        let target_capacity = target_count.max(1);
        let partial_capacity = partial_count.max(1);
        let stream = &self.device.stream;
        let queries = stream.alloc_zeros::<f32>(query_capacity * 3).ok()?;
        let targets = stream.alloc_zeros::<f32>(target_capacity * 3).ok()?;
        let out_sum = stream.alloc_zeros::<f32>(partial_capacity).ok()?;
        let out_max = stream.alloc_zeros::<f32>(partial_capacity).ok()?;
        *self.buffers.borrow_mut() = Some(ChamferBuffers {
            query_capacity,
            target_capacity,
            partial_capacity,
            queries,
            targets,
            out_sum,
            out_max,
        });
        Some(())
    }

    fn run(&self, queries: &[V3], targets: &[V3]) -> Option<crate::DirectedChamfer> {
        if queries.is_empty() || targets.is_empty() {
            return None;
        }
        let query_count = queries.len();
        let target_count = targets.len();
        let partial_count = query_count.div_ceil(BLOCK as usize).max(1);
        let stream = &self.device.stream;
        let flat_q: Vec<f32> = queries.iter().flatten().map(|&v| v as f32).collect();
        let flat_t: Vec<f32> = targets.iter().flatten().map(|&v| v as f32).collect();
        self.ensure_buffers(query_count, target_count, partial_count)?;
        let mut buffers = self.buffers.borrow_mut();
        let buffers = buffers.as_mut().expect("ensure_buffers was just called");
        {
            let mut q_view = buffers.queries.slice_mut(0..query_count * 3);
            stream.memcpy_htod(&flat_q, &mut q_view).ok()?;
        }
        {
            let mut t_view = buffers.targets.slice_mut(0..target_count * 3);
            stream.memcpy_htod(&flat_t, &mut t_view).ok()?;
        }
        let query_count_u32 = query_count as u32;
        let target_count_u32 = target_count as u32;
        let q_view = buffers.queries.slice(0..query_count * 3);
        let t_view = buffers.targets.slice(0..target_count * 3);
        let mut sum_view = buffers.out_sum.slice_mut(0..partial_count);
        let mut max_view = buffers.out_max.slice_mut(0..partial_count);
        let mut launch = stream.launch_builder(&self.kernel);
        launch
            .arg(&query_count_u32)
            .arg(&target_count_u32)
            .arg(&q_view)
            .arg(&t_view)
            .arg(&mut sum_view)
            .arg(&mut max_view);
        unsafe { launch.launch(launch_1d(query_count_u32, BLOCK)) }.ok()?;
        let mut sums = vec![0f32; partial_count];
        let mut maxes = vec![0f32; partial_count];
        stream.memcpy_dtoh(&sum_view, &mut sums).ok()?;
        stream.memcpy_dtoh(&max_view, &mut maxes).ok()?;
        let sum: f64 = sums.into_iter().map(|value| value as f64).sum();
        let max_squared_distance = maxes
            .into_iter()
            .map(|value| value as f64)
            .fold(0., f64::max);
        let mean_squared_distance = sum / query_count as f64;
        Some(crate::DirectedChamfer {
            samples: query_count,
            mean_squared_distance,
            rms_distance: mean_squared_distance.sqrt(),
            max_squared_distance,
        })
    }
}

thread_local! {
    static SHARED_CHAMFER: std::cell::LazyCell<Option<&'static CudaChamfer>> =
        std::cell::LazyCell::new(|| {
            CudaChamfer::new().map(|kernel| Box::leak(Box::new(kernel)) as &'static CudaChamfer)
        });
}

/// Directed Chamfer reduction on the CUDA device; `None` without a device.
pub(crate) fn directed_chamfer_cuda(
    queries: &[V3],
    targets: &[V3],
) -> Option<crate::DirectedChamfer> {
    SHARED_CHAMFER.with(|cell| {
        let shared: &Option<&CudaChamfer> = cell;
        shared.and_then(|kernel| kernel.run(queries, targets))
    })
}

struct CudaPointBounds {
    device: CudaDevice,
    kernel: CudaFunction,
    buffers: std::cell::RefCell<Option<PointBoundsBuffers>>,
}

struct PointBoundsBuffers {
    point_capacity: usize,
    partial_capacity: usize,
    points: CudaSlice<f32>,
    out_min: CudaSlice<f32>,
    out_max: CudaSlice<f32>,
}

impl CudaPointBounds {
    fn new() -> Option<Self> {
        let device = CudaDevice::new()?;
        let module = device.load_ptx(POINT_BOUNDS_PTX)?;
        let kernel = module.load_function("point_bounds_reduce").ok()?;
        Some(Self {
            device,
            kernel,
            buffers: std::cell::RefCell::new(None),
        })
    }

    fn ensure_buffers(&self, point_count: usize, partial_count: usize) -> Option<()> {
        let stale = match &*self.buffers.borrow() {
            Some(b) => b.point_capacity < point_count || b.partial_capacity < partial_count,
            None => true,
        };
        if !stale {
            return Some(());
        }
        let point_capacity = point_count.max(1);
        let partial_capacity = partial_count.max(1);
        let stream = &self.device.stream;
        let points = stream.alloc_zeros::<f32>(point_capacity * 3).ok()?;
        let out_min = stream.alloc_zeros::<f32>(partial_capacity * 3).ok()?;
        let out_max = stream.alloc_zeros::<f32>(partial_capacity * 3).ok()?;
        *self.buffers.borrow_mut() = Some(PointBoundsBuffers {
            point_capacity,
            partial_capacity,
            points,
            out_min,
            out_max,
        });
        Some(())
    }

    fn run(&self, points: &[V3]) -> Option<crate::PointBounds> {
        if points.is_empty() {
            return None;
        }
        let point_count = points.len();
        let partial_count = point_count.div_ceil(BLOCK as usize).max(1);
        let stream = &self.device.stream;
        let flat: Vec<f32> = points.iter().flatten().map(|&v| v as f32).collect();
        self.ensure_buffers(point_count, partial_count)?;
        let mut buffers = self.buffers.borrow_mut();
        let buffers = buffers.as_mut().expect("ensure_buffers was just called");
        {
            let mut points_view = buffers.points.slice_mut(0..point_count * 3);
            stream.memcpy_htod(&flat, &mut points_view).ok()?;
        }
        let point_count_u32 = point_count as u32;
        let points_view = buffers.points.slice(0..point_count * 3);
        let mut min_view = buffers.out_min.slice_mut(0..partial_count * 3);
        let mut max_view = buffers.out_max.slice_mut(0..partial_count * 3);
        let mut launch = stream.launch_builder(&self.kernel);
        launch
            .arg(&point_count_u32)
            .arg(&points_view)
            .arg(&mut min_view)
            .arg(&mut max_view);
        unsafe { launch.launch(launch_1d(point_count_u32, BLOCK)) }.ok()?;
        let mut mins = vec![0f32; partial_count * 3];
        let mut maxes = vec![0f32; partial_count * 3];
        stream.memcpy_dtoh(&min_view, &mut mins).ok()?;
        stream.memcpy_dtoh(&max_view, &mut maxes).ok()?;
        let mut min = [f64::INFINITY; 3];
        let mut max = [f64::NEG_INFINITY; 3];
        for chunk in mins.chunks_exact(3) {
            for axis in 0..3 {
                min[axis] = min[axis].min(chunk[axis] as f64);
            }
        }
        for chunk in maxes.chunks_exact(3) {
            for axis in 0..3 {
                max[axis] = max[axis].max(chunk[axis] as f64);
            }
        }
        Some(crate::PointBounds {
            samples: point_count,
            min,
            max,
            center: [
                0.5 * (min[0] + max[0]),
                0.5 * (min[1] + max[1]),
                0.5 * (min[2] + max[2]),
            ],
            extent: [max[0] - min[0], max[1] - min[1], max[2] - min[2]],
        })
    }
}

thread_local! {
    static SHARED_POINT_BOUNDS: std::cell::LazyCell<Option<&'static CudaPointBounds>> =
        std::cell::LazyCell::new(|| {
            CudaPointBounds::new().map(|kernel| Box::leak(Box::new(kernel)) as &'static CudaPointBounds)
        });
}

/// Point-cloud axis-aligned bounds on the CUDA device; `None` without a device.
pub(crate) fn point_bounds_cuda(points: &[V3]) -> Option<crate::PointBounds> {
    SHARED_POINT_BOUNDS.with(|cell| {
        let shared: &Option<&CudaPointBounds> = cell;
        shared.and_then(|kernel| kernel.run(points))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cuda_nearest_neighbor_matches_cpu_reference_when_available() {
        let queries: Vec<V3> = (0..64)
            .map(|i| {
                let f = i as f64;
                [f * 0.13 - 10., f * -0.07 + 4., (f * 0.031).sin() * 5.]
            })
            .collect();
        let targets: Vec<V3> = (0..20)
            .map(|i| {
                let f = i as f64;
                [f * 0.9 - 6., (f * 0.21).cos() * 4., f * -0.4]
            })
            .collect();
        let Some(got) = nearest_neighbor_cuda(&queries, &targets) else {
            eprintln!("no CUDA device available; skipping");
            return;
        };
        let want = crate::nearest_neighbor(&queries, &targets);
        assert_eq!(got.len(), want.len());
        for ((_gi, gd), (_wi, wd)) in got.iter().zip(&want) {
            assert!((gd - wd).abs() < 5e-3 * wd.max(1.0), "{gd} vs {wd}");
        }
    }

    #[test]
    fn cuda_nearest_neighbor_empty_input() {
        if let Some(got) = nearest_neighbor_cuda(&[], &[[0., 0., 0.]]) {
            assert!(got.is_empty());
        }
    }

    #[test]
    fn cuda_squared_distance_pairs_matches_cpu_reference_when_available() {
        let a: Vec<V3> = (0..128)
            .map(|i| {
                let f = i as f64;
                [f * 0.07, (f * 0.03).sin(), (f * 0.11).cos()]
            })
            .collect();
        let b: Vec<V3> = (0..128)
            .map(|i| {
                let f = i as f64;
                [f * -0.02, (f * 0.13).cos(), (f * 0.17).sin()]
            })
            .collect();
        let Some(got) = squared_distance_pairs_cuda(&a, &b) else {
            eprintln!("no CUDA device available; skipping");
            return;
        };
        let want = crate::squared_distance_pairs(&a, &b).unwrap();
        for (got, want) in got.iter().zip(&want) {
            assert!((got - want).abs() < 1e-4 * want.max(1.0), "{got} vs {want}");
        }
    }

    #[test]
    fn device_probe_does_not_panic() {
        if available() {
            assert!(device_name().is_some());
        }
    }

    #[test]
    fn cuda_squared_distance_pair_sum_matches_cpu_reference_when_available() {
        let a: Vec<V3> = (0..1024)
            .map(|i| {
                let f = i as f64;
                [f * 0.07, (f * 0.03).sin(), (f * 0.11).cos()]
            })
            .collect();
        let b: Vec<V3> = (0..1024)
            .map(|i| {
                let f = i as f64;
                [f * -0.02, (f * 0.13).cos(), (f * 0.17).sin()]
            })
            .collect();
        let Some(got) = squared_distance_pair_sum_cuda(&a, &b) else {
            eprintln!("no CUDA device available; skipping");
            return;
        };
        let want = crate::squared_distance_pair_sum(&a, &b).unwrap();
        assert!((got - want).abs() < 1e-4 * want.max(1.0), "{got} vs {want}");
    }
}
