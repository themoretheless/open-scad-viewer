//! Fail-closed solid operations for the planar subset of the NURBS B-rep.
//!
//! Booleans accept closed, orthogonal, planar solids and return one or more
//! connected bodies, including enclosed orthogonal cavities as inner shells.
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
    if !n.is_finite() || n <= 1e-12 {
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

fn plane_basis(normal: [f64; 3]) -> Result<([f64; 3], [f64; 3])> {
    let reference = if normal[0].abs() < 0.8 {
        [1., 0., 0.]
    } else {
        [0., 1., 0.]
    };
    let u = unit(cross(reference, normal))?;
    Ok((u, cross(normal, u)))
}

fn point_on_segment(point: [f64; 3], a: [f64; 3], b: [f64; 3], tolerance: f64) -> Option<f64> {
    let ab = sub(b, a);
    let length_squared = dot(ab, ab);
    if length_squared <= tolerance * tolerance {
        return None;
    }
    let t = dot(sub(point, a), ab) / length_squared;
    (t > tolerance && t < 1. - tolerance)
        .then(|| norm(sub(point, add(a, mul(ab, t)))) <= tolerance)
        .unwrap_or(false)
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
            if !close(*normalized.last().unwrap(), point, tolerance * 4.) {
                normalized.push(point);
            }
        }
    }
    *polygon = normalized;
}

struct PlanarBoundary {
    outer: Vec<[f64; 3]>,
    holes: Vec<Vec<[f64; 3]>>,
}

