use nurbs_core::ss_intersection::{self as ss, DiscoveredComponentLimits, FiberDiscoveryLimits};
use std::sync::Arc;
#[path = "support/ss_discovered_sources.rs"]
mod fixture;
#[path = "support/ss_mixed_fiber.rs"]
mod mixed;
#[test]
fn automatic_two_root_components_connect_closed_chart_endpoints_and_separate_all_other_pairs() {
    let source = fixture::sources(&[0.1875, -0.3125, 0.1875], 0.);
    let atlas = Arc::new(
        ss::discover_fiber_surfaces(&source, FiberDiscoveryLimits::default())
            .unwrap()
            .into_atlas()
            .unwrap(),
    );
    let r = ss::connect_discovered_fiber_atlas(atlas.clone(), DiscoveredComponentLimits::default())
        .unwrap();
    let o = r
        .components()
        .unwrap_or_else(|| panic!("{} rectangles={}", r.reason(), r.rectangles()));
    assert_eq!(o.components().len(), 2);
    assert!(!o.connections().is_empty());
    assert!(!o.separations().is_empty());
    for c in o.connections() {
        let free = match c.point().free_axis() {
            ss::FiberAxis::FirstU => 0,
            ss::FiberAxis::FirstV => 1,
            ss::FiberAxis::SecondU => 2,
            ss::FiberAxis::SecondV => 3,
        };
        assert_eq!(c.point().domain()[free][0], c.point().domain()[free][1]);
        for i in c.charts() {
            assert!((0..4).all(
                |k| atlas.fibers()[i].domain()[k][0] <= c.point().domain()[k][0]
                    && c.point().domain()[k][1] <= atlas.fibers()[i].domain()[k][1]
            ));
        }
        c.point().reverify().unwrap();
    }
    o.reverify().unwrap();
    let low = ss::connect_discovered_fiber_atlas(
        atlas,
        DiscoveredComponentLimits {
            max_rectangles: 1,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(low.components().is_none());
    assert!(low.rectangles() <= 1);
}
#[test]
fn automatic_nonlinear_component_needs_no_authored_bridge() {
    let atlas = Arc::new(
        ss::discover_fiber_surfaces(&mixed::sources(), FiberDiscoveryLimits::default())
            .unwrap()
            .into_atlas()
            .unwrap(),
    );
    let r =
        ss::connect_discovered_fiber_atlas(atlas, DiscoveredComponentLimits::default()).unwrap();
    let o = r
        .components()
        .unwrap_or_else(|| panic!("{} rectangles={}", r.reason(), r.rectangles()));
    assert_eq!(o.components().len(), 1);
    o.reverify().unwrap();
}
#[test]
fn automatic_empty_and_single_chart_component_census_remain_source_owned() {
    for (level, count) in [(0., 1), (2., 0)] {
        let atlas = Arc::new(
            ss::discover_fiber_surfaces(
                &fixture::sources(&[-0.25, 0.75], level),
                FiberDiscoveryLimits::default(),
            )
            .unwrap()
            .into_atlas()
            .unwrap(),
        );
        let r = ss::connect_discovered_fiber_atlas(atlas, DiscoveredComponentLimits::default())
            .unwrap();
        let o = r.components().unwrap();
        assert_eq!(o.components().len(), count);
        o.reverify().unwrap();
    }
}
