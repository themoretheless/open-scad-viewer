//! Fail-closed solid operations for the planar subset of the NURBS B-rep.
//!
//! Booleans accept closed, orthogonal, planar solids and return one connected
//! boundary (disconnected results and enclosed cavities are rejected).
//! Chamfers and fillets accept one convex planar body. Fillets are represented
//! by planar tangent facets; `segments` controls that declared approximation.
use super::*;
use std::collections::{BTreeMap, BTreeSet};

const UNSUPPORTED: &str = "BREP_UNSUPPORTED_OPERATION";
const OPERATION_FAILED: &str = "BREP_OPERATION_FAILED";

fn unsupported(message: impl Into<String>) -> Error {
    Error::new(UNSUPPORTED, message)
}
fn failed(message: impl Into<String>) -> Error {
    Error::new(OPERATION_FAILED, message)
}
fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] + b[i])
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}
fn mul(a: [f64; 3], s: f64) -> [f64; 3] {
    a.map(|v| v * s)
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|i| a[i] * b[i]).sum()
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn norm(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}
fn unit(a: [f64; 3]) -> Result<[f64; 3]> {
    let n = norm(a);
    if n <= 1e-12 {
        return Err(unsupported("Degenerate planar face"));
    }
    Ok(mul(a, 1. / n))
}
fn close(a: [f64; 3], b: [f64; 3], tolerance: f64) -> bool {
    norm(sub(a, b)) <= tolerance
}
fn loop_vertices(model: &Model, loop_id: usize) -> Result<Vec<usize>> {
    let wire = model
        .loops
        .get(loop_id)
        .ok_or_else(|| failed("Face references an unknown loop"))?;
    Ok(wire
        .coedges
        .iter()
        .map(|c| model.edges[c.edge].vertices[usize::from(c.reversed)])
        .collect())
}

#[derive(Clone, Copy)]
struct Plane {
    normal: [f64; 3],
    offset: f64,
}

fn face_plane(model: &Model, face_id: usize) -> Result<Plane> {
    let face = &model.faces[face_id];
    if !face.holes.is_empty() {
        return Err(unsupported(
            "Planar operations do not accept trimmed face holes",
        ));
    }
    let ids = loop_vertices(model, face.outer)?;
    if ids.len() < 3 {
        return Err(unsupported("Face has fewer than three corners"));
    }
    let points: Vec<_> = ids.iter().map(|&i| model.vertices[i].point).collect();
    let origin = points[0];
    let mut normal = None;
    for i in 1..points.len() {
        for j in i + 1..points.len() {
            let candidate = cross(sub(points[i], origin), sub(points[j], origin));
            if norm(candidate) > model.tolerance_mm {
                normal = Some(unit(candidate)?);
                break;
            }
        }
        if normal.is_some() {
            break;
        }
    }
    let normal = normal.ok_or_else(|| unsupported("Degenerate planar face"))?;
    let offset = dot(normal, origin);
    if points
        .iter()
        .any(|&p| (dot(normal, p) - offset).abs() > model.tolerance_mm * 8.)
    {
        return Err(unsupported("Operation requires planar faces"));
    }
    Ok(Plane { normal, offset })
}

