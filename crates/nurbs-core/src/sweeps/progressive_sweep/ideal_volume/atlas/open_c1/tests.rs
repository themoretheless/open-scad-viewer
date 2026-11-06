use super::*;
fn line(a:[f64;3],b:[f64;3])->Curve {
    Curve{degree:1,knots:vec![0.,0.,1.,1.],control_points:vec![a.to_vec(),b.to_vec()],weights:vec![1.;2],periodic:false}
}
fn carrier()->Surface {
    // X=u+4uv, Y=v+(1-(1-u)^8)/2. All original poles are dyadic.
    // det(DUV)=1+4v-16u(1-u)^7 > 1-2(7/8)^7 > 0.
    // Its endpoint Jacobians have a singular convex combination, so no
    // single interval-Jacobian or whole-field SPD proof can cover the box.
    Surface{degree_u:8,degree_v:1,knots_u:[vec![-3.;9],vec![5.;9]].concat(),knots_v:vec![7.,7.,11.,11.],
        control_points:(0..=8).map(|i|(0..=1).map(|j|vec![i as f64/8.*(1.+4.*j as f64),j as f64+if i==0 {0.}else{0.5},0.]).collect()).collect(),
        weights:vec![vec![1.;2];9],periodic_u:false,periodic_v:false}
}
fn profiles(s:&Surface)->Vec<Curve> {
    let u=|j:usize,rev:bool|{let mut p=s.control_points.iter().map(|r|r[j].clone()).collect::<Vec<_>>();if rev {p.reverse();}
        Curve{degree:8,knots:[vec![0.;9],vec![1.;9]].concat(),control_points:p,weights:vec![1.;9],periodic:false}};
    vec![u(0,false),line(s.control_points[8][0].clone().try_into().unwrap(),s.control_points[8][1].clone().try_into().unwrap()),
        u(1,true),line(s.control_points[0][1].clone().try_into().unwrap(),s.control_points[0][0].clone().try_into().unwrap())]
}
fn report()->IdealVolumeReport {
    IdealVolumeReport{embedding_certified:false,regularity_certified:false,parameter_topology_preserved:false,
        cells:0,exact_work:0,boxes:0,monotonicity_lower:None,projection:[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]],reason:Some("ideal-volume-work-limit")}
}
#[test]
fn original_open_c1_p_matrix_proves_a_complete_nonmonotone_filled_source() {
    let a=carrier();let mut b=a.clone();for p in b.control_points.iter_mut().flatten(){p[0]+=1./64.;p[1]+=1./64.;}
    let profiles=profiles(&a);let path=line([0.;3],[0.,0.,1.]);
    let scale=constant_vector_law([1.,0.,0.]).unwrap();let zero=constant_vector_law([0.;3]).unwrap();
    let sweep=MultiSweep::new(&profiles,&path,&scale,&zero,Options{normal:[1.,0.,0.],orientation:Orientation::FixedNormal,
        spacing:Spacing::Parameter,initial_sections:5,max_sections:17,max_deviation:0.05}).unwrap();
    let run=|max,work| {
        let mut out=report();let difference=cap_difference::Difference::prepare([&a,&b],&mut out.cells,max).unwrap();
        let context=cell::Context{s:&sweep.sweeps[0],caps:[&a,&b],cap_identity:None,cap_difference:difference,
            basis:report().projection.map(|v|v.map(I::point)),start:[I::point(0.);3],zero:zero.clone(),offset:None,
            anchor:None,transport:None,arc:None,guide_arc:None,shared_guide:false,poses:Some(Default::default()),cap_jets:None};
        let ok=certify(&context,report().projection,&mut out,max,work).unwrap();(ok,out)
    };
    let (ok,full)=run(10000,100000);assert!(ok,"{full:?}");
    assert!(full.embedding_certified && full.regularity_certified && full.parameter_topology_preserved);
    assert!(run(full.cells,full.exact_work).0);
    for (cells,work) in [(0,full.exact_work),(full.cells-1,full.exact_work),(full.cells,0),(full.cells,full.exact_work-1)] {
        let (ok,q)=run(cells,work);assert!(!ok && !q.embedding_certified && !q.regularity_certified && !q.parameter_topology_preserved);
        assert!(q.cells<=cells && q.exact_work<=work);
    }
    let q=sweep.certify_ideal_volume_atlas([&a,&b],report().projection,100000,1000000).unwrap();
    assert!(q.embedding_certified && q.regularity_certified && q.parameter_topology_preserved,"{q:?}");
    eprintln!("original C1 P-map primitive={full:?}; public={q:?}");
    #[cfg(feature="codec")]
    eprintln!("original-open-c1-source={}",value_codec::json!({"caps":[a,b],"path":path,"scale":scale,"twist":zero,"normal":[1.,0.,0.],"clock":"parameter","orientation":"fixed_normal"}));
}

