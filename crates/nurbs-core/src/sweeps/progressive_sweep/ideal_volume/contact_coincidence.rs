//! Exact negative witnesses only; no sampled point grants embedding credit.
//! A planar original path and a constant normal rail give N=+/-e_axis.
//! On a carrier isoline parallel to N, contact fitting is rational even with
//! varying scale/axis/center. Exact original-knot coincidences can then refute
//! the FULL filled map before a global pair search. Declining proves nothing.
use super::*;
use crate::core::bounded_rational::{Error,Work};
use num_rational::BigRational as Q;
use num_traits::{Signed,Zero};
mod exact;
use exact::{R,Spline,add,sub,mul,div,same,unit};
#[derive(Debug)]
pub(super) struct Witness {
    pub raw_path_parameters:[f64;2],
    pub normalized_cap_parameters:[[Q;2];2],
    pub world:[Q;3],
}
fn visit(cells:&mut usize,limit:usize)->R<()> {
    if *cells==limit {return Err(Error::ResourceLimit);}*cells+=1;Ok(())
}
fn terms(t:&Q,scale:&Spline,affine:&Option<(Spline,Spline)>,anchor:&Q,w:&mut Work)->R<Option<(Q,Q,Q)>> {
    let mut b=scale.normalized(t,w)?[0].clone();let mut c=w.lift(0.)?;
    if let Some((axes,center))=affine {
        let a=axes.normalized(t,w)?;let offset=center.normalized(t,w)?;
        if !offset[1].is_zero() || !offset[2].is_zero(){return Ok(None);}
        b=mul(w,&b,&a[0])?;c=offset[0].clone();
    }
    let base=mul(w,&b,anchor)?;let d=add(w,&base,&c)?;
    Ok((b.is_positive() && d.is_positive()).then_some((b,c,d)))
}
fn attempt(s:&Sweep<'_>,cap:&Surface,w:&mut Work,cells:&mut usize,limit:usize,closed:bool)->R<Option<Witness>> {
    let guide=s.orientation_guide.unwrap();let (profile,parameter)=s.contact_source.unwrap();
    // Each metadata/leaf comparison is bounded and charged, including failures.
    w.charge((s.path.knots.len()+s.path.weights.len()+9*s.path.control_points.len()+s.twist.control_points.len()) as u64)?;
    if s.path.degree!=guide.degree || s.path.knots!=guide.knots || s.path.weights!=guide.weights
        || s.path.control_points.len()!=guide.control_points.len()
        || s.twist.control_points.iter().any(|p|p[0]!=0.) {return Ok(None);}
    let Some(axis)=(0..3).find(|&axis|{
        s.path.control_points.iter().all(|p|p[axis]==s.path.control_points[0][axis])
        && guide.control_points.iter().all(|p|p[axis]==guide.control_points[0][axis])
        && guide.control_points[0][axis]!=s.path.control_points[0][axis]
        && s.path.control_points.iter().zip(&guide.control_points).all(|(p,g)|(0..3).all(|k|k==axis || p[k]==g[k]))
    }) else {return Ok(None);};
    let mut raw=Vec::new();let domain=s.path.domain();
    for &k in &s.path.knots {
        if domain[0]<=k && k<=domain[1] && raw.last()!=Some(&k){raw.push(k);}
        if raw.len()>64 {return Ok(None);}
    }
    let path=Spline::load(s.path,w)?;let scale=Spline::load(s.scale,w)?;
    let affine=s.affine_laws.map(|(a,c)|Ok((Spline::load(a,w)?,Spline::load(c,w)?))).transpose()?;
    let zero=w.lift(0.)?;let start=path.normalized(&zero,w)?;
    let rail=w.lift(guide.control_points[0][axis])?;let plane=w.lift(s.path.control_points[0][axis])?;
    let offset=sub(w,&rail,&plane)?;let sign=w.lift(if offset.is_positive(){1.}else{-1.})?;
    let width=mul(w,&offset,&sign)?;
    let reference=Spline::load(profile,w)?;let anchor_t=w.lift(parameter)?;let point=reference.point(&anchor_t,w)?;
    let delta=sub(w,&point[axis],&start[axis])?;let anchor=mul(w,&delta,&sign)?;
    if !anchor.is_positive(){return Ok(None);}
    let mut stations=Vec::new();
    for &t in &raw {
        visit(cells,limit)?;let raw_t=w.lift(t)?;let p=path.point(&raw_t,w)?;
        let fraction=path.fraction(t,w)?;let motion=terms(&fraction,&scale,&affine,&anchor,w)?;
        stations.push((p,motion));
    }
    let mut net=Vec::new();
    for row in &cap.control_points {net.push(row.iter().map(|p|p.iter().map(|x|w.lift(*x)).collect::<R<Vec<_>>>()).collect::<R<Vec<_>>>()?);}
    for i in 0..raw.len(){for j in i+1..raw.len(){
        visit(cells,limit)?;
        if closed && raw[i]==domain[0] && raw[j]==domain[1]{continue;}
        let (p1,Some((b1,c1,d1)))=&stations[i] else{continue;};
        let (p2,Some((b2,c2,d2)))=&stations[j] else{continue;};
        let mut equal=true;for k in 0..3 {equal &= same(w,&p1[k],&p2[k])?;}
        if !equal {continue;}
        for along in 0..2 {for fixed in [0.5,0.,1.] {
            visit(cells,limit)?;let f=w.lift(fixed)?;let one=w.lift(1.)?;let g=sub(w,&one,&f)?;
            let mut ends=Vec::new();
            for endpoint in 0..2 {
                let (a,b)=if along==0 {(&net[endpoint][0],&net[endpoint][1])} else {(&net[0][endpoint],&net[1][endpoint])};
                let mut p=Vec::new();for k in 0..3 {let x=mul(w,&a[k],&g)?;let y=mul(w,&b[k],&f)?;p.push(add(w,&x,&y)?);}ends.push(p);
            }
            let mut axial=true;for endpoint in &ends {for k in 0..3 {if k!=axis {axial &= same(w,&endpoint[k],&start[k])?;}}}
            if !axial {continue;}
            let delta=sub(w,&ends[0][axis],&start[axis])?;let alpha=mul(w,&delta,&sign)?;
            let delta=sub(w,&ends[1][axis],&ends[0][axis])?;let beta=mul(w,&delta,&sign)?;
            if beta.is_zero(){continue;}
            for proposed in [0.25,0.75,0.5] {
                visit(cells,limit)?;let u1=w.lift(proposed)?;
                let delta=mul(w,&beta,&u1)?;let normal1=add(w,&alpha,&delta)?;
                let delta=mul(w,b1,&normal1)?;let value1=add(w,&delta,c1)?;
                let delta=mul(w,&value1,d2)?;let scaled=div(w,&delta,d1)?;
                let delta=sub(w,&scaled,c2)?;let normal2=div(w,&delta,b2)?;
                let delta=sub(w,&normal2,&alpha)?;let u2=div(w,&delta,&beta)?;
                if !unit(w,&u2)?{continue;}
                // Recompose BOTH original fields at the constructed rational
                // parameters; a solved proposal alone is never authority.
                let delta=mul(w,&beta,&u2)?;let actual=add(w,&alpha,&delta)?;
                let delta=mul(w,b2,&actual)?;let value2=add(w,&delta,c2)?;
                let delta=mul(w,&width,&value1)?;let z1=div(w,&delta,d1)?;
                let delta=mul(w,&width,&value2)?;let z2=div(w,&delta,d2)?;
                if !same(w,&z1,&z2)?{continue;}
                let mut world=p1.clone();let delta=mul(w,&sign,&z1)?;world[axis]=add(w,&world[axis],&delta)?;
                let uv=if along==0 {[[u1,f.clone()],[u2,f]]} else {[[f.clone(),u1],[f,u2]]};
                return Ok(Some(Witness{raw_path_parameters:[raw[i],raw[j]],normalized_cap_parameters:uv,world:world.try_into().unwrap()}));
            }
        }}
    }}
    Ok(None)
}
/// The caller has freshly proved the original guided frame over this source.
/// This is an optional counterexample search, never a positive regularity gate.
pub(super) fn inspect(s:&Sweep<'_>,caps:[&Surface;2],out:&mut IdealVolumeReport,max_cells:usize,max_work:u64,closed:bool)->Option<Witness>{
    let guide=s.orientation_guide?;let (profile,_)=s.contact_source?;
    if s.frame_laws.is_some() || s.options.spacing!=Spacing::Parameter
        || s.path.control_points.len()==s.path.degree+1
        || caps[0].periodic_u || caps[0].periodic_v || caps[0].degree_u!=1 || caps[0].degree_v!=1
        || caps[0].control_points.len()!=2 || caps[0].control_points[0].len()!=2 || caps[0]!=caps[1]
        || [&s.path,&s.scale,&s.twist,&guide,&profile].iter().any(|c|c.periodic || c.control_points.len()>4096)
        || s.affine_laws.is_some_and(|(a,c)|a.periodic || c.periodic || a.control_points.len()>4096 || c.control_points.len()>4096)
        || caps[0].weights.iter().flatten().any(|w|*w!=caps[0].weights[0][0])
        || [&caps[0].knots_u,&caps[0].knots_v].iter().any(|k|k.len()!=4 || k[0]!=k[1] || k[2]!=k[3]) {return None;}
    let mut w=Work{used:0,limit:max_work.saturating_sub(out.exact_work).min(100000)};
    let mut cells=0;let limit=max_cells.saturating_sub(out.cells).min(100000);
    let result=attempt(s,caps[0],&mut w,&mut cells,limit,closed).ok().flatten();
    out.cells+=cells;out.exact_work+=w.used;result
}
#[cfg(test)] mod tests;
