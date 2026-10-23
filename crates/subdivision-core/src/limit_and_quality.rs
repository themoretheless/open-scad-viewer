//! Exact Catmull–Clark limit surface evaluation (`evaluate_limit_points`, `evaluate_limit_normals`),
//! bicubic B-spline patch extraction (`BicubicPatch`, `to_nurbs_patches`), feature-adaptive refinement,
//! mesh quality diagnostics (`SubdQualityReport`), Laplacian/Taubin smoothing, and Doo–Sabin polyhedral duality.

use rustc_hash::FxHashSet;
use crate::{Cage, Edges, Point, Refined, Result, avg, edge, error};

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

    /// Builds flat CSR active-neighbor adjacency (`neighbor_offsets`, `neighbor_indices`)
    /// and `is_boundary: Vec<bool>` once before iterative smoothing passes.
    fn build_smoothing_csr(&self, edges: &Edges) -> (Vec<usize>, Vec<usize>, Vec<bool>) {
        let v_count = self.vertices.len();
        let mut is_boundary = vec![false; v_count];
        let mut all_deg = vec![0usize; v_count];
        let mut bnd_deg = vec![0usize; v_count];
        for (&(a, b), uses) in edges {
            all_deg[a] += 1;
            all_deg[b] += 1;
            if uses.len() == 1 {
                is_boundary[a] = true;
                is_boundary[b] = true;
                bnd_deg[a] += 1;
                bnd_deg[b] += 1;
            }
        }
        let mut neighbor_offsets = Vec::with_capacity(v_count + 1);
        neighbor_offsets.push(0);
        for v in 0..v_count {
            let deg = if is_boundary[v] { bnd_deg[v] } else { all_deg[v] };
            neighbor_offsets.push(neighbor_offsets[v] + deg);
        }
        let total_neighbors = *neighbor_offsets.last().unwrap_or(&0);
        let mut neighbor_indices = vec![0usize; total_neighbors];
        let mut cursor = neighbor_offsets[..v_count].to_vec();
        for (&(a, b), uses) in edges {
            let edge_is_bnd = uses.len() == 1;
            if !is_boundary[a] || edge_is_bnd {
                neighbor_indices[cursor[a]] = b;
                cursor[a] += 1;
            }
            if !is_boundary[b] || edge_is_bnd {
                neighbor_indices[cursor[b]] = a;
                cursor[b] += 1;
            }
        }
        (neighbor_offsets, neighbor_indices, is_boundary)
    }

    /// Applies umbrella-operator Laplacian smoothing ($p_i \leftarrow (1 - \lambda) p_i + \lambda \bar{p}_i$)
    /// for `iterations` passes over the control cage vertices. When `preserve_boundary` is `true`,
    /// vertices incident to boundary edges remain pinned; otherwise boundary vertices smooth along
    /// their boundary neighbors.
    pub fn laplacian_smooth(
        &self,
        iterations: usize,
        lambda: f64,
        preserve_boundary: bool,
    ) -> Result<Cage> {
        if iterations > 256 || !lambda.is_finite() || !(-2.0..=2.0).contains(&lambda) {
            return Err(error("Invalid Laplacian smoothing parameters"));
        }
        let edges = self.topology()?;
        if iterations == 0 || lambda.abs() <= 1e-15 {
            return Ok(self.clone());
        }
        let (neighbor_offsets, neighbor_indices, is_boundary) = self.build_smoothing_csr(&edges);
        let mut current = self.vertices.clone();
        let mut next = current.clone();
        for _ in 0..iterations {
            for v in 0..current.len() {
                if is_boundary[v] && preserve_boundary {
                    next[v] = current[v];
                    continue;
                }
                let start = neighbor_offsets[v];
                let end = neighbor_offsets[v + 1];
                let ring = &neighbor_indices[start..end];
                if ring.is_empty() {
                    next[v] = current[v];
                    continue;
                }
                let inv_n = 1.0 / ring.len() as f64;
                let mut centroid = [0.0; 3];
                for &nb in ring {
                    let q = current[nb];
                    centroid[0] += q[0];
                    centroid[1] += q[1];
                    centroid[2] += q[2];
                }
                let p = current[v];
                next[v] = [
                    p[0] + lambda * (centroid[0] * inv_n - p[0]),
                    p[1] + lambda * (centroid[1] * inv_n - p[1]),
                    p[2] + lambda * (centroid[2] * inv_n - p[2]),
                ];
            }
            std::mem::swap(&mut current, &mut next);
        }
        let out = Cage {
            vertices: current,
            faces: self.faces.clone(),
        };
        out.validate()?;
        Ok(out)
    }

    /// Applies volume-preserving Taubin $\lambda/\mu$ two-step low-pass smoothing
    /// (a shrinking Laplacian step with $\lambda > 0$ followed by an unshrinking step with
    /// $\mu < -\lambda$, e.g. $\lambda = 0.5, \mu = -0.53$) for `iterations` cycles.
    pub fn taubin_smooth(
        &self,
        iterations: usize,
        lambda: f64,
        mu: f64,
        preserve_boundary: bool,
    ) -> Result<Cage> {
        if iterations > 256
            || !lambda.is_finite()
            || !mu.is_finite()
            || !(0.0..=1.0).contains(&lambda)
            || !(-1.5..=0.0).contains(&mu)
        {
            return Err(error("Invalid Taubin smoothing parameters"));
        }
        let edges = self.topology()?;
        if iterations == 0 {
            return Ok(self.clone());
        }
        let (neighbor_offsets, neighbor_indices, is_boundary) = self.build_smoothing_csr(&edges);
        let step_pass = |verts: &[Point], out_verts: &mut [Point], weight: f64| {
            for v in 0..verts.len() {
                if is_boundary[v] && preserve_boundary {
                    out_verts[v] = verts[v];
                    continue;
                }
                let start = neighbor_offsets[v];
                let end = neighbor_offsets[v + 1];
                let ring = &neighbor_indices[start..end];
                if ring.is_empty() {
                    out_verts[v] = verts[v];
                    continue;
                }
                let inv_n = 1.0 / ring.len() as f64;
                let mut centroid = [0.0; 3];
                for &nb in ring {
                    let q = verts[nb];
                    centroid[0] += q[0];
                    centroid[1] += q[1];
                    centroid[2] += q[2];
                }
                let p = verts[v];
                out_verts[v] = [
                    p[0] + weight * (centroid[0] * inv_n - p[0]),
                    p[1] + weight * (centroid[1] * inv_n - p[1]),
                    p[2] + weight * (centroid[2] * inv_n - p[2]),
                ];
            }
        };
        let mut current = self.vertices.clone();
        let mut half = current.clone();
        for _ in 0..iterations {
            step_pass(&current, &mut half, lambda);
            step_pass(&half, &mut current, mu);
        }
        let out = Cage {
            vertices: current,
            faces: self.faces.clone(),
        };
        out.validate()?;
        Ok(out)
    }

    /// Constructs the topological dual polyhedron (`Cage`) where each original face centroid
    /// becomes a dual vertex and each interior original vertex of valence $k \ge 3$ becomes a
    /// consistently oriented $k$-gon dual face (Doo–Sabin / polyhedral duality).
    pub fn dual_mesh(&self) -> Result<Cage> {
        let edges = self.topology()?;
        // Dual vertices: one at each original face centroid
        let dual_vertices: Vec<Point> = self
            .faces
            .iter()
            .map(|f| avg(f.iter().map(|&vi| self.vertices[vi]), f.len()))
            .collect();

        // Group incident faces per vertex
        let mut vert_faces: Vec<Vec<usize>> = vec![Vec::new(); self.vertices.len()];
        for (fi, f) in self.faces.iter().enumerate() {
            for &v in f {
                vert_faces[v].push(fi);
            }
        }

        // Identify boundary vertices (any vertex incident to an edge with only 1 face use)
        let mut is_boundary = vec![false; self.vertices.len()];
        for (&(a, b), uses) in &edges {
            if uses.len() == 1 {
                is_boundary[a] = true;
                is_boundary[b] = true;
            }
        }

        let mut dual_faces = Vec::with_capacity(self.vertices.len());
        for (v, incident) in vert_faces.iter().enumerate() {
            if is_boundary[v] || incident.len() < 3 {
                continue;
            }
            let k = incident.len();
            let mut ring = Vec::with_capacity(k);
            let mut cur_f = incident[0];
            for _ in 0..k {
                ring.push(cur_f);
                let f = &self.faces[cur_f];
                let pos = f.iter().position(|&x| x == v).unwrap();
                // Incoming half-edge to `v` in `cur_f` is `u_prev -> v`.
                // Across edge `(u_prev, v)`, the adjacent face has the opposite half-edge `v -> u_prev`,
                // cycling counter-clockwise around outward vertex normal `v`.
                let u_prev = f[(pos + f.len() - 1) % f.len()];
                let uses = &edges[&edge(u_prev, v)];
                if uses.len() != 2 {
                    break;
                }
                let next_f = if uses[0].0 == cur_f {
                    uses[1].0
                } else {
                    uses[0].0
                };
                cur_f = next_f;
                if cur_f == incident[0] {
                    break;
                }
            }
            if ring.len() == k {
                dual_faces.push(ring);
            }
        }

        // Keep only referenced dual vertices (in case of open meshes where boundary faces aren't in interior dual rings)
        let mut remap = vec![usize::MAX; dual_vertices.len()];
        let mut compact_vertices = Vec::with_capacity(dual_vertices.len());
        for df in &mut dual_faces {
            for fi in df.iter_mut() {
                if remap[*fi] == usize::MAX {
                    remap[*fi] = compact_vertices.len();
                    compact_vertices.push(dual_vertices[*fi]);
                }
                *fi = remap[*fi];
            }
        }

        let out = Cage {
            vertices: compact_vertices,
            faces: dual_faces,
        };
        out.validate()?;
        Ok(out)
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
mod limit_and_nurbs_tests {
    use super::*;

    #[test]
    fn evaluate_limit_points_matches_converged_catmull_clark_refinement() {
        let cage = Cage::extrude(
            &[[0., 0., 0.], [2., 0., 0.], [2., 2., 0.], [0., 2., 0.]],
            [0., 0., 2.],
        )
        .unwrap();
        let exact_limit = cage.evaluate_limit_points().unwrap();
        let refined = cage.subdivide(5).unwrap();
        for (i, &p_inf) in exact_limit.iter().enumerate() {
            let p_sub = refined.cage.vertices[i];
            for k in 0..3 {
                assert!(
                    (p_inf[k] - p_sub[k]).abs() < 1e-3,
                    "vertex {i} axis {k}: exact={} vs refined={}",
                    p_inf[k],
                    p_sub[k]
                );
            }
        }
        let normals = cage.evaluate_limit_normals().unwrap();
        assert_eq!(normals.len(), 8);
        for (p, n) in exact_limit.iter().zip(&normals) {
            let radial = math_core::sub(*p, [1., 1., 1.]);
            assert!(math_core::dot(*n, radial) > 0.8);
            assert!((math_core::norm(*n) - 1.0).abs() < 1e-12);
        }
    }

    #[test]
    fn to_nurbs_patches_extracts_bicubic_patch_matching_limit_points() {
        // 3x3 grid of quads (16 vertices, 9 quad faces); center face (index 4) is regular interior valence-4.
        let mut vertices = Vec::new();
        for y in 0..4 {
            for x in 0..4 {
                let z = if (x == 1 || x == 2) && (y == 1 || y == 2) {
                    1.0
                } else {
                    0.0
                };
                vertices.push([x as f64, y as f64, z]);
            }
        }
        let mut faces = Vec::new();
        for y in 0..3 {
            for x in 0..3 {
                let i0 = y * 4 + x;
                faces.push(vec![i0, i0 + 1, i0 + 5, i0 + 4]);
            }
        }
        let cage = Cage { vertices, faces };
        let patches = cage.to_nurbs_patches().unwrap();
        assert_eq!(patches.len(), 1);
        assert_eq!(patches[0].face_index, 4);

        let limit_pts = cage.evaluate_limit_points().unwrap();
        // Corners of the center face (face 4 has vertices [5, 6, 10, 9]) must match patch(u, v) at {0,1}x{0,1}
        let corners = [
            (0.0, 0.0, 5),
            (1.0, 0.0, 6),
            (1.0, 1.0, 10),
            (0.0, 1.0, 9),
        ];
        for (u, v, vi) in corners {
            let eval = patches[0].evaluate(u, v);
            let lim = limit_pts[vi];
            for k in 0..3 {
                assert!(
                    (eval[k] - lim[k]).abs() < 1e-12,
                    "corner ({u},{v}) mismatch: {:?} vs {:?}",
                    eval,
                    lim
                );
            }
        }
    }

    #[test]
    fn adaptive_subdivide_and_quality_report_work() {
        let cube = CatmullClarkMesh::extrude(
            &[[0., 0., 0.], [2., 0., 0.], [2., 2., 0.], [0., 2., 0.]],
            [0., 0., 2.],
        )
        .unwrap();
        let report = cube.mesh_quality_report().unwrap();
        assert_eq!(report.vertex_count, 8);
        assert_eq!(report.face_count, 6);
        assert_eq!(report.extraordinary_vertex_count, 8); // 8 corners of valence 3
        assert_eq!(report.valence_histogram, vec![(3, 8)]);
        assert!((report.min_aspect_ratio - 1.0).abs() < 1e-12);
        assert!((report.surface_area - 24.0).abs() < 1e-12);

        // Flat grid has 0 dihedral angle -> adaptive_subdivide stops at level 0
        let flat = CatmullClarkMesh {
            vertices: vec![[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]],
            faces: vec![vec![0, 1, 2, 3]],
        };
        let flat_ref = flat.adaptive_subdivide(0.1, 4).unwrap();
        assert_eq!(flat_ref.cage.faces.len(), 1);

        // Cube has 90 deg (1.57 rad) dihedral angles -> refines until angle <= tol
        let cube_ref = cube.adaptive_subdivide(0.45, 4).unwrap();
        assert!(cube_ref.cage.faces.len() >= 96);

        // Laplacian and Taubin smoothing on a subdivided cube
        let sub1 = cube.subdivide(1).unwrap().cage;
        let lap = sub1.laplacian_smooth(4, 0.5, true).unwrap();
        let taubin = sub1.taubin_smooth(4, 0.5, -0.53, true).unwrap();
        let area_orig = sub1.mesh_quality_report().unwrap().surface_area;
        let area_lap = lap.mesh_quality_report().unwrap().surface_area;
        let area_taubin = taubin.mesh_quality_report().unwrap().surface_area;
        // Laplacian shrinks surface area significantly, whereas Taubin preserves area much better
        assert!(area_lap < area_orig * 0.7);
        assert!(area_taubin > area_lap * 1.3);

        // Polyhedral duality: dual of a cube (8 vertices of valence 3, 6 quad faces) is an octahedron (6 vertices of valence 4, 8 triangle faces)
        let octa = cube.dual_mesh().unwrap();
        assert_eq!(octa.vertices.len(), 6);
        assert_eq!(octa.faces.len(), 8);
        assert!(octa.faces.iter().all(|f| f.len() == 3));
        // And dual of that octahedron is a cube again (8 vertices, 6 quad faces)!
        let double_dual = octa.dual_mesh().unwrap();
        assert_eq!(double_dual.vertices.len(), 8);
        assert_eq!(double_dual.faces.len(), 6);
        assert!(double_dual.faces.iter().all(|f| f.len() == 4));
    }
}
