//! Independent metric checks for a nonplanar closed path, plus owned seam.
use nurbs_core::{
    curve::Curve,
    progressive_sweep::{Options, Orientation, Spacing},
};
fn law(a: f64, b: f64) -> Curve {
    Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![a, 0., 0.], vec![b, 0., 0.]],
        weights: vec![1.; 2],
        periodic: false,
    }
}
fn path() -> Curve {
    Curve {
        degree: 5,
        knots: [vec![0.; 6], vec![1.; 6]].concat(),
        control_points: vec![
            vec![0., 0., 0.],
            vec![2., 0., 0.],
            vec![2., 2., 2.],
            vec![-1., 3., -0.5],
            vec![-2., 0., 0.],
            vec![0., 0., 0.],
        ],
        weights: vec![1.; 6],
        periodic: false,
    }
}
fn options() -> Options {
    Options {
        normal: [0., 0., 1.],
        orientation: Orientation::RotationMinimizing,
        spacing: Spacing::Parameter,
        initial_sections: 5,
        max_sections: 129,
        max_deviation: 0.03,
    }
}
fn analytic(u: f64) -> ([f64; 3], [f64; 3]) {
    let p = [
        [0., 0., 0.],
        [2., 0., 0.],
        [2., 2., 2.],
        [-1., 3., -0.5],
        [-2., 0., 0.],
        [0., 0., 0.],
    ];
    let mut center = [0.; 3];
    let mut tangent = [0.; 3];
    for i in 0..6 {
        let b = [1., 5., 10., 10., 5., 1.][i] * u.powi(i as i32) * (1. - u).powi(5 - i as i32);
        for k in 0..3 {
            center[k] += b * p[i][k];
        }
    }
    for i in 0..5 {
        let b = 5. * [1., 4., 6., 4., 1.][i] * u.powi(i as i32) * (1. - u).powi(4 - i as i32);
        for k in 0..3 {
            tangent[k] += b * (p[i + 1][k] - p[i][k]);
        }
    }
    let length = tangent[0].hypot(tangent[1]).hypot(tangent[2]);
    (center, tangent.map(|v| v / length))
}
#[test]
fn nonplanar_closed_sweep_keeps_circle_metric_and_owns_every_seam_edge() {
    let profile = nurbs_core::primitives::circle([0.; 3], [1., 0., 0.], 0.15).unwrap();
    let source = path();
    let original = source.clone();
    let evidence = brep_core::analytic::progressive_profile_body_with_rmf_policy(
        &[vec![profile]],
        &source,
        &law(1., 1.),
        &law(0., 0.),
        None,
        None,
        None,
        options(),
        None,
        Some((4096, 100000, 1000000)),
    )
    .unwrap();
    assert_eq!(evidence.boundary_error_within_budget, Some(true));
    assert!(evidence.boundary_error_upper.unwrap() <= options().max_deviation);
    let body = &evidence.model;
    let level = evidence.approximation.levels.last().unwrap();
    assert!(level.accepted && level.closed_path && level.continuous_bound);
    assert!(level.continuous_error_upper.unwrap() <= options().max_deviation);
    let n = level.sections;
    assert_eq!(body.faces.len(), 4 * (n - 1));
    assert_eq!(body.vertices.len(), 4 * (n - 1));
    assert_eq!(body.bodies.len(), 1);
    assert_eq!(body.shells.len(), 1);
    assert!(body.shells[0].closed);
    assert_eq!(body.validate().unwrap().boundary_edge_count, 0);
    for (index, face) in body.faces.iter().enumerate() {
        let layer = index / 4;
        for (v, station) in [(0., layer), (1., layer + 1)] {
            let u = station as f64 / (n - 1) as f64;
            let (center, tangent) = analytic(u);
            for angle in [0., 0.19, 0.67, 1.] {
                let p = face.surface.evaluate(angle, v).unwrap().point;
                let d: [f64; 3] = std::array::from_fn(|k| p[k] - center[k]);
                assert!((d[0].hypot(d[1]).hypot(d[2]) - 0.15).abs() < 2e-10);
                assert!((0..3).map(|k| d[k] * tangent[k]).sum::<f64>().abs() < 2e-10);
            }
        }
    }
    for edge in 0..body.edges.len() {
        assert_eq!(
            body.loops
                .iter()
                .flat_map(|l| &l.coedges)
                .filter(|c| c.edge == edge)
                .count(),
            2
        );
    }
    assert_eq!(source, original);
}
#[test]
fn closed_sweep_refuses_scale_mismatch_and_unaccepted_refinement() {
    let profile = nurbs_core::primitives::circle([0.; 3], [1., 0., 0.], 0.15).unwrap();
    let source = path();
    assert!(
        brep_core::progressive_profile_body(
            &[vec![profile.clone()]],
            &source,
            &law(1., 2.),
            &law(0., 0.),
            options()
        )
        .is_err()
    );
    let mut coarse = options();
    coarse.max_sections = 5;
    coarse.max_deviation = 1e-12;
    assert!(
        brep_core::progressive_profile_body(
            &[vec![profile]],
            &source,
            &law(1., 1.),
            &law(0., 0.),
            coarse
        )
        .is_err()
    );
}

#[test]
fn open_profile_closed_surface_has_exact_stored_seam_and_preserves_section_metric() {
    let source = path();
    let profile = nurbs_core::primitives::line([0., 0., -0.15], [0., 0., 0.15]).unwrap();
    let result =
        nurbs_core::framed_sweep::checked_sweep(&profile, &source, [0., 0., 1.], 32, 0.1).unwrap();
    assert!(result.report.accepted && result.report.closed_path);
    let surface = result.surface.unwrap();
    assert!(surface.periodic_v);
    for row in &surface.control_points {
        assert_eq!(row.first(), row.last());
    }
    for row in &surface.weights {
        assert_eq!(row.first(), row.last());
    }
    for station in 0..32 {
        let v = station as f64 / 31.;
        let (center, tangent) = analytic(v);
        for u in [0., 0.2, 0.5, 0.8, 1.] {
            let p = surface.evaluate(u, v).unwrap().point;
            let d: [f64; 3] = std::array::from_fn(|k| p[k] - center[k]);
            assert!((d[0].hypot(d[1]).hypot(d[2]) - 0.15 * (2. * u - 1.).abs()).abs() < 2e-10);
            assert!((0..3).map(|k| d[k] * tangent[k]).sum::<f64>().abs() < 2e-10);
        }
    }
    let refused =
        nurbs_core::framed_sweep::checked_sweep(&profile, &source, [0., 0., 1.], 4, 1e-12).unwrap();
    assert!(!refused.report.accepted && refused.surface.is_none());
}
