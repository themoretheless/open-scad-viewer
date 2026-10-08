//! Piecewise cubic Hermite interpolation of authored positions and tangents.
use crate::{Result, check, curve::Curve};

/// Tangents are dP/dt at increasing authored parameters t. Output domain is [0,1],
/// so its derivative with respect to u is (t_last-t_first)*dP/dt. Shared positions
/// are retained exactly; control construction uses binary64 arithmetic. Interior
/// knots retain C0 multiplicity: callers must use one-sided jets at joins, since
/// the evaluator does not infer higher smoothness from matching control values.
/// Stationary tangents and repeated positions are allowed; regularity is not certified.
pub fn interpolate(
    points: &[[f64; 3]],
    tangents: &[[f64; 3]],
    parameters: &[f64],
) -> Result<Curve> {
    construct(points, tangents, parameters, true)
}

// Solved tangents can contain cancellation roundoff near a mathematical zero.
// Materializing their controls may round that noise away. This is not permitted
// for authored constraints, and it does not certify the inferred continuity.
pub(crate) fn interpolate_inferred(
    points: &[[f64; 3]],
    tangents: &[[f64; 3]],
    parameters: &[f64],
) -> Result<Curve> {
    construct(points, tangents, parameters, false)
}

fn construct(
    points: &[[f64; 3]],
    tangents: &[[f64; 3]],
    parameters: &[f64],
    authored: bool,
) -> Result<Curve> {
    let n = points.len();
    check(
        (2..=86).contains(&n) && tangents.len() == n && parameters.len() == n,
        "Hermite interpolation requires matching 2..86 positions, tangents and parameters",
    )?;
    check(
        points
            .iter()
            .chain(tangents)
            .flatten()
            .all(|v| v.is_finite()),
        "Hermite positions and tangents must be finite",
    )?;
    check(
        parameters.iter().all(|v| v.is_finite()) && parameters.windows(2).all(|p| p[0] < p[1]),
        "Hermite parameters must be finite and strictly increasing",
    )?;
    let a = parameters[0];
    let span = parameters[n - 1] - a;
    check(
        span.is_finite() && span > 0.,
        "Hermite parameter span overflow",
    )?;
    let mut normalized: Vec<f64> = parameters.iter().map(|t| (t - a) / span).collect();
    normalized[0] = 0.;
    normalized[n - 1] = 1.;
    check(
        normalized.windows(2).all(|p| p[0] < p[1]),
        "Hermite normalization collapses distinct parameters; split the range",
    )?;
    let mut controls = vec![points[0].to_vec()];
    let mut knots = vec![0.; 4];
    for i in 0..n - 1 {
        let h = parameters[i + 1] - parameters[i];
        let left: Vec<f64> = (0..3)
            .map(|k| points[i][k] + (h * tangents[i][k]) / 3.)
            .collect();
        let right: Vec<f64> = (0..3)
            .map(|k| points[i + 1][k] - (h * tangents[i + 1][k]) / 3.)
            .collect();
        for k in 0..3 {
            check(
                !authored || tangents[i][k] == 0. || left[k] != points[i][k],
                "Hermite tangent collapses at the supplied coordinate precision",
            )?;
            check(
                !authored || tangents[i + 1][k] == 0. || right[k] != points[i + 1][k],
                "Hermite tangent collapses at the supplied coordinate precision",
            )?;
        }
        controls.push(left);
        controls.push(right);
        controls.push(points[i + 1].to_vec());
        knots.extend(std::iter::repeat_n(normalized[i + 1], 3));
    }
    knots.push(1.);
    let curve = Curve {
        degree: 3,
        knots,
        weights: vec![1.; controls.len()],
        control_points: controls,
        periodic: false,
    };
    curve.validate()?;
    Ok(curve)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn interpolates_sites_tangents_and_independent_cubic_polynomials() {
        let points = [[1., 2., 3.], [4., -2., 7.], [8., 5., 0.]];
        let tangents = [[2., 1., 0.], [-1., 3., 2.], [0., -2., 1.]];
        let parameters = [-2., 1., 5.];
        let curve = interpolate(&points, &tangents, &parameters).unwrap();
        for i in 0..2 {
            let h = parameters[i + 1] - parameters[i];
            let left = (parameters[i] + 2.) / 7.;
            let right = (parameters[i + 1] + 2.) / 7.;
            let segment = curve.trim(left, right).unwrap();
            for s in [0., 0.13, 0.5, 0.87, 1.] {
                let u = left + (right - left) * s;
                let actual = segment.evaluate(u).unwrap();
                let h00 = 2. * s * s * s - 3. * s * s + 1.;
                let h10 = s * s * s - 2. * s * s + s;
                let h01 = -2. * s * s * s + 3. * s * s;
                let h11 = s * s * s - s * s;
                for k in 0..3 {
                    let p = h00 * points[i][k]
                        + h10 * h * tangents[i][k]
                        + h01 * points[i + 1][k]
                        + h11 * h * tangents[i + 1][k];
                    let dt = ((6. * s * s - 6. * s) * points[i][k]
                        + (3. * s * s - 4. * s + 1.) * h * tangents[i][k]
                        + (-6. * s * s + 6. * s) * points[i + 1][k]
                        + (3. * s * s - 2. * s) * h * tangents[i + 1][k])
                        / h;
                    assert!((actual.point[k] - p).abs() < 1e-11);
                    assert!((actual.d1.as_ref().unwrap()[k] - 7. * dt).abs() < 1e-10);
                }
            }
        }
        for (i, p) in points.iter().enumerate() {
            assert_eq!(&curve.control_points[3 * i], &p.to_vec());
        }
        assert_eq!(
            curve.evaluate(3. / 7.).unwrap().derivative_status,
            "insufficient_continuity"
        );
    }
    #[test]
    fn preserves_compensating_parameter_and_tangent_scales_and_refuses_rounding_collapse() {
        let p = [[0., 0., 0.], [1., 0., 0.]];
        let t = [[1e300, 0., 0.]; 2];
        let c = interpolate(&p, &t, &[0., 1e-300]).unwrap();
        for u in [0., 0.17, 0.5, 0.83, 1.] {
            let q = c.evaluate(u).unwrap();
            assert!((q.point[0] - u).abs() < 1e-12);
            assert!((q.d1.unwrap()[0] - 1.).abs() < 1e-12);
        }
        assert!(interpolate(&[[1e9, 0., 0.]; 2], &[[1e-9, 0., 0.]; 2], &[0., 1.]).is_err());
        assert!(interpolate(&p, &[[1., 0., 0.]; 2], &[0., f64::from_bits(1)]).is_err());
    }
    #[cfg(feature = "transport")]
    #[test]
    fn json_dispatch_preserves_the_same_authored_constraints() {
        let points = [[0., 0., 0.], [3., 2., 1.]];
        let tangents = [[1., 0., 0.], [0., 1., 0.]];
        let parameters = [-1., 2.];
        let direct = interpolate(&points, &tangents, &parameters).unwrap();
        let value=crate::dispatch(value_codec::json!({"op":"curve_hermite","points":points,"tangents":tangents,"parameters":parameters})).unwrap();
        assert_eq!(value_codec::to_value(direct).unwrap(), value);
    }
    #[test]
    fn refuses_bad_parameters_counts_and_unrepresentable_controls() {
        let points = [[0.; 3], [1.; 3]];
        let tangents = [[1.; 3]; 2];
        for params in [[0., 0.], [1., 0.], [0., f64::INFINITY], [-1e308, 1e308]] {
            assert!(interpolate(&points, &tangents, &params).is_err());
        }
        assert!(interpolate(&points, &[[1.; 3]], &[0., 1.]).is_err());
        assert!(interpolate(&points, &[[1e300; 3]; 2], &[0., 1.]).is_err());
        let many = vec![[0.; 3]; 87];
        assert!(interpolate(&many, &many, &(0..87).map(|i| i as f64).collect::<Vec<_>>()).is_err());
        let p = vec![[0.; 3]; 86];
        let c = interpolate(&p, &p, &(0..86).map(|i| i as f64).collect::<Vec<_>>()).unwrap();
        assert_eq!(c.control_points.len(), 256);
        assert!(
            interpolate(
                &[[0.; 3]; 4],
                &[[0.; 3]; 4],
                &[-1e9, 0., f64::from_bits(1), 1e9]
            )
            .is_err()
        );
    }
}
