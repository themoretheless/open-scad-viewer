//! CUDA batch transform (feature `cuda`): runs the PTX port of
//! `TRANSFORM_WGSL` (`transform.cu` → `transform.ptx`) over a point set
//! through the CUDA driver API. f32 arithmetic — see [`crate::Acceleration`].
use crate::{M3, V3};
use gpu_compute::cuda::{CudaDevice, CudaFunction, PushKernelArg, launch_1d};

/// PTX generated from `transform.cu` by `scripts/build-cuda-kernels.mjs`.
pub const TRANSFORM_PTX: &str = include_str!("transform.ptx");
const BLOCK: u32 = 256;

struct CudaTransform {
    device: CudaDevice,
    kernel: CudaFunction,
}

impl CudaTransform {
    fn new() -> Option<Self> {
        let device = CudaDevice::new()?;
        let module = device.load_ptx(TRANSFORM_PTX)?;
        let kernel = module.load_function("transform_points").ok()?;
        Some(Self { device, kernel })
    }

    fn run(&self, points: &[V3], m: M3, t: V3) -> Option<Vec<V3>> {
        let count = points.len();
        let stream = &self.device.stream;
        let flat: Vec<f32> = points.iter().flatten().map(|&v| v as f32).collect();
        let input = self.device.upload(&flat).ok()?;
        let mut output = stream.alloc_zeros::<f32>((count * 3).max(1)).ok()?;
        let mrow: [f32; 9] = std::array::from_fn(|i| m[i / 3][i % 3] as f32);
        let tr: [f32; 3] = std::array::from_fn(|i| t[i] as f32);
        let count_u32 = count as u32;
        let mut launch = stream.launch_builder(&self.kernel);
        launch
            .arg(&mrow[0])
            .arg(&mrow[1])
            .arg(&mrow[2])
            .arg(&mrow[3])
            .arg(&mrow[4])
            .arg(&mrow[5])
            .arg(&mrow[6])
            .arg(&mrow[7])
            .arg(&mrow[8])
            .arg(&tr[0])
            .arg(&tr[1])
            .arg(&tr[2])
            .arg(&count_u32)
            .arg(&input)
            .arg(&mut output);
        unsafe { launch.launch(launch_1d(count_u32, BLOCK)) }.ok()?;
        let mut out = stream.clone_dtoh(&output).ok()?;
        out.truncate(count * 3);
        Some(
            out.chunks_exact(3)
                .map(|c| [c[0] as f64, c[1] as f64, c[2] as f64])
                .collect(),
        )
    }
}

thread_local! {
    // Process-lifetime context, module and stream; see `sdf-core`'s `cuda.rs`.
    static SHARED: std::cell::LazyCell<Option<&'static CudaTransform>> =
        std::cell::LazyCell::new(|| {
            CudaTransform::new().map(|t| Box::leak(Box::new(t)) as &'static CudaTransform)
        });
}

/// True when a CUDA device and the kernel module are available on this thread.
pub fn available() -> bool {
    SHARED.with(|cell| {
        let shared: &Option<&CudaTransform> = cell;
        shared.is_some()
    })
}

/// Device name for diagnostics/benchmarks; `None` without a CUDA device.
pub fn device_name() -> Option<String> {
    SHARED.with(|cell| {
        let shared: &Option<&CudaTransform> = cell;
        shared.map(|t| t.device.name.clone())
    })
}

/// Transforms every point on the CUDA device; `None` without a device or on
/// any driver error (the caller then tries the wgpu path and the CPU reference).
pub(crate) fn transform_points_cuda(points: &[V3], m: M3, t: V3) -> Option<Vec<V3>> {
    SHARED.with(|cell| {
        let shared: &Option<&CudaTransform> = cell;
        shared.and_then(|transform| transform.run(points, m, t))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ID, add, mv};

    #[test]
    fn cuda_transform_matches_cpu_reference_when_available() {
        let m = crate::rotation([0.1, 0.4, -0.2]);
        let t = [1.0, -2.0, 3.5];
        let points: Vec<V3> = (0..200)
            .map(|i| {
                let f = i as f64;
                [f * 0.13 - 10., f * -0.07 + 4., (f * 0.031).sin() * 5.]
            })
            .collect();
        let Some(got) = transform_points_cuda(&points, m, t) else {
            eprintln!("no CUDA device available; skipping");
            return;
        };
        assert_eq!(got.len(), points.len());
        for (p, q) in points.iter().zip(&got) {
            let expected = add(mv(m, *p), t);
            for k in 0..3 {
                assert!((expected[k] - q[k]).abs() < 5e-4, "{expected:?} vs {q:?}");
            }
        }
    }

    #[test]
    fn cuda_transform_empty_input() {
        if let Some(got) = transform_points_cuda(&[], ID, [0., 0., 0.]) {
            assert!(got.is_empty());
        }
    }

    #[test]
    fn device_probe_does_not_panic() {
        if available() {
            assert!(device_name().is_some());
        }
    }
}
