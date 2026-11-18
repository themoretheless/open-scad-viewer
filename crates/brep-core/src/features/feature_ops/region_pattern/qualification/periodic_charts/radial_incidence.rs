//! Bind a two-band radial carrier proof to the authored quotient rectangles.
use super::*;
use std::collections::BTreeMap;

pub(crate) struct RadialIncidence {
    pub periodic_axes: [usize; 2],
    pub rims: Vec<usize>,
}

/// Prerequisites are exact original world lifts and simple, positively wound
/// material trim loops. This census checks domain coverage and edge reuse.
pub(crate) fn inspect(
    model: &Model,
    faces: [usize; 2],
    remaining: &mut usize,
) -> Result<Option<RadialIncidence>> {
    if faces[0] == faces[1] {
        return Ok(None);
    }
    let mut axes = [0; 2];
    let mut rims: [[BTreeMap<usize, bool>; 2]; 2] = Default::default();
    for (part, face) in faces.into_iter().enumerate() {
        let f = &model.faces[face];
        let s = &f.surface;
        if s.periodic_u == s.periodic_v || !f.holes.is_empty() {
            return Ok(None);
        }
        let axis = usize::from(s.periodic_v);
        axes[part] = axis;
        let bounds = [
            [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
            [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
        ];
        let coedges = &model.loops[f.outer].coedges;
        let mut traces = Vec::new();
        for coedge in coedges {
            if coedge
                .trim()
                .is_some_and(|trim| trim.curve_parameter != model.edges[coedge.edge].curve.domain())
            {
                return Ok(None);
            }
            let trace = ChartTrace::read(coedge, true, remaining)?;
            charge(remaining, trace.poles().count())?;
            traces.push(trace);
        }
        let sides = full::rectangle_sides(coedges, bounds, &traces)?;
        full::verify_shores(&sides, axis)?;
        let [a, b] = [2 * (1 - axis), 2 * (1 - axis) + 1];
        if sides[a].keys().any(|edge| sides[b].contains_key(edge))
            || [a, b].into_iter().any(|rim| {
                sides[rim]
                    .keys()
                    .any(|edge| sides[2 * axis].contains_key(edge))
            })
        {
            return Ok(None);
        }
        rims[part] = [sides[a].clone(), sides[b].clone()];
    }
    for rim in 0..2 {
        let a = &rims[0][rim];
        let b = &rims[1][1 - rim];
        if a.len() != b.len()
            || a.iter()
                .any(|(edge, reversed)| b.get(edge) != Some(&!*reversed))
        {
            return Ok(None);
        }
    }
    Ok(Some(RadialIncidence {
        periodic_axes: axes,
        rims: rims[0].iter().flat_map(|rim| rim.keys().copied()).collect(),
    }))
}

pub(crate) fn certify_band_contact(
    model: &Model,
    faces: [usize; 2],
    remaining: &mut usize,
    pairs: &mut usize,
    max_pairs: usize,
) -> Result<Option<crate::boundary_hull_contact::Certificate>> {
    let Some(topology) = inspect(model, faces, remaining)? else {
        return Ok(None);
    };
    let surfaces = faces.map(|face| &model.faces[face].surface);
    for height in 0..3 {
        if *remaining == 0 || *pairs == max_pairs {
            break;
        }
        let proof = nurbs_core::surface_injectivity::certify_radial_band_pair(
            surfaces,
            topology.periodic_axes,
            height,
            (*remaining).min(100000),
            (max_pairs - *pairs).min(100000),
        )?;
        charge(remaining, proof.cells)?;
        *pairs += proof.pairs;
        if proof.proven {
            let mut enclosure = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
            for &edge in &topology.rims {
                let poles = &model.edges[edge].curve.control_points;
                charge(remaining, poles.len())?;
                for pole in poles {
                    for (interval, &value) in enclosure.iter_mut().zip(pole) {
                        interval[0] = interval[0].min(value);
                        interval[1] = interval[1].max(value);
                    }
                }
            }
            return Ok(Some(crate::boundary_hull_contact::Certificate {
                faces,
                contact_enclosure: enclosure,
                edges: topology.rims,
                vertex: None,
                joined_proof: None,
                polar_join: None,
                source_join: None,
            }));
        }
    }
    Ok(None)
}
