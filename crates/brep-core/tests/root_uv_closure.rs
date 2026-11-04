use brep_core::root_model::{RootBrep,QualifiedRootUvClosure,qualify_root_uv_joint};
use std::sync::Arc;
#[path="../../nurbs-core/tests/support/ss_root_circle.rs"] mod fixture;
#[test]
fn both_face_projections_have_exact_uv_closed_incidence() {
    let wire=fixture::wire();
    let source=Arc::new(RootBrep::paired_contours(&wire,&[],1e-7).unwrap());
    let proof=QualifiedRootUvClosure::qualify(source.clone(),16).unwrap();
    assert_eq!(proof.joints(),16);assert!(Arc::ptr_eq(proof.source(),&source));proof.reverify().unwrap();
    assert!(QualifiedRootUvClosure::qualify(source.clone(),15).is_err());
    assert!(QualifiedRootUvClosure::qualify(source,0).is_err());
    let a=wire.edges()[0].end();let b=wire.edges()[1].start();
    qualify_root_uv_joint(a,0,b,0).unwrap();
    assert!(qualify_root_uv_joint(a,0,b,1).is_err());
    assert!(qualify_root_uv_joint(a,2,b,0).is_err());
    assert!(qualify_root_uv_joint(a,0,wire.edges()[2].start(),0).is_err());
}
#[test]
fn reversed_coedge_traversal_checks_its_actual_endpoints() {
    let original=RootBrep::paired_contours(&fixture::wire(),&[],1e-7).unwrap();
    let mut t=original.topology().clone();
    for l in &mut t.loops {
        l.coedges.reverse();
        for c in &mut l.coedges {c.reversed=!c.reversed;c.pcurve.reversed=c.reversed;}
    }
    let model=Arc::new(RootBrep::from_topology(t).unwrap());
    let proof=QualifiedRootUvClosure::qualify(model,16).unwrap();
    assert_eq!(proof.joints(),16);proof.reverify().unwrap();
}

use nurbs_core::{ss_intersection as ss,surface::Surface};
use num_rational::BigRational as Q;
fn sheet(x:f64,y:f64)->Surface {
    Surface {degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],
        control_points:vec![vec![vec![x,y,-1.],vec![x,y,1.]],vec![vec![x,y+1.,-1.],vec![x,y+1.,1.]]],
        weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false}
}
fn candidate(curve:&Arc<ss::OriginalSsRootCurve>,third:&Surface)->Arc<ss::QualifiedSsThirdSurfaceCandidates> {
    Arc::new(curve.third_surface_candidates(third,[1,1],Default::default()).unwrap())
}
fn point(curve:Arc<ss::OriginalSsRootCurve>,candidate:&ss::QualifiedSsThirdSurfaceCandidates)->Arc<ss::OriginalSsRootPoint> {
    let clock=Arc::new(candidate.qualify_candidate_clock([Q::from_float(0.688).unwrap(),Q::from_float(0.6894).unwrap()],Default::default()).unwrap());
    Arc::new(ss::OriginalSsRootPoint::from_algebraic(ss::OriginalSsAlgebraicPoint::new(curve,clock).unwrap(),Default::default()).unwrap())
}
fn target_pair(w:&ss::QualifiedSsThirdSurfacePoint, roles:[usize;2])->Arc<ss::OriginalSsRootCurve> {
    let surfaces=[w.source().original().source().first(),w.source().original().source().second(),w.source().third()];
    let src=ss::FiberSources::new(surfaces[roles[0]],surfaces[roles[1]]).unwrap();
    let spans=roles.map(|r| if r==1 {[2,2]} else {[1,1]});
    let ideal=ss::qualify_original_ss_ideal(&src,[spans[0][0],spans[0][1],spans[1][0],spans[1][1]],Default::default()).unwrap();
    let gb=ss::qualify_ss_groebner_basis(&ideal,Default::default()).unwrap();
    let sys=ss::qualify_ss_deflated_system(&gb,Default::default()).unwrap();
    let mut domain=roles.map(|r| if r==2 {[[0.67,0.70],[0.45,0.55]]} else {[[0.24,0.27],[0.67,0.70]]});
    domain[1][if roles[1]==2 {0} else {1}]=[0.66,0.71];
    let axis=if roles[0]==2 {ss::FiberAxis::FirstU} else {ss::FiberAxis::FirstV};
    let f=ss::qualify_deflated_ss_fiber(&sys,[domain[0][0],domain[0][1],domain[1][0],domain[1][1]],axis,Default::default()).unwrap();
    let report=format!("{:?}",f.report());
    Arc::new(ss::OriginalSsRootCurve::from_deflated(f.into_fiber().unwrap_or_else(||panic!("{report}"))))
}

