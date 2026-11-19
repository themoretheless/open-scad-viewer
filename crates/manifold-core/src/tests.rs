use super::*;

#[test]
fn check_defect_lists_are_canonical_sorted_and_deterministic() {
    // A deliberately messy soup: triple edge, flipped face, repeated-index
    // triangle, bowtie vertex, isolated vertices, triangles in scrambled order.
    let positions: Vec<f64> = (0..12)
        .flat_map(|i| [i as f64, (i * i % 7) as f64, (i % 3) as f64])
        .collect();
    let indices = vec![
        9, 8, 10, 1, 0, 2, 0, 1, 3, 1, 0, 4, 5, 5, 6, 2, 1, 3, 8, 7, 9, 2, 3, 0, 7, 8, 11,
    ];
    let first = check(&positions, &indices);
    assert_eq!(first, check(&positions, &indices));
    for edges in [
        &first.boundary_edges,
        &first.non_manifold_edges,
        &first.orientation_edges,
    ] {
        assert!(edges.iter().all(|&(a, b)| a < b), "{edges:?}");
        assert!(edges.windows(2).all(|w| w[0] < w[1]), "{edges:?}");
    }
    for list in [
        &first.degenerate_triangles,
        &first.non_manifold_vertices,
        &first.isolated_vertices,
    ] {
        assert!(list.windows(2).all(|w| w[0] < w[1]), "{list:?}");
    }
    assert!(!first.non_manifold_edges.is_empty());
    assert_eq!(first.degenerate_triangles, vec![4]);

    let empty = check(&[], &[]);
    assert_eq!(empty.component_count, 0);
    assert!(empty.is_manifold_with_boundary() && empty.boundary_edges.is_empty());
}

/// Unit cube: 8 vertices, 12 triangles, consistently oriented (CCW
/// seen from outside).
fn cube() -> (Vec<f64>, Vec<usize>) {
    let positions = vec![
        0.0, 0.0, 0.0, // 0
        1.0, 0.0, 0.0, // 1
        1.0, 1.0, 0.0, // 2
        0.0, 1.0, 0.0, // 3
        0.0, 0.0, 1.0, // 4
        1.0, 0.0, 1.0, // 5
        1.0, 1.0, 1.0, // 6
        0.0, 1.0, 1.0, // 7
    ];
    let indices = vec![
        0, 2, 1, 0, 3, 2, // bottom (z = 0, outward -z)
        4, 5, 6, 4, 6, 7, // top (z = 1, outward +z)
        0, 1, 5, 0, 5, 4, // front (y = 0)
        2, 3, 7, 2, 7, 6, // back (y = 1)
        1, 2, 6, 1, 6, 5, // right (x = 1)
        3, 0, 4, 3, 4, 7, // left (x = 0)
    ];
    (positions, indices)
}

#[test]
fn closed_cube_is_manifold() {
    let (p, i) = cube();
    let report = check(&p, &i);
    assert!(report.is_manifold(), "{report:?}");
    assert!(is_manifold(&p, &i));
    assert_eq!(report.component_count, 1);
    assert_eq!(report.triangle_count, 12);
    assert!(report.isolated_vertices.is_empty());
}

#[test]
fn open_mesh_reports_boundary_only() {
    // Single quad (two triangles): manifold with boundary.
    let p = vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0];
    let i = vec![0, 1, 2, 0, 2, 3];
    let report = check(&p, &i);
    assert!(!report.is_manifold());
    assert!(report.is_manifold_with_boundary(), "{report:?}");
    assert_eq!(report.boundary_edges.len(), 4);
}

#[test]
fn triple_edge_is_non_manifold() {
    // Three triangles share edge (0,1).
    let p = vec![
        0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, -1.0, 0.0,
    ];
    let i = vec![0, 1, 2, 0, 4, 1, 0, 1, 3];
    let report = check(&p, &i);
    assert!(
        report.non_manifold_edges.contains(&edge_key(0, 1)),
        "{report:?}"
    );
    assert!(!report.is_manifold());
}

