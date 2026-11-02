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
fn uv_line(c: &crate::Coedge) -> Result<[[Q; 2]; 2]> {
    let p = &c.pcurve;
    p.validate()?;
    require(
        c.trim().is_none()
            && !p.periodic
            && p.degree == 1
            && p.control_points.len() == 2
            && p.weights[0] == p.weights[1]
            && p.control_points.iter().all(|p| p.len() == 2),
        "Original face needs exact polynomial UV lines",
    )?;
    Ok(std::array::from_fn(|i| {
        std::array::from_fn(|k| Q::from_float(p.control_points[i][k]).unwrap())
    }))
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
        let shell = &source.shells[shells[0]];
        require(
            shell.closed
                && source
                    .bodies
                    .iter()
                    .any(|b| b.outer_shell == shells[0] && b.inner_shells.is_empty()),
            "Expected an original closed outer shell without cavities",
        )?;
        let mut topology = self.topology().clone();
        let mut source_edges = BTreeMap::new();
        let mut source_vertices = BTreeMap::new();
        let mut edge_links = bindings.edges().to_vec();
        let mut face_links = Vec::new();
        for (face, f) in topology.faces.iter().enumerate() {
            if let FaceGeometry::Retained(region) = &f.surface {
                face_links.push((source.1.faces[source_faces[region.side()]], face));
            }
        }
        let mut fields = Vec::new();
        for link in bindings.edges() {
            if self.geometry().open_edges.contains(&link.roof_edge) {
                require(
                    link.source_range == [Q::zero(), Q::one()],
                    "Outer source boundary has an unsupported partial original edge",
                )?;
                require(
                    source_edges
                        .insert(
                            link.source_edge,
                            (link.roof_edge, link.reversed_from_source),
                        )
                        .is_none(),
                    "Ambiguous original outer edge identity",
                )?;
            }
        }
        for p in bindings.endpoints() {
            if let Some(old) = p.source_vertex {
                if let Some(&existing) = source_vertices.get(&old) {
                    require(
                        existing == p.roof_vertex,
                        "Original vertex maps to conflicting roof identities",
                    )?;
                } else {
                    source_vertices.insert(old, p.roof_vertex);
                }
            }
        }
        for face_use in &shell.faces {
            if source_faces.contains(&face_use.face) {
                continue;
            }
            let original = &source.faces[face_use.face];
            require(
                original.holes.is_empty(),
                "Original source holes require separate import qualification",
            )?;
            let cs = &source.loops[original.outer].coedges;
            let lines = cs.iter().map(uv_line).collect::<Result<Vec<_>>>()?;
            convex(&lines)?;
            let field = inspect_source_support_field(&original.surface)?;
            let mut uses = Vec::new();
            let mut coedges = Vec::new();
            for (c, uv) in cs.iter().zip(lines) {
                let old = &source.edges[c.edge];
                let (edge, relation) = if let Some(&mapped) = source_edges.get(&c.edge) {
                    mapped
                } else {
                    let points = curve_points(&old.curve)?;
                    let mut vertices = [0; 2];
                    for k in 0..2 {
                        let id = old.vertices[k];
                        let p = source.vertices[id].point.map(|x| Q::from_float(x).unwrap());
                        vertices[k] = if let Some(&i) = source_vertices.get(&id) {
                            require(
                                topology.vertices[i].point == p,
                                "Original named vertex changed world geometry",
                            )?;
                            i
                        } else {
                            let i = topology.vertices.len();
                            topology.vertices.push(brep_topology::Vertex { point: p });
                            source_vertices.insert(id, i);
                            i
                        };
                    }
                    let id = topology.edges.len();
                    topology.edges.push(brep_topology::Edge {
                        degenerate: false,
                        vertices,
                        curve: Edge::original(points, id),
                    });
                    source_edges.insert(c.edge, (id, false));
                    edge_links.push(OriginalEdgeLink {
                        roof_edge: id,
                        source_edge: c.edge,
                        source_id: source.1.edges[c.edge],
                        source_range: [Q::zero(), Q::one()],
                        reversed_from_source: false,
                    });
                    (id, false)
                };
                let reversed = c.reversed ^ relation;
                let index = uses.len();
                uses.push(ImportedUse {
                    uv,
                    edge,
                    reversed,
                    root: topology.edges[edge].curve.root.clone(),
                    range: topology.edges[edge].curve.range.clone(),
                });
                coedges.push(brep_topology::Coedge::new(
                    edge,
                    reversed,
                    FaceUseGeometry::Imported(index),
                ));
            }
            let face = topology.faces.len();
            let outer = topology.loops.len();
            topology.loops.push(brep_topology::Loop { coedges });
            topology.faces.push(brep_topology::Face {
                surface: FaceGeometry::Imported(Arc::new(ImportedFace {
                    surface: original.surface.clone(),
                    uses,
                    id: source.1.faces[face_use.face],
                })),
                outer,
                holes: vec![],
            });
            topology.shells[0].faces.push(brep_topology::FaceUse {
                face,
                reversed: face_use.reversed,
            });
            fields.push(field);
            face_links.push((source.1.faces[face_use.face], face));
        }
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
