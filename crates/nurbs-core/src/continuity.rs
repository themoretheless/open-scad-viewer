//! Boundary jet matching in homogeneous coordinates. The basis along the seam
//! must agree after affine parameter normalization. Regularity of each seam
//! is checked separately with outward-rounded Bernstein normal enclosures.
//! Representation reference: MIT Hyperbook, sections 1.4.2 and 1.5.
mod exact_strip;
pub use exact_strip::{ExactStripJetReport, inspect_surface_exact_strip_jets, inspect_surface_projective_strip_jets};
use crate::{check,curve::basis,surface::Surface,Result};
use value_codec::{Value,json};
mod regularity;
mod bounds;
mod station_scale;
pub use station_scale::propose_station_normal_scale;
pub mod preparation;
pub mod deviation;
pub mod curve_match;
type H=[f64;4];
#[derive(Clone,Copy)]
struct Boundary {cross_u:bool,max:bool}
impl Boundary {
 fn parse(name:&str)->Result<Self>{match name {
  "uMin"=>Ok(Self{cross_u:true,max:false}),"uMax"=>Ok(Self{cross_u:true,max:true}),
  "vMin"=>Ok(Self{cross_u:false,max:false}),"vMax"=>Ok(Self{cross_u:false,max:true}),
  _=>Err(crate::input("Unknown surface boundary"))}}
 fn cross<'a>(&self,s:&'a Surface)->(usize,usize,&'a [f64],bool){
  if self.cross_u {(s.degree_u,s.control_points.len(),&s.knots_u,s.periodic_u)}
  else {(s.degree_v,s.control_points[0].len(),&s.knots_v,s.periodic_v)}
 }
 fn along<'a>(&self,s:&'a Surface)->(usize,usize,&'a [f64],bool){Self{cross_u:!self.cross_u,max:false}.cross(s)}
 fn index(&self,s:&Surface,i:usize,layer:usize)->(usize,usize){let n=self.cross(s).1;let c=if self.max{n-1-layer}else{layer};if self.cross_u{(c,i)}else{(i,c)}}
 fn get(&self,s:&Surface,i:usize,layer:usize)->H{let(u,v)=self.index(s,i,layer);let w=s.weights[u][v];let p=&s.control_points[u][v];[p[0]*w,p[1]*w,p[2]*w,w]}
 fn set(&self,s:&mut Surface,i:usize,layer:usize,h:H)->Result<()>{
  check(h.iter().all(|x|x.is_finite())&&h[3]>=1e-12&&h[3]<=1e12,"Surface matching produced inadmissible weights; reduce the normal scale or change the reference strip")?;
  let(u,v)=self.index(s,i,layer);s.weights[u][v]=h[3];s.control_points[u][v]=h[..3].iter().map(|x|x/h[3]).collect();Ok(())
 }
 fn coefficients(&self,s:&Surface,order:usize)->Result<Vec<Vec<f64>>>{
  let(p,n,k,periodic)=self.cross(s);
  check(!periodic,"Surface matching requires a non-periodic cross direction")?;
  check(p>=order,"Surface matching requires cross degree at least the requested continuity order")?;
  let k=normalized(k,p,n)?;let endpoint=if self.max{1.}else{0.};
  let edge=if self.max{&k[k.len()-p-1..]}else{&k[..p+1]};
  check(edge.iter().all(|&x|x==endpoint),"Surface matching requires a clamped cross boundary")?;
  let b=basis(p,&k,n,endpoint,false)?;let sign=if self.max{-1.}else{1.};
  let mut rows=vec![b.basis,b.d1.iter().map(|x|x*sign).collect()];if order==2{rows.push(b.d2);}
  let coefficients:Vec<Vec<f64>>=rows.iter().map(|row|(0..=order).map(|l|row[if self.max{n-1-l}else{l}]).collect()).collect();
  for q in 1..=order{check(coefficients[q][q].is_finite()&&coefficients[q][q].abs()>1e-14,"Surface boundary derivative system is singular")?;}
  Ok(coefficients)
 }
}
fn normalized(knots:&[f64],degree:usize,count:usize)->Result<Vec<f64>>{
 let a=knots[degree];let span=knots[count]-a;check(span.is_finite()&&span>0.,"Surface matching parameter span is invalid")?;
 let output:Vec<f64>=knots.iter().map(|k|(k-a)/span).collect();check(output.iter().all(|x|x.is_finite()),"Surface matching normalized knots overflowed")?;Ok(output)
}
// The represented binary64 knots are the input definition of this operation.
// The predicate certifies their affine correspondence, not any earlier unrounded construction.
fn affine_knots(a:&[f64],ad:[f64;2],b:&[f64],bd:[f64;2])->Result<()> {
 use cad_predicates::{AuthoredScalar,SourceArena,ToleranceContext,PredicateContext,Limits,Outcome,Sign,orient2d};
 check(a.len()==b.len(),"Surface seam knot counts differ")?;
 let values:Vec<f64>=[ad[0],bd[0],ad[1],bd[1]].into_iter().chain(a.iter().zip(b).flat_map(|(&x,&y)|[x,y])).collect();
 let arena=SourceArena::authored("represented-surface-knot-basis",1,values.iter().map(|x|AuthoredScalar::Binary64Bits(x.to_bits())).collect()).map_err(|e|crate::input(format!("Invalid seam knot basis: {e:?}")))?;
 let tolerance=ToleranceContext::default_valid();let mut ctx=PredicateContext::new(&arena,&tolerance,Limits::default(),None);
 let pair=|i|[arena.leaf(i).unwrap(),arena.leaf(i+1).unwrap()];
 for i in 0..a.len(){let decision=orient2d(&mut ctx,pair(0),pair(2),pair(4+2*i)).map_err(|e|crate::input(format!("Seam knot predicate failed: {e:?}")))?;
  check(matches!(decision.outcome,Outcome::Sign(Sign::Zero)),"Surface boundaries need exactly affine-compatible seam knots")?;
 }Ok(())
}
/// Matches inward normalized derivatives as E'= -scale R', E''=scale^2 R''.
/// Positive weights make projection from the matched homogeneous jets valid.
/// C1/C2 parametric matching implies geometric continuity only on regular seams.
/// Callers must inspect regularityCertified before applying a geometric match.
pub fn match_surface_jets(reference:&Surface,edited:&Surface,reference_boundary:&str,edited_boundary:&str,order:usize,scale:f64)->Result<Value>{
 reference.validate()?;edited.validate()?;
 check(order==1||order==2,"Surface continuity order must be 1 or 2")?;
 check(scale.is_finite()&&scale>0.,"Surface normal scale must be positive and finite")?;
 let r=Boundary::parse(reference_boundary)?;let e=Boundary::parse(edited_boundary)?;
 let(rp,rn,rk,_)=r.along(reference);let(ep,en,ek,_)=e.along(edited);
 check(rp==ep&&rn==en,"Surface boundaries need compatible control counts and degrees")?;
 affine_knots(rk,[rk[rp],rk[rn]],ek,[ek[ep],ek[en]])?;
 let rc=r.coefficients(reference,order)?;let ec=e.coefficients(edited,order)?;
 let mut output=edited.clone();
 for i in 0..rn {
  let rh:Vec<H>=(0..=order).map(|l|r.get(reference,i,l)).collect();
  let mut eh=vec![rh[0]];
  for q in 1..=order {
   let multiplier=if q==1{-scale}else{scale*scale};
   let h=std::array::from_fn(|axis|{
    let target=multiplier*(0..=order).map(|l|rc[q][l]*rh[l][axis]).sum::<f64>();
    (target-(0..q).map(|l|ec[q][l]*eh[l][axis]).sum::<f64>())/ec[q][q]
   });eh.push(h);
  }
  let(ru,rv)=r.index(reference,i,0);let(eu,ev)=e.index(&output,i,0);
  output.control_points[eu][ev]=reference.control_points[ru][rv].clone();output.weights[eu][ev]=reference.weights[ru][rv];
  for (layer,h) in eh.into_iter().enumerate().skip(1){e.set(&mut output,i,layer,h)?;}
 }
 output.validate()?;
 let reference_regularity=regularity::certify(reference,r)?;let edited_regularity=regularity::certify(&output,e)?;
 let error_bounds=bounds::certify(reference,&output,r,e,order,scale)?;
 let regular=reference_regularity["certified"]==json!(true)&&edited_regularity["certified"]==json!(true);
 Ok(json!({"surface":output,"report":{"continuityOrder":order,"normalScale":scale,
  "method":"homogeneous-normalized-boundary-jets","regularityCertified":regular,"referenceRegularity":reference_regularity,"editedRegularity":edited_regularity,
  "errorBounds":error_bounds,"seamBasis":"exact-affine-knot-correspondence","modifiedLayers":order+1}}))
}

