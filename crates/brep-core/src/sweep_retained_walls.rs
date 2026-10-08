//! Actual wall union ownership and full rectangular chart correspondence.
//! This does not prove regularity, orientation or geometric embedding.
use crate::{Model,Result};
use nurbs_core::{curve::Curve,retained_wall_coefficients,retained_wall_domain_certificate::{self,Coedge}};
#[derive(Clone,Debug)]
pub struct Report {
    pub certified:bool,
    pub face_coverage_certified:bool,
    pub coefficient_family_certified:bool,
    pub inspected_faces:usize,
    pub exact_work:u64,
    pub reason:&'static str,
}
pub(crate) fn covers(model:&Model)->bool {
    if model.bodies.len()!=1 || model.shells.is_empty(){return false;}
    let body=&model.bodies[0];
    if body.inner_shells.len()+1!=model.shells.len(){return false;}
    let mut shells=vec![false;model.shells.len()];let mut faces=vec![false;model.faces.len()];
    for index in std::iter::once(body.outer_shell).chain(body.inner_shells.iter().copied()) {
        let Some(seen)=shells.get_mut(index) else{return false;};
        if *seen{return false;}*seen=true;
        let shell=&model.shells[index];if !shell.closed || shell.faces.is_empty(){return false;}
        for usage in &shell.faces {
            let Some(seen)=faces.get_mut(usage.face) else{return false;};
            if *seen{return false;}*seen=true;
        }
    }
    faces.into_iter().all(|seen|seen)
}
pub fn inspect(model:&Model,sections:&[Vec<Vec<Curve>>],closed:bool,max_faces:usize,max_exact_work:u64)->Result<Report>{
    model.validate()?;
    let mut out=Report {certified:false,face_coverage_certified:false,coefficient_family_certified:false,
        inspected_faces:0,exact_work:0,reason:"work-limit"};
    if max_faces==0 || max_faces>1024 || max_exact_work>1000000 || model.faces.len()>max_faces+if closed {0}else{2}{return Ok(out);}
    out.reason="retained-body-face-coverage-unproved";
    out.face_coverage_certified=covers(model);if !out.face_coverage_certified{return Ok(out);}
    out.reason="retained-wall-mismatch";
    let surfaces=model.faces.iter().map(|face|face.surface.clone()).collect::<Vec<_>>();
    out.coefficient_family_certified=retained_wall_coefficients::family_matches(&surfaces,sections,closed,max_faces);
    if !out.coefficient_family_certified{return Ok(out);}
    out.reason="retained-wall-domain-unproved";
    for face in model.faces.iter().take(model.faces.len()-if closed {0}else{2}) {
        let coedges=model.loops[face.outer].coedges.iter().map(|usage|Coedge {
            world:&model.edges[usage.edge].curve,uv:&usage.pcurve,reversed:usage.reversed,
        }).collect::<Vec<_>>();
        let domain=retained_wall_domain_certificate::inspect(&face.surface,&coedges,!face.holes.is_empty(),max_exact_work-out.exact_work)?;
        out.exact_work+=domain.work;
        if !domain.domain_certified{return Ok(out);}
        out.inspected_faces+=1;
    }
    out.certified=true;out.reason="exact-retained-wall-union";Ok(out)
}

