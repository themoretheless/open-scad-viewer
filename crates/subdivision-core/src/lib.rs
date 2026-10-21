#![doc = include_str!("../README.md")]
//! Bounded Catmull–Clark refinement. Original polygon IDs survive refinement.
//! Tessellation is a neutral triangle buffer; mesh inspect/fit lives in the bridge.
use rustc_hash::{FxHashMap, FxHashSet};
type Point = math_core::V3;
#[cfg(feature = "codec")]
mod serialization;
pub use math_core::{Error, Result};
const INVALID_INPUT: &str = "SUBDIVISION_INVALID_INPUT";
fn error(message: impl Into<String>) -> Error {
    Error::new(INVALID_INPUT, message)
}
#[derive(Clone, Debug)]
pub struct Cage {
    pub vertices: Vec<Point>,
    pub faces: Vec<Vec<usize>>,
}

#[derive(Clone, Debug)]
pub struct Refined {
    pub cage: Cage,
    pub face_ids: Vec<usize>,
}

fn avg(p: impl Iterator<Item = Point>, n: usize) -> Point {
    let mut r = [0.; 3];
    for q in p {
        for k in 0..3 {
            r[k] += q[k] / n as f64;
        }
    }
    r
}
type Edges = FxHashMap<(usize, usize), Vec<(usize, bool)>>;
fn edge(a: usize, b: usize) -> (usize, usize) {
    (a.min(b), a.max(b))
}

impl Refined {
    /// Triangulate faces with ear-clipping fallback for concave polygons.
    pub fn triangulate(&self) -> Result<(geometry_ops::Triangles, Vec<usize>)> {
        // `Refined` fields are public, so callers may have mutated the cage:
        // this is a public entry point and keeps the full input validation.
        self.cage.validate()?;
        if self.face_ids.len() != self.cage.faces.len() {
            return Err(error("Invalid face identity count"));
        }
        let mut indices = Vec::with_capacity(self.cage.faces.len() * 6);
        let mut ids = Vec::with_capacity(self.cage.faces.len() * 2);
        for (fi, f) in self.cage.faces.iter().enumerate() {
            if f.len() <= 4 {
                for i in 1..f.len() - 1 {
                    indices.extend([f[0], f[i], f[i + 1]]);
                    ids.push(self.face_ids[fi]);
                }
            } else {
                // Ear-clipping along dominant Newell normal projection for N-gons
                let mut n = [0.0; 3];
                for i in 0..f.len() {
                    let p = self.cage.vertices[f[i]];
                    let q = self.cage.vertices[f[(i + 1) % f.len()]];
                    n[0] += (p[1] - q[1]) * (p[2] + q[2]);
                    n[1] += (p[2] - q[2]) * (p[0] + q[0]);
                    n[2] += (p[0] - q[0]) * (p[1] + q[1]);
                }
                let (ax, ay, sign) = if n[2].abs() >= n[0].abs() && n[2].abs() >= n[1].abs() {
                    (0, 1, n[2].signum())
                } else if n[1].abs() >= n[0].abs() {
                    (2, 0, n[1].signum())
                } else {
                    (1, 2, n[0].signum())
                };
                let proj = |vi: usize| -> [f64; 2] {
                    let p = self.cage.vertices[vi];
                    [p[ax], p[ay] * sign]
                };
                let cross2 = |a: [f64; 2], b: [f64; 2], c: [f64; 2]| {
                    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
                };
                let in_tri = |p: [f64; 2], a: [f64; 2], b: [f64; 2], c: [f64; 2]| {
                    cross2(a, b, p) >= -1e-12 && cross2(b, c, p) >= -1e-12 && cross2(c, a, p) >= -1e-12
                };
                let mut rem = f.clone();
                while rem.len() > 3 {
                    let m = rem.len();
                    let mut clipped = false;
                    for i in 0..m {
                        let prev = rem[(i + m - 1) % m];
                        let curr = rem[i];
                        let next = rem[(i + 1) % m];
                        let (pa, pb, pc) = (proj(prev), proj(curr), proj(next));
                        if cross2(pa, pb, pc) <= 1e-12 {
                            continue;
                        }
                        if rem
                            .iter()
                            .all(|&v| v == prev || v == curr || v == next || !in_tri(proj(v), pa, pb, pc))
                        {
                            indices.extend([prev, curr, next]);
                            ids.push(self.face_ids[fi]);
                            rem.remove(i);
                            clipped = true;
                            break;
                        }
                    }
                    if !clipped {
                        indices.extend([rem[0], rem[1], rem[2]]);
                        ids.push(self.face_ids[fi]);
                        rem.remove(1);
                    }
                }
                if rem.len() == 3 {
                    indices.extend([rem[0], rem[1], rem[2]]);
                    ids.push(self.face_ids[fi]);
                }
            }
        }
        Ok((
            geometry_ops::Triangles {
                positions: self.cage.vertices.iter().flatten().copied().collect(),
                indices,
            },
            ids,
        ))
    }
}
#[cfg(test)]
#[path = "tests/tests_integration.rs"]
mod tests;

