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

// ---------------------------------------------------------------------------
// Repair options: budgets, spatial-hash weld, stitching, bounded filling
// ---------------------------------------------------------------------------

fn full(options: RepairOptions) -> RepairOptions {
    RepairOptions {
        mode: RepairMode::Full,
        ..options
    }
}

fn open_cube() -> (Vec<f64>, Vec<usize>) {
    let (p, i) = cube_mesh();
    (p, [i[..6].to_vec(), i[12..].to_vec()].concat())
}

/// Regular octahedron (6 vertices, 8 outward triangles, volume 4/3).
fn octahedron() -> (Vec<f64>, Vec<[usize; 3]>) {
    let p = vec![
        1.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, -1.0,
    ];
    // +x=0 -x=1 +y=2 -y=3 +z=4 -z=5
    let t = vec![
        [0, 2, 4],
        [2, 1, 4],
        [1, 3, 4],
        [3, 0, 4],
        [2, 0, 5],
        [1, 2, 5],
        [3, 1, 5],
        [0, 3, 5],
    ];
    (p, t)
}

fn volume(out: &RepairOutcome) -> f64 {
    crate::metrics(&out.positions, &out.indices).signed_volume
}

#[test]
fn work_budget_exhaustion_is_typed_and_deterministic() {
    let (p, i) = open_cube();
    let options = full(RepairOptions {
        budget: RepairBudget {
            max_work: Some(30),
            ..RepairBudget::default()
        },
        ..RepairOptions::default()
    });
    let a = repair_with_options(&p, &i, &options).unwrap_err();
    let b = repair_with_options(&p, &i, &options).unwrap_err();
    assert_eq!(a.kind, RepairBudgetKind::Work);
    assert_eq!(a.partial.positions, b.partial.positions);
    assert_eq!(a.partial.indices, b.partial.indices);
    assert_eq!(a.partial.report, b.partial.report);
    assert!(a.partial.report.work <= 30);
    assert_eq!(a.partial.report.filled_holes, 0);
    assert!(!a.partial.is_manifold());
    // Unlimited budget reports the work it spent and succeeds.
    let ok = repair_with_options(&p, &i, &full(RepairOptions::default())).unwrap();
    assert!(ok.report.work > 30 && ok.is_manifold());
}

#[test]
fn split_and_fill_budgets_stop_with_partial_report() {
    // Three triangles on edge (0,1): needs a shell split.
    let p = vec![
        0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 1.0,
    ];
    let i = vec![0, 1, 2, 1, 0, 3, 0, 1, 4];
    let options = full(RepairOptions {
        budget: RepairBudget {
            max_split_vertices: Some(0),
            ..RepairBudget::default()
        },
        ..RepairOptions::default()
    });
    let err = repair_with_options(&p, &i, &options).unwrap_err();
    assert_eq!(err.kind, RepairBudgetKind::SplitVertices);
    assert_eq!(err.partial.report.split_vertices, 0);
    assert!(!err.partial.report.residual.non_manifold_edges.is_empty());

    let (p, i) = open_cube();
    let options = full(RepairOptions {
        budget: RepairBudget {
            max_filled_triangles: Some(1),
            ..RepairBudget::default()
        },
        ..RepairOptions::default()
    });
    let err = repair_with_options(&p, &i, &options).unwrap_err();
    assert_eq!(err.kind, RepairBudgetKind::FilledTriangles);
    assert_eq!(err.partial.report.filled_triangles, 0);
    assert_eq!(err.partial.indices.len(), i.len());
}

#[test]
fn spatial_hash_weld_merges_across_cell_boundaries() {
    // Two copies straddle the grid line x = 1e-3 (cells 0 and 1).
    let p = vec![0.000_999_9, 0.0, 0.0, 0.001_000_1, 0.0, 0.0];
    let (out, remap, merged) = weld(&p, 1e-3);
    assert_eq!(merged, 1);
    assert_eq!(remap, vec![0, 0]);
    assert_eq!(out, vec![0.000_999_9, 0.0, 0.0]); // lowest index wins
}

