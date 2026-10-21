use nurbs_core::{curve::Curve,progressive_sweep::{Options,Orientation,Spacing,constant_vector_law}};
use brep_core::{analytic::{EndpointCapCorrection,progressive_profile_body_with_automatic_caps},retained_annular_cap::reference_filling::Limits};
fn main(){
 let path=Curve{degree:3,knots:vec![0.,0.,0.,0.,0.5,1.,1.,1.,1.],
  control_points:vec![vec![0.,0.,0.],vec![0.16666666666666666,0.,0.],vec![0.5,0.16666666666666666,0.],vec![0.8333333333333334,0.6666666666666666,0.5],vec![1.,1.,1.125]],weights:vec![1.;5],periodic:false};
 let loops=vec![vec![nurbs_core::primitives::circle([0.;3],[1.,0.,0.],0.1).unwrap()],vec![nurbs_core::primitives::circle([0.;3],[-1.,0.,0.],0.05).unwrap()]];
 let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
 let policy=Limits{exact_work:1000000,products:100000,trim_pairs:10000,trim_cells:100000,trim_domain_cells:1000000,spans:1024,linear_cells:100000,
  contacts:brep_core::face_contacts::Limits{pairs:10000,cells:100000,domain_cells:1000000,cells_per_pair:1000,domain_cells_per_pair:10000}};
 let cap_policy=brep_core::sweep_cap_contacts::Budgets{max_walls:1024,max_exact_work:1000000,max_chart_cells:1000,max_trim_pairs:100000,max_trim_cells:100000,max_trim_domain_cells:1000000};
 let volume_policy=brep_core::volume_validity::Limits{
  boundary:brep_core::boundary_embedding::Limits{exact_work:policy.exact_work,trim_pairs:policy.trim_pairs,trim_cells:policy.trim_cells,trim_domain_cells:policy.trim_domain_cells,spans:policy.spans,contacts:brep_core::face_contacts::Limits{pairs:20000,cells:200000,domain_cells:1000000,cells_per_pair:1000,domain_cells_per_pair:10000}},
  nesting_pairs:1000,nesting_cells:100000,nesting_domain_cells:1000000,orientation_cells:100000,orientation_domain_cells:1000000,orientation_spans:100};
 for spacing in [Spacing::Parameter,Spacing::ArcLength{tolerance:0.001,max_cells:100000}]{
  let options=Options{orientation:Orientation::CorrectedFrenet,normal:[0.,0.,1.],spacing,initial_sections:3,max_sections:129,max_deviation:0.05};
  let correction=EndpointCapCorrection{quantum:2_f64.powi(-40),tolerance:1e-9,max_work:1000000};
  let start=std::time::Instant::now();
  let body=progressive_profile_body_with_automatic_caps(&loops,&path,&scale,&twist,None,None,None,options,Some(correction),Some(correction),None,policy).unwrap();
  let bound=body.complete_boundary().expect("Filled corrected cap/source wall union");assert!(bound.error_upper.unwrap()<=0.05);assert!(bound.filled_cap_error_upper.unwrap().into_iter().all(|v|v<=0.05));
  assert!(body.evidence().cap_correction_error_upper.unwrap()<=1e-9);
  let volume=brep_core::volume_validity::inspect_sweep(body.model(),1e-8,volume_policy,100000,&body.planar_cap_faces(),cap_policy).unwrap();
  assert!(volume.proven,"Fresh global retained-body proof must pass");
  let mut changed=body.model().clone();let pole=&mut changed.faces[0].surface.control_points[0][0];let coordinate=pole.iter_mut().find(|v|**v!=0.).unwrap();*coordinate=coordinate.next_up();
  assert!(!brep_core::volume_validity::inspect_sweep(&changed,1e-8,volume_policy,100000,&body.planar_cap_faces(),cap_policy).is_ok_and(|v|v.proven));
  let short=EndpointCapCorrection{max_work:1,..correction};
  assert!(progressive_profile_body_with_automatic_caps(&loops,&path,&scale,&twist,None,None,None,options,Some(short),Some(correction),None,policy).is_err());
  println!("{spacing:?}: faces={}, corrected full boundary={}, fresh Solid and one-ULP/short-work refusals passed, elapsed={:?}",body.model().faces.len(),bound.error_upper.unwrap(),start.elapsed());
 }
}
