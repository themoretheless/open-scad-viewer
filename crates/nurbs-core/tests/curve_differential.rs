use nurbs_core::{
    curve::Curve,
    curve_differential::{self as differential, Side, SideUsed, Status},
};
fn contains(b: [f64; 2], v: f64) {
    assert!(b[0] <= v && v <= b[1], "{v} outside {b:?}");
}
fn cubic() -> Curve {
    nurbs_core::paths::bezier(
        vec![
            vec![0., 0., 0.],
            vec![1., 0., 0.],
            vec![2., 1., 0.],
            vec![3., 3., 1.],
        ],
        None,
    )
    .unwrap()
}
#[test]
fn cubic_curvature_and_torsion_follow_independent_exact_polynomial() {
    let c = cubic();
    for t in [0_f64, 0.125, 0.25, 0.5, 0.875, 1.] {
        let r = differential::at(&c, t, Side::Automatic).unwrap();
        let a = t.powi(4) + t * t + 1.;
        let b = 1. + 4. * t * t + t.powi(4);
        contains(r.curvature.unwrap(), (2. / 3.) * a.sqrt() / b.powf(1.5));
        contains(r.torsion.unwrap(), 1. / (3. * a));
        assert_eq!(r.curvature_status, Status::Available);
        assert_eq!(r.torsion_status, Status::Available);
        let expected_binormal = [t * t / a.sqrt(), -t / a.sqrt(), 1. / a.sqrt()];
        let expected_normal = [
            -(t.powi(3) + 2. * t) / (a * b).sqrt(),
            (1. - t.powi(4)) / (a * b).sqrt(),
            (2. * t.powi(3) + t) / (a * b).sqrt(),
        ];
        for (bound, value) in r.binormal.unwrap().into_iter().zip(expected_binormal) {
            contains(bound, value);
        }
        for (bound, value) in r.normal.unwrap().into_iter().zip(expected_normal) {
            contains(bound, value);
        }
        let tangent = [1. / b.sqrt(), 2. * t / b.sqrt(), t * t / b.sqrt()];
        for (bound, value) in r.tangent.unwrap().into_iter().zip(tangent) {
            contains(bound, value);
        }
    }
}
#[test]
fn planar_circle_and_parabola_have_known_curvature_and_zero_torsion() {
    let circle = nurbs_core::primitives::circle([0.; 3], [0., 0., 1.], 4.).unwrap();
    for t in [0_f64, 0.125, 0.25, 0.5, 0.75, 1.] {
        let side = if t == 1. { Side::Left } else { Side::Right };
        let r = differential::at(&circle, t, side).unwrap();
        contains(r.curvature.unwrap(), 0.25);
        contains(r.torsion.unwrap(), 0.);
    }
    let p =
        nurbs_core::paths::bezier(vec![vec![0., 0.], vec![0.5, 0.], vec![1., 1.]], None).unwrap();
    for t in [0_f64, 0.25, 0.5, 1.] {
        let r = differential::at(&p, t, Side::Automatic).unwrap();
        contains(r.curvature.unwrap(), 2. / (1. + 4. * t * t).powf(1.5));
        contains(r.torsion.unwrap(), 0.);
    }
}
#[test]
fn direction_and_domain_changes_preserve_scalar_geometry() {
    let c = cubic();
    let reversed = c.reverse().unwrap();
    let mapped = nurbs_core::foundation::reparameterize_curve_report(&c, [2., 6.], None)
        .unwrap()
        .curve;
    for t in [0_f64, 0.25, 0.5, 0.75, 1.] {
        let a = t.powi(4) + t * t + 1.;
        let b = 1. + 4. * t * t + t.powi(4);
        let k = (2. / 3.) * a.sqrt() / b.powf(1.5);
        let torsion = 1. / (3. * a);
        for r in [
            differential::at(&mapped, 2. + 4. * t, Side::Automatic).unwrap(),
            differential::at(&reversed, 1. - t, Side::Automatic).unwrap(),
        ] {
            contains(r.curvature.unwrap(), k);
            contains(r.torsion.unwrap(), torsion);
        }
    }
}
#[test]
fn straight_and_stationary_curves_do_not_invent_frenet_frames() {
    let line = nurbs_core::primitives::line([0.; 3], [3., 4., 0.]).unwrap();
    let r = differential::at(&line, 0.5, Side::Automatic).unwrap();
    contains(r.curvature.unwrap(), 0.);
    assert_eq!(r.torsion_status, Status::CurvatureNotSeparatedFromZero);
    assert_eq!(r.frame_status, Status::CurvatureNotSeparatedFromZero);
    assert!(r.torsion.is_none() && r.normal.is_none() && r.binormal.is_none());
    let stationary = nurbs_core::paths::bezier(vec![vec![1., 2., 3.]; 3], None).unwrap();
    let r = differential::at(&stationary, 0.5, Side::Automatic).unwrap();
    assert_eq!(r.curvature_status, Status::SpeedNotSeparatedFromZero);
    assert!(r.curvature.is_none() && r.tangent.is_none());
    let cusp =
        nurbs_core::paths::bezier(vec![vec![1., 0.], vec![-1., 0.], vec![1., 0.]], None).unwrap();
    let r = differential::at(&cusp, 0.5, Side::Automatic).unwrap();
    assert_eq!(r.curvature_status, Status::SpeedNotSeparatedFromZero);
}
#[test]
fn c0_corner_requires_explicit_sides() {
    let c =
        Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 0., 0.], vec![1., 1., 0.]]).unwrap();
    let auto = differential::at(&c, 1., Side::Automatic).unwrap();
    assert_eq!(auto.continuity, Some(0));
    assert_eq!(auto.curvature_status, Status::ContinuityNotProven);
    assert!(auto.curvature.is_none() && auto.tangent.is_none());
    let left = differential::at(&c, 1., Side::Left).unwrap();
    let right = differential::at(&c, 1., Side::Right).unwrap();
    assert_eq!(left.side, SideUsed::Left);
    assert_eq!(right.side, SideUsed::Right);
    contains(left.tangent.unwrap()[0], 1.);
    contains(left.tangent.unwrap()[1], 0.);
    contains(right.tangent.unwrap()[0], 0.);
    contains(right.tangent.unwrap()[1], 1.);
    contains(left.curvature.unwrap(), 0.);
    contains(right.curvature.unwrap(), 0.);
}
#[test]
fn invalid_and_outside_domain_sides_are_refused() {
    let c = cubic();
    assert!(differential::at(&c, 0., Side::Left).is_err());
    assert!(differential::at(&c, 1., Side::Right).is_err());
    assert!(differential::at(&c, -1., Side::Automatic).is_err());
    assert!(differential::at(&c, f64::NAN, Side::Automatic).is_err());
    let mut malformed = c;
    malformed.weights.clear();
    assert!(differential::at(&malformed, 0.5, Side::Automatic).is_err());
}

