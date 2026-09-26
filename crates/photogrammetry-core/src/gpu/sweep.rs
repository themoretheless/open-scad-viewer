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
//!
//! With the `cuda` feature, [`cuda::CudaSweep`] runs the same sweep-and-select
//! kernel as native PTX (`sweep.cu`) and `Acceleration::Cuda` prefers it; it
//! declines (returns `None`) for option sets it does not cover, leaving the
//! wgpu sweep and then the CPU sweep in charge.

use super::wgpu;
use super::wgpu::util::DeviceExt;

use super::GpuContext;
use compute_core::{Binding, Kernel};

const WG_METAL: u32 = 256;
const WG_DEFAULT: u32 = 256;

const SHADER: &str = crate::dense::SWEEP_WGSL;

// The browser-shared shader dispatches pixels as a 2D grid; the native path
// linearizes both entry points to 1D (one thread per pixel, `index ->
// (x, y)`) to fit the compute-core dispatch convention — the same rewrite the
// host variant (`dense::host_sweep_linear_indexing_wgsl`) already ships for
// browser hosts. The select entry introduces the shared `WG` anchor; the
// scores entry reuses it, so it is declared exactly once.
const SELECT_ENTRY_2D: &str = r#"@compute @workgroup_size(16, 16)
fn sweep_select(@builtin(global_invocation_id) id: vec3<u32>) {
    let x = id.x;
    let y = id.y;"#;

const SELECT_ENTRY_LINEAR: &str = r#"// `WG` is the workgroup-size anchor: the compute-core runtime may substitute
// a per-backend tuned value (powers of two only) before compilation.
const WG: u32 = 256;

@compute @workgroup_size(WG)
fn sweep_select(@builtin(global_invocation_id) id: vec3<u32>) {
    let index = id.x;
    let total = params.width * params.height;
    if (index >= total) {
        return;
    }
    let x = index % params.width;
    let y = index / params.width;"#;

const SCORES_ENTRY_2D: &str = r#"@compute @workgroup_size(16, 16)
fn sweep(@builtin(global_invocation_id) id: vec3<u32>) {
    let x = id.x;
    let y = id.y;"#;

const SCORES_ENTRY_LINEAR: &str = r#"@compute @workgroup_size(WG)
fn sweep(@builtin(global_invocation_id) id: vec3<u32>) {
    let index = id.x;
    let total = params.width * params.height;
    if (index >= total) {
        return;
    }
    let x = index % params.width;
    let y = index / params.width;"#;

/// The native sweep shader: both entries linearized to the compute-core 1D
/// convention with a tunable `WG` anchor.
fn linear_shader() -> String {
    SHADER
        .replace(SELECT_ENTRY_2D, SELECT_ENTRY_LINEAR)
        .replace(SCORES_ENTRY_2D, SCORES_ENTRY_LINEAR)
}

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
    /// Native `sweep_select` (linear-indexed, register-resident selection).
    kernel: Kernel,
    /// The browser-shared `sweep` entry (raw scores), kept for qualification.
    scores_kernel: Kernel,
}

impl GpuSweep {
    pub fn new(context: &GpuContext) -> Self {
        let source = linear_shader();
        let kernel = Kernel::tuned(
            context,
            "ncc_sweep_select",
            &source,
            "sweep_select",
            &[
                Binding::Uniform,
                Binding::StorageRead,
                Binding::StorageRead,
                Binding::StorageRead,
                Binding::StorageRead,
                Binding::StorageReadWrite,
                Binding::StorageRead,
                Binding::StorageReadWrite,
            ],
            WG_METAL,
            WG_DEFAULT,
        )
        .expect("sweep_select kernel builds");
        // The qualification pipeline binds only entries 0..=5 (the browser
        // layout); the module's extra globals stay unused by this entry.
        let scores_kernel = Kernel::new(
            &context.device,
            "ncc_sweep",
            &source,
            "sweep",
            &[
                Binding::Uniform,
                Binding::StorageRead,
                Binding::StorageRead,
                Binding::StorageRead,
                Binding::StorageRead,
                Binding::StorageReadWrite,
            ],
        )
        .expect("sweep kernel builds");
        Self {
            device: context.device.clone(),
            queue: context.queue.clone(),
            kernel,
            scores_kernel,
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
        // The qualification `sweep` entry binds only 0..=5 (the browser
        // layout); the native `sweep_select` entry adds bins and selection.
        let kernel = if raw_scores {
            &self.sweep.scores_kernel
        } else {
            &self.sweep.kernel
        };
        let mut entries = vec![
            entry(0, &params_buf),
            entry(1, &hyp_buf),
            entry(2, &srcf_buf),
            entry(3, &srcm_buf),
            entry(4, &self.atlas.buffer),
            entry(5, &scores_buf),
        ];
        if !raw_scores {
            entries.push(entry(6, &bins_buf));
            entries.push(entry(7, &out_buf));
        }
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ncc_sweep"),
            layout: kernel.bind_group_layout(),
            entries: &entries,
        });
        kernel.record_dispatch(
            &mut self.encoder,
            &bind,
            kernel.workgroup_count(pixels as u32),
        );
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

/// Native CUDA port of the sweep-and-select kernel (feature `cuda`). Mirrors
/// the WGSL `sweep_select` entry line by line and consumes the same
/// [`SweepJob`] payloads, so `Acceleration::Cuda` can skip the wgpu layer on
/// NVIDIA hardware. Every entry point returns `None` when the driver, a
/// device, the module or an unsupported option set makes the native path
/// unavailable; callers then fall back to the wgpu sweep and finally the CPU.
#[cfg(feature = "cuda")]
pub mod cuda {
    use super::{Selection, SweepJob};
    use gpu_compute::cuda::{CudaDevice, CudaFunction, CudaSlice, LaunchConfig, PushKernelArg};

    const SWEEP_PTX: &str = include_str!("sweep.ptx");
    /// 16x16 threads per block, matching the shader's workgroup shape.
    const BLOCK: u32 = 16;
    /// Kernel-side `sc` / `centered` capacities; larger jobs fall back.
    const MAX_HYPOTHESES: usize = 128;
    const MAX_TAPS: usize = 25;

    /// All grayscale rasters of a pass in one device buffer, laid out exactly
    /// like the wgpu [`super::GrayAtlas`].
    pub struct GrayAtlas {
        buffer: CudaSlice<f32>,
        /// (offset in floats, width, height) per image index; None for absent images.
        meta: Vec<Option<(u32, u32, u32)>>,
    }

    pub struct CudaSweep {
        device: CudaDevice,
        kernel: CudaFunction,
    }

    impl CudaSweep {
        fn new() -> Option<Self> {
            let device = CudaDevice::new()?;
            let module = device.load_ptx(SWEEP_PTX)?;
            let kernel = module.load_function("sweep_select").ok()?;
            Some(Self { device, kernel })
        }

        pub fn device_name(&self) -> &str {
            &self.device.name
        }

        /// Uploads every present raster once; `images[i] = Some((values, width, height))`.
        pub fn upload_grays(&self, images: &[Option<(&[f32], usize, usize)>]) -> Option<GrayAtlas> {
            let total: usize = images
                .iter()
                .flatten()
                .map(|(values, _, _)| values.len())
                .sum();
            let mut values: Vec<f32> = Vec::with_capacity(total);
            let mut meta = Vec::with_capacity(images.len());
            for image in images {
                meta.push(image.map(|(raster, width, height)| {
                    let offset = values.len() as u32;
                    values.extend_from_slice(raster);
                    (offset, width as u32, height as u32)
                }));
            }
            let buffer = self.device.upload(&values).ok()?;
            Some(GrayAtlas { buffer, meta })
        }

        /// True when the native kernel covers this job exactly; unsupported
        /// option sets (oversized hypothesis grid or patch, missing raster)
        /// keep the wgpu/CPU path instead of approximating it.
        fn supported(&self, atlas: &GrayAtlas, job: &SweepJob) -> bool {
            let taps = (job.patch_radius * 2 + 1).pow(2);
            let present = |image: usize| atlas.meta.get(image).copied().flatten().is_some();
            (2..=MAX_HYPOTHESES).contains(&job.hypotheses.len())
                && taps <= MAX_TAPS
                && job.bins.len() == job.width * job.height * 2
                && present(job.ref_image)
                && job.sources.iter().all(|source| present(source.image))
        }

        /// Runs one view; `None` when the job is unsupported or any driver call
        /// fails. Selections are laid out per pixel in row-major map order.
        pub fn run(&self, atlas: &GrayAtlas, job: &SweepJob) -> Option<Vec<Option<Selection>>> {
            if !self.supported(atlas, job) {
                return None;
            }
            let pixels = job.width * job.height;
            if pixels == 0 {
                return Some(Vec::new());
            }
            let (ref_offset, ref_width, ref_height) = atlas.meta[job.ref_image]?;
            let mut srcf: Vec<f32> = Vec::with_capacity(job.sources.len() * 16);
            let mut srcm: Vec<u32> = Vec::with_capacity(job.sources.len() * 4);
            for source in &job.sources {
                srcf.extend(source.rotation.iter().flatten().copied());
                srcf.extend_from_slice(&source.translation);
                srcf.extend_from_slice(&[source.focal, source.cx, source.cy, 0.]);
                let (offset, width, height) = atlas.meta[source.image]?;
                srcm.extend_from_slice(&[offset, width, height, 0]);
            }
            let stream = &self.device.stream;
            let hyps = self.device.upload(&job.hypotheses).ok()?;
            let srcf = self.device.upload(&srcf).ok()?;
            let srcm = self.device.upload(&srcm).ok()?;
            let bins = self.device.upload(&job.bins).ok()?;
            let mut results = stream.alloc_zeros::<f32>(pixels * 4).ok()?;
            let width = job.width as u32;
            let height = job.height as u32;
            let n_hyp = job.hypotheses.len() as u32;
            let n_src = job.sources.len() as u32;
            let plen = ((job.patch_radius * 2 + 1).pow(2)) as u32;
            let radius = job.patch_radius as u32;
            let needed = job.needed as u32;
            let mut launch = stream.launch_builder(&self.kernel);
            launch
                .arg(&width)
                .arg(&height)
                .arg(&n_hyp)
                .arg(&n_src)
                .arg(&plen)
                .arg(&radius)
                .arg(&needed)
                .arg(&ref_width)
                .arg(&ref_height)
                .arg(&ref_offset)
                .arg(&job.min_correlation)
                .arg(&job.uniqueness_margin)
                .arg(&job.step)
                .arg(&job.ref_focal)
                .arg(&job.ref_cx)
                .arg(&job.ref_cy)
                .arg(&hyps)
                .arg(&srcf)
                .arg(&srcm)
                .arg(&atlas.buffer)
                .arg(&bins)
                .arg(&mut results);
            let config = LaunchConfig {
                grid_dim: (
                    width.div_ceil(BLOCK).max(1),
                    height.div_ceil(BLOCK).max(1),
                    1,
                ),
                block_dim: (BLOCK, BLOCK, 1),
                shared_mem_bytes: 0,
            };
            unsafe { launch.launch(config) }.ok()?;
            let mut host = vec![0f32; pixels * 4];
            stream.memcpy_dtoh(&results, &mut host).ok()?;
            stream.synchronize().ok()?;
            Some(
                host.chunks_exact(4)
                    .map(|chunk| {
                        (chunk[0] > 0.).then(|| Selection {
                            bin: chunk[1] as u32,
                            offset: chunk[2],
                            score: chunk[3],
                        })
                    })
                    .collect(),
            )
        }

        /// Runs every view of a pass against one uploaded atlas. `None` as soon
        /// as a single job is unsupported, so the caller keeps one consistent
        /// backend for the whole pass.
        pub fn run_batch(
            &self,
            atlas: &GrayAtlas,
            jobs: &[SweepJob],
        ) -> Option<Vec<Vec<Option<Selection>>>> {
            jobs.iter().map(|job| self.run(atlas, job)).collect()
        }
    }

    thread_local! {
        // Process-lifetime device and module; see gpu::matching for the leak note.
        static SHARED: std::cell::LazyCell<Option<&'static CudaSweep>> =
            std::cell::LazyCell::new(|| {
                CudaSweep::new().map(|sweep| Box::leak(Box::new(sweep)) as &'static CudaSweep)
            });
    }

    /// Thread-shared native CUDA sweep; `None` without a CUDA device, letting
    /// the caller fall back to the wgpu sweep and then the CPU.
    pub fn shared() -> Option<&'static CudaSweep> {
        SHARED.with(|cell| {
            let shared: &Option<&CudaSweep> = cell;
            *shared
        })
    }
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
