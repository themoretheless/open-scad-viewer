#[path = "support/closed_solid_policy.rs"]
mod solid_policy;
use brep_core::root_model::{RootBrep,QualifiedRootUvClosure,qualify_root_uv_joint};
use std::sync::Arc;
#[path="../../nurbs-core/tests/support/ss_root_circle.rs"] mod fixture;
#[path = "../../nurbs-core/tests/common/periodic_ss_wire.rs"]
mod periodic_wire;

#[test]
fn quotient_trim_audit_accepts_separated_winding_loops_and_refuses_overlap() {
    use brep_core::root_model::RootFaceContour;
    use nurbs_core::ss_intersection as ss;

    let lower = periodic_wire::wire();
    let surface = lower.edges()[0].curve().source().first().clone();
    let contour = |wire: ss::OriginalSsRootWire| RootFaceContour {
        supports: vec![0; wire.edges().len()], wire,
    };
    let reversed = |wire: &ss::OriginalSsRootWire| ss::OriginalSsRootWire::new(
        wire.edges().iter().rev().map(|edge| {
            ss::OriginalSsRootEdge::new(edge.end().clone(), edge.start().clone()).unwrap()
        }).collect(),
    ).unwrap();
    let strip = RootBrep::periodic_face_contours(
        surface.clone(), contour(lower.clone()),
        vec![contour(reversed(&periodic_wire::wire_at_height(1.5)))], 1e-7,
    ).unwrap();
    strip.audit_face_trim_arc_separation(0, Default::default()).unwrap();
    let overlapping = RootBrep::periodic_face_contours(
        surface, contour(lower), vec![contour(reversed(&periodic_wire::wire()))], 1e-7,
    ).unwrap();
    assert!(overlapping.audit_face_trim_arc_separation(0, Default::default()).is_err());
}

#[test]
fn opposite_periodic_boundary_windings_classify_strip_across_the_seam() {
    use brep_core::root_model::RootFaceContour;
    use nurbs_core::ss_intersection as ss;

    let a = periodic_wire::wire();
    let b = periodic_wire::wire_at_height(1.5);
    let surface = a.edges()[0].curve().source().first().clone();
    let opposite = ss::OriginalSsRootWire::new(b.edges().iter().rev().map(|edge| {
        ss::OriginalSsRootEdge::new(edge.end().clone(), edge.start().clone()).unwrap()
    }).collect()).unwrap();
    let contour = |wire: ss::OriginalSsRootWire| RootFaceContour {
        supports: vec![0; wire.edges().len()], wire,
    };
    let unmatched = Arc::new(RootBrep::periodic_face_contours(
        surface.clone(), contour(a.clone()), vec![], 1e-7,
    ).unwrap());
    assert!(unmatched.qualify_periodic_face_path(0, [3.95, 0.25], [4.05, 0.625], Default::default()).is_err());
    let face = Arc::new(RootBrep::periodic_face_contours(
        surface, contour(a.clone()), vec![contour(opposite)], 1e-7,
    ).unwrap());
    for (to, inside) in [([4.05, 0.625], true), ([4.05, 0.875], false)] {
        let proof = face.qualify_periodic_face_path(0, [3.95, 0.25], to, Default::default()).unwrap();
        assert_eq!(proof.material_at_end(false), inside);
        proof.reverify().unwrap();
    }
    for (point, inside) in [
        ([3.95, 0.625], true),
        ([4., 0.625], true),
        ([4.05, 0.625], true),
        ([8.05, 0.625], true),
        ([4.05, 0.25], false),
        ([4.05, 0.875], false),
    ] {
        let proof = face.qualify_periodic_face_membership(0, point, Default::default()).unwrap();
        assert_eq!(proof.inside(), inside, "{point:?}");
        proof.reverify().unwrap();
    }
    assert!(face.qualify_periodic_face_membership(0, [4.05, 0.5], Default::default()).is_err());

    for (query, inside) in [
        ([[3.95, 4.05], [0.6, 0.65]], true),
        ([[7.95, 8.05], [0.6, 0.65]], true),
        ([[3.95, 4.05], [0.2, 0.3]], false),
    ] {
        let proof = face.qualify_periodic_face_membership_box(0, query, Default::default()).unwrap();
        assert_eq!(proof.inside(), inside);
        proof.reverify().unwrap();
    }
    for query in [
        [[3.95, 4.05], [0.49, 0.65]],
        [[3.95, 4.05], [0.2, 0.99]],
        [[3.95, 4.05], [0.5, 0.65]],
    ] {
        assert!(face.qualify_periodic_face_membership_box(0, query, Default::default()).is_err());
    }

    use brep_core::closed_solid::{self, BooleanFaceDisposition, BooleanMaterialOp, BooleanSourceSide};
    use brep_core::root_model::QualifiedRootPeriodicFaceMaterial;
    let solid = closed_solid::qualify(
        brep_core::cuboid([-10., -10., 0.75], [10., 10., 1.75]).unwrap(),
        solid_policy::policy(),
    ).unwrap().into_solid().unwrap();
    let directions = [[0., 0., 1.], [1., 0.317, 0.173], [0.239, 1., 0.419]];
    let material = QualifiedRootPeriodicFaceMaterial::qualify(
        face.clone(), 0, [[3.95, 4.05], [0.6, 0.65]], &solid, &directions,
        Default::default(), Default::default(), 4096,
    ).unwrap();
    assert!(material.inside());
    let keep = material.boolean_contribution(BooleanMaterialOp::Intersection, BooleanSourceSide::First).unwrap();
    assert_eq!(keep.disposition(), BooleanFaceDisposition::Keep);
    let reverse = material.boolean_contribution(BooleanMaterialOp::Difference, BooleanSourceSide::Second).unwrap();
    assert_eq!(reverse.disposition(), BooleanFaceDisposition::Reverse);
    assert!(!keep.patch().unwrap().topology().shells[0].faces[0].reversed);
    assert!(reverse.patch().unwrap().topology().shells[0].faces[0].reversed);
    material.reverify().unwrap();

    use brep_core::root_model::{QualifiedRootShellPositiveFlux, RootPeriodicBandLimits, RootShellFluxLimits};
    let lower = periodic_wire::section_at_height(1.);
    let upper = periodic_wire::section_at_height(1.5);
    let mut plane = upper.source().second().clone();
    plane.control_points.swap(0, 1);
    let reversed_pair = ss::FiberSources::new(&plane, upper.source().first()).unwrap();
    let upper = ss::qualify_periodic_plane_ss(&reversed_pair, Default::default()).unwrap();
    let limits = RootPeriodicBandLimits {
        max_chart_attempts: 64,
        max_orientation_probes: 128,
        max_chart_spans: 8192,
        max_contact_cells: 4096,
        flux: RootShellFluxLimits {
            axis: 2, origin: 1., max_cells: 2048, max_spans: 8192, max_depth: 16,
            winding: Default::default(),
        },
    };
    assert!(QualifiedRootShellPositiveFlux::qualify_periodic_band([&lower, &lower], 1e-7, limits).is_err());
    let body = QualifiedRootShellPositiveFlux::qualify_periodic_band([&upper, &lower], 1e-7, limits).unwrap();
    let model = body.to_body().unwrap();
    assert_eq!(model.topology().faces.len(), 3);
    assert_eq!(model.topology().edges.len(), 8);
    assert!(model.topology().shells[0].closed);
    assert!(model.topology().boundary_edges().is_empty());
    assert_eq!(model.topology().bodies.len(), 1);
    let flux = body.report();
    let [lo, hi] = flux.signed_bounds;
    let expected = std::f64::consts::PI * 0.5;
    assert!(lo > 0. && lo <= expected && expected <= hi && hi - lo < 0.5, "{flux:?}");


}

