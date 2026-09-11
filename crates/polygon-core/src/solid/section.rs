//! Indexed horizontal mesh sections for toolpath planning.
//!
//! Coordinates are millimeters. The half-open rule is `min_z <= z < max_z`:
//! vertices on the plane belong to the lower side and horizontal triangles do
//! not emit segments. Thus a box includes its bottom section and excludes its
//! top. This rule is deterministic, not a repair or a solid-validity proof.
//!
//! Exactly coincident input vertices share identity (including signed zero).
//! Near vertices are never welded. Endpoints use that identity or a shared
//! mesh edge, rather than a coordinate tolerance. Open/branching graphs fail.
//! Closed contours retain winding and source triangles; they are NOT a planar
//! arrangement or printable regions. Intersections, shell containment and
//! material classification still require a subsequent validation stage.

use crate::{Error, Mesh, Result};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone)]
pub struct SectionContour {
    /// Closed implicitly: the last point connects to the first.
    pub points: Vec<[f64; 2]>,
    /// Source triangle for each outgoing contour segment, in matching order.
    pub source_triangles: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct MeshSection {
    pub z_mm: f64,
    pub contours: Vec<SectionContour>,
    /// Triangles actually intersected after interval-index pruning.
    pub candidate_triangles: usize,
}

#[derive(Debug)]
struct Triangle {
    vertices: [usize; 3],
    source: usize,
    min_z: f64,
    max_z: f64,
}

#[derive(Debug)]
struct Node {
    min_z: f64,
    max_z: f64,
    range: std::ops::Range<usize>,
    children: Option<(Box<Node>, Box<Node>)>,
}

impl Node {
    fn build(triangles: &[Triangle], start: usize) -> Self {
        let min_z = triangles
            .iter()
            .map(|t| t.min_z)
            .fold(f64::INFINITY, f64::min);
        let max_z = triangles
            .iter()
            .map(|t| t.max_z)
            .fold(f64::NEG_INFINITY, f64::max);
        let children = (triangles.len() > 16).then(|| {
            let mid = triangles.len() / 2;
            (
                Box::new(Self::build(&triangles[..mid], start)),
                Box::new(Self::build(&triangles[mid..], start + mid)),
            )
        });
        Self {
            min_z,
            max_z,
            range: start..start + triangles.len(),
            children,
        }
    }

    fn query(&self, z: f64, triangles: &[Triangle], out: &mut Vec<usize>) {
        if z < self.min_z || z >= self.max_z {
            return;
        }
        if let Some((left, right)) = &self.children {
            left.query(z, triangles, out);
            right.query(z, triangles, out);
        } else {
            for index in self.range.clone() {
                let t = &triangles[index];
                if t.min_z <= z && z < t.max_z {
                    out.push(index);
                }
            }
        }
    }
}

/// Owned immutable geometry and height index, reusable across layer queries.
/// Keeps no renderer buffers or process-local geometry handles.
#[derive(Debug)]
pub struct MeshSectionIndex {
    points: Vec<[f64; 3]>,
    triangles: Vec<Triangle>,
    root: Node,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Endpoint {
    Vertex(usize),
    Edge(usize, usize),
}

fn error(code: &'static str, message: &str) -> Error {
    Error {
        code,
        message: message.into(),
    }
}

impl MeshSectionIndex {
    pub fn new(mesh: &Mesh) -> Result<Self> {
        mesh.validate()?;
        let mut points = Vec::new();
        let mut identities = BTreeMap::new();
        let mut vertex_ids = Vec::with_capacity(mesh.positions.len() / 3);
        for p in mesh.positions.chunks_exact(3) {
            let p = [p[0], p[1], p[2]];
            let key = p.map(|v| if v == 0.0 { 0 } else { v.to_bits() });
            let id = *identities.entry(key).or_insert_with(|| {
                points.push(p);
                points.len() - 1
            });
            vertex_ids.push(id);
        }
        let mut triangles = Vec::with_capacity(mesh.indices.len() / 3);
        for (source, t) in mesh.indices.chunks_exact(3).enumerate() {
            let vertices = [vertex_ids[t[0]], vertex_ids[t[1]], vertex_ids[t[2]]];
            let z = vertices.map(|i| points[i][2]);
            let min_z = z.into_iter().fold(f64::INFINITY, f64::min);
            let max_z = z.into_iter().fold(f64::NEG_INFINITY, f64::max);
            triangles.push(Triangle {
                vertices,
                source,
                min_z,
                max_z,
            });
        }
        triangles.sort_by(|a, b| a.min_z.total_cmp(&b.min_z).then(a.source.cmp(&b.source)));
        let root = Node::build(&triangles, 0);
        Ok(Self {
            points,
            triangles,
            root,
        })
    }

