use super::*;
use crate::is_manifold;

/// Cube with each face's vertices duplicated (unwelded soup): 24
/// positions, same 12 triangles as the welded cube.
fn unwelded_cube() -> (Vec<f64>, Vec<usize>) {
    // Face-local vertices; every corner appears 3 times.
    let faces: [[(f64, f64, f64); 4]; 6] = [
        // bottom (z=0, outward -z): CCW from below
        [
            (0.0, 0.0, 0.0),
            (0.0, 1.0, 0.0),
            (1.0, 1.0, 0.0),
            (1.0, 0.0, 0.0),
        ],
        // top (z=1, outward +z)
        [
            (0.0, 0.0, 1.0),
            (1.0, 0.0, 1.0),
            (1.0, 1.0, 1.0),
            (0.0, 1.0, 1.0),
        ],
        // front (y=0, outward -y)
        [
            (0.0, 0.0, 0.0),
            (1.0, 0.0, 0.0),
            (1.0, 0.0, 1.0),
            (0.0, 0.0, 1.0),
        ],
        // back (y=1, outward +y)
        [
            (1.0, 1.0, 0.0),
            (0.0, 1.0, 0.0),
            (0.0, 1.0, 1.0),
            (1.0, 1.0, 1.0),
        ],
        // right (x=1, outward +x)
        [
            (1.0, 0.0, 0.0),
            (1.0, 1.0, 0.0),
            (1.0, 1.0, 1.0),
            (1.0, 0.0, 1.0),
        ],
        // left (x=0, outward -x)
        [
            (0.0, 1.0, 0.0),
            (0.0, 0.0, 0.0),
            (0.0, 0.0, 1.0),
            (0.0, 1.0, 1.0),
        ],
    ];
    let mut positions = Vec::new();
    let mut indices = Vec::new();
    for face in faces {
        let base = positions.len() / 3;
        for (x, y, z) in face {
            positions.extend_from_slice(&[x, y, z]);
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    (positions, indices)
}

#[test]
fn weld_makes_unwelded_cube_manifold() {
    let (p, i) = unwelded_cube();
    assert!(!is_manifold(&p, &i));
    let out = repair(&p, &i, 0.0);
    assert_eq!(out.report.welded_vertices, 16); // 24 -> 8
    assert!(out.is_manifold(), "{:?}", out.report.residual);
}

#[test]
fn epsilon_weld_merges_nearby_vertices() {
    let (mut p, i) = unwelded_cube();
    // Perturb duplicates slightly.
    for k in (0..p.len()).step_by(3) {
        p[k] += (k as f64 % 7.0) * 1e-7;
    }
    let out = repair(&p, &i, 1e-5);
    assert_eq!(out.positions.len() / 3, 8);
    assert!(out.is_manifold(), "{:?}", out.report.residual);
}

#[test]
fn flipped_triangle_is_fixed() {
    let (p, mut i) = unwelded_cube();
    // Flip one triangle of the top face (indices 6..12 region: second
    // triangle of top face is at positions 9..12 of indices).
    i.swap(10, 11);
    let before = crate::check(&p, &i);
    assert!(!before.orientation_edges.is_empty() || !before.is_manifold());
    let out = repair(&p, &i, 0.0);
    assert!(out.is_manifold(), "{:?}", out.report.residual);
    assert!(out.report.flipped_triangles >= 1);
}

#[test]
fn degenerate_triangles_are_removed() {
    let p = vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
    let i = vec![0, 1, 2, 0, 1, 1];
    let out = repair(&p, &i, 0.0);
    assert_eq!(out.report.removed_degenerate_triangles, 1);
    assert_eq!(out.indices.len(), 3);
}

#[test]
fn triple_edge_is_reported_not_fixed() {
    // Repair must not silently drop faces of a 3-use edge.
    let p = vec![
        0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, -1.0, 0.0,
    ];
    let i = vec![0, 1, 2, 0, 4, 1, 0, 1, 3];
    let out = repair(&p, &i, 0.0);
    assert!(!out.is_manifold());
    assert!(out.report.residual.non_manifold_edges.contains(&(0, 1)));
    assert_eq!(out.indices.len(), 9); // nothing dropped
}

/// Closed, consistently oriented unit cube (8 vertices, 12 triangles).
fn cube_mesh() -> (Vec<f64>, Vec<usize>) {
    let positions = vec![
        0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0, // 0-3
        0.0, 0.0, 1.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 1.0, 1.0, // 4-7
    ];
    let indices = vec![
        0, 2, 1, 0, 3, 2, // bottom
        4, 5, 6, 4, 6, 7, // top
        0, 1, 5, 0, 5, 4, // front
        2, 3, 7, 2, 7, 6, // back
        1, 2, 6, 1, 6, 5, // right
        3, 0, 4, 3, 4, 7, // left
    ];
    (positions, indices)
}

#[test]
fn full_fills_missing_face() {
    let (p, i) = cube_mesh();
    // Drop the top face (indices 6..12): boundary loop of 4 edges.
    let open: Vec<usize> = [i[..6].to_vec(), i[12..].to_vec()].concat();
    let conservative = repair(&p, &open, 0.0);
    assert!(!conservative.is_manifold());
    let out = repair_with_mode(&p, &open, 0.0, RepairMode::Full);
    assert_eq!(out.report.filled_holes, 1, "{:?}", out.report);
    assert_eq!(out.report.filled_triangles, 2);
    assert!(out.is_manifold(), "{:?}", out.report.residual);
}

#[test]
fn full_fills_nonconvex_hole() {
    // 5x5-vertex planar grid (4x4 cells) with an L-shaped interior hole
    // (cells (1,1), (2,1), (1,2) removed; reflex vertex at (2,2)).
    // Boundary = outer frame loop + non-convex hole loop; Full mode
    // closes both into a strictly manifold double-layer solid.
    let mut p = Vec::new();
    for y in 0..5 {
        for x in 0..5 {
            p.extend_from_slice(&[x as f64, y as f64, 0.0]);
        }
    }
    let removed = [(1usize, 1usize), (2, 1), (1, 2)];
    let mut i = Vec::new();
    for y in 0..4 {
        for x in 0..4 {
            if removed.contains(&(x, y)) {
                continue;
            }
            let at = |x, y| y * 5 + x;
            let (a, b, c, d) = (at(x, y), at(x + 1, y), at(x + 1, y + 1), at(x, y + 1));
            i.extend_from_slice(&[a, b, c, a, c, d]);
        }
    }
    let out = repair_with_mode(&p, &i, 0.0, RepairMode::Full);
    // Only the interior hole is closed (8-vertex L-loop -> 6 triangles);
    // the exterior frame contour is intentionally left open.
    assert_eq!(out.report.filled_holes, 1, "{:?}", out.report);
    assert_eq!(out.report.filled_triangles, 6);
    let residual = &out.report.residual;
    assert!(residual.non_manifold_edges.is_empty(), "{residual:?}");
    assert!(residual.orientation_edges.is_empty(), "{residual:?}");
    assert!(residual.is_manifold_with_boundary(), "{residual:?}");
}

#[test]
fn full_splits_pinched_vertex() {
    // Two closed fans sharing only vertex 0 (bowtie): conservative
    // repair keeps the pinch, Full separates it.
    let p = vec![
        0.0, 0.0, 0.0, // 0 shared tip
        1.0, 0.0, 0.0, 0.0, 1.0, 0.0, -1.0, 0.0, 0.0, 0.0, -1.0, 0.0, // fan A ring
        0.0, 0.0, 1.0, 0.0, 0.0, -1.0, // fan B ring
    ];
    // Fan A: double-covered triangle pair (closed link); Fan B: same.
    let i = vec![0, 1, 2, 0, 2, 3, 0, 3, 4, 0, 4, 1, 0, 5, 6, 0, 6, 5];
    let conservative = repair(&p, &i, 0.0);
    assert!(
        !conservative
            .report
            .residual
            .non_manifold_vertices
            .is_empty()
    );
    let out = repair_with_mode(&p, &i, 0.0, RepairMode::Full);
    assert!(out.report.split_vertices >= 1, "{:?}", out.report);
    assert!(
        out.report.residual.non_manifold_vertices.is_empty(),
        "{:?}",
        out.report.residual
    );
}

#[test]
fn full_splits_triple_edge_into_shells() {
    // Two closed double-triangle shells sharing edge (0,1) plus one
    // extra triangle: the 3+-use edge is split into manifold shells.
    let p = vec![
        0.0, 0.0, 0.0, 1.0, 0.0, 0.0, // shared edge endpoints
        0.0, 1.0, 0.0, 0.0, -1.0, 0.0, // shell A sides
        0.0, 0.0, 1.0, // shell B side
    ];
    let i = vec![0, 1, 2, 0, 3, 1, 0, 1, 4];
    let conservative = repair(&p, &i, 0.0);
    assert!(
        conservative
            .report
            .residual
            .non_manifold_edges
            .contains(&(0, 1))
    );
    let out = repair_with_mode(&p, &i, 0.0, RepairMode::Full);
    assert!(out.report.split_vertices >= 2, "{:?}", out.report);
    assert!(
        out.report.residual.non_manifold_edges.is_empty(),
        "{:?}",
        out.report.residual
    );
}
