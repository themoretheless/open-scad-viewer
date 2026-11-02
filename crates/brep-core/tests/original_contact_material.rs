//! Public original closed-material owner; no retained or sampled proof flags.
use brep_core::{LoftCap,sweep_boundary_owned::*};
use nurbs_core as nurbs;
use nurbs::progressive_sweep::*;
#[path="../../nurbs-core/tests/support/contact_material_source.rs"] mod source;
fn limits()->OriginalClassifiedClosedMaterialLimits {
    OriginalClassifiedClosedMaterialLimits{volume:OriginalClassifiedVolumeLimits{
        charts:OriginalSmoothVolumeLimits{order:2,cells:1000000,exact_work:1000000,products:1000000},steps:1000000},
        material:MaterialLimits{pairs:10000,cells:100000,domain_cells:1000000}}
}
fn options(reclock:bool)->Options {
    Options{normal:[0.,0.,1.],orientation:Orientation::RotationMinimizing,
        spacing:if reclock {Spacing::ArcLength{tolerance:0.001,max_cells:100000}} else {Spacing::Parameter},
        initial_sections:33,max_sections:129,max_deviation:0.05}
}
fn projection()->[[f64;3];3] {[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]]}
#[test]
fn contact_closed_g2_material_matrix_binds_every_ring_and_preserves_global_topology() {
    for reclock in [false,true] {for warped in [false,true] {for hole in [false,true] {
        let path=source::path(reclock);let guide=source::guide(&path);
        let (profiles,surface,trims)=source::material(hole,warped);let material=LoftCap{surface,trims};
        let before=(profiles.clone(),material.surface.clone(),material.trims.clone(),path.clone(),guide.clone());
        let scale=source::periodic(1.,1.125);let center=source::periodic(0.,source::R/4.);
        let axes=constant_vector_law([1.,1.125,0.875]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
        let sweep=MultiSweep::new(&profiles,&path,&scale,&twist,options(reclock)).unwrap()
            .with_contact_guide(&guide,0,0.).unwrap().with_affine_laws(&axes,&center).unwrap();
        let loops=vec![4;1+usize::from(hole)];let classes=vec![OriginalProfileClass::G2;profiles.len()];
        macro_rules! certify {($m:expr,$l:expr)=>{certify_original_classified_closed_material(
            &sweep,&loops,$m,&classes,projection(),1e-8,$l).unwrap()};}
        let result=certify!(&material,limits());
        let OriginalClassifiedClosedMaterialAttempt::Certified(body)=result else {
            let OriginalClassifiedClosedMaterialAttempt::Refused(r)=result else{unreachable!()};
            panic!("contact material arc={reclock} warped={warped} hole={hole}: {:?} {:?}",r.reason,r.volume)
        };
        let v=body.volume().report();
        assert!(v.certified && v.charts.volume.embedding_certified && v.charts.volume.regularity_certified && v.charts.volume.parameter_topology_preserved);
        assert_eq!(body.closed_seam_order(),2);assert_eq!(body.boundary_classes(),classes);
        assert_eq!(v.profiles.as_ref().unwrap().smooth_joins,profiles.len());
        assert!(body.material_report().certified);
        assert_eq!(body.material_report().bound_spans,profiles.len());
        assert_eq!(body.bindings().len(),profiles.len());
        assert_eq!(body.material_report().trim.as_ref().unwrap().valid,Some(true));
        assert_eq!(body.material_report().trim.as_ref().unwrap().winding,if hole {vec![Some(1),Some(-1)]} else {vec![Some(1)]});
        assert!(std::ptr::eq(body.material(),&material) && std::ptr::eq(body.volume().sweep(),&sweep));
        for (i,binding) in body.bindings().iter().enumerate() {
            assert_eq!((binding.ring(),binding.profile(),binding.trim()),(i/4,(i,0),(i%4,0)));
            assert_eq!(binding.kind(),MaterialSpanBindingKind::NormalizedMap);
        }
        let w=body.work();eprintln!("contact material arc={reclock} warped={warped} hole={hole}: cells={} exact={} products={}",w.cells,w.exact_work,w.products);
        if reclock && warped && hole {
            #[cfg(feature="codec")]
            source_record("positive",&profiles,&material,&path,&guide,&scale,&twist,&axes,&center,"arc-length");
            let exact=OriginalClassifiedClosedMaterialLimits{volume:OriginalClassifiedVolumeLimits{
                charts:OriginalSmoothVolumeLimits{cells:w.cells,exact_work:w.exact_work,products:w.products,..limits().volume.charts},
                steps:v.profiles.as_ref().unwrap().steps},..limits()};
            assert!(matches!(certify!(&material,exact),OriginalClassifiedClosedMaterialAttempt::Certified(_)));
            for l in [OriginalClassifiedClosedMaterialLimits{volume:OriginalClassifiedVolumeLimits{charts:OriginalSmoothVolumeLimits{cells:w.cells-1,..exact.volume.charts},..exact.volume},..exact},
                OriginalClassifiedClosedMaterialLimits{volume:OriginalClassifiedVolumeLimits{charts:OriginalSmoothVolumeLimits{exact_work:w.exact_work-1,..exact.volume.charts},..exact.volume},..exact},
                OriginalClassifiedClosedMaterialLimits{volume:OriginalClassifiedVolumeLimits{charts:OriginalSmoothVolumeLimits{products:w.products-1,..exact.volume.charts},..exact.volume},..exact},
                OriginalClassifiedClosedMaterialLimits{volume:OriginalClassifiedVolumeLimits{steps:exact.volume.steps-1,..exact.volume},..exact}] {
                let OriginalClassifiedClosedMaterialAttempt::Refused(q)=certify!(&material,l) else{panic!("incomplete material proof admitted")};
                assert!(q.work.cells<=l.volume.charts.cells && q.work.exact_work<=l.volume.charts.exact_work && q.work.products<=l.volume.charts.products);
            }
            let mut damaged=material.clone();damaged.trims[1][0].control_points[1][0]=damaged.trims[1][0].control_points[1][0].next_up();
            assert!(matches!(certify!(&damaged,limits()),OriginalClassifiedClosedMaterialAttempt::Refused(_)));
        }
        assert_eq!(before,(profiles.clone(),material.surface.clone(),material.trims.clone(),path.clone(),guide.clone()));
    }}}
}

fn refused(profiles:&[nurbs::curve::Curve],material:&LoftCap,path:&nurbs::curve::Curve,label:&str,global:bool) {
    let guide=source::guide(path);let scale=source::periodic(1.,1.125);let center=source::periodic(0.,source::R/4.);
    let axes=constant_vector_law([1.,1.125,0.875]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
    let sweep=MultiSweep::new(profiles,path,&scale,&twist,options(false)).unwrap()
        .with_contact_guide(&guide,0,0.).unwrap().with_affine_laws(&axes,&center).unwrap();
    let result=certify_original_classified_closed_material(&sweep,&[4,4],material,&[OriginalProfileClass::G2;8],projection(),1e-8,limits()).unwrap();
    let OriginalClassifiedClosedMaterialAttempt::Refused(q)=result else{panic!("invalid contact material admitted: {label}")};
    assert_eq!(q.volume.is_some(),global,"{label}: {:?}",q.reason);
    if global {
        assert!(q.material.certified,"{label}: {:?}",q.reason);
        let v=q.volume.as_ref().unwrap();assert!(!v.certified && !v.charts.volume.embedding_certified && !v.charts.volume.parameter_topology_preserved);
    } else {assert_eq!(q.reason,Some("original-material-region-unproved"));}
    assert!(q.work.cells<=limits().volume.charts.cells && q.work.exact_work<=limits().volume.charts.exact_work && q.work.products<=limits().volume.charts.products);
    #[cfg(feature="codec")]
    if label=="genuine-material-self-intersection" {source_record("collision",profiles,material,path,&guide,&scale,&twist,&axes,&center,"parameter");}
    eprintln!("contact material negative {label}: {:?}, cells={} exact={} products={}",q.reason,q.work.cells,q.work.exact_work,q.work.products);
}
#[test]
fn contact_material_region_orientation_nesting_rank_and_self_intersection_are_independent_gates() {
    let path=source::path(false);let (profiles,surface,trims)=source::material(true,true);
    let material=LoftCap{surface,trims};
    for (label,scale,offset,reverse) in [("wrong-hole-orientation",0.25,[0.;2],false),
        ("hole-outside-outer-inside-carrier",1./16.,[7.*source::R/8.;2],true)] {
        let (inner,hole)=source::ring(scale,offset,reverse,true);
        let mut p=profiles[..4].to_vec();p.extend(inner);
        let mut m=material.clone();m.trims[1]=hole;
        refused(&p,&m,&path,label,false);
    }
    let mut singular=material.clone();let mut p=profiles.clone();
    for q in singular.surface.control_points.iter_mut().flatten().chain(p.iter_mut().flat_map(|c|c.control_points.iter_mut())) {q[1]=0.;}
    refused(&p,&singular,&path,"singular-filled-carrier",true);
    // Two actual material interior points, not just centerline samples,
    // coincide at t=0 and t=1/2. See the independent exact Fraction witness.
    let figure=source::figure_eight();
    refused(&profiles,&material,&figure,"genuine-material-self-intersection",true);
}

#[cfg(feature="codec")]
fn source_record(label:&str,profiles:&[nurbs::curve::Curve],material:&LoftCap,path:&nurbs::curve::Curve,
    guide:&nurbs::curve::Curve,scale:&nurbs::curve::Curve,twist:&nurbs::curve::Curve,axes:&nurbs::curve::Curve,center:&nurbs::curve::Curve,clock:&str) {
    eprintln!("contact-material-source={}",value_codec::json!({"label":label,"path":path,"guide":guide,
        "scale":scale,"twist":twist,"axes":axes,"center":center,"selectedClock":clock,"normal":[0.,0.,1.],
        "profiles":profiles,"surface":&material.surface,"trims":&material.trims}));
}
