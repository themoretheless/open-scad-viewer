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
/// Largest numerator/denominator bit length admitted for an exact roof vertex.
const MAX_VERTEX_BITS: u64 = 8192;
/// Admitted control-point counts of a saved exact roof edge.
const ROOF_EDGE_POINTS: std::ops::RangeInclusive<usize> = 2..=33;
fn roof_error<T>(message: &str) -> Result<T> {
    Err(Error::new("BREP_EXACT_GRAPH_ROOF", message))
}
/// Root restriction and endpoint identity of one saved edge.
fn check_roof_edge(
    m: &RoofTopology,
    edge: &brep_topology::Edge<Edge>,
    budget: &mut Budget,
) -> Result<usize> {
    require(
        !edge.degenerate && ROOF_EDGE_POINTS.contains(&edge.curve.points.len()),
        "Unsupported exact roof edge",
    )?;
    let root = BoundaryDefinition {
        points: edge.curve.root.points.clone(),
        uv: [[Q::zero(), Q::zero()], [Q::one(), Q::zero()]],
    }
    .restricted(edge.curve.range.clone(), budget)?;
    let comparisons = points_equal(&root.points, &edge.curve.points)?;
    let ends_match = match (edge.curve.points.first(), edge.curve.points.last()) {
        (Some(first), Some(last)) => {
            *first == m.vertices[edge.vertices[0]].point
                && *last == m.vertices[edge.vertices[1]].point
        }
        _ => false,
    };
    require(ends_match, "Exact roof edge lost its vertex endpoints")?;
    Ok(comparisons)
}
/// Retained source use `i` of `region` against coedge `c` over `edge`.
fn check_retained_use(
    region: &RetainedRegion,
    index: usize,
    i: usize,
    c: &brep_topology::Coedge<FaceUseGeometry>,
    edge: &Edge,
) -> Result<usize> {
    require(index == i, "Retained source UV loop order changed")?;
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
    points_equal(u.points(), &edge.points)
}
/// Imported original use `i` of a loop with `loop_len` coedges.
fn check_imported_use(
    original: &ImportedFace,
    index: usize,
    i: usize,
    loop_len: usize,
    c: &brep_topology::Coedge<FaceUseGeometry>,
    edge: &Edge,
    budget: &mut Budget,
) -> Result<usize> {
    require(
        index == i && loop_len == original.uses.len(),
        "Imported original UV loop order changed",
    )?;
    let u = &original.uses[i];
    require(
        c.edge == u.edge
            && c.reversed == u.reversed
            && Arc::ptr_eq(&edge.root, &u.root)
            && edge.range == u.range,
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
    Ok(3 * expected.len().max(actual.len()))
}
/// Fresh local topology, roots, endpoints and actual saved face-use lifts.
/// This is not a solid/global intersection or retained-face curvature proof.
pub fn inspect_roof_geometry(m: &RoofTopology, budget: &mut Budget) -> Result<GeometryReport> {
    m.validate_topology_with_vertices(|p| {
        require(
            p.iter().all(|q| {
                q.denom().is_positive()
                    && q.numer().bits() <= MAX_VERTEX_BITS
                    && q.denom().bits() <= MAX_VERTEX_BITS
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
        report.coefficient_comparisons += check_roof_edge(m, edge, budget)?;
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
                    check_retained_use(region, *index, i, c, edge)?
                }
                (FaceGeometry::Imported(original), FaceUseGeometry::Imported(index)) => {
                    check_imported_use(original, *index, i, cs.len(), c, edge, budget)?
                }
                _ => return roof_error("Face and parameter geometry kinds disagree"),
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
/// Mutable roof topology while retained source regions are sewn onto the strip.
struct RoofSewing {
    vertices: Vec<brep_topology::Vertex<[Q; 3]>>,
    edges: Vec<brep_topology::Edge<Edge>>,
    loops: Vec<brep_topology::Loop<FaceUseGeometry>>,
    faces: Vec<brep_topology::Face<FaceGeometry>>,
    face_uses: Vec<brep_topology::FaceUse>,
    map: BTreeMap<VertexKey, usize>,
    creases: BTreeMap<(Q, Q), usize>,
}
impl RoofSewing {
    fn new(topology: &Topology) -> Self {
        let loops = topology
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
            .collect();
        let faces = topology
            .faces
            .iter()
            .map(|f| brep_topology::Face {
                surface: FaceGeometry::Transition(f.surface.clone()),
                outer: f.outer,
                holes: vec![],
            })
            .collect();
        Self {
            vertices: topology.vertices.clone(),
            edges: topology.edges.clone(),
            loops,
            faces,
            face_uses: topology.shells[0].faces.clone(),
            map: BTreeMap::new(),
            creases: BTreeMap::new(),
        }
    }
    /// Establish exact source-chart endpoint identities from BOTH neighboring
    /// source uses before allocating any natural outer-source corner.
    fn bind_source_endpoints(&mut self, regions: &[Arc<RetainedRegion>]) -> Result<()> {
        for region in regions {
            for (u, _) in region.uses() {
                if let Some(id) = u.edge() {
                    let edge = &self.edges[id];
                    require(
                        edge.curve.points == u.points(),
                        "Source binding and saved edge disagree",
                    )?;
                    let uv = u.uv_points();
                    let (Some(first), Some(last)) = (uv.first(), uv.last()) else {
                        return roof_error("Retained source use has no UV points");
                    };
                    let ends = edge.vertices;
                    bind(&mut self.map, key(region.side(), first), ends[0])?;
                    bind(&mut self.map, key(region.side(), last), ends[1])?;
                }
            }
        }
        Ok(())
    }
    /// Edge of a natural (unbound) source use; named crease intervals are shared.
    fn natural_edge(&mut self, side: usize, u: &RetainedUse) -> Result<usize> {
        let uv = u.uv_points();
        let xyz = u.points();
        let (Some(uv_first), Some(uv_last), Some(xyz_first), Some(xyz_last)) =
            (uv.first(), uv.last(), xyz.first(), xyz.last())
        else {
            return roof_error("Retained source use has no UV points");
        };
        let va = vertex(&mut self.map, &mut self.vertices, side, uv_first, xyz_first)?;
        let vb = vertex(&mut self.map, &mut self.vertices, side, uv_last, xyz_last)?;
        let crease = uv_first[1].is_zero() && uv_last[1].is_zero();
        let key = (uv_first[0].clone(), uv_last[0].clone());
        if crease {
            require(key.0 < key.1, "Original source crease clock must increase")?;
            if let Some(&id) = self.creases.get(&key) {
                let old = &self.edges[id];
                require(
                    old.vertices == [va, vb] && old.curve.points == xyz,
                    "Named original crease does not share exact geometry and vertices",
                )?;
                return Ok(id);
            }
        }
        let id = self.edges.len();
        self.edges.push(brep_topology::Edge {
            degenerate: false,
            vertices: [va, vb],
            curve: Edge::original(xyz.to_vec(), id),
        });
        if crease {
            self.creases.insert(key, id);
        }
        Ok(id)
    }
    fn sew_region(&mut self, region: Arc<RetainedRegion>) -> Result<()> {
        let mut coedges = Vec::new();
        for (index, (u, reverse)) in region.uses().iter().enumerate() {
            let edge = match u.edge() {
                Some(id) => id,
                None => self.natural_edge(region.side(), u)?,
            };
            coedges.push(brep_topology::Coedge::new(
                edge,
                *reverse,
                FaceUseGeometry::Retained(index),
            ));
        }
        let face = self.faces.len();
        let outer = self.loops.len();
        self.loops.push(brep_topology::Loop { coedges });
        self.face_uses.push(brep_topology::FaceUse {
            face,
            reversed: region.side() == 0,
        });
        self.faces.push(brep_topology::Face {
            surface: FaceGeometry::Retained(region),
            outer,
            holes: vec![],
        });
        Ok(())
    }
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
        let mut sewing = RoofSewing::new(&self.topology);
        sewing.bind_source_endpoints(&regions)?;
        for region in regions {
            sewing.sew_region(region)?;
        }
        require(
            sewing.creases.len() == 2,
            "Expected the two retained original crease intervals",
        )?;
        let topology = RoofTopology {
            vertices: sewing.vertices,
            edges: sewing.edges,
            loops: sewing.loops,
            faces: sewing.faces,
            shells: vec![brep_topology::Shell {
                faces: sewing.face_uses,
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

impl Roof {
    /// Transactional restriction of a natural original edge. Both children keep
    /// one exact root, including non-dyadic endpoints. Remaining source faces
    /// are split against these intervals by close_original_shell.
    pub fn split_natural_edge(&mut self, edge: usize, parameter: Q, budget: &mut Budget) -> Result<SplitReport> {
        require(edge < self.topology.edges.len() && parameter.denom().is_positive()
            && parameter > Q::zero() && parameter < Q::one()
            && parameter.numer().bits()<=8192 && parameter.denom().bits()<=8192,
            "Choose an interior exact natural-edge parameter")?;
        let old = &self.topology.edges[edge];
        let root_id=old.curve.root.id;
        let root_parameter = &old.curve.range[0] + (&old.curve.range[1]-&old.curve.range[0])*&parameter;
        let def = BoundaryDefinition { points: old.curve.points.clone(), uv: [[Q::zero(),Q::zero()],[Q::one(),Q::zero()]] };
        let left=def.restricted([Q::zero(),parameter.clone()],budget)?;
        let right=def.restricted([parameter.clone(),Q::one()],budget)?;
        require(left.points.last()==right.points.first(),"Natural child endpoints disagree")?;
        let mut top=self.topology.clone();
        let vertex=top.vertices.len();let right_edge=top.edges.len();
        top.vertices.push(brep_topology::Vertex { point:left.points.last().unwrap().clone() });
        top.edges[edge]=brep_topology::Edge { degenerate:false, vertices:[old.vertices[0],vertex],curve:Edge {points:left.points,root:old.curve.root.clone(),range:[old.curve.range[0].clone(),root_parameter.clone()]} };
        top.edges.push(brep_topology::Edge { degenerate:false,vertices:[vertex,old.vertices[1]],curve:Edge {points:right.points,root:old.curve.root.clone(),range:[root_parameter.clone(),old.curve.range[1].clone()]} });
        let mut face_uses=0;
        for face in &mut top.faces {
            let wire=&mut top.loops[face.outer];
            let slots=wire.coedges.iter().enumerate().filter(|(_,c)|c.edge==edge).map(|(i,_)|i).collect::<Vec<_>>();
            if slots.is_empty(){continue;}
            require(slots.len()==1,"Repeated natural edge use is unsupported")?;
            let i=slots[0];let c=wire.coedges[i].clone();
            let FaceGeometry::Retained(region)=&face.surface else {return Err(Error::new("BREP_EXACT_GRAPH_ROOF","Natural split must belong to retained original faces"));};
            require(matches!(c.pcurve,FaceUseGeometry::Retained(index) if index==i),"Natural split lost its source use")?;
            face.surface=FaceGeometry::Retained(Arc::new(region.split_natural_use(i,&parameter,budget)?));
            let a=brep_topology::Coedge::new(edge,c.reversed,FaceUseGeometry::Retained(0));
            let z=brep_topology::Coedge::new(right_edge,c.reversed,FaceUseGeometry::Retained(0));
            wire.coedges.splice(i..=i,if c.reversed {vec![z,a]} else {vec![a,z]});
            for (j,c) in wire.coedges.iter_mut().enumerate(){c.pcurve=FaceUseGeometry::Retained(j);}
            face_uses+=1;
        }
        require((1..=2).contains(&face_uses),"Natural split needs one or two source uses")?;
        let geometry=inspect_roof_geometry(&top,budget)?;
        self.topology=top;self.geometry=geometry;
        Ok(SplitReport {left_edge:edge,right_edge,vertex,root_id,root_parameter,face_uses})
    }
}
