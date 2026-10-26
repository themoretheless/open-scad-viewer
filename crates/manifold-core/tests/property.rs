//! Reproducible property tests for manifold-core.
//!
//! Portable xorshift64* streams need no proptest dependency. Fixed seeds are
//! used by default; MANIFOLD_PROPERTY_SEED derives a fresh deterministic stream
//! for every property and is printed on failure. Cases include duplicate
//! vertices, jittered coordinates, degenerate triangles, flipped winding and
//! duplicated faces.

use manifold_core::{RepairMode, check, metrics, repair, repair_with_mode};

#[path = "support/property_rng.rs"]
mod property_rng;
use property_rng::Rng;

/// Random triangle soup over `vertex_count` vertices.
fn random_mesh(
    rng: &mut Rng,
    vertex_count: usize,
    triangle_count: usize,
) -> (Vec<f64>, Vec<usize>) {
    let mut positions = Vec::with_capacity(vertex_count * 3);
    for _ in 0..vertex_count {
        let (x, y, z) = (rng.coord(), rng.coord(), rng.coord());
        positions.extend_from_slice(&[x, y, z]);
    }
    let mut indices = Vec::with_capacity(triangle_count * 3);
    for _ in 0..triangle_count {
        let (a, b, c) = (
            rng.below(vertex_count),
            rng.below(vertex_count),
            rng.below(vertex_count),
        );
        indices.extend_from_slice(&[a, b, c]);
    }
    (positions, indices)
}

#[test]
fn check_never_panics_on_random_soup() {
    let mut rng = Rng::for_property(0x5EED_0001);
    for case in 0..2000 {
        let (vc, tc) = (1 + rng.below(40), rng.below(60));
        let (p, i) = random_mesh(&mut rng, vc, tc);
        let report = check(&p, &i);
        // Report counts are internally consistent.
        assert_eq!(report.vertex_count, p.len() / 3, "case {case}");
        assert_eq!(report.triangle_count, i.len() / 3, "case {case}");
        assert!(
            report.component_count <= report.triangle_count.max(1),
            "case {case}"
        );
        // Predicates agree with the defect lists.
        assert_eq!(
            report.is_manifold(),
            report.boundary_edges.is_empty()
                && report.non_manifold_edges.is_empty()
                && report.orientation_edges.is_empty()
                && report.degenerate_triangles.is_empty()
                && report.non_manifold_vertices.is_empty(),
            "case {case}"
        );
    }
}

#[test]
fn conservative_repair_is_a_fixpoint() {
    let mut rng = Rng::for_property(0x5EED_0002);
    for case in 0..1500 {
        let (vc, tc) = (1 + rng.below(40), rng.below(60));
        let (p, i) = random_mesh(&mut rng, vc, tc);
        let first = repair(&p, &i, 0.0);
        // No degenerate triangles survive repair.
        assert!(
            first.report.residual.degenerate_triangles.is_empty(),
            "case {case}: {:?}",
            first.report.residual
        );
        // A second conservative pass changes nothing — but only where the
        // first pass actually achieved orientation consistency. Conflicted
        // (non-orientable) components are reported in the residual instead
        // of being forced, and may legitimately flip again.
        let second = repair(&first.positions, &first.indices, 0.0);
        assert_eq!(second.report.welded_vertices, 0, "case {case}");
        assert_eq!(second.report.removed_degenerate_triangles, 0, "case {case}");
        if first.report.residual.orientation_edges.is_empty() {
            assert_eq!(second.report.flipped_triangles, 0, "case {case}");
            assert_eq!(second.positions, first.positions, "case {case}");
            assert_eq!(second.indices, first.indices, "case {case}");
        }
    }
}

#[test]
fn full_repair_never_panics_and_keeps_faces() {
    let mut rng = Rng::for_property(0x5EED_0003);
    for _case in 0..1500 {
        let (vc, tc) = (1 + rng.below(40), rng.below(60));
        let (p, i) = random_mesh(&mut rng, vc, tc);
        let out = repair_with_mode(&p, &i, 0.0, RepairMode::Full);
        // Full repair only adds/splits — it never drops non-degenerate faces.
        let input_live = i.len() / 3 - out.report.removed_degenerate_triangles;
        assert!(out.indices.len() / 3 >= input_live);
        assert!(out.report.residual.degenerate_triangles.is_empty());
    }
}

#[test]
fn metrics_are_finite_and_consistent() {
    let mut rng = Rng::for_property(0x5EED_0004);
    for case in 0..1500 {
        let (vc, tc) = (1 + rng.below(40), rng.below(60));
        let (p, i) = random_mesh(&mut rng, vc, tc);
        let m = metrics(&p, &i);
        assert!(
            m.surface_area.is_finite() && m.surface_area >= 0.0,
            "case {case}"
        );
        assert!(m.signed_volume.is_finite(), "case {case}");
        // Whole-mesh χ equals the sum of per-component χ only when
        // components do not share vertices/edges — i.e. on clean manifold
        // meshes. Face-adjacency components of soup can share vertices,
        // which are then legitimately counted per component.
        if check(&p, &i).is_manifold() {
            assert_eq!(
                m.euler_characteristic,
                m.vertex_count as i64 - m.edge_count as i64 + m.triangle_count as i64
                    - isolated_correction(&p, &i),
                "case {case}"
            );
        }
        // Watertight implies every component closed.
        if m.watertight {
            assert!(
                m.components.iter().all(|c| c.boundary_edges == 0),
                "case {case}"
            );
            assert!(
                m.components
                    .iter()
                    .all(|c| c.genus.is_some() || c.euler_characteristic % 2 != 0),
                "case {case}"
            );
        }
        // Component triangle counts sum to the total.
        assert_eq!(
            m.components.iter().map(|c| c.triangle_count).sum::<usize>(),
            m.triangle_count,
            "case {case}"
        );
    }
}

