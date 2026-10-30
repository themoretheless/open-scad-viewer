use super::*;
use crate::{numerics::curve_jets, surface_measure::jets};
fn derivative(
    pc: &Curve,
    s: &Surface,
    t: [f64; 2],
    reverse: bool,
    work: &mut usize,
    max: usize,
) -> Result<Option<Vec<I>>> {
    let d = pc.domain();
    let a = crate::curve_secant_trace::map(d, t[0], reverse)?;
    let b = crate::curve_secant_trace::map(d, t[1], reverse)?;
    let natural = [a.lo.min(b.lo).max(d[0]), a.hi.max(b.hi).min(d[1])];
    let owning: Vec<_> = (pc.degree..pc.control_points.len())
        .filter(|&i| {
            pc.knots[i] < pc.knots[i + 1]
                && pc.knots[i] <= natural[0]
                && natural[1] <= pc.knots[i + 1]
        })
        .collect();
    if owning.len() != 1 || natural[0] >= natural[1] || *work == max {
        return Ok(None);
    }
    *work += 1;
    let p = curve_jets::enclose(pc, owning[0], natural)?;
    let Some(uv) = field::clip(pc, s, [[p[0][0].lo, p[0][0].hi], [p[0][1].lo, p[0][1].hi]])? else {
        return Ok(None);
    };
    let mut spans = [0; 2];
    for (axis, (degree, knots, n)) in [
        (s.degree_u, &s.knots_u, s.control_points.len()),
        (s.degree_v, &s.knots_v, s.control_points[0].len()),
    ]
    .into_iter()
    .enumerate()
    {
        // Refuse even an endpoint contact with another original span here.
        // The mean-value integral never crosses a possibly discontinuous jet.
        let hits: Vec<_> = (degree..n)
            .filter(|&i| {
                knots[i] < knots[i + 1] && uv[axis][0] <= knots[i + 1] && knots[i] <= uv[axis][1]
            })
            .collect();
        if hits.len() != 1 {
            return Ok(None);
        }
        spans[axis] = hits[0];
    }
    if *work == max {
        return Ok(None);
    }
    *work += 1;
    let signed = I::point(d[1])
        .sub(I::point(d[0]))?
        .mul(I::point(if reverse { -1. } else { 1. }))?;
    let factor = signed.div(I::point(natural[1]).sub(I::point(natural[0]))?)?;
    let width = [
        I::point(s.knots_u[spans[0] + 1]).sub(I::point(s.knots_u[spans[0]]))?,
        I::point(s.knots_v[spans[1] + 1]).sub(I::point(s.knots_v[spans[1]]))?,
    ];
    let rates = [
        p[1][0].mul(factor)?.div(width[0])?,
        p[1][1].mul(factor)?.div(width[1])?,
    ];
    // A sampled normal proposes only a conditioning frame. Its complete
    // original projected Jacobian must be separated from zero below.
    let midpoint = uv.map(|d| d[0] * 0.5 + d[1] * 0.5);
    let frame = s
        .evaluate(midpoint[0], midpoint[1])
        .ok()
        .and_then(|e| e.unit_normal())
        .and_then(|n| {
            let z = (0..3).max_by(|&a, &b| n[a].abs().total_cmp(&n[b].abs()))?;
            if n[z] == 0. {
                return None;
            }
            let axes = [(z + 1) % 3, (z + 2) % 3, z];
            let plane = [-n[axes[0]] / n[z], -n[axes[1]] / n[z]];
            plane.iter().all(|x| x.is_finite()).then_some((axes, plane))
        });
    let original = super::super::trace_mean_value::shape_derivative(
        &jets::calculate_partial_stable(s, spans, uv, [None; 2])?,
        rates,
    )?;
    if let Some(frame) = frame {
        let graph = jets::calculate_partial_stable_graph_third(s, spans, uv, [None; 2], frame)?;
        if let Some(mut projected) =
            super::super::trace_mean_value::graph_shape_derivative(&graph, rates, frame)?
        {
            // Both enclosures describe the same original directional field.
            // Their intersection cannot widen either proved enclosure.
            if let Some(original) = original {
                for (a, b) in projected.iter_mut().zip(original) {
                    *a = I::new(a.lo.max(b.lo), a.hi.min(b.hi))?;
                }
            }
            return Ok(Some(projected));
        }
    }
    Ok(original)
}
pub(super) fn bounds(
    p: [&Curve; 2],
    s: [&Surface; 2],
    reverse: [bool; 2],
    t: [f64; 2],
    work: &mut usize,
    max: usize,
) -> Result<Option<([[f64; 2]; 2], [[f64; 2]; 2])>> {
    if t[0] >= t[1] {
        return Ok(None);
    }
    let Some(da) = derivative(p[0], s[0], t, reverse[0], work, max)? else {
        return Ok(None);
    };
    let Some(db) = derivative(p[1], s[1], t, reverse[1], work, max)? else {
        return Ok(None);
    };
    let mid = t[0] * 0.5 + t[1] * 0.5;
    let Some(a) = field::get(p[0], s[0], [mid, mid], reverse[0], work, max)? else {
        return Ok(None);
    };
    let Some(b) = field::get(p[1], s[1], [mid, mid], reverse[1], work, max)? else {
        return Ok(None);
    };
    let delta = I::new(t[0], t[1])?.sub(I::point(mid))?;
    let mut upper = [[0.; 2]; 2];
    let mut lower = [[0_f64; 2]; 2];
    for sign in 0..2 {
        let scale = I::point(if sign == 0 { 1. } else { -1. });
        let mut residual = [I::point(0.); 12];
        for k in 0..12 {
            residual[k] = a.hull[k]
                .sub(b.hull[k].mul(scale)?)?
                .add(delta.mul(da[k].sub(db[k].mul(scale)?)?)?)?;
        }
        upper[sign] = [
            box_distance(&residual[..3], &[I::point(0.); 3])?.1.min(2.),
            box_distance(&residual[3..], &[I::point(0.); 9])?.1,
        ];
        for x in &a.witnesses {
            for y in &b.witnesses {
                let d = field::distances(x, y, sign)?;
                for k in 0..2 {
                    lower[sign][k] = lower[sign][k].max(d[k][0]);
                }
            }
        }
    }
    Ok(Some((upper, lower)))
}
