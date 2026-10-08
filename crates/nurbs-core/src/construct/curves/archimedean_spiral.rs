//! Planar linear-radius spiral using the shared angle/radius Hermite generator.
use crate::{Result,check};
pub use crate::helix::Approximation;
/// Radius is linear in angle between authored endpoint radii; angles are degrees.
/// The curve uses normalized t in [0,1] and remains explicitly approximate.
pub fn approximate(center:[f64;3],start_radius:f64,end_radius:f64,start_degrees:f64,end_degrees:f64,budget:f64)->Result<Approximation>{
 check(start_degrees.is_finite() && end_degrees.is_finite() && end_degrees>start_degrees,
  "Archimedean spiral angles must be finite and strictly increasing")?;
 let sweep=end_degrees-start_degrees;
 check(sweep.is_finite(),"Archimedean spiral angular range overflow")?;
 crate::helix::approximate_conical(center,start_radius,end_radius,0.,sweep/360.,start_degrees,budget)
}
#[cfg(test)]
mod tests{
 use super::*;
 #[test]
 fn independent_growing_shrinking_and_origin_endpoint_formulas(){
  for (start,end) in [(1.,5.),(5.,0.),(0.,5.)]{
   let a=approximate([3.,4.,5.],start,end,23.,743.,1e-4).unwrap();
   for i in 0..=1000{
    let t=i as f64/1000.;let angle=(23.+720.*t).to_radians();let radius=start+(end-start)*t;
    let p=a.curve.evaluate(t).unwrap().point;let exact=[3.+radius*angle.cos(),4.+radius*angle.sin(),5.];
    let error=(0..3).map(|d|(p[d]-exact[d]).powi(2)).sum::<f64>().sqrt();
    assert!(error<=a.real_arithmetic_error_estimate+1e-12);
   }
   for (t,radius) in [(0.,start),(1.,end)]{
    let angle=(23_f64+720.*t).to_radians();let omega=4.*std::f64::consts::PI;let slope=end-start;
    let d=a.curve.evaluate(t).unwrap().d1.unwrap();
    assert!((d[0]-(slope*angle.cos()-radius*omega*angle.sin())).abs()<1e-10);
    assert!((d[1]-(slope*angle.sin()+radius*omega*angle.cos())).abs()<1e-10);
   }
  }
 }
 #[test]
 fn rejects_reversed_ranges_negative_radii_and_unattainable_budget(){
  assert!(approximate([0.;3],1.,2.,30.,0.,1e-4).is_err());
  assert!(approximate([0.;3],-1.,2.,0.,30.,1e-4).is_err());
  assert!(approximate([0.;3],0.,0.,0.,30.,1e-4).is_err());
  assert!(approximate([0.;3],1.,2.,0.,36000.,1e-12).is_err());
 }
}
