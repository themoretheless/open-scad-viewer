use nurbs_core::ss_intersection::{
    self as ss, CoverageLimits, FiberAxis, FiberBridgeSpec, FiberLimits, FiberSources,
};
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
fn complete_curved_mixed_cover_has_one_owned_parameter_component() {
    let (cover, specs) = connected();
    let r = ss::connect_mixed_fiber_cover(&cover, &specs, 100000).unwrap();
    let owned = r.components().expect(r.reason());
    owned.reverify().unwrap();
    assert_eq!(owned.components(), &[vec![0, 1, 2]]);
    assert_eq!(owned.transitions().len(), 2);
    assert!(owned.separations().is_empty());
    // Omitting a genuine bridge cannot manufacture two disjoint components.
    let missing = ss::connect_mixed_fiber_cover(&cover, &specs[..1], 100000).unwrap();
    assert!(missing.components().is_none());
    assert_eq!(missing.reason(), "cross-component-root-contact-unproven");
    let mut reverse = specs.clone();
    reverse.reverse();
    let permuted = ss::connect_mixed_fiber_cover(&cover, &reverse, 100000).unwrap();
    assert_eq!(
        permuted.components().unwrap().components(),
        owned.components()
    );
}
#[test]
fn actual_distinct_root_components_have_complete_cross_component_separation() {
    let originals = fixture::sources();
    let mut a = originals.first().clone();
    let mut b = originals.second().clone();
    for row in &mut a.control_points {
        for (j, p) in row.iter_mut().enumerate() {
            p[2] = [0.1875, -0.3125, 0.1875][j];
        }
    }
    for row in &mut b.control_points {
        for p in row {
            p[2] = 0.;
        }
    }
    let source = FiberSources::new(&a, &b).unwrap();
    let fibers = [([0.2, 0.3], [0.1, 0.4]), ([0.7, 0.8], [0.6, 0.9])]
        .into_iter()
        .map(|(v, t)| {
            source
                .certify(
                    [[0., 1.], v, [0.2, 0.55], t],
                    FiberAxis::FirstU,
                    FiberLimits::default(),
                )
                .unwrap()
                .into_fiber()
                .unwrap()
        })
        .collect::<Vec<_>>();
    let cover = ss::cover_mixed_fiber_surfaces(&fibers, CoverageLimits::default())
        .unwrap()
        .into_coverage()
        .unwrap();
    let r = ss::connect_mixed_fiber_cover(&cover, &[], 100000).unwrap();
    let owned = r.components().expect(r.reason());
    owned.reverify().unwrap();
    assert_eq!(owned.components(), &[vec![0], vec![1]]);
    assert_eq!(owned.separations().len(), 1);
    let proof = &owned.separations()[0];
    assert_eq!(proof.charts, [0, 1]);
    assert!(proof.intervals[0][1] < proof.intervals[1][0]);
}
#[test]
fn coverage_and_bridges_share_one_original_work_budget() {
    let (cover, specs) = connected();
    let r = ss::connect_mixed_fiber_cover(&cover, &specs, 100000).unwrap();
    let exact = r.rectangles();
    assert!(exact > cover.report().rectangles);
    ss::connect_mixed_fiber_cover(&cover, &specs, exact)
        .unwrap()
        .components()
        .unwrap()
        .reverify()
        .unwrap();
    for budget in [1, cover.report().rectangles, exact - 1] {
        let short = ss::connect_mixed_fiber_cover(&cover, &specs, budget).unwrap();
        assert!(short.components().is_none());
        assert!(short.rectangles() <= budget);
    }
    let mut wrong = specs;
    wrong[0].charts = [0, 99];
    assert!(ss::connect_mixed_fiber_cover(&cover, &wrong, 100000).is_err());
}
