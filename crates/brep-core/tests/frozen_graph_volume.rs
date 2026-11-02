#![cfg(feature="codec")]
#[path="../examples/support/compatible_contact_fixture.rs"]
mod fixture;
use brep_core::exact_graph_strip::{VolumeBounds,transport::{FrozenGraphVolume,RootCut,VolumeRecipe}};
use nurbs_core::{curve::Curve,original_graph_transition::Budget};
use num_rational::BigRational as Q;
use value_codec::{Value,json};
fn budget()->Budget{Budget::new(10_000_000).unwrap()}
fn recipe()->VolumeRecipe{
 let(source,request,_,_)=fixture::fixture();
 let plan=brep_core::region_replacement::plan_radius_network(&source,0,&[request],10000).unwrap();
 VolumeRecipe{source,supports:plan.edges[0].supports,widths:(0..2).map(|i|Curve{degree:1,knots:vec![i as f64,i as f64,i as f64+1.,i as f64+1.],control_points:vec![vec![0.25,0.,0.];2],weights:vec![1.;2],periodic:false}).collect(),
 cuts:vec![RootCut{root:1,parameter:Q::new(1.into(),3.into())},RootCut{root:1,parameter:Q::new(2.into(),3.into())}],tolerance_mm:1e-7,bounds:VolumeBounds::new(Q::from_integer(0.into()),Q::from_integer(40.into())).unwrap()}
}
#[test]
fn exact_body_roundtrips_binary_and_json_with_nondyadic_shared_root_cuts(){
 let r=recipe();let original=r.source.clone();let body=FrozenGraphVolume::construct(r,100,&mut budget()).unwrap();
 let artifact=body.export(4<<20).unwrap();
 let bytes=value_codec::encode_binary(&artifact).unwrap();
 let parsed=value_codec::decode_binary(&bytes).unwrap();
 let restored=FrozenGraphVolume::restore(&parsed,4<<20,100,&mut budget()).unwrap();
 assert_eq!(restored.export(4<<20).unwrap(),artifact);
 assert_eq!(restored.body().volume(),body.body().volume());
 assert_eq!(restored.body().original_shell().original_source(),&original);
 assert!(!artifact["geometry"]["identityLinks"]["edges"].as_array().unwrap().is_empty());
 assert_eq!(restored.body().original_shell().source_vertex_links(),body.body().original_shell().source_vertex_links());
 assert_eq!(restored.body().original_shell().source_face_links(),body.body().original_shell().source_face_links());
 assert_eq!(restored.body().topology().bodies.len(),1);
 assert_eq!(restored.body().topology().faces.len(),9);
 let root=restored.body().topology().edges.iter().filter(|e|e.curve.root_id()==1).collect::<Vec<_>>();
 assert_eq!(root.len(),3);assert!(root.iter().any(|e|e.curve.root_parameters()==&[Q::new(1.into(),3.into()),Q::new(2.into(),3.into())]));
 let text=value_codec::to_string(&artifact).unwrap();let json:Value=value_codec::from_str(&text).unwrap();
 assert_eq!(FrozenGraphVolume::restore(&json,4<<20,100,&mut budget()).unwrap().export(4<<20).unwrap(),artifact);
 assert_eq!(artifact["geometry"]["applicationAdmitted"],false);
 assert_eq!(artifact["geometry"]["radiusLawCertified"],false);
}
#[test]
fn tampered_geometry_recipe_and_false_admission_are_refused(){
 let artifact=FrozenGraphVolume::construct(recipe(),100,&mut budget()).unwrap().export(4<<20).unwrap();
 let mut changed=artifact.clone();changed["geometry"]["signedVolume"]=json!(["1","1"]);
 assert!(FrozenGraphVolume::restore(&changed,4<<20,100,&mut budget()).is_err());
 let mut changed=artifact.clone();changed["geometry"]["applicationAdmitted"]=json!(true);
 assert!(FrozenGraphVolume::restore(&changed,4<<20,100,&mut budget()).is_err());
 let mut changed=artifact.clone();changed["geometry"]["identityLinks"]["edges"][0]["sourceId"]=json!("changed-source-id");
 assert!(FrozenGraphVolume::restore(&changed,4<<20,100,&mut budget()).is_err());
 let mut changed=artifact.clone();changed["recipe"]["profile"]=json!("future-unqualified-profile");
 assert!(FrozenGraphVolume::restore(&changed,4<<20,100,&mut budget()).is_err());
 let mut changed=artifact.clone();changed["recipe"]["cuts"][1]["at"]=json!(["1","3"]);
 assert!(FrozenGraphVolume::restore(&changed,4<<20,100,&mut budget()).is_err());
 let mut changed=artifact;changed["recipe"]["cuts"][0]["at"]=json!(["2","6"]);
 assert!(FrozenGraphVolume::restore(&changed,4<<20,100,&mut budget()).is_err());
}
#[test]
fn restore_never_borrows_success_from_a_previous_receipt_when_limits_run_out(){
 let artifact=FrozenGraphVolume::construct(recipe(),100,&mut budget()).unwrap().export(4<<20).unwrap();
 assert!(FrozenGraphVolume::restore(&artifact,4<<20,100,&mut Budget::new(0).unwrap()).is_err());
 assert!(FrozenGraphVolume::restore(&artifact,1,100,&mut budget()).is_err());
 assert!(FrozenGraphVolume::restore(&artifact,4<<20,1,&mut budget()).is_err());
 let mut invalid=recipe();invalid.tolerance_mm=0.1;
 assert!(FrozenGraphVolume::construct(invalid,100,&mut budget()).is_err());
}
