//! QEM (Garland–Heckbert) edge-collapse decimation with topology guards.
//!
//! Every collapse is admitted only when it keeps the mesh a 2-manifold
//! (link condition, minimum valence), flips no triangle and creates no
//! degenerate one. Boundary vertices are locked: an edge with one boundary
//! endpoint collapses onto that endpoint, an edge with two never collapses, so
//! open boundaries are preserved exactly. Collapses stop at the target
//! triangle count or when the cheapest admissible collapse exceeds
//! `max_error`. The result is re-checked with `manifold_core::check`; the
//! report carries the largest accepted quadric error and a sampled two-sided
//! deviation from `mesh_query::proximity::hausdorff_distance_approx`.
//!
//! The existing `polygon_core::lattice_tools::decimate` is a shortest-edge
//! collapse tuned for lattice beams (no quadrics, no error bound), so it is
//! not reused here.

use crate::geom::{P, add, cross, dot, norm, point, scale, sub, tri_normal};
use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;

#[derive(Debug, Clone, PartialEq)]
pub enum DecimateError {
    InvalidInput(String),
    /// Input has non-manifold edges/vertices, inconsistent orientation or degenerate triangles.
    NotManifold,
    /// The output failed the manifold re-check (an internal invariant violation).
    OutputNotManifold,
}

