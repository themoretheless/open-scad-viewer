//! Optional GPU compute context (feature `gpu`). The CPU path remains the
//! deterministic reference; GPU modes are opt-in and qualified separately.
//!
//! wgpu drives Metal on macOS and Vulkan/DX12 on Linux/Windows (NVIDIA
//! included). `matching` and `sweep` also have native CUDA PTX ports when the
//! `cuda` feature is enabled; other kernels use the portable shader fallback.
//! `matching`'s descriptor kernel templates its workgroup size per backend via
//! `gpu_compute::tuned_workgroup_size` — smaller on Metal's tile-based
//! deferred renderers, larger on the warp-scheduled hardware (NVIDIA/CUDA-
//! class GPUs and others) reached through Vulkan/DX12.

#[cfg(feature = "cuda")]
pub use gpu_compute::cuda::CudaDeviceReport;
pub(crate) use gpu_compute::{BackendReport, GpuContext, pack_f32, read_buffer, wgpu};

pub mod matching;
pub mod rectification;
pub mod sweep;

pub fn backend_label() -> Option<&'static str> {
    backend_report().map(|report| report.label)
}

pub fn backend_report() -> Option<BackendReport> {
    GpuContext::new().map(|context| context.backend_report())
}

pub fn subgroup_report() -> Option<gpu_compute::SubgroupReport> {
    GpuContext::new().map(|context| context.subgroup_report())
}

#[cfg(feature = "cuda")]
pub fn cuda_device_report() -> Option<CudaDeviceReport> {
    gpu_compute::cuda::available_device_report()
}

#[cfg(feature = "cuda")]
pub fn cuda_device_name() -> Option<String> {
    cuda_device_report().map(|report| report.name)
}
