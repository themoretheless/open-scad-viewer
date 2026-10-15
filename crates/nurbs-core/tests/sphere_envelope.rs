use nurbs_core::{
    curve::Curve,
    primitives,
    sphere_envelope::{self as envelope, Limits},
};
fn law(a: f64, b: f64) -> Curve {
    Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![a, 0., 0.], vec![b, 0., 0.]],
        weights: vec![1., 1.],
        periodic: false,
    }
}

fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 2e-11, "{a} != {b}");
}
#[test]
fn moving_sphere_equations_include_the_radius_derivative() {
    let path = primitives::line([0.; 3], [0., 0., 4.]).unwrap();
    let r = law(1., 2.);
    for u in [0., 0.2, 0.7, 1.] {
        let c = envelope::characteristic_circle(&path, &r, [1., 0., 0.], u).unwrap();
        close(c.radius_slope, 0.25);
        close(c.center[2], 4. * u - (1. + u) * 0.25);
        for angle in [0., 0.3, 1., 2.1, 4.7] {
            let p = c.point(angle).unwrap();
            let d: [f64; 3] = std::array::from_fn(|i| p[i] - c.sphere_center[i]);
            close(d.iter().map(|x| x * x).sum(), (1. + u).powi(2));
            close(d[2], -(1. + u) * 0.25);
        }
    }
    let report = envelope::approximate(
        &path,
        &r,
        [1., 0., 0.],
        Limits {
            sampled_tolerance: 1e-12,
            max_patches: 8,
        },
    )
    .unwrap();
    assert!(report.within_sampled_tolerance);
    assert_eq!(report.patches.len(), 1);
    assert!(!report.continuous_bound && !report.rounding_certified);
    let s = &report.patches[0].surface;
    for u in [0., 0.13, 0.61, 1.] {
        for v in [0., 0.11, 0.37, 0.79, 1.] {
            let p = s.evaluate(u, v).unwrap().point;
            let radius = 1. + u;
            close(p[2], 4. * u - radius * 0.25);
            close(
                p[0] * p[0] + p[1] * p[1],
                radius * radius * (1. - 0.25_f64.powi(2)),
            );
        }
    }
}
#[test]
fn independently_normalized_domains_preserve_characteristic_circles() {
    let path = primitives::line([1., 2., 3.], [1., 2., 7.]).unwrap();
    let radius = law(1., 2.);
    let mut p = path.clone();
    p.knots = vec![-3., -3., 5., 5.];
    let mut r = radius.clone();
    r.knots = vec![10., 10., 12., 12.];
    for u in [0., 0.3, 1.] {
        let a = envelope::characteristic_circle(&path, &radius, [1., 1., 0.], u).unwrap();
        let b = envelope::characteristic_circle(&p, &r, [1., 1., 0.], u).unwrap();
        close(a.radius, b.radius);
        close(a.radius_slope, b.radius_slope);
        for i in 0..3 {
            close(a.center[i], b.center[i]);
        }
    }
    assert_eq!(path.control_points, p.control_points);
}
#[test]
fn curved_centerline_refines_covers_every_interval_and_reuses_seams() {
    let path = primitives::circle([0.; 3], [0., 0., 1.], 5.).unwrap();
    let r = law(0.5, 0.5);
    let domain = envelope::inspect_domain(&path, &r, [0., 0., 1.], 4096).unwrap();
    assert!(domain.spanwise_characteristic_circles_proven && domain.angular_chart_proven);
    assert!(domain.unresolved.is_empty());
    assert!(domain.cells <= 4096);
    let short_audit = envelope::approximate_with_domain_audit(
        &path,
        &r,
        [0., 0., 1.],
        Limits {
            sampled_tolerance: 0.03,
            max_patches: 256,
        },
        1,
    )
    .unwrap();
    assert!(short_audit.approximation.is_none());
    assert!(!short_audit.domain.unresolved.is_empty());
    let coarse = envelope::approximate(
        &path,
        &r,
        [0., 0., 1.],
        Limits {
            sampled_tolerance: 0.03,
            max_patches: 4,
        },
    )
    .unwrap();
    assert!(!coarse.within_sampled_tolerance);
    assert_eq!(coarse.patches.len(), 4);
    let fit = envelope::approximate(
        &path,
        &r,
        [0., 0., 1.],
        Limits {
            sampled_tolerance: 0.03,
            max_patches: 256,
        },
    )
    .unwrap();
    assert!(fit.within_sampled_tolerance);
    assert!(fit.patches.len() > 4);
    assert_eq!(fit.patches.first().unwrap().path_interval[0], 0.);
    assert_eq!(fit.patches.last().unwrap().path_interval[1], 1.);
    for pair in fit.patches.windows(2) {
        assert_eq!(pair[0].path_interval[1], pair[1].path_interval[0]);
        assert_eq!(
            pair[0].surface.control_points[1],
            pair[1].surface.control_points[0]
        );
    }
    // Independent implicit equation checks the actual fitted patches,
    // including path stations absent from the refinement diagnostic.
    for patch in &fit.patches {
        let [a, b] = patch.path_interval;
        for q in [0., 0.17, 0.5, 0.83, 1.] {
            let u = (1. - q) * a + q * b;
            for v in [0., 0.11, 0.37, 0.79, 1.] {
                let p = patch.surface.evaluate(u, v).unwrap().point;
                let torus_distance = ((p[0].hypot(p[1]) - 5.).powi(2) + p[2] * p[2]).sqrt();
                assert!((torus_distance - 0.5).abs() <= 0.03 + 1e-10);
            }
        }
    }
    for u in [0.13, 0.37, 0.61, 0.89] {
        let c = envelope::characteristic_circle(&path, &r, [0., 0., 1.], u).unwrap();
        for angle in [0., 0.7, 1.9, 4.3] {
            let p = c.point(angle).unwrap();
            close((p[0].hypot(p[1]) - 5.).powi(2) + p[2] * p[2], 0.25);
        }
    }
}
#[test]
fn invalid_and_singular_envelopes_are_refused() {
    let p = primitives::line([0.; 3], [0., 0., 1.]).unwrap();
    for r in [law(1., 2.), law(2., 1.), law(1., 3.), law(0., 1.)] {
        assert!(envelope::characteristic_circle(&p, &r, [1., 0., 0.], 0.5).is_err());
    }
    for n in [[0.; 3], [0., 0., 1.], [f64::NAN, 0., 0.]] {
        assert!(envelope::characteristic_circle(&p, &law(1., 1.), n, 0.5).is_err());
    }
    for u in [-1., 2., f64::NAN] {
        assert!(envelope::characteristic_circle(&p, &law(1., 1.), [1., 0., 0.], u).is_err());
    }
    for limits in [
        Limits {
            sampled_tolerance: 0.,
            max_patches: 4,
        },
        Limits {
            sampled_tolerance: 0.1,
            max_patches: 0,
        },
        Limits {
            sampled_tolerance: 0.1,
            max_patches: 257,
        },
    ] {
        assert!(envelope::approximate(&p, &law(1., 1.), [1., 0., 0.], limits).is_err());
    }
}

