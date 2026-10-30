#![doc = include_str!("../README.md")]
//! Bounded Catmull–Clark refinement. Original polygon IDs survive refinement.
//! Tessellation is a neutral triangle buffer; mesh inspect/fit lives in the bridge.
use rustc_hash::{FxHashMap, FxHashSet};
pub(crate) type Point = math_core::V3;
#[cfg(feature = "codec")]
mod serialization;
pub use math_core::{Error, Result};
const INVALID_INPUT: &str = "SUBDIVISION_INVALID_INPUT";
pub(crate) fn error(message: impl Into<String>) -> Error {
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

pub(crate) fn avg(p: impl Iterator<Item = Point>, n: usize) -> Point {
    let mut r = [0.; 3];
    for q in p {
        for k in 0..3 {
            r[k] += q[k] / n as f64;
        }
    }
    r
}
pub(crate) type Edges = FxHashMap<(usize, usize), Vec<(usize, bool)>>;
pub(crate) fn edge(a: usize, b: usize) -> (usize, usize) {
    (a.min(b), a.max(b))
}
impl Cage {
    /// Edge map without any validation. Only for cages already proven valid:
    /// malformed indices or nonmanifold edges are the caller's responsibility.
    pub(crate) fn build_edges(&self) -> Edges {
        let mut edges = Edges::with_capacity_and_hasher(self.faces.len() * 2, Default::default());
        for (fi, f) in self.faces.iter().enumerate() {
            for i in 0..f.len() {
                let a = f[i];
                let b = f[(i + 1) % f.len()];
                edges.entry(edge(a, b)).or_default().push((fi, a < b));
            }
        }
        edges
    }
    /// Full input validation plus the edge map: budgets, face sanity,
    /// orientation/manifoldness, and per-vertex fan connectivity (BFS).
    pub(crate) fn topology(&self) -> Result<Edges> {
        if self.vertices.is_empty()
            || self.vertices.len() > 100_000
            || self.faces.is_empty()
            || self.faces.len() > 25_000
            || self
                .vertices
                .iter()
                .flatten()
                .any(|x| !x.is_finite() || x.abs() > 1e6)
        {
            return Err(error("Invalid control cage or budget exceeded"));
        }
        if self.faces.iter().map(Vec::len).sum::<usize>() > 100_000 {
            return Err(error("Control corner budget exceeded"));
        }
        let mut incidence = vec![FxHashSet::default(); self.vertices.len()];
        for (fi, f) in self.faces.iter().enumerate() {
            if f.len() < 3
                || f.len() > 64
                || f.iter().any(|&i| i >= self.vertices.len())
                || f.iter().collect::<FxHashSet<_>>().len() != f.len()
            {
                return Err(error("Invalid control face"));
            }
            for &a in f {
                incidence[a].insert(fi);
            }
        }
        let edges = self.build_edges();
        if edges
            .values()
            .any(|uses| uses.len() > 2 || uses.len() == 2 && uses[0].1 == uses[1].1)
        {
            return Err(error(
                "Cage must be consistently oriented and edge manifold",
            ));
        }
        let mut adjacency = vec![Vec::new(); self.vertices.len()];
        for entry in &edges {
            adjacency[entry.0.0].push(entry);
            adjacency[entry.0.1].push(entry);
        }
        for (v, faces) in incidence.iter().enumerate() {
            if faces.is_empty() || faces.len() > 256 {
                return Err(error("Unused control vertex"));
            }
            let local = &adjacency[v];
            let boundary = local.iter().filter(|(_, u)| u.len() == 1).count();
            if boundary != 0 && boundary != 2 {
                return Err(error("Nonmanifold vertex boundary"));
            }
            let mut visited = FxHashSet::default();
            visited.insert(*faces.iter().next().unwrap());
            loop {
                let before = visited.len();
                for (_, u) in local {
                    if u.iter().any(|(f, _)| visited.contains(f)) {
                        visited.extend(u.iter().map(|(f, _)| *f));
                    }
                }
                if before == visited.len() {
                    break;
                }
            }
            if visited.len() != faces.len() {
                return Err(error("Disconnected vertex fan"));
            }
        }
        Ok(edges)
    }
    pub fn validate(&self) -> Result<()> {
        self.topology().map(|_| ())
    }
    pub fn subdivide(&self, levels: usize) -> Result<Refined> {
        if levels > 5 {
            return Err(error("Subdivision levels must be 0..5"));
        }
        // Full input validation happens once, here at the public entry point.
        self.validate()?;
        self.subdivide_validated(levels)
    }
    /// Refinement loop over an already-valid cage. Each Catmull–Clark step of
    /// a valid cage yields a valid consistently oriented quad cage, so inner
    /// iterations reuse the cheap edge map instead of revalidating topology.
    pub(crate) fn subdivide_validated(&self, levels: usize) -> Result<Refined> {
        let mut c = self.clone();
        let mut ids: Vec<_> = (0..c.faces.len()).collect();
        for _ in 0..levels {
            let count: usize = c.faces.iter().map(Vec::len).sum();
            if count > 25_000 {
                return Err(error("Subdivision face budget exceeded"));
            }
            let edges = c.build_edges();
            let fp: Vec<_> = math_core::par_map(&c.faces, 64, |f| {
                avg(f.iter().map(|&i| c.vertices[i]), f.len())
            });
            let mut adjacency = vec![Vec::new(); c.vertices.len()];
            for entry in &edges {
                adjacency[entry.0.0].push(entry);
                adjacency[entry.0.1].push(entry);
            }
            let mut vertices = math_core::par_map_index(c.vertices.len(), 64, |v| {
                let p = c.vertices[v];
                let adjacent = &adjacency[v];
                let mut b0 = [0.0; 3];
                let mut b1 = [0.0; 3];
                let mut b_count = 0usize;
                for ((a, b), u) in adjacent {
                    if u.len() == 1 {
                        let nb = c.vertices[if *a == v { *b } else { *a }];
                        if b_count == 0 {
                            b0 = nb;
                        } else if b_count == 1 {
                            b1 = nb;
                        }
                        b_count += 1;
                    }
                }
                if b_count == 2 {
                    std::array::from_fn(|k| (6. * p[k] + b0[k] + b1[k]) / 8.)
                } else {
                    let faces: FxHashSet<_> = adjacent
                        .iter()
                        .flat_map(|(_, u)| u.iter().map(|(f, _)| *f))
                        .collect();
                    let n = adjacent.len();
                    let f = avg(faces.iter().map(|&i| fp[i]), faces.len());
                    let r = avg(
                        adjacent.iter().map(|((a, b), _)| {
                            avg([c.vertices[*a], c.vertices[*b]].into_iter(), 2)
                        }),
                        n,
                    );
                    std::array::from_fn(|k| (f[k] + 2. * r[k] + (n as f64 - 3.) * p[k]) / n as f64)
                }
            });
            vertices.reserve(edges.len() + fp.len());
            let mut ei = FxHashMap::with_capacity_and_hasher(edges.len(), Default::default());
            for (&(a, b), u) in &edges {
                ei.insert((a, b), vertices.len());
                let ep = if u.len() == 2 {
                    avg(
                        [c.vertices[a], c.vertices[b], fp[u[0].0], fp[u[1].0]].into_iter(),
                        4,
                    )
                } else {
                    avg([c.vertices[a], c.vertices[b]].into_iter(), 2)
                };
                vertices.push(ep);
            }
            let offset = vertices.len();
            vertices.extend(fp);
            let mut faces = Vec::with_capacity(count);
            let mut next_ids = Vec::with_capacity(count);
            for (fi, f) in c.faces.iter().enumerate() {
                for i in 0..f.len() {
                    faces.push(vec![
                        f[i],
                        ei[&edge(f[i], f[(i + 1) % f.len()])],
                        offset + fi,
                        ei[&edge(f[(i + f.len() - 1) % f.len()], f[i])],
                    ]);
                    next_ids.push(ids[fi]);
                }
            }
            c = Cage { vertices, faces };
            ids = next_ids;
        }
        Ok(Refined {
            cage: c,
            face_ids: ids,
        })
    }

    /// Catmull–Clark subdivision with DeRose semi-sharp edge and vertex creases.
    /// `edge_creases` maps `(a, b)` vertex indices to sharpness $s \in (0, 10]$,
    /// and `vertex_creases` maps vertex indices to sharpness $s \in (0, 10]$.
    /// At each refinement level, sharpness decays by $s \leftarrow \max(s - 1, 0)$,
    /// interpolating smoothly between sharp ($s \ge 1$) and smooth ($s = 0$) rules.
    pub fn subdivide_with_creases(
        &self,
        levels: usize,
        edge_creases: &[(usize, usize, f64)],
        vertex_creases: &[(usize, f64)],
    ) -> Result<Refined> {
        if levels > 5 {
            return Err(error("Subdivision levels must be 0..5"));
        }
        self.validate()?;
        let mut edge_sharp = FxHashMap::default();
        for &(a, b, s) in edge_creases {
            if a >= self.vertices.len()
                || b >= self.vertices.len()
                || a == b
                || !s.is_finite()
                || !(0.0..=10.0).contains(&s)
            {
                return Err(error("Invalid edge crease specification"));
            }
            if s > 0.0 {
                edge_sharp.insert(edge(a, b), s);
            }
        }
        let mut vert_sharp = FxHashMap::default();
        for &(v, s) in vertex_creases {
            if v >= self.vertices.len() || !s.is_finite() || !(0.0..=10.0).contains(&s) {
                return Err(error("Invalid vertex crease specification"));
            }
            if s > 0.0 {
                vert_sharp.insert(v, s);
            }
        }

        let mut c = self.clone();
        let mut ids: Vec<_> = (0..c.faces.len()).collect();
        for _ in 0..levels {
            let count: usize = c.faces.iter().map(Vec::len).sum();
            if count > 25_000 {
                return Err(error("Subdivision face budget exceeded"));
            }
            let edges = c.build_edges();
            let fp: Vec<_> = c
                .faces
                .iter()
                .map(|f| avg(f.iter().map(|&i| c.vertices[i]), f.len()))
                .collect();
            let mut vertices = Vec::with_capacity(c.vertices.len() + edges.len() + fp.len());
            let mut adjacency = vec![Vec::new(); c.vertices.len()];
            for entry in &edges {
                adjacency[entry.0.0].push(entry);
                adjacency[entry.0.1].push(entry);
            }
            for (v, &p) in c.vertices.iter().enumerate() {
                let adjacent = &adjacency[v];
                let boundary: Vec<_> = adjacent
                    .iter()
                    .filter(|(_, u)| u.len() == 1)
                    .map(|((a, b), _)| c.vertices[if *a == v { *b } else { *a }])
                    .collect();
                let faces: FxHashSet<_> = adjacent
                    .iter()
                    .flat_map(|(_, u)| u.iter().map(|(f, _)| *f))
                    .collect();
                let n = adjacent.len();
                let f = avg(faces.iter().map(|&i| fp[i]), faces.len());
                let r = avg(
                    adjacent.iter().map(|((a, b), _)| {
                        avg([c.vertices[*a], c.vertices[*b]].into_iter(), 2)
                    }),
                    n,
                );
                let smooth_q: Point = if boundary.len() == 2 {
                    std::array::from_fn(|k| (6. * p[k] + boundary[0][k] + boundary[1][k]) / 8.)
                } else {
                    std::array::from_fn(|k| (f[k] + 2. * r[k] + (n as f64 - 3.) * p[k]) / n as f64)
                };

                let sharp_incident: Vec<_> = adjacent
                    .iter()
                    .filter_map(|&(&(a, b), _)| {
                        edge_sharp.get(&(a, b)).copied().filter(|&s| s > 0.0).map(|s| {
                            let neighbor = c.vertices[if a == v { b } else { a }];
                            (neighbor, s)
                        })
                    })
                    .collect();
                let v_sharp = vert_sharp.get(&v).copied().unwrap_or(0.0);
                let q = if v_sharp > 0.0 || sharp_incident.len() >= 3 {
                    let s = if v_sharp > 0.0 {
                        v_sharp
                    } else {
                        sharp_incident.iter().map(|x| x.1).sum::<f64>() / sharp_incident.len() as f64
                    };
                    let t = s.clamp(0.0, 1.0);
                    std::array::from_fn(|k| (1.0 - t) * smooth_q[k] + t * p[k])
                } else if sharp_incident.len() == 2 {
                    let s = 0.5 * (sharp_incident[0].1 + sharp_incident[1].1);
                    let t = s.clamp(0.0, 1.0);
                    let crease_q: Point = std::array::from_fn(|k| {
                        (6.0 * p[k] + sharp_incident[0].0[k] + sharp_incident[1].0[k]) / 8.0
                    });
                    std::array::from_fn(|k| (1.0 - t) * smooth_q[k] + t * crease_q[k])
                } else {
                    smooth_q
                };
                vertices.push(q);
            }
            let mut ei = FxHashMap::with_capacity_and_hasher(edges.len(), Default::default());
            let mut next_edge_sharp = FxHashMap::default();
            for (&(a, b), u) in &edges {
                let e_idx = vertices.len();
                ei.insert((a, b), e_idx);
                let mid = avg([c.vertices[a], c.vertices[b]].into_iter(), 2);
                let smooth_e = if u.len() == 2 {
                    avg(
                        [c.vertices[a], c.vertices[b], fp[u[0].0], fp[u[1].0]].into_iter(),
                        4,
                    )
                } else {
                    mid
                };
                let s = edge_sharp.get(&(a, b)).copied().unwrap_or(0.0);
                let t = s.clamp(0.0, 1.0);
                let ep = std::array::from_fn(|k| (1.0 - t) * smooth_e[k] + t * mid[k]);
                vertices.push(ep);
                if s > 1.0 {
                    next_edge_sharp.insert(edge(a, e_idx), s - 1.0);
                    next_edge_sharp.insert(edge(b, e_idx), s - 1.0);
                }
            }
            let mut next_vert_sharp = FxHashMap::default();
            for (&v, &s) in &vert_sharp {
                if s > 1.0 {
                    next_vert_sharp.insert(v, s - 1.0);
                }
            }
            let offset = vertices.len();
            vertices.extend(fp);
            let mut faces = Vec::with_capacity(count);
            let mut next_ids = Vec::with_capacity(count);
            for (fi, f) in c.faces.iter().enumerate() {
                for i in 0..f.len() {
                    faces.push(vec![
                        f[i],
                        ei[&edge(f[i], f[(i + 1) % f.len()])],
                        offset + fi,
                        ei[&edge(f[(i + f.len() - 1) % f.len()], f[i])],
                    ]);
                    next_ids.push(ids[fi]);
                }
            }
            c = Cage { vertices, faces };
            ids = next_ids;
            edge_sharp = next_edge_sharp;
            vert_sharp = next_vert_sharp;
        }
        Ok(Refined {
            cage: c,
            face_ids: ids,
        })
    }

    /// Loop subdivision for triangle control cages ($C^2$ continuous on regular meshes).
    /// Each triangle is split into 4 triangles per refinement level.
    pub fn subdivide_loop(&self, levels: usize) -> Result<Refined> {
        if levels > 5 {
            return Err(error("Subdivision levels must be 0..5"));
        }
        self.validate()?;
        if self.faces.iter().any(|f| f.len() != 3) {
            return Err(error("Loop subdivision requires a pure triangle cage"));
        }
        let mut c = self.clone();
        let mut ids: Vec<_> = (0..c.faces.len()).collect();
        for _ in 0..levels {
            let count: usize = c.faces.iter().map(Vec::len).sum();
            if count > 25_000 {
                return Err(error("Subdivision face budget exceeded"));
            }
            let edges = c.build_edges();
            let mut adjacency = vec![Vec::new(); c.vertices.len()];
            for entry in &edges {
                adjacency[entry.0.0].push(entry);
                adjacency[entry.0.1].push(entry);
            }
            let mut vertices = math_core::par_map_index(c.vertices.len(), 64, |v| {
                let p = c.vertices[v];
                let adjacent = &adjacency[v];
                let mut b0 = [0.0; 3];
                let mut b1 = [0.0; 3];
                let mut b_count = 0usize;
                for ((a, b), u) in adjacent {
                    if u.len() == 1 {
                        let nb = c.vertices[if *a == v { *b } else { *a }];
                        if b_count == 0 {
                            b0 = nb;
                        } else if b_count == 1 {
                            b1 = nb;
                        }
                        b_count += 1;
                    }
                }
                if b_count == 2 {
                    std::array::from_fn(|k| (6. * p[k] + b0[k] + b1[k]) / 8.)
                } else {
                    let n = adjacent.len();
                    let beta = if n == 3 {
                        3.0 / 16.0
                    } else {
                        let cos_term = 3.0 / 8.0 + 0.25 * (2.0 * std::f64::consts::PI / n as f64).cos();
                        (5.0 / 8.0 - cos_term * cos_term) / n as f64
                    };
                    let mut sum = [0.0; 3];
                    for &(&(a, b), _) in adjacent {
                        let nb = c.vertices[if a == v { b } else { a }];
                        for k in 0..3 {
                            sum[k] += nb[k];
                        }
                    }
                    std::array::from_fn(|k| (1.0 - n as f64 * beta) * p[k] + beta * sum[k])
                }
            });
            vertices.reserve(edges.len());
            let mut ei = FxHashMap::with_capacity_and_hasher(edges.len(), Default::default());
            for (&(a, b), u) in &edges {
                ei.insert((a, b), vertices.len());
                let ep = if u.len() == 2 {
                    let opp = |fi: usize| -> Point {
                        let f = &c.faces[fi];
                        let idx = *f.iter().find(|&&v| v != a && v != b).unwrap();
                        c.vertices[idx]
                    };
                    let o0 = opp(u[0].0);
                    let o1 = opp(u[1].0);
                    std::array::from_fn(|k| {
                        0.375 * (c.vertices[a][k] + c.vertices[b][k]) + 0.125 * (o0[k] + o1[k])
                    })
                } else {
                    avg([c.vertices[a], c.vertices[b]].into_iter(), 2)
                };
                vertices.push(ep);
            }
            let mut faces = Vec::with_capacity(c.faces.len() * 4);
            let mut next_ids = Vec::with_capacity(c.faces.len() * 4);
            for (fi, f) in c.faces.iter().enumerate() {
                let (v0, v1, v2) = (f[0], f[1], f[2]);
                let e01 = ei[&edge(v0, v1)];
                let e12 = ei[&edge(v1, v2)];
                let e20 = ei[&edge(v2, v0)];
                faces.push(vec![v0, e01, e20]);
                faces.push(vec![v1, e12, e01]);
                faces.push(vec![v2, e20, e12]);
                faces.push(vec![e01, e12, e20]);
                next_ids.extend([ids[fi]; 4]);
            }
            c = Cage { vertices, faces };
            ids = next_ids;
        }
        Ok(Refined {
            cage: c,
            face_ids: ids,
        })
    }

    /// Loop / Warren $C^2$ triangle subdivision scheme returning the refined `Cage`.
    /// Automatically triangulates non-triangular control faces (e.g. quads) before refinement,
    /// splits each triangle into 4 per level, computes interior edge odd vertices via
    /// $\frac{3}{8}(v_0 + v_1) + \frac{1}{8}(v_{\text{opp0}} + v_{\text{opp1}})$ (or $\frac{1}{2}(v_0 + v_1)$ on boundaries),
    /// and updates even vertices with Loop's valence weight $\beta$.
    pub fn loop_subdivide(&self, levels: usize) -> Result<Cage> {
        if levels > 5 {
            return Err(error("Subdivision levels must be 0..5"));
        }
        self.validate()?;
        let base_cage = if self.faces.iter().all(|f| f.len() == 3) {
            self.clone()
        } else {
            let mut tri_faces = Vec::with_capacity(self.faces.len() * 2);
            for f in &self.faces {
                for i in 1..f.len() - 1 {
                    tri_faces.push(vec![f[0], f[i], f[i + 1]]);
                }
            }
            Cage {
                vertices: self.vertices.clone(),
                faces: tri_faces,
            }
        };
        Ok(base_cage.subdivide_loop(levels)?.cage)
    }
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
#[path = "lib/tests/mod.rs"]
mod tests;


mod construct_and_edit;
mod limit_and_quality;

pub use construct_and_edit::{Fit, fit};
pub use limit_and_quality::{BicubicPatch, CatmullClarkMesh, SubdQualityReport};