/// Reverse only the seam parameter while matching, then restore the edited
/// parameter direction and map diagnostic intervals back to its original domain.
pub fn match_surface_jets_oriented(reference:&Surface,edited:&Surface,reference_boundary:&str,edited_boundary:&str,order:usize,scale:f64,reverse:bool)->Result<Value>{
 if !reverse{return match_surface_jets(reference,edited,reference_boundary,edited_boundary,order,scale);}
 let edge=Boundary::parse(edited_boundary)?;
 let axis=if edge.cross_u{crate::surface::Axis::V}else{crate::surface::Axis::U};
 let aligned=edited.edit_axis(axis,|c|c.reverse())?;
 let mut value=match_surface_jets(reference,&aligned,reference_boundary,edited_boundary,order,scale)?;
 let output:Surface=value_codec::from_value(value["surface"].clone()).map_err(|e|crate::input(e.to_string()))?;
 let restored=output.edit_axis(axis,|c|c.reverse())?;
 let(p,n,ak,_)=edge.along(&aligned);let(_,_,rk,_)=edge.along(&restored);
 let reversed_knots:Vec<f64>=rk.iter().copied().rev().collect();
 affine_knots(ak,[ak[p],ak[n]],&reversed_knots,[rk[n],rk[p]])?;
 value["surface"]=value_codec::to_value(restored).map_err(|e|crate::input(e.to_string()))?;
 value["report"]["reversed"]=json!(true);
 if !value["report"]["editedRegularity"]["unresolvedIntervals"].is_null(){
  let(p,n,k,_)=edge.along(edited);let a=k[p];let b=k[n];
  let intervals:Vec<[f64;2]>=value_codec::from_value(value["report"]["editedRegularity"]["unresolvedIntervals"].clone()).map_err(|e|crate::input(e.to_string()))?;
  let intervals:Vec<[f64;2]>=intervals.into_iter().map(|d|[a+(b-d[1]),a+(b-d[0])]).collect();
  value["report"]["editedRegularity"]["unresolvedIntervals"]=json!(intervals);
 }
 Ok(value)
}

