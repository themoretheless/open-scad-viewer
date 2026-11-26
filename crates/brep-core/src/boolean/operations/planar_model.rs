use super::*;
#[derive(Clone, Copy)]
pub(crate) struct Plane {
    pub(crate) normal: [f64; 3],
    pub(crate) offset: f64,
}

pub(crate) fn face_plane(model: &Model, face_id: usize) -> Result<Plane> {
    let face = &model.faces[face_id];
    if std::iter::once(&face.outer)
        .chain(&face.holes)
        .any(|&wire| {
            model.loops[wire]
                .coedges
                .iter()
                .any(|c| model.edges[c.edge].curve.degree != 1)
        })
    {
        return Err(unsupported(
            "Planar operations require straight boundary edges; curved booleans are unsupported",
        ));
    }
    let ids = loop_vertices(model, face.outer)?;
    if ids.len() < 3 {
        return Err(unsupported("Face has fewer than three corners"));
    }
    let points: Vec<_> = ids.iter().map(|&i| model.vertices[i].point).collect();
    let origin = points[0];
    // Signed area vector respects the complete loop, including a reflex first corner.
    let area_vector = (0..points.len()).fold([0.; 3], |sum, i| {
        add(
            sum,
            cross(
                sub(points[i], origin),
                sub(points[(i + 1) % points.len()], origin),
            ),
        )
    });
    let normal = unit(area_vector)?;
    let offset = dot(normal, origin);
    if points
        .iter()
        .any(|&p| (dot(normal, p) - offset).abs() > model.tolerance_mm * 8.)
        || face.surface.control_points.iter().flatten().any(|point| {
            (dot(normal, [point[0], point[1], point[2]]) - offset).abs() > model.tolerance_mm * 8.
        })
    {
        return Err(unsupported("Operation requires planar faces"));
    }
    Ok(Plane { normal, offset })
}

pub(crate) fn convex_planes(model: &Model) -> Result<Vec<Plane>> {
    model.validate()?;
    if model.bodies.len() != 1
        || model.shells.len() != 1
        || !model.bodies[0].inner_shells.is_empty()
    {
        return Err(unsupported(
            "Edge operations require one body without cavities",
        ));
    }
    let shell = &model.shells[model.bodies[0].outer_shell];
    let mut planes = Vec::with_capacity(shell.faces.len());
    for face_use in &shell.faces {
        if !model.faces[face_use.face].holes.is_empty() {
            return Err(unsupported("Convex operations do not accept face holes"));
        }
        let mut plane = face_plane(model, face_use.face)?;
        if face_use.reversed {
            plane.normal = mul(plane.normal, -1.);
            plane.offset = -plane.offset;
        }
        planes.push(plane);
    }
    if model.vertices.iter().any(|v| {
        planes
            .iter()
            .any(|p| dot(p.normal, v.point) > p.offset + model.tolerance_mm * 8.)
    }) {
        return Err(unsupported(
            "Edge operations require a convex solid with outward planar faces",
        ));
    }
    Ok(planes)
}

pub(crate) fn plane_basis(normal: [f64; 3]) -> Result<([f64; 3], [f64; 3])> {
    let reference = if normal[0].abs() < 0.8 {
        [1., 0., 0.]
    } else {
        [0., 1., 0.]
    };
    let u = unit(cross(reference, normal))?;
    Ok((u, cross(normal, u)))
}

pub(crate) fn point_on_segment(point: [f64; 3], a: [f64; 3], b: [f64; 3], tolerance: f64) -> Option<f64> {
    let ab = sub(b, a);
    let length_squared = dot(ab, ab);
    if length_squared <= tolerance * tolerance {
        return None;
    }
    let t = dot(sub(point, a), ab) / length_squared;
    if t > tolerance && t < 1. - tolerance {
        norm(sub(point, add(a, mul(ab, t)))) <= tolerance
    } else {
        false
    }
    .then_some(t)
}

/// Split polygon edges at every collinear result vertex. Face clipping creates
/// T-junctions unless neighboring polygons agree on these authored edge spans.
fn normalize_ring_edges(polygon: &mut Vec<[f64; 3]>, points: &[[f64; 3]], tolerance: f64) {
    let mut normalized = vec![];
    for i in 0..polygon.len() {
        let a = polygon[i];
        let b = polygon[(i + 1) % polygon.len()];
        normalized.push(a);
        let mut splits: Vec<_> = points
            .iter()
            .copied()
            .filter_map(|point| point_on_segment(point, a, b, tolerance * 4.).map(|t| (t, point)))
            .collect();
        splits.sort_by(|left, right| left.0.total_cmp(&right.0));
        for (_, point) in splits {
            if normalized.last().is_none_or(|&last| !close(last, point, tolerance * 4.)) {
                normalized.push(point);
            }
        }
    }
    *polygon = normalized;
}

