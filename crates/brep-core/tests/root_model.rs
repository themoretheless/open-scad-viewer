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
