use super::*;
use crate::surface::Axis;

/// Bilinear plane patch through the origin spanned by two unit axes.
fn plane(u_dir: [f64; 3], v_dir: [f64; 3], size: f64) -> Surface {
    let p = |su: f64, sv: f64| {
        vec![
            u_dir[0] * su + v_dir[0] * sv,
            u_dir[1] * su + v_dir[1] * sv,
            u_dir[2] * su + v_dir[2] * sv,
        ]
    };
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![p(0., 0.), p(0., size)],
            vec![p(size, 0.), p(size, size)],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}

/// Cylinder of radius 3 about the Y axis through (0, ·, 4), y in [0, 8].
fn cylinder_y() -> Surface {
    let profile = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![3., 0., 4.], vec![3., 8., 4.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    crate::surface::revolve(&profile, [0., 0., 4.], [0., 1., 0.], 360.).unwrap()
}

/// Perpendicular planes z=0 (normal +z) and x=0 (normal +x).
fn corner_planes() -> (Surface, Surface) {
    (
        plane([1., 0., 0.], [0., 1., 0.], 10.),
        plane([0., 1., 0.], [0., 0., 1.], 10.),
    )
}

/// Signed side of the cylinder's normal (outward = +1).
fn cylinder_outward_sign(c: &Surface) -> f64 {
    let (du, dv) = domain(c);
    // Avoid knot seams (revolve joints) where the normal is undefined.
    let e = c
        .evaluate(du[0] + 0.37 * (du[1] - du[0]), dv[0] + 0.41 * (dv[1] - dv[0]))
        .unwrap();
    let n = e.unit_normal().unwrap();
    let radial = sub(e.point, [0., e.point[1], 4.]);
    if dot(n, radial) >= 0. { 1. } else { -1. }
}

#[test]
fn rolling_ball_between_perpendicular_planes_is_exact_cylinder() {
    let (a, b) = corner_planes();
    let r = 1.5;
    let report = rolling_ball_fillet(&a, &b, r, &FilletOptions::default()).unwrap();
    assert!(report.failure.is_none(), "{:?}", report.failure);
    let surface = report.surface.as_ref().unwrap();
    let (du, dv) = domain(surface);
    // Every surface point sits on the pipe of radius r around the spine
    // line x = r, z = r (the analytic rolling-ball locus); tangency edges
    // lie in the two planes.
    for i in 0..=8 {
        for j in 0..=8 {
            let p = surface
                .evaluate(du[0] + (du[1] - du[0]) * i as f64 / 8., dv[0] + (dv[1] - dv[0]) * j as f64 / 8.)
                .unwrap()
                .point;
            let to_axis = ((p[0] - r).powi(2) + (p[2] - r).powi(2)).sqrt();
            assert!((to_axis - r).abs() < 1e-6, "p = {p:?}, |p − axis| = {to_axis}");
            assert!(p[0] >= -1e-9 && p[2] >= -1e-9);
        }
    }
    // Contact edges: u = 0 on plane z = 0, u = 1 on plane x = 0.
    for j in 0..=4 {
        let v = dv[0] + (dv[1] - dv[0]) * j as f64 / 4.;
        let p0 = surface.evaluate(du[0], v).unwrap().point;
        let p1 = surface.evaluate(du[1], v).unwrap().point;
        assert!(p0[2].abs() < 1e-9, "start contact: {p0:?}");
        assert!(p1[0].abs() < 1e-9, "end contact: {p1:?}");
    }
    assert_eq!(report.radius_range, [r, r]);
    assert!(!report.spine.is_empty());
    // Spine centers are at distance r from both planes.
    for s in &report.spine {
        assert!((s.center[0] - r).abs() < 1e-6 && (s.center[2] - r).abs() < 1e-6);
    }
}

#[test]
fn rolling_ball_plane_cylinder_matches_analytic_spine() {
    let a = plane([1., 0., 0.], [0., 1., 0.], 10.); // z = 0, normal +z
    let c = cylinder_y();
    let side_c = cylinder_outward_sign(&c);
    let r = 1.0;
    let options = FilletOptions { side_a: 1., side_b: side_c, ..Default::default() };
    let report = rolling_ball_fillet(&a, &c, r, &options).unwrap();
    assert!(report.failure.is_none(), "{:?}", report.failure);
    // Analytic spine: z = r, distance from the axis = R + r = 4 → x = √7.
    let x0 = 7_f64.sqrt();
    for s in &report.spine {
        assert!((s.center[2] - r).abs() < 1e-6, "center {:?}", s.center);
        let to_axis = (s.center[0].powi(2) + (s.center[2] - 4.).powi(2)).sqrt();
        assert!((to_axis - 4.).abs() < 1e-6, "center {:?}", s.center);
        assert!((s.center[0] - x0).abs() < 1e-6);
        // Contact on the plane lies in z = 0, contact on the cylinder at
        // radius 3 from its axis.
        assert!(s.contact_a[2].abs() < 1e-6);
        let cb = (s.contact_b[0].powi(2) + (s.contact_b[2] - 4.).powi(2)).sqrt();
        assert!((cb - 3.).abs() < 1e-6);
    }
    // Surface points stay on the pipe of radius r around the spine line.
    let surface = report.surface.as_ref().unwrap();
    let (du, dv) = domain(surface);
    for i in 0..=6 {
        for j in 0..=6 {
            let p = surface
                .evaluate(du[0] + (du[1] - du[0]) * i as f64 / 6., dv[0] + (dv[1] - dv[0]) * j as f64 / 6.)
                .unwrap()
                .point;
            let to_axis = ((p[0] - x0).powi(2) + (p[2] - r).powi(2)).sqrt();
            assert!((to_axis - r).abs() < 1e-5, "p = {p:?}, |p − axis| = {to_axis}");
        }
    }
}

#[test]
fn variable_radius_linear_law_averages_at_midpoint() {
    let (a, b) = corner_planes();
    let options = FilletOptions { max_sections: 9, ..Default::default() };
    let report = variable_radius_fillet(
        &a,
        &b,
        RadiusLaw::Linear { start: 0.5, end: 1.5 },
        &options,
    )
    .unwrap();
    assert!(report.failure.is_none(), "{:?}", report.failure);
    // Odd station count puts a station exactly at t = 0.5.
    let mid = report
        .spine
        .iter()
        .find(|s| (s.t - 0.5).abs() < 1e-9)
        .expect("middle station");
    assert!((mid.radius - 1.0).abs() < 1e-12, "{}", mid.radius);
    // The mid section is an exact arc of radius 1.0: its points sit at
    // distance 1.0 from the mid center.
    let surface = report.surface.as_ref().unwrap();
    let (_du, dv) = domain(surface);
    let iso = surface.iso(Axis::V, dv[0] + (dv[1] - dv[0]) * 0.5).unwrap();
    for i in 0..=10 {
        let p = iso.evaluate(i as f64 / 10.).unwrap().point;
        let d = norm(sub(
            [p[0], p[1], p[2]],
            mid.center,
        ));
        assert!((d - 1.0).abs() < 1e-6, "d = {d}");
    }
    // Radii grow monotonically along the spine.
    assert!((report.radius_range[0] - 0.5).abs() < 1e-9);
    assert!((report.radius_range[1] - 1.5).abs() < 1e-9);
}

#[test]
fn chordal_on_right_angle_planes_recovers_c_over_sqrt2() {
    let (a, b) = corner_planes();
    let chord = 1.4;
    let report = chordal_fillet(&a, &b, chord, &FilletOptions::default()).unwrap();
    assert!(report.fillet.failure.is_none(), "{:?}", report.fillet.failure);
    let expected = chord / 2_f64.sqrt();
    assert!((report.radius_range[0] - expected).abs() < 1e-6, "{:?}", report.radius_range);
    assert!((report.radius_range[1] - expected).abs() < 1e-6, "{:?}", report.radius_range);
    let right = std::f64::consts::FRAC_PI_2;
    assert!((report.dihedral_range[0] - right).abs() < 1e-6);
    assert!((report.dihedral_range[1] - right).abs() < 1e-6);
    assert!(report.fillet.surface.is_some());
}

#[test]
fn g2_section_curvature_matches_supports() {
    // Plane (curvature 0) against the radius-3 cylinder (normal curvature
    // magnitude 1/3 in the circumferential section direction).
    let a = plane([1., 0., 0.], [0., 1., 0.], 10.);
    let c = cylinder_y();
    let side_c = cylinder_outward_sign(&c);
    let options = FilletOptions { side_a: 1., side_b: side_c, max_sections: 5, ..Default::default() };
    let report = g2_fillet(&a, &c, 1.0, &options).unwrap();
    assert!(report.failure.is_none(), "{:?}", report.failure);
    let surface = report.surface.as_ref().unwrap();
    let (du, dv) = domain(surface);
    let curvature = |curve: &Curve, u: f64| {
        let e = curve.evaluate(u).unwrap();
        let d1 = e.d1.unwrap();
        let d2 = e.d2.unwrap();
        norm(cross(
            [d1[0], d1[1], d1[2]],
            [d2[0], d2[1], d2[2]],
        )) / norm([d1[0], d1[1], d1[2]]).powi(3)
    };
    // Check several stations: curvature at u = 0 ≈ 0 (plane), at u = 1
    // ≈ 1/3 (cylinder, sign carried by direction not magnitude).
    for k in 1..4 {
        let v = dv[0] + (dv[1] - dv[0]) * k as f64 / 4.;
        let iso = surface.iso(Axis::V, v).unwrap();
        let k_plane = curvature(&iso, du[0]);
        let k_cyl = curvature(&iso, du[1]);
        assert!(k_plane < 1e-3, "station {k}: plane end k = {k_plane}");
        assert!((k_cyl - 1. / 3.).abs() < 1e-3, "station {k}: cylinder end k = {k_cyl}");
    }
}

#[test]
fn hold_line_sections_pass_through_resampled_lines_and_stay_tangent() {
    let (a, b) = corner_planes();
    let line_a: Vec<[f64; 3]> = (0..=4).map(|i| [1., 8. * i as f64 / 4., 0.]).collect();
    let line_b: Vec<[f64; 3]> = (0..=4).map(|i| [0., 8. * i as f64 / 4., 1.]).collect();
    let report = hold_line_fillet(&a, &b, &line_a, &line_b, &FilletOptions::default()).unwrap();
    assert!(report.failure.is_none(), "{:?}", report.failure);
    let surface = report.surface.as_ref().unwrap();
    let (du, dv) = domain(surface);
    // Station iso curves hit the hold points and start/end tangent to the
    // supports (section tangent perpendicular to each plane normal).
    for k in 0..report.spine.len() {
        let v = dv[0] + (dv[1] - dv[0]) * report.spine[k].t;
        let iso = surface.iso(Axis::V, v).unwrap();
        let start = iso.evaluate(iso.domain()[0]).unwrap();
        let end = iso.evaluate(iso.domain()[1]).unwrap();
        for (p, q) in [(start.point.clone(), report.spine[k].contact_a), (end.point.clone(), report.spine[k].contact_b)] {
            assert!(norm(sub([p[0], p[1], p[2]], q)) < 1e-8, "{p:?} vs {q:?}");
        }
        let t0 = start.d1.unwrap();
        let t1 = end.d1.unwrap();
        // Plane z = 0 normal is +z; plane x = 0 normal is +x.
        assert!(t0[2].abs() / norm([t0[0], t0[1], t0[2]]) < 1e-6, "t0 = {t0:?}");
        assert!(t1[0].abs() / norm([t1[0], t1[1], t1[2]]) < 1e-6, "t1 = {t1:?}");
    }
}

#[test]
fn tiny_radius_fails_gracefully_without_panic() {
    let (a, b) = corner_planes();
    let report = rolling_ball_fillet(&a, &b, 1e-12, &FilletOptions::default()).unwrap();
    assert!(report.surface.is_none());
    let failure = report.failure.as_ref().expect("failure must be reported");
    assert!(matches!(
        failure.reason,
        FailureReason::RadiusBelowMinimum { .. } | FailureReason::SeedNotFound
    ));
    // Sensible radius on the wrong side also fails gracefully.
    let wrong = rolling_ball_fillet(
        &a,
        &b,
        1.5,
        &FilletOptions { side_a: -1., side_b: 1., ..Default::default() },
    )
    .unwrap();
    if let Some(f) = wrong.failure {
        assert!(f.detail.len() > 0);
    }
}

#[test]
fn spline_law_and_invalid_inputs() {
    let (a, b) = corner_planes();
    // Invalid inputs are errors, not reports.
    assert!(rolling_ball_fillet(&a, &b, f64::NAN, &FilletOptions::default()).is_err());
    assert!(rolling_ball_fillet(&a, &b, -1., &FilletOptions::default()).is_err());
    assert!(chordal_fillet(&a, &b, 0., &FilletOptions::default()).is_err());
    assert!(
        variable_radius_fillet(&a, &b, RadiusLaw::Spline(vec![(0., 1.), (0., 2.)]), &FilletOptions::default()).is_err()
    );
    // A smooth spline law builds a surface.
    let report = variable_radius_fillet(
        &a,
        &b,
        RadiusLaw::Spline(vec![(0., 0.8), (0.5, 1.2), (1., 0.8)]),
        &FilletOptions::default(),
    )
    .unwrap();
    assert!(report.failure.is_none(), "{:?}", report.failure);
    assert!(report.radius_range[0] >= 0.8 - 0.1 && report.radius_range[1] >= 1.1);
}

#[test]
fn nan_radius_is_a_typed_non_finite_rejection() {
    let (a, b) = corner_planes();
    // A NaN radius must be rejected as NonFinite before any marching.
    let err = rolling_ball_fillet(&a, &b, f64::NAN, &FilletOptions::default()).err().expect("invalid geometry input must fail");
    assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
    assert!(err.contains("radius"), "{err}");
    assert!(err.contains("must be finite"), "{err}");
    let err = g2_fillet(&a, &b, f64::INFINITY, &FilletOptions::default()).err().expect("invalid geometry input must fail");
    assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
    let err = chordal_fillet(&a, &b, f64::NAN, &FilletOptions::default()).err().expect("invalid geometry input must fail");
    assert!(err.contains("chord"), "{err}");
    // NaN inside a spline law names the indexed parameter.
    let err = variable_radius_fillet(
        &a,
        &b,
        RadiusLaw::Spline(vec![(0., 1.), (0.5, f64::NAN), (1., 1.)]),
        &FilletOptions::default(),
    )
    .err().expect("invalid geometry input must fail");
    assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
    assert!(err.contains("law_radius[1]"), "{err}");
}

#[test]
fn nan_hold_line_point_is_rejected_before_projection() {
    let (a, b) = corner_planes();
    let line_a = [[0.5, 0.5, f64::NAN], [1.5, 0.5, 1.]];
    let line_b = [[0.5, 0.5, 0.], [1.5, 0.5, 1.]];
    let err = hold_line_fillet(&a, &b, &line_a, &line_b, &FilletOptions::default()).err().expect("invalid geometry input must fail");
    assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
    assert!(err.contains("hold_line"), "{err}");
}

#[test]
fn march_step_guard_keeps_tiny_budgets_graceful() {
    let (a, b) = corner_planes();
    // A one-step budget must still terminate gracefully (report, not panic).
    let options = FilletOptions { max_march_steps: 1, ..Default::default() };
    let report = rolling_ball_fillet(&a, &b, 1.0, &options).unwrap();
    assert!(report.failure.is_some() || report.surface.is_some());
}
