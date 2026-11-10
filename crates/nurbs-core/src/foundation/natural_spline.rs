//! Cubic interpolation with natural (zero second derivative) endpoint conditions.
use crate::{Result, check, curve::Curve};

/// Interpolates 2..86 sites at strictly increasing authored parameters.
/// Output parameter is normalized to [0,1]. The real-arithmetic spline is C2;
/// binary64 construction and C0 knot multiplicity do not certify continuity.
pub fn interpolate(points: &[[f64; 3]], parameters: &[f64]) -> Result<Curve> {
    construct(points, parameters, None)
}

/// Cubic C2 interpolation with supplied endpoint dP/dt tangents.
/// Authored t is normalized to u; tangents scale by the full parameter span.
pub fn clamped(
    points: &[[f64; 3]],
    parameters: &[f64],
    start_tangent: [f64; 3],
    end_tangent: [f64; 3],
) -> Result<Curve> {
    check(
        start_tangent
            .iter()
            .chain(&end_tangent)
            .all(|x| x.is_finite()),
        "Clamped spline endpoint tangents must be finite",
    )?;
    construct(points, parameters, Some([start_tangent, end_tangent]))
}

fn construct(
    points: &[[f64; 3]],
    parameters: &[f64],
    endpoint_tangents: Option<[[f64; 3]; 2]>,
) -> Result<Curve> {
    let n = points.len();
    check(
        (2..=86).contains(&n) && parameters.len() == n,
        "Natural spline needs matching 2..86 sites and parameters",
    )?;
    check(
        points.iter().flatten().all(|x| x.is_finite())
            && parameters.iter().all(|x| x.is_finite())
            && parameters.windows(2).all(|p| p[0] < p[1]),
        "Natural spline needs finite sites and increasing parameters",
    )?;
    let span = parameters[n - 1] - parameters[0];
    check(
        span.is_finite() && span > 0.,
        "Natural spline parameter span overflow",
    )?;
    let mut u: Vec<f64> = parameters
        .iter()
        .map(|t| (t - parameters[0]) / span)
        .collect();
    u[0] = 0.;
    u[n - 1] = 1.;
    check(
        u.windows(2).all(|p| p[0] < p[1]),
        "Natural spline normalized intervals collapse",
    )?;
    let h: Vec<f64> = u.windows(2).map(|p| p[1] - p[0]).collect();
    if let Some(ts) = endpoint_tangents {
        check(
            ts.iter().flatten().all(|&x| x == 0. || x * span != 0.),
            "Clamped spline normalized endpoint tangent underflow",
        )?;
    }
    let endpoint_tangents = endpoint_tangents.map(|ts| ts.map(|t| t.map(|x| x * span)));
    check(
        endpoint_tangents
            .as_ref()
            .is_none_or(|ts| ts.iter().flatten().all(|x| x.is_finite())),
        "Clamped spline normalized endpoint tangent overflow",
    )?;
    let slopes: Vec<[f64; 3]> = (0..n - 1)
        .map(|i| std::array::from_fn(|k| (points[i + 1][k] - points[i][k]) / h[i]))
        .collect();
    check(
        slopes.iter().flatten().all(|x| x.is_finite()),
        "Natural spline secant overflow",
    )?;
    // Thomas elimination for the positive definite interior moment system.
    let mut upper = vec![0.; n];
    let mut rhs = vec![[0.; 3]; n];
    if let Some(ts) = endpoint_tangents {
        upper[0] = 0.5;
        rhs[0] = std::array::from_fn(|k| 3. * (slopes[0][k] - ts[0][k]) / h[0]);
        check(
            rhs[0].iter().all(|x| x.is_finite()),
            "Clamped spline endpoint moments overflow",
        )?;
    }
    for i in 1..n - 1 {
        let pivot = 2. * (h[i - 1] + h[i]) - h[i - 1] * upper[i - 1];
        check(
            pivot.is_finite() && pivot > 0.,
            "Natural spline moment system is unrepresentable",
        )?;
        upper[i] = h[i] / pivot;
        rhs[i] = std::array::from_fn(|k| {
            (6. * (slopes[i][k] - slopes[i - 1][k]) - h[i - 1] * rhs[i - 1][k]) / pivot
        });
        check(
            rhs[i].iter().all(|x| x.is_finite()),
            "Natural spline moments overflow",
        )?;
    }
    let mut moments = vec![[0.; 3]; n];
    if let Some(ts) = endpoint_tangents {
        let i = n - 1;
        let pivot = 2. * h[i - 1] - h[i - 1] * upper[i - 1];
        check(
            pivot.is_finite() && pivot > 0.,
            "Clamped spline endpoint system is unrepresentable",
        )?;
        moments[i] = std::array::from_fn(|k| {
            (6. * (ts[1][k] - slopes[i - 1][k]) - h[i - 1] * rhs[i - 1][k]) / pivot
        });
        check(
            moments[i].iter().all(|x| x.is_finite()),
            "Clamped spline endpoint moments overflow",
        )?;
    }
    for i in (0..n - 1).rev() {
        moments[i] = std::array::from_fn(|k| rhs[i][k] - upper[i] * moments[i + 1][k]);
    }
    let mut tangents = vec![[0.; 3]; n];
    for i in 0..n - 1 {
        tangents[i] = std::array::from_fn(|k| {
            slopes[i][k] - h[i] * (2. * moments[i][k] + moments[i + 1][k]) / 6.
        });
    }
    tangents[n - 1] = std::array::from_fn(|k| {
        slopes[n - 2][k] + h[n - 2] * (moments[n - 2][k] + 2. * moments[n - 1][k]) / 6.
    });
    if let Some(ts) = endpoint_tangents {
        tangents[0] = ts[0];
        tangents[n - 1] = ts[1];
        // Retain strict precision guards only for authored endpoint conditions.
        // Use each actual adjacent interval, not the full normalized span.
        crate::hermite::interpolate(&points[..2], &[ts[0], [0.; 3]], &u[..2])?;
        crate::hermite::interpolate(&points[n - 2..], &[[0.; 3], ts[1]], &u[n - 2..])?;
    }
    crate::hermite::interpolate_inferred(points, &tangents, &u)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clamped_reproduces_independent_cubic_and_authored_endpoint_derivatives() {
        let t = [-2., -0.7, 0.3, 2.];
        let points = t.map(|t: f64| [t, t * t, t * t * t]);
        let c = clamped(&points, &t, [1., -4., 12.], [1., 4., 12.]).unwrap();
        for i in 0..3 {
            let a = (t[i] + 2.) / 4.;
            let b = (t[i + 1] + 2.) / 4.;
            let segment = c.trim(a, b).unwrap();
            for s in [0., 0.13, 0.5, 0.87, 1.] {
                let u = a + s * (b - a);
                let x = -2. + 4. * u;
                let q = segment.evaluate(u).unwrap();
                for k in 0..3 {
                    assert!((q.point[k] - [x, x * x, x * x * x][k]).abs() < 1e-10);
                    assert!(
                        (q.d1.as_ref().unwrap()[k] - [4., 8. * x, 12. * x * x][k]).abs() < 1e-9
                    );
                    assert!((q.d2.as_ref().unwrap()[k] - [0., 32., 96. * x][k]).abs() < 1e-8);
                }
            }
        }
    }
    #[test]
    fn two_site_clamped_is_hermite_and_invalid_endpoint_conditions_are_refused() {
        let p = [[0., 1., 2.], [3., -1., 0.]];
        let t = [-2., 3.];
        let start = [2., 0., 1.];
        let end = [-1., 3., 0.];
        assert_eq!(
            clamped(&p, &t, start, end).unwrap().control_points,
            crate::hermite::interpolate(&p, &[start, end], &t)
                .unwrap()
                .control_points
        );
        assert!(clamped(&p, &t, [f64::NAN; 3], end).is_err());
        assert!(clamped(&p, &[0., 1e9], [1e300; 3], end).is_err());
        assert!(clamped(&[[1e9; 3]; 2], &[0., 1.], [1e-9; 3], [1e-9; 3]).is_err());
    }
    #[test]
    fn independent_three_site_natural_solution_and_one_sided_jets() {
        let p = [[0., 0., 0.], [1., 2., -1.], [3., 0., 2.]];
        let c = interpolate(&p, &[-2., 0., 4.]).unwrap();
        // u=[0,1/3,1], natural endpoint moments are zero, interior is [0,-27,22.5].
        let u = [0., 1. / 3., 1.];
        let m = [[0.; 3], [0., -27., 22.5], [0.; 3]];
        for i in 0..2 {
            let segment = c.trim(u[i], u[i + 1]).unwrap();
            let h = u[i + 1] - u[i];
            for s in [0., 0.17, 0.5, 0.83, 1.] {
                let a = 1. - s;
                let b = s;
                let q = segment.evaluate(u[i] + s * h).unwrap();
                for k in 0..3 {
                    let expected = a * p[i][k]
                        + b * p[i + 1][k]
                        + h * h * ((a * a * a - a) * m[i][k] + (b * b * b - b) * m[i + 1][k]) / 6.;
                    assert!((q.point[k] - expected).abs() < 1e-11);
                }
            }
        }
        let left = c.trim(0., u[1]).unwrap();
        let right = c.trim(u[1], 1.).unwrap();
        let a = left.evaluate(u[1]).unwrap();
        let b = right.evaluate(u[1]).unwrap();
        for k in 0..3 {
            assert!((a.d1.as_ref().unwrap()[k] - b.d1.as_ref().unwrap()[k]).abs() < 1e-10);
            assert!((a.d2.as_ref().unwrap()[k] - b.d2.as_ref().unwrap()[k]).abs() < 1e-10);
            assert!(left.evaluate(0.).unwrap().d2.as_ref().unwrap()[k].abs() < 1e-10);
            assert!(right.evaluate(1.).unwrap().d2.as_ref().unwrap()[k].abs() < 1e-10);
        }
    }
    #[test]
    fn line_stationary_sites_and_parameter_affine_invariance() {
        let p = [[0.; 3], [2., 4., -2.]];
        let c = interpolate(&p, &[10., 20.]).unwrap();
        for u in [0., 0.13, 0.5, 1.] {
            let q = c.evaluate(u).unwrap();
            for k in 0..3 {
                assert!((q.point[k] - u * p[1][k]).abs() < 1e-12);
            }
        }
        let p = [[0.; 3], [1., 2., 0.], [3., 0., 1.], [4., 1., 3.]];
        assert_eq!(
            interpolate(&p, &[0., 1., 3., 4.]).unwrap().control_points,
            interpolate(&p, &[-2., 0., 4., 6.]).unwrap().control_points
        );
        assert!(interpolate(&[[0.; 3]; 4], &[0., 1., 2., 3.]).is_ok());
    }
    #[test]
    fn invalid_inputs_and_collapsed_intervals_are_refused() {
        let p = [[0.; 3], [1.; 3]];
        for t in [[0., 0.], [1., 0.], [0., f64::INFINITY], [-1e308, 1e308]] {
            assert!(interpolate(&p, &t).is_err());
        }
        assert!(interpolate(&p, &[0.]).is_err());
        assert!(interpolate(&[[0.; 3]; 4], &[-1e9, 0., f64::from_bits(1), 1e9]).is_err());
        assert!(interpolate(&[[0.; 3], [1.; 3]], &[0., f64::from_bits(1)]).is_ok());
        assert!(interpolate(&[[f64::NAN; 3], [0.; 3]], &[0., 1.]).is_err());
    }
    #[test]
    fn many_nonuniform_sites_match_positions_and_both_join_derivatives() {
        let points = [
            [0., 1., 2.],
            [2., -1., 3.],
            [-1., 4., 0.],
            [5., 2., -2.],
            [7., 0., 1.],
        ];
        let t = [0., 0.3, 1.2, 2., 5.];
        let c = interpolate(&points, &t).unwrap();
        let u = t.map(|v| v / 5.);
        for i in 1..4 {
            let left = c.trim(u[i - 1], u[i]).unwrap().evaluate(u[i]).unwrap();
            let right = c.trim(u[i], u[i + 1]).unwrap().evaluate(u[i]).unwrap();
            for k in 0..3 {
                assert!((left.point[k] - points[i][k]).abs() < 1e-12);
                assert!((right.point[k] - points[i][k]).abs() < 1e-12);
                assert!(
                    (left.d1.as_ref().unwrap()[k] - right.d1.as_ref().unwrap()[k]).abs() < 1e-10
                );
                assert!(
                    (left.d2.as_ref().unwrap()[k] - right.d2.as_ref().unwrap()[k]).abs() < 1e-9
                );
            }
        }
        assert_eq!(
            interpolate(
                &[[0.; 3]; 86],
                &(0..86).map(|i| i as f64).collect::<Vec<_>>()
            )
            .unwrap()
            .control_points
            .len(),
            256
        );
        assert!(
            interpolate(
                &[[0.; 3]; 87],
                &(0..87).map(|i| i as f64).collect::<Vec<_>>()
            )
            .is_err()
        );
    }
    #[cfg(feature = "transport")]
    #[test]
    fn json_dispatch_matches_native_interpolation() {
        let points = [[0., 0., 0.], [1., 2., 0.], [3., 0., 1.]];
        let parameters = [0., 1., 3.];
        let direct = interpolate(&points, &parameters).unwrap();
        let value=crate::dispatch(value_codec::json!({"op":"curve_natural_spline","points":points,"parameters":parameters})).unwrap();
        assert_eq!(value_codec::to_value(direct).unwrap(), value);
    }
}
