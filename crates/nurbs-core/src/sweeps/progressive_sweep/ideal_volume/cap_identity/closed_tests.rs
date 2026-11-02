use super::*;
use crate as nurbs;
#[path = "../../../../../tests/support/general_frame_source.rs"]
mod source;
#[test]
fn exact_normalized_caps_keep_the_authored_g2_closed_original_quotient() {
    let path = source::path();
    let (axis, normal, twist) = source::laws();
    let (profiles, cap, _) = source::material();
    let mut equivalent = cap.clone();
    equivalent.knots_u = vec![17., 17., 29., 29.];
    equivalent.knots_v = vec![-7., -7., 13., 13.];
    equivalent
        .weights
        .iter_mut()
        .flatten()
        .for_each(|w| *w *= 2.);
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        Options {
            normal: [0., 0., 1.],
            orientation: Orientation::Fixed,
            spacing: Spacing::ArcLength {
                tolerance: 0.001,
                max_cells: 100000,
            },
            initial_sections: 33,
            max_sections: 129,
            max_deviation: 0.05,
        },
    )
    .unwrap()
    .with_frame_laws(&axis, &normal)
    .unwrap();
    let smooth = sweep.sweeps[0]
        .certify_closed_authored_frame_smoothness(2, 100000, 1000000)
        .unwrap();
    assert!(
        smooth.closed_source_frame_smoothness_certified,
        "{smooth:?}"
    );
    let before = (
        cap.clone(),
        equivalent.clone(),
        path.clone(),
        axis.clone(),
        normal.clone(),
        twist.clone(),
        profiles.clone(),
    );
    let projection = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    let run = |b: &Surface, cells, work| {
        sweep
            .certify_ideal_volume_atlas([&cap, b], projection, cells, work)
            .unwrap()
    };
    let base = run(&cap, 1000000, 1000000);
    let full = run(&equivalent, 1000000, 1000000);
    assert!(base.embedding_certified, "{base:?}");
    assert!(
        full.embedding_certified && full.regularity_certified && full.parameter_topology_preserved,
        "{full:?}"
    );
    let mut identity = IdealVolumeReport {
        cells: 0,
        exact_work: 0,
        ..full.clone()
    };
    assert!(certify([&cap, &equivalent], &mut identity, 100000, 1000000).is_some());
    assert_eq!(full.cells, base.cells + identity.cells);
    assert_eq!(full.exact_work, base.exact_work + identity.exact_work);
    let exact_budget = run(&equivalent, full.cells, full.exact_work);
    assert!(
        exact_budget.embedding_certified,
        "full={full:?}; exact={exact_budget:?}"
    );
    for (cells, work) in [
        (0, 1000000),
        (identity.cells - 1, full.exact_work),
        (full.cells, 0),
        (full.cells, identity.exact_work - 1),
    ] {
        let r = run(&equivalent, cells, work);
        assert!(
            !r.embedding_certified && !r.regularity_certified && !r.parameter_topology_preserved,
            "{r:?}"
        );
        assert!(r.cells <= cells && r.exact_work <= work);
    }
    // A 1-ULP endpoint-carrier change is genuine motion. The periodic theorem
    // must not replace it with a traversal-independent cap or a tolerance seam.
    for weight in [false, true] {
        let mut altered = equivalent.clone();
        if weight {
            altered.weights[0][0] = altered.weights[0][0].next_up();
        } else {
            altered.control_points[0][0][2] = altered.control_points[0][0][2].next_up();
        }
        let r = run(&altered, 1000000, 1000000);
        assert_eq!(
            r.reason,
            Some("ideal-volume-closed-common-carrier-required")
        );
        assert!(
            !r.embedding_certified && !r.regularity_certified && !r.parameter_topology_preserved
        );
    }
    assert_eq!(
        (cap, equivalent, path, axis, normal, twist, profiles),
        before
    );
    eprintln!(
        "exact normalized authored G2 closed original quotient: {full:?}; identity_cells={} identity_exact={}",
        identity.cells, identity.exact_work
    );
}
#[test]
fn different_original_tensor_partitions_keep_the_spatial_g2_frame_and_complete_closed_body() {
    let path = source::path();
    let (axis, normal, twist) = source::laws();
    let (profiles, mut a, _) = source::material();
    let r = 1. / 1024.;
    let xyz = |x: f64, y: f64| vec![-2. * r * x, -r * y, 9. * r * x];
    a.knots_u = vec![-3., -3., 1., 5., 5.];
    a.knots_v = vec![7., 7., 8., 10., 11., 11.];
    a.control_points = [-1., 0., 1.]
        .iter()
        .map(|&x| [-1., -0.5, 0.5, 1.].iter().map(|&y| xyz(x, y)).collect())
        .collect();
    a.weights = vec![vec![1.; 4]; 3];
    let b = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![17., 17., 17., 29., 29., 29.],
        knots_v: vec![-7., -7., -7., 13., 13., 13.],
        control_points: [-1., -0.5, 1.]
            .iter()
            .map(|&x| [-1., -0.5, 1.].iter().map(|&y| xyz(x, y)).collect())
            .collect(),
        weights: [1., 2., 3.]
            .iter()
            .map(|x| [1., 2., 3.].iter().map(|y| x * y).collect())
            .collect(),
        periodic_u: false,
        periodic_v: false,
    };
    a.validate().unwrap();
    b.validate().unwrap();
    let before = (
        a.clone(),
        b.clone(),
        path.clone(),
        axis.clone(),
        normal.clone(),
        twist.clone(),
        profiles.clone(),
    );
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        Options {
            normal: [0., 0., 1.],
            orientation: Orientation::Fixed,
            spacing: Spacing::ArcLength {
                tolerance: 0.001,
                max_cells: 100000,
            },
            initial_sections: 33,
            max_sections: 129,
            max_deviation: 0.05,
        },
    )
    .unwrap()
    .with_frame_laws(&axis, &normal)
    .unwrap();
    let smooth = sweep.sweeps[0]
        .certify_closed_authored_frame_smoothness(2, 100000, 1000000)
        .unwrap();
    assert!(
        smooth.closed_source_frame_smoothness_certified,
        "{smooth:?}"
    );
    let boundary = sweep
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
        boundary.differential_compatibility_certified
            && boundary.frame_certified
            && boundary.laws_certified
            && boundary.profiles_certified,
        "{boundary:?}"
    );
    let projection = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    let run = |b: &Surface, cells, work| {
        sweep
            .certify_ideal_volume_atlas([&a, b], projection, cells, work)
            .unwrap()
    };
    let base = run(&a, 1000000, 1000000);
    let full = run(&b, 1000000, 1000000);
    assert!(base.embedding_certified, "{base:?}");
    assert!(
        full.embedding_certified && full.regularity_certified && full.parameter_topology_preserved,
        "{full:?}"
    );
    let mut identity = IdealVolumeReport {
        cells: 0,
        exact_work: 0,
        ..full.clone()
    };
    assert!(certify([&a, &b], &mut identity, 100000, 1000000).is_some());
    assert_eq!(full.cells, base.cells + identity.cells);
    assert_eq!(full.exact_work, base.exact_work + identity.exact_work);
    let exact = run(&b, full.cells, full.exact_work);
    assert!(exact.embedding_certified, "{exact:?}");
    for (cells, work) in [
        (0, 1000000),
        (identity.cells - 1, 1000000),
        (full.cells, 0),
        (full.cells, identity.exact_work - 1),
    ] {
        let limited = run(&b, cells, work);
        assert!(
            !limited.embedding_certified
                && !limited.regularity_certified
                && !limited.parameter_topology_preserved,
            "{limited:?}"
        );
        assert!(limited.cells <= cells && limited.exact_work <= work);
    }
    let mut bad = b.clone();
    bad.control_points[1][1][2] = bad.control_points[1][1][2].next_up();
    let different = run(&bad, 1000000, 1000000);
    assert_eq!(
        different.reason,
        Some("ideal-volume-closed-common-carrier-required")
    );
    assert!(!different.embedding_certified);
    assert_eq!((a, b, path, axis, normal, twist, profiles), before);
    eprintln!(
        "different tensor partitions closed original body and G2 source frame: {full:?}; identity_cells={} identity_exact={}",
        identity.cells, identity.exact_work
    );
}
