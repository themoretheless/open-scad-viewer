//! Optional GPU compute context (feature `gpu`). The CPU path remains the
//! deterministic reference; GPU modes are opt-in and qualified separately.
//!
//! wgpu drives Metal on macOS and Vulkan on Linux/Windows (NVIDIA included);
//! there is no separate CUDA backend: our kernels are custom shaders and the
//! CUDA hardware class is covered through Vulkan.

pub(crate) use gpu_compute::{
    pack_f32, read_buffer, storage_entry, uniform_entry, wgpu, GpuContext,
};

pub mod matching;
pub mod sweep;
