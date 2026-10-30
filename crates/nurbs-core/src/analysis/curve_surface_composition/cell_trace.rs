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

    let domain = [
        [s.knots_u[cell[0]], s.knots_u[cell[0] + 1]],
        [s.knots_v[cell[1]], s.knots_v[cell[1] + 1]],
    ];
    let origin = center(s, cell);
    let net = surface_net_on_centered(&support, cell, domain, origin)?;
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
                .map(|k| middle(lift[k][i]) / denominators[i] + origin[k])
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

// An exact translation proposal shared by both constant and rational routes.
pub(super) fn center(s: &Surface, cell: [usize; 2]) -> [f64; 3] {
    std::array::from_fn(|axis| {
        let mut counts = std::collections::BTreeMap::<u64, (usize, usize)>::new();
        let mut index = 0;
        for i in cell[0] - s.degree_u..=cell[0] {
            for j in cell[1] - s.degree_v..=cell[1] {
                let value = s.control_points[i][j][axis];
                let key = if value == 0. { 0 } else { value.to_bits() };
                let entry = counts.entry(key).or_insert((0, index));
                entry.0 += 1;
                index += 1;
            }
        }
        let (&bits, _) = counts
            .iter()
            .max_by(|a, b| a.1.0.cmp(&b.1.0).then_with(|| b.1.1.cmp(&a.1.1)))
            .unwrap();
        f64::from_bits(bits)
    })
}
