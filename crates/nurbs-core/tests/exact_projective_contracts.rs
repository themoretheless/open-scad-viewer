use nurbs_core::{
    curve::Curve,
    editing::exact_projective::{self as p, Matrix, Refusal},
    surface::Surface,
};
fn identity() -> Matrix {
    [
        [1., 0., 0., 0.],
        [0., 1., 0., 0.],
        [0., 0., 1., 0.],
        [0., 0., 0., 1.],
    ]
}
fn perspective() -> Matrix {
    let mut m = identity();
    m[3][0] = 1.;
    m
}
fn curve() -> Curve {
    Curve {
        degree: 2,
        knots: vec![-2., -2., -2., 1., 4., 4., 4.],
        control_points: vec![
            vec![0., 0., 0.],
            vec![1., 2., 0.],
            vec![1., 0., 2.],
            vec![0., 2., 2.],
        ],
        weights: vec![1., 2., 4., 1.],
        periodic: false,
    }
}
fn surface() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![-2., -2., 3., 3.],
        knots_v: vec![2., 2., 8., 8.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 2., 1.]],
            vec![vec![1., 0., 2.], vec![1., 2., 3.]],
        ],
        weights: vec![vec![1., 2.], vec![4., 1.]],
        periodic_u: false,
        periodic_v: false,
    }
}
#[cfg(feature = "codec")]
fn record(kind: &str, m: Matrix, source: value_codec::Value, out: value_codec::Value) {
    println!(
        "exact-projective-oracle={}",
        value_codec::json!({"kind":kind,"matrix":m,"source":source,"output":out})
    );
}
fn accepted_curve(s: Curve, m: Matrix) -> Curve {
    let original = s.clone();
    let report = p::curve(&s, m, 1000000).unwrap();
    assert!(report.refusal.is_none(), "{:?}", report.refusal);
    let out = report.geometry.unwrap();
    assert_eq!(s, original);
    assert_eq!(out.degree, s.degree);
    assert_eq!(out.knots, s.knots);
    assert_eq!(out.periodic, s.periodic);
    out.validate().unwrap();
    #[cfg(feature = "codec")]
    record("curve", m, value_codec::json!(s), value_codec::json!(out));
    out
}
fn accepted_surface(s: Surface, m: Matrix) -> Surface {
    let original = s.clone();
    let report = p::surface(&s, m, 1000000).unwrap();
    assert!(report.refusal.is_none(), "{:?}", report.refusal);
    let out = report.geometry.unwrap();
    assert_eq!(s, original);
    assert_eq!(out.degree_u, s.degree_u);
    assert_eq!(out.degree_v, s.degree_v);
    assert_eq!(out.knots_u, s.knots_u);
    assert_eq!(out.knots_v, s.knots_v);
    assert_eq!(out.periodic_u, s.periodic_u);
    assert_eq!(out.periodic_v, s.periodic_v);
    out.validate().unwrap();
    #[cfg(feature = "codec")]
    record("surface", m, value_codec::json!(s), value_codec::json!(out));
    out
}
#[test]
fn rational_multispan_perspective_preserves_clock_and_nonuniform_weights() {
    let s = curve();
    let out = accepted_curve(s.clone(), perspective());
    assert_eq!(out.weights, vec![0.125, 0.5, 1., 0.125]);
    for i in 0..=40 {
        let t = -2. + 6. * i as f64 / 40.;
        let a = s.evaluate(t).unwrap().point;
        let b = out.evaluate(t).unwrap().point;
        for k in 0..3 {
            assert!((b[k] - a[k] / (1. + a[0])).abs() < 1e-12);
        }
    }
}
#[test]
fn tensor_projective_uses_one_global_gauge_in_both_directions() {
    let s = surface();
    let out = accepted_surface(s.clone(), perspective());
    assert_eq!(out.weights, vec![vec![0.125, 0.25], vec![1., 0.25]]);
    for i in 0..=8 {
        for j in 0..=8 {
            let u = -2. + 5. * i as f64 / 8.;
            let v = 2. + 6. * j as f64 / 8.;
            let a = s.evaluate(u, v).unwrap().point;
            let b = out.evaluate(u, v).unwrap().point;
            for k in 0..3 {
                assert!((b[k] - a[k] / (1. + a[0])).abs() < 1e-12);
            }
        }
    }
}
#[test]
fn negative_chart_and_negative_global_matrix_gauge_are_supported() {
    let mut m = perspective();
    for row in &mut m {
        for x in row {
            *x = -*x;
        }
    }
    let a = accepted_curve(curve(), m);
    let b = accepted_curve(curve(), perspective());
    assert_eq!(a, b);
    let mut m = identity();
    m[3][3] = -1.;
    accepted_curve(curve(), m);
    accepted_surface(surface(), m);
}
#[test]
fn rank_pivoting_shear_translation_and_inverse_are_exact() {
    let m = [
        [0., 1., 0., 2.],
        [1., 0., 0., -1.],
        [0., 0., 2., 0.],
        [0., 0., 0., 1.],
    ];
    let inverse = [
        [0., 1., 0., 1.],
        [1., 0., 0., -2.],
        [0., 0., 0.5, 0.],
        [0., 0., 0., 1.],
    ];
    let s = curve();
    let out = accepted_curve(s.clone(), m);
    let back = accepted_curve(out, inverse);
    assert_eq!(back.control_points, s.control_points);
    for (a, b) in back.weights.iter().zip(s.weights) {
        assert_eq!(*a, b / 4.);
    }
    accepted_surface(surface(), m);
}
#[test]
fn periodic_aliases_domains_and_maximum_curve_basis_are_preserved() {
    let s = Curve {
        degree: 2,
        knots: vec![-2., -1., 0., 1., 2., 3., 4., 5.],
        control_points: vec![
            vec![0., 0., 0.],
            vec![1., 0., 0.],
            vec![0., 1., 0.],
            vec![0., 0., 0.],
            vec![1., 0., 0.],
        ],
        weights: vec![1., 2., 4., 1., 2.],
        periodic: true,
    };
    accepted_curve(s, perspective());
    let mut s = surface();
    s.knots_v = vec![-1., 0., 1., 2., 3.];
    s.periodic_v = true;
    for (row, weights) in s.control_points.iter_mut().zip(&mut s.weights) {
        row.push(row[0].clone());
        weights.push(weights[0]);
    }
    accepted_surface(s, perspective());
    let s = Curve {
        degree: 25,
        knots: [
            vec![0.; 26],
            (1..=230).map(|i| i as f64).collect(),
            vec![231.; 26],
        ]
        .concat(),
        control_points: (0..256)
            .map(|i| vec![(i % 2) as f64, (i % 4) as f64, 0.])
            .collect(),
        weights: vec![1.; 256],
        periodic: false,
    };
    accepted_curve(s, perspective());
}
#[test]
fn singular_zero_and_mixed_denominators_never_publish_partial_geometry() {
    let mut singular = identity();
    singular[2] = singular[1];
    for m in [
        singular,
        [
            [1., 0., 0., 0.],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 0.],
        ],
    ] {
        assert_eq!(
            p::curve(&curve(), m, 1000000).unwrap().refusal,
            Some(Refusal::SingularMatrix)
        );
        assert!(
            p::surface(&surface(), m, 1000000)
                .unwrap()
                .geometry
                .is_none()
        );
    }
    for shift in [0., -0.5] {
        let mut m = perspective();
        m[3][3] = shift;
        m[0] = [0., 0., 0., 1.];
        let r = p::curve(&curve(), m, 1000000).unwrap();
        assert_eq!(r.refusal, Some(Refusal::UnprovedDenominator));
        assert!(r.geometry.is_none());
        let r = p::surface(&surface(), m, 1000000).unwrap();
        assert_eq!(r.refusal, Some(Refusal::UnprovedDenominator));
        assert!(r.geometry.is_none());
    }
}
#[test]
fn exact_storage_refusal_includes_late_poles_and_output_range() {
    let mut m = perspective();
    m[3][0] = 2.;
    for r in [
        p::curve(&curve(), m, 1000000).unwrap().refusal,
        p::surface(&surface(), m, 1000000).unwrap().refusal,
    ] {
        assert_eq!(r, Some(Refusal::NotRepresentable));
    }
    let mut m = identity();
    m[0][3] = 1e12;
    assert_eq!(
        p::curve(&curve(), m, 1000000).unwrap().refusal,
        Some(Refusal::InvalidOutput)
    );
}
#[test]
fn work_boundary_is_shared_and_atomic_for_construction_storage_and_proof() {
    let s = curve();
    let full = p::curve(&s, perspective(), 1000000).unwrap();
    let n = full.work_used;
    assert!(n > 1);
    assert!(p::curve(&s, perspective(), n).unwrap().geometry.is_some());
    let r = p::curve(&s, perspective(), n - 1).unwrap();
    assert_eq!(r.refusal, Some(Refusal::ResourceLimit));
    assert!(r.geometry.is_none());
    assert!(r.work_used < n);
    let s = surface();
    let full = p::surface(&s, perspective(), 1000000).unwrap();
    let n = full.work_used;
    assert!(p::surface(&s, perspective(), n).unwrap().geometry.is_some());
    let r = p::surface(&s, perspective(), n - 1).unwrap();
    assert_eq!(r.refusal, Some(Refusal::ResourceLimit));
    assert!(r.geometry.is_none());
    assert!(r.work_used < n);
}
#[test]
fn invalid_inputs_and_unbounded_work_are_errors_without_mutation() {
    for n in [0, 1000001, u64::MAX] {
        assert!(p::curve(&curve(), identity(), n).is_err());
        assert!(p::surface(&surface(), identity(), n).is_err());
    }
    for x in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut m = identity();
        m[0][0] = x;
        assert!(p::curve(&curve(), m, 1000000).is_err());
        assert!(p::surface(&surface(), m, 1000000).is_err());
    }
    let mut s = curve();
    s.weights[3] = 0.;
    assert!(p::curve(&s, identity(), 1000000).is_err());
    let mut s = curve();
    for v in &mut s.control_points {
        v.pop();
    }
    assert!(p::curve(&s, identity(), 1000000).is_err());
    let mut s = surface();
    s.control_points[1][1][0] = f64::NAN;
    assert!(p::surface(&s, identity(), 1000000).is_err());
}
