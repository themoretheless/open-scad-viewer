#[path = "../examples/support/face_window_fixture.rs"]
mod fixture;
use brep_core::face_window;
#[test]
fn retained_face_hole_and_replacement_share_owned_edges_and_split_lineage() {
    let (source, face, patch, boundary) = fixture::build().unwrap();
    let before = source.clone();
    let out = face_window::replace(&source, face, &patch, &boundary, 1e-8, 1000000).unwrap();
    assert_eq!(source, before);
    assert_eq!(out.model.faces.len(), 7);
    assert_eq!(out.model.edges.len(), 16);
    assert_eq!(out.model.vertices.len(), 12);
    assert_eq!(out.model.faces[face].holes.len(), 1);
    assert_eq!(out.model.faces[face].surface, source.faces[face].surface);
    assert!(out.model.1.lineage.iter().any(|r| r.operation == "split"
        && r.entity_kind == "face"
        && r.parents == [out.source_face_identity]
        && r.children == out.face_identities));
    for i in 0..source.faces.len() {
        if i != face {
            assert_eq!(source.1.faces[i], out.model.1.faces[i]);
        }
    }
    for i in 0..source.edges.len() {
        assert_eq!(source.1.edges[i], out.model.1.edges[i]);
    }
    for edge in &out.edges {
        assert_eq!(
            out.model
                .loops
                .iter()
                .flat_map(|l| &l.coedges)
                .filter(|c| c.edge == *edge)
                .count(),
            2
        );
    }
    assert!(out.cells <= 1000000 && out.trim_pairs <= 10000 && out.exact_work <= 1000000);
    out.model.validate().unwrap();
    let exact = brep_core::boundary_agreement::verify_exact(&out.model, 1000000).unwrap();
    assert!(exact.all_equal && exact.all_joins_exact, "{exact:?}");
    let closure = brep_core::watertight::inspect_with_exact(&out.model, 100000, 1000000).unwrap();
    assert!(closure.exact.all_equal);
    assert_eq!(
        closure.closure.status,
        brep_core::watertight::Closure::WithinTolerance
    );
    assert!(
        (out.model.faces[out.replacement_face]
            .surface
            .evaluate(0.5, 0.5)
            .unwrap()
            .point[2]
            - 7.9)
            .abs()
            < 1e-12
    );
}
#[test]
fn invalid_window_lifts_overlap_and_budget_never_change_source() {
    let (source, face, patch, boundary) = fixture::build().unwrap();
    let before = source.clone();
    let mut malformed = boundary.clone();
    malformed[1].curve.control_points.clear();
    assert!(face_window::replace(&source, face, &patch, &malformed, 1e-8, 1000000).is_err());
    let mut wrong = boundary.clone();
    wrong[0].source_pcurve.control_points[3][0] += 0.01;
    assert!(face_window::replace(&source, face, &patch, &wrong, 1e-8, 1000000).is_err());
    let mut wrong_patch = patch.clone();
    wrong_patch.control_points[0][0][2] += 0.1;
    assert!(face_window::replace(&source, face, &wrong_patch, &boundary, 1e-8, 1000000).is_err());
    assert!(face_window::replace(&source, face, &patch, &boundary, 1e-8, 1).is_err());
    let first = face_window::replace(&source, face, &patch, &boundary, 1e-8, 1000000).unwrap();
    let saved = first.model.clone();
    assert!(face_window::replace(&first.model, face, &patch, &boundary, 1e-8, 1000000).is_err());
    assert_eq!(first.model, saved);
    assert_eq!(source, before);
}

#[test]
fn exact_aware_closure_does_not_trust_partial_identity_or_shell_flags() {
    let (source, face, patch, boundary) = fixture::build().unwrap();
    let mut out = face_window::replace(&source, face, &patch, &boundary, 1e-8, 1000000).unwrap();
    if let Ok(incomplete) = brep_core::watertight::inspect_with_exact(&out.model, 1, 1) {
        assert_ne!(
            incomplete.closure.status,
            brep_core::watertight::Closure::WithinTolerance
        );
    }
    // Keep topology and shell flags closed, but break one actual target lift.
    out.model.faces[out.replacement_face].surface.control_points[0][0][2] += 0.1;
    let bad = brep_core::watertight::inspect_with_exact(&out.model, 100000, 1000000).unwrap();
    assert!(!bad.exact.all_equal);
    assert_eq!(bad.closure.status, brep_core::watertight::Closure::Mismatch);
}

#[test]
fn explicit_zero_exact_budget_uses_interval_proofs_and_never_a_hidden_budget() {
    let (source,face,patch,boundary)=fixture::build().unwrap();let before=source.clone();
    let out=face_window::replace_with_exact_limit(&source,face,&patch,&boundary,1e-8,1000000,0).unwrap();
    assert_eq!(source,before);assert_eq!(out.exact_work,0);assert!(out.cells>0);
    assert!(face_window::replace_with_exact_limit(&source,face,&patch,&boundary,1e-8,1000000,1000001).is_err());
}
