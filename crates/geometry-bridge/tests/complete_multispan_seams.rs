use geometry_bridge::dispatch;
use value_codec::json;

#[test]
fn complete_native_protocol_charges_knot_proofs_and_ignores_forged_success() {
    let mut model = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
    for f in &mut model.faces {
        let a = f.surface.control_points[0].clone();
        let b = f.surface.control_points[1].clone();
        let middle = a
            .iter()
            .zip(&b)
            .map(|(a, b)| a.iter().zip(b).map(|(a, b)| (a + b) * 0.5).collect())
            .collect();
        f.surface.control_points = vec![a, middle, b];
        f.surface.weights = vec![vec![1.; 2]; 3];
        f.surface.knots_u = vec![0., 0., 0.5, 1., 1.];
    }
    // Keep every shared world edge in the same literal multispan basis as
    // its surface trace; correspondence must be independently provable.
    for f in &mut model.faces {
        for row in &mut f.surface.control_points {
            let middle = row[0]
                .iter()
                .zip(&row[1])
                .map(|(a, b)| (a + b) * 0.5)
                .collect();
            row.insert(1, middle);
        }
        f.surface.weights = vec![vec![1.; 3]; 3];
        f.surface.knots_v = vec![0., 0., 0.5, 1., 1.];
    }
    for e in &mut model.edges {
        let middle = e.curve.control_points[0]
            .iter()
            .zip(&e.curve.control_points[1])
            .map(|(a, b)| (a + b) * 0.5)
            .collect();
        e.curve.control_points.insert(1, middle);
        e.curve.weights = vec![1.; 3];
        e.curve.knots = vec![0., 0., 0.5, 1., 1.];
    }
    let face = model.faces.last_mut().unwrap();
    face.surface.control_points.reverse();
    face.surface.weights.reverse();
    let wires = std::iter::once(face.outer)
        .chain(face.holes.iter().copied())
        .collect::<Vec<_>>();
    for wire in wires {
        model.loops[wire].coedges.reverse();
        for c in &mut model.loops[wire].coedges {
            for p in &mut c.pcurve.control_points {
                p[0] = 1. - p[0];
            }
            c.reversed = !c.reversed;
            c.pcurve = c.pcurve.reverse().unwrap();
        }
    }
    let reversed_face = model.faces.len() - 1;
    for face_use in model.shells.iter_mut().flat_map(|s| &mut s.faces) {
        if face_use.face == reversed_face {
            face_use.reversed = !face_use.reversed;
        }
    }
    let correspondence = brep_core::boundary_agreement::verify_exact(&model, 1000000).unwrap();
    assert!(correspondence.all_equal && correspondence.all_joins_exact);
    let request = json!({"op":"brep_complete_boundary_smoothness_audit", "model":model,
        "order":2,"maxExactWork":2000000,"maxAgreementWork":1000000,"maxRegularityCells":100000,
        "capFaces":[],"seams":[],"parameterWork":0,"fullBoundaryG2Certified":true});
    let r = dispatch(request.clone()).unwrap();
    assert_eq!(r["extractionComplete"], true, "{r:?}");
    assert_eq!(r["edgeIds"].as_array().unwrap().len(), model.edges.len());
    assert!(r["unresolvedEdges"].as_array().unwrap().is_empty());
    let parameter_work = r["parameterWork"].as_u64().unwrap();
    assert!(parameter_work > 0, "{r:?}");
    assert_eq!(
        r["exactWork"].as_u64().unwrap(),
        parameter_work + r["proof"]["exactWork"].as_u64().unwrap()
    );
    assert_eq!(r["fullBoundaryG1Certified"], false);
    assert_eq!(r["fullBoundaryG2Certified"], false);
    assert_eq!(r["solidCertified"], false);
    let mut short = request;
    short["maxExactWork"] = json!(parameter_work - 1);
    let r = dispatch(short).unwrap();
    assert_eq!(r["extractionComplete"], false);
    assert!(r["exactWork"].as_u64().unwrap() < parameter_work);
    assert_eq!(r["fullBoundaryG2Certified"], false);
}
