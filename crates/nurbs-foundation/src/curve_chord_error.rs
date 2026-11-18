//! Rational Bernstein enclosure of C(t) minus a prescribed affine segment.
use crate::{Result, check, curve::Curve, interval_eval::Interval as I};
/// Pointwise Euclidean error upper bound, including supplied endpoint rounding.
/// `None` preserves unproven work; it does not imply zero deviation.
pub fn curve_affine_error_upper(
    curve: &Curve,
    domain: [f64; 2],
    ends: [[f64; 3]; 2],
    max_spans: usize,
) -> Result<(Option<f64>, usize)> {
    curve.validate()?;
    let natural = curve.domain();
    check(
        curve.control_points[0].len() == 3
            && max_spans <= 100000
            && domain.iter().all(|v| v.is_finite())
            && domain[0] < domain[1]
            && domain[0] >= natural[0]
            && domain[1] <= natural[1]
            && ends.iter().flatten().all(|v| v.is_finite()),
        "Use a finite 3D curve segment and bounded work",
    )?;
    let width = I::point(domain[1]).sub(I::point(domain[0]))?;
    let mut used = 0;
    let mut upper = 0_f64;
    for span in curve.degree..curve.control_points.len() {
        let lo = curve.knots[span].max(domain[0]);
        let hi = curve.knots[span + 1].min(domain[1]);
        if lo >= hi {
            continue;
        }
        if used == max_spans {
            return Ok((None, used));
        }
        used += 1;
        let controls = crate::curve_distance::restricted_controls(curve, span, I::new(lo, hi)?)?;
        let n = curve.degree + 1;
        let origin = &curve.control_points[span - curve.degree];
        let mut line = [[I::point(0.); 3]; 2];
        for (end, t) in [lo, hi].into_iter().enumerate() {
            let alpha = I::point(t).sub(I::point(domain[0]))?.div(width)?;
            for axis in 0..3 {
                line[end][axis] = I::point(ends[0][axis])
                    .sub(I::point(origin[axis]))?
                    .add(alpha.mul(I::point(ends[1][axis]).sub(I::point(ends[0][axis]))?)?)?;
            }
        }
        // Elevate H and W to degree p+1. W*L has the same degree. Positive
        // elevated weights make the residual a rational convex combination.
        for k in 0..=n {
            let a = I::point(k as f64).div(I::point(n as f64))?;
            let b = I::point(1.).sub(a)?;
            let left = if k > 0 { Some(&controls[k - 1]) } else { None };
            let right = if k < n { Some(&controls[k]) } else { None };
            let weight = left
                .map_or(Ok(I::point(0.)), |h| a.mul(h[3]))?
                .add(right.map_or(Ok(I::point(0.)), |h| b.mul(h[3]))?)?;
            let mut norm = I::point(0.);
            for axis in 0..3 {
                let residual = left
                    .map_or(Ok(I::point(0.)), |h| {
                        a.mul(h[axis].sub(h[3].mul(line[1][axis])?)?)
                    })?
                    .add(right.map_or(Ok(I::point(0.)), |h| {
                        b.mul(h[axis].sub(h[3].mul(line[0][axis])?)?)
                    })?)?
                    .div(weight)?;
                norm = norm.add(I::point(residual.lo.abs().max(residual.hi.abs())))?;
            }
            upper = upper.max(norm.hi);
        }
    }
    Ok((Some(upper), used))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn long_line_has_small_error_and_rounding_is_included() {
        let c = Curve::from_polyline(vec![vec![100., 0., 0.], vec![1100., 500., 200.]]).unwrap();
        let (bound, used) =
            curve_affine_error_upper(&c, [0., 1.], [[100., 0., 0.], [1100., 500., 200.]], 1)
                .unwrap();
        assert_eq!(used, 1);
        assert!(bound.unwrap() < 1e-8);
        let (bound, _) =
            curve_affine_error_upper(&c, [0., 1.], [[100., 0., 0.01], [1100., 500., 200.01]], 1)
                .unwrap();
        assert!(bound.unwrap() >= 0.01);
        assert_eq!(
            curve_affine_error_upper(&c, [0., 1.], [[100., 0., 0.], [1100., 500., 200.]], 0)
                .unwrap(),
            (None, 0)
        );
    }
    #[test]
    fn rational_bend_and_subintervals_have_conservative_error() {
        let c = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0., 0.], vec![0.5, 1., 0.], vec![1., 0., 0.]],
            weights: vec![1., 2., 1.],
            periodic: false,
        };
        let (bound, _) =
            curve_affine_error_upper(&c, [0., 1.], [[0., 0., 0.], [1., 0., 0.]], 1).unwrap();
        assert!(bound.unwrap() >= 2. / 3.);
        let p = |t| {
            let v = crate::interval_eval::evaluate_interval(&c, I::point(t)).unwrap();
            std::array::from_fn(|k| (v[k].lo + v[k].hi) / 2.)
        };
        let ends = [p(0.25), p(0.75)];
        let (bound, _) = curve_affine_error_upper(&c, [0.25, 0.75], ends, 1).unwrap();
        for i in 0..=100 {
            let t = 0.25 + 0.5 * (i as f64 / 100.);
            let point = p(t);
            let error = (0..3)
                .map(|k| {
                    (point[k] - (ends[0][k] + (ends[1][k] - ends[0][k]) * (t - 0.25) / 0.5)).powi(2)
                })
                .sum::<f64>()
                .sqrt();
            assert!(error <= bound.unwrap());
        }
    }
    #[test]
    fn every_knot_span_is_required_and_invalid_requests_are_refused() {
        let c = Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 1., 0.], vec![2., 0., 0.]])
            .unwrap();
        let domain = c.domain();
        let ends = [[0., 0., 0.], [2., 0., 0.]];
        let (bound, used) = curve_affine_error_upper(&c, domain, ends, 2).unwrap();
        assert_eq!(used, 2);
        assert!(bound.unwrap() >= 1.);
        assert_eq!(
            curve_affine_error_upper(&c, domain, ends, 1).unwrap(),
            (None, 1)
        );
        assert!(curve_affine_error_upper(&c, [domain[0], domain[0]], ends, 2).is_err());
        assert!(curve_affine_error_upper(&c, [domain[0] - 1., domain[1]], ends, 2).is_err());
        assert!(curve_affine_error_upper(&c, domain, [[f64::NAN, 0., 0.], ends[1]], 2).is_err());
    }
}
