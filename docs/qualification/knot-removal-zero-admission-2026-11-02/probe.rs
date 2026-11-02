use nurbs_core::{curve::Curve,foundation::knot_removal::assess_knot_removal};
fn main(){
 let base=Curve{degree:3,knots:vec![0.,0.,0.,0.,1.,2.,2.,2.,2.],control_points:vec![vec![0.,0.],vec![1.,2.],vec![2.,2.],vec![3.,0.],vec![4.,0.]],weights:vec![1.;5],periodic:false};
 for delta in [0.,1e-10,1e-8] {
  let mut refined=base.insert(0.5,1).unwrap();refined.control_points[2][1]+=delta;
  println!("source_knots={:?}",refined.knots);println!("source_controls={:?}",refined.control_points);
  let r=assess_knot_removal(&refined,0.5,4096).unwrap();
  println!("delta={delta:e} reported_tolerance={:e} control_delta={:e} feasible={}",r.removable_at_tolerance,r.control_delta_norm,r.feasible);
 }
}