#[test]
fn cracked_cube_with_straddling_duplicates_becomes_manifold() {
    let (mut p, i) = unwelded_cube();
    let eps = 1e-3;
    // Shift x of alternate copies in opposite directions so pairs straddle the
    // grid lines through the integer corners.
    for (k, x) in p.iter_mut().enumerate().step_by(3) {
        *x += if (k / 3) % 2 == 1 { 0.4 } else { -0.4 } * eps;
    }
    let out = repair(&p, &i, eps);
    assert_eq!(out.positions.len() / 3, 8);
    assert!(out.is_manifold(), "{:?}", out.report.residual);
}

#[test]
fn near_duplicate_grid_welds_to_exact_count_deterministically() {
    let n = 40usize;
    let eps = 1e-4;
    let mut p = Vec::new();
    for copy in 0..3 {
        for y in 0..n {
            for x in 0..n {
                let j = copy as f64 * 0.25 * eps; // within eps of copy 0
                p.extend_from_slice(&[x as f64 * 0.01 + j, y as f64 * 0.01 - j, j]);
            }
        }
    }
    let (out, remap, merged) = weld(&p, eps);
    assert_eq!(out.len() / 3, n * n);
    assert_eq!(merged, 2 * n * n);
    // Representatives are the first copy (lowest indices), in order.
    assert_eq!(out, p[..n * n * 3].to_vec());
    for (v, &r) in remap.iter().enumerate() {
        assert_eq!(r, v % (n * n));
    }
    // Small epsilon never merges distinct grid points.
    let (out, _, _) = weld(&p[..n * n * 3], 1e-9);
    assert_eq!(out.len() / 3, n * n);
}

#[test]
fn figure_eight_boundary_is_stitched_without_duplicating_the_pinch() {
    let (p, t) = octahedron();
    // Remove two faces that share only +z (vertex 4): the boundary is a
    // figure-eight through vertex 4.
    let i: Vec<usize> = t
        .iter()
        .enumerate()
        .filter(|(k, _)| *k != 0 && *k != 2)
        .flat_map(|(_, f)| *f)
        .collect();
    assert!(!crate::check(&p, &i).non_manifold_vertices.is_empty());
    let options = full(RepairOptions {
        max_iterations: 3,
        ..RepairOptions::default()
    });
    let out = repair_with_options(&p, &i, &options).unwrap();
    assert!(out.is_manifold(), "{:?}", out.report);
    assert!(out.report.stitched_loops >= 1, "{:?}", out.report);
    assert_eq!(out.report.split_vertices, 0);
    assert_eq!(out.positions.len() / 3, 6);
    assert!((volume(&out) - 4.0 / 3.0).abs() < 1e-12);

    // Without stitching the pinch vertex is duplicated instead.
    let legacy = repair_with_options(
        &p,
        &i,
        &RepairOptions {
            stitch_non_simple_loops: false,
            ..options
        },
    )
    .unwrap();
    assert!(legacy.report.split_vertices >= 1);
    assert_eq!(legacy.report.stitched_loops, 0);
}

#[test]
fn boundary_walk_cuts_self_touching_loops_into_simple_loops() {
    let (_, t) = octahedron();
    let tris: Vec<[usize; 3]> = t
        .iter()
        .enumerate()
        .filter(|(k, _)| *k != 0 && *k != 2)
        .map(|(_, f)| *f)
        .collect();
    let loops = boundary_loops(&tris);
    assert_eq!(loops.len(), 2);
    for (l, touches) in &loops {
        assert_eq!(l.len(), 3);
        assert!(touches);
        let unique: BTreeSet<_> = l.iter().collect();
        assert_eq!(unique.len(), l.len());
    }
}

