use crate::{
    Acceleration, Error, PointBounds, PointMoments, Result, V3, point_bounds, point_moments,
};

/// WGSL template for fused point-cloud bounds + moments reduction.
pub const POINT_CLOUD_STATS_WGSL_TEMPLATE: &str = include_str!("point_cloud_stats.wgsl");

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointCloudStats {
    pub samples: usize,
    pub bounds: PointBounds,
    pub moments: PointMoments,
}

impl PointCloudStats {
    pub(crate) fn from_parts(bounds: PointBounds, moments: PointMoments) -> Self {
        debug_assert_eq!(bounds.samples, moments.samples);
        Self {
            samples: bounds.samples,
            bounds,
            moments,
        }
    }
}

/// Exact CPU point-cloud summary: bounds plus centroid/covariance moments.
pub fn point_cloud_stats(points: &[V3]) -> Result<PointCloudStats> {
    Ok(PointCloudStats::from_parts(
        point_bounds(points)?,
        point_moments(points)?,
    ))
}

/// [`point_cloud_stats`] with optional fused GPU/CUDA reduction. This combines
/// bounds and moments in one device pass/upload, which is preferable to running
/// `point_bounds_accelerated` and `point_moments_accelerated` separately when
/// callers need both summaries. `Auto` currently keeps the exact CPU reference;
/// explicit `Gpu`/`Cuda` force f32 device execution and fall back to CPU.
pub fn point_cloud_stats_accelerated(
    points: &[V3],
    #[allow(unused_variables)] acceleration: Acceleration,
) -> Result<PointCloudStats> {
    if points.is_empty() {
        return Err(Error::new(
            "invalid_point_cloud_stats_input",
            "point_cloud_stats_accelerated expects at least one point",
        ));
    }
    if points.iter().flatten().any(|value| !value.is_finite()) {
        return Err(Error::new(
            "invalid_point_cloud_stats_input",
            "point_cloud_stats_accelerated expects finite point coordinates",
        ));
    }
    #[allow(unused_variables)]
    let acceleration = acceleration.resolve_for_point_cloud_stats(points.len());
    #[cfg(feature = "gpu")]
    if acceleration.is_gpu() {
        #[cfg(feature = "cuda")]
        if acceleration == Acceleration::Cuda
            && let Some(stats) = crate::cuda::point_cloud_stats_cuda(points)
        {
            return Ok(stats);
        }
        if let Some(stats) = crate::gpu::point_cloud_stats_gpu(points) {
            return Ok(stats);
        }
    }
    point_cloud_stats(points)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn points(n: usize) -> Vec<V3> {
        (0..n)
            .map(|i| {
                let f = i as f64;
                [
                    (f * 0.013).sin() * 200. - 7.,
                    (f * 0.017).cos() * 120. + 3.,
                    f * 0.00031 - 50.,
                ]
            })
            .collect()
    }

    #[test]
    fn point_cloud_stats_matches_separate_cpu_summaries() {
        let points = [[-1., 2., 3.], [4., -5., 6.], [0., 7., -8.]];
        let got = point_cloud_stats(&points).unwrap();
        assert_eq!(got.samples, 3);
        assert_eq!(got.bounds, point_bounds(&points).unwrap());
        assert_eq!(got.moments, point_moments(&points).unwrap());
    }

    #[test]
    fn point_cloud_stats_rejects_empty_and_nonfinite_points() {
        assert!(point_cloud_stats(&[]).is_err());
        assert!(point_cloud_stats(&[[0., f64::NAN, 0.]]).is_err());
        assert!(point_cloud_stats_accelerated(&[], Acceleration::Gpu).is_err());
    }

    #[test]
    fn point_cloud_stats_auto_matches_cpu_reference() {
        let points = points(513);
        assert_eq!(
            point_cloud_stats_accelerated(&points, Acceleration::Auto).unwrap(),
            point_cloud_stats(&points).unwrap()
        );
    }

    #[cfg(any(feature = "gpu", feature = "cuda"))]
    fn assert_close(got: PointCloudStats, want: PointCloudStats) {
        assert_eq!(got.samples, want.samples);
        for axis in 0..3 {
            assert!((got.bounds.min[axis] - want.bounds.min[axis]).abs() < 1e-4);
            assert!((got.bounds.max[axis] - want.bounds.max[axis]).abs() < 1e-4);
            assert!((got.moments.centroid[axis] - want.moments.centroid[axis]).abs() < 1e-1);
        }
        for i in 0..3 {
            for j in 0..3 {
                assert!(
                    (got.moments.covariance[i][j] - want.moments.covariance[i][j]).abs() < 1e-1
                );
            }
        }
    }

    #[cfg(feature = "gpu")]
    #[test]
    fn point_cloud_stats_gpu_matches_cpu_reference() {
        let points = points(4097);
        assert_close(
            point_cloud_stats_accelerated(&points, Acceleration::Gpu).unwrap(),
            point_cloud_stats(&points).unwrap(),
        );
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn point_cloud_stats_cuda_matches_cpu_reference() {
        let points = points(4097);
        assert_close(
            point_cloud_stats_accelerated(&points, Acceleration::Cuda).unwrap(),
            point_cloud_stats(&points).unwrap(),
        );
    }
}
