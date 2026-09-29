//! Open and periodic seam basis preparation. Candidates from `prepare` require
//! the whole-surface error certificate from `certify`; `checked` returns the
//! original pair whenever that acceptance gate fails.
use super::{Boundary,normalized};
use crate::{Result,check,curve::Curve,foundation::{refit_curve,refit_periodic,periodic_knots},surface::{Axis,Surface}};

pub struct PreparedSeams {
 pub reference:Surface,
 pub edited:Surface,
 pub degree:usize,
 pub control_count:usize,
 pub reference_domain:[f64;2],
 pub edited_domain:[f64;2],
 pub edited_reversed:bool,
}
fn axis(boundary:Boundary)->Axis{if boundary.cross_u{Axis::V}else{Axis::U}}
fn normalize(source:&Surface,boundary:Boundary,reverse:bool)->Result<(Surface,[f64;2])>{
 let(p,n,k,periodic)=boundary.along(source);
 let domain=[k[p],k[n]];
 check(periodic || k[..=p].iter().all(|&x|x==domain[0])&&k[n..].iter().all(|&x|x==domain[1]),"Seam preparation requires clamped seam endpoints")?;
 let knots=normalized(k,p,n)?;
 check(k.windows(2).zip(knots.windows(2)).all(|(a,b)|a[0]==a[1]||b[0]<b[1]),"Seam normalization collapsed distinct knots")?;
 let result=source.edit_axis(axis(boundary),|curve|{
  let mut result=curve.clone();result.knots=knots.clone();
  if reverse{result=result.reverse()?;}Ok(result)
 })?;
 Ok((result,domain))
}
fn elevated_multiplicities(surface:&Surface,b:Boundary,degree:usize)->Vec<(f64,usize)>{
 let(p,n,knots,periodic)=b.along(surface);let knots=if periodic{&knots[p..n]}else{knots};let mut groups:Vec<(f64,usize)>=Vec::new();
 for &k in knots{if let Some(last)=groups.last_mut(){if last.0==k{last.1+=1;continue;}}groups.push((k,1));}
 for (_,count) in &mut groups{*count+=degree-p;}
 groups
}
/// Builds a common normalized seam basis without artificially reducing its
/// continuity to C0. Returned surfaces are candidates, not accepted edits.
/// Cross directions and input definitions remain unchanged.
pub fn prepare(reference:&Surface,edited:&Surface,reference_boundary:&str,edited_boundary:&str,reverse:bool)->Result<PreparedSeams>{
 reference.validate()?;edited.validate()?;
 let r=Boundary::parse(reference_boundary)?;let e=Boundary::parse(edited_boundary)?;
 let periodic=r.along(reference).3;
 check(periodic==e.along(edited).3,"Seam preparation requires matching periodicity; convert the seam explicitly first")?;
 let(reference,reference_domain)=normalize(reference,r,false)?;
 let(edited,edited_domain)=normalize(edited,e,reverse)?;
 let degree=r.along(&reference).0.max(e.along(&edited).0);
 let mut groups=elevated_multiplicities(&reference,r,degree);
 for(k,count)in elevated_multiplicities(&edited,e,degree){
  if let Some(entry)=groups.iter_mut().find(|entry|entry.0==k){entry.1=entry.1.max(count);}else{groups.push((k,count));}
 }
 groups.sort_by(|a,b|a.0.total_cmp(&b.0));
 let mut knots:Vec<f64>=groups.iter().flat_map(|&(k,count)|std::iter::repeat_n(k,count)).collect();
 if periodic{knots.push(1.);}
 let active=knots.clone();
 if periodic{check(active.len()>degree+1,"Periodic aligned basis has too few distinct controls")?;knots=periodic_knots(&active,degree);}
 let control_count=knots.len().checked_sub(degree+1).ok_or_else(||crate::input("Invalid aligned seam basis"))?;
 check(control_count<=256,"Aligned seam exceeds 256 control points")?;
 let fit=|curve:&Curve|if curve.degree==degree&&curve.knots==knots{Ok(curve.clone())}else if periodic{refit_periodic(curve,degree,active.clone())}else{refit_curve(curve,degree,knots.clone())};
 let reference=reference.edit_axis(axis(r),fit)?;let edited=edited.edit_axis(axis(e),fit)?;
 Ok(PreparedSeams{reference,edited,degree,control_count,reference_domain,edited_domain,edited_reversed:reverse})
}

