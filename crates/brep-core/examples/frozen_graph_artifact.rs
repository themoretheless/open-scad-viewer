//! Compact standalone control artifact for real WASM/worker replay gates.
#[cfg(feature="codec")]
#[path="support/compatible_contact_fixture.rs"]
mod fixture;
#[cfg(feature="codec")]
fn main(){
 use brep_core::exact_graph_strip::{VolumeBounds,transport::{FrozenGraphVolume,RootCut,VolumeRecipe}};
 use nurbs_core::{curve::Curve,original_graph_transition::Budget};
 use num_rational::BigRational as Q;
 let recipe=||{
 let(source,request,_,_)=fixture::fixture();
 let plan=brep_core::region_replacement::plan_radius_network(&source,0,&[request],10000).unwrap();
 VolumeRecipe{source,supports:plan.edges[0].supports,widths:(0..2).map(|i|Curve{degree:1,knots:vec![i as f64,i as f64,i as f64+1.,i as f64+1.],control_points:vec![vec![0.25,0.,0.];2],weights:vec![1.;2],periodic:false}).collect(),cuts:vec![RootCut{root:1,parameter:Q::new(1.into(),3.into())},RootCut{root:1,parameter:Q::new(2.into(),3.into())}],tolerance_mm:1e-7,bounds:VolumeBounds::new(Q::from_integer(0.into()),Q::from_integer(40.into())).unwrap()}
 };
 let body=FrozenGraphVolume::construct(recipe(),100,&mut Budget::new(10_000_000).unwrap()).unwrap();
 let body=if std::env::args().any(|a|a=="--natural-cuts") {
  let roof=body.body().original_shell().inherited_roof();
  let cuts=roof.geometry().open_edges.iter().flat_map(|&edge|{let root=roof.topology().edges[edge].curve.root_id();[RootCut{root,parameter:Q::new(1.into(),3.into())},RootCut{root,parameter:Q::new(2.into(),3.into())}]}).collect();
  FrozenGraphVolume::construct_with_natural_cuts(recipe(),cuts,100,&mut Budget::new(10_000_000).unwrap()).unwrap()
 }else{body};
 println!("{}",value_codec::to_string(&body.export(4<<20).unwrap()).unwrap());
}
#[cfg(not(feature="codec"))]
fn main(){panic!("Frozen graph artifacts require codec support")}
