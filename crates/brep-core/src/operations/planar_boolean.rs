use super::*;
pub(crate) fn model_from_planes(planes: &[Plane], tolerance: f64) -> Result<Model> {
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

type ConvexBoundary = (Vec<Plane>, Vec<Vec<[f64; 3]>>);

fn convex_boundary(model: &Model) -> Result<ConvexBoundary> {
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

pub(crate) fn convex_boolean(a: &Model, b: &Model, operation: &str) -> Result<Model> {
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
                    for triangle in fill.indices.as_chunks::<3>().0 {
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
pub(crate) fn planar_boolean(a: &Model, b: &Model, operation: &str) -> Result<Model> {
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

pub(crate) fn orthogonal(model: &Model) -> Result<()> {
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

pub(crate) fn contains_faces(model: &Model, faces: &[usize], point: [f64; 3]) -> Result<bool> {
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

pub(crate) fn contains(model: &Model, point: [f64; 3]) -> Result<bool> {
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
