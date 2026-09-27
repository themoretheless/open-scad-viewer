use crate::{BackendReport, SubgroupReport, block_on};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GpuContextError {
    Adapter(String),
    UnsupportedFeatures {
        required: wgpu::Features,
        available: wgpu::Features,
    },
    Device(String),
}
impl std::fmt::Display for GpuContextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Adapter(error) => write!(f, "GPU adapter unavailable: {error}"),
            Self::UnsupportedFeatures {
                required,
                available,
            } => write!(
                f,
                "GPU adapter does not support required features {required:?}; available: {available:?}"
            ),
            Self::Device(error) => write!(f, "GPU device creation failed: {error}"),
        }
    }
}
impl std::error::Error for GpuContextError {}

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
        Self::with_features(wgpu::Features::empty()).ok()
    }

    /// Creates a device with pass-boundary timestamp queries enabled. An
    /// unsupported adapter is an explicit error; host timing is never used as
    /// a substitute. The default `new` still enables no optional features.
    pub fn with_timestamps() -> Result<Self, GpuContextError> {
        Self::with_features(wgpu::Features::TIMESTAMP_QUERY)
    }

    /// Creates a device with explicitly requested optional features. Missing
    /// adapter support is an error; features are never silently dropped or
    /// enabled by the default constructor. For example, native subgroup timing
    /// requests `SUBGROUP | TIMESTAMP_QUERY`.
    pub fn with_features(required_features: wgpu::Features) -> Result<Self, GpuContextError> {
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
        .map_err(|error| GpuContextError::Adapter(error.to_string()))?;
        let info = adapter.get_info();
        let backend = info.backend;
        let features = adapter.features();
        validate_features(features, required_features)?;
        let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("gpu-compute"),
            required_features,
            required_limits: wgpu::Limits::default(),
            experimental_features: wgpu::ExperimentalFeatures::default(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        }))
        .map_err(|error| GpuContextError::Device(error.to_string()))?;
        Ok(Self {
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

fn validate_features(
    available: wgpu::Features,
    required: wgpu::Features,
) -> Result<(), GpuContextError> {
    if !available.contains(required) {
        return Err(GpuContextError::UnsupportedFeatures {
            required,
            available,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optional_features_require_explicit_adapter_support() {
        assert!(validate_features(wgpu::Features::empty(), wgpu::Features::empty()).is_ok());
        assert!(
            validate_features(
                wgpu::Features::TIMESTAMP_QUERY,
                wgpu::Features::TIMESTAMP_QUERY
            )
            .is_ok()
        );
        assert!(matches!(
            validate_features(wgpu::Features::empty(), wgpu::Features::TIMESTAMP_QUERY),
            Err(GpuContextError::UnsupportedFeatures {
                required: wgpu::Features::TIMESTAMP_QUERY,
                ..
            })
        ));
        let combined = wgpu::Features::SUBGROUP | wgpu::Features::TIMESTAMP_QUERY;
        assert!(validate_features(combined, combined).is_ok());
        assert!(matches!(
            validate_features(wgpu::Features::TIMESTAMP_QUERY, combined),
            Err(GpuContextError::UnsupportedFeatures { required, available })
                if required == combined && available == wgpu::Features::TIMESTAMP_QUERY
        ));
    }
}
