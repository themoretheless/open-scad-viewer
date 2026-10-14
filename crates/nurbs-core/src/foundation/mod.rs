//! Conservative, deterministic certificates used by product NURBS consumers.
//!
//! Every geometric enclosure is derived from positive-weight convex-hull
//! properties. `next_down`/`next_up` make binary64 arithmetic outward rounded.
//! Ambiguous regularity and projection cases remain explicitly unresolved.
use crate::{
    Result, check,
    curve::Curve,
    numeric, resource,
    surface::{Axis, Surface},
};
use cad_predicates::ToleranceContext;
use value_codec::{Value, json};

const MAX_CERTIFICATE_CELLS: usize = 4096;

use crate::intersection::{next_down, next_up};
fn interval(values: impl Iterator<Item = f64>) -> [f64; 2] {
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for value in values {
        lo = lo.min(value);
        hi = hi.max(value);
    }
    [next_down(lo), next_up(hi)]
}
fn box_of(points: &[Vec<f64>]) -> (Vec<f64>, Vec<f64>) {
    let dimension = points[0].len();
    (
        (0..dimension)
            .map(|axis| next_down(points.iter().map(|p| p[axis]).fold(f64::INFINITY, f64::min)))
            .collect(),
        (0..dimension)
            .map(|axis| {
                next_up(
                    points
                        .iter()
                        .map(|p| p[axis])
                        .fold(f64::NEG_INFINITY, f64::max),
                )
            })
            .collect(),
    )
}
fn tolerance_evidence(context: &ToleranceContext) -> Value {
    let spatial = context.spatial_bounds();
    json!({
        "toleranceIdentity": context.spec_identity(),
        "linearAbsoluteMm": spatial.absolute_mm,
        "linearRelative": spatial.relative,
        "parametricFloor": context.parametric_bounds().floor,
        "maxEntityErrorMm": context.entity_error_bounds().maximum_mm
    })
}
fn context(value: Option<ToleranceContext>) -> ToleranceContext {
    value.unwrap_or_else(ToleranceContext::default_valid)
}
fn copy_context(value: &ToleranceContext) -> Result<ToleranceContext> {
    value_codec::from_value(
        value_codec::to_value(value).map_err(|error| crate::numeric_err(error.to_string()))?,
    )
    .map_err(|error| crate::numeric_err(error.to_string()))
}

mod certify;
mod projection;
mod edit;
mod periodic;
mod reparameterization;
pub mod fitting;
mod interval_root;
mod materialize;
mod cloud_fit;
pub use certify::*;
pub use projection::*;
pub use edit::*;
pub use periodic::*;
pub use reparameterization::*;
pub use fitting::*;
use interval_root::*;
pub use materialize::*;
pub use cloud_fit::*;

#[cfg(test)]
mod surface_reduction_validation_tests {
 use super::*;
 fn plane()->Surface {Surface{degree_u:2,degree_v:2,knots_u:vec![0.,0.,0.,1.,1.,1.],knots_v:vec![0.,0.,0.,1.,1.,1.],control_points:(0..3).map(|i|(0..3).map(|j|vec![i as f64,j as f64,0.]).collect()).collect(),weights:vec![vec![1.;3];3],periodic_u:false,periodic_v:false}}
 #[test]fn rejects_degrees_before_unsigned_subtraction(){
  let s=plane();for degree in [0,2,3,usize::MAX] {assert!(reduce_surface_axis(&s,Axis::U,"reduce",0.,degree,0.,None).is_err());}
  assert!(reduce_surface_axis(&s,Axis::V,"other",0.,1,0.,None).is_err());
 }
 #[test]fn reduces_both_axes_with_zero_error(){
  let s=plane();for axis in [Axis::U,Axis::V]{let result=reduce_surface_axis(&s,axis,"reduce",0.,1,0.,None).unwrap();assert_eq!(result["certificate"]["accepted"],json!(true));assert_eq!(result["certificate"]["hausdorffErrorUpper"],json!(0.));}
 }
}

