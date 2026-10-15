use super::*;
use nurbs_core::{
    primitives::{circle, line},
    progressive_sweep::{Options, Orientation, Spacing, constant_vector_law},
};

fn limits() -> Limits {
    Limits {
        exact_work: 1_000_000,
        products: 100_000,
        trim_pairs: 10_000,
        trim_cells: 100_000,
        trim_domain_cells: 1_000_000,
        spans: 1024,
        linear_cells: 100_000,
        contacts: crate::face_contacts::Limits {
            pairs: 10_000,
            cells: 100_000,
            domain_cells: 1_000_000,
            cells_per_pair: 1000,
            domain_cells_per_pair: 10_000,
        },
    }
}
fn options(orientation: Orientation, initial: usize, max: usize) -> Options {
    Options {
        normal: [0., 0., 1.],
        orientation,
        spacing: Spacing::Parameter,
        initial_sections: initial,
        max_sections: max,
        max_deviation: 0.01,
    }
}
fn make(kind:usize)->Boundary {make_with_policy(kind,false)}
fn make_with_policy(kind:usize,automatic:bool)->Boundary {
    let axis = if kind == 0 {
        [3., 4., 0.]
    } else {
        [1., 0., 0.]
    };
    let loops = vec![
        vec![circle([0.; 3], axis, 0.1).unwrap()],
        vec![circle([0.; 3], axis, 0.05).unwrap().reverse().unwrap()],
    ];
    let path = if kind == 0 {
        line([0.; 3], [3., 4., 0.]).unwrap()
    } else {
        Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0.; 3], vec![0.5, 0., 0.], vec![1., 1., 0.]],
            weights: vec![1.; 3],
            periodic: false,
        }
    };
    let scale = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![1., 0., 0.],
            vec![if kind == 0 { 2. } else { 1. }, 0., 0.],
        ],
        weights: vec![1.; 2],
        periodic: false,
    };
    let twist = constant_vector_law([0.; 3]).unwrap();
    let orientation = if kind == 1 {
        Orientation::Frenet
    } else {
        Orientation::RotationMinimizing
    };
    let construct=if automatic {super::super::progressive_profile_body_with_automatic_caps}else{super::super::progressive_profile_body_with_annular_caps};
    let body = construct(
        &loops,
        &path,
        &scale,
        &twist,
        None,
        None,
        None,
        options(orientation, if kind == 0 { 2 } else { 3 }, 129),
        None,
        None,
        None,
        limits(),
    )
    .unwrap();
    let boundary = body
        .complete_boundary()
        .expect("Source-owned complete boundary must be proved");
    assert!(boundary.error_upper.unwrap() <= 0.01 && boundary.wall_error_upper.is_some());
    assert!(
        boundary
            .filled_cap_error_upper
            .unwrap()
            .iter()
            .all(|&e| e <= 1e-8)
    );
    let caps = body.caps().unwrap();
    assert_eq!(caps.face_groups().each_ref().map(|g| g.len()), [4, 4]);
    assert!(caps.exact_work() <= limits().exact_work);
    assert_eq!(
        caps.exact_work(),
        body.cap_selection_work()+caps.construction_work() + caps.filling().exact_work
    );
    assert!(body.evidence().retained_caps.is_none() && body.evidence().cap_projection.is_none());
    assert!(body.evidence().retained_walls.certified);
    assert!(body.model().faces.len() <= crate::MAX_FACES);
    body
}

#[test]
fn owned_oblique_rmf_full_boundary_and_cap_work_refusals() {
    let body = make(0);
    let axis = [3., 4., 0.];
    let profiles = [
        circle([0.; 3], axis, 0.1).unwrap(),
        circle([0.; 3], axis, 0.05).unwrap().reverse().unwrap(),
    ];
    let path = line([0.; 3], axis).unwrap();
    let scale = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![1., 0., 0.], vec![2., 0., 0.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let twist = constant_vector_law([0.; 3]).unwrap();
    let raw = super::super::rational_section_loft(&body.body.station_sections).unwrap();
    let source = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        options(Orientation::RotationMinimizing, 2, 129),
    )
    .unwrap();
    let short = Limits {
        exact_work: body.caps().unwrap().exact_work() - 1,
        ..limits()
    };
    match assemble(&source, &raw, &[1, 1], 0.01, 1e-9, short,0) {
        Ok(denied) => assert!(denied.evidence.filling().filled_cap_error_upper.is_none()),
        Err(_) => {}
    }
    let denied = assemble(
        &source,
        &raw,
        &[1, 1],
        0.01,
        1e-9,
        Limits {
            products: 0,
            ..limits()
        },
        0,
    )
    .unwrap();
    assert!(!denied.evidence.filling().within_budget);
    assert!(denied.evidence.filling().filled_cap_error_upper.is_none());
}

