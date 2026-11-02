use brep_core::{face_cut, fillet_radius};
use nurbs_core::{curve::Curve, offset::fillet_surfaces::RadiusLaw};

#[test]
fn independently_trimmed_skin_faces_keep_saved_contact_center_law() {
    let source = brep_core::cuboid([0.; 3], [2.; 3]).unwrap();
    let vertex = |p| source.vertices.iter().position(|v| v.point == p).unwrap();
    let a = vertex([0., 0., 0.]);
    let b = vertex([2., 0., 2.]);
    let skin = source.faces.iter().position(|f| f.surface.control_points.iter()
        .flatten().all(|p| p[1] == 0.)).unwrap();
    let cut = Curve::from_polyline(vec![vec![0., 0., 0.], vec![2., 0., 2.]]).unwrap();
    let split = face_cut::partition_projected(&source, skin, [a, b], &[cut], 1e-8, 100000).unwrap();
    let model = &split.model;
    let contacts = [0., 2.].map(|x| model.edges.iter().position(|e| {
        let p = model.vertices[e.vertices[0]].point;
        let q = model.vertices[e.vertices[1]].point;
        p[0] == x && q[0] == x && p[1] == 0. && q[1] == 0. && p[2] != q[2]
    }).unwrap());
    let skins = contacts.map(|edge| *split.faces.iter().find(|&&f|
        model.loops[model.faces[f].outer].coedges.iter().any(|c| c.edge == edge)
    ).unwrap());
    assert_ne!(skins[0], skins[1]);
    let mut admitted = 0;
    let mut admitted_sides = None;
    for sides in [[1., 1.], [1., -1.], [-1., 1.], [-1., -1.]] {
        let proof = fillet_radius::inspect_model_skins(model, skins, contacts,
            &RadiusLaw::Constant(1.), model.edges[contacts[0]].curve.domain(),
            sides, 1e-7, 100000, 1000000).unwrap();
        if proof.decision == fillet_radius::Decision::WithinTolerance { admitted += 1; admitted_sides = Some(sides); }
        else { assert_eq!(proof.decision, fillet_radius::Decision::ExceedsTolerance, "{}", proof.reason); }
    }
    assert_eq!(admitted, 1);
    let sides = admitted_sides.unwrap();
    let restored: brep_core::Model = value_codec::from_str(
        &value_codec::to_string(model).unwrap()).unwrap();
    assert_eq!(fillet_radius::inspect_model_skins(&restored, skins, contacts,
        &RadiusLaw::Constant(1.), restored.edges[contacts[0]].curve.domain(),
        sides, 1e-7, 100000, 1000000).unwrap().decision,
        fillet_radius::Decision::WithinTolerance);
    assert_eq!(fillet_radius::inspect_model_skins(&restored, skins, contacts,
        &RadiusLaw::Constant(1.), restored.edges[contacts[0]].curve.domain(),
        sides, 1e-7, 0, 0).unwrap().decision,
        fillet_radius::Decision::Unresolved);
    // Swapping actual skin roles must not accidentally inspect the old face twice.
    assert!(fillet_radius::inspect_model_skins(model, [skins[1], skins[0]], contacts,
        &RadiusLaw::Constant(1.), [0., 1.], [1., -1.], 1e-7, 100000, 1000000).is_err());
    assert_eq!(source.faces.len(), 6);
    assert_eq!(model.faces.len(), 7);
}
