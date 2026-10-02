//! Conservative radius ranges of positive rational Bezier surface images.
//! Product Bernstein coefficients preserve cancellations such as a constant
//! squared radius, without assuming that the input is an analytic sphere.
use crate::{Result, check, distance_bounds::Interval, surface::Surface};

/// None means the surface is not a supported single Bezier patch. This optional
/// bound supplements ordinary Cartesian enclosures, never replaces coverage.
pub fn radius_bounds(s: &Surface, origin: [f64; 3]) -> Result<Option<[f64; 2]>> {
    s.validate()?;
    check(
        origin.iter().all(|x| x.is_finite()),
        "Radius origin must be finite",
    )?;
    let (p, q) = (s.degree_u, s.degree_v);
    let bezier = |knots: &[f64], degree: usize, n: usize| {
        n == degree + 1
            && knots[..=degree].iter().all(|&x| x == knots[degree])
            && knots[degree + 1..].iter().all(|&x| x == knots[degree + 1])
    };
    if p > 8
        || q > 8
        || !bezier(&s.knots_u, p, s.control_points.len())
        || !bezier(&s.knots_v, q, s.control_points[0].len())
    {
        return Ok(None);
    }
    let binomial = |n: usize, k: usize| {
        let mut nck = 1u64;
        for i in 0..k.min(n - k) {
            nck = nck * (n - i) as u64 / (i + 1) as u64
        }
        nck as f64
    };
    let coefficient = |n: usize, a: usize, b: usize| {
        Interval::point(binomial(n, a))
            .mul(Interval::point(binomial(n, b)))?
            .div(Interval::point(binomial(2 * n, a + b)))
    };
    let scale = s.weights.iter().flatten().copied().fold(0_f64, f64::max);
    let mut controls = vec![vec![[Interval::point(0.); 4]; q + 1]; p + 1];
    for i in 0..=p {
        for j in 0..=q {
            let w = Interval::point(s.weights[i][j]).div(Interval::point(scale))?;
            for k in 0..3 {
                controls[i][j][k] = Interval::point(s.control_points[i][j][k])
                    .sub(Interval::point(origin[k]))?
                    .mul(w)?
            }
            controls[i][j][3] = w;
        }
    }
    let mut lower = f64::INFINITY;
    let mut upper = 0_f64;
    for u in 0..=2 * p {
        for v in 0..=2 * q {
            let mut numerator = Interval::point(0.);
            let mut denominator = Interval::point(0.);
            for i in u.saturating_sub(p)..=u.min(p) {
                for j in v.saturating_sub(q)..=v.min(q) {
                    let a = controls[i][j];
                    let b = controls[u - i][v - j];
                    let factor = coefficient(p, i, u - i)?.mul(coefficient(q, j, v - j)?)?;
                    let mut squared = Interval::point(0.);
                    for k in 0..3 {
                        squared = squared.add(a[k].mul(b[k])?)?
                    }
                    numerator = numerator.add(squared.mul(factor)?)?;
                    denominator = denominator.add(a[3].mul(b[3])?.mul(factor)?)?;
                }
            }
            // An underflowed positive weight may prevent this optional certificate.
            if denominator.lo <= 0. {
                return Ok(None);
            }
            let ratio = numerator.div(denominator)?;
            lower = lower.min(ratio.lo);
            upper = upper.max(ratio.hi);
        }
    }
    Ok(Some([
        lower.max(0.).sqrt().next_down().max(0.),
        upper.sqrt().next_up(),
    ]))
}
/// Distance to a Cartesian coordinate axis through `origin`. Orthogonal
/// projection is exact here: replacing one authored coordinate by the axis
/// origin performs no rounded rotation or basis conversion.
pub fn axis_radius_bounds(s: &Surface, axis: usize, origin: [f64; 3]) -> Result<Option<[f64; 2]>> {
    s.validate()?;
    check(axis < 3, "Radius axis must be 0, 1 or 2")?;
    let mut projected = s.clone();
    for row in &mut projected.control_points {
        for point in row {
            point[axis] = origin[axis];
        }
    }
    radius_bounds(&projected, origin)
}
/// Each axis-radius map is 1-Lipschitz, so separation of its image intervals
/// is a lower bound on every Euclidean point-pair distance. Unsupported charts
/// and numeric range failures keep the ordinary Cartesian bound.
pub(crate) fn axis_separation_lower(a: &Surface, b: &Surface) -> f64 {
    let mut lower = 0_f64;
    for axis in 0..3 {
        if let (Some(ra), Some(rb)) = (
            axis_radius_bounds(a, axis, [0.; 3]).ok().flatten(),
            axis_radius_bounds(b, axis, [0.; 3]).ok().flatten(),
        ) {
            let gap = (ra[0] - rb[1]).max(rb[0] - ra[1]).next_down().max(0.);
            lower = lower.max(gap);
        }
    }
    lower
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn constant_radius_is_certified_from_polynomial_coefficients() {
        let weights = (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| 1. + [0., 0., 1.][i] + [0., 0., 1.][j])
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let s = Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: (0..3)
                .map(|i| {
                    (0..3)
                        .map(|j| {
                            vec![
                                6. * [0., 0.5, 1.][i] / weights[i][j],
                                6. * [0., 0.5, 1.][j] / weights[i][j],
                                3. * (1. - [0., 0., 1.][i] - [0., 0., 1.][j]) / weights[i][j],
                            ]
                        })
                        .collect()
                })
                .collect(),
            weights,
            periodic_u: false,
            periodic_v: false,
        };
        let r = radius_bounds(&s, [0.; 3]).unwrap().unwrap();
        assert!(r[0] <= 3. && r[1] >= 3.);
        assert!(r[1] - r[0] < 1e-10, "{r:?}");
        let shifted = radius_bounds(&s, [1., 2., 3.]).unwrap().unwrap();
        assert!(shifted[0] < shifted[1]);
        let before = format!("{s:?}");
        for axis in 0..3 {
            let origin = [1., 2., 3.];
            let range = axis_radius_bounds(&s, axis, origin).unwrap().unwrap();
            for u in [0., 0.25, 0.5, 0.75, 1.] {
                for v in [0., 0.25, 0.5, 0.75, 1.] {
                    let point = s.evaluate(u, v).unwrap().point;
                    let radius = (0..3)
                        .filter(|k| *k != axis)
                        .map(|k| (point[k] - origin[k]).powi(2))
                        .sum::<f64>()
                        .sqrt();
                    assert!(range[0] <= radius + 1e-12 && range[1] >= radius - 1e-12);
                }
            }
        }
        assert_eq!(format!("{s:?}"), before);
        assert!(axis_radius_bounds(&s, 3, [0.; 3]).is_err());
        assert!(axis_radius_bounds(&s, 0, [f64::NAN, 0., 0.]).is_err());
        let mut invalid = s.clone();
        invalid.control_points.clear();
        assert!(axis_radius_bounds(&invalid, 2, [0.; 3]).is_err());
    }
}
