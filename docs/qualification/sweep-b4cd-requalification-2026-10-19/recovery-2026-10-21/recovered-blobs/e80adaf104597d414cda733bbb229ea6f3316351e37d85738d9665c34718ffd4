#[path = "../examples/support/corner_close_fixture.rs"]
mod fixture;
use brep_core::corner_close;
use fixture::fixture;
#[test]
fn interior_trim_rim_closes_without_replacing_original_supports() {
    let (source, uses, options) = fixture::fixture_extended();
    let saved = source.clone();
    let out = corner_close::close(
        &source,
        0,
        uses,
        options,
        1e-8,
        [1e-7, 1e-5, 1e-4],
        100000,
        1000000,
    )
    .unwrap();
    assert!(
        out.corner.admitted
            && out.corner.interior.certified
            && out.corner.complete
            && out.corner.within_tolerance
    );
    assert_eq!(out.model.faces.len(), 30);
    assert!(out.model.shells[0].closed);
    assert_eq!(out.model.vertices, source.vertices);
    assert_eq!(out.model.edges, source.edges);
    for i in 0..source.faces.len() {
        assert_eq!(out.model.faces[i], source.faces[i]);
        assert_eq!(out.model.1.faces[i], source.1.faces[i]);
    }
    assert_eq!(out.model.1.edges, source.1.edges);
    assert_eq!(out.model.1.vertices, source.1.vertices);
    assert!(
        out.cells <= 100000 && out.exact_work <= 1000000 && out.corner.coefficient_work <= 2000000
    );
    assert_eq!(
        out.closure.closure.status,
        brep_core::watertight::Closure::WithinTolerance
    );
    assert_eq!(source, saved);
}
#[test]
fn g2_corner_closes_owned_hole_without_replacing_source_edges() {
    let (source, uses, options) = fixture();
    let before = source.clone();
    let out = corner_close::close(
        &source,
        0,
        uses,
        options,
        1e-8,
        [1e-7, 1e-5, 1e-4],
        100000,
        1000000,
    )
    .unwrap();
    assert_eq!(source, before);
    assert!(out.corner.complete && out.corner.within_tolerance);
    assert_eq!(out.model.faces.len(), 30);
    assert_eq!(out.model.vertices, source.vertices);
    assert_eq!(out.model.edges, source.edges);
    assert_eq!(out.model.1.vertices, source.1.vertices);
    assert_eq!(out.model.1.edges, source.1.edges);
    assert_eq!(&out.model.1.faces[..source.faces.len()], source.1.faces);
    assert_eq!(out.model.bodies.len(), 1);
    assert!(out.model.shells[0].closed);
    assert_eq!(
        out.closure.closure.status,
        brep_core::watertight::Closure::WithinTolerance
    );
    assert!(out.cells <= 100000 && out.exact_work <= 1000000);
    assert!(out.model.persistent_naming_complete());
    let mut wrong = uses;
    wrong[0].edge = uses[1].edge;
    assert!(
        corner_close::close(
            &source,
            0,
            wrong,
            options,
            1e-8,
            [1e-7, 1e-5, 1e-4],
            100000,
            1000000
        )
        .is_err()
    );
    assert!(
        corner_close::close(
            &source,
            0,
            uses,
            options,
            1e-8,
            [1e-7, 1e-5, 1e-4],
            1,
            1000000
        )
        .is_err()
    );
    assert_eq!(source, before);
    let mut unsupported = source.clone();
    let wire = unsupported.faces[uses[0].face].outer;
    let c = unsupported.loops[wire]
        .coedges
        .iter_mut()
        .find(|c| c.edge == uses[0].edge)
        .unwrap();
    c.pcurve = c.pcurve.elevate(2).unwrap();
    unsupported.rebuild_topology_ids();
    unsupported.validate().unwrap();
    let saved = unsupported.clone();
    assert!(
        corner_close::close(
            &unsupported,
            0,
            uses,
            options,
            1e-8,
            [1e-7, 1e-5, 1e-4],
            100000,
            1000000
        )
        .is_err()
    );
    assert_eq!(unsupported, saved);
}
