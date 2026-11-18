//! Full original rectangular source-chart admission.
use super::*;

pub(super) fn rectangle(m: &Model, face: usize) -> Result<()> {
    let f = &m.faces[face];
    let s = &f.surface;
    let lo = [s.knots_u[s.degree_u], s.knots_v[s.degree_v]];
    let hi = [
        s.knots_u[s.control_points.len()],
        s.knots_v[s.control_points[0].len()],
    ];
    let wire = &m.loops[f.outer];
    if !f.holes.is_empty() || wire.coedges.len() != 4 {
        return Err(fail(
            "curved source requires complete rectangular UV faces; curved or holed trim charts are unproved",
        ));
    }
    let corners = [
        [lo[0], lo[1]],
        [hi[0], lo[1]],
        [hi[0], hi[1]],
        [lo[0], hi[1]],
    ];
    let mut starts = Vec::new();
    for c in &wire.coedges {
        let p = &c.pcurve;
        let d = p.domain();
        if p.control_points.len() != p.degree + 1
            || p.knots[..=p.degree].iter().any(|x| *x != d[0])
            || p.knots[p.degree + 1..].iter().any(|x| *x != d[1])
        {
            return Err(fail(
                "rectangular trim needs a clamped rational boundary segment",
            ));
        }
        let a = &p.control_points[0];
        let b = p.control_points.last().unwrap();
        let side = (0..4)
            .find(|i| a[..] == corners[*i] && b[..] == corners[(*i + 1) % 4])
            .ok_or_else(|| fail("trim does not exactly traverse a parameter rectangle side"))?;
        for q in &p.control_points {
            for k in 0..2 {
                if corners[side][k] == corners[(side + 1) % 4][k] {
                    if q[k] != corners[side][k] {
                        return Err(fail("trim leaves its exact rectangle side"));
                    }
                } else if q[k] < lo[k] || q[k] > hi[k] {
                    return Err(fail("trim exits its full rectangle interval"));
                }
            }
        }
        starts.push(side);
    }
    if (0..4).any(|i| starts[(i + 1) % 4] != (starts[i] + 1) % 4) {
        return Err(fail("rectangular trim side census is incomplete"));
    }
    Ok(())
}