#[test]
fn owned_quadratic_frenet_full_boundary() {
    make(1);
}
#[test]
fn owned_planar_quadratic_rmf_full_boundary() {
    make(2);
}

#[test]
fn annular_station_reconstruction_preserves_all_caps_and_actual_g2() {
    let loops = vec![
        vec![circle([0.; 3], [0., 0., 1.], 0.5).unwrap()],
        vec![
            circle([0.; 3], [0., 0., 1.], 0.25)
                .unwrap()
                .reverse()
                .unwrap(),
        ],
    ];
    let path = line([0.; 3], [0., 0., 10.]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let mut opt = options(Orientation::Fixed, 3, 3);
    opt.normal = [1., 0., 0.];
    let mut body = super::super::progressive_profile_body_with_annular_caps(
        &loops,
        &path,
        &scale,
        &twist,
        None,
        None,
        None,
        opt,
        None,
        None,
        None,
        limits(),
    )
    .unwrap();
    assert!(body.complete_boundary().is_some());
    let full = body.reconstruct_stations(0.125, 1e-9, 1_000_000).unwrap();
    let reconstructed = full.reconstruction.as_ref().unwrap();
    let model = reconstructed
        .model
        .as_ref()
        .expect("Complete owned annular reconstruction");
    assert!(reconstructed.candidate.work <= 1_000_000);
    assert!(
        full.boundary.as_ref().unwrap().continuous_bound
            && full.boundary.as_ref().unwrap().within_budget == Some(true)
    );
    preserve_groups(body.model(), model, body.caps().unwrap().face_groups()).unwrap();
    let jets =
        crate::sweep_smoothness::inspect_profile(model, &body.cap_faces(), 2_000_000).unwrap();
    assert!(
        jets.station.g2_certified,
        "Actual reconstructed station jets must be G2"
    );
    assert!(body.reconstruct_stations(0.125, 1e-9, 0).is_err());
    let face = body.cap_faces()[0];
    let pole = &mut body.body.model.faces[face].surface.control_points[0][0][0];
    *pole = pole.next_up();
    assert!(body.reconstruct_stations(0.125, 1e-9, 1_000_000).is_err());
}


#[test]
fn automatic_caps_replace_only_unproved_coedges_and_charge_selection() {
    for kind in 0..3 {
        let body=make_with_policy(kind,true);
        let caps=body.caps().expect("Unproved old endpoint planes need native annular patches");
        assert!(body.complete_boundary().is_some());
        assert!(body.cap_selection_work()>0);
        assert_eq!(caps.exact_work(),body.cap_selection_work()+caps.construction_work()+caps.filling().exact_work);
        assert!(caps.exact_work()<=limits().exact_work);
        assert_eq!(body.cap_faces().len(),8);
        assert!(body.planar_cap_faces().is_empty());
    }
}

#[test]
fn automatic_caps_preserve_exact_legacy_geometry_and_shared_refusal() {
    let loops=vec![vec![circle([0.;3],[0.,0.,1.],0.5).unwrap()],
        vec![circle([0.;3],[0.,0.,1.],0.25).unwrap().reverse().unwrap()]];
    let path=line([0.;3],[0.,0.,10.]).unwrap();
    let scale=constant_vector_law([1.,0.,0.]).unwrap();
    let twist=constant_vector_law([0.;3]).unwrap();
    let opts=Options{normal:[1.,0.,0.],..options(Orientation::Fixed,3,3)};
    let before=super::super::progressive_profile_body_with_rmf_policy_and_corrections(&loops,&path,&scale,&twist,None,None,None,opts,None,None,None).unwrap();
    let body=super::super::progressive_profile_body_with_automatic_caps(&loops,&path,&scale,&twist,None,None,None,opts,None,None,None,limits()).unwrap();
    assert!(body.caps().is_none());
    assert_eq!(*body.model(),before.model);
    assert!(body.complete_boundary().is_some());
    assert!(body.cap_selection_work()>0);
    let caps=body.evidence().retained_caps.as_ref().unwrap();
    assert!(body.cap_selection_work()+caps.exact_work<=limits().exact_work);
    assert_eq!(body.cap_faces(),vec![16,17]);
    assert_eq!(body.planar_cap_faces(),vec![16,17]);
    let empty=Limits{exact_work:0,..limits()};
    assert!(super::super::progressive_profile_body_with_automatic_caps(&loops,&path,&scale,&twist,None,None,None,opts,None,None,None,empty).is_err());
}

#[test]
fn automatic_caps_keep_full_legacy_face_capacity_and_refuse_annular_overflow() {
    let polygon=|points:Vec<[f64;3]>| (0..points.len()).map(|i|line(points[i],points[(i+1)%points.len()]).unwrap()).collect::<Vec<_>>();
    let loops=vec![polygon(vec![[0.,0.,0.],[2.,0.,0.],[4.,1.,0.],[4.,3.,0.],[3.,4.,0.],[1.,4.,0.],[0.,2.,0.]])];
    let path=line([0.;3],[0.,0.,10.]).unwrap();
    let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
    let opts=Options{normal:[1.,0.,0.],..options(Orientation::Fixed,147,147)};
    let body=super::super::progressive_profile_body_with_automatic_caps(&loops,&path,&scale,&twist,None,None,None,opts,None,None,None,limits()).unwrap();
    let legacy=super::super::progressive_profile_body_with_rmf_policy_and_corrections(&loops,&path,&scale,&twist,None,None,None,opts,None,None,None).unwrap();
    assert_eq!(body.model().faces.len(),crate::MAX_FACES);
    assert_eq!(*body.model(),legacy.model);
    assert!(body.caps().is_none());assert!(body.complete_boundary().is_some());
    // 85 wall intervals * 12 spans + two old caps fit (1022),
    // while twelve radial cap patches would exceed1024.
    let points=vec![[0.,0.,0.1],[-0.08,0.06,0.05],[-0.08,0.06,-0.05],
        [0.,0.,-0.1],[0.08,-0.06,-0.05],[0.08,-0.06,0.05]];
    let inner=points.iter().rev().map(|p|p.map(|v|v*0.5)).collect();
    let loops=vec![polygon(points),polygon(inner)];
    let path=line([0.;3],[3.,4.,0.]).unwrap();
    let opts=options(Orientation::RotationMinimizing,86,86);
    let result=super::super::progressive_profile_body_with_automatic_caps(&loops,&path,&scale,&twist,None,None,None,opts,None,None,None,limits());
    match result {Err(error)=>assert!(error.to_string().contains("shared body face budget"),"{error}"),Ok(_)=>panic!("Overflowing radial caps must not discard stations or faces")}
}

#[test]
fn automatic_oblique_station_reconstruction_retries_degree_and_preserves_all_caps() {
    let loops=vec![
        vec![circle([0.;3],[3.,4.,0.],0.1).unwrap()],
        vec![circle([0.;3],[3.,4.,0.],0.05).unwrap().reverse().unwrap()],
    ];
    let path=line([0.;3],[3.,4.,0.]).unwrap();
    let scale=Curve{degree:1,knots:vec![0.,0.,1.,1.],control_points:vec![vec![1.,0.,0.],vec![2.,0.,0.]],weights:vec![1.;2],periodic:false};
    let twist=constant_vector_law([0.;3]).unwrap();
    let body=super::super::progressive_profile_body_with_automatic_caps(
        &loops,&path,&scale,&twist,None,None,None,
        options(Orientation::RotationMinimizing,3,3),None,None,None,limits(),
    ).unwrap();
    assert!(body.caps().is_some());
    let caps=body.cap_faces();
    assert_eq!(caps.len(),8);
    let original=body.model().clone();
    let result=body.reconstruct_stations(2_f64.powi(-40),1e-9,1_000_000).unwrap();
    assert!(result.boundary.as_ref().is_some_and(|b|b.continuous_bound&&b.within_budget==Some(true)),"{}",result.reason);
    let reconstruction=result.reconstruction.as_ref().unwrap();
    assert!(reconstruction.candidate.work<=1_000_000);
    let model=reconstruction.model.as_ref().unwrap();
    preserve_groups(&original,model,&body.cap_face_groups()).unwrap();
    for face in &caps {assert_eq!(original.faces[*face],model.faces[*face]);}
    let smooth=crate::sweep_smoothness::inspect_profile(model,&caps,2_000_000).unwrap();
    assert!(smooth.station.g2_certified,"Actual saved station G2 must be proved");
    let policy=limits();
    let volume=crate::volume_validity::inspect_sweep(model,1e-8,
        crate::volume_validity::Limits {
            boundary:crate::boundary_embedding::Limits {exact_work:policy.exact_work,
                trim_pairs:policy.trim_pairs,trim_cells:policy.trim_cells,trim_domain_cells:policy.trim_domain_cells,
                spans:policy.spans,contacts:policy.contacts},
            nesting_pairs:1000,nesting_cells:100000,nesting_domain_cells:1000000,
            orientation_cells:100000,orientation_domain_cells:1000000,orientation_spans:100},
        policy.linear_cells,&[],crate::sweep_cap_contacts::Budgets {max_walls:1024,max_exact_work:1000000,
            max_chart_cells:1000,max_trim_pairs:10000,max_trim_cells:100000,max_trim_domain_cells:1000000}).unwrap();
    if !volume.proven {
        let b=&volume.boundary;
        eprintln!("oblique global: boundary={}, agreement={}, joins={}, trim={}, injective={}, pairs={}, next={:?}, nesting={:?}",
            b.proven,b.agreement.all_equal,b.agreement.all_joins_exact,b.trim.all_valid,
            b.intersections.faces.all_faces_injective,b.intersections.pairs.all_pairs_classified,
            b.intersections.pairs.next_pair,volume.nesting.as_ref().map(|n|n.roles_consistent));
        for pair in b.intersections.pairs.pairs.iter().filter(|p|p.reason=="pair-unresolved") {
            eprintln!("oblique unresolved pair: {pair:?}");
            #[cfg(feature="codec")]
            {
                let [a,b]=pair.faces.map(|f|&model.faces[f]);
                let common=model.loops[a.outer].coedges.iter().find_map(|ca|
                    model.loops[b.outer].coedges.iter().find(|cb|cb.edge==ca.edge).map(|cb|(ca,cb)));
                if let Some((ca,cb))=common {
                    eprintln!("oblique-pair-fixture={}",value_codec::to_string(&value_codec::json!({
                        "faces":pair.faces,"surfaces":[&a.surface,&b.surface],"pcurves":[&ca.pcurve,&cb.pcurve]
                    })).unwrap());
                }
            }
        }
    }
    assert!(volume.proven,"Reconstructed actual annular body must retain fresh material validity");
    let pairs=&volume.boundary.intersections.pairs;
    assert!(pairs.all_pairs_classified&&pairs.next_pair.is_none());
    assert_eq!(pairs.total_pairs,model.faces.len()*(model.faces.len()-1)/2);
    assert_eq!(pairs.pairs.len()+pairs.grouped_pairs,pairs.total_pairs);
    let zero=body.reconstruct_stations(2_f64.powi(-40),1e-9,0);
    assert!(zero.is_err()||zero.as_ref().is_ok_and(|r|r.reconstruction.as_ref().is_none_or(|c|c.model.is_none())));
}

#[test]
fn applicable_cap_jets_are_not_starved_by_raw_curved_wall_diagnostics() {
    let body=make_with_policy(1,true);
    let before=body.model().clone();
    let report=body.inspect_smoothness(2_000_000).unwrap();
    assert_eq!(report.cap_seams.len(),2);
    assert!(report.cap_seams[0].g2_certified,"Actual initial circular cap G2 must be audited before raw wall refusals");
    assert_eq!(report.cap_seams[0].edge_ids.len(),4);
    let cap_work=report.cap_seams.iter().map(|cap|cap.exact_work).sum::<u64>();
    assert_eq!(report.walls.profile.max_work,2_000_000-cap_work);
    assert_eq!(report.exact_work,cap_work+report.walls.total_exact_work);
    assert!(report.exact_work<=2_000_000);
    let zero=body.inspect_smoothness(0).unwrap();
    assert_eq!(zero.exact_work,0);
    assert!(zero.cap_seams.iter().all(|cap|!cap.g1_certified&&!cap.g2_certified));
    assert_eq!(body.model(),&before);
}

#[test]
fn transformed_annular_internal_g2_covers_every_cap_seam() {
    use crate::sweep_smoothness::geometric::{inspect_edges,Tolerances};
    let t=Tolerances{position:0.,normal:1e-7,curvature:1e-5};
    for kind in 0..3 {
        let body=make_with_policy(kind,true);
        let before=body.model().clone();
        let ownership=body.inspect_smoothness(0).unwrap();
        for (cap,scope) in ownership.cap_seams.iter().enumerate() {
            assert!(scope.extraction_complete);assert_eq!(scope.edge_ids.len(),4);
            let report=inspect_edges(body.model(),&scope.edge_ids,t,128).unwrap();
            for seam in &report.seams {eprintln!("kind={kind} cap={cap} edge={} {:?}",seam.edge,seam.inspection);}
            assert!(report.complete&&report.within_tolerance,"All transformed annular cap seams must have whole-interval geometric G2 bounds");
            assert!(report.cells<=128);
            let zero=inspect_edges(body.model(),&scope.edge_ids,t,0).unwrap();
            assert!(!zero.complete&&!zero.within_tolerance&&zero.cells==0);
            let small=inspect_edges(body.model(),&scope.edge_ids,t,1).unwrap();
            assert!(!small.within_tolerance&&small.cells<=1);
            let mut kink=body.model().clone();
            let face=body.cap_face_groups()[cap][0];
            let normal=kink.faces[face].surface.evaluate(0.5,0.5).unwrap().unit_normal().unwrap();
            for axis in 0..3 {kink.faces[face].surface.control_points[1][0][axis]+=0.01*normal[axis];}
            let wrong=inspect_edges(&kink,&scope.edge_ids,t,128).unwrap();
            assert!(!wrong.within_tolerance,"A changed cap jet must not reuse a prior G2 result");
        }
        assert_eq!(body.model(),&before);
    }
}
