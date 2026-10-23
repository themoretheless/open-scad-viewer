//! Fresh closed-body sewing, all-seam G2, embedding, and STEP evidence.
use brep_core::{analytic::g2_annular,rounded_patch_network,sweep_smoothness};
use value_codec::json;
use sha2::{Sha256,Digest};
fn main()->Result<(),Box<dyn std::error::Error>> {
 let output=std::path::PathBuf::from(std::env::args().nth(1).expect("Output directory"));std::fs::create_dir_all(&output)?;
 let mut cases=Vec::new();
 for (name,outer,inner,height,extents,scale) in [
  ("closed-small-transition",10.,7.,20.,[0.1;4],[1.,1.,1.]),
  ("closed-independent-transitions",10.,7.,20.,[0.1,0.25,0.5,0.75],[1.,1.,1.]),
  ("closed-elliptical-transitions",8.,4.,16.,[0.5;4],[2.,0.5,1.]),
 ] {
  let built=g2_annular::cylinder_with_transitions(outer,inner,height,extents,1e-8)?;
  let mut surfaces=built.model.faces.iter().map(|f|f.surface.clone()).collect::<Vec<_>>();
  for s in &mut surfaces{for p in s.control_points.iter_mut().flatten(){for k in 0..3{p[k]*=scale[k];}}}
  let mut reversed=vec![false;surfaces.len()];for u in &built.model.shells[0].faces{reversed[u.face]=u.reversed;}
  let sewn=rounded_patch_network::assemble(&surfaces,&reversed,1e-8,1000000,&[&built.model])?;
  let jets=sweep_smoothness::inspect_profile(&sewn.model,&[],2000000)?;
  let volume=sewn.qualify(&[])?;let pairs=&volume.boundary.intersections.pairs;
  eprintln!("{name}: G2={}/{} global={} pairs={}/{} sewing_work={} smoothness_work={}",jets.profile.g2_certified,jets.station.g2_certified,volume.proven,pairs.pairs.len()+pairs.grouped_pairs,pairs.total_pairs,sewn.correspondence_work,jets.total_exact_work);
  assert!(jets.profile.g2_certified&&jets.station.g2_certified&&volume.proven);
  let (step,_,_)=brep_core::export_step_v5(&sewn.model)?;let file=format!("{name}.step");std::fs::write(output.join(&file),&step)?;
  std::fs::write(output.join(format!("{name}-model.json")),value_codec::to_string(&sewn.model)?)?;
  let sha=Sha256::digest(step.as_bytes()).iter().map(|b|format!("{b:02x}")).collect::<String>();
  cases.push(json!({"name":name,"file":file,"sha256":sha,"parameters":{"outer":outer,"inner":inner,"height":height,"extent":extents[0],"extents":extents,"scale":scale},
   "profileG2":jets.profile.g2_certified,"stationG2":jets.station.g2_certified,"globalProven":volume.proven,"allPairsClassified":pairs.all_pairs_classified,
   "coveredPairs":pairs.pairs.len()+pairs.grouped_pairs,"totalPairs":pairs.total_pairs,"nextPair":pairs.next_pair,"correspondenceWork":sewn.correspondence_work,"exactWork":jets.total_exact_work}));
  std::fs::write(output.join("native.json"),value_codec::to_string_pretty(&json!({"cases":cases}))?)?;
 }
 Ok(())
}