impl Cage {
    pub fn from_faces(vertices: Vec<Point>, faces: Vec<Vec<usize>>) -> Result<Self> {
        let cage = Self { vertices, faces };
        cage.validate()?;
        Ok(cage)
    }
}

#[derive(Clone, Debug)]
pub struct Fit {
    pub cage: Cage,
    pub iterations: usize,
    pub vertex_residual_before_mm: f64,
    pub vertex_residual_after_mm: f64,
    pub correspondence: &'static str,
}

/// Fits cage positions so one Catmull–Clark step interpolates the original
/// vertices more closely. Not a unique inverse limit surface.
pub fn fit(cage: &Cage, iterations: usize) -> Result<Fit> {
    if iterations > 32 {
        return Err(error("Subdivision fitting requires 0..32 iterations"));
    }
    cage.validate()?;
    let mut cage = cage.clone();
    let target = cage.vertices.clone();
    // Candidates share the input's faces, so topology stays valid; only the
    // numeric coordinate budget can break. Check it cheaply per candidate.
    let within_budget = |c: &Cage| {
        c.vertices
            .iter()
            .flatten()
            .all(|x| x.is_finite() && x.abs() <= 1e6)
    };
    let residual = |c: &Cage| -> Result<(f64, Vec<Point>)> {
        let r = c.subdivide_validated(1)?;
        let delta: Vec<Point> = target
            .iter()
            .zip(&r.cage.vertices)
            .map(|(a, b)| std::array::from_fn(|k| a[k] - b[k]))
            .collect();
        let error =
            (delta.iter().flatten().map(|x| x * x).sum::<f64>() / target.len() as f64).sqrt();
        Ok((error, delta))
    };
    let (before, mut delta) = residual(&cage)?;
    let mut after = before;
    let mut completed = 0;
    for _ in 0..iterations {
        if after <= 1e-12 {
            break;
        }
        let mut accepted = false;
        for step in [1., 0.5, 0.25, 0.125, 0.0625] {
            let mut candidate = cage.clone();
            for (p, d) in candidate.vertices.iter_mut().zip(&delta) {
                for k in 0..3 {
                    p[k] += step * d[k];
                }
            }
            if !within_budget(&candidate) {
                continue;
            }
            let (error, next) = residual(&candidate)?;
            if error < after {
                cage = candidate;
                after = error;
                delta = next;
                accepted = true;
                completed += 1;
                break;
            }
        }
        if !accepted {
            break;
        }
    }
    Ok(Fit {
        cage,
        iterations: completed,
        vertex_residual_before_mm: before,
        vertex_residual_after_mm: after,
        correspondence: "source_triangle_topology_one_refinement_step",
    })
}

