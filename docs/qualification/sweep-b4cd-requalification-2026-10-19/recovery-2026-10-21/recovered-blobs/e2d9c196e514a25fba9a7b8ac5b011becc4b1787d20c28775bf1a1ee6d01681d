//! Three-axis harmonic trajectories with explicit ideal Hermite remainder.
use crate::{Result,check};
pub use crate::helix::Approximation;
/// P_d(t)=center_d+amplitude_d*sin(phase_d+2*pi*frequency_d*t).
/// Frequencies are signed cycles on normalized t; phases are degrees.
pub fn approximate(center:[f64;3],amplitudes:[f64;3],frequencies:[f64;3],phases:[f64;3],budget:f64)->Result<Approximation>{
 check(center.iter().all(|x|x.is_finite()) && amplitudes.iter().all(|x|x.is_finite() && *x>=0.)
  && frequencies.iter().chain(&phases).all(|x|x.is_finite()) && budget.is_finite() && budget>0.,
  "Lissajous requires finite inputs, nonnegative amplitudes and positive budget")?;
 check((0..3).any(|d|amplitudes[d]>0. && frequencies[d]!=0.),"Lissajous must have at least one varying axis")?;
 let omega=frequencies.map(|x|2.*std::f64::consts::PI*x);
 check(omega.iter().all(|x|x.is_finite()),"Lissajous frequency overflow")?;
 let phase=phases.map(|x|x.rem_euclid(360.).to_radians());
 let mut choice=None;
 for spans in 1..=85{
  let mut bound=0_f64;
  for d in 0..3{
   if amplitudes[d]==0. || omega[d]==0.{continue;}
   let step=omega[d]/spans as f64;let squared=step*step;
   bound=bound.hypot(amplitudes[d]*(squared*squared)/384.);
  }
  if bound.is_finite() && bound>0. && bound<=budget{choice=Some((spans,bound));break;}
 }
 let (spans,estimate)=choice.ok_or_else(||crate::input("Lissajous estimate is unrepresentable or exceeds 85 cubic spans"))?;
 let mut points=Vec::new();let mut tangents=Vec::new();let mut parameters=Vec::new();
 for i in 0..=spans{
  let t=i as f64/spans as f64;let mut p=center;let mut tangent=[0.;3];
  for d in 0..3{
   if amplitudes[d]==0.{continue;}
   let (sin,cos)=(phase[d]+omega[d]*t).sin_cos();
   p[d]+=amplitudes[d]*sin;tangent[d]=amplitudes[d]*omega[d]*cos;
  }
  points.push(p);tangents.push(tangent);parameters.push(t);
 }
 let curve=crate::hermite::interpolate_inferred(&points,&tangents,&parameters)?;
 Ok(Approximation{curve,spans,budget,real_arithmetic_error_estimate:estimate})
}
#[cfg(test)]
mod tests{
 use super::*;
 #[test]
 fn independent_three_axis_samples_and_endpoint_derivatives(){
  for frequencies in [[1.,2.,3.],[-1.,0.5,0.],[0.,-2.,1.5]]{
   let amplitudes=[2.,3.,4.];let phases=[23.,47.,91.];
   let a=approximate([3.,4.,5.],amplitudes,frequencies,phases,1e-4).unwrap();
   for i in 0..=1000{
    let t=i as f64/1000.;let p=a.curve.evaluate(t).unwrap().point;
    let exact=std::array::from_fn::<_,3,_>(|d|[3.,4.,5.][d]+amplitudes[d]*(phases[d].to_radians()+2.*std::f64::consts::PI*frequencies[d]*t).sin());
    let error=(0..3).map(|d|(p[d]-exact[d]).powi(2)).sum::<f64>().sqrt();
    assert!(error<=a.real_arithmetic_error_estimate+1e-12);
   }
   for t in [0.,1.]{
    let d1=a.curve.evaluate(t).unwrap().d1.unwrap();
    for d in 0..3{
     let w=2.*std::f64::consts::PI*frequencies[d];
     assert!((d1[d]-amplitudes[d]*w*(phases[d].to_radians()+w*t).cos()).abs()<1e-10);
    }
   }
  }
 }
 #[test]
 fn refuses_constant_negative_amplitude_and_unattainable_budget(){
  assert!(approximate([0.;3],[1.;3],[0.;3],[0.;3],1e-4).is_err());
  assert!(approximate([0.;3],[-1.,2.,3.],[1.;3],[0.;3],1e-4).is_err());
  assert!(approximate([0.;3],[1.;3],[100.;3],[0.;3],1e-12).is_err());
  let a=approximate([0.;3],[1.;3],[1.;3],[0.;3],1e-2).unwrap();
  let b=approximate([0.;3],[1.;3],[1.;3],[0.;3],1e-5).unwrap();assert!(b.spans>a.spans);
 }
}
