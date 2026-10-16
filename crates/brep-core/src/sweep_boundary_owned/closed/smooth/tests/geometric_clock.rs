//! Original geometric-clock source and retained Solid ownership.
use super::*;
use nurbs_core as nurbs;
#[path="../../../../../../nurbs-core/tests/support/arc_bishop_source.rs"]
mod arc_source_case;
#[path="../../../../../../nurbs-core/tests/support/general_frame_source.rs"]
mod general_frame_case;
#[test]
fn geometric_arc_source_clock_has_complete_original_and_actual_g2_solid_with_global_atlases(){
    let (profiles,surface,trims)=arc_source_case::material();let material=LoftCap{surface,trims};
    let path=arc_source_case::path(true);
    let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
    let sweep=MultiSweep::new(&profiles,&path,&scale,&twist,super::super::super::tests::options(true,Spacing::ArcLength{tolerance:0.001,max_cells:100000})).unwrap()
        .with_spatial_rmf_error_limits(4096,100000,1000000).unwrap();
    let mut l=limits();
    l.source.ideal.cells=1000000;
    l.original_smoothness=Some(nurbs_core::progressive_sweep::OriginalBoundarySmoothnessLimits{order:2,cells:100000,exact_work:1000000,products:1000000});
    let attempt=construct_smooth_closed_with_ideal_atlas(&sweep,&[4],65,&material,1e-8,0.05,&[],l).unwrap();
    if let SmoothClosedAttempt::Refused(r)=&attempt {eprintln!("original diagnostic {:?}, cells={} work={} smooth={:?}",r.original.volume,r.original.cells,r.original.exact_work,r.original.smoothness); eprintln!("wall diagnostic {:?}",r.original.wall.as_ref().map(|w|(w.error_certificate_reason,w.error_certificate_cells,w.continuous_error_upper)));}
    let b=body(attempt);
    let r=b.report();assert!(r.certified && r.original.certified && r.topology_preserved && r.shell_roles_certified && r.continuous_bound);
    assert!(r.original.g2_certified());
    assert!(r.smoothness.as_ref().unwrap().g2_certified);
    assert!(r.original.volume.as_ref().unwrap().embedding_certified && r.volume.as_ref().unwrap().embedding_certified);
    eprintln!("geometric clock original/actual G2 Solid; faces={} E={:?} source={:?} actual={:?}",b.model().faces.len(),r.boundary_error_upper,r.original.volume,r.volume);
}
#[test]
fn tilted_authored_geometric_clock_has_complete_original_and_actual_g2_solid(){
    let (profiles,surface,trims)=general_frame_case::material();let material=LoftCap{surface,trims};
    let path=general_frame_case::path();let (a,n,twist)=general_frame_case::laws();
    let scale=constant_vector_law([1.,0.,0.]).unwrap();
    let mut options=super::super::super::tests::options(true,Spacing::ArcLength{tolerance:0.001,max_cells:100000});
    options.orientation=nurbs_core::progressive_sweep::Orientation::Fixed;
    let sweep=MultiSweep::new(&profiles,&path,&scale,&twist,options).unwrap().with_frame_laws(&a,&n).unwrap();
    let mut l=limits();l.source.ideal.cells=1000000;
    l.original_smoothness=Some(nurbs_core::progressive_sweep::OriginalBoundarySmoothnessLimits{order:2,cells:100000,exact_work:1000000,products:1000000});
    let attempt=construct_smooth_closed_with_ideal_atlas(&sweep,&[4],65,&material,1e-8,0.05,&[],l).unwrap();
    if let SmoothClosedAttempt::Refused(r)=&attempt {eprintln!("tilted source={:?} wall={:?} volume={:?} smooth={:?}",r.original.reason,r.original.wall,r.original.volume,r.original.smoothness);}
    let b=body(attempt);let r=b.report();
    assert!(r.certified && r.original.g2_certified() && r.topology_preserved && r.shell_roles_certified && r.continuous_bound);
    assert!(r.smoothness.as_ref().unwrap().g2_certified);
    assert!(r.original.volume.as_ref().unwrap().embedding_certified && r.volume.as_ref().unwrap().embedding_certified);
    eprintln!("tilted authored original/actual G2 Solid; faces={} E={:?} source={:?} actual={:?}",b.model().faces.len(),r.boundary_error_upper,r.original.volume,r.volume);
}

#[test]
fn guided_geometric_clock_has_complete_original_and_actual_g2_solid() {
    let (profiles,surface,trims)=arc_source_case::material();let material=LoftCap{surface,trims};
    let path=arc_source_case::path(true);let mut guide=path.clone();
    for point in &mut guide.control_points {point[2]+=0.25;}
    let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
    let sweep=MultiSweep::new(&profiles,&path,&scale,&twist,
        super::super::super::tests::options(true,Spacing::ArcLength{tolerance:0.001,max_cells:100000})).unwrap()
        .with_orientation_guide(&guide).unwrap();
    let mut l=limits();l.source.ideal.cells=1000000;
    l.original_smoothness=Some(nurbs_core::progressive_sweep::OriginalBoundarySmoothnessLimits{
        order:2,cells:100000,exact_work:1000000,products:1000000});
    let attempt=construct_smooth_closed_with_ideal_atlas(&sweep,&[4],65,&material,1e-8,0.05,&[],l).unwrap();
    if let SmoothClosedAttempt::Refused(r)=&attempt {eprintln!("guided source={:?} wall={:?} volume={:?} smooth={:?}",r.original.reason,r.original.wall,r.original.volume,r.original.smoothness);}
    let b=body(attempt);let r=b.report();
    assert!(r.certified && r.original.g2_certified() && r.topology_preserved && r.shell_roles_certified && r.continuous_bound);
    assert!(r.smoothness.as_ref().unwrap().g2_certified);
    assert!(r.original.volume.as_ref().unwrap().embedding_certified && r.volume.as_ref().unwrap().embedding_certified);
    eprintln!("guided original/actual G2 Solid; faces={} E={:?} source={:?} actual={:?}",b.model().faces.len(),r.boundary_error_upper,r.original.volume,r.volume);
}
