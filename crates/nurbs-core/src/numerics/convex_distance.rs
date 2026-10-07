//! Convex broad-phase culling: GJK distance, EPA penetration and OBB SAT.
//!
//! Complements the interval-based certified enclosures in
//! `crate::distance_bounds` / `crate::curve_distance` with a fast convex
//! pre-filter: control nets of positive-weight NURBS contain the actual
//! geometry inside their convex hull, so hull separation proves disjointness
//! and hull penetration is a necessary (not sufficient) overlap signal.
//!
//! No `unsafe`, no external dependencies beyond `math-core`.
use crate::curve::Curve;
use crate::foundation::guards::{Budget, require_finite_vec};
use crate::surface::Surface;
use crate::{Result, check, numeric};
use math_core::{cross, dot, norm, sub, unit};

/// Iteration budget for GJK (distance phase).
pub const GJK_MAX_ITERATIONS: usize = 64;
/// Iteration budget for EPA (penetration phase).
pub const EPA_MAX_ITERATIONS: usize = 64;

/// Convex set given by its support mapping: the farthest point along `dir`.
pub trait Support {
    /// Farthest point of the set along `dir` (`dir` need not be unit).
    fn support(&self, dir: [f64; 3]) -> [f64; 3];
}

/// Convex hull of a finite point set (e.g. a control net).
#[derive(Clone, Debug)]
pub struct PointCloud {
    pub points: Vec<[f64; 3]>,
}
impl Support for PointCloud {
    fn support(&self, dir: [f64; 3]) -> [f64; 3] {
        self.points
            .iter()
            .copied()
            .max_by(|a, b| dot(*a, dir).total_cmp(&dot(*b, dir)))
            .unwrap_or([0.; 3])
    }
}

/// Axis-aligned bounding box.
#[derive(Clone, Copy, Debug)]
pub struct Aabb {
    pub min: [f64; 3],
    pub max: [f64; 3],
}
impl Support for Aabb {
    fn support(&self, dir: [f64; 3]) -> [f64; 3] {
        [
            if dir[0] >= 0. { self.max[0] } else { self.min[0] },
            if dir[1] >= 0. { self.max[1] } else { self.min[1] },
            if dir[2] >= 0. { self.max[2] } else { self.min[2] },
        ]
    }
}

/// Closed ball.
#[derive(Clone, Copy, Debug)]
pub struct Sphere {
    pub center: [f64; 3],
    pub radius: f64,
}
impl Support for Sphere {
    fn support(&self, dir: [f64; 3]) -> [f64; 3] {
        let len = norm(dir);
        if len <= f64::EPSILON || self.radius <= 0. {
            return self.center;
        }
        let s = self.radius / len;
        [
            self.center[0] + dir[0] * s,
            self.center[1] + dir[1] * s,
            self.center[2] + dir[2] * s,
        ]
    }
}

/// Oriented box: center, orthonormal right-handed axes, half extents.
#[derive(Clone, Copy, Debug)]
pub struct Obb {
    pub center: [f64; 3],
    pub axes: [[f64; 3]; 3],
    pub half_extents: [f64; 3],
}
impl Support for Obb {
    fn support(&self, dir: [f64; 3]) -> [f64; 3] {
        let mut p = self.center;
        for i in 0..3 {
            let s = dot(self.axes[i], dir);
            let sign = if s >= 0. { 1. } else { -1. };
            for k in 0..3 {
                p[k] += sign * self.half_extents[i] * self.axes[i][k];
            }
        }
        p
    }
}

/// Support of the Minkowski difference `A ⊖ B`, keeping witness points.
#[derive(Clone, Copy, Debug)]
struct SupportPoint {
    a: [f64; 3],
    b: [f64; 3],
    w: [f64; 3],
}
fn support_pair<A: Support + ?Sized, B: Support + ?Sized>(
    a: &A,
    b: &B,
    dir: [f64; 3],
) -> SupportPoint {
    let pa = a.support(dir);
    let pb = b.support([-dir[0], -dir[1], -dir[2]]);
    SupportPoint {
        a: pa,
        b: pb,
        w: sub(pa, pb),
    }
}

/// Outcome of a GJK distance query.
#[derive(Clone, Copy, Debug)]
pub struct GjkResult {
    /// Whether the two sets intersect (origin inside the difference).
    pub intersecting: bool,
    /// Distance between the sets (`0.` when intersecting).
    pub distance: f64,
    /// Closest point on the first set.
    pub point_a: [f64; 3],
    /// Closest point on the second set.
    pub point_b: [f64; 3],
    /// Iterations consumed.
    pub iterations: usize,
    /// False when the iteration budget was exhausted before convergence.
    pub converged: bool,
}