impl Cage {
    pub fn deform(&self, operation: &geometry_ops::Deformation) -> Result<Self> {
        self.validate()?;
        operation.validate()?;
        let mut cage = self.clone();
        cage.vertices = self
            .vertices
            .iter()
            .map(|&p| operation.apply(p))
            .collect::<Result<_>>()?;
        cage.validate()?;
        Ok(cage)
    }
    pub fn brush(&self, brush: &geometry_ops::Brush) -> Result<Self> {
        self.validate()?;
        brush.validate()?;
        let mut cage = self.clone();
        cage.vertices = self
            .vertices
            .iter()
            .map(|&p| brush.apply(p))
            .collect::<Result<_>>()?;
        cage.validate()?;
        Ok(cage)
    }
    /// Newell face normals accumulated per control vertex, plus edge adjacency.
    pub fn sculpt_target(&self) -> Result<geometry_ops::SculptTarget> {
        self.validate()?;
        let faces = self.faces.iter().map(Vec::as_slice);
        geometry_ops::SculptTarget::from_faces(self.vertices.clone(), faces)
    }
    /// Sculpts control vertices; faces are unchanged and the cage stays valid.
    pub fn sculpt(&self, brush: &geometry_ops::SculptBrush) -> Result<Self> {
        let mut cage = self.clone();
        cage.vertices = self.sculpt_target()?.sculpt(brush)?;
        cage.validate()?;
        Ok(cage)
    }
}

impl Cage {
    /// Constructs a quad-sided cage directly; section start indices establish correspondence.
    pub fn loft(sections: &[Vec<Point>], caps: bool) -> Result<Self> {
        if !(2..=64).contains(&sections.len())
            || !(3..=64).contains(&sections[0].len())
            || sections.iter().any(|s| s.len() != sections[0].len())
        {
            return Err(error("Invalid subdivision loft sections"));
        }
        let n = sections[0].len();
        let vertices = sections.iter().flatten().copied().collect();
        let mut faces =
            Vec::with_capacity((sections.len() - 1) * n + if caps { 2 } else { 0 });
        for k in 0..sections.len() - 1 {
            for i in 0..n {
                faces.push(vec![
                    k * n + i,
                    k * n + (i + 1) % n,
                    (k + 1) * n + (i + 1) % n,
                    (k + 1) * n + i,
                ]);
            }
        }
        if caps {
            faces.push((0..n).rev().collect());
            faces.push(((sections.len() - 1) * n..sections.len() * n).collect());
        }
        let cage = Self { vertices, faces };
        cage.validate()?;
        Ok(cage)
    }
    pub fn extrude(profile: &[Point], vector: Point) -> Result<Self> {
        if vector.iter().any(|v| !v.is_finite())
            || vector.iter().map(|v| v * v).sum::<f64>() <= 1e-24
        {
            return Err(error("Invalid subdivision extrusion vector"));
        }
        Self::loft(
            &[
                profile.to_vec(),
                profile
                    .iter()
                    .map(|p| std::array::from_fn(|k| p[k] + vector[k]))
                    .collect(),
            ],
            true,
        )
    }
    pub fn sweep(profile: &[[f64; 2]], path: &[Point], up: Point, caps: bool) -> Result<Self> {
        Self::loft(&geometry_ops::sweep_sections(profile, path, up)?, caps)
    }
    pub fn revolve(profile: &[[f64; 2]], segments: usize) -> Result<Self> {
        if !(3..=64).contains(&segments)
            || !(3..=64).contains(&profile.len())
            || profile
                .iter()
                .any(|p| !p[0].is_finite() || !p[1].is_finite() || p[0] <= 0.)
        {
            return Err(error(
                "Subdivision revolve requires 3..64 segments and positive-radius profile",
            ));
        }
        let n = profile.len();
        let mut vertices = Vec::with_capacity(segments * n);
        for k in 0..segments {
            let a = 2. * std::f64::consts::PI * k as f64 / segments as f64;
            let (s, c) = a.sin_cos();
            vertices.extend(profile.iter().map(|p| [p[0] * c, p[0] * s, p[1]]));
        }
        let mut faces = Vec::with_capacity(segments * n);
        for k in 0..segments {
            for i in 0..n {
                faces.push(vec![
                    k * n + i,
                    k * n + (i + 1) % n,
                    ((k + 1) % segments) * n + (i + 1) % n,
                    ((k + 1) % segments) * n + i,
                ]);
            }
        }
        let cage = Self { vertices, faces };
        cage.validate()?;
        Ok(cage)
    }
}

