//! GPU nearest-neighbor search (feature `gpu`): for every query point, finds
//! the index and squared distance of the closest target point by brute
//! force. f32 arithmetic — see [`crate::Acceleration`].
//!
//! Device buffers are cached per query/target capacity (grow-only) so
//! repeated calls at a stable size amortize allocation; only the bytes
//! actually written/read for the current call cross the wire.
use crate::V3;
use gpu_compute::{BackendReport, GpuContext, read_buffer, storage_entry, uniform_entry, wgpu};

const WG_METAL: u32 = 128;
const WG_DEFAULT: u32 = 256;

struct Buffers {
    query_capacity: usize,
    target_capacity: usize,
    params: wgpu::Buffer,
    queries: wgpu::Buffer,
    targets: wgpu::Buffer,
    out_index: wgpu::Buffer,
    out_dist: wgpu::Buffer,
    read_index: wgpu::Buffer,
    read_dist: wgpu::Buffer,
    bind: wgpu::BindGroup,
}

struct GpuNearestNeighbor {
    device: wgpu::Device,
    queue: wgpu::Queue,
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::ComputePipeline,
    buffers: std::cell::RefCell<Option<Buffers>>,
}

impl GpuNearestNeighbor {
    fn new(context: &GpuContext) -> Self {
        let device = &context.device;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("nearest_neighbor"),
            source: wgpu::ShaderSource::Wgsl(crate::NEAREST_NEIGHBOR_WGSL.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("nearest_neighbor"),
            entries: &[
                uniform_entry(0),
                storage_entry(1, true),
                storage_entry(2, true),
                storage_entry(3, false),
                storage_entry(4, false),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("nearest_neighbor"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("nearest_neighbor"),
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
        let params = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nn_params"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let queries = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nn_queries"),
            size: (query_capacity * 12) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let targets = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nn_targets"),
            size: (target_capacity * 12) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let out_index = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nn_out_index"),
            size: (query_capacity * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let out_dist = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nn_out_dist"),
            size: (query_capacity * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let read_index = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nn_read_index"),
            size: (query_capacity * 4) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let read_dist = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nn_read_dist"),
            size: (query_capacity * 4) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nearest_neighbor"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: queries.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: targets.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: out_index.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: out_dist.as_entire_binding(),
                },
            ],
        });
        *self.buffers.borrow_mut() = Some(Buffers {
            query_capacity,
            target_capacity,
            params,
            queries,
            targets,
            out_index,
            out_dist,
            read_index,
            read_dist,
            bind,
        });
    }

    fn run(&self, queries: &[V3], targets: &[V3]) -> Vec<(u32, f64)> {
        let query_count = queries.len();
        let target_count = targets.len();
        self.ensure_buffers(query_count, target_count);
        // Uniform layout matches nearest_neighbor.wgsl's `Params`: two counts
        // padded to a 16-byte uniform buffer.
        let mut params = Vec::with_capacity(16);
        params.extend_from_slice(&(query_count as u32).to_le_bytes());
        params.extend_from_slice(&(target_count as u32).to_le_bytes());
        params.extend_from_slice(&0u32.to_le_bytes());
        params.extend_from_slice(&0u32.to_le_bytes());
        let flat_q: Vec<f32> = queries.iter().flatten().map(|&v| v as f32).collect();
        let flat_t: Vec<f32> = targets.iter().flatten().map(|&v| v as f32).collect();

        let buffers = self.buffers.borrow();
        let b = buffers.as_ref().expect("ensure_buffers was just called");
        self.queue.write_buffer(&b.params, 0, &params);
        if !flat_q.is_empty() {
            self.queue
                .write_buffer(&b.queries, 0, &gpu_compute::pack_f32(&flat_q));
        }
        if !flat_t.is_empty() {
            self.queue
                .write_buffer(&b.targets, 0, &gpu_compute::pack_f32(&flat_t));
        }
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("nearest_neighbor"),
            });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("nearest_neighbor"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &b.bind, &[]);
            pass.dispatch_workgroups((query_count.max(1) as u32).div_ceil(256), 1, 1);
        }
        let out_bytes = (query_count * 4) as u64;
        if out_bytes > 0 {
            encoder.copy_buffer_to_buffer(&b.out_index, 0, &b.read_index, 0, out_bytes);
            encoder.copy_buffer_to_buffer(&b.out_dist, 0, &b.read_dist, 0, out_bytes);
        }
        self.queue.submit([encoder.finish()]);
        if query_count == 0 {
            return Vec::new();
        }
        let raw_index = read_buffer(&self.device, &b.read_index, query_count * 4);
        b.read_index.unmap();
        let raw_dist = read_buffer(&self.device, &b.read_dist, query_count * 4);
        b.read_dist.unmap();
        raw_index
            .chunks_exact(4)
            .zip(raw_dist.chunks_exact(4))
            .take(query_count)
            .map(|(i, d)| {
                (
                    u32::from_ne_bytes(i.try_into().unwrap()),
                    f32::from_ne_bytes(d.try_into().unwrap()) as f64,
                )
            })
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

