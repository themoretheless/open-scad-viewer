//! Exponential radial spiral with an explicit ideal Hermite remainder estimate.
use crate::{Result,check};
pub use crate::helix::Approximation;
/// Radius(theta)=radius*exp(growth*(theta-start)); angles in radians.
pub fn approximate(center:[f64;3],radius:f64,growth:f64,start:f64,end:f64,budget:f64)->Result<Approximation>{
 check(center.iter().all(|x|x.is_finite()) && radius.is_finite() && radius>0.
  && growth.is_finite() && start.is_finite() && end.is_finite() && end>start
  && budget.is_finite() && budget>0.,"Logarithmic spiral requires finite data, increasing angles and positive radius/budget")?;
 let range=end-start;let exponent=growth*range;
 let end_radius=radius*exponent.exp();
 check(range.is_finite() && exponent.is_finite() && end_radius.is_finite() && end_radius>0.,"Logarithmic spiral radius/range overflow or underflow")?;
 let frequency=range.hypot(exponent);
 let max_radius=radius.max(end_radius);
 let mut choice=None;
 for n in 1..=85{
  let h=frequency/n as f64;let h2=h*h;
  let estimate=max_radius*(h2*h2)/384.*2_f64.sqrt();
  if estimate.is_finite() && estimate>0. && estimate<=budget{choice=Some((n,estimate));break;}
 }
 let (spans,estimate)=choice.ok_or_else(||crate::input("Logarithmic spiral estimate is unrepresentable or exceeds 85 cubic spans"))?;
 let mut points=Vec::new();let mut tangents=Vec::new();let mut parameters=Vec::new();
 for i in 0..=spans{
  let t=i as f64/spans as f64;let theta=(1.-t)*start+t*end;
  let r=radius*(exponent*t).exp();let (sin,cos)=theta.sin_cos();
  check(r.is_finite() && r>0.,"Logarithmic spiral station radius is unrepresentable")?;
  points.push([center[0]+r*cos,center[1]+r*sin,center[2]]);
  tangents.push([r*(exponent*cos-range*sin),r*(exponent*sin+range*cos),0.]);parameters.push(t);
 }
 let curve=crate::hermite::interpolate_inferred(&points,&tangents,&parameters)?;
 Ok(Approximation{curve,spans,budget,real_arithmetic_error_estimate:estimate})
}
#[cfg(test)]
mod tests{
 use super::*;
 #[test]
 fn growing_shrinking_and_circular_limits_match_independent_formula(){
  for growth in [-0.5,0.,0.5]{
   let a=approximate([3.,4.,5.],2.,growth,-1.,2.,1e-4).unwrap();
   for i in 0..=1000{
    let t=i as f64/1000.;let theta=-1.+3.*t;let r=2.*(growth*(theta+1.)).exp();
    let exact=[3.+r*theta.cos(),4.+r*theta.sin(),5.];let p=a.curve.evaluate(t).unwrap().point;
    let distance=(0..3).map(|d|(p[d]-exact[d]).powi(2)).sum::<f64>().sqrt();
    assert!(distance<=a.real_arithmetic_error_estimate+1e-12);
   }
  }
 }
 #[test]
 fn rejects_unrepresentable_radius_and_unattainable_budget(){
  for growth in [-1000.,1000.]{assert!(approximate([0.;3],2.,growth,0.,2.,1e-4).is_err());}
  assert!(approximate([0.;3],2.,0.5,0.,100.,1e-12).is_err());
  assert!(approximate([0.;3],0.,0.5,0.,2.,1e-4).is_err());
 }
}
