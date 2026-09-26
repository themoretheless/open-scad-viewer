//! GPU batch kernels (feature `gpu`) on top of the shared `compute-core`
//! runtime: pipeline construction, workgroup-size tuning, dispatch and
//! readback all go through `compute_core::Kernel`; this module keeps only
//! the per-kernel grow-only buffer pools, the cached bind groups, and the
//! CPU-side folds of partial reductions. f32 arithmetic — see
//! [`crate::Acceleration`].
//!
//! Device buffers are cached per input capacity (grow-only) so repeated calls
//! at a stable size amortize allocation; only the bytes actually written/read
//! for the current call cross the wire.

use crate::{M3, V3};
use compute_core::gpu_compute::wgpu;
use compute_core::gpu_compute::{BackendReport, GpuContext, pack_f32};
use compute_core::{Binding, Kernel, read_f32, read_u32};
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

/// Sequential bind group over the kernel's layout: binding 0..n in buffer
/// order, matching each shader's declaration order.
fn bind(device: &Device, kernel: &Kernel, label: &str, buffers: &[&Buffer]) -> BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout: kernel.bind_group_layout(),
        entries: &buffers
            .iter()
            .enumerate()
            .map(|(index, buffer)| wgpu::BindGroupEntry {
                binding: index as u32,
                resource: buffer.as_entire_binding(),
            })
            .collect::<Vec<_>>(),
    })
}

/// Little-endian uniform payload from u32 words (counts, pads).
fn pack_params(words: &[u32]) -> Vec<u8> {
    words.iter().flat_map(|word| word.to_le_bytes()).collect()
}

fn push_f32(bytes: &mut Vec<u8>, value: f32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

/// Uniform + read storage + write storage bindings, the common shape.
const UNIFORM_STORAGE2: [Binding; 4] = [
    Binding::Uniform,
    Binding::StorageRead,
    Binding::StorageReadWrite,
    Binding::StorageReadWrite,
];

/// Uniform + two read-only inputs + two read-write outputs (nearest-neighbor).
const NN_BINDINGS: [Binding; 5] = [
    Binding::Uniform,
    Binding::StorageRead,
    Binding::StorageRead,
    Binding::StorageReadWrite,
    Binding::StorageReadWrite,
];

struct GpuNearestNeighbor {
    device: Device,
    queue: wgpu::Queue,
    kernel: Kernel,
    buffers: std::cell::RefCell<Option<NearestNeighborBuffers>>,
}

struct NearestNeighborBuffers {
    query_capacity: usize,
    target_capacity: usize,
    params: Buffer,
    queries: Buffer,
    targets: Buffer,
    out_index: Buffer,
    out_dist: Buffer,
    bind: BindGroup,
}

impl GpuNearestNeighbor {
    fn new(context: &GpuContext) -> Self {
        let kernel = Kernel::tuned(
            context,
            "nearest_neighbor",
            crate::NEAREST_NEIGHBOR_WGSL,
            "main",
            &NN_BINDINGS,
            WG_METAL,
            WG_DEFAULT,
        )
        .expect("nearest_neighbor kernel builds");
        Self {
            device: context.device.clone(),
            queue: context.queue.clone(),
            kernel,
            buffers: std::cell::RefCell::new(None),
        }
    }

    /// (Re)allocates the device buffers and bind group when either capacity
    /// is exceeded; a no-op otherwise, so repeated calls at a stable or
    /// shrinking size reuse the same GPU allocations.
    fn ensure_buffers(&self, query_count: usize, target_count: usize) {
        let stale = match &*self.buffers.borrow() {
            Some(b) => b.query_capacity < query_count || b.target_capacity < target_count,
            None => true,
        };
        if !stale {
            return;
        }
        let device = &self.device;
        let query_capacity = query_count.max(1);
        let target_capacity = target_count.max(1);
        let params = mk(
            device,
            "nn_params",
            16,
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        );
        let queries = mk(
            device,
            "nn_queries",
            (query_capacity * 12) as u64,
            BufferUsages::STORAGE | BufferUsages::COPY_DST,
        );
        let targets = mk(
            device,
            "nn_targets",
            (target_capacity * 12) as u64,
            BufferUsages::STORAGE | BufferUsages::COPY_DST,
        );
        let out_bytes = (query_capacity * 4) as u64;
        let out_index = mk(
            device,
            "nn_out_index",
            out_bytes,
            BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        );
        let out_dist = mk(
            device,
            "nn_out_dist",
            out_bytes,
            BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        );
        let bind = bind(
            device,
            &self.kernel,
            "nearest_neighbor",
            &[&params, &queries, &targets, &out_index, &out_dist],
        );
        *self.buffers.borrow_mut() = Some(NearestNeighborBuffers {
            query_capacity,
            target_capacity,
            params,
            queries,
            targets,
            out_index,
            out_dist,
            bind,
        });
    }

    fn run(&self, queries: &[V3], targets: &[V3]) -> Vec<(u32, f64)> {
        let query_count = queries.len();
        let target_count = targets.len();
        self.ensure_buffers(query_count, target_count);
        // Uniform layout matches nearest_neighbor.wgsl's `Params`: two counts
        // padded to a 16-byte uniform buffer.
        let params = pack_params(&[
            query_count as u32,
            target_count as u32,
            0,
            0,
        ]);
        let flat_q: Vec<f32> = queries.iter().flatten().map(|&v| v as f32).collect();
        let flat_t: Vec<f32> = targets.iter().flatten().map(|&v| v as f32).collect();

        let buffers = self.buffers.borrow();
        let b = buffers.as_ref().expect("ensure_buffers was just called");
        self.queue.write_buffer(&b.params, 0, &params);
        if !flat_q.is_empty() {
            self.queue.write_buffer(&b.queries, 0, &pack_f32(&flat_q));
        }
        if !flat_t.is_empty() {
            self.queue.write_buffer(&b.targets, 0, &pack_f32(&flat_t));
        }
        if query_count > 0 {
            self.kernel.dispatch_bind_group(
                &self.device,
                &self.queue,
                &b.bind,
                self.kernel.workgroup_count(query_count as u32),
            );
        }
        if query_count == 0 {
            return Vec::new();
        }
        let raw_index = read_u32(&self.device, &self.queue, &b.out_index, query_count);
        let raw_dist = read_f32(&self.device, &self.queue, &b.out_dist, query_count);
        raw_index
            .iter()
            .zip(raw_dist.iter())
            .map(|(&index, &dist)| (index, dist as f64))
            .collect()
    }
}

thread_local! {
    // Process-lifetime device and pipeline; see `sdf-core`'s `gpu.rs`.
    static SHARED: std::cell::LazyCell<Option<&'static (GpuContext, GpuNearestNeighbor)>> =
        std::cell::LazyCell::new(|| GpuContext::new().map(|c| {
            let nn = GpuNearestNeighbor::new(&c);
            Box::leak(Box::new((c, nn))) as &'static (GpuContext, GpuNearestNeighbor)
        }));
}

/// Finds each query's nearest target on the GPU; `None` without an adapter
/// (CPU fallback).
pub fn nearest_neighbor_gpu(queries: &[V3], targets: &[V3]) -> Option<Vec<(u32, f64)>> {
    SHARED.with(|cell| {
        let shared: &Option<&(GpuContext, GpuNearestNeighbor)> = cell;
        shared.map(|(_, nn)| nn.run(queries, targets))
    })
}

pub fn backend_label() -> Option<&'static str> {
    backend_report().map(|report| report.label)
}

