//! Analytic point-cloud oracles for five distinct native fitting models.
use nurbs_core::analysis::primitive_fit::*;
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}
fn length(p: [f64; 3]) -> f64 {
    p[0].hypot(p[1]).hypot(p[2])
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|i| a[i] * b[i]).sum()
}
fn report_matches(report: FitReport, residuals: &[f64]) {
    let max = residuals.iter().map(|x| x.abs()).fold(0., f64::max);
    let rms = (residuals.iter().map(|x| x * x).sum::<f64>() / residuals.len() as f64).sqrt();
    assert!(report.rms.is_finite() && report.max_dev.is_finite());
    assert!((report.rms - rms).abs() < 1e-10);
    assert!((report.max_dev - max).abs() < 1e-10);
    assert!(max < 1e-6, "max residual {max}");
    assert!(report.iterations <= 100);
}
fn angle(i: usize, n: usize) -> f64 {
    std::f64::consts::TAU * i as f64 / n as f64
}

#[test]
fn sphere_recovers_six_analytic_axis_samples_and_reports_actual_residuals() {
    let center = [2., -3., 4.];
    let radius = 5.;
    let mut points = Vec::new();
    for k in 0..3 {
        for sign in [-1., 1.] {
            let mut p = center;
            p[k] += sign * radius;
            points.push(p);
        }
    }
    let before = points.clone();
    let fit = fit_sphere(&points).unwrap();
    assert!(length(sub(fit.center, center)) < 1e-8);
    assert!((fit.radius - radius).abs() < 1e-8);
    report_matches(
        fit.report,
        &points
            .iter()
            .map(|p| length(sub(*p, fit.center)) - fit.radius)
            .collect::<Vec<_>>(),
    );
    assert_eq!(points, before);
}

#[test]
fn cylinder_recovers_a_known_axis_and_radius_from_multiple_rings() {
    let mut points = Vec::new();
    for z in [-3., 0., 3.] {
        for j in 0..24 {
            let a = angle(j, 24);
            points.push([2. + 3. * a.cos(), -1. + 3. * a.sin(), z]);
        }
    }
    let fit = fit_cylinder(&points).unwrap();
    assert!(fit.direction[2].abs() > 1. - 1e-8);
    assert!((fit.radius - 3.).abs() < 1e-8);
    assert!((fit.point_on_axis[0] - 2.).abs() < 1e-8 && (fit.point_on_axis[1] + 1.).abs() < 1e-8);
    let residuals: Vec<_> = points
        .iter()
        .map(|p| {
            let w = sub(*p, fit.point_on_axis);
            let axial = dot(w, fit.direction);
            length(std::array::from_fn(|k| w[k] - axial * fit.direction[k])) - fit.radius
        })
        .collect();
    report_matches(fit.report, &residuals);
}

#[test]
fn cone_recovers_vertex_opening_and_oriented_axis() {
    let alpha = 0.35_f64;
    let vertex = [1., 2., -1.];
    let mut points = Vec::new();
    for height in [1., 2., 4., 7.] {
        for j in 0..24 {
            let a = angle(j, 24);
            let r = height * alpha.tan();
            points.push([
                vertex[0] + r * a.cos(),
                vertex[1] + r * a.sin(),
                vertex[2] + height,
            ]);
        }
    }
    let fit = fit_cone(&points).unwrap();
    assert!(length(sub(fit.vertex, vertex)) < 1e-6);
    assert!(fit.axis[2] > 1. - 1e-8);
    assert!((fit.half_angle - alpha).abs() < 1e-8);
    let (sa, ca) = fit.half_angle.sin_cos();
    let residuals: Vec<_> = points
        .iter()
        .map(|p| {
            let w = sub(*p, fit.vertex);
            let t = dot(w, fit.axis);
            t * sa - length(std::array::from_fn(|k| w[k] - t * fit.axis[k])) * ca
        })
        .collect();
    report_matches(fit.report, &residuals);
}

#[test]
fn torus_recovers_both_radii_and_center_from_a_periodic_grid() {
    let center = [1., -2., 3.];
    let mut points = Vec::new();
    for i in 0..24 {
        for j in 0..12 {
            let a = angle(i, 24);
            let b = angle(j, 12);
            let r = 4. + b.cos();
            points.push([
                center[0] + r * a.cos(),
                center[1] + r * a.sin(),
                center[2] + b.sin(),
            ]);
        }
    }
    let fit = fit_torus(&points).unwrap();
    assert!(length(sub(fit.center, center)) < 1e-8);
    assert!(fit.axis[2].abs() > 1. - 1e-8);
    assert!((fit.major_radius - 4.).abs() < 1e-8 && (fit.minor_radius - 1.).abs() < 1e-8);
    let residuals: Vec<_> = points
        .iter()
        .map(|p| {
            let w = sub(*p, fit.center);
            let z = dot(w, fit.axis);
            let rho = length(std::array::from_fn(|k| w[k] - z * fit.axis[k]));
            (rho - fit.major_radius).hypot(z) - fit.minor_radius
        })
        .collect();
    report_matches(fit.report, &residuals);
}

#[test]
fn plane_fit_agrees_with_an_independent_linear_equation() {
    let mut points = Vec::new();
    for x in -2..=2 {
        for y in -2..=2 {
            points.push([x as f64, y as f64, (2 * x - 3 * y + 1) as f64]);
        }
    }
    let fit = fit_plane(&points).unwrap();
    let expected = [2., -3., -1.];
    assert!((length(fit.normal) - 1.).abs() < 1e-12);
    assert!(dot(fit.normal, expected).abs() / length(expected) > 1. - 1e-10);
    report_matches(
        fit.report,
        &points
            .iter()
            .map(|p| dot(fit.normal, *p) + fit.offset)
            .collect::<Vec<_>>(),
    );
    assert_eq!(fit.report.iterations, 0);
    assert!(fit.report.converged);
}

#[test]
fn every_model_rejects_empty_nonfinite_collapsed_and_over_budget_inputs() {
    type Fitter = fn(&[[f64; 3]]) -> nurbs_core::Result<FitReport>;
    let fitters: [Fitter; 5] = [
        |p| fit_sphere(p).map(|f| f.report),
        |p| fit_cylinder(p).map(|f| f.report),
        |p| fit_cone(p).map(|f| f.report),
        |p| fit_torus(p).map(|f| f.report),
        |p| fit_plane(p).map(|f| f.report),
    ];
    let collapsed = vec![[1., 2., 3.]; 20];
    let excessive = vec![[0.; 3]; MAX_FIT_POINTS + 1];
    for fit in fitters {
        assert!(fit(&[]).is_err());
        assert!(fit(&collapsed).is_err());
        assert!(fit(&excessive).is_err());
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut points = vec![[0., 0., 0.]; 8];
            points[3][1] = bad;
            let error = fit(&points).unwrap_err();
            assert!(error.to_string().contains("points"));
        }
    }
}

#[test]
fn a_coplanar_cloud_cannot_be_qualified_as_a_fitted_sphere() {
    let points = [
        [0., 0., 0.],
        [1., 0., 0.],
        [0., 1., 0.],
        [1., 1., 0.],
        [2., 1., 0.],
    ];
    assert!(fit_sphere(&points).is_err());
}