struct DistancePairBuffers {
    capacity: usize,
    params: wgpu::Buffer,
    a: wgpu::Buffer,
    b: wgpu::Buffer,
    out: wgpu::Buffer,
    read: wgpu::Buffer,
    bind: wgpu::BindGroup,
}

struct GpuDistancePairs {
    device: wgpu::Device,
    queue: wgpu::Queue,
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::ComputePipeline,
    buffers: std::cell::RefCell<Option<DistancePairBuffers>>,
}

impl GpuDistancePairs {
    fn new(context: &GpuContext) -> Self {
        let device = &context.device;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("distance_pairs"),
            source: wgpu::ShaderSource::Wgsl(crate::DISTANCE_PAIRS_WGSL.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("distance_pairs"),
            entries: &[
                uniform_entry(0),
                storage_entry(1, true),
                storage_entry(2, true),
                storage_entry(3, false),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("distance_pairs"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("distance_pairs"),
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

    fn ensure_buffers(&self, pair_count: usize) {
        let stale = match &*self.buffers.borrow() {
            Some(buffers) => buffers.capacity < pair_count,
            None => true,
        };
        if !stale {
            return;
        }
        let capacity = pair_count.max(1);
        let device = &self.device;
        let mk = |label: &str, size: u64, usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage,
                mapped_at_creation: false,
            })
        };
        let params = mk(
            "distance_pairs_params",
            16,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );
        let a = mk(
            "distance_pairs_a",
            (capacity * 12) as u64,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let b = mk(
            "distance_pairs_b",
            (capacity * 12) as u64,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let out = mk(
            "distance_pairs_out",
            (capacity * 4) as u64,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        );
        let read = mk(
            "distance_pairs_read",
            (capacity * 4) as u64,
            wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        );
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("distance_pairs"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: a.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: b.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: out.as_entire_binding(),
                },
            ],
        });
        *self.buffers.borrow_mut() = Some(DistancePairBuffers {
            capacity,
            params,
            a,
            b,
            out,
            read,
            bind,
        });
    }

