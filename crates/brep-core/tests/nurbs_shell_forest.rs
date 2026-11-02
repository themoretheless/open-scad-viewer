use brep_core::minkowski3d::curved::{self, NurbsGraphLimits};
use brep_core::{Model, cuboid};
fn cavity() -> Model {
    curved::certify_enclosed_difference(
        &cuboid([0.; 3], [1.; 3]).unwrap(),
        &cuboid([0.25; 3], [0.75; 3]).unwrap(),
        0,
    )
    .unwrap()
    .into_model()
}
#[test]
fn exact_shell_forest_checks_cavities_and_rejects_forged_roles() {
    let m = cavity();
    let c = curved::certify_nurbs_shell_forest(&m, 0).unwrap();
    assert_eq!(c.orientation, vec![1, -1]);
    assert_eq!(c.parents, vec![None, Some(0)]);
    assert_eq!(c.bodies[0].inner_shells, vec![1]);
    curved::verify_nurbs_shell_forest(&m, &c).unwrap();
    let mut forged = c.clone();
    forged.contains[0][1] = false;
    assert!(curved::verify_nurbs_shell_forest(&m, &forged).is_err());
    let mut bad = m.clone();
    bad.bodies[0].inner_shells.clear();
    bad.bodies.push(brep_core::Body {
        outer_shell: 1,
        inner_shells: vec![],
    });
    bad.rebuild_topology_ids();
    assert!(curved::certify_nurbs_shell_forest(&bad, 0).is_err());
}
#[test]
fn material_in_a_cavity_is_void_and_union_owns_a_nested_island() {
    let a = cavity();
    let b = cuboid([0.4375; 3], [0.5625; 3]).unwrap();
    let limits = NurbsGraphLimits {
        source_depth: 0,
        ..Default::default()
    };
    let empty = curved::author_retained_nurbs_boolean(&a, &b, "intersection", limits).unwrap();
    assert!(empty.model().is_empty());
    let union = curved::author_retained_nurbs_boolean(&a, &b, "union", limits).unwrap();
    assert_eq!(union.model().bodies.len(), 2);
    assert_eq!(union.model().shells.len(), 3);
    let c = union.certificate().result_shells.as_ref().unwrap();
    assert_eq!(c.orientation, vec![1, -1, 1]);
    assert_eq!(c.parents, vec![None, Some(0), Some(1)]);
    assert_eq!(union.model().bodies[0].inner_shells, vec![1]);
    assert!(union.model().bodies[1].inner_shells.is_empty());
    curved::verify_retained_nurbs_boolean(&a, &b, union.model(), union.certificate()).unwrap();
}
#[test]
fn multiple_original_bodies_are_not_reduced_to_one_outer_shell() {
    let a = brep_core::operations::boolean(
        &cuboid([0.; 3], [1.; 3]).unwrap(),
        &cuboid([2., 0., 0.], [3., 1., 1.]).unwrap(),
        "union",
    )
    .unwrap();
    assert_eq!(a.bodies.len(), 2);
    let b = cuboid([4., 0., 0.], [5., 1., 1.]).unwrap();
    let r = curved::author_retained_nurbs_boolean(
        &a,
        &b,
        "union",
        NurbsGraphLimits {
            source_depth: 0,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(r.model().bodies.len(), 3);
    assert_eq!(
        r.certificate().result_shells.as_ref().unwrap().parents,
        vec![None; 3]
    );
}
