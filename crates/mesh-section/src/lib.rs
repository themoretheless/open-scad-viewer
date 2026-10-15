#![doc = include_str!("../README.md")]
//! Indexed horizontal mesh sections for CAD slice/project.
//!
//! Print walls/infill/G-code are `slicer-core`; this module only cuts contours.
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

pub use math_core::{Error, Result};
use math_core::{cross, sub};
pub use mesh_topology::MeshView;
fn check(condition: bool, message: &str) -> Result<()> {
    math_core::ensure(condition, "MESH_SECTION_INVALID_INPUT", message)
}
use planar_geometry::rings::{Rings, area, planar};
use std::collections::{BTreeMap, BTreeSet, HashMap};

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
    pub fn new(mesh: &MeshView<'_>) -> Result<Self> {
        mesh.validate()?;
        let mut points = Vec::new();
        let mut identities = BTreeMap::new();
        let mut vertex_ids = Vec::with_capacity(mesh.positions.len() / 3);
        for p in mesh.positions.as_chunks::<3>().0 {
            let p = [p[0], p[1], p[2]];
            let key = p.map(|v| if v == 0.0 { 0 } else { v.to_bits() });
            let id = *identities.entry(key).or_insert_with(|| {
                points.push(p);
                points.len() - 1
            });
            vertex_ids.push(id);
        }
        let mut triangles = Vec::with_capacity(mesh.indices.len() / 3);
        for (source, t) in mesh.indices.as_chunks::<3>().0.iter().enumerate() {
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
        self.section_impl(z_mm, false).map(|(section, _)| section)
    }

    /// Display-only contour: preserve distinct topological nodes even when their
    /// coordinates round to the same f64. Never weld or remove their segments.
    /// Return source triangles for every such segment; not suitable for toolpaths.
    pub fn section_for_display(&self, z_mm: f64) -> Result<(MeshSection, Vec<usize>)> {
        self.section_impl(z_mm, true)
    }

    fn section_impl(&self, z_mm: f64, display: bool) -> Result<(MeshSection, Vec<usize>)> {
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
        let mut collapsed = Vec::new();
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
                    if display {
                        collapsed.push(triangle.source);
                    } else {
                        return Err(error(
                            "SECTION_UNRESOLVED_EDGE",
                            "Distinct section endpoints collapsed numerically",
                        ));
                    }
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
        Ok((
            MeshSection {
                z_mm,
                contours,
                candidate_triangles: candidates.len(),
            },
            collapsed,
        ))
    }
}

pub fn project(mesh: &MeshView<'_>) -> Result<Rings> {
    mesh.validate()?;
    let mut triangles = Vec::new();
    for t in mesh.indices.as_chunks::<3>().0 {
        let mut r: Vec<_> = t
            .iter()
            .map(|&i| {
                let p = mesh.point(i).unwrap();
                [p[0], p[1]]
            })
            .collect();
        if area(&r).abs() < 1e-12 {
            continue;
        }
        if area(&r) < 0. {
            r.reverse()
        }
        triangles.push(r)
    }
    planar(&triangles, &vec![], "union")
}

pub fn slice(mesh: &MeshView<'_>, z: f64) -> Result<Rings> {
    mesh.validate()?;
    check(z.is_finite(), "Invalid slice height")?;
    let eps = 1e-8;
    let mut points = Vec::new();
    let mut ids = HashMap::new();
    let mut edges = BTreeSet::new();
    for t in mesh.indices.as_chunks::<3>().0 {
        let p = [mesh.point(t[0])?, mesh.point(t[1])?, mesh.point(t[2])?];
        let n = cross(sub(p[1], p[0]), sub(p[2], p[0]));
        let mut hits = Vec::new();
        for i in 0..3 {
            let a = p[i];
            let b = p[(i + 1) % 3];
            if (a[2] > z) != (b[2] > z) {
                let f = (z - a[2]) / (b[2] - a[2]);
                let h = [a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f];
                if hits
                    .iter()
                    .all(|v: &[f64; 2]| (v[0] - h[0]).hypot(v[1] - h[1]) > eps)
                {
                    hits.push(h)
                }
            }
        }
        if hits.len() != 2 {
            continue;
        }
        if (hits[1][0] - hits[0][0]) * (-n[1]) + (hits[1][1] - hits[0][1]) * n[0] < 0. {
            hits.swap(0, 1)
        }
        let mut index = |p: [f64; 2]| {
            *ids.entry(((p[0] / eps).round() as i64, (p[1] / eps).round() as i64))
                .or_insert_with(|| {
                    points.push(p);
                    points.len() - 1
                })
        };
        let a = index(hits[0]);
        let b = index(hits[1]);
        if a != b {
            edges.insert((a, b));
        }
    }
    let mut rings = vec![];
    while let Some(&(start, end)) = edges.iter().next() {
        edges.remove(&(start, end));
        let mut ring = vec![points[start]];
        let mut at = end;
        while at != start {
            ring.push(points[at]);
            check(ring.len() <= mesh.indices.len(), "Slice boundary budget")?;
            let next = edges.iter().find(|e| e.0 == at).copied();
            check(next.is_some(), "Open or ambiguous slice boundary")?;
            let edge = next.unwrap();
            edges.remove(&edge);
            at = edge.1;
        }
        if ring.len() >= 3 {
            rings.push(ring)
        }
    }
    planar(&rings, &vec![], "union")
}

