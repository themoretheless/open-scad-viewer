use crate::{Acceleration, Error, Result, V3};

/// WGSL template for point-cloud axis-aligned bounds reduction.
pub const POINT_BOUNDS_WGSL_TEMPLATE: &str = include_str!("point_bounds.wgsl");

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointBounds {
    pub samples: usize,
    pub min: V3,
    pub max: V3,
    pub center: V3,
    pub extent: V3,
}

impl PointBounds {
    pub(crate) fn new(samples: usize, min: V3, max: V3) -> Self {
        Self {
            samples,
            min,
            max,
            center: [
                0.5 * (min[0] + max[0]),
                0.5 * (min[1] + max[1]),
                0.5 * (min[2] + max[2]),
            ],
            extent: [max[0] - min[0], max[1] - min[1], max[2] - min[2]],
        }
    }
}

/// Exact CPU axis-aligned bounds for a point cloud.
pub fn point_bounds(points: &[V3]) -> Result<PointBounds> {
    if points.is_empty() {
        return Err(Error::new(
            "invalid_point_bounds_input",
            "point_bounds expects at least one point",
        ));
    }
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for &point in points {
        for axis in 0..3 {
            let value = point[axis];
            if !value.is_finite() {
                return Err(Error::new(
                    "invalid_point_bounds_input",
                    "point_bounds expects finite point coordinates",
                ));
            }
            min[axis] = min[axis].min(value);
            max[axis] = max[axis].max(value);
        }
    }
    Ok(PointBounds::new(points.len(), min, max))
}

/// [`point_bounds`] with optional GPU/CUDA reduction kernels. `Auto` currently
/// keeps the CPU reference; explicit `Gpu`/`Cuda` force device execution and
/// fall back to CPU if no device is available.
pub fn point_bounds_accelerated(
    points: &[V3],
    #[allow(unused_variables)] acceleration: Acceleration,
) -> Result<PointBounds> {
    if points.is_empty() {
        return Err(Error::new(
            "invalid_point_bounds_input",
            "point_bounds_accelerated expects at least one point",
        ));
    }
    if points.iter().flatten().any(|value| !value.is_finite()) {
        return Err(Error::new(
            "invalid_point_bounds_input",
            "point_bounds_accelerated expects finite point coordinates",
        ));
    }
    #[cfg(feature = "gpu")]
    if acceleration.is_gpu() && acceleration != Acceleration::Auto {
        #[cfg(feature = "cuda")]
        if acceleration == Acceleration::Cuda
            && let Some(bounds) = crate::cuda::point_bounds_cuda(points)
        {
            return Ok(bounds);
        }
        if let Some(bounds) = crate::gpu::point_bounds_gpu(points) {
            return Ok(bounds);
        }
    }
    point_bounds(points)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn points(n: usize) -> Vec<V3> {
        (0..n)
            .map(|i| {
                let f = i as f64;
                [
                    (f * 0.17).sin() * 20. - 3.,
                    (f * 0.07).cos() * 11. + 2.,
                    f * 0.013 - 5.,
                ]
            })
            .collect()
    }

    #[test]
    fn point_bounds_reports_min_max_center_and_extent() {
        let got = point_bounds(&[[-1., 2., 3.], [4., -5., 6.], [0., 7., -8.]]).unwrap();
        assert_eq!(got.samples, 3);
        assert_eq!(got.min, [-1., -5., -8.]);
        assert_eq!(got.max, [4., 7., 6.]);
        assert_eq!(got.center, [1.5, 1., -1.]);
        assert_eq!(got.extent, [5., 12., 14.]);
    }

    #[test]
    fn point_bounds_rejects_empty_and_nonfinite_points() {
        assert!(point_bounds(&[]).is_err());
        assert!(point_bounds(&[[f64::NAN, 0., 0.]]).is_err());
        assert!(point_bounds_accelerated(&[], Acceleration::Gpu).is_err());
    }

    #[test]
    fn point_bounds_auto_matches_cpu_reference() {
        let points = points(257);
        assert_eq!(
            point_bounds_accelerated(&points, Acceleration::Auto).unwrap(),
            point_bounds(&points).unwrap()
        );
    }

    #[cfg(feature = "gpu")]
    #[test]
    fn point_bounds_gpu_matches_cpu_reference() {
        let points = points(4097);
        let got = point_bounds_accelerated(&points, Acceleration::Gpu).unwrap();
        let want = point_bounds(&points).unwrap();
        for axis in 0..3 {
            assert!((got.min[axis] - want.min[axis]).abs() < 1e-4);
            assert!((got.max[axis] - want.max[axis]).abs() < 1e-4);
        }
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn point_bounds_cuda_matches_cpu_reference() {
        let points = points(4097);
        let got = point_bounds_accelerated(&points, Acceleration::Cuda).unwrap();
        let want = point_bounds(&points).unwrap();
        for axis in 0..3 {
            assert!((got.min[axis] - want.min[axis]).abs() < 1e-4);
            assert!((got.max[axis] - want.max[axis]).abs() < 1e-4);
        }
    }
}
