//! Exact full rectangular trim and world-boundary identity premise.
use crate::{curve::Curve,surface::Surface,curve_surface_agreement,Result,check};
pub struct Coedge<'a>{pub world:&'a Curve,pub uv:&'a Curve,pub reversed:bool}
#[derive(Clone,Debug)]
pub struct Report {pub domain_certified:bool,pub work:u64}
pub fn inspect(surface:&Surface,coedges:&[Coedge<'_>],has_holes:bool,max_work:u64)->Result<Report>{
    check(max_work<=1000000,"Retained wall exact budget exceeds 1000000")?;
    let mut out=Report {domain_certified:false,work:0};
    if has_holes || coedges.len()!=4{return Ok(out);}
    for usage in coedges {
        let c=usage.uv;
        if c.periodic||c.degree!=1||c.knots!=[0.,0.,1.,1.]||c.control_points.len()!=2
            ||c.control_points.iter().any(|p|p.len()!=2||p.iter().any(|x|!x.is_finite()))
            ||c.weights.len()!=2||c.weights.iter().any(|w|!w.is_finite()||*w<=0.) {return Ok(out);}
    }
    let corners=[[0.,0.],[1.,0.],[1.,1.],[0.,1.]];
    let rectangle=(0..4).any(|start|[1isize,-1].into_iter().any(|direction|{
        coedges.iter().enumerate().all(|(i,usage)|{
            let a=(start as isize+direction*i as isize).rem_euclid(4) as usize;
            let b=(start as isize+direction*(i+1) as isize).rem_euclid(4) as usize;
            usage.uv.control_points[0].as_slice()==corners[a]&&usage.uv.control_points[1].as_slice()==corners[b]
        })
    }));
    if !rectangle{return Ok(out);}
    for usage in coedges {
        if out.work>=max_work{return Ok(out);}
        let Some(decision)=curve_surface_agreement::verify_exact(usage.world,usage.uv,surface,usage.reversed,max_work-out.work)? else {
            out.work=max_work;return Ok(out);
        };
        if decision.work_used>max_work-out.work {out.work=max_work;return Ok(out);}
        out.work+=decision.work_used;
        if decision.outcome!=cad_predicates::BezierIdentity::Equal {return Ok(out);}
    }
    out.domain_certified=true;Ok(out)
}
