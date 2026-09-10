//! GPU frontoparallel NCC depth sweep (feature `gpu`). One thread per depth-map
//! pixel evaluates all inverse-depth hypotheses across all source views. The
//! arithmetic is f32 (WGSL has no f64 and float contraction applies), so scores
//! are not bit-identical to the CPU reference; hypothesis selection, per-pixel
//! ranges and all downstream geometry stay on the CPU. Opt-in mode, qualified
//! separately; callers fall back to the CPU sweep when no adapter exists.

use super::wgpu;
use super::wgpu::util::DeviceExt;

use super::GpuContext;

const SHADER: &str = crate::dense::SWEEP_WGSL;

/// Per-source camera/geometry payload for the sweep shader.
pub struct SourcePayload {
    /// Row-major R_source * R_reference^T.
    pub rotation: [[f32; 3]; 3],
    /// t_source - R * t_reference.
    pub translation: [f32; 3],
    pub focal: f32,
    pub cx: f32,
    pub cy: f32,
    pub gray_offset: u32,
    pub gray_width: u32,
    pub gray_height: u32,
}

pub struct GpuSweep {
    device: wgpu::Device,
    queue: wgpu::Queue,
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::ComputePipeline,
}

impl GpuSweep {
    pub fn new(context: &GpuContext) -> Self {
        let device = &context.device;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ncc_sweep"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let entries = [
            super::uniform_entry(0),
            super::storage_entry(1, true),
            super::storage_entry(2, true),
            super::storage_entry(3, true),
            super::storage_entry(4, true),
            super::storage_entry(5, false),
        ];
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ncc_sweep"),
            entries: &entries,
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ncc_sweep"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("ncc_sweep"),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("sweep"),
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

    /// Scores per pixel per hypothesis (row-major pixels, hypothesis-fastest).
    #[allow(clippy::too_many_arguments)]
    pub fn run(
        &self,
        ref_gray: &[f32],
        ref_width: usize,
        ref_height: usize,
        ref_focal: f64,
        ref_cx: f64,
        ref_cy: f64,
        sources: &[SourcePayload],
        grays: &[f32],
        hypotheses: &[f32],
        width: usize,
        height: usize,
        patch_radius: usize,
        step: f64,
        needed: usize,
    ) -> Vec<f32> {
        let device = &self.device;
        let n_hyp = hypotheses.len() as u32;
        let params: Vec<u32> = vec![
            width as u32,
            height as u32,
            n_hyp,
            sources.len() as u32,
            (patch_radius * 2 + 1).pow(2) as u32,
            patch_radius as u32,
            needed as u32,
            ref_width as u32,
            ref_height as u32,
            0,
            0,
            0,
        ];
        let params_f: Vec<f32> = vec![step as f32, ref_focal as f32, ref_cx as f32, ref_cy as f32];
        let mut params_bytes = gpu_compute::pack_u32(&params);
        params_bytes.extend_from_slice(&gpu_compute::pack_f32(&params_f));
        let mut srcf = Vec::with_capacity(sources.len() * 16 * 4);
        let mut srcm = Vec::with_capacity(sources.len() * 4 * 4);
        for source in sources {
            for row in source.rotation {
                for v in row {
                    srcf.extend_from_slice(&v.to_ne_bytes());
                }
            }
            for v in source.translation {
                srcf.extend_from_slice(&v.to_ne_bytes());
            }
            for v in [source.focal, source.cx, source.cy, 0.] {
                srcf.extend_from_slice(&v.to_ne_bytes());
            }
            for v in [source.gray_offset, source.gray_width, source.gray_height, 0] {
                srcm.extend_from_slice(&v.to_ne_bytes());
            }
        }
        let hyp_bytes = gpu_compute::pack_f32(hypotheses);
        // The reference gray image leads the shared gray buffer at offset 0.
        let mut gray_bytes = gpu_compute::pack_f32(ref_gray);
        gray_bytes.extend_from_slice(&gpu_compute::pack_f32(grays));

        let score_count = width * height * hypotheses.len();
        let score_bytes = (score_count * 4) as u64;
        let buf = |label: &str, bytes: &[u8], usage| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytes,
                usage,
            })
        };
        let params_buf = buf("params", &params_bytes, wgpu::BufferUsages::UNIFORM);
        let hyp_buf = buf("hyps", &hyp_bytes, wgpu::BufferUsages::STORAGE);
        let srcf_buf = buf("srcf", &srcf, wgpu::BufferUsages::STORAGE);
        let srcm_buf = buf("srcm", &srcm, wgpu::BufferUsages::STORAGE);
        let gray_buf = buf("grays", &gray_bytes, wgpu::BufferUsages::STORAGE);
        let out_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scores"),
            size: score_bytes.max(16),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let read_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scores_read"),
            size: score_bytes.max(16),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ncc_sweep"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: params_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: hyp_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: srcf_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: srcm_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 4, resource: gray_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 5, resource: out_buf.as_entire_binding() },
            ],
        });
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("sweep") });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("sweep"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.dispatch_workgroups(width.div_ceil(16) as u32, height.div_ceil(16) as u32, 1);
        }
        encoder.copy_buffer_to_buffer(&out_buf, 0, &read_buf, 0, score_bytes.max(16));
        self.queue.submit([encoder.finish()]);

        let raw = {
            let bytes = super::read_buffer(device, &read_buf, score_bytes as usize);
            bytes
        };
        raw.chunks_exact(4)
            .take(score_count)
            .map(|c| f32::from_ne_bytes(c.try_into().unwrap()))
            .collect()
    }
}

thread_local! {
    // Process-lifetime device and pipeline; see gpu::matching for the leak note.
    static SHARED: std::cell::LazyCell<Option<&'static (GpuContext, GpuSweep)>> =
        std::cell::LazyCell::new(|| GpuContext::new().map(|c| {
            let sweep = GpuSweep::new(&c);
            Box::leak(Box::new((c, sweep))) as &'static (GpuContext, GpuSweep)
        }));
}

/// Runs one view's sweep through a thread-shared GPU context; `None` without an
/// adapter, letting the caller fall back to the CPU sweep.
#[allow(clippy::too_many_arguments)]
pub fn sweep_view(
    ref_gray: &[f32],
    ref_width: usize,
    ref_height: usize,
    ref_focal: f64,
    ref_cx: f64,
    ref_cy: f64,
    sources: &[SourcePayload],
    grays: &[f32],
    hypotheses: &[f32],
    width: usize,
    height: usize,
    patch_radius: usize,
    step: f64,
    needed: usize,
) -> Option<Vec<f32>> {
    SHARED.with(|cell| {
        let shared: &Option<&(GpuContext, GpuSweep)> = cell;
        shared.map(|(_, sweep)| {
            sweep.run(
                ref_gray, ref_width, ref_height, ref_focal, ref_cx, ref_cy, sources, grays,
                hypotheses, width, height, patch_radius, step, needed,
            )
        })
    })
}