/// Application gate: require regularity, the requested tangential smoothness,
/// and a continuous bound on all independently varying jet components.
pub fn match_surface_jets_checked(reference:&Surface,edited:&Surface,reference_boundary:&str,edited_boundary:&str,order:usize,scale:f64,reverse:bool,max_error:f64)->Result<Value>{
 check(max_error.is_finite()&&max_error>=0.,"Surface matching tolerance must be finite and nonnegative")?;
 let mut value=match_surface_jets_oriented(reference,edited,reference_boundary,edited_boundary,order,scale,reverse)?;
 let mut smooth=true;
 for(s,name)in[(reference,reference_boundary),(edited,edited_boundary)]{
  let b=Boundary::parse(name)?;let(p,n,k,periodic)=b.along(s);let a=k[p];let z=k[n];
  for &u in k{if (u>a||periodic&&u==a)&&u<z&&k.iter().filter(|&&x|x==u).count()>p.saturating_sub(order){smooth=false;}}
 }
 let error=value["report"]["errorBounds"]["positionUpper"].as_f64().unwrap()
  .max(value["report"]["errorBounds"]["firstDerivativeUpper"].as_f64().unwrap())
  .max(value["report"]["errorBounds"]["secondDerivativeUpper"].as_f64().unwrap_or(0.))
  .max(value["report"]["errorBounds"]["mixedDerivativeUpper"].as_f64().unwrap_or(0.));
 let regular=value["report"]["regularityCertified"]==json!(true);
 let accepted=regular&&smooth&&error<=max_error;
 value["report"]["accepted"]=json!(accepted);value["report"]["maxError"]=json!(max_error);value["report"]["errorUpper"]=json!(error);
 value["report"]["tangentialSmoothnessCertified"]=json!(smooth);
 value["report"]["reason"]=json!(if !regular{"unproven-regularity"}else if !smooth{"unproven-tangential-smoothness"}else if error>max_error{"deviation-exceeds-budget"}else{"accepted"});
 Ok(value)
}

