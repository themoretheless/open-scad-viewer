//! CUDA nearest-neighbor search (feature `cuda`): runs the PTX port of
//! `NEAREST_NEIGHBOR_WGSL` (`nearest_neighbor.cu` → `nearest_neighbor.ptx`)
//! through the CUDA driver API. f32 arithmetic — see [`crate::Acceleration`].
//!
//! Device buffers are cached per query/target capacity (grow-only) so
//! repeated calls at a stable size amortize allocation; only the elements
//! actually used for the current call are copied to/from the device.
use crate::V3;
use gpu_compute::cuda::{CudaDevice, CudaFunction, CudaSlice, PushKernelArg, launch_1d};

/// PTX generated from `nearest_neighbor.cu` by `scripts/build-cuda-kernels.mjs`.
pub const NEAREST_NEIGHBOR_PTX: &str = include_str!("nearest_neighbor.ptx");
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
    SHARED.with(|cell| {
        let shared: &Option<&CudaNearestNeighbor> = cell;
        shared.map(|nn| nn.device.report().name)
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
    fn device_probe_does_not_panic() {
        if available() {
            assert!(device_name().is_some());
        }
    }
}
