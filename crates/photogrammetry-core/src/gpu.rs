//! Optional GPU compute context (feature `gpu`). The CPU path remains the
//! deterministic reference; GPU modes are opt-in and qualified separately.
//!
//! wgpu drives Metal on macOS and Vulkan/DX12 on Linux/Windows (NVIDIA
//! included). These kernels are portable shaders without a dedicated CUDA
//! port, so `Acceleration::Cuda` selects this same path (`is_gpu()`); the
//! CUDA driver backend lives in `gpu_compute::cuda` for kernels that ship PTX.
//! `matching`'s descriptor kernel templates its workgroup size per backend via
//! `gpu_compute::tuned_workgroup_size` — smaller on Metal's tile-based
//! deferred renderers, larger on the warp-scheduled hardware (NVIDIA/CUDA-
//! class GPUs and others) reached through Vulkan/DX12.

pub(crate) use gpu_compute::{
    GpuContext, pack_f32, read_buffer, storage_entry, uniform_entry, wgpu,
};

pub mod matching;
pub mod sweep;

