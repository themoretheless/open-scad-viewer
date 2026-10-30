use super::*;
fn limits() -> Limits {
    Limits {
        operations: 1000000,
        body: crate::graph_body::tests::limits().body,
    }
}
#[test]
fn exact_stock_admits_actual_cuboid_axes_and_retains_original_material() {
    let source = crate::cuboid([0.; 3], [10.; 3]).unwrap();
    let before = source.clone();
    let mut l = limits();
    let body = qualify(&source, &l).unwrap();
    assert_eq!(body.bounds(), [[0., 10.]; 3]);
    assert_eq!(
        body.planes().iter().copied().collect::<BTreeSet<_>>().len(),
        6
    );
    assert!(body.body().volume()[0] <= 1000. && body.body().volume()[1] >= 1000.);
    assert!(body.body().geometry().contacts().all_pairs_qualified);
    body.reverify().unwrap();
    let work = body.operations();
    drop(body);
    l.operations = work;
    assert_eq!(qualify(&source, &l).unwrap().operations(), work);
    l.operations -= 1;
    assert!(qualify(&source, &l).is_err());
    assert_eq!(source, before);
}
#[test]
fn sampled_tolerance_and_closed_metadata_do_not_make_stock_planar_or_owned() {
    let source = crate::cuboid([0.; 3], [10.; 3]).unwrap();
    let l = limits();
    let body = qualify(&source, &l).unwrap();
    let plane = body.planes()[4];
    let mut changed = source.clone();
    changed.0.vertices[0].point[0] = changed.vertices[0].point[0].next_up();
    changed.rebuild_topology_ids();
    changed.validate().unwrap();
    assert!(qualify(&changed, &l).is_err());
    let mut changed = source.clone();
    let s = nurbs_core::surface_edit::elevate(
        &changed.faces[plane].surface,
        nurbs_core::surface::Axis::U,
        2,
    )
    .unwrap();
    changed.0.faces[plane].surface =
        nurbs_core::surface_edit::elevate(&s, nurbs_core::surface::Axis::V, 2).unwrap();
    changed.0.faces[plane].surface.control_points[1][1][2] = 0f64.next_up();
    changed.rebuild_topology_ids();
    changed.validate().unwrap();
    assert!(qualify(&changed, &l).is_err());
    let mut changed = source.clone();
    changed.0.shells[0].faces[0] = changed.shells[0].faces[1].clone();
    assert!(qualify(&changed, &l).is_err());
}
