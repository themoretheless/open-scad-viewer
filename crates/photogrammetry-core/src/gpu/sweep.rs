//! GPU frontoparallel NCC depth sweep (feature `gpu`). One thread per depth-map
//! pixel evaluates all inverse-depth hypotheses across all source views and
//! selects the winning bin in registers (`sweep_select`), so only 16 bytes per
//! pixel return to the host. The arithmetic is f32 (WGSL has no f64 and float
//! contraction applies), so results are not bit-identical to the CPU reference;
//! per-pixel ranges, the f64 depth reconstruction and all downstream geometry
//! stay on the CPU. Opt-in mode, qualified separately; callers fall back to the
//! CPU sweep when no adapter exists.
//!
//! Grayscale rasters upload once per estimation pass (`GrayAtlas`) and every
//! view of the pass is encoded into one command buffer (`SweepBatch`): one
//! submit and one GPU wait per pass instead of one per view.

use super::wgpu;
use super::wgpu::util::DeviceExt;

use super::GpuContext;

const SHADER: &str = crate::dense::SWEEP_WGSL;

/// Per-source camera/geometry payload for the sweep shader.
pub struct SourcePayload {
    /// Index into the `GrayAtlas` images.
    pub image: usize,
    /// Row-major R_source * R_reference^T.
    pub rotation: [[f32; 3]; 3],
    /// t_source - R * t_reference.
    pub translation: [f32; 3],
    pub focal: f32,
    pub cx: f32,
    pub cy: f32,
}

/// One view's sweep-and-select request.
pub struct SweepJob {
    pub ref_image: usize,
    pub ref_focal: f32,
    pub ref_cx: f32,
    pub ref_cy: f32,
    pub sources: Vec<SourcePayload>,
    pub hypotheses: Vec<f32>,
    pub width: usize,
    pub height: usize,
    pub patch_radius: usize,
    pub step: f32,
    pub needed: usize,
    /// Per-pixel inclusive hypothesis bin range `(lo, hi)`, `2 * width * height`.
    pub bins: Vec<u32>,
    /// Smallest f32 not below the f64 correlation threshold (exact comparison).
    pub min_correlation: f32,
    pub uniqueness_margin: f32,
}

/// Per-pixel selection: the winning bin, its parabolic sub-bin offset and score.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Selection {
    pub bin: u32,
    pub offset: f32,
    pub score: f32,
}

/// All grayscale rasters of a pass in one storage buffer.
pub struct GrayAtlas {
    buffer: wgpu::Buffer,
    /// (offset in floats, width, height) per image index; None for absent images.
    meta: Vec<Option<(u32, u32, u32)>>,
}

pub struct GpuSweep {
    device: wgpu::Device,
    queue: wgpu::Queue,
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::ComputePipeline,
    /// The browser-shared `sweep` entry (raw scores), kept for qualification.
    scores_pipeline: wgpu::ComputePipeline,
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
            super::storage_entry(6, true),
            super::storage_entry(7, false),
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
        let compute = |entry: &str| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: Some(&pipeline_layout),
                module: &module,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        Self {
            device: device.clone(),
            queue: context.queue.clone(),
            layout,
            pipeline: compute("sweep_select"),
            scores_pipeline: compute("sweep"),
        }
    }

    /// Uploads every present raster once; `images[i] = Some((values, width, height))`.
    pub fn upload_grays(&self, images: &[Option<(&[f32], usize, usize)>]) -> GrayAtlas {
        let total: usize = images
            .iter()
            .flatten()
            .map(|(values, _, _)| values.len())
            .sum();
        let mut bytes = Vec::with_capacity(total.max(4) * 4);
        let mut meta = Vec::with_capacity(images.len());
        for image in images {
            meta.push(image.map(|(values, width, height)| {
                let offset = (bytes.len() / 4) as u32;
                bytes.extend_from_slice(&gpu_compute::pack_f32(values));
                (offset, width as u32, height as u32)
            }));
        }
        bytes.resize(bytes.len().max(16), 0);
        let buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("grays"),
                contents: &bytes,
                usage: wgpu::BufferUsages::STORAGE,
            });
        GrayAtlas { buffer, meta }
    }

    pub fn batch<'a>(&'a self, atlas: &'a GrayAtlas) -> SweepBatch<'a> {
        SweepBatch {
            sweep: self,
            atlas,
            encoder: self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("sweep"),
                }),
            reads: Vec::new(),
            keep: Vec::new(),
        }
    }
}

