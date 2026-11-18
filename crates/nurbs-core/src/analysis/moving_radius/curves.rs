//! Full ORIGINAL curve-pair distance law on independent normalized clocks.
//! A matched distance is NOT nearest-point or Hausdorff authority by itself.
use super::*;
use std::collections::VecDeque;
pub struct Certificate {
    curves: [Curve; 2],
    reverse: bool,
    requested: f64,
    tolerance: f64,
    error: f64,
    residual: [f64; 2],
}
impl Certificate {
    pub fn curves(&self) -> &[Curve; 2] {
        &self.curves
    }
    pub fn reverse(&self) -> bool {
        self.reverse
    }
    pub fn requested(&self) -> f64 {
        self.requested
    }
    pub fn error_upper(&self) -> f64 {
        self.error
    }
    pub fn squared_residual(&self) -> [f64; 2] {
        self.residual
    }
    pub fn reverify(&self, cells: usize, work: u64) -> Result<Report> {
        qualify(
            [&self.curves[0], &self.curves[1]],
            self.reverse,
            self.requested,
            self.tolerance,
            cells,
            work,
        )
    }
}
pub struct Report {
    pub certificate: Option<Certificate>,
    pub cells: usize,
    pub work: u64,
    pub accepted_cells: usize,
    pub uncertain_t: Option<[f64; 2]>,
    pub witness_t: Option<f64>,
    pub reason: &'static str,
}
fn map(c: &Curve, t: f64, reverse: bool) -> Result<I> {
    let d = c.domain();
    let f = if reverse {
        I::point(1.).sub(I::point(t))?
    } else {
        I::point(t)
    };
    if f.lo == 0. && f.hi == 0. {
        return Ok(I::point(d[0]));
    }
    if f.lo == 1. && f.hi == 1. {
        return Ok(I::point(d[1]));
    }
    I::point(d[0])
        .add(I::point(d[1]).sub(I::point(d[0]))?.mul(f)?)?
        .intersect(d[0], d[1])
}
fn norm(v: &[I]) -> Result<I> {
    let mut n = I::point(0.);
    for x in v {
        n = n.add(x.mul(*x)?.intersect(0., f64::INFINITY)?)?;
    }
    I::new(
        n.lo.max(0.).sqrt().next_down().max(0.),
        n.hi.sqrt().next_up(),
    )
}
fn witness(
    curves: [&Curve; 2],
    reverse: bool,
    requested: f64,
    tolerance: f64,
    t: f64,
) -> Result<bool> {
    let p = curves
        .iter()
        .enumerate()
        .map(|(i, c)| crate::interval_eval::evaluate_interval(c, map(c, t, i == 1 && reverse)?))
        .collect::<Result<Vec<_>>>()?;
    let distance = norm(
        &(0..3)
            .map(|k| p[0][k].sub(p[1][k]))
            .collect::<Result<Vec<_>>>()?,
    )?;
    let delta = distance.sub(I::point(requested))?;
    Ok(delta.lo > tolerance || delta.hi < -tolerance)
}
// The original full-span Bernstein net is reparameterized with OUTWARD affine
// endpoint intervals. No rounded knot-remap/trim Curve replaces the source.
// Extension beyond a chart at rounded split boundaries is deliberately NOT
// clipped; every intersected original chart must cover its actual subset.
fn net(c: &Curve, span: usize, ends: [I; 2], origin: &[f64]) -> Result<Vec<Vec<I>>> {
    let d = [c.knots[span], c.knots[span + 1]];
    let original = super::curve_net(c, span, d, origin)?;
    let alpha = ends
        .into_iter()
        .map(|v| {
            v.sub(I::point(d[0]))?
                .div(I::point(d[1]).sub(I::point(d[0]))?)
        })
        .collect::<Result<Vec<_>>>()?;
    let mut out = Vec::new();
    for k in 0..=c.degree {
        let mut h = original.clone();
        for r in 1..=c.degree {
            let a = alpha[usize::from(r > c.degree - k)];
            let b = I::point(1.).sub(a)?;
            for j in (r..=c.degree).rev() {
                for component in 0..4 {
                    h[j][component] = h[j - 1][component].mul(b)?.add(h[j][component].mul(a)?)?;
                }
            }
        }
        out.push(h[c.degree].clone());
    }
    Ok(out)
}
fn spans(c: &Curve, ends: [I; 2]) -> Vec<usize> {
    let lo = ends[0].lo.min(ends[1].lo);
    let hi = ends[0].hi.max(ends[1].hi);
    (c.degree..c.control_points.len())
        .filter(|i| c.knots[*i] < c.knots[*i + 1] && c.knots[*i] <= hi && lo <= c.knots[*i + 1])
        .collect()
}
/// Every point pair (A(t),B(t)) or (A(t),B(1-t)), with each ORIGINAL natural
/// domain normalized independently, has distance within tolerance of requested.
/// All original knot charts/incident sides participate. Unproved coverage,
/// hidden discrepancies and exhausted work publish no certificate.
pub fn qualify(
    curves: [&Curve; 2],
    reverse: bool,
    requested: f64,
    tolerance: f64,
    max_cells: usize,
    max_work: u64,
) -> Result<Report> {
    for c in curves {
        c.validate()?;
        check(
            c.control_points[0].len() == 3
                && !c.periodic
                && c.weights.iter().all(|w| *w > 0.)
                && c.degree <= 8
                && c.control_points.len() <= 256,
            "Distance laws need bounded original positive nonperiodic 3D curves",
        )?;
    }
    check(
        requested.is_finite()
            && requested >= 0.
            && tolerance.is_finite()
            && tolerance > 0.
            && (1..=100000).contains(&max_cells)
            && (1..=100000000).contains(&max_work),
        "Choose a finite nonnegative distance, positive tolerance and bounded curve-pair work",
    )?;
    let mut out = Report {
        certificate: None,
        cells: 0,
        work: 0,
        accepted_cells: 0,
        uncertain_t: Some([0., 1.]),
        witness_t: None,
        reason: "curve-distance-work-limit",
    };
    // Only a rigorous point enclosure may refute; it never grants admission.
    for t in [0., 0.5, 1.] {
        if out.cells == max_cells {
            return Ok(out);
        }
        out.cells += 1;
        if witness(curves, reverse, requested, tolerance, t)? {
            out.reason = "curve-distance-witness";
            out.witness_t = Some(t);
            return Ok(out);
        }
    }
    let mut cuts = vec![0., 1.];
    for (i, c) in curves.iter().enumerate() {
        let d = c.domain();
        for &k in &c.knots {
            if d[0] < k && k < d[1] {
                let mut t = (k - d[0]) / (d[1] - d[0]);
                if i == 1 && reverse {
                    t = 1. - t;
                }
                cuts.push(t);
            }
        }
    }
    cuts.sort_by(f64::total_cmp);
    cuts.dedup();
    if cuts.len() > max_cells {
        return Ok(out);
    }
    let mut pending = cuts
        .windows(2)
        .map(|t| [t[0], t[1]])
        .collect::<VecDeque<_>>();
    let origin = &curves[0].control_points[0];
    let (mut error, mut residual) = (0f64, [f64::INFINITY, f64::NEG_INFINITY]);
    while let Some(t) = pending.pop_front() {
        out.uncertain_t = Some(t);
        let a = [map(curves[0], t[0], false)?, map(curves[0], t[1], false)?];
        let b = [
            map(curves[1], t[0], reverse)?,
            map(curves[1], t[1], reverse)?,
        ];
        let mut proved = true;
        let mut local_error = 0f64;
        let mut local = [f64::INFINITY, f64::NEG_INFINITY];
        let first = spans(curves[0], a);
        let second = spans(curves[1], b);
        if first.is_empty() || second.is_empty() {
            out.reason = "curve-distance-chart-unresolved";
            return Ok(out);
        }
        'branches: for i in first {
            for &j in &second {
                if out.cells >= max_cells || out.work >= max_work {
                    return Ok(out);
                }
                out.cells += 1;
                let a = net(curves[0], i, a, origin)?;
                let b = net(curves[1], j, b, origin)?;
                let a = a
                    .into_iter()
                    .map(|h| vec![[h[0], h[1], h[2], h[3]]])
                    .collect::<Vec<_>>();
                let law = vec![vec![I::point(requested), I::point(0.), I::point(1.)]];
                let mut reason = "curve-distance-polynomial-unresolved";
                let Some((r, e)) = super::bound_coefficient_nets(
                    super::CoefficientNets {
                        net: &a,
                        centers: &b,
                        radius: &law,
                    },
                    0,
                    requested,
                    &mut out.work,
                    max_work,
                    &mut reason,
                )?
                else {
                    if reason == "moving-radius-denominator-unresolved" {
                        // An adjacent ORIGINAL chart may meet only an outward
                        // endpoint halo. Its polynomial extension can have a
                        // pole far outside its actual subset. Subdivide the
                        // shared clock, never discard that source branch.
                        proved = false;
                        break 'branches;
                    }
                    out.reason = reason;
                    return Ok(out);
                };
                proved &= e <= tolerance;
                local_error = local_error.max(e);
                local[0] = local[0].min(r[0]);
                local[1] = local[1].max(r[1]);
            }
        }
        if proved {
            out.accepted_cells += 1;
            error = error.max(local_error);
            residual[0] = residual[0].min(local[0]);
            residual[1] = residual[1].max(local[1]);
            continue;
        }
        let middle = t[0] * 0.5 + t[1] * 0.5;
        if middle <= t[0] || middle >= t[1] {
            out.reason = "curve-distance-resolution-limit";
            return Ok(out);
        }
        if out.cells == max_cells {
            return Ok(out);
        }
        out.cells += 1;
        if witness(curves, reverse, requested, tolerance, middle)? {
            out.reason = "curve-distance-witness";
            out.witness_t = Some(middle);
            return Ok(out);
        }
        pending.push_back([t[0], middle]);
        pending.push_back([middle, t[1]]);
    }
    out.certificate = Some(Certificate {
        curves: curves.map(Clone::clone),
        reverse,
        requested,
        tolerance,
        error,
        residual,
    });
    out.uncertain_t = None;
    out.reason = "curve-distance-qualified";
    Ok(out)
}