impl std::fmt::Display for DecimateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for DecimateError {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DecimateOptions {
    /// Stop when the triangle count is at or below this.
    pub target_triangles: usize,
    /// Upper bound on the root of a single collapse's quadric error (length units).
    pub max_error: f64,
    /// Compute the sampled deviation between input and output.
    pub measure_deviation: bool,
}

impl Default for DecimateOptions {
    fn default() -> Self {
        Self {
            target_triangles: 0,
            max_error: f64::INFINITY,
            measure_deviation: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DecimateReport {
    pub input_triangles: usize,
    pub output_triangles: usize,
    pub collapses: usize,
    /// Collapses refused by the topology / flip guards.
    pub rejected_collapses: usize,
    /// Largest `sqrt(quadric error)` among accepted collapses.
    pub max_quadric_error: f64,
    /// True when the target count was reached; false when `max_error` or the guards stopped it.
    pub target_reached: bool,
    /// Sampled two-sided deviation (vertices, edge midpoints, centroids; BVH), not a certified Hausdorff bound.
    pub sampled_max_deviation: Option<f64>,
    pub sampled_rms_deviation: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct DecimateOutcome {
    pub positions: Vec<f64>,
    pub indices: Vec<usize>,
    pub report: DecimateReport,
}

/// Symmetric 4x4 quadric, upper triangle row-major.
#[derive(Clone, Copy, Default)]
struct Quadric([f64; 10]);

impl Quadric {
    fn plane(n: P, d: f64, w: f64) -> Self {
        let [a, b, c] = n;
        Self(
            [
                a * a,
                a * b,
                a * c,
                a * d,
                b * b,
                b * c,
                b * d,
                c * c,
                c * d,
                d * d,
            ]
            .map(|v| v * w),
        )
    }
    fn add(self, o: Self) -> Self {
        Self(std::array::from_fn(|k| self.0[k] + o.0[k]))
    }
    fn eval(&self, p: P) -> f64 {
        let q = &self.0;
        let [x, y, z] = p;
        (q[0] * x * x
            + 2.0 * q[1] * x * y
            + 2.0 * q[2] * x * z
            + 2.0 * q[3] * x
            + q[4] * y * y
            + 2.0 * q[5] * y * z
            + 2.0 * q[6] * y
            + q[7] * z * z
            + 2.0 * q[8] * z
            + q[9])
            .max(0.0)
    }
    /// Minimiser of the quadric when the 3x3 system is well conditioned.
    fn optimum(&self) -> Option<P> {
        let q = &self.0;
        let m = [[q[0], q[1], q[2]], [q[1], q[4], q[5]], [q[2], q[5], q[7]]];
        let r = [-q[3], -q[6], -q[8]];
        let det = dot(m[0], cross(m[1], m[2]));
        let scale_ = m.iter().flatten().fold(0.0_f64, |a, v| a.max(v.abs()));
        if det.abs().partial_cmp(&(1e-10 * scale_.powi(3))) != Some(std::cmp::Ordering::Greater) {
            return None;
        }
        let col = |c: usize| [m[0][c], m[1][c], m[2][c]];
        let (c0, c1, c2) = (col(0), col(1), col(2));
        let x = [
            dot(r, cross(c1, c2)) / det,
            dot(c0, cross(r, c2)) / det,
            dot(c0, cross(c1, r)) / det,
        ];
        x.iter().all(|v| v.is_finite()).then_some(x)
    }
}

#[derive(PartialEq)]
struct Cost(f64);
impl Eq for Cost {}
impl PartialOrd for Cost {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Cost {
    fn cmp(&self, o: &Self) -> Ordering {
        self.0.total_cmp(&o.0)
    }
}

struct Mesh {
    pts: Vec<P>,
    faces: Vec<[usize; 3]>,
    alive: Vec<bool>,
    incident: Vec<Vec<usize>>,
    locked: Vec<bool>,
    quadric: Vec<Quadric>,
    version: Vec<u32>,
}

impl Mesh {
    fn neighbours(&self, v: usize) -> Vec<usize> {
        let mut out: Vec<usize> = self.incident[v]
            .iter()
            .flat_map(|&f| self.faces[f])
            .filter(|&w| w != v)
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Target position and cost for collapsing `a`-`b`, or `None` when forbidden by locks.
    fn plan(&self, a: usize, b: usize) -> Option<(f64, P)> {
        let q = self.quadric[a].add(self.quadric[b]);
        let target = match (self.locked[a], self.locked[b]) {
            (true, true) => return None,
            (true, false) => self.pts[a],
            (false, true) => self.pts[b],
            (false, false) => q.optimum().unwrap_or_else(|| {
                let mid = scale(add(self.pts[a], self.pts[b]), 0.5);
                [self.pts[a], self.pts[b], mid]
                    .into_iter()
                    .min_by(|x, y| q.eval(*x).total_cmp(&q.eval(*y)))
                    .unwrap()
            }),
        };
        Some((q.eval(target), target))
    }

    /// Topology and geometry guards; on success performs the collapse of `b` into `a` at `target`.
    fn try_collapse(&mut self, a: usize, b: usize, target: P) -> bool {
        let shared: Vec<usize> = self.incident[a]
            .iter()
            .copied()
            .filter(|f| self.faces[*f].contains(&b))
            .collect();
        let opposite: Vec<usize> = shared
            .iter()
            .map(|&f| {
                self.faces[f]
                    .into_iter()
                    .find(|&w| w != a && w != b)
                    .unwrap()
            })
            .collect();
        let na = self.neighbours(a);
        let nb = self.neighbours(b);
        let common: Vec<usize> = na
            .iter()
            .copied()
            .filter(|w| nb.binary_search(w).is_ok())
            .collect();
        let mut opp_sorted = opposite.clone();
        opp_sorted.sort_unstable();
        // Link condition: the only common neighbours are the apexes of the shared triangles.
        if shared.is_empty() || common != opp_sorted {
            return false;
        }
        // Keep at least valence 3 around the merged vertex and at the apexes.
        if na.len() + nb.len() - 2 - common.len() < 3 {
            return false;
        }
        if opposite.iter().any(|&o| self.neighbours(o).len() <= 3) {
            return false;
        }
        // No flipped or degenerate surviving triangle.
        let scale_ = norm(sub(self.pts[a], self.pts[b])).max(1e-300);
        for &v in &[a, b] {
            for &f in &self.incident[v] {
                if shared.contains(&f) {
                    continue;
                }
                let old = self.faces[f].map(|i| self.pts[i]);
                let new = self.faces[f].map(|i| {
                    if i == a || i == b {
                        target
                    } else {
                        self.pts[i]
                    }
                });
                let n0 = tri_normal(old[0], old[1], old[2]);
                let n1 = tri_normal(new[0], new[1], new[2]);
                if dot(n0, n1) <= 0.0 || norm(n1) <= 1e-12 * scale_ * scale_ {
                    return false;
                }
                // Avoid needle triangles: new normal within ~80 degrees of the old one.
                if dot(n0, n1) < 0.17 * norm(n0) * norm(n1) {
                    return false;
                }
            }
        }
        for &f in &shared {
            self.alive[f] = false;
        }
        for &o in &opposite {
            self.incident[o].retain(|f| !shared.contains(f));
        }
        let moved: Vec<usize> = self.incident[b]
            .iter()
            .copied()
            .filter(|f| !shared.contains(f))
            .collect();
        for &f in &moved {
            for w in self.faces[f].iter_mut() {
                if *w == b {
                    *w = a;
                }
            }
        }
        self.incident[a].retain(|f| !shared.contains(f));
        self.incident[a].extend(moved);
        self.incident[b].clear();
        self.pts[a] = target;
        self.locked[a] |= self.locked[b];
        self.quadric[a] = self.quadric[a].add(self.quadric[b]);
        self.version[a] += 1;
        self.version[b] += 1;
        true
    }
}

/// Decimate a manifold (possibly open) triangle mesh.
pub fn decimate(
    positions: &[f64],
    indices: &[usize],
    options: DecimateOptions,
) -> Result<DecimateOutcome, DecimateError> {
    if !positions.len().is_multiple_of(3) || !indices.len().is_multiple_of(3) {
        return Err(DecimateError::InvalidInput(
            "buffer lengths are not multiples of 3".into(),
        ));
    }
    if positions.iter().any(|v| !v.is_finite()) || indices.iter().any(|&i| i >= positions.len() / 3)
    {
        return Err(DecimateError::InvalidInput(
            "non-finite coordinate or index out of range".into(),
        ));
    }
    if options.max_error.is_nan() || options.max_error < 0.0 {
        return Err(DecimateError::InvalidInput(
            "max_error must be non-negative".into(),
        ));
    }
    let input = manifold_core::check(positions, indices);
    if !input.is_manifold_with_boundary() {
        return Err(DecimateError::NotManifold);
    }
    let n = positions.len() / 3;
    let faces: Vec<[usize; 3]> = indices.chunks(3).map(|t| [t[0], t[1], t[2]]).collect();
    let mut m = Mesh {
        pts: (0..n).map(|i| point(positions, i)).collect(),
        alive: vec![true; faces.len()],
        incident: vec![Vec::new(); n],
        locked: vec![false; n],
        quadric: vec![Quadric::default(); n],
        version: vec![0; n],
        faces,
    };
    for &(a, b) in &input.boundary_edges {
        m.locked[a] = true;
        m.locked[b] = true;
    }
    for (f, t) in m.faces.iter().enumerate() {
        let [a, b, c] = t.map(|i| m.pts[i]);
        let nrm = tri_normal(a, b, c);
        let len = norm(nrm);
        let unit = scale(nrm, 1.0 / len);
        let q = Quadric::plane(unit, -dot(unit, a), len * 0.5);
        for &v in t {
            m.incident[v].push(f);
            m.quadric[v] = m.quadric[v].add(q);
        }
    }
    let mut heap = BinaryHeap::new();
    let push = |m: &Mesh, heap: &mut BinaryHeap<_>, a: usize, b: usize| {
        let (a, b) = (a.min(b), a.max(b));
        if let Some((cost, _)) = m.plan(a, b) {
            heap.push(Reverse((Cost(cost), a, b, m.version[a], m.version[b])));
        }
    };
    for t in &m.faces {
        for k in 0..3 {
            // Interior edges appear in both directions; boundary edges never collapse.
            if t[k] < t[(k + 1) % 3] {
                push(&m, &mut heap, t[k], t[(k + 1) % 3]);
            }
        }
    }
    let mut count = m.faces.len();
    let mut report = DecimateReport {
        input_triangles: count,
        ..Default::default()
    };
    let limit = options.max_error * options.max_error;
    while count > options.target_triangles {
        let Some(Reverse((Cost(cost), a, b, va, vb))) = heap.pop() else {
            break;
        };
        if m.version[a] != va
            || m.version[b] != vb
            || m.incident[a].is_empty()
            || m.incident[b].is_empty()
        {
            continue;
        }
        if cost > limit {
            break;
        }
        let Some((_, target)) = m.plan(a, b) else {
            continue;
        };
        let removed = m.incident[a]
            .iter()
            .filter(|f| m.faces[**f].contains(&b))
            .count();
        // A collapse on a closed component must leave at least a tetrahedron.
        if count - removed < 4 {
            report.rejected_collapses += 1;
            continue;
        }
        // Keep the surviving index at `a` but merge toward whichever end is locked.
        let (keep, gone) = if m.locked[b] && !m.locked[a] {
            (b, a)
        } else {
            (a, b)
        };
        if !m.try_collapse(keep, gone, target) {
            report.rejected_collapses += 1;
            continue;
        }
        count -= removed;
        report.collapses += 1;
        report.max_quadric_error = report.max_quadric_error.max(cost.sqrt());
        for w in m.neighbours(keep) {
            push(&m, &mut heap, keep, w);
        }
    }
    report.target_reached = count <= options.target_triangles;
    // Compact.
    let mut remap = vec![usize::MAX; n];
    let mut out_p = Vec::new();
    let mut out_i = Vec::with_capacity(count * 3);
    for (f, t) in m.faces.iter().enumerate() {
        if !m.alive[f] {
            continue;
        }
        for &v in t {
            if remap[v] == usize::MAX {
                remap[v] = out_p.len() / 3;
                out_p.extend_from_slice(&m.pts[v]);
            }
            out_i.push(remap[v]);
        }
    }
    report.output_triangles = out_i.len() / 3;
    let out_check = manifold_core::check(&out_p, &out_i);
    let ok = if input.is_manifold() {
        out_check.is_manifold()
    } else {
        out_check.is_manifold_with_boundary()
    };
    if !ok || out_check.boundary_edges.len() != input.boundary_edges.len() {
        return Err(DecimateError::OutputNotManifold);
    }
    if options.measure_deviation {
        let a = mesh_topology::MeshView::new(positions, indices);
        let b = mesh_topology::MeshView::new(&out_p, &out_i);
        if let Ok(d) = mesh_query::proximity::hausdorff_distance_approx(&a, &b) {
            report.sampled_max_deviation = Some(d.max_mm);
            report.sampled_rms_deviation = Some(d.rms_mm);
        }
    }
    Ok(DecimateOutcome {
        positions: out_p,
        indices: out_i,
        report,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::fixtures::{area, sphere, volume};

    #[test]
    fn sphere_to_target_keeps_shape_and_manifoldness() {
        let (p, t) = sphere(10.0, 32, 64);
        let v0 = volume(&p, &t);
        let a0 = area(&p, &t);
        let out = decimate(
            &p,
            &t,
            DecimateOptions {
                target_triangles: 800,
                ..Default::default()
            },
        )
        .unwrap();
        let r = &out.report;
        assert!(r.target_reached, "{r:?}");
        assert!(
            r.output_triangles <= 800 && r.output_triangles >= 790,
            "{r:?}"
        );
        assert!(manifold_core::check(&out.positions, &out.indices).is_manifold());
        let v1 = volume(&out.positions, &out.indices);
        let a1 = area(&out.positions, &out.indices);
        assert!((v1 - v0).abs() / v0 < 0.02, "volume {v0} -> {v1}");
        assert!((a1 - a0).abs() / a0 < 0.02, "area {a0} -> {a1}");
        let dev = r.sampled_max_deviation.unwrap();
        assert!(dev > 0.0 && dev < 0.5, "{r:?}");
        assert!(r.max_quadric_error > 0.0);
        assert_eq!(
            manifold_core::check(&out.positions, &out.indices).component_count,
            1
        );
    }

    #[test]
    fn error_bound_stops_early() {
        let (p, t) = sphere(10.0, 24, 48);
        let out = decimate(
            &p,
            &t,
            DecimateOptions {
                target_triangles: 10,
                max_error: 0.05,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(!out.report.target_reached);
        assert!(out.report.max_quadric_error <= 0.05);
        assert!(out.report.output_triangles < t.len() / 3);
        assert!(manifold_core::check(&out.positions, &out.indices).is_manifold());
    }

    #[test]
    fn flat_regions_collapse_losslessly_and_tiny_targets_stop_at_tetrahedron_scale() {
        // Subdivided cube: coplanar faces collapse with zero error.
        let (p, t) = crate::geom::fixtures::cuboid([0.0; 3], [2.0, 3.0, 4.0]);
        let refined = crate::transform::refine_midpoint(&p, &t, 2).unwrap();
        let out = decimate(
            &refined.0,
            &refined.1,
            DecimateOptions {
                target_triangles: 0,
                max_error: 1e-9,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(manifold_core::check(&out.positions, &out.indices).is_manifold());
        assert!((volume(&out.positions, &out.indices) - 24.0).abs() < 1e-9);
        assert!(out.report.output_triangles < refined.1.len() / 3);
        assert!(out.report.output_triangles >= 4);
    }

    #[test]
    fn open_boundary_is_preserved_exactly() {
        // Hemisphere-like open grid: a height field with a fixed rectangular rim.
        let n = 20;
        let mut p = Vec::new();
        for i in 0..=n {
            for j in 0..=n {
                let (x, y) = (i as f64 / n as f64, j as f64 / n as f64);
                p.extend([x, y, 0.1 * (x * 3.0).sin() * (y * 3.0).sin()]);
            }
        }
        let id = |i: usize, j: usize| i * (n + 1) + j;
        let mut t = Vec::new();
        for i in 0..n {
            for j in 0..n {
                t.extend([
                    id(i, j),
                    id(i + 1, j),
                    id(i + 1, j + 1),
                    id(i, j),
                    id(i + 1, j + 1),
                    id(i, j + 1),
                ]);
            }
        }
        let rim: Vec<P> = (0..=n)
            .flat_map(|k| [id(0, k), id(n, k), id(k, 0), id(k, n)])
            .map(|v| point(&p, v))
            .collect();
        let out = decimate(
            &p,
            &t,
            DecimateOptions {
                target_triangles: 200,
                ..Default::default()
            },
        )
        .unwrap();
        let rep = manifold_core::check(&out.positions, &out.indices);
        assert!(rep.is_manifold_with_boundary());
        assert_eq!(rep.boundary_edges.len(), 4 * n);
        for q in rim {
            assert!(
                (0..out.positions.len() / 3).any(|v| point(&out.positions, v) == q),
                "rim vertex moved"
            );
        }
    }

    #[test]
    fn non_manifold_input_is_refused() {
        let (p, mut t) = sphere(1.0, 6, 8);
        t.extend_from_within(..3);
        assert_eq!(
            decimate(&p, &t, DecimateOptions::default()).unwrap_err(),
            DecimateError::NotManifold
        );
    }
}
