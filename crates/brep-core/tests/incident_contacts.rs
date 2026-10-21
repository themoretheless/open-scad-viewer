use brep_core::{
    cuboid,
    face_contacts::{self, Limits},
};
#[path = "../examples/support/corner_close_fixture.rs"]
mod fixture;
#[test]
fn only_fresh_exact_vertex_proofs_promote_incident_pair_status() {
    let (source, uses, options) = fixture::fixture_extended();
    let out = brep_core::corner_close::close(
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
    let r = face_contacts::inspect_incident_with_vertices(
        &out.model,
        out.face,
        1e-8,
        limits(),
        1000000,
    )
    .unwrap();
    assert_eq!(
        r.pairs
            .iter()
            .filter(|p| p.reason == "shared-vertex")
            .map(|p| p.faces)
            .collect::<Vec<_>>(),
        vec![[5, 29], [16, 29]]
    );
    assert_eq!(
        r.pairs
            .iter()
            .filter(|p| p.reason == "pair-unresolved")
            .count(),
        6
    );
    assert!(r.cells <= 100000 && r.domain_cells <= 1000000 && r.vertex_exact_work <= 1000000);
    assert!(!r.all_incident_pairs_classified && !r.all_incident_pairs_disjoint);
    let zero =
        face_contacts::inspect_incident_with_vertices(&out.model, out.face, 1e-8, limits(), 0)
            .unwrap();
    assert_eq!(zero.vertex_exact_work, 0);
    assert_eq!(
        zero.pairs
            .iter()
            .filter(|p| p.reason == "pair-unresolved")
            .count(),
        8
    );
    let raw = face_contacts::inspect_incident(&out.model, out.face, 1e-8, limits()).unwrap();
    let mut tight = limits();
    tight.domain_cells = raw.domain_cells + 1;
    let bounded =
        face_contacts::inspect_incident_with_vertices(&out.model, out.face, 1e-8, tight, 1000000)
            .unwrap();
    assert!(bounded.domain_cells <= tight.domain_cells);
    assert_eq!(bounded.vertex_exact_work, 0);
    assert!(bounded.pairs.iter().all(|p| p.reason != "shared-vertex"));
}
fn limits() -> Limits {
    Limits {
        pairs: 10000,
        cells: 100000,
        domain_cells: 1000000,
        cells_per_pair: 1000,
        domain_cells_per_pair: 10000,
    }
}
#[test]
fn every_selected_pair_matches_full_audit_with_explicit_scope() {
    let m = cuboid([0.; 3], [2., 3., 4.]).unwrap();
    let full = face_contacts::inspect(&m, 1e-8, limits()).unwrap();
    for face in [0, 2, 5] {
        let r = face_contacts::inspect_incident(&m, face, 1e-8, limits()).unwrap();
        assert_eq!(r.face, face);
        assert_eq!(r.total_pairs, 5);
        assert_eq!(r.pairs.len(), 5);
        assert!(r.next_pair.is_none());
        let expected = full
            .pairs
            .iter()
            .filter(|p| p.faces.contains(&face))
            .collect::<Vec<_>>();
        assert_eq!(
            r.all_incident_pairs_classified,
            expected.iter().all(|p| p.boundary.is_some()
                || p.hull_disjoint
                || p.result.as_ref().is_some_and(|s| s.absence_proven))
        );
        assert_eq!(
            r.all_incident_pairs_disjoint,
            expected
                .iter()
                .all(|p| p.hull_disjoint || p.result.as_ref().is_some_and(|s| s.absence_proven))
        );
        for (actual, expected) in r.pairs.iter().zip(expected) {
            assert_eq!(actual.faces, expected.faces);
            assert_eq!(actual.reason, expected.reason);
            assert_eq!(actual.hull_cells, expected.hull_cells);
            assert_eq!(actual.boundary_cells, expected.boundary_cells);
        }
        assert!(r.cells <= 100000 && r.domain_cells <= 1000000);
    }
}
#[test]
fn exhausted_pair_budget_retains_first_unvisited_incident_pair() {
    let m = cuboid([0.; 3], [2., 3., 4.]).unwrap();
    let mut budget = limits();
    budget.pairs = 1;
    let r = face_contacts::inspect_incident(&m, 2, 1e-8, budget).unwrap();
    assert_eq!(r.pairs.len(), 1);
    assert_eq!(r.pairs[0].faces, [0, 2]);
    assert_eq!(r.next_pair, Some([1, 2]));
    assert_eq!(r.total_pairs, 5);
    assert!(!r.all_incident_pairs_classified && !r.all_incident_pairs_disjoint);
    assert!(face_contacts::inspect_incident(&m, 6, 1e-8, limits()).is_err());
}
