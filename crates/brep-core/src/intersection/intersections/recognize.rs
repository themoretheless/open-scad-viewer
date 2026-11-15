//! Shared canonical-structure classification helpers for the analytic solid
//! recognizers in this module: exact side/cap surface classification and
//! unit-square boundary trim certification.
use super::sphere_sphere::ARC_WEIGHT;
use super::{dot, sub};
use crate::Model;
use nurbs_core::{Result, curve::Curve};

pub(crate) fn unit_edge(curve: &Curve, from: [f64; 2], to: [f64; 2]) -> bool {
    curve.degree == 1
        && curve.knots == [0., 0., 1., 1.]
        && curve.control_points == [from.to_vec(), to.to_vec()]
        && curve.weights == [1., 1.]
}

/// Classifies every face of `model` as a canonical side quadrant or cap:
/// each face must be hole-free and either a degree-(2,1) ruled side patch
/// with exact quarter-arc weights or a degree-(1,1) bilinear cap with unit
/// weights. Returns the `(sides, caps)` face indices, or `None` on any
/// structural mismatch.
pub(crate) fn classify_analytic_faces(model: &Model) -> Option<(Vec<usize>, Vec<usize>)> {
    let mut sides: Vec<usize> = Vec::new();
    let mut caps: Vec<usize> = Vec::new();
    for (index, face) in model.faces.iter().enumerate() {
        let surface = &face.surface;
        if !face.holes.is_empty() {
            return None;
        }
        let side = surface.degree_u == 2
            && surface.degree_v == 1
            && !surface.periodic_u
            && !surface.periodic_v
            && surface.knots_u == [0., 0., 0., 1., 1., 1.]
            && surface.knots_v == [0., 0., 1., 1.]
            && surface.control_points.len() == 3
            && surface.control_points.iter().all(|row| row.len() == 2);
        let cap = surface.degree_u == 1
            && surface.degree_v == 1
            && !surface.periodic_u
            && !surface.periodic_v
            && surface.knots_u == [0., 0., 1., 1.]
            && surface.knots_v == [0., 0., 1., 1.]
            && surface.control_points.len() == 2
            && surface.control_points.iter().all(|row| row.len() == 2);
        if side {
            if surface.weights != [[1., 1.], [ARC_WEIGHT, ARC_WEIGHT], [1., 1.]] {
                return None;
            }
            sides.push(index);
        } else if cap {
            if surface.weights != [[1., 1.], [1., 1.]] {
                return None;
            }
            caps.push(index);
        } else {
            return None;
        }
    }
    Some((sides, caps))
}

/// Certifies the unit-square boundary trim of one face: the outer loop is
/// exactly the four unit edges, each exactly once.
pub(crate) fn unit_square_boundary(model: &Model, face_index: usize) -> bool {
    let boundary = [
        ([0., 0.], [1., 0.]),
        ([1., 0.], [1., 1.]),
        ([1., 1.], [0., 1.]),
        ([0., 1.], [0., 0.]),
    ];
    let loop_ = &model.loops[model.faces[face_index].outer];
    if loop_.coedges.len() != 4 {
        return false;
    }
    let mut seen = [false; 4];
    for coedge in &loop_.coedges {
        let mut hit = false;
        for (k, &(a, b)) in boundary.iter().enumerate() {
            if !seen[k] && unit_edge(&coedge.pcurve, a, b) {
                seen[k] = true;
                hit = true;
                break;
            }
        }
        if !hit {
            return false;
        }
    }
    seen.into_iter().all(|hit| hit)
}

/// Ring-frame quadrant directions, counter-clockwise from the frame x axis.
pub(crate) const QUADRANTS: [[f64; 2]; 4] = [[1., 0.], [0., 1.], [-1., 0.], [0., -1.]];
const QUARTER: f64 = std::f64::consts::FRAC_PI_2;
const TAU: f64 = std::f64::consts::TAU;

/// Snaps `angle` to the nearest multiple of a quarter turn and returns its
/// quadrant index, or `None` when the angular residual exceeds `tolerance`.
pub(crate) fn snap_quadrant(angle: f64, tolerance: f64) -> Option<usize> {
    let quadrant = (angle / QUARTER).round() as i64;
    let residual = (angle - quadrant as f64 * QUARTER + std::f64::consts::PI).rem_euclid(TAU)
        - std::f64::consts::PI;
    if residual.abs() > tolerance {
        return None;
    }
    Some(quadrant.rem_euclid(4) as usize)
}

