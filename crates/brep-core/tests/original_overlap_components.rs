use brep_core::profile_certificate as p;
use nurbs_core::curve::Curve;
use std::sync::Arc;
fn own(a: &Curve, b: &Curve) -> p::OwnedOriginalOverlapComponents {
    let c = p::certify_span_intersections(a, b, false, 1e-12).unwrap();
    let nodes = p::own_curve_event_nodes(a, b, &c).unwrap();
    let o = p::own_original_overlap_components(Arc::new(nodes)).unwrap();
    o.reverify_source().unwrap();
    o
}
#[test]
fn partial_reversed_component_preserves_two_sources_and_signed_contributions() {
    let a = Curve::from_polyline(vec![vec![0., 0.], vec![2., 0.]]).unwrap();
    let b = Curve::from_polyline(vec![vec![3., 0.], vec![1., 0.]]).unwrap();
    let o = own(&a, &b);
    let c = &o.components()[0];
    assert_eq!(o.components().len(), 1);
    assert_eq!(c.directions, [1, -1]);
    assert!(c.reversed());
    assert_eq!(c.oriented_contributions([3, 2]).unwrap(), [3, -2]);
    assert_eq!(
        c.image_range,
        [
            ["1".to_string(), "1".to_string()],
            ["2".to_string(), "1".to_string()]
        ]
    );
    assert_eq!(o.owner().sources(), &[a, b]);
}
#[test]
fn nonlinear_clock_and_irrational_partial_boundary_remain_original() {
    let a = Curve {
        degree: 2,
        knots: vec![2., 2., 2., 6., 6., 6.],
        control_points: vec![vec![0., 0.], vec![0., 0.], vec![1., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let b = Curve::from_polyline(vec![vec![0.5, 0.], vec![1.5, 0.]]).unwrap();
    let o = own(&a, &b);
    let c = &o.components()[0];
    assert_eq!(c.directions, [1, 1]);
    assert!(!c.reversed());
    assert_ne!(
        c.endpoints[0].exact.isolating_interval[0],
        c.endpoints[0].exact.isolating_interval[1]
    );
    assert!(c.endpoints[0].exact.parameters[0].len() > 1);
    assert_eq!(o.owner().sources()[0], a);
}
#[test]
fn endpoint_touch_and_separation_never_publish_common_components() {
    let a = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
    for start in [1., 2.] {
        let b = Curve::from_polyline(vec![vec![start, 0.], vec![start + 1., 0.]]).unwrap();
        assert!(own(&a, &b).components().is_empty());
    }
}

#[test]
fn curved_common_image_retains_reverse_noncollinear_geometry() {
    let a = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0., 0.], vec![0.5, 0.], vec![1., 1.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let mut b = a.clone();
    b.control_points.reverse();
    let o = own(&a, &b);
    assert_eq!(o.components().len(), 1);
    assert!(o.components()[0].reversed());
    assert_eq!(o.owner().sources(), &[a, b]);
}

fn pair_nodes(a: &Curve, b: &Curve) -> Arc<p::CurveEventNodes> {
    let c = p::certify_span_intersections(a, b, false, 1e-12).unwrap();
    Arc::new(p::own_curve_event_nodes(a, b, &c).unwrap())
}
#[test]
fn third_curve_cuts_match_irrational_and_linear_clocks_in_common_image_order() {
    let a = Curve {
        degree: 2,
        knots: vec![2., 2., 2., 6., 6., 6.],
        control_points: vec![vec![0., 0.], vec![0., 0.], vec![1., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let b = Curve::from_polyline(vec![vec![1., 0.], vec![0., 0.]]).unwrap();
    let third = Curve::from_polyline(vec![
        vec![0.25, -1.],
        vec![0.25, 1.],
        vec![0.75, 1.],
        vec![0.75, -1.],
    ])
    .unwrap();
    let overlap = Arc::new(own(&a, &b));
    let contacts = p::own_original_overlap_contacts(
        overlap,
        0,
        [pair_nodes(&a, &third), pair_nodes(&b, &third)],
        p::ParameterOrderLimits::default(),
    )
    .unwrap();
    contacts.reverify_source().unwrap();
    assert_eq!(contacts.cuts().len(), 2);
    assert!(contacts.cuts().iter().all(|cut| cut.endpoint.is_none()));
    let first = &contacts.contacts()[0].nodes()[contacts.cuts()[0].nodes[0]];
    let last = &contacts.contacts()[0].nodes()[contacts.cuts()[1].nodes[0]];
    assert_eq!(
        first.exact.position[0][0],
        vec![["1".to_string(), "4".to_string()]]
    );
    assert!(last.exact.parameters[0].len() > 1);
}
#[test]
fn third_curve_endpoint_contacts_and_unrelated_source_refusal() {
    let a = Curve::from_polyline(vec![vec![0., 0.], vec![2., 0.]]).unwrap();
    let b = Curve::from_polyline(vec![vec![3., 0.], vec![1., 0.]]).unwrap();
    let third = Curve::from_polyline(vec![vec![1., -1.], vec![1., 1.]]).unwrap();
    let overlap = Arc::new(own(&a, &b));
    let x = p::own_original_overlap_contacts(
        overlap.clone(),
        0,
        [pair_nodes(&a, &third), pair_nodes(&b, &third)],
        p::ParameterOrderLimits::default(),
    )
    .unwrap();
    assert_eq!(x.cuts().len(), 1);
    assert_eq!(x.cuts()[0].endpoint, Some(0));
    let unrelated = Curve::from_polyline(vec![vec![1.5, -1.], vec![1.5, 1.]]).unwrap();
    assert!(
        p::own_original_overlap_contacts(
            overlap,
            0,
            [pair_nodes(&a, &third), pair_nodes(&b, &unrelated)],
            p::ParameterOrderLimits::default()
        )
        .is_err()
    );
}

#[test]
fn repeated_third_source_traversals_group_into_one_physical_split() {
    let a = Curve::from_polyline(vec![vec![0., 0.], vec![2., 0.]]).unwrap();
    let b = Curve::from_polyline(vec![vec![3., 0.], vec![1., 0.]]).unwrap();
    let t = Curve::from_polyline(vec![
        vec![1.5, -1.],
        vec![1.5, 1.],
        vec![1.5, -1.],
        vec![1.5, 1.],
    ])
    .unwrap();
    let x = p::own_original_overlap_contacts(
        Arc::new(own(&a, &b)),
        0,
        [pair_nodes(&a, &t), pair_nodes(&b, &t)],
        p::ParameterOrderLimits::default(),
    )
    .unwrap();
    x.reverify_source().unwrap();
    assert_eq!(x.cuts().len(), 3);
    assert_eq!(x.groups().len(), 1);
    assert_eq!(x.groups()[0].len(), 3);
    assert_eq!(
        x.segments(),
        &[
            [
                p::OriginalOverlapSplit::Endpoint(0),
                p::OriginalOverlapSplit::ContactGroup(0)
            ],
            [
                p::OriginalOverlapSplit::ContactGroup(0),
                p::OriginalOverlapSplit::Endpoint(1)
            ],
        ]
    );
    for source in 0..2 {
        let (_, parameter) = x
            .split_parameter(p::OriginalOverlapSplit::ContactGroup(0), source)
            .unwrap();
        assert_eq!(parameter, 0);
        let (_, parameter) = x
            .split_parameter(p::OriginalOverlapSplit::Endpoint(0), source)
            .unwrap();
        assert_eq!(parameter, source);
    }
    assert!(
        x.split_parameter(p::OriginalOverlapSplit::ContactGroup(99), 0)
            .is_err()
    );
}
#[test]
fn endpoint_contact_does_not_create_a_zero_length_segment() {
    let a = Curve::from_polyline(vec![vec![0., 0.], vec![2., 0.]]).unwrap();
    let b = Curve::from_polyline(vec![vec![3., 0.], vec![1., 0.]]).unwrap();
    let t = Curve::from_polyline(vec![vec![1., -1.], vec![1., 1.]]).unwrap();
    let x = p::own_original_overlap_contacts(
        Arc::new(own(&a, &b)),
        0,
        [pair_nodes(&a, &t), pair_nodes(&b, &t)],
        p::ParameterOrderLimits::default(),
    )
    .unwrap();
    assert_eq!(x.groups().len(), 1);
    assert_eq!(
        x.segments(),
        &[[
            p::OriginalOverlapSplit::Endpoint(0),
            p::OriginalOverlapSplit::Endpoint(1)
        ]]
    );
    x.reverify_source().unwrap();
}

#[test]
fn multiple_third_sources_merge_coincident_cuts_and_keep_all_original_owners() {
    let a = Curve {
        degree: 2,
        knots: vec![2., 2., 2., 6., 6., 6.],
        control_points: vec![vec![0., 0.], vec![0., 0.], vec![1., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let b = Curve::from_polyline(vec![vec![1., 0.], vec![0., 0.]]).unwrap();
    let vertical = |x: f64| Curve::from_polyline(vec![vec![x, -1.], vec![x, 1.]]).unwrap();
    let sources = vec![vertical(0.75), vertical(0.25), vertical(0.75), vertical(2.)];
    let o = p::own_original_overlap_partition(
        Arc::new(own(&a, &b)),
        0,
        &sources,
        1e-12,
        p::ParameterOrderLimits::default(),
    )
    .unwrap();
    o.reverify_source().unwrap();
    assert_eq!(o.sources(), sources);
    assert_eq!(o.owners().len(), 4);
    assert_eq!(o.groups().len(), 2);
    assert_eq!(o.groups()[0].len(), 1);
    assert_eq!(o.groups()[0][0].source, 1);
    assert_eq!(o.groups()[1].len(), 2);
    assert_eq!(o.segments().len(), 3);
    let (cut, side) = o
        .split_parameter(p::OriginalOverlapSplit::ContactGroup(1), 0)
        .unwrap();
    assert_eq!(side, 0);
    assert!(cut.parameters[0].len() > 1);
}
#[test]
fn no_third_sources_keep_full_component_and_coincident_third_refuses() {
    let a = Curve::from_polyline(vec![vec![0., 0.], vec![2., 0.]]).unwrap();
    let b = Curve::from_polyline(vec![vec![3., 0.], vec![1., 0.]]).unwrap();
    let overlap = Arc::new(own(&a, &b));
    let x = p::own_original_overlap_partition(
        overlap.clone(),
        0,
        &[],
        1e-12,
        p::ParameterOrderLimits::default(),
    )
    .unwrap();
    x.reverify_source().unwrap();
    assert!(x.groups().is_empty());
    assert_eq!(x.segments().len(), 1);
    assert!(
        p::own_original_overlap_partition(
            overlap,
            0,
            &[a],
            1e-12,
            p::ParameterOrderLimits::default()
        )
        .is_err()
    );
}

#[test]
fn original_knot_overlap_boundaries_deduplicate_with_full_provenance() {
    let a = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.], vec![2., 0.]]).unwrap();
    let b = Curve::from_polyline(vec![vec![0.5, 0.], vec![1.5, 0.]]).unwrap();
    let x =
        p::own_original_pair_cuts(pair_nodes(&a, &b), p::ParameterOrderLimits::default()).unwrap();
    x.reverify_source().unwrap();
    assert_eq!(x.cuts().len(), 3);
    assert_eq!(x.cuts()[1].origins.len(), 2);
    assert!(
        x.cuts()
            .iter()
            .flat_map(|c| &c.origins)
            .all(|r| matches!(r, p::OriginalPairCutOrigin::OverlapEndpoint { .. }))
    );
}
#[test]
fn isolated_cut_origin_and_separated_pair_remain_distinct_from_overlaps() {
    let a = Curve::from_polyline(vec![vec![0., 0.], vec![2., 0.]]).unwrap();
    let t = Curve::from_polyline(vec![vec![1., -1.], vec![1., 1.]]).unwrap();
    let x =
        p::own_original_pair_cuts(pair_nodes(&a, &t), p::ParameterOrderLimits::default()).unwrap();
    assert_eq!(x.cuts().len(), 1);
    assert_eq!(
        x.cuts()[0].origins,
        vec![p::OriginalPairCutOrigin::Isolated { node: 0 }]
    );
    let b = Curve::from_polyline(vec![vec![0., 1.], vec![2., 1.]]).unwrap();
    assert!(
        p::own_original_pair_cuts(pair_nodes(&a, &b), p::ParameterOrderLimits::default())
            .unwrap()
            .cuts()
            .is_empty()
    );
}

#[test]
fn owned_original_point_pullback_keeps_irrational_joint_parameters() {
    let a = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0., 0.], vec![0.5, 0.], vec![1., 1.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let line = Curve::from_polyline(vec![vec![0., 0.5], vec![1., 0.5]]).unwrap();
    let point = Arc::new(
        p::own_original_pair_cuts(pair_nodes(&a, &line), p::ParameterOrderLimits::default())
            .unwrap(),
    );
    let third = Curve {
        degree: 3,
        knots: vec![2., 2., 2., 2., 6., 6., 6., 6.],
        control_points: vec![vec![0., 0.5], vec![0., 0.5], vec![0., 0.5], vec![1., 0.5]],
        weights: vec![1.; 4],
        periodic: false,
    };
    let x = p::own_original_point_pullback(point, 0, &third, 1e-9).unwrap();
    x.reverify_source().unwrap();
    assert_eq!(x.preimages().len(), 1);
    assert!(x.preimages()[0].exact.primitive.len() >= 7);
    assert_eq!(x.curve(), &third);
}

#[test]
fn common_third_interval_clips_both_ends_with_original_three_source_clocks() {
    let a = Curve::from_polyline(vec![vec![0., 0.], vec![4., 0.]]).unwrap();
    let b = Curve::from_polyline(vec![vec![3., 0.], vec![1., 0.]]).unwrap();
    let t = Curve::from_polyline(vec![vec![2., 0.], vec![4., 0.]]).unwrap();
    let x = p::own_original_common_clips(
        Arc::new(own(&a, &b)),
        0,
        &t,
        1e-12,
        p::ParameterOrderLimits::default(),
    )
    .unwrap();
    x.reverify_source().unwrap();
    assert_eq!(x.clips().len(), 1);
    assert!(matches!(
        x.clips()[0].boundaries[0].origin,
        p::OriginalCommonClipOrigin::ThirdEndpoint(0)
    ));
    assert!(matches!(
        x.clips()[0].boundaries[1].origin,
        p::OriginalCommonClipOrigin::BaseEndpoint(1)
    ));
    assert_eq!(x.third().owner().sources()[1], t);
}
#[test]
fn common_third_disjoint_or_point_touch_has_no_positive_length_clip() {
    let a = Curve::from_polyline(vec![vec![0., 0.], vec![4., 0.]]).unwrap();
    let b = Curve::from_polyline(vec![vec![1., 0.], vec![2., 0.]]).unwrap();
    for start in [2., 3.] {
        let t = Curve::from_polyline(vec![vec![start, 0.], vec![4., 0.]]).unwrap();
        let x = p::own_original_common_clips(
            Arc::new(own(&a, &b)),
            0,
            &t,
            1e-12,
            p::ParameterOrderLimits::default(),
        )
        .unwrap();
        assert!(x.clips().is_empty());
        x.reverify_source().unwrap();
    }
}
#[test]
fn common_clip_keeps_independent_quadratic_and_cubic_boundary_parameters() {
    let a = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0., 0.], vec![0., 0.], vec![1., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let b = Curve::from_polyline(vec![vec![1., 0.], vec![0.5, 0.]]).unwrap();
    let t = Curve {
        degree: 3,
        knots: vec![2., 2., 2., 2., 6., 6., 6., 6.],
        control_points: vec![vec![0., 0.], vec![0., 0.], vec![0., 0.], vec![2., 0.]],
        weights: vec![1.; 4],
        periodic: false,
    };
    let x = p::own_original_common_clips(
        Arc::new(own(&a, &b)),
        0,
        &t,
        1e-9,
        p::ParameterOrderLimits::default(),
    )
    .unwrap();
    x.reverify_source().unwrap();
    assert_eq!(x.clips().len(), 1);
    let boundary = &x.clips()[0].boundaries[0];
    assert!(boundary.positions[0].parameters[0].len() > 1);
    assert!(boundary.positions[1].primitive.len() >= 7);
    assert!(boundary.positions[1].parameters[1].len() > 1);
}

#[test]
fn mixed_common_and_isolated_sources_partition_with_exact_contributor_sets() {
    let a = Curve::from_polyline(vec![vec![0., 0.], vec![3., 0.]]).unwrap();
    let b = Curve::from_polyline(vec![vec![2.5, 0.], vec![0.5, 0.]]).unwrap();
    let common = Curve::from_polyline(vec![vec![2., 0.], vec![1., 0.]]).unwrap();
    let crossing = Curve::from_polyline(vec![vec![1.5, -1.], vec![1.5, 1.]]).unwrap();
    let x = p::own_original_common_partition(
        Arc::new(own(&a, &b)),
        0,
        &[common, crossing],
        1e-12,
        p::ParameterOrderLimits::default(),
    )
    .unwrap();
    x.reverify_source().unwrap();
    assert_eq!(x.cuts().len(), 5);
    assert_eq!(x.segments().len(), 4);
    assert!(x.segments()[0].contributors.is_empty());
    assert!(x.segments()[3].contributors.is_empty());
    for i in [1, 2] {
        assert_eq!(x.segments()[i].contributors.len(), 1);
        assert_eq!(x.segments()[i].contributors[0].source, 0);
        assert_eq!(x.segments()[i].contributors[0].direction, -1);
    }
}
#[test]
fn common_endpoint_touch_retains_incidence_without_interval_contribution() {
    let a = Curve::from_polyline(vec![vec![0., 0.], vec![3., 0.]]).unwrap();
    let b = Curve::from_polyline(vec![vec![0.5, 0.], vec![1.5, 0.]]).unwrap();
    let t = Curve::from_polyline(vec![vec![1.5, 0.], vec![2.5, 0.]]).unwrap();
    let x = p::own_original_common_partition(
        Arc::new(own(&a, &b)),
        0,
        &[t],
        1e-12,
        p::ParameterOrderLimits::default(),
    )
    .unwrap();
    x.reverify_source().unwrap();
    assert_eq!(x.cuts().len(), 2);
    assert_eq!(x.segments().len(), 1);
    assert!(x.segments()[0].contributors.is_empty());
    assert!(
        x.cuts()[1]
            .origins
            .iter()
            .any(|o| matches!(o, p::OriginalCommonCutOrigin::ThirdCut { .. }))
    );
}
#[test]
fn coincident_third_contributors_keep_distinct_source_labels() {
    let a = Curve::from_polyline(vec![vec![0., 0.], vec![3., 0.]]).unwrap();
    let b = Curve::from_polyline(vec![vec![2.5, 0.], vec![0.5, 0.]]).unwrap();
    let t = Curve::from_polyline(vec![vec![2., 0.], vec![1., 0.]]).unwrap();
    let x = p::own_original_common_partition(
        Arc::new(own(&a, &b)),
        0,
        &[t.clone(), t],
        1e-12,
        p::ParameterOrderLimits::default(),
    )
    .unwrap();
    x.reverify_source().unwrap();
    assert_eq!(x.cuts().len(), 4);
    assert_eq!(x.segments().len(), 3);
    let c = &x.segments()[1].contributors;
    assert_eq!(c.len(), 2);
    assert_eq!(c[0].source, 0);
    assert_eq!(c[1].source, 1);
    assert!(c.iter().all(|c| c.direction == -1));
}

#[test]
fn global_common_dictionary_merges_all_three_pair_aliases_of_one_edge() {
    let a = Curve::from_polyline(vec![vec![0., 0.], vec![2., 0.]]).unwrap();
    let b = Curve::from_polyline(vec![vec![2., 0.], vec![0., 0.]]).unwrap();
    let c = Curve::from_polyline(vec![vec![0., 0.], vec![2., 0.]]).unwrap();
    let x =
        p::own_original_common_dictionary(&[a, b, c], 1e-12, p::ParameterOrderLimits::default())
            .unwrap();
    x.reverify_source().unwrap();
    assert_eq!(x.pairs().len(), 6);
    assert_eq!(x.partitions().len(), 3);
    assert_eq!(x.edges().len(), 1);
    assert_eq!(x.edges()[0].uses.len(), 3);
    assert!(x.edges()[0].uses.iter().any(|u| u.reversed));
}
#[test]
fn global_common_dictionary_splits_pair_aliases_at_all_source_boundaries() {
    let line = |a: f64, b: f64| Curve::from_polyline(vec![vec![a, 0.], vec![b, 0.]]).unwrap();
    let x = p::own_original_common_dictionary(
        &[line(0., 4.), line(1., 3.), line(2., 4.)],
        1e-12,
        p::ParameterOrderLimits::default(),
    )
    .unwrap();
    x.reverify_source().unwrap();
    assert_eq!(x.edges().len(), 3);
    assert_eq!(
        x.edges().iter().map(|e| e.uses.len()).collect::<Vec<_>>(),
        vec![1, 3, 1]
    );
}

#[test]
fn unified_atomic_source_carriers_cover_single_and_common_intervals() {
    let line = |a: f64, b: f64| Curve::from_polyline(vec![vec![a, 0.], vec![b, 0.]]).unwrap();
    let dictionary = Arc::new(
        p::own_original_common_dictionary(
            &[line(0., 4.), line(3., 1.), line(2., 4.)],
            1e-12,
            p::ParameterOrderLimits::default(),
        )
        .unwrap(),
    );
    let x =
        p::own_original_source_carriers(dictionary, p::ParameterOrderLimits::default()).unwrap();
    x.reverify_source().unwrap();
    assert_eq!(x.carriers().len(), 8);
    assert_eq!(
        x.carriers()
            .iter()
            .filter(|c| c.common_edge.is_some())
            .count(),
        7
    );
    assert_eq!(
        x.cuts().iter().map(Vec::len).collect::<Vec<_>>(),
        vec![5, 3, 3]
    );
    assert!(
        x.carriers()
            .iter()
            .filter(|c| c.source == 1)
            .all(|c| c.common_uses.iter().all(|u| u.reversed))
    );
}
#[test]
fn unified_source_knots_and_isolated_crossings_survive_without_common_edges() {
    let a = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.], vec![2., 0.]]).unwrap();
    let b = Curve::from_polyline(vec![vec![0.5, -1.], vec![0.5, 1.]]).unwrap();
    let dictionary = Arc::new(
        p::own_original_common_dictionary(&[a, b], 1e-12, p::ParameterOrderLimits::default())
            .unwrap(),
    );
    let x =
        p::own_original_source_carriers(dictionary, p::ParameterOrderLimits::default()).unwrap();
    x.reverify_source().unwrap();
    assert_eq!(x.carriers().len(), 5);
    assert!(x.carriers().iter().all(|c| c.common_edge.is_none()));
    assert_eq!(
        x.cuts().iter().map(Vec::len).collect::<Vec<_>>(),
        vec![4, 3]
    );
    assert_eq!(
        x.carriers()
            .iter()
            .filter(|c| c.source == 0)
            .map(|c| c.cell)
            .collect::<Vec<_>>(),
        vec![0, 0, 1]
    );
}

fn unified_graph(sources: &[Curve]) -> p::OwnedOriginalUnifiedGraph {
    let dictionary = Arc::new(
        p::own_original_common_dictionary(sources, 1e-12, p::ParameterOrderLimits::default())
            .unwrap(),
    );
    let carriers = Arc::new(
        p::own_original_source_carriers(dictionary, p::ParameterOrderLimits::default()).unwrap(),
    );
    let graph = p::own_original_unified_graph(carriers).unwrap();
    graph.reverify_source().unwrap();
    graph
}
#[test]
fn unified_contact_graph_keeps_crossing_as_one_four_port_vertex() {
    let a = Curve::from_polyline(vec![vec![0., 0.], vec![2., 2.]]).unwrap();
    let b = Curve::from_polyline(vec![vec![0., 2.], vec![2., 0.]]).unwrap();
    let x = unified_graph(&[a, b]);
    assert_eq!(x.vertices().len(), 5);
    assert_eq!(x.edges().len(), 4);
    assert_eq!(
        x.incidences()
            .iter()
            .filter(|ports| ports.len() == 4)
            .count(),
        1
    );
}
#[test]
fn unified_contact_graph_deduplicates_three_original_common_branches() {
    let line = |a: f64, b: f64| Curve::from_polyline(vec![vec![a, 0.], vec![b, 0.]]).unwrap();
    let x = unified_graph(&[line(0., 2.), line(2., 0.), line(0., 2.)]);
    assert_eq!(x.vertices().len(), 2);
    assert_eq!(x.edges().len(), 1);
    assert_eq!(x.edges()[0].carriers.len(), 3);
    assert!(x.incidences().iter().all(|p| p.len() == 1));
}
#[test]
fn unified_contact_graph_refuses_discontinuous_native_knot_join() {
    let a = Curve {
        degree: 1,
        knots: vec![0., 0., 0.5, 0.5, 1., 1.],
        control_points: vec![vec![0., 0.], vec![1., 0.], vec![2., 0.], vec![3., 0.]],
        weights: vec![1.; 4],
        periodic: false,
    };
    let result = p::own_original_common_dictionary(&[a], 1e-12, p::ParameterOrderLimits::default())
        .and_then(|d| {
            p::own_original_source_carriers(Arc::new(d), p::ParameterOrderLimits::default())
        })
        .and_then(|c| p::own_original_unified_graph(Arc::new(c)));
    assert!(result.is_err());
}

#[test]
fn unified_contact_graph_closes_original_wire_without_duplicate_endpoint_vertex() {
    let a = Curve::from_polyline(vec![
        vec![0., 0.],
        vec![1., 0.],
        vec![1., 1.],
        vec![0., 1.],
        vec![0., 0.],
    ])
    .unwrap();
    let x = unified_graph(&[a]);
    assert_eq!(x.vertices().len(), 4);
    assert_eq!(x.edges().len(), 4);
    assert!(x.incidences().iter().all(|p| p.len() == 2));
}

#[test]
fn unified_ports_own_four_actual_outward_crossing_germs() {
    let a = Curve::from_polyline(vec![vec![0., 0.], vec![2., 2.]]).unwrap();
    let b = Curve::from_polyline(vec![vec![0., 2.], vec![2., 0.]]).unwrap();
    let x = p::own_original_unified_ports(Arc::new(unified_graph(&[a, b])), 192).unwrap();
    x.reverify_source().unwrap();
    let v = x.vertices().iter().find(|v| v.len() == 4).unwrap();
    let mut signs = v
        .iter()
        .map(|&i| x.ports()[i].ray.signs)
        .collect::<Vec<_>>();
    signs.sort();
    assert_eq!(signs, vec![[-1, -1], [-1, 1], [1, -1], [1, 1]]);
    assert!(x.ports().iter().all(|p| p.ray.order == 1));
}
#[test]
fn unified_ports_keep_native_knot_incoming_and_outgoing_source_cells() {
    let a = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.], vec![1., 1.]]).unwrap();
    let x = p::own_original_unified_ports(Arc::new(unified_graph(&[a])), 192).unwrap();
    x.reverify_source().unwrap();
    let v = x.vertices().iter().find(|v| v.len() == 2).unwrap();
    let mut cells = v.iter().map(|&i| x.ports()[i].cell).collect::<Vec<_>>();
    cells.sort();
    assert_eq!(cells, vec![0, 1]);
    let mut signs = v
        .iter()
        .map(|&i| x.ports()[i].ray.signs)
        .collect::<Vec<_>>();
    signs.sort();
    assert_eq!(signs, vec![[-1, 0], [0, 1]]);
}
#[test]
fn unified_ports_keep_a_higher_order_stationary_endpoint_germ() {
    let a = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0., 0.], vec![0., 0.], vec![1., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let x = p::own_original_unified_ports(Arc::new(unified_graph(&[a])), 192).unwrap();
    x.reverify_source().unwrap();
    assert_eq!(x.ports().len(), 2);
    assert_eq!(x.ports()[0].ray.order, 2);
    assert_eq!(x.ports()[0].ray.signs, [1, 0]);
}

