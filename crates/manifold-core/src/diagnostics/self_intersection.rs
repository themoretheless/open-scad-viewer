//! Self-intersection detection: BVH broadphase plus exact-sign narrowphase.

use super::tri::{edge_neighbours_overlap, triangles_intersect, vertex_neighbours_overlap};
use super::{DiagnosticsError, Mesh, boxes_overlap, triangle_tree};

/// Work limits of [`self_intersections`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelfIntersectionOptions {
    /// Maximum number of box-overlapping triangle pairs the broadphase may
    /// hand to the exact narrowphase; more fails with
    /// [`DiagnosticsError::WorkBudgetExceeded`].
    pub max_candidate_pairs: usize,
    /// Maximum number of intersecting pairs to collect; the scan stops and
    /// sets `truncated` once reached.
    pub max_reported_pairs: usize,
}

impl Default for SelfIntersectionOptions {
    fn default() -> Self {
        Self {
            max_candidate_pairs: 4_000_000,
            max_reported_pairs: 10_000,
        }
    }
}

/// Intersecting triangle pairs of one mesh.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SelfIntersectionReport {
    /// `(i, j)` with `i < j`, sorted. Neighbours that only meet along their
    /// shared edge or corner are excluded.
    pub pairs: Vec<(usize, usize)>,
    /// Box-overlapping pairs tested exactly.
    pub candidate_pairs: usize,
    /// True when `max_reported_pairs` stopped the scan early.
    pub truncated: bool,
}

impl SelfIntersectionReport {
    pub fn has_self_intersections(&self) -> bool {
        !self.pairs.is_empty()
    }
}

/// Find intersecting pairs of non-degenerate triangles.
///
/// Signs come from exact orientation predicates, so touching counts as
/// intersecting. Triangles sharing an edge are reported only when coplanar
/// and folded onto each other; triangles sharing a corner only when they
/// also meet elsewhere. Distinct vertex indices at equal positions are not
/// treated as shared, so weld first (see [`crate::repair`]). Degenerate
/// triangles are skipped; [`crate::check`] reports them.
pub fn self_intersections(
    positions: &[f64],
    indices: &[usize],
    options: SelfIntersectionOptions,
) -> Result<SelfIntersectionReport, DiagnosticsError> {
    let mesh = Mesh::new(positions, indices)?;
    let Some(tree) = triangle_tree(&mesh, 0.0, |_| true) else {
        return Ok(SelfIntersectionReport::default());
    };
    // Broadphase first, so the budget error comes before any exact work.
    let mut candidates = Vec::new();
    let mut leaves = Vec::new();
    tree.walk(|_, _| true, |item| leaves.push(item));
    for a in leaves {
        tree.walk(
            |min, max| boxes_overlap(min, max, a.min, a.max),
            |b| {
                if b.index > a.index && boxes_overlap(a.min, a.max, b.min, b.max) {
                    candidates.push((a.index, b.index));
                }
            },
        );
        if candidates.len() > options.max_candidate_pairs {
            return Err(DiagnosticsError::WorkBudgetExceeded {
                stage: "self-intersection broadphase",
                budget: options.max_candidate_pairs,
                required: candidates.len(),
            });
        }
    }
    candidates.sort_unstable();
    let mut report = SelfIntersectionReport {
        candidate_pairs: candidates.len(),
        ..Default::default()
    };
    for (i, j) in candidates {
        if pair_intersects(&mesh, i, j) {
            if report.pairs.len() == options.max_reported_pairs {
                report.truncated = true;
                break;
            }
            report.pairs.push((i, j));
        }
    }
    Ok(report)
}

