use nurbs_core::{surface::{Surface,Axis},surface_edit,editing::{exact_knot_removal as curve_edit,exact_surface_knot_removal::{remove,Refusal}}};
use cad_predicates::BezierIdentity;
fn base()->Surface {
    Surface{degree_u:2,degree_v:2,knots_u:vec![2.,2.,2.,6.,6.,6.],knots_v:vec![-3.,-3.,-3.,1.,1.,1.],
        control_points:(0..3).map(|i|(0..3).map(|j|vec![i as f64,j as f64,[0.,2.,0.][i]+[0.,2.,0.][j]]).collect()).collect(),
        weights:vec![vec![1.;3];3],periodic_u:false,periodic_v:false}
}
#[test]
fn removes_both_axes_and_keeps_original_uv_and_analytic_polynomial() {
    let original=base();let source=surface_edit::insert(&original,Axis::U,4.,1).unwrap();let source=surface_edit::insert(&source,Axis::V,-1.,1).unwrap();
    let u=remove(&source,Axis::U,4.,1_000_000).unwrap();assert!(u.removed,"{:?}",u.refusal);assert_eq!(u.rows_verified,4);
    let result=remove(&u.surface,Axis::V,-1.,1_000_000).unwrap();assert!(result.removed,"{:?}",result.refusal);assert_eq!(result.surface,original);
    for i in 0..=8 {for j in 0..=8 {let u=i as f64/8.;let v=j as f64/8.;let p=result.surface.evaluate(2.+4.*u,-3.+4.*v).unwrap().point;let expected=[2.*u,2.*v,4.*u*(1.-u)+4.*v*(1.-v)];for k in 0..3 {assert!((p[k]-expected[k]).abs()<1e-13);}}}
}
#[test]
fn rational_rows_and_large_literal_weights_preserve_exact_tensor() {
    let mut original=base();
    for (i,row) in original.weights.iter_mut().enumerate() {row.fill([1e10,3e10,1e10][i]);}
    for (i,row) in original.control_points.iter_mut().enumerate() {for p in row {p[0]*=2.;p[2]=[0.,4.,0.][i];}}
    let source=surface_edit::insert(&original,Axis::U,4.,1).unwrap();let r=remove(&source,Axis::U,4.,1_000_000).unwrap();assert!(r.removed,"{:?}",r.refusal);assert_eq!(r.surface,original);
    for i in 0..=16 {let t=i as f64/16.;let w=1.+4.*t-4.*t*t;let p=r.surface.evaluate(2.+4.*t,-2.).unwrap().point;
        assert!((p[0]-(12.*t-8.*t*t)/w).abs()<1e-13);assert!((p[1]-0.5).abs()<1e-13);assert!((p[2]-24.*t*(1.-t)/w).abs()<1e-13);}
}
#[test]
fn constant_row_identity_cannot_hide_changed_tensor_denominator() {
    let source=Surface{degree_u:1,degree_v:1,knots_u:vec![0.,0.,0.5,1.,1.],knots_v:vec![0.,0.,1.,1.],
        control_points:vec![vec![vec![0.,0.,0.],vec![0.,1.,0.]];3],weights:vec![vec![1.,1.],vec![2.,1.],vec![1.,1.]],periodic_u:false,periodic_v:false};
    let row=nurbs_core::curve::Curve{degree:1,knots:source.knots_u.clone(),control_points:vec![vec![0.,0.,0.];3],weights:vec![1.,2.,1.],periodic:false};
    assert!(curve_edit::remove(&row,0.5,1_000_000).unwrap().removed);
    let p=source.evaluate(0.5,0.5).unwrap().point;assert!((p[1]-1./3.).abs()<1e-14);
    let r=remove(&source,Axis::U,0.5,1_000_000).unwrap();assert!(!r.removed);assert_eq!(r.surface,source);
    assert_eq!(r.refusal,Some(Refusal::Denominator{row:0,decision:BezierIdentity::Different}));
}
#[test]
fn late_row_failure_is_atomic_and_tiny_changes_are_not_snapped() {
    let mut source=surface_edit::insert(&base(),Axis::U,4.,1).unwrap();source.control_points[1][2][2]+=1e-10;
    let r=remove(&source,Axis::U,4.,1_000_000).unwrap();assert!(!r.removed);assert_eq!(r.rows_verified,2);assert_eq!(r.surface,source);assert!(matches!(r.refusal,Some(Refusal::Curve{row:2,..})));
}
#[test]
fn one_budget_spans_all_rows_and_both_proofs() {
    let source=surface_edit::insert(&base(),Axis::U,4.,1).unwrap();let full=remove(&source,Axis::U,4.,1_000_000).unwrap();assert!(full.removed);
    for budget in [1,full.work_used-1] {let r=remove(&source,Axis::U,4.,budget).unwrap();assert!(!r.removed);assert_eq!(r.surface,source);assert!(r.work_used<=budget);}
    assert_eq!(remove(&source,Axis::U,4.,full.work_used).unwrap().surface,base());
    for budget in [0,1_000_001] {assert!(remove(&source,Axis::U,4.,budget).is_err());}
    for knot in [2.,6.,3.,f64::NAN] {assert!(remove(&source,Axis::U,knot,1_000_000).is_err());}
}
#[test]
fn refuses_selected_periodic_axis_but_preserves_unedited_periodic_storage() {
    let periodic=Surface{degree_u:1,degree_v:1,knots_u:vec![-1.,0.,1.,2.,3.],knots_v:vec![0.,0.,1.,1.],
        control_points:vec![vec![vec![0.,0.,0.],vec![0.,0.,1.]],vec![vec![1.,0.,0.],vec![1.,0.,1.]],vec![vec![0.,0.,0.],vec![0.,0.,1.]]],weights:vec![vec![1.;2];3],periodic_u:true,periodic_v:false};
    periodic.validate().unwrap();assert!(remove(&periodic,Axis::U,1.,1_000_000).is_err());
    let source=surface_edit::insert(&periodic,Axis::V,0.5,1).unwrap();let r=remove(&source,Axis::V,0.5,1_000_000).unwrap();assert!(r.removed,"{:?}",r.refusal);assert_eq!(r.surface,periodic);
}
#[test]
fn degree_twenty_five_never_resets_or_raises_shared_work() {
    let original=Surface{degree_u:25,degree_v:1,knots_u:[vec![0.;26],vec![1.;26]].concat(),knots_v:vec![0.,0.,1.,1.],
        control_points:(0..26).map(|i|vec![vec![i as f64,0.,0.],vec![i as f64,1.,0.]]).collect(),weights:vec![vec![1.;2];26],periodic_u:false,periodic_v:false};
    let source=surface_edit::insert(&original,Axis::U,0.5,1).unwrap();let r=remove(&source,Axis::U,0.5,1_000_000).unwrap();assert!(r.work_used<=1_000_000);
    if r.removed {assert_eq!(r.surface,original);}else{assert_eq!(r.surface,source);assert!(matches!(r.refusal,Some(Refusal::ResourceLimit)|Some(Refusal::Curve{reason:curve_edit::Refusal::ResourceLimit,..})));}
}