#[derive(Clone)]
pub(crate) struct PlanarBoundary {
    pub(crate) outer: Vec<[f64; 3]>,
    pub(crate) holes: Vec<Vec<[f64; 3]>>,
}

pub(crate) fn model_from_polygons(polygons: Vec<Vec<[f64; 3]>>, tolerance: f64) -> Result<Model> {
    model_from_trimmed_polygons(
        polygons
            .into_iter()
            .map(|outer| PlanarBoundary {
                outer,
                holes: vec![],
            })
            .collect(),
        tolerance,
    )
}

/// Largest face count a polygon-built result may carry.
const MAX_RESULT_FACES: usize = 256;
/// Vertex welding distance, as a multiple of the model tolerance.
const VERTEX_WELD_FACTOR: f64 = 4.;

pub(crate) fn model_from_trimmed_polygons(mut polygons: Vec<PlanarBoundary>, tolerance: f64) -> Result<Model> {
    normalize_all_rings(&mut polygons, tolerance);
    if polygons.len() > MAX_RESULT_FACES {
        return Err(Error::new(
            "BREP_RESOURCE_LIMIT",
            "Result exceeds 256 faces",
        ));
    }
    let mut builder = PolygonModelBuilder {
        model: empty_model(tolerance),
        edge_map: BTreeMap::new(),
        tolerance,
    };
    for boundary in polygons {
        builder.add_face(boundary)?;
    }
    let mut model = builder.model;
    assemble_bodies(&mut model, tolerance)?;
    model.rebuild_topology_ids();
    model.validate().map_err(|e| {
        if e.code == "BREP_RESOURCE_LIMIT" {
            e
        } else {
            unsupported(format!(
                "Result is disconnected, has a cavity, or is non-manifold: {}",
                e.message
            ))
        }
    })?;
    Ok(model)
}

fn normalize_all_rings(polygons: &mut [PlanarBoundary], tolerance: f64) {
    let points: Vec<_> = polygons
        .iter()
        .flat_map(|face| std::iter::once(&face.outer).chain(&face.holes))
        .flatten()
        .copied()
        .collect();
    for face in polygons {
        normalize_ring_edges(&mut face.outer, &points, tolerance);
        for hole in &mut face.holes {
            normalize_ring_edges(hole, &points, tolerance);
        }
    }
}

fn empty_model(tolerance: f64) -> Model {
    Model(
        brep_topology::Model {
            vertices: vec![],
            edges: vec![],
            loops: vec![],
            faces: vec![],
            shells: vec![],
            bodies: vec![],
            tolerance_mm: tolerance,
        },
        TopologyIds::default(),
    )
}

/// Exact coordinate-plane chart: fixed axis, its height, the two chart axes
/// and their coordinate ranges.
type CoordinateChart = (usize, f64, [usize; 2], [[f64; 2]; 2]);

/// Projected planar frame of one face polygon.
struct FaceFrame {
    origin: [f64; 3],
    u: [f64; 3],
    v: [f64; 3],
    min: [f64; 2],
    max: [f64; 2],
    chart: Option<CoordinateChart>,
    parallelogram: Option<[[f64; 3]; 4]>,
}

impl FaceFrame {
    fn uv(&self, point: [f64; 3], welded: [f64; 3]) -> [f64; 2] {
        if let Some((_, _, axes, _)) = self.chart {
            return axes.map(|axis| welded[axis]);
        }
        if let Some(corners) = &self.parallelogram {
            if let Some(i) = corners.iter().position(|corner| *corner == point) {
                return [[0.,0.],[1.,0.],[1.,1.],[0.,1.]][i];
            }
        }
        let d = sub(point, self.origin);
        [
            (dot(d, self.u) - self.min[0]) / (self.max[0] - self.min[0]),
            (dot(d, self.v) - self.min[1]) / (self.max[1] - self.min[1]),
        ]
    }

