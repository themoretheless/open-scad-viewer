//! Actual material ownership on a spatial periodic Frenet source, entirely native.
use brep_core::{LoftCap, sweep_boundary_owned::*};
use nurbs_core as nurbs;
use nurbs::progressive_sweep::*;
#[path="../../nurbs-core/tests/support/contact_material_source.rs"] mod source;
#[path="../../nurbs-core/tests/support/closed_frenet_source.rs"] mod frenet;
fn limits()->OriginalClassifiedClosedMaterialLimits {
    OriginalClassifiedClosedMaterialLimits{volume:OriginalClassifiedVolumeLimits{
        charts:OriginalSmoothVolumeLimits{order:2,cells:1000000,exact_work:1000000,products:1000000},steps:1000000},
        material:MaterialLimits{pairs:10000,cells:100000,domain_cells:1000000}}
}
fn options(arc:bool)->Options {Options{normal:[0.,1.,0.],orientation:Orientation::Frenet,
    spacing:if arc {Spacing::ArcLength{tolerance:0.001,max_cells:100000}}else{Spacing::Parameter},
    initial_sections:33,max_sections:129,max_deviation:0.05}}
fn projection()->[[f64;3];3] {[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]]}
#[test]
fn frenet_closed_material_matrix_owns_c2_charts_g2_rings_caps_and_holes() {
    for clock in 0..14 {for warped in [false,true] {for hole in [false,true] {
        let periodic=clock>=4;let arc=matches!(clock,1|2|3|5|7|9|11|13);
        let authored=matches!(clock,6|7);let guided=matches!(clock,8|9);let bishop=matches!(clock,10|11);let fixed_normal=clock>=12;
        let reclock=matches!(clock,2|3);let split=clock==3;
        let path=if periodic {frenet::periodic_path()}else if split {frenet::split_reclocked_path()}else if reclock {frenet::reclocked_path()}else{frenet::path()};let (profiles,surface,trims)=source::material(hole,warped);let material=LoftCap{surface,trims};
        let scale=source::periodic(1.,1.03125);let twist=source::periodic(0.,1./1024.);let center=source::periodic(0.,source::R/16.);
        let mut axes=source::periodic(1.,1.0078125);for p in &mut axes.control_points {p[1]=1.125;p[2]=0.875;}
        let before=(path.clone(),profiles.clone(),material.surface.clone(),material.trims.clone(),scale.clone(),twist.clone(),axes.clone(),center.clone());
        let authored_axis=authored.then(||frenet::periodic_authored_axis(&path));
        let normal=constant_vector_law([0.,0.,1.]).unwrap();
        let rail=guided.then(||{
            let mut rail=path.clone();
            for point in &mut rail.control_points {point[2]+=0.125;}
            rail
        });
        let orientation=if bishop {Orientation::RotationMinimizing}else if authored {Orientation::Fixed}else if guided || fixed_normal {Orientation::FixedNormal}else{Orientation::Frenet};
        let sweep=MultiSweep::new(&profiles,&path,&scale,&twist,Options{orientation,normal:if bishop || fixed_normal {[0.,0.,1.]}else{options(arc).normal},..options(arc)})
            .unwrap().with_affine_laws(&axes,&center).unwrap();
        let sweep=if let Some(axis)=&authored_axis {
            sweep.with_frame_laws(axis,&normal).unwrap()
        }else if let Some(rail)=&rail {
            sweep.with_orientation_guide(rail).unwrap()
        }else if bishop {sweep.with_spatial_rmf_error_limits(4096,100000,1000000).unwrap()}else{sweep};
        let loops=vec![4;1+usize::from(hole)];let classes=vec![OriginalProfileClass::G2;profiles.len()];
        let proof=certify_original_classified_closed_material(&sweep,&loops,&material,&classes,projection(),1e-8,limits()).unwrap();
        let body=match proof {OriginalClassifiedClosedMaterialAttempt::Certified(p)=>p,OriginalClassifiedClosedMaterialAttempt::Refused(q)=>panic!("Closed material fixedNormal={fixed_normal} bishop={bishop} periodic={periodic} authored={authored} guided={guided} split={split} arc={arc} warped={warped} hole={hole}: {:?} {:?}",q.reason,q.volume)};
        let v=body.volume().report();assert!(v.certified && v.charts.volume.embedding_certified && v.charts.volume.regularity_certified && v.charts.volume.parameter_topology_preserved);
        assert_eq!(body.closed_seam_order(),2);assert_eq!(body.boundary_classes(),classes);
        assert_eq!(v.profiles.as_ref().unwrap().smooth_joins,profiles.len());
        assert!(body.material_report().certified);assert_eq!(body.material_report().bound_spans,profiles.len());assert_eq!(body.bindings().len(),profiles.len());
        assert_eq!(body.material_report().trim.as_ref().unwrap().valid,Some(true));
        assert_eq!(body.material_report().trim.as_ref().unwrap().winding,if hole {vec![Some(1),Some(-1)]}else{vec![Some(1)]});
        assert!(std::ptr::eq(body.material(),&material) && std::ptr::eq(body.volume().sweep(),&sweep));
        for (i,b) in body.bindings().iter().enumerate(){assert_eq!((b.ring(),b.profile(),b.trim()),(i/4,(i,0),(i%4,0)));assert_eq!(b.kind(),MaterialSpanBindingKind::NormalizedMap);}
        let w=body.work();eprintln!("Closed material fixedNormal={fixed_normal} bishop={bishop} periodic={periodic} authored={authored} guided={guided} split={split} reclock={reclock} arc={arc} warped={warped} hole={hole}: cells={} exact={} products={}",w.cells,w.exact_work,w.products);
        if arc && warped && hole {
            #[cfg(feature="codec")]
            eprintln!("{}={}",if fixed_normal {"periodic-fixed-normal-material-source"}else if bishop {"periodic-bishop-material-source"}else if authored {"periodic-authored-material-source"}else if guided {"periodic-guided-material-source"}else if periodic {"periodic-frenet-material-source"}else if reclock {"geometric-frenet-material-source"}else{"closed-frenet-material-source"},value_codec::json!({"path":path,"profiles":profiles,"surface":&material.surface,"trims":&material.trims,
                "scale":scale,"twist":twist,"axes":axes,"center":center,"clock":"arc-length","authoredAxis":authored_axis,"authoredNormal":authored.then_some(&normal),"guide":rail,"sourceClock":if split {"split-mobius"}else if reclock {"mobius"}else{"raw"}}));
            let exact=OriginalClassifiedClosedMaterialLimits{volume:OriginalClassifiedVolumeLimits{
                charts:OriginalSmoothVolumeLimits{cells:w.cells,exact_work:w.exact_work,products:w.products,..limits().volume.charts},
                steps:v.profiles.as_ref().unwrap().steps},..limits()};
            macro_rules! certify {($m:expr,$l:expr)=>{certify_original_classified_closed_material(&sweep,&loops,$m,&classes,projection(),1e-8,$l).unwrap()};}
            assert!(matches!(certify!(&material,exact),OriginalClassifiedClosedMaterialAttempt::Certified(_)));
            for l in [OriginalClassifiedClosedMaterialLimits{volume:OriginalClassifiedVolumeLimits{charts:OriginalSmoothVolumeLimits{cells:w.cells-1,..exact.volume.charts},..exact.volume},..exact},
                OriginalClassifiedClosedMaterialLimits{volume:OriginalClassifiedVolumeLimits{charts:OriginalSmoothVolumeLimits{exact_work:w.exact_work-1,..exact.volume.charts},..exact.volume},..exact},
                OriginalClassifiedClosedMaterialLimits{volume:OriginalClassifiedVolumeLimits{charts:OriginalSmoothVolumeLimits{products:w.products-1,..exact.volume.charts},..exact.volume},..exact},
                OriginalClassifiedClosedMaterialLimits{volume:OriginalClassifiedVolumeLimits{steps:exact.volume.steps-1,..exact.volume},..exact}] {
                let OriginalClassifiedClosedMaterialAttempt::Refused(q)=certify!(&material,l) else{panic!("incomplete Frenet material proof admitted")};
                assert!(q.work.cells<=l.volume.charts.cells && q.work.exact_work<=l.volume.charts.exact_work && q.work.products<=l.volume.charts.products);
            }
            let mut damaged=material.clone();damaged.trims[1][0].control_points[1][0]=damaged.trims[1][0].control_points[1][0].next_up();
            assert!(matches!(certify!(&damaged,limits()),OriginalClassifiedClosedMaterialAttempt::Refused(_)));
        }
        assert_eq!(before,(path.clone(),profiles.clone(),material.surface.clone(),material.trims.clone(),scale.clone(),twist.clone(),axes.clone(),center.clone()));
    }}}
}