fn convex_planes(model: &Model) -> Result<Vec<Plane>> {
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

fn plane_basis(normal: [f64; 3]) -> Result<([f64; 3], [f64; 3])> {
    let reference = if normal[0].abs() < 0.8 {
        [1., 0., 0.]
    } else {
        [0., 1., 0.]
    };
    let u = unit(cross(reference, normal))?;
    Ok((u, cross(normal, u)))
}

fn model_from_polygons(polygons: Vec<Vec<[f64; 3]>>, tolerance: f64) -> Result<Model> {
    if polygons.len() > 256 {
        return Err(Error::new(
            "BREP_RESOURCE_LIMIT",
            "Result exceeds 256 faces",
        ));
    }
    let mut model = Model(brep_topology::Model {
        vertices: vec![],
        edges: vec![],
        loops: vec![],
        faces: vec![],
        shells: vec![],
        bodies: vec![],
        tolerance_mm: tolerance,
    });
    let mut edge_map = BTreeMap::<(usize, usize), usize>::new();
    for polygon in polygons {
        if polygon.len() < 3 {
            return Err(failed("Operation produced a degenerate face"));
        }
        let mut ids = Vec::with_capacity(polygon.len());
        for point in &polygon {
            let id = model
                .vertices
                .iter()
                .position(|v| close(v.point, *point, tolerance * 4.))
                .unwrap_or_else(|| {
                    let id = model.vertices.len();
                    model.vertices.push(Vertex { point: *point });
                    id
                });
            ids.push(id);
        }
        let normal = unit(cross(
            sub(polygon[1], polygon[0]),
            sub(polygon[2], polygon[0]),
        ))?;
        let (u, v) = plane_basis(normal)?;
        let origin = polygon[0];
        let coordinates: Vec<_> = polygon
            .iter()
            .map(|&p| {
                let d = sub(p, origin);
                [dot(d, u), dot(d, v)]
            })
            .collect();
        let min_u = coordinates
            .iter()
            .map(|p| p[0])
            .fold(f64::INFINITY, f64::min);
        let max_u = coordinates
            .iter()
            .map(|p| p[0])
            .fold(f64::NEG_INFINITY, f64::max);
        let min_v = coordinates
            .iter()
            .map(|p| p[1])
            .fold(f64::INFINITY, f64::min);
        let max_v = coordinates
            .iter()
            .map(|p| p[1])
            .fold(f64::NEG_INFINITY, f64::max);
        if max_u - min_u <= tolerance || max_v - min_v <= tolerance {
            return Err(failed("Operation produced a collapsed face"));
        }
        let surface_origin = add(origin, add(mul(u, min_u), mul(v, min_v)));
        let du = mul(u, max_u - min_u);
        let dv = mul(v, max_v - min_v);
        let uv: Vec<_> = coordinates
            .iter()
            .map(|p| {
                [
                    (p[0] - min_u) / (max_u - min_u),
                    (p[1] - min_v) / (max_v - min_v),
                ]
            })
            .collect();
        let mut coedges = Vec::with_capacity(ids.len());
        for i in 0..ids.len() {
            let a = ids[i];
            let b = ids[(i + 1) % ids.len()];
            let key = (a.min(b), a.max(b));
            let edge = *edge_map.entry(key).or_insert_with(|| {
                let id = model.edges.len();
                let start = model.vertices[key.0].point.to_vec();
                let end = model.vertices[key.1].point.to_vec();
                model.edges.push(Edge {
                    vertices: [key.0, key.1],
                    curve: line(start, end),
                });
                id
            });
            coedges.push(Coedge {
                edge,
                reversed: a > b,
                pcurve: line(uv[i].to_vec(), uv[(i + 1) % ids.len()].to_vec()),
            });
        }
        let outer = model.loops.len();
        model.loops.push(Loop { coedges });
        model.faces.push(Face {
            surface: Surface {
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
            },
            outer,
            holes: vec![],
        });
    }
    let face_count = model.faces.len();
    model.shells.push(Shell {
        faces: (0..face_count)
            .map(|face| FaceUse {
                face,
                reversed: false,
            })
            .collect(),
        closed: true,
    });
    model.bodies.push(Body {
        outer_shell: 0,
        inner_shells: vec![],
    });
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

fn model_from_planes(planes: &[Plane], tolerance: f64) -> Result<Model> {
    let mut points = Vec::<[f64; 3]>::new();
    for i in 0..planes.len() {
        for j in i + 1..planes.len() {
            for k in j + 1..planes.len() {
                let a = planes[i];
                let b = planes[j];
                let c = planes[k];
                let bc = cross(b.normal, c.normal);
                let determinant = dot(a.normal, bc);
                if determinant.abs() <= 1e-10 {
                    continue;
                }
                let point = mul(
                    add(
                        add(mul(bc, a.offset), mul(cross(c.normal, a.normal), b.offset)),
                        mul(cross(a.normal, b.normal), c.offset),
                    ),
                    1. / determinant,
                );
                if planes
                    .iter()
                    .any(|p| dot(p.normal, point) > p.offset + tolerance * 8.)
                    || points.iter().any(|&p| close(p, point, tolerance * 4.))
                {
                    continue;
                }
                points.push(point);
            }
        }
    }
    if points.len() < 4 {
        return Err(failed("Operation collapses the solid"));
    }
    let mut polygons = vec![];
    for plane in planes {
        let mut face: Vec<_> = points
            .iter()
            .copied()
            .filter(|&p| (dot(plane.normal, p) - plane.offset).abs() <= tolerance * 8.)
            .collect();
        if face.len() < 3 {
            continue;
        }
        let center = mul(
            face.iter().copied().fold([0.; 3], add),
            1. / face.len() as f64,
        );
        let (u, v) = plane_basis(plane.normal)?;
        face.sort_by(|a, b| {
            let da = sub(*a, center);
            let db = sub(*b, center);
            dot(da, v)
                .atan2(dot(da, u))
                .total_cmp(&dot(db, v).atan2(dot(db, u)))
        });
        polygons.push(face);
    }
    model_from_polygons(polygons, tolerance)
}

fn orthogonal(model: &Model) -> Result<()> {
    model.validate()?;
    if model.bodies.len() != 1
        || model.shells.len() != 1
        || !model.bodies[0].inner_shells.is_empty()
    {
        return Err(unsupported(
            "Boolean operands require one connected body without cavities",
        ));
    }
    for face_use in &model.shells[model.bodies[0].outer_shell].faces {
        let plane = face_plane(model, face_use.face)?;
        let axis = plane.normal.iter().filter(|v| v.abs() > 1. - 1e-8).count();
        if axis != 1 {
            return Err(unsupported(
                "Boolean operands must have axis-aligned planar faces",
            ));
        }
    }
    Ok(())
}

fn ray_triangle(
    origin: [f64; 3],
    direction: [f64; 3],
    a: [f64; 3],
    b: [f64; 3],
    c: [f64; 3],
) -> Option<f64> {
    let e1 = sub(b, a);
    let e2 = sub(c, a);
    let h = cross(direction, e2);
    let determinant = dot(e1, h);
    if determinant.abs() < 1e-10 {
        return None;
    }
    let f = 1. / determinant;
    let s = sub(origin, a);
    let u = f * dot(s, h);
    if !(-1e-10..=1. + 1e-10).contains(&u) {
        return None;
    }
    let q = cross(s, e1);
    let v = f * dot(direction, q);
    if v < -1e-10 || u + v > 1. + 1e-10 {
        return None;
    }
    let t = f * dot(e2, q);
    (t > 1e-9).then_some(t)
}

fn contains(model: &Model, point: [f64; 3]) -> Result<bool> {
    let direction = unit([1., 0.371_390_7, 0.217_113_9])?;
    let mut hits = vec![];
    for face_use in &model.shells[model.bodies[0].outer_shell].faces {
        let ids = loop_vertices(model, model.faces[face_use.face].outer)?;
        for i in 1..ids.len() - 1 {
            if let Some(t) = ray_triangle(
                point,
                direction,
                model.vertices[ids[0]].point,
                model.vertices[ids[i]].point,
                model.vertices[ids[i + 1]].point,
            ) {
                if !hits.iter().any(|x: &f64| (*x - t).abs() <= 1e-7) {
                    hits.push(t);
                }
            }
        }
    }
    Ok(hits.len() % 2 == 1)
}

/// Supported exact-topology boolean envelope: two axis-aligned, orthogonal,
/// closed planar bodies. The result must have one connected boundary and no
/// enclosed cavity. Coincident boundaries are resolved on the coordinate grid.
pub fn boolean(a: &Model, b: &Model, operation: &str) -> Result<Model> {
    orthogonal(a)?;
    orthogonal(b)?;
    if !matches!(operation, "union" | "difference" | "intersection") {
        return Err(Error::new(
            "BREP_INVALID_OPERATION",
            "Boolean operation must be union, difference, or intersection",
        ));
    }
    let mut coordinates: [Vec<f64>; 3] = std::array::from_fn(|_| vec![]);
    for vertex in a.vertices.iter().chain(&b.vertices) {
        for axis in 0..3 {
            coordinates[axis].push(vertex.point[axis]);
        }
    }
    for values in &mut coordinates {
        values.sort_by(f64::total_cmp);
        values.dedup_by(|x, y| (*x - *y).abs() <= a.tolerance_mm.max(b.tolerance_mm) * 4.);
        if values.len() > 32 {
            return Err(Error::new(
                "BREP_RESOURCE_LIMIT",
                "Boolean coordinate grid exceeds 31 cells per axis",
            ));
        }
    }
    let shape = [
        coordinates[0].len() - 1,
        coordinates[1].len() - 1,
        coordinates[2].len() - 1,
    ];
    let mut occupied = BTreeSet::<[usize; 3]>::new();
    for x in 0..shape[0] {
        for y in 0..shape[1] {
            for z in 0..shape[2] {
                let p = [
                    (coordinates[0][x] + coordinates[0][x + 1]) / 2.,
                    (coordinates[1][y] + coordinates[1][y + 1]) / 2.,
                    (coordinates[2][z] + coordinates[2][z + 1]) / 2.,
                ];
                let inside_a = contains(a, p)?;
                let inside_b = contains(b, p)?;
                let keep = match operation {
                    "union" => inside_a || inside_b,
                    "difference" => inside_a && !inside_b,
                    _ => inside_a && inside_b,
                };
                if keep {
                    occupied.insert([x, y, z]);
                }
            }
        }
    }
    if occupied.is_empty() {
        return Err(unsupported(
            "Boolean result is empty; empty B-rep bodies are not represented",
        ));
    }
    let mut polygons = vec![];
    let directions = [
        ([-1, 0, 0], 0usize),
        ([1, 0, 0], 0),
        ([0, -1, 0], 1),
        ([0, 1, 0], 1),
        ([0, 0, -1], 2),
        ([0, 0, 1], 2),
    ];
    for &[x, y, z] in &occupied {
        let cell = [x, y, z];
        let x0 = coordinates[0][x];
        let x1 = coordinates[0][x + 1];
        let y0 = coordinates[1][y];
        let y1 = coordinates[1][y + 1];
        let z0 = coordinates[2][z];
        let z1 = coordinates[2][z + 1];
        for (side, &(delta, axis)) in directions.iter().enumerate() {
            let adjacent = cell[axis] as isize + delta[axis] as isize;
            let exposed = adjacent < 0 || adjacent >= shape[axis] as isize || {
                let mut neighbor = cell;
                neighbor[axis] = adjacent as usize;
                !occupied.contains(&neighbor)
            };
            if !exposed {
                continue;
            }
            polygons.push(match side {
                0 => vec![[x0, y0, z0], [x0, y0, z1], [x0, y1, z1], [x0, y1, z0]],
                1 => vec![[x1, y0, z0], [x1, y1, z0], [x1, y1, z1], [x1, y0, z1]],
                2 => vec![[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]],
                3 => vec![[x0, y1, z0], [x0, y1, z1], [x1, y1, z1], [x1, y1, z0]],
                4 => vec![[x0, y0, z0], [x0, y1, z0], [x1, y1, z0], [x1, y0, z0]],
                _ => vec![[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]],
            });
        }
    }
    model_from_polygons(polygons, a.tolerance_mm.max(b.tolerance_mm))
}

fn edge_faces(model: &Model, edge_id: usize) -> Result<[usize; 2]> {
    if edge_id >= model.edges.len() {
        return Err(Error::new("BREP_INVALID_SELECTION", "Unknown edge"));
    }
    let mut faces = vec![];
    for face_use in &model.shells[model.bodies[0].outer_shell].faces {
        let face = &model.faces[face_use.face];
        if std::iter::once(&face.outer)
            .chain(&face.holes)
            .any(|&loop_id| {
                model.loops[loop_id]
                    .coedges
                    .iter()
                    .any(|c| c.edge == edge_id)
            })
        {
            faces.push(face_use.face);
        }
    }
    if faces.len() != 2 {
        return Err(unsupported(
            "Selected edge must have exactly two adjacent faces",
        ));
    }
    Ok([faces[0], faces[1]])
}

fn edge_operation(
    model: &Model,
    edge_id: usize,
    size: f64,
    segments: Option<usize>,
) -> Result<Model> {
    if !size.is_finite() || !(0.01..=1e5).contains(&size) {
        return Err(Error::new(
            "BREP_INVALID_SIZE",
            "Edge size must be finite and 0.01..100000 mm",
        ));
    }
    if let Some(segments) = segments
        && !(2..=32).contains(&segments)
    {
        return Err(Error::new(
            "BREP_RESOURCE_LIMIT",
            "Fillet segments must be 2..32",
        ));
    }
    let mut planes = convex_planes(model)?;
    let adjacent = edge_faces(model, edge_id)?;
    let a = face_plane(model, adjacent[0])?;
    let b = face_plane(model, adjacent[1])?;
    let selected_vertices = model.edges[edge_id].vertices;
    let available = adjacent
        .iter()
        .flat_map(|&face| loop_vertices(model, model.faces[face].outer).unwrap_or_default())
        .filter(|vertex| !selected_vertices.contains(vertex))
        .flat_map(|vertex| {
            selected_vertices
                .map(|end| norm(sub(model.vertices[vertex].point, model.vertices[end].point)))
        })
        .fold(f64::INFINITY, f64::min);
    if size >= available - model.tolerance_mm * 8. {
        return Err(unsupported(
            "Edge size consumes an adjacent face; use a smaller value",
        ));
    }
    let cosine = dot(a.normal, b.normal).clamp(-1., 1.);
    let alpha = cosine.acos();
    if alpha <= 1e-4 || std::f64::consts::PI - alpha <= 1e-4 {
        return Err(unsupported("Select a convex, non-tangent edge"));
    }
    let bisector = unit(add(a.normal, b.normal))?;
    let point = model.vertices[model.edges[edge_id].vertices[0]].point;
    if let Some(segments) = segments {
        let center = sub(point, mul(bisector, size / (alpha / 2.).cos()));
        for i in 1..segments {
            let t = i as f64 / segments as f64;
            let normal = unit(add(
                mul(a.normal, ((1. - t) * alpha).sin()),
                mul(b.normal, (t * alpha).sin()),
            ))?;
            planes.push(Plane {
                normal,
                offset: dot(normal, center) + size,
            });
        }
    } else {
        planes.push(Plane {
            normal: bisector,
            offset: dot(bisector, point) - size * (alpha / 2.).sin(),
        });
    }
    let result = model_from_planes(&planes, model.tolerance_mm)?;
    // A consumed adjacent support face means the requested size crossed a
    // neighboring feature even if the half-space intersection stayed nonempty.
    for original in [a, b] {
        let survives = (0..result.faces.len()).any(|face| {
            face_plane(&result, face)
                .map(|p| {
                    dot(p.normal, original.normal) > 1. - 1e-7
                        && (p.offset - original.offset).abs() <= model.tolerance_mm * 8.
                })
                .unwrap_or(false)
        });
        if !survives {
            return Err(unsupported(
                "Edge size consumes an adjacent face; use a smaller value",
            ));
        }
    }
    Ok(result)
}

/// Chamfer one convex edge of a single convex planar body.
pub fn chamfer(model: &Model, edge_id: usize, size: f64) -> Result<Model> {
    edge_operation(model, edge_id, size, None)
}

/// Apply a circular fillet approximation to one convex edge of a single convex
/// planar body. The result is a true manifold B-rep whose radius is tangent to
/// the adjacent support planes; the round is represented by `segments - 1`
/// planar tangent faces (2..32 segments).
pub fn fillet(model: &Model, edge_id: usize, radius: f64, segments: usize) -> Result<Model> {
    edge_operation(model, edge_id, radius, Some(segments))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds(model: &Model) -> ([f64; 3], [f64; 3]) {
        (
            std::array::from_fn(|axis| {
                model
                    .vertices
                    .iter()
                    .map(|v| v.point[axis])
                    .fold(f64::INFINITY, f64::min)
            }),
            std::array::from_fn(|axis| {
                model
                    .vertices
                    .iter()
                    .map(|v| v.point[axis])
                    .fold(f64::NEG_INFINITY, f64::max)
            }),
        )
    }

    #[test]
    fn overlapping_box_booleans_are_closed_breps() {
        let a = cuboid([0.; 3], [2., 2., 2.]).unwrap();
        let b = cuboid([1., 0., 0.], [3., 2., 2.]).unwrap();
        for operation in ["union", "difference", "intersection"] {
            let result = boolean(&a, &b, operation).unwrap();
            assert!(result.validate().unwrap().boundary_edge_count == 0);
            assert_eq!(result.bodies.len(), 1);
        }
        assert_eq!(
            bounds(&boolean(&a, &b, "union").unwrap()),
            ([0.; 3], [3., 2., 2.])
        );
        assert_eq!(
            bounds(&boolean(&a, &b, "difference").unwrap()),
            ([0.; 3], [1., 2., 2.])
        );
        assert_eq!(
            bounds(&boolean(&a, &b, "intersection").unwrap()),
            ([1., 0., 0.], [2., 2., 2.])
        );
    }

    #[test]
    fn concave_orthogonal_difference_is_supported() {
        let a = cuboid([0.; 3], [3., 3., 3.]).unwrap();
        let b = cuboid([1., 1., 2.], [4., 4., 4.]).unwrap();
        let result = boolean(&a, &b, "difference").unwrap();
        assert!(result.faces.len() > 6);
        result.validate().unwrap();
    }

    #[test]
    fn unsupported_boolean_results_fail_closed() {
        let a = cuboid([0.; 3], [1.; 3]).unwrap();
        let separated = cuboid([2., 0., 0.], [3., 1., 1.]).unwrap();
        assert_eq!(
            boolean(&a, &separated, "union").unwrap_err().code,
            UNSUPPORTED
        );
        assert_eq!(
            boolean(&a, &separated, "intersection").unwrap_err().code,
            UNSUPPORTED
        );
        let outer = cuboid([0.; 3], [4.; 3]).unwrap();
        let inner = cuboid([1.; 3], [3.; 3]).unwrap();
        assert_eq!(
            boolean(&outer, &inner, "difference").unwrap_err().code,
            UNSUPPORTED
        );
    }

    #[test]
    fn box_chamfer_and_faceted_fillet_are_manifold() {
        let model = cuboid([0.; 3], [10.; 3]).unwrap();
        let chamfered = chamfer(&model, 0, 1.).unwrap();
        assert_eq!(chamfered.faces.len(), 7);
        chamfered.validate().unwrap();
        let filleted = fillet(&model, 0, 1., 8).unwrap();
        assert_eq!(filleted.faces.len(), 13);
        filleted.validate().unwrap();
    }

    #[test]
    fn edge_operations_reject_invalid_size_and_selection() {
        let model = cuboid([0.; 3], [1.; 3]).unwrap();
        assert_eq!(
            fillet(&model, 0, 0., 8).unwrap_err().code,
            "BREP_INVALID_SIZE"
        );
        assert_eq!(
            fillet(&model, 99, 0.1, 8).unwrap_err().code,
            "BREP_INVALID_SELECTION"
        );
        assert_eq!(
            fillet(&model, 0, 0.1, 64).unwrap_err().code,
            "BREP_RESOURCE_LIMIT"
        );
    }
}
