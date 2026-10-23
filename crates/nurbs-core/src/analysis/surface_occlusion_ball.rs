//! Sufficient silhouette exclusion by a retained rational ball bound.
//! Candidates only choose a ball; interval Bernstein coefficients prove that
//! every surface point lies inside it and every curve point lies at its rim.
use super::{Curve, I, Result, Surface};
fn binomial(n: usize, k: usize) -> f64 {
    (0..k).fold(1usize, |a, i| a * (n - i) / (i + 1)) as f64
}
fn residual(
    points: &[Vec<Vec<f64>>],
    weights: &[Vec<f64>],
    center: [f64; 3],
    radius2: f64,
    axes: usize,
) -> Result<I> {
    let n = points.len() - 1;
    let m = points[0].len() - 1;
    let mut coefficients = vec![vec![I::point(0.); 2 * m + 1]; 2 * n + 1];
    for i in 0..=n {
        for j in 0..=m {
            for a in 0..=n {
                for b in 0..=m {
                    let factor =
                        I::point(binomial(n, i) * binomial(n, a) * binomial(m, j) * binomial(m, b))
                            .div(I::point(binomial(2 * n, i + a) * binomial(2 * m, j + b)))?;
                    let ww = I::point(weights[i][j]).mul(I::point(weights[a][b]))?;
                    let mut value = I::point(-radius2);
                    for k in 0..axes {
                        value = value.add(
                            I::point(points[i][j][k])
                                .sub(I::point(center[k]))?
                                .mul(I::point(points[a][b][k]).sub(I::point(center[k]))?)?,
                        )?;
                    }
                    coefficients[i + a][j + b] =
                        coefficients[i + a][j + b].add(value.mul(ww)?.mul(factor)?)?;
                }
            }
        }
    }
    let min_weight = weights
        .iter()
        .flatten()
        .copied()
        .fold(f64::INFINITY, f64::min);
    let error = coefficients
        .iter()
        .flatten()
        .map(|x| x.lo.abs().max(x.hi.abs()))
        .fold(0., f64::max);
    let upper = I::point(error)
        .div(I::point(min_weight).mul(I::point(min_weight))?)?
        .hi;
    Ok(I {
        lo: -upper,
        hi: upper,
    })
}
/// `true` proves absence of a depth gap larger than tolerance. `false` makes
/// no statement. Works on whole original Bézier charts, regardless of trims.
pub fn ball_silhouette_excludes(s: &Surface, c: &Curve, tolerance: f64) -> Result<bool> {
    s.validate()?;
    c.validate()?;
    crate::check(
        tolerance.is_finite()
            && tolerance > 0.
            && s.control_points.iter().flatten().all(|p| p.len() == 3)
            && c.control_points.iter().all(|p| p.len() == 3),
        "Invalid ball silhouette request",
    )?;
    let clamped = |knots: &[f64], degree: usize| {
        knots[..=degree].iter().all(|t| *t == knots[degree])
            && knots[degree + 1..].iter().all(|t| *t == knots[degree + 1])
    };
    if !clamped(&s.knots_u, s.degree_u)
        || !clamped(&s.knots_v, s.degree_v)
        || !clamped(&c.knots, c.degree)
        || c.periodic
        || c.degree != 2
        || c.control_points.len() != 3
        || s.degree_u > 3
        || s.degree_v > 3
        || s.control_points.len() != s.degree_u + 1
        || s.control_points[0].len() != s.degree_v + 1
        || s.periodic_u
        || s.periodic_v
    {
        return Ok(false);
    }
    // This projective weight ratio is invariant under a positive fractional
    // reparameterization. It only proposes a center: the original weighted
    // coefficients below independently enclose both complete images.
    let omega=(c.weights[1]/c.weights[0])*(c.weights[1]/c.weights[2]);
    if !(0. < omega && omega < 1.) {
        return Ok(false);
    }
    let p = &c.control_points;
    let center: [f64; 3] =
        std::array::from_fn(|k| ((p[0][k] + p[2][k]) * 0.5 - omega * p[1][k]) / (1. - omega));
    let radius2 = (0..3).map(|k| (p[0][k] - center[k]).powi(2)).sum::<f64>();
    if !radius2.is_finite() || !center.iter().all(|x| x.is_finite()) {
        return Ok(false);
    }
    let curve_points = p.iter().map(|p| vec![p.clone()]).collect::<Vec<_>>();
    let curve_weights = c.weights.iter().map(|w| vec![*w]).collect::<Vec<_>>();
    let a = residual(&s.control_points, &s.weights, center, radius2, 3)?;
    let b = residual(&curve_points, &curve_weights, center, radius2, 3)?;
    // The curve lies on the front branch: z >= center.z. From the two
    // ball residual bounds, (surface.z-center.z)^2 <= (curve.z-center.z)^2
    // + a+b on every common XY ray. Its positive depth gap is <= sqrt(a+b).
    let gap2 = I::point(a.hi).add(I::point(b.hi))?;
    let min_depth = p.iter().map(|p| p[2]).fold(f64::INFINITY, f64::min);
    let below = I::point(center[2]).sub(I::point(min_depth))?.hi.max(0.);
    // A rounded candidate center may put a true front-branch endpoint a few
    // ulps behind its plane. Account for that bound rather than widening a
    // geometry comparison or assuming an exact reconstructed center.
    let gap = I::point(gap2.hi.sqrt().next_up()).add(I::point(below).mul(I::point(2.))?)?;
    Ok(gap.hi < tolerance)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn patch() -> Surface {
        let t = [0., 0.5, 1.];
        let q = [0., 0., 1.];
        let weights: Vec<Vec<f64>> = (0..3)
            .map(|i| (0..3).map(|j| 1. + q[i] + q[j]).collect())
            .collect();
        Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: (0..3)
                .map(|i| {
                    (0..3)
                        .map(|j| {
                            vec![
                                4. * t[i] / weights[i][j],
                                4. * t[j] / weights[i][j],
                                2. * (1. - q[i] - q[j]) / weights[i][j],
                            ]
                        })
                        .collect()
                })
                .collect(),
            weights,
            periodic_u: false,
            periodic_v: false,
        }
    }
    fn rim() -> Curve {
        Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![2., 0., 0.], vec![2., 2., 0.], vec![0., 2., 0.]],
            weights: vec![1., 0.5_f64.sqrt(), 1.],
            periodic: false,
        }
    }
    #[test]
    fn projective_rim_clock_keeps_the_same_original_ball_exclusion() {
        let s=patch();let mut c=rim();c.weights=vec![1.,1.,2.];
        let before=c.clone();
        assert!(ball_silhouette_excludes(&s,&c,1e-5).unwrap());
        assert_eq!(c,before);
        c.control_points[1][0]+=0.01;
        assert!(!ball_silhouette_excludes(&s,&c,1e-5).unwrap());
    }
    #[test]
    fn whole_rational_ball_bound_excludes_rim_and_refuses_a_larger_surface() {
        let s = patch();
        let c = rim();
        assert!(ball_silhouette_excludes(&s, &c, 1e-5).unwrap());
        let mut larger = s.clone();
        for p in larger.control_points.iter_mut().flatten() {
            for x in p {
                *x *= 1.01;
            }
        }
        assert!(!ball_silhouette_excludes(&larger, &c, 1e-5).unwrap());
        let mut elevated = s.clone();
        for p in elevated.control_points.iter_mut().flatten() {
            p[2] += 0.01;
        }
        assert!(!ball_silhouette_excludes(&elevated, &c, 1e-5).unwrap());
        let mut inside = c.clone();
        for p in &mut inside.control_points {
            p[0] *= 0.99;
            p[1] *= 0.99;
        }
        assert!(!ball_silhouette_excludes(&s, &inside, 1e-5).unwrap());
    }
}
