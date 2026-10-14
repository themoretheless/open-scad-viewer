//! Oriented-bounding-box hierarchy over control nets for broad-phase culling.
//!
//! OBBs are fitted by PCA (covariance eigenvectors from `math-core`), the
//! tree is built by recursive median splits along the longest axis, and pairs
//! of trees are traversed with the 15-axis SAT test from
//! `crate::convex_distance`, yielding leaf-level candidate pairs before
//! expensive operations such as surface–surface intersection.
//!
//! No `unsafe`, no external dependencies beyond `math-core`.
use crate::convex_distance::{Obb, curve_control_cloud, obb_sat, surface_control_cloud};
use crate::curve::Curve;
use crate::foundation::guards::{Budget, BudgetGuard, require_finite_point};
use crate::surface::Surface;
use crate::{Result, check};
use math_core::{dot, eigen, norm, point_moments, sub, unit};

/// Default maximum number of points in a leaf.
pub const DEFAULT_LEAF_SIZE: usize = 8;

/// One hierarchy node; leaves hold `count` points starting at `start` in the
/// tree's permuted point array, internal nodes reference two children.
#[derive(Clone, Debug)]
pub struct ObbNode {
    /// Tight PCA-fitted oriented box over the node's points.
    pub obb: Obb,
    /// Start index into `ObbTree::points` (leaves only).
    pub start: usize,
    /// Point count (leaves only).
    pub count: usize,
    /// Left child index into `ObbTree::nodes`.
    pub left: Option<usize>,
    /// Right child index into `ObbTree::nodes`.
    pub right: Option<usize>,
    /// Sequential leaf rank in build order (meaningful for leaves; used to
    /// suppress geometrically adjacent pairs in self-collision queries).
    pub leaf_rank: usize,
}
impl ObbNode {
    /// Whether this node is a leaf.
    pub fn is_leaf(&self) -> bool {
        self.left.is_none()
    }
}

/// A static OBB tree over a permuted point set; leaves are contiguous spans.
#[derive(Clone, Debug)]
pub struct ObbTree {
    /// Control points, permuted so every leaf occupies a contiguous range.
    pub points: Vec<[f64; 3]>,
    /// All nodes; index 0 is the root.
    pub nodes: Vec<ObbNode>,
    /// Build-time leaf size cap.
    pub leaf_size: usize,
    /// Number of leaf nodes.
    pub leaf_count: usize,
}/// Orthonormal basis fallback when the covariance eigensystem is degenerate
/// (collinear or coincident points).
fn orthonormal_basis(seed: [f64; 3]) -> [[f64; 3]; 3] {
    let axis = if norm(seed) > 1e-14 {
        unit(seed)
    } else {
        [1., 0., 0.]
    };
    // Pick the coordinate direction least aligned with `axis`.
    let mut helper = [0.; 3];
    let min_comp = (0..3)
        .min_by(|&i, &j| axis[i].abs().total_cmp(&axis[j].abs()))
        .unwrap_or(0);
    helper[min_comp] = 1.;
    let mut v = [0.; 3];
    let proj = dot(axis, helper);
    for k in 0..3 {
        v[k] = helper[k] - proj * axis[k];
    }
    let v = unit(v);
    let w = [
        axis[1] * v[2] - axis[2] * v[1],
        axis[2] * v[0] - axis[0] * v[2],
        axis[0] * v[1] - axis[1] * v[0],
    ];
    [axis, v, w]
}

/// Fit a tight OBB around `points` via PCA of the covariance matrix.
pub fn obb_from_points(points: &[[f64; 3]]) -> Result<Obb> {
    check(!points.is_empty(), "OBB fit requires at least one point")?;
    let moments = point_moments(points)?;
    let (values, vectors) = eigen(moments.covariance);
    let mut order = [0usize, 1, 2];
    order.sort_by(|&a, &b| values[b].total_cmp(&values[a]));
    let mut axes: [[f64; 3]; 3] =
        order.map(|i| [vectors[0][i], vectors[1][i], vectors[2][i]]);
    // Degenerate cloud: replace vanishing axes with an orthonormal basis.
    let mut valid = axes.map(|a| norm(a) > 1e-14);
    if values[order[2]].abs() <= 1e-30 * (1. + values[order[0]].abs()) {
        valid[2] = false;
    }
    if !valid.iter().all(|&v| v) {
        axes = orthonormal_basis(axes[0]);
    }
    // Right-handedness.
    let det = axes[0][0] * (axes[1][1] * axes[2][2] - axes[1][2] * axes[2][1])
        - axes[0][1] * (axes[1][0] * axes[2][2] - axes[1][2] * axes[2][0])
        + axes[0][2] * (axes[1][0] * axes[2][1] - axes[1][1] * axes[2][0]);
    if det < 0. {
        axes[2] = [-axes[2][0], -axes[2][1], -axes[2][2]];
    }
    let mut half_extents = [0.; 3];
    let mut center = moments.centroid;
    for i in 0..3 {
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for p in points {
            let d = dot(sub(*p, moments.centroid), axes[i]);
            lo = lo.min(d);
            hi = hi.max(d);
        }
        check(
            lo.is_finite() && hi.is_finite(),
            "OBB extents exceeded numeric range",
        )?;
        half_extents[i] = ((hi - lo) / 2.).max(0.);
        let mid = (hi + lo) / 2.;
        for k in 0..3 {
            center[k] += mid * axes[i][k];
        }
    }
    Ok(Obb {
        center,
        axes,
        half_extents,
    })
}