/// Certifies positional drift for a prepared pair. Exact predicates guard
/// domain normalization; a rounded non-affine knot map is not certified.
pub fn certify(reference:&Surface,edited:&Surface,reference_boundary:&str,edited_boundary:&str,candidate:&PreparedSeams,budget:f64)->Result<value_codec::Value>{
 check(budget.is_finite()&&budget>=0.,"Preparation budget must be finite and nonnegative")?;
 let mut errors=Vec::new();
 for(source,target,boundary,reverse)in[(reference,&candidate.reference,reference_boundary,false),(edited,&candidate.edited,edited_boundary,candidate.edited_reversed)]{
  let boundary=Boundary::parse(boundary)?;let(normalized,domain)=normalize(source,boundary,reverse)?;
  let(p,n,k,_)=boundary.along(source);let(_,_,mapped,_)=boundary.along(&normalized);
  let mapped:Vec<f64>=if reverse{mapped.iter().rev().copied().collect()}else{mapped.to_vec()};
  if super::affine_knots(k,[k[p],k[n]],&mapped,if reverse{[1.,0.]}else{[0.,1.]}).is_err(){
   return Ok(value_codec::json!({"accepted":false,"reason":"unproven-parameter-normalization","budget":budget}));
  }
  check(domain[0]<domain[1],"Invalid preparation source domain")?;
  errors.push(super::deviation::positional_upper(&normalized,target)?);
 }
 Ok(value_codec::json!({"accepted":errors.iter().all(|&e|e<=budget),"reason":if errors.iter().all(|&e|e<=budget){"accepted"}else{"deviation-exceeds-budget"},
  "referenceErrorUpper":errors[0],"editedErrorUpper":errors[1],"budget":budget,"wholeSurface":true,"method":"outward-homogeneous-Bernstein-difference"}))
}

/// Application boundary: failed proof returns the original pair.
pub fn checked(reference:&Surface,edited:&Surface,reference_boundary:&str,edited_boundary:&str,reverse:bool,budget:f64)->Result<value_codec::Value>{
 checked_with_conversion(reference,edited,reference_boundary,edited_boundary,reverse,budget,false)
}

/// Explicitly removes periodic storage while preserving the whole active patch.
/// The final proof compares directly with the original pair, including rounding
/// from clamping, normalization and basis fitting.
pub fn checked_with_conversion(reference:&Surface,edited:&Surface,reference_boundary:&str,edited_boundary:&str,reverse:bool,budget:f64,open_periodic:bool)->Result<value_codec::Value>{
 reference.validate()?;edited.validate()?;check(budget.is_finite()&&budget>=0.,"Preparation budget must be finite and nonnegative")?;
 let open=|surface:&Surface,name:&str|->Result<Surface>{
  let boundary=Boundary::parse(name)?;
  if !open_periodic||!boundary.along(surface).3{return Ok(surface.clone());}
  surface.edit_axis(axis(boundary),|curve|{let[a,b]=curve.domain();curve.trim(a,b)})
 };
 let a=open(reference,reference_boundary)?;let b=open(edited,edited_boundary)?;
 let candidate=prepare(&a,&b,reference_boundary,edited_boundary,reverse)?;
 let removed_reference=Boundary::parse(reference_boundary)?.along(reference).3&&!Boundary::parse(reference_boundary)?.along(&candidate.reference).3;
 let removed_edited=Boundary::parse(edited_boundary)?.along(edited).3&&!Boundary::parse(edited_boundary)?.along(&candidate.edited).3;
 let proof=certify(reference,edited,reference_boundary,edited_boundary,&candidate,budget)?;
 let accepted=proof["accepted"]==value_codec::json!(true);
 Ok(value_codec::json!({"reference":if accepted{&candidate.reference}else{reference},"edited":if accepted{&candidate.edited}else{edited},
  "report":proof,"basis":{"degree":candidate.degree,"controlCount":candidate.control_count,"normalizedSeam":true,"editedReversed":reverse,
  "referenceDomain":candidate.reference_domain,"editedDomain":candidate.edited_domain,"periodicityRemoved":{"reference":accepted&&removed_reference,"edited":accepted&&removed_edited}}}))
}