#[test]
fn mandatory_source_spans_reject_hidden_corners_and_insufficient_initial_budget() {
    let path = Curve {
        degree: 1,
        knots: vec![0., 0., 0.3, 1., 1.],
        control_points: vec![vec![0., 0., 0.], vec![0., 0., 1.], vec![1., 0., 2.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let radius = law(0.2, 0.2);
    assert!(
        envelope::approximate(
            &path,
            &radius,
            [0., 1., 0.],
            Limits {
                sampled_tolerance: 100.,
                max_patches: 16
            }
        )
        .is_err()
    );
    let path = primitives::circle([0.; 3], [0., 0., 1.], 5.).unwrap();
    let error = envelope::approximate(
        &path,
        &radius,
        [0., 0., 1.],
        Limits {
            sampled_tolerance: 1.,
            max_patches: 3,
        },
    )
    .unwrap_err();
    assert_eq!(error.code, "NURBS_RESOURCE_LIMIT");
}

#[test]
fn rational_radius_and_negative_slope_satisfy_the_envelope_in_oblique_frames() {
    let path = primitives::line([1., 2., 3.], [4., 6., 3.]).unwrap();
    for (a, b) in [(1., 2.), (2., 1.)] {
        let mut radius = law(a, b);
        radius.weights = vec![1., 2.];
        for u in [0., 0.17, 0.63, 1.] {
            let c = envelope::characteristic_circle(&path, &radius, [0., 0., 1.], u).unwrap();
            let r = (a * (1. - u) + 2. * b * u) / (1. + u);
            let slope = 2. * (b - a) / (5. * (1. + u).powi(2));
            close(c.sphere_radius, r);
            close(c.radius_slope, slope);
            for angle in [0.1, 1.3, 3.7] {
                let p = c.point(angle).unwrap();
                let d: [f64; 3] = std::array::from_fn(|k| p[k] - c.sphere_center[k]);
                close(d.iter().map(|x| x * x).sum(), r * r);
                close(0.6 * d[0] + 0.8 * d[1], -r * slope);
            }
        }
    }
}

#[test]
fn interval_domain_audit_proves_full_feasibility_and_retains_unknown_cells() {
    let path = primitives::line([0.; 3], [0., 0., 4.]).unwrap();
    let report = envelope::inspect_domain(&path, &law(1., 2.), [1., 0., 0.], 1).unwrap();
    assert!(report.spanwise_characteristic_circles_proven && report.angular_chart_proven);
    assert!(report.unresolved.is_empty());
    assert_eq!(report.cells, 1);
    let mut law_r = law(1., 2.);
    law_r.knots = vec![5., 5., 7., 7.];
    let mut path_r = path.clone();
    path_r.knots = vec![-3., -3., 5., 5.];
    let report = envelope::inspect_domain(&path_r, &law_r, [1., 0., 0.], 1).unwrap();
    assert!(report.spanwise_characteristic_circles_proven && report.angular_chart_proven);
    let singular = primitives::line([0.; 3], [0., 0., 1.]).unwrap();
    let report = envelope::inspect_domain(&singular, &law(1., 2.), [1., 0., 0.], 1).unwrap();
    assert!(!report.spanwise_characteristic_circles_proven);
    assert_eq!(report.unresolved, vec![[0., 1.]]);
    let report = envelope::inspect_domain(&path, &law(1., 1.), [0., 0., 1.], 1).unwrap();
    assert!(report.spanwise_characteristic_circles_proven);
    assert!(!report.angular_chart_proven);
    assert!(!report.unresolved.is_empty());
    for budget in [0, 100001] {
        assert!(envelope::inspect_domain(&path, &law(1., 1.), [1., 0., 0.], budget).is_err());
    }
}

#[test]
fn audited_construction_never_fills_in_an_unproved_domain() {
    let path = primitives::line([0.; 3], [0., 0., 4.]).unwrap();
    let limits = Limits {
        sampled_tolerance: 1e-10,
        max_patches: 8,
    };
    let built =
        envelope::approximate_with_domain_audit(&path, &law(1., 2.), [1., 0., 0.], limits, 1)
            .unwrap();
    assert!(built.domain.unresolved.is_empty());
    assert!(built.approximation.unwrap().within_sampled_tolerance);
    let unproved =
        envelope::approximate_with_domain_audit(&path, &law(1., 1.), [0., 0., 1.], limits, 1)
            .unwrap();
    assert!(!unproved.domain.angular_chart_proven);
    assert!(unproved.approximation.is_none());
}

#[test]
fn nonplanar_polynomial_path_with_varying_radius_has_audited_nurbs_sections() {
    let path = nurbs_core::paths::bezier(
        vec![
            vec![0., 0., 0.],
            vec![1. / 3., 0., 0.],
            vec![2. / 3., 1. / 3., 0.],
            vec![1., 1., 1.],
        ],
        None,
    )
    .unwrap();
    let r = law(0.1, 0.15);
    let built = envelope::approximate_with_domain_audit(
        &path,
        &r,
        [0., 0., 1.],
        Limits {
            sampled_tolerance: 0.01,
            max_patches: 128,
        },
        256,
    )
    .unwrap();
    assert!(
        built.domain.spanwise_characteristic_circles_proven && built.domain.angular_chart_proven
    );
    let fit = built.approximation.unwrap();
    assert!(fit.within_sampled_tolerance);
    for patch in &fit.patches {
        for u in patch.path_interval {
            let center = [u, u * u, u * u * u];
            let velocity = [1., 2. * u, 3. * u * u];
            let radius = 0.1 + 0.05 * u;
            for v in [0., 0.13, 0.41, 0.79, 1.] {
                let p = patch.surface.evaluate(u, v).unwrap().point;
                let d: [f64; 3] = std::array::from_fn(|k| p[k] - center[k]);
                close(d.iter().map(|x| x * x).sum(), radius * radius);
                close((0..3).map(|k| d[k] * velocity[k]).sum(), -radius * 0.05);
            }
        }
    }
}
