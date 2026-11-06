use brep_core::{face_contacts,boundary_embedding,volume_validity};
fn main(){
 let args:Vec<_>=std::env::args().collect();let model:brep_core::Model=value_codec::from_str(&std::fs::read_to_string(&args[1]).unwrap()).unwrap();
 let limits=volume_validity::Limits{boundary:boundary_embedding::Limits{exact_work:1000000,trim_pairs:10000,trim_cells:100000,trim_domain_cells:1000000,spans:1000,contacts:face_contacts::Limits{pairs:10000,cells:100000,domain_cells:1000000,cells_per_pair:1000,domain_cells_per_pair:10000}},nesting_pairs:100,nesting_cells:100000,nesting_domain_cells:1000000,orientation_cells:100000,orientation_domain_cells:1000000,orientation_spans:100};
 let report=volume_validity::inspect_with_linear(&model,1e-8,limits,10000).unwrap();
 println!("proven={} faces={:?}\npairs={:#?}\nnesting={} orientations={:?}",report.proven,report.boundary.intersections.faces,report.boundary.intersections.pairs,report.nesting.as_ref().is_some_and(|r|r.roles_consistent==Some(true)),report.orientations.iter().map(|r|(r.shell,r.outward)).collect::<Vec<_>>());
}
