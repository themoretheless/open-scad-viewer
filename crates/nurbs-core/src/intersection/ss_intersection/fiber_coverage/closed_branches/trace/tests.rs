use super::*;
use crate::ss_intersection::construct_qualified_closed_ss_branches;
fn circle() -> FiberSources {
    let a = Surface {
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
    let q = [0.25, -0.25, 0.25];
    let b = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| vec![i as f64 / 2., j as f64 / 2., q[i] + q[j] - 0.0625])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    FiberSources::new(&a, &b).unwrap()
}
#[test]
fn replay_rejects_changed_saved_errors_ports_order_and_tubes() {
    let owner = construct_qualified_closed_ss_branches(&circle(), Default::default()).unwrap();
    let cycle = order_closed_fiber_cycle(&owner, 0, Default::default()).unwrap();
    let t = construct_closed_fiber_trace(
        &cycle,
        ClosedFiberTraceLimits {
            tolerance_world: 1e-4,
            tolerance_uv: 1e-4,
            ..Default::default()
        },
    )
    .unwrap();
    let mut bad = t.clone();
    bad.world_error = 0.;
    assert!(bad.reverify().is_err());
    let mut bad = t.clone();
    bad.ports[0].world[0] += 1e-5;
    assert!(bad.reverify().is_err());
    let mut bad = t.clone();
    bad.segments.swap(0, 1);
    assert!(bad.reverify().is_err());
    let mut bad = t.clone();
    bad.tubes[0].world = 0.;
    assert!(bad.reverify().is_err());
    let mut bad = t.clone();
    bad.tubes[0].fibers.swap(0, 1);
    assert!(bad.reverify().is_err());
}
