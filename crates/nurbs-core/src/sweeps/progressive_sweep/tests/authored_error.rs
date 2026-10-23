use super::*;
#[test]
fn owned_dense_profile_patch_error_includes_decomposition_and_product_budget() {
    let profile = Curve {
        degree: 1,
        knots: std::iter::once(0.)
            .chain((0..=32).map(|i| i as f64))
            .chain(std::iter::once(32.))
            .collect(),
        control_points: (0..=32)
            .map(|i| vec![1. + i as f64 / 32., (i % 2) as f64 / 64., 0.])
            .collect(),
        weights: (0..=32).map(|i| if i % 2 == 0 { 1. } else { 2. }).collect(),
        periodic: false,
    };
    let path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    let axis = constant_vector_law([0., 0., 1.]).unwrap();
    let normal = constant_vector_law([1., 0., 0.]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.25, 0., 0.]).unwrap();
    let options = Options {
        normal: [1., 0., 0.],
        orientation: Orientation::Fixed,
        spacing: Spacing::Parameter,
        initial_sections: 3,
        max_sections: 3,
        max_deviation: 0.01,
    };
    let sweep =
        Sweep::new_authored(&profile, &path, &scale, &twist, &axis, &normal, options).unwrap();
    let report = sweep.authored_patch_error_bound(3, 10000, 1000).unwrap();
    assert_eq!(report.status, Status::Certified);
    assert_eq!(report.products, 384);
    assert!(report.within_budget && report.error_upper.unwrap() < 1e-9);
    assert!(report.decomposition_error_upper.unwrap() > 0.);
    let endpoint_upper=report.retained_endpoint_error_upper.unwrap();
    assert!(endpoint_upper>=report.original_section_endpoint_error_upper.unwrap());
    assert!(endpoint_upper>=report.decomposition_error_upper.unwrap());
    let patches = report.patches.unwrap();
    assert_eq!(patches.len(), 32);
    let (s, c) = 0.25_f64.sin_cos();
    for (span, patch) in patches.iter().enumerate() {
        let a = patch.knots_u[patch.degree_u];
        let b = patch.knots_u[patch.control_points.len()];
        for f in [0., 0.375, 1.] {
            let p = profile.evaluate(span as f64 + f).unwrap().point;
            for t in [0., 0.375, 1.] {
                let got = patch.evaluate(a + (b - a) * f, t).unwrap().point;
                let expected = [p[0] * c - p[1] * s, p[0] * s + p[1] * c, 10. * t];
                assert!(
                    norm(std::array::from_fn(|k| got[k] - expected[k]))
                        <= report.error_upper.unwrap()
                );
                if t==0. || t==1. {
                    assert!(norm(std::array::from_fn(|k|got[k]-expected[k]))<=endpoint_upper);
                }
            }
        }
    }
    let refused = sweep.authored_patch_error_bound(3, 10000, 383).unwrap();
    assert_eq!(refused.status, Status::Unresolved);
    assert!(refused.retained_endpoint_error_upper.is_none());
    assert!(
        refused.patches.is_none() && refused.error_upper.is_none() && refused.products <= 383
    );
}
#[test]
fn owned_section_bound_covers_varying_joint_laws_and_refinement() {
    let mut profile = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
    profile.weights = vec![1., 2.];
    let path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    let axis = constant_vector_law([0., 0., 1.]).unwrap();
    let normal = crate::primitives::line([1., 0., 0.], [1., 1., 0.]).unwrap();
    let scale = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
    let twist = crate::primitives::line([0., 0., 0.], [0.25, 0., 0.]).unwrap();
    let axes = crate::primitives::line([1.; 3], [2., 1., 1.]).unwrap();
    let center = crate::primitives::line([0.; 3], [0.5, 0., 0.]).unwrap();
    let options = Options {
        normal: [1., 0., 0.],
        orientation: Orientation::Fixed,
        spacing: Spacing::Parameter,
        initial_sections: 3,
        max_sections: 9,
        max_deviation: 0.01,
    };
    let sweep = Sweep::new_authored(&profile, &path, &scale, &twist, &axis, &normal, options)
        .unwrap()
        .with_affine_laws(&axes, &center)
        .unwrap();
    let coarse = sweep
        .authored_section_interpolation_bound(3, 10000)
        .unwrap();
    let fine = sweep
        .authored_section_interpolation_bound(9, 10000)
        .unwrap();
    assert_eq!(coarse.status, Status::Certified);
    assert_eq!(fine.status, Status::Certified);
    assert!(fine.error_upper.unwrap() < coarse.error_upper.unwrap());
    let sections = sweep.sections(3).unwrap().0;
    for i in 0..=32 {
        let t = i as f64 / 32.;
        let station = (i / 16).min(1);
        let fraction = t * 2. - station as f64;
        for u in [0., 0.375, 1.] {
            let a = sections[station].evaluate(u).unwrap().point;
            let b = sections[station + 1].evaluate(u).unwrap().point;
            let amplitude = (1. + 3. * u) / (1. + u) * (1. + t).powi(2) + 0.5 * t;
            let angle = t.atan() + 0.25 * t;
            let ideal = [amplitude * angle.cos(), amplitude * angle.sin(), 10. * t];
            let delta =
                std::array::from_fn(|k| (1. - fraction) * a[k] + fraction * b[k] - ideal[k]);
            assert!(norm(delta) <= coarse.error_upper.unwrap());
        }
    }
}
#[test]
fn authored_initial_coordinates_enclose_original_profile_in_independent_domains() {
    let mut profile = crate::primitives::line([1., 2., 3.], [2., 2., 3.]).unwrap();
    profile.weights = vec![1., 2.];
    let mut path = crate::primitives::line([0.5, 1., -1.], [0.5, 1., 9.]).unwrap();
    path.knots = vec![17., 17., 19., 19.];
    let mut axis = constant_vector_law([2., 0., 0.]).unwrap();
    axis.knots = vec![2., 2., 5., 5.];
    let mut normal = constant_vector_law([7., 1., 0.]).unwrap();
    normal.knots = vec![31., 31., 41., 41.];
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.25, 0., 0.]).unwrap();
    let options = Options {
        normal: [0., 1., 0.],
        orientation: Orientation::Fixed,
        spacing: Spacing::Parameter,
        initial_sections: 3,
        max_sections: 3,
        max_deviation: 0.01,
    };
    let sweep =
        Sweep::new_authored(&profile, &path, &scale, &twist, &axis, &normal, options).unwrap();
    let report = sweep.authored_initial_coordinates(100).unwrap();
    assert_eq!(report.status, Status::Certified);
    assert_eq!(report.cells, 10);
    let coordinates = report.coordinates.unwrap();
    for (q, expected) in coordinates.iter().zip([[1., 4., 0.5], [1., 4., 1.5]]) {
        for k in 0..3 {
            assert!(q[k][0] <= expected[k] && expected[k] <= q[k][1]);
        }
    }
    let trajectory = sweep.authored_control_trajectory(0, [0., 1.], 100).unwrap();
    assert_eq!(trajectory.status, Status::Certified);
    assert_eq!(trajectory.cells, 21);
    let jet = trajectory.jet.unwrap();
    let (s, c) = 0.25_f64.sin_cos();
    for i in 0..=16 {
        let t = i as f64 / 16.;
        let expected = [1., 1. + c - 4. * s, -1. + 10. * t + s + 4. * c];
        for k in 0..3 {
            assert!(jet.value[k][0] <= expected[k] && expected[k] <= jet.value[k][1]);
            let derivative = if k == 2 { 10. } else { 0. };
            assert!(jet.first[k][0] <= derivative && derivative <= jet.first[k][1]);
        }
    }
    let limited = sweep.authored_control_trajectory(0, [0., 1.], 20).unwrap();
    assert_eq!(limited.status, Status::Unresolved);
    assert!(limited.jet.is_none() && limited.cells <= 20);
    let bound = sweep.authored_section_interpolation_bound(3, 1000).unwrap();
    assert_eq!(bound.status, Status::Certified);
    // Original source premises are shared across profile controls.
    assert_eq!(bound.cells, 65);
    assert!(bound.error_upper.unwrap() < 1e-10);
    assert!(bound.endpoint_displacement_upper.unwrap() < 1e-10);
    let incomplete = sweep
        .authored_section_interpolation_bound(3, bound.cells - 1)
        .unwrap();
    assert_eq!(incomplete.status, Status::Unresolved);
    assert!(
        incomplete.error_upper.is_none() && incomplete.endpoint_displacement_upper.is_none()
    );
    assert!(incomplete.cells < bound.cells);
    assert!(sweep.authored_control_trajectory(2, [0., 1.], 100).is_err());
    for budget in [0, 9] {
        let refused = sweep.authored_initial_coordinates(budget).unwrap();
        assert_eq!(refused.status, Status::Unresolved);
        assert!(refused.coordinates.is_none() && refused.cells <= budget);
    }
    let plain = Sweep::new(&profile, &path, &scale, &twist, options).unwrap();
    assert_eq!(
        plain.authored_initial_coordinates(100).unwrap().reason,
        Some("mode-not-authored")
    );
    let arc_options = Options {
        spacing: Spacing::ArcLength {
            tolerance: 0.001,
            max_cells: 100,
        },
        ..options
    };
    let arc = Sweep::new_authored(&profile, &path, &scale, &twist, &axis, &normal, arc_options)
        .unwrap();
    assert_eq!(arc.authored_control_trajectory(0,[0.,1.],100).unwrap().status,Status::Certified);
}
