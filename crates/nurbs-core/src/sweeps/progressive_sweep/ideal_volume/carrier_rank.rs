//! Negative parameter-rank witnesses for the original filled-carrier homotopy.
//! Every pose is affine in the source point. Dependent UV columns remain
//! dependent under ANY such pose, so the full volume cannot have rank three.
use super::*;
use super::exact::{R,add,sub,mul,div};
use crate::core::bounded_rational::{Error,Work};
use num_rational::BigRational as Q;
use num_traits::{Signed,Zero};
mod tensor;
#[derive(Debug)]
pub(super) struct Witness {pub normalized_parameters:[Q;3],pub partials:[[Q;3];2]}
fn candidate(caps:[&Surface;2],t:f64)->bool {
    // This floating filter only proposes an exact query. Neither a rounded
    // zero nor declining the query supplies any rank/embedding authority.
    let mut derivatives=Vec::new();for s in caps {
        let u=crate::distance_bounds::midpoint(s.knots_u[s.degree_u],s.knots_u[s.control_points.len()]);
        let v=crate::distance_bounds::midpoint(s.knots_v[s.degree_v],s.knots_v[s.control_points[0].len()]);
        let Ok(p)=s.evaluate(u,v) else{return true;};let Some((du,dv))=p.first_derivatives() else{return true;};
        let su=s.knots_u[s.control_points.len()]-s.knots_u[s.degree_u];
        let sv=s.knots_v[s.control_points[0].len()]-s.knots_v[s.degree_v];
        derivatives.push([du.map(|x|x*su),dv.map(|x|x*sv)]);
    }
    let d:[[f64;3];2]=std::array::from_fn(|i|std::array::from_fn(|k|(1.-t)*derivatives[0][i][k]+t*derivatives[1][i][k]));
    let a=d[0].iter().map(|x|x*x).sum::<f64>();let b=d[1].iter().map(|x|x*x).sum::<f64>();
    let c=[d[0][1]*d[1][2]-d[0][2]*d[1][1],d[0][2]*d[1][0]-d[0][0]*d[1][2],d[0][0]*d[1][1]-d[0][1]*d[1][0]];let cross=c.iter().map(|x|x*x).sum::<f64>();
    !a.is_finite() || !b.is_finite() || !cross.is_finite() || a==0. || b==0. || cross<=1e-16*a*b
}
fn prove(caps:[&Surface;2],t:f64,w:&mut Work)->R<Option<Witness>> {
    let uv=[w.lift(0.5)?,w.lift(0.5)?];let phase=w.lift(t)?;let one=w.lift(1.)?;let before=sub(w,&one,&phase)?;
    let a=tensor::Tensor::load(caps[0],w)?.first(&uv,w)?;
    let b=if caps[0]==caps[1] {a.clone()} else {tensor::Tensor::load(caps[1],w)?.first(&uv,w)?};
    let mut columns=Vec::new();for i in 0..2 {let mut row=Vec::new();for k in 0..3 {
        let x=mul(w,&a[i][k],&before)?;let y=mul(w,&b[i][k],&phase)?;row.push(add(w,&x,&y)?);
    }columns.push(row);}
    for (i,j) in [(0,1),(0,2),(1,2)] {
        let a=mul(w,&columns[0][i],&columns[1][j])?;let b=mul(w,&columns[0][j],&columns[1][i])?;
        if !sub(w,&a,&b)?.is_zero(){return Ok(None);}
    }
    Ok(Some(Witness{normalized_parameters:[uv[0].clone(),uv[1].clone(),phase],
        partials:columns.into_iter().map(|p|p.try_into().unwrap()).collect::<Vec<_>>().try_into().unwrap()}))
}
pub(super) fn inspect(caps:[&Surface;2],out:&mut IdealVolumeReport,max_cells:usize,max_work:u64)->Option<Witness> {
    if caps.iter().any(|s|s.periodic_u || s.periodic_v || s.control_points.len()*s.control_points[0].len()>4096){return None;}
    for t in [0.5,0.,1.] {
        if out.cells==max_cells{return None;}out.cells+=1;
        // Reserve the same proposal cells for equivalent cap encodings.
        // Literal equality skips redundant evaluation, never the ledger charge.
        if (t!=0.5 && caps[0]==caps[1]) || !candidate(caps,t){continue;}
        let mut w=Work{used:0,limit:max_work.saturating_sub(out.exact_work).min(100000)};
        let found=prove(caps,t,&mut w).ok().flatten();out.exact_work+=w.used;
        if found.is_some(){return found;}
    }
    None
}
#[cfg(test)] mod tests;
