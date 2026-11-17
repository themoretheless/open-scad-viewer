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
