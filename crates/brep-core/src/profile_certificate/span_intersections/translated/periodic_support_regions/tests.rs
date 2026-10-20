use super::*;
fn lifts(selection: Arc<OwnedPeriodicSupportSelection>) -> Arc<OwnedPeriodicSupportLifts> {
    let _replay = super::super::periodic_replay::scope();
    let charts = selection.rule().material().base().owner().owner().clone();
    let ports = Arc::new(own_periodic_ports(charts, 192).unwrap());
    let rotation =
        Arc::new(own_periodic_rotation(ports, ContactPortOrderLimits::default()).unwrap());
    Arc::new(
        own_periodic_support_lifts(
            Arc::new(own_periodic_support_cycles(selection, rotation, 192, 128).unwrap()),
            192,
            128,
        )
        .unwrap(),
    )
}
pub(in crate::profile_certificate::span_intersections::translated) fn signatures(
    selection: Arc<OwnedPeriodicSupportSelection>,
) -> Arc<OwnedPeriodicSupportSignatures> {
    Arc::new(
        own_periodic_support_signatures(lifts(selection), ParameterOrderLimits::default(), 10000)
            .unwrap(),
    )
}
fn point(o: &OwnedPeriodicSupportSignatures, i: usize) -> [ExactRatio; 2] {
    let [edge, side] = o.probes()[i].endpoint;
    o.owner().owner().owner().sides()[edge].probes().queries()[side].clone()
}
pub(in crate::profile_certificate::span_intersections::translated) fn connection(
    o: &Arc<OwnedPeriodicSupportSignatures>,
    a: usize,
    b: usize,
    middle: &[[ExactRatio; 2]],
) -> Arc<OwnedPeriodicSupportPath> {
    let mut path = vec![point(o, a)];
    path.extend_from_slice(middle);
    path.push(point(o, b));
    Arc::new(
        own_periodic_support_path(
            o.owner().clone(),
            [o.probes()[a].endpoint, o.probes()[b].endpoint],
            &path,
            ParameterOrderLimits::default(),
            10000,
        )
        .unwrap(),
    )
}
#[test]
fn exact_kernel_spans_zero_rank_one_and_rank_two_even_without_integer_saturation() {
    let q = |n: i64| BigInt::from(n);
    for h in [
        vec![],
        vec![[q(0), q(0)], [q(0), q(0)]],
        vec![[q(2), q(0)], [q(4), q(0)], [q(0), q(0)]],
        vec![[q(2), q(0)], [q(0), q(3)], [q(1), q(1)], [q(-1), q(2)]],
    ] {
        let rank = if h.iter().all(|v| v.iter().all(BigInt::is_zero)) {
            0
        } else if h.iter().any(|a| h.iter().any(|b| !det(a, b).is_zero())) {
            2
        } else {
            1
        };
        let rows = kernel(&h);
        assert_eq!(rows.len(), h.len() - rank);
        for row in &rows {
            for axis in 0..2 {
                assert_eq!(
                    row.iter().map(|(i, c)| c * &h[*i][axis]).sum::<BigInt>(),
                    q(0)
                );
            }
        }
        // Independent free columns: each row has its own unique free index;
        // find an exact nonzero maximal square minor over rational arithmetic.
        if !rows.is_empty() {
            let mut matrix = rows
                .iter()
                .map(|r| {
                    let mut v = vec![Q::zero(); h.len()];
                    for (i, c) in r {
                        v[*i] = Q::from_integer(c.clone());
                    }
                    v
                })
                .collect::<Vec<_>>();
            let mut pivot = 0;
            for col in 0..h.len() {
                if let Some(p) = (pivot..matrix.len()).find(|i| !matrix[*i][col].is_zero()) {
                    matrix.swap(pivot, p);
                    let scale = matrix[pivot][col].clone();
                    for x in &mut matrix[pivot] {
                        *x /= &scale;
                    }
                    let row = matrix[pivot].clone();
                    for i in pivot + 1..matrix.len() {
                        let scale = matrix[i][col].clone();
                        for j in col..h.len() {
                            matrix[i][j] -= &scale * &row[j];
                        }
                    }
                    pivot += 1;
                    if pivot == matrix.len() {
                        break;
                    }
                }
            }
            assert_eq!(pivot, rows.len());
        }
    }
}
#[test]
fn actual_plane_and_empty_fill_have_complete_partitions_and_replay_rejects_forgery() {
    let full = signatures(super::super::periodic_support_material::tests::full_plane(
        true,
    ));
    assert_eq!(full.probes().len(), 1);
    assert_eq!(full.probes()[0].signature, vec!["0"]);
    let region = own_periodic_support_regions(full.clone(), &[], 10000).unwrap();
    assert_eq!(region.components().len(), 1);
    assert_eq!(region.components()[0].boundary_walks, vec![0]);
    region.reverify_source().unwrap();
    let mut bad = (*full).clone();
    bad.data.cochains[0].terms[0].coefficient = "2".into();
    assert!(bad.reverify_source().is_err());
    let mut bad = (*full).clone();
    bad.data.probes[0].signature[0] = "1".into();
    assert!(bad.reverify_source().is_err());
    let mut bad = region;
    bad.data.components[0].boundary_walks.clear();
    assert!(bad.reverify_source().is_err());
    let empty = signatures(super::super::periodic_support_material::tests::full_plane(
        false,
    ));
    assert!(empty.probes().is_empty());
    let region = own_periodic_support_regions(empty, &[], 10000).unwrap();
    assert!(!region.base_sample().selected());
    assert!(region.components().is_empty());
    assert!(
        own_periodic_support_signatures(full.owner().clone(), ParameterOrderLimits::default(), 1)
            .is_err()
    );
}
#[test]
fn actual_hole_equal_signatures_require_an_actual_positive_connection_forest() {
    let o = signatures(super::super::periodic_support_material::tests::with_hole());
    assert_eq!(o.probes().len(), 2);
    assert_eq!(o.probes()[0].signature, o.probes()[1].signature);
    assert!(own_periodic_support_regions(o.clone(), &[], 10000).is_err());
    let p = connection(&o, 0, 1, &[]);
    let region = own_periodic_support_regions(o.clone(), &[p.clone()], 10000).unwrap();
    assert_eq!(region.components().len(), 1);
    assert_eq!(region.components()[0].boundary_walks, vec![0, 1]);
    assert_eq!(region.components()[0].forest_paths, vec![0]);
    region.reverify_source().unwrap();
    let mut bad = region.clone();
    bad.paths.clear();
    assert!(bad.reverify_source().is_err());
    assert!(own_periodic_support_regions(o.clone(), &[p.clone()], region.work() - 1).is_err());
    // Geometrically equal copied lift owners do not authorize source mixing.
    let other = Arc::new((**o.owner()).clone());
    let copied = Arc::new(
        own_periodic_support_path(
            other,
            *p.endpoints(),
            p.path(),
            ParameterOrderLimits::default(),
            10000,
        )
        .unwrap(),
    );
    assert!(own_periodic_support_regions(o, &[copied], 10000).is_err());
}
pub(in crate::profile_certificate::span_intersections::translated) fn discs(
    touching: bool,
) -> Arc<OwnedPeriodicSupportSelection> {
    let _replay = super::super::periodic_replay::scope();
    let surface = nurbs_core::surface::Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1., 1.]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let ranges = if touching {
        [(0.2, 0.5), (0.5, 0.8)]
    } else {
        [(0.2, 0.4), (0.6, 0.8)]
    };
    let curves = ranges.map(|(a, b)| {
        Curve::from_polyline(vec![
            vec![a, a],
            vec![b, a],
            vec![b, b],
            vec![a, b],
            vec![a, a],
        ])
        .unwrap()
    });
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
        [false, false],
        &curves,
        &uses,
    );
    let support = Arc::new(
        own_periodic_support_boundaries(input, 1e-12, ParameterOrderLimits::default(), 64, 100000)
            .unwrap(),
    );
    let jumps =
        super::super::periodic_support_boundaries::tests::graph(support.augmented().clone());
    let a = [["1".into(), "8".into()], ["1".into(), "8".into()]];
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
        let samples = std::array::from_fn(|side| {
            if p.exterior()[side] {
                None
            } else {
                Some(Arc::new(
                    own_periodic_material_sample(
                        rule.material().clone(),
                        Arc::new(
                            own_periodic_relative_winding(
                                jumps.clone(),
                                &[a.clone(), p.queries()[side].clone()],
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
    Arc::new(own_periodic_support_selection(rule, &sides).unwrap())
}
#[test]
fn actual_separated_filled_discs_have_distinct_signatures_and_two_components() {
    let o = signatures(discs(false));
    assert_eq!(o.probes().len(), 2);
    assert_ne!(o.probes()[0].signature, o.probes()[1].signature);
    let mut actual = o
        .probes()
        .iter()
        .map(|p| p.signature.clone())
        .collect::<Vec<_>>();
    actual.sort();
    assert_eq!(
        actual,
        vec![
            vec!["0".to_string(), "1".to_string()],
            vec!["1".to_string(), "0".to_string()]
        ]
    );
    let regions = own_periodic_support_regions(o, &[], 10000).unwrap();
    assert_eq!(regions.components().len(), 2);
    assert!(
        regions
            .components()
            .iter()
            .all(|c| c.boundary_walks.len() == 1 && c.forest_paths.is_empty())
    );
    regions.reverify_source().unwrap();
}
#[test]
fn actual_cylinder_kernel_and_material_paths_bind_all_three_boundary_walks() {
    let lifts = super::super::periodic_support_path::tests::cylinder_with_hole();
    let o = Arc::new(
        own_periodic_support_signatures(lifts, ParameterOrderLimits::default(), 10000).unwrap(),
    );
    assert_eq!(o.probes().len(), 3);
    assert_eq!(o.cochains().len(), 2);
    assert!(o.probes().iter().all(|p| p.signature == vec!["0", "0"]));
    assert!(own_periodic_support_regions(o.clone(), &[], 10000).is_err());
    let middle = [[["1".into(), "1".into()], ["1".into(), "8".into()]]];
    let p = connection(&o, 0, 1, &middle);
    let q = connection(&o, 0, 2, &middle);
    let region = own_periodic_support_regions(o, &[p, q], 10000).unwrap();
    assert_eq!(region.components().len(), 1);
    assert_eq!(region.components()[0].boundary_walks, vec![0, 1, 2]);
    assert_eq!(region.components()[0].forest_paths.len(), 2);
    region.reverify_source().unwrap();
}
#[test]
fn actual_untrimmed_torus_has_one_full_component_without_invented_boundary_walks() {
    let _replay = super::super::periodic_replay::scope();
    let surface = nurbs_core::primitives::torus([0.; 3], 3., 1., 1.).unwrap();
    let input = super::super::periodic_support_boundaries::tests::original(
        &surface,
        [true, true],
        &[],
        &[],
    );
    let support = Arc::new(
        own_periodic_support_boundaries(input, 1e-12, ParameterOrderLimits::default(), 64, 100000)
            .unwrap(),
    );
    assert!(support.boundaries().is_empty());
    let jumps = super::super::periodic_support_boundaries::tests::graph_with_pair_limits(
        support.augmented().clone(),
        OriginalAtlasPairLimits {
            max_span_work: 1000000,
            refinement_depth: 4,
            ..Default::default()
        },
    );
    let a = [["1".into(), "2".into()], ["1".into(), "2".into()]];
    let base = Arc::new(
        own_periodic_relative_winding(
            jumps,
            &[a.clone(), a],
            ParameterOrderLimits::default(),
            1000,
        )
        .unwrap(),
    );
    let rule = Arc::new(
        own_periodic_support_material_rule(
            support,
            base,
            &[OriginalPeriodicOperand {
                wire_weights: vec![],
                base_winding: "1".into(),
                fill: OriginalPeriodicFillRule::NonZero,
            }],
            &[OriginalPeriodicBooleanOp::Operand(0)],
        )
        .unwrap(),
    );
    let selection = Arc::new(own_periodic_support_selection(rule, &[]).unwrap());
    let o = signatures(selection);
    assert!(o.probes().is_empty());
    assert!(o.cochains().is_empty());
    let region = own_periodic_support_regions(o, &[], 10000).unwrap();
    assert!(region.base_sample().selected());
    assert_eq!(region.components().len(), 1);
    assert!(region.components()[0].boundary_walks.is_empty());
    assert!(region.components()[0].witness_paths.is_empty());
}

#[test]
fn actual_vertex_touching_material_lobes_remain_distinct_open_components() {
    let o = signatures(discs(true));
    assert_eq!(o.probes().len(), 2);
    assert!(
        o.owner()
            .owner()
            .rotation()
            .rotations()
            .iter()
            .any(|ports| ports.len() == 4)
    );
    assert_ne!(o.probes()[0].signature, o.probes()[1].signature);
    let region = own_periodic_support_regions(o, &[], 10000).unwrap();
    assert_eq!(region.components().len(), 2);
    assert!(
        region
            .components()
            .iter()
            .all(|c| c.boundary_walks.len() == 1)
    );
}

#[test]
fn actual_rational_tangent_hole_has_one_connected_region_and_a_repeated_boundary_vertex() {
    let o = signatures(super::super::periodic_support_material::tests::tangent_hole());
    assert_eq!(o.probes().len(), 1);
    let cycles = o.owner().owner();
    assert!(!cycles.rotation().tangent_witnesses().is_empty());
    assert_eq!(cycles.walks().len(), 1);
    assert_eq!(cycles.walks()[0].len(), 9);
    let ports = cycles.rotation().owner().ports();
    let mut visits = BTreeMap::new();
    for &dart in &cycles.walks()[0] {
        *visits.entry(ports[dart].vertex).or_insert(0usize) += 1;
    }
    assert!(visits.values().any(|n| *n == 2));
    assert_eq!(o.probes()[0].signature, vec!["0"]);
    let region = own_periodic_support_regions(o, &[], 10000).unwrap();
    assert_eq!(region.components().len(), 1);
    assert_eq!(region.components()[0].boundary_walks, vec![0]);
    region.reverify_source().unwrap();
}

pub(in crate::profile_certificate::span_intersections::translated) fn cylinder_bands()
-> Arc<OwnedPeriodicSupportLifts> {
    let _replay = super::super::periodic_replay::scope();
    let rim = Curve::from_polyline(vec![
        vec![0., 0., 0.],
        vec![1., 0., 0.],
        vec![1., 1., 0.],
        vec![0., 1., 0.],
        vec![0., 0., 0.],
    ])
    .unwrap();
    let surface = nurbs_core::surface::extrude(&rim, [0., 0., 1.]).unwrap();
    let levels = [0.25, 0.375, 0.625, 0.75];
    let curves = levels
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let (a, b) = if i % 2 == 0 { (0., 4.) } else { (4., 0.) };
            Curve::from_polyline(vec![vec![a, v], vec![b, v]]).unwrap()
        })
        .collect::<Vec<_>>();
    let uses = (0..4)
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
    let a = [["1".into(), "1".into()], ["1".into(), "2".to_string()]];
    let base = Arc::new(
        own_periodic_relative_winding(
            jumps.clone(),
            &[a.clone(), a.clone()],
            ParameterOrderLimits::default(),
            1000,
        )
        .unwrap(),
    );
    let rule = Arc::new(
        own_periodic_support_material_rule(
            support.clone(),
            base,
            &[OriginalPeriodicOperand {
                wire_weights: vec![1; 4],
                base_winding: "0".to_string(),
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
                                1000,
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

    lifts(Arc::new(
        own_periodic_support_selection(rule, &sides).unwrap(),
    ))
}

#[test]
fn actual_disjoint_cylinder_bands_separate_essential_boundary_walks_with_the_same_true_label() {
    let o = Arc::new(
        own_periodic_support_signatures(cylinder_bands(), ParameterOrderLimits::default(), 10000)
            .unwrap(),
    );
    assert_eq!(o.probes().len(), 4);
    assert_eq!(o.cochains().len(), 3);
    assert!(
        o.owner()
            .walks()
            .iter()
            .all(|w| w.homology[0] != "0" && w.homology[1] == "0")
    );
    let mut expected = BTreeMap::<bool, Vec<usize>>::new();
    for (i, p) in o.probes().iter().enumerate() {
        let v = decode_ratio(&point(&o, i)[1]).unwrap();
        assert!(
            (v > Q::new(1.into(), 4.into()) && v < Q::new(3.into(), 8.into()))
                || (v > Q::new(5.into(), 8.into()) && v < Q::new(3.into(), 4.into()))
        );
        expected
            .entry(v > Q::new(1.into(), 2.into()))
            .or_default()
            .push(p.walk);
    }
    assert_eq!(expected.len(), 2);
    assert!(expected.values().all(|w| w.len() == 2));
    let mut paths = Vec::new();
    for walks in expected.values() {
        assert_eq!(
            o.probes()[walks[0]].signature,
            o.probes()[walks[1]].signature
        );
        paths.push(connection(&o, walks[0], walks[1], &[]));
    }
    let values = expected.values().collect::<Vec<_>>();
    assert_ne!(
        o.probes()[values[0][0]].signature,
        o.probes()[values[1][0]].signature
    );
    assert!(own_periodic_support_regions(o.clone(), &[], 10000).is_err());
    let result = own_periodic_support_regions(o, &paths, 10000).unwrap();
    assert_eq!(result.components().len(), 2);
    let mut got = result
        .components()
        .iter()
        .map(|c| c.boundary_walks.clone())
        .collect::<Vec<_>>();
    got.sort();
    let mut want = expected.into_values().collect::<Vec<_>>();
    want.sort();
    assert_eq!(got, want);
    assert!(
        result
            .components()
            .iter()
            .all(|c| c.forest_paths.len() == 1)
    );
    result.reverify_source().unwrap();
}