#[cfg(test)]
mod deviation_span_tests {
    use super::*;
    #[test]
    fn discontinuity_cannot_receive_a_lipschitz_certificate() {
        let curve = Curve {
            degree: 1, knots: vec![0., 0., 0.5, 0.5, 1., 1.],
            control_points: vec![vec![0., 0., 0.], vec![1., 0., 0.], vec![2., 0., 0.], vec![3., 0., 0.]],
            weights: vec![1.; 4], periodic: false,
        };
        assert!(periodic_error(&curve, &curve).is_err());
    }
    #[test]
    fn narrow_peak_between_samples_is_bounded() {
        for scale in [0.1, 1., 10.] {
            let source = Curve {
                degree: 1,
                knots: vec![0., 0., 0.0001 * scale, 0.0002 * scale, scale, scale],
                control_points: vec![vec![0., 0., 0.], vec![0., 1., 0.], vec![0., 0., 0.], vec![0., 0., 0.]],
                weights: vec![1.; 4], periodic: false,
            };
            let target = Curve {
                degree: 1, knots: vec![0., 0., scale, scale],
                control_points: vec![vec![0., 0., 0.]; 2],
                weights: vec![1.; 2], periodic: false,
            };
            let error = periodic_error(&source, &target).unwrap();
            assert!(error >= 1., "missed narrow peak: {error}");
            assert!((error - 4.8828125).abs() < 1e-10);
        }
    }
}

#[cfg(test)]
mod rebuild_tests {
    use super::*;
    fn curve() -> Curve { Curve { degree:2, knots:vec![2.,2.,2.,5.,5.,5.],
        control_points:vec![vec![0.,0.,0.],vec![1.,2.,0.],vec![2.,0.,0.]],
        weights:vec![1.;3],periodic:false } }
    #[test] fn rebuild_controls_and_degree_with_rollback() {
        let c=curve();
        let refused=rebuild_curve(&c,1,2,0.,None).unwrap();
        assert_eq!(refused["certificate"]["accepted"],json!(false));
        assert_eq!(refused["curve"],value_codec::to_value(&c).unwrap());
        let accepted=rebuild_curve(&c,3,6,0.1,None).unwrap();
        assert_eq!(accepted["certificate"]["accepted"],json!(true));
        let rebuilt:Curve=value_codec::from_value(accepted["curve"].clone()).unwrap();
        assert_eq!(rebuilt.degree,3); assert_eq!(rebuilt.control_points.len(),6);
        assert_eq!(rebuilt.domain(),[2.,5.]);
        let bound=accepted["certificate"]["hausdorffErrorUpper"].as_f64().unwrap();
        for i in 0..=300 {let u=2.+3.*i as f64/300.;
            assert!(distance(&c.evaluate(u).unwrap().point,&rebuilt.evaluate(u).unwrap().point)<=bound);
        }
    }
    #[test] fn rebuild_preserves_rational_shape_within_reported_bound() {
        let c=Curve { degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
            control_points:vec![vec![1.,0.,0.],vec![1.,1.,0.],vec![0.,1.,0.]],
            weights:vec![1.,0.5_f64.sqrt(),1.],periodic:false };
        let result=rebuild_curve(&c,3,6,0.05,None).unwrap();
        assert_eq!(result["certificate"]["accepted"],json!(true));
        let rebuilt:Curve=value_codec::from_value(result["curve"].clone()).unwrap();
        assert!(rebuilt.weights.iter().any(|w|(*w-1.).abs()>0.01));
        let bound=result["certificate"]["hausdorffErrorUpper"].as_f64().unwrap();
        for i in 0..=500 {let u=i as f64/500.;let p=rebuilt.evaluate(u).unwrap().point;
            assert!(distance(&c.evaluate(u).unwrap().point,&p)<=bound);
            assert!((p[0].hypot(p[1])-1.).abs()<1e-10);
        }
    }
    #[test] fn rebuild_rejects_invalid_budgets_and_basis() {
        let c=curve();
        for (d,n) in [(0,4),(26,30),(3,3),(1,257),(usize::MAX,4)] {
            assert!(rebuild_curve(&c,d,n,0.1,None).is_err());
        }
        for budget in [-1.,f64::NAN,f64::INFINITY] {
            assert!(rebuild_curve(&c,2,3,budget,None).is_err());
        }
    }
}

