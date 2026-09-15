//! Tessellation and interchange preserve topological face identity.
use super::*;
use std::collections::BTreeMap;

pub struct Tessellation {
    pub built: BuiltMesh,
    pub face_ids: Vec<usize>,
    pub topology_face_ids: Option<Vec<String>>,
}
impl value_codec::Serialize for Tessellation {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        if let value_codec::Value::Object(fields) = value_codec::Serialize::to_value(&self.built) {
            object.extend(fields);
        }
        object.insert(
            "faceIds".into(),
            value_codec::Serialize::to_value(&self.face_ids),
        );
        if let Some(ids) = &self.topology_face_ids {
            object.insert(
                "topologyFaceIds".into(),
                value_codec::Serialize::to_value(ids),
            );
        }
        value_codec::Value::Object(object)
    }
}
pub(crate) fn weld(mesh: Mesh, tolerance: f64) -> Result<Mesh> {
    let mut positions = Vec::<f64>::new();
    let mut cells = BTreeMap::<[i64; 3], Vec<usize>>::new();
    let mut remap = vec![];
    for p in mesh.positions.as_chunks::<3>().0 {
        let cell = [
            (p[0] / tolerance).floor() as i64,
            (p[1] / tolerance).floor() as i64,
            (p[2] / tolerance).floor() as i64,
        ];
        let mut found = None;
        'search: for x in -1..=1 {
            for y in -1..=1 {
                for z in -1..=1 {
                    if let Some(ids) = cells.get(&[cell[0] + x, cell[1] + y, cell[2] + z]) {
                        for &id in ids {
                            if (0..3)
                                .map(|a| (positions[3 * id + a] - p[a]).powi(2))
                                .sum::<f64>()
                                .sqrt()
                                <= tolerance
                            {
                                found = Some(id);
                                break 'search;
                            }
                        }
                    }
                }
            }
        }
        let id = found.unwrap_or_else(|| {
            let id = positions.len() / 3;
            positions.extend(p);
            cells.entry(cell).or_default().push(id);
            id
        });
        remap.push(id);
    }
    let out = Mesh {
        positions,
        indices: mesh.indices.iter().map(|i| remap[*i]).collect(),
        uv: None,
    };
    out.validate()?;
    Ok(out)
}
fn finish(
    mesh: Mesh,
    face_ids: Vec<usize>,
    topology_face_ids: Option<Vec<String>>,
    tolerance: f64,
    closed: bool,
) -> Result<Tessellation> {
    finish_indexed(weld(mesh, tolerance)?, face_ids, topology_face_ids, closed)
}
fn finish_indexed(
    mesh: Mesh,
    face_ids: Vec<usize>,
    topology_face_ids: Option<Vec<String>>,
    closed: bool,
) -> Result<Tessellation> {
    let report = mesh.inspect()?;
    if report.degenerate_triangles > 0
        || report.non_manifold_edges > 0
        || report.orientation_conflicts > 0
        || closed && !report.closed
    {
        return Err(input(
            "B-rep tessellation does not preserve manifold seams at this resolution/tolerance",
        ));
    }
    Ok(Tessellation {
        built: BuiltMesh { mesh, report },
        face_ids,
        topology_face_ids,
    })
}
fn is_affine_plane(surface: &Surface, tolerance: f64) -> bool {
    surface.degree_u == 1
        && surface.degree_v == 1
        && surface.control_points.len() == 2
        && surface.control_points.iter().all(|r| r.len() == 2)
        && surface
            .weights
            .iter()
            .flatten()
            .all(|w| (*w - surface.weights[0][0]).abs() <= 1e-14)
        && (0..3).all(|i| {
            (surface.control_points[0][0][i] + surface.control_points[1][1][i]
                - surface.control_points[0][1][i]
                - surface.control_points[1][0][i])
                .abs()
                <= tolerance
        })
}
#[derive(Clone, Copy)]
struct BoundarySample {
    uv: [f64; 2],
    position: usize,
}
/// Geometric positions belong to authored vertices and edges, never to a
/// tolerance bucket. Two nearby disconnected shells keep disjoint indices.
struct EdgeSamplingRegistry {
    mesh: Mesh,
    edges: Vec<Vec<usize>>,
}
impl EdgeSamplingRegistry {
    fn new(model: &brep_core::Model, segments: usize) -> Result<Self> {
        let mut schedule = vec![1; model.edges.len()];
        for face in &model.faces {
            let curved = !is_affine_plane(&face.surface, model.tolerance_mm);
            for &wire in std::iter::once(&face.outer).chain(&face.holes) {
                for coedge in &model.loops[wire].coedges {
                    if curved || model.edges[coedge.edge].curve.degree > 1 {
                        schedule[coedge.edge] = segments;
                    }
                }
            }
        }
        let count = model.vertices.len() + schedule.iter().map(|n| n - 1).sum::<usize>();
        if count > 60000 {
            return Err(input("B-rep boundary sampling exceeds 60000 positions"));
        }
        let mut registry = Self {
            mesh: Mesh {
                positions: model.vertices.iter().flat_map(|v| v.point).collect(),
                indices: vec![],
                uv: None,
            },
            edges: Vec::with_capacity(model.edges.len()),
        };
        for (edge, divisions) in model.edges.iter().zip(schedule) {
            if edge.degenerate {
                registry.edges.push(vec![edge.vertices[0]; divisions + 1]);
                continue;
            }
            let [a, b] = edge.curve.domain();
            let mut samples = vec![edge.vertices[0]];
            for i in 1..divisions {
                let point = edge
                    .curve
                    .evaluate(a + (b - a) * i as f64 / divisions as f64)?
                    .point;
                samples.push(registry.mesh.positions.len() / 3);
                registry.mesh.positions.extend(point);
            }
            samples.push(edge.vertices[1]);
            registry.edges.push(samples);
        }
        Ok(registry)
    }
    fn divisions(&self, coedge: &brep_core::Coedge) -> usize {
        self.edges[coedge.edge].len() - 1
    }
    fn position(&self, coedge: &brep_core::Coedge, index: usize) -> Result<usize> {
        let divisions = self.divisions(coedge);
        if index > divisions {
            return Err(input(
                "Face sampling disagrees with its authored edge schedule",
            ));
        }
        Ok(self.edges[coedge.edge][if coedge.reversed {
            divisions - index
        } else {
            index
        }])
    }
    fn sample_wire(
        &self,
        model: &brep_core::Model,
        face: &brep_core::Face,
        wire: usize,
    ) -> Result<Vec<BoundarySample>> {
        let mut samples = Vec::new();
        for coedge in &model.loops[wire].coedges {
            let divisions = self.divisions(coedge);
            let [a, b] = coedge.pcurve.domain();
            for i in 0..=divisions {
                let canonical_index = if coedge.reversed { divisions - i } else { i };
                let parameter = canonical_index as f64 / divisions as f64;
                let local_parameter = if coedge.reversed {
                    1. - parameter
                } else {
                    parameter
                };
                let p = coedge.pcurve.evaluate(a + (b - a) * local_parameter)?.point;
                let uv = [p[0], p[1]];
                let position = self.position(coedge, i)?;
                let expected = self.mesh.point(position)?;
                let actual = face.surface.evaluate(uv[0], uv[1])?.point;
                if expected
                    .iter()
                    .zip(actual)
                    .map(|(a, b)| (a - b).powi(2))
                    .sum::<f64>()
                    .sqrt()
                    > model.tolerance_mm
                {
                    return Err(input(
                        "Face pcurve disagrees with its authoritative edge samples",
                    ));
                }
                if i < divisions {
                    samples.push(BoundarySample { uv, position });
                }
            }
        }
        Ok(samples)
    }
    fn verify_boundary_uses(&self, model: &brep_core::Model, face_ids: &[usize]) -> Result<()> {
        let mut actual = BTreeMap::<(usize, usize, usize), i32>::new();
        let add = |map: &mut BTreeMap<(usize, usize, usize), i32>, face, a: usize, b: usize| {
            *map.entry((face, a.min(b), a.max(b))).or_default() += if a < b { 1 } else { -1 };
        };
        for (triangle, &face) in self.mesh.indices.as_chunks::<3>().0.iter().zip(face_ids) {
            for i in 0..3 {
                add(&mut actual, face, triangle[i], triangle[(i + 1) % 3]);
            }
        }
        let mut expected = BTreeMap::<(usize, usize, usize), i32>::new();
        for shell in &model.shells {
            for use_ in &shell.faces {
                let face = &model.faces[use_.face];
                for &wire in std::iter::once(&face.outer).chain(&face.holes) {
                    for coedge in &model.loops[wire].coedges {
                        for segment in self.edges[coedge.edge].windows(2) {
                            let [a, b] = [segment[0], segment[1]];
                            if a == b {
                                if model.edges[coedge.edge].degenerate {
                                    continue;
                                }
                                return Err(input(
                                    "Resolution collapses a nondegenerate authored edge",
                                ));
                            }
                            let reversed = coedge.reversed != use_.reversed;
                            add(
                                &mut expected,
                                use_.face,
                                if reversed { b } else { a },
                                if reversed { a } else { b },
                            );
                        }
                    }
                }
            }
        }
        actual.retain(|_, count| *count != 0);
        expected.retain(|_, count| *count != 0);
        if actual != expected {
            return Err(input(
                "Tessellation face boundaries do not match the authored edge sample registry",
            ));
        }
        Ok(())
    }
    fn append(
        &mut self,
        face: &brep_core::Face,
        use_: &brep_core::FaceUse,
        built: FaceMesh,
        face_ids: &mut Vec<usize>,
    ) -> Result<()> {
        let mut remap = Vec::with_capacity(built.uv.len());
        for (uv, shared) in built.uv.iter().zip(built.shared) {
            remap.push(if let Some(position) = shared {
                position
            } else {
                let index = self.mesh.positions.len() / 3;
                if index >= 60000 {
                    return Err(input("B-rep tessellation exceeds 60000 positions"));
                }
                self.mesh
                    .positions
                    .extend(face.surface.evaluate(uv[0], uv[1])?.point);
                index
            });
        }
        for triangle in built.triangles {
            let mut mapped = triangle.map(|i| remap[i]);
            if use_.reversed {
                mapped.swap(1, 2);
            }
            self.mesh.indices.extend(mapped);
            face_ids.push(use_.face);
        }
        if face_ids.len() > 20000 {
            return Err(input("B-rep tessellation exceeds 20000 triangles"));
        }
        Ok(())
    }
}
struct FaceMesh {
    uv: Vec<[f64; 2]>,
    shared: Vec<Option<usize>>,
    triangles: Vec<[usize; 3]>,
}
fn uv_key(point: [f64; 2]) -> [u64; 2] {
    point.map(|x| if x == 0. { 0 } else { x.to_bits() })
}
fn triangulate_boundary(
    outer: &[BoundarySample],
    holes: &[Vec<BoundarySample>],
) -> Result<FaceMesh> {
    let mut boundary = BTreeMap::new();
    for sample in outer.iter().chain(holes.iter().flatten()) {
        if let Some(previous) = boundary.insert(uv_key(sample.uv), sample.position)
            && previous != sample.position
        {
            return Err(input(
                "Distinct authored boundaries have an ambiguous shared UV sample",
            ));
        }
    }
    let fill = planar_geometry::triangulation::triangulate_profile(
        &outer.iter().map(|p| p.uv).collect::<Vec<_>>(),
        &holes
            .iter()
            .map(|wire| wire.iter().map(|p| p.uv).collect())
            .collect::<Vec<_>>(),
    )?;
    let mut out = FaceMesh {
        uv: vec![],
        shared: vec![],
        triangles: vec![],
    };
    let mut local = BTreeMap::<[u64; 2], usize>::new();
    let mut remap = Vec::new();
    for point in fill.positions {
        let key = uv_key(point);
        let global = *boundary
            .get(&key)
            .ok_or_else(|| input("Triangulation introduced an unowned boundary position"))?;
        let index = *local.entry(key).or_insert_with(|| {
            let index = out.uv.len();
            out.uv.push(point);
            out.shared.push(Some(global));
            index
        });
        remap.push(index);
    }
    for triangle in fill.indices.as_chunks::<3>().0 {
        let triangle = triangle.map(|index| remap[index as usize]);
        if (0..3).any(|i| triangle[i] == triangle[(i + 1) % 3]) {
            return Err(input("Trim triangulation collapsed an authored boundary"));
        }
        out.triangles.push(triangle);
    }
    Ok(out)
}
pub fn nurbs(model: &brep_core::Model, segments: usize) -> Result<Tessellation> {
    model.validate()?;
    if !(1..=32).contains(&segments) {
        return Err(input("B-rep tessellation segments must be 1..32"));
    }
    let mut registry = EdgeSamplingRegistry::new(model, segments)?;
    let mut face_ids = Vec::new();
    for shell in &model.shells {
        for use_ in &shell.faces {
            let face = &model.faces[use_.face];
            let outer = registry.sample_wire(model, face, face.outer)?;
            let holes = face
                .holes
                .iter()
                .map(|&wire| registry.sample_wire(model, face, wire))
                .collect::<Result<Vec<_>>>()?;
            let built = if is_affine_plane(&face.surface, model.tolerance_mm) {
                triangulate_boundary(&outer, &holes)?
            } else if let Some(rectangular) = rectangular_face(model, face, segments, &registry)? {
                rectangular
            } else if let Some(quarter) = quarter_disk_face(model, face, segments, &registry)? {
                quarter
            } else {
                refine_trimmed_face(face, triangulate_boundary(&outer, &holes)?, segments)?
            };
            registry.append(face, use_, built, &mut face_ids)?;
        }
    }
    registry.verify_boundary_uses(model, &face_ids)?;
    let topology_face_ids = face_ids
        .iter()
        .map(|&face| model.1.faces[face].clone())
        .collect();
    finish_indexed(
        registry.mesh,
        face_ids,
        Some(topology_face_ids),
        !model.shells.is_empty() && model.shells.iter().all(|s| s.closed),
    )
}
fn rectangular_face(
    model: &brep_core::Model,
    face: &brep_core::Face,
    segments: usize,
    registry: &EdgeSamplingRegistry,
) -> Result<Option<FaceMesh>> {
    let corners = model.loop_uv(face.outer, 1)?;
    let coedges = &model.loops[face.outer].coedges;
    let rectangular = face.holes.is_empty()
        && corners.len() == 4
        && coedges.iter().all(|c| {
            c.pcurve.degree == 1
                && c.pcurve.control_points.len() == 2
                && c.pcurve.weights.iter().all(|w| *w == c.pcurve.weights[0])
        })
        && corners[0][1] == corners[1][1]
        && corners[1][0] == corners[2][0]
        && corners[2][1] == corners[3][1]
        && corners[3][0] == corners[0][0]
        && corners[1][0] > corners[0][0]
        && corners[3][1] > corners[0][1];
    if !rectangular {
        return Ok(None);
    }
    if coedges.iter().any(|c| registry.divisions(c) != segments) {
        return Err(input("Rectangular patch edge schedules disagree"));
    }
    let poles: Vec<_> = coedges
        .iter()
        .filter(|c| model.edges[c.edge].degenerate)
        .map(|c| model.edges[c.edge].vertices[0])
        .collect();
    let mut out = FaceMesh {
        uv: vec![],
        shared: vec![],
        triangles: vec![],
    };
    for y in 0..=segments {
        for x in 0..=segments {
            let mut shared = None;
            for (boundary, coedge, sample) in [
                (y == 0, 0, x),
                (x == segments, 1, y),
                (y == segments, 2, segments - x),
                (x == 0, 3, segments - y),
            ] {
                if boundary {
                    let id = registry.position(&coedges[coedge], sample)?;
                    if let Some(previous) = shared
                        && previous != id
                    {
                        return Err(input(
                            "Rectangular patch corners have conflicting authored vertices",
                        ));
                    }
                    shared = Some(id);
                }
            }
            out.uv.push([
                corners[0][0] + (corners[1][0] - corners[0][0]) * x as f64 / segments as f64,
                corners[0][1] + (corners[3][1] - corners[0][1]) * y as f64 / segments as f64,
            ]);
            out.shared.push(shared);
        }
    }
    for y in 0..segments {
        for x in 0..segments {
            let a = y * (segments + 1) + x;
            let b = a + 1;
            let d = a + segments + 1;
            let c = d + 1;
            for triangle in [[a, b, c], [a, c, d]] {
                let repeated = (0..3).find_map(|i| {
                    let first = out.shared[triangle[i]];
                    first.filter(|_| first == out.shared[triangle[(i + 1) % 3]])
                });
                if let Some(vertex) = repeated {
                    if !poles.contains(&vertex) {
                        return Err(input(
                            "Sampling resolution collapses a nondegenerate boundary",
                        ));
                    }
                    continue;
                }
                out.triangles.push(triangle);
            }
        }
    }
    Ok(Some(out))
}
fn quarter_disk_face(
    model: &brep_core::Model,
    face: &brep_core::Face,
    segments: usize,
    registry: &EdgeSamplingRegistry,
) -> Result<Option<FaceMesh>> {
    let coedges = &model.loops[face.outer].coedges;
    if !face.holes.is_empty() || coedges.len() != 3 {
        return Ok(None);
    }
    let a = &coedges[0].pcurve;
    let arc = &coedges[1].pcurve;
    let b = &coedges[2].pcurve;
    let bezier = |c: &Curve| {
        let [low, high] = c.domain();
        c.knots[..=c.degree].iter().all(|k| *k == low)
            && c.knots[c.control_points.len()..].iter().all(|k| *k == high)
    };
    let straight = |c: &Curve, points: &[Vec<f64>]| {
        bezier(c) && c.degree == 1 && c.control_points == points && c.weights[0] == c.weights[1]
    };
    let quarter = straight(a, &[vec![0., 0.], vec![1., 0.]])
        && straight(b, &[vec![0., 1.], vec![0., 0.]])
        && bezier(arc)
        && arc.degree == 2
        && arc.control_points == vec![vec![1., 0.], vec![1., 1.], vec![0., 1.]]
        && arc.weights == vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.];
    if !quarter {
        return Ok(None);
    }
    if coedges.iter().any(|c| registry.divisions(c) != segments) {
        return Err(input("Quarter-disk edge schedules disagree"));
    }
    let mut out = FaceMesh {
        uv: vec![[0., 0.]],
        shared: vec![Some(registry.position(&coedges[0], 0)?)],
        triangles: vec![],
    };
    let [low, high] = arc.domain();
    for row in 1..=segments {
        for column in 0..=segments {
            let p = arc
                .evaluate(low + (high - low) * column as f64 / segments as f64)?
                .point;
            let radius = row as f64 / segments as f64;
            out.uv.push([p[0] * radius, p[1] * radius]);
            out.shared.push(if row == segments {
                Some(registry.position(&coedges[1], column)?)
            } else if column == 0 {
                Some(registry.position(&coedges[0], row)?)
            } else if column == segments {
                Some(registry.position(&coedges[2], segments - row)?)
            } else {
                None
            });
        }
    }
    for j in 0..segments {
        out.triangles.push([0, 1 + j, 2 + j]);
    }
    for row in 1..segments {
        for j in 0..segments {
            let a = 1 + (row - 1) * (segments + 1) + j;
            let b = a + 1;
            let d = a + segments + 1;
            let c = d + 1;
            out.triangles.extend([[a, d, c], [a, c, b]]);
        }
    }
    Ok(Some(out))
}
/// Split only interior UV edges; all boundary positions retain their registry ownership.
fn refine_trimmed_face(
    face: &brep_core::Face,
    mesh: FaceMesh,
    segments: usize,
) -> Result<FaceMesh> {
    let surface = &face.surface;
    let mut uv = mesh.uv;
    let mut shared = mesh.shared;
    let mut triangles = mesh.triangles;
    let width = surface.knots_u[surface.control_points.len()] - surface.knots_u[surface.degree_u];
    let height =
        surface.knots_v[surface.control_points[0].len()] - surface.knots_v[surface.degree_v];
    let key = |a: usize, b: usize| if a < b { [a, b] } else { [b, a] };
    for _ in 0..24 {
        let mut incidence = BTreeMap::<[usize; 2], usize>::new();
        for t in &triangles {
            for i in 0..3 {
                *incidence.entry(key(t[i], t[(i + 1) % 3])).or_default() += 1;
            }
        }
        let mut splits = BTreeMap::<[usize; 2], usize>::new();
        for (edge, count) in incidence {
            let [a, b] = edge;
            let length =
                ((uv[a][0] - uv[b][0]) / width).powi(2) + ((uv[a][1] - uv[b][1]) / height).powi(2);
            if count == 2 && length > 2. / (segments * segments) as f64 {
                splits.insert(edge, uv.len());
                uv.push([(uv[a][0] + uv[b][0]) / 2., (uv[a][1] + uv[b][1]) / 2.]);
                shared.push(None);
            }
        }
        if splits.is_empty() {
            return Ok(FaceMesh {
                uv,
                shared,
                triangles,
            });
        }
        let mut refined = Vec::new();
        for mut t in triangles {
            let count = (0..3)
                .filter(|&i| splits.contains_key(&key(t[i], t[(i + 1) % 3])))
                .count();
            if count == 0 {
                refined.push(t);
                continue;
            }
            if count == 3 {
                let [a, b, c] = t;
                let ab = splits[&key(a, b)];
                let bc = splits[&key(b, c)];
                let ca = splits[&key(c, a)];
                refined.extend([[a, ab, ca], [ab, b, bc], [ca, bc, c], [ab, bc, ca]]);
            } else {
                // Rotate so the one marked edge is AB, or the two are AB and BC.
                while !(splits.contains_key(&key(t[0], t[1]))
                    && (count == 1 || splits.contains_key(&key(t[1], t[2]))))
                {
                    t.rotate_left(1);
                }
                let [a, b, c] = t;
                let ab = splits[&key(a, b)];
                if count == 1 {
                    refined.extend([[a, ab, c], [ab, b, c]]);
                } else {
                    let bc = splits[&key(b, c)];
                    refined.extend([[ab, b, bc], [a, ab, c], [ab, bc, c]]);
                }
            }
        }
        if refined.len() > 20000 {
            return Err(input("B-rep tessellation exceeds 20000 triangles"));
        }
        triangles = refined;
    }
    Err(input(
        "B-rep interior tessellation exceeded its refinement budget",
    ))
}
pub fn polygons(model: &polygon_core::solid::brep::Model) -> Result<Tessellation> {
    let t = polygon_core::solid::brep::tessellate(model)?;
    finish(
        t.mesh,
        t.face_ids,
        None,
        model.tolerance_mm,
        model.shells.iter().all(|s| s.closed),
    )
}

