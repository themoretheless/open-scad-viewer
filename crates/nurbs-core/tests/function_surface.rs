use nurbs_core::function_surface::{self, Request};
fn request() -> Request {
    Request {
        domain: [[0., 1.], [0., 1.]],
        second_derivative_bounds: [2., 2.],
        tolerance: 0.01,
        max_cells: 256,
    }
}
fn paraboloid(u: f64, v: f64) -> nurbs_core::Result<[[f64; 2]; 3]> {
    let z = u * u + v * v;
    Ok([[u, u], [v, v], [z, z]])
}
#[test]
fn paraboloid_has_continuous_known_interpolation_error() {
    let r = function_surface::approximate(request(), paraboloid).unwrap();
    assert!(r.within_tolerance);
    assert!(r.cells <= 256);
    r.surface.validate().unwrap();
    let n = r.surface.control_points.len() - 1;
    let m = r.surface.control_points[0].len() - 1;
    // Tensor interpolation of u²+v² has exact maximum at cell centers.
    let exact_error = 1. / (4. * (n * n) as f64) + 1. / (4. * (m * m) as f64);
    assert!(r.error_upper >= exact_error);
    for i in 0..n {
        for j in 0..m {
            let u = (i as f64 + 0.5) / n as f64;
            let v = (j as f64 + 0.5) / m as f64;
            let p = r.surface.evaluate(u, v).unwrap().point;
            assert!((p[2] - u * u - v * v - exact_error).abs() < 1e-12);
        }
    }
    assert_eq!(r.evaluations, (n + 1) * (m + 1));
}
#[test]
fn oracle_uncertainty_and_work_limit_never_issue_false_certificates() {
    let limited = Request {
        max_cells: 1,
        ..request()
    };
    let r = function_surface::approximate(limited, paraboloid).unwrap();
    assert!(!r.within_tolerance);
    assert_eq!(r.cells, 1);
    let noisy = function_surface::approximate(request(), |u, v| {
        Ok([[u, u], [v, v], [u * u + v * v - 0.1, u * u + v * v + 0.1]])
    })
    .unwrap();
    assert!(!noisy.within_tolerance);
    assert!(noisy.sample_error_upper >= 0.1);
    let tight = Request {
        tolerance: 1e-20,
        ..request()
    };
    let unresolved = function_surface::approximate(tight, paraboloid).unwrap();
    assert!(!unresolved.within_tolerance);
    assert!(unresolved.surface.control_points.len() <= 32);
}
#[test]
fn invalid_requests_and_oracle_ranges_are_refused() {
    for r in [
        Request {
            max_cells: 0,
            ..request()
        },
        Request {
            tolerance: f64::NAN,
            ..request()
        },
        Request {
            second_derivative_bounds: [-1., 0.],
            ..request()
        },
        Request {
            domain: [[1., 0.], [0., 1.]],
            ..request()
        },
    ] {
        assert!(function_surface::approximate(r, paraboloid).is_err());
    }
    assert!(function_surface::approximate(request(), |_, _| Ok([[1., 0.]; 3])).is_err());
}

#[test]
fn nonpolynomial_sqrt_graph_has_continuous_certificate() {
    let q = Request {
        second_derivative_bounds: [0.25, 0.],
        tolerance: 0.001,
        ..request()
    };
    // All requested u values are dyadic in [0,1], so 1+u is exact here.
    let r = function_surface::approximate(q, |u, v| {
        let z = (1. + u).sqrt();
        Ok([[u, u], [v, v], [z.next_down(), z.next_up()]])
    })
    .unwrap();
    assert!(r.within_tolerance);
    // |d²sqrt(1+u)/du²|<=1/4, independently of the produced NURBS basis.
    for u in [0.17, 0.39, 0.63, 0.91] {
        let p = r.surface.evaluate(u, 0.4).unwrap().point;
        assert!((p[2] - (1. + u).sqrt()).abs() <= r.error_upper);
    }
}

#[test]
fn detailed_outcomes_distinguish_budget_samples_and_combined_bounds() {
    use function_surface::{CertificateOutcome as C, GridStop as G};
    let limited = Request {
        max_cells: 1,
        ..request()
    };
    let r = function_surface::approximate_detailed(limited, paraboloid).unwrap();
    assert_eq!(r.grid_stop, G::CellBudget);
    assert_eq!(r.certificate, C::InterpolationBoundTooLarge);
    r.report.surface.validate().unwrap();
    let noisy = |u: f64, v: f64| Ok([[u, u], [v, v], [u * u + v * v - 0.1, u * u + v * v + 0.1]]);
    let r = function_surface::approximate_detailed(request(), noisy).unwrap();
    assert_eq!(r.grid_stop, G::InterpolationBoundReached);
    assert_eq!(r.certificate, C::SampleBoundTooLarge);
    let r = function_surface::approximate_detailed(limited, noisy).unwrap();
    assert_eq!(r.certificate, C::BothBoundsTooLarge);
    let q = Request {
        second_derivative_bounds: [0.02, 0.02],
        tolerance: 0.01,
        max_cells: 1,
        ..request()
    };
    let r = function_surface::approximate_detailed(q, |u, v| {
        Ok([
            [u, u],
            [v, v],
            [
                0.01 * (u * u + v * v) - 0.006,
                0.01 * (u * u + v * v) + 0.006,
            ],
        ])
    })
    .unwrap();
    assert_eq!(r.grid_stop, G::InterpolationBoundReached);
    assert_eq!(r.certificate, C::CombinedBoundTooLarge);
    assert!(!r.report.within_tolerance);
    assert!(
        function_surface::approximate_detailed(
            Request {
                max_cells: 0,
                ..request()
            },
            paraboloid
        )
        .is_err()
    );
}

#[test]
fn detailed_success_preserves_old_geometry_and_bounds() {
    use function_surface::{CertificateOutcome as C, GridStop as G};
    let old = function_surface::approximate(request(), paraboloid).unwrap();
    let new = function_surface::approximate_detailed(request(), paraboloid).unwrap();
    assert_eq!(new.grid_stop, G::InterpolationBoundReached);
    assert_eq!(new.certificate, C::WithinTolerance);
    assert_eq!(
        old.surface.control_points,
        new.report.surface.control_points
    );
    assert_eq!(old.surface.knots_u, new.report.surface.knots_u);
    assert_eq!(old.surface.knots_v, new.report.surface.knots_v);
    assert_eq!(old.error_upper, new.report.error_upper);
}
