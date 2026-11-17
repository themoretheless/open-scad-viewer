use brep_core::feature_ops::periodic_trim_sheet::build_exact_periodic_trim_sheet_with_bindings;
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