impl ObbTree {
    /// Build a tree over an arbitrary point set; leaves hold at most
    /// `leaf_size` points (clamped to at least 1).
    pub fn build_points(points: &[[f64; 3]], leaf_size: usize) -> Result<Self> {
        check(!points.is_empty(), "OBB tree requires at least one point")?;
        for p in points {
            require_finite_point(p, "points")?;
        }
        let leaf_size = leaf_size.max(1);
        let mut tree = ObbTree {
            points: Vec::new(),
            nodes: Vec::new(),
            leaf_size,
            leaf_count: 0,
        };
        // Unified depth budget (item 1065) around the recursive build; the
        // median split is balanced, so depth 64 covers 2^64 points.
        let mut guard = Budget::default().guard("obb-tree-build");
        // The working array is permuted in place: splits sort contiguous
        // slices, so every leaf ends up in a contiguous range.
        let mut work: Vec<[f64; 3]> = points.to_vec();
        tree.build_node(&mut work, 0, &mut guard)?;
        tree.points = work;
        Ok(tree)
    }

    /// Build a tree over the control net of a 3D NURBS curve.
    pub fn build_curve(curve: &Curve, leaf_size: usize) -> Result<Self> {
        Self::build_points(&curve_control_cloud(curve)?.points, leaf_size)
    }

    /// Build a tree over the control net of a 3D NURBS surface.
    pub fn build_surface(surface: &Surface, leaf_size: usize) -> Result<Self> {
        Self::build_points(&surface_control_cloud(surface)?.points, leaf_size)
    }

    fn build_node(
        &mut self,
        work: &mut [[f64; 3]],
        base: usize,
        guard: &mut BudgetGuard,
    ) -> Result<usize> {
        guard.enter_depth()?;
        let result = self.build_node_inner(work, base, guard);
        guard.exit_depth();
        result
    }

    fn build_node_inner(
        &mut self,
        work: &mut [[f64; 3]],
        base: usize,
        guard: &mut BudgetGuard,
    ) -> Result<usize> {
        let obb = obb_from_points(work)?;
        let id = self.nodes.len();
        self.nodes.push(ObbNode {
            obb,
            start: 0,
            count: 0,
            left: None,
            right: None,
            leaf_rank: usize::MAX,
        });
        if work.len() <= self.leaf_size {
            let rank = self.leaf_count;
            self.leaf_count += 1;
            self.nodes[id].start = base;
            self.nodes[id].count = work.len();
            self.nodes[id].leaf_rank = rank;
            return Ok(id);
        }
        // Split along the longest OBB axis at the projection median.
        let axis = (0..3)
            .max_by(|&i, &j| {
                obb.half_extents[i].total_cmp(&obb.half_extents[j])
            })
            .unwrap_or(0);
        let dir = obb.axes[axis];
        work.sort_by(|a, b| dot(*a, dir).total_cmp(&dot(*b, dir)));
        let mid = work.len() / 2;
        let (left_slice, right_slice) = work.split_at_mut(mid);
        let left = self.build_node(left_slice, base, guard)?;
        let right = self.build_node(right_slice, base + mid, guard)?;
        self.nodes[id].left = Some(left);
        self.nodes[id].right = Some(right);
        Ok(id)
    }

    /// Points of a leaf node as a contiguous slice.
    pub fn leaf_points(&self, node: usize) -> Result<&[[f64; 3]]> {
        let n = self
            .nodes
            .get(node)
            .ok_or_else(|| crate::input("OBB tree node index out of range"))?;
        check(n.is_leaf(), "OBB tree node is not a leaf")?;
        Ok(&self.points[n.start..n.start + n.count])
    }