/// Diagnostic counterpart of the exact original-section correspondence route.
/// Proof phases and their refusals are native; malformed topology cannot panic.
pub fn inspect_correspondence(model:&Model,sections:&[Vec<Vec<Curve>>],closed:bool,max_faces:usize,max_exact_work:u64)->Result<Report> {
    let mut out=Report {certified:false,face_coverage_certified:false,coefficient_family_certified:false,
        inspected_faces:0,exact_work:0,reason:"work-limit"};
    if max_faces==0 || max_faces>1024 || max_exact_work==0 || max_exact_work>1000000
        || model.faces.len()>max_faces+if closed {0}else{2} {return Ok(out);}
    out.reason="unsupported-section-decomposition";
    if !(2..=1025).contains(&sections.len()) {return Ok(out);}
    out.reason="retained-body-face-coverage-unproved";
    out.face_coverage_certified=covers(model);
    if !out.face_coverage_certified {return Ok(out);}
    out.reason="unsupported-section-decomposition";
    let mut rows_left=1000000;
    for section in sections {
        if section.is_empty() || section.len()>1024 {return Ok(out);}
        let mut curves=0;
        for ring in section {
            if ring.len()>1024 {return Ok(out);}
            for curve in ring {
                let Some(parts)=retained_wall_coefficients::segmented_bezier_controls(curve,rows_left) else {return Ok(out);};
                rows_left-=parts.iter().map(|c|c.control_points.len()).sum::<usize>();
                curves+=1;
            }
        }
        if curves==0 {return Ok(out);}
    }
    out.reason="retained-wall-mismatch";
    let surfaces=model.faces.iter().map(|face|face.surface.clone()).collect::<Vec<_>>();
    out.coefficient_family_certified=retained_wall_coefficients::family_matches(&surfaces,sections,closed,max_faces);
    if !out.coefficient_family_certified {return Ok(out);}
    out.reason="retained-wall-domain-unproved";
    let wall_faces=model.faces.len().checked_sub(if closed {0}else{2}).unwrap_or(0);
    for face in model.faces.iter().take(wall_faces) {
        let Some(wire)=model.loops.get(face.outer) else {return Ok(out);};
        if wire.coedges.len()!=4 || max_exact_work==out.exact_work {return Ok(out);}
        let mut coedges=Vec::new();
        for usage in &wire.coedges {
            let Some(edge)=model.edges.get(usage.edge) else {return Ok(out);};
            coedges.push(Coedge {world:&edge.curve,uv:&usage.pcurve,reversed:usage.reversed});
        }
        let Ok(domain)=retained_wall_domain_certificate::inspect(&face.surface,&coedges,!face.holes.is_empty(),max_exact_work-out.exact_work)
            else {return Ok(out);};
        if domain.work>max_exact_work-out.exact_work {return Ok(out);}
        out.exact_work+=domain.work;
        if !domain.domain_certified {return Ok(out);}
        out.inspected_faces+=1;
    }
    out.certified=true;out.reason="exact-retained-coefficients";Ok(out)
}

#[cfg(test)]
mod correspondence_tests {
    use super::*;
    #[test]
    fn original_sections_require_owned_domains_world_edges_and_shared_exact_work() {
        let sections=[0.,10.].iter().map(|&z|vec![vec![nurbs_core::primitives::circle(
            [0.,0.,z],[0.,0.,1.],0.5).unwrap()]]).collect::<Vec<_>>();
        let model=crate::rational_section_loft(&sections).unwrap();
        let r=inspect_correspondence(&model,&sections,false,1024,1000000).unwrap();
        assert!(r.certified && r.inspected_faces==4);
        assert_eq!(r.reason,"exact-retained-coefficients");
        let short=inspect_correspondence(&model,&sections,false,1024,r.exact_work-1).unwrap();
        assert!(!short.certified && short.exact_work<=r.exact_work-1);
        let mut changed=model.clone(); changed.shells[0].faces.pop();
        assert_eq!(inspect_correspondence(&changed,&sections,false,1024,1000000).unwrap().reason,"retained-body-face-coverage-unproved");
        let mut changed=model.clone();let wire=changed.faces[0].outer;
        let edge=changed.loops[wire].coedges[0].edge;
        changed.edges[edge].curve.control_points[0][2]+=0.125;
        assert_eq!(inspect_correspondence(&changed,&sections,false,1024,1000000).unwrap().reason,"retained-wall-domain-unproved");
        changed=model.clone();changed.faces[0].outer=changed.loops.len();
        assert!(!inspect_correspondence(&changed,&sections,false,1024,1000000).unwrap().certified);
        let mut source=sections.clone();source[0][0][0].weights[0]=0.5;
        assert_eq!(inspect_correspondence(&model,&source,false,1024,1000000).unwrap().reason,"retained-wall-mismatch");
    }
}
