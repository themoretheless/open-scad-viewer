//! Exact retained endpoint material regions. No source transport error or
//! complete boundary/global embedding claim is made by this audit.
use crate::{Model,Result,sweep_cap_contacts::{self,Budgets}};
use nurbs_core::{curve::Curve,retained_wall_coefficients};

#[derive(Clone,Debug)]
pub struct Report {
    pub exact:bool,
    pub exact_work:u64,
    pub inspected_edges:usize,
    pub reason:&'static str,
}
/// Bind both actual cap regions to segmented endpoint contours. Exact contour
/// identity and native planar chart/trim audits are necessary for every ring.
pub fn inspect(model:&Model,endpoints:&[Vec<Vec<Curve>>;2],budgets:Budgets,max_edges:usize)->Result<Report>{
    model.validate()?;
    let mut out=Report {exact:false,exact_work:0,inspected_edges:0,reason:"work-limit"};
    if max_edges==0 || max_edges>1024 || budgets.max_exact_work==0 || budgets.max_exact_work>1000000 {return Ok(out);}
    out.reason="cap-contour-mismatch";
    if model.faces.len()<2 {return Ok(out);}
    let faces=[model.faces.len()-2,model.faces.len()-1];
    for endpoint in 0..2 {
        let face=&model.faces[faces[endpoint]];
        let wires=std::iter::once(face.outer).chain(face.holes.iter().copied()).collect::<Vec<_>>();
        if wires.len()!=endpoints[endpoint].len() || wires.is_empty() {return Ok(out);}
        for (wire,ring) in wires.iter().zip(&endpoints[endpoint]) {
            let mut expected=Vec::new();
            for curve in ring {
                let remaining=(budgets.max_exact_work-out.exact_work) as usize;
                if remaining==0 {out.reason="work-limit";return Ok(out);}
                let Some(parts)=retained_wall_coefficients::segmented_bezier_controls(curve,remaining) else {
                    out.reason="unsupported-section-decomposition";return Ok(out);
                };
                // Preparation and comparison share one budget across both caps
                // and every ring. Do not allocate a fresh allowance per curve.
                out.exact_work+=parts.iter().map(|part|part.control_points.len() as u64).sum::<u64>();
                expected.extend(parts);
            }
            let uses=&model.loops[*wire].coedges;
            if expected.len()!=uses.len() || uses.is_empty(){return Ok(out);}
            if uses.len()>max_edges-out.inspected_edges {out.reason="work-limit";return Ok(out);}
            out.inspected_edges+=uses.len();
            let actual=uses.iter().map(|u|model.edges[u.edge].curve.clone()).collect::<Vec<_>>();
            let reversed=uses.iter().map(|u|u.reversed).collect::<Vec<_>>();
            let (exact,work)=retained_wall_coefficients::contour_matches(&expected,&actual,&reversed,(budgets.max_exact_work-out.exact_work) as usize);
            out.exact_work+=work as u64;
            if !exact {out.reason=if out.exact_work==budgets.max_exact_work {"work-limit"}else{"cap-contour-mismatch"};return Ok(out);}
        }
        let audit=sweep_cap_contacts::inspect(model,faces[endpoint],&faces,Budgets {max_exact_work:budgets.max_exact_work-out.exact_work,..budgets})?;
        out.exact_work+=audit.exact_work;
        if !audit.cap_certified || !audit.planar_control_hull_certified {out.reason="cap-region-unproved";return Ok(out);}
    }
    out.exact=true;out.reason="exact-planar-regions";Ok(out)
}
