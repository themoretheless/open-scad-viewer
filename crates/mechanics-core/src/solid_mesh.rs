//! Automatic linear tetrahedral meshing of a closed, consistently oriented
//! triangle surface (the finished printed body).
//!
//! Method: a structured grid over the bounding box is classified by the
//! generalized winding number at each cell centre; every inside cell is split
//! into six Kuhn tetrahedra (conforming across neighbours). Boundary nodes are
//! then moved to the closest point of the input surface. A node's move is
//! bisected back whenever an adjacent tetrahedron would lose positive volume
//! or fall below the quality bound, so the result always keeps positive,
//! bounded-quality elements. This is not a constrained Delaunay mesher: thin
//! features below the grid spacing are lost, sharp features not aligned with
//! the grid are rounded, and the residual distance between boundary nodes and
//! the surface is reported (`max_snap_gap_mm`) instead of hidden.
//!
//! Units: millimetres. Each boundary face carries the group id of the nearest
//! input triangle so boundary conditions can be chosen by face groups.

use crate::{Error, Result};
use std::collections::HashMap;

pub const MAX_GRID_CELLS: usize = 400_000;
pub const MAX_SURFACE_TRIANGLES: usize = 200_000;

/// Closed, outward-oriented triangle surface with an optional group per triangle.
#[derive(Clone, Debug, Default)]
pub struct Surface {
    pub vertices_mm: Vec<[f64; 3]>,
    pub triangles: Vec<[usize; 3]>,
    /// Empty means every triangle belongs to group 0.
    pub groups: Vec<u32>,
}

#[derive(Clone, Copy, Debug)]
pub struct MeshOptions {
    /// Target grid spacing; each axis is divided into `ceil(extent / size)` cells.
    pub target_size_mm: f64,
    /// Lower bound on `6*sqrt(2)*V / l_rms^3` (1 for a regular tetrahedron).
    pub min_quality: f64,
}

