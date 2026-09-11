//! Bounded Catmull–Clark refinement. Original polygon IDs survive refinement.
//! Tessellation is a neutral triangle buffer; mesh inspect/fit lives in the bridge.
use std::collections::{BTreeMap, BTreeSet};
type Point = [f64; 3];
pub use math_core::{Error, Result};
fn error(message: impl Into<String>) -> Error {
    Error::new("SUBDIVISION_INVALID_INPUT", message)
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
type Edges = BTreeMap<(usize, usize), Vec<(usize, bool)>>;
fn edge(a: usize, b: usize) -> (usize, usize) {
    (a.min(b), a.max(b))
}
impl Cage {
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
        let mut edges: Edges = BTreeMap::new();
        let mut incidence = vec![BTreeSet::new(); self.vertices.len()];
        for (fi, f) in self.faces.iter().enumerate() {
            if f.len() < 3
                || f.len() > 64
                || f.iter().any(|&i| i >= self.vertices.len())
                || f.iter().collect::<BTreeSet<_>>().len() != f.len()
            {
                return Err(error("Invalid control face"));
            }
            for i in 0..f.len() {
                let a = f[i];
                let b = f[(i + 1) % f.len()];
                edges.entry(edge(a, b)).or_default().push((fi, a < b));
                incidence[a].insert(fi);
            }
        }
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
            let mut visited = BTreeSet::from([*faces.first().unwrap()]);
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
        self.validate()?;
        let mut c = self.clone();
        let mut ids: Vec<_> = (0..c.faces.len()).collect();
        for _ in 0..levels {
            let count: usize = c.faces.iter().map(Vec::len).sum();
            if count > 25_000 {
                return Err(error("Subdivision face budget exceeded"));
            }
            let edges = c.topology()?;
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
                    let faces: BTreeSet<_> = adjacent
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
            let mut ei = BTreeMap::new();
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
    fn boundary_edges(t: &geometry_ops::Triangles) -> usize {
        let mut edges = BTreeMap::new();
        for tri in t.indices.chunks_exact(3) {
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
    let residual = |c: &Cage| -> Result<(f64, Vec<Point>)> {
        let r = c.subdivide(1)?;
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
