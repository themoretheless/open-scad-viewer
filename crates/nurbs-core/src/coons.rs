//! Tensor-product Coons patch in homogeneous coordinates for compatible rational boundaries.
//! Boundary order: bottom/top in +U; left/right in +V. No endpoint snapping.
use crate::{check,Result,curve::Curve,surface::{Surface,loft_aligned}};

pub fn patch(boundaries:&[Curve])->Result<Surface> {
 check(boundaries.len()==4,"Coons patch requires bottom, top, left and right curves")?;
 let mut curves=boundaries.to_vec();
 for c in &mut curves {
  c.validate()?;
  check(c.control_points[0].len()==3&&!c.periodic,"Coons boundaries must be non-periodic 3D curves")?;
  let [a,b]=c.domain();*c=c.trim(a,b)?;
 }
 // Constant rescaling preserves each rational curve. Match homogeneous corner
 // weights around the boundary cycle; incompatible cycles require reparameterization.
 let scale=|c:&mut Curve,target:f64|{let factor=target/c.weights[0];for w in &mut c.weights{*w*=factor;}};
 scale(&mut curves[0],1.);
 let w00=curves[0].weights[0];let w10=*curves[0].weights.last().unwrap();
 scale(&mut curves[2],w00);scale(&mut curves[3],w10);
 let w01=*curves[2].weights.last().unwrap();scale(&mut curves[1],w01);
 check(curves[1].weights.last()==curves[3].weights.last(),"Coons corner weights are incompatible; reparameterize the boundaries")?;
 let endpoint=|c:&Curve,end:usize|->Result<Vec<f64>>{Ok(c.evaluate(c.domain()[end])?.point)};
 let corners=[endpoint(&curves[0],0)?,endpoint(&curves[0],1)?,endpoint(&curves[1],0)?,endpoint(&curves[1],1)?];
 let other=[endpoint(&curves[2],0)?,endpoint(&curves[3],0)?,endpoint(&curves[2],1)?,endpoint(&curves[3],1)?];
 for i in 0..4 {check(corners[i]==other[i],&format!("Coons corner {} does not coincide; connect or reverse the boundary curves",i+1))?;}
 let corner_weights=[curves[0].weights[0],*curves[0].weights.last().unwrap(),curves[1].weights[0],*curves[1].weights.last().unwrap()];
 let corners:Vec<Vec<f64>>=corners.iter().zip(corner_weights).map(|(p,w)|vec![p[0]*w,p[1]*w,p[2]*w,w]).collect();
 let u=loft_aligned(&curves[..2])?;let v=loft_aligned(&curves[2..])?;
 let nu=u.control_points.len();let nv=v.control_points.len();
 let greville=|knots:&[f64],degree:usize,i:usize|knots[i+1..=i+degree].iter().sum::<f64>()/degree as f64;
 let homogeneous:Vec<Vec<Vec<f64>>>=(0..nu).map(|i|{
  let x=greville(&u.knots_u,u.degree_u,i);
  (0..nv).map(|j|{
   let y=greville(&v.knots_u,v.degree_u,j);
   (0..4).map(|k|{
    let h=|s:&Surface,i:usize,j:usize|if k==3{s.weights[i][j]}else{s.control_points[i][j][k]*s.weights[i][j]};
    (1.-y)*h(&u,i,0)+y*h(&u,i,1)
    +(1.-x)*h(&v,j,0)+x*h(&v,j,1)
    -((1.-x)*(1.-y)*corners[0][k]+x*(1.-y)*corners[1][k]+(1.-x)*y*corners[2][k]+x*y*corners[3][k])
   }).collect()
  }).collect()
 }).collect();
 check(homogeneous.iter().flatten().all(|h|h.iter().all(|v|v.is_finite())&&h[3]>0.),"Coons patch cannot certify positive finite weights")?;
 let weights=homogeneous.iter().map(|row|row.iter().map(|h|h[3]).collect()).collect();
 let control_points=homogeneous.iter().map(|row|row.iter().map(|h|(0..3).map(|k|h[k]/h[3]).collect()).collect()).collect();
 let surface=Surface{degree_u:u.degree_u,degree_v:v.degree_u,knots_u:u.knots_u,knots_v:v.knots_u,control_points,weights,periodic_u:false,periodic_v:false};
 surface.validate()?;Ok(surface)
}
#[cfg(test)]
mod tests {
 use super::*;
 fn curve(points:Vec<Vec<f64>>)->Curve{let p=points.len()-1;Curve{degree:p,knots:[vec![0.;p+1],vec![1.;p+1]].concat(),weights:vec![1.;p+1],control_points:points,periodic:false}}
 fn boundaries()->Vec<Curve>{vec![
 curve(vec![vec![0.,0.,0.],vec![1.,0.,1.],vec![2.,0.,0.]]),
 curve(vec![vec![0.,2.,0.],vec![2.,2.,0.]]),
 curve(vec![vec![0.,0.,0.],vec![0.,2.,0.]]),
 curve(vec![vec![2.,0.,0.],vec![2.,1.,2.],vec![2.,2.,0.]])
 ]}
 #[test]fn reproduces_all_four_boundaries_and_analytic_interior(){
  let mut c=boundaries();c[0]=c[0].insert(0.3,1).unwrap();let s=patch(&c).unwrap();
  for i in 0..=20 {let t=i as f64/20.;for (edge,(u,v)) in [(t,0.),(t,1.),(0.,t),(1.,t)].into_iter().enumerate(){let a=s.evaluate(u,v).unwrap().point;let b=c[edge].evaluate(t).unwrap().point;for k in 0..3{assert!((a[k]-b[k]).abs()<1e-12);}}}
  for i in 1..10{for j in 1..10{let u=i as f64/10.;let v=j as f64/10.;let p=s.evaluate(u,v).unwrap().point;let z=(1.-v)*2.*u*(1.-u)+u*4.*v*(1.-v);assert!((p[0]-2.*u).abs()<1e-12);assert!((p[1]-2.*v).abs()<1e-12);assert!((p[2]-z).abs()<1e-12);}}
 }
 #[test]fn rational_quarter_cylinder_preserves_boundaries_and_radius(){
  let mut bottom=curve(vec![vec![1.,0.,0.],vec![1.,1.,0.],vec![0.,1.,0.]]);
  bottom.weights=vec![1.,std::f64::consts::FRAC_1_SQRT_2,1.];
  let mut top=bottom.clone();for p in &mut top.control_points{p[2]=2.;}
  let left=curve(vec![vec![1.,0.,0.],vec![1.,0.,2.]]);
  let right=curve(vec![vec![0.,1.,0.],vec![0.,1.,2.]]);
  let boundaries=vec![bottom,top,left,right];let s=patch(&boundaries).unwrap();
  for i in 0..=32{let t=i as f64/32.;
   for(edge,(u,v))in [(t,0.),(t,1.),(0.,t),(1.,t)].into_iter().enumerate(){
    let p=s.evaluate(u,v).unwrap().point;let q=boundaries[edge].evaluate(t).unwrap().point;
    for k in 0..3{assert!((p[k]-q[k]).abs()<1e-12);}
   }
   for j in 0..=16{let v=j as f64/16.;let p=s.evaluate(t,v).unwrap().point;
    assert!((p[0]*p[0]+p[1]*p[1]-1.).abs()<1e-12);assert!((p[2]-2.*v).abs()<1e-12);
   }
  }
  let mut scaled=boundaries.clone();for(c,f)in scaled.iter_mut().zip([2.,4.,8.,16.]){for w in &mut c.weights{*w*=f;}}
  let same=patch(&scaled).unwrap();assert_eq!(same.evaluate(0.3,0.7).unwrap().point,s.evaluate(0.3,0.7).unwrap().point);
 }
 #[test]fn refuses_gaps_incompatible_weights_and_wrong_roles(){
  let mut c=boundaries();c[2].control_points[0][0]=0.001;assert!(patch(&c).unwrap_err().to_string().contains("corner"));
  let mut c=boundaries();c[0].weights[2]=0.5;assert!(patch(&c).unwrap_err().to_string().contains("corner weights"));
  assert!(patch(&boundaries()[..3]).is_err());
 }
}
