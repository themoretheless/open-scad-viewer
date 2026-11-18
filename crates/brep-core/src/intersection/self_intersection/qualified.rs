//! Fresh layered boundary diagnostics with explicit exact/locality separation.
use super::*;
#[derive(Clone, Debug)]
pub struct LocalizedPair {
    pub faces: [usize; 2],
    pub owned_edges: Vec<usize>,
    pub owned_vertex: Option<usize>,
    pub distance_upper_mm: f64,
}
pub struct QualifiedReport {
    /// Initial inspectors remain unchanged; additional proofs have their own
    /// collections and the combined ledger below.
    pub faces: face_injectivity::Report,
    pub pairs: face_contacts::Report,
    pub exact_vertices: Vec<crate::trimmed_vertex_contact::Certificate>,
    pub localized: Vec<LocalizedPair>,
    pub unresolved: Vec<[usize; 2]>,
    pub interior_contacts: Vec<[usize; 2]>,
    pub absence_proven: bool,
    /// Absence, owned contacts or bounded contact tubes cover all pairs. This
    /// never upgrades absence_proven or global solid embedding.
    pub tolerance_contact_locality_proven: bool,
    pub cells: usize,
    pub domain_cells: usize,
    /// Exact work in the additional vertex/tube checks. Initial pair work
    /// remains attributed to its own geometry/domain ledger.
    pub fallback_exact_work: u64,
}
fn classified(p: &face_contacts::Pair) -> bool {
    p.boundary.is_some() || p.hull_disjoint || p.result.as_ref().is_some_and(|r| r.absence_proven)
}
pub fn inspect_qualified(
    model: &Model,
    tolerance_uv: f64,
    max_spans: usize,
    limits: face_contacts::Limits,
    max_exact: u64,
) -> Result<QualifiedReport> {
    if max_exact > 1000000
        || !(1..=100000).contains(&max_spans)
        || limits.cells > 100000
        || limits.domain_cells > 1000000
    {
        return Err(nurbs_core::Error::new(
            "BREP_SELF_INTERSECTION_LIMITS",
            "Boundary diagnostic work exceeds its bounded allowance",
        ));
    }
    let span_allowance = max_spans.min(limits.cells);
    let faces = face_injectivity::inspect_with_linear(
        model,
        span_allowance,
        limits.cells - span_allowance,
    )?;
    let remaining = limits.cells - faces.spans - faces.linear_cells;
    let pairs = if remaining == 0 || limits.domain_cells == 0 {
        let total = model
            .faces
            .len()
            .saturating_mul(model.faces.len().saturating_sub(1))
            / 2;
        face_contacts::Report {
            pairs: vec![],
            disjoint_groups: vec![],
            grouped_pairs: 0,
            group_cells: 0,
            total_pairs: total,
            next_pair: if total > 0 { Some([0, 1]) } else { None },
            cells: 0,
            domain_cells: 0,
            all_pairs_disjoint: total == 0,
            all_pairs_classified: total == 0,
        }
    } else {
        face_contacts::inspect(
            model,
            tolerance_uv,
            face_contacts::Limits {
                cells: limits.cells - faces.spans - faces.linear_cells,
                ..limits
            },
        )?
    };
    let mut cells = faces.spans + faces.linear_cells + pairs.cells;
    let mut domain_cells = pairs.domain_cells;
    let mut exact = 0;
    let mut exact_vertices = Vec::new();
    let mut localized = Vec::new();
    let mut unresolved = Vec::new();
    let mut interior_contacts = Vec::new();
    for p in &pairs.pairs {
        if !classified(p) && p.reason != "pair-unresolved" {
            if p.reason == "interior-contact" {
                interior_contacts.push(p.faces);
            } else {
                unresolved.push(p.faces);
            }
        }
    }
    let pending = pairs
        .pairs
        .iter()
        .filter(|p| p.reason == "pair-unresolved")
        .count();
    let mut visited = 0;
    for pair in &pairs.pairs {
        if pair.reason != "pair-unresolved" {
            continue;
        }
        let count = pending - visited;
        visited += 1;
        let allowance =
            ((limits.cells - cells) / count).min((limits.domain_cells - domain_cells) / count);
        let exact_allowance = (max_exact - exact) / count as u64;
        let vertex =
            crate::trimmed_vertex_contact::inspect(model, pair.faces, allowance, exact_allowance)?;
        cells += vertex.cells;
        domain_cells += vertex.cells;
        exact += vertex.exact_work;
        if let Some(c) = vertex.certificate {
            if c.exact_vertex_contact {
                exact_vertices.push(c);
                continue;
            }
            localized.push(LocalizedPair {
                faces: pair.faces,
                owned_edges: vec![],
                owned_vertex: Some(c.vertex),
                distance_upper_mm: c.distance_upper,
            });
            continue;
        }
        let allowance =
            ((limits.cells - cells) / count).min((limits.domain_cells - domain_cells) / count);
        let exact_allowance = (max_exact - exact) / count as u64;
        let tube = crate::boundary_contact_tube::inspect_pair(
            model,
            pair.faces,
            allowance,
            exact_allowance,
        )?;
        cells += tube.cells;
        domain_cells += tube.cells;
        exact += tube.exact_work;
        if let Some(c) = tube.certificate {
            localized.push(LocalizedPair {
                faces: pair.faces,
                owned_edges: c.edges,
                owned_vertex: None,
                distance_upper_mm: c.boundary_error_upper,
            });
        } else {
            unresolved.push(pair.faces);
        }
    }
    let all_exact = pairs.next_pair.is_none()
        && pairs
            .pairs
            .iter()
            .all(|p| classified(p) || exact_vertices.iter().any(|c| c.faces == p.faces));
    let absence_proven = faces.all_faces_injective && all_exact;
    let tolerance_contact_locality_proven = faces.all_faces_injective
        && pairs.next_pair.is_none()
        && unresolved.is_empty()
        && interior_contacts.is_empty();
    Ok(QualifiedReport {
        faces,
        pairs,
        exact_vertices,
        localized,
        unresolved,
        interior_contacts,
        absence_proven,
        tolerance_contact_locality_proven,
        cells,
        domain_cells,
        fallback_exact_work: exact,
    })
}