/// Solve a small dense linear system `m x = r` by Gaussian elimination with
/// partial pivoting. Returns `None` on (near-)singular matrices.
fn solve_small(m: &[Vec<f64>], n: usize, r: &[f64]) -> Option<Vec<f64>> {
    let mut a: Vec<Vec<f64>> = (0..n).map(|i| m[i][..n].to_vec()).collect();
    let mut b: Vec<f64> = r[..n].to_vec();
    for col in 0..n {
        let pivot = (col..n)
            .max_by(|&i, &j| a[i][col].abs().total_cmp(&a[j][col].abs()))?;
        if a[pivot][col].abs() <= 1e-14 {
            return None;
        }
        a.swap(col, pivot);
        b.swap(col, pivot);
        for row in col + 1..n {
            let f = a[row][col] / a[col][col];
            for k in col..n {
                a[row][k] -= f * a[col][k];
            }
            b[row] -= f * b[col];
        }
    }
    let mut x = vec![0.; n];
    for i in (0..n).rev() {
        let mut s = b[i];
        for k in i + 1..n {
            s -= a[i][k] * x[k];
        }
        x[i] = s / a[i][i];
    }
    Some(x)
}

/// Closest point to the origin in the convex hull of `pts` (`w` components),
/// enumerated over face subsets (Johnson-style, robust to degeneracy).
/// Returns the closest point and the indices + barycentric weights of the
/// supporting subset.
fn closest_to_origin(simplex: &[SupportPoint]) -> ([f64; 3], Vec<(usize, f64)>) {
    let n = simplex.len();
    let mut best: Option<(f64, [f64; 3], Vec<(usize, f64)>)> = None;
    // Enumerate every nonempty subset; keep those whose unconstrained affine
    // minimizer lies inside the hull (all barycentric weights >= -tol).
    for mask in 1usize..(1 << n) {
        let idx: Vec<usize> = (0..n).filter(|&i| mask & (1 << i) != 0).collect();
        let s = idx.len();
        if s == 1 {
            let w = simplex[idx[0]].w;
            let d = norm(w);
            let better = best.as_ref().map_or(true, |(bd, _, _)| d < *bd);
            if better {
                best = Some((d, w, vec![(idx[0], 1.)]));
            }
            continue;
        }
        // KKT system for min |sum lambda_i p_i|^2 s.t. sum lambda = 1.
        let mut m = vec![vec![0.; s + 1]; s + 1];
        let mut r = vec![0.; s + 1];
        for (i, &pi) in idx.iter().enumerate() {
            for (j, &pj) in idx.iter().enumerate() {
                m[i][j] = dot(simplex[pi].w, simplex[pj].w);
            }
            m[i][s] = 1.;
            m[s][i] = 1.;
        }
        r[s] = 1.;
        let Some(x) = solve_small(&m, s + 1, &r) else {
            continue;
        };
        let lambda = &x[..s];
        const TOL: f64 = -1e-9;
        if lambda.iter().any(|&l| l < TOL) {
            continue;
        }
        let mut v = [0.; 3];
        for (i, &pi) in idx.iter().enumerate() {
            for k in 0..3 {
                v[k] += lambda[i].max(0.) * simplex[pi].w[k];
            }
        }
        let d = norm(v);
        let better = best.as_ref().map_or(true, |(bd, _, _)| d < *bd - 1e-12);
        if better {
            best = Some((
                d,
                v,
                idx.iter()
                    .enumerate()
                    .map(|(i, &pi)| (pi, lambda[i].max(0.)))
                    .collect(),
            ));
        }
    }
    match best {
        Some((_, v, weights)) => (v, weights),
        // Unreachable for nonempty simplex, but degrade gracefully.
        None => ([0.; 3], vec![(0, 1.)]),
    }
}

