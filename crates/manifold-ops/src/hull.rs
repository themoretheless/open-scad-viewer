//! 3D convex hull (quickhull) with exact orientation signs.
//!
//! Visibility of a point from a face is decided by the exact `orient3d`
//! predicate of `manifold-core`, so the visible region is always a connected
//! disk and the horizon a simple cycle, independently of rounding. Points on
//! a face plane (or on an edge) are never visible and never become vertices:
//! the output holds only strictly extreme points (coplanar facets are
//! triangulated). The output is a closed, outward-oriented 2-manifold, checked
//! with `manifold_core::check` before it is returned.
//!
//! Lower-dimensional inputs (all points equal, collinear or coplanar) have no
//! closed hull and are refused with [`HullError::Degenerate`].
//!
//! Two entry points share one output contract (item 48): [`hull_points`] for
//! a point cloud (optional weld first) and [`hull_mesh`] for a manifold mesh.

use crate::geom::{P, cross, dot, norm, point, sub};
use manifold_core::orient3d;
use std::cmp::Ordering;
use std::collections::HashMap;

/// Why the input has no closed hull.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HullDegeneracy {
    /// Fewer than four distinct points or all points coincide.
    TooFewPoints,
    /// All distinct points lie on one line.
    Collinear,
    /// All distinct points lie in one plane.
    Coplanar,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HullError {
    /// Non-finite coordinates, malformed buffers or bad options.
    InvalidInput(String),
    Degenerate(HullDegeneracy),
    /// `hull_mesh` input is not a strict 2-manifold.
    NotManifold,
    /// More points or iterations than the configured budget.
    BudgetExceeded {
        budget: usize,
        required: usize,
    },
}

impl std::fmt::Display for HullError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for HullError {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HullOptions {
    /// Points closer than this (grid-clustered) are welded before the hull;
    /// `0` welds exact duplicates only.
    pub weld_epsilon: f64,
    /// Maximum number of distinct input points.
    pub max_points: usize,
}

impl Default for HullOptions {
    fn default() -> Self {
        Self {
            weld_epsilon: 0.0,
            max_points: 1_000_000,
        }
    }
}

/// Hull metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HullReport {
    pub input_points: usize,
    /// After exact deduplication / weld.
    pub distinct_points: usize,
    pub hull_vertices: usize,
    pub hull_triangles: usize,
    /// Points added to the hull (quickhull iterations).
    pub iterations: usize,
    /// Exact orientation predicate evaluations.
    pub predicate_calls: usize,
}

/// Closed outward-oriented hull mesh.
#[derive(Debug, Clone)]
pub struct HullOutcome {
    pub positions: Vec<f64>,
    pub indices: Vec<usize>,
    /// For each hull vertex, the index of an input point it came from.
    pub source_points: Vec<usize>,
    pub report: HullReport,
}

/// Hull of a point cloud given as `xyz` triples.
pub fn hull_points(positions: &[f64], options: HullOptions) -> Result<HullOutcome, HullError> {
    if !positions.len().is_multiple_of(3) {
        return Err(HullError::InvalidInput(
            "positions length is not a multiple of 3".into(),
        ));
    }
    if positions.iter().any(|v| !v.is_finite()) {
        return Err(HullError::InvalidInput("non-finite coordinate".into()));
    }
    if !options.weld_epsilon.is_finite() || options.weld_epsilon < 0.0 {
        return Err(HullError::InvalidInput(
            "weld epsilon must be finite and non-negative".into(),
        ));
    }
    let ids: Vec<usize> = (0..positions.len() / 3).collect();
    hull_of(positions, &ids, options)
}

