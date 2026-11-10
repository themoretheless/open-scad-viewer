use super::*;
fn point(x: &str, y: &str) -> [ExactRatio; 2] {
    [x, y].map(|s| {
        let (a, b) = s.split_once('/').unwrap_or((s, "1"));
        [a.to_string(), b.to_string()]
    })
}
#[test]
fn actual_cylinder_shore_owns_every_view_and_selects_a_complete_closed_material_boundary() {
    let _replay = super::super::periodic_replay::scope();
    let curve = Curve::from_polyline(vec![
        vec![0., 0.25],
        vec![1., 0.25],
        vec![1., 0.75],
        vec![0., 0.75],
        vec![0., 0.25],
    ])
    .unwrap();
    let owner = Arc::new(super::super::periodic_wire_jumps::tests::own(
        &[curve],
        &[vec![OriginalPeriodicSourceUse {
            source: 0,
            reversed: false,
        }]],
    ));
    let graph = owner.owner().owner();
    let cover = graph.owner().owner().pieces();
    let seam = graph
        .edges()
        .iter()
        .position(|e| {
            e.carriers
                .iter()
                .any(|c| cover.pieces()[c.piece].on_shore[0])
        })
        .unwrap();
    let base_point = point("2", "1/2");
    let base = Arc::new(
        own_periodic_relative_winding(
            owner.clone(),
            &[base_point.clone(), base_point.clone()],
            ParameterOrderLimits::default(),
            1000,
        )
        .unwrap(),
    );
    let rule = Arc::new(
        own_periodic_material_rule(
            base,
            &[OriginalPeriodicOperand {
                wire_weights: vec![1],
                base_winding: "0".into(),
                fill: OriginalPeriodicFillRule::NonZero,
            }],
            &[OriginalPeriodicBooleanOp::Operand(0)],
        )
        .unwrap(),
    );
    let mut sides = Vec::new();
    for edge in 0..graph.edges().len() {
        let probes = Arc::new(
            own_periodic_physical_side_probes(
                owner.clone(),
                edge,
                ["1".into(), "64".into()],
                ParameterOrderLimits::default(),
            )
            .unwrap(),
        );
        assert_eq!(probes.views().len(), 3 * cover.pieces().len());
        let samples = probes.queries().clone().map(|p| {
            Arc::new(
                own_periodic_material_sample(
                    rule.clone(),
                    Arc::new(
                        own_periodic_relative_winding(
                            owner.clone(),
                            &[base_point.clone(), p],
                            ParameterOrderLimits::default(),
                            1000,
                        )
                        .unwrap(),
                    ),
                )
                .unwrap(),
            )
        });
        if edge == seam {
            assert!(
                own_periodic_chart_side_probes(
                    owner.clone(),
                    edge,
                    ["1".into(), "64".into()],
                    ParameterOrderLimits::default()
                )
                .is_err()
            );
            assert!(probes.period_shifts().iter().any(|k| k[0] != "0"));
            let mut bad = (*probes).clone();
            bad.views.pop();
            assert!(bad.reverify_source().is_err());
            let mut bad = (*probes).clone();
            bad.period_shifts[1][0] = "0".into();
            assert!(bad.reverify_source().is_err());
        }
        sides.push(Arc::new(
            own_periodic_material_sides(probes, samples).unwrap(),
        ));
    }
    let selection = own_periodic_boundary_selection(rule, &sides).unwrap();
    assert_eq!(selection.selected().len(), 4);
    selection.reverify_source().unwrap();
}
#[test]
fn actual_torus_seam_preserves_individual_wire_deck_corrections_before_grouping() {
    let _replay = super::super::periodic_replay::scope();
    let surface = nurbs_core::primitives::torus([0.; 3], 3., 1., 1.).unwrap();
    let curves = [
        Curve::from_polyline(vec![vec![0., 0.], vec![0., 4.]]).unwrap(),
        Curve::from_polyline(vec![vec![0.5, 4.], vec![0.5, 0.]]).unwrap(),
    ];
    let owner = Arc::new(super::super::periodic_wire_jumps::tests::own_surface(
        &curves,
        &[
            vec![OriginalPeriodicSourceUse {
                source: 0,
                reversed: false,
            }],
            vec![OriginalPeriodicSourceUse {
                source: 1,
                reversed: false,
            }],
        ],
        &surface,
        [true, true],
        OriginalAtlasPairLimits {
            max_span_work: 1000000,
            refinement_depth: 4,
            ..Default::default()
        },
    ));
    let graph = owner.owner().owner();
    let cover = graph.owner().owner().pieces();
    let seam = graph
        .edges()
        .iter()
        .position(|e| {
            e.carriers
                .iter()
                .any(|c| cover.pieces()[c.piece].on_shore[0])
        })
        .unwrap();
    let probes = Arc::new(
        own_periodic_physical_side_probes(
            owner.clone(),
            seam,
            ["1".into(), "64".into()],
            ParameterOrderLimits::default(),
        )
        .unwrap(),
    );
    assert_eq!(probes.views().len(), 9 * cover.pieces().len());
    assert_eq!(probes.period_shifts()[0], ["-1", "0"]);
    assert_eq!(probes.jumps(), &["1", "0"]);
    let a = point("1/4", "1");
    let base = Arc::new(
        own_periodic_relative_winding(
            owner.clone(),
            &[a.clone(), a.clone()],
            ParameterOrderLimits::default(),
            1000,
        )
        .unwrap(),
    );
    let rule = Arc::new(
        own_periodic_material_rule(
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
    assert_eq!(rule.periods()[0].homology, ["0", "0"]);
    let samples = probes.queries().clone().map(|p| {
        Arc::new(
            own_periodic_material_sample(
                rule.clone(),
                Arc::new(
                    own_periodic_relative_winding(
                        owner.clone(),
                        &[a.clone(), p],
                        ParameterOrderLimits::default(),
                        1000,
                    )
                    .unwrap(),
                ),
            )
            .unwrap(),
        )
    });
    assert_eq!(samples[0].query().deltas(), &["0", "1"]);
    assert_eq!(samples[1].query().deltas(), &["0", "0"]);
    let sides = own_periodic_material_sides(probes, samples).unwrap();
    assert_eq!(
        sides.transported_wire_values(),
        &[
            vec!["1".to_string(), "0".to_string()],
            vec!["0".to_string(), "0".to_string()]
        ]
    );
    assert_eq!(
        sides.transported_operand_values(),
        &[vec!["1".to_string()], vec!["0".to_string()]]
    );
    assert_eq!(sides.boundary_direction(), Some(1));
    let mut bad = sides.clone();
    bad.wire_values[0][0] = "0".into();
    assert!(bad.reverify_source().is_err());
}
#[test]
fn actual_common_periodic_edge_keeps_two_minus_one_source_jumps_and_selects_one_boundary() {
    let line = |a, b| Curve::from_polyline(vec![vec![a, 0.5], vec![b, 0.5]]).unwrap();
    let owner = Arc::new(super::super::periodic_wire_jumps::tests::own(
        &[line(0., 8.), line(4., 0.)],
        &[
            vec![OriginalPeriodicSourceUse {
                source: 0,
                reversed: false,
            }],
            vec![OriginalPeriodicSourceUse {
                source: 1,
                reversed: false,
            }],
        ],
    ));
    let probes = Arc::new(
        own_periodic_chart_side_probes(
            owner.clone(),
            0,
            ["1".into(), "64".into()],
            ParameterOrderLimits::default(),
        )
        .unwrap(),
    );
    assert_eq!(probes.jumps(), &["2", "-1"]);
    probes.reverify_source().unwrap();
    let a = [["1".into(), "1".into()], ["1".into(), "8".into()]];
    let base = Arc::new(
        own_periodic_relative_winding(
            owner.clone(),
            &[a.clone(), a.clone()],
            ParameterOrderLimits::default(),
            100,
        )
        .unwrap(),
    );
    let rule = Arc::new(
        own_periodic_material_rule(
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
    let samples = probes.queries().clone().map(|p| {
        Arc::new(
            own_periodic_material_sample(
                rule.clone(),
                Arc::new(
                    own_periodic_relative_winding(
                        owner.clone(),
                        &[a.clone(), p],
                        ParameterOrderLimits::default(),
                        100,
                    )
                    .unwrap(),
                ),
            )
            .unwrap(),
        )
    });
    let sides = own_periodic_material_sides(probes.clone(), samples.clone()).unwrap();
    assert_eq!(sides.operand_jumps(), &["1"]);
    assert_eq!(sides.boundary_direction(), Some(1));
    sides.reverify_source().unwrap();
    let selection =
        own_periodic_boundary_selection(rule.clone(), &[Arc::new(sides.clone())]).unwrap();
    assert_eq!(
        selection.selected(),
        &[OriginalPeriodicSelectedEdge {
            edge: 0,
            direction: 1
        }]
    );
    selection.reverify_source().unwrap();
    let mut bad = selection.clone();
    bad.selected.clear();
    assert!(bad.reverify_source().is_err());
    assert!(own_periodic_boundary_selection(rule.clone(), &[]).is_err());
    let mut bad = (*probes).clone();
    bad.jumps[0] = "1".into();
    assert!(bad.reverify_source().is_err());
    let mut bad = sides.clone();
    bad.direction = Some(-1);
    assert!(bad.reverify_source().is_err());
    assert!(
        own_periodic_material_sides(probes.clone(), [samples[1].clone(), samples[0].clone()])
            .is_err()
    );
    assert!(
        own_periodic_chart_side_probes(
            owner,
            0,
            ["1".into(), "1".into()],
            ParameterOrderLimits::default()
        )
        .is_err()
    );
}
