//! Run with --features tensor-wgsl/tensor-mlx/tensor-cuda and the matching argument.
#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "gpu", feature = "tensor-mlx", feature = "tensor-cuda")
))]
use math_compute::tensor::{NeighborOptions, TensorMath};
#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "gpu", feature = "tensor-mlx", feature = "tensor-cuda")
))]
use tensor_core::{
    MatmulPrecision, Shape, TensorEvalBackend, TensorReduceBackend, TensorScatterBackend,
    TensorStatsBackend,
};
type Result = std::result::Result<(), Box<dyn std::error::Error>>;

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "gpu", feature = "tensor-mlx", feature = "tensor-cuda")
))]
fn run<B>(backend: &B) -> Result
where
    B: TensorReduceBackend + TensorScatterBackend + TensorStatsBackend + TensorEvalBackend,
    B::Error: 'static,
{
    let math = TensorMath::new(backend);
    let points = math.upload_points(&[[0., 0., 0.], [2., 0., 0.], [0., 2., 0.], [2., 2., 0.]])?;
    let matrix = backend.upload_f32(
        Shape::new(vec![3, 3])?,
        &[1., 0., 0., 0., 1., 0., 0., 0., 1.],
    )?;
    let translation = backend.upload_f32(Shape::new(vec![3])?, &[1., -2., 3.])?;
    let transformed = math.transform(&points, &matrix, &translation, MatmulPrecision::F32)?;
    let stats = math.point_cloud_stats(&transformed)?;
    let nearest = math.nearest(&transformed, &transformed, 2, NeighborOptions::default())?;
    // Everything above composes on the selected backend. Read only final values.
    let centroid = backend.read_f32(&stats.moments.centroid)?;
    let covariance = backend.read_f32(&stats.moments.covariance)?;
    let indices = backend.read_u32(&nearest.indices)?;
    for (actual, expected) in centroid.iter().zip([2., -1., 3.]) {
        assert!((actual - expected).abs() < 1e-5);
    }
    for (actual, expected) in covariance.iter().zip([1., 0., 0., 0., 1., 0., 0., 0., 0.]) {
        assert!((actual - expected).abs() < 1e-5);
    }
    assert_eq!(indices, [0, 1, 1, 0, 2, 0, 3, 1]);
    println!(
        "backend={:?} centroid={centroid:?} covariance={covariance:?} nearest={indices:?}",
        math.kind()
    );
    Ok(())
}
fn main() -> Result {
    math_compute::install();
    let backend = std::env::args().nth(1).unwrap_or_else(|| "wgsl".into());
    match backend.as_str() {
        #[cfg(all(feature = "gpu", not(target_arch = "wasm32")))]
        "wgsl" => {
            let context = gpu_compute::GpuContext::new().ok_or("WGSL device unavailable")?;
            let session = math_compute::gpu::MathGpuSession::new(&context);
            run(session.tensor()?.backend())
        }
        #[cfg(all(feature = "tensor-mlx", not(target_arch = "wasm32")))]
        "mlx" => run(&compute_mlx::MlxBackend::new_gpu()?),
        #[cfg(all(feature = "tensor-cuda", not(target_arch = "wasm32")))]
        "cuda" => run(&compute_cuda::CudaRuntime::new()?),
        _ => Err("select wgsl, mlx or cuda and enable its matching tensor-* Cargo feature".into()),
    }
}
