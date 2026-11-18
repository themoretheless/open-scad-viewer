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

#[test]
fn closed_original_frenet_geometric_c2_seam_does_not_require_equal_raw_speed() {
    for split in [false,true] {
    let p=if split {frenet_source::split_reclocked_path()}else{frenet_source::reclocked_path()};let (profiles,cap,_)=source::material(false,true);
    let scale=source::periodic(1.,1.03125);let zero=constant_vector_law([0.;3]).unwrap();let before=p.clone();
    #[cfg(feature="codec")]
    {
        let mut reference=frenet_source::path();if split {reference.weights.fill(1.);}
        eprintln!("geometric-frenet-source={}",value_codec::json!({"path":p,"reference":reference,"sourceClock":if split {"split-mobius"}else{"mobius"},"scale":scale,"twist":zero,"orientation":"frenet","clock":"arc-length"}));
    }
    let raw=MultiSweep::new(&profiles,&p,&scale,&zero,options(false)).unwrap();
    assert!(!raw.certify_closed_path_frame_smoothness(2,100000,1000000).unwrap().closed_source_frame_smoothness_certified);
    let sweep=MultiSweep::new(&profiles,&p,&scale,&zero,options(true)).unwrap();
    let seam=sweep.certify_closed_path_frame_smoothness(2,100000,1000000).unwrap();
    assert!(seam.closed_source_frame_smoothness_certified,"{seam:?}");
    assert!(sweep.certify_closed_path_frame_smoothness(2,seam.cells,seam.exact_work).unwrap().closed_source_frame_smoothness_certified);
    for (cells,work) in [(0,seam.exact_work),(seam.cells-1,seam.exact_work),(seam.cells,0),(seam.cells,seam.exact_work-1)] {
        let q=sweep.certify_closed_path_frame_smoothness(2,cells,work).unwrap();assert!(!q.closed_source_frame_smoothness_certified && q.cells<=cells && q.exact_work<=work);
    }
    let q=sweep.certify_ideal_volume_atlas([&cap,&cap],[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]],1000000,1000000).unwrap();
    assert!(q.embedding_certified && q.regularity_certified && q.parameter_topology_preserved,"{q:?}");
    for warped in [false,true] {
        let (profiles,cap,_)=source::material(false,warped);let twist=source::periodic(0.,1./1024.);
        let center=source::periodic(0.,source::R/16.);let mut axes=source::periodic(1.,1.0078125);
        for q in &mut axes.control_points {q[1]=1.125;q[2]=0.875;}
        let dynamic=MultiSweep::new(&profiles,&p,&scale,&twist,options(true)).unwrap().with_affine_laws(&axes,&center).unwrap();
        let r=dynamic.certify_ideal_volume_atlas([&cap,&cap],[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]],1000000,1000000).unwrap();
        assert!(r.embedding_certified && r.regularity_certified && r.parameter_topology_preserved,"split={split} warped={warped}: {r:?}");
        eprintln!("geometric Frenet dynamic split={split} warped={warped}: {r:?}");
    }
    let mut wrong=p.clone();wrong.control_points[7][2]=wrong.control_points[7][2].next_up();
    let changed=MultiSweep::new(&profiles,&wrong,&scale,&zero,options(true)).unwrap();
    assert!(changed.certify_closed_path_frame_smoothness(1,100000,1000000).unwrap().closed_source_frame_smoothness_certified);
    let refused=changed.certify_closed_path_frame_smoothness(2,100000,1000000).unwrap();
    assert!(!refused.closed_source_frame_smoothness_certified);assert_eq!(refused.reason,Some("closed-frenet-geometric-direction-jets-unproved"));
    let mut gap=p.clone();for q in &mut gap.control_points {q[0]+=1.;}gap.control_points.last_mut().unwrap()[0]=1f64.next_up();
    let changed=MultiSweep::new(&profiles,&gap,&scale,&zero,options(true)).unwrap();
    let refused=changed.certify_closed_path_frame_smoothness(2,100000,1000000).unwrap();
    assert!(!refused.closed_source_frame_smoothness_certified);assert_eq!(refused.reason,Some("closed-frenet-geometric-position-unproved"));
    assert_eq!(p,before);eprintln!("geometric Frenet split={split} C2 seam={seam:?}; complete original body={q:?}");
    }
}