#[test]
fn unified_rotation_orders_crossing_ports_counterclockwise() {
    let a = Curve::from_polyline(vec![vec![0., 0.], vec![2., 2.]]).unwrap();
    let b = Curve::from_polyline(vec![vec![0., 2.], vec![2., 0.]]).unwrap();
    let ports =
        Arc::new(p::own_original_unified_ports(Arc::new(unified_graph(&[a, b])), 192).unwrap());
    let x = p::own_original_unified_rotation(ports, p::ContactPortOrderLimits::default()).unwrap();
    x.reverify_source().unwrap();
    let r = x.rotations().iter().find(|r| r.len() == 4).unwrap();
    assert_eq!(
        r.iter()
            .map(|&i| x.owner().ports()[i].ray.signs)
            .collect::<Vec<_>>(),
        vec![[1, 1], [-1, 1], [-1, -1], [1, -1]]
    );
}
#[test]
fn unified_rotation_separates_tangent_branches_and_handles_positive_x_cut() {
    let curve = |y: f64| Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0., 0.], vec![0.5, 0.], vec![1., y]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let ports = Arc::new(
        p::own_original_unified_ports(Arc::new(unified_graph(&[curve(-1.), curve(1.)])), 192)
            .unwrap(),
    );
    let x = p::own_original_unified_rotation(ports, p::ContactPortOrderLimits::default()).unwrap();
    x.reverify_source().unwrap();
    let r = x.rotations().iter().find(|r| r.len() == 2).unwrap();
    assert_eq!(
        r.iter()
            .map(|&i| x.owner().ports()[i].component_signs[1])
            .collect::<Vec<_>>(),
        vec![1, -1]
    );
}

