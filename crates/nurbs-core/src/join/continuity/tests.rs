#[cfg(feature = "codec")]
use value_codec::json;

use super::*;
fn patch() -> Surface {
    Surface {
        degree_u: 3,
        degree_v: 3,
        knots_u: vec![2., 2., 2., 2., 5., 5., 5., 5.],
        knots_v: vec![-1., -1., -1., -1., 3., 3., 3., 3.],
        control_points: (0..4)
            .map(|i| {
                (0..4)
                    .map(|j| {
                        vec![
                            i as f64,
                            j as f64,
                            0.15 * (i * i) as f64 + 0.1 * (i * j) as f64 + 0.2 * (j * j) as f64,
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: (0..4)
            .map(|i| {
                (0..4)
                    .map(|j| 1. + 0.03 * i as f64 + 0.02 * j as f64 + 0.01 * (i * j) as f64)
                    .collect()
            })
            .collect(),
        periodic_u: false,
        periodic_v: false,
    }
}

#[cfg(feature = "codec")]
fn jet(s: &Surface, b: Boundary, t: f64) -> Vec<Vec<f64>> {
    let (_, nc, kc, _) = b.cross(s);
    let (pc, _, _, _) = b.cross(s);
    let (ps, ns, ks, _) = b.along(s);
    let cross = kc[if b.max { nc } else { pc }];
    let along = ks[ps] + t * (ks[ns] - ks[ps]);
    let (u, v) = if b.cross_u {
        (cross, along)
    } else {
        (along, cross)
    };
    let value = value_codec::to_value(s.evaluate(u, v).unwrap()).unwrap();
    let cross_span = (kc[nc] - kc[pc]) * if b.max { -1. } else { 1. };
    let seam_span = ks[ns] - ks[ps];
    let fields = if b.cross_u {
        ["point", "du", "duu", "dv", "dvv", "duv"]
    } else {
        ["point", "dv", "dvv", "du", "duu", "duv"]
    };
    let factors = [
        1.,
        cross_span,
        cross_span * cross_span,
        seam_span,
        seam_span * seam_span,
        cross_span * seam_span,
    ];
    fields
        .iter()
        .zip(factors)
        .map(|(field, f)| {
            let a: Vec<f64> = value_codec::from_value(value[*field].clone()).unwrap();
            a.into_iter().map(|x| x * f).collect()
        })
        .collect()
}
fn close(a: &[f64], b: &[f64], factor: f64) {
    for (x, y) in a.iter().zip(b) {
        assert!((x - factor * y).abs() < 2e-10, "{x} != {factor} * {y}");
    }
}
// This allowance covers the binary64 test evaluator, not the certified bound.
// scripts/verify-surface-jets.py checks the bounds independently at 80 digits.
fn evaluation_slack(a: &[f64], b: &[f64], factor: f64) -> f64 {
    256. * f64::EPSILON
        * (1.
            + a.iter().map(|x| x.abs()).sum::<f64>()
            + factor.abs() * b.iter().map(|x| x.abs()).sum::<f64>())
}
#[test]
#[cfg(feature = "codec")]
fn rational_jets_match_all_boundary_pairs_in_normalized_coordinates() {
    let source = patch();
    let mut target = patch();
    for row in &mut target.control_points {
        for p in row {
            p[0] += 20.;
            p[2] -= 3.;
        }
    }
    let before = value_codec::to_value(&target).unwrap();
    for rn in ["uMin", "uMax", "vMin", "vMax"] {
        for en in ["uMin", "uMax", "vMin", "vMax"] {
            for order in [1, 2] {
                for scale in [0.4, 1., 1.7] {
                    let result =
                        match_surface_jets(&source, &target, rn, en, order, scale).unwrap();
                    assert_eq!(result["report"]["regularityCertified"], json!(true));
                    let output: Surface =
                        value_codec::from_value(result["surface"].clone()).unwrap();
                    for t in [0., 0.13, 0.5, 0.83, 1.] {
                        let a = jet(&source, Boundary::parse(rn).unwrap(), t);
                        let b = jet(&output, Boundary::parse(en).unwrap(), t);
                        close(&b[0], &a[0], 1.);
                        close(&b[1], &a[1], -scale);
                        close(&b[3], &a[3], 1.);
                        if order == 2 {
                            close(&b[2], &a[2], scale * scale);
                            close(&b[4], &a[4], 1.);
                            close(&b[5], &a[5], -scale);
                        }
                        for (field, key, factor) in [
                            (0, "positionUpper", 1.),
                            (1, "firstDerivativeUpper", -scale),
                        ] {
                            let residual = b[field]
                                .iter()
                                .zip(&a[field])
                                .map(|(x, y)| (x - factor * y).powi(2))
                                .sum::<f64>()
                                .sqrt();
                            let bound = result["report"]["errorBounds"][key].as_f64().unwrap();
                            assert!(
                                residual <= bound + evaluation_slack(&b[field], &a[field], factor),
                                "{key}: measured {residual}, bound {bound}"
                            );
                        }
                        if order == 2 {
                            for (field, key, factor) in [
                                (2, "secondDerivativeUpper", scale * scale),
                                (5, "mixedDerivativeUpper", -scale),
                            ] {
                                let residual = b[field]
                                    .iter()
                                    .zip(&a[field])
                                    .map(|(x, y)| (x - factor * y).powi(2))
                                    .sum::<f64>()
                                    .sqrt();
                                let bound = result["report"]["errorBounds"][key].as_f64().unwrap();
                                assert!(
                                    residual
                                        <= bound + evaluation_slack(&b[field], &a[field], factor),
                                    "{key}: measured {residual}, bound {bound}"
                                );
                            }
                        }
                    }
                    let edge = Boundary::parse(en).unwrap();
                    for i in 0..4 {
                        for layer in order + 1..4 {
                            assert_eq!(edge.get(&output, i, layer), edge.get(&target, i, layer));
                        }
                    }
                }
            }
        }
    }
    assert_eq!(value_codec::to_value(&target).unwrap(), before);
}
#[test]
#[cfg(feature = "codec")]
fn handles_different_cross_degrees_and_nonuniform_end_spans() {
    let source = patch();
    let mut target = patch();
    target.degree_u = 4;
    target.knots_u = vec![7., 7., 7., 7., 7., 9., 17., 17., 17., 17., 17.];
    target.control_points = (0..6)
        .map(|i| source.control_points[i % 4].clone())
        .collect();
    target.weights = (0..6).map(|i| source.weights[i % 4].clone()).collect();
    for end in ["uMin", "uMax"] {
        let value = match_surface_jets(&source, &target, "uMax", end, 2, 1.3).unwrap();
        let output: Surface = value_codec::from_value(value["surface"].clone()).unwrap();
        for i in 0..=32 {
            let t = i as f64 / 32.;
            let a = jet(&source, Boundary::parse("uMax").unwrap(), t);
            let b = jet(&output, Boundary::parse(end).unwrap(), t);
            close(&b[0], &a[0], 1.);
            close(&b[1], &a[1], -1.3);
            close(&b[2], &a[2], 1.3 * 1.3);
            close(&b[5], &a[5], -1.3);
        }
    }
    target = patch();
    target.knots_u = (-2..6).map(|i| i as f64).collect();
    assert!(match_surface_jets(&source, &target, "uMax", "uMin", 2, 1.).is_err());
}
#[test]
#[cfg(feature = "codec")]
fn reversed_matching_restores_parameter_direction() {
    let source = patch();
    let edited = patch();
    for name in ["uMin", "uMax", "vMin", "vMax"] {
        let value =
            match_surface_jets_oriented(&source, &edited, "uMax", name, 2, 1., true).unwrap();
        let output: Surface = value_codec::from_value(value["surface"].clone()).unwrap();
        assert_eq!(output.knots_u, edited.knots_u);
        assert_eq!(output.knots_v, edited.knots_v);
        assert_eq!(value["report"]["regularityCertified"], json!(true));
        for t in [0., 0.2, 0.5, 0.9, 1.] {
            let a = jet(&source, Boundary::parse("uMax").unwrap(), t);
            let b = jet(&output, Boundary::parse(name).unwrap(), 1. - t);
            close(&b[0], &a[0], 1.);
            close(&b[1], &a[1], -1.);
            close(&b[2], &a[2], 1.);
            close(&b[3], &a[3], -1.);
            close(&b[4], &a[4], 1.);
            close(&b[5], &a[5], 1.);
        }
    }
}
#[test]
#[cfg(feature = "codec")]
fn rejects_rank_loss_between_sampling_sites() {
    let a = 123_f64 / 1024.;
    let y = [0., a * a, 2. * a * a - a, 1. - 3. * a + 3. * a * a];
    let mut source = patch();
    source.weights = vec![vec![1.; 4]; 4];
    source.control_points = (0..4)
        .map(|i| y.iter().map(|&v| vec![i as f64, v, 0.]).collect())
        .collect();
    let result = match_surface_jets(&source, &patch(), "uMax", "uMin", 2, 1.).unwrap();
    assert_eq!(result["report"]["regularityCertified"], json!(false));
    let root = -1. + 4. * a;
    let intervals: Vec<[f64; 2]> = value_codec::from_value(
        result["report"]["referenceRegularity"]["unresolvedIntervals"].clone(),
    )
    .unwrap();
    assert!(intervals.iter().any(|d| d[0] <= root && root <= d[1]));
}
#[test]
#[cfg(feature = "codec")]
fn periodic_and_multispan_seams_are_proved_without_sampling() {
    let mut s = patch();
    s.degree_v = 2;
    s.knots_v = (0..9).map(|i| i as f64).collect();
    s.periodic_v = true;
    s.control_points = (0..4)
        .map(|i| {
            [[1., 0.], [0., 1.], [-1., 0.], [0., -1.], [1., 0.], [0., 1.]]
                .iter()
                .map(|p| vec![p[0], p[1], i as f64])
                .collect()
        })
        .collect();
    s.weights = vec![vec![1., 0.8, 1.2, 1., 1., 0.8]; 4];
    let result = match_surface_jets(&s, &s, "uMax", "uMin", 2, 1.).unwrap();
    assert_eq!(
        result["report"]["regularityCertified"],
        json!(true),
        "{result:?}"
    );
    assert_eq!(result["report"]["referenceRegularity"]["spans"], json!(4));
    let mut broken = patch();
    broken.degree_v = 1;
    broken.knots_v = vec![0., 0., 1., 2., 3., 3.];
    let result = match_surface_jets(&broken, &broken, "uMax", "uMin", 1, 1.).unwrap();
    assert_eq!(result["report"]["regularityCertified"], json!(false));
    assert_eq!(
        result["report"]["referenceRegularity"]["reason"],
        json!("seam-basis-is-not-C1")
    );
}
#[test]
#[cfg(feature = "codec")]
fn application_gate_requires_a_finite_budget_and_second_order_seam_smoothness() {
    let s = patch();
    let accepted = match_surface_jets_checked(&s, &s, "uMax", "uMin", 2, 1., false, 1e-8).unwrap();
    assert_eq!(accepted["report"]["accepted"], json!(true), "{accepted:?}");
    let refused = match_surface_jets_checked(&s, &s, "uMax", "uMin", 2, 1., false, 0.).unwrap();
    assert_eq!(refused["report"]["accepted"], json!(false));
    assert_eq!(
        refused["report"]["reason"],
        json!("deviation-exceeds-budget")
    );
    assert!(match_surface_jets_checked(&s, &s, "uMax", "uMin", 2, 1., false, f64::NAN).is_err());
    let mut c = patch();
    c.degree_v = 2;
    c.knots_v = vec![0., 0., 0., 0.5, 1., 1., 1.];
    let result = match_surface_jets_checked(&c, &c, "uMax", "uMin", 2, 1., false, 1e-6).unwrap();
    assert_eq!(result["report"]["accepted"], json!(false));
    assert_eq!(
        result["report"]["reason"],
        json!("unproven-tangential-smoothness")
    );
    assert_eq!(
        match_surface_jets_checked(&c, &c, "uMax", "uMin", 1, 1., false, 1e-6).unwrap()["report"]["accepted"],
        json!(true)
    );
}
#[test]
fn rounded_normalized_knots_cannot_hide_an_incompatible_basis() {
    assert_eq!(1_f64 / 3., 0.3_f64 / 0.9);
    assert!(affine_knots(&[0., 1., 3.], [0., 3.], &[0., 0.3, 0.9], [0., 0.9]).is_err());
    assert!(affine_knots(&[0., 1., 3.], [0., 3.], &[2., 4., 8.], [2., 8.]).is_ok());
}
#[test]
#[cfg(feature = "codec")]
fn refuses_incompatible_basis_and_nonpositive_result_weights() {
    let source = patch();
    let mut target = patch();
    for scale in [0., -1., f64::NAN, f64::INFINITY] {
        assert!(match_surface_jets(&source, &target, "uMax", "uMin", 1, scale).is_err());
    }
    assert!(match_surface_jets(&source, &target, "uMax", "bad", 1, 1.).is_err());
    assert!(match_surface_jets(&source, &target, "uMax", "uMin", 3, 1.).is_err());
    target.degree_v = 2;
    target.knots_v = vec![-1., -1., -1., 1., 3., 3., 3.];
    assert!(match_surface_jets(&source, &target, "uMax", "uMin", 1, 1.).is_err());
    let mut invalid = source.clone();
    invalid.weights[3] = vec![0.01; 4];
    assert!(match_surface_jets(&invalid, &source, "uMax", "uMin", 1, 1.).is_err());
}