#[test]
fn hole_limits_refuse_long_large_or_nonplanar_loops() {
    let (p, i) = open_cube();
    let with = |limits: HoleFillLimits| {
        repair_with_options(
            &p,
            &i,
            &full(RepairOptions {
                hole_limits: limits,
                ..RepairOptions::default()
            }),
        )
        .unwrap()
    };
    let refused = with(HoleFillLimits {
        max_loop_vertices: Some(3),
        ..HoleFillLimits::default()
    });
    assert_eq!(
        (refused.report.refused_holes, refused.report.filled_holes),
        (1, 0)
    );
    assert!(!refused.is_manifold());
    let refused = with(HoleFillLimits {
        max_area: Some(0.5),
        ..HoleFillLimits::default()
    });
    assert_eq!(refused.report.refused_holes, 1);
    let filled = with(HoleFillLimits {
        max_loop_vertices: Some(4),
        max_area: Some(1.0),
        max_plane_deviation: Some(1e-9),
    });
    assert_eq!(
        (filled.report.refused_holes, filled.report.filled_holes),
        (0, 1)
    );
    assert!(filled.is_manifold());
    assert!((volume(&filled) - 1.0).abs() < 1e-12);

    // Octahedron minus the two +z faces adjacent along edge (+y,+z):
    // boundary +x,+y,-x,+z is non-planar.
    let (p, t) = octahedron();
    let i: Vec<usize> = t[2..].iter().flatten().copied().collect();
    let run = |dev: f64| {
        repair_with_options(
            &p,
            &i,
            &full(RepairOptions {
                hole_limits: HoleFillLimits {
                    max_plane_deviation: Some(dev),
                    ..HoleFillLimits::default()
                },
                ..RepairOptions::default()
            }),
        )
        .unwrap()
    };
    let refused = run(0.01);
    assert_eq!(refused.report.refused_holes, 1);
    assert!(!refused.is_manifold());
    let filled = run(1.0);
    assert!(filled.is_manifold(), "{:?}", filled.report);
    let v = volume(&filled);
    assert!(v > 0.5 && v <= 4.0 / 3.0 + 1e-12, "volume {v}");
}

#[test]
fn duplicate_faces_are_removed_with_orientation() {
    let (mut p, mut i) = cube_mesh();
    i.extend_from_slice(&[0, 2, 1]); // exact duplicate of the first face
    // A free triangle and its reversed twin cancel.
    p.extend_from_slice(&[5.0, 0.0, 0.0, 6.0, 0.0, 0.0, 5.0, 1.0, 0.0]);
    i.extend_from_slice(&[8, 9, 10, 8, 10, 9]);
    let out = repair_with_options(
        &p,
        &i,
        &RepairOptions {
            remove_duplicate_faces: true,
            ..RepairOptions::default()
        },
    )
    .unwrap();
    assert_eq!(out.report.removed_duplicate_triangles, 3);
    assert_eq!(out.indices.len(), 36);
    assert!(
        crate::check(&out.positions, &out.indices)
            .non_manifold_edges
            .is_empty()
    );
}

#[test]
fn inverted_closed_component_is_oriented_outward() {
    let (p, mut i) = cube_mesh();
    for t in i.chunks_mut(3) {
        t.swap(1, 2);
    }
    let out = repair_with_options(
        &p,
        &i,
        &RepairOptions {
            orient_outward: true,
            ..RepairOptions::default()
        },
    )
    .unwrap();
    assert_eq!(out.report.inverted_components, 1);
    assert!((volume(&out) - 1.0).abs() < 1e-12);
    assert!(out.is_manifold());
}

#[test]
fn iterative_repair_reaches_fixed_point_and_tracks_sources() {
    // Cube without top and bottom: with two loops the first pass skips the
    // exterior-classified loop; later passes close it.
    let (p, i) = cube_mesh();
    let open: Vec<usize> = i[12..].to_vec();
    let once = repair_with_options(&p, &open, &full(RepairOptions::default())).unwrap();
    let iterated = repair_with_options(
        &p,
        &open,
        &full(RepairOptions {
            max_iterations: 5,
            ..RepairOptions::default()
        }),
    )
    .unwrap();
    assert!(iterated.is_manifold(), "{:?}", iterated.report);
    assert!(iterated.report.converged);
    assert!(!iterated.report.oscillation_detected);
    assert!(iterated.report.iterations >= 2 && iterated.report.iterations <= 5);
    assert!(iterated.report.filled_holes >= once.report.filled_holes);
    assert!((volume(&iterated) - 1.0).abs() < 1e-12);
    // Sources: the 8 side triangles keep their input ids; fills are None.
    assert_eq!(iterated.source_triangles.len(), iterated.indices.len() / 3);
    assert_eq!(
        iterated.source_triangles[..8],
        (0..8).map(Some).collect::<Vec<_>>()[..]
    );
    assert!(iterated.source_triangles[8..].iter().all(Option::is_none));
}
