use crate::{BackendReport, SubgroupReport, block_on};

/// Shared device/queue for GPU-accelerated stages; `None` falls back to CPU.
#[derive(Clone)]
pub struct GpuContext {
    inner: std::sync::Arc<GpuContextHandles>,
}

/// Immutable handles shared by clones of a context.
pub struct GpuContextHandles {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    /// The wgpu backend actually bound (Metal on macOS, Vulkan/DX12 elsewhere —
    /// including NVIDIA/"CUDA-class" hardware, which wgpu drives through
    /// Vulkan or DX12; the dedicated CUDA path lives in `crate::cuda`). Kernels
    /// that template their workgroup size read this to pick a per-backend
    /// tuning.
    pub backend: wgpu::Backend,
    /// Features advertised by the adapter, not necessarily enabled on the device.
    pub features: wgpu::Features,
    pub subgroup_min_size: u32,
    pub subgroup_max_size: u32,
}

impl std::ops::Deref for GpuContext {
    type Target = GpuContextHandles;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl GpuContext {
    pub fn same_device(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.inner, &other.inner)
    }
    pub fn new() -> Option<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            flags: wgpu::InstanceFlags::default(),
            memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
            backend_options: wgpu::BackendOptions::default(),
            display: None,
        });
        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        }))
        .ok()?;
        let info = adapter.get_info();
        let backend = info.backend;
        let features = adapter.features();
        let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("gpu-compute"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            experimental_features: wgpu::ExperimentalFeatures::default(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        }))
        .ok()?;
        Some(Self {
            inner: std::sync::Arc::new(GpuContextHandles {
                device,
                queue,
                backend,
                features,
                subgroup_min_size: info.subgroup_min_size,
                subgroup_max_size: info.subgroup_max_size,
            }),
        })
    }

    pub fn backend_label(&self) -> &'static str {
        self.backend_report().label
    }

    pub fn backend_report(&self) -> BackendReport {
        BackendReport::from_wgpu(self.backend)
    }

    pub fn enabled_features(&self) -> wgpu::Features {
        self.device.features()
    }

    /// Whether subgroups can be used on this device.
    pub fn subgroup_report(&self) -> SubgroupReport {
        SubgroupReport {
            supported: self.enabled_features().contains(wgpu::Features::SUBGROUP),
            min_size: self.subgroup_min_size,
            max_size: self.subgroup_max_size,
        }
    }
}
