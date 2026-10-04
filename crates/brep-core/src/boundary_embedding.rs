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
    pub cap_contacts: Vec<(usize,crate::sweep_cap_contacts::Report)>,
}
/// Every component retains its unresolved entries. Budgets are explicit per
/// stage; callers must not interpret one successful stage as the whole result.
pub fn inspect(model: &Model, tolerance_uv: f64, limits: Limits) -> Result<Report> {
    inspect_with_linear(model, tolerance_uv, limits, 0)
}
/// Optional independent shared whole-chart oblique injectivity cell budget.
pub fn inspect_with_linear(model: &Model, tolerance_uv: f64, limits: Limits,
    max_linear_cells: usize) -> Result<Report> {
    inspect_impl(model,tolerance_uv,limits,max_linear_cells,None)
}
/// Explicit cap selection; per-cap budgets are independent of joint audit budgets.
pub fn inspect_sweep(model: &Model, tolerance_uv: f64, limits: Limits,
    max_linear_cells: usize, caps: &[usize], cap_limits: crate::sweep_cap_contacts::Budgets) -> Result<Report> {
    inspect_impl(model,tolerance_uv,limits,max_linear_cells,Some((caps,cap_limits)))
}
fn inspect_impl(model: &Model, tolerance_uv: f64, limits: Limits,
    max_linear_cells: usize, caps: Option<(&[usize],crate::sweep_cap_contacts::Budgets)>) -> Result<Report> {
    // Includes connected shells, vertex fans, opposite edge uses and ownership.
    model.validate()?;
    if model.shells.is_empty() || model.shells.iter().any(|s| !s.closed || s.faces.is_empty()) {
        return Err(Error::new("BREP_INVALID_INPUT", "Embedding audit requires nonempty closed shells"));
    }
    let agreement = boundary_agreement::verify_exact(model, limits.exact_work)?;
    let trim = face_domain::audit_trim_regions(model, tolerance_uv,
        limits.trim_pairs, limits.trim_cells, limits.trim_domain_cells)?;
    let faces = crate::face_injectivity::inspect_with_linear(model, limits.spans, max_linear_cells)?;
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
    let mut cap_contacts=Vec::new();
    let mut allowed_caps=Vec::new();
    if let Some((caps,budget))=caps {
        if caps.is_empty() || caps.len()>16 {
            return Err(Error::new("BREP_SWEEP_CAP_CONTACT_INVALID","Select 1..16 cap faces"));
        }
        // Recompute even when another stage is unresolved so diagnostics remain
        // observable, but never authorize a pair without joint prerequisites.
        for &cap in caps {
            let report=crate::sweep_cap_contacts::inspect(model,cap,caps,budget)?;
            if exact_domain && report.cap_certified {
                for &[wall,edge] in &report.allowed_boundaries {
                    allowed_caps.push([cap,wall,edge]);
                }
            }
            cap_contacts.push((cap,report));
        }
    }
    let pairs = face_contacts::inspect_with_certificates(model, tolerance_uv, limits.contacts, &hull_contacts,&allowed_caps)?;
    let intersections = self_intersection::Report { absence_proven: faces.all_faces_injective && pairs.all_pairs_classified, faces, pairs };
    let proven = agreement.all_equal && agreement.all_joins_exact && trim.all_valid
        && winding && intersections.absence_proven;
    Ok(Report { proven, hull_contacts, agreement, trim, intersections, cap_contacts })
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
    fn exact_quadrant_sweep_admits_contacts_only_after_joint_prerequisites() {
        let section=|z|vec![[
            [[0.5,0.],[0.5,0.5],[0.,0.5]],
            [[0.,0.5],[-0.5,0.5],[-0.5,0.]],
            [[-0.5,0.],[-0.5,-0.5],[0.,-0.5]],
            [[0.,-0.5],[0.5,-0.5],[0.5,0.]],
        ].into_iter().map(|p|nurbs_core::curve::Curve {
            degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
            control_points:p.into_iter().map(|xy|vec![xy[0],xy[1],z]).collect(),
            weights:vec![1.,std::f64::consts::FRAC_1_SQRT_2,1.],periodic:false,
        }).collect()];
        let model=crate::rational_section_loft(&[section(0.),section(5.)]).unwrap();
        let mut l=limits();
        l.contacts.pairs=1;
        let report=inspect_with_linear(&model,1e-8,l,10000).unwrap();
        assert!(report.agreement.all_equal && report.agreement.all_joins_exact);
        assert!(report.trim.all_valid);
        assert!(report.intersections.faces.all_faces_injective);
        assert!(!report.hull_contacts.is_empty());
        assert!(report.intersections.pairs.pairs[0].boundary.is_some());
        assert!(report.intersections.pairs.next_pair.is_some());
        assert!(!report.proven, "one classified pair cannot certify the body");
        let full=inspect_with_linear(&model,1e-8,limits(),10000).unwrap();
        assert!(full.intersections.pairs.next_pair.is_none());
        assert_eq!(full.intersections.pairs.pairs.len(),15);
        assert!(!full.proven);
        assert!(full.intersections.pairs.pairs.iter()
            .filter(|p|p.reason=="pair-unresolved")
            .all(|p|p.faces[0]<4 && p.faces[1]>=4));
        for pair in &full.intersections.pairs.pairs {
            println!("retained pair {:?}: {} boundary={}",pair.faces,pair.reason,pair.boundary.is_some());
        }
        println!("retained embedded boundary proven={}",full.proven);
        let cap_budget=crate::sweep_cap_contacts::Budgets {
            max_walls:1024,max_exact_work:1000000,max_chart_cells:1000,
            max_trim_pairs:100000,max_trim_cells:100000,max_trim_domain_cells:1000000,
        };
        let joined=inspect_sweep(&model,1e-8,limits(),10000,&[4,5],cap_budget).unwrap();
        assert!(joined.proven,"joint cap contacts must complete this exact fixture");
        assert_eq!(joined.cap_contacts.len(),2);
        assert_eq!(joined.intersections.pairs.pairs.iter().filter(|p|
            matches!(p.boundary,Some(face_contacts::SharedBoundary::SweepCap{..}))).count(),8);
        let short=inspect_sweep(&model,1e-8,limits(),10000,&[4,5],
            crate::sweep_cap_contacts::Budgets{max_exact_work:0,..cap_budget}).unwrap();
        assert!(!short.proven);
        assert!(short.cap_contacts.iter().all(|(_,r)|!r.cap_certified));
        assert!(!short.intersections.pairs.pairs.iter().any(|p|
            matches!(p.boundary,Some(face_contacts::SharedBoundary::SweepCap{..}))));
    }
    #[test]
    fn oblique_chart_proof_does_not_bypass_exact_boundary_prerequisites() {
        let section=|z|vec![vec![nurbs_core::primitives::circle(
            [0.,0.,z],[0.,0.,1.],0.5).unwrap()]];
        let model=crate::rational_section_loft(&[section(0.),section(5.)]).unwrap();
        let mut l=limits();
        l.exact_work=1;
        l.contacts.pairs=1;
        let report=inspect_with_linear(&model,1e-8,l,10000).unwrap();
        assert!(report.intersections.faces.all_faces_injective);
        assert!(report.intersections.faces.linear_cells>0);
        assert!(!report.agreement.all_equal);
        assert!(report.hull_contacts.is_empty());
        assert!(!report.proven);
        assert!(report.intersections.pairs.next_pair.is_some());
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
