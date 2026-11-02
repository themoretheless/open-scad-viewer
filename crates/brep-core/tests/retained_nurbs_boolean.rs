use brep_core::minkowski3d::curved::{self, NurbsGraphLimits};
#[path = "support/independent_graph_solids.rs"]
mod fixture;

#[test]
fn independent_curved_carriers_partition_material_and_author_an_ordinary_model() {
    let (a, b) = fixture::layered_solids();
    let r =
        curved::author_retained_nurbs_boolean(&a, &b, "intersection", NurbsGraphLimits::default())
            .unwrap();
    r.model().validate().unwrap();
    assert!(!r.model().is_empty());
    assert_eq!(r.model().bodies.len(), 1);
    assert_eq!(r.certificate().graph.cycles.len(), 2);
    assert!(
        r.certificate()
            .partitions
            .iter()
            .any(|p| p.model.faces.len() > a.faces.len())
    );
    assert!(r.certificate().regions.iter().any(|r| r.keep));
    assert!(r.certificate().regions.iter().any(|r| !r.keep));
    assert!(r.model().faces.iter().any(|f| f.surface.degree_u > 1));
    assert_eq!(r.model().vertices.len(), 8);
    for v in &r.model().vertices {
        let [x, y, z] = v.point;
        assert!(x == 0. || x == 1.);
        assert!(y == x * x / 16. || y == 1. + x * x / 16.);
        assert!(z == 0.25 + x * x / 16. || z == 0.75 + x * x / 16.);
    }
    eprintln!(
        "faces={} edges={} regions={}",
        r.model().faces.len(),
        r.model().edges.len(),
        r.certificate().regions.len()
    );
    curved::verify_retained_nurbs_boolean(&a, &b, r.model(), r.certificate()).unwrap();

    // A middle-pole edit stays far below Model's tolerance and keeps endpoints
    // unchanged. Exact source lift, not those tolerance checks, must reject it.
    let mut forged = r.certificate().partitions.clone();
    let pc = forged[0]
        .model
        .loops
        .iter_mut()
        .flat_map(|l| &mut l.coedges)
        .find(|c| c.pcurve.degree == 2)
        .unwrap();
    pc.pcurve.control_points[1][1] += 1e-12;
    assert!(
        curved::certify_retained_nurbs_boolean(
            &a,
            &b,
            "intersection",
            &forged,
            NurbsGraphLimits::default()
        )
        .is_err()
    );
}

#[test]
fn independently_warped_union_and_differences_survive_original_seams() {
    let (a, b) = fixture::layered_solids();
    for (left, right, op, bodies) in [
        (&a, &b, "difference", 2),
        (&a, &b, "union", 1),
        (&b, &a, "difference", 1),
    ] {
        let r = curved::author_retained_nurbs_boolean(left, right, op, NurbsGraphLimits::default())
            .unwrap();
        r.model().validate().unwrap();
        assert_eq!(r.model().bodies.len(), bodies, "{op}");
        assert!(r.model().persistent_naming_complete());
        assert_eq!(
            r.certificate().regions.iter().filter(|r| r.keep).count(),
            r.model().faces.len()
        );
        if op == "difference" {
            assert!(r.certificate().regions.iter().any(|r| r.keep && r.reverse));
        }
        eprintln!(
            "{op}: bodies={} faces={}",
            r.model().bodies.len(),
            r.model().faces.len()
        );
    }
}

#[test]
fn standard_boolean_uses_the_checked_retained_region_author() {
    let (a, b) = fixture::layered_solids();
    let r = brep_core::operations::boolean(&a, &b, "intersection").unwrap();
    r.validate().unwrap();
    assert_eq!(r.bodies.len(), 1);
    assert_eq!(r.faces.len(), 6);
    // The carrier is copied, not tessellated or fitted into a planar result.
    assert!(r.faces.iter().all(|f| {
        a.faces
            .iter()
            .chain(&b.faces)
            .any(|s| s.surface == f.surface)
    }));
}

#[test]
fn irrational_authored_endpoints_and_exhausted_graph_work_do_not_become_models() {
    let (a, b) = fixture::solids();
    assert!(
        curved::author_retained_nurbs_boolean(&a, &b, "intersection", NurbsGraphLimits::default())
            .is_err()
    );
    let (a, b) = fixture::layered_solids();
    let limits = NurbsGraphLimits {
        max_rectangles: 1,
        ..Default::default()
    };
    assert!(curved::author_retained_nurbs_boolean(&a, &b, "intersection", limits).is_err());
}
