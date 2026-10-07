//! Inside/outside circle rolling trajectories with an ideal Hermite estimate.
use crate::{Result,check};
pub use crate::helix::Approximation;
pub fn approximate_epicycloid(center:[f64;3],fixed_radius:f64,rolling_radius:f64,start:f64,end:f64,budget:f64)->Result<Approximation>{
 rolling(center,fixed_radius,rolling_radius,start,end,budget,false)
}
pub fn approximate_hypocycloid(center:[f64;3],fixed_radius:f64,rolling_radius:f64,start:f64,end:f64,budget:f64)->Result<Approximation>{
 rolling(center,fixed_radius,rolling_radius,start,end,budget,true)
}
fn rolling(center:[f64;3],fixed:f64,radius:f64,start:f64,end:f64,budget:f64,inside:bool)->Result<Approximation>{
 check(center.iter().all(|x|x.is_finite()) && fixed.is_finite() && fixed>0.
  && radius.is_finite() && radius>0. && start.is_finite() && end.is_finite()
  && end>start && budget.is_finite() && budget>0.,"Circular rolling needs finite inputs, positive radii/budget and increasing angles")?;
 check(!inside || fixed>radius,"Hypocycloid rolling circle must fit strictly inside the fixed circle")?;
 let a=if inside{fixed-radius}else{fixed+radius};let ratio=a/radius;let range=end-start;
 check(a.is_finite() && ratio.is_finite() && range.is_finite(),"Circular rolling ratio/range overflow")?;
 let mut choice=None;
 for spans in 1..=85{
  let h=range/spans as f64;let h2=h*h;let q=ratio*h;let q2=q*q;
  let estimate=(a*(h2*h2)+radius*(q2*q2))/384.*2_f64.sqrt();
  if estimate.is_finite() && estimate>0. && estimate<=budget{choice=Some((spans,estimate));break;}
 }
 let (spans,estimate)=choice.ok_or_else(||crate::input("Circular rolling estimate is unrepresentable or exceeds 85 cubic spans"))?;
 let mut points=Vec::new();let mut tangents=Vec::new();let mut parameters=Vec::new();
 let sign=if inside{1.}else{-1.};
 for i in 0..=spans{
  let t=i as f64/spans as f64;let theta=(1.-t)*start+t*end;
  let (s,c)=theta.sin_cos();let (sq,cq)=(ratio*theta).sin_cos();
  points.push([center[0]+a*c+sign*radius*cq,center[1]+a*s-radius*sq,center[2]]);
  tangents.push([(-a*s-sign*radius*ratio*sq)*range,(a*c-radius*ratio*cq)*range,0.]);parameters.push(t);
 }
 let curve=crate::hermite::interpolate_inferred(&points,&tangents,&parameters)?;
 Ok(Approximation{curve,spans,budget,real_arithmetic_error_estimate:estimate})
}
#[cfg(test)]
mod tests{
 use super::*;
 #[test]
 fn independent_inside_outside_formulas_and_cusp_tangents(){
  for inside in [false,true]{
   let a=if inside{approximate_hypocycloid([3.,4.,5.],3.,1.,0.,2.,1e-4)}else{approximate_epicycloid([3.,4.,5.],3.,1.,0.,2.,1e-4)}.unwrap();
   let rho=if inside{2.}else{4.};let sign=if inside{1.}else{-1.};
   for i in 0..=1000{
    let t=i as f64/1000.;let theta=2.*t;
    let exact=[3.+rho*theta.cos()+sign*(rho*theta).cos(),4.+rho*theta.sin()-(rho*theta).sin(),5.];
    let p=a.curve.evaluate(t).unwrap().point;
    let error=(0..3).map(|d|(p[d]-exact[d]).powi(2)).sum::<f64>().sqrt();
    assert!(error<=a.real_arithmetic_error_estimate+1e-12);
   }
   assert_eq!(a.curve.evaluate(0.).unwrap().d1.unwrap(),vec![0.;3]);
   let derivative=a.curve.evaluate(1.).unwrap().d1.unwrap();
   assert!((derivative[0]-(-rho*2_f64.sin()-sign*rho*(rho*2.).sin())*2.).abs()<1e-10);
   assert!((derivative[1]-(rho*2_f64.cos()-rho*(rho*2.).cos())*2.).abs()<1e-10);
  }
 }
 #[test]
 fn rejects_impossible_inside_radius_and_excess_frequency(){
  assert!(approximate_hypocycloid([0.;3],1.,1.,0.,2.,1e-4).is_err());
  assert!(approximate_hypocycloid([0.;3],1.,2.,0.,2.,1e-4).is_err());
  assert!(approximate_epicycloid([0.;3],3.,0.,0.,2.,1e-4).is_err());
  assert!(approximate_epicycloid([0.;3],3.,1e-6,0.,2.,1e-12).is_err());
 }
}
