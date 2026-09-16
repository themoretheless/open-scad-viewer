//! GPU batch transform (feature `gpu`): runs the shared `TRANSFORM_WGSL`
//! shader over a point set. f32 arithmetic — see [`crate::Acceleration`].
use crate::{M3, V3};
use gpu_compute::{GpuContext, read_buffer, storage_entry, uniform_entry, wgpu};
use wgpu::util::DeviceExt;

struct GpuTransform {
    device: wgpu::Device,
    queue: wgpu::Queue,
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::ComputePipeline,
}

impl GpuTransform {
    fn new(context: &GpuContext) -> Self {
        let device = &context.device;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("transform_points"),
            source: wgpu::ShaderSource::Wgsl(crate::TRANSFORM_WGSL.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("transform_points"),
            entries: &[
                uniform_entry(0),
                storage_entry(1, true),
                storage_entry(2, false),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("transform_points"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("transform_points"),
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

    fn run(&self, points: &[V3], m: M3, t: V3) -> Vec<V3> {
        let device = &self.device;
        let count = points.len();
        // Uniform layout matches transform.wgsl's `Params`: three padded
        // vec3<f32> matrix rows, then a padded vec3<f32> translation + u32 count.
        let mut params = Vec::with_capacity(64);
        for row in m {
            for v in row {
                params.extend_from_slice(&(v as f32).to_le_bytes());
            }
            params.extend_from_slice(&0f32.to_le_bytes());
        }
        for v in t {
            params.extend_from_slice(&(v as f32).to_le_bytes());
        }
        params.extend_from_slice(&(count as u32).to_le_bytes());
        let flat: Vec<f32> = points.iter().flatten().map(|&v| v as f32).collect();
        let input = gpu_compute::pack_f32(&flat);
        let mk = |label: &str, bytes: &[u8], usage| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytes,
                usage,
            })
        };
        let params_buf = mk("transform_params", &params, wgpu::BufferUsages::UNIFORM);
        let in_buf = mk(
            "transform_in",
            if input.is_empty() { &[0u8; 12] } else { &input },
            wgpu::BufferUsages::STORAGE,
        );
        let value_bytes = (count.max(1) * 12) as u64;
        let out_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("transform_out"),
            size: value_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let read_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("transform_read"),
            size: value_bytes,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("transform_points"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: in_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: out_buf.as_entire_binding(),
                },
            ],
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("transform_points"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("transform_points"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.dispatch_workgroups((count.max(1) as u32).div_ceil(256), 1, 1);
        }
        encoder.copy_buffer_to_buffer(&out_buf, 0, &read_buf, 0, value_bytes);
        self.queue.submit([encoder.finish()]);
        let raw = read_buffer(device, &read_buf, count * 12);
        raw.chunks_exact(4)
            .take(count * 3)
            .map(|c| f32::from_ne_bytes(c.try_into().unwrap()) as f64)
            .collect::<Vec<f64>>()
            .chunks_exact(3)
            .map(|c| [c[0], c[1], c[2]])
            .collect()
    }
}

thread_local! {
    // Process-lifetime device and pipeline; see `sdf-core`'s `gpu.rs`.
    static SHARED: std::cell::LazyCell<Option<&'static (GpuContext, GpuTransform)>> =
        std::cell::LazyCell::new(|| GpuContext::new().map(|c| {
            let transform = GpuTransform::new(&c);
            Box::leak(Box::new((c, transform))) as &'static (GpuContext, GpuTransform)
        }));
}

/// Transforms every point on the GPU; `None` without an adapter (CPU fallback).
pub fn transform_points_gpu(points: &[V3], m: M3, t: V3) -> Option<Vec<V3>> {
    SHARED.with(|cell| {
        let shared: &Option<&(GpuContext, GpuTransform)> = cell;
        shared.map(|(_, transform)| transform.run(points, m, t))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ID, add, mv};

    #[test]
    fn gpu_transform_matches_cpu_reference_when_available() {
        let m = crate::rotation([0.1, 0.4, -0.2]);
        let t = [1.0, -2.0, 3.5];
        let points: Vec<V3> = (0..200)
            .map(|i| {
                let f = i as f64;
                [f * 0.13 - 10., f * -0.07 + 4., (f * 0.031).sin() * 5.]
            })
            .collect();
        let Some(got) = transform_points_gpu(&points, m, t) else {
            eprintln!("no wgpu adapter available; skipping");
            return;
        };
        assert_eq!(got.len(), points.len());
        for (p, q) in points.iter().zip(&got) {
            let expected = add(mv(m, *p), t);
            for k in 0..3 {
                assert!((expected[k] - q[k]).abs() < 5e-4, "{expected:?} vs {q:?}");
            }
        }
    }

    #[test]
    fn gpu_transform_empty_input() {
        if let Some(got) = transform_points_gpu(&[], ID, [0., 0., 0.]) {
            assert!(got.is_empty());
        }
    }
}
