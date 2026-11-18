use super::*;
#[test]
fn owned_contact_fit_c2_volume_keeps_material_trims_and_c0_cap_wall() {
    let xyz=vec![vec![1.,0.,0.],vec![1.,1.,0.],vec![0.,1.,0.],vec![0.,0.,0.]];
    let source=cap(xyz.clone());
    let profiles=(0..4).map(|i|nurbs_core::primitives::line(
        xyz[i].clone().try_into().unwrap(),xyz[(i+1)%4].clone().try_into().unwrap()).unwrap()).collect::<Vec<_>>();
    let path=nurbs_core::primitives::line([0.;3],[0.,0.,2.]).unwrap();
    let guide=nurbs_core::primitives::line([1.,0.,0.],[1.125,0.,2.]).unwrap();
    let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
    let axes=constant_vector_law([0.875,1.125,0.875]).unwrap();let center=constant_vector_law([0.125,0.,0.]).unwrap();
    let sweep=MultiSweep::new(&profiles,&path,&scale,&twist,Options{
        normal:[1.,0.,0.],orientation:Orientation::RotationMinimizing,spacing:Spacing::Parameter,
        initial_sections:5,max_sections:5,max_deviation:0.05}).unwrap()
        .with_contact_guide(&guide,0,0.).unwrap().with_affine_laws(&axes,&center).unwrap();
    // Contact fitting changes the normal axis only; side scaling stays authored.
    let endpoint=|fit:f64,z:f64|cap(xyz.iter().map(|p|vec![fit*(0.875*p[0]+0.125),1.125*p[1],z]).collect());
    let start=endpoint(1.,0.);let end=endpoint(1.125,2.);
    let before=(source.surface.clone(),source.trims.clone(),profiles.clone(),guide.clone());
    let original=OriginalSmoothVolumeLimits{order:2,cells:1000000,exact_work:1000000,products:1000000};
    let certify=|l|construct_with_original_smooth_volume(
        SweepSource { sweep: &sweep, loop_sizes: &[4], sections: 5, source_caps: [&source,&source] },
        [&start,&end],
        SweepAccuracy { tolerance_uv: 1e-8, error_budget: 0.05 },
        limits(),
        [[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]],
        l,
    ).unwrap();
    let result=certify(original);
    let OriginalSmoothBodyAttempt::Certified(body)=result else {
        let OriginalSmoothBodyAttempt::Refused(r)=result else{unreachable!()};
        panic!("owned contact fit: {} retained_reason={:?} original={:?}",r.reason,r.retained.reason,r.original)
    };
    assert!(body.original().report().certified && body.original().report().motion_continuity_certified);
    assert!(body.retained_report().certified && body.retained_report().volume.as_ref().unwrap().proven);
    assert_eq!(body.cap_wall_continuity(),"C0");
    assert_eq!(body.model().bodies.len(),1);
    let work=body.work();
    eprintln!("owned contact C2 material: cells={} exact={} products={} error={:?}",work.cells,work.exact_work,work.products,body.retained_report().boundary_error_upper);
    let exact=OriginalSmoothVolumeLimits{cells:work.cells,exact_work:work.exact_work,products:work.products,..original};
    assert!(matches!(certify(exact),OriginalSmoothBodyAttempt::Certified(_)));
    for short in [OriginalSmoothVolumeLimits{cells:work.cells-1,..exact},OriginalSmoothVolumeLimits{exact_work:work.exact_work-1,..exact}] {
        let OriginalSmoothBodyAttempt::Refused(r)=certify(short) else{panic!("partial contact material proof admitted")};
        assert!(r.work.cells<=short.cells && r.work.exact_work<=short.exact_work);
    }
    let mut broken=LoftCap{surface:source.surface.clone(),trims:source.trims.clone()};
    broken.trims[0][0].control_points[0][0]=0.125;
    assert!(matches!(construct_with_original_smooth_volume(
        SweepSource { sweep: &sweep, loop_sizes: &[4], sections: 5, source_caps: [&broken,&source] },
        [&start,&end],
        SweepAccuracy { tolerance_uv: 1e-8, error_budget: 0.05 },
        limits(),
        [[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]],
        original,
    ).unwrap(),OriginalSmoothBodyAttempt::Refused(_)));
    assert_eq!(before,(source.surface.clone(),source.trims.clone(),profiles.clone(),guide.clone()));
}