    /// Maximum depth of the tree (root depth 0).
    pub fn depth(&self) -> usize {
        fn walk(tree: &ObbTree, node: usize) -> usize {
            let n = &tree.nodes[node];
            match (n.left, n.right) {
                (Some(l), Some(r)) => 1 + walk(tree, l).max(walk(tree, r)),
                _ => 0,
            }
        }
        if self.nodes.is_empty() {
            0
        } else {
            walk(self, 0)
        }
    }
}

/// A candidate pair of interacting leaf patches.
#[derive(Clone, Debug)]
pub struct LeafPair {
    /// Leaf index (into `ObbTree::nodes`) in the first tree.
    pub node_a: usize,
    /// Leaf index in the second tree.
    pub node_b: usize,
}

/// Traverse two trees jointly, pruning with the OBB SAT test, and return the
/// surviving leaf-level candidate pairs. `tolerance` inflates each box
/// symmetrically before the SAT test, so "near" pairs can be kept too.
pub fn traverse(a: &ObbTree, b: &ObbTree, tolerance: f64) -> Result<Vec<LeafPair>> {
    check(
        !a.nodes.is_empty() && !b.nodes.is_empty(),
        "OBB tree traverse requires nonempty trees",
    )?;
    check(
        tolerance.is_finite() && tolerance >= 0.,
        "OBB traverse tolerance must be nonnegative and finite",
    )?;
    let inflate = |obb: &Obb| Obb {
        center: obb.center,
        axes: obb.axes,
        half_extents: [
            obb.half_extents[0] + tolerance,
            obb.half_extents[1] + tolerance,
            obb.half_extents[2] + tolerance,
        ],
    };
    let mut pairs = Vec::new();
    // Unified iteration budget (item 1065) on stack pops: pathological
    // overlapping hierarchies must fail loudly instead of spinning.
    let mut guard = Budget {
        max_iterations: 1_000_000,
        ..Budget::default()
    }
    .guard("obb-tree-traverse");
    let mut stack = vec![(0usize, 0usize)];
    while let Some((ia, ib)) = stack.pop() {
        guard.tick()?;
        let na = &a.nodes[ia];
        let nb = &b.nodes[ib];
        let oa = inflate(&na.obb);
        let ob = inflate(&nb.obb);
        if obb_sat(&oa, &ob).separated {
            continue;
        }
        match (na.is_leaf(), nb.is_leaf()) {
            (true, true) => pairs.push(LeafPair {
                node_a: ia,
                node_b: ib,
            }),
            (true, false) => {
                if let Some(r) = nb.right {
                    stack.push((ia, r));
                }
                if let Some(l) = nb.left {
                    stack.push((ia, l));
                }
            }
            (false, true) => {
                if let Some(r) = na.right {
                    stack.push((r, ib));
                }
                if let Some(l) = na.left {
                    stack.push((l, ib));
                }
            }
            (false, false) => {
                // Descend the larger box.
                let va = na.obb.half_extents.iter().product::<f64>();
                let vb = nb.obb.half_extents.iter().product::<f64>();
                if va >= vb {
                    if let Some(r) = na.right {
                        stack.push((r, ib));
                    }
                    if let Some(l) = na.left {
                        stack.push((l, ib));
                    }
                } else {
                    if let Some(r) = nb.right {
                        stack.push((ia, r));
                    }
                    if let Some(l) = nb.left {
                        stack.push((ia, l));
                    }
                }
            }
        }
    }
    Ok(pairs)
}

/// A candidate pair of leaf-level control-point patches (cloned).
#[derive(Clone, Debug)]
pub struct PatchPair {
    /// Control points of the leaf from the first surface.
    pub a: Vec<[f64; 3]>,
    /// Control points of the leaf from the second surface.
    pub b: Vec<[f64; 3]>,
}

/// Pre-SSI culling: leaf-level candidate patch pairs between two surfaces.
/// Only pairs whose (tolerance-inflated) OBBs overlap are returned.
pub fn candidate_patch_pairs(
    surf_a: &Surface,
    surf_b: &Surface,
    leaf_size: usize,
    tolerance: f64,
) -> Result<Vec<PatchPair>> {
    let ta = ObbTree::build_surface(surf_a, leaf_size)?;
    let tb = ObbTree::build_surface(surf_b, leaf_size)?;
    let mut out = Vec::new();
    for pair in traverse(&ta, &tb, tolerance)? {
        out.push(PatchPair {
            a: ta.leaf_points(pair.node_a)?.to_vec(),
            b: tb.leaf_points(pair.node_b)?.to_vec(),
        });
    }
    Ok(out)
}

