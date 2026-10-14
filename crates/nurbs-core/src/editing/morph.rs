//! Curve morphing between compatible NURBS shapes (checklist 213–214).
//!
//! Both curves are clamped, normalized to `[0, 1]`, elevated to the common
//! degree and knot-merged through [`crate::sections::compatible`], so the
//! intermediates share one control grid. Interpolation is linear in the
//! homogeneous `(w·P, w)` coordinates — weights are never lerped raw without
//! the `w·P` companion — which keeps every blend inside the positive-weight
//! cone whenever both endpoints are positive-weight.
//!
//! Optional feature correspondences `(u_from, u_to)` (for example curvature
//! extrema from [`crate::curve_analysis`]) are honored by warping the `from`
//! parameterization before blending: a piecewise-linear map `w` through
//! `(0,0)`, each normalized pair, and `(1,1)` defines the warped base curve
//! `F̂(v) = F(w⁻¹(v))`, which is refit onto the shared knot vector by
//! interpolation at the Greville sites (Schoenberg–Whitney). The warp is a
//! reparameterization of the base only; it is exact whenever the warped
//! image stays in the shared spline space (identity warp, or warp breakpoints
//! on shared knots with matching smoothness) and otherwise an approximation
//! whose endpoint evidence is reported in `max_deviation`.
//!
//! Endpoint exactness: the blend at `t = 0` is the (possibly warped) base and
//! at `t = 1` is the target; `max_deviation` is the outward-rounded maximum
//! point distance between the reconstructed endpoints and the original
//! curves, sampled on a 65-point grid.
use crate::{Result, check, curve::Curve, curve::basis, numeric, sections};
use math_core::next_up;

pub struct MorphReport {
    /// Intermediate shapes at `t_j = j / (steps + 1)`, `j = 1..=steps`.
    pub curves: Vec<Curve>,
    pub degree: usize,
    pub knot_count: usize,
    /// Endpoint-exactness evidence at `t = 0` and `t = 1`, outward-rounded.
    pub max_deviation: f64,
}

/// Dense Gaussian elimination with partial pivoting for a small nonsingular
/// `n × n` system with multiple right-hand sides (Greville refit system).
fn dense_solve(mut matrix: Vec<Vec<f64>>, mut values: Vec<Vec<f64>>) -> Result<Vec<Vec<f64>>> {
    let n = matrix.len();
    let scale = matrix
        .iter()
        .flatten()
        .map(|x| x.abs())
        .fold(0_f64, f64::max);
    let threshold = 64. * f64::EPSILON * scale.max(f64::MIN_POSITIVE);
    for column in 0..n {
        let pivot = (column..n)
            .max_by(|&a, &b| matrix[a][column].abs().total_cmp(&matrix[b][column].abs()))
            .unwrap();
        numeric(
            matrix[pivot][column].abs() > threshold,
            "Morph refit system is singular",
        )?;
        matrix.swap(column, pivot);
        values.swap(column, pivot);
        let divisor = matrix[column][column];
        for value in &mut matrix[column][column..] {
            *value /= divisor;
        }
        for value in &mut values[column] {
            *value /= divisor;
        }
        for row in 0..n {
            if row == column {
                continue;
            }
            let factor = matrix[row][column];
            for j in column..n {
                matrix[row][j] -= factor * matrix[column][j];
            }
            let pivot_values = values[column].clone();
            for (value, pivot_value) in values[row].iter_mut().zip(pivot_values) {
                *value -= factor * pivot_value;
            }
        }
    }
    Ok(values)
}

fn homogeneous(curve: &Curve) -> Vec<Vec<f64>> {
    curve
        .control_points
        .iter()
        .zip(&curve.weights)
        .map(|(point, weight)| {
            point
                .iter()
                .map(|x| x * weight)
                .chain(std::iter::once(*weight))
                .collect()
        })
        .collect()
}

fn from_homogeneous(degree: usize, knots: &[f64], controls: Vec<Vec<f64>>) -> Result<Curve> {
    let dimension = controls[0].len() - 1;
    let weights: Vec<f64> = controls.iter().map(|h| h[dimension]).collect();
    numeric(
        weights
            .iter()
            .all(|w| w.is_finite() && *w >= 1e-12 && *w <= 1e12),
        "Morph produced inadmissible weights",
    )?;
    let curve = Curve {
        degree,
        knots: knots.to_vec(),
        control_points: controls
            .iter()
            .map(|h| h[..dimension].iter().map(|x| x / h[dimension]).collect())
            .collect(),
        weights,
        periodic: false,
    };
    curve.validate()?;
    Ok(curve)
}

