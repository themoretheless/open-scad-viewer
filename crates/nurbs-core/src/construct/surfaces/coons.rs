//! Tensor-product Coons patch in homogeneous coordinates for compatible rational boundaries.
//! Boundary order: bottom/top in +U; left/right in +V. No endpoint snapping.
use crate::{
    Result, check,
    curve::Curve,
    surface::{Surface, loft_aligned},
};

fn is_clamped(curve: &Curve) -> bool {
    let [a, b] = curve.domain();
    curve.knots[..=curve.degree].iter().all(|k| *k == a)
        && curve.knots[curve.control_points.len()..]
            .iter()
            .all(|k| *k == b)
}

/// Multiply each homogeneous boundary by a positive affine scalar. This
/// preserves the rational parameterization while making both endpoint weights 1.
/// Explicit preparation; checked_endpoint_weights certifies the resulting curve.
pub fn normalize_endpoint_weights(curve: &Curve) -> Result<Curve> {
    curve.validate()?;
    check(
        !curve.periodic && curve.degree < 25,
        "Corner weight normalization requires an open curve of degree below 25",
    )?;
    let [a, b] = curve.domain();
    let p = curve.degree;
    let degree = p + 1;
    check(
        (b - a).is_finite(),
        "Corner weight normalization domain overflow",
    )?;
    let mut breaks: Vec<f64> = curve
        .knots
        .iter()
        .copied()
        .filter(|k| *k >= a && *k <= b)
        .collect();
    breaks.dedup();
    if (breaks.len() - 1) * degree + 1 > 256 {
        return Err(crate::resource("The result exceeds 256 control points"));
    }
    let clamped = if is_clamped(curve) {
        curve.clone()
    } else {
        curve.trim(a, b)?
    };
    let f0 = 1. / clamped.weights[0];
    let f1 = 1. / clamped.weights.last().unwrap();
    check(
        f0.is_finite() && f1.is_finite(),
        "Corner weight normalization scale overflow",
    )?;
    let mut controls: Vec<Vec<f64>> = Vec::new();
    let mut weights = Vec::new();
    let mut knots = Vec::new();
    for segment in clamped.decompose()? {
        let [left, right] = segment.domain();
        let c = segment.definition();
        let scalar = |t: f64| f0 * (1. - (t - a) / (b - a)) + f1 * ((t - a) / (b - a));
        let factors = [scalar(left), scalar(right)];
        let first = controls.is_empty();
        for k in (if first { 0 } else { 1 })..=degree {
            let mut h = vec![0_f64; c.control_points[0].len() + 1];
            for (index, factor) in [
                (k, factors[0] * (degree - k) as f64 / degree as f64),
                (k.wrapping_sub(1), factors[1] * k as f64 / degree as f64),
            ] {
                if index > p {
                    continue;
                }
                let w = c.weights[index] * factor;
                for axis in 0..h.len() - 1 {
                    h[axis] += c.control_points[index][axis] * w;
                }
                let last = h.len() - 1;
                h[last] += w;
            }
            let w = *h.last().unwrap();
            check(
                w.is_finite() && w > 0.,
                "Corner weight normalization produced invalid weights",
            )?;
            controls.push(h[..h.len() - 1].iter().map(|v| v / w).collect());
            weights.push(w);
        }
        if knots.is_empty() {
            knots.extend(std::iter::repeat_n(left, degree + 1));
        }
        knots.extend(std::iter::repeat_n(right, degree));
    }
    knots.push(b);
    // The affine multiplier has exact endpoint weight 1. Materialize that
    // representation directly instead of retaining reciprocal-product roundoff.
    // Keep the clamped source endpoints; the whole-domain gate covers this edit.
    let last = controls.len() - 1;
    controls[0] = clamped.control_points[0].clone();
    controls[last] = clamped.control_points.last().unwrap().clone();
    weights[0] = 1.;
    weights[last] = 1.;
    let result = Curve {
        degree,
        knots,
        control_points: controls,
        weights,
        periodic: false,
    };
    result.validate()?;
    Ok(result)
}