    fn surface(&self) -> Surface {
        let surface_origin = add(
            self.origin,
            add(mul(self.u, self.min[0]), mul(self.v, self.min[1])),
        );
        let du = mul(self.u, self.max[0] - self.min[0]);
        let dv = mul(self.v, self.max[1] - self.min[1]);
        let mut surface = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![surface_origin.to_vec(), add(surface_origin, dv).to_vec()],
                vec![
                    add(surface_origin, du).to_vec(),
                    add(add(surface_origin, du), dv).to_vec(),
                ],
            ],
            weights: vec![vec![1., 1.], vec![1., 1.]],
            periodic_u: false,
            periodic_v: false,
        };
        if let Some((fixed, height, axes, ranges)) = self.chart {
            for (a, row) in surface.control_points.iter_mut().enumerate() {
                for (b, control) in row.iter_mut().enumerate() {
                    let mut p = vec![0.; 3];
                    p[fixed] = height;
                    p[axes[0]] = ranges[0][a];
                    p[axes[1]] = ranges[1][b];
                    *control = p;
                }
            }
            surface.knots_u = vec![ranges[0][0], ranges[0][0], ranges[0][1], ranges[0][1]];
            surface.knots_v = vec![ranges[1][0], ranges[1][0], ranges[1][1], ranges[1][1]];
        }
        if let Some([a,b,c,d]) = self.parallelogram {
            surface.control_points = vec![vec![a.to_vec(),d.to_vec()],vec![b.to_vec(),c.to_vec()]];
        }
        surface
    }
}

fn coordinate_range(points: &[[f64; 3]], axis: usize) -> [f64; 2] {
    [
        points.iter().map(|p| p[axis]).fold(f64::INFINITY, f64::min),
        points.iter().map(|p| p[axis]).fold(f64::NEG_INFINITY, f64::max),
    ]
}

fn polygon_normal(polygon: &[[f64; 3]]) -> Result<[f64; 3]> {
    let first = polygon[0];
    unit((0..polygon.len()).fold([0.; 3], |sum, i| {
        add(
            sum,
            cross(
                sub(polygon[i], first),
                sub(polygon[(i + 1) % polygon.len()], first),
            ),
        )
    }))
}

/// Incremental model assembly: welds vertices, shares edges and authors one
/// planar face per boundary.
struct PolygonModelBuilder {
    model: Model,
    edge_map: BTreeMap<(usize, usize), usize>,
    tolerance: f64,
}

impl PolygonModelBuilder {
    fn add_face(&mut self, boundary: PlanarBoundary) -> Result<()> {
        let polygon = boundary.outer;
        if polygon.len() < 3 || boundary.holes.iter().any(|hole| hole.len() < 3) {
            return Err(failed("Operation produced a degenerate face"));
        }
        let all_points: Vec<_> = polygon
            .iter()
            .chain(boundary.holes.iter().flatten())
            .copied()
            .collect();
        let frame = self.face_frame(&polygon, &all_points)?;
        let outer = self.add_ring(&polygon, &frame);
        let holes = boundary
            .holes
            .iter()
            .map(|ring| self.add_ring(ring, &frame))
            .collect();
        self.model.faces.push(Face {
            surface: frame.surface(),
            outer,
            holes,
        });
        Ok(())
    }

    fn face_frame(&self, polygon: &[[f64; 3]], all_points: &[[f64; 3]]) -> Result<FaceFrame> {
        let normal = polygon_normal(polygon)?;
        let (u, v) = plane_basis(normal)?;
        let origin = polygon[0];
        let chart = (0..3).find_map(|fixed| self.coordinate_chart(all_points, normal, fixed));
        let coordinates: Vec<_> = all_points
            .iter()
            .map(|&p| {
                let d = sub(p, origin);
                [dot(d, u), dot(d, v)]
            })
            .collect();
        let min = [0, 1].map(|k| coordinates.iter().map(|c| c[k]).fold(f64::INFINITY, f64::min));
        let max = [0, 1].map(|k| coordinates.iter().map(|c| c[k]).fold(f64::NEG_INFINITY, f64::max));
        if max[0] - min[0] <= self.tolerance || max[1] - min[1] <= self.tolerance {
            return Err(failed("Operation produced a collapsed face"));
        }
        Ok(FaceFrame {
            origin,
            u,
            v,
            min,
            max,
            parallelogram: if chart.is_none() && all_points.len() == 4 {
                self.exact_parallelogram(polygon)
            } else {
                None
            },
            chart,
        })
    }