fn unified_cycles(sources: &[Curve]) -> p::OwnedOriginalUnifiedCycles {
    let ports =
        Arc::new(p::own_original_unified_ports(Arc::new(unified_graph(sources)), 192).unwrap());
    let rotation = Arc::new(
        p::own_original_unified_rotation(ports, p::ContactPortOrderLimits::default()).unwrap(),
    );
    let cycles = p::own_original_unified_cycles(rotation).unwrap();
    cycles.reverify_source().unwrap();
    cycles
}
#[test]
fn unified_cycles_cover_both_sides_of_a_closed_source_wire() {
    let a = Curve::from_polyline(vec![
        vec![0., 0.],
        vec![1., 0.],
        vec![1., 1.],
        vec![0., 1.],
        vec![0., 0.],
    ])
    .unwrap();
    let x = unified_cycles(&[a]);
    assert_eq!(x.cycles().len(), 2);
    assert!(x.cycles().iter().all(|c| c.len() == 4));
    assert_eq!(x.dart_cycles().len(), 8);
}
#[test]
fn unified_cycles_preserve_tree_walk_without_inventing_a_material_face() {
    let a = Curve::from_polyline(vec![vec![0., 0.], vec![2., 2.]]).unwrap();
    let b = Curve::from_polyline(vec![vec![0., 2.], vec![2., 0.]]).unwrap();
    let x = unified_cycles(&[a, b]);
    assert_eq!(x.cycles().len(), 1);
    assert_eq!(x.cycles()[0].len(), 8);
}
#[test]
fn unified_cycles_use_one_common_branch_for_three_coincident_sources() {
    let line = |a: f64, b: f64| Curve::from_polyline(vec![vec![a, 0.], vec![b, 0.]]).unwrap();
    let x = unified_cycles(&[line(0., 2.), line(2., 0.), line(0., 2.)]);
    assert_eq!(x.cycles().len(), 1);
    assert_eq!(x.cycles()[0].len(), 2);
}
#[test]
fn unified_rotation_orders_both_semicubic_cusp_germs_intrinsically() {
    let a = Curve {
        degree: 3,
        knots: vec![-1., -1., -1., -1., 1., 1., 1., 1.],
        control_points: vec![vec![3., -3.], vec![-1., 3.], vec![-1., -3.], vec![3., 3.]],
        weights: vec![1.; 4],
        periodic: false,
    };
    let ports =
        Arc::new(p::own_original_unified_ports(Arc::new(unified_graph(&[a])), 192).unwrap());
    let x = p::own_original_unified_rotation(ports, p::ContactPortOrderLimits::default()).unwrap();
    x.reverify_source().unwrap();
    let r = x.rotations().iter().find(|r| r.len() == 2).unwrap();
    assert!(r.iter().all(|&i| x.owner().ports()[i].ray.order == 2));
    assert_eq!(
        r.iter()
            .map(|&i| x.owner().ports()[i].component_signs[1])
            .collect::<Vec<_>>(),
        vec![1, -1]
    );
}

