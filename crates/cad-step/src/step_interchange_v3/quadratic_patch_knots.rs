use super::*;

pub(super) fn quadratic_patch_knots(segments: usize) -> Vec<f64> {
    let mut knots = vec![0.; 3];
    for knot in 1..segments {
        knots.extend([knot as f64, knot as f64])
    }
    knots.extend([segments as f64; 3]);
    knots
}
pub(super) fn merge_periodic_patch_grid(
    model: &Model,
    face_ids: &[usize],
    u_count: usize,
    v_count: usize,
) -> Result<Model> {
    if face_ids.len() != u_count * v_count {
        return Err(refuse("Periodic patch grid size mismatch"));
    }
    let first = &model.faces[face_ids[0]].surface;
    if first.degree_u != 2 || first.degree_v != 2 && v_count > 1 {
        return Err(refuse("Periodic patch grid degree mismatch"));
    }
    let rows = 2 * u_count + 1;
    let columns = if v_count == 1 {
        first.control_points[0].len()
    } else {
        2 * v_count + 1
    };
    let mut points = vec![vec![vec![f64::NAN; 3]; columns]; rows];
    let mut weights = vec![vec![f64::NAN; columns]; rows];
    for v in 0..v_count {
        for u in 0..u_count {
            let surface = &model.faces[face_ids[v * u_count + u]].surface;
            if surface.degree_u != first.degree_u
                || surface.degree_v != first.degree_v
                || surface.control_points.len() != 3
                || surface.control_points[0].len() != (if v_count == 1 { columns } else { 3 })
            {
                return Err(refuse("Periodic patch grid contains incompatible surface"));
            }
            for i in 0..surface.control_points.len() {
                for j in 0..surface.control_points[i].len() {
                    let (row, column) = (2 * u + i, if v_count == 1 { j } else { 2 * v + j });
                    if points[row][column][0].is_finite() {
                        let error = points[row][column]
                            .iter()
                            .zip(&surface.control_points[i][j])
                            .map(|(a, b)| (a - b) * (a - b))
                            .sum::<f64>()
                            .sqrt();
                        if error > 1e-9
                            || (weights[row][column] - surface.weights[i][j]).abs() > 1e-12
                        {
                            return Err(refuse("Periodic patch grid control boundaries disagree"));
                        }
                    } else {
                        points[row][column] = surface.control_points[i][j].clone();
                        weights[row][column] = surface.weights[i][j];
                    }
                }
            }
        }
    }
    let mut merged = Surface {
        degree_u: 2,
        degree_v: first.degree_v,
        knots_u: quadratic_patch_knots(u_count),
        knots_v: if v_count == 1 {
            first.knots_v.clone()
        } else {
            quadratic_patch_knots(v_count)
        },
        control_points: points,
        weights,
        periodic_u: false,
        periodic_v: false,
    };
    for column in [0, columns - 1] {
        let pole = merged.control_points[0][column].clone();
        if merged.control_points.iter().all(|row| {
            row[column]
                .iter()
                .zip(&pole)
                .map(|(left, right)| (left - right) * (left - right))
                .sum::<f64>()
                .sqrt()
                <= 1e-7
        }) {
            for row in &mut merged.control_points {
                row[column] = pole.clone()
            }
        }
    }
    for &face_id in face_ids {
        let face = &model.faces[face_id];
        for loop_id in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
            for coedge in &model.loops[loop_id].coedges {
                let edge = &model.edges[coedge.edge];
                if !edge.degenerate {
                    continue;
                }
                let endpoints = &coedge.pcurve.control_points;
                if endpoints.len() != 2 || endpoints[0][1] != endpoints[1][1] {
                    continue;
                }
                let column = if (endpoints[0][1] - merged.knots_v[merged.degree_v]).abs() <= 1e-12 {
                    0
                } else if (endpoints[0][1]
                    - merged.knots_v[merged.knots_v.len() - merged.degree_v - 1])
                    .abs()
                    <= 1e-12
                {
                    columns - 1
                } else {
                    continue;
                };
                let pole = model.vertices[edge.vertices[0]].point.to_vec();
                for row in &mut merged.control_points {
                    row[column] = pole.clone()
                }
            }
        }
    }
    merged.validate()?;
    let mut result = model.clone();
    for v in 0..v_count {
        for u in 0..u_count {
            let face = face_ids[v * u_count + u];
            result.faces[face].surface = merged.clone();
            let mut loops = vec![result.faces[face].outer];
            loops.extend(result.faces[face].holes.iter().copied());
            for loop_id in loops {
                for coedge in &mut result.loops[loop_id].coedges {
                    for point in &mut coedge.pcurve.control_points {
                        point[0] += u as f64;
                        if v_count > 1 {
                            point[1] += v as f64
                        }
                    }
                }
            }
        }
    }
    result.validate()?;
    Ok(result)
}
pub(super) fn periodicized_sphere(model: &Model) -> Result<Model> {
    let radius = model
        .vertices
        .iter()
        .map(|vertex| {
            vertex
                .point
                .iter()
                .map(|value| value * value)
                .sum::<f64>()
                .sqrt()
        })
        .fold(0., f64::max);
    let w = std::f64::consts::FRAC_1_SQRT_2;
    let circle = [
        [1., 0.],
        [1., 1.],
        [0., 1.],
        [-1., 1.],
        [-1., 0.],
        [-1., -1.],
        [0., -1.],
        [1., -1.],
        [1., 0.],
    ];
    let circle_weights = [1., w, 1., w, 1., w, 1., w, 1.];
    let profile = [
        [0., radius],
        [radius, radius],
        [radius, 0.],
        [radius, -radius],
        [0., -radius],
    ];
    let profile_weights = [1., w, 1., w, 1.];
    let control_points = circle
        .iter()
        .map(|u| {
            profile
                .iter()
                .map(|v| vec![v[0] * u[0], v[0] * u[1], v[1]])
                .collect()
        })
        .collect();
    let weights = circle_weights
        .iter()
        .map(|wu| profile_weights.iter().map(|wv| wu * wv).collect())
        .collect();
    let surface = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: quadratic_patch_knots(4),
        knots_v: quadratic_patch_knots(2),
        control_points,
        weights,
        periodic_u: false,
        periodic_v: false,
    };
    surface.validate()?;
    let mut result = model.clone();
    let source_edges = result.edges.clone();
    let north = model
        .vertices
        .iter()
        .position(|vertex| (vertex.point[2] - radius).abs() < radius * 1e-9)
        .ok_or_else(|| refuse("Sphere north pole is missing"))?;
    let south = model
        .vertices
        .iter()
        .position(|vertex| (vertex.point[2] + radius).abs() < radius * 1e-9)
        .ok_or_else(|| refuse("Sphere south pole is missing"))?;
    let selected = [0, 1, 3, 5, 7, 8];
    result.0.edges = selected
        .iter()
        .map(|index| {
            let mut edge = source_edges[*index].clone();
            if edge.curve.degree == 2 {
                edge.curve.weights = vec![1., w, 1.]
            }
            edge
        })
        .collect();
    let pole_edges = [north, south].map(|vertex| {
        let edge = result.edges.len();
        let point = result.vertices[vertex].point.to_vec();
        result.0.edges.push(Edge {
            degenerate: true,
            vertices: [vertex, vertex],
            curve: Curve {
                degree: 1,
                knots: vec![0., 0., 1., 1.],
                control_points: vec![point.clone(), point],
                weights: vec![1., 1.],
                periodic: false,
            },
        });
        edge
    });
    let line = |a: [f64; 2], b: [f64; 2]| Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![a.to_vec(), b.to_vec()],
        weights: vec![1., 1.],
        periodic: false,
    };
    result.0.loops = vec![
        Loop {
            coedges: vec![
                Coedge::new(
                    pole_edges[0],
                    false,
                    line([0., 0.], [4., 0.]),
                ),
                Coedge::new(
                    0,
                    true,
                    line([4., 0.], [4., 1.]),
                ),
                Coedge::new(
                    4,
                    false,
                    line([4., 1.], [3., 1.]),
                ),
                Coedge::new(
                    3,
                    true,
                    line([3., 1.], [2., 1.]),
                ),
                Coedge::new(
                    2,
                    true,
                    line([2., 1.], [1., 1.]),
                ),
                Coedge::new(
                    1,
                    true,
                    line([1., 1.], [0., 1.]),
                ),
                Coedge::new(
                    0,
                    false,
                    line([0., 1.], [0., 0.]),
                ),
            ],
        },
        Loop {
            coedges: vec![
                Coedge::new(
                    1,
                    false,
                    line([0., 1.], [1., 1.]),
                ),
                Coedge::new(
                    2,
                    false,
                    line([1., 1.], [2., 1.]),
                ),
                Coedge::new(
                    3,
                    false,
                    line([2., 1.], [3., 1.]),
                ),
                Coedge::new(
                    4,
                    true,
                    line([3., 1.], [4., 1.]),
                ),
                Coedge::new(
                    5,
                    false,
                    line([4., 1.], [4., 2.]),
                ),
                Coedge::new(
                    pole_edges[1],
                    false,
                    line([4., 2.], [0., 2.]),
                ),
                Coedge::new(
                    5,
                    true,
                    line([0., 2.], [0., 1.]),
                ),
            ],
        },
    ];
    result.0.faces = vec![
        Face {
            surface: surface.clone(),
            outer: 0,
            holes: vec![],
        },
        Face {
            surface,
            outer: 1,
            holes: vec![],
        },
    ];
    result.0.shells[model.bodies[0].outer_shell].faces = vec![
        FaceUse {
            face: 0,
            reversed: false,
        },
        FaceUse {
            face: 1,
            reversed: false,
        },
    ];
    result.rebuild_topology_ids();
    for (face_index, face) in result.faces.iter().enumerate() {
        for coedge in &result.loops[face.outer].coedges {
            for sample in 0..=8 {
                let t = sample as f64 / 8.;
                let uv = coedge.pcurve.evaluate(t)?.point;
                let p = face.surface.evaluate(uv[0], uv[1])?.point;
                let edge = &result.edges[coedge.edge];
                let domain = edge.curve.domain();
                let q = edge
                    .curve
                    .evaluate(if coedge.reversed {
                        domain[1] - t * (domain[1] - domain[0])
                    } else {
                        domain[0] + t * (domain[1] - domain[0])
                    })?
                    .point;
                let error = p
                    .iter()
                    .zip(&q)
                    .map(|(a, b)| (a - b) * (a - b))
                    .sum::<f64>()
                    .sqrt();
                if error > 1e-7 {
                    return Err(refuse(format!(
                        "Periodic sphere face {face_index} edge {} mismatch {error} at {t}: uv={uv:?} surface={p:?} edge={q:?}",
                        coedge.edge
                    )));
                }
            }
        }
    }
    result.validate()?;
    Ok(result)
}
// Face counts and degrees do not establish a periodic carrier: partial
// revolutions can have the same shape of patch records as a complete band.
pub(super) fn periodic_patch_grid_compatible(model: &Model, u_count: usize, v_count: usize) -> bool {
    let compatible = |a: &Surface, ai: usize, aj: usize, b: &Surface, bi: usize, bj: usize| {
        a.control_points[ai][aj].iter().zip(&b.control_points[bi][bj])
            .map(|(x,y)| (x-y)*(x-y)).sum::<f64>().sqrt() <= 1e-9
            && (a.weights[ai][aj]-b.weights[bi][bj]).abs() <= 1e-12
    };
    for v in 0..v_count {
        for u in 0..u_count {
            let a = &model.faces[v*u_count+u].surface;
            let b = &model.faces[v*u_count+(u+1)%u_count].surface;
            if a.control_points.len()!=3 || b.control_points.len()!=3
                || a.control_points[0].len()!=b.control_points[0].len() { return false; }
            for j in 0..a.control_points[0].len() {
                if !compatible(a,2,j,b,0,j) { return false; }
            }
            if v_count>1 {
                let b = &model.faces[((v+1)%v_count)*u_count+u].surface;
                if a.control_points[0].len()!=3 || b.control_points[0].len()!=3 { return false; }
                for i in 0..3 { if !compatible(a,i,2,b,i,0) { return false; } }
            }
        }
    }
    true
}
pub(super) fn periodicized_step_v6(model: &Model) -> Result<Model> {
    if model.faces.len() == 8
        && model.vertices.len() == 6
        && model
            .faces
            .iter()
            .all(|face| face.surface.degree_u == 2 && face.surface.degree_v == 2)
    {
        periodicized_sphere(model)
    } else if model.faces.len() >= 4
        && model.faces[..4].iter().all(|face| {
            face.surface.degree_u == 2
                && face.surface.degree_v == 1
                && face.surface.control_points.len() == 3
        })
    {
        if periodic_patch_grid_compatible(model,4,1) {
            merge_periodic_patch_grid(model, &[0, 1, 2, 3], 4, 1)
        } else { Ok(model.clone()) }
    } else if model.faces.len() == 16
        && model.faces.iter().all(|face| {
            face.surface.degree_u == 2
                && face.surface.degree_v == 2
                && face.surface.control_points.len() == 3
                && face.surface.control_points[0].len() == 3
        })
    {
        if periodic_patch_grid_compatible(model,4,4) {
            merge_periodic_patch_grid(model, &(0..16).collect::<Vec<_>>(), 4, 4)
        } else { Ok(model.clone()) }
    } else {
        Ok(model.clone())
    }
}

pub(super) fn constant_boundary(surface: &Surface, column: usize) -> bool {
    surface
        .control_points
        .iter()
        .all(|row| row[column] == surface.control_points[0][column])
}
