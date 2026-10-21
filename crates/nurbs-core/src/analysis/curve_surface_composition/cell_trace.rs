//! Rational/nonlinear candidate composition on one ORIGINAL knot cell.
use super::*;
pub(super) fn candidate(p: &Curve, s: &Surface) -> Result<Option<Curve>> {
    let degree = p.degree * (s.degree_u + s.degree_v);
    if p.periodic || !bezier(&p.knots, p.degree, p.control_points.len()) || degree > 25 {
        return Ok(None);
    }
    let mut cell = [0; 2];
    for a in 0..2 {
        let lo = p
            .control_points
            .iter()
            .map(|p| p[a])
            .fold(f64::INFINITY, f64::min);
        let hi = p
            .control_points
            .iter()
            .map(|p| p[a])
            .fold(f64::NEG_INFINITY, f64::max);
        let (knots, d, n) = if a == 0 {
            (&s.knots_u, s.degree_u, s.control_points.len())
        } else {
            (&s.knots_v, s.degree_v, s.control_points[0].len())
        };
        let Some(i) =
            (d..n).find(|&i| knots[i] < knots[i + 1] && lo >= knots[i] && hi <= knots[i + 1])
        else {
            return Ok(None);
        };
        cell[a] = i;
    }
    let pc_constant = p.weights.iter().all(|w| *w == p.weights[0]);
    let support_constant = s.weights.iter().flatten().all(|w| *w == s.weights[0][0]);
    let mut pc = p.clone();
    let mut support = s.clone();
    // Uniform rational gauges cancel exactly. This affects only the proposal;
    // the public lift admission still uses both original immutable definitions.
    if pc_constant {
        pc.weights.fill(1.);
    }
    if support_constant {
        support.weights.iter_mut().for_each(|r| r.fill(1.));
    }
    let net = surface_net(&support, cell)?;
    let domain = [
        [s.knots_u[cell[0]], s.knots_u[cell[0] + 1]],
        [s.knots_v[cell[1]], s.knots_v[cell[1] + 1]],
    ];
    let lift = match lift_coefficients(
        &homogeneous(&pc, false)?,
        &net,
        [s.degree_u, s.degree_v],
        domain,
    ) {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };
    let middle = |v: Interval| 0.5 * v.lo + 0.5 * v.hi;
    let polynomial = pc_constant && support_constant;
    let denominators = if polynomial {
        vec![1.; degree + 1]
    } else {
        lift[3].iter().map(|&w| middle(w)).collect::<Vec<_>>()
    };
    if denominators.iter().any(|w| !w.is_finite() || *w <= 0.) {
        return Ok(None);
    }
    let points = (0..=degree)
        .map(|i| {
            (0..3)
                .map(|k| middle(lift[k][i]) / denominators[i])
                .collect()
        })
        .collect();
    let scale = denominators.iter().copied().fold(0., f64::max);
    let c = Curve {
        degree,
        knots: [vec![0.; degree + 1], vec![1.; degree + 1]].concat(),
        control_points: points,
        weights: denominators.iter().map(|w| w / scale).collect(),
        periodic: false,
    };
    if c.validate().is_err() {
        return Ok(None);
    }
    Ok(Some(c))
}
