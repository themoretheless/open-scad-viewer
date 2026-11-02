use super::*;
#[path = "../../../../../tests/support/arc_bishop_source.rs"]
mod arc_source;
fn options(spacing: Spacing) -> Options {
    Options { normal:[1.,0.,0.], orientation:Orientation::RotationMinimizing,
        spacing,initial_sections:5,max_sections:33,max_deviation:0.05 }
}
fn guide() -> Curve {
    Curve {degree:2,knots:vec![-3.,-3.,-3.,5.,5.,5.],
        control_points:vec![vec![0.5,0.,0.],vec![0.75,0.,1.5],vec![0.625,0.,3.]],
        weights:vec![1.;3],periodic:false}
}
#[test]
fn contact_original_c2_volume_owns_fit_affine_laws_caps_and_one_budget() {
    let profiles=[crate::primitives::line([0.5,0.,0.],[0.5,0.25,0.]).unwrap()];
    let path=crate::primitives::line([0.;3],[0.,0.,3.]).unwrap(); let guide=guide();
    let scale=Curve {degree:1,knots:vec![7.,7.,11.,11.],
        control_points:vec![vec![1.,0.,0.],vec![1.125,0.,0.]],weights:vec![1.,2.],periodic:false};
    let twist=constant_vector_law([0.;3]).unwrap();
    let axes=constant_vector_law([1.,1.125,0.875]).unwrap();
    let center=crate::primitives::line([0.0625,0.,0.],[0.125,0.,0.]).unwrap();
    let c=cap(0); let mut end=c.clone();
    for p in end.control_points.iter_mut().flatten() {p[2]*=0.5;}
    let before=(profiles.clone(),path.clone(),guide.clone(),scale.clone(),axes.clone(),center.clone(),c.clone(),end.clone());
    for spacing in [Spacing::Parameter,Spacing::ArcLength{tolerance:0.001,max_cells:100000}] {
        let s=MultiSweep::new(&profiles,&path,&scale,&twist,options(spacing)).unwrap()
            .with_contact_guide(&guide,0,0.).unwrap().with_affine_laws(&axes,&center).unwrap();
        for order in [1,2] {
            let result=s.certify_original_smooth_volume_atlas([&c,&end],&[1],projection(),limits(order)).unwrap();
            let OriginalSmoothVolumeAttempt::Certified(full)=result else {
                let OriginalSmoothVolumeAttempt::Refused(r)=result else{unreachable!()};
                panic!("contact {spacing:?} C{order}: {r:?}")
            };
            let r=full.report();
            assert!(r.motion_continuity_certified && r.cap_continuity_certified && r.volume.embedding_certified);
            let exact=OriginalSmoothVolumeLimits{cells:r.cells,exact_work:r.exact_work,..limits(order)};
            assert!(matches!(s.certify_original_smooth_volume_atlas([&c,&end],&[1],projection(),exact).unwrap(),OriginalSmoothVolumeAttempt::Certified(_)));
            for short in [OriginalSmoothVolumeLimits{cells:r.cells-1,..exact},OriginalSmoothVolumeLimits{exact_work:r.exact_work-1,..exact}] {
                let OriginalSmoothVolumeAttempt::Refused(q)=s.certify_original_smooth_volume_atlas([&c,&end],&[1],projection(),short).unwrap() else {panic!("partial contact proof admitted")};
                assert!(!q.certified && q.cells<=short.cells && q.exact_work<=short.exact_work);
            }
            eprintln!("original contact {spacing:?} C{order}: cells={} exact={}",r.cells,r.exact_work);
        }
    }
    assert_eq!(before,(profiles,path,guide,scale,axes,center,c,end));
}
#[test]
fn contact_original_c1_guide_does_not_acquire_c2() {
    let profiles=[crate::primitives::line([0.5,0.,0.],[0.5,0.25,0.]).unwrap()];
    let path=crate::primitives::line([0.;3],[0.,0.,3.]).unwrap();
    let guide=Curve {degree:2,knots:vec![-3.,-3.,-3.,1.,1.,5.,5.,5.],
        control_points:[0.5,0.5,0.625,0.75,1.125].into_iter().enumerate()
            .map(|(i,x)|vec![x,0.,i as f64*0.75]).collect(),weights:vec![1.;5],periodic:false};
    let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();let c=plane();
    let s=MultiSweep::new(&profiles,&path,&scale,&twist,options(Spacing::Parameter)).unwrap().with_contact_guide(&guide,0,0.).unwrap();
    assert!(matches!(run(&s,&c,limits(1)),OriginalSmoothVolumeAttempt::Certified(_)));
    let OriginalSmoothVolumeAttempt::Refused(r)=run(&s,&c,limits(2)) else{panic!("C1 contact acquired C2")};
    assert!(r.volume.embedding_certified && !r.motion_continuity_certified && !r.certified,"{r:?}");
}
fn periodic_law(base:f64,peak:f64) -> Curve {
    Curve {degree:6,knots:[vec![7.;7],vec![11.;7]].concat(),
        control_points:[base,base,base,peak,base,base,base].map(|x|vec![x,0.,0.]).to_vec(),
        weights:vec![1.;7],periodic:false}
}
#[test]
fn closed_contact_original_c2_owns_periodic_fit_and_geometric_arc_clock() {
    for reclock in [false,true] {
        let mut path=arc_source::path(reclock);
        for p in &mut path.control_points {p[2]=0.;}
        let mut guide=path.clone();for p in &mut guide.control_points {p[2]+=1./128.;}
        let (mut profiles,mut c,_)=arc_source::material();
        for p in profiles.iter_mut().flat_map(|c|c.control_points.iter_mut()).chain(c.control_points.iter_mut().flatten()) {
            let y=p[1];p[1]=-p[2];p[2]=y;
        }
        let scale=periodic_law(1.,1.125);let twist=constant_vector_law([0.;3]).unwrap();
        let axes=constant_vector_law([1.,1.125,0.875]).unwrap();
        let center=periodic_law(0.,1./512.);
        let spacing=if reclock {Spacing::ArcLength{tolerance:0.001,max_cells:100000}} else {Spacing::Parameter};
        let opts=Options{normal:[0.,0.,1.],initial_sections:33,max_sections:129,..options(spacing)};
        let s=MultiSweep::new(&profiles,&path,&scale,&twist,opts).unwrap().with_contact_guide(&guide,0,0.).unwrap()
            .with_affine_laws(&axes,&center).unwrap();
        let result=s.certify_original_smooth_volume_atlas([&c,&c],&[4],projection(),limits(2)).unwrap();
        let OriginalSmoothVolumeAttempt::Certified(full)=result else {
            let OriginalSmoothVolumeAttempt::Refused(r)=result else{unreachable!()};panic!("closed contact {reclock}: {r:?}")
        };
        let r=full.report();assert!(r.certified && r.closed_boundary.as_ref().unwrap().differential_compatibility_certified);
        eprintln!("closed original contact reclock={reclock}: cells={} exact={}",r.cells,r.exact_work);
        let exact=OriginalSmoothVolumeLimits{cells:r.cells,exact_work:r.exact_work,products:r.products,..limits(2)};
        assert!(matches!(s.certify_original_smooth_volume_atlas([&c,&c],&[4],projection(),exact).unwrap(),OriginalSmoothVolumeAttempt::Certified(_)));
        for short in [OriginalSmoothVolumeLimits{cells:r.cells-1,..exact},OriginalSmoothVolumeLimits{exact_work:r.exact_work-1,..exact},OriginalSmoothVolumeLimits{products:r.products-1,..exact}] {
            let OriginalSmoothVolumeAttempt::Refused(q)=s.certify_original_smooth_volume_atlas([&c,&c],&[4],projection(),short).unwrap() else {panic!("partial closed contact proof admitted")};
            assert!(q.cells<=short.cells && q.exact_work<=short.exact_work && q.products<=short.products);
        }
        if !reclock {
            // A smooth frame alone cannot certify a singular/negative fit in
            // the public closed-boundary API, which has no global-volume gate.
            let one=constant_vector_law([1.,0.,0.]).unwrap();
            for x in [-1./128.,-0.125] {
                let bad_center=constant_vector_law([x,0.,0.]).unwrap();
                let bad=MultiSweep::new(&profiles,&path,&one,&twist,opts).unwrap()
                    .with_contact_guide(&guide,0,0.).unwrap().with_affine_laws(&axes,&bad_center).unwrap();
                let q=bad.certify_original_closed_boundary_smoothness(&[4],OriginalBoundarySmoothnessLimits{
                    order:2,cells:100000,exact_work:1000000,products:1000000}).unwrap();
                assert!(q.frame_certified && !q.laws_certified && !q.differential_compatibility_certified,"{q:?}");
                assert!(q.reason.unwrap().starts_with("original-contact-fit-"),"{q:?}");
            }
            let diamond:Vec<_>=profiles.iter().map(|p|crate::primitives::line(
                p.control_points[0].clone().try_into().unwrap(),p.control_points[2].clone().try_into().unwrap()).unwrap()).collect();
            let sharp=MultiSweep::new(&diamond,&path,&scale,&twist,opts).unwrap().with_contact_guide(&guide,0,0.).unwrap()
                .with_affine_laws(&axes,&center).unwrap();
            let classify=|classes:&[OriginalProfileClass]| sharp.certify_original_classified_volume_atlas([&c,&c],&[4],classes,projection(),
                OriginalClassifiedVolumeLimits{charts:limits(2),steps:1000000}).unwrap();
            let OriginalClassifiedVolumeAttempt::Certified(body)=classify(&[OriginalProfileClass::C0;4]) else{panic!("declared contact C0 corners lost")};
            assert_eq!(body.classes(),&[OriginalProfileClass::C0;4]);
            assert!(matches!(classify(&[OriginalProfileClass::G1;4]),OriginalClassifiedVolumeAttempt::Refused(_)));
        }
    }
}
