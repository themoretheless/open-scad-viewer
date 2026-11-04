use brep_core::{ss_trimmed_face as trim,root_model::RootBrep};
use nurbs_core::{ss_intersection as ss,curve::Curve};
use std::sync::Arc;
#[path="../../nurbs-core/tests/support/ss_root_circle.rs"] mod circle;
fn context(x:f64,hole:bool)->(ss::OriginalSsRootWire,Arc<trim::QualifiedOrderedSsFaceEvents>) {
    let polygon=|p:&[[f64;2]]| (0..p.len()).map(|i|Curve::from_polyline(vec![vec![p[i][0],p[i][1],x],vec![p[(i+1)%p.len()][0],p[(i+1)%p.len()][1],x]]).unwrap()).collect::<Vec<_>>();
    let mut loops=vec![polygon(&[[0.,-1.],[1.,-1.],[1.,1.],[0.,1.]])];
    if hole {loops.push(polygon(&[[0.375,-0.125],[0.375,0.125],[0.4375,0.125],[0.4375,-0.125]]));}
    let mut model=brep_core::planar_cap::build_sheet(&loops,x,1e-7,100000).unwrap();
    let rotate=|p:&mut [f64]| {let a=[p[0],p[1],p[2]];p[0]=a[2];p[1]=a[0];p[2]=a[1];};
    for v in &mut model.vertices {rotate(&mut v.point);}for e in &mut model.edges {for p in &mut e.curve.control_points {rotate(p);}}
    for f in &mut model.faces {for row in &mut f.surface.control_points {for p in row {rotate(p);}}}model.rebuild_topology_ids();
    let wire=circle::wire();let curve=wire.edges()[2].curve().clone();let s=&model.faces[0].surface;
    let source=Arc::new(curve.third_surface_candidates(s,[s.degree_u,s.degree_v],Default::default()).unwrap());
    let span=Arc::new(ss::QualifiedSsThirdSurfaceBranchCensus::qualify(source,curve,Default::default()).unwrap());
    let face=Arc::new(trim::QualifiedSsTrimmedFaceCensus::qualify(Arc::new(model),0,vec![span],Default::default()).unwrap());
    (wire,Arc::new(trim::QualifiedOrderedSsFaceEvents::qualify(face,Default::default()).unwrap()))
}
#[test]
fn irrational_cuts_partition_original_arc_and_sew_into_the_existing_paired_brep() {
    let (wire,events)=context(13./64.,false);let source=Arc::new(wire.edges()[2].clone());assert!(!source.forward());
    let proof=trim::QualifiedSsEventEdgePartition::qualify(events.clone(),source.clone(),trim::SsEventCutSelection::RetainedFace,Default::default()).unwrap();
    let pieces=proof.partition().pieces();assert_eq!(pieces.len(),3);
    assert_eq!(proof.event_uses(),&[trim::SsEventCutUse::Vertex(2),trim::SsEventCutUse::Vertex(1)]);
    assert!(pieces[0].start().point().same_selector(source.start().point()));
    assert!(pieces[2].end().point().same_selector(source.end().point()));
    for piece in pieces {assert!(Arc::ptr_eq(piece.curve(),source.curve()));assert!(!piece.forward());}
    for adjacent in pieces.windows(2) {assert!(adjacent[0].end().point().same_selector(adjacent[1].start().point()));}
    let mut edges=wire.edges()[..2].to_vec();edges.extend_from_slice(pieces);edges.extend_from_slice(&wire.edges()[3..]);
    let split=ss::OriginalSsRootWire::new(edges).unwrap();split.reverify().unwrap();
    let model=RootBrep::paired_contours(&split,&[],1e-7).unwrap();model.reverify().unwrap();
    assert_eq!(model.topology().edges.len(),10);assert_eq!(model.topology().vertices.len(),10);
    assert!(model.topology().boundary_edges().is_empty());assert_eq!(model.topology().euler_characteristic(),2);
    proof.reverify().unwrap();
}
#[test]
fn reversed_and_restricted_edges_keep_traversal_and_explicit_outside_decisions() {
    let (wire,events)=context(13./64.,false);let original=&wire.edges()[2];
    let reverse=Arc::new(ss::OriginalSsRootEdge::new(original.end().clone(),original.start().clone()).unwrap());
    let proof=trim::QualifiedSsEventEdgePartition::qualify(events.clone(),reverse,trim::SsEventCutSelection::AllCandidates,Default::default()).unwrap();
    assert!(proof.partition().pieces().iter().all(|e|e.forward()));
    assert_eq!(proof.event_uses(),&[trim::SsEventCutUse::Vertex(1),trim::SsEventCutUse::Vertex(2)]);proof.reverify().unwrap();
    let end=events.bind_port(1,original.curve().clone(),[0;4],Default::default()).unwrap();
    let restricted=Arc::new(ss::OriginalSsRootEdge::new(original.start().clone(),end).unwrap());
    let proof=trim::QualifiedSsEventEdgePartition::qualify(events,restricted,trim::SsEventCutSelection::AllCandidates,Default::default()).unwrap();
    assert_eq!(proof.partition().pieces().len(),1);
    assert_eq!(proof.event_uses(),&[trim::SsEventCutUse::OutsideEdge,trim::SsEventCutUse::Vertex(1)]);proof.reverify().unwrap();
}
#[test]
fn existing_endpoint_aliases_and_hole_exclusions_never_create_zero_length_pieces() {
    let (wire,events)=context(0.25,false);
    let proof=trim::QualifiedSsEventEdgePartition::qualify(events,Arc::new(wire.edges()[2].clone()),trim::SsEventCutSelection::RetainedFace,Default::default()).unwrap();
    assert_eq!(proof.partition().pieces().len(),1);
    assert_eq!(proof.event_uses(),&[trim::SsEventCutUse::Vertex(1),trim::SsEventCutUse::Vertex(0)]);proof.reverify().unwrap();
    let (wire,events)=context(13./64.,true);
    let proof=trim::QualifiedSsEventEdgePartition::qualify(events.clone(),Arc::new(wire.edges()[2].clone()),trim::SsEventCutSelection::RetainedFace,Default::default()).unwrap();
    assert_eq!(proof.partition().pieces().len(),2);
    assert_eq!(proof.event_uses(),&[trim::SsEventCutUse::ExcludedFace,trim::SsEventCutUse::Vertex(1)]);proof.reverify().unwrap();
    let all=trim::QualifiedSsEventEdgePartition::qualify(events,Arc::new(wire.edges()[2].clone()),trim::SsEventCutSelection::AllCandidates,Default::default()).unwrap();
    assert_eq!(all.partition().pieces().len(),3);
}
#[test]
fn duplicate_cut_lineage_is_exact_and_invalid_order_or_budget_never_partitions() {
    let (wire,events)=context(13./64.,false);let source=wire.edges()[2].clone();
    let upper=events.bind_port(1,source.curve().clone(),[0;4],Default::default()).unwrap();
    let lower=events.bind_port(0,source.curve().clone(),[0;4],Default::default()).unwrap();
    let native=ss::QualifiedOriginalSsEdgePartition::qualify(source.clone(),vec![upper.clone(),upper.clone(),lower.clone()],Default::default()).unwrap();
    assert_eq!(native.pieces().len(),3);assert_eq!(native.cut_vertices(),&[1,1,2]);native.reverify().unwrap();
    assert!(ss::QualifiedOriginalSsEdgePartition::qualify(source.clone(),vec![lower,upper],Default::default()).is_err());
    for limits in [trim::SsEventEdgePartitionLimits {max_event_queries:1,..Default::default()},
        trim::SsEventEdgePartitionLimits {max_reserved_work:1,..Default::default()},
        trim::SsEventEdgePartitionLimits {edge:ss::SsRootEdgePartitionLimits {max_cuts:1,..Default::default()},..Default::default()}] {
        assert!(trim::QualifiedSsEventEdgePartition::qualify(events.clone(),Arc::new(source.clone()),trim::SsEventCutSelection::AllCandidates,limits).is_err());
    }
}