    fn endpoint(&self, a: usize, b: usize, z: f64) -> Result<(Endpoint, [f64; 2])> {
        if self.points[a][2] == z {
            return Ok((Endpoint::Vertex(a), [self.points[a][0], self.points[a][1]]));
        }
        if self.points[b][2] == z {
            return Ok((Endpoint::Vertex(b), [self.points[b][0], self.points[b][1]]));
        }
        let (a, b) = (a.min(b), a.max(b));
        let (p, q) = (self.points[a], self.points[b]);
        let dz = q[2] - p[2];
        let t = (z - p[2]) / dz;
        let point = [p[0] * (1.0 - t) + q[0] * t, p[1] * (1.0 - t) + q[1] * t];
        if !dz.is_finite() || !t.is_finite() || !point.iter().all(|x| x.is_finite()) {
            return Err(error(
                "SECTION_NUMERIC_RANGE",
                "Section interpolation exceeded the numeric range",
            ));
        }
        if t <= 0.0 || t >= 1.0 {
            return Err(error(
                "SECTION_UNRESOLVED_EDGE",
                "Interior edge intersection rounded to an endpoint",
            ));
        }
        Ok((Endpoint::Edge(a, b), point))
    }

    pub fn section(&self, z_mm: f64) -> Result<MeshSection> {
        if !z_mm.is_finite() {
            return Err(error(
                "SECTION_INVALID_HEIGHT",
                "Section height must be finite",
            ));
        }
        let mut candidates = Vec::new();
        self.root.query(z_mm, &self.triangles, &mut candidates);
        let mut positions = BTreeMap::new();
        let mut outgoing = BTreeMap::new();
        let mut incoming = BTreeSet::new();
        for &index in &candidates {
            let triangle = &self.triangles[index];
            let mut start = None;
            let mut end = None;
            for i in 0..3 {
                let (a, b) = (triangle.vertices[i], triangle.vertices[(i + 1) % 3]);
                let above_a = self.points[a][2] > z_mm;
                if above_a != (self.points[b][2] > z_mm) {
                    let endpoint = self.endpoint(a, b, z_mm)?;
                    if above_a {
                        start = Some(endpoint);
                    } else {
                        end = Some(endpoint);
                    }
                }
            }
            if let (Some((a, p)), Some((b, q))) = (start, end) {
                if a == b {
                    continue;
                } // Isolated vertex contact, no area boundary.
                if p == q {
                    return Err(error(
                        "SECTION_UNRESOLVED_EDGE",
                        "Distinct section endpoints collapsed numerically",
                    ));
                }
                positions.insert(a, p);
                positions.insert(b, q);
                if outgoing.insert(a, (b, triangle.source)).is_some() || !incoming.insert(b) {
                    return Err(error(
                        "SECTION_AMBIGUOUS_BOUNDARY",
                        "Section has duplicate or branching directed edges",
                    ));
                }
            }
        }
        if outgoing.len() != incoming.len() || outgoing.keys().any(|key| !incoming.contains(key)) {
            return Err(error(
                "SECTION_OPEN_BOUNDARY",
                "Section contains an open boundary",
            ));
        }
        let mut contours = Vec::new();
        while let Some((&start, _)) = outgoing.first_key_value() {
            let mut at = start;
            let mut contour = SectionContour {
                points: Vec::new(),
                source_triangles: Vec::new(),
            };
            loop {
                let (next, source) = outgoing.remove(&at).ok_or_else(|| {
                    error(
                        "SECTION_AMBIGUOUS_BOUNDARY",
                        "Section cycle is inconsistent",
                    )
                })?;
                contour.points.push(positions[&at]);
                contour.source_triangles.push(source);
                at = next;
                if at == start {
                    break;
                }
            }
            if contour.points.len() < 3 {
                return Err(error(
                    "SECTION_DEGENERATE_BOUNDARY",
                    "Section boundary has fewer than three points",
                ));
            }
            contours.push(contour);
        }
        Ok(MeshSection {
            z_mm,
            contours,
            candidate_triangles: candidates.len(),
        })
    }
}