    fn run(&self, a: &[V3], b: &[V3]) -> Vec<f64> {
        let pair_count = a.len();
        self.ensure_buffers(pair_count);
        let flat_a: Vec<f32> = a.iter().flatten().map(|&v| v as f32).collect();
        let flat_b: Vec<f32> = b.iter().flatten().map(|&v| v as f32).collect();
        let mut params = Vec::with_capacity(16);
        params.extend_from_slice(&(pair_count as u32).to_le_bytes());
        params.extend_from_slice(&0u32.to_le_bytes());
        params.extend_from_slice(&0u32.to_le_bytes());
        params.extend_from_slice(&0u32.to_le_bytes());
        let device = &self.device;
        let buffers = self.buffers.borrow();
        let buffers = buffers.as_ref().expect("ensure_buffers was just called");
        self.queue.write_buffer(&buffers.params, 0, &params);
        self.queue
            .write_buffer(&buffers.a, 0, &gpu_compute::pack_f32(&flat_a));
        self.queue
            .write_buffer(&buffers.b, 0, &gpu_compute::pack_f32(&flat_b));
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("distance_pairs"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("distance_pairs"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &buffers.bind, &[]);
            pass.dispatch_workgroups((pair_count.max(1) as u32).div_ceil(256), 1, 1);
        }
        encoder.copy_buffer_to_buffer(&buffers.out, 0, &buffers.read, 0, (pair_count * 4) as u64);
        self.queue.submit([encoder.finish()]);
        let raw = read_buffer(device, &buffers.read, pair_count * 4);
        buffers.read.unmap();
        raw.chunks_exact(4)
            .take(pair_count)
            .map(|d| f32::from_ne_bytes(d.try_into().unwrap()) as f64)
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
    device: wgpu::Device,
    queue: wgpu::Queue,
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::ComputePipeline,
    buffers: std::cell::RefCell<Option<DistancePairSumBuffers>>,
}

struct DistancePairSumBuffers {
    pair_capacity: usize,
    partial_capacity: usize,
    params: wgpu::Buffer,
    a: wgpu::Buffer,
    b: wgpu::Buffer,
    out: wgpu::Buffer,
    read: wgpu::Buffer,
    bind: wgpu::BindGroup,
}

impl GpuDistancePairSum {
    fn new(context: &GpuContext) -> Self {
        let device = &context.device;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("distance_pair_sum"),
            source: wgpu::ShaderSource::Wgsl(crate::DISTANCE_PAIR_SUM_WGSL.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("distance_pair_sum"),
            entries: &[
                uniform_entry(0),
                storage_entry(1, true),
                storage_entry(2, true),
                storage_entry(3, false),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("distance_pair_sum"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("distance_pair_sum"),
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

    fn ensure_buffers(&self, pair_count: usize, partial_count: usize) {
        let stale = match &*self.buffers.borrow() {
            Some(b) => b.pair_capacity < pair_count || b.partial_capacity < partial_count,
            None => true,
        };
        if !stale {
            return;
        }
        let pair_capacity = pair_count.max(1);
        let partial_capacity = partial_count.max(1);
        let device = &self.device;
        let mk = |label: &str, size: u64, usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage,
                mapped_at_creation: false,
            })
        };
        let params = mk(
            "distance_pair_sum_params",
            16,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );
        let a = mk(
            "distance_pair_sum_a",
            (pair_capacity * 12) as u64,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let b = mk(
            "distance_pair_sum_b",
            (pair_capacity * 12) as u64,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let out = mk(
            "distance_pair_sum_out",
            (partial_capacity * 4) as u64,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        );
        let read = mk(
            "distance_pair_sum_read",
            (partial_capacity * 4) as u64,
            wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        );
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("distance_pair_sum"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: a.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: b.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: out.as_entire_binding(),
                },
            ],
        });
        *self.buffers.borrow_mut() = Some(DistancePairSumBuffers {
            pair_capacity,
            partial_capacity,
            params,
            a,
            b,
            out,
            read,
            bind,
        });
    }