#[test]
fn unified_side_winding_counts_both_coincident_original_wires() {
    let a = Curve::from_polyline(vec![
        vec![0., 0.],
        vec![1., 0.],
        vec![1., 1.],
        vec![0., 1.],
        vec![0., 0.],
    ])
    .unwrap();
    let graph = Arc::new(unified_graph(&[a.clone(), a.clone()]));
    let loops = vec![vec![a.clone()], vec![a]];
    let sides = p::own_original_unified_sides_automatically(
        graph,
        &loops,
        p::ParameterOrderLimits::default(),
        32,
    )
    .unwrap();
    assert_eq!(sides.len(), 4);
    for side in sides {
        side.reverify_source().unwrap();
        assert_eq!(side.jumps(), &[1, 1]);
    }
}
#[test]
fn unified_side_winding_preserves_opposite_original_wire_contributions() {
    let a = Curve::from_polyline(vec![
        vec![0., 0.],
        vec![1., 0.],
        vec![1., 1.],
        vec![0., 1.],
        vec![0., 0.],
    ])
    .unwrap();
    let mut b = a.clone();
    b.control_points.reverse();
    let graph = Arc::new(unified_graph(&[a.clone(), b.clone()]));
    let loops = vec![vec![a], vec![b]];
    let sides = p::own_original_unified_sides_automatically(
        graph,
        &loops,
        p::ParameterOrderLimits::default(),
        32,
    )
    .unwrap();
    assert_eq!(sides.len(), 4);
    for side in sides {
        side.reverify_source().unwrap();
        assert_eq!(side.jumps(), &[1, -1]);
    }
}