#[test]
fn periodic_active_basis_frenet_certifies_the_original_filled_volume() {
    let path = frenet_source::periodic_path();
    let (profiles, cap, _) = source::material(true, true);
    let scale = source::periodic(1., 1.03125);
    let twist = source::periodic(0., 1. / 1024.);
    for arc in [false, true] {
        let sweep = MultiSweep::new(&profiles, &path, &scale, &twist, options(arc)).unwrap();
        let seam = sweep.certify_closed_path_frame_smoothness(2, 100000, 1000000).unwrap();
        assert!(seam.closed_source_frame_smoothness_certified, "{seam:?}");
        let body = sweep.certify_ideal_volume_atlas(
            [&cap, &cap], [[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]], 1000000, 1000000,
        ).unwrap();
        assert!(body.embedding_certified && body.regularity_certified && body.parameter_topology_preserved, "arc={arc}: {body:?}");
        eprintln!("periodic active basis arc={arc} body={body:?}");
    }
}

#[test]
fn periodic_frenet_rejects_an_inexact_exterior_knot_seam() {
    let path = frenet_source::periodic_path();
    let mut changed = path.clone();
    changed.knots[1] = changed.knots[1].next_up();
    changed.validate().unwrap();
    let (profiles, cap, _) = source::material(false, true);
    let scale = source::periodic(1., 1.03125);
    let zero = constant_vector_law([0.; 3]).unwrap();
    let sweep = MultiSweep::new(&profiles, &changed, &scale, &zero, options(true)).unwrap();
    assert!(sweep.source_is_closed().unwrap());
    let seam = sweep.certify_closed_path_frame_smoothness(2, 100000, 1000000).unwrap();
    assert!(!seam.closed_source_frame_smoothness_certified, "{seam:?}");
    let body = sweep.certify_ideal_volume_atlas(
        [&cap, &cap], [[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]], 1000000, 1000000,
    ).unwrap();
    assert!(!body.embedding_certified && !body.regularity_certified && !body.parameter_topology_preserved, "{body:?}");
    assert_eq!(body.reason, Some("ideal-volume-exact-source-seam-unproved"));
}

#[test]
fn periodic_path_owns_authored_and_guided_closed_volume_charts() {
    let path = frenet_source::periodic_path();
    let axis = frenet_source::periodic_authored_axis(&path);
    let normal = constant_vector_law([0., 0., 1.]).unwrap();
    let mut guide = path.clone();
    for p in &mut guide.control_points {
        p[2] += 0.125;
    }
    let (profiles, cap, _) = source::material(true, true);
    let scale = source::periodic(1., 1.03125);
    let twist = source::periodic(0., 1. / 1024.);
    for arc in [false, true] {
        for authored in [false, true] {
            let orientation = if authored { Orientation::Fixed } else { Orientation::FixedNormal };
            let sweep = MultiSweep::new(&profiles, &path, &scale, &twist, Options {
                orientation, ..options(arc)
            }).unwrap();
            let sweep = if authored {
                sweep.with_frame_laws(&axis, &normal)
            } else {
                sweep.with_orientation_guide(&guide)
            }.unwrap();
            let body = sweep.certify_ideal_volume_atlas(
                [&cap, &cap], [[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]], 1000000, 1000000,
            ).unwrap();
            assert!(body.embedding_certified && body.regularity_certified && body.parameter_topology_preserved,
                "arc={arc} authored={authored}: {body:?}");
            eprintln!("periodic volume arc={arc} authored={authored}: {body:?}");
        }
    }
}