    fn run(&self, a: &[V3], b: &[V3]) -> f64 {
        let pair_count = a.len();
        let partial_count = pair_count.div_ceil(256).max(1);
        self.ensure_buffers(pair_count, partial_count);
        let flat_a: Vec<f32> = a.iter().flatten().map(|&v| v as f32).collect();
        let flat_b: Vec<f32> = b.iter().flatten().map(|&v| v as f32).collect();
        let mut params = Vec::with_capacity(16);
        params.extend_from_slice(&(pair_count as u32).to_le_bytes());
        params.extend_from_slice(&0u32.to_le_bytes());
        params.extend_from_slice(&0u32.to_le_bytes());
        params.extend_from_slice(&0u32.to_le_bytes());
        let device = &self.device;
        let buffers = self.buffers.borrow();
        let buffers = buffers.as_ref().expect("ensure_buffers was just called");
        self.queue.write_buffer(&buffers.params, 0, &params);
        self.queue
            .write_buffer(&buffers.a, 0, &gpu_compute::pack_f32(&flat_a));
        self.queue
            .write_buffer(&buffers.b, 0, &gpu_compute::pack_f32(&flat_b));
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("distance_pair_sum"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("distance_pair_sum"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &buffers.bind, &[]);
            pass.dispatch_workgroups(partial_count as u32, 1, 1);
        }
        encoder.copy_buffer_to_buffer(
            &buffers.out,
            0,
            &buffers.read,
            0,
            (partial_count * 4) as u64,
        );
        self.queue.submit([encoder.finish()]);
        let raw = read_buffer(device, &buffers.read, partial_count * 4);
        buffers.read.unmap();
        raw.chunks_exact(4)
            .take(partial_count)
            .map(|d| f32::from_ne_bytes(d.try_into().unwrap()) as f64)
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

struct GpuPointBounds {
    device: wgpu::Device,
    queue: wgpu::Queue,
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::ComputePipeline,
    workgroup_size: u32,
    buffers: std::cell::RefCell<Option<PointBoundsBuffers>>,
}

struct PointBoundsBuffers {
    point_capacity: usize,
    partial_capacity: usize,
    params: wgpu::Buffer,
    points: wgpu::Buffer,
    out_min: wgpu::Buffer,
    out_max: wgpu::Buffer,
    read_min: wgpu::Buffer,
    read_max: wgpu::Buffer,
    bind: wgpu::BindGroup,
}

impl GpuPointBounds {
    fn new(context: &GpuContext) -> Self {
        let workgroup_size =
            gpu_compute::tuned_workgroup_size(context.backend, WG_METAL, WG_DEFAULT);
        let device = &context.device;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("point_bounds"),
            source: wgpu::ShaderSource::Wgsl(
                crate::POINT_BOUNDS_WGSL_TEMPLATE
                    .replace("__WG__", &workgroup_size.to_string())
                    .into(),
            ),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("point_bounds"),
            entries: &[
                uniform_entry(0),
                storage_entry(1, true),
                storage_entry(2, false),
                storage_entry(3, false),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("point_bounds"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("point_bounds"),
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
            workgroup_size,
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
        let point_capacity = point_count.max(1);
        let partial_capacity = partial_count.max(1);
        let device = &self.device;
        let mk = |label: &str, size: u64, usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage,
                mapped_at_creation: false,
            })
        };
        let params = mk(
            "point_bounds_params",
            16,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );
        let points = mk(
            "point_bounds_points",
            (point_capacity * 12) as u64,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let partial_bytes = (partial_capacity * 12) as u64;
        let out_min = mk(
            "point_bounds_out_min",
            partial_bytes,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        );
        let out_max = mk(
            "point_bounds_out_max",
            partial_bytes,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        );
        let read_min = mk(
            "point_bounds_read_min",
            partial_bytes,
            wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        );
        let read_max = mk(
            "point_bounds_read_max",
            partial_bytes,
            wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        );
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("point_bounds"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: points.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: out_min.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: out_max.as_entire_binding(),
                },
            ],
        });
        *self.buffers.borrow_mut() = Some(PointBoundsBuffers {
            point_capacity,
            partial_capacity,
            params,
            points,
            out_min,
            out_max,
            read_min,
            read_max,
            bind,
        });
    }

