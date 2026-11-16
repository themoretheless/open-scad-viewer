//! Closed incidence shell assembled from exact transition/source geometry.
//! Not a global embedding, radius/thickness, volume or solid-admission owner.
use super::*;
use crate::{Model, TopoId};
use nurbs_core::surface::Surface;
use std::collections::BTreeMap;
pub(super) struct ImportedUse {
    pub(super) uv: [[Q; 2]; 2],
    pub(super) edge: usize,
    pub(super) reversed: bool,
    pub(super) root: Arc<RootCurve>,
    pub(super) range: [Q; 2],
}
pub struct ImportedFace {
    pub(super) surface: Surface,
    pub(super) uses: Vec<ImportedUse>,
    id: TopoId,
}
impl ImportedFace {
    pub fn source(&self) -> &Surface {
        &self.surface
    }
    pub fn source_id(&self) -> TopoId {
        self.id
    }
}
pub struct ClosedShell {
    topology: RoofTopology,
    geometry: GeometryReport,
    roof: Roof,
    source: Model,
    source_fields: Vec<SourceFieldReport>,
    edge_links: Vec<OriginalEdgeLink>,
    vertex_links: Vec<(TopoId, usize)>,
    face_links: Vec<(TopoId, usize)>,
}
impl ClosedShell {
    pub fn topology(&self) -> &RoofTopology {
        &self.topology
    }
    pub fn geometry(&self) -> &GeometryReport {
        &self.geometry
    }
    pub fn inherited_roof(&self) -> &Roof {
        &self.roof
    }
    pub fn original_source(&self) -> &Model {
        &self.source
    }
    pub fn imported_fields(&self) -> &[SourceFieldReport] {
        &self.source_fields
    }
    pub fn source_edge_links(&self) -> &[OriginalEdgeLink] {
        &self.edge_links
    }
    pub fn source_vertex_links(&self) -> &[(TopoId, usize)] {
        &self.vertex_links
    }
    pub fn source_face_links(&self) -> &[(TopoId, usize)] {
        &self.face_links
    }
}
fn cross(a: &[Q; 2], b: &[Q; 2], c: &[Q; 2]) -> Q {
    (&b[0] - &a[0]) * (&c[1] - &a[1]) - (&b[1] - &a[1]) * (&c[0] - &a[0])
}
fn convex(lines: &[[[Q; 2]; 2]]) -> Result<()> {
    require(
        (3..=128).contains(&lines.len()),
        "Unsupported original UV polygon size",
    )?;
    let mut area = Q::zero();
    for (i, line) in lines.iter().enumerate() {
        require(
            line[0] != line[1] && line[1] == lines[(i + 1) % lines.len()][0],
            "Original UV polygon is not exactly closed",
        )?;
        for other in lines {
            require(
                cross(&line[0], &line[1], &other[0]) >= Q::zero(),
                "Original UV polygon is not convex CCW",
            )?;
        }
        area += &line[0][0] * &line[1][1] - &line[0][1] * &line[1][0];
    }
    require(area > Q::zero(), "Original UV polygon has no positive area")
}
fn curve_points(c: &nurbs_core::curve::Curve) -> Result<Vec<[Q; 3]>> {
    c.validate()?;
    let d = c.domain();
    require(
        !c.periodic
            && c.control_points.len() == c.degree + 1
            && c.control_points.iter().all(|p| p.len() == 3)
            && c.weights.iter().all(|w| *w == c.weights[0])
            && c.knots[..=c.degree].iter().all(|x| *x == d[0])
            && c.knots[c.degree + 1..].iter().all(|x| *x == d[1]),
        "Original world edge needs a clamped polynomial Bezier",
    )?;
    Ok(c.control_points
        .iter()
        .map(|p| std::array::from_fn(|k| Q::from_float(p[k]).unwrap()))
        .collect())
}
/// One roof edge fragment of an original edge: roof edge, reversal relative
/// to the source edge and the consumed source parameter range.
type Fragment = (usize, bool, [Q; 2]);
fn full_range() -> [Q; 2] {
    [Q::zero(), Q::one()]
}
pub(super) fn exact_point(point: [f64; 3]) -> Result<[Q; 3]> {
    let coords = point.map(Q::from_float);
    require(
        coords.iter().all(Option::is_some),
        "Original vertex coordinate is not finite",
    )?;
    Ok(coords.map(Option::unwrap_or_default))
}
/// The single closed original outer shell containing both source supports.
fn select_original_shell(source: &Model, source_faces: [usize; 2]) -> Result<usize> {
    let shells = source
        .shells
        .iter()
        .enumerate()
        .filter(|(_, s)| {
            source_faces
                .iter()
                .all(|f| s.faces.iter().any(|u| u.face == *f))
        })
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    require(
        shells.len() == 1,
        "Source supports do not select one original shell",
    )?;
    let index = shells[0];
    require(
        source.shells[index].closed
            && source
                .bodies
                .iter()
                .any(|b| b.outer_shell == index && b.inner_shells.is_empty()),
        "Expected an original closed outer shell without cavities",
    )?;
    Ok(index)
}
/// Mutable state while original faces are imported onto the roof topology.
struct ShellCloser<'a> {
    source: &'a Model,
    topology: RoofTopology,
    source_edges: BTreeMap<usize, Vec<Fragment>>,
    source_vertices: BTreeMap<usize, usize>,
    edge_links: Vec<OriginalEdgeLink>,
    face_links: Vec<(TopoId, usize)>,
    fields: Vec<SourceFieldReport>,
}
impl<'a> ShellCloser<'a> {
    fn new(
        roof: &Roof,
        source: &'a Model,
        source_faces: [usize; 2],
        bindings: &OriginalBoundaryMap,
    ) -> Self {
        let topology = roof.topology().clone();
        let mut face_links = Vec::new();
        for (face, f) in topology.faces.iter().enumerate() {
            if let FaceGeometry::Retained(region) = &f.surface {
                face_links.push((source.1.faces[source_faces[region.side()]], face));
            }
        }
        let mut source_edges = BTreeMap::new();
        for link in bindings.edges() {
            if roof.geometry().open_edges.contains(&link.roof_edge) {
                source_edges
                    .entry(link.source_edge)
                    .or_insert_with(Vec::new)
                    .push((
                        link.roof_edge,
                        link.reversed_from_source,
                        link.source_range.clone(),
                    ));
            }
        }
        Self {
            source,
            topology,
            source_edges,
            source_vertices: BTreeMap::new(),
            edge_links: bindings.edges().to_vec(),
            face_links,
            fields: Vec::new(),
        }
    }
    /// A neighboring face must consume the complete original edge exactly
    /// once. Gaps, overlaps and differently rooted joins are never welded.
    fn check_partitions(&mut self) -> Result<()> {
        for children in self.source_edges.values_mut() {
            children.sort_by(|a, b| a.2[0].cmp(&b.2[0]));
            let complete = match (children.first(), children.last()) {
                (Some(first), Some(last)) => first.2[0] == Q::zero() && last.2[1] == Q::one(),
                _ => false,
            };
            require(complete, "Incomplete outer source edge partition")?;
            for pair in children.windows(2) {
                let (a, ar, range) = &pair[0];
                let (z, zr, next) = &pair[1];
                let left = &self.topology.edges[*a];
                let right = &self.topology.edges[*z];
                require(
                    range[1] == next[0]
                        && left.vertices[usize::from(!*ar)] == right.vertices[usize::from(*zr)]
                        && Arc::ptr_eq(&left.curve.root, &right.curve.root),
                    "Original edge partition has a gap, overlap or conflicting root endpoint",
                )?;
            }
        }
        Ok(())
    }
    fn bind_vertices(&mut self, bindings: &OriginalBoundaryMap) -> Result<()> {
        for p in bindings.endpoints() {
            if let Some(old) = p.source_vertex {
                if let Some(&existing) = self.source_vertices.get(&old) {
                    require(
                        existing == p.roof_vertex,
                        "Original vertex maps to conflicting roof identities",
                    )?;
                } else {
                    self.source_vertices.insert(old, p.roof_vertex);
                }
            }
        }
        Ok(())
    }
    /// Roof vertex for an original vertex, created on first use.
    fn import_vertex(&mut self, id: usize) -> Result<usize> {
        let p = exact_point(self.source.vertices[id].point)?;
        if let Some(&i) = self.source_vertices.get(&id) {
            require(
                self.topology.vertices[i].point == p,
                "Original named vertex changed world geometry",
            )?;
            return Ok(i);
        }
        let i = self.topology.vertices.len();
        self.topology.vertices.push(brep_topology::Vertex { point: p });
        self.source_vertices.insert(id, i);
        Ok(i)
    }
    /// Roof fragments of an original edge, importing the whole edge when no
    /// roof edge consumed it yet.
    fn edge_fragments(&mut self, source_edge: usize) -> Result<Vec<Fragment>> {
        if let Some(mapped) = self.source_edges.get(&source_edge) {
            return Ok(mapped.clone());
        }
        let source = self.source;
        let old = &source.edges[source_edge];
        let points = curve_points(&old.curve)?;
        let vertices = [
            self.import_vertex(old.vertices[0])?,
            self.import_vertex(old.vertices[1])?,
        ];
        let id = self.topology.edges.len();
        self.topology.edges.push(brep_topology::Edge {
            degenerate: false,
            vertices,
            curve: Edge::original(points, id),
        });
        self.source_edges
            .insert(source_edge, vec![(id, false, full_range())]);
        self.edge_links.push(OriginalEdgeLink {
            roof_edge: id,
            source_edge,
            source_id: source.1.edges[source_edge],
            source_range: full_range(),
            reversed_from_source: false,
        });
        Ok(vec![(id, false, full_range())])
    }
    /// Imports one original face (`face`, used with `face_reversed`).
    fn import_face(&mut self, face: usize, face_reversed: bool) -> Result<()> {
        let source = self.source;
        let original = &source.faces[face];
        require(
            original.holes.is_empty(),
            "Original source holes require separate import qualification",
        )?;
        let cs = &source.loops[original.outer].coedges;
        let lines = cs
            .iter()
            .map(|c| super::original_boundary::traversal_uv(c, &source.edges[c.edge].curve))
            .collect::<Result<Vec<_>>>()?;
        convex(&lines)?;
        let field = inspect_source_support_field(&original.surface)?;
        let mut uses = Vec::new();
        let mut coedges = Vec::new();
        for (c, uv) in cs.iter().zip(lines) {
            let mut fragments = self.edge_fragments(c.edge)?;
            if c.reversed {
                fragments.reverse();
            }
            for (edge, relation, range) in fragments {
                let interval = if c.reversed {
                    [Q::one() - &range[1], Q::one() - &range[0]]
                } else {
                    range
                };
                let part = interval
                    .map(|t| std::array::from_fn(|k| &uv[0][k] + (&uv[1][k] - &uv[0][k]) * &t));
                let reversed = c.reversed ^ relation;
                let index = uses.len();
                uses.push(ImportedUse {
                    uv: part,
                    edge,
                    reversed,
                    root: self.topology.edges[edge].curve.root.clone(),
                    range: self.topology.edges[edge].curve.range.clone(),
                });
                coedges.push(brep_topology::Coedge::new(
                    edge,
                    reversed,
                    FaceUseGeometry::Imported(index),
                ));
            }
        }
        let new_face = self.topology.faces.len();
        let outer = self.topology.loops.len();
        self.topology.loops.push(brep_topology::Loop { coedges });
        self.topology.faces.push(brep_topology::Face {
            surface: FaceGeometry::Imported(Arc::new(ImportedFace {
                surface: original.surface.clone(),
                uses,
                id: source.1.faces[face],
            })),
            outer,
            holes: vec![],
        });
        self.topology.shells[0].faces.push(brep_topology::FaceUse {
            face: new_face,
            reversed: face_reversed,
        });
        self.fields.push(field);
        self.face_links.push((source.1.faces[face], new_face));
        Ok(())
    }
}
impl Roof {
    /// Closed topology with exact saved lifts and all-support curvature bounds.
    /// Bodies remains empty pending global geometry/volume admission.
    pub fn close_original_shell(
        self,
        source: &Model,
        source_faces: [usize; 2],
        budget: &mut Budget,
    ) -> Result<ClosedShell> {
        source.validate_topology()?;
        let bindings = self.bind_original_boundaries(source, source_faces, budget)?;
        require(
            bindings.is_current(source, &self),
            "Original boundary bindings changed",
        )?;
        let shell = &source.shells[select_original_shell(source, source_faces)?];
        let mut closer = ShellCloser::new(&self, source, source_faces, &bindings);
        closer.check_partitions()?;
        closer.bind_vertices(&bindings)?;
        for face_use in &shell.faces {
            if !source_faces.contains(&face_use.face) {
                closer.import_face(face_use.face, face_use.reversed)?;
            }
        }
        let ShellCloser {
            mut topology,
            source_vertices,
            edge_links,
            face_links,
            fields,
            ..
        } = closer;
        topology.shells[0].closed = true;
        let geometry = inspect_roof_geometry(&topology, budget)?;
        require(
            geometry.open_edges.is_empty(),
            "Original faces did not close the exact shell",
        )?;
        let vertex_links = source_vertices
            .iter()
            .map(|(old, new)| (source.1.vertices[*old], *new))
            .collect();
        Ok(ClosedShell {
            topology,
            geometry,
            roof: self,
            source: source.clone(),
            source_fields: fields,
            edge_links,
            vertex_links,
            face_links,
        })
    }
}
