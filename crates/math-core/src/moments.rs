use crate::{Acceleration, Error, M3, Result, V3, dot, eigen, unit};

/// WGSL template for point-cloud moment reduction.
pub const POINT_MOMENTS_WGSL_TEMPLATE: &str = include_str!("point_moments.wgsl");

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointMoments {
    pub samples: usize,
    pub centroid: V3,
    /// Mean outer product `E[p * p^T]`.
    pub second_moment: M3,
    /// Central covariance `E[(p-c) * (p-c)^T]`.
    pub covariance: M3,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointPrincipalAxes {
    pub moments: PointMoments,
    /// Covariance eigenvalues in descending order.
    pub variances: V3,
    /// Unit eigenvectors in the same order as `variances`.
    pub axes: [V3; 3],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointPlane {
    pub samples: usize,
    /// Unit normal of the least-squares plane. The sign is normalized
    /// deterministically from its largest component.
    pub normal: V3,
    /// Plane equation offset: `dot(normal, p) + offset = 0`.
    pub offset: f64,
    /// Root-mean-square orthogonal distance to the plane.
    pub rms_distance: f64,
    pub axes: PointPrincipalAxes,
}

impl PointMoments {
    pub(crate) fn from_sums(samples: usize, sum: V3, outer: [f64; 6]) -> Self {
        let n = samples as f64;
        let centroid = [sum[0] / n, sum[1] / n, sum[2] / n];
        let second_moment = [
            [outer[0] / n, outer[1] / n, outer[2] / n],
            [outer[1] / n, outer[3] / n, outer[4] / n],
            [outer[2] / n, outer[4] / n, outer[5] / n],
        ];
        let covariance = std::array::from_fn(|i| {
            std::array::from_fn(|j| second_moment[i][j] - centroid[i] * centroid[j])
        });
        Self {
            samples,
            centroid,
            second_moment,
            covariance,
        }
    }
}

/// Exact CPU centroid, second moment and covariance for a finite point cloud.
pub fn point_moments(points: &[V3]) -> Result<PointMoments> {
    if points.is_empty() {
        return Err(Error::new(
            "invalid_point_moments_input",
            "point_moments expects at least one point",
        ));
    }
    let mut sum = [0.; 3];
    let mut outer = [0.; 6];
    for &point in points {
        if point.iter().any(|value| !value.is_finite()) {
            return Err(Error::new(
                "invalid_point_moments_input",
                "point_moments expects finite point coordinates",
            ));
        }
        let [x, y, z] = point;
        sum[0] += x;
        sum[1] += y;
        sum[2] += z;
        outer[0] += x * x;
        outer[1] += x * y;
        outer[2] += x * z;
        outer[3] += y * y;
        outer[4] += y * z;
        outer[5] += z * z;
    }
    Ok(PointMoments::from_sums(points.len(), sum, outer))
}

/// [`point_moments`] with optional fused GPU/CUDA reductions. `Auto` currently
/// preserves the exact CPU reference; explicit `Gpu`/`Cuda` force f32 device
/// execution and fall back to CPU if no device is available.
pub fn point_moments_accelerated(
    points: &[V3],
    #[allow(unused_variables)] acceleration: Acceleration,
) -> Result<PointMoments> {
    if points.is_empty() {
        return Err(Error::new(
            "invalid_point_moments_input",
            "point_moments_accelerated expects at least one point",
        ));
    }
    if points.iter().flatten().any(|value| !value.is_finite()) {
        return Err(Error::new(
            "invalid_point_moments_input",
            "point_moments_accelerated expects finite point coordinates",
        ));
    }
    #[cfg(feature = "gpu")]
    if acceleration.is_gpu() && acceleration != Acceleration::Auto {
        #[cfg(feature = "cuda")]
        if acceleration == Acceleration::Cuda
            && let Some(moments) = crate::cuda::point_moments_cuda(points)
        {
            return Ok(moments);
        }
        if let Some(moments) = crate::gpu::point_moments_gpu(points) {
            return Ok(moments);
        }
    }
    point_moments(points)
}

/// Principal point-cloud axes from the covariance eigensystem. Moments can be
/// CPU, GPU or CUDA; the final 3x3 eigensolve runs on the CPU.
pub fn point_principal_axes(
    points: &[V3],
    acceleration: Acceleration,
) -> Result<PointPrincipalAxes> {
    let moments = point_moments_accelerated(points, acceleration)?;
    let (values, vectors) = eigen(moments.covariance);
    let mut order = [0usize, 1, 2];
    order.sort_by(|&a, &b| values[b].total_cmp(&values[a]));
    let variances = order.map(|axis| values[axis].max(0.));
    let axes = order.map(|axis| [vectors[0][axis], vectors[1][axis], vectors[2][axis]]);
    Ok(PointPrincipalAxes {
        moments,
        variances,
        axes,
    })
}

fn orient_normal(mut normal: V3) -> V3 {
    normal = unit(normal);
    let axis = (0..3)
        .max_by(|&a, &b| normal[a].abs().total_cmp(&normal[b].abs()))
        .unwrap();
    if normal[axis] < 0. {
        normal = [-normal[0], -normal[1], -normal[2]];
    }
    normal
}

/// Least-squares plane fit from point-cloud PCA. Moment reduction can run on
/// CPU, GPU or CUDA; the final 3x3 eigensolve runs on CPU.
pub fn point_fit_plane(points: &[V3], acceleration: Acceleration) -> Result<PointPlane> {
    if points.len() < 3 {
        return Err(Error::new(
            "invalid_point_plane_input",
            "point_fit_plane expects at least 3 points",
        ));
    }
    let axes = point_principal_axes(points, acceleration)?;
    let normal = orient_normal(axes.axes[2]);
    let offset = -dot(normal, axes.moments.centroid);
    Ok(PointPlane {
        samples: points.len(),
        normal,
        offset,
        rms_distance: axes.variances[2].sqrt(),
        axes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn points(n: usize) -> Vec<V3> {
        (0..n)
            .map(|i| {
                let f = i as f64;
                [
                    (f * 0.013).sin() * 20. - 3.,
                    (f * 0.017).cos() * 12. + 2.,
                    f * 0.001 - 5.,
                ]
            })
            .collect()
    }

    #[test]
    fn point_moments_reports_centroid_second_moment_and_covariance() {
        let got = point_moments(&[[0., 0., 0.], [2., 0., 0.], [0., 2., 0.]]).unwrap();
        assert_eq!(got.samples, 3);
        assert_eq!(got.centroid, [2. / 3., 2. / 3., 0.]);
        assert_eq!(got.second_moment[0][0], 4. / 3.);
        assert_eq!(got.second_moment[1][1], 4. / 3.);
        assert_eq!(got.second_moment[0][1], 0.);
        assert!((got.covariance[0][0] - 8. / 9.).abs() < 1e-15);
        assert!((got.covariance[0][1] + 4. / 9.).abs() < 1e-15);
    }

    #[test]
    fn point_moments_rejects_empty_and_nonfinite_points() {
        assert!(point_moments(&[]).is_err());
        assert!(point_moments(&[[0., f64::INFINITY, 0.]]).is_err());
        assert!(point_moments_accelerated(&[], Acceleration::Gpu).is_err());
    }

    #[test]
    fn point_moments_auto_matches_cpu_reference() {
        let points = points(513);
        assert_eq!(
            point_moments_accelerated(&points, Acceleration::Auto).unwrap(),
            point_moments(&points).unwrap()
        );
    }

    #[test]
    fn point_principal_axes_orders_variance_descending() {
        let points = [
            [-3., 0., 0.],
            [-1., 0., 0.],
            [1., 0., 0.],
            [3., 0., 0.],
            [0., 0.2, 0.],
            [0., -0.2, 0.],
        ];
        let got = point_principal_axes(&points, Acceleration::Cpu).unwrap();
        assert!(got.variances[0] > got.variances[1]);
        assert!(got.variances[1] > got.variances[2]);
        assert!(got.axes[0][0].abs() > 0.99);
    }

    #[test]
    fn point_fit_plane_recovers_known_plane() {
        let points: Vec<_> = (0..32)
            .flat_map(|ix| {
                (0..16).map(move |iy| {
                    let x = ix as f64 * 0.1 - 1.5;
                    let y = iy as f64 * 0.1 - 0.7;
                    let z = 2. + 0.25 * x - 0.5 * y;
                    [x, y, z]
                })
            })
            .collect();
        let got = point_fit_plane(&points, Acceleration::Cpu).unwrap();
        let want = orient_normal([-0.25, 0.5, 1.]);
        for axis in 0..3 {
            assert!((got.normal[axis] - want[axis]).abs() < 1e-10);
        }
        assert!(got.rms_distance < 1e-10);
        for point in &points {
            assert!((dot(got.normal, *point) + got.offset).abs() < 1e-10);
        }
    }

    #[test]
    fn point_fit_plane_rejects_too_few_points() {
        assert!(point_fit_plane(&[[0.; 3], [1.; 3]], Acceleration::Cpu).is_err());
    }

    #[cfg(feature = "gpu")]
    #[test]
    fn point_moments_gpu_matches_cpu_reference() {
        let points = points(4097);
        let got = point_moments_accelerated(&points, Acceleration::Gpu).unwrap();
        let want = point_moments(&points).unwrap();
        for axis in 0..3 {
            assert!((got.centroid[axis] - want.centroid[axis]).abs() < 2e-3);
        }
        for i in 0..3 {
            for j in 0..3 {
                assert!((got.covariance[i][j] - want.covariance[i][j]).abs() < 2e-2);
            }
        }
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn point_moments_cuda_matches_cpu_reference() {
        let points = points(4097);
        let got = point_moments_accelerated(&points, Acceleration::Cuda).unwrap();
        let want = point_moments(&points).unwrap();
        for axis in 0..3 {
            assert!((got.centroid[axis] - want.centroid[axis]).abs() < 2e-3);
        }
        for i in 0..3 {
            for j in 0..3 {
                assert!((got.covariance[i][j] - want.covariance[i][j]).abs() < 2e-2);
            }
        }
    }

    #[cfg(feature = "gpu")]
    #[test]
    fn point_fit_plane_gpu_matches_cpu_reference() {
        let points = points(4097);
        let got = point_fit_plane(&points, Acceleration::Gpu).unwrap();
        let want = point_fit_plane(&points, Acceleration::Cpu).unwrap();
        for axis in 0..3 {
            assert!((got.normal[axis] - want.normal[axis]).abs() < 1e-3);
        }
        assert!((got.rms_distance - want.rms_distance).abs() < 1e-3);
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn point_fit_plane_cuda_matches_cpu_reference() {
        let points = points(4097);
        let got = point_fit_plane(&points, Acceleration::Cuda).unwrap();
        let want = point_fit_plane(&points, Acceleration::Cpu).unwrap();
        for axis in 0..3 {
            assert!((got.normal[axis] - want.normal[axis]).abs() < 1e-3);
        }
        assert!((got.rms_distance - want.rms_distance).abs() < 1e-3);
    }
}
