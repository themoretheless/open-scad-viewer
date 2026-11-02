use nurbs_core::{curve::Curve, moving_radius::curves};
fn arc(radius: f64) -> Curve {
    Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![
            vec![radius, 0., 0.],
            vec![radius, radius, 0.],
            vec![0., radius, 0.],
        ],
        weights: vec![1., 0.5f64.sqrt(), 1.],
        periodic: false,
    }
}
#[test]
fn full_original_rational_curves_keep_independent_clocks_and_orientation() {
    let a = arc(3.);
    let mut b = arc(3.5).reverse().unwrap();
    for t in &mut b.knots {
        *t = 10. + 20. * *t;
    }
    let before = (a.clone(), b.clone());
    let q = curves::qualify([&a, &b], true, 0.5, 1e-8, 1000, 1000000)
        .unwrap()
        .certificate
        .unwrap();
    assert!(q.reverse());
    assert_eq!(q.curves(), &[a.clone(), b.clone()]);
    assert!(q.error_upper() <= 1e-8);
    assert!(q.reverify(1000, 1000000).unwrap().certificate.is_some());
    assert!(
        curves::qualify([&a, &b], false, 0.5, 1e-8, 1000, 1000000)
            .unwrap()
            .certificate
            .is_none()
    );
    assert_eq!((a, b), before);
}
#[test]
fn independent_multispan_original_breaks_and_zero_distance_are_not_remapped_proxies() {
    let a = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 0.37, 0.61, 1., 1., 1.],
        control_points: vec![
            vec![0., 0., 0.],
            vec![0.2, 0.7, 0.],
            vec![0.8, -0.2, 0.],
            vec![1.3, 0.5, 0.],
            vec![2., 0., 0.],
        ],
        weights: vec![1., 0.8, 1.3, 0.9, 1.],
        periodic: false,
    };
    let mut b = a.clone();
    for p in &mut b.control_points {
        p[2] += 0.5;
    }
    for t in &mut b.knots {
        *t = 10. + 20. * *t;
    }
    let r = curves::qualify([&a, &b], false, 0.5, 1e-7, 10000, 10000000).unwrap();
    assert!(
        r.certificate.is_some(),
        "{} {:?} cells {} work {}",
        r.reason,
        r.uncertain_t,
        r.cells,
        r.work
    );
    assert!(r.accepted_cells > 1 && r.work <= 10000000);
    let r = curves::qualify([&a, &a], false, 0., 1e-7, 10000, 10000000).unwrap();
    assert!(r.certificate.is_some(), "{} {:?}", r.reason, r.uncertain_t);
}
#[test]
fn matching_end_and_middle_distances_do_not_hide_a_quartic_deviation() {
    let a = Curve {
        degree: 4,
        knots: vec![0., 0., 0., 0., 0., 1., 1., 1., 1., 1.],
        control_points: (0..5).map(|i| vec![i as f64 / 4., 0., 0.]).collect(),
        weights: vec![1.; 5],
        periodic: false,
    };
    let mut b = a.clone();
    for (i, p) in b.control_points.iter_mut().enumerate() {
        p[2] = 1. + [0., 0.125, -1. / 6., 0.125, 0.][i];
    }
    for t in [0., 0.5, 1.] {
        assert!((b.evaluate(t).unwrap().point[2] - 1.).abs() < 1e-12);
    }
    let r = curves::qualify([&a, &b], false, 1., 1e-7, 1000, 1000000).unwrap();
    assert!(r.certificate.is_none());
    assert!(r.witness_t.is_some());
}
#[test]
fn immutable_original_owners_and_exhausted_work_cannot_borrow_a_receipt() {
    let a = arc(3.);
    let mut b = arc(3.5);
    let q = curves::qualify([&a, &b], false, 0.5, 1e-8, 1000, 1000000)
        .unwrap()
        .certificate
        .unwrap();
    b.control_points[1][2] = 0.2;
    assert!(q.reverify(1000, 1000000).unwrap().certificate.is_some());
    assert!(
        curves::qualify([&a, &b], false, 0.5, 1e-8, 1000, 1000000)
            .unwrap()
            .certificate
            .is_none()
    );
    assert!(q.reverify(1, 1).unwrap().certificate.is_none());
    let mut bad = a.clone();
    bad.weights[1] = -0.1;
    assert!(curves::qualify([&bad, &b], false, 0.5, 1e-8, 1000, 1000000).is_err());
}
