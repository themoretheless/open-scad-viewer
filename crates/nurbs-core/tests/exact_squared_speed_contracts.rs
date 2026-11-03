use nurbs_core::{
    curve::Curve,
    editing::exact_derivative::{self as d, Refusal},
};
fn bezier(points: Vec<Vec<f64>>, weights: Vec<f64>, a: f64, b: f64) -> Curve {
    let p = points.len() - 1;
    Curve {
        degree: p,
        knots: [vec![a; p + 1], vec![b; p + 1]].concat(),
        control_points: points,
        weights,
        periodic: false,
    }
}
fn rational() -> Curve {
    bezier(vec![vec![0., 0.], vec![2., 0.]], vec![1., 2.], -2., 2.)
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}
fn accepted(s: Curve) -> Vec<Curve> {
    let before = s.clone();
    let r = d::squared_speed(&s, 1000000).unwrap();
    assert!(r.refusal.is_none(), "{:?},work={}", r.refusal, r.work_used);
    let out = r.pieces.unwrap();
    assert_eq!(s, before);
    let spans = (s.degree..s.control_points.len())
        .filter(|&i| s.knots[i] < s.knots[i + 1])
        .collect::<Vec<_>>();
    assert_eq!(out.len(), spans.len());
    for (p, k) in out.iter().zip(spans) {
        p.validate().unwrap();
        assert!(!p.periodic);
        assert_eq!(p.domain(), [s.knots[k], s.knots[k + 1]]);
        assert_eq!(p.control_points[0].len(), s.control_points[0].len());
        assert!(
            p.control_points
                .iter()
                .all(|x| x[1..].iter().all(|&v| v == 0.))
        );
        let [a, b] = p.domain();
        for i in 1..8 {
            let u = a + (b - a) * i as f64 / 8.;
            let velocity = s.evaluate(u).unwrap().d1.unwrap();
            close(
                p.evaluate(u).unwrap().point[0],
                velocity.iter().map(|x| x * x).sum(),
            );
        }
    }
    #[cfg(feature = "codec")]
    println!(
        "exact-squared-speed-oracle={}",
        value_codec::json!({"source":s,"pieces":out})
    );
    out
}
#[test]
fn planar_and_spatial_lines_use_physical_clock_and_all_components() {
    for (end, expected) in [(vec![12., 16.], 25.), (vec![12., 16., 48.], 169.)] {
        let out = accepted(bezier(vec![vec![0.; end.len()], end], vec![1., 1.], 2., 6.));
        assert_eq!(out[0].degree, 1);
        close(out[0].evaluate(4.).unwrap().point[0], expected);
    }
}
#[test]
fn rational_velocity_is_squared_before_any_binary_storage() {
    for end in [vec![2., 0.], vec![2., -4., 6.]] {
        let dim = end.len();
        let factor = if dim == 2 { 1. } else { 14. };
        let out = accepted(bezier(vec![vec![0.; dim], end], vec![1., 2.], -2., 2.));
        for u in [-2., -1., 2.] {
            let t: f64 = (u + 2.) / 4.;
            close(
                out[0].evaluate(u).unwrap().point[0],
                factor / (1. + t).powi(4),
            );
        }
    }
}
#[test]
fn cubic_polynomial_field_matches_closed_form_on_original_clock() {
    let out = accepted(bezier(
        vec![
            vec![0., 0., 1.],
            vec![0., 0., 1.],
            vec![0., 2., 1.],
            vec![6., 6., 1.],
        ],
        vec![1.; 4],
        2.,
        6.,
    ));
    for u in [2., 3., 6.] {
        let t: f64 = (u - 2.) / 4.;
        close(
            out[0].evaluate(u).unwrap().point[0],
            20.25 * t.powi(4) + 9. * t * t,
        );
    }
}
#[test]
fn stationary_points_are_valid_without_a_regularity_claim() {
    let end = accepted(bezier(
        vec![vec![0., 0.], vec![0., 0.], vec![0., 0.], vec![8., 0.]],
        vec![1.; 4],
        -1.,
        1.,
    ));
    close(end[0].evaluate(-1.).unwrap().point[0], 0.);
    close(end[0].evaluate(1.).unwrap().point[0], 144.);
    let interior = accepted(bezier(
        vec![vec![0.25, 0.], vec![-0.25, 0.], vec![0.25, 0.]],
        vec![1.; 3],
        0.,
        1.,
    ));
    close(interior[0].evaluate(0.5).unwrap().point[0], 0.);
    assert!(interior[0].control_points.iter().any(|x| x[0] < 0.));
}
#[test]
fn weighted_constant_curve_produces_minimum_degree_zero_carrier() {
    let p = accepted(bezier(vec![vec![6., 8.]; 3], vec![1., 2., 4.], -2., 2.));
    assert_eq!(p[0].degree, 1);
    assert_eq!(p[0].control_points, vec![vec![0., 0.]; 2]);
}
#[test]
fn nonuniform_rational_spans_keep_original_parameter_units() {
    let s = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 3., 3., 3.],
        control_points: vec![vec![12., 0.], vec![8., 0.], vec![4., 0.], vec![3., 0.]],
        weights: vec![2., 3., 6., 8.],
        periodic: false,
    };
    for p in accepted(s) {
        let [a, b] = p.domain();
        for u in [a, (a + b) / 2., b] {
            close(p.evaluate(u).unwrap().point[0], 144. / (1. + u).powi(4));
        }
    }
}
#[test]
fn periodic_source_spans_preserve_speed_without_gluing_tangent_traces() {
    let s = Curve {
        degree: 1,
        knots: vec![-1., 0., 1., 2., 3.],
        control_points: vec![vec![0., 0.], vec![2., 0.], vec![0., 0.]],
        weights: vec![1., 2., 1.],
        periodic: true,
    };
    let p = accepted(s);
    assert_eq!(p.len(), 2);
    close(p[0].evaluate(0.).unwrap().point[0], 16.);
    close(p[1].evaluate(2.).unwrap().point[0], 16.);
    close(p[0].evaluate(1.).unwrap().point[0], 1.);
    close(p[1].evaluate(1.).unwrap().point[0], 1.);
}
#[test]
fn repeated_knots_keep_distinct_one_sided_scalar_traces() {
    let s = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 2., 2., 2.],
        control_points: vec![
            vec![0., 0., 1.],
            vec![1., 1., 1.],
            vec![2., 0., 1.],
            vec![2., 1., 1.],
            vec![3., 2., 1.],
        ],
        weights: vec![1.; 5],
        periodic: false,
    };
    let p = accepted(s);
    close(p[0].evaluate(1.).unwrap().point[0], 8.);
    close(p[1].evaluate(1.).unwrap().point[0], 4.);
}
#[test]
fn unclamped_curve_has_complete_fundamental_span_cover() {
    let s = Curve {
        degree: 2,
        knots: vec![-2., -1., 0., 1., 2., 3., 4., 5.],
        control_points: vec![
            vec![-0.5, 0.],
            vec![0.5, 0.],
            vec![1.5, 2.],
            vec![2.5, 6.],
            vec![3.5, 12.],
        ],
        weights: vec![1.; 5],
        periodic: false,
    };
    let out = accepted(s);
    assert_eq!(out.len(), 3);
    for p in out {
        let [a, b] = p.domain();
        for u in [a, b] {
            close(p.evaluate(u).unwrap().point[0], 1. + 4. * u * u);
        }
    }
}
#[test]
fn reverse_and_stretched_clocks_obey_even_sign_and_inverse_square_scale() {
    let mut reverse = rational();
    reverse.control_points.reverse();
    reverse.weights.reverse();
    let r = accepted(reverse);
    for u in [-2., 0., 2.] {
        let t: f64 = (-u + 2.) / 4.;
        close(r[0].evaluate(u).unwrap().point[0], 1. / (1. + t).powi(4));
    }
    let mut stretched = rational();
    stretched.knots = vec![-2., -2., 6., 6.];
    let p = accepted(stretched);
    close(p[0].evaluate(-2.).unwrap().point[0], 0.25);
}
#[test]
fn orthogonal_rotation_preserves_the_actual_scalar_carrier() {
    let base = accepted(rational());
    let mut s = rational();
    s.control_points[1] = vec![0., -2.];
    assert_eq!(base, accepted(s));
}
#[test]
fn degree_twenty_five_linear_curve_reduces_to_constant_squared_speed() {
    let p = accepted(bezier(
        (0..26).map(|i| vec![i as f64, 0.]).collect(),
        vec![1.; 26],
        0.,
        2.,
    ));
    assert_eq!(p[0].degree, 1);
    close(p[0].evaluate(1.).unwrap().point[0], 156.25);
}
#[test]
fn late_unrepresentable_span_discards_every_earlier_piece() {
    let s = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 2., 2.],
        control_points: vec![vec![0., 0.], vec![2., 0.], vec![3., 0.]],
        weights: vec![1., 2., 3.],
        periodic: false,
    };
    let first = bezier(
        s.control_points[..2].to_vec(),
        s.weights[..2].to_vec(),
        0.,
        1.,
    );
    assert!(d::squared_speed(&first, 1000000).unwrap().pieces.is_some());
    let before = s.clone();
    let r = d::squared_speed(&s, 1000000).unwrap();
    assert!(r.pieces.is_none());
    assert_eq!(r.refusal, Some(Refusal::NotRepresentable));
    assert_eq!(before, s);
}
#[test]
fn last_proof_operation_uses_the_shared_work_budget() {
    let s = rational();
    let full = d::squared_speed(&s, 1000000).unwrap();
    assert!(full.pieces.is_some());
    let r = d::squared_speed(&s, full.work_used - 1).unwrap();
    assert!(r.pieces.is_none());
    assert_eq!(r.refusal, Some(Refusal::ResourceLimit));
}
#[test]
fn output_degree_and_coordinate_caps_are_unchanged() {
    let mut points = vec![vec![0., 0.]; 8];
    points[7][0] = 1.;
    let mut w = vec![1.; 8];
    w[7] = 2.;
    let r = d::squared_speed(&bezier(points, w, 0., 1.), 1000000).unwrap();
    assert!(r.pieces.is_none());
    assert_eq!(r.refusal, Some(Refusal::OutputDegreeLimit));
    let r = d::squared_speed(
        &bezier(vec![vec![0., 0.], vec![1e9, 0.]], vec![1., 1.], 0., 1.),
        1000000,
    )
    .unwrap();
    assert!(r.pieces.is_none());
    assert_eq!(r.refusal, Some(Refusal::InvalidOutput));
}
#[test]
fn invalid_budgets_and_sources_are_errors() {
    for b in [0, 1000001, u64::MAX] {
        assert!(d::squared_speed(&rational(), b).is_err());
    }
    let mut s = rational();
    s.weights[0] = 0.;
    assert!(d::squared_speed(&s, 1000000).is_err());
    let mut s = rational();
    s.control_points[0][0] = f64::NAN;
    assert!(d::squared_speed(&s, 1000000).is_err());
}