fn model_from_polygons(polygons: Vec<Vec<[f64; 3]>>, tolerance: f64) -> Result<Model> {
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

fn model_from_trimmed_polygons(mut polygons: Vec<PlanarBoundary>, tolerance: f64) -> Result<Model> {
    let points: Vec<_> = polygons
        .iter()
        .flat_map(|face| std::iter::once(&face.outer).chain(&face.holes))
        .flatten()
        .copied()
        .collect();
    for face in &mut polygons {
        normalize_ring_edges(&mut face.outer, &points, tolerance);
        for hole in &mut face.holes {
            normalize_ring_edges(hole, &points, tolerance);
        }
    }
    if polygons.len() > 256 {
        return Err(Error::new(
            "BREP_RESOURCE_LIMIT",
            "Result exceeds 256 faces",
        ));
    }
    let mut model = Model(
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
    );
    let mut edge_map = BTreeMap::<(usize, usize), usize>::new();
    for boundary in polygons {
        let polygon = boundary.outer;
        if polygon.len() < 3 || boundary.holes.iter().any(|hole| hole.len() < 3) {
            return Err(failed("Operation produced a degenerate face"));
        }
        let normal = unit((0..polygon.len()).fold([0.; 3], |sum, i| {
            add(
                sum,
                cross(
                    sub(polygon[i], polygon[0]),
                    sub(polygon[(i + 1) % polygon.len()], polygon[0]),
                ),
            )
        }))?;
        let (u, v) = plane_basis(normal)?;
        let origin = polygon[0];
        let all_points: Vec<_> = polygon
            .iter()
            .chain(boundary.holes.iter().flatten())
            .copied()
            .collect();
        let coordinates: Vec<_> = all_points
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
        let mut loop_ids = vec![];
        for ring in std::iter::once(&polygon).chain(&boundary.holes) {
            let mut ids = Vec::with_capacity(ring.len());
            let mut uv = Vec::with_capacity(ring.len());
            for point in ring {
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
                let d = sub(*point, origin);
                uv.push([
                    (dot(d, u) - min_u) / (max_u - min_u),
                    (dot(d, v) - min_v) / (max_v - min_v),
                ]);
            }
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
                        degenerate: false,
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
            loop_ids.push(model.loops.len());
            model.loops.push(Loop { coedges });
        }
        let outer = loop_ids[0];
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
            holes: loop_ids[1..].to_vec(),
        });
    }
    // Each edge-connected boundary component is an independent solid body.
    // Keeping all disconnected faces in one shell would violate the topology
    // contract and made useful results such as a split difference fail closed.
    let mut edge_faces = vec![Vec::<usize>::new(); model.edges.len()];
    for (face_id, face) in model.faces.iter().enumerate() {
        for &loop_id in std::iter::once(&face.outer).chain(&face.holes) {
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
    let mut unseen: BTreeSet<_> = (0..model.faces.len()).collect();
    let mut components = vec![];
    while let Some(seed) = unseen.pop_first() {
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
        let signed_volume = component
            .iter()
            .flat_map(|&face| {
                std::iter::once(&model.faces[face].outer).chain(&model.faces[face].holes)
            })
            .map(|&loop_id| loop_vertices(&model, loop_id))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .map(|vertices| {
                (1..vertices.len() - 1)
                    .map(|i| {
                        let a = model.vertices[vertices[0]].point;
                        let b = model.vertices[vertices[i]].point;
                        let c = model.vertices[vertices[i + 1]].point;
                        dot(a, cross(b, c)) / 6.
                    })
                    .sum::<f64>()
            })
            .sum::<f64>();
        if signed_volume.abs() <= tolerance.powi(3) {
            return Err(unsupported(
                "Boolean result contains a zero-volume boundary",
            ));
        }
        let shell = model.shells.len();
        model.shells.push(Shell {
            faces: component
                .iter()
                .copied()
                .map(|face| FaceUse {
                    face,
                    reversed: false,
                })
                .collect(),
            closed: true,
        });
        components.push((shell, component, signed_volume));
    }
    for &(shell, _, volume) in &components {
        if volume > 0. {
            model.bodies.push(Body {
                outer_shell: shell,
                inner_shells: vec![],
            });
        }
    }
    for (shell, faces, volume) in &components {
        if *volume > 0. {
            continue;
        }
        let sample = model.vertices[loop_vertices(&model, model.faces[faces[0]].outer)?[0]].point;
        let mut containers: Vec<_> = components
            .iter()
            .filter(|(_, _, candidate_volume)| *candidate_volume > 0.)
            .filter_map(|(candidate_shell, candidate_faces, candidate_volume)| {
                contains_faces(&model, candidate_faces, sample)
                    .ok()
                    .filter(|inside| *inside)
                    .map(|_| (*candidate_shell, *candidate_volume))
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
            .push(*shell);
    }
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

pub fn extrude_polygon(profile: &[[f64; 2]], z_min: f64, z_max: f64) -> Result<Model> {
    extrude_polygon_with_holes(profile, &[], z_min, z_max)
}

fn ring_area(ring: &[[f64; 2]]) -> f64 {
    (0..ring.len())
        .map(|i| {
            let a = ring[i];
            let b = ring[(i + 1) % ring.len()];
            a[0] * b[1] - b[0] * a[1]
        })
        .sum::<f64>()
        / 2.
}

fn planar_segments_intersect(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> bool {
    let orient = |p: [f64; 2], q: [f64; 2], r: [f64; 2]| {
        (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0])
    };
    let boxes_overlap = a[0].min(b[0]) <= c[0].max(d[0]) + 1e-7
        && c[0].min(d[0]) <= a[0].max(b[0]) + 1e-7
        && a[1].min(b[1]) <= c[1].max(d[1]) + 1e-7
        && c[1].min(d[1]) <= a[1].max(b[1]) + 1e-7;
    boxes_overlap
        && orient(a, b, c) * orient(a, b, d) <= 1e-14
        && orient(c, d, a) * orient(c, d, b) <= 1e-14
}

fn point_in_ring(point: [f64; 2], ring: &[[f64; 2]]) -> bool {
    let mut inside = false;
    for i in 0..ring.len() {
        let a = ring[i];
        let b = ring[(i + 1) % ring.len()];
        if (a[1] > point[1]) != (b[1] > point[1])
            && point[0] < (b[0] - a[0]) * (point[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            inside = !inside;
        }
    }
    inside
}

fn rings_intersect(a: &[[f64; 2]], b: &[[f64; 2]]) -> bool {
    (0..a.len()).any(|i| {
        (0..b.len()).any(|j| {
            planar_segments_intersect(a[i], a[(i + 1) % a.len()], b[j], b[(j + 1) % b.len()])
        })
    })
}

pub(super) fn validate_extrusion_ring(ring: &[[f64; 2]], ccw: bool, name: &str) -> Result<()> {
    if ring.len() < 3 || ring.len() > 128 {
        return Err(unsupported(format!("{name} must have 3..128 vertices")));
    }
    if ring
        .iter()
        .flatten()
        .any(|value| !value.is_finite() || value.abs() > 1e6)
    {
        return Err(Error::new(
            "BREP_INVALID_SIZE",
            "Extrusion coordinates must be finite and within 1000000 mm",
        ));
    }
    let area = ring_area(ring);
    if area.abs() <= 1e-12 || area.is_sign_positive() != ccw {
        return Err(unsupported(format!(
            "{name} must be {}",
            if ccw {
                "counter-clockwise"
            } else {
                "clockwise"
            }
        )));
    }
    for i in 0..ring.len() {
        let a = ring[i];
        let b = ring[(i + 1) % ring.len()];
        if (a[0] - b[0]).hypot(a[1] - b[1]) <= 1e-7 {
            return Err(unsupported(format!(
                "{name} has duplicate consecutive vertices"
            )));
        }
        for j in i + 1..ring.len() {
            if j == i || j == (i + 1) % ring.len() || i == (j + 1) % ring.len() {
                continue;
            }
            let c = ring[j];
            let d = ring[(j + 1) % ring.len()];
            if planar_segments_intersect(a, b, c, d) {
                return Err(unsupported(format!("{name} must be simple")));
            }
        }
    }
    Ok(())
}

/// Extrude a simple, possibly concave planar profile with optional clockwise
/// holes into a manifold B-rep with two genuinely trimmed cap faces.
pub fn extrude_polygon_with_holes(
    profile: &[[f64; 2]],
    holes: &[Vec<[f64; 2]>],
    z_min: f64,
    z_max: f64,
) -> Result<Model> {
    validate_extrusion_ring(profile, true, "Extrusion outer profile")?;
    if holes.len() > 16 || profile.len() + holes.iter().map(Vec::len).sum::<usize>() > 512 {
        return Err(Error::new(
            "BREP_RESOURCE_LIMIT",
            "Extrusion supports at most 16 holes and 512 boundary vertices",
        ));
    }
    for (index, hole) in holes.iter().enumerate() {
        validate_extrusion_ring(hole, false, "Extrusion hole")?;
        if !point_in_ring(hole[0], profile)
            || rings_intersect(profile, hole)
            || holes[..index].iter().any(|other| {
                rings_intersect(other, hole)
                    || point_in_ring(hole[0], other)
                    || point_in_ring(other[0], hole)
            })
        {
            return Err(unsupported(
                "Extrusion holes must be disjoint and strictly inside the outer profile",
            ));
        }
    }
    if !z_min.is_finite() || !z_max.is_finite() || z_max <= z_min {
        return Err(Error::new(
            "BREP_INVALID_SIZE",
            "Extrusion requires finite zMax greater than zMin",
        ));
    }
    if z_min.abs() > 1e6 || z_max.abs() > 1e6 {
        return Err(Error::new(
            "BREP_INVALID_SIZE",
            "Extrusion coordinates must be finite and within 1000000 mm",
        ));
    }
    let tolerance = 1e-7;
    let lower: Vec<_> = profile.iter().rev().map(|p| [p[0], p[1], z_min]).collect();
    let upper: Vec<_> = profile.iter().map(|p| [p[0], p[1], z_max]).collect();
    let lower_holes: Vec<_> = holes
        .iter()
        .map(|hole| hole.iter().rev().map(|p| [p[0], p[1], z_min]).collect())
        .collect();
    let upper_holes: Vec<_> = holes
        .iter()
        .map(|hole| hole.iter().map(|p| [p[0], p[1], z_max]).collect())
        .collect();
    let mut polygons = vec![
        PlanarBoundary {
            outer: lower,
            holes: lower_holes,
        },
        PlanarBoundary {
            outer: upper,
            holes: upper_holes,
        },
    ];
    for ring in std::iter::once(profile).chain(holes.iter().map(Vec::as_slice)) {
        for i in 0..ring.len() {
            let a = ring[i];
            let b = ring[(i + 1) % ring.len()];
            polygons.push(PlanarBoundary {
                outer: vec![
                    [a[0], a[1], z_min],
                    [b[0], b[1], z_min],
                    [b[0], b[1], z_max],
                    [a[0], a[1], z_max],
                ],
                holes: vec![],
            });
        }
    }
    model_from_trimmed_polygons(polygons, tolerance)
}

fn validate_convex_profile(profile: &[[f64; 2]], name: &str) -> Result<()> {
    if profile.len() < 3 || profile.len() > 128 {
        return Err(unsupported(format!(
            "{name} profile must have 3..128 vertices"
        )));
    }
    if profile
        .iter()
        .flatten()
        .any(|value| !value.is_finite() || value.abs() > 1e6)
    {
        return Err(Error::new(
            "BREP_INVALID_SIZE",
            format!("{name} coordinates must be finite and within 1000000 mm"),
        ));
    }
    validate_convex_boundary(profile, name)
}

// Use after dimensional and world-coordinate admission. Rigid local placement
// can exceed the world-coordinate bound without making the input inadmissible.
fn validate_convex_boundary(profile: &[[f64; 2]], name: &str) -> Result<()> {
    let tolerance = 1e-7;
    // Every nonincident vertex must lie strictly inside every oriented support.
    // Adjacent turn signs alone admit multiply wound stars and repeated loops.
    for i in 0..profile.len() {
        let next = (i + 1) % profile.len();
        let a = profile[i];
        let b = profile[next];
        for (j, p) in profile.iter().enumerate() {
            if j == i || j == next {
                continue;
            }
            let side = (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]);
            if !side.is_finite() || side <= tolerance {
                return Err(unsupported(format!(
                    "{name} currently requires a simple strictly convex CCW profile"
                )));
            }
        }
    }
    Ok(())
}

fn triangulated_strip(sections: &[Vec<[f64; 3]>]) -> Vec<Vec<[f64; 3]>> {
    let count = sections[0].len();
    let mut polygons = vec![
        sections[0].iter().rev().copied().collect(),
        sections.last().unwrap().clone(),
    ];
    for pair in sections.windows(2) {
        for i in 0..count {
            let next = (i + 1) % count;
            polygons.push(vec![pair[0][i], pair[0][next], pair[1][next]]);
            polygons.push(vec![pair[0][i], pair[1][next], pair[1][i]]);
        }
    }
    polygons
}

/// Loft strictly convex, consistently oriented parallel sections into a faceted B-rep.
///
/// Side quads are triangulated because arbitrary corresponding section edges
/// need not be coplanar. This is planar construction, not a smooth NURBS loft.
pub fn faceted_loft(sections: &[Vec<[f64; 3]>]) -> Result<Model> {
    admit_loft_sections(sections)?;
    model_from_polygons(triangulated_strip(sections), 1e-7)
}

pub(super) fn admit_loft_sections(sections: &[Vec<[f64; 3]>]) -> Result<()> {
    if sections.len() < 2 || sections.len() > 64 {
        return Err(unsupported("Loft requires 2..64 sections"));
    }
    let count = sections[0].len();
    if count < 3 || count > 128 || sections.iter().any(|section| section.len() != count) {
        return Err(unsupported(
            "Loft sections must have the same 3..128 vertex count",
        ));
    }
    if sections
        .iter()
        .flatten()
        .flatten()
        .any(|value| !value.is_finite() || value.abs() > 1e6)
    {
        return Err(Error::new(
            "BREP_INVALID_SIZE",
            "Loft coordinates must be finite and within 1000000 mm",
        ));
    }
    let origin = sections[0][0];
    let u = unit(sub(sections[0][1], origin))?;
    let normal = unit(cross(u, sub(sections[0][2], origin)))?;
    let v = cross(normal, u);
    let mut previous_height = f64::NEG_INFINITY;
    for section in sections {
        let height = dot(sub(section[0], origin), normal);
        if section
            .iter()
            .any(|point| (dot(sub(*point, origin), normal) - height).abs() > 1e-7)
            || height <= previous_height + 1e-7
        {
            return Err(unsupported(
                "Loft sections must be parallel planar profiles ordered along their oriented normal",
            ));
        }
        let profile: Vec<_> = section
            .iter()
            .map(|point| {
                let p = sub(*point, origin);
                [dot(p, u), dot(p, v)]
            })
            .collect();
        validate_convex_boundary(&profile, "Loft")?;
        previous_height = height;
    }
    Ok(())
}

/// Sweep a strictly convex CCW profile along a polyline with transported frames.
///
/// Every side patch is triangulated and planar. The path is sampled exactly as
/// authored; no analytic pipe or smooth transition is claimed.
pub fn faceted_sweep(profile: &[[f64; 2]], path: &[[f64; 3]], up: [f64; 3]) -> Result<Model> {
    validate_convex_profile(profile, "Sweep")?;
    if path.len() < 2 || path.len() > 64 {
        return Err(unsupported("Sweep path must have 2..64 points"));
    }
    if path
        .iter()
        .flatten()
        .chain(up.iter())
        .any(|value| !value.is_finite() || value.abs() > 1e6)
    {
        return Err(Error::new(
            "BREP_INVALID_SIZE",
            "Sweep coordinates must be finite and within 1000000 mm",
        ));
    }
    let tolerance = 1e-7;
    let segments: Vec<_> = path
        .windows(2)
        .map(|pair| unit(sub(pair[1], pair[0])))
        .collect::<Result<_>>()?;
    let mut sections = Vec::with_capacity(path.len());
    let mut previous_u: Option<[f64; 3]> = None;
    for i in 0..path.len() {
        let tangent = if i == 0 {
            segments[0]
        } else if i + 1 == path.len() {
            segments[i - 1]
        } else {
            unit(add(segments[i - 1], segments[i]))
                .map_err(|_| unsupported("Sweep path contains a 180 degree reversal"))?
        };
        let projected_up = sub(up, mul(tangent, dot(up, tangent)));
        let mut v = unit(projected_up)
            .map_err(|_| unsupported("Sweep up vector must not be parallel to the path"))?;
        let mut u = unit(cross(v, tangent))?;
        if let Some(previous) = previous_u
            && dot(previous, u) < 0.
        {
            u = mul(u, -1.);
            v = mul(v, -1.);
        }
        previous_u = Some(u);
        sections.push(
            profile
                .iter()
                .map(|point| add(path[i], add(mul(u, point[0]), mul(v, point[1]))))
                .collect(),
        );
    }
    model_from_polygons(triangulated_strip(&sections), tolerance)
}

/// Revolve a closed `(radius, z)` profile around Z as a faceted planar B-rep.
///
/// A full turn is required. Radius-zero profile vertices are supported as
/// poles; negative radii and analytic cylindrical/spherical claims are not.
pub fn faceted_revolve(profile: &[[f64; 2]], segments: usize) -> Result<Model> {
    validate_convex_profile(profile, "Revolve")?;
    if !(3..=128).contains(&segments) {
        return Err(Error::new(
            "BREP_RESOURCE_LIMIT",
            "Faceted revolve segments must be 3..128",
        ));
    }
    if profile.iter().any(|point| point[0] < 0.) || profile.iter().all(|point| point[0] <= 1e-7) {
        return Err(unsupported(
            "Revolve profile radii must be nonnegative with positive extent",
        ));
    }
    let rings: Vec<Vec<_>> = (0..segments)
        .map(|segment| {
            let angle = std::f64::consts::TAU * segment as f64 / segments as f64;
            profile
                .iter()
                .map(|point| [point[0] * angle.cos(), point[0] * angle.sin(), point[1]])
                .collect()
        })
        .collect();
    let mut polygons = vec![];
    for segment in 0..segments {
        let next_segment = (segment + 1) % segments;
        for i in 0..profile.len() {
            let next = (i + 1) % profile.len();
            let a = rings[segment][i];
            let b = rings[next_segment][i];
            let c = rings[next_segment][next];
            let d = rings[segment][next];
            if !close(a, b, 1e-7) && !close(b, c, 1e-7) {
                polygons.push(vec![a, b, c]);
            }
            if !close(a, c, 1e-7) && !close(c, d, 1e-7) {
                polygons.push(vec![a, c, d]);
            }
        }
    }
    model_from_polygons(polygons, 1e-7)
}

/// Construct a declared faceted cylindrical B-rep with planar side faces.
pub fn faceted_cylinder(radius: f64, height: f64, segments: usize) -> Result<Model> {
    if !radius.is_finite() || !height.is_finite() || radius < 0.01 || height < 0.01 {
        return Err(Error::new(
            "BREP_INVALID_SIZE",
            "Faceted cylinder radius and height must be at least 0.01 mm",
        ));
    }
    if !(3..=128).contains(&segments) {
        return Err(Error::new(
            "BREP_RESOURCE_LIMIT",
            "Faceted cylinder segments must be 3..128",
        ));
    }
    let profile: Vec<_> = (0..segments)
        .map(|i| {
            let angle = std::f64::consts::TAU * i as f64 / segments as f64;
            [radius * angle.cos(), radius * angle.sin()]
        })
        .collect();
    extrude_polygon(&profile, 0., height)
}

/// Construct an honest faceted sphere: every patch is a planar B-rep face.
pub fn faceted_sphere(
    radius: f64,
    radial_segments: usize,
    latitude_segments: usize,
) -> Result<Model> {
    if !radius.is_finite() || radius < 0.01 {
        return Err(Error::new(
            "BREP_INVALID_SIZE",
            "Faceted sphere radius must be at least 0.01 mm",
        ));
    }
    if !(3..=32).contains(&radial_segments) || !(2..=16).contains(&latitude_segments) {
        return Err(Error::new(
            "BREP_RESOURCE_LIMIT",
            "Faceted sphere segments must be radial 3..32 and latitude 2..16",
        ));
    }
    let ring = |latitude: usize, radial: usize| {
        let phi = std::f64::consts::PI * latitude as f64 / latitude_segments as f64;
        let theta = std::f64::consts::TAU * radial as f64 / radial_segments as f64;
        [
            radius * phi.sin() * theta.cos(),
            radius * phi.sin() * theta.sin(),
            radius * phi.cos(),
        ]
    };
    let mut polygons = vec![];
    let top = [0., 0., radius];
    let bottom = [0., 0., -radius];
    for radial in 0..radial_segments {
        let next = (radial + 1) % radial_segments;
        polygons.push(vec![top, ring(1, radial), ring(1, next)]);
        for latitude in 1..latitude_segments - 1 {
            let a = ring(latitude, radial);
            let b = ring(latitude + 1, radial);
            let c = ring(latitude + 1, next);
            let d = ring(latitude, next);
            polygons.push(vec![a, b, c]);
            polygons.push(vec![a, c, d]);
        }
        polygons.push(vec![
            bottom,
            ring(latitude_segments - 1, next),
            ring(latitude_segments - 1, radial),
        ]);
    }
    model_from_polygons(polygons, 1e-7)
}

fn model_from_planes(planes: &[Plane], tolerance: f64) -> Result<Model> {
    let mut unique = Vec::<Plane>::new();
    for &plane in planes {
        if let Some(existing) = unique
            .iter_mut()
            .find(|candidate| dot(candidate.normal, plane.normal) > 1. - 1e-9)
        {
            // Equal-direction half-spaces collapse to the tighter support.
            existing.offset = existing.offset.min(plane.offset);
        } else {
            unique.push(plane);
        }
    }
    let planes = unique.as_slice();
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

fn split_polygon(
    polygon: &[[f64; 3]],
    plane: Plane,
    tolerance: f64,
) -> (Vec<[f64; 3]>, Vec<[f64; 3]>) {
    let mut inside = vec![];
    let mut outside = vec![];
    for i in 0..polygon.len() {
        let a = polygon[i];
        let b = polygon[(i + 1) % polygon.len()];
        let snap = |d: f64| if d.abs() <= tolerance { 0. } else { d };
        let da = snap(dot(plane.normal, a) - plane.offset);
        let db = snap(dot(plane.normal, b) - plane.offset);
        let a_inside = da <= 0.;
        let b_inside = db <= 0.;
        (if a_inside { &mut inside } else { &mut outside }).push(a);
        if a_inside != b_inside {
            let point = add(a, mul(sub(b, a), da / (da - db)));
            inside.push(point);
            outside.push(point);
        }
    }
    let clean = |mut polygon: Vec<[f64; 3]>| {
        polygon.dedup_by(|a, b| close(*a, *b, tolerance));
        if polygon.len() > 1 && close(polygon[0], *polygon.last().unwrap(), tolerance) {
            polygon.pop();
        }
        if polygon.len() < 3 {
            return vec![];
        }
        let area = (0..polygon.len()).fold([0.; 3], |sum, i| {
            add(
                sum,
                cross(
                    sub(polygon[i], polygon[0]),
                    sub(polygon[(i + 1) % polygon.len()], polygon[0]),
                ),
            )
        });
        if norm(area) <= tolerance * tolerance {
            vec![]
        } else {
            polygon
        }
    };
    (clean(inside), clean(outside))
}

fn partition_polygon(
    polygon: Vec<[f64; 3]>,
    planes: &[Plane],
    tolerance: f64,
) -> (Vec<Vec<[f64; 3]>>, Vec<[f64; 3]>) {
    let mut active = polygon;
    let mut outside = vec![];
    for &plane in planes {
        if active.is_empty() {
            break;
        }
        let (inside, fragment) = split_polygon(&active, plane, tolerance * 8.);
        if !fragment.is_empty() {
            outside.push(fragment);
        }
        active = inside;
    }
    (outside, active)
}

fn convex_boundary(model: &Model) -> Result<(Vec<Plane>, Vec<Vec<[f64; 3]>>)> {
    let planes = convex_planes(model)?;
    let shell = &model.shells[model.bodies[0].outer_shell];
    let polygons = shell
        .faces
        .iter()
        .map(|face_use| {
            let mut ids = loop_vertices(model, model.faces[face_use.face].outer)?;
            if face_use.reversed {
                ids.reverse();
            }
            Ok(ids
                .into_iter()
                .map(|vertex| model.vertices[vertex].point)
                .collect())
        })
        .collect::<Result<Vec<_>>>()?;
    Ok((planes, polygons))
}

fn convex_boolean(a: &Model, b: &Model, operation: &str) -> Result<Model> {
    let tolerance = a.tolerance_mm.max(b.tolerance_mm);
    let (a_planes, a_faces) = convex_boundary(a)?;
    let (b_planes, b_faces) = convex_boundary(b)?;
    if operation == "intersection" {
        let mut planes = a_planes;
        planes.extend(b_planes);
        let mut result = match model_from_planes(&planes, tolerance) {
            Ok(model) => model,
            // The half-space constructor requires positive volume. The
            // boundary arrangement can represent regularized empty contacts.
            Err(_) => return planar_boolean(a, b, operation),
        };
        result.inherit_topology_ids(&[a, b]);
        return Ok(result);
    }
    let shared_support = |plane: Plane, others: &[Plane]| {
        others.iter().any(|other| {
            dot(plane.normal, other.normal) > 1. - 1e-9
                && (plane.offset - other.offset).abs() <= tolerance * 8.
        })
    };
    let mut polygons = vec![];
    for (face, &plane) in a_faces.into_iter().zip(&a_planes) {
        let (outside, inside) = partition_polygon(face, &b_planes, tolerance);
        polygons.extend(outside);
        // On a shared outward support, one operand must author the overlap.
        if operation == "union" && shared_support(plane, &b_planes) && !inside.is_empty() {
            polygons.push(inside);
        }
    }
    for (mut face, &plane) in b_faces.into_iter().zip(&b_planes) {
        if operation == "union" {
            let (outside, _) = partition_polygon(face, &a_planes, tolerance);
            polygons.extend(outside);
        } else {
            let (_, inside) = partition_polygon(face, &a_planes, tolerance);
            if !inside.is_empty() && !shared_support(plane, &a_planes) {
                face = inside;
                face.reverse();
                polygons.push(face);
            }
        }
    }
    if polygons.is_empty() {
        return Model::empty(a.tolerance_mm.max(b.tolerance_mm));
    }
    let mut result = model_from_polygons(polygons, tolerance)?;
    result.inherit_topology_ids(&[a, b]);
    result.validate()?;
    Ok(result)
}

fn planar_boundary(model: &Model) -> Result<Vec<(Plane, Vec<[f64; 3]>)>> {
    model.validate()?;
    if model.bodies.is_empty() {
        return Err(unsupported("Boolean operands require closed bodies"));
    }
    let mut boundary = vec![];
    let mut seen_shells = BTreeSet::new();
    for body in &model.bodies {
        for shell_id in std::iter::once(&body.outer_shell).chain(&body.inner_shells) {
            if !seen_shells.insert(*shell_id) || !model.shells[*shell_id].closed {
                return Err(unsupported(
                    "Boolean operands require distinct closed shells",
                ));
            }
            for face_use in &model.shells[*shell_id].faces {
                let face = &model.faces[face_use.face];
                let mut plane = face_plane(model, face_use.face)?;
                if face_use.reversed {
                    plane.normal = mul(plane.normal, -1.);
                    plane.offset = -plane.offset;
                }
                let (u, v) = plane_basis(plane.normal)?;
                let origin = model.vertices[loop_vertices(model, face.outer)?[0]].point;
                let project = |wire| -> Result<Vec<[f64; 2]>> {
                    Ok(loop_vertices(model, wire)?
                        .iter()
                        .map(|&vertex| {
                            let delta = sub(model.vertices[vertex].point, origin);
                            [dot(delta, u), dot(delta, v)]
                        })
                        .collect())
                };
                let mut outer = project(face.outer)?;
                if ring_area(&outer) < 0. {
                    outer.reverse();
                }
                let convex = (0..outer.len()).all(|i| {
                    let a = outer[i];
                    let b = outer[(i + 1) % outer.len()];
                    let c = outer[(i + 2) % outer.len()];
                    (b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0])
                        >= -model.tolerance_mm.powi(2)
                });
                let lift = |p: [f64; 2]| add(origin, add(mul(u, p[0]), mul(v, p[1])));
                if face.holes.is_empty() && convex {
                    boundary.push((plane, outer.into_iter().map(lift).collect()));
                } else {
                    let holes = face
                        .holes
                        .iter()
                        .map(|&wire| project(wire))
                        .collect::<Result<Vec<_>>>()?;
                    let fill = planar_geometry::triangulation::triangulate_profile(&outer, &holes)?;
                    for triangle in fill.indices.chunks_exact(3) {
                        boundary.push((
                            plane,
                            triangle
                                .iter()
                                .map(|&i| lift(fill.positions[i as usize]))
                                .collect(),
                        ));
                    }
                }
            }
        }
    }
    Ok(boundary)
}

fn fragment_polygon(
    polygon: Vec<[f64; 3]>,
    splitters: &[Plane],
    tolerance: f64,
) -> Result<Vec<Vec<[f64; 3]>>> {
    let mut fragments = vec![polygon];
    for &splitter in splitters {
        let mut next = vec![];
        for fragment in fragments {
            let (negative, positive) = split_polygon(&fragment, splitter, tolerance * 8.);
            if !negative.is_empty() {
                next.push(negative);
            }
            if !positive.is_empty() {
                next.push(positive);
            }
        }
        fragments = next;
        if fragments.len() > 4096 {
            return Err(Error::new(
                "BREP_RESOURCE_LIMIT",
                "Planar Boolean arrangement exceeds 4096 fragments",
            ));
        }
    }
    Ok(fragments)
}

fn boolean_state(operation: &str, inside_a: bool, inside_b: bool) -> bool {
    match operation {
        "union" => inside_a || inside_b,
        "difference" => inside_a && !inside_b,
        "xor" => inside_a != inside_b,
        _ => inside_a && inside_b,
    }
}

/// Bounded boundary arrangement for closed planar solids. Every convex input
/// face is split by the other operand's support planes and by in-plane edge
/// lines for coplanar overlaps. Two-sided point classification authors only
/// fragments across which the requested Boolean state changes.
fn planar_boolean(a: &Model, b: &Model, operation: &str) -> Result<Model> {
    let tolerance = a.tolerance_mm.max(b.tolerance_mm);
    let a_boundary = planar_boundary(a)?;
    let b_boundary = planar_boundary(b)?;
    let mut polygons = vec![];
    for source in [a_boundary.as_slice(), b_boundary.as_slice()] {
        for &(source_plane, ref polygon) in source {
            // Both operands must see the same arrangement. Using only the other
            // operand's planes leaves a large face coincident with several smaller
            // faces and produces multiply-owned edges after sewing.
            let all = || a_boundary.iter().chain(&b_boundary);
            let mut splitters: Vec<_> = all().map(|(plane, _)| *plane).collect();
            for &(other_plane, ref other_polygon) in all() {
                let alignment = dot(source_plane.normal, other_plane.normal);
                let plane_distance = if alignment >= 0. {
                    (source_plane.offset - other_plane.offset).abs()
                } else {
                    (source_plane.offset + other_plane.offset).abs()
                };
                if alignment.abs() > 1. - 1e-9 && plane_distance <= tolerance * 8. {
                    for i in 0..other_polygon.len() {
                        let edge = sub(
                            other_polygon[(i + 1) % other_polygon.len()],
                            other_polygon[i],
                        );
                        let normal = unit(cross(source_plane.normal, edge))?;
                        splitters.push(Plane {
                            normal,
                            offset: dot(normal, other_polygon[i]),
                        });
                    }
                }
            }
            for mut fragment in fragment_polygon(polygon.clone(), &splitters, tolerance)? {
                let center = mul(
                    fragment.iter().copied().fold([0.; 3], add),
                    1. / fragment.len() as f64,
                );
                let epsilon = tolerance * 64.;
                let minus = sub(center, mul(source_plane.normal, epsilon));
                let plus = add(center, mul(source_plane.normal, epsilon));
                let states = [
                    boolean_state(operation, contains(a, minus)?, contains(b, minus)?),
                    boolean_state(operation, contains(a, plus)?, contains(b, plus)?),
                ];
                if states[0] == states[1] {
                    continue;
                }
                if !states[0] && states[1] {
                    fragment.reverse();
                }
                let on_boundary = |point: [f64; 3], ring: &[[f64; 3]]| {
                    (0..ring.len()).any(|i| {
                        close(point, ring[i], tolerance * 4.)
                            || point_on_segment(
                                point,
                                ring[i],
                                ring[(i + 1) % ring.len()],
                                tolerance * 4.,
                            )
                            .is_some()
                    })
                };
                // Equal fragments can have different collinear subdivisions.
                // Comparing sorted vertex lists would retain both coincident faces.
                let duplicate = polygons.iter().any(|existing: &Vec<[f64; 3]>| {
                    existing.iter().all(|&point| on_boundary(point, &fragment))
                        && fragment.iter().all(|&point| on_boundary(point, existing))
                });
                if !duplicate {
                    polygons.push(fragment);
                }
            }
        }
    }
    if polygons.is_empty() {
        return Model::empty(a.tolerance_mm.max(b.tolerance_mm));
    }
    let mut result = model_from_polygons(polygons, tolerance)?;
    result.inherit_topology_ids(&[a, b]);
    result.validate()?;
    Ok(result)
}

fn orthogonal(model: &Model) -> Result<()> {
    model.validate()?;
    if model.bodies.is_empty()
        || model.bodies.iter().any(|body| {
            std::iter::once(&body.outer_shell)
                .chain(&body.inner_shells)
                .any(|shell| !model.shells[*shell].closed)
        })
    {
        return Err(unsupported("Boolean operands require closed bodies"));
    }
    for body in &model.bodies {
        for shell in std::iter::once(&body.outer_shell).chain(&body.inner_shells) {
            for face_use in &model.shells[*shell].faces {
                let plane = face_plane(model, face_use.face)?;
                let axis = plane.normal.iter().filter(|v| v.abs() > 1. - 1e-8).count();
                if axis != 1 {
                    return Err(unsupported(
                        "Boolean operands must have axis-aligned planar faces",
                    ));
                }
            }
        }
    }
    Ok(())
}

fn contains_faces(model: &Model, faces: &[usize], point: [f64; 3]) -> Result<bool> {
    // Intersect the supporting plane once, then test the actual trimmed region.
    // A fan from vertex zero incorrectly fills concave notches and counts holes twice.
    let direction = unit([1., 0.371_390_7, 0.217_113_9])?;
    let mut hits = Vec::<f64>::new();
    for &face in faces {
        let plane = face_plane(model, face)?;
        let denominator = dot(plane.normal, direction);
        if denominator.abs() < 1e-12 {
            continue;
        }
        let t = (plane.offset - dot(plane.normal, point)) / denominator;
        if t <= model.tolerance_mm {
            continue;
        }
        let hit = add(point, mul(direction, t));
        let (u, v) = plane_basis(plane.normal)?;
        let projected = [dot(hit, u), dot(hit, v)];
        let in_wire = |wire| -> Result<bool> {
            let polygon = loop_vertices(model, wire)?
                .iter()
                .map(|&id| {
                    let p = model.vertices[id].point;
                    [dot(p, u), dot(p, v)]
                })
                .collect::<Vec<_>>();
            Ok(point_in_ring(projected, &polygon))
        };
        if in_wire(model.faces[face].outer)?
            && !model.faces[face]
                .holes
                .iter()
                .map(|&wire| in_wire(wire))
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .any(|inside| inside)
            && !hits
                .iter()
                .any(|existing| (*existing - t).abs() <= model.tolerance_mm * 4.)
        {
            hits.push(t);
        }
    }
    Ok(hits.len() % 2 == 1)
}

fn contains(model: &Model, point: [f64; 3]) -> Result<bool> {
    let mut inside = false;
    for body in &model.bodies {
        let outer: Vec<_> = model.shells[body.outer_shell]
            .faces
            .iter()
            .map(|face| face.face)
            .collect();
        if !contains_faces(model, &outer, point)? {
            continue;
        }
        let in_cavity = body.inner_shells.iter().try_fold(false, |inside, shell| {
            let faces: Vec<_> = model.shells[*shell]
                .faces
                .iter()
                .map(|face| face.face)
                .collect();
            Ok::<_, Error>(inside || contains_faces(model, &faces, point)?)
        })?;
        inside |= !in_cavity;
    }
    Ok(inside)
}

/// Bounded regularized CSG over retained boundary geometry. Empty results have
/// no placeholder topology. Unsupported curved intersections remain explicit.
pub fn boolean(a: &Model, b: &Model, operation: &str) -> Result<Model> {
    a.validate()?;
    b.validate()?;
    if !matches!(operation, "union" | "difference" | "intersection" | "xor") {
        return Err(Error::new(
            "BREP_INVALID_OPERATION",
            "Boolean operation must be union, difference, intersection, or xor",
        ));
    }
    if let Some(result) = crate::boolean_support::simplify(a, b, operation)? {
        return Ok(result);
    }
    // Canonical sphere/sphere pairs: exact regularized curved closure.
    if let Some(result) = crate::sphere_boolean::boolean(a, b, operation)? {
        return Ok(result);
    }
    if a.edges
        .iter()
        .chain(&b.edges)
        .any(|edge| edge.curve.degree > 1)
        || a.faces
            .iter()
            .chain(&b.faces)
            .any(|face| face.surface.degree_u > 1 || face.surface.degree_v > 1)
    {
        if let Some(result) = crate::prismatic_boolean::boolean(a, b, operation)? {
            return Ok(result);
        }
    }
    // Convex planar CSG is built directly from clipped boundary polygons,
    // independent of face orientation in world axes.
    if operation != "xor"
        && a.bodies.len() == 1
        && b.bodies.len() == 1
        && a.bodies[0].inner_shells.is_empty()
        && b.bodies[0].inner_shells.is_empty()
        && (orthogonal(a).is_err() || orthogonal(b).is_err())
    {
        if convex_planes(a).is_ok() && convex_planes(b).is_ok() {
            return convex_boolean(a, b, operation).map_err(|e| {
                if e.code == OPERATION_FAILED {
                    unsupported("Boolean result is empty or dimensionally collapsed")
                } else {
                    e
                }
            });
        }
    }
    if orthogonal(a).is_err() || orthogonal(b).is_err() {
        return planar_boolean(a, b, operation);
    }
    orthogonal(a)?;
    orthogonal(b)?;
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
                    "xor" => inside_a != inside_b,
                    _ => inside_a && inside_b,
                };
                if keep {
                    occupied.insert([x, y, z]);
                }
            }
        }
    }
    if occupied.is_empty() {
        return Model::empty(a.tolerance_mm.max(b.tolerance_mm));
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
    let mut result = model_from_polygons(polygons, a.tolerance_mm.max(b.tolerance_mm))?;
    result.inherit_topology_ids(&[a, b]);
    result.validate()?;
    Ok(result)
}

/// Translate one supporting plane of a convex planar body. All adjacent faces
/// are rebuilt from their exact half-spaces; no display mesh participates.
/// Draft a convex planar prism about a neutral plane perpendicular to `axis`.
/// Positive angles expand sections in the positive axis direction. Caps remain
/// fixed; each lateral support tilts by the requested geometric angle.
/// This numerical half-space construction does not accept curved or oblique faces.
pub fn draft_planar_prism(
    model: &Model,
    axis: [f64; 3],
    origin: [f64; 3],
    angle: f64,
) -> Result<Model> {
    if !axis
        .iter()
        .chain(&origin)
        .chain([&angle])
        .all(|x| x.is_finite())
        || angle.abs() > 60.
    {
        return Err(failed(
            "Draft requires finite parameters and an angle within ±60 degrees",
        ));
    }
    let magnitude = axis.iter().map(|x| x.abs()).fold(0., f64::max);
    if magnitude == 0. {
        return Err(failed("Draft requires a nonzero axis"));
    }
    let axis = unit(axis.map(|x| x / magnitude))?;
    let mut planes = convex_planes(model)?;
    if planes.len() > 64 {
        return Err(Error::new(
            "BREP_RESOURCE_LIMIT",
            "Draft supports at most 64 support planes",
        ));
    }
    let tangent = angle.to_radians().tan();
    let neutral = dot(axis, origin);
    let mut sides = 0;
    let mut caps = [false; 2];
    for plane in &mut planes {
        let alignment = dot(plane.normal, axis);
        if alignment.abs() <= 1e-10 {
            sides += 1;
            let normal = sub(plane.normal, mul(axis, tangent));
            let length = norm(normal);
            plane.normal = mul(normal, 1. / length);
            plane.offset = (plane.offset - tangent * neutral) / length;
            if !plane.offset.is_finite() {
                return Err(failed("Draft exceeds finite numeric range"));
            }
        } else if alignment.abs() >= 1. - 1e-10 {
            caps[usize::from(alignment > 0.)] = true;
        } else {
            return Err(unsupported(
                "Draft requires planar prism sides parallel to the axis and perpendicular caps",
            ));
        }
    }
    if sides < 3 || !caps.iter().all(|x| *x) {
        return Err(unsupported("Draft requires a closed planar prism"));
    }
    if angle == 0. {
        return Ok(model.clone());
    }
    let mut out = model_from_planes(&planes, model.tolerance_mm)?;
    // Reject disappearance of a cap or lateral support, including taper collapse.
    for plane in &planes {
        let count = out
            .vertices
            .iter()
            .filter(|v| {
                (dot(plane.normal, v.point) - plane.offset).abs() <= model.tolerance_mm * 8.
            })
            .count();
        if count < 3 {
            return Err(failed("Draft consumes a support face"));
        }
    }
    out.inherit_topology_ids(&[model]);
    out.validate()?;
    Ok(out)
}

pub fn push_planar_face(model: &Model, face_id: usize, distance: f64) -> Result<Model> {
    if !distance.is_finite() || distance.abs() > 1e6 {
        return Err(Error::new(
            "BREP_INVALID_SIZE",
            "Face displacement must be finite and within ±1000000 mm",
        ));
    }
    let mut planes = convex_planes(model)?;
    let faces = &model.shells[model.bodies[0].outer_shell].faces;
    let selected = faces
        .iter()
        .position(|u| u.face == face_id)
        .ok_or_else(|| Error::new("BREP_INVALID_SELECTION", "Unknown face"))?;
    if planes.len() > 64 {
        return Err(Error::new(
            "BREP_RESOURCE_LIMIT",
            "Planar editing supports at most 64 support planes",
        ));
    }
    planes[selected].offset += distance;
    let mut out = model_from_planes(&planes, model.tolerance_mm)?;
    if !out.vertices.iter().any(|v| {
        (dot(planes[selected].normal, v.point) - planes[selected].offset).abs()
            <= model.tolerance_mm * 8.
    }) {
        return Err(failed("Displacement consumes the selected face"));
    }
    out.inherit_topology_ids(&[model]);
    out.validate()?;
    Ok(out)
}
/// Exact planar inward shell of a convex body. Selected opening planes extend
/// the inner cutter outside the original envelope before the native Boolean.
pub fn shell_planar(model: &Model, openings: &[usize], thickness: f64) -> Result<Model> {
    if !thickness.is_finite() || !(1e-5..=1e6).contains(&thickness) {
        return Err(Error::new(
            "BREP_INVALID_SIZE",
            "Shell thickness must be 0.00001..1000000 mm",
        ));
    }
    let mut planes = convex_planes(model)?;
    let mut cavity_planes = planes.clone();
    if planes.len() > 64 {
        return Err(Error::new(
            "BREP_RESOURCE_LIMIT",
            "Planar shell supports at most 64 support planes",
        ));
    }
    let faces = &model.shells[model.bodies[0].outer_shell].faces;
    if openings.iter().any(|f| !faces.iter().any(|u| u.face == *f)) {
        return Err(Error::new(
            "BREP_INVALID_SELECTION",
            "Unknown shell opening face",
        ));
    }
    for (plane, face) in planes.iter_mut().zip(faces) {
        plane.offset += if openings.contains(&face.face) {
            thickness * 2.
        } else {
            -thickness
        };
    }
    // Opening supports extend the cutter, but an actual cavity must still lie
    // inside the original body. Otherwise an excessive thickness can move the
    // complete cutter outside stock and make difference return unchanged stock.
    cavity_planes.extend(planes.iter().copied());
    model_from_planes(&cavity_planes, model.tolerance_mm)?;
    let inner = model_from_planes(&planes, model.tolerance_mm)?;
    let mut out = boolean(model, &inner, "difference")?;
    out.inherit_topology_ids(&[model]);
    out.validate()?;
    Ok(out)
}
/// Two closed pieces from an exact support-plane cut of a convex planar body.
pub fn split_planar(model: &Model, normal: [f64; 3], offset: f64) -> Result<[Model; 2]> {
    if normal.iter().any(|v| !v.is_finite()) || !offset.is_finite() {
        return Err(Error::new(
            "BREP_INVALID_OPERATION",
            "Split plane must be finite",
        ));
    }
    // Scale first so equivalent finite plane equations cannot overflow the
    // norm (or collapse to zero) merely because of their coefficient units.
    let magnitude = normal.iter().map(|x| x.abs()).fold(0., f64::max);
    if magnitude == 0. {
        return Err(Error::new(
            "BREP_INVALID_OPERATION",
            "Split plane normal must be nonzero",
        ));
    }
    let scaled = normal.map(|x| x / magnitude);
    let length = norm(scaled);
    let normal = mul(scaled, 1. / length);
    let offset = (offset / magnitude) / length;
    if !offset.is_finite() {
        return Err(Error::new(
            "BREP_INVALID_OPERATION",
            "Normalized split offset must be finite",
        ));
    }
    let planes = convex_planes(model)?;
    if planes.len() > 64 {
        return Err(Error::new(
            "BREP_RESOURCE_LIMIT",
            "Planar split supports at most 64 support planes",
        ));
    }
    let mut a = planes.clone();
    a.push(Plane { normal, offset });
    let mut b = planes;
    b.push(Plane {
        normal: mul(normal, -1.),
        offset: -offset,
    });
    let mut pair = [
        model_from_planes(&a, model.tolerance_mm)?,
        model_from_planes(&b, model.tolerance_mm)?,
    ];
    for part in &mut pair {
        part.inherit_topology_ids(&[model]);
        part.validate()?;
    }
    Ok(pair)
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
    edge_ids: &[usize],
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
    if edge_ids.is_empty() {
        return Err(Error::new(
            "BREP_INVALID_SELECTION",
            "Select at least one edge",
        ));
    }
    let selected: BTreeSet<_> = edge_ids.iter().copied().collect();
    if selected.len() != edge_ids.len() || selected.iter().any(|edge| *edge >= model.edges.len()) {
        return Err(Error::new(
            "BREP_INVALID_SELECTION",
            "Selected edges must be unique authored edges",
        ));
    }
    let mut connected = BTreeSet::from([edge_ids[0]]);
    loop {
        let vertices: BTreeSet<_> = connected
            .iter()
            .flat_map(|edge| model.edges[*edge].vertices)
            .collect();
        let before = connected.len();
        connected.extend(selected.iter().copied().filter(|edge| {
            model.edges[*edge]
                .vertices
                .iter()
                .any(|vertex| vertices.contains(vertex))
        }));
        if connected.len() == before {
            break;
        }
    }
    if connected.len() != selected.len() {
        return Err(unsupported("Selected edges must form one connected chain"));
    }
    let mut supports = vec![];
    for &edge_id in edge_ids {
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
            return Err(unsupported("Select convex, non-tangent edges"));
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
        supports.extend([a, b]);
    }
    let mut result = model_from_planes(&planes, model.tolerance_mm)?;
    // A consumed adjacent support face means the requested size crossed a
    // neighboring feature even if the half-space intersection stayed nonempty.
    for original in supports {
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
    result.inherit_topology_ids(&[model]);
    for &edge_id in edge_ids {
        let original = &model.edges[edge_id];
        let [a, b] = original.vertices.map(|vertex| model.vertices[vertex].point);
        let direction = unit(sub(b, a))?;
        let children: Vec<_> = result
            .edges
            .iter()
            .enumerate()
            .filter(|(index, edge)| {
                result.1.edges[*index] != model.1.edges[edge_id] && {
                    let [c, d] = edge.vertices.map(|vertex| result.vertices[vertex].point);
                    unit(sub(d, c))
                        .map(|candidate| dot(direction, candidate).abs() > 1. - 1e-8)
                        .unwrap_or(false)
                }
            })
            .map(|(index, _)| result.1.edges[index].clone())
            .collect();
        if !children.is_empty() {
            result.1.lineage.push(TopologyLineageRecord {
                operation: if children.len() > 1 {
                    "split"
                } else {
                    "persist"
                }
                .into(),
                entity_kind: "edge".into(),
                parents: vec![model.1.edges[edge_id].clone()],
                children,
            });
        }
    }
    Ok(result)
}

/// Chamfer one convex edge of a single convex planar body.
pub fn chamfer(model: &Model, edge_id: usize, size: f64) -> Result<Model> {
    chamfer_edges(model, &[edge_id], size)
}

/// Chamfer a connected chain of authored convex edges.
pub fn chamfer_edges(model: &Model, edge_ids: &[usize], size: f64) -> Result<Model> {
    edge_operation(model, edge_ids, size, None)
}

/// Apply a circular fillet approximation to one convex edge of a single convex
/// planar body. The result is a true manifold B-rep whose radius is tangent to
/// the adjacent support planes; the round is represented by `segments - 1`
/// planar tangent faces (2..32 segments).
pub fn fillet(model: &Model, edge_id: usize, radius: f64, segments: usize) -> Result<Model> {
    fillet_edges(model, &[edge_id], radius, segments)
}

/// Apply the same faceted circular fillet to a connected edge chain.
pub fn fillet_edges(
    model: &Model,
    edge_ids: &[usize],
    radius: f64,
    segments: usize,
) -> Result<Model> {
    edge_operation(model, edge_ids, radius, Some(segments))
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

    fn rotate_z(model: &mut Model, angle: f64) {
        let transform = |point: &mut Vec<f64>| {
            let (sin, cos) = angle.sin_cos();
            let [x, y] = [point[0], point[1]];
            point[0] = x * cos - y * sin;
            point[1] = x * sin + y * cos;
        };
        for vertex in &mut model.vertices {
            let mut point = vertex.point.to_vec();
            transform(&mut point);
            vertex.point = point.try_into().unwrap();
        }
        for edge in &mut model.edges {
            edge.curve.control_points.iter_mut().for_each(transform);
        }
        for face in &mut model.faces {
            face.surface
                .control_points
                .iter_mut()
                .flatten()
                .for_each(transform);
        }
    }

    #[test]
    fn prism_draft_zero_preserves_authored_topology() {
        let model = cuboid([0.; 3], [10.; 3]).unwrap();
        let before = format!("{model:?}");
        let out = draft_planar_prism(&model, [0., 0., 1.], [0.; 3], 0.).unwrap();
        assert_eq!(format!("{out:?}"), before);
        assert_eq!(format!("{model:?}"), before);
    }

    #[test]
    fn prism_draft_caps_survive_and_collapse_refuses() {
        let model = cuboid([0.; 3], [10.; 3]).unwrap();
        let before = format!("{model:?}");
        for angle in [-10_f64, 10.] {
            let out = draft_planar_prism(&model, [0., 0., 1e300], [0.; 3], angle).unwrap();
            out.validate().unwrap();
            assert_eq!(out.faces.len(), 6);
            assert_eq!(out.vertices.len(), 8);
            let growth = 10. * angle.to_radians().tan();
            for v in &out.vertices {
                let z = v.point[2];
                assert!(z.abs() < 1e-8 || (z - 10.).abs() < 1e-8);
                let delta = if z.abs() < 1e-8 { 0. } else { growth };
                for k in 0..2 {
                    assert!(
                        (v.point[k] + delta).abs() < 1e-8
                            || (v.point[k] - 10. - delta).abs() < 1e-8
                    );
                }
            }
        }
        assert!(draft_planar_prism(&model, [0., 0., 1.], [0.; 3], -60.).is_err());
        assert!(draft_planar_prism(&model, [1., 0., 1.], [0.; 3], 10.).is_err());
        assert_eq!(format!("{model:?}"), before);
    }

    #[test]
    fn planar_direct_edits_preserve_native_boundaries() {
        let stock = cuboid([0.; 3], [10.; 3]).unwrap();
        let top = stock
            .faces
            .iter()
            .position(|f| {
                f.surface
                    .control_points
                    .iter()
                    .flatten()
                    .all(|p| p[2] == 10.)
            })
            .unwrap();
        let volume = |m: &Model| {
            crate::analysis::mass_properties(m, 1e-7, 200_000)
                .unwrap()
                .signed_volume_mm3
        };
        assert!((volume(&push_planar_face(&stock, top, 2.).unwrap()) - 1200.).abs() < 1e-6);
        assert!(
            (volume(&shell_planar(&stock, &[top], 1.).unwrap()) - (1000. - 8. * 8. * 9.)).abs()
                < 1e-6
        );
        assert!((volume(&shell_planar(&stock, &[], 1.).unwrap()) - 488.).abs() < 1e-6);
        let pair = split_planar(&stock, [1., 0., 0.], 4.).unwrap();
        assert!((volume(&pair[0]) - 400.).abs() < 1e-6);
        assert!((volume(&pair[1]) - 600.).abs() < 1e-6);
        for factor in [1e-300, 1e300] {
            let scaled = split_planar(&stock, [factor, 0., 0.], 4. * factor).unwrap();
            assert!((volume(&scaled[0]) - 400.).abs() < 1e-6);
        }
        assert!(shell_planar(&stock, &[top], 6.).is_err());
        assert!(split_planar(&stock, [1., 0., 0.], 11.).is_err());
        assert!(push_planar_face(&stock, usize::MAX, 2.).is_err());
        assert!(shell_planar(&crate::cylinder(3., 5.).unwrap(), &[], 1.).is_err());
    }

    #[test]
    fn trimmed_concave_boolean_classification_and_rotated_reuse() {
        let outline = [[3., 2.], [3., 5.], [0., 5.], [0., 0.], [5., 0.], [5., 2.]];
        let hole = vec![[1., 1.], [1., 2.], [2., 2.], [2., 1.]];
        let mut stock = extrude_polygon_with_holes(&outline, &[hole], 0., 2.).unwrap();
        assert!(contains(&stock, [0.5, 1.5, 1.]).unwrap());
        assert!(!contains(&stock, [1.5, 1.5, 1.]).unwrap());
        assert!(!contains(&stock, [4., 3., 1.]).unwrap());
        let mut cutter = cuboid([0., 0., -1.], [2.5, 6., 3.]).unwrap();
        rotate_z(&mut stock, 0.37);
        rotate_z(&mut cutter, 0.37);
        for op in ["intersection", "difference", "union"] {
            let result = boolean(&stock, &cutter, op).unwrap_or_else(|e| panic!("{op}: {e:?}"));
            assert_eq!(result.validate().unwrap().boundary_edge_count, 0);
            // Consuming the result again exercises newly split boundary faces.
            let again =
                boolean(&result, &cutter, "union").unwrap_or_else(|e| panic!("reuse {op}: {e:?}"));
            assert_eq!(again.validate().unwrap().boundary_edge_count, 0);
        }
    }

    #[test]
    fn overlapping_box_booleans_are_closed_breps() {
        let a = cuboid([0.; 3], [2., 2., 2.]).unwrap();
        let b = cuboid([1., 0., 0.], [3., 2., 2.]).unwrap();
        for operation in ["union", "difference", "intersection"] {
            let result = boolean(&a, &b, operation).unwrap();
            assert!(result.validate().unwrap().boundary_edge_count == 0);
            assert_eq!(result.bodies.len(), 1, "{operation}");
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
    fn rotated_convex_planar_booleans_use_clipped_boundaries() {
        let a = cuboid([-2., -2., -1.], [2., 2., 1.]).unwrap();
        let mut b = cuboid([-2., -1., -1.], [2., 1., 1.]).unwrap();
        rotate_z(&mut b, std::f64::consts::FRAC_PI_4);
        b.validate().unwrap();
        for operation in ["union", "difference", "intersection"] {
            let result = boolean(&a, &b, operation).unwrap();
            assert_eq!(
                result.bodies.len(),
                if operation == "difference" { 4 } else { 1 },
                "{operation}"
            );
            assert!(result.faces.len() >= 8);
            assert_eq!(result.validate().unwrap().boundary_edge_count, 0);
        }
    }

    #[test]
    fn topology_ids_survive_preserved_boolean_entities_and_round_trip() {
        let stock = cuboid([0., 0., 0.], [3., 2., 2.]).unwrap();
        let cutter = cuboid([2., 0., 0.], [4., 2., 2.]).unwrap();
        let result = boolean(&stock, &cutter, "difference").unwrap();
        let shared_vertices = result
            .1
            .vertices
            .iter()
            .filter(|id| stock.1.vertices.contains(id))
            .count();
        let shared_edges = result
            .1
            .edges
            .iter()
            .filter(|id| stock.1.edges.contains(id))
            .count();
        let shared_faces = result
            .1
            .faces
            .iter()
            .filter(|id| stock.1.faces.contains(id))
            .count();
        assert!(shared_vertices >= 4);
        assert!(shared_edges >= 4);
        assert!(shared_faces >= 1);
        let restored: Model =
            value_codec::from_str(&value_codec::to_string(&result).unwrap()).unwrap();
        assert_eq!(restored.1.vertices, result.1.vertices);
        assert_eq!(restored.1.edges, result.1.edges);
        assert_eq!(restored.1.faces, result.1.faces);
        let split = boolean(
            &stock,
            &cuboid([1., -1., -1.], [2., 3., 3.]).unwrap(),
            "difference",
        )
        .unwrap();
        assert!(split.1.lineage.iter().any(|record| {
            record.operation == "split"
                && matches!(record.entity_kind.as_str(), "edge" | "face")
                && record.children.len() > 1
        }));
        let merged = boolean(
            &cuboid([0., 0., 0.], [2., 2., 2.]).unwrap(),
            &cuboid([1., 0., 0.], [3., 2., 2.]).unwrap(),
            "union",
        )
        .unwrap();
        assert!(merged.1.lineage.iter().any(|record| {
            record.operation == "merge"
                && matches!(record.entity_kind.as_str(), "edge" | "face")
                && record.parents.len() > 1
        }));
        assert_eq!(restored.1.lineage, result.1.lineage);
    }

    #[test]
    fn concave_and_holed_extrusions_use_planar_face_loops() {
        let concave = extrude_polygon(
            &[[0., 0.], [5., 0.], [5., 2.], [3., 2.], [3., 5.], [0., 5.]],
            0.,
            2.,
        )
        .unwrap();
        assert_eq!(concave.validate().unwrap().boundary_edge_count, 0);
        let result = extrude_polygon_with_holes(
            &[[0., 0.], [5., 0.], [5., 5.], [0., 5.]],
            &[vec![[1., 1.], [1., 2.], [2., 2.], [2., 1.]]],
            0.,
            2.,
        )
        .unwrap();
        assert_eq!(
            result
                .faces
                .iter()
                .filter(|face| !face.holes.is_empty())
                .count(),
            2
        );
        assert_eq!(result.validate().unwrap().boundary_edge_count, 0);
        assert!(
            extrude_polygon_with_holes(
                &[[0., 0.], [5., 0.], [5., 2.], [3., 2.], [3., 5.], [0., 5.]],
                &[vec![[1., 1.], [1., 2.], [2., 2.], [2., 1.]]],
                0.,
                2.,
            )
            .is_ok()
        );
    }

    #[test]
    fn blend_records_selected_edge_replacement_lineage() {
        let source = cuboid([0.; 3], [4.; 3]).unwrap();
        let result = fillet(&source, 0, 0.5, 4).unwrap();
        assert!(result.1.lineage.iter().any(|record| {
            record.entity_kind == "edge"
                && record.parents == [source.1.edges[0].clone()]
                && !record.children.is_empty()
        }));
    }

    #[test]
    fn rotated_nonconvex_planar_boolean_uses_bounded_arrangement() {
        let stock = cuboid([0., 0., 0.], [4., 4., 2.]).unwrap();
        let notch = cuboid([2., 2., -1.], [5., 5., 3.]).unwrap();
        let mut nonconvex = boolean(&stock, &notch, "difference").unwrap();
        rotate_z(&mut nonconvex, std::f64::consts::PI / 9.);
        let mut cutter = cuboid([1., -1., -1.], [3., 5., 3.]).unwrap();
        rotate_z(&mut cutter, -std::f64::consts::PI / 12.);
        let result = boolean(&nonconvex, &cutter, "intersection").unwrap();
        assert_eq!(result.validate().unwrap().boundary_edge_count, 0);
        assert_eq!(result.bodies.len(), 1);
        assert!(result.faces.len() > 6);
    }

    #[test]
    fn faceted_round_primitives_are_labeled_topological_solids() {
        let cylinder = faceted_cylinder(2., 5., 16).unwrap();
        assert_eq!((cylinder.faces.len(), cylinder.bodies.len()), (18, 1));
        assert_eq!(cylinder.validate().unwrap().boundary_edge_count, 0);

        let sphere = faceted_sphere(2., 16, 8).unwrap();
        assert_eq!((sphere.faces.len(), sphere.bodies.len()), (224, 1));
        assert_eq!(sphere.validate().unwrap().boundary_edge_count, 0);
    }

    #[test]
    fn convex_profile_admission_refuses_a_same_turn_pentagram() {
        let ring: Vec<[f64; 2]> = (0..5)
            .map(|i| {
                let angle = (i as f64) * std::f64::consts::TAU / 5.;
                [angle.cos(), angle.sin()]
            })
            .collect();
        let star: Vec<_> = [0, 2, 4, 1, 3].iter().map(|&i| ring[i]).collect();
        assert!(validate_convex_profile(&star, "Test").is_err());
        assert!(validate_convex_profile(&ring, "Test").is_ok());
        let twice: Vec<_> = ring.iter().chain(&ring).copied().collect();
        assert!(validate_convex_profile(&twice, "Test").is_err());
    }

    #[test]
    fn loft_world_bounds_do_not_limit_translated_local_coordinates() {
        let sections = vec![
            vec![
                [-800000., -1., 0.],
                [800000., -1., 0.],
                [800000., 1., 0.],
                [-800000., 1., 0.],
            ],
            vec![
                [-800000., -1., 2.],
                [800000., -1., 2.],
                [800000., 1., 2.],
                [-800000., 1., 2.],
            ],
        ];
        faceted_loft(&sections).unwrap().validate().unwrap();
    }

    #[test]
    fn faceted_loft_accepts_placed_parallel_sections_and_refuses_nonparallel_ones() {
        let c = 0.5_f64.sqrt();
        let place = |p: [f64; 3]| {
            [
                c * p[0] + c * p[2] + 5.,
                p[1] - 3.,
                -c * p[0] + c * p[2] + 7.,
            ]
        };
        let sections = vec![
            vec![[-2., -2., 0.], [2., -2., 0.], [2., 2., 0.], [-2., 2., 0.]]
                .into_iter()
                .map(place)
                .collect(),
            vec![[-1., -1., 3.], [1., -1., 3.], [1., 1., 3.], [-1., 1., 3.]]
                .into_iter()
                .map(place)
                .collect(),
        ];
        let model = faceted_loft(&sections).unwrap();
        assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
        assert_eq!(model.faces.len(), 10);
        let mut tilted = sections.clone();
        tilted[1][0][2] += 0.1;
        assert!(faceted_loft(&tilted).is_err());
        let mut backwards = sections.clone();
        backwards.reverse();
        assert!(faceted_loft(&backwards).is_err());
    }

    #[test]
    fn faceted_loft_sweep_and_revolve_are_closed_planar_breps() {
        let loft = faceted_loft(&[
            vec![[-2., -2., 0.], [2., -2., 0.], [2., 2., 0.], [-2., 2., 0.]],
            vec![[-1., -1., 3.], [1., -1., 3.], [1., 1., 3.], [-1., 1., 3.]],
        ])
        .unwrap();
        assert_eq!((loft.faces.len(), loft.bodies.len()), (10, 1));
        assert_eq!(loft.validate().unwrap().boundary_edge_count, 0);

        let sweep = faceted_sweep(
            &[[-1., -1.], [1., -1.], [1., 1.], [-1., 1.]],
            &[[0., 0., 0.], [0., 0., 3.], [2., 0., 5.]],
            [0., 1., 0.],
        )
        .unwrap();
        assert_eq!((sweep.faces.len(), sweep.bodies.len()), (18, 1));
        assert_eq!(sweep.validate().unwrap().boundary_edge_count, 0);

        let revolve = faceted_revolve(&[[0., -2.], [2., -2.], [2., 2.], [0., 2.]], 16).unwrap();
        assert_eq!(revolve.bodies.len(), 1);
        assert_eq!(revolve.validate().unwrap().boundary_edge_count, 0);
    }

    #[test]
    fn separated_boolean_preserves_components_and_regularized_empty() {
        let a = cuboid([0.; 3], [1.; 3]).unwrap();
        let separated = cuboid([2., 0., 0.], [3., 1., 1.]).unwrap();
        let union = boolean(&a, &separated, "union").unwrap();
        assert_eq!(union.bodies.len(), 2);
        union.validate().unwrap();
        assert!(boolean(&a, &separated, "intersection").unwrap().is_empty());
    }

    #[test]
    fn enclosed_difference_builds_an_inner_shell_and_remains_a_boolean_operand() {
        let outer = cuboid([0.; 3], [4.; 3]).unwrap();
        let inner = cuboid([1.; 3], [3.; 3]).unwrap();
        let cavity = boolean(&outer, &inner, "difference").unwrap();
        assert_eq!(cavity.bodies.len(), 1);
        assert_eq!(cavity.shells.len(), 2);
        assert_eq!(cavity.bodies[0].inner_shells.len(), 1);
        cavity.validate().unwrap();
        let filled = boolean(&cavity, &inner, "union").unwrap();
        assert!(filled.bodies[0].inner_shells.is_empty());
        assert_eq!(bounds(&filled), ([0.; 3], [4.; 3]));
        filled.validate().unwrap();

        let untouched = boolean(
            &cavity,
            &cuboid([1.25; 3], [2.75; 3]).unwrap(),
            "difference",
        )
        .unwrap();
        assert_eq!(untouched.bodies[0].inner_shells.len(), 1);
        untouched.validate().unwrap();
    }

    #[test]
    fn difference_can_split_a_body_and_chained_booleans_accept_it() {
        let stock = cuboid([0., 0., 0.], [3., 1., 1.]).unwrap();
        let splitter = cuboid([1., -1., -1.], [2., 2., 2.]).unwrap();
        let split = boolean(&stock, &splitter, "difference").unwrap();
        assert_eq!(split.bodies.len(), 2);
        assert_eq!(split.shells.len(), 2);
        split.validate().unwrap();

        let cap = cuboid([0., 0., 0.], [1.5, 1., 1.]).unwrap();
        let result = boolean(&split, &cap, "intersection").unwrap();
        assert_eq!(result.bodies.len(), 1);
        assert_eq!(bounds(&result), ([0.; 3], [1., 1., 1.]));
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
    fn convex_profile_extrusion_and_connected_edge_chains_are_manifold() {
        let wedge = extrude_polygon(&[[0., 0.], [4., 0.], [0., 3.]], -1., 2.).unwrap();
        assert_eq!(
            (
                wedge.vertices.len(),
                wedge.edges.len(),
                wedge.faces.len(),
                wedge.bodies.len()
            ),
            (6, 9, 5, 1)
        );
        wedge.validate().unwrap();
        let slanted = wedge
            .edges
            .iter()
            .position(|edge| {
                let a = wedge.vertices[edge.vertices[0]].point;
                let b = wedge.vertices[edge.vertices[1]].point;
                (a[0] - b[0]).abs() > 1e-6 && (a[1] - b[1]).abs() > 1e-6
            })
            .unwrap();
        chamfer(&wedge, slanted, 0.25).unwrap().validate().unwrap();
        fillet(&wedge, slanted, 0.25, 6)
            .unwrap()
            .validate()
            .unwrap();

        let model = cuboid([0.; 3], [10.; 3]).unwrap();
        let connected = model.edges[0]
            .vertices
            .iter()
            .find_map(|vertex| {
                (1..model.edges.len()).find(|edge| model.edges[*edge].vertices.contains(vertex))
            })
            .unwrap();
        let chamfered = chamfer_edges(&model, &[0, connected], 1.).unwrap();
        let filleted = fillet_edges(&model, &[0, connected], 1., 4).unwrap();
        assert!(chamfered.faces.len() > 7);
        assert!(filleted.faces.len() > chamfered.faces.len());
        chamfered.validate().unwrap();
        filleted.validate().unwrap();
        assert_eq!(
            chamfer_edges(&model, &[0, 6], 1.).unwrap_err().code,
            UNSUPPORTED
        );
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