#[test]
fn frenet_closed_material_sharp_ring_stays_explicitly_c0() {
    let path=frenet::path();let (_,surface,_)=source::material(false,true);
    let indices=[(0,0),(1,0),(1,1),(0,1),(0,0)];let uv=[[-1.,2.],[3.,2.],[3.,6.],[-1.,6.],[-1.,2.]];
    let line=|p,q|nurbs::curve::Curve{degree:1,knots:vec![0.,0.,1.,1.],control_points:vec![p,q],weights:vec![1.;2],periodic:false};
    let profiles=(0..4).map(|i|line(surface.control_points[indices[i].0][indices[i].1].clone(),surface.control_points[indices[i+1].0][indices[i+1].1].clone())).collect::<Vec<_>>();
    let trims=vec![(0..4).map(|i|line(uv[i].to_vec(),uv[i+1].to_vec())).collect()];let material=LoftCap{surface,trims};
    let scale=constant_vector_law([1.,0.,0.]).unwrap();let zero=constant_vector_law([0.;3]).unwrap();
    let sweep=MultiSweep::new(&profiles,&path,&scale,&zero,options(true)).unwrap();
    let proof=certify_original_classified_closed_material(&sweep,&[4],&material,&[OriginalProfileClass::C0;4],projection(),1e-8,limits()).unwrap();
    let body=match proof {OriginalClassifiedClosedMaterialAttempt::Certified(p)=>p,OriginalClassifiedClosedMaterialAttempt::Refused(q)=>panic!("sharp Frenet material refused: {:?} {:?}",q.reason,q.volume)};
    assert_eq!(body.closed_seam_order(),2);assert_eq!(body.boundary_classes(),[OriginalProfileClass::C0;4]);
    let q=body.volume().report();assert!(q.certified && q.charts.volume.embedding_certified && q.charts.volume.regularity_certified);
    assert_eq!(q.profiles.as_ref().unwrap().smooth_joins,0);assert_eq!(q.profiles.as_ref().unwrap().inspected_joins,4);
    assert!(body.material_report().certified);assert_eq!(body.bindings().len(),4);
    let claimed=certify_original_classified_closed_material(&sweep,&[4],&material,&[OriginalProfileClass::G1;4],projection(),1e-8,limits()).unwrap();
    let OriginalClassifiedClosedMaterialAttempt::Refused(q)=claimed else{panic!("sharp original corners acquired G1")};
    assert!(q.material.certified && q.volume.is_some());
    eprintln!("Frenet sharp original material remains C0; requested G1 refused: {:?}",q.reason);
}