/// GJK distance / intersection query between two convex sets.
pub fn gjk<A: Support + ?Sized, B: Support + ?Sized>(a: &A, b: &B) -> Result<GjkResult> {
    let pa = a.support([1., 0., 0.]);
    let pb = b.support([1., 0., 0.]);
    let mut dir = sub(pb, pa);
    if norm(dir) <= f64::EPSILON {
        dir = [1., 0., 0.];
    }
    let mut simplex: Vec<SupportPoint> = Vec::with_capacity(4);
    let mut converged = false;
    let mut iterations = 0;
    let mut v = [0.; 3];
    let mut weights: Vec<(usize, f64)> = vec![];
    // Unified budget guard (item 1065) mirroring GJK_MAX_ITERATIONS.
    let mut guard = Budget::with_iterations(GJK_MAX_ITERATIONS)?.guard("gjk");
    for iter in 0..GJK_MAX_ITERATIONS {
        guard.tick()?;
        iterations = iter + 1;
        let p = support_pair(a, b, dir);
        if simplex
            .iter()
            .any(|q| norm(sub(q.w, p.w)) <= 1e-12 * (1. + norm(p.w)))
        {
            // Support point repeated: the simplex cannot grow, degenerate.
            converged = true;
            break;
        }
        simplex.push(p);
        let (cv, cw) = closest_to_origin(&simplex);
        v = cv;
        // Reduce the simplex to the closest feature and rebase weights.
        let reduced: Vec<SupportPoint> = cw.iter().map(|&(i, _)| simplex[i]).collect();
        simplex = reduced;
        weights = cw
            .iter()
            .enumerate()
            .map(|(new_i, &(_, lam))| (new_i, lam))
            .collect();
        if simplex.len() > 4 {
            simplex.truncate(4);
            weights.truncate(4);
        }
        let d = norm(v);
        if d <= 1e-12 {
            // Origin on the simplex: intersection.
            converged = true;
            break;
        }
        if simplex.len() == 4 {
            // Closest feature is the full tetrahedron: origin enclosed.
            v = [0.; 3];
            converged = true;
            break;
        }
        dir = [-v[0], -v[1], -v[2]];
        // Progress check: the new support barely advances past the current
        // closest point along `dir`, so `d` is the distance.
        let next = support_pair(a, b, dir);
        let progress = dot(sub(next.w, v), dir);
        if progress <= 1e-12 * (1. + d * d) {
            converged = true;
            break;
        }
    }
    let distance = norm(v);
    let intersecting = distance <= 1e-12;
    let mut point_a = [0.; 3];
    let mut point_b = [0.; 3];
    for &(i, lam) in &weights {
        for k in 0..3 {
            point_a[k] += lam * simplex[i].a[k];
            point_b[k] += lam * simplex[i].b[k];
        }
    }
    if simplex.is_empty() {
        point_a = pa;
        point_b = pb;
    }
    numeric(
        distance.is_finite(),
        "GJK distance exceeded numeric range",
    )?;
    Ok(GjkResult {
        intersecting,
        distance: if intersecting { 0. } else { distance },
        point_a,
        point_b,
        iterations,
        converged,
    })
}

/// Outcome of an EPA penetration query.
#[derive(Clone, Copy, Debug)]
pub struct EpaResult {
    /// Penetration depth (lower bound when not converged).
    pub depth: f64,
    /// Unit push direction that separates the sets (applied to B).
    pub normal: [f64; 3],
    /// Iterations consumed.
    pub iterations: usize,
    /// False on budget exhaustion or degeneracy; `depth` is then best-effort.
    pub converged: bool,
}

#[derive(Clone, Copy)]
struct EpaFace {
    /// Three vertices of the Minkowski difference (with witnesses).
    verts: [SupportPoint; 3],
    /// Outward unit normal.
    normal: [f64; 3],
    /// Distance from origin to the face plane.
    distance: f64,
}

/// Face with CCW winding as seen from `normal`'s side; `distance` is signed
/// (negative means the origin sits on the normal side of the plane).
fn epa_face(a: SupportPoint, b: SupportPoint, c: SupportPoint) -> Option<EpaFace> {
    let ab = sub(b.w, a.w);
    let ac = sub(c.w, a.w);
    let n = cross(ab, ac);
    let len = norm(n);
    if len <= 1e-14 {
        return None;
    }
    let normal = [n[0] / len, n[1] / len, n[2] / len];
    let distance = dot(normal, a.w);
    Some(EpaFace {
        verts: [a, b, c],
        normal,
        distance,
    })
}

/// Reverse winding and normal, keeping the same plane.
fn epa_flip(f: &mut EpaFace) {
    f.verts.swap(1, 2);
    f.normal = [-f.normal[0], -f.normal[1], -f.normal[2]];
    f.distance = -f.distance;
}

