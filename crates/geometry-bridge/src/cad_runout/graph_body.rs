//! Fresh native body reconstruction; this is not fillet/application admission.
use crate::{Result,Value,field,input};
use brep_core::exact_graph_strip::transport::FrozenGraphVolume;
use nurbs_core::original_graph_transition::Budget;
use value_codec::json;
pub(super) fn restore(v:Value)->Result<Value>{restore_impl(v,false)}
pub(super) fn display(v:Value)->Result<Value>{restore_impl(v,true)}
fn restore_impl(v:Value,display:bool)->Result<Value>{
 let object=v.as_object().ok_or_else(||input("Expected graph body restore request"))?;
 let keys:&[&str]=if display{&["op","artifact","maxBytes","maxWork","maxPairs","segments"]}else{&["op","artifact","maxBytes","maxWork","maxPairs"]};
 if object.len()!=keys.len()||object.keys().any(|k|!keys.contains(&k.as_str())){return Err(input("Unexpected or missing graph body restore field"));}
 let max_bytes=field::<usize>(&v,"maxBytes")?;
 let max_work=field::<usize>(&v,"maxWork")?;
 let max_pairs=field::<usize>(&v,"maxPairs")?;
 if max_pairs==0||max_pairs>1_000_000{return Err(input("Invalid graph body pair allowance"));}
 let mut b=Budget::new(max_work)?;
 let body=FrozenGraphVolume::restore(&v["artifact"],max_bytes,max_pairs,&mut b)?;
 let native=body.body();let top=native.topology();
 let roof=native.original_shell().inherited_roof();
 let maximum=roof.transition_fields().iter().filter_map(|r|r.absolute_curvature_upper)
  .chain(roof.source_fields().iter().map(|r|r.absolute_curvature_upper))
  .chain(native.original_shell().imported_fields().iter().map(|r|r.absolute_curvature_upper)).fold(0f64,f64::max);
 let n=top.faces.len();let pairs=n*(n-1)/2;
 let mut display_edges=Vec::new();let mut storage_error=0f64;
 if display{
  let segments=field::<usize>(&v,"segments")?;
  if !(1..=64).contains(&segments)||top.edges.len().saturating_mul(segments+1)>32768{return Err(input("Graph body display sample allowance exceeded"));}
  for (index,edge) in top.edges.iter().enumerate(){
   let sampled=nurbs_core::original_graph_transition::sample_polynomial_edge(edge.curve.points(),segments,&mut b)?;
   storage_error=storage_error.max(sampled.sample_storage_error_upper_mm);
   display_edges.push(json!({"edge":index,"root":edge.curve.root_id(),"points":sampled.points}));
  }
 }
 let report=json!({"method":"native-frozen-graph-body-restore/1","nativeVolumeCertified":true,"globalEmbeddingCertified":true,
 "faces":n,"edges":top.edges.len(),"vertices":top.vertices.len(),"bodies":top.bodies.len(),"openEdges":0,
 "pairProducts":pairs,"allPairsReproved":native.qualification().embedding().globally_embedded(),
 "internalG2Joins":roof.transition_joins().len(),"curvatureLimitPerMm":10.,"absoluteCurvatureUpper":maximum,
 "signedVolumeMm3":[native.volume().numer().to_string(),native.volume().denom().to_string()],
 "coefficientWork":b.used(),"maxWork":max_work,"maxPairs":max_pairs,"maxBytes":max_bytes,
 "radiusLawCertified":false,"thicknessCertified":false,"applicationAdmitted":false});
 let mut result=json!({"representation":"native-exact-graph-body","artifact":body.export(max_bytes)?,"report":report});
 if display{result["display"]=json!({"method":"native-exact-graph-edges/1","segments":field::<usize>(&v,"segments")?,"edges":display_edges,"sampleStorageErrorUpperMm":storage_error,"polylineDeviationCertified":false});}
 Ok(result)
}
#[cfg(test)]
mod tests {
 use super::*;
 use std::io::Read;
 fn request()->Value{
  let mut text=String::new();
  flate2::read::GzDecoder::new(&include_bytes!("../../../../tests/fixtures/frozen-graph-body.json.gz")[..]).read_to_string(&mut text).unwrap();
  let artifact:Value=value_codec::from_str(&text).unwrap();
  json!({"op":"brep_frozen_graph_body_restore","artifact":artifact,"maxBytes":4<<20,"maxWork":10000000,"maxPairs":100})
 }
 #[test]
 fn dispatched_restore_reproves_exact_body_without_fillet_or_application_claim(){
  let v=request();let before=v.clone();let r=crate::dispatch(v.clone()).unwrap();
  assert_eq!(r["representation"],"native-exact-graph-body");
  assert_eq!(r["artifact"],v["artifact"]);
  assert_eq!(r["report"]["faces"],9);assert_eq!(r["report"]["pairProducts"],36);
  for key in ["nativeVolumeCertified","globalEmbeddingCertified","allPairsReproved"]{assert_eq!(r["report"][key],true);}
  for key in ["radiusLawCertified","thicknessCertified","applicationAdmitted"]{assert_eq!(r["report"][key],false);}
  assert!(r["report"]["absoluteCurvatureUpper"].as_f64().unwrap()<=10.);assert_eq!(v,before);
 }
 #[test]
 fn restore_refuses_tampered_artifacts_and_exhausted_work(){
  let mut v=request();v["artifact"]["geometry"]["applicationAdmitted"]=json!(true);assert!(crate::dispatch(v).is_err());
  let mut v=request();v["maxWork"]=json!(0);assert!(crate::dispatch(v).is_err());
  let mut v=request();v["applicationAdmitted"]=json!(true);assert!(crate::dispatch(v).is_err());
 }
 #[test]
 fn native_display_keeps_edge_indices_and_closed_endpoint_samples(){
  let mut v=request();v["op"]=json!("brep_frozen_graph_body_display");v["segments"]=json!(8);
  let result=crate::dispatch(v.clone()).unwrap();
  let lines=result["display"]["edges"].as_array().unwrap();
  let g=&result["artifact"]["geometry"];
  assert_eq!(lines.len(),g["edges"].as_array().unwrap().len());
  for (i,line) in lines.iter().enumerate(){
   assert_eq!(line["edge"],i);assert_eq!(line["root"],g["edges"][i]["root"]);
   assert_eq!(line["points"].as_array().unwrap().len(),9);
  }
  assert_eq!(result["display"]["polylineDeviationCertified"],false);
  assert_eq!(result["report"]["applicationAdmitted"],false);
  v["segments"]=json!(0);assert!(crate::dispatch(v).is_err());
 }

}