/// Exact bicubic uniform B-spline patch extracted from a regular interior Catmull–Clark quad face.
#[derive(Clone, Debug, PartialEq)]
pub struct BicubicPatch {
    pub face_index: usize,
    pub control_points: [[Point; 4]; 4],
}

impl BicubicPatch {
    /// Evaluates the bicubic uniform B-spline patch at parameter $(u, v) \in [0, 1]^2$.
    pub fn evaluate(&self, u: f64, v: f64) -> Point {
        let basis = |t: f64| -> [f64; 4] {
            let t2 = t * t;
            let t3 = t2 * t;
            let omt = 1.0 - t;
            [
                omt * omt * omt / 6.0,
                (3.0 * t3 - 6.0 * t2 + 4.0) / 6.0,
                (-3.0 * t3 + 3.0 * t2 + 3.0 * t + 1.0) / 6.0,
                t3 / 6.0,
            ]
        };
        let bu = basis(u.clamp(0.0, 1.0));
        let bv = basis(v.clamp(0.0, 1.0));
        let mut p = [0.0; 3];
        for (i, &wu) in bu.iter().enumerate() {
            for (j, &wv) in bv.iter().enumerate() {
                let w = wu * wv;
                for k in 0..3 {
                    p[k] += w * self.control_points[i][j][k];
                }
            }
        }
        p
    }
}

impl Cage {
    /// Computes exact Catmull–Clark limit positions for all vertices without infinite subdivision.
    /// Applies one Catmull–Clark topological refinement step (so all faces are quads and every
    /// original vertex is surrounded by quads) followed by the exact Halstead–Kass–DeRose limit
    /// stencil:
    /// - Interior vertex of valence $n$: $v^\infty = \frac{n^2 P + 4 \sum_i E_i + \sum_i F_i}{n(n + 5)}$,
    ///   where $E_i$ are neighbor vertices and $F_i$ are opposite diagonal corners of incident quads.
    /// - Boundary vertex: $v^\infty = \frac{4 P + B_0 + B_1}{6}$.
    pub fn evaluate_limit_points(&self) -> Result<Vec<Point>> {
        self.validate()?;
        let refined = self.subdivide_validated(1)?.cage;
        let edges = refined.build_edges();
        let mut adjacency = vec![Vec::new(); refined.vertices.len()];
        for entry in &edges {
            adjacency[entry.0.0].push(entry);
            adjacency[entry.0.1].push(entry);
        }
        let mut out = Vec::with_capacity(self.vertices.len());
        let mut faces = FxHashSet::default();
        for v in 0..self.vertices.len() {
            let p = refined.vertices[v];
            let adjacent = &adjacency[v];
            let mut b0 = [0.0; 3];
            let mut b1 = [0.0; 3];
            let mut b_count = 0usize;
            for ((a, b), u) in adjacent {
                if u.len() == 1 {
                    let nb = refined.vertices[if *a == v { *b } else { *a }];
                    if b_count == 0 {
                        b0 = nb;
                    } else if b_count == 1 {
                        b1 = nb;
                    }
                    b_count += 1;
                }
            }
            if b_count == 2 {
                out.push(std::array::from_fn(|k| {
                    (4.0 * p[k] + b0[k] + b1[k]) / 6.0
                }));
            } else {
                faces.clear();
                faces.extend(adjacent.iter().flat_map(|(_, u)| u.iter().map(|(f, _)| *f)));
                let n = adjacent.len() as f64;
                let mut sum_e = [0.0; 3];
                for &(&(a, b), _) in adjacent {
                    let nb = refined.vertices[if a == v { b } else { a }];
                    for k in 0..3 {
                        sum_e[k] += nb[k];
                    }
                }
                let mut sum_f = [0.0; 3];
                for &fi in &faces {
                    let qf = &refined.faces[fi];
                    let pos = qf.iter().position(|&x| x == v).unwrap();
                    let opp = refined.vertices[qf[(pos + 2) % qf.len()]];
                    for k in 0..3 {
                        sum_f[k] += opp[k];
                    }
                }
                let denom = n * (n + 5.0);
                out.push(std::array::from_fn(|k| {
                    (n * n * p[k] + 4.0 * sum_e[k] + sum_f[k]) / denom
                }));
            }
        }
        Ok(out)
    }