#[test]
fn flipped_triangle_is_orientation_mismatch() {
    // Quad where the second triangle is flipped.
    let p = vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0];
    let i = vec![0, 1, 2, 0, 3, 2]; // (0,2) traversed 2->0 and 0->2? second: 0->3,3->2,2->0
    let report = check(&p, &i);
    assert_eq!(report.orientation_edges, vec![edge_key(0, 2)]);
    assert!(!report.is_manifold_with_boundary());
}

#[test]
fn pinched_vertex_is_detected() {
    // Two tetra-like fans touching only at vertex 0 (bowtie).
    let p = vec![
        0.0, 0.0, 0.0, // 0 shared tip
        1.0, 0.0, 0.0, 0.0, 1.0, 0.0, // fan A ring
        0.0, 0.0, 1.0, -1.0, 0.0, 0.0, // fan B ring
    ];
    let i = vec![0, 1, 2, 0, 3, 4];
    let report = check(&p, &i);
    assert_eq!(report.non_manifold_vertices, vec![0]);
    assert!(!report.is_manifold());
}

#[test]
fn degenerate_triangle_is_reported() {
    let p = vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
    let i = vec![0, 1, 1];
    let report = check(&p, &i);
    assert_eq!(report.degenerate_triangles, vec![0]);
    assert_eq!(report.isolated_vertices, vec![2]);
}

#[test]
fn disconnected_components_are_counted() {
    let (p, i) = cube();
    // Two separate quads: two components, four boundary edges each.
    let report = check(&p[..12], &i[..6]); // bottom face only -> 1 component
    assert_eq!(report.component_count, 1);
    let both = check(&p, &[i[..6].to_vec(), i[6..12].to_vec()].concat()); // bottom + top
    assert_eq!(both.component_count, 2);
}

/// Closed band of `n` quads around the z axis; `twist` reconnects the
/// last segment with top and bottom swapped (a Möbius strip).
fn band(n: usize, twist: bool) -> (Vec<f64>, Vec<usize>) {
    let mut p = Vec::new();
    for i in 0..n {
        let a = i as f64 / n as f64 * std::f64::consts::TAU;
        p.extend([a.cos(), a.sin(), 1.0]); // top 2i
        p.extend([a.cos(), a.sin(), -1.0]); // bottom 2i+1
    }
    let mut idx = Vec::new();
    for i in 0..n {
        let (t0, b0) = (2 * i, 2 * i + 1);
        let (t1, b1) = if i + 1 < n {
            (2 * i + 2, 2 * i + 3)
        } else if twist {
            (1, 0)
        } else {
            (0, 1)
        };
        idx.extend([t0, b0, t1, b0, b1, t1]);
    }
    (p, idx)
}

#[test]
fn mobius_strip_is_non_orientable() {
    let (p, i) = band(8, true);
    let report = check(&p, &i);
    assert_eq!(report.non_orientable_components, 1, "{report:?}");
    assert_eq!(report.manifold_component_count, 1);
    assert!(!report.orientation_edges.is_empty());
    let (p, i) = band(8, false);
    let report = check(&p, &i);
    assert_eq!(report.non_orientable_components, 0, "{report:?}");
    assert!(report.is_manifold_with_boundary());
    // A flipped triangle in an orientable mesh is repairable, not
    // non-orientable.
    let (p, mut i) = cube();
    i.swap(0, 1);
    let report = check(&p, &i);
    assert!(!report.orientation_edges.is_empty());
    assert_eq!(report.non_orientable_components, 0);
    assert!(check(&cube().0, &cube().1).non_orientable_components == 0);
}

