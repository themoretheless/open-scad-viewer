//! CUDA driver-API device for kernels with a dedicated PTX port (feature
//! `cuda`). Mirrors [`crate::GpuContext`]: construction returns `None` when
//! the driver library, a device or the primary context is unavailable, and
//! callers fall through to the wgpu shader and then the CPU reference.
//!
//! The driver (`nvcuda.dll` / `libcuda.so`) is loaded with `libloading` at run
//! time; nothing links against the CUDA toolkit, and the toolkit is only
//! needed to regenerate the checked-in PTX (`npm run build:cuda-kernels`).
pub use cudarc;
pub use cudarc::driver::{
    CudaContext, CudaFunction, CudaModule, CudaSlice, CudaStream, DriverError, LaunchConfig,
    PushKernelArg,
};

use std::sync::Arc;

/// Environment override for the device ordinal (`OSV_CUDA_DEVICE=1`); the
/// default is device 0.
pub const DEVICE_ENV: &str = "OSV_CUDA_DEVICE";

pub struct CudaDevice {
    pub context: Arc<CudaContext>,
    pub stream: Arc<CudaStream>,
    pub name: String,
    pub multiprocessors: u32,
}

impl CudaDevice {
    /// Retains the primary context of the selected device. `None` when the
    /// driver library is absent, no device exists, or initialization fails.
    pub fn new() -> Option<Self> {
        // The generated bindings panic when the library is missing; probe first
        // so a CPU-only machine simply reports "no CUDA".
        if !unsafe { cudarc::driver::sys::is_culib_present() } {
            return None;
        }
        let ordinal = std::env::var(DEVICE_ENV)
            .ok()
            .and_then(|value| value.trim().parse::<usize>().ok())
            .unwrap_or(0);
        let context = std::panic::catch_unwind(|| CudaContext::new(ordinal))
            .ok()?
            .ok()?;
        let name = context.name().ok()?;
        let multiprocessors = context
            .attribute(
                cudarc::driver::sys::CUdevice_attribute::CU_DEVICE_ATTRIBUTE_MULTIPROCESSOR_COUNT,
            )
            .ok()?
            .max(1) as u32;
        let stream = context.default_stream();
        Some(Self {
            context,
            stream,
            name,
            multiprocessors,
        })
    }

    /// JIT-loads PTX text into this context.
    pub fn load_ptx(&self, ptx: &str) -> Option<Arc<CudaModule>> {
        self.context
            .load_module(cudarc::nvrtc::Ptx::from_src(ptx))
            .ok()
    }

    /// Uploads a host slice; empty inputs upload one zero element so every
    /// kernel parameter is a valid device pointer.
    pub fn upload<T: cudarc::driver::DeviceRepr + Default + Clone>(
        &self,
        values: &[T],
    ) -> Result<CudaSlice<T>, DriverError> {
        if values.is_empty() {
            self.stream.clone_htod(&[T::default()])
        } else {
            self.stream.clone_htod(values)
        }
    }
}

/// One-dimensional launch over `total` threads with `block` threads per block.
pub fn launch_1d(total: u32, block: u32) -> LaunchConfig {
    LaunchConfig {
        grid_dim: (total.div_ceil(block).max(1), 1, 1),
        block_dim: (block, 1, 1),
        shared_mem_bytes: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_1d_rounds_up() {
        let cfg = launch_1d(1000, 256);
        assert_eq!(cfg.grid_dim, (4, 1, 1));
        assert_eq!(cfg.block_dim, (256, 1, 1));
        assert_eq!(launch_1d(0, 256).grid_dim, (1, 1, 1));
    }

    #[test]
    fn device_probe_does_not_panic() {
        // Either a device or a clean None; both are valid on the test host.
        if let Some(device) = CudaDevice::new() {
            assert!(!device.name.is_empty());
            assert!(device.multiprocessors >= 1);
        }
    }
}