// Outward arithmetic for the coefficients of Xa*Wb-Xb*Wa.
fn rebuild_add(a:[f64;2],b:[f64;2])->[f64;2] { [next_down(a[0]+b[0]),next_up(a[1]+b[1])] }
fn rebuild_mul(a:[f64;2],b:[f64;2])->[f64;2] {
    let v=[a[0]*b[0],a[0]*b[1],a[1]*b[0],a[1]*b[1]];
    [next_down(v.into_iter().fold(f64::INFINITY,f64::min)),next_up(v.into_iter().fold(f64::NEG_INFINITY,f64::max))]
}
fn rebuild_binomial(n:usize,k:usize)->[f64;2] {
    let k=k.min(n-k);let mut x=[1.,1.];
    for i in 0..k {x=rebuild_mul(x,[(n-i) as f64,(n-i) as f64]);x=[next_down(x[0]/(i+1) as f64),next_up(x[1]/(i+1) as f64)];} x
}
fn rebuild_product_factor(p:usize,q:usize,i:usize,j:usize)->[f64;2] {
    let n=rebuild_mul(rebuild_binomial(p,i),rebuild_binomial(q,j));let d=rebuild_binomial(p+q,i+j);
    [next_down(n[0]/d[1]),next_up(n[1]/d[0])]
}
fn rebuild_surface_error(a:&Surface,b:&Surface)->Result<f64> {
    crate::continuity::deviation::positional_upper(a,b)
}

pub fn rebuild_surface(source:&Surface,axis:Axis,degree:usize,control_count:usize,max_error:f64,tolerance:Option<ToleranceContext>)->Result<Value> {
    source.validate()?;
    check(max_error.is_finite()&&max_error>=0.,"Maximum error must be finite and nonnegative")?;
    let candidate=source.edit_axis(axis,|curve|rebuild_candidate(curve,degree,control_count))?;
    let exact=exact_surface(source,&candidate);
    let error=if exact {0.} else {rebuild_surface_error(source,&candidate)?};
    let accepted=error<=max_error;let tolerance=context(tolerance);
    Ok(json!({"surface":if accepted {candidate} else {source.clone()},"certificate":{
        "version":"nurbs-foundation/3","operation":"surface-rebuild","accepted":accepted,"rolledBack":!accepted,
        "exactZeroRecognized":exact,"hausdorffErrorUpper":error,"budget":max_error,
        "method":"outward-homogeneous-Bernstein-difference","evidence":tolerance_evidence(&tolerance)}}))
}

#[cfg(test)]
mod surface_rebuild_tests {
    use super::*;
    fn surface(rational:bool)->Surface {
        Surface {degree_u:2,degree_v:2,knots_u:vec![2.,2.,2.,5.,5.,5.],knots_v:vec![0.,0.,0.,1.,1.,1.],
            control_points:(0..3).map(|i|(0..3).map(|j|vec![i as f64,j as f64,if i==1&&j==1 {1.} else {0.}]).collect()).collect(),
            weights:(0..3).map(|i|(0..3).map(|j|if rational&&i==1&&j==1 {0.6} else {1.}).collect()).collect(),periodic_u:false,periodic_v:false}
    }
    #[test]fn rational_and_polynomial_rebuilds_bound_the_whole_patch() {
        for rational in [false,true] {for axis in [Axis::U,Axis::V] {
            let s=surface(rational);let result=rebuild_surface(&s,axis,3,6,0.001,None).unwrap();
            assert_eq!(result["certificate"]["accepted"],json!(true));
            let rebuilt:Surface=value_codec::from_value(result["surface"].clone()).unwrap();
            assert_eq!(match axis {Axis::U=>rebuilt.control_points.len(),Axis::V=>rebuilt.control_points[0].len()},6);
            let bound=result["certificate"]["hausdorffErrorUpper"].as_f64().unwrap();
            for i in 0..=20 {for j in 0..=20 {
                let u=2.+3.*i as f64/20.;let v=j as f64/20.;
                let error=distance(&s.evaluate(u,v).unwrap().point,&rebuilt.evaluate(u,v).unwrap().point);
                assert!(error<=bound,"{error} > {bound}");
            }}
        }}
    }
    #[test]fn surface_rebuild_never_accepts_an_overflowed_bound() {
        let mut s=surface(true);
        for row in &mut s.control_points {for point in row {for coordinate in point {*coordinate *= 1e290;}}}
        for row in &mut s.weights {for weight in row {*weight *= 1e10;}}
        let result=rebuild_surface(&s,Axis::U,3,6,0.01,None);
        if let Ok(value)=result {assert_eq!(value["certificate"]["accepted"],json!(false));}
    }
    #[test]fn surface_bound_detects_a_peak_between_uniform_samples() {
        let s=Surface {degree_u:1,degree_v:1,
            knots_u:vec![0.,0.,0.0001,0.0002,1.,1.],knots_v:vec![0.,0.,1.,1.],
            control_points:[0.,0.0001,0.0002,1.].into_iter().enumerate().map(|(i,x)|
                vec![vec![x,0.,if i==1 {1.} else {0.}],vec![x,1.,if i==1 {1.} else {0.}]]).collect(),
            weights:vec![vec![1.;2];4],periodic_u:false,periodic_v:false};
        let result=rebuild_surface(&s,Axis::U,1,2,0.01,None).unwrap();
        assert_eq!(result["certificate"]["accepted"],json!(false));
        assert!(result["certificate"]["hausdorffErrorUpper"].as_f64().unwrap()>=1.);
    }
    #[test]fn refuses_a_removed_bump_and_preserves_the_original() {
        let s=surface(false);let result=rebuild_surface(&s,Axis::U,1,2,0.01,None).unwrap();
        assert_eq!(result["certificate"]["accepted"],json!(false));
        assert_eq!(result["surface"],value_codec::to_value(&s).unwrap());
        assert!(result["certificate"]["hausdorffErrorUpper"].as_f64().unwrap()>=0.25);
        assert!(rebuild_surface(&s,Axis::U,0,2,0.1,None).is_err());
        assert!(rebuild_surface(&s,Axis::U,3,3,0.1,None).is_err());
    }
}

