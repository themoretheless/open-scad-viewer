use super::*;
#[test]
fn removing_a_closed_curved_region_creates_and_reproves_a_cavity() {
    let container = crate::cuboid([-4.; 3], [4.; 3]).unwrap();
    let tool = crate::frustum(1., 0., 2.).unwrap();
    let original = container.clone();
    let original_tool = tool.clone();
    let result = certify_enclosed_difference(&container, &tool, 0).unwrap();
    assert_eq!(container, original);
    assert_eq!(tool, original_tool);
    assert_eq!(result.model().bodies.len(), 1);
    assert_eq!(result.model().bodies[0].inner_shells.len(), 1);
    assert_eq!(result.model().shells.len(), 2);
    assert!(result.model().shells[1]
        .faces
        .iter()
        .zip(&tool.shells[0].faces)
        .all(|(a, b)| a.reversed != b.reversed));
    verify_enclosed_difference(&container, &tool, result.model(), result.certificate()).unwrap();
    let mut false_support = result.certificate().clone();
    false_support.convex_supports.pop();
    assert!(verify_enclosed_difference(&container, &tool, result.model(), &false_support).is_err());
    let mut forged = result.model().clone();
    forged.shells[1].faces[0].reversed = !forged.shells[1].faces[0].reversed;
    forged.rebuild_topology_ids();
    assert!(verify_enclosed_difference(&container, &tool, &forged, result.certificate()).is_err());
    let mut changed = tool;
    changed.vertices[0].point[0] += 0.25;
    changed.rebuild_topology_ids();
    assert!(
        verify_enclosed_difference(&container, &changed, result.model(), result.certificate())
            .is_err()
    );
}
#[test]
fn crossing_and_inward_tools_do_not_create_unproved_open_bodies() {
    let container = crate::cuboid([0.; 3], [2.; 3]).unwrap();
    let touching = crate::cuboid([0.5, 0.5, 0.], [1., 1., 0.5]).unwrap();
    assert!(certify_enclosed_difference(&container, &touching, 0).is_err());
    let crossing = crate::cuboid([1.5; 3], [2.5; 3]).unwrap();
    assert!(certify_enclosed_difference(&container, &crossing, 0).is_err());
    let mut inward = crate::cuboid([0.5; 3], [1.; 3]).unwrap();
    for f in &mut inward.shells[0].faces {
        f.reversed = !f.reversed;
    }
    inward.rebuild_topology_ids();
    assert!(certify_enclosed_difference(&container, &inward, 0).is_err());
}