/// EPA penetration depth after GJK reports intersection. `seed` may carry the
/// final GJK simplex witnesses (up to 4); when absent or degenerate the
/// polytope is bootstrapped from support samples around the axis frame.
pub fn epa<A: Support + ?Sized, B: Support + ?Sized>(a: &A, b: &B) -> Result<EpaResult> {
    // Bootstrap a polytope around the origin from axial *and* diagonal
    // supports: purely axial seeds can leave the origin on the hull boundary
    // (all off-axis supports share one coordinate), which stalls EPA.
    let s = 1. / 3f64.sqrt();
    let dirs = [
        [1., 0., 0.],
        [-1., 0., 0.],
        [0., 1., 0.],
        [0., -1., 0.],
        [0., 0., 1.],
        [0., 0., -1.],
        [s, s, s],
        [s, s, -s],
        [s, -s, s],
        [s, -s, -s],
        [-s, s, s],
        [-s, s, -s],
        [-s, -s, s],
        [-s, -s, -s],
    ];
    let mut verts: Vec<SupportPoint> = Vec::new();
    for d in dirs {
        let p = support_pair(a, b, d);
        if !verts
            .iter()
            .any(|q| norm(sub(q.w, p.w)) <= 1e-12 * (1. + norm(p.w)))
        {
            verts.push(p);
        }
    }
    // Best-effort axial fallback for (nearly) flat difference sets: minimal
    // support span over the sampled directions.
    let axial_fallback = |verts_ok: bool| -> Result<EpaResult> {
        let _ = verts_ok;
        let mut depth = f64::INFINITY;
        let mut normal = [1., 0., 0.];
        for d in dirs {
            let p = support_pair(a, b, d);
            let q = support_pair(a, b, [-d[0], -d[1], -d[2]]);
            let span = dot(p.w, d) - dot(q.w, d);
            if span < depth {
                depth = span.max(0.);
                normal = unit(d);
            }
        }
        numeric(depth.is_finite(), "EPA depth exceeded numeric range")?;
        Ok(EpaResult {
            depth: if depth.is_finite() { depth } else { 0. },
            normal,
            iterations: 0,
            converged: false,
        })
    };
    if verts.len() < 4 {
        return axial_fallback(false);
    }
    // Pick a non-degenerate tetrahedron: farthest point, then farthest from
    // the line, then farthest from the plane.
    let v0 = verts[0];
    let v1 = *verts
        .iter()
        .max_by(|p, q| {
            norm(sub(p.w, v0.w)).total_cmp(&norm(sub(q.w, v0.w)))
        })
        .unwrap();
    let line_dir = sub(v1.w, v0.w);
    let v2 = *verts
        .iter()
        .max_by(|p, q| {
            norm(cross(sub(p.w, v0.w), line_dir))
                .total_cmp(&norm(cross(sub(q.w, v0.w), line_dir)))
        })
        .unwrap();
    let plane_n = cross(sub(v1.w, v0.w), sub(v2.w, v0.w));
    let v3 = *verts
        .iter()
        .max_by(|p, q| {
            dot(sub(p.w, v0.w), plane_n)
                .abs()
                .total_cmp(&dot(sub(q.w, v0.w), plane_n).abs())
        })
        .unwrap();
    let tetra = [v0, v1, v2, v3];
    if dot(sub(v3.w, v0.w), plane_n).abs() <= 1e-14 {
        // Coplanar seeds (e.g. flat crossing control nets): best-effort.
        return axial_fallback(true);
    }
    let mut faces: Vec<EpaFace> = Vec::new();
    let combos = [(0, 1, 2), (0, 1, 3), (0, 2, 3), (1, 2, 3)];
    for &(i, j, k) in &combos {
        // Opposite vertex of this face within the tetrahedron.
        let opposite = tetra[(0..4).find(|&m| m != i && m != j && m != k).unwrap()];
        if let Some(mut f) = epa_face(tetra[i], tetra[j], tetra[k]) {
            // Orient the normal (and winding) away from the opposite vertex.
            if dot(f.normal, sub(opposite.w, f.verts[0].w)) > 0. {
                epa_flip(&mut f);
            }
            faces.push(f);
        }
    }
    check(!faces.is_empty(), "EPA failed to build an initial polytope")?;
    // Vertices referenced by the current polytope (a subset of the sampled
    // seeds): the duplicate/stall guard must check against these only.
    let mut polytope_verts: Vec<SupportPoint> = tetra.to_vec();
    let mut iterations = 0;
    let mut converged = false;
    // Unified budget guard (item 1065) mirroring EPA_MAX_ITERATIONS.
    let mut guard = Budget::with_iterations(EPA_MAX_ITERATIONS)?.guard("epa");
    for iter in 0..EPA_MAX_ITERATIONS {
        guard.tick()?;
        iterations = iter + 1;
        // Closest face to the origin.
        let Some(face_idx) = faces
            .iter()
            .enumerate()
            .min_by(|(_, x), (_, y)| x.distance.total_cmp(&y.distance))
            .map(|(i, _)| i)
        else {
            break;
        };
        let face = faces[face_idx];
        let p = support_pair(a, b, face.normal);
        let new_distance = dot(p.w, face.normal);
        if new_distance - face.distance <= 1e-10 * (1. + face.distance) {
            converged = true;
            break;
        }
        let duplicate = polytope_verts
            .iter()
            .any(|q| norm(sub(q.w, p.w)) <= 1e-12 * (1. + norm(p.w)));
        if duplicate {
            // The support in this direction is already a polytope vertex:
            // the boundary cannot move farther along this normal, so the
            // current closest distance is the best attainable answer.
            converged = true;
            break;
        }
        polytope_verts.push(p);
        // Remove faces visible from the new vertex; collect horizon edges.
        let mut horizon: Vec<(SupportPoint, SupportPoint)> = Vec::new();
        let mut kept: Vec<EpaFace> = Vec::new();
        for f in &faces {
            // Coplanar faces (dot ~ 0) count as visible: a strictly-zero
            // test would keep internal diagonal faces that pass through the
            // origin and falsely report zero depth.
            let visible = dot(f.normal, sub(p.w, f.verts[0].w)) > -1e-12;
            if visible {
                for e in 0..3 {
                    horizon.push((f.verts[e], f.verts[(e + 1) % 3]));
                }
            } else {
                kept.push(*f);
            }
        }
        // Horizon edges belong to exactly one removed face (unmatched twins).
        let mut boundary: Vec<(SupportPoint, SupportPoint)> = Vec::new();
        'edges: for (i, &(u, v)) in horizon.iter().enumerate() {
            for (j, &(uu, vv)) in horizon.iter().enumerate() {
                if i != j
                    && norm(sub(u.w, vv.w)) <= 1e-12
                    && norm(sub(v.w, uu.w)) <= 1e-12
                {
                    continue 'edges;
                }
            }
            boundary.push((u, v));
        }
        faces = kept;
        for (u, v) in boundary {
            if let Some(f) = epa_face(u, v, p) {
                faces.push(f);
            }
        }
        if faces.is_empty() {
            break;
        }
    }
    // Best-effort answer from the closest remaining face.
    let Some(face) = faces
        .iter()
        .min_by(|x, y| x.distance.total_cmp(&y.distance))
    else {
        return Ok(EpaResult {
            depth: 0.,
            normal: [1., 0., 0.],
            iterations,
            converged: false,
        });
    };
    numeric(
        face.distance.is_finite(),
        "EPA depth exceeded numeric range",
    )?;
    Ok(EpaResult {
        depth: face.distance.max(0.),
        normal: face.normal,
        iterations,
        converged,
    })
}

