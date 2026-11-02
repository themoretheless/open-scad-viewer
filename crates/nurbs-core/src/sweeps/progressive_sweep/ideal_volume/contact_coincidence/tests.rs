use super::*;
use crate as nurbs;
#[path="../../../../../tests/support/contact_material_source.rs"] mod source;
fn report()->IdealVolumeReport {
    IdealVolumeReport{embedding_certified:false,regularity_certified:false,parameter_topology_preserved:false,
        cells:0,exact_work:0,boxes:0,monotonicity_lower:None,projection:[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]],reason:None}
}
fn transform(p:&mut [f64],axis:usize,sign:f64){let q=p.to_vec();p[(axis+1)%3]=q[0];p[(axis+2)%3]=q[1];p[axis]=sign*q[2];}
#[test]
fn exact_original_contact_coincidence_owns_arbitrary_axis_sign_and_rational_uv_witness() {
    for axis in 0..3 {for sign in [-1.,1.] {
        let mut path=source::figure_eight();let mut guide=source::guide(&path);
        let (mut profiles,mut cap,trims)=source::material(true,true);
        for p in path.control_points.iter_mut().chain(guide.control_points.iter_mut())
            .chain(profiles.iter_mut().flat_map(|c|c.control_points.iter_mut())).chain(cap.control_points.iter_mut().flatten()) {
            transform(p,axis,sign);
        }
        let before=(path.clone(),guide.clone(),profiles.clone(),cap.clone());
        let scale=source::periodic(1.,1.125);let center=source::periodic(0.,source::R/4.);
        let axes=constant_vector_law([1.,1.125,0.875]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
        let sweep=MultiSweep::new(&profiles,&path,&scale,&twist,Options{
            normal:[0.,0.,1.],orientation:Orientation::RotationMinimizing,spacing:Spacing::Parameter,
            initial_sections:5,max_sections:33,max_deviation:0.05}).unwrap()
            .with_contact_guide(&guide,0,0.).unwrap().with_affine_laws(&axes,&center).unwrap();
        assert!(sweep.certify_closed_guided_frame_smoothness(1,100000,1000000).unwrap().closed_source_frame_smoothness_certified);
        let primitive=|cells,work|{let mut r=report();let q=inspect(&sweep.sweeps[0],[&cap,&cap],&mut r,cells,work,true);(q,r)};
        let (Some(witness),r)=primitive(100000,100000) else{panic!("exact contact witness lost axis={axis} sign={sign}")};
        assert_eq!(witness.raw_path_parameters,[0.,2.]);
        assert_eq!(witness.normalized_cap_parameters,[[Q::new(1.into(),4.into()),Q::new(1.into(),2.into())],
            [Q::new(103.into(),532.into()),Q::new(1.into(),2.into())]]);
        for k in 0..3 {assert_eq!(witness.world[k],Q::from_float(if k==axis {-sign*source::R/2.} else {0.}).unwrap());}
        assert!(!r.embedding_certified && !r.regularity_certified && !r.parameter_topology_preserved);
        assert!(primitive(r.cells,r.exact_work).0.is_some());
        for (cells,work) in [(0,r.exact_work),(r.cells-1,r.exact_work),(r.cells,0),(r.cells,r.exact_work-1)] {
            let (q,short)=primitive(cells,work);assert!(q.is_none() && short.cells<=cells && short.exact_work<=work);
        }
        let full=sweep.certify_ideal_volume_atlas([&cap,&cap],report().projection,1000000,1000000).unwrap();
        assert_eq!(full.reason,Some("ideal-volume-exact-filled-map-coincidence"));
        assert!(!full.embedding_certified && !full.regularity_certified && !full.parameter_topology_preserved);
        eprintln!("exact original contact axis={axis} sign={sign}: primitive cells={} exact={}, whole cells={} exact={}, uv={:?} world={:?}",r.cells,r.exact_work,full.cells,full.exact_work,witness.normalized_cap_parameters,witness.world);
        #[cfg(feature="codec")]
        eprintln!("exact-contact-witness={}",value_codec::json!({"axis":axis,"normalSign":sign,
            "rawPathParameters":witness.raw_path_parameters,
            "capNormalizedUV":witness.normalized_cap_parameters.map(|uv|uv.map(|x|x.to_string())),
            "world":witness.world.map(|x|x.to_string()),"path":path,"guide":guide,"profiles":profiles,
            "surface":cap,"trims":trims,"scale":scale,"axes":axes,"center":center,"twist":twist}));
        if axis==2 && sign==1. {
            let mut skew=cap.clone();skew.control_points[1][1][0]=skew.control_points[1][1][0].next_up();
            let mut q=report();assert!(inspect(&sweep.sweeps[0],[&skew,&skew],&mut q,100000,100000,true).is_none());
            assert!(!q.embedding_certified && !q.regularity_certified && !q.parameter_topology_preserved);
        }
        assert_eq!(before,(path.clone(),guide.clone(),profiles.clone(),cap.clone()));
    }}
}
#[test]
fn original_knot_near_miss_and_identified_closed_seam_never_supply_positive_credit() {
    let mut path=source::figure_eight();
    for p in &mut path.control_points[8..=12] {p[0]+=1./1024.;}
    let guide=source::guide(&path);let (profiles,cap,_)=source::material(false,true);
    let scale=source::periodic(1.,1.125);let center=source::periodic(0.,source::R/4.);
    let axes=constant_vector_law([1.,1.125,0.875]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
    let sweep=MultiSweep::new(&profiles,&path,&scale,&twist,Options{normal:[0.,0.,1.],orientation:Orientation::RotationMinimizing,
        spacing:Spacing::Parameter,initial_sections:5,max_sections:33,max_deviation:0.05}).unwrap()
        .with_contact_guide(&guide,0,0.).unwrap().with_affine_laws(&axes,&center).unwrap();
    assert!(sweep.certify_closed_guided_frame_smoothness(1,100000,1000000).unwrap().closed_source_frame_smoothness_certified);
    let mut r=report();assert!(inspect(&sweep.sweeps[0],[&cap,&cap],&mut r,100000,100000,true).is_none());
    assert!(!r.embedding_certified && !r.regularity_certified && !r.parameter_topology_preserved);
    let mut r=report();let Some(w)=inspect(&sweep.sweeps[0],[&cap,&cap],&mut r,100000,100000,false) else{panic!("open interval endpoints really coincide")};
    assert_eq!(w.raw_path_parameters,[0.,4.]);
}
#[test]
fn original_rational_anchor_and_unequal_law_weights_are_not_replaced_by_the_cached_point() {
    let path=source::figure_eight();let mut guide=path.clone();
    let rail=27./1280.;for p in &mut guide.control_points {p[2]+=rail;}
    let mut profile=crate::primitives::line([0.,0.,source::R/2.],[0.,0.,source::R]).unwrap();
    profile.knots=vec![7.,7.,11.,11.];profile.weights=vec![1.,4.];let profiles=[profile];
    let (_,cap,trims)=source::material(false,true);
    let mut scale=source::periodic(1.,1.125);scale.weights[3]=2.;
    let mut center=source::periodic(0.,source::R/4.);center.weights[3]=0.5;
    let mut axes=scale.clone();axes.knots=[vec![-5.;7],vec![13.;7]].concat();
    axes.control_points=vec![vec![1.,1.125,0.875];7];axes.control_points[3]=vec![1.25,1.5,1.];
    let twist=constant_vector_law([0.;3]).unwrap();
    let sweep=MultiSweep::new(&profiles,&path,&scale,&twist,Options{normal:[0.,0.,1.],orientation:Orientation::RotationMinimizing,
        spacing:Spacing::Parameter,initial_sections:5,max_sections:33,max_deviation:0.05}).unwrap()
        .with_contact_guide(&guide,0,9.).unwrap().with_affine_laws(&axes,&center).unwrap();
    assert!(sweep.certify_closed_guided_frame_smoothness(1,100000,1000000).unwrap().closed_source_frame_smoothness_certified);
    // The exact original profile value at authored parameter 9 is 27/1280;
    // the accepted binary64 rail/cache value differs from that rational.
    assert_ne!(Q::from_float(rail).unwrap(),Q::new(27.into(),1280.into()));
    let mut r=report();let Some(witness)=inspect(&sweep.sweeps[0],[&cap,&cap],&mut r,100000,100000,true) else{panic!("rational original anchor was rounded")};
    assert_eq!(witness.raw_path_parameters,[0.,2.]);
    assert_ne!(witness.normalized_cap_parameters[1][0],Q::new(103.into(),532.into()));
    let full=sweep.certify_ideal_volume_atlas([&cap,&cap],report().projection,1000000,1000000).unwrap();
    assert_eq!(full.reason,Some("ideal-volume-exact-filled-map-coincidence"));
    assert!(!full.embedding_certified && !full.regularity_certified && !full.parameter_topology_preserved);
    eprintln!("exact rational original anchor: primitive cells={} exact={}, whole cells={} exact={}",r.cells,r.exact_work,full.cells,full.exact_work);
    #[cfg(feature="codec")]
    eprintln!("exact-contact-witness={}",value_codec::json!({"axis":2,"normalSign":1.,"anchorParameter":9.,
        "rawPathParameters":witness.raw_path_parameters,"capNormalizedUV":witness.normalized_cap_parameters.map(|uv|uv.map(|x|x.to_string())),
        "world":witness.world.map(|x|x.to_string()),"path":path,"guide":guide,"profiles":profiles,
        "surface":cap,"trims":trims,"scale":scale,"axes":axes,"center":center,"twist":twist}));
}