/// Piecewise-linear increasing warp through `(0, 0)`, the normalized
/// correspondences, and `(1, 1)`. Returned as breakpoint pairs `(x, y)`.
fn warp_breakpoints(align: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut points = vec![(0., 0.)];
    points.extend_from_slice(align);
    points.push((1., 1.));
    points
}

/// Invert the piecewise-linear warp: given `y` on the target side, return the
/// `x` on the source side with `w(x) = y`.
fn warp_inverse(breakpoints: &[(f64, f64)], y: f64) -> f64 {
    if y <= 0. {
        return 0.;
    }
    if y >= 1. {
        return 1.;
    }
    for pair in breakpoints.windows(2) {
        let ((x0, y0), (x1, y1)) = (pair[0], pair[1]);
        if y <= y1 {
            return x0 + (y - y0) * (x1 - x0) / (y1 - y0);
        }
    }
    1.
}

/// Homogeneous sample of `curve` at `u`: `(C(u)·W(u), W(u))` with
/// `W(u) = Σ N_{i,p}(u) w_i`, ready as a refit right-hand side.
fn homogeneous_sample(curve: &Curve, u: f64) -> Result<Vec<f64>> {
    let b = basis(
        curve.degree,
        &curve.knots,
        curve.control_points.len(),
        u,
        curve.periodic,
    )?;
    let weight: f64 = b
        .basis
        .iter()
        .zip(&curve.weights)
        .map(|(b, w)| b * w)
        .sum();
    numeric(
        weight > 0. && weight.is_finite(),
        "Rational denominator lost its positive finite value",
    )?;
    let point = curve.evaluate_validated(u)?.point;
    Ok(point
        .iter()
        .map(|x| x * weight)
        .chain(std::iter::once(weight))
        .collect())
}

/// Refit `source` onto `knots` by interpolation at the Greville sites,
/// sampling the warped image `source(w⁻¹(g_i))` when `breakpoints` warps.
fn refit_warped(
    source: &Curve,
    degree: usize,
    knots: &[f64],
    breakpoints: Option<&[(f64, f64)]>,
) -> Result<Vec<Vec<f64>>> {
    let count = knots.len() - degree - 1;
    check(
        count > degree && count <= 256,
        "Morph refit would produce an invalid control count",
    )?;
    let sites: Vec<f64> = (0..count)
        .map(|i| knots[i + 1..=i + degree].iter().sum::<f64>() / degree as f64)
        .collect();
    let mut matrix = Vec::with_capacity(count);
    let mut values = Vec::with_capacity(count);
    for &site in &sites {
        matrix.push(
            basis(degree, knots, count, site, false)
                .map(|b| b.basis)?,
        );
        let u = breakpoints.map_or(site, |bp| warp_inverse(bp, site));
        values.push(homogeneous_sample(source, u)?);
    }
    numeric(
        matrix.iter().flatten().all(|x| x.is_finite()),
        "Morph refit matrix overflowed finite precision",
    )?;
    dense_solve(matrix, values)
}

/// Maximum point distance between the original curve (on its own domain) and
/// the reconstructed normalized curve, on a shared 65-point parameter grid.
fn endpoint_deviation(original: &Curve, reconstructed: &Curve) -> Result<f64> {
    let [a, b] = original.domain();
    let mut deviation: f64 = 0.;
    for i in 0..=64 {
        let s = i as f64 / 64.;
        let p = original.evaluate(a + (b - a) * s)?.point;
        let q = reconstructed.evaluate(s)?.point;
        deviation = deviation.max(
            p.iter()
                .zip(&q)
                .map(|(x, y)| (x - y) * (x - y))
                .sum::<f64>()
                .sqrt(),
        );
    }
    numeric(deviation.is_finite(), "Morph deviation is not finite")?;
    Ok(deviation)
}

