//! CUDA grid sampler (feature `cuda`): runs the PTX port of `SDF_WGSL`
//! (`sdf_grid.cu` → `sdf_grid.ptx`) over the flattened field tree through
//! the CUDA driver API. f32 arithmetic — see `math_core::Acceleration`.
use crate::Grid;
use crate::flat::FlatField;
use gpu_compute::cuda::{
    CudaDevice, CudaDeviceReport, CudaFunction, CudaSlice, PushKernelArg, launch_1d,
};

/// PTX generated from `sdf_grid.cu` by `scripts/build-cuda-kernels.mjs`.
pub const SDF_PTX: &str = include_str!("sdf_grid.ptx");
const BLOCK: u32 = 256;

struct CudaSdf {
    device: CudaDevice,
    kernel: CudaFunction,
    buffers: std::cell::RefCell<Option<Buffers>>,
}

struct Buffers {
    kind_capacity: usize,
    param_capacity: usize,
    aux_capacity: usize,
    triangle_capacity: usize,
    value_capacity: usize,
    kinds: CudaSlice<u32>,
    node_params: CudaSlice<f32>,
    aux: CudaSlice<u32>,
    triangles: CudaSlice<f32>,
    values: CudaSlice<f32>,
}

impl CudaSdf {
    fn new() -> Option<Self> {
        let device = CudaDevice::new()?;
        let module = device.load_ptx(SDF_PTX)?;
        let kernel = module.load_function("sdf_grid").ok()?;
        Some(Self {
            device,
            kernel,
            buffers: std::cell::RefCell::new(None),
        })
    }

    fn ensure_buffers(&self, flat: &FlatField, value_count: usize) -> Option<()> {
        let stale = match &*self.buffers.borrow() {
            Some(b) => {
                b.kind_capacity < flat.kinds.len()
                    || b.param_capacity < flat.params.len()
                    || b.aux_capacity < flat.aux.len()
                    || b.triangle_capacity < flat.triangles.len()
                    || b.value_capacity < value_count
            }
            None => true,
        };
        if !stale {
            return Some(());
        }
        let stream = &self.device.stream;
        let kind_capacity = flat.kinds.len().max(1);
        let param_capacity = flat.params.len().max(1);
        let aux_capacity = flat.aux.len().max(1);
        let triangle_capacity = flat.triangles.len().max(1);
        let value_capacity = value_count.max(1);
        let kinds = stream.alloc_zeros::<u32>(kind_capacity).ok()?;
        let node_params = stream.alloc_zeros::<f32>(param_capacity).ok()?;
        let aux = stream.alloc_zeros::<u32>(aux_capacity).ok()?;
        let triangles = stream.alloc_zeros::<f32>(triangle_capacity).ok()?;
        let values = stream.alloc_zeros::<f32>(value_capacity).ok()?;
        *self.buffers.borrow_mut() = Some(Buffers {
            kind_capacity,
            param_capacity,
            aux_capacity,
            triangle_capacity,
            value_capacity,
            kinds,
            node_params,
            aux,
            triangles,
            values,
        });
        Some(())
    }

    fn run(&self, flat: &FlatField, grid: &Grid) -> Option<Vec<f32>> {
        let [nx, ny, nz] = grid.cells;
        let total = (nx + 1) * (ny + 1) * (nz + 1);
        let stream = &self.device.stream;
        self.ensure_buffers(flat, total)?;
        let mut buffers = self.buffers.borrow_mut();
        let b = buffers.as_mut().expect("ensure_buffers was just called");
        if !flat.kinds.is_empty() {
            let mut view = b.kinds.slice_mut(0..flat.kinds.len());
            stream.memcpy_htod(&flat.kinds, &mut view).ok()?;
        }
        if !flat.params.is_empty() {
            let mut view = b.node_params.slice_mut(0..flat.params.len());
            stream.memcpy_htod(&flat.params, &mut view).ok()?;
        }
        if !flat.aux.is_empty() {
            let mut view = b.aux.slice_mut(0..flat.aux.len());
            stream.memcpy_htod(&flat.aux, &mut view).ok()?;
        }
        if !flat.triangles.is_empty() {
            let mut view = b.triangles.slice_mut(0..flat.triangles.len());
            stream.memcpy_htod(&flat.triangles, &mut view).ok()?;
        }
        let dims = [nx as u32, ny as u32, nz as u32, flat.kinds.len() as u32];
        let min: [f32; 3] = std::array::from_fn(|i| grid.min[i] as f32);
        let step: [f32; 3] =
            std::array::from_fn(|i| ((grid.max[i] - grid.min[i]) / grid.cells[i] as f64) as f32);
        let kinds = b.kinds.slice(0..flat.kinds.len().max(1));
        let node_params = b.node_params.slice(0..flat.params.len().max(1));
        let aux = b.aux.slice(0..flat.aux.len().max(1));
        let tris = b.triangles.slice(0..flat.triangles.len().max(1));
        let mut values = b.values.slice_mut(0..total.max(1));
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
        let mut out = vec![0f32; total];
        if total > 0 {
            stream.memcpy_dtoh(&values, &mut out).ok()?;
        }
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
    device_report().map(|report| report.name)
}

pub fn device_report() -> Option<CudaDeviceReport> {
    SHARED.with(|cell| {
        let shared: &Option<&CudaSdf> = cell;
        shared.map(|sdf| sdf.device.report())
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