fn pair_intersects(mesh: &Mesh<'_>, i: usize, j: usize) -> bool {
    let (ti, tj) = (mesh.triangles[i], mesh.triangles[j]);
    let shared: Vec<usize> = ti.iter().copied().filter(|v| tj.contains(v)).collect();
    let others = |t: [usize; 3]| -> Vec<usize> {
        t.iter().copied().filter(|v| !shared.contains(v)).collect()
    };
    let p = |v: usize| mesh.point(v);
    match shared.len() {
        0 => triangles_intersect(&mesh.corners(i), &mesh.corners(j)),
        1 => {
            let (a, b) = (others(ti), others(tj));
            vertex_neighbours_overlap(p(shared[0]), [p(a[0]), p(a[1])], [p(b[0]), p(b[1])])
        }
        2 => edge_neighbours_overlap(
            p(shared[0]),
            p(shared[1]),
            p(others(ti)[0]),
            p(others(tj)[0]),
        ),
        // Duplicate faces are a topology defect reported by `check`.
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::tests::{box_mesh, merge};

    #[test]
    fn closed_box_has_no_self_intersections() {
        let (p, i) = box_mesh([0.; 3], [1., 2., 3.]);
        let r = self_intersections(&p, &i, Default::default()).unwrap();
        assert!(!r.has_self_intersections(), "{:?}", r.pairs);
        assert!(r.candidate_pairs > 0);
        assert!(!r.truncated);
    }

    #[test]
    fn overlapping_boxes_report_crossing_pairs() {
        let (p, i) = merge(box_mesh([0.; 3], [2.; 3]), box_mesh([1.; 3], [3.; 3]));
        let r = self_intersections(&p, &i, Default::default()).unwrap();
        assert!(r.has_self_intersections());
        // Every reported pair crosses the two shells (12 triangles each).
        assert!(
            r.pairs.iter().all(|&(a, b)| a < 12 && b >= 12),
            "{:?}",
            r.pairs
        );
    }

    #[test]
    fn disjoint_boxes_are_clean() {
        let (p, i) = merge(box_mesh([0.; 3], [1.; 3]), box_mesh([2.; 3], [3.; 3]));
        let r = self_intersections(&p, &i, Default::default()).unwrap();
        assert!(r.pairs.is_empty());
    }

    #[test]
    fn folded_neighbour_is_reported_but_flat_fan_is_not() {
        // Two triangles sharing edge 0-1, the second folded flat onto the first.
        let p = [0., 0., 0., 2., 0., 0., 0., 2., 0., 1., 1., 0.];
        let r = self_intersections(&p, &[0, 1, 2, 1, 0, 3], Default::default()).unwrap();
        assert_eq!(r.pairs, vec![(0, 1)]);
        let flat = [0., 0., 0., 2., 0., 0., 0., 2., 0., 0., -2., 0.];
        let r = self_intersections(&flat, &[0, 1, 2, 1, 0, 3], Default::default()).unwrap();
        assert!(r.pairs.is_empty());
    }

    #[test]
    fn budget_and_truncation_are_typed() {
        let (p, i) = merge(box_mesh([0.; 3], [2.; 3]), box_mesh([1.; 3], [3.; 3]));
        let tight = SelfIntersectionOptions {
            max_candidate_pairs: 3,
            ..Default::default()
        };
        assert!(matches!(
            self_intersections(&p, &i, tight),
            Err(DiagnosticsError::WorkBudgetExceeded { budget: 3, .. })
        ));
        let one = SelfIntersectionOptions {
            max_reported_pairs: 1,
            ..Default::default()
        };
        let r = self_intersections(&p, &i, one).unwrap();
        assert_eq!(r.pairs.len(), 1);
        assert!(r.truncated);
    }

    /// Closed UV sphere with poles: many shared-corner fans and thin slivers.
    fn sphere(center: [f64; 3], r: f64, rings: usize, segs: usize) -> (Vec<f64>, Vec<usize>) {
        let mut p = vec![center[0], center[1], center[2] + r];
        for i in 1..rings {
            let th = std::f64::consts::PI * i as f64 / rings as f64;
            for j in 0..segs {
                let ph = std::f64::consts::TAU * j as f64 / segs as f64;
                p.extend([
                    center[0] + r * th.sin() * ph.cos(),
                    center[1] + r * th.sin() * ph.sin(),
                    center[2] + r * th.cos(),
                ]);
            }
        }
        p.extend([center[0], center[1], center[2] - r]);
        let ring = |i: usize, j: usize| 1 + (i - 1) * segs + j % segs;
        let south = p.len() / 3 - 1;
        let mut t = Vec::new();
        for j in 0..segs {
            t.extend([0, ring(1, j), ring(1, j + 1)]);
            t.extend([south, ring(rings - 1, j + 1), ring(rings - 1, j)]);
        }
        for i in 1..rings - 1 {
            for j in 0..segs {
                let (a, b, c, d) = (
                    ring(i, j),
                    ring(i, j + 1),
                    ring(i + 1, j),
                    ring(i + 1, j + 1),
                );
                t.extend([a, c, d, a, d, b]);
            }
        }
        (p, t)
    }

    #[test]
    fn tessellated_spheres_have_no_false_positives() {
        let (p, i) = sphere([0.; 3], 1.0, 24, 48);
        assert!(crate::check(&p, &i).is_manifold());
        let r = self_intersections(&p, &i, Default::default()).unwrap();
        assert!(r.pairs.is_empty(), "{:?}", &r.pairs[..r.pairs.len().min(5)]);
        let (p, i) = merge(
            sphere([0.; 3], 1.0, 24, 48),
            sphere([1.5, 0., 0.], 1.0, 24, 48),
        );
        let n = 2 * 48 + 22 * 48 * 2;
        let r = self_intersections(&p, &i, Default::default()).unwrap();
        assert!(!r.pairs.is_empty());
        assert!(r.pairs.iter().all(|&(a, b)| a < n && b >= n));
    }

    #[test]
    fn invalid_mesh_is_rejected() {
        assert!(matches!(
            self_intersections(&[0.; 9], &[0, 1, 5], Default::default()),
            Err(DiagnosticsError::InvalidMesh(_))
        ));
    }
}
