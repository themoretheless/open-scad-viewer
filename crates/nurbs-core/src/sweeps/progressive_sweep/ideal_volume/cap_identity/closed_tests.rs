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
