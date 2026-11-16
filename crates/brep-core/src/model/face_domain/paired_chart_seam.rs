//! A single periodic rectangle can use a topological seam to close its
//! lifted outer loop. For quotient trim, its two rims bound the material.
use super::{Curve, Model, Result};
use std::collections::BTreeMap;

pub(super) fn open(model: &Model, face: &crate::Face, loops: &mut Vec<Vec<Curve>>) -> Result<()> {
    let surface = &face.surface;
    let axis = match (surface.periodic_u, surface.periodic_v) {
        (true, false) => 0,
        (false, true) => 1,
        _ => return Ok(()),
    };
    let natural = crate::boolean_robustness::surface_uv_domains(surface);
    let [low, high] = natural[axis];
    if !loops[1..].iter().flatten().any(|curve| {
        curve
            .control_points
            .iter()
            .any(|p| p[axis] < low || p[axis] > high)
    }) {
        return Ok(());
    }
    let outer = &loops[0];
    let Some(rectangle) = super::rectangle_closure::inspect(outer) else {
        return Ok(());
    };
    let authored = &model.loops[face.outer].coedges;
    if rectangle[axis] != natural[axis] {
        return Ok(());
    }
    let unproved = || {
        crate::Error::new(
            "BREP_TRIM_DOMAIN",
            "Cross-seam hole requires the exact paired outer seam",
        )
    };
    if outer.len() != authored.len()
        || authored.iter().any(|c| {
            c.trim().is_some_and(|t| {
                t.pcurve_parameter != c.pcurve.domain()
                    || t.curve_parameter != model.edges[c.edge].curve.domain()
            })
        })
        || nurbs_core::periodic_trim::verify_surface_seam_identity(surface, axis)?.is_none()
    {
        return Err(unproved());
    }
    let transverse = 1 - axis;
    let mut rims: [Vec<Curve>; 2] = std::array::from_fn(|_| Vec::new());
    let mut seams: [BTreeMap<usize, (bool, [f64; 2])>; 2] =
        std::array::from_fn(|_| BTreeMap::new());
    for (curve, coedge) in outer.iter().zip(authored) {
        let p = &curve.control_points;
        if p[0][axis] == p[1][axis] {
            let side = usize::from(p[0][axis] == high);
            if seams[side]
                .insert(
                    coedge.edge,
                    (coedge.reversed, [p[0][transverse], p[1][transverse]]),
                )
                .is_some()
            {
                return Err(unproved());
            }
        } else {
            let side = usize::from(p[0][transverse] == rectangle[transverse][1]);
            rims[side].push(curve.clone());
        }
    }
    if seams[0].is_empty()
        || seams[0].len() != seams[1].len()
        || seams[0].iter().any(|(edge, (reversed, [start, end]))| {
            seams[1].get(edge) != Some(&(!*reversed, [*end, *start]))
        })
    {
        return Err(unproved());
    }
    // The rectangle audit established complete, nonoverlapping side coverage.
    // Its cyclic wire may start in the middle of one rim.
    for rim in &mut rims {
        let forward = rim[0].control_points[0][axis] < rim[0].control_points[1][axis];
        rim.sort_by(|a, b| {
            let cmp = a.control_points[0][axis].total_cmp(&b.control_points[0][axis]);
            if forward { cmp } else { cmp.reverse() }
        });
    }
    loops.splice(..1, rims);
    Ok(())
}
