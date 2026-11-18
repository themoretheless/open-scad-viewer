//! step v3 export algorithm: merge_periodic_patch_grid.
use super::*;
pub fn merge_periodic_patch_grid(
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
