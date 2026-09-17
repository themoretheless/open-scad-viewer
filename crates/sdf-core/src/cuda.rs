//! CUDA grid sampler (feature `cuda`): runs the PTX port of `SDF_WGSL`
//! (`sdf_grid.cu` → `sdf_grid.ptx`) over the flattened field tree through
//! the CUDA driver API. f32 arithmetic — see `math_core::Acceleration`.
use crate::Grid;
use crate::flat::FlatField;
use gpu_compute::cuda::{CudaDevice, CudaFunction, PushKernelArg, launch_1d};

/// PTX generated from `sdf_grid.cu` by `scripts/build-cuda-kernels.mjs`.
pub const SDF_PTX: &str = include_str!("sdf_grid.ptx");
const BLOCK: u32 = 256;

struct CudaSdf {
    device: CudaDevice,
    kernel: CudaFunction,
}

impl CudaSdf {
    fn new() -> Option<Self> {
        let device = CudaDevice::new()?;
        let module = device.load_ptx(SDF_PTX)?;
        let kernel = module.load_function("sdf_grid").ok()?;
        Some(Self { device, kernel })
    }

    fn run(&self, flat: &FlatField, grid: &Grid) -> Option<Vec<f32>> {
        let [nx, ny, nz] = grid.cells;
        let total = (nx + 1) * (ny + 1) * (nz + 1);
        let stream = &self.device.stream;
        let kinds = self.device.upload(&flat.kinds).ok()?;
        let node_params = self.device.upload(&flat.params).ok()?;
        let aux = self.device.upload(&flat.aux).ok()?;
        let tris = self.device.upload(&flat.triangles).ok()?;
        let mut values = stream.alloc_zeros::<f32>(total.max(1)).ok()?;
        let dims = [nx as u32, ny as u32, nz as u32, flat.kinds.len() as u32];
        let min: [f32; 3] = std::array::from_fn(|i| grid.min[i] as f32);
        let step: [f32; 3] =
            std::array::from_fn(|i| ((grid.max[i] - grid.min[i]) / grid.cells[i] as f64) as f32);
        let mut launch = stream.launch_builder(&self.kernel);
        launch
            .arg(&dims[0])
            .arg(&dims[1])
            .arg(&dims[2])
            .arg(&dims[3])
            .arg(&min[0])
            .arg(&min[1])
            .arg(&min[2])
            .arg(&step[0])
            .arg(&step[1])
            .arg(&step[2])
            .arg(&kinds)
            .arg(&node_params)
            .arg(&aux)
            .arg(&tris)
            .arg(&mut values);
        unsafe { launch.launch(launch_1d(total as u32, BLOCK)) }.ok()?;
        let mut out = stream.clone_dtoh(&values).ok()?;
        out.truncate(total);
        Some(out)
    }
}

thread_local! {
    // Process-lifetime context, module and stream; see `gpu.rs`.
    static SHARED: std::cell::LazyCell<Option<&'static CudaSdf>> =
        std::cell::LazyCell::new(|| {
            CudaSdf::new().map(|sdf| Box::leak(Box::new(sdf)) as &'static CudaSdf)
        });
}

/// True when a CUDA device and the kernel module are available on this thread.
pub fn available() -> bool {
    SHARED.with(|cell| {
        let shared: &Option<&CudaSdf> = cell;
        shared.is_some()
    })
}

/// Device name for diagnostics/benchmarks; None without a CUDA device.
pub fn device_name() -> Option<String> {
    SHARED.with(|cell| {
        let shared: &Option<&CudaSdf> = cell;
        shared.map(|sdf| sdf.device.report().name)
    })
}

/// Samples the whole grid on the CUDA device; None without a device or on any
/// driver error (the caller then tries the wgpu path and the CPU reference).
pub(crate) fn sample_grid_cuda(flat: &FlatField, grid: &Grid) -> Option<Vec<f32>> {
    SHARED.with(|cell| {
        let shared: &Option<&CudaSdf> = cell;
        shared.and_then(|sdf| sdf.run(flat, grid))
    })
}
