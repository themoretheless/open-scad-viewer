use super::*;
#[test]

#[cfg(feature = "codec")]
fn box_incidence_and_serialization() {
    let m = cuboid([0.; 3], [2., 3., 4.]).unwrap();
    let r = m.validate().unwrap();
    assert_eq!(
        (r.vertex_count, r.edge_count, r.face_count, r.body_count),
        (8, 12, 6, 1)
    );
    let restored: Model = value_codec::from_str(&value_codec::to_string(&m).unwrap()).unwrap();
    restored.validate().unwrap();
}
#[test]
fn rejects_broken_incidence_geometry_and_orientation() {
    let a = cuboid([0.; 3], [1.; 3]).unwrap();
    for kind in 0..6 {
        let mut m = a.clone();
        match kind {
            0 => m.0.edges[0].vertices[0] = 999,
            1 => m.0.loops[0].coedges[0].reversed ^= true,
            2 => m.0.shells[0].faces[0].reversed = true,
            3 => m.0.faces[0].outer = m.0.faces[1].outer,
            4 => m.0.vertices[0].point[0] = 0.2,
            _ => m.0.bodies[0].inner_shells.push(0),
        }
        assert!(m.validate().is_err(), "mutation {kind}");
    }
}
#[test]
fn open_shell_is_not_a_body() {
    let mut m = cuboid([0.; 3], [1.; 3]).unwrap();
    m.0.shells[0].closed = false;
    assert!(m.validate().is_err());
    m.0.bodies.clear();
    m.rebuild_topology_ids();
    m.validate().unwrap();
}
