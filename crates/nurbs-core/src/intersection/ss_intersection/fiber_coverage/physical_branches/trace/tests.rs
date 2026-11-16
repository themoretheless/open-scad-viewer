use super::*;
use crate::intersection::ss_intersection::{
    QualifiedSurfaceIntersection, construct_qualified_ss_branches,
};

fn sources() -> FiberSources {
    let a = Surface {
        degree_u: 1,
        degree_v: 2,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: [0.25, 0.5]
            .into_iter()
            .map(|x| {
                [0.1875, -0.3125, 0.1875]
                    .into_iter()
                    .enumerate()
                    .map(|(j, z)| vec![x, j as f64 / 2., z])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let b = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    FiberSources::new(&a, &b).unwrap()
}

fn trace() -> QualifiedPhysicalFiberTrace {
    let dispatched = construct_qualified_ss_branches(&sources(), Default::default()).unwrap();
    let QualifiedSurfaceIntersection::RegularOpenArcs(owner) = dispatched else {
        panic!("Expected regular open arcs")
    };
    construct_physical_fiber_trace(&owner, 0, Default::default()).unwrap()
}

#[test]
fn replay_rejects_changed_ports_segments_tubes_and_saved_errors() {
    let good = trace();
    good.reverify().unwrap();
    let replay_cells = good.reverify_with_budget(good.limits.max_cells).unwrap();
    assert!(replay_cells > 0 && replay_cells <= good.limits.max_cells);
    assert_eq!(
        good.reverify_with_budget(good.limits.max_cells).unwrap(),
        replay_cells
    );
    assert!(good.reverify_with_budget(0).is_err());
    assert!(good.reverify_with_budget(replay_cells - 1).is_err());

    let mut bad = good.clone();
    bad.ports[0].parameters[0] += 1e-8;
    assert!(bad.reverify().is_err());

    let mut bad = good.clone();
    bad.ports[0].bounds[0][1] += 1e-8;
    assert!(bad.reverify().is_err());

    let mut bad = good.clone();
    bad.ports[0].world[0] += 1e-8;
    assert!(bad.reverify().is_err());

    let mut bad = good.clone();
    bad.segments.remove(0);
    assert!(bad.reverify().is_err());

    let mut bad = good.clone();
    bad.tubes[0].world = 0.;
    assert!(bad.reverify().is_err());

    let mut bad = good.clone();
    bad.world_error = 0.;
    assert!(bad.reverify().is_err());

    let mut bad = good.clone();
    bad.uv_errors[0] = 0.;
    assert!(bad.reverify().is_err());
}

#[test]
fn invalid_budgets_and_unknown_branches_publish_no_partial_trace() {
    let dispatched = construct_qualified_ss_branches(&sources(), Default::default()).unwrap();
    let QualifiedSurfaceIntersection::RegularOpenArcs(owner) = dispatched else {
        panic!("Expected regular open arcs")
    };
    assert!(
        construct_physical_fiber_trace(
            &owner,
            0,
            PhysicalFiberTraceLimits {
                max_cells: 1,
                ..Default::default()
            },
        )
        .is_err()
    );
    assert!(
        construct_physical_fiber_trace(
            &owner,
            0,
            PhysicalFiberTraceLimits {
                max_segments: 0,
                ..Default::default()
            },
        )
        .is_err()
    );
    assert!(
        construct_physical_fiber_trace(&owner, owner.branches().len(), Default::default()).is_err()
    );

    let high_evaluation_budget = construct_physical_fiber_trace(
        &owner,
        0,
        PhysicalFiberTraceLimits {
            max_evaluations: 16385,
            ..Default::default()
        },
    )
    .unwrap();
    high_evaluation_budget.reverify().unwrap();
}