#[test]
fn inferred_phase_closes_original_ss_ports_on_the_periodic_face() {
    use brep_core::root_model::{
        QualifiedRootLoopWinding, QualifiedRootPeriodicUvClosure, RootFaceContour,
        qualify_root_uv_joint_period_delta,
    };
    let wire = periodic_wire::wire();
    let edges = wire.edges();
    let end = edges.last().unwrap().end();
    let start = edges[0].start();
    assert_eq!(qualify_root_uv_joint_period_delta(end, 0, start, 0).unwrap(), [-1, 0]);
    assert_eq!(qualify_root_uv_joint_period_delta(start, 0, end, 0).unwrap(), [1, 0]);
    assert_eq!(qualify_root_uv_joint_period_delta(end, 1, start, 1).unwrap(), [0, 0]);
    assert!(qualify_root_uv_joint(end, 0, start, 0).is_err());
    let support = edges[0].curve().source().first().clone();
    let supports = vec![0; edges.len()];
    let face = Arc::new(RootBrep::periodic_face_contours(
        support, RootFaceContour { wire, supports }, vec![], 1e-7,
    ).unwrap());
    let closure = QualifiedRootPeriodicUvClosure::qualify(face.clone(), 4).unwrap();
    assert_eq!(closure.loops()[0].coedge_offsets, vec![[0, 0]; 4]);
    assert_eq!(closure.loops()[0].winding, [1, 0]);
    closure.reverify().unwrap();
    assert!(QualifiedRootUvClosure::qualify(face.clone(), 4).is_err());
    assert!(QualifiedRootLoopWinding::qualify_lifted(
        face, 0, 0, [[0.25; 2]; 2], Default::default(),
    ).is_err());
}

