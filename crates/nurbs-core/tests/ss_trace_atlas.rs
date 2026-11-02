use nurbs_core::ss_intersection::{self as ss, CoverageLimits, FiberBridgeSpec};
#[path = "support/ss_mixed_fiber.rs"]
mod fixture;
fn connected() -> (ss::QualifiedMixedFiberCoverage, Vec<FiberBridgeSpec>) {
    let fs = fixture::family();
    let cover = ss::cover_mixed_fiber_surfaces(&fs, CoverageLimits::default())
        .unwrap()
        .into_coverage()
        .unwrap();
    let specs = vec![
        FiberBridgeSpec {
            charts: [0, 2],
            domain: [
                [0.28125, 0.34375],
                [0.375, 0.625],
                [0.3125, 0.4375],
                [0.3, 0.7],
            ],
        },
        FiberBridgeSpec {
            charts: [1, 2],
            domain: [
                [0.65625, 0.71875],
                [0.375, 0.625],
                [0.3125, 0.4375],
                [0.3, 0.7],
            ],
        },
    ];
    (cover, specs)
}

#[test]
fn complete_original_roots_require_every_source_bound_trace_partition() {
    let (cover, specs) = connected();
    let owner = ss::connect_mixed_fiber_cover(&cover, &specs, 100000)
        .unwrap()
        .into_components()
        .unwrap();
    let pieces = cover
        .fibers()
        .iter()
        .map(|f| {
            let build = ss::construct_fiber_trace(
                f,
                ss::FiberTraceBuildLimits {
                    tolerance_world: 1e-6,
                    ..Default::default()
                },
            )
            .unwrap();
            build.trace.expect(build.reason)
        })
        .collect::<Vec<_>>();
    let atlas = ss::qualify_fiber_trace_atlas(&owner, &pieces, 1e-6).unwrap();
    atlas.reverify().unwrap();
    assert!(atlas.maximum_world_error() <= 1e-6);
    assert_eq!(atlas.pieces().len(), 3);
    assert_eq!(atlas.owner().components(), &[vec![0, 1, 2]]);
    assert!(ss::qualify_fiber_trace_atlas(&owner, &pieces[..2], 1e-6).is_err());
    let mut missing = pieces.clone();
    missing[0].clear();
    assert!(ss::qualify_fiber_trace_atlas(&owner, &missing, 1e-6).is_err());
    let mut duplicate = pieces.clone();
    duplicate[0].insert(0, pieces[0][0].clone());
    assert!(ss::qualify_fiber_trace_atlas(&owner, &duplicate, 1e-6).is_err());
    let mut foreign = pieces.clone();
    foreign.swap(0, 2);
    assert!(ss::qualify_fiber_trace_atlas(&owner, &foreign, 1e-6).is_err());
    let mut shortened = pieces.clone();
    shortened[0].pop();
    assert!(ss::qualify_fiber_trace_atlas(&owner, &shortened, 1e-6).is_err());
    assert!(ss::qualify_fiber_trace_atlas(&owner, &pieces, f64::NAN).is_err());
    // A complete local root chart must not masquerade as whole-surface coverage.
    let first = &cover.fibers()[0];
    let partial =
        ss::cover_mixed_fiber_box(&[first.clone()], first.domain(), CoverageLimits::default())
            .unwrap()
            .into_coverage()
            .unwrap();
    let partial_owner = ss::connect_mixed_fiber_cover(&partial, &[], 100000)
        .unwrap()
        .into_components()
        .unwrap();
    assert!(ss::qualify_fiber_trace_atlas(&partial_owner, &pieces[..1], 1e-6).is_err());
}