    /// Reuse the four model vertices as the bilinear control net only when
    /// the two diagonals have exactly the same midpoint in binary arithmetic.
    fn exact_parallelogram(&self, polygon: &[[f64; 3]]) -> Option<[[f64; 3]; 4]> {
        use num_rational::BigRational as Q;
        let corners: [[f64; 3]; 4] = polygon.try_into().ok()?;
        for (i, &p) in corners.iter().enumerate() {
            if self.welded_vertex(p).is_some_and(|id| self.model.vertices[id].point != p)
                || corners[..i].iter().any(|&q| close(p, q, self.tolerance * VERTEX_WELD_FACTOR))
            {
                return None;
            }
        }
        for axis in 0..3 {
            let [a,b,c,d] = corners.map(|p| Q::from_float(p[axis]));
            if a? + c? != b? + d? {
                return None;
            }
        }
        Some(corners)
    }

    /// Author exact coordinate-plane charts from the original polygon
    /// coordinates. Normalizing decimal UVs and lifting them independently
    /// can introduce a nonzero exact edge/face discrepancy.
    fn coordinate_chart(&self, all_points: &[[f64; 3]], normal: [f64; 3], fixed: usize) -> Option<CoordinateChart> {
        let height = all_points[0][fixed];
        if !all_points.iter().all(|p| p[fixed] == height) {
            return None;
        }
        let mut axes = [(fixed + 1) % 3, (fixed + 2) % 3];
        if normal[fixed] < 0. {
            axes.swap(0, 1);
        }
        let ranges = axes.map(|axis| coordinate_range(all_points, axis));
        if ranges.iter().any(|r| r[1] - r[0] <= self.tolerance) {
            return None;
        }
        // The edge graph chooses the canonical vertex. Do not project a
        // tolerance-merged vertex into this plane or outside its rectangle.
        let leaves_chart = |v: &Vertex| {
            v.point[fixed] != height
                || (0..2).any(|k| v.point[axes[k]] < ranges[k][0] || v.point[axes[k]] > ranges[k][1])
        };
        if all_points
            .iter()
            .any(|p| self.welded_vertex(*p).is_some_and(|id| leaves_chart(&self.model.vertices[id])))
        {
            return None;
        }
        Some((fixed, height, axes, ranges))
    }

    fn welded_vertex(&self, point: [f64; 3]) -> Option<usize> {
        let distance = self.tolerance * VERTEX_WELD_FACTOR;
        self.model.vertices.iter().position(|v| close(v.point, point, distance))
    }

    fn vertex_id(&mut self, point: [f64; 3]) -> usize {
        self.welded_vertex(point).unwrap_or_else(|| {
            let id = self.model.vertices.len();
            self.model.vertices.push(Vertex { point });
            id
        })
    }

    fn edge_id(&mut self, a: usize, b: usize) -> usize {
        let key = (a.min(b), a.max(b));
        let model = &mut self.model;
        *self.edge_map.entry(key).or_insert_with(|| {
            let id = model.edges.len();
            let start = model.vertices[key.0].point.to_vec();
            let end = model.vertices[key.1].point.to_vec();
            model.edges.push(Edge {
                degenerate: false,
                vertices: [key.0, key.1],
                curve: line(start, end),
            });
            id
        })
    }

    fn add_ring(&mut self, ring: &[[f64; 3]], frame: &FaceFrame) -> usize {
        let mut ids = Vec::with_capacity(ring.len());
        let mut uv = Vec::with_capacity(ring.len());
        for &point in ring {
            let id = self.vertex_id(point);
            ids.push(id);
            uv.push(frame.uv(point, self.model.vertices[id].point));
        }
        let mut coedges = Vec::with_capacity(ids.len());
        for i in 0..ids.len() {
            let next = (i + 1) % ids.len();
            let (a, b) = (ids[i], ids[next]);
            let edge = self.edge_id(a, b);
            coedges.push(Coedge::new(
                edge,
                a > b,
                line(uv[i].to_vec(), uv[next].to_vec()),
            ));
        }
        let loop_id = self.model.loops.len();
        self.model.loops.push(Loop { coedges });
        loop_id
    }
}

fn face_loops(model: &Model, face: usize) -> impl Iterator<Item = &usize> {
    std::iter::once(&model.faces[face].outer).chain(&model.faces[face].holes)
}

