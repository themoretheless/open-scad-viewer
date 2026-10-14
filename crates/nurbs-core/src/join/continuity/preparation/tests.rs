use super::*;
fn patch(degree: usize, knots: Vec<f64>) -> Surface {
    let n = knots.len() - degree - 1;
    Surface {
        degree_u: 3,
        degree_v: degree,
        knots_u: vec![2., 2., 2., 2., 5., 5., 5., 5.],
        knots_v: knots,
        control_points: (0..4)
            .map(|i| {
                (0..n)
                    .map(|j| vec![i as f64, j as f64, 0.1 * (i * j) as f64])
                    .collect()
            })
            .collect(),
        weights: (0..4)
            .map(|i| {
                (0..n)
                    .map(|j| 1. + 0.02 * i as f64 + 0.03 * j as f64)
                    .collect()
            })
            .collect(),
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
#[cfg(feature = "codec")]
fn aligns_degrees_domains_and_knots_without_introducing_C0_seams() {
    let a = patch(2, vec![3., 3., 3., 5., 7., 7., 7.]);
    let b = patch(3, vec![-2., -2., -2., -2., 1., 2., 2., 2., 2.]);
    let before = value_codec::to_value((&a, &b)).unwrap();
    for reverse in [false, true] {
        let result = prepare(&a, &b, "uMax", "uMin", reverse).unwrap();
        let proof = certify(&a, &b, "uMax", "uMin", &result, 1e-8).unwrap();
        assert_eq!(proof["accepted"], value_codec::json!(true));
        assert_eq!(
            certify(&a, &b, "uMax", "uMin", &result, 0.).unwrap()["accepted"],
            value_codec::json!(false)
        );
        assert_eq!(result.degree, 3);
        assert_eq!(result.reference.knots_v, result.edited.knots_v);
        assert_eq!(
            result
                .reference
                .knots_v
                .iter()
                .filter(|&&k| k == 0.5)
                .count(),
            2
        );
        assert_eq!(result.reference_domain, [3., 7.]);
        assert_eq!(result.edited_domain, [-2., 2.]);
        for i in 0..=20 {
            for j in 0..=20 {
                let u = 2. + 3. * i as f64 / 20.;
                let t = j as f64 / 20.;
                for (source, target, v) in [
                    (&a, &result.reference, 3. + 4. * t),
                    (
                        &b,
                        &result.edited,
                        -2. + 4. * if reverse { 1. - t } else { t },
                    ),
                ] {
                    let p = source.evaluate(u, v).unwrap();
                    let q = target.evaluate(u, t).unwrap();
                    for (x, y) in p.point.iter().zip(&q.point) {
                        assert!((x - y).abs() < 1e-10);
                    }
                    let sign = if std::ptr::eq(source, &b) && reverse {
                        -1.
                    } else {
                        1.
                    };
                    let p = value_codec::to_value(p).unwrap();
                    let q = value_codec::to_value(q).unwrap();
                    for (field, factor) in [
                        ("du", 1.),
                        ("dv", 4. * sign),
                        ("duu", 1.),
                        ("duv", 4. * sign),
                    ] {
                        let a: Vec<f64> = value_codec::from_value(p[field].clone()).unwrap();
                        let b: Vec<f64> = value_codec::from_value(q[field].clone()).unwrap();
                        for (x, y) in a.iter().zip(b) {
                            assert!((x * factor - y).abs() < 1e-9, "{field}: {x} versus {y}");
                        }
                    }
                }
            }
        }
    }
    assert_eq!(value_codec::to_value((&a, &b)).unwrap(), before);
}
#[test]
#[cfg(feature = "codec")]
fn already_compatible_normalized_surfaces_keep_their_controls() {
    let a = patch(3, vec![0., 0., 0., 0., 0.5, 1., 1., 1., 1.]);
    let result = prepare(&a, &a, "uMax", "uMin", false).unwrap();
    assert_eq!(
        certify(&a, &a, "uMax", "uMin", &result, 0.).unwrap()["accepted"],
        value_codec::json!(true)
    );
    assert_eq!(
        value_codec::to_value(&a).unwrap(),
        value_codec::to_value(result.reference).unwrap()
    );
    assert_eq!(
        value_codec::to_value(&a).unwrap(),
        value_codec::to_value(result.edited).unwrap()
    );
}

#[test]
#[cfg(feature = "codec")]
fn certificate_detects_candidate_translation_and_refuses_rounded_knot_maps() {
    let a = patch(3, vec![0., 0., 0., 0., 0.5, 1., 1., 1., 1.]);
    let mut result = prepare(&a, &a, "uMax", "uMin", false).unwrap();
    for row in &mut result.edited.control_points {
        for p in row {
            p[0] += 0.001;
        }
    }
    let proof = certify(&a, &a, "uMax", "uMin", &result, 1e-6).unwrap();
    assert_eq!(proof["accepted"], value_codec::json!(false));
    assert!(proof["editedErrorUpper"].as_f64().unwrap() >= 0.001 - 1e-14);
    let a = patch(3, vec![0., 0., 0., 0., 0.3, 0.9, 0.9, 0.9, 0.9]);
    let result = prepare(&a, &a, "uMax", "uMin", false).unwrap();
    assert_eq!(
        certify(&a, &a, "uMax", "uMin", &result, 1.).unwrap()["reason"],
        value_codec::json!("unproven-parameter-normalization")
    );
}

fn periodic_patch(degree: usize, unique: usize) -> Surface {
    let points: Vec<Vec<Vec<f64>>> = (0..4)
        .map(|i| {
            (0..unique + degree)
                .map(|j| {
                    let t = (j % unique) as f64 * std::f64::consts::TAU / unique as f64;
                    vec![i as f64, t.cos(), t.sin()]
                })
                .collect()
        })
        .collect();
    Surface {
        degree_u: 3,
        degree_v: degree,
        knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        knots_v: (0..unique + 2 * degree + 1).map(|i| i as f64).collect(),
        control_points: points,
        weights: (0..4)
            .map(|i| {
                (0..unique + degree)
                    .map(|j| 1. + 0.02 * i as f64 + 0.01 * (j % unique) as f64)
                    .collect()
            })
            .collect(),
        periodic_u: false,
        periodic_v: true,
    }
}
#[test]
#[cfg(feature = "codec")]
fn prepares_periodic_seams_with_different_degrees_and_preserves_wrapping() {
    let a = periodic_patch(2, 4);
    let b = periodic_patch(3, 8);
    for reverse in [false, true] {
        let result = prepare(&a, &b, "uMax", "uMin", reverse).unwrap();
        assert!(result.reference.periodic_v && result.edited.periodic_v);
        assert_eq!(result.reference.knots_v, result.edited.knots_v);
        let proof = certify(&a, &b, "uMax", "uMin", &result, 1e-8).unwrap();
        assert_eq!(proof["accepted"], value_codec::json!(true), "{proof}");
        for (source, target, domain, rev) in [
            (&a, &result.reference, [2., 6.], false),
            (&b, &result.edited, [3., 11.], reverse),
        ] {
            for i in 0..=64 {
                let t = i as f64 / 64.;
                let v = domain[0] + (domain[1] - domain[0]) * if rev { 1. - t } else { t };
                let p = source.evaluate(0.3, v).unwrap().point;
                let q = target.evaluate(0.3, t).unwrap().point;
                for (x, y) in p.iter().zip(q) {
                    assert!((x - y).abs() < 1e-10);
                }
            }
            for row in &target.control_points {
                assert_eq!(&row[..3], &row[row.len() - 3..]);
            }
        }
    }
}

#[test]
#[cfg(feature = "codec")]
fn checked_preparation_returns_original_pair_on_failed_budget() {
    let a = patch(2, vec![0., 0., 0., 0.5, 1., 1., 1.]);
    let b = patch(3, vec![0., 0., 0., 0., 1., 1., 1., 1.]);
    let result = checked(&a, &b, "uMax", "uMin", false, 0.).unwrap();
    assert_eq!(result["report"]["accepted"], value_codec::json!(false));
    assert_eq!(result["reference"], value_codec::to_value(&a).unwrap());
    assert_eq!(result["edited"], value_codec::to_value(&b).unwrap());
}

#[test]
#[cfg(feature = "codec")]
fn explicit_conversion_prepares_mixed_seams_and_bounds_the_original_geometry() {
    let periodic = periodic_patch(2, 4);
    let open = patch(3, vec![0., 0., 0., 0., 1., 1., 1., 1.]);
    assert!(checked(&periodic, &open, "uMax", "uMin", false, 1e-8).is_err());
    for reverse in [false, true] {
        let result =
            checked_with_conversion(&periodic, &open, "uMax", "uMin", reverse, 1e-8, true).unwrap();
        assert_eq!(result["report"]["accepted"], value_codec::json!(true));
        assert_eq!(
            result["basis"]["periodicityRemoved"]["reference"],
            value_codec::json!(true)
        );
        assert_eq!(
            result["basis"]["periodicityRemoved"]["edited"],
            value_codec::json!(false)
        );
        let a: Surface = value_codec::from_value(result["reference"].clone()).unwrap();
        assert!(!a.periodic_v);
        for i in 0..=32 {
            let u = i as f64 / 32.;
            let first = a.evaluate(u, 0.).unwrap().point;
            let last = a.evaluate(u, 1.).unwrap().point;
            for (x, y) in first.iter().zip(last) {
                assert!((x - y).abs() < 1e-12);
            }
        }
        let failed =
            checked_with_conversion(&periodic, &open, "uMax", "uMin", reverse, 0., true).unwrap();
        assert_eq!(failed["report"]["accepted"], value_codec::json!(false));
        assert_eq!(
            failed["reference"],
            value_codec::to_value(&periodic).unwrap()
        );
        assert_eq!(failed["edited"], value_codec::to_value(&open).unwrap());
    }
}

#[test]
#[cfg(feature = "codec")]
fn explicit_conversion_can_unlink_both_periodic_inputs() {
    let a = periodic_patch(2, 4);
    let b = periodic_patch(3, 8);
    let result = checked_with_conversion(&a, &b, "uMax", "uMin", true, 1e-8, true).unwrap();
    assert_eq!(result["report"]["accepted"], value_codec::json!(true));
    assert_eq!(
        result["basis"]["periodicityRemoved"],
        value_codec::json!({"reference":true,"edited":true})
    );
    assert_eq!(result["reference"]["periodicV"], value_codec::json!(false));
    assert_eq!(result["edited"]["periodicV"], value_codec::json!(false));
}
