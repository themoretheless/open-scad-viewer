use brep_core::feature_ops::periodic_trim_sheet::{
    build_exact_periodic_trim_sheet_with_bindings,
    build_exact_periodic_trim_sheet_with_edge_bindings,
    build_exact_periodic_trim_sheet_with_input_bindings,
};
use nurbs_core::{
    curve::Curve,
    surface::{Surface, extrude},
};

fn support() -> Surface {
    extrude(
        &Curve {
            degree: 1,
            knots: vec![-1., 0., 1., 2., 3., 4., 5.],
            control_points: vec![
                vec![0., 0., 0.],
                vec![1., 0., 0.],
                vec![1., 1., 0.],
                vec![0., 1., 0.],
                vec![0., 0., 0.],
            ],
            weights: vec![1.; 5],
            periodic: true,
        },
        [0., 0., 2.],
    )
    .unwrap()
}

#[test]
fn source_order_maps_hole_and_island_to_persistent_wires() {
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let hole = [
        [0.125, 0.25],
        [0.375, 0.25],
        [0.375, 0.75],
        [0.125, 0.75],
        [0.125, 0.25],
    ];
    let island = [
        [0.1875, 0.375],
        [0.3125, 0.375],
        [0.3125, 0.625],
        [0.1875, 0.625],
        [0.1875, 0.375],
    ];
    let report = build_exact_periodic_trim_sheet_with_bindings(
        &support(),
        &[&hole, &island, &outer],
        1e-8,
        1_000_000,
    )
    .unwrap();
    let model = report.sheet.model.as_ref().unwrap();
    let [hole, island, outer] = report.contour_bindings.as_slice() else {
        panic!("Expected one binding per original contour");
    };
    assert!(hole.is_hole);
    assert!(!island.is_hole);
    assert!(!outer.is_hole);
    assert_eq!(hole.face_id, outer.face_id);
    assert_ne!(island.face_id, outer.face_id);
    let outer_face = model
        .1
        .faces
        .iter()
        .position(|id| id == &outer.face_id)
        .unwrap();
    let island_face = model
        .1
        .faces
        .iter()
        .position(|id| id == &island.face_id)
        .unwrap();
    assert_eq!(outer.loop_id, model.1.loops[model.faces[outer_face].outer]);
    assert_eq!(
        hole.loop_id,
        model.1.loops[model.faces[outer_face].holes[0]]
    );
    assert_eq!(
        island.loop_id,
        model.1.loops[model.faces[island_face].outer]
    );
}

#[test]
fn refused_build_has_no_contour_bindings() {
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let report =
        build_exact_periodic_trim_sheet_with_bindings(&support(), &[&outer], 1e-8, 1).unwrap();
    assert!(report.sheet.refusal.is_some());
    assert!(report.sheet.model.is_none());
    assert!(report.contour_bindings.as_slice().is_empty());
}

#[test]
fn split_segment_intervals_resolve_to_their_persistent_brep_edges() {
    let hole = [
        [0.125, 0.25],
        [0.375, 0.25],
        [0.375, 0.75],
        [0.125, 0.75],
        [0.125, 0.25],
    ];
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let result = build_exact_periodic_trim_sheet_with_edge_bindings(
        &support(),
        &[&hole, &outer],
        1e-8,
        1_000_000,
    )
    .unwrap();
    let model = result.bound_sheet.sheet.model.unwrap();
    assert_eq!(result.edge_bindings.len(), model.edges.len());
    let face = &model.faces[0];
    for (wire, contour) in [(face.outer, 1), (face.holes[0], 0)] {
        for coedge in &model.loops[wire].coedges {
            let binding = &result.edge_bindings[coedge.edge];
            assert_eq!(binding.edge_id, model.1.edges[coedge.edge]);
            assert_eq!(binding.source.contour, contour);
            let segment = binding.source.segment as f64;
            assert_eq!(
                binding.source.interval,
                model.edges[coedge.edge].curve.domain().map(|t| t - segment),
            );
        }
    }
    let split: Vec<_> = result
        .edge_bindings
        .iter()
        .filter(|binding| binding.source.contour == 1 && binding.source.segment == 0)
        .map(|binding| binding.source.interval)
        .collect();
    assert_eq!(split, [[0., 0.5], [0.5, 1.]]);
}

#[test]
fn refused_build_withholds_persistent_edge_sources() {
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let report =
        build_exact_periodic_trim_sheet_with_edge_bindings(&support(), &[&outer], 1e-8, 1).unwrap();
    assert!(report.bound_sheet.sheet.model.is_none());
    assert!(report.bound_sheet.sheet.refusal.is_some());
    assert!(report.edge_bindings.is_empty());
}

