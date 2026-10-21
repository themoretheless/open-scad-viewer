use brep_core::{corner_close, region_replacement};
#[path = "../examples/support/corner_close_fixture.rs"]
mod fixture;
#[test]
fn mapped_corner_replaces_owned_sector_and_preserves_saved_rim_identities() {
    let (open, uses, options) = fixture::fixture();
    let closed = corner_close::close(
        &open,
        0,
        uses,
        options,
        1e-8,
        [1e-7, 1e-5, 1e-4],
        100000,
        1000000,
    )
    .unwrap();
    let source = &closed.model;
    let saved = source.clone();
    let mut mapped_options = options;
    mapped_options.reverse_along = [false, true, true, false];
    mapped_options.gains = [4.; 4];
    let result = region_replacement::replace_g2_mapped(
        source,
        &[closed.face],
        uses,
        [[-0.25, 0.], [0.25, 0.], [0., -0.25], [0., 0.25]],
        mapped_options,
        1e-8,
        [1e-7, 1e-5, 1e-4],
        100000,
        2000000,
        1000000,
    )
    .unwrap();
    assert!(result.corner.admitted);
    assert_eq!(
        result.replacement.closure.closure.status,
        brep_core::watertight::Closure::WithinTolerance
    );
    assert_eq!(result.replacement.model.bodies.len(), 1);
    assert!(result.cells <= 100000 && result.exact_work <= 1000000);
    for u in uses {
        let edge = result.replacement.source_edges[u.edge].unwrap();
        assert_eq!(result.replacement.model.edges[edge], source.edges[u.edge]);
        assert_eq!(
            result.replacement.model.1.edges[edge],
            source.1.edges[u.edge]
        );
        for v in source.edges[u.edge].vertices {
            assert!(result.replacement.source_vertices[v].is_some());
            let new_vertex = result.replacement.source_vertices[v].unwrap();
            assert_eq!(
                result.replacement.model.1.vertices[new_vertex],
                source.1.vertices[v]
            );
        }
    }
    assert_eq!(*source, saved);
    let encoded = value_codec::to_string(&result.replacement.model).unwrap();
    let restored: brep_core::Model = value_codec::from_str(&encoded).unwrap();
    restored.validate().unwrap();
    assert!(restored.persistent_naming_complete());
    let closure = brep_core::watertight::inspect_with_exact(&restored, 100000, 1000000).unwrap();
    assert_eq!(
        closure.closure.status,
        brep_core::watertight::Closure::WithinTolerance
    );
    let mut repeated = uses;
    repeated[1] = repeated[0];
    assert!(
        region_replacement::replace_g2_mapped(
            source,
            &[closed.face],
            repeated,
            [[-0.25, 0.], [0.25, 0.], [0., -0.25], [0., 0.25]],
            mapped_options,
            1e-8,
            [1e-7, 1e-5, 1e-4],
            100000,
            2000000,
            1000000
        )
        .is_err()
    );
    println!(
        "mapped B-rep: cells={} exact={} coefficients={}",
        result.cells, result.exact_work, result.corner.coefficient_work
    );
}
