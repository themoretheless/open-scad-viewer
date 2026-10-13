use super::*;

#[test]
fn triangle_inset_offsets_each_original_edge_once() {
    let polygon = vec![[0., 0.], [8., 0.], [4., 4. * 3f64.sqrt()]];
    let inset = inset_polygon(polygon, 0.5);
    assert_eq!(inset.len(), 3);
    for i in 0..3 {
        let a = inset[i];
        let b = inset[(i + 1) % 3];
        assert!(((a[0] - b[0]).hypot(a[1] - b[1]) - (8. - 3f64.sqrt())).abs() < 1e-10);
    }
}

#[test]
fn oversized_inset_returns_empty_without_indexing_removed_vertices() {
    assert!(inset_polygon(vec![[0., 0.], [1., 0.], [0., 1.]], 2.).is_empty());
}

#[test]
fn honeycomb_does_not_apply_web_jitter() {
    let regular =
        generate_lightening_cells([0., 0.], [1., 9.], "honeycomb", 6., 0.2, 42., 0.).unwrap();
    let candidate =
        generate_lightening_cells([0., 0.], [1., 9.], "honeycomb", 6., 0.2, 99., 1.).unwrap();
    assert_eq!(candidate, regular);
}
