use brep_core::root_model::RootBrep;
#[path = "../../nurbs-core/tests/support/ss_root_circle.rs"]
mod fixture;
#[test]
fn paired_brep_keeps_shared_original_world_edges_and_both_uv_projections() {
    let contour = fixture::wire();
    let model = RootBrep::paired_contours(&contour, &[], 1e-7).unwrap();
    model.reverify().unwrap();
    let topology = model.topology();
    assert_eq!(topology.vertices.len(), 8);
    assert_eq!(topology.edges.len(), 8);
    assert_eq!(topology.loops.len(), 2);
    assert_eq!(topology.faces.len(), 2);
    assert_eq!(topology.shells.len(), 1);
    assert!(topology.shells[0].closed);
    assert_eq!(topology.euler_characteristic(), 2);
    assert!(topology.boundary_edges().is_empty());
    for i in 0..8 {
        assert_eq!(
            topology.loops[0].coedges[i].edge,
            topology.loops[1].coedges[i].edge
        );
        assert_eq!(topology.loops[0].coedges[i].pcurve.support, 0);
        assert_eq!(topology.loops[1].coedges[i].pcurve.support, 1);
        let a = model.pcurve_bounds(0, 0, i, Default::default()).unwrap();
        let b = model.pcurve_bounds(1, 0, i, Default::default()).unwrap();
        assert!(a.iter().chain(&b).all(|d| d[0] <= d[1]));
        let world = topology.edges[i]
            .curve
            .enclose(Default::default())
            .unwrap()
            .world;
        assert!(world[2][0] <= 0. && 0. <= world[2][1]);
    }
    assert!(model.pcurve_bounds(2, 0, 0, Default::default()).is_err());
}
#[test]
fn forged_surface_projection_vertex_and_rounded_trim_are_rejected() {
    let model = RootBrep::paired_contours(&fixture::wire(), &[], 1e-7).unwrap();
    let mut swapped = model.topology().clone();
    swapped.loops[0].coedges[0].pcurve.support = 1;
    assert!(RootBrep::from_topology(swapped).is_err());
    let mut wrong = model.topology().clone();
    wrong.faces[0].surface.control_points[0][0][2] += 1.;
    assert!(RootBrep::from_topology(wrong).is_err());
    let mut vertex = model.topology().clone();
    vertex.vertices[0].point = vertex.vertices[1].point.clone();
    assert!(RootBrep::from_topology(vertex).is_err());
    let mut trim = model.topology().clone();
    trim.loops[0].coedges[0] = trim.loops[0].coedges[0]
        .clone()
        .with_trim(brep_topology::CoedgeTrim {
            curve_parameter: [0., 1.],
            pcurve_parameter: [0., 1.],
            periodic_lift: [[0; 2]; 2],
        })
        .unwrap();
    assert!(RootBrep::from_topology(trim).is_err());
    let mut degenerate = model.topology().clone();
    degenerate.edges[0].degenerate = true;
    assert!(RootBrep::from_topology(degenerate).is_err());
}
#[test]
fn irrational_vertex_flows_through_common_ports_wire_and_paired_brep() {
    use nurbs_core::ss_intersection as ss;
    use std::sync::Arc;
    let contour = fixture::wire();
    let first = &contour.edges()[0];
    let clock = Arc::new(ss::QualifiedSsAlgebraicClock::from_binary(
        &[-1., 0., 8.], [0.34375, 0.375], Default::default(),
    ).unwrap());
    let algebraic = ss::OriginalSsAlgebraicPoint::new(first.curve().clone(), clock).unwrap();
    let point = Arc::new(ss::OriginalSsRootPoint::from_algebraic(algebraic.clone(), Default::default()).unwrap());
    assert!(point.binary_parameter().is_none());
    point.reverify().unwrap();
    let again = ss::OriginalSsRootPoint::from_algebraic(algebraic, Default::default()).unwrap();
    assert!(point.same_selector(&again));
    let independent = ss::OriginalSsAlgebraicPoint::new(first.curve().clone(), Arc::new(
        ss::QualifiedSsAlgebraicClock::from_binary(&[-1., 0., 8.], [0.34375, 0.375], Default::default()).unwrap(),
    )).unwrap();
    assert!(!point.same_selector(&ss::OriginalSsRootPoint::from_algebraic(independent, Default::default()).unwrap()));
    let middle = ss::bind_original_ss_port(point.clone(), first.curve().clone(), [0;4], Default::default()).unwrap();
    let independent=ss::OriginalSsAlgebraicPoint::new(first.curve().clone(),Arc::new(
        ss::QualifiedSsAlgebraicClock::from_binary(&[2.,-1.,-16.,8.],[0.34,0.38],Default::default()).unwrap(),
    )).unwrap();
    let canonical=Arc::new(ss::OriginalSsRootPoint::from_algebraic(independent,Default::default()).unwrap());
    let other_middle=ss::bind_original_ss_port(canonical.clone(),first.curve().clone(),[0;4],Default::default()).unwrap();
    assert!(!middle.point().same_selector(other_middle.point()));
    assert!(middle.point().same_source_point(other_middle.point(),Default::default()).unwrap());
    let mut edges = vec![
        ss::OriginalSsRootEdge::new(first.start().clone(), middle.clone()).unwrap(),
        ss::OriginalSsRootEdge::new(other_middle, first.end().clone()).unwrap(),
    ];
    edges.extend_from_slice(&contour.edges()[1..]);
    assert!(ss::OriginalSsRootWire::new(edges.clone()).is_err());
    assert!(ss::OriginalSsRootWire::reconcile_selectors(edges.clone(),ss::SsAlgebraicClockLimits {max_operations:1,..Default::default()}).is_err());
    let wire = ss::OriginalSsRootWire::reconcile_selectors(edges,Default::default()).unwrap();
    wire.reverify().unwrap();
    let model = RootBrep::paired_contours(&wire, &[], 1e-7).unwrap();
    model.reverify().unwrap();
    assert_eq!(model.topology().vertices.len(), 9);
    assert_eq!(model.topology().edges.len(), 9);
    assert_eq!(model.topology().euler_characteristic(), 2);
    assert!(model.topology().boundary_edges().is_empty());
    assert!(model.topology().vertices.iter().any(|v| v.point.selector.same_selector(&canonical)));
    for face in 0..2 {
        let uv = model.pcurve_bounds(face,0,0,Default::default()).unwrap();
        assert!(uv.iter().all(|d| d[0]<=d[1]));
    }
}
#[test]
fn cross_axis_algebraic_junction_is_preserved_by_paired_brep() {
    use nurbs_core::ss_intersection as ss;
    use std::sync::Arc;
    let contour=fixture::wire();
    let edge=&contour.edges()[1];
    let make=|curve,p:&[f64],d| {
        let clock=Arc::new(ss::QualifiedSsAlgebraicClock::from_binary(p,d,Default::default()).unwrap());
        Arc::new(ss::OriginalSsRootPoint::from_algebraic(ss::OriginalSsAlgebraicPoint::new(curve,clock).unwrap(),Default::default()).unwrap())
    };
    let x=make(edge.curve().clone(),&[-257.,0.,4096.],[0.25,0.251]);
    let y=Arc::new(x.reparameterize_to(contour.edges()[2].curve().clone(),Default::default(),Default::default()).unwrap());
    let bind=|point|ss::bind_original_ss_port(point,edge.curve().clone(),[0;4],Default::default()).unwrap();
    let mut edges=vec![contour.edges()[0].clone(),
        ss::OriginalSsRootEdge::new(edge.start().clone(),bind(x)).unwrap(),
        ss::OriginalSsRootEdge::new(bind(y.clone()),edge.end().clone()).unwrap()];
    edges.extend_from_slice(&contour.edges()[2..]);
    assert!(ss::OriginalSsRootWire::new(edges.clone()).is_err());
    let wire=ss::OriginalSsRootWire::reconcile_selectors(edges,Default::default()).unwrap();
    wire.reverify().unwrap();
    let brep=RootBrep::paired_contours(&wire,&[],1e-7).unwrap();
    brep.reverify().unwrap();
    assert_eq!(brep.topology().vertices.len(),9);
    assert_eq!(brep.topology().edges.len(),9);
    assert!(brep.topology().boundary_edges().is_empty());
    assert!(brep.topology().vertices.iter().any(|v|v.point.selector.same_selector(&y)));
    for support in 0..2 {
        let uv=brep.pcurve_bounds(support,0,1,Default::default()).unwrap();
        assert!(uv.iter().all(|d|d[0]<=d[1]));
    }
}

#[test]
fn explicit_face_retention_keeps_only_complete_bodies() {
    use brep_core::root_model::RootFaceRetention;
    use std::sync::Arc;
    let source=Arc::new(RootBrep::paired_contours(&fixture::wire(),&[],1e-7).unwrap());
    let all=RootFaceRetention::retain(source.clone(),vec![1,0]).unwrap();
    assert_eq!(all.body_map(),&[Some(0)]);assert!(all.output().topology().shells[0].closed);
    all.reverify().unwrap();
    let partial=RootFaceRetention::retain(source.clone(),vec![0]).unwrap();
    assert_eq!(partial.body_map(),&[None]);assert!(partial.output().topology().bodies.is_empty());
    assert!(!partial.output().topology().shells[0].closed);partial.reverify().unwrap();
    assert!(RootFaceRetention::retain(source,vec![2]).is_err());
}