#[cfg(test)]
mod registry_tests {
    use super::*;
    use std::collections::BTreeSet;

    fn empty_model() -> brep_core::Model {
        brep_core::Model(
            brep_topology::Model {
                vertices: vec![],
                edges: vec![],
                loops: vec![],
                faces: vec![],
                shells: vec![],
                bodies: vec![],
                tolerance_mm: 1e-7,
            },
            brep_core::TopologyIds::default(),
        )
    }
    fn append_model(target: &mut brep_core::Model, mut source: brep_core::Model) {
        let (vertices, edges, loops, faces, shells) = (
            target.vertices.len(),
            target.edges.len(),
            target.loops.len(),
            target.faces.len(),
            target.shells.len(),
        );
        for edge in &mut source.edges {
            edge.vertices = edge.vertices.map(|id| id + vertices);
        }
        for wire in &mut source.loops {
            for coedge in &mut wire.coedges {
                coedge.edge += edges;
            }
        }
        for face in &mut source.faces {
            face.outer += loops;
            for wire in &mut face.holes {
                *wire += loops;
            }
        }
        for shell in &mut source.shells {
            for face in &mut shell.faces {
                face.face += faces;
            }
        }
        for body in &mut source.bodies {
            body.outer_shell += shells;
            for shell in &mut body.inner_shells {
                *shell += shells;
            }
        }
        target.vertices.append(&mut source.vertices);
        target.edges.append(&mut source.edges);
        target.loops.append(&mut source.loops);
        target.faces.append(&mut source.faces);
        target.shells.append(&mut source.shells);
        target.bodies.append(&mut source.bodies);
        target.rebuild_topology_ids();
    }
    fn components(mesh: &Mesh) -> usize {
        let mut adjacency = vec![Vec::new(); mesh.positions.len() / 3];
        for triangle in mesh.indices.as_chunks::<3>().0 {
            for i in 0..3 {
                adjacency[triangle[i]].push(triangle[(i + 1) % 3]);
                adjacency[triangle[(i + 1) % 3]].push(triangle[i]);
            }
        }
        let mut visited = BTreeSet::new();
        let mut components = 0;
        for i in 0..adjacency.len() {
            if adjacency[i].is_empty() || visited.contains(&i) {
                continue;
            }
            components += 1;
            let mut pending = vec![i];
            while let Some(vertex) = pending.pop() {
                if visited.insert(vertex) {
                    pending.extend(&adjacency[vertex]);
                }
            }
        }
        components
    }
    #[test]
    fn nearby_disconnected_shells_keep_distinct_authored_vertices() {
        let gap = 1e-8;
        let mut model = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
        append_model(
            &mut model,
            brep_core::cuboid([1. + gap, 0., 0.], [2. + gap, 1., 1.]).unwrap(),
        );
        model.validate().unwrap();
        assert!(gap < model.tolerance_mm);
        let built = nurbs(&model, 4).unwrap();
        assert!(built.built.report.closed);
        assert_eq!(built.built.report.vertex_count, 16);
        assert_eq!(components(&built.built.mesh), 2);
        assert!((built.built.report.signed_volume_mm3 - 2.).abs() < 1e-12);
        for (vertex, expected) in model.vertices.iter().enumerate() {
            assert_eq!(built.built.mesh.point(vertex).unwrap(), expected.point);
        }
    }
    #[test]
    fn poles_round_holes_and_partial_turns_share_edge_indices_at_each_detail() {
        let models = [
            brep_core::sphere(3.).unwrap(),
            brep_core::frustum(3., 0., 5.).unwrap(),
            brep_core::frustum(0., 3., 5.).unwrap(),
            brep_core::tube(3., 2., 5.).unwrap(),
            brep_core::revolve_angle(&[[0., 0.], [2., 0.], [2., 3.], [0., 3.]], 125.).unwrap(),
        ];
        for (kind, model) in models.iter().enumerate() {
            for detail in [1, 2, 4, 8, 16, 32] {
                let result = nurbs(model, detail)
                    .unwrap_or_else(|e| panic!("model {kind}, detail {detail}: {e}"));
                assert!(result.built.report.closed, "model {kind}, detail {detail}");
                assert_eq!(result.built.report.boundary_edges, 0);
                assert_eq!(result.built.report.non_manifold_edges, 0);
                assert_eq!(result.built.report.orientation_conflicts, 0);
                assert_eq!(result.built.report.degenerate_triangles, 0);
                assert!(result.built.report.signed_volume_mm3 > 0.);
                assert_eq!(components(&result.built.mesh), 1);
                assert_eq!(
                    result.topology_face_ids.unwrap(),
                    result
                        .face_ids
                        .iter()
                        .map(|&face| model.1.faces[face].clone())
                        .collect::<Vec<_>>()
                );
            }
        }
    }
    #[test]
    fn torus_periodic_seams_preserve_genus_and_obey_triangle_budget() {
        let model = brep_core::torus(5., 2.).unwrap();
        for detail in [1, 2, 4, 8, 16] {
            let built = nurbs(&model, detail).unwrap();
            assert!(built.built.report.closed);
            assert_eq!(components(&built.built.mesh), 1);
            let edges: BTreeSet<_> = built
                .built
                .mesh
                .indices
                .as_chunks::<3>()
                .0
                .iter()
                .flat_map(|triangle| {
                    (0..3).map(|i| {
                        let a = triangle[i];
                        let b = triangle[(i + 1) % 3];
                        [a.min(b), a.max(b)]
                    })
                })
                .collect();
            assert_eq!(
                built.built.report.vertex_count + built.built.report.triangle_count,
                edges.len()
            );
        }
        assert!(
            nurbs(&model, 32)
                .err()
                .unwrap()
                .message
                .contains("20000 triangles")
        );
    }
    #[test]
    fn curved_prism_xor_keeps_contacting_components_and_trim_samples_separate() {
        let a = brep_core::cylinder(2., 3.).unwrap();
        let mut b = a.clone();
        for vertex in &mut b.vertices {
            vertex.point[0] += 2.;
        }
        for edge in &mut b.edges {
            for point in &mut edge.curve.control_points {
                point[0] += 2.;
            }
        }
        for face in &mut b.faces {
            for point in face.surface.control_points.iter_mut().flatten() {
                point[0] += 2.;
            }
        }
        b.validate().unwrap();
        let model = brep_core::boolean(&a, &b, "xor").unwrap();
        model.validate().unwrap();
        assert_eq!(model.bodies.len(), 2);
        let exact = (8. * std::f64::consts::PI / 3. + 4. * 3_f64.sqrt()) * 3.;
        for detail in [1, 2, 4, 8, 16] {
            let built = nurbs(&model, detail)
                .unwrap_or_else(|error| panic!("XOR detail {detail}: {error}"));
            assert!(built.built.report.closed);
            assert_eq!(components(&built.built.mesh), 2);
            assert_eq!(built.built.report.orientation_conflicts, 0);
            assert_eq!(built.built.report.non_manifold_edges, 0);
            assert_eq!(built.built.report.degenerate_triangles, 0);
            if detail == 16 {
                assert!((built.built.report.signed_volume_mm3 - exact).abs() / exact < 0.003);
            }
        }
    }
    fn holed_curved_patch(touching: bool) -> brep_core::Model {
        let surface = Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: (0..3)
                .map(|i| {
                    (0..3)
                        .map(|j| {
                            vec![
                                i as f64 / 2.,
                                j as f64 / 2.,
                                (if i == 2 { 1. } else { 0. }) + (if j == 2 { 1. } else { 0. }),
                            ]
                        })
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 3]; 3],
            periodic_u: false,
            periodic_v: false,
        };
        let mut model = empty_model();
        let hole = if touching {
            vec![[0., 0.], [0., 0.5], [0.5, 0.5], [0.5, 0.]]
        } else {
            vec![[0.25, 0.25], [0.25, 0.75], [0.75, 0.75], [0.75, 0.25]]
        };
        for ring in [vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]], hole] {
            let start = model.vertices.len();
            for p in &ring {
                model.vertices.push(brep_core::Vertex {
                    point: surface.evaluate(p[0], p[1]).unwrap().point,
                });
            }
            let mut coedges = Vec::new();
            for i in 0..ring.len() {
                let a = ring[i];
                let b = ring[(i + 1) % ring.len()];
                let pa = surface.evaluate(a[0], a[1]).unwrap().point;
                let pb = surface.evaluate(b[0], b[1]).unwrap().point;
                let mid = surface
                    .evaluate((a[0] + b[0]) / 2., (a[1] + b[1]) / 2.)
                    .unwrap()
                    .point;
                let control = (0..3)
                    .map(|axis| 2. * mid[axis] - (pa[axis] + pb[axis]) / 2.)
                    .collect();
                let curve = Curve {
                    degree: 2,
                    knots: vec![0., 0., 0., 1., 1., 1.],
                    control_points: vec![pa.to_vec(), control, pb.to_vec()],
                    weights: vec![1.; 3],
                    periodic: false,
                };
                coedges.push(brep_core::Coedge {
                    edge: model.edges.len(),
                    reversed: false,
                    pcurve: Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap(),
                });
                model.edges.push(brep_core::Edge {
                    vertices: [start + i, start + (i + 1) % ring.len()],
                    curve,
                    degenerate: false,
                });
            }
            model.loops.push(brep_core::Loop { coedges });
        }
        model.faces.push(brep_core::Face {
            surface,
            outer: 0,
            holes: vec![1],
        });
        model.shells.push(brep_core::Shell {
            faces: vec![brep_core::FaceUse {
                face: 0,
                reversed: false,
            }],
            closed: false,
        });
        model.rebuild_topology_ids();
        if touching {
            model.1.vertices[4] = "v:separate-touching-vertex".into();
        }
        model.validate().unwrap();
        model
    }
    #[test]
    fn curved_trim_holes_preserve_registry_ownership_through_bridge_refinement() {
        let model = holed_curved_patch(false);
        for detail in [1, 2, 4, 8, 16] {
            let built =
                nurbs(&model, detail).unwrap_or_else(|error| panic!("detail {detail}: {error}"));
            assert!(!built.built.report.closed);
            assert_eq!(built.built.report.boundary_edges, 8 * detail);
            assert_eq!(built.built.report.non_manifold_edges, 0);
            assert_eq!(built.built.report.orientation_conflicts, 0);
            assert_eq!(built.built.report.degenerate_triangles, 0);
            assert_eq!(components(&built.built.mesh), 1);
        }
    }
    #[test]
    fn coincident_uv_samples_from_different_topology_are_rejected() {
        let model = holed_curved_patch(true);
        let error = match nurbs(&model, 4) {
            Ok(_) => panic!("ambiguous topology accepted"),
            Err(error) => error,
        };
        assert!(error.message.contains("ambiguous shared UV"));
    }
    #[test]
    fn empty_brep_returns_an_empty_mesh_without_claiming_a_closed_shell() {
        let model = empty_model();
        let built = nurbs(&model, 4).unwrap();
        assert!(built.built.mesh.positions.is_empty());
        assert!(built.built.mesh.indices.is_empty());
        assert!(!built.built.report.closed);
        assert_eq!(built.built.report.signed_volume_mm3, 0.);
        assert!(built.face_ids.is_empty());
        assert!(built.topology_face_ids.unwrap().is_empty());
    }

    #[test]
    fn shared_edge_registry_gives_identical_indices_to_adjacent_faces() {
        let model = brep_core::cylinder(2., 4.).unwrap();
        let built = nurbs(&model, 8).unwrap();
        assert!(
            !built.built.report.closed || built.built.report.non_manifold_edges == 0,
            "shared-edge tess must remain manifold: {:?}",
            built.built.report
        );
        // Adjacent wall/cap triangles must share exact vertex indices along the
        // circular rim — not merely coincident coordinates.
        let positions = &built.built.mesh.positions;
        let indices = &built.built.mesh.indices;
        let mut edge_uses: std::collections::BTreeMap<(usize, usize), usize> =
            std::collections::BTreeMap::new();
        for tri in indices.chunks_exact(3) {
            for [a, b] in [[tri[0], tri[1]], [tri[1], tri[2]], [tri[2], tri[0]]] {
                let key = if a < b { (a, b) } else { (b, a) };
                *edge_uses.entry(key).or_default() += 1;
            }
        }
        let shared = edge_uses.values().filter(|&&n| n == 2).count();
        let boundary = edge_uses.values().filter(|&&n| n == 1).count();
        assert!(shared > 0, "expected dual-face shared mesh edges");
        assert_eq!(
            boundary, 0,
            "closed cylinder tess must not leave singleton mesh edges"
        );
        // Spot-check: every shared edge endpoint has finite coords (no NaN weld).
        for &(a, b) in edge_uses.iter().filter(|(_, n)| **n == 2).map(|(k, _)| k) {
            for i in [a, b] {
                for c in 0..3 {
                    assert!(positions[i * 3 + c].is_finite());
                }
            }
        }
        let _ = positions;
    }
}
