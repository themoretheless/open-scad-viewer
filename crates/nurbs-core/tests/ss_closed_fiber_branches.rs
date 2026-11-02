use nurbs_core::ss_intersection::{self as ss, FiberSources};
use std::sync::Arc;
#[path = "support/ss_closed_fiber.rs"]
mod fixture;
#[path = "support/ss_discovered_sources.rs"]
mod open_fixture;
#[test]
fn full_original_circle_is_an_embedded_closed_branch_with_two_procedural_trims() {
    let source = fixture::circle();
    let charts = fixture::circle_family(&source);
    let owner = fixture::owner(&charts, None);
    let closed = ss::assemble_closed_fiber_branches(owner.clone(), Default::default()).unwrap();
    assert_eq!(closed.branches().len(), 1);
    assert_eq!(closed.branches()[0].charts.len(), 32);
    assert_eq!(closed.trims().len(), 2);
    assert_eq!(closed.trims()[0].surface, 0);
    assert_eq!(closed.trims()[1].surface, 1);
    assert!(closed.support_injectivity().iter().all(|r| r.proven));
    assert!(
        closed
            .root_enclosures()
            .iter()
            .all(|b| b.iter().all(|d| 0. < d[0] && d[1] < 1.))
    );
    assert!(Arc::ptr_eq(closed.owner(), &owner));
    closed.reverify().unwrap();
}
#[test]
fn closed_branch_stations_remain_original_root_selectors_and_reject_unowned_queries() {
    let source = fixture::circle();
    let charts = fixture::circle_family(&source);
    let closed =
        ss::assemble_closed_fiber_branches(fixture::owner(&charts, None), Default::default())
            .unwrap();
    for chart in (0..32).step_by(4) {
        let axis = charts[chart].free_axis();
        let free = match axis {
            ss::FiberAxis::FirstU => 0,
            ss::FiberAxis::FirstV => 1,
            _ => unreachable!(),
        };
        let d = charts[chart].domain()[free];
        let t = 0.5 * d[0] + 0.5 * d[1];
        let point = closed.point_at(0, chart, t, Default::default()).unwrap();
        point.reverify().unwrap();
        let image = point.report().root_image.unwrap();
        let other = if free == 0 { 1 } else { 0 };
        let index = (0..4)
            .filter(|&k| k != free)
            .position(|k| k == other)
            .unwrap();
        let mid = 0.5 * point.domain()[other][0] + 0.5 * point.domain()[other][1];
        let exact = 0.5 + (0.0625 - (t - 0.5).powi(2)).sqrt() * if mid > 0.5 { 1. } else { -1. };
        assert!(image[index][0] <= exact && exact <= image[index][1]);
    }
    assert!(closed.point_at(1, 0, 0.5, Default::default()).is_err());
    assert!(closed.point_at(0, 999, 0.5, Default::default()).is_err());
    assert!(closed.point_at(0, 0, -1., Default::default()).is_err());
}
#[test]
fn open_original_arcs_cannot_be_relabelled_as_closed_branches() {
    let source = open_fixture::sources(&[0.1875, -0.3125, 0.1875], 0.);
    let atlas = ss::discover_fiber_surfaces(&source, Default::default())
        .unwrap()
        .into_atlas()
        .unwrap();
    let owner = fixture::owner(atlas.fibers(), None);
    assert_eq!(owner.proof().components().len(), 2);
    let e = ss::assemble_closed_fiber_branches(owner, Default::default()).unwrap_err();
    assert!(e.to_string().contains("interior"), "{e}");
}
#[test]
fn local_product_coverage_and_invalid_work_never_publish_a_full_closed_census() {
    let source = fixture::circle();
    let charts = fixture::circle_family(&source);
    let local = fixture::owner(&charts, Some([[0.1, 0.9]; 4]));
    assert!(
        ss::assemble_closed_fiber_branches(local, Default::default())
            .unwrap_err()
            .to_string()
            .contains("entire")
    );
    let full = fixture::owner(&charts, None);
    for limits in [
        ss::ClosedFiberBranchLimits {
            max_branches: 0,
            ..Default::default()
        },
        ss::ClosedFiberBranchLimits {
            max_injectivity_spans_per_surface: 0,
            ..Default::default()
        },
        ss::ClosedFiberBranchLimits {
            injectivity_refinement_depth: 7,
            ..Default::default()
        },
    ] {
        assert!(ss::assemble_closed_fiber_branches(full.clone(), limits).is_err());
    }
    full.reverify().unwrap();
}
#[test]
fn source_wide_c1_gate_does_not_inherit_smoothness_from_root_samples() {
    let source = fixture::circle();
    let mut a = fixture::plane();
    a.knots_u = vec![0., 0., 0.125, 1., 1.];
    a.control_points = [(0., 0.001), (0.125, 0.), (1., 0.)]
        .map(|(x, z)| vec![vec![x, 0., z], vec![x, 1., z]])
        .to_vec();
    a.weights = vec![vec![1.; 2]; 3];
    let source = FiberSources::new(&a, source.second()).unwrap();
    let owner = fixture::owner(&fixture::circle_family(&source), None);
    let e = ss::assemble_closed_fiber_branches(owner, Default::default()).unwrap_err();
    assert!(e.to_string().contains("C1"), "{e}");
}
#[test]
fn folded_complete_carrier_never_inherits_world_injection_from_local_root_charts() {
    let source = fixture::circle();
    let mut a = fixture::plane();
    a.degree_u = 2;
    a.knots_u = vec![0., 0., 0., 0.125, 1., 1., 1.];
    a.control_points = [0.125, 0.0625, 0.5625, 1.]
        .map(|x| vec![vec![x, 0., 0.], vec![x, 1., 0.]])
        .to_vec();
    a.weights = vec![vec![1.; 2]; 4];
    let source = FiberSources::new(&a, source.second()).unwrap();
    let owner = fixture::owner(&fixture::circle_family(&source), None);
    let e = ss::assemble_closed_fiber_branches(owner, Default::default()).unwrap_err();
    assert!(e.to_string().contains("injection"), "{e}");
}
#[test]
fn two_original_closed_ovals_have_complete_separate_world_ownership() {
    let source = fixture::two_ovals();
    let charts = fixture::family(
        &source,
        &[
            ([1. / 6., 0.5], [1. / 12., (1. / 48_f64).sqrt()]),
            ([5. / 6., 0.5], [1. / 12., (1. / 48_f64).sqrt()]),
        ],
    );
    let owner = fixture::owner(&charts, None);
    assert_eq!(owner.proof().components().len(), 2);
    assert!(
        ss::assemble_closed_fiber_branches(
            owner.clone(),
            ss::ClosedFiberBranchLimits {
                max_branches: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    let closed = ss::assemble_closed_fiber_branches(owner, Default::default()).unwrap();
    assert_eq!(closed.branches().len(), 2);
    assert_eq!(closed.trims().len(), 4);
    assert!(closed.branches().iter().all(|b| b.charts.len() == 32));
    closed.reverify().unwrap();
}
