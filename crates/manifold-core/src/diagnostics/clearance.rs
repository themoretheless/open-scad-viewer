//! Clearance (gap) between disjoint shells of one mesh.

use super::tri::triangle_distance;
use super::{DiagnosticsError, Mesh, boxes_overlap, triangle_tree};
use std::collections::BTreeMap;

/// Parameters of [`clearance`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClearanceOptions {
    /// Gaps strictly below this distance are reported (model units, `> 0`).
    pub threshold: f64,
    /// Maximum number of triangle pairs from different shells whose padded
    /// boxes overlap; more fails with [`DiagnosticsError::WorkBudgetExceeded`].
    pub max_candidate_pairs: usize,
}

impl Default for ClearanceOptions {
    fn default() -> Self {
        Self {
            threshold: 0.2,
            max_candidate_pairs: 4_000_000,
        }
    }
}

/// Closest approach of two shells.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClearancePair {
    /// Shell ids in face-adjacency order (`component_a < component_b`).
    pub component_a: usize,
    pub component_b: usize,
    /// Exact-up-to-rounding triangle distance; `0.0` when the shells touch
    /// or intersect.
    pub distance: f64,
    pub triangle_a: usize,
    pub triangle_b: usize,
}

/// Shell pairs closer than the threshold.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ClearanceReport {
    /// Connected components of the face adjacency graph.
    pub component_count: usize,
    /// One entry per shell pair closer than the threshold, sorted by shells.
    pub pairs: Vec<ClearancePair>,
    /// Triangle pairs measured.
    pub candidate_pairs: usize,
}

impl ClearanceReport {
    /// Smallest reported gap.
    pub fn min_gap(&self) -> Option<f64> {
        self.pairs.iter().map(|p| p.distance).min_by(f64::total_cmp)
    }

    pub fn below_threshold(&self) -> bool {
        !self.pairs.is_empty()
    }
}

/// Report every pair of shells (face-connected components) that come closer
/// than `options.threshold`, with the closest triangle pair of each.
///
/// Every triangle box is padded by half the threshold, so the broadphase
/// keeps every pair that can be within the threshold and the result is
/// complete for gaps below it.
pub fn clearance(
    positions: &[f64],
    indices: &[usize],
    options: ClearanceOptions,
) -> Result<ClearanceReport, DiagnosticsError> {
    if !options.threshold.is_finite() || options.threshold <= 0.0 {
        return Err(DiagnosticsError::InvalidParameter("threshold"));
    }
    let mesh = Mesh::new(positions, indices)?;
    let labels = crate::component_labels(&mesh.triangles);
    let mut dense = BTreeMap::new();
    let component: Vec<usize> = labels
        .iter()
        .map(|&root| {
            let next = dense.len();
            *dense.entry(root).or_insert(next)
        })
        .collect();
    let mut report = ClearanceReport {
        component_count: dense.len(),
        ..Default::default()
    };
    if report.component_count < 2 {
        return Ok(report);
    }
    let Some(tree) = triangle_tree(&mesh, options.threshold * 0.5, |_| true) else {
        return Ok(report);
    };
    let mut candidates = Vec::new();
    let mut leaves = Vec::new();
    tree.walk(|_, _| true, |item| leaves.push(item));
    for a in leaves {
        tree.walk(
            |min, max| boxes_overlap(min, max, a.min, a.max),
            |b| {
                if component[a.index] < component[b.index]
                    && boxes_overlap(a.min, a.max, b.min, b.max)
                {
                    candidates.push((a.index, b.index));
                }
            },
        );
        if candidates.len() > options.max_candidate_pairs {
            return Err(DiagnosticsError::WorkBudgetExceeded {
                stage: "clearance broadphase",
                budget: options.max_candidate_pairs,
                required: candidates.len(),
            });
        }
    }
    report.candidate_pairs = candidates.len();
    let mut best: BTreeMap<(usize, usize), ClearancePair> = BTreeMap::new();
    for (ta, tb) in candidates {
        let distance = triangle_distance(&mesh.corners(ta), &mesh.corners(tb));
        if distance >= options.threshold {
            continue;
        }
        let key = (component[ta], component[tb]);
        let pair = ClearancePair {
            component_a: key.0,
            component_b: key.1,
            distance,
            triangle_a: ta,
            triangle_b: tb,
        };
        best.entry(key)
            .and_modify(|p| {
                if (distance, ta, tb) < (p.distance, p.triangle_a, p.triangle_b) {
                    *p = pair;
                }
            })
            .or_insert(pair);
    }
    report.pairs = best.into_values().collect();
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::tests::{box_mesh, merge};

    #[test]
    fn reports_gap_between_close_boxes_only() {
        let (p, i) = merge(
            box_mesh([0.; 3], [1.; 3]),
            box_mesh([1.15, 0., 0.], [2., 1., 1.]),
        );
        let (p, i) = merge((p, i), box_mesh([5.; 3], [6.; 3]));
        let r = clearance(
            &p,
            &i,
            ClearanceOptions {
                threshold: 0.2,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(r.component_count, 3);
        assert_eq!(r.pairs.len(), 1);
        let pair = r.pairs[0];
        assert_eq!((pair.component_a, pair.component_b), (0, 1));
        assert!((pair.distance - 0.15).abs() < 1e-9, "{pair:?}");
        assert!((r.min_gap().unwrap() - 0.15).abs() < 1e-9);
        let wide = clearance(
            &p,
            &i,
            ClearanceOptions {
                threshold: 0.1,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(!wide.below_threshold());
    }

    #[test]
    fn edge_to_edge_gap_is_measured() {
        // Second box rotated so only edges face each other diagonally.
        let (p, i) = merge(
            box_mesh([0.; 3], [1.; 3]),
            box_mesh([1.1, 1.1, 0.], [2., 2., 1.]),
        );
        let r = clearance(
            &p,
            &i,
            ClearanceOptions {
                threshold: 0.5,
                ..Default::default()
            },
        )
        .unwrap();
        let expected = (0.1f64 * 0.1 * 2.0).sqrt();
        assert!(
            (r.pairs[0].distance - expected).abs() < 1e-9,
            "{:?}",
            r.pairs
        );
    }

    #[test]
    fn intersecting_shells_have_zero_gap() {
        let (p, i) = merge(box_mesh([0.; 3], [2.; 3]), box_mesh([1.; 3], [3.; 3]));
        let r = clearance(&p, &i, Default::default()).unwrap();
        assert_eq!(r.pairs[0].distance, 0.0);
    }

    #[test]
    fn single_shell_and_bad_parameters() {
        let (p, i) = box_mesh([0.; 3], [1.; 3]);
        assert_eq!(
            clearance(&p, &i, Default::default())
                .unwrap()
                .component_count,
            1
        );
        let bad = ClearanceOptions {
            threshold: 0.0,
            ..Default::default()
        };
        assert_eq!(
            clearance(&p, &i, bad),
            Err(DiagnosticsError::InvalidParameter("threshold"))
        );
        let (p, i) = merge(
            box_mesh([0.; 3], [1.; 3]),
            box_mesh([1.05, 0., 0.], [2., 1., 1.]),
        );
        let tight = ClearanceOptions {
            max_candidate_pairs: 1,
            ..Default::default()
        };
        assert!(matches!(
            clearance(&p, &i, tight),
            Err(DiagnosticsError::WorkBudgetExceeded { .. })
        ));
    }
}
