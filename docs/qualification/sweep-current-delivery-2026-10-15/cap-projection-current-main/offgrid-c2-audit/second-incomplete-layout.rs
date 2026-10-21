use nurbs_core::{curve::Curve,progressive_sweep::{Sweep,Options,Orientation,Spacing,constant_vector_law}};
fn cross(a:[f64;3],b:[f64;3])->[f64;3]{[a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]]}
fn unit(a:[f64;3])->[f64;3]{let n=a[0].hypot(a[1]).hypot(a[2]);assert!(n>0.);a.map(|v|v/n)}
fn main(){
 let profile=nurbs_core::primitives::line([0.,0.1,0.],[0.,0.2,0.]).unwrap();let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
 for knot in [0.375,0.484375,0.37]{
  let path=Curve{degree:3,knots:vec![0.,0.,0.,0.,knot,1.,1.,1.,1.],control_points:vec![vec![0.,0.,0.],vec![0.16666666666666666,0.,0.],vec![0.5,0.16666666666666666,0.],vec![0.8333333333333334,0.6666666666666666,0.5],vec![1.,1.,1.125]],weights:vec![1.;5],periodic:false};
  for spacing in [Spacing::Parameter,Spacing::ArcLength{tolerance:0.001,max_cells:100000}]{
   let sweep=Sweep::new(&profile,&path,&scale,&twist,Options{orientation:Orientation::CorrectedFrenet,normal:[0.,0.,1.],spacing,initial_sections:3,max_sections:129,max_deviation:0.05}).unwrap();
   for count in [17,65]{
    let level=sweep.preview_at(count).unwrap();let r=&level.report;assert!(r.continuous_bound,"knot={knot}/{spacing:?}/{count}: {r:?}");let bound=r.continuous_error_upper.unwrap();
    let mut maximum=0_f64;
    if spacing==Spacing::Parameter{
     assert_eq!(level.patches.len(),1);
     for i in 0..=2048{
      let t=i as f64/2048.;let value=path.evaluate(t).unwrap();let v=value.d1.unwrap();let a=value.d2.unwrap();let tangent=unit([v[0],v[1],v[2]]);let binormal=unit(cross([v[0],v[1],v[2]],[a[0],a[1],a[2]]));let normal=cross(binormal,tangent);
      
      for (u,radius) in [(0.,0.1),(1.,0.2)]{let retained=level.patches[0].evaluate(u,t).unwrap().point;let delta=std::array::from_fn::<_,3,_>(|k|retained[k]-(value.point[k]+radius*normal[k]));maximum=maximum.max(delta[0].hypot(delta[1]).hypot(delta[2]));}
     }
     assert!(maximum<=bound,"Independent principal-frame sample violates claimed upper: knot={knot},count={count},observed={maximum},upper={bound}");
    }
    println!("knot={knot},spacing={spacing:?},count={count},continuous={},accepted={},upper={bound},independent_parameter_sample_max={maximum},cells={}",r.continuous_bound,r.accepted,r.error_certificate_cells);
   }
  }
 }
}
