use nurbs_core::ss_intersection::{self as ss, FiberSources};
use std::sync::Arc;
#[path = "support/ss_discovered_sources.rs"]
mod fixture;
#[path = "support/ss_mixed_fiber.rs"]
mod mixed;
fn components(s: &FiberSources) -> Arc<ss::QualifiedDiscoveredFiberComponents> {
    let atlas = ss::discover_fiber_surfaces(s, Default::default())
        .unwrap()
        .into_atlas()
        .expect("complete atlas");
    Arc::new(
        ss::connect_discovered_fiber_atlas(Arc::new(atlas), Default::default())
            .unwrap()
            .into_components()
            .expect("complete parameter components"),
    )
}
#[test]
fn actual_two_roots_publish_disjoint_world_arcs_and_both_original_trim_projections() {
    let c = components(&fixture::sources(&[0.1875, -0.3125, 0.1875], 0.));
    let o = ss::assemble_physical_fiber_branches(c.clone(), Default::default()).unwrap();
    assert!(o.covers_entire_surfaces());
    assert_eq!(o.branches().len(), 2);
    assert_eq!(o.vertices().len(), 4);
    assert_eq!(o.trims().len(), 4);
    for (i, b) in o.branches().iter().enumerate() {
        assert_eq!(b.domain, [0., 1.]);
        for end in 0..2 {
            let v = &o.vertices()[b.vertices[end]];
            assert_eq!(v.branch(), i);
            assert_eq!(v.end(), end);
            let free = match b.free_axis {
                ss::FiberAxis::FirstU => 0,
                ss::FiberAxis::FirstV => 1,
                ss::FiberAxis::SecondU => 2,
                ss::FiberAxis::SecondV => 3,
            };
            assert_eq!(v.point().domain()[free], [b.domain[end]; 2]);
            v.point().reverify().unwrap();
        }
        assert_eq!(o.trims()[2 * i].surface, 0);
        assert_eq!(o.trims()[2 * i + 1].surface, 1);
    }
    o.reverify().unwrap();
    assert!(
        ss::assemble_physical_fiber_branches(
            c,
            ss::PhysicalFiberBranchLimits {
                max_branches: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
}
#[test]
fn nonlinear_original_world_arc_is_not_replaced_by_a_sampled_polyline() {
    let o = ss::assemble_physical_fiber_branches(components(&mixed::sources()), Default::default())
        .unwrap();
    assert_eq!(o.branches().len(), 1);
    assert_eq!(o.trims().len(), 2);
    assert!(o.branches()[0].charts.len() > 1);
    o.reverify().unwrap();
}
#[test]
fn folded_support_parameter_component_never_becomes_an_embedded_physical_arc() {
    let s = fixture::sources(&[-0.25, 0.75], 0.);
    let mut a = s.first().clone();
    a.degree_u = 2;
    a.knots_u = vec![0., 0., 0., 1., 1., 1.];
    a.control_points = [0.25, 0.5, 0.25]
        .map(|x| vec![vec![x, 0., -0.25], vec![x, 1., 0.75]])
        .to_vec();
    a.weights = vec![vec![1.; 2]; 3];
    let folded = FiberSources::new(&a, s.second()).unwrap();
    let c = components(&folded);
    assert_eq!(c.components().len(), 1);
    assert!(ss::assemble_physical_fiber_branches(c, Default::default()).is_err());
}
#[test]
fn original_empty_intersection_has_no_physical_vertices_or_trim_uses() {
    let o = ss::assemble_physical_fiber_branches(
        components(&fixture::sources(&[-0.25, 0.75], 2.)),
        Default::default(),
    )
    .unwrap();
    assert!(o.branches().is_empty() && o.vertices().is_empty() && o.trims().is_empty());
    o.reverify().unwrap();
}

#[test]
fn exact_branch_stations_enclose_independent_analytic_roots_and_reject_unowned_queries() {
    let o = ss::assemble_physical_fiber_branches(
        components(&fixture::sources(&[0.1875, -0.3125, 0.1875], 0.)),
        Default::default(),
    )
    .unwrap();
    let mut found = vec![];
    for i in 0..2 {
        let mut chosen = None;
        for t in [0., 0.125, 0.5, 0.875, 1.] {
            let p = o.station(i, t, 10000).unwrap();
            assert_eq!(p.free_axis(), ss::FiberAxis::FirstU);
            let image = p.report().root_image.unwrap();
            let v = if image[0][1] < 0.5 { 0.25 } else { 0.75 };
            if let Some(previous) = chosen {
                assert_eq!(v, previous);
            }
            chosen = Some(v);
            // Independent algebraic roots of v^2-v+3/16 and x=1/4+u/4.
            for (bound, value) in image.iter().zip([v, 0.25 + 0.25 * t, v]) {
                assert!(
                    bound[0] <= value && value <= bound[1],
                    "{bound:?} misses {value}"
                );
            }
        }
        found.push(chosen.unwrap());
    }
    found.sort_by(f64::total_cmp);
    assert_eq!(found, vec![0.25, 0.75]);
    for t in [f64::NAN, f64::INFINITY, -0.01, 1.01] {
        assert!(o.station(0, t, 10000).is_err());
    }
    assert!(o.station(2, 0.5, 10000).is_err());
    assert!(o.station(0, 0.5, 0).is_err());
    assert!(o.station(0, 0.5, 1).is_err());
}