/// Morph `from` into `to` through `steps` intermediate curves.
pub fn morph_curves_report(
    from: &Curve,
    to: &Curve,
    steps: usize,
    align: &[(f64, f64)],
) -> Result<MorphReport> {
    from.validate()?;
    to.validate()?;
    check(
        (1..=64).contains(&steps),
        "Morph requires 1..=64 intermediate steps",
    )?;
    check(
        align.len() <= 32,
        "Morph accepts at most 32 feature correspondences",
    )?;
    check(
        from.control_points[0].len() == to.control_points[0].len(),
        "Morph requires matching dimensions",
    )?;
    // Normalize the correspondences into the shared [0, 1] parameter and
    // require strict increase on both sides so the warp is invertible.
    let [fa, fb] = from.domain();
    let [ta, tb] = to.domain();
    let mut normalized: Vec<(f64, f64)> = Vec::with_capacity(align.len());
    for &(uf, ut) in align {
        check(
            uf.is_finite() && ut.is_finite() && uf > fa && uf < fb && ut > ta && ut < tb,
            "Morph correspondences must lie strictly inside the active domains",
        )?;
        normalized.push(((uf - fa) / (fb - fa), (ut - ta) / (tb - ta)));
    }
    check(
        normalized
            .windows(2)
            .all(|w| w[0].0 < w[1].0 && w[0].1 < w[1].1),
        "Morph correspondences must strictly increase in both parameters",
    )?;
    let pair = sections::compatible(&[from.clone(), to.clone()])?;
    let base = &pair[0];
    let target = &pair[1];
    let degree = base.degree;
    let knots = base.knots.clone();
    check(
        knots == target.knots,
        "Morph compatibility failed to unify knot vectors",
    )?;
    let warped = if normalized.is_empty() {
        homogeneous(base)
    } else {
        let breakpoints = warp_breakpoints(&normalized);
        refit_warped(base, degree, &knots, Some(&breakpoints))?
    };
    let target_h = homogeneous(target);
    let mut curves = Vec::with_capacity(steps);
    for j in 1..=steps {
        let t = j as f64 / (steps + 1) as f64;
        let blend: Vec<Vec<f64>> = warped
            .iter()
            .zip(&target_h)
            .map(|(a, b)| {
                a.iter()
                    .zip(b)
                    .map(|(x, y)| (1. - t) * x + t * y)
                    .collect()
            })
            .collect();
        curves.push(from_homogeneous(degree, &knots, blend)?);
    }
    // Endpoint evidence: t = 0 is the (possibly warped) base, t = 1 the
    // target; measure both against the original curves.
    let t0 = from_homogeneous(degree, &knots, warped)?;
    let mut max_deviation = endpoint_deviation(from, &t0)?;
    max_deviation = max_deviation.max(endpoint_deviation(to, target)?);
    Ok(MorphReport {
        curves,
        degree,
        knot_count: knots.len(),
        max_deviation: next_up(max_deviation),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line() -> Curve {
        Curve::from_polyline(vec![vec![0., 0., 0.], vec![2., 0., 0.]]).unwrap()
    }

    fn semicircle() -> Curve {
        // Quadratic rational quarter arcs joined at the top: a semicircle of
        // radius 1 centered at the origin, parameter domain [0, 1].
        let w = std::f64::consts::FRAC_1_SQRT_2;
        Curve {
            degree: 2,
            knots: vec![0., 0., 0., 0.5, 0.5, 1., 1., 1.],
            control_points: vec![
                vec![-1., 0., 0.],
                vec![-1., 1., 0.],
                vec![0., 1., 0.],
                vec![1., 1., 0.],
                vec![1., 0., 0.],
            ],
            weights: vec![1., w, 1., w, 1.],
            periodic: false,
        }
    }

    #[test]
    fn line_to_semicircle_morph_endpoints_exact_intermediates_valid() {
        let from = line();
        let to = semicircle();
        let report = morph_curves_report(&from, &to, 4, &[]).unwrap();
        assert_eq!(report.curves.len(), 4);
        assert_eq!(report.degree, 2);
        assert!(report.max_deviation < 1e-12, "{}", report.max_deviation);
        for (j, curve) in report.curves.iter().enumerate() {
            curve.validate().unwrap();
            assert!(curve.weights.iter().all(|w| *w > 0.));
            assert_eq!(curve.knots.len(), report.knot_count);
            // Intermediates lie between the endpoints: endpoints blend from
            // (±? , 0) — at t=0 the base start is (0,0,0), at t=1 (-1,0,0).
            let [a, b] = curve.domain();
            let start = curve.evaluate(a).unwrap().point;
            let end = curve.evaluate(b).unwrap().point;
            let t = (j + 1) as f64 / 5.;
            assert!((start[0] - (1. - t) * 0. - t * (-1.)).abs() < 1e-12);
            assert!(start[1].abs() < 1e-12);
            assert!((end[0] - (1. - t) * 2. - t * 1.).abs() < 1e-12);
        }
    }

    #[test]
    fn morph_with_misaligned_correspondence_stays_valid() {
        let from = line();
        let to = semicircle();
        // Deliberately misaligned: the from-curve's quarter point should reach
        // the semicircle's three-quarter point. The warped base is a
        // reparameterized line whose kink does not sit on a shared knot, so
        // the Greville refit is an approximation: the report must surface a
        // positive (finite, bounded) endpoint deviation while every
        // intermediate stays a valid positive-weight curve and the blend
        // endpoints still coincide with the original endpoints exactly.
        let report = morph_curves_report(&from, &to, 3, &[(0.25, 0.75)]).unwrap();
        assert_eq!(report.curves.len(), 3);
        assert!(report.max_deviation > 0.);
        assert!(report.max_deviation.is_finite() && report.max_deviation < 2.);
        for curve in &report.curves {
            curve.validate().unwrap();
            assert!(curve.weights.iter().all(|w| *w > 0.));
            // Blend endpoints interpolate the original endpoints exactly.
            let [a, b] = curve.domain();
            let start = curve.evaluate(a).unwrap().point;
            let end = curve.evaluate(b).unwrap().point;
            assert!(start[1].abs() < 1e-12 && start[2].abs() < 1e-12);
            assert!(end[1].abs() < 1e-12 && end[2].abs() < 1e-12);
        }
        // The warped blend really honors the correspondence direction: the
        // point at normalized parameter 0.75 of a mid-morph intermediate
        // interpolates between from(0.25) and to(0.75) in homogeneous space,
        // i.e. it is the weight-normalized blend, not the arithmetic midpoint.
        let mid = &report.curves[1];
        let q = mid.evaluate(0.75).unwrap().point;
        let a = from.evaluate(0.25).unwrap().point; // polyline domain is [0,1]
        let b = to.evaluate(0.75).unwrap().point;
        let wa = 1.; // unit-weight line
        let wb = to
            .weights
            .iter()
            .zip(
                crate::curve::basis(2, &to.knots, to.control_points.len(), 0.75, false)
                    .unwrap()
                    .basis
                    .iter(),
            )
            .map(|(w, b)| w * b)
            .sum::<f64>();
        for axis in 0..3 {
            let expected = (0.5 * wa * a[axis] + 0.5 * wb * b[axis]) / (0.5 * wa + 0.5 * wb);
            assert!(
                (q[axis] - expected).abs() < 1e-9,
                "axis {axis}: {} vs {expected}",
                q[axis]
            );
        }
    }

    #[test]
    fn morph_rejects_bad_budgets_and_correspondences() {
        let from = line();
        let to = semicircle();
        assert!(morph_curves_report(&from, &to, 0, &[]).is_err());
        assert!(morph_curves_report(&from, &to, 65, &[]).is_err());
        // Out-of-domain and non-increasing correspondences are rejected.
        assert!(morph_curves_report(&from, &to, 1, &[(2.5, 0.5)]).is_err());
        assert!(morph_curves_report(&from, &to, 1, &[(0.5, 0.5), (0.25, 0.25)]).is_err());
    }

    #[test]
    fn morph_preserves_positive_weights_under_homogeneous_lerp() {
        // Strongly non-uniform weights still blend inside the positive cone.
        let mut from = semicircle();
        from.weights = vec![1., 0.1, 1., 0.1, 1.];
        from.validate().unwrap();
        let to = semicircle();
        let report = morph_curves_report(&from, &to, 8, &[]).unwrap();
        assert_eq!(report.curves.len(), 8);
        for curve in &report.curves {
            curve.validate().unwrap();
            assert!(curve.weights.iter().all(|w| *w > 0. && *w <= 1e12));
        }
    }
}