/// Ring-frame (x, y) coefficients of the three control points of one
/// quadrant's quarter arc: start, corner, end.
pub(crate) fn quarter_arc_pattern(quadrant: usize) -> [[f64; 2]; 3] {
    let qa = QUADRANTS[quadrant];
    let qb = QUADRANTS[(quadrant + 1) % 4];
    [
        [qa[0], qa[1]],
        [qa[0] + qb[0], qa[1] + qb[1]],
        [qb[0], qb[1]],
    ]
}

/// Euclidean distance between a three-coordinate control point and its
/// expected position; the caller certifies the coordinate count.
pub(crate) fn deviation3(actual: &[f64], expected: [f64; 3]) -> f64 {
    let d = [
        actual[0] - expected[0],
        actual[1] - expected[1],
        actual[2] - expected[2],
    ];
    d[0].hypot(d[1]).hypot(d[2])
}

/// Certified deviation of a control point that must carry exactly three
/// coordinates and sit within `bound` of `expected`.
pub(crate) fn certified_deviation(actual: &[f64], expected: [f64; 3], bound: f64) -> Option<f64> {
    if actual.len() != 3 {
        return None;
    }
    let deviation = deviation3(actual, expected);
    if !deviation.is_finite() || deviation > bound {
        return None;
    }
    Some(deviation)
}

/// Component of `point - from` perpendicular to the unit `axis`.
pub(crate) fn radial(point: [f64; 3], from: [f64; 3], axis: [f64; 3]) -> [f64; 3] {
    let d = sub(point, from);
    let axial = dot(d, axis);
    sub(d, axis.map(|x| x * axial))
}

/// Ring centers `[bottom, top]` of four ruled side patches: each ring vertex
/// is shared by two adjacent patches, so the eight evaluated corners average
/// to the ring center.
pub(crate) fn ring_centers(model: &Model, sides: &[usize]) -> Result<[[f64; 3]; 2]> {
    let mut bottom = [0.; 3];
    let mut top = [0.; 3];
    for &index in sides {
        let surface = &model.faces[index].surface;
        for u in [0., 1.] {
            let b = surface.evaluate(u, 0.)?.point;
            let t = surface.evaluate(u, 1.)?.point;
            for k in 0..3 {
                bottom[k] += b[k] / 8.;
                top[k] += t[k] / 8.;
            }
        }
    }
    Ok([bottom, top])
}

/// Ring slot (0 = bottom, 1 = top) of an axial coordinate measured from the
/// bottom ring, with its deviation from that ring plane.
pub(crate) fn ring_slot(axial: f64, height: f64, tolerance: f64) -> Option<(usize, f64)> {
    if axial.abs() <= tolerance * height {
        Some((0, axial.abs()))
    } else if (axial - height).abs() <= tolerance * height {
        Some((1, (axial - height).abs()))
    } else {
        None
    }
}

/// One quadrant's exact cap trim arc: the quarter circle of radius 1/2
/// centered at [1/2, 1/2] in cap UV, weights cos(pi/4).
fn cap_quarter_arc(curve: &Curve, quadrant: usize) -> bool {
    let [x, y] = QUADRANTS[quadrant];
    let [nx, ny] = QUADRANTS[(quadrant + 1) % 4];
    curve.degree == 2
        && curve.knots == [0., 0., 0., 1., 1., 1.]
        && curve.weights == [1., ARC_WEIGHT, 1.]
        && curve.control_points
            == [
                [0.5 + x / 2., 0.5 + y / 2.].to_vec(),
                [0.5 + (x + nx) / 2., 0.5 + (y + ny) / 2.].to_vec(),
                [0.5 + nx / 2., 0.5 + ny / 2.].to_vec(),
            ]
}

/// Certifies a cap face's inscribed-circle trim: the outer loop is exactly
/// the four quarter arcs, each exactly once.
pub(crate) fn cap_quarter_trims(model: &Model, face_index: usize) -> bool {
    let loop_ = &model.loops[model.faces[face_index].outer];
    if loop_.coedges.len() != 4 {
        return false;
    }
    let mut seen_quadrant = [false; 4];
    for coedge in &loop_.coedges {
        let mut hit = false;
        for (quadrant, seen) in seen_quadrant.iter_mut().enumerate() {
            if !*seen && cap_quarter_arc(&coedge.pcurve, quadrant) {
                *seen = true;
                hit = true;
                break;
            }
        }
        if !hit {
            return false;
        }
    }
    seen_quadrant.into_iter().all(|hit| hit)
}