#[test]
fn mixed_pair_uv_joint_uses_the_common_surface_role() {
    let wire=fixture::wire();let curve=wire.edges()[2].curve().clone();
    let source=candidate(&curve,&sheet(257./1024.,0.));let selected=point(curve,&source);
    let witness=Arc::new(ss::QualifiedSsThirdSurfacePoint::qualify(source,selected,Default::default()).unwrap());
    let ac=ss::bind_original_ss_third_port(witness.clone(),target_pair(&witness,[0,2]),[0,2],Default::default()).unwrap();
    let cb=ss::bind_original_ss_third_port(witness.clone(),target_pair(&witness,[2,1]),[2,1],Default::default()).unwrap();
    qualify_root_uv_joint(&ac,1,&cb,0).unwrap();
    assert!(qualify_root_uv_joint(&ac,0,&cb,1).is_err());
    ac.reverify().unwrap();cb.reverify().unwrap();
}
#[test]
fn actual_mixed_pair_face_preserves_world_and_uv_curves() {
    use brep_core::root_model::RootFaceContour;
    let wire=fixture::wire();let ab=wire.edges()[2].curve().clone();
    let third=sheet(13./64.,0.);let source=candidate(&ab,&third);
    let mut witnesses=Vec::new();
    for d in [[0.402,0.403],[0.597,0.598]] {
        let clock=Arc::new(source.qualify_candidate_clock(d.map(|x|Q::from_float(x).unwrap()),Default::default()).unwrap());
        let point=Arc::new(ss::OriginalSsRootPoint::from_algebraic(ss::OriginalSsAlgebraicPoint::new(ab.clone(),clock).unwrap(),Default::default()).unwrap());
        witnesses.push(Arc::new(ss::QualifiedSsThirdSurfacePoint::qualify(source.clone(),point,Default::default()).unwrap()));
    }
    let s=ss::FiberSources::new(ab.source().first(),&third).unwrap();
    let ideal=ss::qualify_original_ss_ideal(&s,[1;4],Default::default()).unwrap();
    let gb=ss::qualify_ss_groebner_basis(&ideal,Default::default()).unwrap();
    let sys=ss::qualify_ss_deflated_system(&gb,Default::default()).unwrap();
    let f=ss::qualify_deflated_ss_fiber(&sys,[[0.19,0.22],[0.39,0.61],[0.38,0.62],[0.4,0.6]],ss::FiberAxis::FirstV,Default::default()).unwrap();
    let ac=Arc::new(ss::OriginalSsRootCurve::from_deflated(f.into_fiber().unwrap()));
    let plain=|i:usize| ss::bind_original_ss_port(witnesses[i].point().clone(),ab.clone(),[0;4],Default::default()).unwrap();
    let cross=|i:usize| ss::bind_original_ss_third_port(witnesses[i].clone(),ac.clone(),[0,2],Default::default()).unwrap();
    // Split a complete existing face, sharing one inserted chord between children.
    let mut boundary=wire.edges()[..2].to_vec();
    boundary.push(ss::OriginalSsRootEdge::new(wire.edges()[2].start().clone(),plain(1)).unwrap());
    boundary.push(ss::OriginalSsRootEdge::new(plain(1),plain(0)).unwrap());
    boundary.push(ss::OriginalSsRootEdge::new(plain(0),wire.edges()[2].end().clone()).unwrap());
    boundary.extend_from_slice(&wire.edges()[3..]);
    let full=ss::OriginalSsRootWire::new(boundary).unwrap();
    let full=Arc::new(RootBrep::face_contours(ab.source().first().clone(),RootFaceContour {supports:vec![0;10],wire:full},vec![],1e-7).unwrap());
    let chord=ss::OriginalSsRootEdge::new(cross(0),cross(1)).unwrap();
    let split=brep_core::root_model::RootFaceChordSplit::split(full.clone(),0,chord.clone(),0,vec![]).unwrap();
    assert_eq!(split.face_children(),[0,1]);assert_eq!(split.inserted_edge(),10);
    let out=split.output();assert_eq!(out.topology().faces.len(),2);assert_eq!(out.topology().edges.len(),11);
    assert_eq!(out.topology().vertices.len(),10);assert_eq!(out.topology().shells[0].faces.len(),2);
    let uses:Vec<_>=out.topology().loops.iter().flat_map(|l|&l.coedges).filter(|c|c.edge==10).collect();
    assert_eq!(uses.len(),2);assert_ne!(uses[0].reversed,uses[1].reversed);
    QualifiedRootUvClosure::qualify(out.clone(),12).unwrap().reverify().unwrap();split.reverify().unwrap();
    for (face,small,large) in [(0,0,1),(1,1,0)] {
        use brep_core::root_model::QualifiedRootLoopWinding as W;
        assert_eq!(W::qualify(out.clone(),face,0,[[0.195;2],[0.5;2]],Default::default()).unwrap().winding(),small);
        assert_eq!(W::qualify(out.clone(),face,0,[[0.5;2],[0.5;2]],Default::default()).unwrap().winding(),large);
    }
    assert!(brep_core::root_model::RootFaceChordSplit::split(full,0,chord,1,vec![]).is_err());
    let contour=ss::OriginalSsRootWire::new(vec![ss::OriginalSsRootEdge::new(plain(0),plain(1)).unwrap(),ss::OriginalSsRootEdge::new(cross(1),cross(0)).unwrap()]).unwrap();
    let face=RootBrep::face_contours(ab.source().first().clone(),RootFaceContour {wire:contour.clone(),supports:vec![0,0]},vec![],1e-7).unwrap();
    assert_eq!(face.topology().edges.len(),2);assert_eq!(face.topology().vertices.len(),2);
    assert!(!face.topology().shells[0].closed);assert!(face.topology().bodies.is_empty());
    assert!(Arc::ptr_eq(face.topology().edges[0].curve.curve(),&ab));
    assert!(Arc::ptr_eq(face.topology().edges[1].curve.curve(),&ac));
    for i in 0..2 {assert!(face.pcurve_bounds(0,0,i,Default::default()).is_ok());}
    let face=Arc::new(face);
    QualifiedRootUvClosure::qualify(face.clone(),2).unwrap().reverify().unwrap();
    use brep_core::root_model::QualifiedRootLoopWinding as W;
    let inside=W::qualify(face.clone(),0,0,[[0.195;2],[0.5;2]],Default::default()).unwrap();
    assert_eq!(inside.winding(),-1);inside.reverify().unwrap();
    assert_eq!(W::qualify(face.clone(),0,0,[[0.194,0.196],[0.49,0.51]],Default::default()).unwrap().winding(),-1);
    let mut reversed=face.topology().clone();
    reversed.loops[0].coedges.reverse();
    for c in &mut reversed.loops[0].coedges {c.reversed=true;c.pcurve.reversed=true;}
    let reversed=Arc::new(RootBrep::from_topology(reversed).unwrap());
    assert_eq!(W::qualify(reversed,0,0,[[0.195;2],[0.5;2]],Default::default()).unwrap().winding(),1);
    assert_eq!(W::qualify(face.clone(),0,0,[[0.21;2],[0.5;2]],Default::default()).unwrap().winding(),0);
    assert!(W::qualify(face.clone(),0,0,[[13./64.;2],[0.5;2]],ss::OriginalSsWindingLimits {max_segments:16,..Default::default()}).is_err());
    assert!(W::qualify(face,0,0,[[0.195;2],[0.5;2]],ss::OriginalSsWindingLimits {max_segments:1,..Default::default()}).is_err());
    assert!(RootBrep::face_contours(ab.source().first().clone(),RootFaceContour {wire:contour,supports:vec![0,1]},vec![],1e-7).is_err());
}
