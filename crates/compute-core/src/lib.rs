//! Compute-core: the runtime library for WGSL compute kernels — the compute
//! counterpart of raster-core.
//!
//! Domain kernels (math-core, photogrammetry-core, geometry-bridge) own their
//! WGSL sources behind `include_str!` with naga validation in their own cargo
//! tests. What they share is the dispatch plumbing, and that lives here:
//!
//! - [`Kernel`]: a validated, cached compute pipeline with a declared binding
//!   layout and per-backend workgroup-size tuning (the `WG` anchor convention).
//! - Buffer helpers: typed storage/uniform uploads and readbacks on top of
//!   [`gpu_compute`] primitives.
//! - [`shaders`]: generic building-block kernels (elementwise maps, block
//!   reductions) reusable across domains.
//!
//! GPU-backed tests skip with a notice on machines without an adapter,
//! matching the `gpu-compute` convention.

pub use gpu_compute;
pub use gpu_compute::wgpu;

pub mod shaders {
    //! Generic building-block kernels, embedded at compile time.
    //!
    //! `WG` anchor convention: each source declares
    //! `const WG: u32 = 256;` and `@workgroup_size(WG)`; the runtime may
    //! substitute a tuned power-of-two before compilation.

    pub const SCALE_ADD_WGSL: &str = include_str!("../shaders/scale_add.wgsl");
    pub const ZIP_MUL_WGSL: &str = include_str!("../shaders/zip_mul.wgsl");
    pub const BLOCK_SUM_WGSL: &str = include_str!("../shaders/block_sum.wgsl");
    /// vec4 variants: 16 bytes streamed per thread instead of 4.
    pub const SCALE_ADD4_WGSL: &str = include_str!("../shaders/scale_add4.wgsl");
    pub const ZIP_MUL4_WGSL: &str = include_str!("../shaders/zip_mul4.wgsl");

    /// Every shipped kernel (name, source), for validation tests.
    pub const ALL: [(&str, &str); 5] = [
        ("scale_add", SCALE_ADD_WGSL),
        ("zip_mul", ZIP_MUL_WGSL),
        ("block_sum", BLOCK_SUM_WGSL),
        ("scale_add4", SCALE_ADD4_WGSL),
        ("zip_mul4", ZIP_MUL4_WGSL),
    ];
}

use gpu_compute::block_on;
use wgpu::{BindGroupLayout, Buffer, ComputePipeline, Device, Queue};

/// The binding a kernel expects at `group(0)`, sequential from binding 0:
/// uniforms and read-only/read-write storage buffers in declaration order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Binding {
    Uniform,
    StorageRead,
    StorageReadWrite,
}

/// Why a kernel failed to build. Construction validates eagerly (via a
/// device error scope) so a broken WGSL source is a `Result`, not a panic.
#[derive(Debug)]
pub struct KernelError {
    pub message: String,
}

impl std::fmt::Display for KernelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "kernel build failed: {}", self.message)
    }
}

impl std::error::Error for KernelError {}

/// Textual anchor every tunable kernel source must declare.
const WG_ANCHOR: &str = "const WG: u32 = 256;";

/// A compiled compute kernel: shader module, binding layout and cached
/// pipeline, ready for repeated [`dispatch`](Kernel::dispatch).
pub struct Kernel {
    label: String,
    source: String,
    entry: String,
    bindings: Vec<Binding>,
    pipeline: ComputePipeline,
    bgl: BindGroupLayout,
    workgroup_size: u32,
}

impl Kernel {
    /// Compiles `wgsl` with the `WG` anchor at its default (256).
    pub fn new(
        device: &Device,
        label: &str,
        wgsl: &str,
        entry: &str,
        bindings: &[Binding],
    ) -> Result<Self, KernelError> {
        Self::with_workgroup_size(device, label, wgsl, entry, bindings, 256)
    }