pub fn backend_report() -> Option<BackendReport> {
    SHARED.with(|cell| {
        let shared: &Option<&(GpuContext, GpuNearestNeighbor)> = cell;
        shared.map(|(context, _)| context.backend_report())
    })
}

/// Uniform + two read storages + one write storage (pairwise kernels).
const UNIFORM_PAIR: [Binding; 4] = [
    Binding::Uniform,
    Binding::StorageRead,
    Binding::StorageRead,
    Binding::StorageReadWrite,
];

struct GpuDistancePairs {
    device: Device,
    queue: wgpu::Queue,
    kernel: Kernel,
    buffers: std::cell::RefCell<Option<DistancePairBuffers>>,
}

struct DistancePairBuffers {
    capacity: usize,
    params: Buffer,
    a: Buffer,
    b: Buffer,
    out: Buffer,
    bind: BindGroup,
}

impl GpuDistancePairs {
    fn new(context: &GpuContext) -> Self {
        let kernel = Kernel::tuned(
            context,
            "distance_pairs",
            crate::DISTANCE_PAIRS_WGSL,
            "main",
            &UNIFORM_PAIR,
            WG_METAL,
            WG_DEFAULT,
        )
        .expect("distance_pairs kernel builds");
        Self {
            device: context.device.clone(),
            queue: context.queue.clone(),
            kernel,
            buffers: std::cell::RefCell::new(None),
        }
    }

    fn ensure_buffers(&self, pair_count: usize) {
        let stale = match &*self.buffers.borrow() {
            Some(buffers) => buffers.capacity < pair_count,
            None => true,
        };
        if !stale {
            return;
        }
        let device = &self.device;
        let capacity = pair_count.max(1);
        let params = mk(
            device,
            "distance_pairs_params",
            16,
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        );
        let a = mk(
            device,
            "distance_pairs_a",
            (capacity * 12) as u64,
            BufferUsages::STORAGE | BufferUsages::COPY_DST,
        );
        let b = mk(
            device,
            "distance_pairs_b",
            (capacity * 12) as u64,
            BufferUsages::STORAGE | BufferUsages::COPY_DST,
        );
        let out = mk(
            device,
            "distance_pairs_out",
            (capacity * 4) as u64,
            BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        );
        let bind = bind(device, &self.kernel, "distance_pairs", &[&params, &a, &b, &out]);
        *self.buffers.borrow_mut() = Some(DistancePairBuffers {
            capacity,
            params,
            a,
            b,
            out,
            bind,
        });
    }

    fn run(&self, a: &[V3], b: &[V3]) -> Vec<f64> {
        let pair_count = a.len();
        self.ensure_buffers(pair_count);
        let flat_a: Vec<f32> = a.iter().flatten().map(|&v| v as f32).collect();
        let flat_b: Vec<f32> = b.iter().flatten().map(|&v| v as f32).collect();
        let params = pack_params(&[pair_count as u32, 0, 0, 0]);
        let buffers = self.buffers.borrow();
        let buffers = buffers.as_ref().expect("ensure_buffers was just called");
        self.queue.write_buffer(&buffers.params, 0, &params);
        self.queue
            .write_buffer(&buffers.a, 0, &pack_f32(&flat_a));
        self.queue
            .write_buffer(&buffers.b, 0, &pack_f32(&flat_b));
        self.kernel.dispatch_bind_group(
            &self.device,
            &self.queue,
            &buffers.bind,
            self.kernel.workgroup_count(pair_count.max(1) as u32),
        );
        read_f32(&self.device, &self.queue, &buffers.out, pair_count)
            .iter()
            .map(|&dist| dist as f64)
            .collect()
    }
}

