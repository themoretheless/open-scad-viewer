//! Straight-line rolling-circle trajectories with explicit ideal Hermite remainder.
use crate::{Result,check};
pub use crate::helix::Approximation;
/// P(theta)=center+[rolling_radius*theta-tracing_radius*sin(theta),
/// rolling_radius-tracing_radius*cos(theta),0]. Angles are radians.
pub fn approximate(center:[f64;3],rolling_radius:f64,tracing_radius:f64,start:f64,end:f64,budget:f64)->Result<Approximation>{
 check(center.iter().all(|x|x.is_finite()) && rolling_radius.is_finite() && rolling_radius>0.
  && tracing_radius.is_finite() && tracing_radius>=0. && start.is_finite() && end.is_finite()
  && end>start && budget.is_finite() && budget>0.,"Trochoid requires finite data, positive rolling radius/budget, nonnegative tracing radius and increasing angles")?;
 let range=end-start;check(range.is_finite(),"Trochoid angle range overflow")?;
 let mut choice=None;
 for spans in 1..=85{
  let h=range/spans as f64;let h2=h*h;
  let estimate=if tracing_radius==0.{0.}else{tracing_radius*(h2*h2)/384.*2_f64.sqrt()};
  if estimate.is_finite() && (estimate>0. || tracing_radius==0.) && estimate<=budget{choice=Some((spans,estimate));break;}
 }
 let (spans,estimate)=choice.ok_or_else(||crate::input("Trochoid estimate is unrepresentable or exceeds 85 cubic spans"))?;
 let mut points=Vec::new();let mut tangents=Vec::new();let mut parameters=Vec::new();
 for i in 0..=spans{
  let t=i as f64/spans as f64;let theta=(1.-t)*start+t*end;let (sin,cos)=theta.sin_cos();
  points.push([center[0]+rolling_radius*theta-tracing_radius*sin,center[1]+rolling_radius-tracing_radius*cos,center[2]]);
  tangents.push([(rolling_radius-tracing_radius*cos)*range,tracing_radius*sin*range,0.]);parameters.push(t);
 }
 let curve=crate::hermite::interpolate_inferred(&points,&tangents,&parameters)?;
 Ok(Approximation{curve,spans,budget,real_arithmetic_error_estimate:estimate})
}
/// Cycloid is the rolling-circle point itself, tracing_radius=rolling_radius.
pub fn approximate_cycloid(center:[f64;3],radius:f64,start:f64,end:f64,budget:f64)->Result<Approximation>{
 approximate(center,radius,radius,start,end,budget)
}
#[cfg(test)]
mod tests{
 use super::*;
 #[test]
 fn independent_curtate_cycloid_prolate_and_line_formulas(){
  for distance in [0.,1.,2.,3.]{
   let a=approximate([3.,4.,5.],2.,distance,-1.,5.,1e-4).unwrap();
   for i in 0..=1000{
    let t=i as f64/1000.;let theta=-1.+6.*t;
    let p=a.curve.evaluate(t).unwrap().point;
    let exact=[3.+2.*theta-distance*theta.sin(),6.-distance*theta.cos(),5.];
    let error=(0..3).map(|d|(p[d]-exact[d]).powi(2)).sum::<f64>().sqrt();
    assert!(error<=a.real_arithmetic_error_estimate+1e-12);
   }
   if distance==0.{assert_eq!(a.spans,1);assert_eq!(a.real_arithmetic_error_estimate,0.);}
  }
 }
 #[test]
 fn cycloid_cusp_and_endpoint_derivatives_are_preserved(){
  let a=approximate_cycloid([0.;3],2.,0.,2.,1e-4).unwrap();
  let start=a.curve.evaluate(0.).unwrap();assert_eq!(start.point,vec![0.,0.,0.]);assert_eq!(start.d1.unwrap(),vec![0.,0.,0.]);
  let end=a.curve.evaluate(1.).unwrap();let d=end.d1.unwrap();
  assert!((d[0]-4.*(1.-2_f64.cos())).abs()<1e-11);assert!((d[1]-4.*2_f64.sin()).abs()<1e-11);
  assert!(approximate([0.;3],2.,-1.,0.,2.,1e-4).is_err());
  assert!(approximate_cycloid([0.;3],2.,0.,100.,1e-12).is_err());
 }
}
