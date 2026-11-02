use nurbs_core::{curve::Curve, periodic_seam};
fn source() -> Curve {
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
        control_points: p.iter().map(|p| p.to_vec()).collect(),
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
fn arbitrary_rational_phase_and_independent_weight_gauges_have_small_continuous_bounds() {
    let mut c = source();
    for k in &mut c.knots {
        *k = -3. + 2.5 * *k;
    }
    for seam in [-2.731, 0.125, 6.999] {
        let mut candidate = periodic_seam::curve_candidate(&c, seam).unwrap();
        for w in &mut candidate.weights {
            *w *= 3.25;
        }
        let report = periodic_seam::verify_curve(&c, &candidate, 1e-9, 128).unwrap();
        assert!(report.accepted, "{seam}: {report:?}");
        assert!(report.error_upper <= 1e-9);
        assert!(report.cells <= 128);
    }
}
#[test]
fn current_pole_mutation_and_nonlinear_knot_clock_do_not_borrow_a_phase_receipt() {
    let c = source();
    let candidate = periodic_seam::curve_candidate(&c, 0.375).unwrap();
    assert!(
        periodic_seam::verify_curve(&c, &candidate, 1e-10, 128)
            .unwrap()
            .accepted
    );
    let mut bad = candidate.clone();
    bad.control_points[2][0] += 0.01;
    assert!(
        !periodic_seam::verify_curve(&c, &bad, 1e-10, 128)
            .unwrap()
            .accepted
    );
    let d = candidate.domain();
    let mut bad = candidate.trim(d[0], d[1]).unwrap();
    for k in &mut bad.knots {
        if *k == 1. {
            *k = 1.01;
        }
    }
    assert!(
        !periodic_seam::verify_curve(&c, &bad, 1e-10, 128)
            .unwrap()
            .accepted
    );
}
#[test]
fn large_common_origin_does_not_destroy_correlated_phase_evidence() {
    let mut c = source();
    for p in &mut c.control_points {
        p[0] += 1e7;
        p[1] -= 2e7;
    }
    let candidate = periodic_seam::curve_candidate(&c, 0.375).unwrap();
    let report = periodic_seam::verify_curve(&c, &candidate, 1e-6, 128).unwrap();
    assert!(report.accepted, "{report:?}");
    assert!(report.error_upper <= 1e-6);
}