/// Whole-mesh Euler characteristic counts every vertex, including isolated
/// ones; per-component counts exclude them. The difference is the number of
/// isolated vertices.
fn isolated_correction(p: &[f64], i: &[usize]) -> i64 {
    let mut used = vec![false; p.len() / 3];
    for &idx in i {
        used[idx] = true;
    }
    used.iter().filter(|&&u| !u).count() as i64
}

#[test]
fn welded_random_soup_makes_exact_duplicates_merge() {
    let mut rng = Rng::for_property(0x5EED_0005);
    for case in 0..800 {
        let vertex_count = 2 + rng.below(20);
        let tc = 1 + rng.below(30);
        let (p, i) = random_mesh(&mut rng, vertex_count, tc);
        let out = repair(&p, &i, 0.0);
        // Grid coordinates are exact duplicates frequently; welding must not
        // increase the vertex count, and output indices stay in range.
        assert!(out.positions.len() <= p.len(), "case {case}");
        assert!(
            out.indices.iter().all(|&v| v < out.positions.len() / 3),
            "case {case}"
        );
    }
}

fn random_unwelded_cube(rng: &mut Rng, omitted_face: Option<usize>) -> (Vec<f64>, Vec<usize>, f64) {
    #[rustfmt::skip]
    let cube_indices = [
        0, 2, 1, 0, 3, 2,
        4, 5, 6, 4, 6, 7,
        0, 1, 5, 0, 5, 4,
        2, 3, 7, 2, 7, 6,
        1, 2, 6, 1, 6, 5,
        3, 0, 4, 3, 4, 7,
    ];
    let [x, y, z] = [
        rng.below(8) as f64,
        rng.below(8) as f64,
        rng.below(8) as f64,
    ];
    let s = 1.0 + rng.below(8) as f64;
    #[rustfmt::skip]
    let cube = [
        x, y, z, x + s, y, z, x + s, y + s, z, x, y + s, z,
        x, y, z + s, x + s, y, z + s, x + s, y + s, z + s, x, y + s, z + s,
    ];
    let mut positions = Vec::with_capacity(cube_indices.len() * 3);
    let mut indices = Vec::with_capacity(cube_indices.len());
    for (triangle, corners) in cube_indices.as_chunks::<3>().0.iter().enumerate() {
        if omitted_face == Some(triangle / 2) {
            continue;
        }
        // Independent corners force welding to do real work while preserving
        // the oriented surface and the reference triangle order.
        for &corner in corners {
            let index = indices.len();
            indices.push(index);
            positions.extend_from_slice(&cube[corner * 3..corner * 3 + 3]);
        }
    }
    (positions, indices, s)
}

fn triangle_soup_signed_volume(positions: &[f64], indices: &[usize]) -> f64 {
    indices
        .as_chunks::<3>()
        .0
        .iter()
        .map(|t| {
            let a = &positions[t[0] * 3..t[0] * 3 + 3];
            let b = &positions[t[1] * 3..t[1] * 3 + 3];
            let c = &positions[t[2] * 3..t[2] * 3 + 3];
            (a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
                + a[2] * (b[0] * c[1] - b[1] * c[0]))
                / 6.0
        })
        .sum()
}

#[test]
fn full_repair_is_idempotent() {
    let mut rng = Rng::for_property(0x5EED_0006);
    for case in 0..800 {
        let omitted_face = (rng.below(2) == 1).then(|| rng.below(6));
        let (p, i, _) = random_unwelded_cube(&mut rng, omitted_face);
        let first = repair_with_mode(&p, &i, 0.0, RepairMode::Full);
        assert!(first.is_manifold(), "case {case}");
        let second = repair_with_mode(&first.positions, &first.indices, 0.0, RepairMode::Full);
        assert_eq!(second.positions, first.positions, "case {case}");
        assert_eq!(second.indices, first.indices, "case {case}");
    }
}

#[test]
fn exact_weld_preserves_signed_volume() {
    let mut rng = Rng::for_property(0x5EED_0007);
    for case in 0..800 {
        let (positions, indices, size) = random_unwelded_cube(&mut rng, None);
        let before = triangle_soup_signed_volume(&positions, &indices);
        let repaired = repair(&positions, &indices, 0.0);
        let after = metrics(&repaired.positions, &repaired.indices).signed_volume;
        assert!(repaired.is_manifold(), "case {case}");
        let tolerance = 1e-12 * before.abs().max(1.0);
        assert!(
            (after - before).abs() <= tolerance,
            "case {case}: {after} != {before}"
        );
        assert!(
            (after - size * size * size).abs() <= tolerance,
            "case {case}: {after} != {}",
            size * size * size
        );
    }
}
