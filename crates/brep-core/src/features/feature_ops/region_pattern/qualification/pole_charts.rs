//! Original ruled-pole quotient for ordinary analytic cone patches. A strict
//! source ray denominator and angular derivative also prove rank away from the
//! declared pole; injectivity alone would not supply this regularity proof.
use super::*;
pub(super) fn verify(model: &Model, face: usize, remaining: &mut usize) -> Result<bool> {
    let f = &model.faces[face];
    let s = &f.surface;
    let natural = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    let coedges = std::iter::once(f.outer)
        .chain(f.holes.iter().copied())
        .flat_map(|w| model.loops[w].coedges.iter())
        .collect::<Vec<_>>();
    let poles = coedges
        .iter()
        .filter(|c| model.edges[c.edge].degenerate)
        .collect::<Vec<_>>();
    if poles.len() != 1 {
        return Ok(false);
    }
    let c = poles[0];
    let e = &model.edges[c.edge];
    if e.vertices[0] != e.vertices[1] {
        return Ok(false);
    }
    let pole = model.vertices[e.vertices[0]].point;
    if e.curve
        .control_points
        .iter()
        .any(|p| p.as_slice() != pole.as_slice())
    {
        return Ok(false);
    }
    if coedges.iter().any(|c| {
        c.pcurve
            .control_points
            .iter()
            .any(|p| (0..2).any(|k| p[k] < natural[k][0] || p[k] > natural[k][1]))
    }) {
        return Ok(false);
    }
    let a = &c.pcurve.control_points[0];
    let b = c.pcurve.control_points.last().unwrap();
    let Some((axis, end)) = (0..2)
        .flat_map(|axis| (0..2).map(move |end| (axis, end)))
        .find(|&(axis, end)| a[axis] == b[axis] && a[axis] == natural[axis][end])
    else {
        return Ok(false);
    };
    if *remaining == 0 {
        return Err(error("Ruled pole proof work exhausted"));
    }
    let transposed;
    let source = if axis == 0 {
        s
    } else {
        transposed = nurbs_core::surface_edit::transpose(s)?;
        &transposed
    };
    let r = nurbs_core::surface_quotient_injectivity::certify_ruled_source_frame(
        source, end, 8, *remaining,
    )?;
    if r.cells > *remaining {
        return Err(error("Ruled pole proof exceeds its allowance"));
    }
    *remaining -= r.cells;
    Ok(r.proven)
}