/// Views encoded into one command buffer; `finish` submits once and reads all
/// results back in job order.
pub struct SweepBatch<'a> {
    sweep: &'a GpuSweep,
    atlas: &'a GrayAtlas,
    encoder: wgpu::CommandEncoder,
    /// Readback buffer and byte length per job.
    reads: Vec<(wgpu::Buffer, usize)>,
    keep: Vec<wgpu::Buffer>,
}

impl SweepBatch<'_> {
    pub fn push(&mut self, job: &SweepJob) {
        self.encode(job, false);
    }

    /// Encodes the browser-shared `sweep` entry: raw per-hypothesis scores
    /// (`width * height * n_hyp` floats) instead of selections. Qualification only.
    pub fn push_scores(&mut self, job: &SweepJob) {
        self.encode(job, true);
    }

    fn encode(&mut self, job: &SweepJob, raw_scores: bool) {
        let device = &self.sweep.device;
        let (ref_offset, ref_width, ref_height) = self.atlas.meta[job.ref_image]
            .expect("reference image must be present in the gray atlas");
        let params: Vec<u32> = vec![
            job.width as u32,
            job.height as u32,
            job.hypotheses.len() as u32,
            job.sources.len() as u32,
            (job.patch_radius * 2 + 1).pow(2) as u32,
            job.patch_radius as u32,
            job.needed as u32,
            ref_width,
            ref_height,
            ref_offset,
        ];
        let mut params_bytes = gpu_compute::pack_u32(&params);
        params_bytes.extend_from_slice(&gpu_compute::pack_f32(&[
            job.min_correlation,
            job.uniqueness_margin,
            job.step,
            job.ref_focal,
            job.ref_cx,
            job.ref_cy,
        ]));
        let mut srcf = Vec::with_capacity(job.sources.len() * 16 * 4);
        let mut srcm = Vec::with_capacity(job.sources.len() * 4 * 4);
        for source in &job.sources {
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
            let (offset, width, height) = self.atlas.meta[source.image]
                .expect("source image must be present in the gray atlas");
            for v in [offset, width, height, 0] {
                srcm.extend_from_slice(&v.to_ne_bytes());
            }
        }
        let pixels = job.width * job.height;
        assert_eq!(
            job.bins.len(),
            pixels * 2,
            "one (lo, hi) bin pair per pixel"
        );
        let (result_bytes, scores_bytes) = if raw_scores {
            (16u64, (pixels * job.hypotheses.len() * 4) as u64)
        } else {
            ((pixels * 16) as u64, 16u64)
        };
        let buf = |label: &str, bytes: &[u8], usage| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytes,
                usage,
            })
        };
        let params_buf = buf("params", &params_bytes, wgpu::BufferUsages::UNIFORM);
        let hyp_buf = buf(
            "hyps",
            &gpu_compute::pack_f32(&job.hypotheses),
            wgpu::BufferUsages::STORAGE,
        );
        let srcf_buf = buf("srcf", &srcf, wgpu::BufferUsages::STORAGE);
        let srcm_buf = buf("srcm", &srcm, wgpu::BufferUsages::STORAGE);
        let bins_buf = buf(
            "bins",
            &gpu_compute::pack_u32(&job.bins),
            wgpu::BufferUsages::STORAGE,
        );
        // Whichever output the entry point does not write stays a 16-byte
        // placeholder satisfying the shared layout.
        let scores_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scores"),
            size: scores_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let out_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("selection"),
            size: result_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let read_bytes = result_bytes.max(scores_bytes);
        let read_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("read"),
            size: read_bytes,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        fn entry(binding: u32, buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
            wgpu::BindGroupEntry {
                binding,
                resource: buffer.as_entire_binding(),
            }
        }
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ncc_sweep"),
            layout: &self.sweep.layout,
            entries: &[
                entry(0, &params_buf),
                entry(1, &hyp_buf),
                entry(2, &srcf_buf),
                entry(3, &srcm_buf),
                entry(4, &self.atlas.buffer),
                entry(5, &scores_buf),
                entry(6, &bins_buf),
                entry(7, &out_buf),
            ],
        });
        {
            let mut pass = self
                .encoder
                .begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("sweep_select"),
                    timestamp_writes: None,
                });
            pass.set_pipeline(if raw_scores {
                &self.sweep.scores_pipeline
            } else {
                &self.sweep.pipeline
            });
            pass.set_bind_group(0, &bind, &[]);
            pass.dispatch_workgroups(
                job.width.div_ceil(16) as u32,
                job.height.div_ceil(16) as u32,
                1,
            );
        }
        let copied = if raw_scores { &scores_buf } else { &out_buf };
        self.encoder
            .copy_buffer_to_buffer(copied, 0, &read_buf, 0, read_bytes);
        self.reads.push((read_buf, read_bytes as usize));
        self.keep.extend([
            params_buf, hyp_buf, srcf_buf, srcm_buf, bins_buf, scores_buf, out_buf,
        ]);
    }

    /// Submits once; returns the raw readback bytes per job, in push order.
    fn finish_raw(self) -> Vec<Vec<u8>> {
        let SweepBatch {
            sweep,
            encoder,
            reads,
            keep,
            ..
        } = self;
        sweep.queue.submit([encoder.finish()]);
        let results = reads
            .iter()
            .map(|(read_buf, bytes)| super::read_buffer(&sweep.device, read_buf, *bytes))
            .collect();
        drop(keep);
        results
    }

    /// Submits once; returns one selection map per pushed job (`push`), in push order.
    pub fn finish(self) -> Vec<Vec<Option<Selection>>> {
        self.finish_raw()
            .into_iter()
            .map(|raw| {
                raw.chunks_exact(16)
                    .map(|chunk| {
                        let f = |i: usize| {
                            f32::from_ne_bytes(chunk[i * 4..i * 4 + 4].try_into().unwrap())
                        };
                        (f(0) > 0.).then(|| Selection {
                            bin: f(1) as u32,
                            offset: f(2),
                            score: f(3),
                        })
                    })
                    .collect()
            })
            .collect()
    }

    /// Submits once; returns raw score rows per job (`push_scores`), in push order.
    pub fn finish_scores(self) -> Vec<Vec<f32>> {
        self.finish_raw()
            .into_iter()
            .map(|raw| {
                raw.chunks_exact(4)
                    .map(|c| f32::from_ne_bytes(c.try_into().unwrap()))
                    .collect()
            })
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

/// Thread-shared GPU sweep; `None` without an adapter, letting the caller fall
/// back to the CPU sweep.
pub fn shared() -> Option<&'static GpuSweep> {
    SHARED.with(|cell| {
        let shared: &Option<&(GpuContext, GpuSweep)> = cell;
        shared.map(|(_, sweep)| sweep)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The browser binds only entries 0..=5 for the `sweep` entry point; the
    /// native-only `sweep_select` bindings 6 and 7 must not leak into it.
    #[test]
    fn browser_sweep_entry_compiles_against_six_binding_layout() {
        let Some(context) = GpuContext::new() else {
            eprintln!("skipping: no GPU adapter");
            return;
        };
        let device = &context.device;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("browser_sweep"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                gpu_compute::uniform_entry(0),
                gpu_compute::storage_entry(1, true),
                gpu_compute::storage_entry(2, true),
                gpu_compute::storage_entry(3, true),
                gpu_compute::storage_entry(4, true),
                gpu_compute::storage_entry(5, false),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let _pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("sweep"),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("sweep"),
            compilation_options: Default::default(),
            cache: None,
        });
        let error = gpu_compute::block_on(scope.pop());
        assert!(error.is_none(), "browser layout rejected: {error:?}");
    }
}
