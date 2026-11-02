use nurbs_core::{
    curve::Curve,
    curve_surface_agreement::{self as agreement, Status},
    surface::Surface,
};
fn curve() -> Curve {
    Curve {
        degree: 2,
        knots: vec![-3., -3., -3., 7., 7., 7.],
        control_points: vec![vec![1., 0., 0.], vec![1., 1., 0.], vec![0., 1., 0.]],
        weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
        periodic: false,
    }
}
fn support(c: &Curve) -> Surface {
    Surface {
        degree_u: c.degree,
        degree_v: 1,
        knots_u: c.knots.clone(),
        knots_v: vec![0., 0., 1., 1.],
        control_points: c
            .control_points
            .iter()
            .map(|p| vec![p.clone(), vec![p[0], p[1], 1.]])
            .collect(),
        weights: c.weights.iter().map(|w| vec![*w, *w]).collect(),
        periodic_u: false,
        periodic_v: false,
    }
}
fn pcurve() -> Curve {
    Curve {
        degree: 1,
        knots: vec![-11., -11., 29., 29.],
        control_points: vec![vec![-3., 0.], vec![7., 0.]],
        weights: vec![1., 1.],
        periodic: false,
    }
}
#[test]
fn literal_original_clock_relations_keep_rational_geometry_and_both_directions() {
    let c = curve();
    let s = support(&c);
    let p = pcurve();
    for (nodes, reverse) in [
        ([[-1., -3.], [5., 21.]], false),
        ([[5., 21.], [-1., -3.]], false),
        ([[-1., 21.], [5., -3.]], true),
    ] {
        let r =
            agreement::verify_parameter_correspondence(&c, &p, &s, &nodes, reverse, 1e-8, 100000)
                .unwrap();
        assert_eq!(r.status, Status::WithinTolerance);
        assert!(r.unresolved_relation.is_empty());
    }
}
#[test]
fn partial_original_relations_do_not_claim_missing_ranges_or_repair_bad_clocks() {
    let c = curve();
    let s = support(&c);
    let p = pcurve();
    let wrong = agreement::verify_parameter_correspondence(
        &c,
        &p,
        &s,
        &[[-1., 21.], [5., -3.]],
        false,
        1e-8,
        10000,
    )
    .unwrap();
    assert_ne!(wrong.status, Status::WithinTolerance);
    let scarce = agreement::verify_parameter_correspondence(
        &c,
        &p,
        &s,
        &[[-1., -3.], [5., 21.]],
        false,
        1e-8,
        1,
    )
    .unwrap();
    assert!(scarce.cells <= 1);
    assert!(
        agreement::verify_parameter_correspondence(
            &c,
            &p,
            &s,
            &[[-4., -3.], [5., 21.]],
            false,
            1e-8,
            100
        )
        .is_err()
    );
}
#[test]
fn reflected_clock_transport_uses_original_rational_derivatives_and_replays() {
    let c = curve();
    let before = c.clone();
    let r =
        agreement::verify_parameter_clocks(&c, &[[-1., 5.], [5., -1.]], [false, true], 1e-8, 10000)
            .unwrap();
    assert_eq!(r.status, Status::WithinTolerance);
    assert!(r.distance_upper.unwrap() <= 1e-8);
    assert_eq!(c, before);
    assert_ne!(
        agreement::verify_parameter_clocks(&c, &[[-1., 5.], [5., -1.]], [false, true], 1e-8, 1)
            .unwrap()
            .status,
        Status::WithinTolerance
    );
}
#[test]
fn discontinuous_original_spans_cannot_borrow_a_zero_derivative_clock_bound() {
    let c = Curve {
        degree: 1,
        knots: vec![0., 0., 0.5, 0.5, 1., 1.],
        control_points: vec![
            vec![0., 0., 0.],
            vec![0., 0., 0.],
            vec![100., 0., 0.],
            vec![100., 0., 0.],
        ],
        weights: vec![1.; 4],
        periodic: false,
    };
    assert!(
        agreement::verify_parameter_clocks(
            &c,
            &[[0.49, 0.51], [0.499, 0.519]],
            [false, false],
            1.,
            100
        )
        .is_err()
    );
    let c = Curve {
        degree: 1,
        knots: vec![0., 0., 0.49, 0.51, 1., 1.],
        control_points: vec![
            vec![0., 0., 0.],
            vec![0., 0., 0.],
            vec![100., 0., 0.],
            vec![100., 0., 0.],
        ],
        weights: vec![1.; 4],
        periodic: false,
    };
    let r = agreement::verify_parameter_clocks(
        &c,
        &[[0.49, 0.51], [0.499, 0.519]],
        [false, false],
        1.,
        100,
    )
    .unwrap();
    assert_ne!(r.status, Status::WithinTolerance);
    let same = agreement::verify_parameter_clocks(
        &c,
        &[[0.49, 0.49], [0.51, 0.51]],
        [false, false],
        1e-8,
        1,
    )
    .unwrap();
    assert_eq!(same.status, Status::WithinTolerance);
}

#[test]
fn conditional_clock_error_bounds_cover_world_displacement_and_refuse_unfinished_work() {
    let line = Curve {
        degree: 1,
        knots: vec![-2., -2., 3., 3.],
        control_points: vec![vec![0., 0., 0.], vec![30., -40., 0.]],
        weights: vec![1., 1.],
        periodic: false,
    };
    let delta = 1e-5;
    let (bound, cells) = agreement::parameter_clock_error_bound(&line, delta, 100).unwrap();
    assert_eq!(cells, 2);
    let upper = bound.unwrap();
    assert!(upper >= 14. * delta && upper < 14. * delta * 1.000001);
    let (missing, cells) = agreement::parameter_clock_error_bound(&line, delta, 1).unwrap();
    assert_eq!(missing, None);
    assert_eq!(cells, 1);
    let c = curve();
    let bound = agreement::parameter_clock_error_bound(&c, 0.01, 100)
        .unwrap()
        .0
        .unwrap();
    for t in [-2., 0., 2., 6.] {
        let a = c.evaluate(t).unwrap().point;
        let b = c.evaluate(t + 0.01).unwrap().point;
        let distance = a
            .iter()
            .zip(b)
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            .sqrt();
        assert!(distance <= bound);
    }
}
#[test]
fn zero_clock_error_keeps_identity_but_invalid_clocks_never_gain_a_bound() {
    let c = curve();
    assert_eq!(
        agreement::parameter_clock_error_bound(&c, 0., 1).unwrap(),
        (Some(0.), 1)
    );
    for error in [-1., f64::NAN, f64::INFINITY] {
        assert!(agreement::parameter_clock_error_bound(&c, error, 100).is_err());
    }
    assert!(agreement::parameter_clock_error_bound(&c, 0.01, 0).is_err());
    let broken = Curve {
        degree: 1,
        knots: vec![0., 0., 0.5, 0.5, 1., 1.],
        control_points: vec![
            vec![0., 0., 0.],
            vec![1., 0., 0.],
            vec![4., 0., 0.],
            vec![5., 0., 0.],
        ],
        weights: vec![1.; 4],
        periodic: false,
    };
    assert!(agreement::parameter_clock_error_bound(&broken, 0.01, 100).is_err());
}