#[test]
fn input_directed_edge_bindings_follow_persistent_wire_order() {
    let outer = [
        [3.5, 0.125],
        [3.5, 0.875],
        [0.5, 0.875],
        [0.5, 0.125],
        [3.5, 0.125],
    ];
    let hole = [
        [0.125, 0.25],
        [0.375, 0.25],
        [0.375, 0.75],
        [0.125, 0.75],
        [0.125, 0.25],
    ];
    let result = build_exact_periodic_trim_sheet_with_input_bindings(
        &support(),
        &[&hole, &outer],
        1e-8,
        1_000_000,
    )
    .unwrap();
    let model = result.bound_sheet.sheet.model.unwrap();
    assert_eq!(result.edge_bindings.len(), model.edges.len());
    for (binding, id) in result.edge_bindings.iter().zip(&model.1.edges) {
        assert_eq!(&binding.edge_id, id);
        assert!(binding.source.parameters[0] > binding.source.parameters[1]);
    }
    let split = &result.edge_bindings[..2];
    assert!(
        split
            .iter()
            .all(|entry| entry.source.contour == 1 && entry.source.segment == 3)
    );
    assert_eq!(split[0].source.parameters[1], split[1].source.parameters[0]);
    let hole_edge = model.loops[model.faces[0].holes[0]].coedges[0].edge;
    assert_eq!(result.edge_bindings[hole_edge].source.contour, 0);
    assert_eq!(result.edge_bindings[hole_edge].source.segment, 3);
}

#[test]
fn refused_build_withholds_input_directed_bindings() {
    let outer = [
        [3.5, 0.125],
        [3.5, 0.875],
        [0.5, 0.875],
        [0.5, 0.125],
        [3.5, 0.125],
    ];
    let result =
        build_exact_periodic_trim_sheet_with_input_bindings(&support(), &[&outer], 1e-8, 1)
            .unwrap();
    assert!(result.bound_sheet.sheet.model.is_none());
    assert!(result.bound_sheet.sheet.refusal.is_some());
    assert!(result.edge_bindings.is_empty());
}