/// SAT verdict for two oriented boxes.
#[derive(Clone, Copy, Debug)]
pub struct SatResult {
    /// True when a separating axis exists.
    pub separated: bool,
    /// Minimum overlap depth across all 15 axes (`0.` when separated).
    pub min_overlap: f64,
    /// Axis of least overlap (undefined direction when separated).
    pub axis: [f64; 3],
}

/// Separating-axis test between two OBBs over all 15 candidate axes
/// (Gottschalk formulation), with early exit on the first separating axis.
pub fn obb_sat(a: &Obb, b: &Obb) -> SatResult {
    // Rotation of B's axes into A's frame and its absolute value (robustified).
    let mut r = [[0.; 3]; 3];
    let mut abs_r = [[0.; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            r[i][j] = dot(a.axes[i], b.axes[j]);
            abs_r[i][j] = r[i][j].abs() + 1e-12;
        }
    }
    let t_global = sub(b.center, a.center);
    let t = [
        dot(t_global, a.axes[0]),
        dot(t_global, a.axes[1]),
        dot(t_global, a.axes[2]),
    ];
    let mut min_overlap = f64::INFINITY;
    let mut best_axis = [1., 0., 0.];
    let mut track = |axis: [f64; 3], ra: f64, rb: f64, dist: f64| -> bool {
        let overlap = ra + rb - dist.abs();
        if overlap < 0. {
            return false; // separating axis found
        }
        if overlap < min_overlap {
            min_overlap = overlap;
            best_axis = if dist >= 0. {
                axis
            } else {
                [-axis[0], -axis[1], -axis[2]]
            };
        }
        true
    };
    // 3 axes of A.
    for i in 0..3 {
        let rb = (0..3)
            .map(|j| b.half_extents[j] * abs_r[i][j])
            .sum::<f64>();
        if !track(a.axes[i], a.half_extents[i], rb, t[i]) {
            return SatResult {
                separated: true,
                min_overlap: 0.,
                axis: a.axes[i],
            };
        }
    }
    // 3 axes of B.
    for j in 0..3 {
        let ra = (0..3)
            .map(|i| a.half_extents[i] * abs_r[i][j])
            .sum::<f64>();
        let dist = t[0] * r[0][j] + t[1] * r[1][j] + t[2] * r[2][j];
        if !track(b.axes[j], ra, b.half_extents[j], dist) {
            return SatResult {
                separated: true,
                min_overlap: 0.,
                axis: b.axes[j],
            };
        }
    }
    // 9 cross-product axes.
    for i in 0..3 {
        for j in 0..3 {
            let ra = a.half_extents[(i + 1) % 3] * abs_r[(i + 2) % 3][j]
                + a.half_extents[(i + 2) % 3] * abs_r[(i + 1) % 3][j];
            let rb = b.half_extents[(j + 1) % 3] * abs_r[i][(j + 2) % 3]
                + b.half_extents[(j + 2) % 3] * abs_r[i][(j + 1) % 3];
            let dist = t[(i + 2) % 3] * r[(i + 1) % 3][j]
                - t[(i + 1) % 3] * r[(i + 2) % 3][j];
            let axis = cross(a.axes[i], b.axes[j]);
            if norm(axis) <= 1e-12 {
                continue; // nearly parallel axes: axis carries no information
            }
            if !track(unit(axis), ra, rb, dist) {
                return SatResult {
                    separated: true,
                    min_overlap: 0.,
                    axis: unit(axis),
                };
            }
        }
    }
    SatResult {
        separated: false,
        min_overlap: if min_overlap.is_finite() {
            min_overlap.max(0.)
        } else {
            0.
        },
        axis: best_axis,
    }
}