#[cfg(test)]
mod periodic_rebuild_tests {
 use super::*;
 fn source(rational:bool)->Curve {
  Curve{degree:2,knots:(0..9).map(|i|i as f64).collect(),
   control_points:vec![vec![1.,0.,0.],vec![0.,1.,0.],vec![-1.,0.,0.],vec![0.,-1.,0.],vec![1.,0.,0.],vec![0.,1.,0.]],
   weights:if rational {vec![1.,0.8,1.2,1.,1.,0.8]}else{vec![1.;6]},periodic:true}
 }
 #[test]fn rebuilds_periodic_degrees_with_wrapped_controls_and_bounded_error(){
  for rational in [false,true] {for degree in 1..=4 {
   let c=source(rational);let result=rebuild_curve(&c,degree,12+degree,0.2,None).unwrap();
   assert_eq!(result["certificate"]["accepted"],json!(true),"degree={degree} rational={rational}: {result:?}");
   let rebuilt:Curve=value_codec::from_value(result["curve"].clone()).unwrap();
   rebuilt.validate().unwrap();assert!(rebuilt.periodic);assert_eq!(rebuilt.control_points.len(),12+degree);
   let bound=result["certificate"]["hausdorffErrorUpper"].as_f64().unwrap();
   for i in 0..=2000 {let u=2.+4.*i as f64/2000.;assert!(distance(&c.evaluate(u).unwrap().point,&rebuilt.evaluate(u).unwrap().point)<=bound);}
   let first=rebuilt.evaluate(2.).unwrap();let last=rebuilt.evaluate(6.).unwrap();
   assert!(distance(&first.point,&last.point)<1e-12);
   if degree>=2 {assert!(distance(&first.d1.unwrap(),&last.d1.unwrap())<1e-11);}
   if degree>=3 {assert!(distance(&first.d2.unwrap(),&last.d2.unwrap())<1e-10);}
  }}
 }
 #[test]fn rollback_retains_periodic_source_and_invalid_counts_fail(){
  let c=source(true);let result=rebuild_curve(&c,1,5,0.,None).unwrap();
  assert_eq!(result["certificate"]["accepted"],json!(false));
  assert_eq!(result["curve"],value_codec::to_value(&c).unwrap());
  assert!(rebuild_curve(&c,3,6,1.,None).is_err());
  assert!(rebuild_curve(&c,2,257,1.,None).is_err());
 }
}