#[test]
fn unified_filled_union_deduplicates_common_boundary_and_subtraction_cancels() {
    let a = Curve::from_polyline(vec![
        vec![0., 0.],
        vec![1., 0.],
        vec![1., 1.],
        vec![0., 1.],
        vec![0., 0.],
    ])
    .unwrap();
    let loops = vec![vec![a.clone()], vec![a.clone()]];
    let cycles = Arc::new(unified_cycles(&[a.clone(), a]));
    let graph = cycles.owner().owner().graph().clone();
    let sides = p::own_original_unified_sides_automatically(
        graph,
        &loops,
        p::ParameterOrderLimits::default(),
        32,
    )
    .unwrap();
    let union = p::own_original_unified_filled(cycles.clone(), sides.clone(), &[1, 1]).unwrap();
    union.reverify_source().unwrap();
    assert_eq!(union.retained().len(), 4);
    assert_eq!(union.boundaries().len(), 1);
    assert_eq!(union.boundaries()[0].len(), 4);
    let difference = p::own_original_unified_filled(cycles, sides, &[1, -1]).unwrap();
    difference.reverify_source().unwrap();
    assert!(difference.retained().is_empty());
    assert!(difference.boundaries().is_empty());
}

fn unified_filled(
    loops: &[Vec<Curve>],
    coefficients: &[i32],
) -> Arc<p::OwnedOriginalUnifiedFilled> {
    let sources = loops.iter().flatten().cloned().collect::<Vec<_>>();
    let cycles = Arc::new(unified_cycles(&sources));
    let graph = cycles.owner().owner().graph().clone();
    let sides = p::own_original_unified_sides_automatically(
        graph,
        loops,
        p::ParameterOrderLimits::default(),
        32,
    )
    .unwrap();
    Arc::new(p::own_original_unified_filled(cycles, sides, coefficients).unwrap())
}
#[test]
fn unified_regions_own_a_hole_and_a_nested_material_island() {
    let square = |lo: f64, hi: f64| {
        Curve::from_polyline(vec![
            vec![lo, lo],
            vec![hi, lo],
            vec![hi, hi],
            vec![lo, hi],
            vec![lo, lo],
        ])
        .unwrap()
    };
    let owner = unified_filled(
        &[
            vec![square(0., 6.)],
            vec![square(1., 5.)],
            vec![square(2., 4.)],
        ],
        &[1, -1, 1],
    );
    let regions =
        p::own_original_unified_regions(owner.clone(), p::ParameterOrderLimits::default()).unwrap();
    regions.reverify_source().unwrap();
    assert_eq!(regions.regions().len(), 2);
    assert_eq!(
        regions
            .regions()
            .iter()
            .map(|r| r.holes.len())
            .sum::<usize>(),
        1
    );
    let q = |n: &str| [n.to_string(), "1".to_string()];
    let w = p::own_original_unified_boundary_winding(
        owner,
        [q("3"), q("3")],
        p::ParameterOrderLimits::default(),
    )
    .unwrap();
    w.reverify_source().unwrap();
    let mut values = w.windings().to_vec();
    values.sort();
    assert_eq!(values, vec![-1, 1, 1]);
}
#[test]
fn unified_regions_do_not_duplicate_a_common_boundary() {
    let a = Curve::from_polyline(vec![
        vec![0., 0.],
        vec![1., 0.],
        vec![1., 1.],
        vec![0., 1.],
        vec![0., 0.],
    ])
    .unwrap();
    let owner = unified_filled(&[vec![a.clone()], vec![a]], &[1, 1]);
    let regions =
        p::own_original_unified_regions(owner, p::ParameterOrderLimits::default()).unwrap();
    regions.reverify_source().unwrap();
    assert_eq!(regions.regions().len(), 1);
    assert!(regions.regions()[0].holes.is_empty());
}