#[cfg(test)]mod tests{
 use super::*;
 fn patch(degree:usize,knots:Vec<f64>)->Surface{
  let n=knots.len()-degree-1;
  Surface{degree_u:3,degree_v:degree,knots_u:vec![2.,2.,2.,2.,5.,5.,5.,5.],knots_v:knots,
   control_points:(0..4).map(|i|(0..n).map(|j|vec![i as f64,j as f64,0.1*(i*j)as f64]).collect()).collect(),
   weights:(0..4).map(|i|(0..n).map(|j|1.+0.02*i as f64+0.03*j as f64).collect()).collect(),periodic_u:false,periodic_v:false}
 }
 #[test]fn aligns_degrees_domains_and_knots_without_introducing_C0_seams(){
  let a=patch(2,vec![3.,3.,3.,5.,7.,7.,7.]);
  let b=patch(3,vec![-2.,-2.,-2.,-2.,1.,2.,2.,2.,2.]);
  let before=value_codec::to_value((&a,&b)).unwrap();
  for reverse in [false,true]{
   let result=prepare(&a,&b,"uMax","uMin",reverse).unwrap();
   let proof=certify(&a,&b,"uMax","uMin",&result,1e-8).unwrap();
   assert_eq!(proof["accepted"],value_codec::json!(true));
   assert_eq!(certify(&a,&b,"uMax","uMin",&result,0.).unwrap()["accepted"],value_codec::json!(false));
   assert_eq!(result.degree,3);assert_eq!(result.reference.knots_v,result.edited.knots_v);
   assert_eq!(result.reference.knots_v.iter().filter(|&&k|k==0.5).count(),2);
   assert_eq!(result.reference_domain,[3.,7.]);assert_eq!(result.edited_domain,[-2.,2.]);
   for i in 0..=20{for j in 0..=20{
    let u=2.+3.*i as f64/20.;let t=j as f64/20.;
    for(source,target,v)in[(&a,&result.reference,3.+4.*t),(&b,&result.edited,-2.+4.*if reverse{1.-t}else{t})]{
     let p=source.evaluate(u,v).unwrap();let q=target.evaluate(u,t).unwrap();
     for(x,y)in p.point.iter().zip(&q.point){assert!((x-y).abs()<1e-10);}
     let sign=if std::ptr::eq(source,&b)&&reverse{-1.}else{1.};
     let p=value_codec::to_value(p).unwrap();let q=value_codec::to_value(q).unwrap();
     for(field,factor)in[("du",1.),("dv",4.*sign),("duu",1.),("duv",4.*sign)]{
      let a:Vec<f64>=value_codec::from_value(p[field].clone()).unwrap();let b:Vec<f64>=value_codec::from_value(q[field].clone()).unwrap();
      for(x,y)in a.iter().zip(b){assert!((x*factor-y).abs()<1e-9,"{field}: {x} versus {y}");}
     }
    }
   }}
  }
  assert_eq!(value_codec::to_value((&a,&b)).unwrap(),before);
 }
 #[test]fn already_compatible_normalized_surfaces_keep_their_controls(){
  let a=patch(3,vec![0.,0.,0.,0.,0.5,1.,1.,1.,1.]);
  let result=prepare(&a,&a,"uMax","uMin",false).unwrap();
  assert_eq!(certify(&a,&a,"uMax","uMin",&result,0.).unwrap()["accepted"],value_codec::json!(true));
  assert_eq!(value_codec::to_value(&a).unwrap(),value_codec::to_value(result.reference).unwrap());
  assert_eq!(value_codec::to_value(&a).unwrap(),value_codec::to_value(result.edited).unwrap());
 }

 #[test]fn certificate_detects_candidate_translation_and_refuses_rounded_knot_maps(){
  let a=patch(3,vec![0.,0.,0.,0.,0.5,1.,1.,1.,1.]);
  let mut result=prepare(&a,&a,"uMax","uMin",false).unwrap();
  for row in &mut result.edited.control_points{for p in row{p[0]+=0.001;}}
  let proof=certify(&a,&a,"uMax","uMin",&result,1e-6).unwrap();
  assert_eq!(proof["accepted"],value_codec::json!(false));
  assert!(proof["editedErrorUpper"].as_f64().unwrap()>=0.001-1e-14);
  let a=patch(3,vec![0.,0.,0.,0.,0.3,0.9,0.9,0.9,0.9]);
  let result=prepare(&a,&a,"uMax","uMin",false).unwrap();
  assert_eq!(certify(&a,&a,"uMax","uMin",&result,1.).unwrap()["reason"],value_codec::json!("unproven-parameter-normalization"));
 }

 fn periodic_patch(degree:usize,unique:usize)->Surface{
  let points:Vec<Vec<Vec<f64>>>=(0..4).map(|i|(0..unique+degree).map(|j|{let t=(j%unique)as f64*std::f64::consts::TAU/unique as f64;vec![i as f64,t.cos(),t.sin()]}).collect()).collect();
  Surface{degree_u:3,degree_v:degree,knots_u:vec![0.,0.,0.,0.,1.,1.,1.,1.],knots_v:(0..unique+2*degree+1).map(|i|i as f64).collect(),control_points:points,weights:(0..4).map(|i|(0..unique+degree).map(|j|1.+0.02*i as f64+0.01*(j%unique)as f64).collect()).collect(),periodic_u:false,periodic_v:true}
 }
 #[test]fn prepares_periodic_seams_with_different_degrees_and_preserves_wrapping(){
  let a=periodic_patch(2,4);let b=periodic_patch(3,8);
  for reverse in [false,true]{
   let result=prepare(&a,&b,"uMax","uMin",reverse).unwrap();
   assert!(result.reference.periodic_v&&result.edited.periodic_v);
   assert_eq!(result.reference.knots_v,result.edited.knots_v);
   let proof=certify(&a,&b,"uMax","uMin",&result,1e-8).unwrap();assert_eq!(proof["accepted"],value_codec::json!(true),"{proof}");
   for(source,target,domain,rev)in[(&a,&result.reference,[2.,6.],false),(&b,&result.edited,[3.,11.],reverse)]{
    for i in 0..=64{let t=i as f64/64.;let v=domain[0]+(domain[1]-domain[0])*if rev{1.-t}else{t};
     let p=source.evaluate(0.3,v).unwrap().point;let q=target.evaluate(0.3,t).unwrap().point;
     for(x,y)in p.iter().zip(q){assert!((x-y).abs()<1e-10);}
    }
    for row in &target.control_points{assert_eq!(&row[..3],&row[row.len()-3..]);}
   }
  }
 }

 #[test]fn checked_preparation_returns_original_pair_on_failed_budget(){
  let a=patch(2,vec![0.,0.,0.,0.5,1.,1.,1.]);let b=patch(3,vec![0.,0.,0.,0.,1.,1.,1.,1.]);
  let result=checked(&a,&b,"uMax","uMin",false,0.).unwrap();
  assert_eq!(result["report"]["accepted"],value_codec::json!(false));
  assert_eq!(result["reference"],value_codec::to_value(&a).unwrap());assert_eq!(result["edited"],value_codec::to_value(&b).unwrap());
 }

 #[test]fn explicit_conversion_prepares_mixed_seams_and_bounds_the_original_geometry(){
  let periodic=periodic_patch(2,4);let open=patch(3,vec![0.,0.,0.,0.,1.,1.,1.,1.]);
  assert!(checked(&periodic,&open,"uMax","uMin",false,1e-8).is_err());
  for reverse in [false,true]{
   let result=checked_with_conversion(&periodic,&open,"uMax","uMin",reverse,1e-8,true).unwrap();
   assert_eq!(result["report"]["accepted"],value_codec::json!(true));
   assert_eq!(result["basis"]["periodicityRemoved"]["reference"],value_codec::json!(true));
   assert_eq!(result["basis"]["periodicityRemoved"]["edited"],value_codec::json!(false));
   let a:Surface=value_codec::from_value(result["reference"].clone()).unwrap();assert!(!a.periodic_v);
   for i in 0..=32{let u=i as f64/32.;let first=a.evaluate(u,0.).unwrap().point;let last=a.evaluate(u,1.).unwrap().point;for(x,y)in first.iter().zip(last){assert!((x-y).abs()<1e-12);}}
   let failed=checked_with_conversion(&periodic,&open,"uMax","uMin",reverse,0.,true).unwrap();
   assert_eq!(failed["report"]["accepted"],value_codec::json!(false));
   assert_eq!(failed["reference"],value_codec::to_value(&periodic).unwrap());assert_eq!(failed["edited"],value_codec::to_value(&open).unwrap());
  }
 }

 #[test]fn explicit_conversion_can_unlink_both_periodic_inputs(){
  let a=periodic_patch(2,4);let b=periodic_patch(3,8);
  let result=checked_with_conversion(&a,&b,"uMax","uMin",true,1e-8,true).unwrap();
  assert_eq!(result["report"]["accepted"],value_codec::json!(true));
  assert_eq!(result["basis"]["periodicityRemoved"],value_codec::json!({"reference":true,"edited":true}));
  assert_eq!(result["reference"]["periodicV"],value_codec::json!(false));assert_eq!(result["edited"]["periodicV"],value_codec::json!(false));
 }

}
