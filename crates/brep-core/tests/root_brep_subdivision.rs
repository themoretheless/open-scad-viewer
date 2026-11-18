use brep_core::root_model::{RootBrep,RootEdgePartitionRequest as Request,RootBrepPartitionLimits as Limits,QualifiedRootBrepEdgeSubdivision as Subdivision};
use nurbs_core::ss_intersection as ss;
use std::sync::Arc;
#[path="../../nurbs-core/tests/support/ss_root_circle.rs"] mod circle;
fn source()->Arc<RootBrep> {Arc::new(RootBrep::paired_contours(&circle::wire(),&[],1e-7).unwrap())}
fn partition(source:&RootBrep,edge:usize)->Arc<ss::QualifiedOriginalSsEdgePartition> {
    let root=&source.topology().edges[edge].curve;let curve=root.curve().clone();
    let mut cuts=Vec::new();
    for interval in [[0.40,0.41],[0.59,0.60]] {
        let clock=Arc::new(ss::QualifiedSsAlgebraicClock::from_binary(&[985.,-4096.,4096.],interval,Default::default()).unwrap());
        let point=Arc::new(ss::OriginalSsRootPoint::from_algebraic(ss::OriginalSsAlgebraicPoint::new(curve.clone(),clock).unwrap(),Default::default()).unwrap());
        cuts.push(ss::bind_original_ss_port(point,curve.clone(),[0;4],Default::default()).unwrap());
    }
    if !root.forward() {cuts.reverse();}
    Arc::new(ss::QualifiedOriginalSsEdgePartition::qualify(root.clone(),cuts,Default::default()).unwrap())
}
#[test]
fn one_partition_updates_every_coedge_and_preserves_the_existing_brep_snapshot() {
    let source=source();let native=partition(&source,2);
    let proof=Subdivision::qualify(source.clone(),vec![Request {edge:2,partition:native}],Default::default()).unwrap();
    let model=proof.model().topology();let old=source.topology();
    assert!(proof.source_matches(&source));assert!(!proof.source_matches(&Arc::new((*source).clone())));
    assert_eq!(model.vertices.len(),10);assert_eq!(model.edges.len(),10);assert_eq!(proof.new_vertices(),8..10);assert_eq!(proof.new_edges(),8..10);
    assert_eq!(proof.new_coedges(),4);assert_eq!(proof.edge_children(2),Some(&[2,8,9][..]));assert_eq!(proof.input_cut_vertices(0),Some(&[8,9][..]));
    assert_eq!(model.faces,old.faces);assert_eq!(model.shells,old.shells);assert_eq!(model.bodies,old.bodies);
    for i in 0..old.vertices.len() {assert!(model.vertices[i].point.selector.same_selector(&old.vertices[i].point.selector));}
    for face in 0..2 {
        assert_eq!(proof.coedge_children(face,2),Some(&[2,3,4][..]));assert_eq!(model.loops[face].coedges.len(),10);
        for c in &model.loops[face].coedges {assert_eq!(c.pcurve.world_edge,c.edge);assert_eq!(c.pcurve.support,face);}
    }
    assert_eq!(model.euler_characteristic(),2);assert!(model.boundary_edges().is_empty());proof.reverify().unwrap();
}
#[test]
fn reversed_original_edge_uses_reverse_child_order_in_both_unchanged_face_loops() {
    let mut topology=source().topology().clone();let original=&topology.edges[2].curve;
    let reversed=ss::OriginalSsRootEdge::new(original.end().clone(),original.start().clone()).unwrap();
    topology.edges[2].curve=reversed;topology.edges[2].vertices.swap(0,1);
    for wire in &mut topology.loops {for c in &mut wire.coedges {if c.edge==2 {c.reversed=true;c.pcurve.reversed=true;}}}
    let source=Arc::new(RootBrep::from_topology(topology).unwrap());let native=partition(&source,2);
    let proof=Subdivision::qualify(source.clone(),vec![Request {edge:2,partition:native}],Default::default()).unwrap();
    let model=proof.model().topology();
    for wire in &model.loops {
        assert_eq!(wire.coedges[2..5].iter().map(|c|c.edge).collect::<Vec<_>>(),vec![9,8,2]);
        assert!(wire.coedges[2..5].iter().all(|c|c.reversed && c.pcurve.reversed));
    }
    assert_eq!(model.shells,source.topology().shells);assert!(model.boundary_edges().is_empty());proof.reverify().unwrap();
}
#[test]
fn endpoint_aliases_are_noops_and_foreign_duplicate_or_exhausted_requests_fail() {
    let source=source();let edge=&source.topology().edges[2].curve;
    let alias=Arc::new(ss::QualifiedOriginalSsEdgePartition::qualify(edge.clone(),vec![edge.start().clone(),edge.end().clone()],Default::default()).unwrap());
    let proof=Subdivision::qualify(source.clone(),vec![Request {edge:2,partition:alias}],Default::default()).unwrap();
    assert!(proof.new_edges().is_empty() && proof.new_vertices().is_empty());assert_eq!(proof.new_coedges(),0);
    assert_eq!(proof.input_cut_vertices(0),Some(source.topology().edges[2].vertices.as_slice()));proof.reverify().unwrap();
    let native=partition(&source,2);let request=Request {edge:2,partition:native.clone()};
    assert!(Subdivision::qualify(source.clone(),vec![request.clone(),request.clone()],Default::default()).is_err());
    assert!(Subdivision::qualify(source.clone(),vec![Request {edge:8,partition:native}],Default::default()).is_err());
    let foreign=Arc::new(RootBrep::paired_contours(&circle::wire(),&[],1e-7).unwrap());
    assert!(Subdivision::qualify(source.clone(),vec![Request {edge:2,partition:partition(&foreign,2)}],Default::default()).is_err());
    for limits in [Limits {max_new_vertices:1,..Default::default()},Limits {max_new_edges:1,..Default::default()},Limits {max_new_coedges:1,..Default::default()}] {
        assert!(Subdivision::qualify(source.clone(),vec![request.clone()],limits).is_err());
    }
    let empty=Subdivision::qualify(source,vec![],Default::default()).unwrap();assert!(empty.new_edges().is_empty());empty.reverify().unwrap();
}
#[test]
fn multiple_requests_have_deterministic_children_and_request_local_cut_maps() {
    let source=source();let first=partition(&source,2);let edge=&source.topology().edges[1].curve;
    let point=Arc::new(ss::OriginalSsRootPoint::new(edge.curve().clone(),0.28125).unwrap());
    let cut=ss::bind_original_ss_port(point,edge.curve().clone(),[0;4],Default::default()).unwrap();
    let second=Arc::new(ss::QualifiedOriginalSsEdgePartition::qualify(edge.clone(),vec![cut],Default::default()).unwrap());
    let requests=vec![Request {edge:2,partition:first},Request {edge:1,partition:second}];
    let a=Subdivision::qualify(source.clone(),requests.clone(),Default::default()).unwrap();let mut reversed=requests.clone();reversed.reverse();
    let b=Subdivision::qualify(source.clone(),reversed,Default::default()).unwrap();
    for i in 0..source.topology().edges.len() {assert_eq!(a.edge_children(i),b.edge_children(i));}
    assert_eq!(a.input_cut_vertices(0),b.input_cut_vertices(1));assert_eq!(a.input_cut_vertices(1),b.input_cut_vertices(0));
    assert_eq!(a.model().topology().edges.len(),11);assert_eq!(a.model().topology().vertices.len(),11);
    assert!(Subdivision::qualify(source,requests,Limits {max_requests:1,..Default::default()}).is_err());a.reverify().unwrap();b.reverify().unwrap();
}
