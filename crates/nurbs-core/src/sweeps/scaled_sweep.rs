//! Algebraic rational translation sweep with a positive rational scale law.
use crate::{Result, check, curve::Curve, surface::Surface};

pub(crate) fn normalized(c: &Curve) -> Result<Curve> {
    c.validate()?;
    check(
        !c.periodic,
        "Rational sweep guides must use clamped nonperiodic encoding",
    )?;
    let [a, b] = c.domain();
    let mut c = c.trim(a, b)?;
    c.knots.iter_mut().for_each(|k| *k = (*k - a) / (b - a));
    let max = c.weights.iter().copied().fold(0., f64::max);
    for w in &mut c.weights {
        *w /= max;
    }
    c.validate()?;
    Ok(c)
}
pub(crate) fn binomial(n: usize, k: usize) -> f64 {
    let k = k.min(n - k);
    (1..=k).fold(1., |a, i| a * (n + 1 - i) as f64 / i as f64)
}

/// S(u,v)=origin+C(v)-C(0)+r(v)*(P(u)-origin). Orientation is fixed.
/// The scale curve uses [r,0,0] controls with strictly positive r and weights.
/// Path and scale domains normalize independently to [0,1]. No sampled fit.
pub fn sweep(profile: &Curve, path: &Curve, scale: &Curve, origin: [f64; 3]) -> Result<Surface> {
    profile.validate()?;
    crate::foundation::guards::require_finite_point(&origin, "origin")?;
    check(
        profile.control_points[0].len() == 3
            && profile.control_points.len() <= 32,
        "Scaled sweep needs a 3D profile within the 32-control budget",
    )?;
    let path = normalized(path)?;
    let scale = normalized(scale)?;
    check(
        path.control_points[0].len() == 3
            && scale
                .control_points
                .iter()
                .all(|p| p.len() == 3 && p[0] > 0. && p[1] == 0. && p[2] == 0.),
        "Scaled sweep needs a 3D path and positive [scale,0,0] controls",
    )?;
    let degree = path.degree + scale.degree;
    check(degree <= 25, "Scaled sweep product degree exceeds 25")?;
    let mut cuts: Vec<f64> = path.knots.iter().chain(&scale.knots).copied().collect();
    cuts.sort_by(f64::total_cmp);
    cuts.dedup();
    cuts.retain(|k| *k >= 0. && *k <= 1.);
    check(
        cuts.len() >= 2 && (cuts.len() - 1) * degree + 1 <= 32,
        "Scaled sweep product exceeds 32 path controls",
    )?;
    let start = path.control_points[0].clone();
    let profile_max = profile.weights.iter().copied().fold(0., f64::max);
    let mut points = vec![Vec::<Vec<f64>>::new(); profile.control_points.len()];
    let mut weights = vec![Vec::<f64>::new(); profile.control_points.len()];
    let mut knots = vec![0.; degree + 1];
    // Unified guard as a backstop over the span-decomposition budget.
    let mut guard =
        crate::foundation::guards::Budget::with_iterations(cuts.len())?.guard("scaled_sweep");
    for (span, cut) in cuts.windows(2).enumerate() {
        guard.tick()?;
        let a = path.trim(cut[0], cut[1])?;
        let b = scale.trim(cut[0], cut[1])?;
        check(
            a.control_points.len() == a.degree + 1 && b.control_points.len() == b.degree + 1,
            "Scaled sweep span decomposition is not Bezier",
        )?;
        for i in 0..profile.control_points.len() {
            let wp = profile.weights[i] / profile_max;
            check(
                wp.is_finite() && wp > 0.,
                "Scaled sweep profile weight collapsed",
            )?;
            for k in 0..=degree {
                let mut denominator = 0.;
                let mut numerator = [0.; 3];
                for j in 0..=a.degree {
                    if k < j || k - j > b.degree {
                        continue;
                    }
                    let l = k - j;
                    let coefficient =
                        binomial(a.degree, j) * binomial(b.degree, l) / binomial(degree, k);
                    let weight = coefficient * a.weights[j] * b.weights[l];
                    check(
                        weight.is_finite() && weight > 0.,
                        "Scaled sweep product weight is unrepresentable",
                    )?;
                    denominator += weight;
                    for d in 0..3 {
                        let relative = profile.control_points[i][d] - origin[d];
                        let scaled = b.control_points[l][0] * relative;
                        let base = origin[d] + (a.control_points[j][d] - start[d]);
                        let q = base + scaled;
                        let term = weight * q;
                        check(
                            scaled.is_finite()
                                && (relative == 0. || scaled != 0.)
                                && q.is_finite()
                                && term.is_finite()
                                && (q == 0. || term != 0.),
                            "Scaled sweep displacement or homogeneous control collapsed",
                        )?;
                        numerator[d] += term;
                    }
                }
                let point: Vec<f64> = numerator.iter().map(|x| x / denominator).collect();
                let weight = wp * denominator;
                check(
                    point.iter().all(|x| x.is_finite()) && weight.is_finite() && weight > 0.,
                    "Scaled sweep control is unrepresentable",
                )?;
                if span > 0 && k == 0 {
                    check(
                        points[i].last() == Some(&point) && weights[i].last() == Some(&weight),
                        "Scaled sweep binary64 span endpoints disagree",
                    )?;
                } else {
                    points[i].push(point);
                    weights[i].push(weight);
                }
            }
        }
        if span + 1 < cuts.len() - 1 {
            knots.extend(vec![cut[1]; degree]);
        }
    }
    knots.extend(vec![1.; degree + 1]);
    let s = Surface {
        degree_u: profile.degree,
        degree_v: degree,
        knots_u: profile.knots.clone(),
        knots_v: knots,
        control_points: points,
        weights,
        periodic_u: profile.periodic,
        periodic_v: false,
    };
    s.validate()?;
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_explicit_scale_center_and_distant_path_anchor() {
        let p = line([11., 20., 30.], [12., 20., 30.]);
        let c = line([1e9, 0., 10.], [1e9, 0., 12.]);
        let r = line([1., 0., 0.], [3., 0., 0.]);
        let s = sweep(&p, &c, &r, [10., 20., 30.]).unwrap();
        for u in [0., 0.13, 0.5, 0.87, 1.] {
            for v in [0., 0.17, 0.5, 0.83, 1.] {
                let q = s.evaluate(u, v).unwrap().point;
                for k in 0..3 {
                    assert!(
                        (q[k] - [10. + (1. + u) * (1. + 2. * v), 20., 30. + 2. * v][k]).abs()
                            < 1e-10
                    );
                }
            }
        }
    }
    #[cfg(feature = "transport")]
    #[test]
    fn json_constructor_preserves_scale_path_and_profile_roles() {
        let value = crate::transport::dispatch(value_codec::json!({"op":"surface_scaled_sweep",
            "profile":line([1.,0.,0.],[2.,0.,0.]),"path":line([0.,0.,10.],[0.,0.,12.]),
            "scale":line([1.,0.,0.],[3.,0.,0.]),"origin":[0.,0.,0.]}))
        .unwrap();
        let s: Surface = value_codec::from_value(value).unwrap();
        let p = s.evaluate(0.5, 0.25).unwrap().point;
        assert!((p[0] - 2.25).abs() < 1e-10);
        assert!((p[2] - 0.5).abs() < 1e-10);
    }
    fn line(a: [f64; 3], b: [f64; 3]) -> Curve {
        crate::primitives::line(a, b).unwrap()
    }
    #[test]
    fn matches_independent_polynomial_product_and_jets() {
        let p = line([1., 0., 0.], [2., 0., 0.]);
        let c = crate::paths::bezier(
            vec![vec![0., 0., 10.], vec![0., 0., 11.], vec![0., 0., 14.]],
            None,
        )
        .unwrap();
        let s = sweep(&p, &c, &line([1., 0., 0.], [3., 0., 0.]), [0.; 3]).unwrap();
        for u in [0., 0.13, 0.5, 0.87, 1.] {
            for v in [0., 0.17, 0.5, 0.83, 1.] {
                let q = s.evaluate(u, v).unwrap();
                let (du, dv) = q.first_derivatives().unwrap();
                let (_, uv, vv) = q.second_derivatives().unwrap();
                for (a, b) in [
                    (q.point, [(1. + u) * (1. + 2. * v), 0., 2. * v + 2. * v * v]),
                    (du, [1. + 2. * v, 0., 0.]),
                    (dv, [2. * (1. + u), 0., 2. + 4. * v]),
                    (uv, [2., 0., 0.]),
                    (vv, [0., 0., 4.]),
                ] {
                    for k in 0..3 {
                        assert!((a[k] - b[k]).abs() < 1e-10);
                    }
                }
            }
        }
    }
    #[test]
    fn retains_independent_rational_scale_path_and_circular_profile() {
        let p = crate::paths::bezier(
            vec![vec![2., 0., 0.], vec![2., 2., 0.], vec![0., 2., 0.]],
            Some(vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.]),
        )
        .unwrap();
        let mut c = line([0.; 3], [0., 0., 2.]);
        c.weights = vec![1., 2.];
        let mut r = line([1., 0., 0.], [3., 0., 0.]);
        r.weights = vec![2., 1.];
        let s = sweep(&p, &c, &r, [0.; 3]).unwrap();
        for u in [0., 0.13, 0.5, 0.87, 1.] {
            for v in [0., 0.17, 0.5, 0.83, 1.] {
                let q = s.evaluate(u, v).unwrap().point;
                let radius = 2. * (2. + v) / (2. - v);
                assert!((q[0] * q[0] + q[1] * q[1] - radius * radius).abs() < 1e-9);
                assert!((q[2] - 4. * v / (1. + v)).abs() < 1e-10);
            }
        }
    }
    #[test]
    fn supports_multispan_path_and_refuses_nonpositive_scale_or_budget() {
        let p = line([1., 0., 0.], [2., 0., 0.]);
        let c = crate::primitives::polyline(&[[0., 0., 0.], [0., 0., 1.], [0., 0., 3.]], false)
            .unwrap();
        let r = line([1., 0., 0.], [2., 0., 0.]);
        let s = sweep(&p, &c, &r, [0.; 3]).unwrap();
        assert!((s.evaluate(0.5, 0.5).unwrap().point[0] - 2.25).abs() < 1e-10);
        assert!((s.evaluate(0.5, 0.5).unwrap().point[2] - 1.).abs() < 1e-10);
        assert!(sweep(&p, &c, &line([0.; 3], [1., 0., 0.]), [0.; 3]).is_err());
        let long = crate::primitives::polyline(
            &(0..17).map(|i| [0., 0., i as f64]).collect::<Vec<_>>(),
            false,
        )
        .unwrap();
        assert!(sweep(&p, &long, &r, [0.; 3]).is_err());
    }
    #[test]
    fn permits_ordinary_addition_rounding_of_tiny_profile_components() {
        let p=line([1e-16,1.,0.],[1e-16,2.,0.]);
        let c=line([0.,0.,0.],[8.,0.,30.]);
        let r=line([1.,0.,0.],[2.,0.,0.]);
        let s=sweep(&p,&c,&r,[0.;3]).unwrap();
        let q=s.evaluate(0.5,1.).unwrap().point;
        assert_eq!(q[0],8.);
        assert!((q[1]-3.).abs()<1e-12);
        assert!((q[2]-30.).abs()<1e-12);
    }
}
