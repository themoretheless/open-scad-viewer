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
impl Cage {
    /// Edge map without any validation. Only for cages already proven valid:
    /// malformed indices or nonmanifold edges are the caller's responsibility.
    fn build_edges(&self) -> Edges {
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
    fn topology(&self) -> Result<Edges> {
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
    fn subdivide_validated(&self, levels: usize) -> Result<Refined> {
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
mod tests {
    use super::*;
    fn square() -> Cage {
        Cage {
            vertices: vec![[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]],
            faces: vec![vec![0, 1, 2, 3]],
        }
    }
    #[test]
    fn boundary_rule_and_identity() {
        let r = square().subdivide(2).unwrap();
        assert_eq!(r.cage.faces.len(), 16);
        assert!(r.face_ids.iter().all(|x| *x == 0));
        let first = square().subdivide(1).unwrap();
        assert_eq!(first.cage.vertices[0], [0.125, 0.125, 0.]);
        assert_eq!(boundary_edges(&first.triangulate().unwrap().0), 8);
    }
    #[test]
    fn invalid_inputs() {
        let mut c = square();
        c.faces.push(vec![0, 1, 2]);
        assert!(c.validate().is_err());
        assert!(square().subdivide(6).is_err());
    }
    #[test]
    fn brush_edits_cage_vertices_locally_and_preserves_faces() {
        let cage = Cage::extrude(
            &[[0., 0., 0.], [2., 0., 0.], [2., 2., 0.], [0., 2., 0.]],
            [0., 0., 2.],
        )
        .unwrap();
        let b = geometry_ops::Brush {
            center: [2., 2., 2.],
            radius: 0.5,
            displacement: [0., 0., 1.],
        };
        let out = cage.brush(&b).unwrap();
        assert_eq!(out.faces, cage.faces);
        assert_eq!(out.vertices.len(), cage.vertices.len());
        let moved: Vec<_> = cage
            .vertices
            .iter()
            .zip(&out.vertices)
            .filter(|(a, b)| a != b)
            .collect();
        assert_eq!(moved.len(), 1);
        assert_eq!(*moved[0].0, [2., 2., 2.]);
        assert_eq!(*moved[0].1, [2., 2., 3.]);
        assert!(out.subdivide(2).unwrap().triangulate().is_ok());
    }
    #[test]
    fn sculpt_uses_cage_normals_and_keeps_faces() {
        use geometry_ops::{Falloff, SculptBrush, SculptKind};
        let cage = Cage::extrude(
            &[[0., 0., 0.], [2., 0., 0.], [2., 2., 0.], [0., 2., 0.]],
            [0., 0., 2.],
        )
        .unwrap();
        let geometry_ops::SculptTarget {
            normals,
            adjacency: rings,
            ..
        } = cage.sculpt_target().unwrap();
        assert_eq!(normals.len(), 8);
        assert!(rings.iter().all(|r| r.len() == 3));
        for (p, n) in cage.vertices.iter().zip(&normals) {
            assert!(math_core::dot(*n, math_core::sub(*p, [1., 1., 1.])) > 0.);
        }
        let inflate = cage
            .sculpt(&SculptBrush {
                falloff: Falloff::Constant,
                ..SculptBrush::new(SculptKind::Inflate { strength: 1. }, [1., 1., 1.], 100.)
            })
            .unwrap();
        assert_eq!(inflate.faces, cage.faces);
        for (p, q) in cage.vertices.iter().zip(&inflate.vertices) {
            assert!((math_core::norm(math_core::sub(*q, *p)) - 1.).abs() < 1e-12);
        }
        let smooth = cage
            .sculpt(&SculptBrush::new(
                SculptKind::Smooth { strength: 1. },
                [2., 2., 2.],
                0.5,
            ))
            .unwrap();
        let mean = [
            (2. + 2. + 0.) / 3.,
            (0. + 2. + 2.) / 3.,
            (2. + 0. + 2.) / 3.,
        ];
        let moved = smooth
            .vertices
            .iter()
            .zip(&cage.vertices)
            .find(|(a, b)| a != b)
            .unwrap()
            .0;
        for k in 0..3 {
            assert!((moved[k] - mean[k]).abs() < 1e-12);
        }
        assert!(smooth.subdivide(2).unwrap().triangulate().is_ok());
        assert!(
            cage.sculpt(&SculptBrush::new(
                SculptKind::Pinch { strength: 2. },
                [0.; 3],
                1.
            ))
            .is_err()
        );
    }
    #[test]
    fn brush_rejects_invalid_brush_and_invalid_cage() {
        let b = geometry_ops::Brush {
            center: [0.; 3],
            radius: 1.,
            displacement: [0., 0., 1.],
        };
        assert!(
            square()
                .brush(&geometry_ops::Brush {
                    radius: 0.,
                    ..b.clone()
                })
                .is_err()
        );
        assert!(
            square()
                .brush(&geometry_ops::Brush {
                    displacement: [f64::NAN; 3],
                    ..b.clone()
                })
                .is_err()
        );
        let mut broken = square();
        broken.faces.push(vec![0, 1, 2]);
        assert!(broken.brush(&b).is_err());
        assert_eq!(square().brush(&b).unwrap().vertices[0], [0., 0., 1.]);
    }
    fn boundary_edges(t: &geometry_ops::Triangles) -> usize {
        let mut edges = std::collections::BTreeMap::new();
        for tri in t.indices.as_chunks::<3>().0 {
            for (a, b) in [(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])] {
                let key = (a.min(b), a.max(b));
                *edges.entry(key).or_insert(0) += 1;
            }
        }
        edges.values().filter(|&&n| n == 1).count()
    }
    #[test]
    fn semi_sharp_creases_pin_corners_and_edges() {
        let cage = Cage::extrude(
            &[[0., 0., 0.], [2., 0., 0.], [2., 2., 0.], [0., 2., 0.]],
            [0., 0., 2.],
        )
        .unwrap();
        let smooth = cage.subdivide(2).unwrap();
        let creased = cage
            .subdivide_with_creases(
                2,
                &[(0, 1, 10.0), (1, 2, 10.0), (2, 3, 10.0), (3, 0, 10.0)],
                &[(0, 10.0)],
            )
            .unwrap();
        // Vertex 0 has sharpness 10.0 so it stays pinned at [0., 0., 0.]
        assert_eq!(creased.cage.vertices[0], [0., 0., 0.]);
        assert_ne!(smooth.cage.vertices[0], [0., 0., 0.]);
    }
    #[test]
    fn loop_subdivision_refines_triangle_cage() {
        let tetra = Cage {
            vertices: vec![
                [0., 0., 0.],
                [2., 0., 0.],
                [1., 2., 0.],
                [1., 0.8, 2.],
            ],
            faces: vec![
                vec![0, 2, 1],
                vec![0, 1, 3],
                vec![1, 2, 3],
                vec![2, 0, 3],
            ],
        };
        let refined = tetra.subdivide_loop(2).unwrap();
        assert_eq!(refined.cage.faces.len(), 64);
        assert!(refined.cage.faces.iter().all(|f| f.len() == 3));
        assert!(refined.triangulate().is_ok());
    }
}

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
}