/// Hull of a strict 2-manifold mesh (only referenced vertices count).
pub fn hull_mesh(
    positions: &[f64],
    indices: &[usize],
    options: HullOptions,
) -> Result<HullOutcome, HullError> {
    if !positions.len().is_multiple_of(3) || !indices.len().is_multiple_of(3) {
        return Err(HullError::InvalidInput(
            "buffer lengths are not multiples of 3".into(),
        ));
    }
    let n = positions.len() / 3;
    if indices.iter().any(|&i| i >= n) || positions.iter().any(|v| !v.is_finite()) {
        return Err(HullError::InvalidInput(
            "index out of range or non-finite coordinate".into(),
        ));
    }
    if !manifold_core::check(positions, indices).is_manifold() {
        return Err(HullError::NotManifold);
    }
    let mut ids: Vec<usize> = indices.to_vec();
    ids.sort_unstable();
    ids.dedup();
    hull_of(positions, &ids, options)
}

fn weld(positions: &[f64], ids: &[usize], eps: f64) -> Vec<usize> {
    // Exact dedup by bit pattern (with -0 == 0), then optional grid cluster.
    let key = |p: P| p.map(|v| if v == 0.0 { 0u64 } else { v.to_bits() });
    let mut seen = HashMap::new();
    let mut out = Vec::new();
    for &i in ids {
        if seen.insert(key(point(positions, i)), i).is_none() {
            out.push(i);
        }
    }
    if eps == 0.0 {
        return out;
    }
    let cell = |p: P| p.map(|v| (v / eps).floor() as i64);
    let mut grid: HashMap<[i64; 3], Vec<usize>> = HashMap::new();
    let mut kept = Vec::new();
    'next: for i in out {
        let p = point(positions, i);
        let c = cell(p);
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    if let Some(list) = grid.get(&[c[0] + dx, c[1] + dy, c[2] + dz])
                        && list
                            .iter()
                            .any(|&j| norm(sub(point(positions, j), p)) <= eps)
                        {
                            continue 'next;
                        }
                }
            }
        }
        grid.entry(c).or_default().push(i);
        kept.push(i);
    }
    kept
}

struct Face {
    v: [usize; 3],
    alive: bool,
    outside: Vec<usize>,
}

struct State<'a> {
    pts: &'a [P],
    faces: Vec<Face>,
    edges: HashMap<(usize, usize), usize>,
    calls: usize,
}

impl State<'_> {
    fn sign(&mut self, f: [usize; 3], p: usize) -> Ordering {
        self.calls += 1;
        orient3d(self.pts[f[0]], self.pts[f[1]], self.pts[f[2]], self.pts[p])
    }
    fn add_face(&mut self, v: [usize; 3]) -> usize {
        let id = self.faces.len();
        for k in 0..3 {
            self.edges.insert((v[k], v[(k + 1) % 3]), id);
        }
        self.faces.push(Face {
            v,
            alive: true,
            outside: Vec::new(),
        });
        id
    }
    fn distance(&self, f: usize, p: usize) -> f64 {
        let [a, b, c] = self.faces[f].v.map(|i| self.pts[i]);
        let n = cross(sub(b, a), sub(c, a));
        dot(n, sub(self.pts[p], a)) / norm(n).max(f64::MIN_POSITIVE)
    }
}

fn collinear(a: P, b: P, c: P) -> bool {
    // Exactly collinear iff the 2D orientation vanishes in all three projections.
    (0..3).all(|axis| manifold_core::orient2d(a, b, c, axis) == Ordering::Equal)
}

