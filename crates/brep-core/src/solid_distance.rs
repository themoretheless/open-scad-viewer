//! Distance of filled volumes after independent geometry validation. Shell
//! distance alone is insufficient: a boundary may lie inside the other solid.
use crate::{Error, Model, Result, shell_relation, solid_audit, volume_validity};
#[derive(Clone, Copy)]
pub struct Limits {
    /// Applied independently to each input model.
    pub validity: volume_validity::Limits,
    /// Shared by every cross-model shell pair, including parity queries.
    pub pairs: usize,
    pub contact_pairs: usize,
    pub cells: usize,
    pub domain_cells: usize,
}
pub struct Pair {
    pub shells: [usize;2],
    pub relation: shell_relation::Report,
}
pub struct Report {
    pub validity: [volume_validity::Report;2],
    pub pairs: Vec<Pair>,
    pub total_pairs: usize,
    pub cells: usize,
    pub domain_cells: usize,
    /// None unless both inputs are certified volumes. Unresolved containment
    /// retains zero as the lower bound even with a positive boundary distance.
    pub distance_interval_mm: Option<[f64;2]>,
    /// Intersection of closed material sets; does not imply positive overlap volume.
    pub material_overlap: Option<bool>,
    pub contact: Option<([usize;2], nurbs_core::surface_contact::Witness)>,
    pub contact_pairs_visited: usize,
    pub converged: bool,
    pub reason: &'static str,
}
/// An admitted pair on the original authored faces that supplies the reported
/// separation upper bound. Containment/contact do not reuse boundary witnesses.
pub struct SeparationWitness<'a> {
    pub faces: [usize;2],
    pub geometry: &'a nurbs_core::trimmed_surface_distance::TrimmedDistance,
}
impl Report {
    pub fn separation_witness<'a>(&'a self, a:&Model, b:&Model)->Option<SeparationWitness<'a>> {
        if self.material_overlap!=Some(false){return None;}
        let upper=self.distance_interval_mm?[1];
        let pair=self.pairs.iter().find(|p|p.relation.boundary.upper_bound_mm==Some(upper))?;
        let local=pair.relation.boundary.faces?;
        let mut faces=[0;2];
        for (side,model) in [a,b].into_iter().enumerate(){
            let shell=model.shells.get(pair.shells[side])?;
            // isolated_outward_shell numbers faces by sorted original index.
            let ids=shell.faces.iter().map(|f|f.face).collect::<std::collections::BTreeSet<_>>();
            faces[side]=*ids.iter().nth(local[side])?;
        }
        Some(SeparationWitness{faces,geometry:pair.relation.boundary.witness.as_ref()?})
    }
}
pub fn distance(a:&Model,b:&Model,tolerance_mm:f64,tolerance_uv:f64,limits:Limits)->Result<Report>{
    if !tolerance_mm.is_finite() || tolerance_mm<=0. || !(1..=100000).contains(&limits.pairs)
        || !(1..=100000).contains(&limits.contact_pairs) || !(2..=1000000).contains(&limits.cells) || !(2..=8000000).contains(&limits.domain_cells) {
        return Err(Error::new("BREP_INVALID_INPUT","Solid distance requires positive tolerance and bounded pair/work budgets"));
    }
    let validity=[volume_validity::inspect(a,tolerance_uv,limits.validity)?,volume_validity::inspect(b,tolerance_uv,limits.validity)?];
    let total_pairs=a.shells.len().checked_mul(b.shells.len()).ok_or_else(||Error::new("BREP_RESOURCE_LIMIT","Too many shell pairs"))?;
    if total_pairs>100000{return Err(Error::new("BREP_RESOURCE_LIMIT","Solid distance requires at most 100000 shell pairs"));}
    let mut out=Report{validity,pairs:Vec::new(),total_pairs,cells:0,domain_cells:0,
        distance_interval_mm:None,material_overlap:None,contact:None,contact_pairs_visited:0,converged:false,reason:"volume-validity-unproven"};
    if out.validity.iter().any(|r|!r.proven){return Ok(out);}
    let shells=[a,b].map(|m|(0..m.shells.len()).map(|i|solid_audit::isolated_outward_shell(m,i,false)).collect::<Result<Vec<_>>>());
    let [sa,sb]=shells;let sa=sa?;let sb=sb?;
    let mut lower=f64::INFINITY;
    let mut upper=f64::INFINITY;
    out.reason="shell-pairs-incomplete";
    'pairs:for (i,a) in sa.iter().enumerate(){for (j,b) in sb.iter().enumerate(){
        if out.pairs.len()==limits.pairs || limits.cells-out.cells<2 || limits.domain_cells-out.domain_cells<2 {break 'pairs;}
        let remaining=total_pairs-out.pairs.len();
        let relation=shell_relation::inspect(a,b,tolerance_mm,tolerance_uv,
            ((limits.cells-out.cells)/(remaining*2)).max(2),((limits.domain_cells-out.domain_cells)/(remaining*2)).max(2))?;
        out.cells+=relation.boundary.cells+relation.witness_parity.iter().flatten().map(|r|r.cells).sum::<usize>();
        out.domain_cells+=relation.boundary.domain_cells+relation.witness_parity.iter().flatten().map(|r|r.domain_cells).sum::<usize>();
        lower=lower.min(relation.boundary.lower_bound_mm);
        if let Some(u)=relation.boundary.upper_bound_mm{upper=upper.min(u);}
        out.pairs.push(Pair{shells:[i,j],relation});
    }}
    // Points on either certified closed boundary belong to its closed solid,
    // including cavity boundaries. Any such pair is a valid distance upper bound.
    if upper.is_finite(){out.distance_interval_mm=Some([0.,upper]);}
    // A single admitted contact proves zero distance even if other pairs are
    // unvisited. Reserve half the work above so unresolved separation can use
    // this existence test without exceeding the same global budget.
    if lower<=0. && out.cells<limits.cells && out.domain_cells<limits.domain_cells {
        let domains=[a,b].map(|m|(0..m.faces.len()).map(|i|crate::face_domain::FaceDomain::new(m,i,tolerance_uv)).collect::<Result<Vec<_>>>());
        let [da,db]=domains;let da=da?;let db=db?;
        let face_pairs=a.faces.len().saturating_mul(b.faces.len());
        'contacts: for (i,fa) in a.faces.iter().enumerate(){for (j,fb) in b.faces.iter().enumerate(){
            if out.contact_pairs_visited==limits.contact_pairs || out.cells==limits.cells || out.domain_cells==limits.domain_cells {break 'contacts;}
            let remaining=(face_pairs-out.contact_pairs_visited).min(limits.contact_pairs-out.contact_pairs_visited);
            out.contact_pairs_visited+=1;
            let r=nurbs_core::surface_contact_search::search_trimmed(&fa.surface,&fb.surface,[&da[i].region,&db[j].region],
                ((limits.cells-out.cells)/remaining).max(1).min(100000),
                ((limits.domain_cells-out.domain_cells)/remaining).max(1).min(1000000))?;
            out.cells+=r.cells;out.domain_cells+=r.domain_cells;
            if let Some(w)=r.contact {
                out.contact=Some(([i,j],w));out.material_overlap=Some(true);
                out.distance_interval_mm=Some([0.,0.]);out.converged=true;out.reason="certified-boundary-contact";
                return Ok(out);
            }
        }}
    }
    if out.pairs.len()!=total_pairs{return Ok(out);}
    out.reason="containment-unproven";
    let mut a_in_b=vec![false;sa.len()];let mut b_in_a=vec![false;sb.len()];
    for p in &out.pairs{
        let Some(ab)=p.relation.witness_parity[0].as_ref().and_then(|r|r.parity) else{return Ok(out)};
        let Some(ba)=p.relation.witness_parity[1].as_ref().and_then(|r|r.parity) else{return Ok(out)};
        // Every connected shell lies in one complement component of every
        // opposite shell (strict positive separation proved by shell_relation).
        // Thus different witnesses can be combined by parity across all shells.
        a_in_b[p.shells[0]]^=ab;b_in_a[p.shells[1]]^=ba;
    }
    let overlap=a_in_b.into_iter().chain(b_in_a).any(|v|v);
    out.material_overlap=Some(overlap);
    if overlap{
        out.distance_interval_mm=Some([0.,0.]);out.converged=true;out.reason="material-containment";
    }else{
        if upper.is_finite(){out.distance_interval_mm=Some([lower,upper]);out.converged=upper-lower<=tolerance_mm;}
        out.reason=if out.converged{"separated-volumes"}else{"boundary-distance-unresolved"};
    }
    Ok(out)
}
#[cfg(test)]
mod tests{
    use super::*;
    fn limits()->Limits{
        Limits{validity:volume_validity::Limits{
            boundary:crate::boundary_embedding::Limits{exact_work:1000000,trim_pairs:10000,trim_cells:100000,trim_domain_cells:1000000,spans:1000,
                contacts:crate::face_contacts::Limits{pairs:10000,cells:100000,domain_cells:1000000,cells_per_pair:1000,domain_cells_per_pair:10000}},
            nesting_pairs:100,nesting_cells:100000,nesting_domain_cells:1000000,orientation_cells:100000,orientation_domain_cells:1000000,orientation_spans:100},
            pairs:100,contact_pairs:1000,cells:100000,domain_cells:1000000}
    }
    #[test]
    fn separated_cylinders_keep_the_analytic_axial_gap_and_authored_witnesses(){
        let a=crate::analytic::cylinder(2.,4.).unwrap();let mut b=a.clone();
        for v in &mut b.vertices{v.point[2]+=7.;}
        for e in &mut b.edges{for p in &mut e.curve.control_points{p[2]+=7.;}}
        for f in &mut b.faces{for row in &mut f.surface.control_points{for p in row{p[2]+=7.;}}}
        let r=distance(&a,&b,1e-5,1e-8,limits()).unwrap();
        assert!(r.converged,"{} {:?}",r.reason,r.distance_interval_mm);assert_eq!(r.material_overlap,Some(false));
        let d=r.distance_interval_mm.unwrap();assert!(d[0]<=3.&&d[1]>=3.&&d[1]-d[0]<=1e-5);
        let w=r.separation_witness(&a,&b).unwrap();let uv=w.geometry.parameters.unwrap();let points=w.geometry.points.unwrap();
        for (i,m) in [&a,&b].into_iter().enumerate(){assert_eq!(m.faces[w.faces[i]].surface.evaluate(uv[i][0],uv[i][1]).unwrap().point,points[i]);}
    }
    #[test]
    fn separation_witness_evaluates_on_original_faces_and_matches_upper_bound(){
        let a=crate::cuboid([0.;3],[2.;3]).unwrap();
        let mut b=crate::cuboid([5.,0.5,0.5],[6.,1.5,1.5]).unwrap();
        b.shells[0].faces.reverse();
        let r=distance(&a,&b,1e-5,1e-8,limits()).unwrap();
        let w=r.separation_witness(&a,&b).unwrap();
        let uv=w.geometry.parameters.unwrap();let points=w.geometry.points.unwrap();
        for (i,m) in [&a,&b].into_iter().enumerate(){
            let p=m.faces[w.faces[i]].surface.evaluate(uv[i][0],uv[i][1]).unwrap().point;
            assert_eq!(p,points[i]);
        }
        let gap=points[0].iter().zip(points[1]).map(|(a,b)|(a-b).powi(2)).sum::<f64>().sqrt();
        let bounds=r.distance_interval_mm.unwrap();
        assert!((gap-3.).abs()<1e-5);assert!(gap<=bounds[1]);
        let nested=crate::cuboid([0.5;3],[1.5;3]).unwrap();
        let r=distance(&a,&nested,1e-5,1e-8,limits()).unwrap();
        assert!(r.separation_witness(&a,&nested).is_none());
    }
    #[test]
    fn nested_material_has_zero_distance_but_cavity_keeps_its_gap(){
        let outer=crate::cuboid([0.;3],[10.;3]).unwrap();
        let hole=crate::cuboid([2.;3],[8.;3]).unwrap();
        let island=crate::cuboid([3.;3],[4.;3]).unwrap();
        let r=distance(&outer,&island,1e-5,1e-8,limits()).unwrap();
        assert_eq!(r.distance_interval_mm,Some([0.,0.]),"{} validity={:?}",r.reason,r.validity.iter().map(|v|(v.boundary.proven,v.boundary.agreement.all_equal,v.boundary.trim.all_valid,v.boundary.intersections.absence_proven,v.orientations.iter().map(|o|o.outward).collect::<Vec<_>>())).collect::<Vec<_>>());assert_eq!(r.material_overlap,Some(true));
        let cavity=crate::operations::boolean(&outer,&hole,"difference").unwrap();
        let before=format!("{cavity:?}{island:?}");
        for (a,b) in [(&cavity,&island),(&island,&cavity)]{
            let r=distance(a,b,1e-5,1e-8,limits()).unwrap();
            assert_eq!(r.material_overlap,Some(false),"{}",r.reason);
            let d=r.distance_interval_mm.unwrap();assert!(d[0]>0. && d[0]<=1. && d[1]>=1.);
            assert!(r.cells<=100000 && r.domain_cells<=1000000);
            let witness=r.separation_witness(a,b).unwrap();
            let uv=witness.geometry.parameters.unwrap();let points=witness.geometry.points.unwrap();
            for (i,model) in [a,b].into_iter().enumerate(){
                assert_eq!(model.faces[witness.faces[i]].surface.evaluate(uv[i][0],uv[i][1]).unwrap().point,points[i]);
                if model.shells.len()>1{
                    assert!(model.bodies[0].inner_shells.iter().any(|&shell|model.shells[shell].faces.iter().any(|f|f.face==witness.faces[i])));
                }
            }
            let gap=points[0].iter().zip(points[1]).map(|(a,b)|(a-b).powi(2)).sum::<f64>().sqrt();
            assert!((gap-1.).abs()<1e-5);
        }
        assert_eq!(format!("{cavity:?}{island:?}"),before);
    }
    #[test]
    fn crossing_boundaries_prove_zero_without_complete_pair_enumeration(){
        let a=crate::cuboid([0.;3],[2.;3]).unwrap();
        let b=crate::cuboid([1.,0.5,0.5],[3.,1.5,1.5]).unwrap();
        let before=format!("{a:?}{b:?}");
        for (a,b) in [(&a,&b),(&b,&a)]{
            let r=distance(a,b,1e-5,1e-8,limits()).unwrap();
            assert_eq!(r.distance_interval_mm,Some([0.,0.]),"{}",r.reason);
            assert!(r.contact.is_some());assert!(r.converged);
            assert_eq!(r.reason,"certified-boundary-contact");
            assert!(r.cells<=100000 && r.domain_cells<=1000000);
        }
        assert_eq!(format!("{a:?}{b:?}"),before);
    }
    #[test]
    fn separate_touching_and_incomplete_shell_pairs_remain_distinct(){
        let a=crate::cuboid([0.;3],[10.;3]).unwrap();
        let b=crate::cuboid([12.,0.,0.],[13.,1.,1.]).unwrap();
        let r=distance(&a,&b,1e-5,1e-8,limits()).unwrap();
        assert_eq!(r.material_overlap,Some(false));
        let d=r.distance_interval_mm.unwrap();assert!(d[0]>0. && d[0]<=2. && d[1]>=2.);
        let touching=crate::cuboid([10.,0.,0.],[11.,1.,1.]).unwrap();
        let r=distance(&a,&touching,1e-5,1e-8,limits()).unwrap();
        assert_eq!(r.material_overlap,None);assert!(!r.converged);
        assert!(r.distance_interval_mm.is_none_or(|d|d[0]==0.));
        let cavity=crate::operations::boolean(&a,&crate::cuboid([2.;3],[8.;3]).unwrap(),"difference").unwrap();
        let r=distance(&cavity,&b,1e-5,1e-8,Limits{pairs:1,..limits()}).unwrap();
        assert_eq!(r.total_pairs,2);assert_eq!(r.pairs.len(),1);
        assert_eq!(r.material_overlap,None);assert!(!r.converged);
        assert!(r.distance_interval_mm.is_none_or(|d|d[0]==0.));
    }
    #[test]
    fn unresolved_pairs_and_invalid_volume_never_publish_boundary_gap_as_solid_distance(){
        let a=crate::cuboid([0.;3],[10.;3]).unwrap();let mut b=crate::cuboid([2.;3],[8.;3]).unwrap();
        let r=distance(&a,&b,1e-5,1e-8,Limits{cells:2,domain_cells:2,..limits()}).unwrap();
        assert_eq!(r.material_overlap,None);assert!(!r.converged);
        assert!(r.distance_interval_mm.is_none_or(|d|d[0]==0.));
        for u in &mut b.shells[0].faces{u.reversed=!u.reversed;}
        let r=distance(&a,&b,1e-5,1e-8,limits()).unwrap();
        assert_eq!(r.distance_interval_mm,None);assert_eq!(r.reason,"volume-validity-unproven");
    }
}