thread_local! {
    static SHARED_DISTANCE_PAIRS: std::cell::LazyCell<Option<&'static GpuDistancePairs>> =
        std::cell::LazyCell::new(|| {
            GpuContext::new()
                .map(|context| Box::leak(Box::new(GpuDistancePairs::new(&context))) as &'static GpuDistancePairs)
        });
}

/// One-to-one squared distances on the GPU; `None` without an adapter.
pub fn squared_distance_pairs_gpu(a: &[V3], b: &[V3]) -> Option<Vec<f64>> {
    if a.len() != b.len() {
        return None;
    }
    SHARED_DISTANCE_PAIRS.with(|cell| {
        let shared: &Option<&GpuDistancePairs> = cell;
        shared.map(|kernel| kernel.run(a, b))
    })
}

struct GpuDistancePairSum {
    device: Device,
    queue: wgpu::Queue,
    kernel: Kernel,
    buffers: std::cell::RefCell<Option<DistancePairSumBuffers>>,
}

struct DistancePairSumBuffers {
    pair_capacity: usize,
    partial_capacity: usize,
    params: Buffer,
    a: Buffer,
    b: Buffer,
    out: Buffer,
    bind: BindGroup,
}

impl GpuDistancePairSum {
    fn new(context: &GpuContext) -> Self {
        let kernel = Kernel::tuned(
            context,
            "distance_pair_sum",
            crate::DISTANCE_PAIR_SUM_WGSL,
            "main",
            &UNIFORM_PAIR,
            WG_METAL,
            WG_DEFAULT,
        )
        .expect("distance_pair_sum kernel builds");
        Self {
            device: context.device.clone(),
            queue: context.queue.clone(),
            kernel,
            buffers: std::cell::RefCell::new(None),
        }
    }

    fn ensure_buffers(&self, pair_count: usize, partial_count: usize) {
        let stale = match &*self.buffers.borrow() {
            Some(b) => b.pair_capacity < pair_count || b.partial_capacity < partial_count,
            None => true,
        };
        if !stale {
            return;
        }
        let device = &self.device;
        let pair_capacity = pair_count.max(1);
        let partial_capacity = partial_count.max(1);
        let params = mk(
            device,
            "distance_pair_sum_params",
            16,
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        );
        let a = mk(
            device,
            "distance_pair_sum_a",
            (pair_capacity * 12) as u64,
            BufferUsages::STORAGE | BufferUsages::COPY_DST,
        );
        let b = mk(
            device,
            "distance_pair_sum_b",
            (pair_capacity * 12) as u64,
            BufferUsages::STORAGE | BufferUsages::COPY_DST,
        );
        let out = mk(
            device,
            "distance_pair_sum_out",
            (partial_capacity * 4) as u64,
            BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        );
        let bind = bind(device, &self.kernel, "distance_pair_sum", &[&params, &a, &b, &out]);
        *self.buffers.borrow_mut() = Some(DistancePairSumBuffers {
            pair_capacity,
            partial_capacity,
            params,
            a,
            b,
            out,
            bind,
        });
    }

    fn run(&self, a: &[V3], b: &[V3]) -> f64 {
        let pair_count = a.len();
        let partial_count = self.kernel.workgroup_count(pair_count.max(1) as u32) as usize;
        self.ensure_buffers(pair_count, partial_count);
        let flat_a: Vec<f32> = a.iter().flatten().map(|&v| v as f32).collect();
        let flat_b: Vec<f32> = b.iter().flatten().map(|&v| v as f32).collect();
        let params = pack_params(&[pair_count as u32, 0, 0, 0]);
        let buffers = self.buffers.borrow();
        let buffers = buffers.as_ref().expect("ensure_buffers was just called");
        self.queue.write_buffer(&buffers.params, 0, &params);
        self.queue
            .write_buffer(&buffers.a, 0, &pack_f32(&flat_a));
        self.queue
            .write_buffer(&buffers.b, 0, &pack_f32(&flat_b));
        self.kernel
            .dispatch_bind_group(&self.device, &self.queue, &buffers.bind, partial_count as u32);
        read_f32(&self.device, &self.queue, &buffers.out, partial_count)
            .iter()
            .map(|&dist| dist as f64)
            .sum()
    }
}

thread_local! {
    static SHARED_DISTANCE_PAIR_SUM: std::cell::LazyCell<Option<&'static GpuDistancePairSum>> =
        std::cell::LazyCell::new(|| {
            GpuContext::new()
                .map(|context| Box::leak(Box::new(GpuDistancePairSum::new(&context))) as &'static GpuDistancePairSum)
        });
}

/// Sum of one-to-one squared distances on the GPU; `None` without an adapter.
pub fn squared_distance_pair_sum_gpu(a: &[V3], b: &[V3]) -> Option<f64> {
    if a.len() != b.len() {
        return None;
    }
    SHARED_DISTANCE_PAIR_SUM.with(|cell| {
        let shared: &Option<&GpuDistancePairSum> = cell;
        shared.map(|kernel| kernel.run(a, b))
    })
}

struct GpuTransformedDistancePairSum {
    device: Device,
    queue: wgpu::Queue,
    kernel: Kernel,
    buffers: std::cell::RefCell<Option<TransformedDistancePairSumBuffers>>,
}

struct TransformedDistancePairSumBuffers {
    pair_capacity: usize,
    partial_capacity: usize,
    params: Buffer,
    source: Buffer,
    target: Buffer,
    out: Buffer,
    bind: BindGroup,
}

impl GpuTransformedDistancePairSum {
    fn new(context: &GpuContext) -> Self {
        let kernel = Kernel::tuned(
            context,
            "transformed_distance_pair_sum",
            crate::TRANSFORMED_DISTANCE_PAIR_SUM_WGSL,
            "main",
            &UNIFORM_PAIR,
            WG_METAL,
            WG_DEFAULT,
        )
        .expect("transformed_distance_pair_sum kernel builds");
        Self {
            device: context.device.clone(),
            queue: context.queue.clone(),
            kernel,
            buffers: std::cell::RefCell::new(None),
        }
    }

    fn ensure_buffers(&self, pair_count: usize, partial_count: usize) {
        let stale = match &*self.buffers.borrow() {
            Some(b) => b.pair_capacity < pair_count || b.partial_capacity < partial_count,
            None => true,
        };
        if !stale {
            return;
        }
        let device = &self.device;
        let pair_capacity = pair_count.max(1);
        let partial_capacity = partial_count.max(1);
        let params = mk(
            device,
            "transformed_distance_pair_sum_params",
            64,
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        );
        let source = mk(
            device,
            "transformed_distance_pair_sum_source",
            (pair_capacity * 12) as u64,
            BufferUsages::STORAGE | BufferUsages::COPY_DST,
        );
        let target = mk(
            device,
            "transformed_distance_pair_sum_target",
            (pair_capacity * 12) as u64,
            BufferUsages::STORAGE | BufferUsages::COPY_DST,
        );
        let out = mk(
            device,
            "transformed_distance_pair_sum_out",
            (partial_capacity * 4) as u64,
            BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        );
        let bind = bind(
            device,
            &self.kernel,
            "transformed_distance_pair_sum",
            &[&params, &source, &target, &out],
        );
        *self.buffers.borrow_mut() = Some(TransformedDistancePairSumBuffers {
            pair_capacity,
            partial_capacity,
            params,
            source,
            target,
            out,
            bind,
        });
    }

    fn run(&self, source: &[V3], target: &[V3], m: M3, t: V3) -> f64 {
        let pair_count = source.len();
        let partial_count = self.kernel.workgroup_count(pair_count.max(1) as u32) as usize;
        self.ensure_buffers(pair_count, partial_count);
        let flat_source: Vec<f32> = source.iter().flatten().map(|&v| v as f32).collect();
        let flat_target: Vec<f32> = target.iter().flatten().map(|&v| v as f32).collect();
        // Params: count, pad x3, then 4 rows of (matrix row, translation comp).
        let mut params = pack_params(&[pair_count as u32, 0, 0, 0]);
        for row in 0..3 {
            for col in 0..3 {
                push_f32(&mut params, m[row][col] as f32);
            }
            push_f32(&mut params, t[row] as f32);
        }
        let buffers = self.buffers.borrow();
        let buffers = buffers.as_ref().expect("ensure_buffers was just called");
        self.queue.write_buffer(&buffers.params, 0, &params);
        self.queue
            .write_buffer(&buffers.source, 0, &pack_f32(&flat_source));
        self.queue
            .write_buffer(&buffers.target, 0, &pack_f32(&flat_target));
        self.kernel
            .dispatch_bind_group(&self.device, &self.queue, &buffers.bind, partial_count as u32);
        read_f32(&self.device, &self.queue, &buffers.out, partial_count)
            .iter()
            .map(|&dist| dist as f64)
            .sum()
    }
}

thread_local! {
    static SHARED_TRANSFORMED_DISTANCE_PAIR_SUM: std::cell::LazyCell<Option<&'static GpuTransformedDistancePairSum>> =
        std::cell::LazyCell::new(|| {
            GpuContext::new()
                .map(|context| Box::leak(Box::new(GpuTransformedDistancePairSum::new(&context))) as &'static GpuTransformedDistancePairSum)
        });
}

/// Fused transform-and-sum of one-to-one squared distances on the GPU; `None` without an adapter.
pub fn transformed_squared_distance_pair_sum_gpu(
    source: &[V3],
    target: &[V3],
    m: M3,
    t: V3,
) -> Option<f64> {
    if source.len() != target.len() {
        return None;
    }
    SHARED_TRANSFORMED_DISTANCE_PAIR_SUM.with(|cell| {
        let shared: &Option<&GpuTransformedDistancePairSum> = cell;
        shared.map(|kernel| kernel.run(source, target, m, t))
    })
}

struct GpuPointBounds {
    device: Device,
    queue: wgpu::Queue,
    kernel: Kernel,
    buffers: std::cell::RefCell<Option<PointBoundsBuffers>>,
}

struct PointBoundsBuffers {
    point_capacity: usize,
    partial_capacity: usize,
    params: Buffer,
    points: Buffer,
    out_min: Buffer,
    out_max: Buffer,
    bind: BindGroup,
}

impl GpuPointBounds {
    fn new(context: &GpuContext) -> Self {
        let kernel = Kernel::tuned(
            context,
            "point_bounds",
            crate::POINT_BOUNDS_WGSL,
            "main",
            &UNIFORM_STORAGE2,
            WG_METAL,
            WG_DEFAULT,
        )
        .expect("point_bounds kernel builds");
        Self {
            device: context.device.clone(),
            queue: context.queue.clone(),
            kernel,
            buffers: std::cell::RefCell::new(None),
        }
    }

    fn ensure_buffers(&self, point_count: usize, partial_count: usize) {
        let stale = match &*self.buffers.borrow() {
            Some(b) => b.point_capacity < point_count || b.partial_capacity < partial_count,
            None => true,
        };
        if !stale {
            return;
        }
        let device = &self.device;
        let point_capacity = point_count.max(1);
        let partial_capacity = partial_count.max(1);
        let params = mk(
            device,
            "point_bounds_params",
            16,
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        );
        let points = mk(
            device,
            "point_bounds_points",
            (point_capacity * 12) as u64,
            BufferUsages::STORAGE | BufferUsages::COPY_DST,
        );
        let partial_bytes = (partial_capacity * 12) as u64;
        let out_min = mk(
            device,
            "point_bounds_out_min",
            partial_bytes,
            BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        );
        let out_max = mk(
            device,
            "point_bounds_out_max",
            partial_bytes,
            BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        );
        let bind = bind(
            device,
            &self.kernel,
            "point_bounds",
            &[&params, &points, &out_min, &out_max],
        );
        *self.buffers.borrow_mut() = Some(PointBoundsBuffers {
            point_capacity,
            partial_capacity,
            params,
            points,
            out_min,
            out_max,
            bind,
        });
    }

    fn run(&self, points: &[V3]) -> crate::PointBounds {
        let point_count = points.len();
        let partial_count = self.kernel.workgroup_count(point_count as u32) as usize;
        self.ensure_buffers(point_count, partial_count);
        let flat: Vec<f32> = points.iter().flatten().map(|&v| v as f32).collect();
        let params = pack_params(&[point_count as u32, 0, 0, 0]);
        let buffers = self.buffers.borrow();
        let buffers = buffers.as_ref().expect("ensure_buffers was just called");
        self.queue.write_buffer(&buffers.params, 0, &params);
        self.queue
            .write_buffer(&buffers.points, 0, &pack_f32(&flat));
        self.kernel
            .dispatch_bind_group(&self.device, &self.queue, &buffers.bind, partial_count as u32);
        let raw_min = read_f32(&self.device, &self.queue, &buffers.out_min, partial_count * 3);
        let raw_max = read_f32(&self.device, &self.queue, &buffers.out_max, partial_count * 3);
        let mut min = [f64::INFINITY; 3];
        let mut max = [f64::NEG_INFINITY; 3];
        for chunk in raw_min.chunks_exact(3).take(partial_count) {
            for axis in 0..3 {
                min[axis] = min[axis].min(chunk[axis] as f64);
            }
        }
        for chunk in raw_max.chunks_exact(3).take(partial_count) {
            for axis in 0..3 {
                max[axis] = max[axis].max(chunk[axis] as f64);
            }
        }
        crate::PointBounds {
            samples: point_count,
            min,
            max,
            center: [
                0.5 * (min[0] + max[0]),
                0.5 * (min[1] + max[1]),
                0.5 * (min[2] + max[2]),
            ],
            extent: [max[0] - min[0], max[1] - min[1], max[2] - min[2]],
        }
    }
}

thread_local! {
    static SHARED_POINT_BOUNDS: std::cell::LazyCell<Option<&'static GpuPointBounds>> =
        std::cell::LazyCell::new(|| {
            GpuContext::new()
                .map(|context| Box::leak(Box::new(GpuPointBounds::new(&context))) as &'static GpuPointBounds)
        });
}

/// Point-cloud axis-aligned bounds on the GPU; `None` without an adapter.
pub fn point_bounds_gpu(points: &[V3]) -> Option<crate::PointBounds> {
    if points.is_empty() {
        return None;
    }
    SHARED_POINT_BOUNDS.with(|cell| {
        let shared: &Option<&GpuPointBounds> = cell;
        shared.map(|kernel| kernel.run(points))
    })
}

struct GpuTransformedPointBounds {
    device: Device,
    queue: wgpu::Queue,
    kernel: Kernel,
    buffers: std::cell::RefCell<Option<TransformedPointBoundsBuffers>>,
}

struct TransformedPointBoundsBuffers {
    point_capacity: usize,
    partial_capacity: usize,
    params: Buffer,
    points: Buffer,
    out_min: Buffer,
    out_max: Buffer,
    bind: BindGroup,
}

impl GpuTransformedPointBounds {
    fn new(context: &GpuContext) -> Self {
        let kernel = Kernel::tuned(
            context,
            "transformed_point_bounds",
            crate::TRANSFORMED_POINT_BOUNDS_WGSL,
            "main",
            &UNIFORM_STORAGE2,
            WG_METAL,
            WG_DEFAULT,
        )
        .expect("transformed_point_bounds kernel builds");
        Self {
            device: context.device.clone(),
            queue: context.queue.clone(),
            kernel,
            buffers: std::cell::RefCell::new(None),
        }
    }

    fn ensure_buffers(&self, point_count: usize, partial_count: usize) {
        let stale = match &*self.buffers.borrow() {
            Some(b) => b.point_capacity < point_count || b.partial_capacity < partial_count,
            None => true,
        };
        if !stale {
            return;
        }
        let device = &self.device;
        let point_capacity = point_count.max(1);
        let partial_capacity = partial_count.max(1);
        let params = mk(
            device,
            "transformed_point_bounds_params",
            64,
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        );
        let points = mk(
            device,
            "transformed_point_bounds_points",
            (point_capacity * 12) as u64,
            BufferUsages::STORAGE | BufferUsages::COPY_DST,
        );
        let partial_bytes = (partial_capacity * 12) as u64;
        let out_min = mk(
            device,
            "transformed_point_bounds_out_min",
            partial_bytes,
            BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        );
        let out_max = mk(
            device,
            "transformed_point_bounds_out_max",
            partial_bytes,
            BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        );
        let bind = bind(
            device,
            &self.kernel,
            "transformed_point_bounds",
            &[&params, &points, &out_min, &out_max],
        );
        *self.buffers.borrow_mut() = Some(TransformedPointBoundsBuffers {
            point_capacity,
            partial_capacity,
            params,
            points,
            out_min,
            out_max,
            bind,
        });
    }

    fn run(&self, points: &[V3], m: M3, t: V3) -> crate::PointBounds {
        let point_count = points.len();
        let partial_count = self.kernel.workgroup_count(point_count as u32) as usize;
        self.ensure_buffers(point_count, partial_count);
        let flat: Vec<f32> = points.iter().flatten().map(|&v| v as f32).collect();
        let mut params = pack_params(&[point_count as u32, 0, 0, 0]);
        for row in 0..3 {
            for col in 0..3 {
                push_f32(&mut params, m[row][col] as f32);
            }
            push_f32(&mut params, t[row] as f32);
        }
        let buffers = self.buffers.borrow();
        let buffers = buffers.as_ref().expect("ensure_buffers was just called");
        self.queue.write_buffer(&buffers.params, 0, &params);
        self.queue
            .write_buffer(&buffers.points, 0, &pack_f32(&flat));
        self.kernel
            .dispatch_bind_group(&self.device, &self.queue, &buffers.bind, partial_count as u32);
        let raw_min = read_f32(&self.device, &self.queue, &buffers.out_min, partial_count * 3);
        let raw_max = read_f32(&self.device, &self.queue, &buffers.out_max, partial_count * 3);
        let mut min = [f64::INFINITY; 3];
        let mut max = [f64::NEG_INFINITY; 3];
        for chunk in raw_min.chunks_exact(3).take(partial_count) {
            for axis in 0..3 {
                min[axis] = min[axis].min(chunk[axis] as f64);
            }
        }
        for chunk in raw_max.chunks_exact(3).take(partial_count) {
            for axis in 0..3 {
                max[axis] = max[axis].max(chunk[axis] as f64);
            }
        }
        crate::PointBounds::new(point_count, min, max)
    }
}

thread_local! {
    static SHARED_TRANSFORMED_POINT_BOUNDS: std::cell::LazyCell<Option<&'static GpuTransformedPointBounds>> =
        std::cell::LazyCell::new(|| {
            GpuContext::new()
                .map(|context| Box::leak(Box::new(GpuTransformedPointBounds::new(&context))) as &'static GpuTransformedPointBounds)
        });
}

/// Transformed point-cloud axis-aligned bounds on the GPU; `None` without an adapter.
pub fn transformed_point_bounds_gpu(points: &[V3], m: M3, t: V3) -> Option<crate::PointBounds> {
    if points.is_empty() {
        return None;
    }
    SHARED_TRANSFORMED_POINT_BOUNDS.with(|cell| {
        let shared: &Option<&GpuTransformedPointBounds> = cell;
        shared.map(|kernel| kernel.run(points, m, t))
    })
}

/// Uniform + read points + one partial-output reduction.
const UNIFORM_REDUCE1: [Binding; 3] = [
    Binding::Uniform,
    Binding::StorageRead,
    Binding::StorageReadWrite,
];

struct GpuPointMoments {
    device: Device,
    queue: wgpu::Queue,
    kernel: Kernel,
    buffers: std::cell::RefCell<Option<PointMomentsBuffers>>,
}

struct PointMomentsBuffers {
    point_capacity: usize,
    partial_capacity: usize,
    params: Buffer,
    points: Buffer,
    out: Buffer,
    bind: BindGroup,
}

impl GpuPointMoments {
    fn new(context: &GpuContext) -> Self {
        let kernel = Kernel::tuned(
            context,
            "point_moments",
            crate::POINT_MOMENTS_WGSL,
            "main",
            &UNIFORM_REDUCE1,
            WG_METAL,
            WG_DEFAULT,
        )
        .expect("point_moments kernel builds");
        Self {
            device: context.device.clone(),
            queue: context.queue.clone(),
            kernel,
            buffers: std::cell::RefCell::new(None),
        }
    }

    fn ensure_buffers(&self, point_count: usize, partial_count: usize) {
        let stale = match &*self.buffers.borrow() {
            Some(b) => b.point_capacity < point_count || b.partial_capacity < partial_count,
            None => true,
        };
        if !stale {
            return;
        }
        let device = &self.device;
        let point_capacity = point_count.max(1);
        let partial_capacity = partial_count.max(1);
        let params = mk(
            device,
            "point_moments_params",
            16,
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        );
        let points = mk(
            device,
            "point_moments_points",
            (point_capacity * 12) as u64,
            BufferUsages::STORAGE | BufferUsages::COPY_DST,
        );
        let partial_bytes = (partial_capacity * 9 * 4) as u64;
        let out = mk(
            device,
            "point_moments_out",
            partial_bytes,
            BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        );
        let bind = bind(device, &self.kernel, "point_moments", &[&params, &points, &out]);
        *self.buffers.borrow_mut() = Some(PointMomentsBuffers {
            point_capacity,
            partial_capacity,
            params,
            points,
            out,
            bind,
        });
    }

    fn run(&self, points: &[V3]) -> crate::PointMoments {
        let point_count = points.len();
        let partial_count = self.kernel.workgroup_count(point_count as u32) as usize;
        self.ensure_buffers(point_count, partial_count);
        let flat: Vec<f32> = points.iter().flatten().map(|&v| v as f32).collect();
        let params = pack_params(&[point_count as u32, 0, 0, 0]);
        let buffers = self.buffers.borrow();
        let buffers = buffers.as_ref().expect("ensure_buffers was just called");
        self.queue.write_buffer(&buffers.params, 0, &params);
        self.queue
            .write_buffer(&buffers.points, 0, &pack_f32(&flat));
        self.kernel
            .dispatch_bind_group(&self.device, &self.queue, &buffers.bind, partial_count as u32);
        let raw = read_f32(&self.device, &self.queue, &buffers.out, partial_count * 9);
        let mut accum = [0.; 9];
        for chunk in raw.chunks_exact(9).take(partial_count) {
            for item in 0..9 {
                accum[item] += chunk[item] as f64;
            }
        }
        crate::PointMoments::from_sums(
            point_count,
            [accum[0], accum[1], accum[2]],
            [accum[3], accum[4], accum[5], accum[6], accum[7], accum[8]],
        )
    }
}

thread_local! {
    static SHARED_POINT_MOMENTS: std::cell::LazyCell<Option<&'static GpuPointMoments>> =
        std::cell::LazyCell::new(|| {
            GpuContext::new()
                .map(|context| Box::leak(Box::new(GpuPointMoments::new(&context))) as &'static GpuPointMoments)
        });
}

/// Point-cloud centroid/covariance reduction on the GPU; `None` without an adapter.
pub fn point_moments_gpu(points: &[V3]) -> Option<crate::PointMoments> {
    if points.is_empty() {
        return None;
    }
    SHARED_POINT_MOMENTS.with(|cell| {
        let shared: &Option<&GpuPointMoments> = cell;
        shared.map(|kernel| kernel.run(points))
    })
}

struct GpuPointCloudStats {
    device: Device,
    queue: wgpu::Queue,
    kernel: Kernel,
    buffers: std::cell::RefCell<Option<PointCloudStatsBuffers>>,
}

struct PointCloudStatsBuffers {
    point_capacity: usize,
    partial_capacity: usize,
    params: Buffer,
    points: Buffer,
    out: Buffer,
    bind: BindGroup,
}

impl GpuPointCloudStats {
    fn new(context: &GpuContext) -> Self {
        let kernel = Kernel::tuned(
            context,
            "point_cloud_stats",
            crate::POINT_CLOUD_STATS_WGSL,
            "main",
            &UNIFORM_REDUCE1,
            WG_METAL,
            WG_DEFAULT,
        )
        .expect("point_cloud_stats kernel builds");
        Self {
            device: context.device.clone(),
            queue: context.queue.clone(),
            kernel,
            buffers: std::cell::RefCell::new(None),
        }
    }

    fn ensure_buffers(&self, point_count: usize, partial_count: usize) {
        let stale = match &*self.buffers.borrow() {
            Some(b) => b.point_capacity < point_count || b.partial_capacity < partial_count,
            None => true,
        };
        if !stale {
            return;
        }
        let device = &self.device;
        let point_capacity = point_count.max(1);
        let partial_capacity = partial_count.max(1);
        let params = mk(
            device,
            "point_cloud_stats_params",
            16,
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        );
        let points = mk(
            device,
            "point_cloud_stats_points",
            (point_capacity * 12) as u64,
            BufferUsages::STORAGE | BufferUsages::COPY_DST,
        );
        let partial_bytes = (partial_capacity * 15 * 4) as u64;
        let out = mk(
            device,
            "point_cloud_stats_out",
            partial_bytes,
            BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        );
        let bind = bind(
            device,
            &self.kernel,
            "point_cloud_stats",
            &[&params, &points, &out],
        );
        *self.buffers.borrow_mut() = Some(PointCloudStatsBuffers {
            point_capacity,
            partial_capacity,
            params,
            points,
            out,
            bind,
        });
    }

    fn run(&self, points: &[V3]) -> crate::PointCloudStats {
        let point_count = points.len();
        let partial_count = self.kernel.workgroup_count(point_count as u32) as usize;
        self.ensure_buffers(point_count, partial_count);
        let flat: Vec<f32> = points.iter().flatten().map(|&v| v as f32).collect();
        let params = pack_params(&[point_count as u32, 0, 0, 0]);
        let buffers = self.buffers.borrow();
        let buffers = buffers.as_ref().expect("ensure_buffers was just called");
        self.queue.write_buffer(&buffers.params, 0, &params);
        self.queue
            .write_buffer(&buffers.points, 0, &pack_f32(&flat));
        self.kernel
            .dispatch_bind_group(&self.device, &self.queue, &buffers.bind, partial_count as u32);
        let raw = read_f32(&self.device, &self.queue, &buffers.out, partial_count * 15);
        let mut min = [f64::INFINITY; 3];
        let mut max = [f64::NEG_INFINITY; 3];
        let mut accum = [0.; 9];
        for chunk in raw.chunks_exact(15).take(partial_count) {
            for axis in 0..3 {
                min[axis] = min[axis].min(chunk[axis] as f64);
                max[axis] = max[axis].max(chunk[axis + 3] as f64);
            }
            for item in 0..9 {
                accum[item] += chunk[item + 6] as f64;
            }
        }
        let bounds = crate::PointBounds::new(point_count, min, max);
        let moments = crate::PointMoments::from_sums(
            point_count,
            [accum[0], accum[1], accum[2]],
            [accum[3], accum[4], accum[5], accum[6], accum[7], accum[8]],
        );
        crate::PointCloudStats::from_parts(bounds, moments)
    }
}

thread_local! {
    static SHARED_POINT_CLOUD_STATS: std::cell::LazyCell<Option<&'static GpuPointCloudStats>> =
        std::cell::LazyCell::new(|| {
            GpuContext::new()
                .map(|context| Box::leak(Box::new(GpuPointCloudStats::new(&context))) as &'static GpuPointCloudStats)
        });
}

/// Fused point-cloud bounds + moments reduction on the GPU; `None` without an adapter.
pub fn point_cloud_stats_gpu(points: &[V3]) -> Option<crate::PointCloudStats> {
    if points.is_empty() {
        return None;
    }
    SHARED_POINT_CLOUD_STATS.with(|cell| {
        let shared: &Option<&GpuPointCloudStats> = cell;
        shared.map(|kernel| kernel.run(points))
    })
}

struct GpuChamfer {
    device: Device,
    queue: wgpu::Queue,
    kernel: Kernel,
    buffers: std::cell::RefCell<Option<ChamferBuffers>>,
}

struct ChamferBuffers {
    query_capacity: usize,
    target_capacity: usize,
    partial_capacity: usize,
    params: Buffer,
    queries: Buffer,
    targets: Buffer,
    out_sum: Buffer,
    out_max: Buffer,
    bind: BindGroup,
}

impl GpuChamfer {
    fn new(context: &GpuContext) -> Self {
        let kernel = Kernel::tuned(
            context,
            "chamfer",
            crate::CHAMFER_WGSL,
            "main",
            &NN_BINDINGS,
            WG_METAL,
            WG_DEFAULT,
        )
        .expect("chamfer kernel builds");
        Self {
            device: context.device.clone(),
            queue: context.queue.clone(),
            kernel,
            buffers: std::cell::RefCell::new(None),
        }
    }

    fn ensure_buffers(&self, query_count: usize, target_count: usize, partial_count: usize) {
        let stale = match &*self.buffers.borrow() {
            Some(b) => {
                b.query_capacity < query_count
                    || b.target_capacity < target_count
                    || b.partial_capacity < partial_count
            }
            None => true,
        };
        if !stale {
            return;
        }
        let device = &self.device;
        let query_capacity = query_count.max(1);
        let target_capacity = target_count.max(1);
        let partial_capacity = partial_count.max(1);
        let params = mk(
            device,
            "chamfer_params",
            16,
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        );
        let queries = mk(
            device,
            "chamfer_queries",
            (query_capacity * 12) as u64,
            BufferUsages::STORAGE | BufferUsages::COPY_DST,
        );
        let targets = mk(
            device,
            "chamfer_targets",
            (target_capacity * 12) as u64,
            BufferUsages::STORAGE | BufferUsages::COPY_DST,
        );
        let partial_bytes = (partial_capacity * 4) as u64;
        let out_sum = mk(
            device,
            "chamfer_out_sum",
            partial_bytes,
            BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        );
        let out_max = mk(
            device,
            "chamfer_out_max",
            partial_bytes,
            BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        );
        let bind = bind(
            device,
            &self.kernel,
            "chamfer",
            &[&params, &queries, &targets, &out_sum, &out_max],
        );
        *self.buffers.borrow_mut() = Some(ChamferBuffers {
            query_capacity,
            target_capacity,
            partial_capacity,
            params,
            queries,
            targets,
            out_sum,
            out_max,
            bind,
        });
    }

    fn run(&self, queries: &[V3], targets: &[V3]) -> crate::DirectedChamfer {
        let query_count = queries.len();
        let target_count = targets.len();
        let partial_count = self.kernel.workgroup_count(query_count as u32) as usize;
        self.ensure_buffers(query_count, target_count, partial_count);
        let flat_q: Vec<f32> = queries.iter().flatten().map(|&v| v as f32).collect();
        let flat_t: Vec<f32> = targets.iter().flatten().map(|&v| v as f32).collect();
        let params = pack_params(&[query_count as u32, target_count as u32, 0, 0]);
        let buffers = self.buffers.borrow();
        let buffers = buffers.as_ref().expect("ensure_buffers was just called");
        self.queue.write_buffer(&buffers.params, 0, &params);
        self.queue
            .write_buffer(&buffers.queries, 0, &pack_f32(&flat_q));
        self.queue
            .write_buffer(&buffers.targets, 0, &pack_f32(&flat_t));
        self.kernel
            .dispatch_bind_group(&self.device, &self.queue, &buffers.bind, partial_count as u32);
        let raw_sum = read_f32(&self.device, &self.queue, &buffers.out_sum, partial_count);
        let raw_max = read_f32(&self.device, &self.queue, &buffers.out_max, partial_count);
        let sum: f64 = raw_sum.iter().map(|&v| v as f64).sum();
        let max_squared_distance = raw_max
            .iter()
            .map(|&v| v as f64)
            .fold(0., f64::max);
        let mean_squared_distance = sum / query_count as f64;
        crate::DirectedChamfer {
            samples: query_count,
            mean_squared_distance,
            rms_distance: mean_squared_distance.sqrt(),
            max_squared_distance,
        }
    }
}

thread_local! {
    static SHARED_CHAMFER: std::cell::LazyCell<Option<&'static GpuChamfer>> =
        std::cell::LazyCell::new(|| {
            GpuContext::new()
                .map(|context| Box::leak(Box::new(GpuChamfer::new(&context))) as &'static GpuChamfer)
        });
}

/// Directed Chamfer partial reduction on the GPU; `None` without an adapter.
pub fn directed_chamfer_gpu(queries: &[V3], targets: &[V3]) -> Option<crate::DirectedChamfer> {
    if queries.is_empty() || targets.is_empty() {
        return None;
    }
    SHARED_CHAMFER.with(|cell| {
        let shared: &Option<&GpuChamfer> = cell;
        shared.map(|kernel| kernel.run(queries, targets))
    })
}

struct GpuNearestTwo {
    device: Device,
    queue: wgpu::Queue,
    kernel: Kernel,
    buffers: std::cell::RefCell<Option<NearestTwoBuffers>>,
}

struct NearestTwoBuffers {
    query_capacity: usize,
    target_capacity: usize,
    params: Buffer,
    queries: Buffer,
    targets: Buffer,
    out_i0: Buffer,
    out_d0: Buffer,
    out_i1: Buffer,
    out_d1: Buffer,
    bind: BindGroup,
}

impl GpuNearestTwo {
    fn new(context: &GpuContext) -> Self {
        let kernel = Kernel::tuned(
            context,
            "nearest_two",
            crate::NEAREST_TWO_WGSL,
            "main",
            &[
                Binding::Uniform,
                Binding::StorageRead,
                Binding::StorageRead,
                Binding::StorageReadWrite,
                Binding::StorageReadWrite,
                Binding::StorageReadWrite,
                Binding::StorageReadWrite,
            ],
            WG_METAL,
            WG_DEFAULT,
        )
        .expect("nearest_two kernel builds");
        Self {
            device: context.device.clone(),
            queue: context.queue.clone(),
            kernel,
            buffers: std::cell::RefCell::new(None),
        }
    }

    fn ensure_buffers(&self, query_count: usize, target_count: usize) {
        let stale = match &*self.buffers.borrow() {
            Some(b) => b.query_capacity < query_count || b.target_capacity < target_count,
            None => true,
        };
        if !stale {
            return;
        }
        let device = &self.device;
        let query_capacity = query_count.max(1);
        let target_capacity = target_count.max(1);
        let params = mk(
            device,
            "nearest_two_params",
            16,
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        );
        let queries = mk(
            device,
            "nearest_two_queries",
            (query_capacity * 12) as u64,
            BufferUsages::STORAGE | BufferUsages::COPY_DST,
        );
        let targets = mk(
            device,
            "nearest_two_targets",
            (target_capacity * 12) as u64,
            BufferUsages::STORAGE | BufferUsages::COPY_DST,
        );
        let out_bytes = (query_capacity * 4) as u64;
        let outs: [Buffer; 4] = std::array::from_fn(|k| {
            mk(
                device,
                &["nearest_two_i0", "nearest_two_d0", "nearest_two_i1", "nearest_two_d1"][k],
                out_bytes,
                BufferUsages::STORAGE | BufferUsages::COPY_SRC,
            )
        });
        let bind = bind(
            device,
            &self.kernel,
            "nearest_two",
            &[
                &params,
                &queries,
                &targets,
                &outs[0],
                &outs[1],
                &outs[2],
                &outs[3],
            ],
        );
        *self.buffers.borrow_mut() = Some(NearestTwoBuffers {
            query_capacity,
            target_capacity,
            params,
            queries,
            targets,
            out_i0: outs[0].clone(),
            out_d0: outs[1].clone(),
            out_i1: outs[2].clone(),
            out_d1: outs[3].clone(),
            bind,
        });
    }

    fn run(&self, queries: &[V3], targets: &[V3]) -> Vec<crate::TwoNearest> {
        let query_count = queries.len();
        let target_count = targets.len();
        let flat_q: Vec<f32> = queries.iter().flatten().map(|&v| v as f32).collect();
        let flat_t: Vec<f32> = targets.iter().flatten().map(|&v| v as f32).collect();
        self.ensure_buffers(query_count, target_count);
        let buffers = self.buffers.borrow();
        let b = buffers.as_ref().expect("ensure_buffers was just called");
        let params = pack_params(&[query_count as u32, target_count as u32, 0, 0]);
        self.queue.write_buffer(&b.params, 0, &params);
        self.queue
            .write_buffer(&b.queries, 0, &pack_f32(&flat_q));
        self.queue
            .write_buffer(&b.targets, 0, &pack_f32(&flat_t));
        self.kernel.dispatch_bind_group(
            &self.device,
            &self.queue,
            &b.bind,
            self.kernel.workgroup_count(query_count.max(1) as u32),
        );
        let i0 = read_u32(&self.device, &self.queue, &b.out_i0, query_count);
        let d0 = read_f32(&self.device, &self.queue, &b.out_d0, query_count);
        let i1 = read_u32(&self.device, &self.queue, &b.out_i1, query_count);
        let d1 = read_f32(&self.device, &self.queue, &b.out_d1, query_count);
        (0..query_count)
            .map(|idx| {
                [
                    (i0[idx], d0[idx] as f64),
                    (i1[idx], d1[idx] as f64),
                ]
            })
            .collect()
    }
}

thread_local! {
    static SHARED_NEAREST_TWO: std::cell::LazyCell<Option<&'static GpuNearestTwo>> =
        std::cell::LazyCell::new(|| {
            GpuContext::new()
                .map(|context| Box::leak(Box::new(GpuNearestTwo::new(&context))) as &'static GpuNearestTwo)
        });
}

pub fn nearest_two_gpu(queries: &[V3], targets: &[V3]) -> Option<Vec<crate::TwoNearest>> {
    SHARED_NEAREST_TWO.with(|cell| {
        let shared: &Option<&GpuNearestTwo> = cell;
        shared.map(|kernel| kernel.run(queries, targets))
    })
}

struct GpuNearestFour {
    device: Device,
    queue: wgpu::Queue,
    kernel: Kernel,
    buffers: std::cell::RefCell<Option<NearestFourBuffers>>,
}

struct NearestFourBuffers {
    query_capacity: usize,
    target_capacity: usize,
    params: Buffer,
    queries: Buffer,
    targets: Buffer,
    out_indices: Buffer,
    out_distances: Buffer,
    bind: BindGroup,
}

impl GpuNearestFour {
    fn new(context: &GpuContext) -> Self {
        let kernel = Kernel::tuned(
            context,
            "nearest_four",
            crate::NEAREST_FOUR_WGSL,
            "main",
            &NN_BINDINGS,
            WG_METAL,
            WG_DEFAULT,
        )
        .expect("nearest_four kernel builds");
        Self {
            device: context.device.clone(),
            queue: context.queue.clone(),
            kernel,
            buffers: std::cell::RefCell::new(None),
        }
    }

    fn ensure_buffers(&self, query_count: usize, target_count: usize) {
        let stale = match &*self.buffers.borrow() {
            Some(b) => b.query_capacity < query_count || b.target_capacity < target_count,
            None => true,
        };
        if !stale {
            return;
        }
        let device = &self.device;
        let query_capacity = query_count.max(1);
        let target_capacity = target_count.max(1);
        let params = mk(
            device,
            "nearest_four_params",
            16,
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        );
        let queries = mk(
            device,
            "nearest_four_queries",
            (query_capacity * 12) as u64,
            BufferUsages::STORAGE | BufferUsages::COPY_DST,
        );
        let targets = mk(
            device,
            "nearest_four_targets",
            (target_capacity * 12) as u64,
            BufferUsages::STORAGE | BufferUsages::COPY_DST,
        );
        let out_bytes = (query_capacity * 4 * 4) as u64;
        let out_indices = mk(
            device,
            "nearest_four_indices",
            out_bytes,
            BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        );
        let out_distances = mk(
            device,
            "nearest_four_distances",
            out_bytes,
            BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        );
        let bind = bind(
            device,
            &self.kernel,
            "nearest_four",
            &[&params, &queries, &targets, &out_indices, &out_distances],
        );
        *self.buffers.borrow_mut() = Some(NearestFourBuffers {
            query_capacity,
            target_capacity,
            params,
            queries,
            targets,
            out_indices,
            out_distances,
            bind,
        });
    }

    fn run(&self, queries: &[V3], targets: &[V3]) -> Vec<crate::FourNearest> {
        let query_count = queries.len();
        let target_count = targets.len();
        let flat_q: Vec<f32> = queries.iter().flatten().map(|&v| v as f32).collect();
        let flat_t: Vec<f32> = targets.iter().flatten().map(|&v| v as f32).collect();
        self.ensure_buffers(query_count, target_count);
        let buffers = self.buffers.borrow();
        let b = buffers.as_ref().expect("ensure_buffers was just called");
        let params = pack_params(&[query_count as u32, target_count as u32, 0, 0]);
        self.queue.write_buffer(&b.params, 0, &params);
        self.queue
            .write_buffer(&b.queries, 0, &pack_f32(&flat_q));
        self.queue
            .write_buffer(&b.targets, 0, &pack_f32(&flat_t));
        self.kernel.dispatch_bind_group(
            &self.device,
            &self.queue,
            &b.bind,
            self.kernel.workgroup_count(query_count.max(1) as u32),
        );
        let indices = read_u32(&self.device, &self.queue, &b.out_indices, query_count * 4);
        let distances = read_f32(&self.device, &self.queue, &b.out_distances, query_count * 4);
        (0..query_count)
            .map(|idx| {
                std::array::from_fn(|k| {
                    let off = idx * 4 + k;
                    (indices[off], distances[off] as f64)
                })
            })
            .collect()
    }
}

thread_local! {
    static SHARED_NEAREST_FOUR: std::cell::LazyCell<Option<&'static GpuNearestFour>> =
        std::cell::LazyCell::new(|| {
            GpuContext::new()
                .map(|context| Box::leak(Box::new(GpuNearestFour::new(&context))) as &'static GpuNearestFour)
        });
}

pub fn nearest_four_gpu(queries: &[V3], targets: &[V3]) -> Option<Vec<crate::FourNearest>> {
    SHARED_NEAREST_FOUR.with(|cell| {
        let shared: &Option<&GpuNearestFour> = cell;
        shared.map(|kernel| kernel.run(queries, targets))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_nearest_neighbor_matches_cpu_reference_when_available() {
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
        let Some(got) = nearest_neighbor_gpu(&queries, &targets) else {
            eprintln!("no wgpu adapter available; skipping");
            return;
        };
        let want = crate::nearest_neighbor(&queries, &targets);
        assert_eq!(got.len(), want.len());
        for ((_gi, gd), (_wi, wd)) in got.iter().zip(&want) {
            assert!((gd - wd).abs() < 5e-3 * wd.max(1.0), "{gd} vs {wd}");
        }
    }

    #[test]
    fn gpu_nearest_neighbor_empty_input() {
        if let Some(got) = nearest_neighbor_gpu(&[], &[[0., 0., 0.]]) {
            assert!(got.is_empty());
        }
    }

    #[test]
    fn gpu_squared_distance_pairs_matches_cpu_reference_when_available() {
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
        let Some(got) = squared_distance_pairs_gpu(&a, &b) else {
            eprintln!("no wgpu adapter available; skipping");
            return;
        };
        let want = crate::squared_distance_pairs(&a, &b).unwrap();
        for (got, want) in got.iter().zip(&want) {
            assert!((got - want).abs() < 1e-4 * want.max(1.0), "{got} vs {want}");
        }
    }

    #[test]
    fn gpu_squared_distance_pair_sum_matches_cpu_reference_when_available() {
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
        let Some(got) = squared_distance_pair_sum_gpu(&a, &b) else {
            eprintln!("no wgpu adapter available; skipping");
            return;
        };
        let want = crate::squared_distance_pair_sum(&a, &b).unwrap();
        assert!((got - want).abs() < 1e-4 * want.max(1.0), "{got} vs {want}");
    }
}
