use nurbs_core::{
    curve::Curve,
    editing::exact_derivative::{self as d, Refusal},
};
fn curve(points: Vec<Vec<f64>>, weights: Vec<f64>, a: f64, b: f64) -> Curve {
    let degree = points.len() - 1;
    Curve {
        degree,
        knots: [vec![a; degree + 1], vec![b; degree + 1]].concat(),
        control_points: points,
        weights,
        periodic: false,
    }
}
fn parabola() -> Curve {
    curve(
        vec![vec![0., 0., 0.], vec![2., 0., 0.], vec![4., 4., 0.]],
        vec![1.; 3],
        2.,
        6.,
    )
}
fn accepted(s: &Curve) -> Vec<Curve> {
    let before = s.clone();
    let r = d::binormal(s, 1_000_000).unwrap();
    assert!(r.refusal.is_none(), "{:?}", r.refusal);
    assert_eq!(*s, before);
    let out = r.pieces.unwrap();
    let spans = (s.degree..s.control_points.len())
        .filter(|&i| s.knots[i] < s.knots[i + 1])
        .collect::<Vec<_>>();
    assert_eq!(out.len(), spans.len());
    for (p, k) in out.iter().zip(spans) {
        p.validate().unwrap();
        assert_eq!(p.domain(), [s.knots[k], s.knots[k + 1]]);
        for i in 1..8 {
            let u = p.domain()[0] + (p.domain()[1] - p.domain()[0]) * i as f64 / 8.;
            let jet = s.evaluate(u).unwrap();
            let a = jet.d1.unwrap();
            let b = jet.d2.unwrap();
            let expected = [
                a[1] * b[2] - a[2] * b[1],
                a[2] * b[0] - a[0] * b[2],
                a[0] * b[1] - a[1] * b[0],
            ];
            for (x, y) in p.evaluate(u).unwrap().point.iter().zip(expected) {
                assert!((x - y).abs() < 1e-8, "{x} != {y}");
            }
        }
    }
    #[cfg(feature = "codec")]
    println!(
        "exact-binormal-oracle={}",
        value_codec::json!({"source":s,"pieces":out})
    );
    out
}
#[test]
fn physical_clock_parabola() {
    let out = accepted(&parabola());
    assert_eq!(out[0].evaluate(4.).unwrap().point, vec![0., 0., 0.5]);
}
#[test]
fn spatial_curve_all_components() {
    accepted(&curve(
        vec![vec![0., 0., 0.], vec![2., 0., 2.], vec![4., 4., 0.]],
        vec![1.; 3],
        0.,
        2.,
    ));
}
#[test]
fn rational_source_not_intermediate_derivative() {
    accepted(&curve(
        vec![vec![0., 0., 0.], vec![0.25, 0., 0.], vec![0.5, 0.25, 0.]],
        vec![1., 2., 4.],
        0.,
        1.,
    ));
}
#[test]
fn straight_rational_curve_has_zero_field() {
    let out = accepted(&curve(
        vec![vec![0., 0., 0.], vec![2., 4., 6.]],
        vec![1., 2.],
        0.,
        1.,
    ));
    assert!(out[0].control_points.iter().flatten().all(|&x| x == 0.));
}
#[test]
fn stationary_point_is_valid() {
    accepted(&curve(
        vec![vec![1., 0., 0.], vec![-1., 0., 0.], vec![1., 0., 0.]],
        vec![1.; 3],
        0.,
        1.,
    ));
}
#[test]
fn repeated_knot_retains_both_traces() {
    let s = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 2., 2., 2.],
        control_points: vec![
            vec![0., 0., 0.],
            vec![1., 0., 0.],
            vec![2., 2., 0.],
            vec![3., 2., 0.],
            vec![4., 0., 0.],
        ],
        weights: vec![1.; 5],
        periodic: false,
    };
    let out = accepted(&s);
    assert_eq!(out.len(), 2);
    assert_ne!(
        out[0].evaluate(1.).unwrap().point,
        out[1].evaluate(1.).unwrap().point
    );
}
#[test]
fn work_refusal_is_atomic() {
    let s = parabola();
    let full = d::binormal(&s, 1_000_000).unwrap();
    let r = d::binormal(&s, full.work_used - 1).unwrap();
    assert!(r.pieces.is_none());
    assert_eq!(r.refusal, Some(Refusal::ResourceLimit));
    assert!(d::binormal(&s, 0).is_err());
    assert!(d::binormal(&s, 1_000_001).is_err());
}
#[test]
fn rejects_planar_storage_and_invalid_source() {
    let mut s = parabola();
    for p in &mut s.control_points {
        p.pop();
    }
    assert!(d::binormal(&s, 1_000_000).is_err());
    s.weights[0] = f64::NAN;
    assert!(d::binormal(&s, 1_000_000).is_err());
}
#[test]
fn reversed_and_stretched_clock_scale_as_inverse_cube() {
    let s = parabola();
    let p = accepted(&s);
    let mut reverse = s.clone();
    reverse.control_points.reverse();
    reverse.weights.reverse();
    let r = accepted(&reverse);
    let mut stretched = s;
    stretched.knots = stretched.knots.iter().map(|x| 2. * x).collect();
    let t = accepted(&stretched);
    for i in 0..=8 {
        let u = 2. + i as f64 / 2.;
        let a = p[0].evaluate(u).unwrap().point;
        let b = r[0].evaluate(8. - u).unwrap().point;
        let c = t[0].evaluate(2. * u).unwrap().point;
        for j in 0..3 {
            assert!((a[j] + b[j]).abs() < 1e-10);
            assert!((a[j] - 8. * c[j]).abs() < 1e-10);
        }
    }
}
#[test]
fn unclamped_three_span_cover() {
    let s = Curve {
        degree: 2,
        knots: vec![-2., -1., 0., 1., 2., 3., 4., 5.],
        control_points: vec![
            vec![-0.5, 0., 0.],
            vec![0.5, 0., 0.],
            vec![1.5, 2., 0.],
            vec![2.5, 6., 0.],
            vec![3.5, 12., 0.],
        ],
        weights: vec![1.; 5],
        periodic: false,
    };
    assert_eq!(accepted(&s).len(), 3);
}
#[test]
fn periodic_source_not_glued_into_a_unit_frame() {
    let s = Curve {
        degree: 1,
        knots: vec![-1., 0., 1., 2., 3.],
        control_points: vec![vec![0., 0., 0.], vec![2., 4., 0.], vec![0., 0., 0.]],
        weights: vec![1., 2., 1.],
        periodic: true,
    };
    assert_eq!(accepted(&s).len(), 2);
}

#[test]
fn inexact_storage_refuses_without_partial_output() {
    let s = curve(
        vec![vec![0., 0., 0.], vec![2., 0., 0.], vec![2., 2., 0.]],
        vec![1., 2., 1.],
        0.,
        1.,
    );
    let r = d::binormal(&s, 1_000_000).unwrap();
    assert_eq!(r.refusal, Some(Refusal::NotRepresentable));
    assert!(r.pieces.is_none());
}
#[test]
fn late_inexact_span_discards_the_earlier_exact_span() {
    let s = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 2., 2., 2.],
        control_points: vec![
            vec![-2., 0., 0.],
            vec![-1., 0., 0.],
            vec![0., 0., 0.],
            vec![2., 0., 0.],
            vec![2., 2., 0.],
        ],
        weights: vec![1., 1., 1., 2., 1.],
        periodic: false,
    };
    let r = d::binormal(&s, 1_000_000).unwrap();
    assert_eq!(r.refusal, Some(Refusal::NotRepresentable));
    assert!(r.pieces.is_none());
}
