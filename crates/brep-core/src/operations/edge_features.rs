use super::*;
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
            .map(|(index, _)| result.1.edges[index])
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
                parents: vec![model.1.edges[edge_id]],
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
