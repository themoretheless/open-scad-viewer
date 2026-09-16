//! Optional GPU compute context (feature `gpu`). The CPU path remains the
//! deterministic reference; GPU modes are opt-in and qualified separately.
//!
//! wgpu drives Metal on macOS and Vulkan on Linux/Windows (NVIDIA included);
//! there is no separate CUDA backend: our kernels are custom shaders and the
//! CUDA hardware class is covered through Vulkan. `matching`'s descriptor
//! kernel templates its workgroup size per backend via
//! `gpu_compute::tuned_workgroup_size` — smaller on Metal's tile-based
//! deferred renderers, larger on the warp-scheduled hardware (NVIDIA/CUDA-
//! class GPUs and others) reached through Vulkan.

pub(crate) use gpu_compute::{
    GpuContext, pack_f32, read_buffer, storage_entry, uniform_entry, wgpu,
};

pub mod matching;
pub mod sweep;