    /// Computes normalized limit surface normals at all control vertices.
    pub fn evaluate_limit_normals(&self) -> Result<Vec<Point>> {
        let limit_cage = Cage {
            vertices: self.evaluate_limit_points()?,
            faces: self.faces.clone(),
        };
        Ok(limit_cage.sculpt_target()?.normals)
    }

    /// Extracts exact $4 \times 4$ bicubic B-spline control grids (`BicubicPatch`) for all
    /// regular interior quad faces (where all 4 vertices are interior and have valence 4,
    /// and all 8 surrounding faces are quads).
    pub fn to_nurbs_patches(&self) -> Result<Vec<BicubicPatch>> {
        let edges = self.topology()?;
        let mut vert_faces = vec![Vec::new(); self.vertices.len()];
        for (fi, f) in self.faces.iter().enumerate() {
            for &v in f {
                vert_faces[v].push(fi);
            }
        }
        let mut is_boundary_vert = vec![false; self.vertices.len()];
        for (&(a, b), uses) in &edges {
            if uses.len() == 1 {
                is_boundary_vert[a] = true;
                is_boundary_vert[b] = true;
            }
        }
        let opp_face = |fi: usize, a: usize, b: usize| -> Option<usize> {
            let uses = edges.get(&edge(a, b))?;
            if uses.len() != 2 {
                return None;
            }
            Some(if uses[0].0 == fi { uses[1].0 } else { uses[0].0 })
        };
        // Given a quad face `f_idx` that shares edge `(u, v)`, returns `(opp_u, opp_v)` where
        // `opp_u` is adjacent to `u` and `opp_v` is adjacent to `v` in `f_idx`.
        let quad_opposite_edge = |f_idx: usize, u: usize, v: usize| -> Option<(usize, usize)> {
            let f = &self.faces[f_idx];
            if f.len() != 4 {
                return None;
            }
            let pos_u = f.iter().position(|&x| x == u)?;
            if f[(pos_u + 1) % 4] == v {
                // f = [u, v, opp_v, opp_u]
                Some((f[(pos_u + 3) % 4], f[(pos_u + 2) % 4]))
            } else if f[(pos_u + 3) % 4] == v {
                // f = [u, opp_u, opp_v, v]
                Some((f[(pos_u + 1) % 4], f[(pos_u + 2) % 4]))
            } else {
                None
            }
        };

        let mut patches = Vec::with_capacity(self.faces.len());
        for (fi, f) in self.faces.iter().enumerate() {
            if f.len() != 4 {
                continue;
            }
            let [v0, v1, v2, v3] = [f[0], f[1], f[2], f[3]];
            if [v0, v1, v2, v3]
                .iter()
                .any(|&v| is_boundary_vert[v] || vert_faces[v].len() != 4)
            {
                continue;
            }
            let (Some(f_bottom), Some(f_right), Some(f_top), Some(f_left)) = (
                opp_face(fi, v0, v1),
                opp_face(fi, v1, v2),
                opp_face(fi, v2, v3),
                opp_face(fi, v3, v0),
            ) else {
                continue;
            };
            let (
                Some((b0, b1)),
                Some((r1, r2)),
                Some((t2, t3)),
                Some((l3, l0)),
            ) = (
                quad_opposite_edge(f_bottom, v0, v1),
                quad_opposite_edge(f_right, v1, v2),
                quad_opposite_edge(f_top, v2, v3),
                quad_opposite_edge(f_left, v3, v0),
            ) else {
                continue;
            };

            let corner_diag = |v_center: usize, n_a: usize, n_b: usize| -> Option<usize> {
                let diag_face = *vert_faces[v_center].iter().find(|&&cand| {
                    cand != fi
                        && cand != f_bottom
                        && cand != f_right
                        && cand != f_top
                        && cand != f_left
                        && self.faces[cand].contains(&n_a)
                        && self.faces[cand].contains(&n_b)
                })?;
                if self.faces[diag_face].len() != 4 {
                    return None;
                }
                self.faces[diag_face]
                    .iter()
                    .copied()
                    .find(|&x| x != v_center && x != n_a && x != n_b)
            };

            let (Some(c00), Some(c30), Some(c33), Some(c03)) = (
                corner_diag(v0, b0, l0),
                corner_diag(v1, b1, r1),
                corner_diag(v2, r2, t2),
                corner_diag(v3, t3, l3),
            ) else {
                continue;
            };

            let pt = |idx: usize| self.vertices[idx];
            patches.push(BicubicPatch {
                face_index: fi,
                control_points: [
                    [pt(c00), pt(l0), pt(l3), pt(c03)],
                    [pt(b0), pt(v0), pt(v3), pt(t3)],
                    [pt(b1), pt(v1), pt(v2), pt(t2)],
                    [pt(c30), pt(r1), pt(r2), pt(c33)],
                ],
            });
        }
        Ok(patches)
    }