#[derive(Debug, Clone, Copy)]
pub struct SvgProjectionMesh<'a> {
    /// Interleaved 6-float vertex buffer (`[x, y, z, nx, ny, nz, ...]`).
    pub vertices: &'a [f32],
    pub indices: &'a [u32],
    pub transform: [f64; 16],
    pub face_ids: &'a [u32],
}

pub fn bounded_svg_contours(contours: Rings) -> Result<Rings> {
    let total_points: usize = contours.iter().map(|ring| ring.len()).sum();
    if total_points > 20000 {
        return Err(error(
            "SVG_CONTOUR_LIMIT",
            "SVG exceeds 20000 contour points.",
        ));
    }
    if contours.is_empty()
        || contours
            .iter()
            .any(|ring| ring.len() < 3 || ring.iter().any(|p| !p[0].is_finite() || !p[1].is_finite()))
    {
        return Err(error("SVG_EMPTY_AREA", "SVG has no finite area."));
    }
    for ring in &contours {
        let [x, y] = ring[0];
        let mut twice_area = 0.0;
        for (i, a) in ring.iter().enumerate() {
            let b = ring[(i + 1) % ring.len()];
            twice_area += (a[0] - x) * (b[1] - y) - (b[0] - x) * (a[1] - y);
        }
        if !twice_area.is_finite() || twice_area == 0.0 {
            return Err(error("SVG_EMPTY_AREA", "SVG has no finite area."));
        }
    }
    Ok(contours)
}

