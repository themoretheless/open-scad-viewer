use nurbs_core::worm_wheel::{
    ChartDomain, Flank, Spec, certify_envelope_grid, enclose_chart, fit_chart,
};
#[test]
fn quintic_grid_matches_the_original_envelope_continuously_and_has_exact_saved_seams() {
    let domain = ChartDomain {
        u: [0., 1.],
        beta: [-0.15, 0.15],
    };
    for flank in [Flank::Descending, Flank::Ascending] {
        let owned = fit_chart(Spec::default(), flank, domain, 1e-3, 128).unwrap();
        owned.reverify().unwrap();
        assert!(owned.certificate().error_upper <= 1e-3);
        assert!(owned.certificate().source_regular && owned.certificate().saved_regular);
        for s in owned.surfaces() {
            assert_eq!(s.degree_u, 5);
            assert_eq!(s.degree_v, 5);
            let ua = s.knots_u[5];
            let ub = s.knots_u[6];
            let ba = s.knots_v[5];
            let bb = s.knots_v[6];
            for i in 0..=4 {
                for j in 0..=4 {
                    let u = if i == 4 {
                        ub
                    } else {
                        ua + (ub - ua) * i as f64 / 4.
                    };
                    let b = if j == 4 {
                        bb
                    } else {
                        ba + (bb - ba) * j as f64 / 4.
                    };
                    let q = enclose_chart(
                        Spec::default(),
                        flank,
                        ChartDomain {
                            u: [u, u],
                            beta: [b, b],
                        },
                    )
                    .unwrap()
                    .point();
                    let p = s.evaluate(u, b).unwrap().point;
                    let error = (0..3)
                        .map(|d| (p[d] - (q[d][0] + q[d][1]) / 2.).powi(2))
                        .sum::<f64>()
                        .sqrt();
                    assert!(error <= owned.certificate().error_upper + 1e-9, "{error}");
                }
            }
        }
    }
}
#[test]
fn actual_controls_original_spec_and_basis_are_independently_requalified() {
    let d = ChartDomain {
        u: [0., 1.],
        beta: [-0.1, 0.1],
    };
    let owned = fit_chart(Spec::default(), Flank::Descending, d, 1e-3, 128).unwrap();
    let mut changed = owned.surfaces().to_vec();
    changed[0].control_points[2][2][0] += 0.01;
    let report = certify_envelope_grid(
        Spec::default(),
        Flank::Descending,
        d,
        owned.u_breaks(),
        owned.beta_breaks(),
        &changed,
        128,
    )
    .unwrap();
    assert!(report.control_difference_upper > 0.009);
    changed = owned.surfaces().to_vec();
    changed[0].weights[0][0] = 2.;
    assert!(
        certify_envelope_grid(
            Spec::default(),
            Flank::Descending,
            d,
            owned.u_breaks(),
            owned.beta_breaks(),
            &changed,
            128
        )
        .is_err()
    );
    let report = certify_envelope_grid(
        Spec {
            axial_shift: 0.1,
            ..Spec::default()
        },
        Flank::Descending,
        d,
        owned.u_breaks(),
        owned.beta_breaks(),
        owned.surfaces(),
        128,
    )
    .unwrap();
    assert!(report.error_upper > 0.05);
    assert!(fit_chart(Spec::default(), Flank::Descending, d, 1e-30, 1).is_err());
}

#[test]
fn lossless_multi_span_packing_retains_actual_original_definitions_and_positions() {
    use nurbs_core::worm_wheel::verify_packed_grid;
    let domain = ChartDomain {
        u: [0., 1.],
        beta: [-0.15, 0.15],
    };
    let owned = fit_chart(Spec::default(), Flank::Descending, domain, 1e-4, 256).unwrap();
    verify_packed_grid(&owned, owned.packed_surfaces()).unwrap();
    eprintln!(
        "quintic cells={} packed={} error_upper={}",
        owned.surfaces().len(),
        owned.packed_surfaces().len(),
        owned.certificate().error_upper
    );
    assert!(owned.packed_surfaces().len() <= owned.surfaces().len());
    for s in owned.surfaces() {
        let ua = s.knots_u[5];
        let ub = s.knots_u[6];
        let ba = s.knots_v[5];
        let bb = s.knots_v[6];
        for (u, b) in [(ua, ba), ((ua + ub) / 2., (ba + bb) / 2.), (ub, bb)] {
            let packed = owned
                .packed_surfaces()
                .iter()
                .find(|p| {
                    p.knots_u[5] <= u
                        && u <= p.knots_u[p.control_points.len()]
                        && p.knots_v[5] <= b
                        && b <= p.knots_v[p.control_points[0].len()]
                })
                .unwrap();
            let a = s.evaluate(u, b).unwrap().point;
            let q = packed.evaluate(u, b).unwrap().point;
            assert!((a[0] - q[0]).hypot(a[1] - q[1]).hypot(a[2] - q[2]) < 1e-10);
        }
    }
    let mut changed = owned.packed_surfaces().to_vec();
    changed[0].control_points[1][1][0] += 1e-9;
    assert!(verify_packed_grid(&owned, &changed).is_err());
}