#[test]
fn periodic_guide_and_authored_axis_require_exact_active_seam() {
    let path = frenet_source::periodic_path();
    let (profiles, cap, _) = source::material(false, true);
    let scale = source::periodic(1., 1.03125);
    let zero = constant_vector_law([0.; 3]).unwrap();
    let normal = constant_vector_law([0., 0., 1.]).unwrap();
    let mut guide = path.clone();
    for point in &mut guide.control_points {
        point[2] += 0.125;
    }
    guide.knots[1] = guide.knots[1].next_up();
    guide.validate().unwrap();

    let mut axis = frenet_source::periodic_authored_axis(&path);
    axis.knots[1] = axis.knots[1].next_up();
    axis.validate().unwrap();
    let projection = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    for arc in [false, true] {
        let guided = MultiSweep::new(&profiles, &path, &scale, &zero, options(arc))
            .unwrap().with_orientation_guide(&guide).unwrap();
        let proof = guided.certify_closed_guided_frame_smoothness(2, 100000, 1000000).unwrap();
        assert!(!proof.closed_source_frame_smoothness_certified, "{proof:?}");
        let body = guided.certify_ideal_volume_atlas([&cap, &cap], projection, 1000000, 1000000).unwrap();
        assert_eq!(body.reason, Some("ideal-volume-exact-source-seam-unproved"));
        assert!(!body.embedding_certified && !body.regularity_certified && !body.parameter_topology_preserved);

        let authored = MultiSweep::new(&profiles, &path, &scale, &zero, Options {
            orientation: Orientation::Fixed, ..options(arc)
        }).unwrap().with_frame_laws(&axis, &normal).unwrap();
        let proof = authored.certify_closed_authored_frame_smoothness(2, 100000, 1000000).unwrap();
        assert!(!proof.closed_source_frame_smoothness_certified, "{proof:?}");
        let body = authored.certify_ideal_volume_atlas([&cap, &cap], projection, 1000000, 1000000).unwrap();
        assert!(!body.embedding_certified && !body.regularity_certified && !body.parameter_topology_preserved, "{body:?}");
    }
}

#[test]
fn periodic_spatial_bishop_owns_the_closed_volume_and_rejects_a_broken_seam() {
    let path = frenet_source::periodic_path();
    let mut damaged = path.clone();
    damaged.knots[1] = damaged.knots[1].next_up();
    damaged.validate().unwrap();
    let (profiles, cap, _) = source::material(true, true);
    let scale = source::periodic(1., 1.03125);
    let twist = source::periodic(0., 1. / 1024.);
    let projection = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    for arc in [false, true] {
        for (source, expected) in [(&path, true), (&damaged, false)] {
            let sweep = MultiSweep::new(&profiles, source, &scale, &twist, Options {
                normal: [0., 0., 1.],
                orientation: Orientation::RotationMinimizing,
                ..options(arc)
            }).unwrap().with_spatial_rmf_error_limits(4096, 100000, 1000000).unwrap();
            let seam = sweep.certify_closed_path_frame_smoothness(2, 100000, 1000000).unwrap();
            assert_eq!(seam.closed_source_frame_smoothness_certified, expected, "{seam:?}");
            let body = sweep.certify_ideal_volume_atlas(
                [&cap, &cap], projection, 1000000, 1000000,
            ).unwrap();
            assert_eq!([body.embedding_certified, body.regularity_certified, body.parameter_topology_preserved],
                [expected; 3], "arc={arc}, exact seam={expected}: {body:?}");
            eprintln!("periodic Bishop arc={arc} exact seam={expected} body={body:?}");
        }
    }
}

#[test]
fn periodic_bishop_frame_does_not_certify_a_double_traversal() {
    let mut path = frenet_source::periodic_path();
    let cycle = path.control_points[..8].to_vec();
    path.control_points.splice(8..8, cycle);
    path.weights = vec![1.; path.control_points.len()];
    path.knots = (0..27).map(|i| (f64::from(i) - 5.) / 8.).collect();
    path.validate().unwrap();
    let (profiles, cap, _) = source::material(false, false);
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let zero = constant_vector_law([0.; 3]).unwrap();
    let sweep = MultiSweep::new(&profiles, &path, &scale, &zero, Options {
        normal: [0., 0., 1.],
        orientation: Orientation::RotationMinimizing,
        ..options(true)
    }).unwrap().with_spatial_rmf_error_limits(4096, 100000, 1000000).unwrap();
    let seam = sweep.certify_closed_path_frame_smoothness(2, 100000, 1000000).unwrap();
    assert!(seam.closed_source_frame_smoothness_certified, "{seam:?}");
    // The center of this filled cap coincides with the exact source start.
    // Both traversals carry this same material point through the origin,
    // independently of which rotation the transported frames have there.
    let body = sweep.certify_ideal_volume_atlas(
        [&cap, &cap], [[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]], 500000, 1000000,
    ).unwrap();
    assert!(!body.embedding_certified && !body.regularity_certified && !body.parameter_topology_preserved,
        "{body:?}");
    assert!(body.boxes > 0 && body.cells <= 500000, "{body:?}");
    eprintln!("periodic Bishop double traversal refused: {body:?}");
}
