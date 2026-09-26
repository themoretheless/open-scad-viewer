//! Shared canonical-structure classification helpers for the analytic solid
//! recognizers in this module: exact side/cap surface classification and
//! unit-square boundary trim certification.
use super::sphere_sphere::ARC_WEIGHT;
use crate::Model;
use nurbs_core::curve::Curve;

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
