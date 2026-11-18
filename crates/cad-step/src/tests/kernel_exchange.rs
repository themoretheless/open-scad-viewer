//! STEP and IGES round trips of models built by kernel features. They live here because
//! `brep-core` unit tests cannot call `cad-step` (ADR 0011).
use brep_core::close_topology::{BodyRole, ComplexPart, FaceRef, MixedDimensionalBrep, SharedFace};
use brep_core::cuboid;

fn solid(model: brep_core::Model) -> ComplexPart {
    ComplexPart { role: BodyRole::Solid, model }
}

#[test]
fn step_roundtrip_not_faceted() {
    let model = cuboid([0., 0., 0.], [3., 2., 1.]).unwrap();
    let (text, cert) = crate::export_step(&model).unwrap();
    assert!(cert.complete);
    assert!(text.contains("ADVANCED_FACE"));
    assert!(text.contains("PLANE"));
    assert!(text.contains("VERTEX_POINT"));
    assert!(text.contains("EDGE_CURVE"));
    assert!(text.contains("AP242"));
    assert!(!text.contains("FACETED_BREP"));
    let (back, _) = crate::import_step(&text).unwrap();
    back.validate().unwrap();
}

#[test]
fn mixed_complex_round_trips_through_step_and_iges() {
    let a = cuboid([0.; 3], [1.; 3]).unwrap();
    let b = a.clone();
    let relation = SharedFace {
        uses: vec![
            FaceRef { part: 0, face: 0, reversed: false },
            FaceRef { part: 1, face: 0, reversed: true },
        ],
    };
    let mixed = MixedDimensionalBrep::new(vec![solid(a), solid(b)], vec![relation], vec![], vec![], vec![])
        .unwrap()
        .audit()
        .unwrap();
    let (step, _) = crate::export_complex_step(&mixed).unwrap();
    let (step_back, step_cert) = crate::import_complex_step(&step).unwrap();
    assert_eq!(step_back.certificate().shared_face_count, 1);
    assert!(step_cert.supplemental_incidence_preserved);
    let (iges, _) = crate::export_complex_iges(&mixed).unwrap();
    let (iges_back, iges_cert) = crate::import_complex_iges(&iges).unwrap();
    assert_eq!(iges_back.boundary_faces().len(), 10);
    assert!(iges_cert.direct_manifold_payloads);
}

#[test]
fn malformed_complex_step_is_refused() {
    assert_eq!(
        crate::import_complex_step("OSV-CLOSE-TOPOLOGY-STEP/1\nBOGUS\nEND").unwrap_err().code,
        "BREP_COMPLEX_INTERCHANGE_REFUSED"
    );
}