#[cfg(test)]mod tests {
 use super::*;
 fn patch()->Surface {
  Surface{degree_u:3,degree_v:3,knots_u:vec![2.,2.,2.,2.,5.,5.,5.,5.],knots_v:vec![-1.,-1.,-1.,-1.,3.,3.,3.,3.],
   control_points:(0..4).map(|i|(0..4).map(|j|vec![i as f64,j as f64,0.15*(i*i)as f64+0.1*(i*j)as f64+0.2*(j*j)as f64]).collect()).collect(),
   weights:(0..4).map(|i|(0..4).map(|j|1.+0.03*i as f64+0.02*j as f64+0.01*(i*j)as f64).collect()).collect(),periodic_u:false,periodic_v:false}
 }
 fn jet(s:&Surface,b:Boundary,t:f64)->Vec<Vec<f64>>{
  let(_,nc,kc,_)=b.cross(s);let(pc,_,_,_)=b.cross(s);let(ps,ns,ks,_)=b.along(s);
  let cross=kc[if b.max{nc}else{pc}];let along=ks[ps]+t*(ks[ns]-ks[ps]);
  let(u,v)=if b.cross_u{(cross,along)}else{(along,cross)};
  let value=value_codec::to_value(s.evaluate(u,v).unwrap()).unwrap();
  let cross_span=(kc[nc]-kc[pc])*if b.max{-1.}else{1.};let seam_span=ks[ns]-ks[ps];
  let fields=if b.cross_u{["point","du","duu","dv","dvv","duv"]}else{["point","dv","dvv","du","duu","duv"]};
  let factors=[1.,cross_span,cross_span*cross_span,seam_span,seam_span*seam_span,cross_span*seam_span];
  fields.iter().zip(factors).map(|(field,f)|{let a:Vec<f64>=value_codec::from_value(value[*field].clone()).unwrap();a.into_iter().map(|x|x*f).collect()}).collect()
 }
 fn close(a:&[f64],b:&[f64],factor:f64){for (x,y)in a.iter().zip(b){assert!((x-factor*y).abs()<2e-10,"{x} != {factor} * {y}");}}
 // This allowance covers the binary64 test evaluator, not the certified bound.
 // scripts/verify-surface-jets.py checks the bounds independently at 80 digits.
 fn evaluation_slack(a:&[f64],b:&[f64],factor:f64)->f64 {
  256.*f64::EPSILON*(1.+a.iter().map(|x|x.abs()).sum::<f64>()+factor.abs()*b.iter().map(|x|x.abs()).sum::<f64>())
 }
 #[test]fn rational_jets_match_all_boundary_pairs_in_normalized_coordinates(){
  let source=patch();let mut target=patch();for row in &mut target.control_points{for p in row{p[0]+=20.;p[2]-=3.;}}
  let before=value_codec::to_value(&target).unwrap();
  for rn in ["uMin","uMax","vMin","vMax"]{for en in ["uMin","uMax","vMin","vMax"]{for order in [1,2]{for scale in [0.4,1.,1.7]{
   let result=match_surface_jets(&source,&target,rn,en,order,scale).unwrap();
   assert_eq!(result["report"]["regularityCertified"],json!(true));
   let output:Surface=value_codec::from_value(result["surface"].clone()).unwrap();
   for t in [0.,0.13,0.5,0.83,1.]{
    let a=jet(&source,Boundary::parse(rn).unwrap(),t);let b=jet(&output,Boundary::parse(en).unwrap(),t);
    close(&b[0],&a[0],1.);close(&b[1],&a[1],-scale);close(&b[3],&a[3],1.);
    if order==2{close(&b[2],&a[2],scale*scale);close(&b[4],&a[4],1.);close(&b[5],&a[5],-scale);}
    for (field,key,factor) in [(0,"positionUpper",1.),(1,"firstDerivativeUpper",-scale)]{
     let residual=b[field].iter().zip(&a[field]).map(|(x,y)|(x-factor*y).powi(2)).sum::<f64>().sqrt();
     let bound=result["report"]["errorBounds"][key].as_f64().unwrap();
     assert!(residual<=bound+evaluation_slack(&b[field],&a[field],factor),"{key}: measured {residual}, bound {bound}");
    }
    if order==2{for(field,key,factor)in[(2,"secondDerivativeUpper",scale*scale),(5,"mixedDerivativeUpper",-scale)]{
     let residual=b[field].iter().zip(&a[field]).map(|(x,y)|(x-factor*y).powi(2)).sum::<f64>().sqrt();
     let bound=result["report"]["errorBounds"][key].as_f64().unwrap();
     assert!(residual<=bound+evaluation_slack(&b[field],&a[field],factor),"{key}: measured {residual}, bound {bound}");
    }}
   }
   let edge=Boundary::parse(en).unwrap();for i in 0..4{for layer in order+1..4{assert_eq!(edge.get(&output,i,layer),edge.get(&target,i,layer));}}
  }}}}
  assert_eq!(value_codec::to_value(&target).unwrap(),before);
 }
 #[test]fn handles_different_cross_degrees_and_nonuniform_end_spans(){
  let source=patch();let mut target=patch();
  target.degree_u=4;target.knots_u=vec![7.,7.,7.,7.,7.,9.,17.,17.,17.,17.,17.];
  target.control_points=(0..6).map(|i|source.control_points[i%4].clone()).collect();
  target.weights=(0..6).map(|i|source.weights[i%4].clone()).collect();
  for end in ["uMin","uMax"] {
   let value=match_surface_jets(&source,&target,"uMax",end,2,1.3).unwrap();
   let output:Surface=value_codec::from_value(value["surface"].clone()).unwrap();
   for i in 0..=32 {let t=i as f64/32.;let a=jet(&source,Boundary::parse("uMax").unwrap(),t);let b=jet(&output,Boundary::parse(end).unwrap(),t);
    close(&b[0],&a[0],1.);close(&b[1],&a[1],-1.3);close(&b[2],&a[2],1.3*1.3);close(&b[5],&a[5],-1.3);
   }
  }
  target=patch();target.knots_u=(-2..6).map(|i|i as f64).collect();
  assert!(match_surface_jets(&source,&target,"uMax","uMin",2,1.).is_err());
 }
 #[test]fn reversed_matching_restores_parameter_direction(){
  let source=patch();let edited=patch();
  for name in ["uMin","uMax","vMin","vMax"]{
   let value=match_surface_jets_oriented(&source,&edited,"uMax",name,2,1.,true).unwrap();
   let output:Surface=value_codec::from_value(value["surface"].clone()).unwrap();
   assert_eq!(output.knots_u,edited.knots_u);assert_eq!(output.knots_v,edited.knots_v);
   assert_eq!(value["report"]["regularityCertified"],json!(true));
   for t in [0.,0.2,0.5,0.9,1.]{let a=jet(&source,Boundary::parse("uMax").unwrap(),t);let b=jet(&output,Boundary::parse(name).unwrap(),1.-t);
    close(&b[0],&a[0],1.);close(&b[1],&a[1],-1.);close(&b[2],&a[2],1.);close(&b[3],&a[3],-1.);close(&b[4],&a[4],1.);close(&b[5],&a[5],1.);
   }
  }
 }
 #[test]fn rejects_rank_loss_between_sampling_sites(){
  let a=123_f64/1024.;let y=[0.,a*a,2.*a*a-a,1.-3.*a+3.*a*a];
  let mut source=patch();source.weights=vec![vec![1.;4];4];
  source.control_points=(0..4).map(|i|y.iter().map(|&v|vec![i as f64,v,0.]).collect()).collect();
  let result=match_surface_jets(&source,&patch(),"uMax","uMin",2,1.).unwrap();
  assert_eq!(result["report"]["regularityCertified"],json!(false));
  let root=-1.+4.*a;
  let intervals:Vec<[f64;2]>=value_codec::from_value(result["report"]["referenceRegularity"]["unresolvedIntervals"].clone()).unwrap();
  assert!(intervals.iter().any(|d|d[0]<=root&&root<=d[1]));
 }
 #[test]fn periodic_and_multispan_seams_are_proved_without_sampling(){
  let mut s=patch();s.degree_v=2;s.knots_v=(0..9).map(|i|i as f64).collect();s.periodic_v=true;
  s.control_points=(0..4).map(|i|[[1.,0.],[0.,1.],[-1.,0.],[0.,-1.],[1.,0.],[0.,1.]].iter().map(|p|vec![p[0],p[1],i as f64]).collect()).collect();
  s.weights=vec![vec![1.,0.8,1.2,1.,1.,0.8];4];
  let result=match_surface_jets(&s,&s,"uMax","uMin",2,1.).unwrap();
  assert_eq!(result["report"]["regularityCertified"],json!(true),"{result:?}");
  assert_eq!(result["report"]["referenceRegularity"]["spans"],json!(4));
  let mut broken=patch();broken.degree_v=1;broken.knots_v=vec![0.,0.,1.,2.,3.,3.];
  let result=match_surface_jets(&broken,&broken,"uMax","uMin",1,1.).unwrap();
  assert_eq!(result["report"]["regularityCertified"],json!(false));
  assert_eq!(result["report"]["referenceRegularity"]["reason"],json!("seam-basis-is-not-C1"));
 }
 #[test]fn application_gate_requires_a_finite_budget_and_second_order_seam_smoothness(){
  let s=patch();let accepted=match_surface_jets_checked(&s,&s,"uMax","uMin",2,1.,false,1e-8).unwrap();
  assert_eq!(accepted["report"]["accepted"],json!(true),"{accepted:?}");
  let refused=match_surface_jets_checked(&s,&s,"uMax","uMin",2,1.,false,0.).unwrap();
  assert_eq!(refused["report"]["accepted"],json!(false));
  assert_eq!(refused["report"]["reason"],json!("deviation-exceeds-budget"));
  assert!(match_surface_jets_checked(&s,&s,"uMax","uMin",2,1.,false,f64::NAN).is_err());
  let mut c=patch();c.degree_v=2;c.knots_v=vec![0.,0.,0.,0.5,1.,1.,1.];
  let result=match_surface_jets_checked(&c,&c,"uMax","uMin",2,1.,false,1e-6).unwrap();
  assert_eq!(result["report"]["accepted"],json!(false));
  assert_eq!(result["report"]["reason"],json!("unproven-tangential-smoothness"));
  assert_eq!(match_surface_jets_checked(&c,&c,"uMax","uMin",1,1.,false,1e-6).unwrap()["report"]["accepted"],json!(true));
 }
 #[test]fn rounded_normalized_knots_cannot_hide_an_incompatible_basis(){
  assert_eq!(1_f64/3.,0.3_f64/0.9);
  assert!(affine_knots(&[0.,1.,3.],[0.,3.],&[0.,0.3,0.9],[0.,0.9]).is_err());
  assert!(affine_knots(&[0.,1.,3.],[0.,3.],&[2.,4.,8.],[2.,8.]).is_ok());
 }
 #[test]fn refuses_incompatible_basis_and_nonpositive_result_weights(){
  let source=patch();let mut target=patch();
  for scale in [0.,-1.,f64::NAN,f64::INFINITY]{assert!(match_surface_jets(&source,&target,"uMax","uMin",1,scale).is_err());}
  assert!(match_surface_jets(&source,&target,"uMax","bad",1,1.).is_err());
  assert!(match_surface_jets(&source,&target,"uMax","uMin",3,1.).is_err());
  target.degree_v=2;target.knots_v=vec![-1.,-1.,-1.,1.,3.,3.,3.];
  assert!(match_surface_jets(&source,&target,"uMax","uMin",1,1.).is_err());
  let mut invalid=source.clone();invalid.weights[3]=vec![0.01;4];
  assert!(match_surface_jets(&invalid,&source,"uMax","uMin",1,1.).is_err());
 }
}