#[test]
fn both_face_projections_have_exact_uv_closed_incidence() {
    let wire=fixture::wire();
    let source=Arc::new(RootBrep::paired_contours(&wire,&[],1e-7).unwrap());
    let proof=QualifiedRootUvClosure::qualify(source.clone(),16).unwrap();
    assert_eq!(proof.joints(),16);assert!(Arc::ptr_eq(proof.source(),&source));proof.reverify().unwrap();
    use brep_core::root_model::QualifiedRootPeriodicUvClosure;
    let lifted = QualifiedRootPeriodicUvClosure::qualify(source.clone(), 16).unwrap();
    assert_eq!(lifted.loops().len(), 2);
    for (face, lift) in lifted.loops().iter().enumerate() {
        assert_eq!(lift.face, face);
        assert_eq!(lift.coedge_offsets, vec![[0, 0]; 8]);
        assert_eq!(lift.winding, [0, 0]);
    }
    lifted.reverify().unwrap();
    use brep_core::root_model::QualifiedRootLoopWinding;
    let winding = QualifiedRootLoopWinding::qualify_lifted(
        source.clone(), 0, 0, [[0.5; 2]; 2], Default::default(),
    ).unwrap();
    assert_eq!(winding.winding().abs(), 1);
    winding.reverify().unwrap();
    assert!(QualifiedRootPeriodicUvClosure::qualify(source.clone(), 15).is_err());
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
    let s_bc=ss::FiberSources::new(&third,ab.source().second()).unwrap();
    let ideal_bc=ss::qualify_original_ss_ideal(&s_bc,[1,1,2,2],Default::default()).unwrap();
    let gb_bc=ss::qualify_ss_groebner_basis(&ideal_bc,Default::default()).unwrap();
    let sys_bc=ss::qualify_ss_deflated_system(&gb_bc,Default::default()).unwrap();
    let f_bc=ss::qualify_deflated_ss_fiber(&sys_bc,[[0.38,0.62],[0.35,0.65],[0.19,0.22],[0.39,0.61]],ss::FiberAxis::SecondV,Default::default()).unwrap();
    let cb=Arc::new(ss::OriginalSsRootCurve::from_deflated(f_bc.into_fiber().unwrap()));
    let plain=|i:usize| ss::bind_original_ss_port(witnesses[i].point().clone(),ab.clone(),[0;4],Default::default()).unwrap();
    let cross=|i:usize| ss::bind_original_ss_third_port(witnesses[i].clone(),ac.clone(),[0,2],Default::default()).unwrap();
    let cross_b=|i:usize| ss::bind_original_ss_third_port(witnesses[i].clone(),cb.clone(),[2,1],Default::default()).unwrap();
    // Split a complete existing face, sharing one inserted chord between children.
    let mut boundary=wire.edges()[..2].to_vec();
    boundary.push(ss::OriginalSsRootEdge::new(wire.edges()[2].start().clone(),plain(1)).unwrap());
    boundary.push(ss::OriginalSsRootEdge::new(plain(1),plain(0)).unwrap());
    boundary.push(ss::OriginalSsRootEdge::new(plain(0),wire.edges()[2].end().clone()).unwrap());
    boundary.extend_from_slice(&wire.edges()[3..]);
    let full=ss::OriginalSsRootWire::new(boundary).unwrap();
    let full_wire=full.clone();
    let full_b=Arc::new(RootBrep::face_contours(ab.source().second().clone(),RootFaceContour {supports:vec![1;10],wire:full.clone()},vec![],1e-7).unwrap());
    let full=Arc::new(RootBrep::face_contours(ab.source().first().clone(),RootFaceContour {supports:vec![0;10],wire:full},vec![],1e-7).unwrap());
    let chord=ss::OriginalSsRootEdge::new(cross(0),cross(1)).unwrap();
    let chord_b=ss::OriginalSsRootEdge::new(cross_b(0),cross_b(1)).unwrap();
    let paired=brep_core::root_model::QualifiedRootPairedChordSplit::qualify([full.clone(),full_b.clone()],[chord.clone(),chord_b.clone()],[0,1],Default::default()).unwrap();
    assert_eq!(paired.paired_children(false),[0,0]);
    assert_eq!(paired.paired_children(true),[1,1]);
    for side in [false,true] {
        let [ra,rb]=paired.retain_pair(side).unwrap();
        assert_eq!(ra.output().topology().faces.len(),1);
        assert_eq!(rb.output().topology().faces.len(),1);
    }
    paired.reverify().unwrap();
    assert!(brep_core::root_model::QualifiedRootPairedChordSplit::qualify([full.clone(),full_b],[chord.clone(),ss::OriginalSsRootEdge::new(cross_b(1),cross_b(0)).unwrap()],[0,1],Default::default()).is_err());
    let split=brep_core::root_model::RootFaceChordSplit::split(full.clone(),0,chord.clone(),0,vec![]).unwrap();
    assert_eq!(split.face_children(),[0,1]);assert_eq!(split.inserted_edge(),10);
    let boundary=brep_core::root_model::QualifiedRootChordBoundary::qualify(split.clone(),Default::default()).unwrap();
    assert_eq!(boundary.censuses().len(),10);assert!(boundary.entries()>0 && boundary.identity_queries()>0);
    boundary.reverify().unwrap();
    assert!(brep_core::root_model::QualifiedRootChordBoundary::qualify(split.clone(),brep_core::root_model::RootChordBoundaryLimits {max_censuses:1,..Default::default()}).is_err());
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
    use brep_core::root_model::{QualifiedRootHoleAssignment as H,RootFaceChordSplit as S};
    let holes=vec![[0.5,0.5],[0.1953125,0.5]].into_iter().zip([0.1,0.0078125]).map(|(center,scale)| {
        RootFaceContour {wire:fixture::wire_circle_on(ab.source().first().clone(),center,scale),supports:vec![0;8]}
    }).collect();
    let crossing_hole=RootFaceContour {wire:fixture::wire_circle_on(ab.source().first().clone(),[13./64.,0.5],0.1),supports:vec![0;8]};
    let crossing=Arc::new(RootBrep::face_contours(ab.source().first().clone(),RootFaceContour {wire:full_wire.clone(),supports:vec![0;10]},vec![crossing_hole],1e-7).unwrap());
    let crossing=S::split(crossing,0,chord.clone(),0,vec![false]).unwrap();
    assert!(brep_core::root_model::QualifiedRootChordBoundary::qualify(crossing,Default::default()).is_err());
    let with_holes=Arc::new(RootBrep::face_contours(ab.source().first().clone(),RootFaceContour {wire:full_wire,supports:vec![0;10]},holes,1e-7).unwrap());
    let candidate_split=S::split(with_holes,0,chord.clone(),0,vec![false,false]).unwrap();
    let routed=H::qualify(candidate_split.clone(),Default::default()).unwrap();
    assert_eq!(routed.sides(),&[false,true]);assert!(routed.cells()>=16 && routed.queries()>=32);
    let routed_topology=routed.output().output().topology();
    assert_eq!(routed_topology.faces[0].holes,vec![1]);assert_eq!(routed_topology.faces[1].holes,vec![2]);
    routed.reverify().unwrap();
    use brep_core::root_model::{QualifiedRootFaceRegion as R,RootFaceRegion as Region};
    let region_source=routed.output().output().clone();
    let hole=R::qualify(region_source.clone(),0,[[0.5;2],[0.5;2]],Default::default()).unwrap();
    assert_eq!(hole.region(),&Region::ExcludedByHoles(vec![0]));hole.reverify().unwrap();
    assert_eq!(R::qualify(region_source.clone(),0,[[0.6;2],[0.5;2]],Default::default()).unwrap().region(),&Region::InsideSupport);
    assert_eq!(R::qualify(region_source.clone(),1,[[0.5;2],[0.5;2]],Default::default()).unwrap().region(),&Region::OutsideOuter);
    assert_eq!(R::qualify(region_source.clone(),1,[[0.1953125;2],[0.5;2]],Default::default()).unwrap().region(),&Region::ExcludedByHoles(vec![0]));
    let retained=brep_core::root_model::RootFaceRetention::retain(region_source.clone(),vec![1]).unwrap();
    assert_eq!(retained.face_map(),&[None,Some(0)]);
    assert_eq!(retained.output().topology().faces.len(),1);
    assert_eq!(retained.output().topology().edges.len(),10);
    assert!(retained.output().topology().bodies.is_empty());
    assert!(!retained.output().topology().shells[0].closed);
    QualifiedRootUvClosure::qualify(retained.output().clone(),10).unwrap().reverify().unwrap();
    retained.reverify().unwrap();
    let mixed=brep_core::mixed_root_model::MixedBrep::from_root(retained.output()).unwrap();
    let roundtrip=Arc::new(mixed.to_root().unwrap());
    QualifiedRootUvClosure::qualify(roundtrip.clone(),10).unwrap().reverify().unwrap();
    assert_eq!(R::qualify(roundtrip,0,[[0.1953125;2],[0.5;2]],Default::default()).unwrap().region(),&Region::ExcludedByHoles(vec![0]));
    assert!(brep_core::root_model::RootFaceRetention::retain(region_source.clone(),vec![1,1]).is_err());
    let empty=brep_core::root_model::RootFaceRetention::retain(region_source.clone(),vec![]).unwrap();
    assert!(empty.output().topology().vertices.is_empty() && empty.output().topology().faces.is_empty());
    assert!(R::qualify(region_source,0,[[0.6;2],[0.5;2]],ss::OriginalSsWindingLimits {max_segments:1,..Default::default()}).is_err());
    assert!(H::qualify(candidate_split,brep_core::root_model::RootHoleAssignmentLimits {max_queries:1,..Default::default()}).is_err());
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

#[test]
fn periodic_face_contour_reports_signed_winding_on_original_ss_arcs() {
    use brep_core::root_model::{
        QualifiedRootLoopWinding, QualifiedRootPeriodicUvClosure, RootFaceContour,
    };

    let original = fixture::wire();
    let mut plane = original.edges()[2].curve().source().first().clone();
    for point in plane.control_points.iter_mut().flatten() {
        point[2] = -1.;
    }
    let wire = fixture::wire_on(plane);
    let ab = wire.edges()[2].curve().clone();
    let mut third = sheet(0.25, 0.);
    third.periodic_v = true;
    third.knots_v = vec![-0.5, 0., 0.5, 1., 1.5];
    for row in &mut third.control_points {
        row[0][2] = -1. + row[0][1] - 0.6875;
        row[1][2] = row[0][2] + 0.375;
        row.push(row[0].clone());
    }
    third.weights = vec![vec![1.; 3]; 2];

    let candidates = candidate(&ab, &third);
    let witness = |v| {
        let point = Arc::new(ss::OriginalSsRootPoint::new(ab.clone(), v).unwrap());
        Arc::new(ss::QualifiedSsThirdSurfacePoint::qualify(
            candidates.clone(), point, Default::default(),
        ).unwrap())
    };
    let seam = witness(0.6875);
    let middle = witness(0.3125);
    let sources = ss::FiberSources::new(ab.source().first(), &third).unwrap();
    let graph = |span| {
        let ideal = ss::qualify_original_ss_ideal(&sources, [1, 1, 1, span], Default::default()).unwrap();
        let basis = ss::qualify_ss_groebner_basis(&ideal, Default::default()).unwrap();
        let system = ss::qualify_ss_deflated_system(&basis, Default::default()).unwrap();
        let fiber = ss::qualify_deflated_ss_fiber(
            &system, [[0.24, 0.27], [0.25, 0.75], [0.25, 0.75], [0., 1.]],
            ss::FiberAxis::SecondV, Default::default(),
        ).unwrap();
        let report = format!("{:?}", fiber.report());
        Arc::new(ss::OriginalSsRootCurve::from_deflated(
            fiber.into_fiber().unwrap_or_else(|| panic!("{report}")),
        ))
    };
    let first = graph(1);
    let second = graph(2);
    let port = |witness: &Arc<ss::QualifiedSsThirdSurfacePoint>, graph: &Arc<ss::OriginalSsRootCurve>, phase| {
        ss::bind_original_ss_third_port_with_phase(
            witness.clone(), graph.clone(), [0, 2], [0, 0, 0, phase], Default::default(),
        ).unwrap()
    };
    let a = ss::OriginalSsRootEdge::new(port(&seam, &first, 0), port(&middle, &first, 0)).unwrap();
    let b = ss::OriginalSsRootEdge::new(port(&middle, &second, 0), port(&seam, &second, 1)).unwrap();
    let contour = RootFaceContour {
        wire: ss::OriginalSsRootWire::new(vec![a, b]).unwrap(),
        supports: vec![1, 1],
    };
    assert!(RootBrep::face_contours(third.clone(), contour.clone(), vec![], 1e-7).is_err());
    let face = Arc::new(RootBrep::periodic_face_contours(third, contour, vec![], 1e-7).unwrap());
    let closure = QualifiedRootPeriodicUvClosure::qualify(face.clone(), 2).unwrap();
    assert_eq!(closure.loops()[0].coedge_offsets, [[0, 0]; 2]);
    assert_eq!(closure.loops()[0].winding, [0, 1]);
    closure.reverify().unwrap();
    assert!(QualifiedRootLoopWinding::qualify_lifted(
        face.clone(), 0, 0, [[0.5; 2]; 2], Default::default(),
    ).is_err());

    let mut topology = face.topology().clone();
    for coedge in &mut topology.loops[0].coedges {
        coedge.reversed = true;
        coedge.pcurve.reversed = true;
    }
    let reversed = Arc::new(RootBrep::from_topology(topology).unwrap());
    let closure = QualifiedRootPeriodicUvClosure::qualify(reversed, 2).unwrap();
    assert_eq!(closure.loops()[0].coedge_offsets, [[0, 0], [0, -1]]);
    assert_eq!(closure.loops()[0].winding, [0, -1]);
}

#[test]
fn lifted_winding_classifies_a_face_crossing_the_periodic_seam_twice() {
    use brep_core::root_model::{
        QualifiedRootLoopWinding as W, QualifiedRootPeriodicUvClosure, RootFaceContour,
    };

    // A square tube with its seam at x=z=0. The patch runs from native
    // v=7/8 through v=0 to v=1/8, between u=1/4 and u=3/4.
    let ring = [[0., 0.], [1., 1.], [2., 0.], [1., -1.], [0., 0.]];
    let mut tube = sheet(0., 0.);
    tube.periodic_v = true;
    tube.knots_v = vec![-0.25, 0., 0.25, 0.5, 0.75, 1., 1.25];
    tube.control_points = [0., 1.].map(|y| ring.map(|[x, z]| vec![x, y, z]).to_vec()).to_vec();
    tube.weights = vec![vec![1.; 5]; 2];
    let plane = |axis: usize, value| {
        let mut surface = sheet(0., 0.);
        for (i, row) in surface.control_points.iter_mut().enumerate() {
            for (j, p) in row.iter_mut().enumerate() {
                p[axis] = value;
                p[1 - axis] = -1. + 4. * i as f64;
                p[2] = -2. + 4. * j as f64;
                if axis == 1 {
                    p[1] += 0.375 * p[2];
                }
            }
        }
        surface
    };
    let make_contour = |bounds: [f64; 2], width: f64, rotation: usize, cut: bool| {
        let sides = bounds.map(|u| plane(1, u));
        let cap = plane(0, width);
        let seam = plane(0, 0.);
        let graph = |other: &Surface, span, axis, domain| {
            let sources = ss::FiberSources::new(&tube, other).unwrap();
            let ideal = ss::qualify_original_ss_ideal(&sources, [1, span, 1, 1], Default::default()).unwrap();
            let basis = ss::qualify_ss_groebner_basis(&ideal, Default::default()).unwrap();
            let system = ss::qualify_ss_deflated_system(&basis, Default::default()).unwrap();
            let fiber = ss::qualify_deflated_ss_fiber(&system, domain, axis, Default::default()).unwrap();
            let report = format!("{:?}", fiber.report());
            Arc::new(ss::OriginalSsRootCurve::from_deflated(
                fiber.into_fiber().unwrap_or_else(|| panic!("{report}")),
            ))
        };
        let horizontal = sides.each_ref().map(|side| [1, 4].map(|span| {
            graph(side, span, ss::FiberAxis::FirstV, [[0.02, 0.98], if span == 1 { [0., 0.5] } else { [0.5, 1.] }, [0.2, 0.6], [0.2, 0.8]])
        }));
        let vertical = [1, 4].map(|span| {
            graph(&cap, span, ss::FiberAxis::FirstU, [[0.02, 0.98], [0.1, 0.9], [0.2, 0.6], [0.2, 0.8]])
        });
        let witness = |curve: &Arc<ss::OriginalSsRootCurve>, parameter, other: &Surface| {
            Arc::new(ss::QualifiedSsThirdSurfacePoint::qualify(
                candidate(curve, other),
                Arc::new(ss::OriginalSsRootPoint::new(curve.clone(), parameter).unwrap()),
                Default::default(),
            ).unwrap())
        };
        let port = |witness: &Arc<ss::QualifiedSsThirdSurfacePoint>,
                    graph: &Arc<ss::OriginalSsRootCurve>, role, phase| {
            ss::bind_original_ss_third_port_with_phase(
                witness.clone(), graph.clone(), [0, role], [0, phase, 0, 0], Default::default(),
            ).unwrap()
        };

        let seams = horizontal.each_ref().map(|graphs| witness(&graphs[0], 0., &seam));
        let corners = horizontal.each_ref().map(|graphs| [witness(&graphs[0], width, &cap), witness(&graphs[1], 1. - width, &cap)]);
        let edge = |start, end| ss::OriginalSsRootEdge::new(start, end).unwrap();
        let [low, high] = &horizontal;
        let [bottom, top] = &corners;
        let mut edges = vec![
            edge(port(&seams[0], &low[0], 1, 0), port(&bottom[0], &low[0], 1, 0)),
            edge(port(&bottom[0], &vertical[0], 2, 0), port(&top[0], &vertical[0], 2, 0)),
            edge(port(&top[0], &high[0], 1, 0), port(&seams[1], &high[0], 1, 0)),
            edge(port(&seams[1], &high[1], 1, 1), port(&top[1], &high[1], 1, 0)),
            edge(port(&top[1], &vertical[1], 2, 0), port(&bottom[1], &vertical[1], 2, 0)),
            edge(port(&bottom[1], &low[1], 1, 0), port(&seams[0], &low[1], 1, 1)),
        ];
        let chord = if cut {
            let cut_plane = plane(0, width * 0.5);
            let graph = graph(&cut_plane, 1, ss::FiberAxis::FirstU,
                [[0.02, 0.98], [0.1, 0.9], [0.2, 0.6], [0.2, 0.8]]);
            let lower = witness(&low[0], width * 0.5, &cut_plane);
            let upper = witness(&high[0], width * 0.5, &cut_plane);
            let a = port(&lower, &low[0], 1, 0);
            let b = port(&upper, &high[0], 1, 0);
            edges.splice(2..3, [
                edge(edges[2].start().clone(), b.clone()),
                edge(b, edges[2].end().clone()),
            ]);
            edges.splice(0..1, [
                edge(edges[0].start().clone(), a.clone()),
                edge(a, edges[0].end().clone()),
            ]);
            Some(edge(port(&lower, &graph, 2, 0), port(&upper, &graph, 2, 0)))
        } else {
            None
        };
        edges.rotate_left(rotation);
        let supports = vec![0; edges.len()];
        (RootFaceContour { wire: ss::OriginalSsRootWire::new(edges).unwrap(), supports }, chord)
    };
    let (cut_contour, chord) = make_contour([0.25, 0.75], 0.5, 0, true);
    let cut_hole = make_contour([0.375, 0.625], 0.1875, 3, false).0;
    let cut_source = Arc::new(RootBrep::periodic_face_contours(
        tube.clone(), cut_contour, vec![cut_hole], 1e-7,
    ).unwrap());
    let qualified = brep_core::root_model::QualifiedRootPeriodicChordSplit::qualify(
        cut_source, 0, chord.unwrap(), 0, Default::default(), Default::default(),
    ).unwrap();
    assert!(qualified.boundary().entries() > 0);
    assert_eq!(qualified.hole_assignment().sides(), &[true]);
    qualified.reverify().unwrap();
    let split = qualified.split();
    assert!(split.output().topology().faces[0].holes.is_empty());
    assert_eq!(split.output().topology().faces[1].holes.len(), 1);
    let cut_closure = QualifiedRootPeriodicUvClosure::qualify(split.output().clone(), 16).unwrap();
    assert_eq!(cut_closure.loops().len(), 3);
    assert!(cut_closure.loops().iter().all(|lift| lift.winding == [0, 0]));
    assert!(cut_closure.loops().iter().any(|lift| lift.coedge_offsets.iter().any(|&offset| offset != [0, 0])));
    for (child, u, v) in [(0, 0.5, 0.09375), (1, 0.5, 0.90625)] {
        let proof = brep_core::root_model::QualifiedRootPeriodicFacePoint::qualify(
            split.output().clone(), child, [u, v], Default::default(),
        ).unwrap();
        assert_eq!(proof.region(), &brep_core::root_model::RootFaceRegion::InsideSupport);
    }
    let selection = qualified.select_child(
        [[0.49, 0.51], [0.90, 0.91]], Default::default(),
    ).unwrap();
    let solid = brep_core::closed_solid::qualify(
        brep_core::cuboid([0.25, 0.25, -0.5], [0.5, 0.75, -0.25]).unwrap(),
        solid_policy::policy(),
    ).unwrap().into_solid().unwrap();
    let directions = [[1., 0.317, 0.173], [0.239, 1., 0.419], [0.137, 0.283, 1.]];
    use brep_core::root_model::QualifiedRootPeriodicChildMaterial;
    let whole = QualifiedRootPeriodicChildMaterial::qualify(
        qualified.clone(), 0, &solid, &directions, Default::default(), Default::default(),
    ).unwrap();
    assert!(!whole.inside());
    use brep_core::closed_solid::{BooleanFaceDisposition as D, BooleanMaterialOp as Op, BooleanSourceSide as Side};
    assert_eq!(whole.boolean_disposition(Op::Union, Side::First), D::Keep);
    assert_eq!(whole.boolean_disposition(Op::Intersection, Side::First), D::Discard);
    let contribution = whole.boolean_contribution(Op::Union, Side::First).unwrap();
    assert_eq!(contribution.disposition(), D::Keep);
    assert_eq!(contribution.lineage().unwrap().face_map(), &[Some(0), None]);
    assert!(!contribution.patch().unwrap().topology().shells[0].faces[0].reversed);
    let discarded = whole.boolean_contribution(Op::Intersection, Side::First).unwrap();
    assert!(discarded.patch().is_none());
    use brep_core::root_model::{QualifiedRootShellPositiveFlux, RootShellFluxLimits};
    let flux = RootShellFluxLimits {
        axis: 2,
        origin: 0.,
        max_cells: 32,
        max_spans: 32,
        max_depth: 2,
        winding: Default::default(),
    };
    let empty = QualifiedRootShellPositiveFlux::from_boolean_contributions(
        &[discarded.clone()], 1e-7, 32, vec![], 128, flux,
    ).unwrap_err();
    assert!(empty.to_string().contains("no retained shell faces"));
    assert!(QualifiedRootShellPositiveFlux::assemble_boolean_selection(
        std::slice::from_ref(&discarded), 1e-7, 32, vec![], 128, flux,
    ).unwrap().is_none());
    assert!(QualifiedRootShellPositiveFlux::assemble_boolean_selection(
        &[], 1e-7, 32, vec![], 128, flux,
    ).is_err());
    // A retained patch from a real Boolean classification still has open
    // edges. Discarding its sibling must not turn it into a closed shell.
    assert!(QualifiedRootShellPositiveFlux::assemble_boolean_selection(
        &[discarded, contribution], 1e-7, 32, vec![], 128, flux,
    ).is_err());

    let enclosing = brep_core::closed_solid::qualify(
        brep_core::cuboid([-1.; 3], [3.; 3]).unwrap(), solid_policy::policy(),
    ).unwrap().into_solid().unwrap();
    let enclosed = QualifiedRootPeriodicChildMaterial::qualify(
        qualified.clone(), 0, &enclosing, &directions, Default::default(), Default::default(),
    ).unwrap();
    let reversed = enclosed.boolean_contribution(Op::Difference, Side::Second).unwrap();
    assert_eq!(reversed.disposition(), D::Reverse);
    assert!(reversed.patch().unwrap().topology().shells[0].faces[0].reversed);
    assert_eq!(Op::Difference.face_disposition(Side::Second, true), D::Reverse);
    assert_eq!(Op::Xor.face_disposition(Side::First, true), D::Reverse);
    let both = qualified.boolean_contributions(
        &enclosing, &directions, Default::default(), Default::default(),
        Op::Difference, Side::Second,
    ).unwrap();
    for (child, contribution) in both.iter().enumerate() {
        assert_eq!(contribution.disposition(), D::Reverse);
        let lineage = contribution.lineage().unwrap();
        assert_eq!(lineage.face_map()[child], Some(0));
        let topology = contribution.patch().unwrap().topology();
        assert!(topology.shells[0].faces[0].reversed);
        assert_eq!(topology.faces[0].holes.len(), child);
    }

    assert!(!whole.boxes().is_empty());
    assert!(QualifiedRootPeriodicChildMaterial::qualify(
        qualified.clone(), 1, &solid, &directions, Default::default(), Default::default(),
    ).is_err());
    assert!(qualified.boolean_contributions(
        &solid, &directions, Default::default(), Default::default(),
        Op::Union, Side::First,
    ).is_err());
    let corner_body = brep_core::closed_solid::qualify(
        brep_core::cuboid([0.46, 0.35, 0.46], [0.51, 0.37, 0.51]).unwrap(),
        solid_policy::policy(),
    ).unwrap().into_solid().unwrap();
    let refined = QualifiedRootPeriodicChildMaterial::qualify(
        qualified.clone(), 0, &corner_body, &directions, Default::default(), Default::default(),
    ).unwrap();
    assert!(!refined.inside());
    assert!(refined.excluded_cells() > 0);
    let seed = selection.qualify_material_seed(&solid, &directions, Default::default()).unwrap();
    assert_eq!(seed.child(), 1);
    assert!(seed.material().inside());
    let opposite = qualified.select_child(
        [[0.49, 0.51], [0.09, 0.10]], Default::default(),
    ).unwrap();
    assert!(!opposite.qualify_material_seed(&solid, &directions, Default::default())
        .unwrap().material().inside());
    assert_eq!(selection.child(), 1);
    let retained = selection.retention();
    assert_eq!(retained.face_map(), &[None, Some(0)]);
    assert_eq!(retained.output().topology().faces[0].holes.len(), 1);
    selection.reverify().unwrap();
    assert_eq!(selection.trim().unwrap().face_map(), retained.face_map());
    assert!(qualified.retain_child_containing(
        [[0.49, 0.51], [0.06, 0.07]], Default::default(),
    ).is_err());
    let (contour, _) = make_contour([0.25, 0.75], 0.5, 0, false);
    let hole = make_contour([0.375, 0.625], 0.25, 3, false).0;
    let nested = make_contour([0.4375, 0.5625], 0.1875, 0, false).0;
    let sample_edge = nested.wire.edges()[0].clone();
    let invalid = Arc::new(RootBrep::periodic_face_contours(
        tube.clone(), contour.clone(), vec![hole.clone(), nested], 1e-7,
    ).unwrap());
    let error = brep_core::root_model::QualifiedRootFaceRegion::qualify_lifted_auto(
        invalid, 0, [[0.5; 2], [0.; 2]], Default::default(),
    ).unwrap_err();
    assert!(error.to_string().contains("inside another hole"), "{error}");
    // The wider hole begins inside the first hole's axial interval and
    // ends beyond it. Its lower boundary intersects both periodic shores
    // of the first hole, whose lift starts from the opposite direction.
    let crossing = make_contour([0.5, 0.6875], 0.375, 0, false).0;
    let crossing_face = Arc::new(RootBrep::periodic_face_contours(
        tube.clone(), contour.clone(), vec![hole.clone(), crossing], 1e-7,
    ).unwrap());
    let error = brep_core::root_model::QualifiedRootPeriodicFace::qualify(
        crossing_face, 0, Default::default(),
    ).unwrap_err();
    assert!(error.to_string().contains("Lifted hole"), "{error}");
    let face = Arc::new(RootBrep::periodic_face_contours(tube, contour, vec![hole], 1e-7).unwrap());
    face.audit_face_trim_arc_separation(0, Default::default()).unwrap();
    assert!(face.audit_face_trim_arc_separation(
        0, ss::OriginalSsCurveLimits { interval_operations: 0, ..Default::default() },
    ).is_err());
    let closure = QualifiedRootPeriodicUvClosure::qualify(face.clone(), 12).unwrap();
    assert_eq!(closure.loops()[0].winding, [0, 0]);
    assert_eq!(closure.loops()[0].coedge_offsets, [[0, 0], [0, 0], [0, 0], [0, -1], [0, -1], [0, -1]]);
    let inside = W::qualify_lifted(face.clone(), 0, 0, [[0.49, 0.51], [-0.02, 0.02]], Default::default()).unwrap();
    assert_eq!(inside.winding(), -1);
    inside.reverify().unwrap();
    assert_eq!(W::qualify_lifted(face.clone(), 0, 0, [[0.1; 2], [0.; 2]], Default::default()).unwrap().winding(), 0);
    use brep_core::root_model::{QualifiedRootFaceRegion, RootFaceRegion};
    let classify = |u, offsets| QualifiedRootFaceRegion::qualify_lifted(
        face.clone(), 0, [[u; 2], [0.; 2]], offsets, Default::default(),
    );
    let excluded = classify(0.5, vec![[0, -1]]).unwrap();
    assert_eq!(excluded.region(), &RootFaceRegion::ExcludedByHoles(vec![0]));
    excluded.reverify().unwrap();
    assert!(W::qualify_lifted(
        face.clone(), 0, 0, [[0.28125, 0.71875], [-0.0625, 0.0625]],
        ss::OriginalSsWindingLimits { max_segments: 64, ..Default::default() },
    ).is_err());
    let automatic = QualifiedRootFaceRegion::qualify_lifted_auto(
        face.clone(), 0, [[0.5; 2], [0.; 2]], Default::default(),
    ).unwrap();
    assert_eq!(automatic.hole_offsets(), Some([[0, -1]].as_slice()));
    assert_eq!(automatic.region(), excluded.region());
    automatic.reverify().unwrap();
    use brep_core::root_model::QualifiedRootPeriodicFacePoint as PeriodicPoint;
    let native = PeriodicPoint::qualify(face.clone(), 0, [0.5, 0.96875], Default::default()).unwrap();
    assert_eq!(native.image_offset(), Some([0, -1]));
    assert_eq!(native.region(), &RootFaceRegion::ExcludedByHoles(vec![0]));
    native.reverify().unwrap();
    let domain = Arc::new(brep_core::root_model::QualifiedRootPeriodicFace::qualify(
        face.clone(), 0, Default::default(),
    ).unwrap());
    use brep_core::root_model::QualifiedRootPeriodicFaceEdge;
    let classified = QualifiedRootPeriodicFaceEdge::qualify(domain.clone(), sample_edge.clone(), 0, 32).unwrap();
    assert_eq!(classified.region(), &RootFaceRegion::ExcludedByHoles(vec![0]));
    assert!(QualifiedRootPeriodicFaceEdge::qualify(domain.clone(), sample_edge, 1, 32).is_err());
    let across_seam = domain.classify_box([[0.49, 0.51], [0.99, 1.01]]).unwrap();
    assert_eq!(across_seam.region(), &RootFaceRegion::ExcludedByHoles(vec![0]));
    assert_eq!(across_seam.image_offset(), Some([0, -1]));
    assert!(domain.classify_box([[0.3, 0.5], [0.99, 1.01]]).is_err());
    assert!(domain.classify_box([[0.5; 2], [1., 0.]]).is_err());
    let support = domain.classify_point([0.28125, 0.96875]).unwrap();
    assert_eq!(support.region(), &RootFaceRegion::InsideSupport);
    let outside = domain.classify_point([0.5, 0.5]).unwrap();
    assert_eq!(outside.region(), &RootFaceRegion::OutsideOuter);
    assert_eq!(outside.image_offset(), None);

    assert_eq!(classify(0.3125, vec![[0, -1]]).unwrap().region(), &RootFaceRegion::InsideSupport);
    assert_eq!(classify(0.1, vec![[0, -1]]).unwrap().region(), &RootFaceRegion::OutsideOuter);
    assert!(classify(0.5, vec![[1, -1]]).is_err());
    assert!(classify(0.5, vec![]).is_err());
    assert!(W::qualify(face, 0, 0, [[0.5; 2], [0.; 2]], Default::default()).is_err());
}

#[test]
fn periodic_band_trims_away_a_proved_collapsed_shore() {
    use brep_core::root_model::{QualifiedRootShellPositiveFlux, RootPeriodicBandLimits, RootShellFluxLimits};
    use nurbs_core::ss_intersection as ss;

    let section = |height, rational_cap| {
        let seed = periodic_wire::section_at_height(height);
        let mut side = seed.source().first().clone();
        for row in &mut side.control_points {
            row[1][0] = 0.;
            row[1][1] = 0.;
        }
        let mut cap = seed.source().second().clone();
        if rational_cap {
            let first = cap.control_points[0].clone();
            let last = cap.control_points[1].clone();
            let middle = first.iter().zip(&last).map(|(a, b)| {
                a.iter().zip(b).map(|(a, b)| 0.75 * a + 0.25 * b).collect()
            }).collect();
            cap.degree_u = 2;
            let [lo, hi] = [cap.knots_u[1], cap.knots_u[2]];
            cap.knots_u = vec![lo, lo, lo, hi, hi, hi];
            cap.control_points = vec![first, middle, last];
            cap.weights = vec![vec![1.; 2], vec![2.; 2], vec![3.; 2]];
        }
        let pair = ss::FiberSources::new(&side, &cap).unwrap();
        ss::qualify_periodic_plane_ss(&pair, Default::default()).unwrap()
    };
    let lower = section(1., false);
    let upper = section(1.5, true);
    assert_eq!(upper.support_proof().collapsed_end, Some(1));

    let limits = RootPeriodicBandLimits {
        max_chart_attempts: 64,
        max_orientation_probes: 128,
        max_chart_spans: 8192,
        max_contact_cells: 4096,
        flux: RootShellFluxLimits {
            axis: 2, origin: 1., max_cells: 2048, max_spans: 8192, max_depth: 16,
            winding: Default::default(),
        },
    };
    let body = QualifiedRootShellPositiveFlux::qualify_periodic_band(
        [&lower, &upper], 1e-7, limits,
    ).unwrap();
    body.reverify().unwrap();
    assert!(body.report().cells < 64);
    let [lo, hi] = body.report().signed_bounds;
    let expected = 7. * std::f64::consts::PI / 96.;
    assert!(lo > 0. && lo <= expected && expected <= hi && hi - lo < 0.001, "[{lo}, {hi}]");
    let model = body.to_body().unwrap();
    assert_eq!(model.topology().bodies.len(), 1);
    assert_eq!(model.topology().faces.len(), 3);
    assert!(model.topology().boundary_edges().is_empty());
    assert!(model.topology().faces.iter().any(|face| &face.surface == upper.source().second()));
}
