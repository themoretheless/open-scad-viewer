#[path = "../examples/support/corner_close_fixture.rs"]
mod fixture;
use brep_core::{corner_close, trimmed_edge_contact};
#[test]
fn trimmed_original_neighbors_have_only_tolerance_local_shared_edge_contacts() {
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
    for use_ in uses {
        let a = trimmed_edge_contact::inspect(
            &out.model,
            [use_.face, out.face],
            100000 - cells,
            1000000 - exact,
        )
        .unwrap();
        cells += a.cells;
        exact += a.exact_work;
        let c = a.certificate.unwrap();
        assert_eq!(c.edge, use_.edge);
        assert!(c.distance_upper <= out.model.tolerance_mm);
        assert!(c.hull_penetration_upper < 1e-12);
    }
    assert!(cells <= 100000 && exact <= 1000000);
    assert_eq!(out.model, before);
    let zero = trimmed_edge_contact::inspect(&out.model, [uses[0].face, out.face], 0, 0).unwrap();
    assert!(zero.certificate.is_none() && zero.cells == 0 && zero.exact_work == 0);
    let vertex_only =
        trimmed_edge_contact::inspect(&out.model, [1, out.face], 100000, 1000000).unwrap();
    assert!(vertex_only.certificate.is_none());
    let mut crossing_hull = out.model.clone();
    crossing_hull.faces[out.face].surface.control_points[2][2][0] = -0.1;
    crossing_hull.rebuild_topology_ids();
    let a =
        trimmed_edge_contact::inspect(&crossing_hull, [uses[0].face, out.face], 100000, 1000000)
            .unwrap();
    assert!(a.certificate.is_none());
}