/// Inspect represented boundary jets without editing either input surface.
/// The seam basis must already agree; returned bounds concern these inputs.
pub fn inspect_surface_jets_checked_report(
    reference:&Surface, edited:&Surface, reference_boundary:&str,
    edited_boundary:&str, order:usize, scale:f64, max_error:f64,
)->Result<Value>{
    reference.validate()?;edited.validate()?;
    check((order==1||order==2)&&scale.is_finite()&&scale>0.&&max_error.is_finite()&&max_error>=0.,"Invalid surface jet inspection options")?;
    let r=Boundary::parse(reference_boundary)?;let e=Boundary::parse(edited_boundary)?;
    let (rp,rn,rk,_)=r.along(reference);let (ep,en,ek,_)=e.along(edited);
    check(rp==ep&&rn==en,"Surface jet inspection requires a common seam basis")?;
    affine_knots(rk,[rk[rp],rk[rn]],ek,[ek[ep],ek[en]])?;
    r.coefficients(reference,order.min(r.cross(reference).0))?;e.coefficients(edited,order.min(e.cross(edited).0))?;
    let reference_regularity=regularity::certify(reference,r)?;
    let edited_regularity=regularity::certify(edited,e)?;
    let error_bounds=bounds::certify(reference,edited,r,e,order,scale)?;
    let smooth=[(reference,r),(edited,e)].iter().all(|(s,b)|{
        let (p,n,k,periodic)=b.along(s);let a=k[p];let z=k[n];
        k.iter().all(|&u|!((u>a||periodic&&u==a)&&u<z&&k.iter().filter(|&&x|x==u).count()>p.saturating_sub(order)))
    });
    let regular=reference_regularity["certified"]==json!(true)&&edited_regularity["certified"]==json!(true);
    let error=["positionUpper","firstDerivativeUpper","secondDerivativeUpper","mixedDerivativeUpper"].iter().filter_map(|k|error_bounds[*k].as_f64()).fold(0_f64,f64::max);
    let accepted=regular&&smooth&&error<=max_error;
    Ok(json!({"continuityOrder":order,"normalScale":scale,"regularityCertified":regular,"referenceRegularity":reference_regularity,"editedRegularity":edited_regularity,"errorBounds":error_bounds,"accepted":accepted,"errorUpper":error,"maxError":max_error,"tangentialSmoothnessCertified":smooth,"method":"homogeneous-normalized-boundary-jets"}))
}
