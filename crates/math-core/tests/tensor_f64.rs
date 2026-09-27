#![cfg(all(feature = "tensor-cuda", not(target_arch = "wasm32")))]
use math_core::tensor::{
    TensorMathF64,
    cuda::{CudaError, CudaRuntime},
};
use tensor_core::{Shape, TensorF64Backend};
fn shape(dims: &[usize]) -> Shape {
    Shape::new(dims.to_vec()).unwrap()
}
#[test]
fn native_f64_geometry_preserves_local_differences_and_large_range()
-> Result<(), Box<dyn std::error::Error>> {
    let b = match CudaRuntime::new() {
        Ok(b) => b,
        Err(error @ CudaError::Unavailable(_)) => {
            assert!(
                !["CUDA_REQUIRED", "COMPUTE_REQUIRE_CUDA"]
                    .iter()
                    .any(|name| std::env::var(name).as_deref() == Ok("1")),
                "required CUDA f64 geometry unavailable: {error}"
            );
            eprintln!("SKIP native CUDA f64 geometry: {error}");
            return Ok(());
        }
        Err(error) => return Err(error.into()),
    };
    let math = TensorMathF64::new(&b);
    let origin = 100_000_000.;
    let points = math.upload_points(&[
        [origin, origin, origin],
        [origin + 1., origin + 2., origin + 3.],
    ])?;
    let bounds = math.bounds(&points)?;
    assert_eq!(b.read_f64(&bounds.min)?, [origin; 3]);
    assert_eq!(
        b.read_f64(&bounds.max)?,
        [origin + 1., origin + 2., origin + 3.]
    );
    let reference = math.upload_points(&[[origin; 3]; 2])?;
    assert_eq!(
        b.read_f64(&math.squared_distance_pairs(&points, &reference)?)?,
        [0., 14.]
    );
    let stats = math.covariance(&points)?;
    assert_eq!(stats.samples, 2);
    assert_eq!(
        b.read_f64(&stats.centroid)?,
        [origin + 0.5, origin + 1., origin + 1.5]
    );
    assert_eq!(
        b.read_f64(&stats.covariance)?,
        [0.25, 0.5, 0.75, 0.5, 1., 1.5, 0.75, 1.5, 2.25]
    );
    let matrix = b.upload_f64(shape(&[3, 3]), &[2., 0., 0., 0., 3., 0., 0., 0., 4.])?;
    let translation = b.upload_f64(shape(&[3]), &[-2. * origin, -3. * origin, -4. * origin])?;
    assert_eq!(
        math.read_points(&math.transform(&points, &matrix, &translation)?)?,
        [[0.; 3], [2., 6., 12.]]
    );
    let wide = math.upload_points(&[[1e200, -1e200, 1e-200]])?;
    assert_eq!(math.read_points(&wide)?, [[1e200, -1e200, 1e-200]]);
    let covariance = math.covariance(&wide)?;
    assert_eq!(b.read_f64(&covariance.covariance)?, [0.; 9]);
    let empty = math.upload_points(&[])?;
    assert!(math.bounds(&empty).is_err());
    assert!(math.covariance(&empty).is_err());
    assert!(math.upload_points(&[[f64::NAN, 0., 0.]]).is_err());
    Ok(())
}
