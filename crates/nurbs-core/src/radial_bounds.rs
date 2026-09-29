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
    }
}
