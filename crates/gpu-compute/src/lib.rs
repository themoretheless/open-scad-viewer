//! Shared GPU compute foundation. Used by kernels behind their own feature
//! flags; the CPU paths remain the deterministic reference and fallback.
//!
//! - `GpuContext` (always compiled): wgpu compute shaders. wgpu drives Metal on
//!   macOS, Vulkan on Linux/Windows and DX12 on Windows — NVIDIA/"CUDA-class"
//!   hardware included, because these kernels are portable WGSL and the same
//!   text runs in the browser through WebGPU.
//! - `cuda::CudaDevice` (feature `cuda`): the CUDA driver API on NVIDIA
//!   hardware for kernels that ship a dedicated PTX port. The driver library
//!   is dlopen'd at run time, so the feature builds without the CUDA toolkit
//!   and degrades to `None` (→ wgpu → CPU) on machines without it.
#![feature(
    try_blocks,
    gen_blocks,
    yield_expr,
    super_let,
    deref_patterns,
    yeet_expr
)]
#![allow(unused_features)]

pub use wgpu;

#[cfg(feature = "cuda")]
pub mod cuda;

use std::future::Future;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};
use std::time::Duration;

/// Minimal std-only block_on for one-time adapter/device requests. The waker
/// never notifies; we re-poll on a 1 ms cadence, which is negligible for
/// initialization-only futures.
struct NopWaker;
impl Wake for NopWaker {
    fn wake(self: Arc<Self>) {}
}

pub fn block_on<F: Future>(future: F) -> F::Output {
    let waker = Waker::from(Arc::new(NopWaker));
    let mut context = Context::from_waker(&waker);
    let mut future = Box::pin(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::sleep(Duration::from_millis(1)),
        }
    }
}

/// Shared device/queue for GPU-accelerated stages; `None` falls back to CPU.
pub struct GpuContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    /// The wgpu backend actually bound (Metal on macOS, Vulkan/DX12 elsewhere —
    /// including NVIDIA/"CUDA-class" hardware, which wgpu drives through
    /// Vulkan or DX12; the dedicated CUDA path lives in [`cuda`]). Kernels
    /// that template their workgroup size read this to pick a per-backend
    /// tuning.
    pub backend: wgpu::Backend,
}

impl GpuContext {
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
        let backend = adapter.get_info().backend;
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
            device,
            queue,
            backend,
        })
    }
}

/// Per-backend workgroup-size tuning for compute kernels that template their
/// WGSL source with a `WG` constant. Apple GPUs (Metal) are tile-based
/// deferred renderers whose occupancy is limited by threadgroup memory and
/// per-core execution-unit count; smaller workgroups (aligned to the 32-wide
/// SIMD-group) keep more threadgroups resident and in flight. NVIDIA GPUs —
/// which wgpu drives through Vulkan/DX12 (the dedicated CUDA path is
/// [`cuda`]) — have deep multi-warp schedulers per SM and benefit
/// from larger workgroups that hide memory latency with more warps in
/// flight, so every non-Metal backend keeps the larger default.
///
/// `metal_size` and `default_size` should both be powers of two; callers
/// that reduce across the workgroup (e.g. tree reductions halving the
/// stride) depend on that.
pub fn tuned_workgroup_size(backend: wgpu::Backend, metal_size: u32, default_size: u32) -> u32 {
    match backend {
        wgpu::Backend::Metal => metal_size,
        _ => default_size,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tuned_workgroup_size_selects_metal_variant() {
        assert_eq!(tuned_workgroup_size(wgpu::Backend::Metal, 128, 256), 128);
    }

    #[test]
    fn tuned_workgroup_size_falls_back_to_default_off_metal() {
        assert_eq!(tuned_workgroup_size(wgpu::Backend::Vulkan, 128, 256), 256);
        assert_eq!(tuned_workgroup_size(wgpu::Backend::Dx12, 128, 256), 256);
        assert_eq!(tuned_workgroup_size(wgpu::Backend::Gl, 128, 256), 256);
    }
}

/// Storage-buffer binding layout entry used by all GPU stages.
pub fn storage_entry(binding: u32, read_only: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

pub fn uniform_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

/// Blocking readback of a copy-destination buffer into a fresh Vec.
/// Returns an empty Vec on map failure, letting callers treat it as unavailability.
pub fn read_buffer(device: &wgpu::Device, buffer: &wgpu::Buffer, size: usize) -> Vec<u8> {
    let slice = buffer.slice(..size.max(4) as u64);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = tx.send(result);
    });
    let _ = device.poll(wgpu::PollType::wait_indefinitely());
    match rx.recv() {
        Ok(Ok(())) => slice
            .get_mapped_range()
            .map(|view| view.to_vec())
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// Packs values into little-endian bytes for GPU buffers (LE is the WGSL wire
/// order; on the little-endian target platforms this equals `to_ne_bytes`).
pub fn pack_f32(values: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(values.len() * 4);
    for value in values {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out
}

/// Same packing for u32 payloads (indices, aux records).
pub fn pack_u32(values: &[u32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(values.len() * 4);
    for value in values {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out
}