    /// Feature-adaptive Catmull–Clark subdivision that refines the cage up to `max_levels` (in `0..=5`),
    /// stopping early as soon as the maximum dihedral angle between adjacent faces falls below
    /// `angle_tol_rad` ($\ge 0$).
    pub fn adaptive_subdivide(&self, angle_tol_rad: f64, max_levels: usize) -> Result<Refined> {
        if max_levels > 5 || !angle_tol_rad.is_finite() || angle_tol_rad < 0.0 {
            return Err(error("Invalid adaptive subdivision parameters"));
        }
        self.validate()?;
        let mut current = Refined {
            cage: self.clone(),
            face_ids: (0..self.faces.len()).collect(),
        };
        for _ in 0..max_levels {
            if current.cage.max_dihedral_angle_rad() <= angle_tol_rad {
                break;
            }
            let step = current.cage.subdivide_validated(1)?;
            let next_ids: Vec<usize> = step
                .face_ids
                .iter()
                .map(|&parent| current.face_ids[parent])
                .collect();
            current = Refined {
                cage: step.cage,
                face_ids: next_ids,
            };
        }
        Ok(current)
    }

    fn max_dihedral_angle_rad(&self) -> f64 {
        let edges = self.build_edges();
        let face_normals: Vec<Point> = self
            .faces
            .iter()
            .map(|f| {
                let mut n = [0.0; 3];
                for i in 0..f.len() {
                    let p = self.vertices[f[i]];
                    let q = self.vertices[f[(i + 1) % f.len()]];
                    n[0] += (p[1] - q[1]) * (p[2] + q[2]);
                    n[1] += (p[2] - q[2]) * (p[0] + q[0]);
                    n[2] += (p[0] - q[0]) * (p[1] + q[1]);
                }
                let len = math_core::norm(n);
                if len > 1e-15 {
                    [n[0] / len, n[1] / len, n[2] / len]
                } else {
                    [0.0, 0.0, 1.0]
                }
            })
            .collect();
        let mut max_angle = 0.0_f64;
        for uses in edges.values() {
            if uses.len() == 2 {
                let n0 = face_normals[uses[0].0];
                let n1 = face_normals[uses[1].0];
                let cos_a = math_core::dot(n0, n1).clamp(-1.0, 1.0);
                max_angle = max_angle.max(cos_a.acos());
            }
        }
        max_angle
    }