pub struct PreparedBoundary {
    pub curve: Curve,
    pub accepted: bool,
    pub error_upper: f64,
}
/// Whole-domain positional gate. Extrusion's vMin is the exact curve control
/// definition; the existing outward surface bound includes every point on it.
pub fn checked_endpoint_weights(curve: &Curve, budget: f64) -> Result<PreparedBoundary> {
    check(
        budget.is_finite() && budget >= 0.,
        "Boundary preparation budget must be finite and nonnegative",
    )?;
    let candidate = normalize_endpoint_weights(curve)?;
    let source = crate::surface::extrude(curve, [0., 0., 1.])?;
    let target = crate::surface::extrude(&candidate, [0., 0., 1.])?;
    let error_upper = crate::continuity::deviation::positional_upper(&source, &target)?;
    let accepted = error_upper <= budget;
    Ok(PreparedBoundary {
        curve: if accepted { candidate } else { curve.clone() },
        accepted,
        error_upper,
    })
}

pub fn patch(boundaries: &[Curve]) -> Result<Surface> {
    check(
        boundaries.len() == 4,
        "Coons patch requires bottom, top, left and right curves",
    )?;
    let mut curves = boundaries.to_vec();
    for c in &mut curves {
        c.validate()?;
        check(
            c.control_points[0].len() == 3 && !c.periodic,
            "Coons boundaries must be non-periodic 3D curves",
        )?;
        if !is_clamped(c) {
            let [a, b] = c.domain();
            *c = c.trim(a, b)?;
        }
    }
    // Report misplaced endpoints before weight compatibility so reversing a
    // boundary points the user to the actual gap instead of weight preparation.
    // trim() clamps every active domain, so endpoint control points are the
    // exact rational endpoints. Evaluation would multiply/divide their weights
    // and can manufacture different rounded positions for identical corners.
    let endpoint = |c: &Curve, end: usize| {
        if end == 0 {
            c.control_points[0].clone()
        } else {
            c.control_points.last().unwrap().clone()
        }
    };
    let corners = [
        endpoint(&curves[0], 0),
        endpoint(&curves[0], 1),
        endpoint(&curves[1], 0),
        endpoint(&curves[1], 1),
    ];
    let other = [
        endpoint(&curves[2], 0),
        endpoint(&curves[3], 0),
        endpoint(&curves[2], 1),
        endpoint(&curves[3], 1),
    ];
    for i in 0..4 {
        check(
            corners[i] == other[i],
            &format!(
                "Coons corner {} does not coincide; connect or reverse the boundary curves",
                i + 1
            ),
        )?;
    }
    // Constant rescaling preserves each rational curve. Match homogeneous corner
    // weights around the boundary cycle; incompatible cycles require reparameterization.
    let scale = |c: &mut Curve, target: f64| {
        let factor = target / c.weights[0];
        for w in &mut c.weights {
            *w *= factor;
        }
    };
    scale(&mut curves[0], 1.);
    let w00 = curves[0].weights[0];
    let w10 = *curves[0].weights.last().unwrap();
    scale(&mut curves[2], w00);
    scale(&mut curves[3], w10);
    let w01 = *curves[2].weights.last().unwrap();
    scale(&mut curves[1], w01);
    check(
        curves[1].weights.last() == curves[3].weights.last(),
        "Coons corner weights are incompatible; reparameterize the boundaries",
    )?;
    let corner_weights = [
        curves[0].weights[0],
        *curves[0].weights.last().unwrap(),
        curves[1].weights[0],
        *curves[1].weights.last().unwrap(),
    ];
    let corners: Vec<Vec<f64>> = corners
        .iter()
        .zip(corner_weights)
        .map(|(p, w)| vec![p[0] * w, p[1] * w, p[2] * w, w])
        .collect();
    let u = loft_aligned(&curves[..2])?;
    let v = loft_aligned(&curves[2..])?;
    let nu = u.control_points.len();
    let nv = v.control_points.len();
    let greville = |knots: &[f64], degree: usize, i: usize| {
        knots[i + 1..=i + degree].iter().sum::<f64>() / degree as f64
    };
    let homogeneous: Vec<Vec<Vec<f64>>> = (0..nu)
        .map(|i| {
            let x = greville(&u.knots_u, u.degree_u, i);
            (0..nv)
                .map(|j| {
                    let y = greville(&v.knots_u, v.degree_u, j);
                    (0..4)
                        .map(|k| {
                            let h = |s: &Surface, i: usize, j: usize| {
                                if k == 3 {
                                    s.weights[i][j]
                                } else {
                                    s.control_points[i][j][k] * s.weights[i][j]
                                }
                            };
                            (1. - y) * h(&u, i, 0)
                                + y * h(&u, i, 1)
                                + (1. - x) * h(&v, j, 0)
                                + x * h(&v, j, 1)
                                - ((1. - x) * (1. - y) * corners[0][k]
                                    + x * (1. - y) * corners[1][k]
                                    + (1. - x) * y * corners[2][k]
                                    + x * y * corners[3][k])
                        })
                        .collect()
                })
                .collect()
        })
        .collect();
    check(
        homogeneous
            .iter()
            .flatten()
            .all(|h| h.iter().all(|v| v.is_finite()) && h[3] > 0.),
        "Coons patch cannot certify positive finite weights",
    )?;
    let weights = homogeneous
        .iter()
        .map(|row| row.iter().map(|h| h[3]).collect())
        .collect();
    let control_points = homogeneous
        .iter()
        .map(|row| {
            row.iter()
                .map(|h| (0..3).map(|k| h[k] / h[3]).collect())
                .collect()
        })
        .collect();
    let surface = Surface {
        degree_u: u.degree_u,
        degree_v: v.degree_u,
        knots_u: u.knots_u,
        knots_v: v.knots_u,
        control_points,
        weights,
        periodic_u: false,
        periodic_v: false,
    };
    surface.validate()?;
    Ok(surface)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn curve(points: Vec<Vec<f64>>) -> Curve {
        let p = points.len() - 1;
        Curve {
            degree: p,
            knots: [vec![0.; p + 1], vec![1.; p + 1]].concat(),
            weights: vec![1.; p + 1],
            control_points: points,
            periodic: false,
        }
    }
    fn boundaries() -> Vec<Curve> {
        vec![
            curve(vec![vec![0., 0., 0.], vec![1., 0., 1.], vec![2., 0., 0.]]),
            curve(vec![vec![0., 2., 0.], vec![2., 2., 0.]]),
            curve(vec![vec![0., 0., 0.], vec![0., 2., 0.]]),
            curve(vec![vec![2., 0., 0.], vec![2., 1., 2.], vec![2., 2., 0.]]),
        ]
    }
    #[test]
    fn reproduces_all_four_boundaries_and_analytic_interior() {
        let mut c = boundaries();
        c[0] = c[0].insert(0.3, 1).unwrap();
        let s = patch(&c).unwrap();
        for i in 0..=20 {
            let t = i as f64 / 20.;
            for (edge, (u, v)) in [(t, 0.), (t, 1.), (0., t), (1., t)].into_iter().enumerate() {
                let a = s.evaluate(u, v).unwrap().point;
                let b = c[edge].evaluate(t).unwrap().point;
                for k in 0..3 {
                    assert!((a[k] - b[k]).abs() < 1e-12);
                }
            }
        }
        for i in 1..10 {
            for j in 1..10 {
                let u = i as f64 / 10.;
                let v = j as f64 / 10.;
                let p = s.evaluate(u, v).unwrap().point;
                let z = (1. - v) * 2. * u * (1. - u) + u * 4. * v * (1. - v);
                assert!((p[0] - 2. * u).abs() < 1e-12);
                assert!((p[1] - 2. * v).abs() < 1e-12);
                assert!((p[2] - z).abs() < 1e-12);
            }
        }
    }
    #[test]
    fn rational_quarter_cylinder_preserves_boundaries_and_radius() {
        let mut bottom = curve(vec![vec![1., 0., 0.], vec![1., 1., 0.], vec![0., 1., 0.]]);
        bottom.weights = vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.];
        let mut top = bottom.clone();
        for p in &mut top.control_points {
            p[2] = 2.;
        }
        let left = curve(vec![vec![1., 0., 0.], vec![1., 0., 2.]]);
        let right = curve(vec![vec![0., 1., 0.], vec![0., 1., 2.]]);
        let boundaries = vec![bottom, top, left, right];
        let s = patch(&boundaries).unwrap();
        for i in 0..=32 {
            let t = i as f64 / 32.;
            for (edge, (u, v)) in [(t, 0.), (t, 1.), (0., t), (1., t)].into_iter().enumerate() {
                let p = s.evaluate(u, v).unwrap().point;
                let q = boundaries[edge].evaluate(t).unwrap().point;
                for k in 0..3 {
                    assert!((p[k] - q[k]).abs() < 1e-12);
                }
            }
            for j in 0..=16 {
                let v = j as f64 / 16.;
                let p = s.evaluate(t, v).unwrap().point;
                assert!((p[0] * p[0] + p[1] * p[1] - 1.).abs() < 1e-12);
                assert!((p[2] - 2. * v).abs() < 1e-12);
            }
        }
        let mut scaled = boundaries.clone();
        for (c, f) in scaled.iter_mut().zip([2., 4., 8., 16.]) {
            for w in &mut c.weights {
                *w *= f;
            }
        }
        let same = patch(&scaled).unwrap();
        assert_eq!(
            same.evaluate(0.3, 0.7).unwrap().point,
            s.evaluate(0.3, 0.7).unwrap().point
        );
    }
    #[test]
    #[cfg(feature = "codec")]
    fn mixed_rational_bases_domains_and_scales_preserve_boundary_parameters() {
        let mut c = boundaries();
        c[0].weights = vec![1., 0.8, 2.];
        c[1].weights = vec![3., 4.];
        c[2].weights = vec![1., 3.];
        c[3].weights = vec![2., 1.5, 4.];
        c[0] = c[0].insert(0.17, 1).unwrap().insert(0.61, 1).unwrap();
        c[3] = c[3].insert(0.42, 1).unwrap();
        let domains = [[-3., 7.], [11., 13.], [0.2, 0.9], [-20., -10.]];
        for (curve, [a, b]) in c.iter_mut().zip(domains) {
            for k in &mut curve.knots {
                *k = a + (b - a) * *k;
            }
        }
        let before = value_codec::to_string(&c).unwrap();
        let surface = patch(&c).unwrap();
        assert_eq!(value_codec::to_string(&c).unwrap(), before);
        for i in 0..=200 {
            let t = i as f64 / 200.;
            for (edge, (u, v)) in [(t, 0.), (t, 1.), (0., t), (1., t)].into_iter().enumerate() {
                let [a, b] = domains[edge];
                let p = surface.evaluate(u, v).unwrap().point;
                let q = c[edge].evaluate(a + (b - a) * t).unwrap().point;
                for k in 0..3 {
                    assert!(
                        (p[k] - q[k]).abs() < 2e-12,
                        "edge {edge}, station {i}, axis {k}"
                    );
                }
            }
        }
        let mut scaled = c.clone();
        for (curve, factor) in scaled.iter_mut().zip([0.25, 8., 2., 16.]) {
            for w in &mut curve.weights {
                *w *= factor;
            }
        }
        let same = patch(&scaled).unwrap();
        for i in 0..=20 {
            for j in 0..=20 {
                let u = i as f64 / 20.;
                let v = j as f64 / 20.;
                let p = surface.evaluate(u, v).unwrap().point;
                let q = same.evaluate(u, v).unwrap().point;
                for k in 0..3 {
                    assert!((p[k] - q[k]).abs() < 2e-12);
                }
            }
        }
    }
    #[test]
    #[cfg(feature = "codec")]
    fn refuses_nonpositive_interior_weights_even_with_positive_boundary_weights() {
        let mut c = vec![
            curve(vec![vec![0., 0., 0.], vec![1., 0., 0.], vec![2., 0., 0.]]),
            curve(vec![vec![0., 2., 0.], vec![1., 2., 0.], vec![2., 2., 0.]]),
            curve(vec![vec![0., 0., 0.], vec![0., 1., 0.], vec![0., 2., 0.]]),
            curve(vec![vec![2., 0., 0.], vec![2., 1., 0.], vec![2., 2., 0.]]),
        ];
        for boundary in &mut c {
            boundary.weights = vec![1., 0.1, 1.];
            boundary.validate().unwrap();
        }
        let before = value_codec::to_string(&c).unwrap();
        assert!(
            patch(&c)
                .unwrap_err()
                .to_string()
                .contains("positive finite weights")
        );
        assert_eq!(value_codec::to_string(&c).unwrap(), before);
    }
    #[test]
    #[cfg(feature = "codec")]
    fn affine_homogeneous_normalization_preserves_rational_curve_and_domain() {
        let mut c = boundaries()[0].clone();
        c.weights = vec![1., 0.8, 2.];
        c = c.insert(0.17, 1).unwrap().insert(0.61, 1).unwrap();
        for knot in &mut c.knots {
            *knot = -3. + 10. * *knot;
        }
        let before = value_codec::to_string(&c).unwrap();
        let prepared = normalize_endpoint_weights(&c).unwrap();
        assert_eq!(prepared.domain(), c.domain());
        assert_eq!(prepared.degree, c.degree + 1);
        assert!((prepared.weights[0] - 1.).abs() < 1e-15);
        assert!((prepared.weights.last().unwrap() - 1.).abs() < 1e-15);
        for i in 0..=1000 {
            let t = -3. + 10. * i as f64 / 1000.;
            let p = c.evaluate(t).unwrap().point;
            let q = prepared.evaluate(t).unwrap().point;
            for k in 0..3 {
                assert!((p[k] - q[k]).abs() < 2e-12);
            }
        }
        assert_eq!(value_codec::to_string(&c).unwrap(), before);
    }
    #[test]
    fn normalization_preserves_points_and_jets_across_degrees_and_weight_scales() {
        for degree in [1, 2, 3, 5, 12, 24] {
            for scale in [1e-10, 1., 1e10] {
                let points = (0..=degree)
                    .map(|i| {
                        let x = i as f64 / degree as f64;
                        vec![x, x * x, (3. * x).sin()]
                    })
                    .collect();
                let mut c = curve(points);
                c.weights = (0..=degree)
                    .map(|i| scale * (0.5 + i as f64 / degree as f64))
                    .collect();
                let prepared = normalize_endpoint_weights(&c).unwrap();
                for i in 1..100 {
                    let t = i as f64 / 100.;
                    let original = c.evaluate(t).unwrap();
                    let actual = prepared.evaluate(t).unwrap();
                    for (a, b) in [
                        (original.point, actual.point),
                        (original.d1.unwrap(), actual.d1.unwrap()),
                        (original.d2.unwrap(), actual.d2.unwrap()),
                    ] {
                        for k in 0..3 {
                            assert!(
                                (a[k] - b[k]).abs() < 2e-9 * (1. + a[k].abs()),
                                "degree {degree}, scale {scale}, station {i}"
                            );
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn normalization_refuses_degree_limit_and_invalid_weights() {
        let high = curve((0..=25).map(|i| vec![i as f64, 0., 0.]).collect());
        assert!(normalize_endpoint_weights(&high).is_err());
        let mut tiny = boundaries()[0].clone();
        tiny.weights = vec![f64::from_bits(1); 3];
        assert!(
            normalize_endpoint_weights(&tiny)
                .unwrap_err()
                .to_string()
                .contains("Weights")
        );
    }
    #[test]
    fn prepared_incompatible_weight_cycle_builds_with_original_boundary_parameters() {
        let mut c = boundaries();
        c[0].weights = vec![1., 0.8, 0.5];
        assert!(patch(&c).is_err());
        let prepared = c
            .iter()
            .map(normalize_endpoint_weights)
            .collect::<Result<Vec<_>>>()
            .unwrap();
        let surface = patch(&prepared).unwrap();
        for i in 0..=200 {
            let t = i as f64 / 200.;
            for (edge, (u, v)) in [(t, 0.), (t, 1.), (0., t), (1., t)].into_iter().enumerate() {
                let p = surface.evaluate(u, v).unwrap().point;
                let q = c[edge].evaluate(t).unwrap().point;
                for k in 0..3 {
                    assert!((p[k] - q[k]).abs() < 2e-12);
                }
            }
        }
    }
    #[test]
    #[cfg(feature = "codec")]
    fn checked_preparation_bounds_whole_domain_and_rolls_back_below_budget() {
        let mut c = boundaries()[0].clone();
        c.weights = vec![1., 0.8, 0.5];
        c = c.insert(0.17, 1).unwrap();
        let prepared = checked_endpoint_weights(&c, 1e-6).unwrap();
        assert!(prepared.accepted);
        assert!(prepared.error_upper.is_finite() && prepared.error_upper > 0.);
        for i in 0..=1000 {
            let t = i as f64 / 1000.;
            let p = c.evaluate(t).unwrap().point;
            let q = prepared.curve.evaluate(t).unwrap().point;
            let distance = p
                .iter()
                .zip(q)
                .map(|(a, b)| (a - b) * (a - b))
                .sum::<f64>()
                .sqrt();
            assert!(distance <= prepared.error_upper);
        }
        let refused = checked_endpoint_weights(&c, 0.).unwrap();
        assert!(!refused.accepted);
        assert_eq!(
            value_codec::to_string(&refused.curve).unwrap(),
            value_codec::to_string(&c).unwrap()
        );
        for budget in [-1., f64::NAN, f64::INFINITY] {
            assert!(checked_endpoint_weights(&c, budget).is_err());
        }
    }
    #[test]
    fn preparation_handles_nonbinary_corner_weights() {
        for seed in 1..=100 {
            let mut source = boundaries();
            for (edge, c) in source.iter_mut().enumerate() {
                c.weights = (0..c.weights.len())
                    .map(|i| 0.1 + (seed * 7 + edge * 13 + i * 17) as f64 / 31.)
                    .collect();
            }
            let prepared = source
                .iter()
                .map(|c| checked_endpoint_weights(c, 1e-6).unwrap())
                .collect::<Vec<_>>();
            assert!(prepared.iter().all(|p| p.accepted));
            let curves = prepared.into_iter().map(|p| p.curve).collect::<Vec<_>>();
            let surface = patch(&curves).unwrap_or_else(|e| panic!("seed {seed}: {e}"));
            for i in 0..=20 {
                let t = i as f64 / 20.;
                for (edge, (u, v)) in [(t, 0.), (t, 1.), (0., t), (1., t)].into_iter().enumerate() {
                    let p = surface.evaluate(u, v).unwrap().point;
                    let q = source[edge].evaluate(t).unwrap().point;
                    for k in 0..3 {
                        assert!(
                            (p[k] - q[k]).abs() < 2e-12,
                            "seed {seed}, edge {edge}, station {i}"
                        );
                    }
                }
            }
        }
    }
    #[test]
    #[cfg(feature = "transport")]
    fn checked_weight_transport_preserves_report_and_rollback() {
        let c = boundaries()[0].clone();
        for budget in [0., 1e-6] {
            let result=crate::dispatch(value_codec::json!({"op":"curve_prepare_coons_weights","curve":c,"maxError":budget})).unwrap();
            assert_eq!(result["report"]["wholeCurve"], value_codec::json!(true));
            assert_eq!(
                result["report"]["accepted"],
                value_codec::json!(budget > 0.)
            );
            if budget == 0. {
                assert_eq!(result["curve"], value_codec::to_value(&c).unwrap());
            }
        }
    }
    #[test]
    fn prepared_mixed_knots_and_conditioned_domains_keep_boundary_parameters() {
        let mut sources = boundaries();
        for (edge, c) in sources.iter_mut().enumerate() {
            c.weights = (0..c.weights.len())
                .map(|i| 0.1 + (7 + edge * 13 + i * 17) as f64 / 31.)
                .collect();
            *c = c.insert(0.17, 1).unwrap().insert(0.61, 1).unwrap();
            let (a, b) = [(-3., 7.), (1e6, 1e6 + 0.001), (-1e8, -1e8 + 2.), (0.2, 0.9)][edge];
            for k in &mut c.knots {
                *k = a + (b - a) * *k;
            }
        }
        let prepared = sources
            .iter()
            .map(|c| checked_endpoint_weights(c, 1e-6).unwrap())
            .collect::<Vec<_>>();
        assert!(prepared.iter().all(|p| p.accepted));
        let surface = patch(&prepared.into_iter().map(|p| p.curve).collect::<Vec<_>>()).unwrap();
        for i in 0..=200 {
            let t = i as f64 / 200.;
            for (edge, (u, v)) in [(t, 0.), (t, 1.), (0., t), (1., t)].into_iter().enumerate() {
                let [a, b] = sources[edge].domain();
                let p = surface.evaluate(u, v).unwrap().point;
                let q = sources[edge].evaluate(a + (b - a) * t).unwrap().point;
                for k in 0..3 {
                    assert!((p[k] - q[k]).abs() < 1e-6, "edge {edge}, station {i}");
                }
            }
        }
    }
    #[test]
    fn refuses_gaps_incompatible_weights_and_wrong_roles() {
        let mut c = boundaries();
        c[2].control_points[0][0] = 0.001;
        assert!(patch(&c).unwrap_err().to_string().contains("corner"));
        let mut c = boundaries();
        c[0].weights[2] = 0.5;
        assert!(
            patch(&c)
                .unwrap_err()
                .to_string()
                .contains("corner weights")
        );
        assert!(patch(&boundaries()[..3]).is_err());
    }
    #[test]
    fn reports_reversed_rational_boundary_gap_before_weight_cycle() {
        let mut c = boundaries();
        c[0].weights = vec![1., 0.8, 2.];
        c[1].weights = vec![3., 4.];
        c[2].weights = vec![1., 3.];
        c[3].weights = vec![2., 1.5, 4.];
        patch(&c).unwrap();
        c[3] = c[3].reverse().unwrap();
        assert!(
            patch(&c)
                .unwrap_err()
                .to_string()
                .contains("Coons corner 2")
        );
    }
    #[test]
    #[cfg(feature = "codec")]
    fn uniform_weight_rescaling_does_not_create_a_rounded_corner_gap() {
        let mut c = boundaries();
        for (curve, weight) in c.iter_mut().zip([0.1, 0.7, 0.3, 0.9]) {
            for point in &mut curve.control_points {
                point[0] = 0.1 + 0.3 * point[0];
                point[1] = 0.1 + 0.7 * point[1];
            }
            curve.weights.fill(weight);
        }
        let before = value_codec::to_string(&c).unwrap();
        patch(&c).unwrap();
        assert_eq!(value_codec::to_string(&c).unwrap(), before);
    }
}