#[cfg(test)]
mod periodic_surface_rebuild_tests {
 use super::*;
 fn torus()->Surface {
  let points=(0..6).map(|i|(0..6).map(|j|{
   let u=(i%4) as f64*std::f64::consts::FRAC_PI_2;
   let v=(j%4) as f64*std::f64::consts::FRAC_PI_2;
   vec![(2.+0.5*v.cos())*u.cos(),(2.+0.5*v.cos())*u.sin(),0.5*v.sin()]
  }).collect()).collect();
  Surface{degree_u:2,degree_v:2,knots_u:(0..9).map(|i|i as f64).collect(),knots_v:(0..9).map(|i|i as f64).collect(),control_points:points,
   weights:(0..6).map(|i|(0..6).map(|j|[1.,0.8,1.2,1.][i%4]*[1.,0.9,1.1,1.][j%4]).collect()).collect(),periodic_u:true,periodic_v:true}
 }
 #[test]fn mixed_periodic_and_open_axes_preserve_cylinder_seam(){
  let base=torus();
  let cylinder=Surface{degree_u:2,degree_v:1,knots_u:base.knots_u.clone(),knots_v:vec![0.,0.,1.,1.],
   control_points:base.control_points.iter().map(|row|vec![row[0].clone(),vec![row[0][0],row[0][1],3.]]).collect(),
   weights:base.weights.iter().map(|row|vec![row[0];2]).collect(),periodic_u:true,periodic_v:false};
  for axis in [Axis::U,Axis::V]{
   let count=if matches!(axis,Axis::U){15}else{6};
   let budget=if matches!(axis,Axis::U){0.2}else{0.001};
   let result=rebuild_surface(&cylinder,axis,3,count,budget,None).unwrap();
   assert_eq!(result["certificate"]["accepted"],json!(true),"{result:?}");
   let target:Surface=value_codec::from_value(result["surface"].clone()).unwrap();
   assert!(target.periodic_u&&!target.periodic_v);
   let bound=result["certificate"]["hausdorffErrorUpper"].as_f64().unwrap();
   for i in 0..=32{for j in 0..=16{
    let u=2.+4.*i as f64/32.;let v=j as f64/16.;
    assert!(distance(&cylinder.evaluate(u,v).unwrap().point,&target.evaluate(u,v).unwrap().point)<=bound);
   }}
  }
 }
 #[test]fn both_periodic_axes_keep_wrapping_and_certify_dense_error(){
  let source=torus();source.validate().unwrap();
  for axis in [Axis::U,Axis::V]{
   let value=rebuild_surface(&source,axis,3,15,0.2,None).unwrap();
   assert_eq!(value["certificate"]["accepted"],json!(true),"{value:?}");
   let target:Surface=value_codec::from_value(value["surface"].clone()).unwrap();
   target.validate().unwrap();assert!(target.periodic_u&&target.periodic_v);
   let bound=value["certificate"]["hausdorffErrorUpper"].as_f64().unwrap();
   for i in 0..=32{for j in 0..=32{
    let u=2.+4.*i as f64/32.;let v=2.+4.*j as f64/32.;
    assert!(distance(&source.evaluate(u,v).unwrap().point,&target.evaluate(u,v).unwrap().point)<=bound);
   }}
   for i in 0..=32 {let t=2.+4.*i as f64/32.;
    assert!(distance(&target.evaluate(2.,t).unwrap().point,&target.evaluate(6.,t).unwrap().point)<1e-12);
    assert!(distance(&target.evaluate(t,2.).unwrap().point,&target.evaluate(t,6.).unwrap().point)<1e-12);
    for (a,b) in [(target.evaluate(2.,t).unwrap(),target.evaluate(6.,t).unwrap()),(target.evaluate(t,2.).unwrap(),target.evaluate(t,6.).unwrap())] {
     let (au,av)=a.first_derivatives().unwrap();let (bu,bv)=b.first_derivatives().unwrap();
     assert!(distance(&au,&bu)<1e-11);assert!(distance(&av,&bv)<1e-11);
    }
   }
   let refused=rebuild_surface(&source,axis,1,5,0.,None).unwrap();
   assert_eq!(refused["certificate"]["accepted"],json!(false));
   assert_eq!(refused["surface"],value_codec::to_value(&source).unwrap());
  }
 }
}

