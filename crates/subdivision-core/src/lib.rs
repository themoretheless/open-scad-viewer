//! Bounded Catmull–Clark refinement. Original polygon IDs survive refinement.
//! Tessellation is a neutral triangle buffer; mesh inspect/fit lives in the bridge.
#![feature(
    try_blocks,
    gen_blocks,
    yield_expr,
    super_let,
    deref_patterns,
    yeet_expr
)]
#![allow(unused_features)]
use rustc_hash::{FxHashMap, FxHashSet};
type Point = math_core::V3;
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
impl value_codec::Serialize for Cage {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "vertices".into(),
            value_codec::Serialize::to_value(&self.vertices),
        );
        object.insert(
            "faces".into(),
            value_codec::Serialize::to_value(&self.faces),
        );
        value_codec::Value::Object(object)
    }
}
impl<'de> value_codec::Deserialize<'de> for Cage {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let vertices: Vec<Point> = value_codec::Deserialize::from_value(
            object
                .remove("vertices")
                .ok_or_else(|| value_codec::error("Missing field vertices"))?,
        )?;
        let faces: Vec<Vec<usize>> = value_codec::Deserialize::from_value(
            object
                .remove("faces")
                .ok_or_else(|| value_codec::error("Missing field faces"))?,
        )?;
        Ok(Self { vertices, faces })
    }
}
#[derive(Clone, Debug)]
pub struct Refined {
    pub cage: Cage,
    pub face_ids: Vec<usize>,
}
impl value_codec::Serialize for Refined {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert("cage".into(), value_codec::Serialize::to_value(&self.cage));
        object.insert(
            "faceIds".into(),
            value_codec::Serialize::to_value(&self.face_ids),
        );
        value_codec::Value::Object(object)
    }
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
        let mut edges = Edges::default();
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
                let q = if boundary.len() == 2 {
                    std::array::from_fn(|k| (6. * p[k] + boundary[0][k] + boundary[1][k]) / 8.)
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
                };
                vertices.push(q);
            }
            let mut ei = FxHashMap::default();
            for (&(a, b), u) in &edges {
                ei.insert((a, b), vertices.len());
                let mut pts = vec![c.vertices[a], c.vertices[b]];
                if u.len() == 2 {
                    pts.extend(u.iter().map(|(f, _)| fp[*f]));
                }
                vertices.push(avg(pts.iter().copied(), pts.len()));
            }
            let offset = vertices.len();
            vertices.extend(fp);
            let mut faces = Vec::new();
            let mut next_ids = Vec::new();
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
}
impl Refined {
    /// Fan triangulation of convex faces. Concave/folded faces are not certified.
    pub fn triangulate(&self) -> Result<(geometry_ops::Triangles, Vec<usize>)> {
        // `Refined` fields are public, so callers may have mutated the cage:
        // this is a public entry point and keeps the full input validation.
        self.cage.validate()?;
        let mut indices = Vec::new();
        let mut ids = Vec::new();
        if self.face_ids.len() != self.cage.faces.len() {
            return Err(error("Invalid face identity count"));
        }
        for (fi, f) in self.cage.faces.iter().enumerate() {
            for i in 1..f.len() - 1 {
                indices.extend([f[0], f[i], f[i + 1]]);
                ids.push(self.face_ids[fi]);
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
        let mut faces = Vec::new();
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
        let mut vertices = Vec::new();
        for k in 0..segments {
            let a = 2. * std::f64::consts::PI * k as f64 / segments as f64;
            vertices.extend(
                profile
                    .iter()
                    .map(|p| [p[0] * a.cos(), p[0] * a.sin(), p[1]]),
            );
        }
        let mut faces = Vec::new();
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