#[test]
fn original_chart_shore_census_preserves_irrational_natural_cut_and_crossing() {
    let a = Curve {
        degree: 2,
        knots: vec![2., 2., 2., 6., 6., 6.],
        control_points: vec![vec![0., 0.], vec![0., 0.5], vec![1., 1.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let x = p::own_original_chart_events(
        &[a.clone()],
        [[0.5, 2.], [-2., 2.]],
        [true, false],
        1e-12,
        192,
    )
    .unwrap();
    x.reverify_source().unwrap();
    assert_eq!(x.events().len(), 1);
    let e = &x.events()[0];
    assert_eq!(e.sides, [Some(-1), Some(1)]);
    assert_eq!(e.order, 1);
    assert_ne!(e.parameter.interval[0], e.parameter.interval[1]);
    assert!(e.parameter.parameter.len() > 1);
    assert_eq!(x.sources(), &[a]);
}
#[test]
fn original_chart_shore_census_keeps_tangent_and_coincident_cell_separate() {
    let a = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![1., 0.], vec![-1., 0.5], vec![1., 1.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let b = Curve::from_polyline(vec![vec![0., 0.], vec![0., 1.]]).unwrap();
    let x = p::own_original_chart_events(&[a, b], [[0., 2.], [-2., 2.]], [true, false], 1e-12, 192)
        .unwrap();
    x.reverify_source().unwrap();
    assert_eq!(x.events().len(), 1);
    assert_eq!(x.events()[0].sides, [Some(1), Some(1)]);
    assert_eq!(x.events()[0].order, 2);
    assert_eq!(x.coincident_cells().len(), 1);
}
#[test]
fn original_chart_shore_native_knot_has_both_one_sided_cell_records() {
    let a = Curve::from_polyline(vec![vec![-1., 0.], vec![0., 1.], vec![1., 0.]]).unwrap();
    let x = p::own_original_chart_events(&[a], [[0., 2.], [-2., 2.]], [true, false], 1e-12, 192)
        .unwrap();
    x.reverify_source().unwrap();
    assert_eq!(x.events().len(), 2);
    assert_eq!(x.events()[0].sides, [Some(-1), None]);
    assert_eq!(x.events()[1].sides, [None, Some(1)]);
}

#[test]
fn original_chart_shore_rejects_invalid_or_nonplanar_curves_before_extraction() {
    let line = Curve::from_polyline(vec![vec![0., 0.], vec![1., 1.]]).unwrap();
    let mut malformed = line.clone();
    malformed.control_points[0].clear();
    let mut spatial = line.clone();
    for p in &mut spatial.control_points {
        p.push(1.);
    }
    let mut weight = line;
    weight.weights[0] = 0.;
    for curve in [malformed, spatial, weight] {
        assert!(
            p::own_original_chart_events(&[curve], [[0., 1.], [0., 1.]], [true, true], 1e-12, 192)
                .is_err()
        );
    }
}

#[test]
fn original_chart_intervals_partition_full_source_at_irrational_shore_cut() {
    let a = Curve {
        degree: 2,
        knots: vec![2., 2., 2., 6., 6., 6.],
        control_points: vec![vec![0., 0.], vec![0., 0.5], vec![1., 1.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let events = Arc::new(
        p::own_original_chart_events(&[a], [[0.5, 2.], [-2., 2.]], [true, false], 1e-12, 192)
            .unwrap(),
    );
    let x = p::own_original_chart_intervals(events, p::ParameterOrderLimits::default()).unwrap();
    x.reverify_source().unwrap();
    assert_eq!(x.cuts()[0].len(), 3);
    assert_eq!(x.intervals().len(), 2);
    assert_eq!(
        x.intervals()[0].regions,
        [Some(p::OriginalChartRegion::Below), None]
    );
    assert_eq!(
        x.intervals()[1].regions,
        [Some(p::OriginalChartRegion::Interior), None]
    );
}
#[test]
fn original_chart_intervals_keep_tangent_sides_and_whole_shore_intervals() {
    let a = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![1., 0.], vec![-1., 0.5], vec![1., 1.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let b = Curve::from_polyline(vec![vec![0., 0.], vec![0., 1.]]).unwrap();
    let events = Arc::new(
        p::own_original_chart_events(&[a, b], [[0., 2.], [-2., 2.]], [true, false], 1e-12, 192)
            .unwrap(),
    );
    let x = p::own_original_chart_intervals(events, p::ParameterOrderLimits::default()).unwrap();
    x.reverify_source().unwrap();
    assert_eq!(x.intervals().len(), 3);
    assert!(
        x.intervals()
            .iter()
            .filter(|i| i.source == 0)
            .all(|i| i.regions[0] == Some(p::OriginalChartRegion::Interior))
    );
    assert_eq!(
        x.intervals()
            .iter()
            .find(|i| i.source == 1)
            .unwrap()
            .regions[0],
        Some(p::OriginalChartRegion::LowerShore)
    );
}
#[test]
fn original_chart_intervals_merge_corner_and_native_knot_provenance() {
    let a = Curve::from_polyline(vec![vec![-1., -1.], vec![0., 0.], vec![1., 1.]]).unwrap();
    let events = Arc::new(
        p::own_original_chart_events(&[a], [[0., 2.], [0., 2.]], [true, true], 1e-12, 192).unwrap(),
    );
    let x = p::own_original_chart_intervals(events, p::ParameterOrderLimits::default()).unwrap();
    x.reverify_source().unwrap();
    assert_eq!(x.cuts()[0].len(), 3);
    assert_eq!(x.intervals().len(), 2);
    assert_eq!(
        x.cuts()[0][1]
            .origins
            .iter()
            .filter(|x| matches!(x, p::OriginalChartCutOrigin::Event { .. }))
            .count(),
        4
    );
}

#[test]
fn original_chart_intervals_split_mixed_shore_and_off_shore_cells() {
    let a = Curve::from_polyline(vec![vec![0., 0.], vec![0., 1.], vec![1., 1.]]).unwrap();
    let e = Arc::new(
        p::own_original_chart_events(&[a], [[0., 2.], [-2., 2.]], [true, false], 1e-12, 192)
            .unwrap(),
    );
    let x = p::own_original_chart_intervals(e, p::ParameterOrderLimits::default()).unwrap();
    x.reverify_source().unwrap();
    assert_eq!(x.intervals().len(), 2);
    assert_eq!(
        x.intervals()[0].regions[0],
        Some(p::OriginalChartRegion::LowerShore)
    );
    assert_eq!(
        x.intervals()[1].regions[0],
        Some(p::OriginalChartRegion::Interior)
    );
    assert!(
        x.cuts()[0][1]
            .origins
            .iter()
            .any(|x| matches!(x, p::OriginalChartCutOrigin::Coincident { .. }))
    );
    assert!(
        x.cuts()[0][1]
            .origins
            .iter()
            .any(|x| matches!(x, p::OriginalChartCutOrigin::Event { .. }))
    );
}

#[test]
fn original_periodic_views_split_every_crossed_tile_and_own_exact_transitions() {
    let a = Curve::from_polyline(vec![vec![2.25, 0.], vec![4.25, 1.]]).unwrap();
    let x = p::own_original_periodic_pieces(
        &[a.clone()],
        [[0., 1.], [-2., 2.]],
        [true, false],
        1e-12,
        p::ParameterOrderLimits::default(),
        32,
    )
    .unwrap();
    x.reverify_source().unwrap();
    assert_eq!(x.pieces().len(), 3);
    assert_eq!(x.events().len(), 2);
    assert_eq!(x.transitions().len(), 2);
    assert_eq!(x.sources(), &[a]);
    assert_eq!(
        x.pieces()
            .iter()
            .map(|p| p.tiles[0].as_deref().unwrap())
            .collect::<Vec<_>>(),
        vec!["2", "3", "4"]
    );
    assert!(
        x.transitions()
            .iter()
            .all(|t| t.period_delta == ["1".to_string(), "0".to_string()])
    );
    let before = x.piece_endpoint(0, 1).unwrap();
    let after = x.piece_endpoint(1, 0).unwrap();
    assert_eq!(
        before.coordinates[0][0],
        vec![["1".to_string(), "1".to_string()]]
    );
    assert!(after.coordinates[0][0].is_empty());
}
#[test]
fn original_periodic_translation_keeps_nonrepresentable_rational_offset() {
    use num_rational::BigRational as Q;
    use num_traits::ToPrimitive;
    let a = Curve::from_polyline(vec![vec![0.8, 0.], vec![0.81, 1.]]).unwrap();
    let x = p::own_original_periodic_pieces(
        &[a.clone()],
        [[0.1, 0.3], [-2., 2.]],
        [true, false],
        1e-12,
        p::ParameterOrderLimits::default(),
        32,
    )
    .unwrap();
    x.reverify_source().unwrap();
    assert_eq!(x.pieces().len(), 1);
    assert_eq!(x.pieces()[0].tiles[0].as_deref(), Some("3"));
    let r = &x.pieces()[0].translation[0];
    let exact = Q::new(r[0].parse().unwrap(), r[1].parse().unwrap());
    assert_ne!(Q::from_float(exact.to_f64().unwrap()).unwrap(), exact);
    assert_eq!(x.sources(), &[a]);
    x.piece_endpoint(0, 0).unwrap();
    x.piece_endpoint(0, 1).unwrap();
}
#[test]
fn original_periodic_corner_transitions_and_lattice_budget_are_explicit() {
    let a = Curve::from_polyline(vec![vec![-0.5, -0.5], vec![1.5, 1.5]]).unwrap();
    let x = p::own_original_periodic_pieces(
        &[a.clone()],
        [[0., 1.], [0., 1.]],
        [true, true],
        1e-12,
        p::ParameterOrderLimits::default(),
        32,
    )
    .unwrap();
    x.reverify_source().unwrap();
    assert_eq!(x.pieces().len(), 3);
    assert!(
        x.transitions()
            .iter()
            .all(|t| t.period_delta == ["1".to_string(), "1".to_string()])
    );
    assert!(
        p::own_original_periodic_pieces(
            &[a],
            [[0., 1.], [0., 1.]],
            [true, true],
            1e-12,
            p::ParameterOrderLimits::default(),
            1
        )
        .is_err()
    );
}
#[test]
fn original_periodic_views_keep_irrational_lattice_cuts_on_nonlinear_source() {
    let a = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0., 0.], vec![0., 0.5], vec![4., 1.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let x = p::own_original_periodic_pieces(
        &[a.clone()],
        [[0., 1.], [-2., 2.]],
        [true, false],
        1e-12,
        p::ParameterOrderLimits::default(),
        32,
    )
    .unwrap();
    x.reverify_source().unwrap();
    assert_eq!(x.pieces().len(), 4);
    assert_eq!(x.transitions().len(), 3);
    assert!(
        x.events()
            .iter()
            .any(|e| e.parameter.interval[0] != e.parameter.interval[1])
    );
    assert_eq!(x.sources(), &[a]);
}
#[test]
fn original_periodic_views_keep_full_lattice_shore_identity() {
    let a = Curve::from_polyline(vec![vec![3., 0.], vec![3., 1.]]).unwrap();
    let x = p::own_original_periodic_pieces(
        &[a],
        [[0., 1.], [-2., 2.]],
        [true, false],
        1e-12,
        p::ParameterOrderLimits::default(),
        32,
    )
    .unwrap();
    x.reverify_source().unwrap();
    assert!(x.events().is_empty());
    assert_eq!(x.coincident_cells().len(), 1);
    assert_eq!(x.pieces().len(), 1);
    assert!(x.pieces()[0].on_shore[0]);
    assert_eq!(x.pieces()[0].tiles[0].as_deref(), Some("3"));
    assert!(x.piece_endpoint(0, 0).unwrap().coordinates[0][0].is_empty());
}

fn periodic_square_tube() -> nurbs_core::surface::Surface {
    let c = Curve::from_polyline(vec![
        vec![0., 0., 0.],
        vec![1., 0., 0.],
        vec![1., 1., 0.],
        vec![0., 1., 0.],
        vec![0., 0., 0.],
    ])
    .unwrap();
    nurbs_core::surface::extrude(&c, [0., 0., 1.]).unwrap()
}
#[test]
fn original_periodic_surface_admission_proves_actual_chart_and_surface_seams() {
    let surface = periodic_square_tube();
    let a = Curve::from_polyline(vec![vec![9., 0.2], vec![17., 0.8]]).unwrap();
    let pieces = Arc::new(
        p::own_original_periodic_pieces(
            &[a.clone()],
            [[0., 4.], [0., 1.]],
            [true, false],
            1e-12,
            p::ParameterOrderLimits::default(),
            32,
        )
        .unwrap(),
    );
    let x = p::bind_original_periodic_pieces_to_surface(
        pieces,
        &surface,
        p::ParameterOrderLimits::default(),
    )
    .unwrap();
    x.reverify_source().unwrap();
    assert!(x.seam_proofs()[0].is_some());
    assert!(x.seam_proofs()[1].is_none());
    assert_eq!(x.coverage().len(), 3);
    assert_eq!(x.pieces().sources(), &[a]);
}
#[test]
fn original_periodic_surface_admission_checks_actual_extrema_not_control_hull() {
    let surface = periodic_square_tube();
    let a = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![9., 0.], vec![13., 2.], vec![17., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let make = |c: &Curve| {
        Arc::new(
            p::own_original_periodic_pieces(
                &[c.clone()],
                [[0., 4.], [0., 1.]],
                [true, false],
                1e-12,
                p::ParameterOrderLimits::default(),
                32,
            )
            .unwrap(),
        )
    };
    let x = p::bind_original_periodic_pieces_to_surface(
        make(&a),
        &surface,
        p::ParameterOrderLimits::default(),
    )
    .unwrap();
    x.reverify_source().unwrap();
    assert!(
        x.coverage()
            .iter()
            .any(|c| c.critical_points.iter().sum::<usize>() > 0)
    );
    let mut outside = a;
    outside.control_points[1][1] = 2_f64.next_up();
    assert!(
        p::bind_original_periodic_pieces_to_surface(
            make(&outside),
            &surface,
            p::ParameterOrderLimits::default()
        )
        .is_err()
    );
}
#[test]
fn original_periodic_surface_admission_refuses_open_surface_despite_uv_wrapping() {
    let line = Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 0., 0.]]).unwrap();
    let surface = nurbs_core::surface::extrude(&line, [0., 0., 1.]).unwrap();
    let a = Curve::from_polyline(vec![vec![0.25, 0.2], vec![1.25, 0.8]]).unwrap();
    let pieces = Arc::new(
        p::own_original_periodic_pieces(
            &[a],
            [[0., 1.], [0., 1.]],
            [true, false],
            1e-12,
            p::ParameterOrderLimits::default(),
            32,
        )
        .unwrap(),
    );
    assert!(
        p::bind_original_periodic_pieces_to_surface(
            pieces,
            &surface,
            p::ParameterOrderLimits::default()
        )
        .is_err()
    );
}

fn periodic_support(sources: &[Curve]) -> Arc<p::OwnedOriginalPeriodicSurface> {
    let surface = periodic_square_tube();
    let pieces = Arc::new(
        p::own_original_periodic_pieces(
            sources,
            [[0., 4.], [0., 1.]],
            [true, false],
            1e-12,
            p::ParameterOrderLimits::default(),
            32,
        )
        .unwrap(),
    );
    Arc::new(
        p::bind_original_periodic_pieces_to_surface(
            pieces,
            &surface,
            p::ParameterOrderLimits::default(),
        )
        .unwrap(),
    )
}
#[test]
fn original_periodic_wire_closes_two_turns_and_preserves_reversed_orientation() {
    let a = Curve::from_polyline(vec![vec![9., 0.5], vec![17., 0.5]]).unwrap();
    let support = periodic_support(&[a]);
    for reversed in [false, true] {
        let x = p::own_original_periodic_wires(
            support.clone(),
            &[vec![p::OriginalPeriodicSourceUse {
                source: 0,
                reversed,
            }]],
        )
        .unwrap();
        x.reverify_source().unwrap();
        assert_eq!(x.wires()[0].uses.len(), 3);
        assert_eq!(x.wires()[0].joins.len(), 3);
        assert_eq!(
            x.wires()[0].homology[0].as_deref(),
            Some(if reversed { "-2" } else { "2" })
        );
    }
}
#[test]
fn original_periodic_wire_joins_different_sources_exactly_modulo_period() {
    let a = Curve::from_polyline(vec![vec![0.5, 0.2], vec![4.5, 0.8]]).unwrap();
    let b = Curve::from_polyline(vec![vec![0.5, 0.8], vec![0.5, 0.2]]).unwrap();
    let uses = vec![
        p::OriginalPeriodicSourceUse {
            source: 0,
            reversed: false,
        },
        p::OriginalPeriodicSourceUse {
            source: 1,
            reversed: false,
        },
    ];
    let x =
        p::own_original_periodic_wires(periodic_support(&[a.clone(), b.clone()]), &[uses.clone()])
            .unwrap();
    x.reverify_source().unwrap();
    assert_eq!(x.wires()[0].homology[0].as_deref(), Some("1"));
    let mut gap = b;
    gap.control_points[0][1] = 0.8_f64.next_up();
    assert!(p::own_original_periodic_wires(periodic_support(&[a, gap]), &[uses]).is_err());
}
#[test]
fn original_periodic_wire_source_coverage_cannot_omit_or_repeat_a_curve() {
    let a = Curve::from_polyline(vec![vec![0., 0.5], vec![4., 0.5]]).unwrap();
    let s = periodic_support(&[a]);
    let use_ = p::OriginalPeriodicSourceUse {
        source: 0,
        reversed: false,
    };
    assert!(p::own_original_periodic_wires(s.clone(), &[]).is_err());
    assert!(p::own_original_periodic_wires(s, &[vec![use_, use_]]).is_err());
}

fn periodic_wires_on_surface(
    surface: &nurbs_core::surface::Surface,
    domain: [[f64; 2]; 2],
) -> Arc<p::OwnedOriginalPeriodicWires> {
    let a = Curve::from_polyline(vec![vec![domain[0][0], 0.5], vec![domain[0][1], 0.5]]).unwrap();
    let pieces = Arc::new(
        p::own_original_periodic_pieces(
            &[a],
            domain,
            [true, false],
            1e-12,
            p::ParameterOrderLimits::default(),
            32,
        )
        .unwrap(),
    );
    let support = Arc::new(
        p::bind_original_periodic_pieces_to_surface(
            pieces,
            surface,
            p::ParameterOrderLimits::default(),
        )
        .unwrap(),
    );
    Arc::new(
        p::own_original_periodic_wires(
            support,
            &[vec![p::OriginalPeriodicSourceUse {
                source: 0,
                reversed: false,
            }]],
        )
        .unwrap(),
    )
}
#[test]
fn original_periodic_embedding_proves_full_rank_cover_and_every_chart_pair() {
    let s = periodic_square_tube();
    let wires = periodic_wires_on_surface(&s, [[0., 4.], [0., 1.]]);
    let x = p::own_original_periodic_embedding(
        wires,
        4096,
        p::OriginalInjectiveAtlasLimits::default(),
        p::OriginalAtlasPairLimits::default(),
    )
    .unwrap();
    x.reverify_source().unwrap();
    assert_eq!(x.charts().len(), 4);
    assert_eq!(x.pairs().len(), 6);
    assert!(x.pairs().iter().all(|p| p.proof.is_proven()));
    assert!(x.regularity().proven_cells > 0);
}
#[test]
fn original_periodic_embedding_refuses_collapsed_and_double_covered_supports() {
    let mut collapsed = periodic_square_tube();
    for row in &mut collapsed.control_points {
        for p in row {
            p[0] = 0.;
            p[1] = 0.;
        }
    }
    let wires = periodic_wires_on_surface(&collapsed, [[0., 4.], [0., 1.]]);
    assert!(
        p::own_original_periodic_embedding(
            wires,
            64,
            p::OriginalInjectiveAtlasLimits::default(),
            p::OriginalAtlasPairLimits::default()
        )
        .is_err()
    );
    let curve =
        Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 0., 0.], vec![0., 0., 0.]]).unwrap();
    let double = nurbs_core::surface::extrude(&curve, [0., 0., 1.]).unwrap();
    let wires = periodic_wires_on_surface(&double, [[0., 2.], [0., 1.]]);
    assert!(
        nurbs_core::surface_regularity::inspect(&double, 64)
            .unwrap()
            .spanwise_regular
    );
    let limits = p::OriginalAtlasPairLimits {
        max_pair_depth: 0,
        max_span_work: 4096,
        ..Default::default()
    };
    let error = p::own_original_periodic_embedding(
        wires,
        64,
        p::OriginalInjectiveAtlasLimits::default(),
        limits,
    )
    .unwrap_err();
    assert!(error.message.contains("cross-chart embedding"));
}

#[test]
fn translated_original_span_census_keeps_curved_and_nonlinear_common_clocks() {
    let a = Curve {
        degree: 2,
        knots: vec![2.,2.,2.,6.,6.,6.],
        control_points: vec![vec![0.,0.],vec![0.5,0.],vec![1.,1.]],
        weights: vec![1.;3], periodic: false,
    };
    let mut b = a.clone(); b.control_points.reverse();
    let third = ["1".to_string(),"3".to_string()];
    let zero = ["0".to_string(),"1".to_string()];
    let offsets = [[third.clone(),third.clone()],[third.clone(),third.clone()]];
    let o = p::own_translated_span_intersections(&a,&b,&offsets,false,1e-12).unwrap();
    let v = o.reverify_source().unwrap();
    assert_eq!(v.overlaps().len(),1);
    assert_eq!(v.overlaps()[0].first_domain,[2.,6.]);
    assert_eq!(o.sources(),&[a.clone(),b]);
    assert_eq!(v.overlaps()[0].image_range[0],third.clone());
    let mut nonlinear = a;
    nonlinear.control_points = vec![vec![0.,0.],vec![0.,0.],vec![1.,0.]];
    let line = Curve::from_polyline(vec![vec![0.,0.],vec![1.,0.]]).unwrap();
    let o = p::own_translated_span_intersections(&nonlinear,&line,&[[zero.clone(),zero.clone()],[third.clone(),zero]],false,1e-12).unwrap();
    let v = o.reverify_source().unwrap();
    assert_eq!(v.overlaps().len(),1);
    assert_eq!(v.overlaps()[0].image_range[0],third);
    assert!(o.certificate().pairs.iter().any(|p| p.relation.len()>1));
}

#[test]
fn translated_exact_nodes_and_common_endpoints_keep_source_provenance() {
    let a=Curve::from_polyline(vec![vec![0.,0.],vec![1.,0.]]).unwrap();
    let b=Curve::from_polyline(vec![vec![0.,-1.],vec![0.,1.]]).unwrap();
    let z=["0".to_string(),"1".to_string()];
    let t=["1".to_string(),"3".to_string()];
    let offsets=[[z.clone(),z.clone()],[t,z.clone()]];
    let o=p::own_translated_span_intersections(&a,&b,&offsets,false,1e-12).unwrap();
    let nodes=o.exact_nodes().unwrap();
    assert_eq!(nodes.len(),1);assert_eq!(nodes[0].source_cells,[0,0]);
    assert!(o.exact_overlap_endpoints().unwrap().is_empty());
    assert_eq!(o.sources(),&[a,b]);
    assert!(!nodes[0].exact.parameters[0].is_empty());
}

#[test]
fn selected_union_winding_accepts_a_point_on_an_excluded_part_of_original_cell() {
    let a=Curve::from_polyline(vec![vec![0.,0.],vec![3.,0.],vec![3.,1.],vec![0.,1.],vec![0.,0.]]).unwrap();
    let b=Curve::from_polyline(vec![vec![1.,-1.],vec![2.,-1.],vec![2.,0.5],vec![1.,0.5],vec![1.,-1.]]).unwrap();
    let filled=unified_filled(&[vec![a],vec![b]],&[1,1]);
    assert_eq!(filled.boundaries().len(),1);
    let point=[["3".to_string(),"2".to_string()],["0".to_string(),"1".to_string()]];
    let query=p::own_original_unified_boundary_winding(filled.clone(),point,p::ParameterOrderLimits::default()).unwrap();
    query.reverify_source().unwrap();assert_eq!(query.windings(),&[1]);
    let boundary=[["0".to_string(),"1".to_string()],["0".to_string(),"1".to_string()]];
    assert!(p::own_original_unified_boundary_winding(filled,boundary,p::ParameterOrderLimits::default()).is_err());
}
