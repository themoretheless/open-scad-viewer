//! Discrete rotation-minimizing sweep, using Wang et al. (2008), Table I.
//! The loft is piecewise linear along the path; it is not an exact RMF surface.
use crate::{check, curve::Curve, surface::{Surface,loft}, Result};
type V = [f64;3];
fn dot(a:V,b:V)->f64{(0..3).map(|i|a[i]*b[i]).sum()}
fn sub(a:V,b:V)->V{std::array::from_fn(|i|a[i]-b[i])}
fn cross(a:V,b:V)->V{[a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]]}
fn unit(a:V)->Result<V>{let n=dot(a,a).sqrt();check(n.is_finite()&&n>1e-12,"Sweep frame has a zero or nonfinite direction")?;Ok(a.map(|x|x/n))}
fn reflect(a:V,v:V)->Result<V>{let d=dot(v,v);check(d.is_finite()&&d>1e-24,"Sweep frame reflection is degenerate; refine or split the path")?;let s=2.*dot(a,v)/d;Ok(std::array::from_fn(|i|a[i]-s*v[i]))}
fn sample(path:&Curve,t:f64)->Result<(V,V)>{
 let e=path.evaluate(t)?;
 let d=e.d1.ok_or_else(||crate::Error::new(crate::INVALID_INPUT,"Sweep requires a regular differentiable path"))?;
 Ok(([e.point[0],e.point[1],e.point[2]],unit([d[0],d[1],d[2]])?))
}
/// Origin is the path start. Initial normal selects a frame, but the first profile
/// stays at its original world position. Subsequent sections are rigidly transported.
/// Closed paths are refused until seam holonomy is handled explicitly.
fn frames(profile:&Curve,path:&Curve,normal:V,sections:usize)->Result<Vec<Curve>>{
 profile.validate()?;path.validate()?;
 check((2..=125).contains(&sections),"Framed sweep frame count exceeds 125")?;
 check(profile.control_points[0].len()==3&&path.control_points[0].len()==3,"Framed sweep requires 3D curves")?;
 check(!path.periodic,"Framed sweep closed-path seam is not supported")?;
 let [a,b]=path.domain();let (start,t0)=sample(path,a)?;
 let end=sample(path,b)?.0;
 check(dot(sub(end,start),sub(end,start))>1e-24,"Framed sweep closed-path seam is not supported")?;
 // Reject internal knots without a unique tangent, including ones not sampled.
 for &k in &path.knots{if k>a&&k<b {check(path.knots.iter().filter(|&&v|v==k).count()<path.degree,"Split the sweep path at tangent discontinuities")?;}}
 let n=unit(normal)?;let r0=unit(sub(n,t0.map(|x|x*dot(n,t0))))?;let s0=cross(t0,r0);
 let coordinates:Vec<V>=profile.control_points.iter().map(|p|{let q=sub([p[0],p[1],p[2]],start);[dot(q,r0),dot(q,s0),dot(q,t0)]}).collect();
 let(mut position,mut tangent,mut r)=(start,t0,r0);let mut curves=Vec::new();
 for i in 0..sections{
  let (next,t)=sample(path,a+(b-a)*i as f64/(sections-1)as f64)?;
  if i>0{
   let v=sub(next,position);let reflected_r=reflect(r,v)?;let reflected_t=reflect(tangent,v)?;
   r=unit(reflect(reflected_r,sub(t,reflected_t))?)?;
   r=unit(sub(r,t.map(|x|x*dot(r,t))))?;
  }
  let s=cross(t,r);let mut c=profile.clone();
  c.control_points=coordinates.iter().map(|q|(0..3).map(|k|next[k]+q[0]*r[k]+q[1]*s[k]+q[2]*t[k]).collect()).collect();
  curves.push(c);position=next;tangent=t;
 }
 Ok(curves)
}
pub fn sweep(profile:&Curve,path:&Curve,normal:V,sections:usize)->Result<Surface>{
 check((2..=32).contains(&sections),"Framed sweep requires 2..32 sections")?;
 let curves=frames(profile,path,normal,sections)?;
 let mut result=loft(&curves)?;
 for k in &mut result.knots_v{*k/=(sections-1)as f64;}
 result.validate()?;Ok(result)
}
/// A refinement diagnostic, NOT a continuous Hausdorff or exact RMF certificate.
/// All profile controls share the same positive rational weights in every section,
/// so their maximum displacement bounds the whole profile at each checked station.
pub fn checked_sweep(profile:&Curve,path:&Curve,normal:V,sections:usize,max_deviation:f64)->Result<value_codec::Value>{
 check(max_deviation.is_finite()&&max_deviation>=0.,"Sweep deviation budget must be finite and nonnegative")?;
 let surface=sweep(profile,path,normal,sections)?;
 let fine=frames(profile,path,normal,4*(sections-1)+1)?;
 let mut maximum=0_f64;
 for (i,section) in fine.iter().enumerate(){
  let interval=(i/4).min(sections-2);let fraction=(i-4*interval)as f64/4.;
  for (k,p) in section.control_points.iter().enumerate(){
   let delta:V=std::array::from_fn(|axis|p[axis]-((1.-fraction)*surface.control_points[k][interval][axis]+fraction*surface.control_points[k][interval+1][axis]));
   let error=dot(delta,delta).sqrt();check(error.is_finite(),"Sweep refinement error overflowed")?;maximum=maximum.max(error);
  }
 }
 let accepted=maximum<=max_deviation;
 use value_codec::json;
 Ok(json!({"surface":if accepted {Some(surface)}else{None},"report":{
  "accepted":accepted,"sampledControlDeviation":maximum,"budget":max_deviation,
  "stations":fine.len(),"sections":sections,"continuousBound":false,
  "method":"double-reflection-fourfold-section-refinement"}}))
}
#[cfg(test)]mod tests{
 use super::*;
 fn line(a:V,b:V)->Curve{Curve{degree:1,knots:vec![0.,0.,1.,1.],control_points:vec![a.to_vec(),b.to_vec()],weights:vec![1.,1.],periodic:false}}
 #[test]fn straight_path_preserves_profile_and_translates(){
  let p=line([1.,0.,0.],[2.,0.,0.]);let q=line([0.,0.,0.],[0.,0.,5.]);let s=sweep(&p,&q,[1.,0.,0.],8).unwrap();
  for i in 0..=10{let t=i as f64/10.;let x=s.evaluate(0.3,t).unwrap().point;assert!((x[0]-1.3).abs()<1e-12);assert!((x[2]-5.*t).abs()<1e-12);}
 }
 #[test]fn circular_path_turns_profile_without_changing_section_length(){
  let p=line([1.,0.,0.],[1.2,0.,0.]);let q=Curve{degree:2,knots:vec![0.,0.,0.,1.,1.,1.],control_points:vec![vec![1.,0.,0.],vec![1.,1.,0.],vec![0.,1.,0.]],weights:vec![1.,std::f64::consts::FRAC_1_SQRT_2,1.],periodic:false};
  let s=sweep(&p,&q,[1.,0.,0.],17).unwrap();
  for i in 0..17{let t=i as f64/16.;let a=s.evaluate(0.,t).unwrap().point;let b=s.evaluate(1.,t).unwrap().point;let center=q.evaluate(t).unwrap().point;
   for k in 0..3{assert!((a[k]-center[k]).abs()<1e-12);}
   assert!(((b[0]*b[0]+b[1]*b[1]).sqrt()-1.2).abs()<1e-12);
  }
 }
 #[test]fn rejects_ambiguous_initial_frame_and_invalid_counts(){let p=line([1.,0.,0.],[2.,0.,0.]);let q=line([0.,0.,0.],[0.,0.,5.]);assert!(sweep(&p,&q,[0.,0.,1.],8).is_err());assert!(sweep(&p,&q,[1.,0.,0.],33).is_err());}
 #[test]fn refinement_detects_arc_error_and_improves_with_sections(){
  let p=line([1.,0.,0.],[1.2,0.,0.]);let q=Curve{degree:2,knots:vec![0.,0.,0.,1.,1.,1.],control_points:vec![vec![1.,0.,0.],vec![1.,1.,0.],vec![0.,1.,0.]],weights:vec![1.,std::f64::consts::FRAC_1_SQRT_2,1.],periodic:false};
  let coarse=checked_sweep(&p,&q,[1.,0.,0.],3,0.001).unwrap();
  assert_eq!(coarse["report"]["accepted"].as_bool(),Some(false));assert!(coarse["surface"].is_null());
  let fine=checked_sweep(&p,&q,[1.,0.,0.],32,0.001).unwrap();
  assert_eq!(fine["report"]["accepted"].as_bool(),Some(true));
  assert!(fine["report"]["sampledControlDeviation"].as_f64().unwrap()<coarse["report"]["sampledControlDeviation"].as_f64().unwrap()/100.);
  assert_eq!(fine["report"]["continuousBound"].as_bool(),Some(false));
  assert!(checked_sweep(&p,&q,[1.,0.,0.],32,f64::NAN).is_err());
 }

}