    /// Compiles `wgsl` after substituting the `WG` anchor with `workgroup_size`
    /// (must be a power of two — tree-reduction kernels halve the stride).
    pub fn with_workgroup_size(
        device: &Device,
        label: &str,
        wgsl: &str,
        entry: &str,
        bindings: &[Binding],
        workgroup_size: u32,
    ) -> Result<Self, KernelError> {
        if !workgroup_size.is_power_of_two() {
            return Err(KernelError {
                message: format!("workgroup size {workgroup_size} is not a power of two"),
            });
        }
        // Clamp to the device's per-workgroup limits so a tuner-picked size
        // never fails pipeline validation on a tighter backend (default wgpu
        // limits cap invocations per workgroup at 256). The size is a power
        // of two, so halving preserves that.
        let limits = device.limits();
        let max_size = limits
            .max_compute_invocations_per_workgroup
            .min(limits.max_compute_workgroup_size_x)
            .max(1);
        let mut workgroup_size = workgroup_size;
        while workgroup_size > max_size {
            workgroup_size /= 2;
        }
        let source = if workgroup_size == 256 {
            wgsl.to_string()
        } else {
            let replaced = wgsl.replacen(WG_ANCHOR, &format!("const WG: u32 = {workgroup_size};"), 1);
            if replaced == wgsl {
                return Err(KernelError {
                    message: format!("WG anchor ({WG_ANCHOR}) missing from kernel source"),
                });
            }
            replaced
        };
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(label),
            source: wgpu::ShaderSource::Wgsl(source.clone().into()),
        });
        let entries: Vec<wgpu::BindGroupLayoutEntry> = bindings
            .iter()
            .enumerate()
            .map(|(index, binding)| match binding {
                Binding::Uniform => gpu_compute::uniform_entry(index as u32),
                Binding::StorageRead => gpu_compute::storage_entry(index as u32, true),
                Binding::StorageReadWrite => gpu_compute::storage_entry(index as u32, false),
            })
            .collect();
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(label),
            entries: &entries,
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(label),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(label),
            layout: Some(&layout),
            module: &module,
            entry_point: Some(entry),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        if let Some(error) = block_on(scope.pop()) {
            return Err(KernelError { message: error.to_string() });
        }
        Ok(Self {
            label: label.to_string(),
            source,
            entry: entry.to_string(),
            bindings: bindings.to_vec(),
            pipeline,
            bgl,
            workgroup_size,
        })
    }

    /// Compiles with a workgroup size tuned per backend via
    /// [`gpu_compute::tuned_workgroup_size`]: smaller groups on Metal
    /// (tile-based GPUs), larger elsewhere.
    pub fn tuned(
        context: &gpu_compute::GpuContext,
        label: &str,
        wgsl: &str,
        entry: &str,
        bindings: &[Binding],
        metal_size: u32,
        default_size: u32,
    ) -> Result<Self, KernelError> {
        let size = gpu_compute::tuned_workgroup_size(context.backend, metal_size, default_size);
        Self::with_workgroup_size(&context.device, label, wgsl, entry, bindings, size)
    }

    pub fn workgroup_size(&self) -> u32 {
        self.workgroup_size
    }

    /// The WGSL source after `WG` substitution, as compiled.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// The shader entry point this pipeline binds.
    pub fn entry_point(&self) -> &str {
        &self.entry
    }

    /// The bind group layout buffers must match to be dispatched — lets
    /// callers cache bind groups alongside their grow-only buffer pools and
    /// dispatch them via [`dispatch_bind_group`](Kernel::dispatch_bind_group).
    pub fn bind_group_layout(&self) -> &BindGroupLayout {
        &self.bgl
    }

    /// Number of workgroups needed to cover `invocations` threads.
    pub fn workgroup_count(&self, invocations: u32) -> u32 {
        invocations.div_ceil(self.workgroup_size)
    }

    /// Largest invocation count a single dispatch covers: the wgpu per-dimension
    /// workgroup-count limit (65535) times this kernel's workgroup size.
    /// Larger workloads are the caller's job to chunk.
    pub fn max_dispatch_invocations(&self) -> u32 {
        65535 * self.workgroup_size
    }

    /// Dispatches over `buffers` (one per declared [`Binding`], binding 0..n),
    /// covering `invocations` threads.
    pub fn dispatch(&self, device: &Device, queue: &Queue, buffers: &[&Buffer], invocations: u32) {
        assert!(
            invocations <= self.max_dispatch_invocations(),
            "{}: {} invocations exceed the single-dispatch limit of {} (chunk the workload)",
            self.label,
            invocations,
            self.max_dispatch_invocations()
        );
        self.dispatch_groups(device, queue, buffers, self.workgroup_count(invocations))
    }

    /// Dispatches exactly `groups` workgroups — for grid-strided kernels
    /// (e.g. [`shaders::BLOCK_SUM_WGSL`]) that cover arbitrary lengths with a
    /// fixed grid. `groups` must be within the 65535 per-dimension limit.
    pub fn dispatch_groups(&self, device: &Device, queue: &Queue, buffers: &[&Buffer], groups: u32) {
        assert_eq!(
            buffers.len(),
            self.bindings.len(),
            "{}: expected {} buffers, got {}",
            self.label,
            self.bindings.len(),
            buffers.len()
        );
        assert!(
            groups >= 1 && groups <= 65535,
            "{}: workgroup count {} outside 1..=65535",
            self.label,
            groups
        );
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&self.label),
            layout: &self.bgl,
            entries: &buffers
                .iter()
                .enumerate()
                .map(|(index, buffer)| wgpu::BindGroupEntry {
                    binding: index as u32,
                    resource: buffer.as_entire_binding(),
                })
                .collect::<Vec<_>>(),
        });
        self.dispatch_bind_group(device, queue, &bind_group, groups);
    }

    /// Dispatches with a caller-cached bind group (built against
    /// [`bind_group_layout`](Kernel::bind_group_layout)) — the fast path for
    /// repeated dispatches over grow-only buffer pools.
    pub fn dispatch_bind_group(
        &self,
        device: &Device,
        queue: &Queue,
        bind_group: &wgpu::BindGroup,
        groups: u32,
    ) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some(&self.label),
        });
        self.record_dispatch(&mut encoder, bind_group, groups);
        queue.submit([encoder.finish()]);
    }

    /// Records a dispatch into a caller-owned encoder — the building block
    /// for multi-kernel command buffers that submit once (batch pipelines),
    /// where each dispatch may bind different buffers.
    pub fn record_dispatch(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        bind_group: &wgpu::BindGroup,
        groups: u32,
    ) {
        assert!(
            groups >= 1 && groups <= 65535,
            "{}: workgroup count {} outside 1..=65535",
            self.label,
            groups
        );
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some(&self.label),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.dispatch_workgroups(groups, 1, 1);
    }
}