fn hull_of(
    positions: &[f64],
    ids: &[usize],
    options: HullOptions,
) -> Result<HullOutcome, HullError> {
    let input_points = ids.len();
    let ids = weld(positions, ids, options.weld_epsilon);
    if ids.len() > options.max_points {
        return Err(HullError::BudgetExceeded {
            budget: options.max_points,
            required: ids.len(),
        });
    }
    let pts: Vec<P> = ids.iter().map(|&i| point(positions, i)).collect();
    let n = pts.len();
    if n < 4 {
        return Err(HullError::Degenerate(if n < 3 {
            HullDegeneracy::TooFewPoints
        } else if collinear(pts[0], pts[1], pts[2]) {
            HullDegeneracy::Collinear
        } else {
            HullDegeneracy::Coplanar
        }));
    }
    // Initial simplex: extreme pair, farthest from their line, farthest from that plane;
    // float choices are confirmed (or replaced) by exact tests.
    let i0 = (0..n)
        .min_by(|&a, &b| pts[a].partial_cmp(&pts[b]).unwrap())
        .unwrap();
    let i1 = (0..n)
        .max_by(|&a, &b| norm(sub(pts[a], pts[i0])).total_cmp(&norm(sub(pts[b], pts[i0]))))
        .unwrap();
    if i1 == i0 {
        return Err(HullError::Degenerate(HullDegeneracy::TooFewPoints));
    }
    let line = sub(pts[i1], pts[i0]);
    let off_line = |k: usize| norm(cross(line, sub(pts[k], pts[i0])));
    let mut i2 = (0..n)
        .max_by(|&a, &b| off_line(a).total_cmp(&off_line(b)))
        .unwrap();
    if collinear(pts[i0], pts[i1], pts[i2]) {
        i2 = (0..n)
            .find(|&k| !collinear(pts[i0], pts[i1], pts[k]))
            .ok_or(HullError::Degenerate(HullDegeneracy::Collinear))?;
    }
    let plane = cross(line, sub(pts[i2], pts[i0]));
    let off_plane = |k: usize| dot(plane, sub(pts[k], pts[i0])).abs();
    let mut i3 = (0..n)
        .max_by(|&a, &b| off_plane(a).total_cmp(&off_plane(b)))
        .unwrap();
    let mut calls = 1;
    if orient3d(pts[i0], pts[i1], pts[i2], pts[i3]) == Ordering::Equal {
        i3 = (0..n)
            .find(|&k| {
                calls += 1;
                orient3d(pts[i0], pts[i1], pts[i2], pts[k]) != Ordering::Equal
            })
            .ok_or(HullError::Degenerate(HullDegeneracy::Coplanar))?;
    }
    let mut st = State {
        pts: &pts,
        faces: Vec::new(),
        edges: HashMap::new(),
        calls,
    };
    // Orient so the base triangle sees the apex on its negative (inner) side.
    let (b0, b1, b2) = if st.sign([i0, i1, i2], i3) == Ordering::Greater {
        (i0, i2, i1)
    } else {
        (i0, i1, i2)
    };
    for f in [[b0, b1, b2], [b0, i3, b1], [b1, i3, b2], [b2, i3, b0]] {
        st.add_face(f);
    }
    for p in 0..n {
        if [i0, i1, i2, i3].contains(&p) {
            continue;
        }
        for f in 0..4 {
            if st.sign(st.faces[f].v, p) == Ordering::Greater {
                st.faces[f].outside.push(p);
                break;
            }
        }
    }
    let mut iterations = 0;
    let mut cursor = 0;
    while cursor < st.faces.len() {
        if !st.faces[cursor].alive || st.faces[cursor].outside.is_empty() {
            cursor += 1;
            continue;
        }
        let start = cursor;
        let apex = *st.faces[start]
            .outside
            .iter()
            .max_by(|&&a, &&b| st.distance(start, a).total_cmp(&st.distance(start, b)))
            .unwrap();
        iterations += 1;
        // Visible region by flood fill over edge adjacency.
        let mut visible = vec![start];
        let mut is_visible: HashMap<usize, bool> = HashMap::from([(start, true)]);
        let mut horizon = Vec::new();
        let mut k = 0;
        while k < visible.len() {
            let f = visible[k];
            k += 1;
            let v = st.faces[f].v;
            for e in 0..3 {
                let (a, b) = (v[e], v[(e + 1) % 3]);
                let g = st.edges[&(b, a)];
                let vis = match is_visible.get(&g) {
                    Some(&s) => s,
                    None => {
                        let s = st.sign(st.faces[g].v, apex) == Ordering::Greater;
                        is_visible.insert(g, s);
                        if s {
                            visible.push(g);
                        }
                        s
                    }
                };
                if !vis {
                    horizon.push((a, b));
                }
            }
        }
        let mut orphans = Vec::new();
        for &f in &visible {
            st.faces[f].alive = false;
            orphans.append(&mut st.faces[f].outside);
            let v = st.faces[f].v;
            for e in 0..3 {
                st.edges.remove(&(v[e], v[(e + 1) % 3]));
            }
        }
        let new_faces: Vec<usize> = horizon
            .iter()
            .map(|&(a, b)| st.add_face([a, b, apex]))
            .collect();
        for p in orphans {
            if p == apex {
                continue;
            }
            for &f in &new_faces {
                if st.sign(st.faces[f].v, p) == Ordering::Greater {
                    st.faces[f].outside.push(p);
                    break;
                }
            }
        }
    }
    // Compact.
    let mut remap = vec![usize::MAX; n];
    let mut out_positions = Vec::new();
    let mut source_points = Vec::new();
    let mut indices = Vec::new();
    for f in st.faces.iter().filter(|f| f.alive) {
        for &v in &f.v {
            if remap[v] == usize::MAX {
                remap[v] = source_points.len();
                source_points.push(ids[v]);
                out_positions.extend_from_slice(&pts[v]);
            }
            indices.push(remap[v]);
        }
    }
    debug_assert!(manifold_core::check(&out_positions, &indices).is_manifold());
    let report = HullReport {
        input_points,
        distinct_points: n,
        hull_vertices: source_points.len(),
        hull_triangles: indices.len() / 3,
        iterations,
        predicate_calls: st.calls,
    };
    Ok(HullOutcome {
        positions: out_positions,
        indices,
        source_points,
        report,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::fixtures::{Rng, volume};
    use std::collections::BTreeSet;

    fn brute_force_vertices(pts: &[P]) -> BTreeSet<usize> {
        let n = pts.len();
        let mut out = BTreeSet::new();
        for i in 0..n {
            for j in i + 1..n {
                for k in j + 1..n {
                    let signs: Vec<Ordering> = (0..n)
                        .filter(|&m| m != i && m != j && m != k)
                        .map(|m| orient3d(pts[i], pts[j], pts[k], pts[m]))
                        .collect();
                    if signs.iter().all(|s| *s == Ordering::Less)
                        || signs.iter().all(|s| *s == Ordering::Greater)
                    {
                        out.extend([i, j, k]);
                    }
                }
            }
        }
        out
    }

    fn assert_supporting(out: &HullOutcome, all: &[f64]) {
        let n = all.len() / 3;
        for t in out.indices.chunks(3) {
            let [a, b, c] = [t[0], t[1], t[2]].map(|i| point(&out.positions, i));
            for m in 0..n {
                assert_ne!(
                    orient3d(a, b, c, point(all, m)),
                    Ordering::Greater,
                    "point outside a hull face"
                );
            }
        }
        let report = manifold_core::check(&out.positions, &out.indices);
        assert!(report.is_manifold(), "{report:?}");
        assert!(volume(&out.positions, &out.indices) > 0.0);
    }

    #[test]
    fn random_clouds_match_brute_force() {
        let mut rng = Rng(0x9e3779b97f4a7c15);
        for case in 0..25 {
            let n = 8 + case;
            let flat: Vec<f64> = (0..3 * n).map(|_| rng.next() * 10.0 - 5.0).collect();
            let pts: Vec<P> = (0..n).map(|i| point(&flat, i)).collect();
            let out = hull_points(&flat, HullOptions::default()).unwrap();
            assert_supporting(&out, &flat);
            let got: BTreeSet<usize> = out.source_points.iter().copied().collect();
            assert_eq!(got, brute_force_vertices(&pts), "case {case}");
            assert_eq!(out.report.hull_triangles, 2 * out.report.hull_vertices - 4);
        }
    }

    #[test]
    fn grid_with_duplicates_and_coplanar_faces() {
        // 4x4x4 lattice, every point twice: many coplanar and collinear points on the hull.
        let mut flat = Vec::new();
        for _ in 0..2 {
            for x in 0..4 {
                for y in 0..4 {
                    for z in 0..4 {
                        flat.extend([x as f64, y as f64, z as f64]);
                    }
                }
            }
        }
        let out = hull_points(&flat, HullOptions::default()).unwrap();
        assert_eq!(out.report.distinct_points, 64);
        assert_eq!(out.report.hull_vertices, 8, "only cube corners are extreme");
        assert_eq!(out.report.hull_triangles, 12);
        assert!((volume(&out.positions, &out.indices) - 27.0).abs() < 1e-12);
        assert_supporting(&out, &flat);
    }

    #[test]
    fn lower_dimensional_inputs_are_refused() {
        let same = [1.0, 2.0, 3.0].repeat(10);
        assert_eq!(
            hull_points(&same, HullOptions::default()).unwrap_err(),
            HullError::Degenerate(HullDegeneracy::TooFewPoints)
        );
        let line: Vec<f64> = (0..10)
            .flat_map(|i| [i as f64, 2.0 * i as f64, -(i as f64)])
            .collect();
        assert_eq!(
            hull_points(&line, HullOptions::default()).unwrap_err(),
            HullError::Degenerate(HullDegeneracy::Collinear)
        );
        // Tilted plane x + y + z = 1 hit exactly by dyadic coordinates.
        let plane: Vec<f64> = (0..20)
            .flat_map(|i| {
                let (x, y) = ((i % 5) as f64 * 0.25, (i / 5) as f64 * 0.5);
                [x, y, 1.0 - x - y]
            })
            .collect();
        assert_eq!(
            hull_points(&plane, HullOptions::default()).unwrap_err(),
            HullError::Degenerate(HullDegeneracy::Coplanar)
        );
        assert!(matches!(
            hull_points(&[f64::NAN, 0.0, 0.0], HullOptions::default()),
            Err(HullError::InvalidInput(_))
        ));
    }

    #[test]
    fn nearly_coplanar_cloud_stays_manifold() {
        // Points within ~1e-14 of a plane: exact signs keep the hull consistent where float
        // signs would disagree. (Magnitudes stay clear of underflow, where the expansion
        // arithmetic of the predicate stops being exact.)
        let mut rng = Rng(42);
        let mut flat = Vec::new();
        for _ in 0..200 {
            let (x, y) = (rng.next(), rng.next());
            flat.extend([x, y, 1e-14 * rng.next()]);
        }
        flat.extend([0.5, 0.5, 1e-12]);
        let out = hull_points(&flat, HullOptions::default()).unwrap();
        assert_supporting(&out, &flat);
    }

    #[test]
    fn weld_and_mesh_entry_points_share_contract() {
        let (p, t) = crate::geom::fixtures::sphere(2.0, 8, 12);
        let from_mesh = hull_mesh(&p, &t, HullOptions::default()).unwrap();
        let from_points = hull_points(&p, HullOptions::default()).unwrap();
        assert_eq!(
            from_mesh.report.hull_vertices,
            from_points.report.hull_vertices
        );
        assert!((volume(&from_mesh.positions, &from_mesh.indices) - volume(&p, &t)).abs() < 1e-9);
        // Jittered duplicates collapse under weld.
        let mut jitter = p.clone();
        jitter.extend(p.iter().map(|v| v + 1e-9));
        let welded = hull_points(
            &jitter,
            HullOptions {
                weld_epsilon: 1e-6,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(welded.report.distinct_points, p.len() / 3);
        let open = &t[3..];
        assert_eq!(
            hull_mesh(&p, open, HullOptions::default()).unwrap_err(),
            HullError::NotManifold
        );
        let budget = hull_points(
            &p,
            HullOptions {
                max_points: 5,
                ..Default::default()
            },
        );
        assert!(matches!(budget, Err(HullError::BudgetExceeded { .. })));
    }
}