fn face_adjacency(model: &Model) -> Vec<Vec<usize>> {
    let mut edge_faces = vec![Vec::<usize>::new(); model.edges.len()];
    for face_id in 0..model.faces.len() {
        for &loop_id in face_loops(model, face_id) {
            for coedge in &model.loops[loop_id].coedges {
                edge_faces[coedge.edge].push(face_id);
            }
        }
    }
    let mut adjacency = vec![Vec::<usize>::new(); model.faces.len()];
    for owners in edge_faces {
        for &a in &owners {
            adjacency[a].extend(owners.iter().copied().filter(|&b| b != a));
        }
    }
    adjacency
}

fn connected_component(adjacency: &[Vec<usize>], unseen: &mut BTreeSet<usize>, seed: usize) -> Vec<usize> {
    let mut stack = vec![seed];
    let mut component = vec![];
    while let Some(face) = stack.pop() {
        component.push(face);
        for &neighbor in &adjacency[face] {
            if unseen.remove(&neighbor) {
                stack.push(neighbor);
            }
        }
    }
    component
}

fn component_signed_volume(model: &Model, component: &[usize]) -> Result<f64> {
    let loops = component
        .iter()
        .flat_map(|&face| face_loops(model, face))
        .map(|&loop_id| loop_vertices(model, loop_id))
        .collect::<Result<Vec<_>>>()?;
    Ok(loops
        .into_iter()
        .map(|vertices| {
            (1..vertices.len().saturating_sub(1))
                .map(|i| {
                    let a = model.vertices[vertices[0]].point;
                    let b = model.vertices[vertices[i]].point;
                    let c = model.vertices[vertices[i + 1]].point;
                    dot(a, cross(b, c)) / 6.
                })
                .sum::<f64>()
        })
        .sum::<f64>())
}

/// A closed shell with its faces and signed enclosed volume.
struct ShellComponent {
    shell: usize,
    faces: Vec<usize>,
    volume: f64,
}

/// Each edge-connected boundary component is an independent solid body.
/// Keeping all disconnected faces in one shell would violate the topology
/// contract and made useful results such as a split difference fail closed.
fn assemble_bodies(model: &mut Model, tolerance: f64) -> Result<()> {
    let adjacency = face_adjacency(model);
    let mut unseen: BTreeSet<_> = (0..model.faces.len()).collect();
    let mut components = vec![];
    while let Some(seed) = unseen.pop_first() {
        let faces = connected_component(&adjacency, &mut unseen, seed);
        let volume = component_signed_volume(model, &faces)?;
        if volume.abs() <= tolerance.powi(3) {
            return Err(unsupported(
                "Boolean result contains a zero-volume boundary",
            ));
        }
        let shell = model.shells.len();
        model.shells.push(Shell {
            faces: faces
                .iter()
                .copied()
                .map(|face| FaceUse {
                    face,
                    reversed: false,
                })
                .collect(),
            closed: true,
        });
        components.push(ShellComponent { shell, faces, volume });
    }
    for component in components.iter().filter(|c| c.volume > 0.) {
        model.bodies.push(Body {
            outer_shell: component.shell,
            inner_shells: vec![],
        });
    }
    for cavity in components.iter().filter(|c| c.volume <= 0.) {
        attach_cavity(model, &components, cavity)?;
    }
    Ok(())
}

fn attach_cavity(model: &mut Model, components: &[ShellComponent], cavity: &ShellComponent) -> Result<()> {
    let sample = model.vertices[loop_vertices(model, model.faces[cavity.faces[0]].outer)?[0]].point;
    let mut containers: Vec<_> = components
        .iter()
        .filter(|candidate| candidate.volume > 0.)
        .filter_map(|candidate| {
            contains_faces(model, &candidate.faces, sample)
                .ok()
                .filter(|inside| *inside)
                .map(|_| (candidate.shell, candidate.volume))
        })
        .collect();
    containers.sort_by(|a, b| a.1.total_cmp(&b.1));
    let outer = containers
        .first()
        .map(|entry| entry.0)
        .ok_or_else(|| unsupported("Inverted boundary is not enclosed by an outer shell"))?;
    model
        .bodies
        .iter_mut()
        .find(|body| body.outer_shell == outer)
        .ok_or_else(|| failed("Cavity outer body was not constructed"))?
        .inner_shells
        .push(cavity.shell);
    Ok(())
}