/// Uploads `data` as a storage buffer (STORAGE | COPY_DST).
pub fn storage_f32(device: &Device, queue: &Queue, data: &[f32]) -> Buffer {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("compute storage f32"),
        size: (data.len() * 4).max(4) as u64,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    queue.write_buffer(&buffer, 0, &gpu_compute::pack_f32(data));
    buffer
}

/// Zero-initialized f32 storage buffer (read_write outputs).
pub fn storage_f32_zeroed(device: &Device, queue: &Queue, len: usize) -> Buffer {
    storage_f32(device, queue, &vec![0.0f32; len])
}

/// Same packing for u32 payloads (indices, counters).
pub fn storage_u32(device: &Device, queue: &Queue, data: &[u32]) -> Buffer {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("compute storage u32"),
        size: (data.len() * 4).max(4) as u64,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    queue.write_buffer(&buffer, 0, &gpu_compute::pack_u32(data));
    buffer
}

/// Uniform buffer from raw float payloads (callers pack structs with
/// [`gpu_compute::pack_f32`] semantics: 16-byte alignment is the caller's job).
pub fn uniform_f32(device: &Device, queue: &Queue, floats: &[f32]) -> Buffer {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("compute uniform"),
        size: (floats.len() * 4).max(16) as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&buffer, 0, &gpu_compute::pack_f32(floats));
    buffer
}

/// Copies `buffer[0..size]` into a MAP_READ staging buffer and maps it.
/// Storage buffers cannot hold MAP_READ (wgpu restricts it to COPY_DST
/// companions), so readback goes through an explicit copy.
fn readback(device: &Device, queue: &Queue, buffer: &Buffer, size: usize) -> Vec<u8> {
    let size = size.max(4);
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("compute readback staging"),
        size: size as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("compute readback"),
    });
    encoder.copy_buffer_to_buffer(buffer, 0, &staging, 0, size as u64);
    queue.submit([encoder.finish()]);
    gpu_compute::read_buffer(device, &staging, size)
}

/// Typed readback of `count` f32 values (little-endian wire order).
pub fn read_f32(device: &Device, queue: &Queue, buffer: &Buffer, count: usize) -> Vec<f32> {
    let bytes = readback(device, queue, buffer, count * 4);
    bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes(chunk.try_into().expect("f32 chunk")))
        .collect()
}

/// Typed readback of `count` u32 values.
pub fn read_u32(device: &Device, queue: &Queue, buffer: &Buffer, count: usize) -> Vec<u32> {
    let bytes = readback(device, queue, buffer, count * 4);
    bytes
        .chunks_exact(4)
        .map(|chunk| u32::from_le_bytes(chunk.try_into().expect("u32 chunk")))
        .collect()
}

/// Full reduction of `input[0..count]` to a scalar via repeated
/// [`shaders::BLOCK_SUM_WGSL`] passes: each pass runs a grid-strided dispatch
/// (up to 65535 workgroups), feeding the partials back as the next pass's
/// input until a single partial remains. Lengths are arbitrary — the 65535
/// per-dispatch group limit only shapes the pass schedule, not the total.
pub fn reduce_f32(device: &Device, queue: &Queue, sum: &Kernel, input: &Buffer, mut count: u32) -> f32 {
    let params = uniform_f32(device, queue, &[0.0; 4]);
    let mut source = input.clone();
    loop {
        let groups = count.div_ceil(sum.workgroup_size()).min(65535);
        let mut floats = vec![0.0f32; 4];
        floats[0] = f32::from_le_bytes(count.to_le_bytes());
        floats[1] = f32::from_le_bytes(groups.to_le_bytes());
        queue.write_buffer(&params, 0, &gpu_compute::pack_f32(&floats));
        let partials = storage_f32_zeroed(device, queue, groups as usize);
        sum.dispatch_groups(device, queue, &[&params, &source, &partials], groups);
        if groups == 1 {
            return read_f32(device, queue, &partials, 1)
                .first()
                .copied()
                .unwrap_or(0.0);
        }
        source = partials;
        count = groups;
    }
}