impl Default for MeshOptions {
    fn default() -> Self {
        Self {
            target_size_mm: 1.,
            min_quality: 0.05,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoundaryFace {
    /// Outward-oriented node triple.
    pub nodes: [usize; 3],
    pub group: u32,
}

#[derive(Clone, Debug)]
pub struct TetMesh {
    pub nodes_mm: Vec<[f64; 3]>,
    /// Positively oriented node quadruples.
    pub tets: Vec<[usize; 4]>,
    pub boundary: Vec<BoundaryFace>,
}

#[derive(Clone, Debug)]
pub struct MeshReport {
    pub mesh: TetMesh,
    pub grid_cells: [usize; 3],
    pub mesh_volume_mm3: f64,
    pub surface_volume_mm3: f64,
    pub surface_area_mm2: f64,
    pub boundary_area_mm2: f64,
    pub min_quality: f64,
    pub min_volume_mm3: f64,
    /// Largest distance from a boundary node to the input surface after snapping.
    pub max_snap_gap_mm: f64,
}

fn invalid(s: &str) -> Error {
    Error::new("SOLID_MESH_INVALID_INPUT", s)
}

type V = [f64; 3];
fn sub(a: V, b: V) -> V {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn add(a: V, b: V) -> V {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn scale(a: V, s: f64) -> V {
    [a[0] * s, a[1] * s, a[2] * s]
}
fn dot(a: V, b: V) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: V, b: V) -> V {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn norm(a: V) -> f64 {
    dot(a, a).sqrt()
}

/// Signed volume of tetrahedron (a, b, c, d).
pub fn tet_volume(a: V, b: V, c: V, d: V) -> f64 {
    dot(sub(b, a), cross(sub(c, a), sub(d, a))) / 6.
}

/// `6*sqrt(2)*V / l_rms^3`: 1 for a regular tetrahedron, <= 0 when inverted.
pub fn tet_quality(p: [V; 4]) -> f64 {
    let mut sum = 0.;
    for i in 0..4 {
        for j in i + 1..4 {
            let e = sub(p[j], p[i]);
            sum += dot(e, e);
        }
    }
    let l = (sum / 6.).sqrt();
    if l <= 0. {
        return 0.;
    }
    6. * 2f64.sqrt() * tet_volume(p[0], p[1], p[2], p[3]) / (l * l * l)
}

fn closest_on_triangle(p: V, a: V, b: V, c: V) -> V {
    // Ericson, Real-Time Collision Detection, 5.1.5.
    let ab = sub(b, a);
    let ac = sub(c, a);
    let ap = sub(p, a);
    let d1 = dot(ab, ap);
    let d2 = dot(ac, ap);
    if d1 <= 0. && d2 <= 0. {
        return a;
    }
    let bp = sub(p, b);
    let d3 = dot(ab, bp);
    let d4 = dot(ac, bp);
    if d3 >= 0. && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0. && d1 >= 0. && d3 <= 0. {
        return add(a, scale(ab, d1 / (d1 - d3)));
    }
    let cp = sub(p, c);
    let d5 = dot(ab, cp);
    let d6 = dot(ac, cp);
    if d6 >= 0. && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0. && d2 >= 0. && d6 <= 0. {
        return add(a, scale(ac, d2 / (d2 - d6)));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0. && (d4 - d3) >= 0. && (d5 - d6) >= 0. {
        return add(b, scale(sub(c, b), (d4 - d3) / ((d4 - d3) + (d5 - d6))));
    }
    let denom = 1. / (va + vb + vc);
    add(a, add(scale(ab, vb * denom), scale(ac, vc * denom)))
}

impl Surface {
    fn tri(&self, t: usize) -> [V; 3] {
        let [a, b, c] = self.triangles[t];
        [
            self.vertices_mm[a],
            self.vertices_mm[b],
            self.vertices_mm[c],
        ]
    }
    fn group(&self, t: usize) -> u32 {
        self.groups.get(t).copied().unwrap_or(0)
    }

    /// Rejects open, non-manifold or inconsistently oriented input.
    pub fn validate(&self) -> Result<()> {
        if self.triangles.is_empty() || self.triangles.len() > MAX_SURFACE_TRIANGLES {
            return Err(invalid("Surface needs 1..=200000 triangles"));
        }
        if !self.groups.is_empty() && self.groups.len() != self.triangles.len() {
            return Err(invalid("groups must be empty or one per triangle"));
        }
        if self.vertices_mm.iter().flatten().any(|x| !x.is_finite()) {
            return Err(invalid("Non-finite vertex"));
        }
        let mut directed: HashMap<(usize, usize), usize> = HashMap::new();
        for t in &self.triangles {
            if t.iter().any(|&i| i >= self.vertices_mm.len())
                || t[0] == t[1]
                || t[1] == t[2]
                || t[0] == t[2]
            {
                return Err(invalid("Triangle index out of range or repeated"));
            }
            for k in 0..3 {
                *directed.entry((t[k], t[(k + 1) % 3])).or_default() += 1;
            }
        }
        for (&(a, b), &n) in &directed {
            if n != 1 || directed.get(&(b, a)) != Some(&1) {
                return Err(invalid(
                    "Surface is not a closed, consistently oriented 2-manifold",
                ));
            }
        }
        if self.volume() <= 0. {
            return Err(invalid(
                "Surface must enclose positive volume (outward normals)",
            ));
        }
        Ok(())
    }

    pub fn volume(&self) -> f64 {
        (0..self.triangles.len())
            .map(|t| {
                let [a, b, c] = self.tri(t);
                dot(a, cross(b, c)) / 6.
            })
            .sum()
    }

    pub fn area(&self) -> f64 {
        (0..self.triangles.len())
            .map(|t| {
                let [a, b, c] = self.tri(t);
                norm(cross(sub(b, a), sub(c, a))) / 2.
            })
            .sum()
    }

    /// Generalized winding number (1 inside, 0 outside for a closed surface).
    pub fn winding_number(&self, p: V) -> f64 {
        let mut omega = 0.;
        for t in 0..self.triangles.len() {
            let [a, b, c] = self.tri(t);
            let (a, b, c) = (sub(a, p), sub(b, p), sub(c, p));
            let (la, lb, lc) = (norm(a), norm(b), norm(c));
            let num = dot(a, cross(b, c));
            let den = la * lb * lc + dot(a, b) * lc + dot(b, c) * la + dot(c, a) * lb;
            omega += 2. * num.atan2(den);
        }
        omega / (4. * std::f64::consts::PI)
    }

    /// Closest surface point and the triangle it lies on.
    pub fn closest_point(&self, p: V) -> (V, usize) {
        let mut best = (p, 0, f64::INFINITY);
        for t in 0..self.triangles.len() {
            let [a, b, c] = self.tri(t);
            let q = closest_on_triangle(p, a, b, c);
            let d = dot(sub(q, p), sub(q, p));
            if d < best.2 {
                best = (q, t, d);
            }
        }
        (best.0, best.1)
    }
}

/// Kuhn split of the unit cube: corner index bits are (x, y, z).
const KUHN: [[usize; 4]; 6] = [
    [0, 1, 3, 7],
    [0, 1, 5, 7],
    [0, 2, 3, 7],
    [0, 2, 6, 7],
    [0, 4, 5, 7],
    [0, 4, 6, 7],
];

pub fn mesh_surface(surface: &Surface, options: MeshOptions) -> Result<MeshReport> {
    surface.validate()?;
    if !(options.target_size_mm.is_finite() && options.target_size_mm > 0.) {
        return Err(invalid("target_size_mm must be positive"));
    }
    if !(options.min_quality.is_finite() && options.min_quality > 0. && options.min_quality < 1.) {
        return Err(invalid("min_quality must be in (0, 1)"));
    }
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for t in &surface.triangles {
        for &i in t {
            for k in 0..3 {
                lo[k] = lo[k].min(surface.vertices_mm[i][k]);
                hi[k] = hi[k].max(surface.vertices_mm[i][k]);
            }
        }
    }
    let mut n = [0usize; 3];
    let mut h = [0.; 3];
    for k in 0..3 {
        let ext = hi[k] - lo[k];
        n[k] = ((ext / options.target_size_mm) - 1e-9).ceil().max(1.) as usize;
        h[k] = ext / n[k] as f64;
    }
    if n.iter().product::<usize>() > MAX_GRID_CELLS {
        return Err(invalid(
            "Grid exceeds MAX_GRID_CELLS; increase target_size_mm",
        ));
    }
    let node_id = |i: usize, j: usize, k: usize| (k * (n[1] + 1) + j) * (n[0] + 1) + i;
    let mut map: HashMap<usize, usize> = HashMap::new();
    let mut nodes: Vec<V> = Vec::new();
    let mut tets: Vec<[usize; 4]> = Vec::new();
    for k in 0..n[2] {
        for j in 0..n[1] {
            for i in 0..n[0] {
                let centre = [
                    lo[0] + (i as f64 + 0.5) * h[0],
                    lo[1] + (j as f64 + 0.5) * h[1],
                    lo[2] + (k as f64 + 0.5) * h[2],
                ];
                if surface.winding_number(centre) < 0.5 {
                    continue;
                }
                let mut corner = [0usize; 8];
                for (c, slot) in corner.iter_mut().enumerate() {
                    let (di, dj, dk) = (c & 1, (c >> 1) & 1, (c >> 2) & 1);
                    let g = node_id(i + di, j + dj, k + dk);
                    *slot = *map.entry(g).or_insert_with(|| {
                        nodes.push([
                            lo[0] + (i + di) as f64 * h[0],
                            lo[1] + (j + dj) as f64 * h[1],
                            lo[2] + (k + dk) as f64 * h[2],
                        ]);
                        nodes.len() - 1
                    });
                }
                for kt in KUHN {
                    let mut t = kt.map(|c| corner[c]);
                    if tet_volume(nodes[t[0]], nodes[t[1]], nodes[t[2]], nodes[t[3]]) < 0. {
                        t.swap(2, 3);
                    }
                    tets.push(t);
                }
            }
        }
    }
    if tets.is_empty() {
        return Err(invalid(
            "No grid cell lies inside the surface; reduce target_size_mm",
        ));
    }
    // Boundary faces: faces used by exactly one tetrahedron, oriented outward.
    let mut faces: HashMap<[usize; 3], ([usize; 3], usize)> = HashMap::new();
    for t in &tets {
        for f in [
            [t[1], t[2], t[3]],
            [t[0], t[3], t[2]],
            [t[0], t[1], t[3]],
            [t[0], t[2], t[1]],
        ] {
            let mut key = f;
            key.sort_unstable();
            faces.entry(key).or_insert((f, 0)).1 += 1;
        }
    }
    let mut boundary_faces: Vec<[usize; 3]> = faces
        .into_values()
        .filter(|(_, c)| *c == 1)
        .map(|(f, _)| f)
        .collect();
    boundary_faces.sort_unstable();
    let mut on_boundary = vec![false; nodes.len()];
    for f in &boundary_faces {
        for &v in f {
            on_boundary[v] = true;
        }
    }
    let origin = nodes.clone();
    let target: Vec<Option<V>> = (0..nodes.len())
        .map(|v| on_boundary[v].then(|| surface.closest_point(origin[v]).0))
        .collect();
    let mut alpha: Vec<f64> = target
        .iter()
        .map(|t| if t.is_some() { 1. } else { 0. })
        .collect();
    let place = |alpha: &[f64], nodes: &mut Vec<V>| {
        for v in 0..nodes.len() {
            if let Some(t) = target[v] {
                nodes[v] = add(origin[v], scale(sub(t, origin[v]), alpha[v]));
            }
        }
    };
    let quality_of = |nodes: &[V], t: &[usize; 4]| tet_quality(t.map(|v| nodes[v]));
    let mut ok = false;
    for _ in 0..40 {
        place(&alpha, &mut nodes);
        let mut bad = false;
        let mut shrink = vec![false; nodes.len()];
        for t in &tets {
            if t.iter().any(|&v| alpha[v] > 0.) && quality_of(&nodes, t) < options.min_quality {
                bad = true;
                for &v in t {
                    shrink[v] = true;
                }
            }
        }
        if !bad {
            ok = true;
            break;
        }
        for v in 0..nodes.len() {
            if shrink[v] {
                alpha[v] = if alpha[v] < 1e-3 { 0. } else { alpha[v] * 0.5 };
            }
        }
    }
    if !ok {
        // Fall back: undo snapping on every node of any remaining bad element.
        loop {
            place(&alpha, &mut nodes);
            let mut changed = false;
            for t in &tets {
                if quality_of(&nodes, t) < options.min_quality {
                    for &v in t {
                        if alpha[v] != 0. {
                            alpha[v] = 0.;
                            changed = true;
                        }
                    }
                }
            }
            if !changed {
                break;
            }
        }
        place(&alpha, &mut nodes);
    }
    let boundary: Vec<BoundaryFace> = boundary_faces
        .iter()
        .map(|&f| {
            let c = scale(add(add(nodes[f[0]], nodes[f[1]]), nodes[f[2]]), 1. / 3.);
            BoundaryFace {
                nodes: f,
                group: surface.group(surface.closest_point(c).1),
            }
        })
        .collect();
    let mut min_q = f64::INFINITY;
    let mut min_v = f64::INFINITY;
    let mut vol = 0.;
    for t in &tets {
        let p = t.map(|v| nodes[v]);
        let v = tet_volume(p[0], p[1], p[2], p[3]);
        vol += v;
        min_v = min_v.min(v);
        min_q = min_q.min(tet_quality(p));
    }
    let boundary_area = boundary
        .iter()
        .map(|f| {
            let [a, b, c] = f.nodes.map(|v| nodes[v]);
            norm(cross(sub(b, a), sub(c, a))) / 2.
        })
        .sum();
    let max_gap = (0..nodes.len())
        .filter(|&v| on_boundary[v])
        .map(|v| norm(sub(surface.closest_point(nodes[v]).0, nodes[v])))
        .fold(0., f64::max);
    Ok(MeshReport {
        mesh: TetMesh {
            nodes_mm: nodes,
            tets,
            boundary,
        },
        grid_cells: n,
        mesh_volume_mm3: vol,
        surface_volume_mm3: surface.volume(),
        surface_area_mm2: surface.area(),
        boundary_area_mm2: boundary_area,
        min_quality: min_q,
        min_volume_mm3: min_v,
        max_snap_gap_mm: max_gap,
    })
}

/// Test and example helpers: closed prisms with face groups.
pub mod shapes {
    use super::Surface;

    pub const GROUP_BOTTOM: u32 = 1;
    pub const GROUP_TOP: u32 = 2;
    /// Side faces get `GROUP_SIDE + edge index`.
    pub const GROUP_SIDE: u32 = 10;

    /// Extrudes a counter-clockwise simple polygon from z = 0 to z = height.
    /// Caps are fans from `apex`, which must see every edge (convex polygons:
    /// any vertex; an L shape: its reflex vertex).
    pub fn extrude(polygon: &[[f64; 2]], apex: usize, height: f64) -> Surface {
        let n = polygon.len();
        let mut s = Surface::default();
        for z in [0., height] {
            for p in polygon {
                s.vertices_mm.push([p[0], p[1], z]);
            }
        }
        for i in 0..n {
            let a = (apex + 1 + i) % n;
            let b = (apex + 2 + i) % n;
            if a == apex || b == apex {
                continue;
            }
            s.triangles.push([apex, b, a]);
            s.groups.push(GROUP_BOTTOM);
            s.triangles.push([n + apex, n + a, n + b]);
            s.groups.push(GROUP_TOP);
        }
        for i in 0..n {
            let j = (i + 1) % n;
            s.triangles.push([i, j, n + j]);
            s.triangles.push([i, n + j, n + i]);
            s.groups.push(GROUP_SIDE + i as u32);
            s.groups.push(GROUP_SIDE + i as u32);
        }
        s
    }

    pub fn rectangle(w: f64, d: f64) -> Vec<[f64; 2]> {
        vec![[0., 0.], [w, 0.], [w, d], [0., d]]
    }

    pub fn regular_polygon(radius: f64, segments: usize) -> Vec<[f64; 2]> {
        (0..segments)
            .map(|i| {
                let a = std::f64::consts::TAU * i as f64 / segments as f64;
                [radius * a.cos(), radius * a.sin()]
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::shapes::*;
    use super::*;

    fn check(report: &MeshReport, volume_tol: f64) {
        let m = &report.mesh;
        assert!(report.min_volume_mm3 > 0., "non-positive tet");
        assert!(report.min_quality >= 0.05, "quality {}", report.min_quality);
        let rel =
            (report.mesh_volume_mm3 - report.surface_volume_mm3).abs() / report.surface_volume_mm3;
        assert!(rel < volume_tol, "volume error {rel}");
        let area_rel =
            (report.boundary_area_mm2 - report.surface_area_mm2).abs() / report.surface_area_mm2;
        assert!(area_rel < 2. * volume_tol, "area error {area_rel}");
        // Boundary is a closed surface: every edge shared by two faces in opposite directions.
        let mut directed: HashMap<(usize, usize), i32> = HashMap::new();
        for f in &m.boundary {
            for k in 0..3 {
                *directed
                    .entry((f.nodes[k], f.nodes[(k + 1) % 3]))
                    .or_default() += 1;
            }
        }
        for (&(a, b), &c) in &directed {
            assert_eq!(c, 1);
            assert_eq!(directed.get(&(b, a)), Some(&1));
        }
        // Signed volume enclosed by the boundary equals the tet volume.
        let enclosed: f64 = m
            .boundary
            .iter()
            .map(|f| {
                let [a, b, c] = f.nodes.map(|v| m.nodes_mm[v]);
                dot(a, cross(b, c)) / 6.
            })
            .sum();
        assert!((enclosed - report.mesh_volume_mm3).abs() < 1e-9 * report.mesh_volume_mm3);
    }

    #[test]
    fn cube_is_meshed_exactly() {
        let s = extrude(&rectangle(10., 10.), 0, 10.);
        let r = mesh_surface(
            &s,
            MeshOptions {
                target_size_mm: 2.5,
                min_quality: 0.05,
            },
        )
        .unwrap();
        check(&r, 1e-12);
        assert_eq!(r.mesh.tets.len(), 4 * 4 * 4 * 6);
        assert!(r.max_snap_gap_mm < 1e-12);
        let groups: std::collections::BTreeSet<u32> =
            r.mesh.boundary.iter().map(|f| f.group).collect();
        assert_eq!(
            groups.len(),
            6,
            "all six faces keep their groups: {groups:?}"
        );
    }

    #[test]
    fn cylinder_volume_and_surface_match() {
        let s = extrude(&regular_polygon(10., 48), 0, 20.);
        let r = mesh_surface(
            &s,
            MeshOptions {
                target_size_mm: 1.25,
                min_quality: 0.05,
            },
        )
        .unwrap();
        check(&r, 0.03);
        assert!(r.max_snap_gap_mm < 0.7, "gap {}", r.max_snap_gap_mm);
    }

    #[test]
    fn l_bracket_volume_and_surface_match() {
        let l = [
            [0., 0.],
            [20., 0.],
            [20., 5.],
            [5., 5.],
            [5., 20.],
            [0., 20.],
        ];
        let s = extrude(&l, 3, 8.);
        let r = mesh_surface(
            &s,
            MeshOptions {
                target_size_mm: 1.25,
                min_quality: 0.05,
            },
        )
        .unwrap();
        check(&r, 1e-9);
        // Two bodies with equal bounding boxes but different cut-outs give different models.
        let full = mesh_surface(
            &extrude(&rectangle(20., 20.), 0, 8.),
            MeshOptions {
                target_size_mm: 1.25,
                min_quality: 0.05,
            },
        )
        .unwrap();
        assert!(full.mesh.tets.len() > r.mesh.tets.len());
    }

    #[test]
    fn rejects_open_and_inverted_surfaces() {
        let mut s = extrude(&rectangle(1., 1.), 0, 1.);
        s.triangles.pop();
        s.groups.pop();
        assert!(mesh_surface(&s, MeshOptions::default()).is_err());
        let mut s = extrude(&rectangle(1., 1.), 0, 1.);
        for t in &mut s.triangles {
            t.swap(1, 2);
        }
        assert!(mesh_surface(&s, MeshOptions::default()).is_err());
    }
}
