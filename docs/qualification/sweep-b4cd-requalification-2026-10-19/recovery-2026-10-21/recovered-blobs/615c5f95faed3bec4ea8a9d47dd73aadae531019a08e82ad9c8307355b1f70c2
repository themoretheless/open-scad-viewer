//! Planar catenary with ideal Hermite remainder and stable near-vertex heights.
use crate::{Result,check};
pub struct Approximation { pub fit:crate::helix::Approximation }
#[cfg(feature="codec")]
impl value_codec::Serialize for Approximation{
 fn to_value(&self)->value_codec::Value{
  value_codec::json!({"curve":self.fit.curve,"report":{"spans":self.fit.spans,"budget":self.fit.budget,
   "realArithmeticErrorEstimate":self.fit.real_arithmetic_error_estimate,"continuousBound":false,
   "roundingCertified":false,"method":"uniform-abscissa-cubic-Hermite-fourth-derivative-estimate"}})
 }
}
/// P(x)=center+[x,scale*(cosh(x/scale)-1),0], increasing local x bounds.
pub fn approximate(center:[f64;3],scale:f64,start:f64,end:f64,budget:f64)->Result<Approximation>{
 check(center.iter().all(|x|x.is_finite()) && scale.is_finite() && scale>0.
  && start.is_finite() && end.is_finite() && end>start && budget.is_finite() && budget>0.,
  "Catenary requires finite inputs, positive scale/budget and increasing local x bounds")?;
 let range=end-start;let maximum=start.abs().max(end.abs())/scale;
 check(range.is_finite() && maximum.is_finite(),"Catenary range overflow")?;
 let cosh=maximum.cosh();let mut choice=None;
 for spans in 1..=85{
  let step=(range/spans as f64)/scale;let squared=step*step;
  let estimate=scale*cosh*(squared*squared)/384.;
  if estimate.is_finite() && estimate>0. && estimate<=budget{choice=Some((spans,estimate));break;}
 }
 let (spans,estimate)=choice.ok_or_else(||crate::input("Catenary estimate is unrepresentable or exceeds 85 cubic spans"))?;
 let mut points=Vec::new();let mut tangents=Vec::new();let mut parameters=Vec::new();
 for i in 0..=spans{
  let t=i as f64/spans as f64;let x=(1.-t)*start+t*end;let u=x/scale;
  // cosh(u)-1 = 2*sinh(u/2)^2 avoids subtractive cancellation near zero.
  let half=(u/2.).sinh();let y=scale*(2.*half*half);
  points.push([center[0]+x,center[1]+y,center[2]]);
  tangents.push([range,range*u.sinh(),0.]);parameters.push(t);
 }
 let curve=crate::hermite::interpolate_inferred(&points,&tangents,&parameters)?;
 Ok(Approximation{fit:crate::helix::Approximation{curve,spans,budget,real_arithmetic_error_estimate:estimate}})
}
#[cfg(test)]
mod tests{
 use super::*;
 #[test]
 fn independent_cosh_samples_and_endpoint_tangents(){
  let a=approximate([3.,4.,5.],2.,-3.,4.,1e-4).unwrap().fit;
  for i in 0..=1000{
   let t=i as f64/1000.;let x=-3.+7.*t;let p=a.curve.evaluate(t).unwrap().point;
   let exact=[3.+x,4.+2.*((x/2.).cosh()-1.),5.];
   let error=(0..3).map(|d|(p[d]-exact[d]).powi(2)).sum::<f64>().sqrt();
   assert!(error<=a.real_arithmetic_error_estimate+1e-12);
  }
  for (t,x) in [(0.,-3_f64),(1.,4_f64)]{
   let d=a.curve.evaluate(t).unwrap().d1.unwrap();
   assert!((d[0]-7.).abs()<1e-11);assert!((d[1]-7.*(x/2.).sinh()).abs()<1e-11);
  }
 }
 #[test]
 fn near_vertex_height_survives_and_unrepresentable_requests_refuse(){
  let a=approximate([0.;3],1.,1e-9,2e-9,1e-30).unwrap().fit;
  let height=a.curve.evaluate(0.).unwrap().point[1];assert!(height>0.);assert!((height-5e-19).abs()<1e-33);
  assert!(approximate([0.;3],0.,-1.,1.,1e-4).is_err());
  assert!(approximate([0.;3],1.,-1000.,1000.,1e-4).is_err());
  assert!(approximate([0.;3],1.,-10.,10.,1e-20).is_err());
 }
}
