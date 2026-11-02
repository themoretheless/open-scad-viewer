use nurbs_core::{curve::Curve, curve_partition_agreement as partition};
fn circle() -> Curve {
    let p = [
        [1., 0.],
        [1., 1.],
        [0., 1.],
        [-1., 1.],
        [-1., 0.],
        [-1., -1.],
        [0., -1.],
        [1., -1.],
        [1., 0.],
        [1., 1.],
    ];
    Curve {
        degree: 2,
        knots: vec![-1., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 5., 5.],
        control_points: p.iter().map(|p| vec![p[0], p[1], 0.]).collect(),
        weights: (0..10)
            .map(|i| {
                if i % 2 == 0 {
                    1.
                } else {
                    std::f64::consts::FRAC_1_SQRT_2
                }
            })
            .collect(),
        periodic: true,
    }
}
#[test]
fn one_original_period_has_shared_clamped_segments_and_an_exact_closing_join() {
    let c = circle();
    let before = c.clone();
    let p = partition::partition(&c, &[0.23, 1.4, 3.7], 1e-7, 100000).unwrap();
    assert!(p.geometry_preserved && p.endpoint_joins_exact);
    assert_eq!(p.periodic_closure_exact, Some(true));
    assert_eq!(p.pieces[0].curve.domain()[0], 0.);
    assert_eq!(p.pieces.last().unwrap().curve.domain()[1], 4.);
    for pair in p.pieces.windows(2) {
        assert_eq!(pair[0].curve.domain()[1], pair[1].curve.domain()[0]);
    }
    for part in &p.pieces {
        assert!(!part.curve.periodic);
        assert!(
            partition::certify(&c, &part.curve, 1e-7, 100000)
                .unwrap()
                .agreement_proven
        );
    }
    assert_eq!(c, before);
}
#[test]
fn approximate_period_storage_never_gains_exact_periodic_closure() {
    let mut c = circle();
    c.knots[0] = (-1f64).next_up();
    c.validate().unwrap();
    let p = partition::partition(&c, &[1., 2., 3.], 1e-7, 100000).unwrap();
    assert_eq!(p.periodic_closure_exact, Some(false));
}
#[test]
fn incomplete_work_cannot_be_confused_with_original_geometry_preservation() {
    let c = circle();
    let p = partition::partition(&c, &[0.23, 1.4, 3.7], 1e-7, 1).unwrap();
    assert!(!p.geometry_preserved);
    assert!(p.visited <= 1);
    assert!(partition::partition(&c, &[4.], 1e-7, 100).is_err());
}
#[test]
fn original_span_uv_partition_keeps_exact_cycle_joins() {
    let mut c = circle();
    for p in &mut c.control_points {
        p.truncate(2);
    }
    let r = partition::partition(&c, &[1., 2., 3.], 1e-8, 100000).unwrap();
    assert!(
        r.geometry_preserved && r.endpoint_joins_exact && r.periodic_closure_exact == Some(true),
        "geometry={}, joins={}, closure={:?}, cells={}",
        r.geometry_preserved,
        r.endpoint_joins_exact,
        r.periodic_closure_exact,
        r.visited
    );
}

#[test]
fn complete_span_identity_cannot_hide_an_authored_interior_pole_change() {
    let c = circle();
    let r = partition::partition(&c, &[1., 2., 3.], 1e-8, 100).unwrap();
    assert!(r.geometry_preserved);
    assert_eq!(r.visited, 4);
    assert!(partition::verify_exact_span_restriction(&c, &r.pieces[0].curve).unwrap());
    assert_eq!(
        partition::exact_bezier_span_partition(&c)
            .unwrap()
            .unwrap()
            .len(),
        4
    );
    let mut bad = r.pieces[0].curve.clone();
    bad.control_points[1][0] += 0.01;
    assert!(!partition::verify_exact_span_restriction(&c, &bad).unwrap());
    let proof = partition::certify(&c, &bad, 1e-8, 1000).unwrap();
    assert!(!proof.agreement_proven);
    assert!(
        proof
            .cells
            .iter()
            .any(|c| c.error_upper.is_some_and(|v| v > 0.))
    );
}

#[test]
fn correlated_original_bounds_accept_independent_weight_gauges_and_degree_changes() {
    let c = circle();
    let p = partition::partition(&c, &[0.23, 1.4, 3.7], 1e-10, 128).unwrap();
    assert!(p.geometry_preserved && p.endpoint_joins_exact);
    for piece in p.pieces {
        let mut candidate = piece.curve.elevate(4).unwrap();
        for w in &mut candidate.weights {
            *w *= 0.125;
        }
        assert!(!partition::verify_exact_span_restriction(&c, &candidate).unwrap());
        assert!(
            partition::certify(&c, &candidate, 1e-10, 128)
                .unwrap()
                .agreement_proven
        );
    }
}
#[test]
fn uniform_cubic_periodic_original_is_qualified_without_bernstein_span_identity() {
    let p = [
        [1., 0.],
        [0.2, 1.],
        [-1., 0.4],
        [-0.7, -0.8],
        [0.4, -1.],
        [1., 0.],
        [0.2, 1.],
        [-1., 0.4],
    ];
    let c = Curve {
        degree: 3,
        knots: (-3..=8).map(|v| v as f64).collect(),
        control_points: p.iter().map(|p| vec![p[0], p[1], 0.]).collect(),
        weights: vec![1., 0.8, 1.2, 0.9, 1.1, 1., 0.8, 1.2],
        periodic: true,
    };
    let before = c.clone();
    c.validate().unwrap();
    assert!(
        partition::exact_bezier_span_partition(&c)
            .unwrap()
            .is_none()
    );
    let r = partition::partition(&c, &[0.3, 2.7, 4.6], 1e-9, 256).unwrap();
    assert!(r.geometry_preserved && r.endpoint_joins_exact);
    assert_eq!(r.periodic_closure_exact, Some(true));
    assert_eq!(c, before);
}
