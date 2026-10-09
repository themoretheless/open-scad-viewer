//! STEP and IGES envelopes for audited topology complexes.
use brep_core::{BodyRole, ComplexPart, FaceRef, MixedDimensionalBrep, Model, SharedFace, cuboid};
use cad_step::{
    export_complex_iges, export_complex_step, import_complex_iges, import_complex_step,
};

fn solid(model: Model) -> ComplexPart {
    ComplexPart {
        role: BodyRole::Solid,
        model,
    }
}

#[test]
fn shared_face_complex_round_trips_through_step_and_iges() {
    let a = cuboid([0.; 3], [1.; 3]).unwrap();
    let b = a.clone();
    let relation = SharedFace {
        uses: vec![
            FaceRef {
                part: 0,
                face: 0,
                reversed: false,
            },
            FaceRef {
                part: 1,
                face: 0,
                reversed: true,
            },
        ],
    };
    let mixed = MixedDimensionalBrep::new(
        vec![solid(a), solid(b)],
        vec![relation],
        vec![],
        vec![],
        vec![],
    )
    .unwrap()
    .audit()
    .unwrap();
    assert_eq!(mixed.boundary_faces().len(), 10);
    let (step, _) = export_complex_step(&mixed).unwrap();
    let (step_back, step_cert) = import_complex_step(&step).unwrap();
    assert_eq!(step_back.certificate().shared_face_count, 1);
    assert!(step_cert.supplemental_incidence_preserved);
    let (iges, _) = export_complex_iges(&mixed).unwrap();
    let (iges_back, iges_cert) = import_complex_iges(&iges).unwrap();
    assert_eq!(iges_back.boundary_faces().len(), 10);
    assert!(iges_cert.direct_manifold_payloads);
}

#[test]
fn malformed_step_envelope_is_refused() {
    assert_eq!(
        import_complex_step("OSV-CLOSE-TOPOLOGY-STEP/1\nBOGUS\nEND")
            .unwrap_err()
            .code,
        "BREP_COMPLEX_INTERCHANGE_REFUSED"
    );
}