pub fn svg_contours(
    meshes: &[SvgProjectionMesh<'_>],
    axis: Option<&str>,
    face_triangle: Option<usize>,
) -> Result<Rings> {
    if let Some(axis) = axis
        && !matches!(axis, "x" | "y" | "z")
    {
        return Err(error(
            "SVG_INVALID_AXIS",
            "SVG projection axis must be x, y or z.",
        ));
    }
    if let Some(triangle_index) = face_triangle
        && (meshes.len() != 1 || triangle_index * 3 + 2 >= meshes[0].indices.len())
    {
        return Err(error("SVG_INVALID_FACE", "Select a valid planar face."));
    }
    for mesh in meshes {
        if mesh.vertices.len() % 6 != 0
            || mesh.indices.len() % 3 != 0
            || !mesh.transform.iter().all(|v| v.is_finite())
            || mesh.transform[12] != 0.0
            || mesh.transform[13] != 0.0
            || mesh.transform[14] != 0.0
            || mesh.transform[15] != 1.0
            || (!mesh.face_ids.is_empty() && mesh.face_ids.len() != mesh.indices.len() / 3)
        {
            return Err(error(
                "SVG_INVALID_MESH",
                "SVG projection requires a valid triangle mesh and affine transform.",
            ));
        }
    }
    let face_id = face_triangle.and_then(|tri| meshes[0].face_ids.get(tri).copied());
    let includes_triangle = |mesh: &SvgProjectionMesh<'_>, triangle: usize| -> bool {
        match face_triangle {
            None => true,
            Some(target) => match face_id {
                None => triangle == target,
                Some(id) => mesh.face_ids.get(triangle).copied() == Some(id),
            },
        }
    };
    let mut triangle_count = 0usize;
    for mesh in meshes {
        for triangle in 0..(mesh.indices.len() / 3) {
            if includes_triangle(mesh, triangle) {
                triangle_count += 1;
                if triangle_count > 20000 {
                    return Err(error(
                        "SVG_TRIANGLE_LIMIT",
                        "SVG projection is limited to 20000 triangles.",
                    ));
                }
            }
        }
    }
    let point = |mesh: &SvgProjectionMesh<'_>, index: usize| -> Result<[f64; 3]> {
        let vertex = mesh.indices[index] as usize;
        if vertex * 6 + 2 >= mesh.vertices.len() {
            return Err(error(
                "SVG_INVALID_VERTEX_INDEX",
                "SVG projection contains an invalid vertex index.",
            ));
        }
        let x = mesh.vertices[vertex * 6] as f64;
        let y = mesh.vertices[vertex * 6 + 1] as f64;
        let z = mesh.vertices[vertex * 6 + 2] as f64;
        let m = &mesh.transform;
        let w = m[12] * x + m[13] * y + m[14] * z + m[15];
        let iw = if w.is_finite() && w.abs() > 1e-12 {
            1.0 / w
        } else {
            1.0
        };
        let pos = [
            (m[0] * x + m[1] * y + m[2] * z + m[3]) * iw,
            (m[4] * x + m[5] * y + m[6] * z + m[7]) * iw,
            (m[8] * x + m[9] * y + m[10] * z + m[11]) * iw,
        ];
        if !pos.iter().all(|v| v.is_finite()) {
            return Err(error(
                "SVG_NONFINITE_COORDINATES",
                "SVG projection contains non-finite coordinates.",
            ));
        }
        Ok(pos)
    };
    let mut origin = [0.0, 0.0, 0.0];
    let mut u = if axis == Some("x") {
        [0.0, 1.0, 0.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let mut v = if axis == Some("z") || axis.is_none() {
        [0.0, 1.0, 0.0]
    } else {
        [0.0, 0.0, 1.0]
    };
    let mut normal: Option<[f64; 3]> = None;
    if let Some(triangle) = face_triangle {
        let mesh = &meshes[0];
        origin = point(mesh, triangle * 3)?;
        let b = point(mesh, triangle * 3 + 1)?;
        let c = point(mesh, triangle * 3 + 2)?;
        let ab = sub(b, origin);
        let ac = sub(c, origin);
        let length = ab[0].hypot(ab[1]).hypot(ab[2]);
        let n = cross(ab, ac);
        let normal_length = n[0].hypot(n[1]).hypot(n[2]);
        if !(normal_length > 0.0) || !normal_length.is_finite() {
            return Err(error("SVG_DEGENERATE_FACE", "Degenerate face."));
        }
        u = [ab[0] / length, ab[1] / length, ab[2] / length];
        let unit_n = [
            n[0] / normal_length,
            n[1] / normal_length,
            n[2] / normal_length,
        ];
        v = cross(unit_n, u);
        normal = Some(unit_n);
    }
    let mut triangles = Vec::new();
    for mesh in meshes {
        for i in (0..mesh.indices.len()).step_by(3) {
            if !includes_triangle(mesh, i / 3) {
                continue;
            }
            let mut ring = Vec::with_capacity(3);
            for j in 0..3 {
                let p = sub(point(mesh, i + j)?, origin);
                if let Some(n) = normal
                    && (p[0] * n[0] + p[1] * n[1] + p[2] * n[2]).abs() > 1e-5
                {
                    return Err(error(
                        "SVG_NONPLANAR_FACE",
                        "The selected surface is not planar; use a projection.",
                    ));
                }
                ring.push([
                    p[0] * u[0] + p[1] * u[1] + p[2] * u[2],
                    p[0] * v[0] + p[1] * v[1] + p[2] * v[2],
                ]);
            }
            let area = (ring[1][0] - ring[0][0]) * (ring[2][1] - ring[0][1])
                - (ring[1][1] - ring[0][1]) * (ring[2][0] - ring[0][0]);
            if !area.is_finite() {
                return Err(error(
                    "SVG_NUMERIC_RANGE",
                    "SVG projection coordinates exceed the numeric range.",
                ));
            }
            if area.abs() > 1e-12 {
                if area <= 0.0 {
                    ring.reverse();
                }
                triangles.push(ring);
            }
        }
    }
    if triangles.is_empty() {
        return bounded_svg_contours(Vec::new());
    }
    bounded_svg_contours(planar_geometry::rings::nonzero(&triangles)?)
}

#[cfg(test)]
mod display_precision_tests {
    use super::*;
    #[test]
    fn display_preserves_topological_nodes_while_strict_section_refuses_collapse() {
        let mesh = MeshView {
            positions: &[1., 1., 0., 2., 1., 1., 1., 2., 1., 0., 0., 1.],
            indices: &[0, 1, 3, 0, 2, 1, 0, 3, 2, 1, 2, 3],
            uv: None,
        };
        let index = MeshSectionIndex::new(&mesh).unwrap();
        assert_eq!(
            index.section(1e-17).unwrap_err().code,
            "SECTION_UNRESOLVED_EDGE"
        );
        let (section, collapsed) = index.section_for_display(1e-17).unwrap();
        assert_eq!(collapsed.len(), 3);
        assert_eq!(section.contours.len(), 1);
        assert_eq!(section.contours[0].points.len(), 3);
        assert_eq!(section.contours[0].source_triangles.len(), 3);
        assert!(section.contours[0].points.iter().all(|p| *p == [1., 1.]));
        let (normal, warnings) = index.section_for_display(0.5).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(
            normal.contours[0].points,
            index.section(0.5).unwrap().contours[0].points
        );
    }
}
