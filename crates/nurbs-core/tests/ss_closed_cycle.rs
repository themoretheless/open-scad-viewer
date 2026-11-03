use nurbs_core::ss_intersection as ss;
#[path = "support/ss_closed_fiber.rs"]
mod fixture;
#[test]
fn original_closed_component_has_an_oriented_first_return_cycle_and_shared_ports() {
    let source = fixture::circle();
    let before = (source.first().clone(), source.second().clone());
    let owner = ss::construct_qualified_closed_ss_branches(&source, Default::default()).unwrap();
    let cycle = ss::order_closed_fiber_cycle(&owner, 0, Default::default()).unwrap();
    assert!(cycle.arcs().len() >= 2);
    assert_eq!(cycle.arcs().len(), cycle.ports().len());
    assert!(
        cycle
            .arcs()
            .iter()
            .any(|a| a.fiber().free_axis() != cycle.arcs()[0].fiber().free_axis())
    );
    assert!(cycle.rectangles() <= 100000);
    cycle.reverify().unwrap();
    for port in cycle.ports() {
        port.selector().reverify().unwrap();
    }
    assert_eq!(before, (source.first().clone(), source.second().clone()));
}
#[test]
fn missing_cycle_work_or_unknown_components_cannot_publish_a_closed_traversal() {
    let owner =
        ss::construct_qualified_closed_ss_branches(&fixture::circle(), Default::default()).unwrap();
    assert!(ss::order_closed_fiber_cycle(&owner, 1, Default::default()).is_err());
    for limits in [
        ss::ClosedFiberCycleLimits {
            max_rectangles: 1,
            ..Default::default()
        },
        ss::ClosedFiberCycleLimits {
            max_steps: 2,
            ..Default::default()
        },
        ss::ClosedFiberCycleLimits {
            max_candidates: 1,
            ..Default::default()
        },
    ] {
        assert!(ss::order_closed_fiber_cycle(&owner, 0, limits).is_err());
    }
}

#[test]
fn whole_closed_cycle_saves_literal_paired_endpoints_with_original_root_bounds() {
    let owner =
        ss::construct_qualified_closed_ss_branches(&fixture::circle(), Default::default()).unwrap();
    let cycle = ss::order_closed_fiber_cycle(&owner, 0, Default::default()).unwrap();
    for limits in [
        ss::ClosedFiberTraceLimits {
            max_cells: 1,
            ..Default::default()
        },
        ss::ClosedFiberTraceLimits {
            max_evaluations: 1,
            ..Default::default()
        },
        ss::ClosedFiberTraceLimits {
            max_segments: 2,
            ..Default::default()
        },
    ] {
        assert!(ss::construct_closed_fiber_trace(&cycle, limits).is_err());
    }
    let trace = ss::construct_closed_fiber_trace(
        &cycle,
        ss::ClosedFiberTraceLimits {
            tolerance_world: 1e-4,
            tolerance_uv: 1e-4,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(trace.world_error_upper() <= 1e-4);
    assert!(trace.uv_error_upper().iter().all(|e| *e <= 1e-4));
    trace.reverify().unwrap();
    let segments = trace.segments();
    for i in 0..segments.len() {
        let a = &segments[i];
        let b = &segments[(i + 1) % segments.len()];
        assert_eq!(
            a.curve().control_points.last(),
            b.curve().control_points.first()
        );
        for k in 0..2 {
            assert_eq!(
                a.pcurves()[k].control_points.last(),
                b.pcurves()[k].control_points.first()
            );
        }
    }
}
#[test]
fn rational_projective_carriers_keep_original_closed_cycle_and_saved_trace() {
    let original = fixture::circle();
    let transform = |mut s: nurbs_core::surface::Surface| {
        for (row, weights) in s.control_points.iter_mut().zip(&mut s.weights) {
            for (p, w) in row.iter_mut().zip(weights) {
                let d = 1. + 0.25 * p[0] + 0.125 * p[1] + 0.125 * p[2];
                *w *= d;
                for x in p {
                    *x /= d;
                }
            }
        }
        s
    };
    let a = transform(original.first().clone());
    let b = transform(original.second().clone());
    let sources = ss::FiberSources::new(&a, &b).unwrap();
    let owner = ss::construct_qualified_closed_ss_branches(
        &sources,
        ss::ClosedSSBuildLimits {
            max_repairs: 32,
            ..Default::default()
        },
    )
    .unwrap();
    let cycle = ss::order_closed_fiber_cycle(&owner, 0, Default::default()).unwrap();
    let trace = ss::construct_closed_fiber_trace(
        &cycle,
        ss::ClosedFiberTraceLimits {
            tolerance_world: 1e-4,
            tolerance_uv: 1e-4,
            ..Default::default()
        },
    )
    .unwrap();
    trace.reverify().unwrap();
}
