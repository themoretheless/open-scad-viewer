use nurbs_core::{surface::{Surface,Axis},editing::{exact_surface_degree_reduction::{reduce,Refusal},exact_degree_reduction}};
use cad_predicates::BezierIdentity;
fn polynomial()->Surface {
 let x=[0.,2.,4.,6.];let z=[0.,4.,4.,0.];
 Surface{degree_u:3,degree_v:3,knots_u:vec![2.,2.,2.,2.,5.,5.,5.,5.],knots_v:vec![-3.,-3.,-3.,-3.,1.,1.,1.,1.],control_points:(0..4).map(|i|(0..4).map(|j|vec![x[i],x[j],z[i]+z[j]]).collect()).collect(),weights:vec![vec![1.;4];4],periodic_u:false,periodic_v:false}
}
#[test]
fn both_axes_reduce_the_true_bicubic_quadratic_law_and_keep_nonunit_uv(){
 let source=polynomial();let u=reduce(&source,Axis::U,1_000_000).unwrap();assert!(u.reduced,"{:?}",u.refusal);assert_eq!(u.surface.degree_u,2);assert_eq!(u.rows_verified,4);
 let r=reduce(&u.surface,Axis::V,1_000_000).unwrap();assert!(r.reduced,"{:?}",r.refusal);assert_eq!(r.surface.degree_v,2);assert_eq!(r.surface.control_points.len(),3);assert_eq!(r.surface.control_points[0].len(),3);
 for i in 0..=8{for j in 0..=8{let u=i as f64/8.;let v=j as f64/8.;let p=r.surface.evaluate(2.+3.*u,-3.+4.*v).unwrap().point;let expected=[6.*u,6.*v,12.*u*(1.-u)+12.*v*(1.-v)];for axis in 0..3{assert!((p[axis]-expected[axis]).abs()<1e-12);}}}
 assert_eq!(source,polynomial());
}
#[test]
fn rational_tensor_keeps_nonconstant_denominator_and_parameter_clock(){
 let mut source=polynomial();let x=[0.,8.,7.,3.];let weights=[1.,3.,4.,4.];
 for i in 0..4{for j in 0..4{source.control_points[i][j][0]=x[i];source.control_points[i][j][2]=0.;source.weights[i][j]=weights[i];}}
 let r=reduce(&source,Axis::U,1_000_000).unwrap();assert!(r.reduced,"{:?}",r.refusal);assert_eq!(r.surface.weights,vec![vec![1.;4],vec![4.;4],vec![4.;4]]);
 for i in 0..=16{let t=i as f64/16.;let p=r.surface.evaluate(2.+3.*t,-2.).unwrap().point;assert!((p[0]-(72.*t-60.*t*t)/(1.+6.*t-3.*t*t)).abs()<1e-12);assert!((p[1]-1.5).abs()<1e-12);}
}
#[test]
fn same_constant_rows_cannot_hide_a_changed_tensor_gauge(){
 let source=Surface{degree_u:2,degree_v:1,knots_u:vec![0.,0.,0.,1.,1.,1.],knots_v:vec![0.,0.,1.,1.],control_points:vec![vec![vec![0.,0.,0.],vec![0.,1.,0.]];3],weights:vec![vec![1.,1.],vec![2.,1.],vec![1.,1.]],periodic_u:false,periodic_v:false};
 let curve=nurbs_core::curve::Curve{degree:2,knots:source.knots_u.clone(),control_points:vec![vec![0.,0.,0.];3],weights:vec![1.,2.,1.],periodic:false};assert!(exact_degree_reduction::reduce(&curve,1_000_000).unwrap().reduced);
 assert!((source.evaluate(0.5,0.5).unwrap().point[1]-0.4).abs()<1e-14);
 let r=reduce(&source,Axis::U,1_000_000).unwrap();assert!(!r.reduced);assert_eq!(r.surface,source);assert_eq!(r.refusal,Some(Refusal::Denominator{row:0,decision:BezierIdentity::Different}));
}
#[test]
fn last_row_failure_and_whole_operation_budget_always_roll_back(){
 let source=polynomial();let full=reduce(&source,Axis::U,1_000_000).unwrap();assert!(full.reduced);
 for budget in [1,full.work_used-1]{let r=reduce(&source,Axis::U,budget).unwrap();assert!(!r.reduced);assert_eq!(r.surface,source);assert!(r.work_used<=budget);}
 assert_eq!(reduce(&source,Axis::U,full.work_used).unwrap().surface,full.surface);
 let mut modified=source.clone();modified.control_points[1][3][2]+=1e-10;let r=reduce(&modified,Axis::U,1_000_000).unwrap();assert!(!r.reduced);assert_eq!(r.rows_verified,3);assert_eq!(r.surface,modified);
 for budget in [0,1_000_001]{assert!(reduce(&source,Axis::U,budget).is_err());}
}
#[test]
fn unedited_periodic_axis_is_preserved_and_selected_periodic_axis_refuses(){
 let mut source=polynomial();let y=[0.,2.,-2.,0.,2.];let x=[0.,2.,4.,6.];let z=[0.,4.,4.,0.];source.degree_v=2;source.knots_v=vec![-2.,-1.,0.,1.,2.,3.,4.,5.];source.periodic_v=true;source.control_points=(0..4).map(|i|y.iter().map(|&v|vec![x[i],v,z[i]]).collect()).collect();source.weights=vec![vec![1.;5];4];source.validate().unwrap();
 let r=reduce(&source,Axis::U,1_000_000).unwrap();assert!(r.reduced,"{:?}",r.refusal);assert_eq!(r.surface.knots_v,source.knots_v);assert!(r.surface.periodic_v);assert_eq!(r.surface.degree_v,2);assert!(reduce(&source,Axis::V,1_000_000).is_err());
}
#[test]
fn maximum_degree_refusals_do_not_reset_the_tensor_budget(){
 let source=Surface{degree_u:25,degree_v:1,knots_u:[vec![0.;26],vec![1.;26]].concat(),knots_v:vec![0.,0.,1.,1.],control_points:(0..26).map(|i|vec![vec![i as f64,0.,0.],vec![i as f64,1.,0.]]).collect(),weights:vec![vec![1.;2];26],periodic_u:false,periodic_v:false};let r=reduce(&source,Axis::U,1_000_000).unwrap();assert!(r.work_used<=1_000_000);if !r.reduced{assert_eq!(r.surface,source);assert!(r.refusal.is_some());}
}
#[test]
fn piecewise_bezier_axis_retains_unequal_span_boundaries(){
 let x=[0.,2.,4.,6.,8.,10.,12.];let z=[0.,4.,4.,0.,-4.,-4.,0.];let source=Surface{degree_u:3,degree_v:1,knots_u:[vec![2.;4],vec![5.;3],vec![9.;4]].concat(),knots_v:vec![-3.,-3.,1.,1.],control_points:(0..7).map(|i|vec![vec![x[i],0.,z[i]],vec![x[i],2.,z[i]]]).collect(),weights:vec![vec![1.;2];7],periodic_u:false,periodic_v:false};let r=reduce(&source,Axis::U,1_000_000).unwrap();assert!(r.reduced,"{:?}",r.refusal);assert_eq!(r.surface.knots_u,[vec![2.;3],vec![5.;2],vec![9.;3]].concat());let p=r.surface.evaluate(7.,-1.).unwrap().point;assert_eq!(p,[9.,1.,-3.]);
}

#[test]
fn smooth_axis_extraction_and_compaction_preserve_complete_original_tensor(){
 use nurbs_core::{surface_edit,editing::exact_surface_degree_reduction::reduce_smooth};
 let original=polynomial();
 for (axis,knot) in [(Axis::U,3.5),(Axis::V,-1.)]{
  let source=surface_edit::insert(&original,axis,knot,1).unwrap();
  let r=reduce_smooth(&source,axis,1_000_000).unwrap();assert!(r.reduced,"{:?}",r.refusal);
  assert_eq!(r.surface,reduce(&original,axis,1_000_000).unwrap().surface);
  assert!(reduce_smooth(&source,axis,r.work_used).unwrap().reduced);
  let interrupted=reduce_smooth(&source,axis,r.work_used-1).unwrap();assert!(!interrupted.reduced);assert_eq!(interrupted.surface,source);assert!(interrupted.work_used<r.work_used);
 }
}
