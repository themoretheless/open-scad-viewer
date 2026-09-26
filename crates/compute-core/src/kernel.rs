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
#[derive(Clone, Debug)]
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
            let replaced =
                wgsl.replacen(WG_ANCHOR, &format!("const WG: u32 = {workgroup_size};"), 1);
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
            return Err(KernelError {
                message: error.to_string(),
            });
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
    /// (e.g. [`crate::shaders::BLOCK_SUM_WGSL`]) that cover arbitrary lengths with a
    /// fixed grid. `groups` must be within the 65535 per-dimension limit.
    pub fn dispatch_groups(
        &self,
        device: &Device,
        queue: &Queue,
        buffers: &[&Buffer],
        groups: u32,
    ) {
        let bind_group = self.create_bind_group(device, buffers);
        self.dispatch_bind_group(device, queue, &bind_group, groups);
    }

    /// Creates bindings once for repeated dispatches or recorded command chains.
    pub fn create_bind_group(&self, device: &Device, buffers: &[&Buffer]) -> wgpu::BindGroup {
        let resources: Vec<_> = buffers.iter().map(|buffer| buffer.as_entire_binding()).collect();
        self.create_bind_group_resources(device, &resources)
    }

    /// Binds validated subranges supplied by domain adapters. Resources must
    /// match the declared layout; use GpuBufferView to validate owners and usage.
    pub fn create_bind_group_resources(&self, device: &Device, resources: &[wgpu::BindingResource<'_>]) -> wgpu::BindGroup {
        assert_eq!(resources.len(), self.bindings.len(), "{}: wrong buffer count", self.label);
        let entries: Vec<_> = resources.iter().enumerate().map(|(index, resource)| wgpu::BindGroupEntry {
            binding: index as u32, resource: resource.clone(),
        }).collect();
        device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some(&self.label), layout: &self.bgl, entries: &entries })
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
            (1..=65535).contains(&groups),
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
