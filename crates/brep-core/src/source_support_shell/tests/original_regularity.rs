use super::*;
use crate::source_volume::{self, regularity};

#[test]
fn material_body_owns_all_original_face_ranks_with_one_budget() {
    let model = crate::cuboid([0.; 3], [10., 20., 30.]).unwrap();
    let shell = prepare_projective(&model, limits()).unwrap().shell.unwrap();
    let geometry = crate::source_shell_geometry::qualify_projective(shell, geometry_limits())
        .unwrap()
        .geometry
        .unwrap();
    let body = source_volume::qualify(
        geometry,
        source_volume::Limits {
            axis: 0,
            origin: 0.,
            absolute_error: 0.01,
            tolerance_uv: 1e-8,
            cells: 10000,
            spans: 100000,
            domain_cells: 100000,
        },
    )
    .unwrap()
    .body
    .unwrap();
    let original = body.definition().unwrap();
    let report = regularity::qualify(&body, 100000).unwrap();
    let cells = report.cells;
    let proof = report.certificate.expect(report.reason);
    assert!(std::ptr::eq(proof.body(), &body));
    assert_eq!(proof.faces().len(), 6);
    assert!(proof.faces().iter().all(|f| f.spanwise_regular));
    assert_eq!(cells, proof.faces().iter().map(|f| f.cells).sum::<usize>());
    assert!(cells > 0);
    assert!(
        regularity::qualify(&body, cells)
            .unwrap()
            .certificate
            .is_some()
    );
    let short = regularity::qualify(&body, cells - 1).unwrap();
    assert!(short.certificate.is_none());
    assert!(short.cells <= cells - 1);
    assert!(short.uncertain_face.is_some());
    let zero = regularity::qualify(&body, 0).unwrap();
    assert!(zero.certificate.is_none());
    assert_eq!(zero.completed_faces, 0);
    assert_eq!(zero.cells, 0);
    assert!(regularity::qualify(&body, 100001).is_err());
    let smooth = regularity::smooth::qualify(&proof, 2, 100000, 1000000).unwrap();
    let (cells, work) = (smooth.cells, smooth.exact_work);
    let interiors = smooth
        .certificate
        .expect("Every unchanged original box face is C2");
    assert!(std::ptr::eq(interiors.regular(), &proof));
    assert_eq!(interiors.order(), 2);
    assert_eq!(interiors.faces().len(), 6);
    for (face, source) in interiors
        .faces()
        .iter()
        .zip(body.geometry().shell().faces())
    {
        assert!(std::ptr::eq(face.source(), source[0].edges()[0].surface()));
    }
    assert!(cells > 0 && work > 0);
    assert!(
        regularity::smooth::qualify(&proof, 2, cells, work)
            .unwrap()
            .certificate
            .is_some()
    );
    for (cells, work) in [(cells - 1, work), (cells, work - 1), (0, work), (cells, 0)] {
        let short = regularity::smooth::qualify(&proof, 2, cells, work).unwrap();
        assert!(short.certificate.is_none());
        assert!(short.cells <= cells && short.exact_work <= work);
    }
    assert_eq!(body.definition().unwrap(), original);
}