#[test]
fn zero_area_and_sliver_triangles_are_split_by_threshold() {
    let p = vec![
        0.0, 0.0, 0.0, // 0
        1.0, 0.0, 0.0, // 1
        2.0, 0.0, 0.0, // 2 collinear with 0, 1
        0.0, 1.0, 0.0, // 3
        1.0, 1e-6, 0.0, // 4 needle apex
    ];
    // repeated index, zero area, needle, regular.
    let i = vec![0, 0, 1, 0, 1, 2, 0, 2, 4, 0, 1, 3];
    let default = check(&p, &i);
    assert_eq!(default.degenerate_triangles, vec![0]);
    assert_eq!(default.zero_area_triangles, vec![1]);
    assert!(default.sliver_triangles.is_empty());
    let report = check_with_options(
        &p,
        &i,
        &CheckOptions {
            sliver_area_threshold: 1e-3,
        },
    );
    assert_eq!(report.degenerate_triangles, vec![0]);
    assert_eq!(report.zero_area_triangles, vec![1]);
    assert_eq!(report.sliver_triangles, vec![2]);
    let negative = check_with_options(
        &p,
        &i,
        &CheckOptions {
            sliver_area_threshold: -1.0,
        },
    );
    assert!(negative.sliver_triangles.is_empty());
}

#[test]
fn isolated_vertex_removal_round_trips_through_remap() {
    let (cp, ci) = cube();
    // Interleave an unused vertex before every cube vertex.
    let mut p = Vec::new();
    for v in 0..8 {
        p.extend([9.0, 9.0, v as f64]);
        p.extend_from_slice(&cp[v * 3..v * 3 + 3]);
    }
    let i: Vec<usize> = ci.iter().map(|&v| 2 * v + 1).collect();
    assert_eq!(check(&p, &i).isolated_vertices.len(), 8);
    let out = remove_isolated_vertices(&p, &i);
    assert_eq!(out.removed, 8);
    assert_eq!(out.positions, cp);
    assert_eq!(out.indices, ci);
    for (old, new) in out.remap.iter().enumerate() {
        match new {
            Some(n) => assert_eq!(&p[old * 3..old * 3 + 3], &out.positions[n * 3..n * 3 + 3]),
            None => assert_eq!(old % 2, 0),
        }
    }
    for (k, &old) in i.iter().enumerate() {
        assert_eq!(out.remap[old], Some(out.indices[k]));
    }
    assert!(check(&out.positions, &out.indices).is_manifold());

    let repaired = repair_with_options(
        &p,
        &i,
        &RepairOptions {
            remove_isolated_vertices: true,
            ..RepairOptions::default()
        },
    )
    .unwrap();
    assert_eq!(repaired.report.removed_isolated_vertices, 8);
    assert_eq!(repaired.positions.len(), 24);
    assert!(repaired.is_manifold());
    for (k, &old) in i.iter().enumerate() {
        assert_eq!(repaired.vertex_remap[old], Some(repaired.indices[k]));
    }
    let kept = repair(&p, &i, 0.0);
    assert_eq!(kept.report.removed_isolated_vertices, 0);
    assert_eq!(kept.positions.len(), p.len());
}

#[test]
fn bowtie_vertex_and_fin_edge_split_manifold_components() {
    let tetra = [0usize, 2, 1, 0, 1, 3, 1, 2, 3, 2, 0, 3];
    let p = vec![
        0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, // A
        -1.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, -1.0, // B shares vertex 0
    ];
    let map_b = [0usize, 4, 5, 6];
    let mut i = tetra.to_vec();
    i.extend(tetra.iter().map(|&v| map_b[v]));
    let report = check(&p, &i);
    assert_eq!(report.non_manifold_vertices, vec![0]);
    assert_eq!(report.component_count, 2);
    assert_eq!(report.manifold_component_count, 2);
    assert_eq!(report.non_orientable_components, 0);

    // Three triangles on one edge: one face-graph component, three
    // manifold components.
    let p = vec![
        0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, -1.0, 0.0,
    ];
    let report = check(&p, &[0, 1, 2, 0, 4, 1, 0, 1, 3]);
    assert_eq!(report.component_count, 1);
    assert_eq!(report.manifold_component_count, 3);
    assert_eq!(check(&[], &[]).manifold_component_count, 0);
}