    /// Computes valence distribution, extraordinary vertex count, face aspect ratios,
    /// and total surface area for the control or subdivided cage.
    pub fn mesh_quality_report(&self) -> Result<SubdQualityReport> {
        let edges = self.topology()?;
        let mut valence = vec![0usize; self.vertices.len()];
        let mut boundary_vert = vec![false; self.vertices.len()];
        for (&(a, b), uses) in &edges {
            valence[a] += 1;
            valence[b] += 1;
            if uses.len() == 1 {
                boundary_vert[a] = true;
                boundary_vert[b] = true;
            }
        }
        let mut hist_map: std::collections::BTreeMap<usize, usize> =
            std::collections::BTreeMap::new();
        let mut extraordinary_vertex_count = 0usize;
        for (v, &val) in valence.iter().enumerate() {
            *hist_map.entry(val).or_insert(0) += 1;
            let regular = if boundary_vert[v] { val == 3 } else { val == 4 };
            if !regular {
                extraordinary_vertex_count += 1;
            }
        }
        let mut min_ar = f64::INFINITY;
        let mut max_ar = 0.0_f64;
        let mut sum_ar = 0.0_f64;
        let mut surface_area = 0.0_f64;
        for f in &self.faces {
            let mut min_e = f64::INFINITY;
            let mut max_e = 0.0_f64;
            for i in 0..f.len() {
                let p = self.vertices[f[i]];
                let q = self.vertices[f[(i + 1) % f.len()]];
                let len = math_core::norm(math_core::sub(q, p));
                min_e = min_e.min(len);
                max_e = max_e.max(len);
            }
            let ar = if min_e > 1e-12 { max_e / min_e } else { 1.0 };
            min_ar = min_ar.min(ar);
            max_ar = max_ar.max(ar);
            sum_ar += ar;
            let p0 = self.vertices[f[0]];
            for i in 1..f.len() - 1 {
                let p1 = self.vertices[f[i]];
                let p2 = self.vertices[f[i + 1]];
                let n = math_core::cross(math_core::sub(p1, p0), math_core::sub(p2, p0));
                surface_area += 0.5 * math_core::norm(n);
            }
        }
        let face_count = self.faces.len();
        Ok(SubdQualityReport {
            vertex_count: self.vertices.len(),
            face_count,
            extraordinary_vertex_count,
            valence_histogram: hist_map.into_iter().collect(),
            min_aspect_ratio: if min_ar.is_finite() { min_ar } else { 1.0 },
            max_aspect_ratio: max_ar,
            mean_aspect_ratio: if face_count > 0 {
                sum_ar / face_count as f64
            } else {
                1.0
            },
            surface_area,
        })
    }
}

/// Type alias for Catmull–Clark control and subdivided meshes.
pub type CatmullClarkMesh = Cage;

/// Quality and topology statistics for a subdivision cage.
#[derive(Clone, Debug, PartialEq)]
pub struct SubdQualityReport {
    pub vertex_count: usize,
    pub face_count: usize,
    pub extraordinary_vertex_count: usize,
    pub valence_histogram: Vec<(usize, usize)>,
    pub min_aspect_ratio: f64,
    pub max_aspect_ratio: f64,
    pub mean_aspect_ratio: f64,
    pub surface_area: f64,
}

#[cfg(test)]
#[path = "tests/limit_and_nurbs_tests_integration.rs"]
mod limit_and_nurbs_tests;

#[path = "cage_refinement.rs"]
mod cage_refinement;