#[cfg(test)]mod normal_regularity_regressions {
 use super::*;
 #[test]fn aligned_corners_do_not_certify_an_interior_rank_loss(){
  let s=Surface{degree_u:3,degree_v:1,knots_u:vec![0.,0.,0.,0.,1.,1.,1.,1.],knots_v:vec![0.,0.,1.,1.],
   control_points:[0.,1.,0.,1.].iter().map(|&x|vec![vec![x,0.,0.],vec![x,1.,0.]]).collect(),weights:vec![vec![1.;2];4],periodic_u:false,periodic_v:false};
  assert!(s.evaluate(0.5,0.5).unwrap().unit_normal().is_none());
  assert_eq!(certify_surface_cell(&s,3,1)["classification"],json!("unresolved"));
 }
 #[test]fn flat_corner_normals_do_not_prove_a_patch_is_planar(){
  let s=Surface{degree_u:3,degree_v:3,knots_u:vec![0.,0.,0.,0.,1.,1.,1.,1.],knots_v:vec![0.,0.,0.,0.,1.,1.,1.,1.],
   control_points:(0..4).map(|i|(0..4).map(|j|vec![i as f64,j as f64,if (1..=2).contains(&i)&&(1..=2).contains(&j){1.}else{0.}]).collect()).collect(),weights:vec![vec![1.;4];4],periodic_u:false,periodic_v:false};
  assert!(s.evaluate(0.5,0.5).unwrap().point[2]>0.);
  assert_eq!(certify_surface_cell(&s,3,3)["classification"],json!("certified_regular"));
  assert_eq!(classify_normal_box(&[[0.,0.],[0.,0.],[1.,2.]]),"certified_regular");
 }
}

#[path = "../reparameterization_preimages.rs"]
mod preimages;
pub use preimages::bound_reparameterization_preimages;

#[path = "../reparameterization_residual.rs"]
mod reparameterization_residual;

#[path = "../reparameterization_retention.rs"]
mod reparameterization_retention;
pub use reparameterization_retention::{certify_reparameterized_curve_retention,certify_reparameterized_surface_section_retention};

#[path = "../reparameterization_materialization.rs"]
mod reparameterization_materialization;
pub use reparameterization_materialization::materialize_reparameterized_curve_bounded;

pub mod guards;

mod native_fitting;

pub mod parameter_mapping;
pub mod bracketed;

pub mod interpolation;

mod native_knot_removal;

mod curve_projection;
pub use curve_projection::{ProjectionStatus,CandidateClassification,ProjectionCandidate,CurveProjection,project_curve_report};

// Typed native APIs (report-returning) restored alongside the legacy Value
// entry points above. Only typed names are re-exported here so the legacy
// codec functions keep their existing implementations and outputs.
pub mod approximate_edits;
pub use approximate_edits::{
    SurfaceSimplification, rebuild_curve_report, rebuild_surface_report,
    reduce_curve_degree_report, reduce_surface_axis_report, remove_curve_knot_report,
};
pub mod periodic_edits;
pub use periodic_edits::{
    PeriodicEditOperation, edit_periodic_curve_report, edit_periodic_surface_report,
    split_periodic_curve_report,
};
pub mod approximation;
pub use approximation::{approximate_curve_report, interpolate_polyline_report};
pub mod exact_edits;
pub use exact_edits::{ExactEditOperation, certify_exact_edit_report, reparameterize_curve_report};
pub mod min_zone_fit;
pub mod oslo;
pub use native_fitting::{
    fit_curve_cloud_certified_report, fit_curve_points_report, fit_surface_cloud_certified_report,
    interpolate_surface_grid_report,
};
pub use parameter_mapping::{
    MapPiece, ParameterMapping, certify_reparameterization_report,
    evaluate_reparameterized_curve_report, materialize_reparameterized_curve_report,
};
pub mod certificates;
pub use certificates::{certify_curve_report, certify_surface_report};
mod surface_projection;
pub use surface_projection::{
    BoundaryProjection, SurfaceCandidateClassification, SurfaceProjection,
    SurfaceProjectionCandidate, SurfaceProjectionProof, project_surface_report,
};
mod common;