    fn run(&self, points: &[V3]) -> crate::PointBounds {
        let point_count = points.len();
        let partial_count = point_count.div_ceil(self.workgroup_size as usize).max(1);
        self.ensure_buffers(point_count, partial_count);
        let flat: Vec<f32> = points.iter().flatten().map(|&v| v as f32).collect();
        let mut params = Vec::with_capacity(16);
        params.extend_from_slice(&(point_count as u32).to_le_bytes());
        params.extend_from_slice(&0u32.to_le_bytes());
        params.extend_from_slice(&0u32.to_le_bytes());
        params.extend_from_slice(&0u32.to_le_bytes());
        let device = &self.device;
        let buffers = self.buffers.borrow();
        let buffers = buffers.as_ref().expect("ensure_buffers was just called");
        self.queue.write_buffer(&buffers.params, 0, &params);
        self.queue
            .write_buffer(&buffers.points, 0, &gpu_compute::pack_f32(&flat));
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("point_bounds"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("point_bounds"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &buffers.bind, &[]);
            pass.dispatch_workgroups(partial_count as u32, 1, 1);
        }
        let partial_bytes = (partial_count * 12) as u64;
        encoder.copy_buffer_to_buffer(&buffers.out_min, 0, &buffers.read_min, 0, partial_bytes);
        encoder.copy_buffer_to_buffer(&buffers.out_max, 0, &buffers.read_max, 0, partial_bytes);
        self.queue.submit([encoder.finish()]);
        let raw_min = read_buffer(device, &buffers.read_min, partial_count * 12);
        buffers.read_min.unmap();
        let raw_max = read_buffer(device, &buffers.read_max, partial_count * 12);
        buffers.read_max.unmap();
        let mut min = [f64::INFINITY; 3];
        let mut max = [f64::NEG_INFINITY; 3];
        for chunk in raw_min.chunks_exact(12).take(partial_count) {
            for axis in 0..3 {
                let start = axis * 4;
                min[axis] = min[axis]
                    .min(f32::from_ne_bytes(chunk[start..start + 4].try_into().unwrap()) as f64);
            }
        }
        for chunk in raw_max.chunks_exact(12).take(partial_count) {
            for axis in 0..3 {
                let start = axis * 4;
                max[axis] = max[axis]
                    .max(f32::from_ne_bytes(chunk[start..start + 4].try_into().unwrap()) as f64);
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

struct GpuPointMoments {
    device: wgpu::Device,
    queue: wgpu::Queue,
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::ComputePipeline,
    workgroup_size: u32,
    buffers: std::cell::RefCell<Option<PointMomentsBuffers>>,
}

struct PointMomentsBuffers {
    point_capacity: usize,
    partial_capacity: usize,
    params: wgpu::Buffer,
    points: wgpu::Buffer,
    out: wgpu::Buffer,
    read: wgpu::Buffer,
    bind: wgpu::BindGroup,
}

impl GpuPointMoments {
    fn new(context: &GpuContext) -> Self {
        let workgroup_size =
            gpu_compute::tuned_workgroup_size(context.backend, WG_METAL, WG_DEFAULT);
        let device = &context.device;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("point_moments"),
            source: wgpu::ShaderSource::Wgsl(
                crate::POINT_MOMENTS_WGSL_TEMPLATE
                    .replace("__WG__", &workgroup_size.to_string())
                    .into(),
            ),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("point_moments"),
            entries: &[
                uniform_entry(0),
                storage_entry(1, true),
                storage_entry(2, false),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("point_moments"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("point_moments"),
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
            workgroup_size,
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
        let point_capacity = point_count.max(1);
        let partial_capacity = partial_count.max(1);
        let device = &self.device;
        let mk = |label: &str, size: u64, usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage,
                mapped_at_creation: false,
            })
        };
        let params = mk(
            "point_moments_params",
            16,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );
        let points = mk(
            "point_moments_points",
            (point_capacity * 12) as u64,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let partial_bytes = (partial_capacity * 9 * 4) as u64;
        let out = mk(
            "point_moments_out",
            partial_bytes,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        );
        let read = mk(
            "point_moments_read",
            partial_bytes,
            wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        );
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("point_moments"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: points.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: out.as_entire_binding(),
                },
            ],
        });
        *self.buffers.borrow_mut() = Some(PointMomentsBuffers {
            point_capacity,
            partial_capacity,
            params,
            points,
            out,
            read,
            bind,
        });
    }

    fn run(&self, points: &[V3]) -> crate::PointMoments {
        let point_count = points.len();
        let partial_count = point_count.div_ceil(self.workgroup_size as usize).max(1);
        self.ensure_buffers(point_count, partial_count);
        let flat: Vec<f32> = points.iter().flatten().map(|&v| v as f32).collect();
        let mut params = Vec::with_capacity(16);
        params.extend_from_slice(&(point_count as u32).to_le_bytes());
        params.extend_from_slice(&0u32.to_le_bytes());
        params.extend_from_slice(&0u32.to_le_bytes());
        params.extend_from_slice(&0u32.to_le_bytes());
        let device = &self.device;
        let buffers = self.buffers.borrow();
        let buffers = buffers.as_ref().expect("ensure_buffers was just called");
        self.queue.write_buffer(&buffers.params, 0, &params);
        self.queue
            .write_buffer(&buffers.points, 0, &gpu_compute::pack_f32(&flat));
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("point_moments"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("point_moments"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &buffers.bind, &[]);
            pass.dispatch_workgroups(partial_count as u32, 1, 1);
        }
        let partial_bytes = partial_count * 9 * 4;
        encoder.copy_buffer_to_buffer(&buffers.out, 0, &buffers.read, 0, partial_bytes as u64);
        self.queue.submit([encoder.finish()]);
        let raw = read_buffer(device, &buffers.read, partial_bytes);
        buffers.read.unmap();
        let mut accum = [0.; 9];
        for chunk in raw.chunks_exact(36).take(partial_count) {
            for item in 0..9 {
                let start = item * 4;
                accum[item] +=
                    f32::from_ne_bytes(chunk[start..start + 4].try_into().unwrap()) as f64;
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

struct GpuChamfer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::ComputePipeline,
    workgroup_size: u32,
    buffers: std::cell::RefCell<Option<ChamferBuffers>>,
}

struct ChamferBuffers {
    query_capacity: usize,
    target_capacity: usize,
    partial_capacity: usize,
    params: wgpu::Buffer,
    queries: wgpu::Buffer,
    targets: wgpu::Buffer,
    out_sum: wgpu::Buffer,
    out_max: wgpu::Buffer,
    read_sum: wgpu::Buffer,
    read_max: wgpu::Buffer,
    bind: wgpu::BindGroup,
}

impl GpuChamfer {
    fn new(context: &GpuContext) -> Self {
        let workgroup_size =
            gpu_compute::tuned_workgroup_size(context.backend, WG_METAL, WG_DEFAULT);
        let device = &context.device;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("chamfer"),
            source: wgpu::ShaderSource::Wgsl(
                crate::CHAMFER_WGSL_TEMPLATE
                    .replace("__WG__", &workgroup_size.to_string())
                    .into(),
            ),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("chamfer"),
            entries: &[
                uniform_entry(0),
                storage_entry(1, true),
                storage_entry(2, true),
                storage_entry(3, false),
                storage_entry(4, false),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("chamfer"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("chamfer"),
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
            workgroup_size,
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
        let query_capacity = query_count.max(1);
        let target_capacity = target_count.max(1);
        let partial_capacity = partial_count.max(1);
        let device = &self.device;
        let mk = |label: &str, size: u64, usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage,
                mapped_at_creation: false,
            })
        };
        let params = mk(
            "chamfer_params",
            16,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );
        let queries = mk(
            "chamfer_queries",
            (query_capacity * 12) as u64,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let targets = mk(
            "chamfer_targets",
            (target_capacity * 12) as u64,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let out_sum = mk(
            "chamfer_out_sum",
            (partial_capacity * 4) as u64,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        );
        let out_max = mk(
            "chamfer_out_max",
            (partial_capacity * 4) as u64,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        );
        let read_sum = mk(
            "chamfer_read_sum",
            (partial_capacity * 4) as u64,
            wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        );
        let read_max = mk(
            "chamfer_read_max",
            (partial_capacity * 4) as u64,
            wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        );
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("chamfer"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: queries.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: targets.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: out_sum.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: out_max.as_entire_binding(),
                },
            ],
        });
        *self.buffers.borrow_mut() = Some(ChamferBuffers {
            query_capacity,
            target_capacity,
            partial_capacity,
            params,
            queries,
            targets,
            out_sum,
            out_max,
            read_sum,
            read_max,
            bind,
        });
    }

    fn run(&self, queries: &[V3], targets: &[V3]) -> crate::DirectedChamfer {
        let query_count = queries.len();
        let target_count = targets.len();
        let partial_count = query_count.div_ceil(self.workgroup_size as usize).max(1);
        self.ensure_buffers(query_count, target_count, partial_count);
        let flat_q: Vec<f32> = queries.iter().flatten().map(|&v| v as f32).collect();
        let flat_t: Vec<f32> = targets.iter().flatten().map(|&v| v as f32).collect();
        let mut params = Vec::with_capacity(16);
        params.extend_from_slice(&(query_count as u32).to_le_bytes());
        params.extend_from_slice(&(target_count as u32).to_le_bytes());
        params.extend_from_slice(&0u32.to_le_bytes());
        params.extend_from_slice(&0u32.to_le_bytes());
        let device = &self.device;
        let buffers = self.buffers.borrow();
        let buffers = buffers.as_ref().expect("ensure_buffers was just called");
        self.queue.write_buffer(&buffers.params, 0, &params);
        self.queue
            .write_buffer(&buffers.queries, 0, &gpu_compute::pack_f32(&flat_q));
        self.queue
            .write_buffer(&buffers.targets, 0, &gpu_compute::pack_f32(&flat_t));
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("chamfer"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("chamfer"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &buffers.bind, &[]);
            pass.dispatch_workgroups(partial_count as u32, 1, 1);
        }
        let partial_bytes = (partial_count * 4) as u64;
        encoder.copy_buffer_to_buffer(&buffers.out_sum, 0, &buffers.read_sum, 0, partial_bytes);
        encoder.copy_buffer_to_buffer(&buffers.out_max, 0, &buffers.read_max, 0, partial_bytes);
        self.queue.submit([encoder.finish()]);
        let raw_sum = read_buffer(device, &buffers.read_sum, partial_count * 4);
        buffers.read_sum.unmap();
        let raw_max = read_buffer(device, &buffers.read_max, partial_count * 4);
        buffers.read_max.unmap();
        let sum: f64 = raw_sum
            .chunks_exact(4)
            .take(partial_count)
            .map(|value| f32::from_ne_bytes(value.try_into().unwrap()) as f64)
            .sum();
        let max_squared_distance = raw_max
            .chunks_exact(4)
            .take(partial_count)
            .map(|value| f32::from_ne_bytes(value.try_into().unwrap()) as f64)
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
    device: wgpu::Device,
    queue: wgpu::Queue,
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::ComputePipeline,
    buffers: std::cell::RefCell<Option<NearestTwoBuffers>>,
}

struct NearestTwoBuffers {
    query_capacity: usize,
    target_capacity: usize,
    params: wgpu::Buffer,
    queries: wgpu::Buffer,
    targets: wgpu::Buffer,
    out_i0: wgpu::Buffer,
    out_d0: wgpu::Buffer,
    out_i1: wgpu::Buffer,
    out_d1: wgpu::Buffer,
    read_i0: wgpu::Buffer,
    read_d0: wgpu::Buffer,
    read_i1: wgpu::Buffer,
    read_d1: wgpu::Buffer,
    bind: wgpu::BindGroup,
}

impl GpuNearestTwo {
    fn new(context: &GpuContext) -> Self {
        let device = &context.device;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("nearest_two"),
            source: wgpu::ShaderSource::Wgsl(crate::NEAREST_TWO_WGSL.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("nearest_two"),
            entries: &[
                uniform_entry(0),
                storage_entry(1, true),
                storage_entry(2, true),
                storage_entry(3, false),
                storage_entry(4, false),
                storage_entry(5, false),
                storage_entry(6, false),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("nearest_two"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("nearest_two"),
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

    fn ensure_buffers(&self, query_count: usize, target_count: usize) {
        let stale = match &*self.buffers.borrow() {
            Some(b) => b.query_capacity < query_count || b.target_capacity < target_count,
            None => true,
        };
        if !stale {
            return;
        }
        let query_capacity = query_count.max(1);
        let target_capacity = target_count.max(1);
        let device = &self.device;
        let mk = |label: &str, size: u64, usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage,
                mapped_at_creation: false,
            })
        };
        let params = mk(
            "nearest_two_params",
            16,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );
        let queries = mk(
            "nearest_two_queries",
            (query_capacity * 12) as u64,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let targets = mk(
            "nearest_two_targets",
            (target_capacity * 12) as u64,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let out_bytes = (query_capacity * 4) as u64;
        let out_i0 = mk(
            "nearest_two_i0",
            out_bytes,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        );
        let out_d0 = mk(
            "nearest_two_d0",
            out_bytes,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        );
        let out_i1 = mk(
            "nearest_two_i1",
            out_bytes,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        );
        let out_d1 = mk(
            "nearest_two_d1",
            out_bytes,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        );
        let read_i0 = mk(
            "nearest_two_read_i0",
            out_bytes,
            wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        );
        let read_d0 = mk(
            "nearest_two_read_d0",
            out_bytes,
            wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        );
        let read_i1 = mk(
            "nearest_two_read_i1",
            out_bytes,
            wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        );
        let read_d1 = mk(
            "nearest_two_read_d1",
            out_bytes,
            wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        );
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nearest_two"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: queries.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: targets.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: out_i0.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: out_d0.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: out_i1.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: out_d1.as_entire_binding(),
                },
            ],
        });
        *self.buffers.borrow_mut() = Some(NearestTwoBuffers {
            query_capacity,
            target_capacity,
            params,
            queries,
            targets,
            out_i0,
            out_d0,
            out_i1,
            out_d1,
            read_i0,
            read_d0,
            read_i1,
            read_d1,
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
        let mut params = Vec::with_capacity(16);
        params.extend_from_slice(&(query_count as u32).to_le_bytes());
        params.extend_from_slice(&(target_count as u32).to_le_bytes());
        params.extend_from_slice(&0u32.to_le_bytes());
        params.extend_from_slice(&0u32.to_le_bytes());
        self.queue.write_buffer(&b.params, 0, &params);
        self.queue
            .write_buffer(&b.queries, 0, &gpu_compute::pack_f32(&flat_q));
        self.queue
            .write_buffer(&b.targets, 0, &gpu_compute::pack_f32(&flat_t));
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("nearest_two"),
            });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("nearest_two"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &b.bind, &[]);
            pass.dispatch_workgroups((query_count.max(1) as u32).div_ceil(256), 1, 1);
        }
        let copy_bytes = (query_count * 4) as u64;
        encoder.copy_buffer_to_buffer(&b.out_i0, 0, &b.read_i0, 0, copy_bytes);
        encoder.copy_buffer_to_buffer(&b.out_d0, 0, &b.read_d0, 0, copy_bytes);
        encoder.copy_buffer_to_buffer(&b.out_i1, 0, &b.read_i1, 0, copy_bytes);
        encoder.copy_buffer_to_buffer(&b.out_d1, 0, &b.read_d1, 0, copy_bytes);
        self.queue.submit([encoder.finish()]);
        let i0 = read_buffer(&self.device, &b.read_i0, query_count * 4);
        b.read_i0.unmap();
        let d0 = read_buffer(&self.device, &b.read_d0, query_count * 4);
        b.read_d0.unmap();
        let i1 = read_buffer(&self.device, &b.read_i1, query_count * 4);
        b.read_i1.unmap();
        let d1 = read_buffer(&self.device, &b.read_d1, query_count * 4);
        b.read_d1.unmap();
        (0..query_count)
            .map(|idx| {
                let off = idx * 4;
                [
                    (
                        u32::from_ne_bytes(i0[off..off + 4].try_into().unwrap()),
                        f32::from_ne_bytes(d0[off..off + 4].try_into().unwrap()) as f64,
                    ),
                    (
                        u32::from_ne_bytes(i1[off..off + 4].try_into().unwrap()),
                        f32::from_ne_bytes(d1[off..off + 4].try_into().unwrap()) as f64,
                    ),
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
