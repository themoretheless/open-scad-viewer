use nurbs_core::ss_intersection::{
    self as ss, CoverageLimits, FiberCoverageStatus, FiberDiscoveryLimits, FiberSources,
};
#[path = "support/ss_discovered_sources.rs"]
mod fixture;
use fixture::sources;

#[test]
fn automatic_original_fiber_needs_no_authored_seed_or_chart_in_either_order() {
    let source = sources(&[-0.25, 0.75], 0.);
    for source in [
        source.clone(),
        FiberSources::new(source.second(), source.first()).unwrap(),
    ] {
        let r = ss::discover_fiber_surfaces(&source, FiberDiscoveryLimits::default()).unwrap();
        assert_eq!(
            r.report().status,
            FiberCoverageStatus::Complete,
            "{:?}",
            r.report()
        );
        let a = r.into_atlas().unwrap();
        assert!(a.covers_entire_surfaces());
        assert!(!a.fibers().is_empty());
        a.reverify().unwrap();
        assert!(
            a.fibers()
                .iter()
                .all(|f| f.report().support_regularity_proven == [true, true])
        );
    }
}
#[test]
fn automatic_empty_set_has_a_complete_source_owned_exclusion_partition() {
    let r = ss::discover_fiber_surfaces(
        &sources(&[-0.25, 0.75], 2.),
        FiberDiscoveryLimits::default(),
    )
    .unwrap();
    assert_eq!(r.report().status, FiberCoverageStatus::Complete);
    let a = r.into_atlas().unwrap();
    assert!(a.fibers().is_empty());
    assert_eq!(a.nodes().len(), 1);
    a.reverify().unwrap();
}
#[test]
fn automatic_two_components_cover_both_exact_roots_and_shared_cell_faces() {
    let source = sources(&[0.1875, -0.3125, 0.1875], 0.);
    let r = ss::discover_fiber_surfaces(&source, FiberDiscoveryLimits::default()).unwrap();
    assert_eq!(
        r.report().status,
        FiberCoverageStatus::Complete,
        "{:?}",
        r.report()
    );
    let a = r.into_atlas().unwrap();
    a.reverify().unwrap();
    assert!(a.fibers().len() >= 2);
    // Independent polynomial oracle: v^2-v+3/16 has precisely these two roots.
    for v in [0.25, 0.75] {
        for u in [0., 0.125, 0.5, 0.875, 1.] {
            let p = [u, v, 0.25 + 0.25 * u, v];
            assert!(
                a.fibers()
                    .iter()
                    .any(|f| (0..4).all(|k| f.domain()[k][0] <= p[k] && p[k] <= f.domain()[k][1]))
            );
            assert!(
                source
                    .first()
                    .evaluate(u, v)
                    .unwrap()
                    .point
                    .iter()
                    .zip(source.second().evaluate(p[2], p[3]).unwrap().point)
                    .all(|(x, y)| (x - y).abs() < 1e-13)
            );
        }
    }
}
#[test]
fn discovery_budget_or_tangency_never_authorizes_a_partial_atlas() {
    let source = sources(&[0.1875, -0.3125, 0.1875], 0.);
    for limits in [
        FiberDiscoveryLimits {
            coverage: CoverageLimits {
                max_rectangles: 1,
                ..Default::default()
            },
            ..Default::default()
        },
        FiberDiscoveryLimits {
            coverage: CoverageLimits {
                max_cells: 1,
                ..Default::default()
            },
            ..Default::default()
        },
        FiberDiscoveryLimits {
            coverage: CoverageLimits {
                max_depth: 0,
                ..Default::default()
            },
            ..Default::default()
        },
        FiberDiscoveryLimits {
            max_fibers: 1,
            ..Default::default()
        },
    ] {
        let r = ss::discover_fiber_surfaces(&source, limits).unwrap();
        assert!(r.atlas().is_none(), "{:?}", r.report());
        assert_ne!(r.report().status, FiberCoverageStatus::Complete);
        assert!(r.report().rectangles <= limits.coverage.max_rectangles);
    }
    let tangent = sources(&[0.25, -0.25, 0.25], 0.);
    let r = ss::discover_fiber_surfaces(
        &tangent,
        FiberDiscoveryLimits {
            coverage: CoverageLimits {
                max_cells: 127,
                max_depth: 8,
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap();
    assert!(r.atlas().is_none());
    assert_ne!(r.report().status, FiberCoverageStatus::Complete);
}
#[path = "support/ss_mixed_fiber.rs"]
mod mixed;
#[test]
fn automatic_nonlinear_original_graph_discovers_its_own_overlapping_charts() {
    let source = mixed::sources();
    let r = ss::discover_fiber_surfaces(&source, FiberDiscoveryLimits::default()).unwrap();
    assert_eq!(
        r.report().status,
        FiberCoverageStatus::Complete,
        "{:?}",
        r.report()
    );
    let atlas = r.into_atlas().unwrap();
    atlas.reverify().unwrap();
    // Reuse in the existing independently replayed mixed-family coverage API.
    let second = ss::cover_mixed_fiber_surfaces(atlas.fibers(), CoverageLimits::default()).unwrap();
    assert_eq!(
        second.report().status,
        FiberCoverageStatus::Complete,
        "{:?}",
        second.report()
    );
    second.coverage().unwrap().reverify().unwrap();
}