/// Convenience: whether two OBBs are separated by some axis.
pub fn obb_separated(a: &Obb, b: &Obb) -> bool {
    obb_sat(a, b).separated
}

/// Fast broad-phase verdict from convex hulls of control nets.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CullVerdict {
    /// Convex hulls are separated; geometry is provably disjoint.
    Disjoint {
        /// Distance between the convex hulls. Hulls are supersets of the
        /// geometry, so hull distance is a lower bound on the true
        /// geometry distance: `hull_distance <= geometry_distance`.
        min_distance: f64,
    },
    /// Convex hulls penetrate; geometry may overlap.
    Overlap {
        /// EPA penetration depth of the hulls.
        penetration: f64,
    },
    /// The query failed to converge within its budget.
    Unknown,
}

/// Extract 3D control points of a curve as a convex set.
pub fn curve_control_cloud(curve: &Curve) -> Result<PointCloud> {
    check(
        !curve.control_points.is_empty(),
        "Curve control net is empty",
    )?;
    let mut points = Vec::with_capacity(curve.control_points.len());
    for p in &curve.control_points {
        check(p.len() == 3, "Convex culling expects 3D control points")?;
        require_finite_vec(p, "control_points")?;
        points.push([p[0], p[1], p[2]]);
    }
    Ok(PointCloud { points })
}

/// Extract 3D control points of a surface as a convex set.
pub fn surface_control_cloud(surface: &Surface) -> Result<PointCloud> {
    check(
        !surface.control_points.is_empty(),
        "Surface control net is empty",
    )?;
    let mut points = Vec::new();
    for row in &surface.control_points {
        for p in row {
            check(p.len() == 3, "Convex culling expects 3D control points")?;
            require_finite_vec(p, "control_points")?;
            points.push([p[0], p[1], p[2]]);
        }
    }
    check(!points.is_empty(), "Surface control net is empty")?;
    Ok(PointCloud { points })
}

fn clouds_verdict(a: &PointCloud, b: &PointCloud) -> Result<CullVerdict> {
    let g = gjk(a, b)?;
    if !g.converged {
        return Ok(CullVerdict::Unknown);
    }
    if !g.intersecting {
        return Ok(CullVerdict::Disjoint {
            min_distance: g.distance,
        });
    }
    let e = epa(a, b)?;
    Ok(CullVerdict::Overlap {
        penetration: e.depth,
    })
}

/// Quick disjointness verdict between the convex hulls of two curve control
/// nets. For positive-weight NURBS the curve lies inside the hull, so
/// `Disjoint` is a proof; `Overlap` means "possibly interacting".
pub fn control_nets_disjoint(a: &Curve, b: &Curve) -> Result<CullVerdict> {
    clouds_verdict(&curve_control_cloud(a)?, &curve_control_cloud(b)?)
}

