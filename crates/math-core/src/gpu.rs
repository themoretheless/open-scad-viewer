//! GPU nearest-neighbor search (feature `gpu`): for every query point, finds
//! the index and squared distance of the closest target point by brute
//! force. f32 arithmetic — see [`crate::Acceleration`].
//!
//! Device buffers are cached per query/target capacity (grow-only) so
//! repeated calls at a stable size amortize allocation; only the bytes
//! actually written/read for the current call cross the wire.
use crate::V3;
use gpu_compute::{GpuContext, read_buffer, storage_entry, uniform_entry, wgpu};

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
}
