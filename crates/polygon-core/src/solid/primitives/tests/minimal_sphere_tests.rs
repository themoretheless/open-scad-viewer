#[test]
fn three_segments_form_a_closed_positive_volume_mesh() {
    let mesh = super::sphere(2., 3).unwrap();
    assert_eq!(mesh.indices.len(), 18);
    let mut edges = std::collections::BTreeMap::new();
    for triangle in mesh.indices.chunks_exact(3) {
        for (a, b) in [
            (triangle[0], triangle[1]),
            (triangle[1], triangle[2]),
            (triangle[2], triangle[0]),
        ] {
            let entry = edges.entry((a.min(b), a.max(b))).or_insert((0, 0));
            entry.0 += 1;
            entry.1 += if a < b { 1 } else { -1 };
        }
    }
    assert!(
        edges
            .values()
            .all(|&(count, orientation)| count == 2 && orientation == 0)
    );
    assert!(mesh.inspect().unwrap().signed_volume_mm3 > 0.);
    assert!(super::sphere(2., 2).is_err());
}
