//! Rasterizer library: the pixel-render counterpart of the compute kernels.
//!
//! The WGSL sources in `shaders/` are the single source of truth for the
//! raster shaders. The same text runs in the browser through WebGPU (the
//! TypeScript renderer embeds generated copies) and natively through wgpu
//! (Metal on macOS, Vulkan/DX12 elsewhere — including NVIDIA hardware).
//!
//! - [`shaders`]: WGSL sources via `include_str!`.
//! - [`uniform`]: CPU-side mirror of the `Scene`/`Obj` uniform structs.
//! - [`variants`]: textual `instanced`/`immediate` shader variants.
//! - [`pipeline`]: cached render pipelines for every shader.
//! - [`rasterizer`]: headless offscreen rasterizer used by tests and
//!   available for native embedding.

pub use gpu_compute;
pub use wgpu;

pub mod pipeline;
pub mod rasterizer;
pub mod shaders;
pub mod uniform;
pub mod variants;