#[test]
fn original_open_c1_p_matrix_frames_guides_contact_and_arc_own_c2_volume_charts() {
    use crate::sweeps::progressive_sweep::original_smooth_volume::{OriginalSmoothVolumeAttempt,OriginalSmoothVolumeLimits};
    let a=carrier();let mut b=a.clone();for (i,row) in b.control_points.iter_mut().enumerate(){for (j,p) in row.iter_mut().enumerate(){
        p[0]+=1./64.;p[1]+=1./64.;p[2]=(i*j) as f64/(8.*4096.);
    }}
    let profiles=profiles(&a);let path=line([0.;3],[0.,0.,1.]);let zero=constant_vector_law([0.;3]).unwrap();
    let e=1./4096.;let scale=line([1.,0.,0.],[1.+e,0.,0.]);
    let axes=line([1.,1.,1.],[1.+e,1.-e,1.]);let center=line([0.;3],[e,e,0.]);
    let longitudinal=line([0.,0.,1.],[e,0.,1.]);let normal=line([1.,0.,0.],[1.,e,0.]);
    for arc in [false,true] {for mode in 0..4 {
        let guide=if mode==3 {line([1.,0.5,0.],[1.,0.5+e,1.])}else{line([1.,0.,0.],[1.,e,1.])};
        let before=(a.clone(),b.clone(),profiles.clone(),path.clone(),guide.clone(),scale.clone(),axes.clone(),center.clone(),longitudinal.clone(),normal.clone());
        let sweep=MultiSweep::new(&profiles,&path,&scale,&zero,Options{normal:[1.,0.,0.],
            orientation:if mode==1 {Orientation::Fixed}else{Orientation::FixedNormal},
            spacing:if arc {Spacing::ArcLength{tolerance:1e-8,max_cells:10000}}else{Spacing::Parameter},
            initial_sections:5,max_sections:17,max_deviation:0.05}).unwrap();
        let sweep=match mode {1=>sweep.with_frame_laws(&longitudinal,&normal).unwrap(),
            2=>sweep.with_orientation_guide(&guide).unwrap(),3=>sweep.with_contact_guide(&guide,1,0.).unwrap(),_=>sweep}
            .with_affine_laws(&axes,&center).unwrap();
        let proof=sweep.certify_original_smooth_volume_atlas([&a,&b],&[4],report().projection,
            OriginalSmoothVolumeLimits{order:2,cells:100000,exact_work:1000000,products:10000}).unwrap();
        let proof=match proof {OriginalSmoothVolumeAttempt::Certified(p)=>p,OriginalSmoothVolumeAttempt::Refused(q)=>panic!("C2 original volume failed mode={mode} arc={arc}: {q:?}")};
        let q=proof.report();assert!(q.certified && q.cap_continuity_certified && q.motion_continuity_certified);
        assert!(q.volume.embedding_certified && q.volume.regularity_certified && q.volume.parameter_topology_preserved);
        eprintln!("original C1 P-map C2 mode={mode} arc={arc}: cells={} exact={} boxes={}",q.cells,q.exact_work,q.volume.boxes);
        assert_eq!(before,(a.clone(),b.clone(),profiles.clone(),path.clone(),guide.clone(),scale.clone(),axes.clone(),center.clone(),longitudinal.clone(),normal.clone()));
    }}
}

#[test]
fn original_open_c1_p_matrix_refuses_c0_laws_and_hidden_original_folds() {
    for fold in [false,true] {
        let mut a=carrier();if fold {for row in a.control_points.iter_mut().skip(1){for (j,p) in row.iter_mut().enumerate(){p[1]=j as f64+4.;}}}
        let profiles=profiles(&a);let path=line([0.;3],[0.,0.,1.]);let zero=constant_vector_law([0.;3]).unwrap();
        let scale=if fold {constant_vector_law([1.,0.,0.]).unwrap()} else {
            Curve{degree:1,knots:vec![0.,0.,0.5,1.,1.],control_points:vec![vec![1.,0.,0.],vec![1.,0.,0.],vec![2.,0.,0.]],weights:vec![1.;3],periodic:false}
        };
        let sweep=MultiSweep::new(&profiles,&path,&scale,&zero,Options{normal:[1.,0.,0.],orientation:Orientation::FixedNormal,
            spacing:Spacing::Parameter,initial_sections:5,max_sections:17,max_deviation:0.05}).unwrap();
        let mut out=report();let identity=cap_identity::certify([&a,&a],&mut out,10000,100000).unwrap();
        let context=cell::Context{s:&sweep.sweeps[0],caps:[&a,&a],cap_identity:Some(identity),cap_difference:None,
            basis:report().projection.map(|v|v.map(I::point)),start:[I::point(0.);3],zero:zero.clone(),offset:None,
            anchor:None,transport:None,arc:None,guide_arc:None,shared_guide:false,poses:Some(Default::default()),cap_jets:None};
        assert!(!certify(&context,report().projection,&mut out,10000,100000).unwrap());
        assert!(!out.embedding_certified && !out.regularity_certified && !out.parameter_topology_preserved);
        assert!(out.cells<=10000 && out.exact_work<=100000);
    }
}