#[test]
fn tiny_geometry_does_not_need_underflowing_speed_cubes() {
    let scale = 1e-110;
    let mut c = cubic();
    for p in &mut c.control_points {
        for coordinate in p {
            *coordinate *= scale;
        }
    }
    let r = differential::at(&c, 0., Side::Automatic).unwrap();
    let curvature = 2. / (3. * scale);
    let torsion = 1. / (3. * scale);
    contains(r.curvature.unwrap(), curvature);
    contains(r.torsion.unwrap(), torsion);
    let k = r.curvature.unwrap();
    assert!((k[1] - k[0]) / curvature < 1e-10);
}

#[test]
fn c2_knot_preserves_curvature_but_requires_a_side_for_third_order() {
    let c = cubic().insert(0.5, 1).unwrap();
    let r = differential::at(&c, 0.5, Side::Automatic).unwrap();
    assert_eq!(r.continuity, Some(2));
    assert_eq!(r.curvature_status, Status::Available);
    assert_eq!(r.torsion_status, Status::ContinuityNotProven);
    assert_eq!(r.frame_status, Status::Available);
    assert!(r.torsion.is_none());
    let exact = 1. / (3. * (0.5_f64.powi(4) + 0.5_f64.powi(2) + 1.));
    for side in [Side::Left, Side::Right] {
        let r = differential::at(&c, 0.5, side).unwrap();
        contains(r.torsion.unwrap(), exact);
    }
}
#[test]
fn periodic_endpoints_are_explicit_one_sided_queries() {
    let c = Curve {
        degree: 2,
        knots: (0..11).map(|i| i as f64).collect(),
        control_points: vec![
            vec![1., 0.],
            vec![0.5, 1.],
            vec![-0.5, 1.],
            vec![-1., 0.],
            vec![-0.5, -1.],
            vec![0.5, -1.],
            vec![1., 0.],
            vec![0.5, 1.],
        ],
        weights: vec![1., 2., 1., 2., 1., 2., 1., 2.],
        periodic: true,
    };
    c.validate().unwrap();
    let a = differential::at(&c, 2., Side::Automatic).unwrap();
    let b = differential::at(&c, 8., Side::Automatic).unwrap();
    assert_eq!(a.side, SideUsed::Right);
    assert_eq!(b.side, SideUsed::Left);
    assert_eq!(a.curvature_status, Status::Available);
    assert_eq!(b.curvature_status, Status::Available);
    contains(a.torsion.unwrap(), 0.);
    contains(b.torsion.unwrap(), 0.);
    assert!(differential::at(&c, 2., Side::Left).is_err());
}
