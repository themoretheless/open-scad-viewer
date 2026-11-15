use nurbs_core::{
    curve::Curve,
    editing::exact_projective::{Refusal, mobius_spline_curve, mobius_spline_surface},
    surface::Surface,
};

const WORK: u64 = 1_000_000;

fn source_curve() -> Curve {
    Curve {
        degree: 2,
        knots: vec![0., 0., 0., 2., 2., 3., 3., 3.],
        control_points: vec![
            vec![0., 0.],
            vec![1., 1.],
            vec![2., 0.],
            vec![3., 1.],
            vec![4., 0.],
        ],
        weights: vec![1., 2., 1., 1., 1.],
        periodic: false,
    }
}

fn clock(t: f64, lambda: f64) -> f64 {
    lambda * t / (1. + (lambda - 1.) * t / 3.)
}

#[test]
fn multiple_spans_preserve_rational_geometry_and_repeat_the_interior_knot() {
    let source = source_curve();
    let r = mobius_spline_curve(&source, 2., WORK).unwrap();
    let out = r.geometry.unwrap_or_else(|| panic!("{:?}", r.refusal));
    assert_eq!(out.control_points, source.control_points);
    assert_eq!(out.knots, [0., 0., 0., 1.5, 1.5, 3., 3., 3.]);
    assert_eq!(
        out.weights
            .iter()
            .map(|w| w / out.weights[0])
            .collect::<Vec<_>>(),
        [1., 3., 2.25, 3., 4.]
    );
    for t in [0., 0.25, 1., 1.5, 1.75, 2.75, 3.] {
        let a = out.evaluate(t).unwrap().point;
        let b = source.evaluate(clock(t, 2.)).unwrap().point;
        assert!(a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-14));
    }
    let inverse = mobius_spline_curve(&out, 0.5, WORK)
        .unwrap()
        .geometry
        .unwrap();
    assert_eq!(inverse.knots, source.knots);
    assert_eq!(
        inverse
            .weights
            .iter()
            .map(|w| w / inverse.weights[0])
            .collect::<Vec<_>>(),
        source.weights
    );
}

#[test]
fn tensor_preserves_two_multispan_clocks_and_one_global_weight_ratio() {
    let c = source_curve();
    let source = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: c.knots,
        knots_v: vec![0., 0., 1., 3., 3.],
        control_points: c
            .control_points
            .iter()
            .map(|p| {
                [0., 1., 3.]
                    .map(|v| vec![p[0], v, p[1] * (1. + v)])
                    .to_vec()
            })
            .collect(),
        weights: c.weights.iter().map(|w| vec![*w; 3]).collect(),
        periodic_u: false,
        periodic_v: false,
    };
    let r = mobius_spline_surface(&source, [2., 0.5], WORK).unwrap();
    let out = r.geometry.unwrap_or_else(|| panic!("{:?}", r.refusal));
    assert_eq!(out.knots_u, [0., 0., 0., 1.5, 1.5, 3., 3., 3.]);
    assert_eq!(out.knots_v, [0., 0., 1.5, 3., 3.]);
    assert_eq!(out.weights[2][1] / out.weights[0][0], 2.25 * 0.75);
    assert_eq!(out.control_points, source.control_points);
    for (u, v) in [(0., 0.), (0.75, 1.75), (1.5, 1.5), (2.5, 0.5), (3., 3.)] {
        let a = out.evaluate(u, v).unwrap().point;
        let b = source.evaluate(clock(u, 2.), clock(v, 0.5)).unwrap().point;
        assert!(a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-14));
    }
}

#[test]
fn nonbinary_inverse_knot_and_insufficient_work_are_refused_atomically() {
    let mut s = source_curve();
    s.knots = vec![0., 0., 0., 0.5, 0.5, 1., 1., 1.];
    let r = mobius_spline_curve(&s, 2., WORK).unwrap();
    assert_eq!(r.refusal, Some(Refusal::NotRepresentable));
    assert!(r.geometry.is_none());
    let short = mobius_spline_curve(&source_curve(), 2., 1).unwrap();
    assert_eq!(short.refusal, Some(Refusal::ResourceLimit));
    assert!(short.geometry.is_none());
}

#[test]
fn landmark_fitting_keeps_a_nonbinary_factor_and_exactly_moves_the_break() {
    use nurbs_core::editing::exact_projective::{MobiusLandmark, fit_mobius_spline_curve};
    let curve = Curve {
        degree: 1,
        knots: vec![0., 0., 2., 5., 5.],
        control_points: vec![vec![0., 0.], vec![2., 1.], vec![5., 1.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let correspondence = MobiusLandmark {
        original: 2.,
        new: 2.5,
    };
    let report = fit_mobius_spline_curve(&curve, correspondence, WORK).unwrap();
    let out = report
        .geometry
        .unwrap_or_else(|| panic!("{:?}", report.refusal));
    assert_eq!(out.knots, [0., 0., 2.5, 5., 5.]);
    assert_eq!(out.control_points, curve.control_points);
    for new in [0., 1., 2.5, 4., 5.] {
        let old = 10. * new / (15. - new);
        let a = curve.evaluate(old).unwrap().point;
        let b = out.evaluate(new).unwrap().point;
        assert!(a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-14));
    }
}

#[test]
fn fits_two_independent_landmarks_and_refuses_invalid_or_short_requests() {
    use nurbs_core::editing::exact_projective::{MobiusLandmark, fit_mobius_spline_surface};
    let source = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 5., 5.],
        knots_v: vec![0., 0., 5., 5.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 5., 0.]],
            vec![vec![5., 0., 0.], vec![5., 5., 1.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let landmarks = [
        MobiusLandmark {
            original: 2.,
            new: 2.5,
        },
        MobiusLandmark {
            original: 4.,
            new: 2.5,
        },
    ];
    let r = fit_mobius_spline_surface(&source, landmarks, WORK).unwrap();
    let s = r.geometry.unwrap();
    assert_eq!(s.control_points, source.control_points);
    let actual = s.evaluate(2.5, 2.5).unwrap().point;
    let expected = source.evaluate(2., 4.).unwrap().point;
    assert!(
        actual
            .iter()
            .zip(expected)
            .all(|(x, y)| (x - y).abs() < 1e-14)
    );
    let short = fit_mobius_spline_surface(&source, landmarks, r.work_used - 1).unwrap();
    assert_eq!(short.refusal, Some(Refusal::ResourceLimit));
    assert!(short.geometry.is_none());
    let mut invalid = landmarks;
    invalid[0].new = 5.;
    assert!(fit_mobius_spline_surface(&source, invalid, WORK).is_err());
}
