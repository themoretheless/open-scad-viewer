use super::*;
use crate as nurbs;
#[path="../../../../../tests/support/contact_material_source.rs"] mod source;
#[path="../../../../../tests/support/closed_frenet_source.rs"] mod frenet_source;
use frenet_source::path;
fn options(arc:bool)->Options {Options{normal:[0.,1.,0.],orientation:Orientation::Frenet,
    spacing:if arc {Spacing::ArcLength{tolerance:1e-8,max_cells:10000}}else{Spacing::Parameter},
    initial_sections:5,max_sections:33,max_deviation:0.05}}
#[test]
fn closed_original_frenet_seam_owns_c4_path_and_whole_curvature_cover() {
    let p=path();let (profiles,_,_)=source::material(false,false);
    let scale=constant_vector_law([1.,0.,0.]).unwrap();let zero=constant_vector_law([0.;3]).unwrap();
    for arc in [false,true] {let sweep=MultiSweep::new(&profiles,&p,&scale,&zero,options(arc)).unwrap();
        for order in [1,2] {
            let q=sweep.certify_closed_path_frame_smoothness(order,100000,1000000).unwrap();
            assert!(q.closed_source_frame_smoothness_certified,"arc={arc} {q:?}");
            assert!(sweep.certify_closed_path_frame_smoothness(order,q.cells,q.exact_work).unwrap().closed_source_frame_smoothness_certified);
            for (cells,work) in [(0,q.exact_work),(q.cells-1,q.exact_work),(q.cells,0),(q.cells,q.exact_work-1)] {
                let r=sweep.certify_closed_path_frame_smoothness(order,cells,work).unwrap();
                assert!(!r.closed_source_frame_smoothness_certified && r.cells<=cells && r.exact_work<=work);
            }
            eprintln!("closed Frenet source seam arc={arc} order={order}: {q:?}");
            #[cfg(feature="codec")]
            if !arc && order==2 {record("positive-c4",&p,&zero);}
        }
    }
}
#[test]
fn closed_original_frenet_source_has_a_complete_global_body_certificate() {
    let p=path();let (profiles,cap,_)=source::material(false,false);
    let scale=source::periodic(1.,1.03125);let zero=constant_vector_law([0.;3]).unwrap();
    let sweep=MultiSweep::new(&profiles,&p,&scale,&zero,options(true)).unwrap();
    let projection=[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]];
    let q=sweep.certify_ideal_volume_atlas([&cap,&cap],projection,1000000,1000000).unwrap();
    assert!(q.embedding_certified && q.regularity_certified && q.parameter_topology_preserved,"{q:?}");
    eprintln!("closed Frenet actual global body={q:?}");
}

#[test]
fn closed_original_frenet_requires_actual_c4_not_only_matching_c3() {
    let (profiles,_,_)=source::material(false,false);let scale=constant_vector_law([1.,0.,0.]).unwrap();let zero=constant_vector_law([0.;3]).unwrap();
    let mut p=path();p.control_points[7][2]=p.control_points[7][2].next_up();
    let sweep=MultiSweep::new(&profiles,&p,&scale,&zero,options(false)).unwrap();
    assert!(sweep.certify_closed_path_frame_smoothness(1,100000,1000000).unwrap().closed_source_frame_smoothness_certified);
    let q=sweep.certify_closed_path_frame_smoothness(2,100000,1000000).unwrap();
    assert!(!q.closed_source_frame_smoothness_certified);assert_eq!(q.reason,Some("closed-frenet-path-jets-unproved"));
    #[cfg(feature="codec")]
    record("c3-only",&p,&zero);
    let p=path();let twist=Curve{degree:5,knots:[vec![0.;6],vec![1.;6]].concat(),
        control_points:[0.,0.,0.,1./1024.,0.,0.].map(|x|vec![x,0.,0.]).to_vec(),weights:vec![1.;6],periodic:false};
    let sweep=MultiSweep::new(&profiles,&p,&scale,&twist,options(false)).unwrap();
    assert!(sweep.certify_closed_path_frame_smoothness(1,100000,1000000).unwrap().closed_source_frame_smoothness_certified);
    let q=sweep.certify_closed_path_frame_smoothness(2,100000,1000000).unwrap();
    assert!(!q.closed_source_frame_smoothness_certified);assert_eq!(q.reason,Some("closed-frenet-twist-jets-unproved"));
    #[cfg(feature="codec")]
    record("twist-c1-only",&p,&twist);
}
#[test]
fn closed_original_frenet_perfect_seam_does_not_hide_zero_interior_curvature() {
    let mut p=path();for q in &mut p.control_points {q[1]*=21.;q[2]=0.;}p.control_points[5][1]=23.;p.control_points[6][1]=23.;p.weights.fill(1.);
    // Symmetric Y and antisymmetric X give P''(1/2)=0 exactly. Every
    // source endpoint derivative through order four still agrees exactly.
    let seam=super::super::super::endpoint_jets::certify(&p,4,1000000).unwrap();assert!(seam.certified);
    let (profiles,_,_)=source::material(false,false);let scale=constant_vector_law([1.,0.,0.]).unwrap();let zero=constant_vector_law([0.;3]).unwrap();
    let sweep=MultiSweep::new(&profiles,&p,&scale,&zero,options(false)).unwrap();
    let q=sweep.certify_closed_path_frame_smoothness(2,1000,1000000).unwrap();assert!(!q.closed_source_frame_smoothness_certified && q.cells<=1000);
    #[cfg(feature="codec")]
    record("zero-curvature",&p,&zero);
}

#[test]
fn closed_original_frenet_two_cover_has_c2_frame_but_no_global_embedding_credit() {
    let p=frenet_source::twice();let (profiles,cap,_)=source::material(false,false);
    let scale=constant_vector_law([1.,0.,0.]).unwrap();let zero=constant_vector_law([0.;3]).unwrap();
    let sweep=MultiSweep::new(&profiles,&p,&scale,&zero,options(false)).unwrap();
    let seam=sweep.certify_closed_path_frame_smoothness(2,100000,1000000).unwrap();
    assert!(seam.closed_source_frame_smoothness_certified,"{seam:?}");
    // The original path and all original Frenet/law data repeat after half a
    // traversal. Hence F(u,v,0)=F(u,v,1/2), distinct in the declared S1 domain.
    let q=sweep.certify_ideal_volume_atlas([&cap,&cap],[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]],20000,1000000).unwrap();
    assert!(!q.embedding_certified && !q.regularity_certified && !q.parameter_topology_preserved && q.cells<=20000,"{q:?}");
    eprintln!("closed Frenet genuine two-cover source: frame={seam:?} volume={q:?}");
    #[cfg(feature="codec")]
    record("double-cover",&p,&zero);
}

#[cfg(feature="codec")]
fn record(label:&str,path:&Curve,twist:&Curve) {
    let scale=constant_vector_law([1.,0.,0.]).unwrap();
    eprintln!("closed-frenet-source={}",value_codec::json!({"label":label,"path":path,"twist":twist,"scale":scale,"orientation":"frenet","clock":"parameter"}));
}
