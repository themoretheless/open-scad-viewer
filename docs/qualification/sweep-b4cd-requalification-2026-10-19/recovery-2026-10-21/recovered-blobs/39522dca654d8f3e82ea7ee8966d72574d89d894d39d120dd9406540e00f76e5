//! Compact rational traces across original tensor knot cells. Candidate
//! construction is independent from the full original-source admission bound.
use super::*;
/// Preserve a complete original multi-span pcurve. Piecewise trace
/// knots/controls are candidates only; the caller must verify against the
/// unchanged original pcurve and surface, including all knot-rounding error.
pub(super) fn piecewise(p: &Curve, s: &Surface) -> Result<Option<Curve>> {
    if p.periodic {
        return Ok(None);
    }
    let mut points = Vec::new();
    let mut weights = Vec::<f64>::new();
    let mut knots = Vec::new();
    let mut degree = None;
    for segment in p.decompose()? {
        let [a, b] = segment.domain();
        if !(b - a).is_finite() {
            return Ok(None);
        }
        // Rounded extraction is only a coefficient proposal. The caller's
        // complete agreement check still uses the unchanged ORIGINAL pcurve.
        let Some(c) = super::trace_candidate(segment.definition(), s)? else {
            return Ok(None);
        };
        let d = c.degree;
        if degree.is_some_and(|old| old != d)
            || points.len() + c.control_points.len() - usize::from(!points.is_empty()) > 256
        {
            return Ok(None);
        }
        if points.is_empty() {
            knots.extend(std::iter::repeat_n(a, d + 1));
            points.extend(c.control_points.iter().cloned());
            weights.extend(c.weights.iter().copied());
        } else {
            knots.extend(std::iter::repeat_n(a, d));
            points.extend(c.control_points.iter().skip(1).cloned());
            let factor = *weights.last().unwrap() / c.weights[0];
            weights.extend(c.weights.iter().skip(1).map(|w| w * factor));
        }
        let cd = c.domain();
        for &t in &c.knots[d + 1..c.control_points.len()] {
            let v = a + (b - a) * ((t - cd[0]) / (cd[1] - cd[0]));
            if !v.is_finite() || v <= a || v >= b {
                return Ok(None);
            }
            knots.push(v);
        }
        degree = Some(d);
    }
    let Some(degree) = degree else {
        return Ok(None);
    };
    knots.extend(std::iter::repeat_n(p.domain()[1], degree + 1));
    let scale = weights.iter().copied().fold(0., f64::max);
    let curve = Curve {
        degree,
        knots,
        weights: weights.iter().map(|w| w / scale).collect(),
        control_points: points,
        periodic: false,
    };
    if curve.validate().is_err() {
        return Ok(None);
    }
    Ok(Some(curve))
}
pub(super) fn trace(p: &Curve, s: &Surface) -> Result<Option<Curve>> {
    let (du, dv) = (s.degree_u, s.degree_v);
    let (nu, nv) = (s.control_points.len(), s.control_points[0].len());
    if p.degree != 1
        || p.control_points.len() != 2
        || p.control_points[0].len() != 2
        || p.periodic
        || !bezier(&p.knots, p.degree, p.control_points.len())
        || p.weights[0] != p.weights[1]
        || s.periodic_u
        || s.periodic_v
        // The unchanged verifier admits spatial degree <=16; candidate
        // construction does not enlarge its proof or control-work limits.
        || du + dv > 16
        || nu.saturating_mul(nv) > 4096
        || !s.weights.iter().flatten().all(|&w| w == s.weights[0][0])
    {
        return Ok(None);
    }
    let domain = [
        [s.knots_u[du], s.knots_u[nu]],
        [s.knots_v[dv], s.knots_v[nv]],
    ];
    if p.control_points
        .iter()
        .any(|q| (0..2).any(|k| q[k] < domain[k][0] || q[k] > domain[k][1]))
    {
        return Ok(None);
    }
    let mut breaks = vec![0., 1.];
    for axis in 0..2 {
        let a = p.control_points[0][axis];
        let b = p.control_points[1][axis];
        if a == b {
            continue;
        }
        let knots = if axis == 0 { &s.knots_u } else { &s.knots_v };
        for &k in knots {
            if k > a.min(b) && k < a.max(b) {
                let t = (k - a) / (b - a);
                if t > 0. && t < 1. {
                    breaks.push(t);
                }
            }
        }
    }
    breaks.sort_by(f64::total_cmp);
    breaks.dedup();
    if breaks.len() > 33 {
        return Ok(None);
    }
    let mut normalized = s.clone();
    for w in normalized.weights.iter_mut().flatten() {
        *w = 1.;
    }
    let uv = |t: f64| -> Vec<f64> {
        (0..2)
            .map(|k| {
                let value = if t == 0. {
                    p.control_points[0][k]
                } else if t == 1. {
                    p.control_points[1][k]
                } else {
                    (1. - t) * p.control_points[0][k] + t * p.control_points[1][k]
                };
                value.clamp(domain[k][0], domain[k][1])
            })
            .collect()
    };
    let mut joints = Vec::new();
    for &t in &breaks {
        let q = uv(t);
        joints.push(normalized.evaluate(q[0], q[1])?.point.to_vec());
    }
    let degree = du + dv;
    let mut points = Vec::new();
    for (piece, ends) in breaks.windows(2).enumerate() {
        let middle = (ends[0] + ends[1]) * 0.5;
        if !(middle > ends[0] && middle < ends[1]) {
            return Ok(None);
        }
        let q = uv(middle);
        let Some(iu) = span(&s.knots_u, du, nu, Interval::point(q[0])) else {
            return Ok(None);
        };
        let Some(iv) = span(&s.knots_v, dv, nv, Interval::point(q[1])) else {
            return Ok(None);
        };
        let line = Curve::from_polyline(vec![uv(ends[0]), uv(ends[1])])?;
        let net = surface_net(&normalized, [iu, iv])?;
        let lift = lift_coefficients(
            &homogeneous(&line, false)?,
            &net,
            [du, dv],
            [
                [s.knots_u[iu], s.knots_u[iu + 1]],
                [s.knots_v[iv], s.knots_v[iv + 1]],
            ],
        )?;
        let mut poles: Vec<Vec<f64>> = (0..=degree)
            .map(|j| {
                (0..3)
                    .map(|k| lift[k][j].lo * 0.5 + lift[k][j].hi * 0.5)
                    .collect()
            })
            .collect();
        // Known constant weights make the exact composition polynomial. Do not
        // introduce artificial rational denominators from midpoint roundoff.
        poles[0] = joints[piece].clone();
        poles[degree] = joints[piece + 1].clone();
        if piece == 0 {
            points.push(poles[0].clone());
        }
        points.extend(poles.into_iter().skip(1));
    }
    let mut knots = vec![0.; degree + 1];
    for &t in &breaks[1..breaks.len() - 1] {
        knots.extend(std::iter::repeat_n(t, degree));
    }
    knots.extend(std::iter::repeat_n(1., degree + 1));
    let curve = Curve {
        degree,
        knots,
        weights: vec![1.; points.len()],
        control_points: points,
        periodic: false,
    };
    curve.validate()?;
    Ok(Some(curve))
}
