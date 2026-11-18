use nurbs_core::{
    curve::Curve,
    editing::{bounded_differential as b, exact_derivative as e},
};
fn bezier(p: Vec<Vec<f64>>, w: Vec<f64>, a: f64, z: f64) -> Curve {
    let degree = p.len() - 1;
    Curve {
        degree,
        knots: [vec![a; degree + 1], vec![z; degree + 1]].concat(),
        control_points: p,
        weights: w,
        periodic: false,
    }
}
fn cubic() -> Curve {
    bezier(
        vec![
            vec![0., 0., 0.],
            vec![1., 0., 0.],
            vec![2., 1., 0.],
            vec![3., 3., 3.],
        ],
        vec![1.; 4],
        0.,
        1.,
    )
}
fn exact(s: &Curve) -> e::Report {
    let before = s.clone();
    let r = e::torsion(s, 1_000_000).unwrap();
    assert!(r.refusal.is_none(), "{:?}", r.refusal);
    assert_eq!(*s, before);
    #[cfg(feature = "codec")]
    println!(
        "torsion-field-oracle={}",
        value_codec::json!({"source":s,"mode":"exact","pieces":r.pieces.as_ref().unwrap()})
    );
    r
}
fn bounded(s: &Curve, tol: f64) -> b::Report {
    let before = s.clone();
    let r = b::torsion(s, tol, 1_000_000).unwrap();
    assert!(r.refusal.is_none(), "{:?},work={}", r.refusal, r.work_used);
    assert_eq!(*s, before);
    let spans = (s.degree..s.control_points.len())
        .filter(|&k| s.knots[k] < s.knots[k + 1])
        .collect::<Vec<_>>();
    let pieces = r.pieces.as_ref().unwrap();
    let bounds = r.error_upper.as_ref().unwrap();
    assert_eq!(pieces.len(), spans.len());
    assert_eq!(pieces.len(), bounds.len());
    for ((p, &upper), k) in pieces.iter().zip(bounds).zip(spans) {
        p.validate().unwrap();
        assert_eq!(p.domain(), [s.knots[k], s.knots[k + 1]]);
        assert!(!p.periodic);
        assert!(upper.is_finite() && upper >= 0. && upper <= tol);
        assert!(p.control_points.iter().all(|p| p[1] == 0. && p[2] == 0.));
    }
    #[cfg(feature = "codec")]
    println!(
        "torsion-field-oracle={}",
        value_codec::json!({"source":s,"mode":"bounded","tolerance":tol,"pieces":pieces,"bounds":bounds})
    );
    r
}
fn value(r: &b::Report, u: f64) -> f64 {
    r.pieces.as_ref().unwrap()[0].evaluate(u).unwrap().point[0]
}
fn expected(t: f64) -> f64 {
    1. / (1. + 9. * t * t + 9. * t.powi(4))
}
#[test]
fn nonplanar_cubic_has_the_closed_form_nonconstant_torsion() {
    let s = cubic();
    assert_eq!(
        e::torsion(&s, 1_000_000).unwrap().refusal,
        Some(e::Refusal::NotRepresentable)
    );
    let r = bounded(&s, 1e-10);
    assert!(r.error_upper.as_ref().unwrap()[0] > 0.);
    for i in 0..=32 {
        let t = i as f64 / 32.;
        assert!((value(&r, t) - expected(t)).abs() < 1e-10);
    }
}
#[test]
fn rational_reparameterization_uses_original_unrounded_jets() {
    let s = bezier(
        vec![
            vec![0., 0., 0.],
            vec![0.5, 0., 0.],
            vec![1., 0.25, 0.],
            vec![1.5, 0.75, 0.375],
        ],
        vec![1., 2., 4., 8.],
        0.,
        1.,
    );
    let r = bounded(&s, 1e-10);
    for i in 0..=16 {
        let t = i as f64 / 16.;
        assert!((value(&r, t) - expected(t / (1. + t))).abs() < 1e-10);
    }
}
#[test]
fn affine_clock_reversal_and_stretch_preserve_torsion() {
    let mut reverse = cubic();
    reverse.control_points.reverse();
    let r = bounded(&reverse, 1e-10);
    let mut stretch = cubic();
    stretch.knots = stretch.knots.iter().map(|x| 8. * x - 2.).collect();
    let t = bounded(&stretch, 1e-10);
    for i in 0..=16 {
        let u = i as f64 / 16.;
        assert!((value(&r, u) - expected(1. - u)).abs() < 1e-10);
        assert!((value(&t, 8. * u - 2.) - expected(u)).abs() < 1e-10);
    }
}
#[test]
fn rigid_motion_mirror_and_spatial_scale_have_correct_sign_and_units() {
    for (kind, factor) in [(0, 1.), (1, -1.), (2, 0.5)] {
        let mut s = cubic();
        for p in &mut s.control_points {
            match kind {
                0 => {
                    let old = p.clone();
                    *p = vec![old[1] + 4., old[2] - 8., old[0] + 2.];
                }
                1 => p[0] = -p[0],
                _ => {
                    for x in p {
                        *x *= 2.;
                    }
                }
            }
        }
        let r = bounded(&s, 1e-10);
        for i in 0..=16 {
            let u = i as f64 / 16.;
            assert!((value(&r, u) - factor * expected(u)).abs() < 1e-10);
        }
    }
}
#[test]
fn curved_planar_sources_have_exact_zero_torsion() {
    let s = bezier(
        vec![vec![0., 0., 0.], vec![1.5, 0., 0.], vec![3., 3., 0.]],
        vec![1.; 3],
        0.,
        1.,
    );
    assert!(
        exact(&s).pieces.unwrap()[0]
            .control_points
            .iter()
            .flatten()
            .all(|&x| x == 0.)
    );
    assert_eq!(
        bounded(&s, f64::from_bits(1)).error_upper.unwrap(),
        vec![0.]
    );
}
#[test]
fn rational_planar_source_does_not_require_a_zero_jerk() {
    let s = bezier(
        vec![vec![1., 0., 0.], vec![1., 1., 0.], vec![0., 1., 0.]],
        vec![1., 1., 2.],
        -2.,
        2.,
    );
    assert!(exact(&s).pieces.is_some());
    assert_eq!(bounded(&s, 1e-20).error_upper.unwrap(), vec![0.]);
}
fn refused(s: &Curve) {
    let a = e::torsion(s, 1_000_000).unwrap();
    let b = b::torsion(s, 1e30, 1_000_000).unwrap();
    assert_eq!(a.refusal, Some(e::Refusal::UnprovedDenominator));
    assert_eq!(b.refusal, Some(b::Refusal::UnprovedDenominator));
    assert!(a.pieces.is_none() && b.pieces.is_none() && b.error_upper.is_none());
}
#[test]
fn straight_source_cannot_be_cancelled_into_zero_torsion() {
    refused(&bezier(
        vec![vec![0., 0., 0.], vec![2., 4., 6.]],
        vec![1., 2.],
        0.,
        1.,
    ));
}
#[test]
fn interior_and_endpoint_inflections_do_not_borrow_a_zero_numerator() {
    refused(&bezier(
        vec![
            vec![0., -0.375, 0.],
            vec![1., 0.375, 0.],
            vec![2., -0.375, 0.],
            vec![3., 0.375, 0.],
        ],
        vec![1.; 4],
        0.,
        1.,
    ));
    refused(&bezier(
        vec![
            vec![0., 0., 0.],
            vec![1., 0., 0.],
            vec![2., 0., 0.],
            vec![3., 3., 0.],
        ],
        vec![1.; 4],
        0.,
        1.,
    ));
}
#[test]
fn valid_but_unresolved_bernstein_regularity_is_not_called_singular() {
    refused(&bezier(
        vec![
            vec![-3., 3., -3.],
            vec![-1., -1., 3.],
            vec![1., -1., -3.],
            vec![3., 3., 3.],
        ],
        vec![1.; 4],
        -1.,
        1.,
    ));
}
#[test]
fn late_undefined_span_discards_earlier_exact_result() {
    let s = Curve {
        degree: 3,
        knots: [vec![0.; 4], vec![1.; 3], vec![2.; 4]].concat(),
        control_points: vec![
            vec![-3., 3., 0.],
            vec![-2., 1., 0.],
            vec![-1., 0., 0.],
            vec![0., 0., 0.],
            vec![1., 0., 0.],
            vec![2., 0., 0.],
            vec![3., 0., 0.],
        ],
        weights: vec![1.; 7],
        periodic: false,
    };
    refused(&s);
}
#[test]
fn repeated_nonuniform_spans_retain_distinct_one_sided_torsion() {
    let s = Curve {
        degree: 3,
        knots: [vec![0.; 4], vec![1.; 3], vec![3.; 4]].concat(),
        control_points: vec![
            vec![0., 0., 0.],
            vec![1., 0., 0.],
            vec![2., 1., 0.],
            vec![3., 3., 3.],
            vec![4., 3., 3.],
            vec![5., 4., 3.],
            vec![6., 6., 6.],
        ],
        weights: vec![1.; 7],
        periodic: false,
    };
    let r = bounded(&s, 1e-10);
    let p = r.pieces.unwrap();
    assert_eq!(p.len(), 2);
    assert!((p[0].evaluate(1.).unwrap().point[0] - expected(1.)).abs() < 1e-10);
    assert!((p[1].evaluate(1.).unwrap().point[0] - expected(0.)).abs() < 1e-10);
}
#[test]
fn unclamped_storage_keeps_all_original_fundamental_spans() {
    let knots = (-3..=6).map(|i| i as f64).collect::<Vec<_>>();
    let p = (0..6)
        .map(|i| {
            let a = knots[i + 1];
            let b = knots[i + 2];
            let c = knots[i + 3];
            vec![a + b + c, a * b + a * c + b * c, 3. * a * b * c]
        })
        .collect();
    let s = Curve {
        degree: 3,
        knots,
        control_points: p,
        weights: vec![1.; 6],
        periodic: false,
    };
    let r = bounded(&s, 1e-10);
    assert_eq!(r.pieces.unwrap().len(), 3);
}
#[test]
fn tolerance_and_final_rounding_work_refuse_atomically() {
    let s = cubic();
    let a = b::torsion(&s, 1e-30, 1_000_000).unwrap();
    assert_eq!(a.refusal, Some(b::Refusal::ToleranceExceeded));
    assert!(a.pieces.is_none() && a.error_upper.is_none());
    let full = b::torsion(&s, 1e-10, 1_000_000).unwrap();
    let a = b::torsion(&s, 1e-10, full.work_used - 1).unwrap();
    assert_eq!(a.refusal, Some(b::Refusal::ResourceLimit));
    assert!(a.pieces.is_none() && a.error_upper.is_none());
}
#[test]
fn inputs_and_resource_ceiling_are_validated() {
    let s = cubic();
    for work in [0, 1_000_001] {
        assert!(e::torsion(&s, work).is_err());
        assert!(b::torsion(&s, 1e-10, work).is_err());
    }
    for t in [0., -1., f64::INFINITY, f64::NAN] {
        assert!(b::torsion(&s, t, 1_000_000).is_err());
    }
    let mut planar = s.clone();
    for p in &mut planar.control_points {
        p.pop();
    }
    assert!(e::torsion(&planar, 1_000_000).is_err());
    assert!(b::torsion(&planar, 1e-10, 1_000_000).is_err());
    let mut bad = s;
    bad.weights[0] = 0.;
    assert!(e::torsion(&bad, 1_000_000).is_err());
}
#[test]
fn original_binormal_proof_degree_is_not_widened() {
    let s = bezier(
        (0..26)
            .map(|i| vec![i as f64, (i % 2) as f64, 0.])
            .collect(),
        vec![1.; 26],
        0.,
        1.,
    );
    assert_eq!(
        e::torsion(&s, 1_000_000).unwrap().refusal,
        Some(e::Refusal::OutputDegreeLimit)
    );
    assert_eq!(
        b::torsion(&s, 1e30, 1_000_000).unwrap().refusal,
        Some(b::Refusal::OutputDegreeLimit)
    );
}
#[test]
fn a_large_tolerance_cannot_relax_output_geometry_limits() {
    let mut s = cubic();
    for p in &mut s.control_points {
        for x in p {
            *x *= 1e-12;
        }
    }
    let r = b::torsion(&s, 1e30, 1_000_000).unwrap();
    assert_eq!(r.refusal, Some(b::Refusal::InvalidOutput));
    assert!(r.pieces.is_none() && r.error_upper.is_none());
}
