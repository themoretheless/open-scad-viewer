//! Canonical face senses: every reversed face use is rewritten into a forward
//! one by mirroring the surface in u, so writers and incidence proposals see a
//! single orientation per face. The result is re-validated before it is
//! returned.
use crate::Model;
use nurbs_core::Result;
use std::collections::BTreeSet;

pub fn canonical_face_senses(model: &Model) -> Result<Model> {
    let mut result = model.clone();
    let reversed = result
        .shells
        .iter()
        .flat_map(|shell| &shell.faces)
        .filter(|usage| usage.reversed)
        .map(|usage| usage.face)
        .collect::<BTreeSet<_>>();
    for face_index in reversed {
        let face = &mut result.0.faces[face_index];
        let sum = face.surface.knots_u[face.surface.degree_u]
            + face.surface.knots_u[face.surface.knots_u.len() - face.surface.degree_u - 1];
        face.surface.control_points.reverse();
        face.surface.weights.reverse();
        face.surface.knots_u.reverse();
        for knot in &mut face.surface.knots_u {
            *knot = sum - *knot
        }
        let mut loops = vec![face.outer];
        loops.extend(face.holes.iter().copied());
        for loop_index in loops {
            for coedge in &mut result.0.loops[loop_index].coedges {
                for point in &mut coedge.pcurve.control_points {
                    point[0] = sum - point[0]
                }
                coedge.pcurve = coedge.pcurve.reverse()?;
                coedge.reversed = !coedge.reversed;
            }
            result.0.loops[loop_index].coedges.reverse();
        }
    }
    for shell in &mut result.0.shells {
        for usage in &mut shell.faces {
            usage.reversed = false
        }
    }
    result.validate()?;
    Ok(result)
}
