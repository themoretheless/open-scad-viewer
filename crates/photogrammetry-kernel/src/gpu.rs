//! Optional GPU compute context (feature `gpu`). The CPU path remains the
//! deterministic reference; GPU modes are opt-in and qualified separately.
//!
//! wgpu drives Metal on macOS and Vulkan on Linux/Windows (NVIDIA included);
//! there is no separate CUDA backend: our kernels are custom shaders and the
//! CUDA hardware class is covered through Vulkan.

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

pub(crate) fn block_on<F: Future>(future: F) -> F::Output {
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
        let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("photogrammetry-gpu"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            experimental_features: wgpu::ExperimentalFeatures::default(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        }))
        .ok()?;
        Some(Self { device, queue })
    }
}

pub mod matching;
pub mod sweep;
