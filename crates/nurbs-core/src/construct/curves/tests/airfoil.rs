use super::*;

#[test]
fn naca0012_symmetric_with_twelve_percent_thickness_at_thirty_chord() {
    let mean = MeanLine::FourDigit {
        camber: 0.,
        position: 0.,
    };
    let points = naca_points(&|x| mean.at(x), 0.12, 1., true, 200).unwrap();
    // Loop: 2*count+1 points; upper index i pairs with lower index
    // (2*count - i) at the same x station.
    let count = 200;
    for i in 0..=count {
        let upper = points[i];
        let lower = points[2 * count - i];
        assert!((upper[0] - lower[0]).abs() < 1e-12, "x stations must pair");
        assert!(
            (upper[1] + lower[1]).abs() < 1e-12,
            "NACA0012 must be symmetric: yu={} yl={}",
            upper[1],
            lower[1]
        );
    }
    // Max half-thickness 6% of chord at x = 0.3.
    let mut max_half = 0_f64;
    let mut argmax = 0_f64;
    for i in 0..=count {
        let x = (i as f64 / count as f64).powi(2);
        let half = naca4_half_thickness(x, 0.12, true);
        if half > max_half {
            max_half = half;
            argmax = x;
        }
    }
    assert!(
        (2. * max_half - 0.12).abs() < 1e-3,
        "max thickness must be 12% of chord, got {}",
        2. * max_half
    );
    assert!(
        (argmax - 0.3).abs() < 0.02,
        "max thickness at x~0.3c, got {argmax}"
    );
    // Closed trailing edge.
    assert!(naca4_half_thickness(1., 0.12, true).abs() < 1e-15);
    assert!(naca4_half_thickness(1., 0.12, false).abs() > 1e-4);
}

#[test]
fn naca2412_camber_positive_and_curve_fits() {
    for i in 1..100 {
        let x = i as f64 / 100.;
        let (yc, _) = naca4_camber(x, 0.02, 0.4);
        assert!(yc > 0., "NACA2412 camber must be positive at x={x}");
    }
    // Camber peaks at x = 0.4.
    let (peak, slope) = naca4_camber(0.4, 0.02, 0.4);
    assert!((peak - 0.02).abs() < 1e-12);
    assert!(slope.abs() < 0.06);
    let curve = naca4(2412, 1., true, 120, 3, 24).unwrap();
    curve.validate().unwrap();
    // Upper surface near mid-chord lies above the chord line.
    let mut saw_upper = false;
    for i in 0..=200 {
        let u = i as f64 / 200.;
        let p = curve.evaluate(u).unwrap().point;
        if (0.3..0.5).contains(&p[0]) && p[1] > 0.03 {
            saw_upper = true;
        }
    }
    assert!(saw_upper, "fitted cambered profile must rise above the chord");
}

#[test]
fn naca5_solver_matches_tabulated_transition() {
    // Tabulated NACA 5-digit data: position 0.15c -> m = 0.2025.
    let solved = naca5_camber(0.15, 0.02).unwrap();
    assert!(
        (solved.transition - 0.2025).abs() < 1e-3,
        "transition must match the tabulated 0.2025, got {}",
        solved.transition
    );
    // Mean line must peak at x = 0.15 with the requested camber.
    let (yc, dyc) = naca5_camber_line(0.15, &solved);
    assert!((yc - 0.02).abs() < 1e-9);
    assert!(dyc.abs() < 1e-6);
    // Position 0.10c -> m = 0.1260 (tabulated).
    let solved = naca5_camber(0.10, 0.02).unwrap();
    assert!((solved.transition - 0.1260).abs() < 1e-3);
    let curve = naca5(0.15, 0.02, 0.12, 1., true, 120, 3, 24).unwrap();
    curve.validate().unwrap();
}

#[test]
fn cst_round_trip_recovers_coefficients() {
    let known = CstProfile {
        n1: 0.5,
        n2: 1.0,
        upper: vec![0.10, 0.16, 0.13, 0.08],
        lower: vec![-0.07, -0.11, -0.09, -0.05],
    };
    let points = cst_points(&known, 1., 160).unwrap();
    let fit = cst_fit(&points, 3, 0.5, 1.0, 1e-10).unwrap();
    for (recovered, truth) in fit
        .profile
        .upper
        .iter()
        .zip(&known.upper)
        .chain(fit.profile.lower.iter().zip(&known.lower))
    {
        assert!(
            (recovered - truth).abs() < 1e-3,
            "recovered {recovered} vs truth {truth}"
        );
    }
    assert!(
        fit.max_residual < 1e-3,
        "vertical residual {} too large",
        fit.max_residual
    );
    assert!(
        fit.hausdorff_residual < 5e-3,
        "hausdorff residual {} too large",
        fit.hausdorff_residual
    );
    // Interior CST shape vanishes at the end points.
    assert_eq!(cst_shape(0., 0.5, 1., &known.upper), 0.);
    assert_eq!(cst_shape(1., 0.5, 1., &known.upper), 0.);
}

