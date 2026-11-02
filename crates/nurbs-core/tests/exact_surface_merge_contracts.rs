use nurbs_core::{surface::{Surface,Axis},surface_edit,editing::{exact_surface_merge::{merge,Refusal},exact_surface_knot_removal::simplify}};
fn base()->Surface {
 Surface{degree_u:2,degree_v:2,knots_u:vec![2.,2.,2.,6.,6.,6.],knots_v:vec![-3.,-3.,-3.,1.,1.,1.],control_points:(0..3).map(|i|(0..3).map(|j|vec![i as f64,j as f64,[0.,2.,0.][i]+[0.,2.,0.][j]]).collect()).collect(),weights:vec![vec![1.;3];3],periodic_u:false,periodic_v:false}
}
#[test]
fn both_axes_merge_absolute_rectangles_and_simplify_to_original_law(){
 for (axis,knot) in [(Axis::U,4.),(Axis::V,-1.)]{
  let source=base();let [a,b]=surface_edit::split(&source,axis,knot).unwrap();let saved=(a.clone(),b.clone());
  let report=merge(&a,&b,axis).unwrap();let output=report.surface.unwrap();assert!(report.refusal.is_none());assert_eq!((a,b),saved);
  let knots=match axis{Axis::U=>&output.knots_u,Axis::V=>&output.knots_v};assert_eq!(knots.iter().filter(|&&u|u==knot).count(),2);
  for i in 0..=16{for j in 0..=16{let u=i as f64/16.;let v=j as f64/16.;let point=output.evaluate(2.+4.*u,-3.+4.*v).unwrap().point;let law=[2.*u,2.*v,4.*u*(1.-u)+4.*v*(1.-v)];for k in 0..3{assert!((point[k]-law[k]).abs()<1e-13);}}}
  let compact=simplify(&output,axis,1_000_000).unwrap();assert!(compact.completed_pass);assert_eq!(compact.removed,2);assert_eq!(compact.surface,source);
 }
}
fn rational()->(Surface,Surface){
 let make=|domain:[f64;2],x:[f64;2],weights:[f64;2]|Surface{degree_u:1,degree_v:1,knots_u:vec![domain[0],domain[0],domain[1],domain[1]],knots_v:vec![0.,0.,1.,1.],control_points:x.iter().map(|&x|vec![vec![x,0.,0.],vec![x,1.,0.]]).collect(),weights:weights.iter().map(|&w|vec![w,3.*w]).collect(),periodic_u:false,periodic_v:false};
 (make([-2.,2.],[0.,1.],[1.,2.]),make([2.,8.],[1.,3.],[2.,4.]))
}
#[test]
fn rational_tensor_keeps_two_original_clocks_without_normalization(){
 let (a,b)=rational();let out=merge(&a,&b,Axis::U).unwrap().surface.unwrap();assert_eq!(out.knots_u,vec![-2.,-2.,2.,8.,8.]);assert_eq!(out.weights,vec![vec![1.,3.],vec![2.,6.],vec![4.,12.]]);
 for i in 0..=16{for j in 0..=16{let t=i as f64/16.;let v=j as f64/16.;for (u,x) in [(-2.+4.*t,2.*t/(1.+t)),(2.+6.*t,(1.+5.*t)/(1.+t))]{let p=out.evaluate(u,v).unwrap().point;assert!((p[0]-x).abs()<1e-13);assert!((p[1]-3.*v/(1.+2.*v)).abs()<1e-13);}}}
}
#[test]
fn equal_boundary_image_with_different_gauge_or_one_ulp_is_refused(){
 let (a,b)=rational();let mut changed=b.clone();for row in &mut changed.weights{for w in row{*w*=2.;}}
 for right in [changed,{let mut s=b.clone();s.control_points[0][1][0]=1_f64.next_up();s}]{let r=merge(&a,&right,Axis::U).unwrap();assert!(r.surface.is_none());assert!(matches!(r.refusal,Some(Refusal::SeamMismatch{..})));}
 assert!(merge(&a,&b,Axis::U).unwrap().surface.is_some());
}
#[test]
fn incompatible_domains_bases_and_nonclamped_storage_are_explicit(){
 let (a,b)=rational();let mut gap=b.clone();gap.knots_u=vec![3.,3.,8.,8.];assert_eq!(merge(&a,&gap,Axis::U).unwrap().refusal,Some(Refusal::NonAdjacentDomain));
 let mut basis=b.clone();basis.knots_v=vec![0.,0.,2.,2.];assert_eq!(merge(&a,&basis,Axis::U).unwrap().refusal,Some(Refusal::OtherBasisMismatch));
 let mut unclamped=b.clone();unclamped.knots_u=vec![0.,2.,8.,9.];unclamped.validate().unwrap();assert_eq!(merge(&a,&unclamped,Axis::U).unwrap().refusal,Some(Refusal::NonClamped));
 let elevated=surface_edit::elevate(&b,Axis::U,2).unwrap();assert_eq!(merge(&a,&elevated,Axis::U).unwrap().refusal,Some(Refusal::DegreeMismatch));
 let mut malformed=b;malformed.weights[0][0]=f64::NAN;assert!(merge(&a,&malformed,Axis::U).is_err());
}
#[test]
fn unedited_periodic_storage_is_retained_without_a_new_quotient_claim(){
 let make=|u:f64|Surface{degree_u:1,degree_v:1,knots_u:vec![u,u,u+1.,u+1.],knots_v:vec![-1.,0.,1.,2.,3.],control_points:[u,u+1.].iter().map(|&x|vec![vec![x,0.,0.],vec![x,1.,0.],vec![x,0.,0.]]).collect(),weights:vec![vec![1.;3];2],periodic_u:false,periodic_v:true};
 let a=make(0.);let b=make(1.);let output=merge(&a,&b,Axis::U).unwrap().surface.unwrap();assert!(output.periodic_v);assert_eq!(output.knots_v,a.knots_v);assert_eq!(output.control_points.len(),3);
 assert_eq!(merge(&a,&a,Axis::V).unwrap().refusal,Some(Refusal::SelectedPeriodic));
}
#[test]
fn merging_does_not_raise_the_existing_control_limit(){
 let make=|start:usize|Surface{degree_u:1,degree_v:1,knots_u:[vec![start as f64;2],(start+1..start+129).map(|x|x as f64).collect(),vec![(start+129) as f64;2]].concat(),knots_v:vec![0.,0.,1.,1.],control_points:(start..start+130).map(|x|vec![vec![x as f64,0.,0.],vec![x as f64,1.,0.]]).collect(),weights:vec![vec![1.;2];130],periodic_u:false,periodic_v:false};
 let a=make(0);let b=make(129);a.validate().unwrap();b.validate().unwrap();let r=merge(&a,&b,Axis::U).unwrap();assert!(r.surface.is_none());assert_eq!(r.refusal,Some(Refusal::ControlLimit));
}
