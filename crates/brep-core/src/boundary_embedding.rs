//! Joint evidence for a closed embedded boundary. Material roles, nesting and
//! outward orientation are deliberately separate requirements for solid volume.
use crate::{Error, Model, Result, boundary_agreement, face_contacts, face_domain, self_intersection};

#[derive(Clone, Copy)]
pub struct Limits {
    pub exact_work: u64,
    pub trim_pairs: usize,
    pub trim_cells: usize,
    pub trim_domain_cells: usize,
    pub spans: usize,
    pub contacts: face_contacts::Limits,
}
pub struct Report {
    pub proven: bool,
    pub hull_contacts: Vec<crate::boundary_hull_contact::Certificate>,
    pub agreement: boundary_agreement::ExactReport,
    pub trim: face_domain::TrimAudit,
    pub intersections: self_intersection::Report,
}
/// Every component retains its unresolved entries. Budgets are explicit per
/// stage; callers must not interpret one successful stage as the whole result.
pub fn inspect(model: &Model, tolerance_uv: f64, limits: Limits) -> Result<Report> {
    // Includes connected shells, vertex fans, opposite edge uses and ownership.
    model.validate()?;
    if model.shells.is_empty() || model.shells.iter().any(|s| !s.closed || s.faces.is_empty()) {
        return Err(Error::new("BREP_INVALID_INPUT", "Embedding audit requires nonempty closed shells"));
    }
    let agreement = boundary_agreement::verify_exact(model, limits.exact_work)?;
    let trim = face_domain::audit_trim_regions(model, tolerance_uv,
        limits.trim_pairs, limits.trim_cells, limits.trim_domain_cells)?;
    let faces = crate::face_injectivity::inspect(model, limits.spans)?;
    let winding = trim.faces.iter().all(|r| r.as_ref().is_some_and(|r| r.winding.first() == Some(&Some(1))));
    let mut hull_contacts = Vec::new();
    let exact_domain = agreement.all_equal && agreement.all_joins_exact && trim.all_valid
        && winding && faces.all_faces_injective;
    if exact_domain {
        // Bound certificate enumeration by the same lexicographic pair prefix.
        let mut visited=0;
        'pairs: for a in 0..model.faces.len() { for b in a+1..model.faces.len() {
            if visited==limits.contacts.pairs { break 'pairs; }
            visited+=1;
            if let Some(c)=crate::boundary_hull_contact::certify(model,[a,b]) { hull_contacts.push(c); }
        }}
    }
    let pairs = face_contacts::inspect_with_hulls(model, tolerance_uv, limits.contacts, &hull_contacts)?;
    let intersections = self_intersection::Report { absence_proven: faces.all_faces_injective && pairs.all_pairs_classified, faces, pairs };
    let proven = agreement.all_equal && agreement.all_joins_exact && trim.all_valid
        && winding && intersections.absence_proven;
    Ok(Report { proven, hull_contacts, agreement, trim, intersections })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn limits() -> Limits {
        Limits { exact_work: 1000000, trim_pairs: 1000, trim_cells: 10000,
            trim_domain_cells: 100000, spans: 100,
            contacts: face_contacts::Limits { pairs: 100, cells: 10000,
                domain_cells: 100000, cells_per_pair: 16, domain_cells_per_pair: 1000 } }
    }
    #[test]
    fn exact_sphere_boundary_embedding_covers_poles_equator_edges_and_vertices(){
        let model=crate::analytic::sphere(3.).unwrap();let before=format!("{model:?}");
        let mut l=limits();l.spans=1000;
        let r=inspect(&model,1e-8,l).unwrap();
        assert!(r.agreement.all_equal&&r.agreement.all_joins_exact&&r.trim.all_valid);
        assert!(r.intersections.faces.all_faces_injective);
        for pair in [[0,2],[1,3],[4,6],[5,7]]{
            assert!(r.hull_contacts.iter().any(|c|c.faces==pair&&c.vertex.is_some()));
        }
        assert!(r.proven);assert!(r.intersections.absence_proven);
        assert!(r.intersections.pairs.all_pairs_classified);
        assert_eq!(r.hull_contacts.iter().filter(|c|c.vertex.is_some()).count(),12);
        let rounded=crate::analytic::sphere(2.).unwrap();let r=inspect(&rounded,1e-8,l).unwrap();
        assert!(!r.proven);assert!(!r.agreement.all_equal);assert!(r.hull_contacts.is_empty());
        assert_eq!(format!("{model:?}"),before);
    }
    #[test]
    fn cube_requires_every_stage_and_preserves_input() {
        let m = crate::cuboid([0.;3], [1.;3]).unwrap();
        let before = format!("{m:?}");
        assert!(inspect(&m,1e-8,limits()).unwrap().proven);
        for l in [Limits { exact_work: 1, ..limits() },
            Limits { trim_pairs: 1, ..limits() }, Limits { trim_domain_cells: 1, ..limits() },
            Limits { spans: 1, ..limits() },
            Limits { contacts: face_contacts::Limits { pairs: 1, ..limits().contacts }, ..limits() }] {
            let r = inspect(&m,1e-8,l).unwrap();
            assert!(!r.proven);
            assert_eq!(r.agreement.uses.len(),24);
            assert_eq!(r.trim.faces.len(),6);
            assert_eq!(r.intersections.faces.faces.len(),6);
        }
        assert_eq!(format!("{m:?}"),before);
    }
    #[test]
    fn tolerance_accepted_gap_is_not_an_exact_embedded_boundary() {
        let mut m = crate::cuboid([0.;3], [1.;3]).unwrap();
        m.edges[0].curve.control_points[0][2] += 1e-12;
        m.validate().unwrap();
        let r = inspect(&m,1e-8,limits()).unwrap();
        assert!(!r.agreement.all_equal);
        assert!(!r.proven);
    }
}
