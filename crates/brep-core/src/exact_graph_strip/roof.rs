//! Exact top-surface sewing. The remaining original side/floor faces and
//! whole-solid admission are deliberately outside this OPEN roof owner.
use super::*;
use std::collections::BTreeMap;
#[derive(Clone)]
pub enum FaceGeometry {
    Transition(Arc<Patch>),
    Retained(Arc<RetainedRegion>),
    Imported(Arc<super::original_body::ImportedFace>),
}
#[derive(Clone)]
pub enum FaceUseGeometry {
    Transition(ParameterLine),
    Retained(usize),
    Imported(usize),
}
pub type RoofTopology = brep_topology::Model<Edge, FaceGeometry, FaceUseGeometry, [Q; 3]>;
pub struct SourceFieldReport {
    pub absolute_curvature_upper: f64,
    pub cells: usize,
    pub shape_cells: usize,
}
pub fn inspect_source_support_field(s: &nurbs_core::surface::Surface) -> Result<SourceFieldReport> {
    use nurbs_core::surface_shape_operator as shape;
    s.validate()?;
    let domain = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    let mut stack = vec![domain];
    let mut out = SourceFieldReport {
        absolute_curvature_upper: 0.,
        cells: 0,
        shape_cells: 0,
    };
    while let Some(r) = stack.pop() {
        require(
            out.cells < 20000 && out.shape_cells < 100000,
            "Original source field budget exhausted",
        )?;
        out.cells += 1;
        let checked = shape::rectangle_bounds(s, r, (100000 - out.shape_cells).min(1000))?;
        out.shape_cells += checked.cells;
        if checked.shape.normal.is_some() {
            if let Some(t) = checked.shape.tensor {
                if let Some(bound) = shape::absolute_spectral_bounds(t)? {
                    require(
                        bound[0] <= 10.,
                        "Original source support has a certified curvature excess above 10/mm",
                    )?;
                    if bound[1] <= 10. {
                        out.absolute_curvature_upper = out.absolute_curvature_upper.max(bound[1]);
                        continue;
                    }
                }
            }
        }
        let axis = usize::from(
            (r[1][1] - r[1][0]) / (domain[1][1] - domain[1][0])
                > (r[0][1] - r[0][0]) / (domain[0][1] - domain[0][0]),
        );
        let mid = r[axis][0] * 0.5 + r[axis][1] * 0.5;
        require(
            mid > r[axis][0] && mid < r[axis][1],
            "Original source field is unresolved at parameter resolution",
        )?;
        let mut a = r;
        let mut b = r;
        a[axis][1] = mid;
        b[axis][0] = mid;
        stack.push(b);
        stack.push(a);
    }
    Ok(out)
}
pub struct Roof {
    topology: RoofTopology,
    geometry: GeometryReport,
    transition_joins: Vec<JoinReport>,
    transition_fields: Vec<CurvatureReport>,
    source_fields: Vec<SourceFieldReport>,
}
impl Roof {
    pub fn source_fields(&self) -> &[SourceFieldReport] {
        &self.source_fields
    }
    pub fn topology(&self) -> &RoofTopology {
        &self.topology
    }
    pub fn geometry(&self) -> &GeometryReport {
        &self.geometry
    }
    pub fn transition_joins(&self) -> &[JoinReport] {
        &self.transition_joins
    }
    pub fn transition_fields(&self) -> &[CurvatureReport] {
        &self.transition_fields
    }
}
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum VertexKey {
    Crease(Q),
    Source(usize, Q, Q),
}
fn key(side: usize, uv: &[Q; 2]) -> VertexKey {
    if uv[1].is_zero() {
        VertexKey::Crease(uv[0].clone())
    } else {
        VertexKey::Source(side, uv[0].clone(), uv[1].clone())
    }
}
fn bind(map: &mut BTreeMap<VertexKey, usize>, k: VertexKey, id: usize) -> Result<()> {
    if let Some(old) = map.get(&k) {
        require(
            *old == id,
            "Named original chart endpoint has conflicting vertex identities",
        )?;
    } else {
        map.insert(k, id);
    }
    Ok(())
}
fn vertex(
    map: &mut BTreeMap<VertexKey, usize>,
    vertices: &mut Vec<brep_topology::Vertex<[Q; 3]>>,
    side: usize,
    uv: &[Q; 2],
    xyz: &[Q; 3],
) -> Result<usize> {
    let k = key(side, uv);
    if let Some(&i) = map.get(&k) {
        require(
            vertices[i].point == *xyz,
            "Named original source endpoint changed geometry",
        )?;
        return Ok(i);
    }
    let i = vertices.len();
    vertices.push(brep_topology::Vertex { point: xyz.clone() });
    map.insert(k, i);
    Ok(i)
}
fn points_equal(a: &[[Q; 3]], b: &[[Q; 3]]) -> Result<usize> {
    require(
        a == b,
        "Saved exact roof edge differs from actual face lift",
    )?;
    Ok(3 * a.len())
}
fn panel_lift(
    p: &Patch,
    line: &ParameterLine,
    reversed: bool,
    edge: &Edge,
    budget: &mut Budget,
) -> Result<usize> {
    for side in [
        Boundary::UMin,
        Boundary::UMax,
        Boundary::VMin,
        Boundary::VMax,
    ] {
        let mut data = p.boundary_definition(side, budget)?;
        let free = usize::from(data.uv[0][0] == data.uv[1][0]);
        let fixed = 1 - free;
        if line.points.iter().any(|p| p[fixed] != data.uv[0][fixed]) {
            continue;
        }
        let width = &data.uv[1][free] - &data.uv[0][free];
        let r = line
            .points
            .each_ref()
            .map(|p| (&p[free] - &data.uv[0][free]) / &width);
        if r[0] == r[1] || r.iter().any(|q| q < &Q::zero() || q > &Q::one()) {
            continue;
        }
        let forward = r[0] < r[1];
        let range = if forward {
            r
        } else {
            [r[1].clone(), r[0].clone()]
        };
        data = data.restricted(range, budget)?;
        if !forward {
            data.points.reverse();
        }
        let mut saved = edge.points.clone();
        if reversed {
            saved.reverse();
        }
        return points_equal(&data.points, &saved);
    }
    Err(Error::new(
        "BREP_EXACT_GRAPH_ROOF",
        "Saved panel UV use is not an original boundary restriction",
    ))
}
/// Fresh local topology, roots, endpoints and actual saved face-use lifts.
/// This is not a solid/global intersection or retained-face curvature proof.
pub fn inspect_roof_geometry(m: &RoofTopology, budget: &mut Budget) -> Result<GeometryReport> {
    m.validate_topology_with_vertices(|p| {
        require(
            p.iter().all(|q| {
                q.denom().is_positive() && q.numer().bits() <= 8192 && q.denom().bits() <= 8192
            }),
            "Invalid exact roof vertex",
        )
    })?;
    let mut report = GeometryReport {
        boundary_uses: 0,
        coefficient_comparisons: 0,
        open_edges: Vec::new(),
    };
    let mut counts = vec![0; m.edges.len()];
    for edge in &m.edges {
        require(
            !edge.degenerate && (2..=33).contains(&edge.curve.points.len()),
            "Unsupported exact roof edge",
        )?;
        let root = BoundaryDefinition {
            points: edge.curve.root.points.clone(),
            uv: [[Q::zero(), Q::zero()], [Q::one(), Q::zero()]],
        }
        .restricted(edge.curve.range.clone(), budget)?;
        report.coefficient_comparisons += points_equal(&root.points, &edge.curve.points)?;
        require(
            edge.curve.points[0] == m.vertices[edge.vertices[0]].point
                && *edge.curve.points.last().unwrap() == m.vertices[edge.vertices[1]].point,
            "Exact roof edge lost its vertex endpoints",
        )?;
    }
    for face in &m.faces {
        require(face.holes.is_empty(), "Unsupported exact roof holes")?;
        let cs = &m.loops[face.outer].coedges;
        if let FaceGeometry::Retained(region) = &face.surface {
            require(
                cs.len() == region.uses().len(),
                "Retained loop lost source uses",
            )?;
        }
        for (i, c) in cs.iter().enumerate() {
            require(c.trim().is_none(), "Unchecked roof coedge trim")?;
            let edge = &m.edges[c.edge].curve;
            report.coefficient_comparisons += match (&face.surface, &c.pcurve) {
                (FaceGeometry::Transition(p), FaceUseGeometry::Transition(line)) => {
                    panel_lift(p, line, c.reversed, edge, budget)?
                }
                (FaceGeometry::Retained(region), FaceUseGeometry::Retained(index)) => {
                    require(*index == i, "Retained source UV loop order changed")?;
                    let (u, reverse) = &region.uses()[i];
                    if let Some(original) = u.edge() {
                        require(
                            original == c.edge,
                            "Retained use lost its bound transition edge identity",
                        )?;
                    }

                    match u {
                        RetainedUse::Contact(binding) => require(
                            Arc::ptr_eq(&edge.root, &binding.root)
                                && edge.range == *binding.contact.root_parameters(),
                            "Retained contact root ownership changed",
                        )?,
                        RetainedUse::End(binding) => require(
                            Arc::ptr_eq(&edge.root, &binding.root)
                                && edge.range == *binding.contact.root_parameters(),
                            "Retained end root ownership changed",
                        )?,
                        RetainedUse::Natural(_) => (),
                    }
                    require(
                        c.reversed == *reverse,
                        "Retained source use direction changed",
                    )?;
                    points_equal(u.points(), &edge.points)?
                }
                (FaceGeometry::Imported(original), FaceUseGeometry::Imported(index)) => {
                    require(
                        *index == i && cs.len() == original.uses.len(),
                        "Imported original UV loop order changed",
                    )?;
                    let u = &original.uses[i];
                    require(
                        c.edge == u.edge && c.reversed == u.reversed
                            && Arc::ptr_eq(&edge.root,&u.root) && edge.range==u.range,
                        "Imported original use identity/direction changed",
                    )?;
                    let expected = nurbs_core::original_graph_transition::polynomial_surface_line(
                        &original.surface,
                        u.uv.clone(),
                        budget,
                    )?;
                    let mut actual = edge.points.clone();
                    if c.reversed {
                        actual.reverse();
                    }
                    require(
                        nurbs_core::original_graph_transition::polynomial_points_equal(
                            &expected, &actual, budget,
                        )?,
                        "Imported original face lift differs from saved exact edge",
                    )?;
                    3 * expected.len().max(actual.len())
                }
                _ => {
                    return Err(Error::new(
                        "BREP_EXACT_GRAPH_ROOF",
                        "Face and parameter geometry kinds disagree",
                    ));
                }
            };
            report.boundary_uses += 1;
            counts[c.edge] += 1;
        }
    }
    report.open_edges = counts
        .iter()
        .enumerate()
        .filter_map(|(i, &n)| (n == 1).then_some(i))
        .collect();
    Ok(report)
}
impl Strip {
    /// Consume the proposal strip and sew fresh retained original-source regions.
    /// Original chart keys and already-bound edge indices provide identities;
    /// no Cartesian proximity matching or closed-body publication is performed.
    pub fn sew_retained(self, budget: &mut Budget) -> Result<Roof> {
        let regions = self.retained_regions(budget)?.map(Arc::new);
        require(
            regions.iter().all(|r| r.is_current(&self)),
            "Source regions changed before sewing",
        )?;
        // Full-original-support bounds are stronger than trimmed-domain bounds.
        // Partial coverage or an exhausted support query never publishes a Roof.
        let source_fields = regions
            .iter()
            .map(|r| inspect_source_support_field(r.source()))
            .collect::<Result<Vec<_>>>()?;
        let mut vertices = self.topology.vertices.clone();
        let mut edges = self.topology.edges.clone();
        let mut map = BTreeMap::new();
        // Establish exact source-chart endpoint identities from BOTH neighboring
        // source uses before allocating any natural outer-source corner.
        for region in &regions {
            for (u, _) in region.uses() {
                if let Some(id) = u.edge() {
                    let edge = &edges[id];
                    require(
                        edge.curve.points == u.points(),
                        "Source binding and saved edge disagree",
                    )?;
                    bind(
                        &mut map,
                        key(region.side(), &u.uv_points()[0]),
                        edge.vertices[0],
                    )?;
                    bind(
                        &mut map,
                        key(region.side(), u.uv_points().last().unwrap()),
                        edge.vertices[1],
                    )?;
                }
            }
        }
        let mut loops = self
            .topology
            .loops
            .iter()
            .map(|l| brep_topology::Loop {
                coedges: l
                    .coedges
                    .iter()
                    .map(|c| {
                        brep_topology::Coedge::new(
                            c.edge,
                            c.reversed,
                            FaceUseGeometry::Transition(c.pcurve.clone()),
                        )
                    })
                    .collect(),
            })
            .collect::<Vec<_>>();
        let mut faces = self
            .topology
            .faces
            .iter()
            .map(|f| brep_topology::Face {
                surface: FaceGeometry::Transition(f.surface.clone()),
                outer: f.outer,
                holes: vec![],
            })
            .collect::<Vec<_>>();
        let mut face_uses = self.topology.shells[0].faces.clone();
        let mut creases = BTreeMap::new();
        for region in regions {
            let mut coedges = Vec::new();
            for (index, (u, reverse)) in region.uses().iter().enumerate() {
                let edge = if let Some(id) = u.edge() {
                    id
                } else {
                    let uv = u.uv_points();
                    let xyz = u.points();
                    let va = vertex(&mut map, &mut vertices, region.side(), &uv[0], &xyz[0])?;
                    let vb = vertex(
                        &mut map,
                        &mut vertices,
                        region.side(),
                        uv.last().unwrap(),
                        xyz.last().unwrap(),
                    )?;
                    let crease = uv[0][1].is_zero() && uv.last().unwrap()[1].is_zero();
                    let key = (uv[0][0].clone(), uv.last().unwrap()[0].clone());
                    if crease {
                        require(key.0 < key.1, "Original source crease clock must increase")?;
                    }
                    if let Some(&id) = if crease { creases.get(&key) } else { None } {
                        let old: &brep_topology::Edge<Edge> = &edges[id];
                        require(
                            old.vertices == [va, vb] && old.curve.points == xyz,
                            "Named original crease does not share exact geometry and vertices",
                        )?;
                        id
                    } else {
                        let id = edges.len();
                        edges.push(brep_topology::Edge {
                            degenerate: false,
                            vertices: [va, vb],
                            curve: Edge::original(xyz.to_vec(), id),
                        });
                        if crease {
                            creases.insert(key, id);
                        }
                        id
                    }
                };
                coedges.push(brep_topology::Coedge::new(
                    edge,
                    *reverse,
                    FaceUseGeometry::Retained(index),
                ));
            }
            let face = faces.len();
            let outer = loops.len();
            loops.push(brep_topology::Loop { coedges });
            face_uses.push(brep_topology::FaceUse {
                face,
                reversed: region.side() == 0,
            });
            faces.push(brep_topology::Face {
                surface: FaceGeometry::Retained(region),
                outer,
                holes: vec![],
            });
        }
        require(
            creases.len() == 2,
            "Expected the two retained original crease intervals",
        )?;
        let topology = RoofTopology {
            vertices,
            edges,
            loops,
            faces,
            shells: vec![brep_topology::Shell {
                faces: face_uses,
                closed: false,
            }],
            bodies: vec![],
            tolerance_mm: self.topology.tolerance_mm,
        };
        let geometry = inspect_roof_geometry(&topology, budget)?;
        require(
            geometry.open_edges.len() == 6,
            "Unexpected original outer roof boundary",
        )?;
        Ok(Roof {
            topology,
            geometry,
            transition_joins: self.joins,
            transition_fields: self.fields,
            source_fields,
        })
    }
}
