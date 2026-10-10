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
            if !close(*normalized.last().unwrap(), point, tolerance * 4.) {
                normalized.push(point);
            }
        }
    }
    *polygon = normalized;
}

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

pub(crate) fn model_from_trimmed_polygons(mut polygons: Vec<PlanarBoundary>, tolerance: f64) -> Result<Model> {
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
        // Author exact coordinate-plane charts from the original polygon
        // coordinates. Normalizing decimal UVs and lifting them independently
        // can introduce a nonzero exact edge/face discrepancy.
        let coordinate_chart = (0..3).find_map(|fixed| {
            let height=all_points[0][fixed];
            if !all_points.iter().all(|p|p[fixed]==height) {return None;}
            let mut axes=[(fixed+1)%3,(fixed+2)%3];
            if normal[fixed]<0. {axes.swap(0,1);}
            let ranges=axes.map(|axis| [
                all_points.iter().map(|p|p[axis]).fold(f64::INFINITY,f64::min),
                all_points.iter().map(|p|p[axis]).fold(f64::NEG_INFINITY,f64::max)]);
            if ranges.iter().any(|r|r[1]-r[0]<=tolerance) {return None;}
            // The edge graph chooses the canonical vertex. Do not project a
            // tolerance-merged vertex into this plane or outside its rectangle.
            if all_points.iter().any(|p|model.vertices.iter().find(|v|close(v.point,*p,tolerance*4.))
                .is_some_and(|v|v.point[fixed]!=height || (0..2).any(|k|v.point[axes[k]]<ranges[k][0] || v.point[axes[k]]>ranges[k][1]))) {return None;}
            Some((fixed,height,axes,ranges))
        });
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
                uv.push(if let Some((_,_,axes,_))=coordinate_chart {
                    axes.map(|axis|model.vertices[id].point[axis])
                } else {[
                    (dot(d, u) - min_u) / (max_u - min_u),
                    (dot(d, v) - min_v) / (max_v - min_v),
                ]});
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
        let mut surface=Surface {
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
        if let Some((fixed,height,axes,ranges))=coordinate_chart {
            for a in 0..2 {for b in 0..2 {
                let mut p=vec![0.;3];p[fixed]=height;p[axes[0]]=ranges[0][a];p[axes[1]]=ranges[1][b];
                surface.control_points[a][b]=p;
            }}
            surface.knots_u=vec![ranges[0][0],ranges[0][0],ranges[0][1],ranges[0][1]];
            surface.knots_v=vec![ranges[1][0],ranges[1][0],ranges[1][1],ranges[1][1]];
        }
        model.faces.push(Face {
            surface,
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
