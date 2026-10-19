use super::*;
use crate as nurbs;
#[path = "../../../../../tests/support/affine_frame_source.rs"]
mod affine_case;
#[path = "../../../../../tests/support/general_frame_source.rs"]
mod source;

fn ledger() -> IdealVolumeReport {
    IdealVolumeReport {
        embedding_certified: false,
        regularity_certified: false,
        parameter_topology_preserved: false,
        cells: 0,
        exact_work: 0,
        boxes: 0,
        monotonicity_lower: None,
        projection: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        reason: None,
    }
}

#[test]
fn positive_affine_radius_and_axial_rate_own_domain_units_and_work() {
    let path = crate::primitives::line([0.; 3], [0., 0., 2.]).unwrap();
    let profile = crate::primitives::line([0.1, 0., 0.], [0.2, 0., 0.]).unwrap();
    let uniform = constant_vector_law([2., 0., 0.]).unwrap();
    let axes = constant_vector_law([3., 4., 5.]).unwrap();
    let center = constant_vector_law([6., 8., 10.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let options = Options {
        normal: [1., 0., 0.],
        orientation: Orientation::Fixed,
        spacing: Spacing::ArcLength {
            tolerance: 0.001,
            max_cells: 100000,
        },
        initial_sections: 3,
        max_sections: 9,
        max_deviation: 0.05,
    };
    let sweep = Sweep::new(&profile, &path, &uniform, &twist, options)
        .unwrap()
        .with_affine_laws(&axes, &center)
        .unwrap();
    let mut out = ledger();
    let radius = affine::radius(&sweep, 0.5, &mut out, 1000)
        .unwrap()
        .unwrap();
    assert!(296_f64.sqrt() <= radius && radius < 296_f64.sqrt() + 1e-10);
    assert!(out.cells > 0);
    for max in [0, out.cells - 1] {
        let mut short = ledger();
        assert!(
            affine::radius(&sweep, 0.5, &mut short, max)
                .unwrap()
                .is_none()
        );
        assert!(short.cells <= max);
    }
    let mut center = crate::primitives::line([6., 8., 0.], [6., 8., 20.]).unwrap();
    center.knots = vec![17., 17., 29., 29.];
    let sweep = Sweep::new(&profile, &path, &uniform, &twist, options)
        .unwrap()
        .with_affine_laws(&axes, &center)
        .unwrap();
    let mut out = ledger();
    let rate = affine::axial_rate(&sweep, [0., 1.], &mut out, 1000)
        .unwrap()
        .unwrap();
    assert!(20. <= rate && rate < 20. + 1e-10);
    for max in [0, out.cells - 1] {
        let mut short = ledger();
        assert!(
            affine::axial_rate(&sweep, [0., 1.], &mut short, max)
                .unwrap()
                .is_none()
        );
        assert!(short.cells <= max);
    }
}

#[test]
fn original_uv_chart_can_prove_an_outward_offset_outside_the_containing_disk_focal_bound() {
    let path = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0.; 3], vec![0., 0., 1.], vec![0., 0.125, 2.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let profiles = [crate::primitives::line([-0.1, -100.1, 0.], [0.1, -100.1, 0.]).unwrap()];
    let cap = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![-0.1, -100.1, 0.], vec![-0.1, -99.9, 0.]],
            vec![vec![0.1, -100.1, 0.], vec![0.1, -99.9, 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let uniform = constant_vector_law([1., 0., 0.]).unwrap();
    let axes = constant_vector_law([1.25, 1., 2.]).unwrap();
    let center = constant_vector_law([0.; 3]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &uniform,
        &twist,
        Options {
            normal: [1., 0., 0.],
            orientation: Orientation::FixedNormal,
            spacing: Spacing::ArcLength {
                tolerance: 0.001,
                max_cells: 100000,
            },
            initial_sections: 3,
            max_sections: 9,
            max_deviation: 0.05,
        },
    )
    .unwrap()
    .with_affine_laws(&axes, &center)
    .unwrap();
    // The containing disk includes a focal point, although this actual
    // carrier lies entirely on the outward side of the bending path.
    assert!(
        !tube_bound_with_axial(
            I::point(0.),
            I::point(100.),
            I::point(2.),
            I::point(0.),
            I::point(0.125),
            I::point(1.),
            I::point(0.)
        )
        .unwrap()
    );
    let result = sweep
        .certify_ideal_volume_atlas(
            [&cap, &cap],
            [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            100000,
            1000000,
        )
        .unwrap();
    assert!(
        result.embedding_certified
            && result.regularity_certified
            && result.parameter_topology_preserved,
        "{result:?}"
    );
    eprintln!("original affine outward carrier uses actual UV chart: {result:?}");
}

#[test]
fn original_tilted_affine_tube_keeps_positive_rational_laws_g2_and_global_topology() {
    let path = source::path();
    let (profiles, cap, _) = source::material();
    let (axis, normal, twist) = source::laws();
    let (uniform, axes, center) = affine_case::laws();
    let before = (uniform.clone(), axes.clone(), center.clone());
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &uniform,
        &twist,
        Options {
            normal: [0., 0., 1.],
            orientation: Orientation::Fixed,
            spacing: Spacing::ArcLength {
                tolerance: 0.001,
                max_cells: 100000,
            },
            initial_sections: 5,
            max_sections: 65,
            max_deviation: 0.05,
        },
    )
    .unwrap()
    .with_frame_laws(&axis, &normal)
    .unwrap()
    .with_affine_laws(&axes, &center)
    .unwrap();
    let smooth = sweep
        .certify_original_closed_boundary_smoothness(
            &[4],
            super::super::super::OriginalBoundarySmoothnessLimits {
                order: 2,
                cells: 100000,
                exact_work: 1000000,
                products: 1000000,
            },
        )
        .unwrap();
    assert!(
        smooth.differential_compatibility_certified
            && smooth.frame_certified
            && smooth.laws_certified
            && smooth.profiles_certified,
        "{smooth:?}"
    );
    let global = sweep
        .certify_ideal_volume_atlas(
            [&cap, &cap],
            [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            1000000,
            1000000,
        )
        .unwrap();
    assert!(
        global.embedding_certified
            && global.regularity_certified
            && global.parameter_topology_preserved,
        "{global:?}"
    );
    assert!(global.cells > 0 && global.exact_work > 0);
    assert_eq!((uniform, axes, center), before);
    eprintln!("original tilted affine tube: {global:?}; original G2={smooth:?}");
}

#[test]
fn axial_center_cancellation_never_certifies_a_rank_two_body() {
    let path = crate::primitives::line([0.; 3], [0., 0., 2.]).unwrap();
    let profile = crate::primitives::line([-0.25, -0.25, 0.], [0.25, -0.25, 0.]).unwrap();
    let profiles = [profile];
    let cap = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![-0.25, -0.25, 0.], vec![-0.25, 0.25, 0.]],
            vec![vec![0.25, -0.25, 0.], vec![0.25, 0.25, 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let uniform = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
    let axes = constant_vector_law([1., 2., 3.]).unwrap();
    let mut center = crate::primitives::line([0.; 3], [0., 0., -2.]).unwrap();
    center.knots = vec![17., 17., 29., 29.];
    let twist = constant_vector_law([0.; 3]).unwrap();
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &uniform,
        &twist,
        Options {
            normal: [1., 0., 0.],
            orientation: Orientation::Fixed,
            spacing: Spacing::ArcLength {
                tolerance: 0.001,
                max_cells: 100000,
            },
            initial_sections: 3,
            max_sections: 9,
            max_deviation: 0.05,
        },
    )
    .unwrap()
    .with_affine_laws(&axes, &center)
    .unwrap();
    let result = sweep
        .certify_ideal_volume_atlas(
            [&cap, &cap],
            [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            3000,
            1000000,
        )
        .unwrap();
    assert!(
        !result.embedding_certified
            && !result.regularity_certified
            && !result.parameter_topology_preserved,
        "{result:?}"
    );
    assert!(result.cells <= 3000);
    assert!(
        !tube_bound_with_axial(
            I::point(0.),
            I::point(1.),
            I::point(2.),
            I::point(0.),
            I::point(0.),
            I::point(1.),
            I::point(2.)
        )
        .unwrap()
    );
    assert!(
        tube_bound_with_axial(
            I::point(0.),
            I::point(1.),
            I::point(2.),
            I::point(0.),
            I::point(0.),
            I::point(1.),
            I::point(1.)
        )
        .unwrap()
    );
}
