use brep_core::minkowski3d::curved;
#[path = "support/independent_graph_solids.rs"]
mod fixture;
#[test]
fn touching_independent_curved_bodies_have_no_common_material_interior() {
    let (a, b) = fixture::boundary_touch_solids();
    let r = curved::certify_nurbs_boundary_only_boolean(&a, &b, "intersection", 0).unwrap();
    assert_eq!(r.certificate().separating_band, [1., 1.]);
    assert!(r.model().is_empty());
    curved::verify_nurbs_boundary_only_boolean(&a, &b, r.model(), r.certificate()).unwrap();
    let d = curved::certify_nurbs_boundary_only_boolean(&a, &b, "difference", 0).unwrap();
    assert_eq!(d.model(), &a);
    assert!(
        brep_core::operations::boolean(&a, &b, "intersection")
            .unwrap()
            .is_empty()
    );
    let mut forged = r.certificate().clone();
    forged.separating_band = [1.125; 2];
    assert!(curved::verify_nurbs_boundary_only_boolean(&a, &b, r.model(), &forged).is_err());
}
#[test]
fn a_small_positive_overlap_never_receives_a_boundary_only_certificate() {
    let (a, b) = fixture::boundary_touch_solids();
    let mut b = b;
    for v in &mut b.vertices {
        v.point[0] -= 0.125;
    }
    for e in &mut b.edges {
        for p in &mut e.curve.control_points {
            p[0] -= 0.125;
        }
    }
    for f in &mut b.faces {
        for p in f.surface.control_points.iter_mut().flatten() {
            p[0] -= 0.125;
        }
    }
    b.rebuild_topology_ids();
    b.validate().unwrap();
    assert!(curved::certify_nurbs_boundary_only_boolean(&a, &b, "intersection", 0).is_err());
    assert!(curved::certify_nurbs_boundary_only_boolean(&a, &b, "union", 0).is_err());
}
