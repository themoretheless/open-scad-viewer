//! Sampled boundary agreement. This is explicitly not a whole-domain certificate.
use super::{field,input,Result,Value};
use nurbs_core::surface::{Surface,SurfaceSampler};
use value_codec::json;
fn domain(s:&Surface)->([f64;2],[f64;2]) {
 ([s.knots_u[s.degree_u],s.knots_u[s.control_points.len()]],
  [s.knots_v[s.degree_v],s.knots_v[s.control_points[0].len()]])
}
fn parameters(s:&Surface,boundary:&str,t:f64)->Result<(f64,f64)> {
 let (u,v)=domain(s);let x=u[0]+t*(u[1]-u[0]);let y=v[0]+t*(v[1]-v[0]);
 match boundary {"uMin"=>Ok((u[0],y)),"uMax"=>Ok((u[1],y)),"vMin"=>Ok((x,v[0])),"vMax"=>Ok((x,v[1])),_=>Err(input("Choose uMin, uMax, vMin or vMax boundary."))}
}
pub fn measure(v:Value)->Result<Value> {
 let a:Surface=field(&v,"a")?;let b:Surface=field(&v,"b")?;
 let sa=SurfaceSampler::new(&a)?;let sb=SurfaceSampler::new(&b)?;
 let ba:String=field(&v,"boundaryA")?;let bb:String=field(&v,"boundaryB")?;
 let reverse:bool=field(&v,"reverse")?;let count:usize=field(&v,"samples")?;
 let tolerance:f64=field(&v,"toleranceMm")?;let angle_tolerance:f64=field(&v,"angleToleranceDeg")?;
 if !(2..=257).contains(&count)||!tolerance.is_finite()||tolerance<=0.||!angle_tolerance.is_finite()||!(0.0..=90.).contains(&angle_tolerance){return Err(input("Use 2..257 samples, positive distance tolerance and an angle tolerance from 0 to 90 degrees."));}
 let mut samples=Vec::new();let mut max_gap=0.0_f64;let mut max_angle=0.0_f64;let mut undefined_normals=0;let mut worst=0;
 for i in 0..count {
  let t=i as f64/(count-1) as f64;let (u,v)=parameters(&a,&ba,t)?;let x=sa.evaluate(u,v)?;
  let (u,v)=parameters(&b,&bb,if reverse{1.-t}else{t})?;let y=sb.evaluate(u,v)?;
  let gap=(x.point[0]-y.point[0]).hypot(x.point[1]-y.point[1]).hypot(x.point[2]-y.point[2]);
  if !gap.is_finite(){return Err(input("Boundary distance exceeded numeric range."));}
  if gap>max_gap {max_gap=gap;worst=i;}
  // Tangent-plane agreement is independent of the authored normal orientation.
  let angle=match (x.unit_normal(),y.unit_normal()) {
   (Some(n),Some(m))=>{let cosine=(0..3).map(|j|n[j]*m[j]).sum::<f64>().abs().clamp(0.,1.);let angle=cosine.acos().to_degrees();max_angle=max_angle.max(angle);Some(angle)},
   _=>{undefined_normals+=1;None}
  };
  samples.push(json!({"t":t,"a":x.point,"b":y.point,"gapMm":gap,"tangentPlaneAngleDeg":angle}));
 }
 Ok(json!({"samplingOnly":true,"samples":samples,"maxGapMm":max_gap,"maxTangentPlaneAngleDeg":if undefined_normals==0{Some(max_angle)}else{None},"undefinedNormals":undefined_normals,"worstGapSample":worst,"sampledWithinTolerance":max_gap<=tolerance&&undefined_normals==0&&max_angle<=angle_tolerance}))
}
#[cfg(test)]
mod tests {
 use super::*;
 fn plane(x:f64,slope:f64)->Surface {Surface{degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],control_points:vec![vec![vec![x,0.,0.],vec![x,10.,0.]],vec![vec![x+10.,0.,10.*slope],vec![x+10.,10.,10.*slope]]],weights:vec![vec![1.,1.],vec![1.,1.]],periodic_u:false,periodic_v:false}}
 fn report(a:Surface,b:Surface,reverse:bool)->Value {measure(json!({"a":a,"b":b,"boundaryA":"uMax","boundaryB":"uMin","reverse":reverse,"samples":17,"toleranceMm":0.001,"angleToleranceDeg":0.01})).unwrap()}
 #[test]fn distinguishes_gap_from_fold(){
  let aligned=report(plane(0.,0.),plane(10.,0.),false);assert_eq!(aligned["sampledWithinTolerance"],json!(true));
  let gap=report(plane(0.,0.),plane(10.25,0.),false);assert_eq!(gap["maxGapMm"],json!(0.25));assert_eq!(gap["sampledWithinTolerance"],json!(false));
  let fold=report(plane(0.,0.),plane(10.,1.),false);assert_eq!(fold["maxGapMm"],json!(0.));assert!((fold["maxTangentPlaneAngleDeg"].as_f64().unwrap()-45.).abs()<1e-10);
  let backwards=report(plane(0.,0.),plane(10.,0.),true);assert_eq!(backwards["maxGapMm"],json!(10.));
 }
 #[test]fn rejects_invalid_sampling(){assert!(measure(json!({"a":plane(0.,0.),"b":plane(10.,0.),"boundaryA":"uMax","boundaryB":"uMin","reverse":false,"samples":1,"toleranceMm":0.001,"angleToleranceDeg":0.01})).is_err());}
}