#[test]
fn fit_airfoil_closes_trailing_edge_and_bounds_nose_radius() {
    let mean = MeanLine::FourDigit {
        camber: 0.,
        position: 0.,
    };
    let points = naca_points(&|x| mean.at(x), 0.12, 1., true, 160).unwrap();
    let options = AirfoilFitOptions {
        degree: 3,
        controls: 28,
        min_nose_radius: 0.005,
        close_te: true,
        max_iterations: 24,
    };
    let report = fit_airfoil(&points, &options).unwrap();
    assert!(report.nose_radius >= 0.005);
    assert!(report.iterations <= 24);
    let first = &report.curve.control_points[0];
    let last = report.curve.control_points.last().unwrap();
    for k in 0..3 {
        assert_eq!(first[k], last[k], "trailing edge must close exactly");
    }
    assert!(
        report.max_residual < 5e-3,
        "fit residual {} too large",
        report.max_residual
    );
}

#[test]
fn curvature_analysis_flags_waviness() {
    // Smooth reference: a single cubic Bezier arc (one curvature
    // inflection at most) must show 0..=2 sign changes.
    let smooth = Curve {
        degree: 3,
        knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        control_points: vec![
            vec![0., 0., 0.],
            vec![0.3, 0.4, 0.],
            vec![0.7, -0.4, 0.],
            vec![1., 0., 0.],
        ],
        weights: vec![1.; 4],
        periodic: false,
    };
    smooth.validate().unwrap();
    let quality = analyze_curvature(&smooth, 512).unwrap();
    assert!(
        quality.second_difference_sign_changes <= 2,
        "smooth arc shows {} sign changes",
        quality.second_difference_sign_changes
    );
    assert!(quality.curvature_variation.is_finite());
    assert!(quality.min_radius > 0.);
    // Jagged curve: alternating control polygon.
    let mut controls = Vec::new();
    for i in 0..12 {
        let x = i as f64 / 11.;
        let y = if i % 2 == 0 { 0.02 } else { -0.02 };
        controls.push(vec![x, y, 0.]);
    }
    let jagged = Curve {
        degree: 3,
        knots: std::iter::repeat_n(0., 4)
            .chain((1..9).map(|i| i as f64 / 9.))
            .chain(std::iter::repeat_n(1., 4))
            .collect(),
        control_points: controls,
        weights: vec![1.; 12],
        periodic: false,
    };
    jagged.validate().unwrap();
    let quality = analyze_curvature(&jagged, 512).unwrap();
    assert!(
        quality.second_difference_sign_changes >= 6,
        "jagged curve must show many sign changes, got {}",
        quality.second_difference_sign_changes
    );
}

#[test]
fn naca5_camber_rejects_non_finite_parameters_with_typed_payload() {
    let err = naca5_camber(f64::NAN, 0.02).unwrap_err();
    assert!(err.contains("naca5_position"), "{err}");
    assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
    assert!(naca5_camber(0.15, f64::INFINITY).is_err());
    // Budgeted solver still converges on valid inputs.
    assert!(naca5_camber(0.15, 0.02).unwrap().iterations <= 2 * SOLVER_BUDGET);
}

#[test]
fn cst_rejects_non_finite_coefficients_with_typed_payload() {
    let bad = CstProfile {
        n1: 0.5,
        n2: 1.0,
        upper: vec![0.1, f64::NAN, 0.1],
        lower: vec![-0.1, -0.1, -0.1],
    };
    let err = cst_points(&bad, 1., 64).unwrap_err();
    assert!(err.contains("cst_upper[1]"), "{err}");
    let bad_lower = CstProfile {
        n1: 0.5,
        n2: 1.0,
        upper: vec![0.1, 0.1, 0.1],
        lower: vec![-0.1, f64::INFINITY, -0.1],
    };
    let err = cst_points(&bad_lower, 1., 64).unwrap_err();
    assert!(err.contains("cst_lower[1]"), "{err}");
    // Non-finite data sites in cst_fit.
    let mut points = cst_points(&bad_lower_replacement(), 1., 64).unwrap();
    points[10][1] = f64::NAN;
    let err = cst_fit(&points, 2, 0.5, 1.0, 1e-10).unwrap_err();
    assert!(err.contains("cst_fit_data[31]"), "{err}");
}

fn bad_lower_replacement() -> CstProfile {
    CstProfile {
        n1: 0.5,
        n2: 1.0,
        upper: vec![0.1, 0.15, 0.1],
        lower: vec![-0.1, -0.12, -0.1],
    }
}

#[test]
fn naca_points_rejects_non_finite_thickness_and_chord() {
    let mean = MeanLine::FourDigit {
        camber: 0.,
        position: 0.,
    };
    assert!(naca_points(&|x| mean.at(x), f64::NAN, 1., true, 64).is_err());
    assert!(naca_points(&|x| mean.at(x), 0.12, f64::INFINITY, true, 64).is_err());
}
