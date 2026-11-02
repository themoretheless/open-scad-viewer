//! Native, offline topology/closure regression. This candidate fixture is NOT
//! a qualified material solid and its planar far stops are NOT G2 transitions.
use brep_core::region_replacement as r;
#[path = "fixtures/trihedral_candidate.rs"]
mod saved;
use saved::candidate;
fn fixture() -> (brep_core::Model, r::TrihedralStrips, r::CornerCollars) {
    let m = brep_core::cuboid([0.; 3], [10.; 3]).unwrap();
    let requests = m
        .edges
        .iter()
        .enumerate()
        .filter(|(_, e)| e.vertices.contains(&0))
        .enumerate()
        .map(|(i, (edge, _))| r::EdgeRadius {
            edge,
            radius_mm: [0.1, 0.2, 0.35][i],
        })
        .collect::<Vec<_>>();
    let strips = r::build_trihedral_strips(
        &m,
        0,
        0,
        &requests,
        0.1,
        2.,
        r::StripLimits {
            tolerances: [1e-7, 1e-5, 1e-4],
            tolerance_uv: 1e-8,
            max_geometry_work: 10000,
            max_cells: 300000,
            max_exact: 1000000,
            max_coefficients: 2000000,
        },
    )
    .unwrap();
    let collars = r::prepare_trihedral_collars(
        &m,
        &strips,
        [1e-7, 1e-5, 1e-4],
        1e-8,
        0.001,
        100000,
        20000000,
        1000000,
    )
    .unwrap();
    (m, strips, collars)
}
#[test]
fn finite_strips_graft_with_owned_roots_and_continuous_closed_boundary() {
    let (m, strips, collars) = fixture();
    let before = m.clone();
    let graph = candidate(&collars);
    let out = r::propose_trihedral_graft(&m, &strips, &collars, &graph, 1e-8).unwrap();
    assert_eq!(
        m, before,
        "Graft proposals cannot mutate the source document"
    );
    assert_eq!(
        (
            out.model.vertices.len(),
            out.model.edges.len(),
            out.model.faces.len()
        ),
        (22, 33, 13)
    );
    assert_eq!(
        out.model.vertices.len() + out.model.faces.len(),
        out.model.edges.len() + 2
    );
    assert!(out.model.persistent_naming_complete());
    for &face in &out.modified_source_faces {
        let wire = &out.model.loops[out.model.faces[face].outer].coedges;
        for i in 0..wire.len() {
            assert_eq!(
                wire[i].pcurve.control_points.last(),
                wire[(i + 1) % wire.len()].pcurve.control_points.first(),
                "One source-owned vertex must have one exact face-local UV port"
            );
        }
    }
    for f in 0..m.faces.len() {
        if !out.modified_source_faces.contains(&f) {
            assert_eq!(
                out.model.1.faces[f], m.1.faces[f],
                "Unmodified source face identity changed"
            );
        }
    }
    for (i, edge) in m.edges.iter().enumerate() {
        if !edge.vertices.contains(&0) {
            let j = out
                .model
                .edges
                .iter()
                .position(|e| e.curve == edge.curve)
                .unwrap();
            assert_eq!(
                out.model.1.edges[j], m.1.edges[i],
                "Unmodified source edge identity changed"
            );
        }
    }
    let proof = brep_core::watertight::inspect_with_exact(&out.model, 100000, 1000000).unwrap();
    assert_eq!(
        proof.closure.status,
        brep_core::watertight::Closure::WithinTolerance
    );
    let trims = brep_core::face_domain::audit_trim_regions(&out.model, 1e-8, 10000, 100000, 100000)
        .unwrap();
    assert!(
        trims.all_valid,
        "All thirteen actual trimmed regions must be valid"
    );
    // A continuously closed boundary with valid trims is STILL not a material
    // solid certificate. This fixture deliberately retains a known excursion.
    let material = brep_core::face_domain::inspect_box_containment(
        &out.model,
        out.central_face,
        [[-m.tolerance_mm, 10. + m.tolerance_mm]; 3],
        1e-8,
        20000,
        300000,
    )
    .unwrap();
    assert_eq!(
        material.decision,
        brep_core::face_domain::BoxDecision::Outside
    );
    assert!(material.witness_uv.is_some());
    assert!(out.partition_cells > 0);
}
#[test]
fn changed_original_edge_identity_refuses_graft_without_mutating_source() {
    let (m, mut strips, collars) = fixture();
    let before = m.clone();
    let graph = candidate(&collars);
    strips.strips[0].source_identity =
        m.1.edges
            .iter()
            .copied()
            .find(|id| *id != strips.strips[0].source_identity)
            .unwrap();
    assert!(r::propose_trihedral_graft(&m, &strips, &collars, &graph, 1e-8).is_err());
    assert_eq!(m, before);
}