#[test]
fn fixed_frame_rank_obstruction_refuses_closed_material_with_valid_g2_hole() {
    let (profiles, surface, trims) = source::material(true, false);
    let material = LoftCap { surface, trims };
    let mut path = frenet::periodic_path();
    for point in &mut path.control_points {
        point[2] = 0.;
    }
    let scale = source::periodic(1., 1.03125);
    let twist = source::periodic(0., 0.125);
    let sweep = MultiSweep::new(&profiles, &path, &scale, &twist, Options {
        orientation: Orientation::Fixed, normal: [0., 0., 1.], ..options(true)
    }).unwrap();
    let result = certify_original_classified_closed_material(
        &sweep, &[4, 4], &material, &[OriginalProfileClass::G2; 8],
        projection(), 1e-8, limits(),
    ).unwrap();
    let OriginalClassifiedClosedMaterialAttempt::Refused(report) = result else {
        panic!("A closed fixed transverse sweep acquired volume credit");
    };
    #[cfg(feature = "codec")]
    eprintln!("closed-fixed-transverse-material-source={}", value_codec::json!({
        "path": &path, "cap": &material.surface, "twist": &twist, "scale": &scale
    }));
    assert!(report.material.certified);
    let body = &report.volume.unwrap().charts.volume;
    assert_eq!(body.reason, Some("ideal-volume-fixed-transverse-rank-loss"));
    assert_eq!(body.boxes, 0);
    assert!(!body.embedding_certified && !body.regularity_certified && !body.parameter_topology_preserved);
}
