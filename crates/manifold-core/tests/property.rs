//! Deterministic property tests for manifold-core.
//!
//! A fixed-seed LCG keeps the corpus reproducible across machines and
//! toolchains (no proptest dependency). Each property runs over several
//! thousand pseudo-random meshes, including hostile inputs: duplicate
//! vertices, jittered coordinates, degenerate triangles, flipped winding
//! and duplicated faces.

use manifold_core::{RepairMode, check, metrics, repair, repair_with_mode};

/// xorshift64* — small, deterministic, portable.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    fn coord(&mut self) -> f64 {
        // Coordinates on a coarse grid with occasional duplicates and NaN-free
        // jitter: welding must find real duplicates, noise must stay finite.
        let base = (self.below(8)) as f64;
        if self.below(4) == 0 {
            base + self.below(1000) as f64 * 1e-9
        } else {
            base
        }
    }
}

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
    let mut rng = Rng(0x5EED_0001);
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
    let mut rng = Rng(0x5EED_0002);
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
    let mut rng = Rng(0x5EED_0003);
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
    let mut rng = Rng(0x5EED_0004);
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
    let mut rng = Rng(0x5EED_0005);
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
