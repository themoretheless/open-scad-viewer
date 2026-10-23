#[path = "../examples/support/corner_close_fixture.rs"]
mod fixture;
use brep_core::{corner_close, trimmed_vertex_contact};
#[test]
fn all_four_vertex_neighbors_are_bounded_to_owned_corners() {
    let (source, uses, options) = fixture::fixture_extended();
    let out = corner_close::close(
        &source,
        0,
        uses,
        options,
        1e-8,
        [1e-7, 1e-5, 1e-4],
        100000,
        1000000,
    )
    .unwrap();
    let before = out.model.clone();
    let mut cells = out.cells;
    let mut exact = out.exact_work;
    for face in [1, 5, 12, 16] {
        let a = trimmed_vertex_contact::inspect(
            &out.model,
            [face, out.face],
            100000 - cells,
            1000000 - exact,
        )
        .unwrap();
        cells += a.cells;
        exact += a.exact_work;
        let c = a.certificate.unwrap();
        assert_eq!(c.exact_vertex_contact, face == 5 || face == 16);
        if c.exact_vertex_contact {
            assert_eq!(c.distance_upper, 0.);
        }
        assert!(c.distance_upper <= out.model.tolerance_mm);
        assert!(c.hull_penetrations.iter().all(|v| *v < 1e-12));
    }
    assert!(cells <= 100000 && exact <= 1000000);
    assert_eq!(out.model, before);
    let without_exact =
        trimmed_vertex_contact::inspect(&out.model, [5, out.face], 100000, 0).unwrap();
    assert!(!without_exact.certificate.unwrap().exact_vertex_contact);
    let mut small_penetration = out.model.clone();
    small_penetration.faces[out.face].surface.control_points[2][2][1] = 1. + 1e-12;
    small_penetration.rebuild_topology_ids();
    let p = trimmed_vertex_contact::inspect(&small_penetration, [5, out.face], 100000, 1000000)
        .unwrap()
        .certificate
        .unwrap();
    assert!(p.distance_upper <= out.model.tolerance_mm && !p.exact_vertex_contact);
    assert!(p.hull_penetrations.iter().any(|x| *x > 0.));
    let zero = trimmed_vertex_contact::inspect(&out.model, [1, out.face], 0, 0).unwrap();
    assert!(zero.certificate.is_none() && zero.cells == 0 && zero.exact_work == 0);
    let edge_pair =
        trimmed_vertex_contact::inspect(&out.model, [uses[0].face, out.face], 100000, 1000000)
            .unwrap();
    assert!(edge_pair.certificate.is_none());
    let mut crossing_hull = out.model.clone();
    crossing_hull.faces[out.face].surface.control_points[2][2][0] = -0.1;
    crossing_hull.rebuild_topology_ids();
    let a =
        trimmed_vertex_contact::inspect(&crossing_hull, [1, out.face], 100000, 1000000).unwrap();
    assert!(a.certificate.is_none());
    let mut coupled = out.model.clone();
    coupled.faces[1].surface.control_points[0][0][1] += 0.01;
    coupled.rebuild_topology_ids();
    let a = trimmed_vertex_contact::inspect(&coupled, [1, out.face], 100000, 1000000).unwrap();
    assert!(a.certificate.is_none());
}