/// Self-collision pre-filter: pairs of *non-adjacent* leaves of one surface
/// whose OBBs overlap. Adjacency is approximated by consecutive leaf ranks
/// (build-order neighbours), which suppresses the trivially touching patches
/// of a regular net while keeping genuinely distant self-approaches.
pub fn self_collision_candidates(
    surf: &Surface,
    leaf_size: usize,
    tolerance: f64,
) -> Result<Vec<PatchPair>> {
    let tree = ObbTree::build_surface(surf, leaf_size)?;
    let leaves: Vec<usize> = (0..tree.nodes.len())
        .filter(|&i| tree.nodes[i].is_leaf())
        .collect();
    let inflate = |obb: &Obb| Obb {
        center: obb.center,
        axes: obb.axes,
        half_extents: [
            obb.half_extents[0] + tolerance,
            obb.half_extents[1] + tolerance,
            obb.half_extents[2] + tolerance,
        ],
    };
    let mut out = Vec::new();
    for (x, &i) in leaves.iter().enumerate() {
        for &j in leaves.iter().skip(x + 1) {
            let ra = tree.nodes[i].leaf_rank;
            let rb = tree.nodes[j].leaf_rank;
            if ra.abs_diff(rb) <= 1 {
                continue; // adjacent patches of the net
            }
            let oa = inflate(&tree.nodes[i].obb);
            let ob = inflate(&tree.nodes[j].obb);
            if obb_sat(&oa, &ob).separated {
                continue;
            }
            out.push(PatchPair {
                a: tree.leaf_points(i)?.to_vec(),
                b: tree.leaf_points(j)?.to_vec(),
            });
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(nx: usize, ny: usize, z: f64) -> Vec<[f64; 3]> {
        let mut pts = Vec::new();
        for i in 0..nx {
            for j in 0..ny {
                pts.push([i as f64, j as f64, z]);
            }
        }
        pts
    }

    fn surface_from(points: &[[f64; 3]], nx: usize, ny: usize) -> Surface {
        let rows: Vec<Vec<Vec<f64>>> = (0..nx)
            .map(|i| {
                (0..ny)
                    .map(|j| points[i * ny + j].to_vec())
                    .collect()
            })
            .collect();
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: rows,
            weights: vec![vec![1.; ny]; nx],
            periodic_u: false,
            periodic_v: false,
        }
    }

    #[test]
    fn build_rejects_nan_point_with_param_and_index() {
        let pts = vec![[0., 0., 0.], [1., f64::NAN, 2.]];
        let err = ObbTree::build_points(&pts, 4).unwrap_err();
        assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
        assert!(err.contains("points[1]"), "{err}");
    }

    #[test]
    fn build_depth_budget_guard_stays_balanced() -> Result<()> {
        // Depth budget 64 covers any realistic point count; a moderately
        // large build must not trip the guard.
        let pts = grid(64, 64, 0.);
        let tree = ObbTree::build_points(&pts, 2)?;
        assert!(tree.depth() < 64);
        Ok(())
    }

    #[test]
    fn obb_contains_all_points() -> Result<()> {
        let pts = grid(4, 3, 2.);
        let obb = obb_from_points(&pts)?;
        for p in &pts {
            let d = sub(*p, obb.center);
            for i in 0..3 {
                let proj = dot(d, obb.axes[i]);
                assert!(
                    proj.abs() <= obb.half_extents[i] + 1e-9,
                    "point {p:?} outside axis {i}: {proj} > {}",
                    obb.half_extents[i]
                );
            }
        }
        Ok(())
    }

    #[test]
    fn obb_axes_are_unit_and_extent_tight() -> Result<()> {
        let pts = grid(5, 2, 0.);
        let obb = obb_from_points(&pts)?;
        for axis in obb.axes {
            assert!((norm(axis) - 1.).abs() < 1e-9);
        }
        // Long axis spans x in [0,4]: half extent 2.
        let max_half = obb
            .half_extents
            .iter()
            .copied()
            .fold(0., f64::max);
        assert!((max_half - 2.).abs() < 1e-9, "{}", max_half);
        Ok(())
    }

    #[test]
    fn obb_degenerate_collinear_points() -> Result<()> {
        let pts = vec![[0., 0., 0.], [1., 0., 0.], [2., 0., 0.]];
        let obb = obb_from_points(&pts)?;
        assert!(obb.half_extents.iter().all(|h| h.is_finite()));
        for axis in obb.axes {
            assert!((norm(axis) - 1.).abs() < 1e-6);
        }
        Ok(())
    }

    #[test]
    fn tree_balanced_depth() -> Result<()> {
        let pts = grid(16, 16, 0.);
        let tree = ObbTree::build_points(&pts, 4)?;
        let d = tree.depth();
        // 256 points, leaf cap 4 -> 64 leaves, balanced depth ~6.
        assert!(d >= 6 && d <= 8, "depth {d}");
        // Leaves partition the point set.
        let mut total = 0;
        for n in &tree.nodes {
            if n.is_leaf() {
                total += n.count;
            }
        }
        assert_eq!(total, pts.len());
        Ok(())
    }

    #[test]
    fn leaf_points_are_contiguous_and_complete() -> Result<()> {
        let pts = grid(8, 8, 1.);
        let tree = ObbTree::build_points(&pts, 4)?;
        let mut seen = vec![false; pts.len()];
        for i in 0..tree.nodes.len() {
            if tree.nodes[i].is_leaf() {
                for p in tree.leaf_points(i)? {
                    let idx = pts.iter().position(|q| q == p).unwrap();
                    seen[idx] = true;
                }
            }
        }
        assert!(seen.iter().all(|&s| s));
        Ok(())
    }

    #[test]
    fn traverse_far_surfaces_yields_no_candidates() -> Result<()> {
        let a = surface_from(&grid(4, 4, 0.), 4, 4);
        let b = surface_from(&grid(4, 4, 100.), 4, 4);
        let pairs = candidate_patch_pairs(&a, &b, 4, 0.)?;
        assert!(pairs.is_empty());
        Ok(())
    }

    #[test]
    fn traverse_close_surfaces_yields_candidates() -> Result<()> {
        let a = surface_from(&grid(6, 6, 0.), 6, 6);
        let b = surface_from(&grid(6, 6, 0.1), 6, 6);
        let pairs = candidate_patch_pairs(&a, &b, 4, 0.05)?;
        assert!(!pairs.is_empty());
        Ok(())
    }

    #[test]
    fn traverse_is_complete_for_true_close_pairs() -> Result<()> {
        // Completeness: every leaf pair whose true point distance is below
        // the gap must be reported by the traversal.
        let pts_a = grid(6, 6, 0.);
        let pts_b = grid(6, 6, 0.2);
        let a = surface_from(&pts_a, 6, 6);
        let b = surface_from(&pts_b, 6, 6);
        let ta = ObbTree::build_surface(&a, 4)?;
        let tb = ObbTree::build_surface(&b, 4)?;
        let pairs = traverse(&ta, &tb, 0.15)?;
        let leaves_a: Vec<usize> = (0..ta.nodes.len())
            .filter(|&i| ta.nodes[i].is_leaf())
            .collect();
        let leaves_b: Vec<usize> = (0..tb.nodes.len())
            .filter(|&i| tb.nodes[i].is_leaf())
            .collect();
        let reported: std::collections::HashSet<(usize, usize)> =
            pairs.iter().map(|p| (p.node_a, p.node_b)).collect();
        for &i in &leaves_a {
            for &j in &leaves_b {
                let close = ta.leaf_points(i)?.iter().any(|pa| {
                    tb.leaf_points(j)
                        .unwrap()
                        .iter()
                        .any(|pb| norm(sub(*pa, *pb)) < 0.25)
                });
                if close {
                    assert!(
                        reported.contains(&(i, j)),
                        "true close pair ({i},{j}) was pruned"
                    );
                }
            }
        }
        Ok(())
    }

    #[test]
    fn self_collision_flat_sheet_has_no_distant_candidates() -> Result<()> {
        // A flat regular sheet: only build-order-adjacent leaves touch, so
        // the distant-pair filter returns nothing.
        let a = surface_from(&grid(8, 8, 0.), 8, 8);
        let pairs = self_collision_candidates(&a, 4, 0.)?;
        assert!(pairs.is_empty(), "{} unexpected pairs", pairs.len());
        Ok(())
    }

    #[test]
    fn self_collision_folded_sheet_detected() -> Result<()> {
        // Folded sheet: rows 8..11 bend back over rows 4..7 (same x range,
        // z gap 0.01), so distant-in-build-order leaves approach closely.
        let mut pts = Vec::new();
        for i in 0..12 {
            for j in 0..4 {
                let (x, z) = if i < 8 {
                    (i as f64, 0.)
                } else {
                    ((15 - i) as f64, 0.01)
                };
                pts.push([x, j as f64, z]);
            }
        }
        let a = surface_from(&pts, 12, 4);
        let pairs = self_collision_candidates(&a, 2, 0.02)?;
        assert!(!pairs.is_empty(), "fold not detected");
        Ok(())
    }
}