/// Quick disjointness verdict between the convex hulls of two surface
/// control nets, as a pre-filter before expensive surface operations (SSI).
pub fn surfaces_disjoint(a: &Surface, b: &Surface) -> Result<CullVerdict> {
    clouds_verdict(&surface_control_cloud(a)?, &surface_control_cloud(b)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sphere(c: [f64; 3], r: f64) -> Sphere {
        Sphere {
            center: c,
            radius: r,
        }
    }

    #[test]
    fn gjk_far_spheres_matches_exact_distance() -> Result<()> {
        let a = sphere([0., 0., 0.], 1.);
        let b = sphere([5., 0., 0.], 0.5);
        let g = gjk(&a, &b)?;
        assert!(!g.intersecting);
        assert!(g.converged);
        assert!((g.distance - 3.5).abs() < 1e-9, "distance {}", g.distance);
        assert!((g.point_a[0] - 1.).abs() < 1e-9);
        assert!((g.point_b[0] - 4.5).abs() < 1e-9);
        Ok(())
    }

    #[test]
    fn gjk_tangent_spheres_touch() -> Result<()> {
        let a = sphere([0., 0., 0.], 1.);
        let b = sphere([2., 0., 0.], 1.);
        let g = gjk(&a, &b)?;
        assert!(g.distance < 1e-6, "distance {}", g.distance);
        Ok(())
    }

    #[test]
    fn gjk_overlapping_spheres_intersect() -> Result<()> {
        let a = sphere([0., 0., 0.], 2.);
        let b = sphere([1., 0., 0.], 2.);
        let g = gjk(&a, &b)?;
        assert!(g.intersecting);
        assert_eq!(g.distance, 0.);
        Ok(())
    }

    #[test]
    fn gjk_point_vs_aabb_exact() -> Result<()> {
        let p = PointCloud {
            points: vec![[3., 4., 0.]],
        };
        let b = Aabb {
            min: [-1., -1., -1.],
            max: [1., 1., 1.],
        };
        let g = gjk(&p, &b)?;
        assert!(!g.intersecting);
        assert!((g.distance - 5. * 0.6 - 0.4 - 1.4).abs() < 1.5, "loose");
        // exact: sqrt((3-1)^2 + (4-1)^2) = sqrt(13)
        assert!((g.distance - 13f64.sqrt()).abs() < 1e-9);
        Ok(())
    }

    #[test]
    fn gjk_degenerate_identical_spheres() -> Result<()> {
        let a = sphere([1., 2., 3.], 0.);
        let b = sphere([1., 2., 3.], 0.);
        let g = gjk(&a, &b)?;
        assert!(g.intersecting);
        Ok(())
    }

    #[test]
    fn epa_penetration_of_overlapping_boxes_exact() -> Result<()> {
        // Two unit AABBs offset by 0.5 along x: penetration 1.5 along x.
        let a = Aabb {
            min: [-1., -1., -1.],
            max: [1., 1., 1.],
        };
        let b = Aabb {
            min: [-0.5, -1., -1.],
            max: [1.5, 1., 1.],
        };
        let g = gjk(&a, &b)?;
        assert!(g.intersecting);
        let e = epa(&a, &b)?;
        assert!(e.depth > 0., "depth {}", e.depth);
        assert!(
            (e.depth - 1.5).abs() < 1e-6,
            "expected 1.5, got {}",
            e.depth
        );
        Ok(())
    }

    #[test]
    fn epa_penetration_of_overlapping_spheres_best_effort() -> Result<()> {
        // Two unit spheres with centers 1.2 apart: true penetration 0.8;
        // within the iteration budget EPA must approach it from below.
        let a = sphere([0., 0., 0.], 1.);
        let b = sphere([1.2, 0., 0.], 1.);
        let g = gjk(&a, &b)?;
        assert!(g.intersecting);
        let e = epa(&a, &b)?;
        assert!(
            e.depth > 0.4 && e.depth <= 0.8 + 1e-9,
            "depth {} outside (0.4, 0.8]",
            e.depth
        );
        Ok(())
    }

    #[test]
    fn epa_depth_is_best_effort_under_budget() -> Result<()> {
        // Concentric identical spheres: maximally degenerate difference set.
        let a = sphere([0., 0., 0.], 1.);
        let b = sphere([0., 0., 0.], 1.);
        let e = epa(&a, &b)?;
        assert!(e.depth.is_finite() && e.depth >= 0.);
        Ok(())
    }

    fn unit_obb(center: [f64; 3]) -> Obb {
        Obb {
            center,
            axes: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            half_extents: [1., 1., 1.],
        }
    }

    #[test]
    fn sat_separated_boxes() {
        let a = unit_obb([0., 0., 0.]);
        let b = unit_obb([3., 0., 0.]);
        assert!(obb_separated(&a, &b));
    }

    #[test]
    fn sat_touching_boxes() {
        let a = unit_obb([0., 0., 0.]);
        let b = unit_obb([2., 0., 0.]);
        let r = obb_sat(&a, &b);
        assert!(!r.separated);
        assert!(r.min_overlap < 1e-9);
    }

    #[test]
    fn sat_rotated_box_corner_overlap() {
        let a = unit_obb([0., 0., 0.]);
        let s = 0.7071067811865476;
        let b = Obb {
            center: [1.9, 0., 0.],
            axes: [[s, s, 0.], [-s, s, 0.], [0., 0., 1.]],
            half_extents: [1., 1., 1.],
        };
        // Rotated box reaches x = 1.9 - sqrt(2) ≈ 0.486 < 1: overlapping.
        let r = obb_sat(&a, &b);
        assert!(!r.separated);
        assert!(r.min_overlap > 0.);
    }

    #[test]
    fn sat_min_overlap_depth() {
        let a = unit_obb([0., 0., 0.]);
        let b = unit_obb([1.5, 0., 0.]);
        let r = obb_sat(&a, &b);
        assert!(!r.separated);
        assert!((r.min_overlap - 0.5).abs() < 1e-9, "{}", r.min_overlap);
    }

    fn curve(points: &[[f64; 3]]) -> Curve {
        let n = points.len();
        Curve {
            degree: 1.min(n - 1),
            knots: (0..=n + 1).map(|i| i as f64).collect(),
            control_points: points.iter().map(|p| p.to_vec()).collect(),
            weights: vec![1.; n],
            periodic: false,
        }
    }

    #[test]
    fn control_cloud_rejects_nan_with_param_and_index() {
        let a = curve(&[[0., 0., 0.], [1., f64::NAN, 0.]]);
        let err = curve_control_cloud(&a).unwrap_err();
        assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
        assert!(err.contains("control_points[1]"), "{err}");
    }

    #[test]
    fn gjk_epa_budget_guards_match_loop_constants() -> Result<()> {
        // The guards mirror the loop bounds: a normal query never trips them.
        let a = sphere([0., 0., 0.], 1.);
        let b = sphere([3., 0., 0.], 1.);
        let g = gjk(&a, &b)?;
        assert!(g.converged && g.iterations <= GJK_MAX_ITERATIONS);
        Ok(())
    }

    #[test]
    fn control_nets_disjoint_far_curves() -> Result<()> {
        let a = curve(&[[0., 0., 0.], [1., 0., 0.]]);
        let b = curve(&[[0., 10., 0.], [1., 10., 0.]]);
        match control_nets_disjoint(&a, &b)? {
            CullVerdict::Disjoint { min_distance } => {
                assert!((min_distance - 10.).abs() < 1e-9)
            }
            other => panic!("expected Disjoint, got {other:?}"),
        }
        Ok(())
    }

    #[test]
    fn control_nets_overlap_crossing_curves() -> Result<()> {
        let a = curve(&[[-1., 0., 0.], [1., 0., 0.]]);
        let b = curve(&[[0., -1., 0.], [0., 1., 0.]]);
        match control_nets_disjoint(&a, &b)? {
            CullVerdict::Overlap { penetration } => assert!(penetration >= 0.),
            CullVerdict::Unknown => {}
            other => panic!("expected Overlap, got {other:?}"),
        }
        Ok(())
    }

    #[test]
    fn hull_distance_bounds_curve_distance() -> Result<()> {
        // Hull distance is a *lower* bound on the true curve-curve distance:
        // hulls are supersets of the curves. Verified against the exact
        // endpoint distance of two straight degree-1 segments.
        let a = curve(&[[0., 0., 0.], [1., 0., 0.]]);
        let b = curve(&[[0., 3., 0.], [1., 4., 0.]]);
        let exact = 3f64; // both segments vertical offset by >= 3
        match control_nets_disjoint(&a, &b)? {
            CullVerdict::Disjoint { min_distance } => {
                assert!(min_distance <= exact + 1e-9);
                assert!((min_distance - exact).abs() < 1e-9);
            }
            other => panic!("expected Disjoint, got {other:?}"),
        }
        Ok(())
    }

    #[test]
    fn surfaces_disjoint_far_planes() -> Result<()> {
        let plane = |z: f64| Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., z], vec![0., 1., z]],
                vec![vec![1., 0., z], vec![1., 1., z]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        match surfaces_disjoint(&plane(0.), &plane(5.))? {
            CullVerdict::Disjoint { min_distance } => {
                assert!((min_distance - 5.).abs() < 1e-9)
            }
            other => panic!("expected Disjoint, got {other:?}"),
        }
        Ok(())
    }
}
