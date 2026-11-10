use super::super::periodic_support_regions::tests as fixtures;
use super::*;
#[test]
fn real_hole_budget_exhaustion_retains_unknown_and_success_returns_replayable_regions() {
    let owner = fixtures::signatures(super::super::periodic_support_material::tests::with_hole());
    let stopped = construct_periodic_support_regions(
        owner.clone(),
        ParameterOrderLimits::default(),
        PeriodicRegionSearchLimits {
            max_work: 1,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(stopped.regions().is_none());
    assert_eq!(stopped.stop(), Some(PeriodicRegionSearchStop::WorkBudget));
    assert!(stopped.paths().is_empty());
    let done = construct_periodic_support_regions(
        owner,
        ParameterOrderLimits::default(),
        PeriodicRegionSearchLimits::default(),
    )
    .unwrap();
    assert_eq!(done.stop(), None);
    let regions = done.regions().unwrap();
    assert_eq!(regions.components().len(), 1);
    assert_eq!(done.paths().len(), 1);
    regions.reverify_source().unwrap();
}
#[test]
fn actual_cylinder_hole_grid_constructs_a_real_routed_positive_forest() {
    let lifts = super::super::periodic_support_path::tests::cylinder_with_hole();
    let owner = Arc::new(
        own_periodic_support_signatures(lifts, ParameterOrderLimits::default(), 10000).unwrap(),
    );
    let limited = construct_periodic_support_regions(
        owner.clone(),
        ParameterOrderLimits::default(),
        PeriodicRegionSearchLimits {
            max_grid_nodes: 1,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(limited.regions().is_none());
    assert_eq!(
        limited.stop(),
        Some(PeriodicRegionSearchStop::GridNodeBudget)
    );
    assert!(limited.paths().len() < 2);
    let r = construct_periodic_support_regions(
        owner,
        ParameterOrderLimits::default(),
        PeriodicRegionSearchLimits::default(),
    )
    .unwrap();
    assert_eq!(r.stop(), None);
    assert!(r.grids() > 0);
    assert!(r.visited_nodes() > 0);
    assert!(r.paths().iter().any(|p| p.path().len() > 2));
    let regions = r.regions().unwrap();
    assert_eq!(regions.components().len(), 1);
    assert_eq!(regions.components()[0].boundary_walks.len(), 3);
    regions.reverify_source().unwrap();
}
#[test]
fn actual_disconnected_essential_bands_are_not_joined_by_search() {
    let owner = Arc::new(
        own_periodic_support_signatures(
            fixtures::cylinder_bands(),
            ParameterOrderLimits::default(),
            10000,
        )
        .unwrap(),
    );
    let r = construct_periodic_support_regions(
        owner,
        ParameterOrderLimits::default(),
        PeriodicRegionSearchLimits::default(),
    )
    .unwrap();
    assert_eq!(r.stop(), None);
    assert_eq!(r.regions().unwrap().components().len(), 2);
    assert_eq!(r.paths().len(), 2);
    assert!(
        r.regions()
            .unwrap()
            .components()
            .iter()
            .all(|c| c.boundary_walks.len() == 2 && c.forest_paths.len() == 1)
    );
}
#[test]
fn actual_point_touching_lobes_need_no_false_connection_search() {
    let owner = fixtures::signatures(fixtures::discs(true));
    let r = construct_periodic_support_regions(
        owner,
        ParameterOrderLimits::default(),
        PeriodicRegionSearchLimits::default(),
    )
    .unwrap();
    assert_eq!(r.stop(), None);
    assert_eq!(r.regions().unwrap().components().len(), 2);
    assert!(r.paths().is_empty());
    assert_eq!(r.charged_work(), 0);
}

fn seam_annulus() -> Arc<OwnedPeriodicSupportSignatures> {
    let rim = Curve::from_polyline(vec![
        vec![0., 0., 0.],
        vec![1., 0., 0.],
        vec![1., 1., 0.],
        vec![0., 1., 0.],
        vec![0., 0., 0.],
    ])
    .unwrap();
    let surface = nurbs_core::surface::extrude(&rim, [0., 0., 1.]).unwrap();
    let curves = [
        Curve::from_polyline(vec![
            vec![-0.5, 0.],
            vec![0.5, 0.],
            vec![0.5, 1.],
            vec![-0.5, 1.],
            vec![-0.5, 0.],
        ])
        .unwrap(),
        Curve::from_polyline(vec![
            vec![0.25, 0.75],
            vec![0.25, 0.25],
            vec![-0.25, 0.25],
            vec![-0.25, 0.75],
            vec![0.25, 0.75],
        ])
        .unwrap(),
    ];
    let uses = (0..2)
        .map(|source| {
            vec![OriginalPeriodicSourceUse {
                source,
                reversed: false,
            }]
        })
        .collect::<Vec<_>>();
    let input = super::super::periodic_support_boundaries::tests::original(
        &surface,
        [true, false],
        &curves,
        &uses,
    );
    let support = Arc::new(
        own_periodic_support_boundaries(input, 1e-12, ParameterOrderLimits::default(), 64, 100000)
            .unwrap(),
    );
    let jumps =
        super::super::periodic_support_boundaries::tests::graph(support.augmented().clone());
    let a = [["1".into(), "1".into()], ["1".into(), "2".into()]];
    let base = Arc::new(
        own_periodic_relative_winding(
            jumps.clone(),
            &[a.clone(), a.clone()],
            ParameterOrderLimits::default(),
            10000,
        )
        .unwrap(),
    );
    let rule = Arc::new(
        own_periodic_support_material_rule(
            support.clone(),
            base,
            &[OriginalPeriodicOperand {
                wire_weights: vec![1, 1],
                base_winding: "0".into(),
                fill: OriginalPeriodicFillRule::NonZero,
            }],
            &[OriginalPeriodicBooleanOp::Operand(0)],
        )
        .unwrap(),
    );
    let mut sides = Vec::new();
    for edge in 0..jumps.owner().owner().edges().len() {
        let p = Arc::new(
            own_periodic_support_side_probes(
                support.clone(),
                jumps.clone(),
                edge,
                ["1".into(), "1024".into()],
                ParameterOrderLimits::default(),
            )
            .unwrap(),
        );
        let samples = std::array::from_fn(|s| {
            if p.exterior()[s] {
                None
            } else {
                Some(Arc::new(
                    own_periodic_material_sample(
                        rule.material().clone(),
                        Arc::new(
                            own_periodic_relative_winding(
                                jumps.clone(),
                                &[a.clone(), p.queries()[s].clone()],
                                ParameterOrderLimits::default(),
                                10000,
                            )
                            .unwrap(),
                        ),
                    )
                    .unwrap(),
                ))
            }
        });
        sides.push(Arc::new(
            own_periodic_support_sides(p, rule.clone(), samples).unwrap(),
        ));
    }
    let selection = Arc::new(own_periodic_support_selection(rule, &sides).unwrap());
    let charts = selection.rule().material().base().owner().owner().clone();
    let ports = Arc::new(own_periodic_ports(charts, 192).unwrap());
    let rotation =
        Arc::new(own_periodic_rotation(ports, ContactPortOrderLimits::default()).unwrap());
    let cycles = Arc::new(own_periodic_support_cycles(selection, rotation, 192, 128).unwrap());
    let lifts = Arc::new(own_periodic_support_lifts(cycles, 192, 128).unwrap());
    Arc::new(
        own_periodic_support_signatures(lifts, ParameterOrderLimits::default(), 10000).unwrap(),
    )
}
#[test]
fn actual_seam_split_annulus_needs_a_noncanonical_lift_to_connect_its_boundary_walks() {
    let o = seam_annulus();
    assert_eq!(o.probes().len(), 2);
    assert_eq!(o.probes()[0].signature, o.probes()[1].signature);
    let limits = PeriodicRegionSearchLimits {
        max_level: 5,
        max_period_radius: 0,
        max_grid_nodes: 65536,
        max_work: 1000000,
        max_region_work: 1000000,
    };
    let local =
        construct_periodic_support_regions(o.clone(), ParameterOrderLimits::default(), limits)
            .unwrap();
    assert!(local.regions().is_none());
    assert_eq!(
        local.stop(),
        Some(PeriodicRegionSearchStop::ResolutionLimit)
    );
    let r = construct_periodic_support_regions(
        o,
        ParameterOrderLimits::default(),
        PeriodicRegionSearchLimits {
            max_period_radius: 1,
            ..limits
        },
    )
    .unwrap();
    assert_eq!(r.stop(), None);
    assert_eq!(r.regions().unwrap().components().len(), 1);
    assert_eq!(r.paths().len(), 1);
    assert!(
        r.paths()[0].endpoint_period_shifts()[1]
            .iter()
            .any(|s| s != "0")
    );
    r.regions().unwrap().reverify_source().unwrap();
}