#[test]
fn input_parameter_locates_both_seam_edges_and_exact_reversed_clock() {
    use num_rational::BigRational;
    let rational = |n: i64, d: i64| BigRational::new(n.into(), d.into());
    let outer = [
        [3.5, 0.125],
        [3.5, 0.875],
        [0.5, 0.875],
        [0.5, 0.125],
        [3.5, 0.125],
    ];
    let result =
        build_exact_periodic_trim_sheet_with_input_bindings(&support(), &[&outer], 1e-8, 1_000_000)
            .unwrap();
    let hits = result
        .locate_input_parameter(0, 3, &rational(1, 2))
        .unwrap();
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].edge_id, result.edge_bindings[0].edge_id);
    assert_eq!(hits[1].edge_id, result.edge_bindings[1].edge_id);
    assert_eq!(hits[0].parameter, rational(1, 2));
    assert_eq!(hits[1].parameter, rational(1, 2));
    let hits = result
        .locate_input_parameter(0, 3, &rational(1, 3))
        .unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].edge_id, result.edge_bindings[1].edge_id);
    assert_eq!(hits[0].parameter, rational(2, 3));
    assert!(
        result
            .locate_input_parameter(0, 3, &rational(4, 3))
            .unwrap()
            .is_empty()
    );
    assert!(
        result
            .locate_input_parameter(1, 3, &rational(1, 3))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn input_parameter_lookup_skips_refused_sheet_and_detects_identity_mismatch() {
    use num_rational::BigRational;
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let parameter = BigRational::new(1.into(), 3.into());
    let refused =
        build_exact_periodic_trim_sheet_with_input_bindings(&support(), &[&outer], 1e-8, 1)
            .unwrap();
    assert!(
        refused
            .locate_input_parameter(0, 0, &parameter)
            .unwrap()
            .is_empty()
    );
    let mut result =
        build_exact_periodic_trim_sheet_with_input_bindings(&support(), &[&outer], 1e-8, 1_000_000)
            .unwrap();
    result.edge_bindings.swap(0, 1);
    assert!(result.locate_input_parameter(0, 0, &parameter).is_err());
}

#[test]
fn edge_parameter_round_trips_to_original_contours_and_includes_split_endpoints() {
    use num_rational::BigRational;
    let rational = |n: i64, d: i64| BigRational::new(n.into(), d.into());
    let outer = [
        [3.5, 0.125],
        [3.5, 0.875],
        [0.5, 0.875],
        [0.5, 0.125],
        [3.5, 0.125],
    ];
    let hole = [
        [0.125, 0.25],
        [0.375, 0.25],
        [0.375, 0.75],
        [0.125, 0.75],
        [0.125, 0.25],
    ];
    let result = build_exact_periodic_trim_sheet_with_input_bindings(
        &support(),
        &[&outer, &hole],
        1e-8,
        1_000_000,
    )
    .unwrap();
    for (contour, segment, parameter, count) in [
        (0, 3, rational(1, 2), 2),
        (0, 3, rational(1, 3), 1),
        (1, 3, rational(2, 3), 1),
    ] {
        let hits = result
            .locate_input_parameter(contour, segment, &parameter)
            .unwrap();
        assert_eq!(hits.len(), count);
        for hit in hits {
            let original = result
                .locate_edge_parameter(hit.edge_id, &hit.parameter)
                .unwrap()
                .unwrap();
            assert_eq!(
                (original.contour, original.segment, original.parameter),
                (contour, segment, parameter.clone())
            );
        }
    }
}

#[test]
fn edge_parameter_returns_no_hit_outside_domain_or_for_unknown_id() {
    use num_rational::BigRational;
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let result =
        build_exact_periodic_trim_sheet_with_input_bindings(&support(), &[&outer], 1e-8, 1_000_000)
            .unwrap();
    let model = result.bound_sheet.sheet.model.as_ref().unwrap();
    let outside = BigRational::new(3.into(), 4.into());
    assert!(
        result
            .locate_edge_parameter(model.1.edges[0], &outside)
            .unwrap()
            .is_none()
    );
    assert!(
        result
            .locate_edge_parameter(model.1.vertices[0], &outside)
            .unwrap()
            .is_none()
    );
}

#[test]
fn input_range_clips_both_seam_edges_in_requested_order() {
    use num_rational::BigRational;
    let q = |n: i64, d: i64| BigRational::new(n.into(), d.into());
    let outer = [
        [3.5, 0.125],
        [3.5, 0.875],
        [0.5, 0.875],
        [0.5, 0.125],
        [3.5, 0.125],
    ];
    let result =
        build_exact_periodic_trim_sheet_with_input_bindings(&support(), &[&outer], 1e-8, 1_000_000)
            .unwrap();
    let pieces = result
        .locate_input_range(0, 3, [&q(1, 3), &q(3, 4)])
        .unwrap();
    assert_eq!(pieces.len(), 2);
    assert_eq!(pieces[0].edge_id, result.edge_bindings[1].edge_id);
    assert_eq!(pieces[1].edge_id, result.edge_bindings[0].edge_id);
    assert_eq!(pieces[0].input_parameters, [q(1, 3), q(1, 2)]);
    assert_eq!(pieces[1].input_parameters, [q(1, 2), q(3, 4)]);
    assert_eq!(pieces[0].edge_parameters, [q(2, 3), q(1, 2)]);
    assert_eq!(pieces[1].edge_parameters, [q(1, 2), q(1, 4)]);
    let reverse = result
        .locate_input_range(0, 3, [&q(3, 4), &q(1, 3)])
        .unwrap();
    assert_eq!(reverse.len(), pieces.len());
    for (forward, reverse) in pieces.iter().zip(reverse.iter().rev()) {
        assert_eq!(forward.edge_id, reverse.edge_id);
        assert_eq!(
            forward.input_parameters,
            [
                reverse.input_parameters[1].clone(),
                reverse.input_parameters[0].clone()
            ]
        );
        assert_eq!(
            forward.edge_parameters,
            [
                reverse.edge_parameters[1].clone(),
                reverse.edge_parameters[0].clone()
            ]
        );
    }
}

#[test]
fn input_range_clips_to_segment_and_omits_point_contacts() {
    use num_rational::BigRational;
    let q = |n: i64, d: i64| BigRational::new(n.into(), d.into());
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let result =
        build_exact_periodic_trim_sheet_with_input_bindings(&support(), &[&outer], 1e-8, 1_000_000)
            .unwrap();
    let pieces = result
        .locate_input_range(0, 0, [&q(-1, 3), &q(1, 2)])
        .unwrap();
    assert_eq!(pieces.len(), 1);
    assert_eq!(pieces[0].input_parameters, [q(0, 1), q(1, 2)]);
    assert!(
        result
            .locate_input_range(0, 0, [&q(1, 2), &q(1, 2)])
            .unwrap()
            .is_empty()
    );
    assert!(
        result
            .locate_input_range(0, 0, [&q(2, 1), &q(3, 1)])
            .unwrap()
            .is_empty()
    );
}

#[test]
fn extract_input_range_curves_preserves_exact_seam_geometry_and_direction() {
    use num_rational::BigRational;
    let q = |n: i64, d: i64| BigRational::new(n.into(), d.into());
    let outer = [
        [3.5, 0.125],
        [3.5, 0.875],
        [0.5, 0.875],
        [0.5, 0.125],
        [3.5, 0.125],
    ];
    let result =
        build_exact_periodic_trim_sheet_with_input_bindings(&support(), &[&outer], 1e-8, 1_000_000)
            .unwrap();
    let report = result
        .extract_input_range_curves(0, 3, [&q(1, 4), &q(3, 4)], 1_000_000)
        .unwrap();
    let pieces = report.pieces.unwrap();
    assert_eq!(pieces.len(), 2);
    assert_eq!(pieces[0].curve.domain(), [0.5, 0.75]);
    assert_eq!(pieces[1].curve.domain(), [0.25, 0.5]);
    assert_eq!(pieces[0].range.edge_parameters, [q(3, 4), q(1, 2)]);
    assert_eq!(pieces[1].range.edge_parameters, [q(1, 2), q(1, 4)]);
    let model = result.bound_sheet.sheet.model.unwrap();
    for piece in pieces {
        let index = model
            .1
            .edges
            .iter()
            .position(|id| *id == piece.range.edge_id)
            .unwrap();
        let [lo, hi] = piece.curve.domain();
        for parameter in [lo, (lo + hi) / 2., hi] {
            assert_eq!(
                piece.curve.evaluate(parameter).unwrap().point,
                model.edges[index].curve.evaluate(parameter).unwrap().point
            );
        }
    }
}

#[test]
fn extract_input_range_curves_refuses_unstorable_cuts_and_exhausted_work() {
    use num_rational::BigRational;
    use nurbs_core::editing::rational_factor::Refusal;
    let q = |n: i64, d: i64| BigRational::new(n.into(), d.into());
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let result =
        build_exact_periodic_trim_sheet_with_input_bindings(&support(), &[&outer], 1e-8, 1_000_000)
            .unwrap();
    let refused = result
        .extract_input_range_curves(0, 0, [&q(1, 4), &q(2, 3)], 1_000_000)
        .unwrap();
    assert!(refused.pieces.is_none());
    assert_eq!(refused.refusal, Some(Refusal::NotRepresentable));
    assert!(refused.work_used > 0);
    let exhausted = result
        .extract_input_range_curves(0, 0, [&q(1, 4), &q(3, 4)], 1)
        .unwrap();
    assert!(exhausted.pieces.is_none());
    assert_eq!(exhausted.refusal, Some(Refusal::ResourceLimit));
}

#[test]
fn extracted_curve_pairs_preserve_face_lift_and_shared_seam_clock() {
    use num_rational::BigRational;
    let q = |n: i64, d: i64| BigRational::new(n.into(), d.into());
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let result =
        build_exact_periodic_trim_sheet_with_input_bindings(&support(), &[&outer], 1e-8, 1_000_000)
            .unwrap();
    let report = result
        .extract_input_range_curve_pairs(0, 0, [&q(1, 4), &q(3, 4)], 1_000_000)
        .unwrap();
    let pairs = report.pieces.unwrap();
    assert_eq!(pairs.len(), 2);
    let model = result.bound_sheet.sheet.model.unwrap();
    for pair in &pairs {
        assert_eq!(pair.face_id, model.1.faces[0]);
        assert_eq!(pair.pcurve.domain(), pair.world.curve.domain());
        let [lo, hi] = pair.pcurve.domain();
        for t in [lo, (lo + hi) / 2., hi] {
            let uv = pair.pcurve.evaluate(t).unwrap().point;
            let lifted = model.faces[0].surface.evaluate(uv[0], uv[1]).unwrap().point;
            assert_eq!(pair.world.curve.evaluate(t).unwrap().point, lifted);
        }
    }
    assert_eq!(
        pairs[0].pcurve.evaluate(0.5).unwrap().point,
        pairs[1].pcurve.evaluate(0.5).unwrap().point
    );
}

#[test]
fn curve_pair_budget_withholds_world_pieces_when_uv_budget_is_exhausted() {
    use num_rational::BigRational;
    use nurbs_core::editing::rational_factor::Refusal;
    let q = |n: i64, d: i64| BigRational::new(n.into(), d.into());
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let result =
        build_exact_periodic_trim_sheet_with_input_bindings(&support(), &[&outer], 1e-8, 1_000_000)
            .unwrap();
    let range = [&q(1, 4), &q(3, 4)];
    let world = result
        .extract_input_range_curves(0, 0, range, 1_000_000)
        .unwrap();
    assert!(world.work_used > 0);
    let limited = result
        .extract_input_range_curve_pairs(0, 0, range, world.work_used)
        .unwrap();
    assert!(limited.pieces.is_none());
    assert_eq!(limited.refusal, Some(Refusal::ResourceLimit));
}
