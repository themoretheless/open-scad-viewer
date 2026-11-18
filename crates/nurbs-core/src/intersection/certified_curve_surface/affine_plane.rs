//! Global original-curve monotonicity owns plane roots at internal span joins.
use super::*;
use num_rational::BigRational as Q;
use num_traits::{FromPrimitive, ToPrimitive, Zero};
pub(super) fn q(x: f64) -> Q {
    Q::from_f64(x).unwrap()
}
pub(super) fn enclosure(x: &Q) -> Option<I> {
    let f = x.to_f64().filter(|v| v.is_finite())?;
    let mut lo = f;
    let mut hi = f;
    for _ in 0..8 {
        if q(lo) > *x {
            lo = lo.next_down();
        }
        if q(hi) < *x {
            hi = hi.next_up();
        }
        if lo.is_finite() && hi.is_finite() && q(lo) <= *x && *x <= q(hi) {
            return I::new(lo, hi).ok();
        }
    }
    None
}
pub(super) fn dot(a: &[Q; 3], b: &[Q; 3]) -> Q {
    (0..3).map(|i| &a[i] * &b[i]).sum()
}
pub(super) fn subtract(a: &[Q; 3], b: &[Q; 3]) -> [Q; 3] {
    std::array::from_fn(|i| &a[i] - &b[i])
}
fn exact_point(c: &Curve, t: f64) -> Option<[Q; 3]> {
    let p = c.degree;
    let n = c.control_points.len();
    if t == c.domain()[1] {
        return Some(std::array::from_fn(|k| q(c.control_points[n - 1][k])));
    }
    let span = (p..n).find(|&i| c.knots[i] <= t && t < c.knots[i + 1])?;
    let mut d: Vec<[Q; 4]> = (span - p..=span)
        .map(|i| {
            let w = q(c.weights[i]);
            [
                q(c.control_points[i][0]) * &w,
                q(c.control_points[i][1]) * &w,
                q(c.control_points[i][2]) * &w,
                w,
            ]
        })
        .collect();
    for r in 1..=p {
        for j in (r..=p).rev() {
            let i = span - p + j;
            let den = q(c.knots[i + p - r + 1]) - q(c.knots[i]);
            if den.is_zero() {
                return None;
            }
            let alpha = (q(t) - q(c.knots[i])) / den;
            let complement = q(1.) - &alpha;
            d[j] = std::array::from_fn(|k| &d[j - 1][k] * &complement + &d[j][k] * &alpha);
        }
    }
    if d[p][3] <= Q::zero() {
        return None;
    }
    Some(std::array::from_fn(|k| &d[p][k] / &d[p][3]))
}
pub(super) struct Attempt {
    pub report: Option<Report>,
    pub cells: usize,
}
/// Only a bounded fraction of the caller's work is spent on this alternate
/// proof. Unsupported/failed attempts retain their cost for the generic route.
pub(super) fn attempt(c: &Curve, s: &Surface, max_cells: usize) -> Result<Attempt> {
    let mut used = 0;
    let budget = (max_cells / 4).min(8192);
    macro_rules! no {
        () => {
            return Ok(Attempt {
                report: None,
                cells: used,
            })
        };
    }
    if budget == 0
        || c.degree < 2
        || c.degree > 16
        || c.control_points.len() > 4096
        || !c.weights.iter().all(|w| *w > 0.)
        || s.degree_u != 1
        || s.degree_v != 1
        || s.control_points.len() != 2
        || s.control_points[0].len() != 2
        || !s
            .weights
            .iter()
            .flatten()
            .all(|w| *w == s.weights[0][0] && *w > 0.)
    {
        no!();
    }
    let [a, b] = c.domain();
    let n = c.control_points.len();
    if !c.knots[..=c.degree].iter().all(|x| *x == a)
        || !c.knots[n..].iter().all(|x| *x == b)
        || !c.knots.iter().any(|x| a < *x && *x < b)
        || c.knots
            .iter()
            .any(|x| a < *x && *x < b && c.knots.iter().filter(|y| *y == x).count() > c.degree)
    {
        no!();
    }
    if s.knots_u[0] != s.knots_u[1]
        || s.knots_u[2] != s.knots_u[3]
        || s.knots_v[0] != s.knots_v[1]
        || s.knots_v[2] != s.knots_v[3]
    {
        no!();
    }
    used += 1;
    let pole = |u: usize, v: usize| std::array::from_fn(|k| q(s.control_points[u][v][k]));
    let origin = pole(0, 0);
    let u = subtract(&pole(1, 0), &origin);
    let v = subtract(&pole(0, 1), &origin);
    if subtract(&pole(1, 1), &pole(1, 0)) != v {
        no!();
    }
    let normal = [
        &u[1] * &v[2] - &u[2] * &v[1],
        &u[2] * &v[0] - &u[0] * &v[2],
        &u[0] * &v[1] - &u[1] * &v[0],
    ];
    let Some(ni) = normal.iter().map(enclosure).collect::<Option<Vec<_>>>() else {
        no!();
    };
    let f = |t: f64| exact_point(c, t).map(|p| dot(&normal, &subtract(&p, &origin)));
    let (Some(fa), Some(fb)) = (f(a), f(b)) else {
        no!();
    };
    let increasing = fa < Q::zero() && fb > Q::zero();
    if !increasing && !(fa > Q::zero() && fb < Q::zero()) {
        no!();
    }
    // Complete original-span sign cover. At unrelated C0 joins continuity of
    // the validated spline plus one-sided signs preserves global monotonicity.
    for span in c.degree..n {
        if c.knots[span] == c.knots[span + 1] {
            continue;
        }
        let mut stack = vec![[c.knots[span], c.knots[span + 1]]];
        while let Some(range) = stack.pop() {
            if used >= budget {
                no!();
            }
            used += 1;
            let jets = crate::curve_jets::enclose(c, span, range)?;
            let mut derivative = I::point(0.);
            for k in 0..3 {
                derivative = derivative.add(jets[1][k].mul(ni[k])?)?;
            }
            if (increasing && derivative.lo > 0.) || (!increasing && derivative.hi < 0.) {
                continue;
            }
            if (increasing && derivative.hi <= 0.) || (!increasing && derivative.lo >= 0.) {
                no!();
            }
            let mid = range[0] * 0.5 + range[1] * 0.5;
            if mid == range[0] || mid == range[1] {
                no!();
            }
            stack.push([mid, range[1]]);
            stack.push([range[0], mid]);
        }
    }
    let mut band = [a, b];
    loop {
        let mid = band[0] * 0.5 + band[1] * 0.5;
        if mid == band[0] || mid == band[1] {
            break;
        }
        if used >= budget {
            no!();
        }
        used += 1;
        let Some(value) = f(mid) else {
            no!();
        };
        if value.is_zero() {
            band = [mid, mid];
            break;
        }
        if (value < Q::zero()) == increasing {
            band[0] = mid;
        } else {
            band[1] = mid;
        }
    }
    if band[0] == band[1] && c.knots.iter().filter(|&&x| x == band[0]).count() > c.degree - 1 {
        no!();
    }
    let Some(span) = (c.degree..n).find(|&i| {
        c.knots[i] < c.knots[i + 1] && c.knots[i] <= band[0] && band[1] <= c.knots[i + 1]
    }) else {
        no!();
    };
    if used >= budget {
        no!();
    }
    used += 1;
    let position = crate::interval_eval::evaluate_interval(c, I::new(band[0], band[1])?)?;
    let Some((i, j, det)) = [(0, 1), (0, 2), (1, 2)]
        .into_iter()
        .map(|(i, j)| (i, j, &u[i] * &v[j] - &u[j] * &v[i]))
        .find(|(_, _, d)| !d.is_zero())
    else {
        no!();
    };
    let Some(det_i) = enclosure(&det) else {
        no!();
    };
    let Some(ui) = u.iter().map(enclosure).collect::<Option<Vec<_>>>() else {
        no!();
    };
    let Some(vi) = v.iter().map(enclosure).collect::<Option<Vec<_>>>() else {
        no!();
    };
    let Some(oi) = origin.iter().map(enclosure).collect::<Option<Vec<_>>>() else {
        no!();
    };
    let di = position[i].sub(oi[i])?;
    let dj = position[j].sub(oi[j])?;
    let alpha = di.mul(vi[j])?.sub(dj.mul(vi[i])?)?.div_signed(det_i)?;
    let beta = ui[i].mul(dj)?.sub(ui[j].mul(di)?)?.div_signed(det_i)?;
    if alpha.hi < 0. || alpha.lo > 1. || beta.hi < 0. || beta.lo > 1. {
        return Ok(Attempt {
            cells: used,
            report: Some(Report {
                roots: vec![],
                unresolved: vec![],
                cells: used,
                excluded: 1,
                complete: true,
            }),
        });
    }
    if alpha.lo <= 0. || alpha.hi >= 1. || beta.lo <= 0. || beta.hi >= 1. {
        no!();
    }
    let natural = |r: I, k: &[f64]| -> Result<I> {
        I::point(k[1]).add(I::point(k[2]).sub(I::point(k[1]))?.mul(r)?)
    };
    let ub = natural(alpha, &s.knots_u)?;
    let vb = natural(beta, &s.knots_v)?;
    let norm = I::new(band[0], band[1])?
        .sub(I::point(c.knots[span]))?
        .div(I::point(c.knots[span + 1]).sub(I::point(c.knots[span]))?)?
        .intersect(0., 1.)?;
    let root = Root {
        proof: RootProof::MonotoneAffinePlane,
        existence_domain: None,
        spans: [span, 1, 1],
        normalized_cell: [[norm.lo, norm.hi], [alpha.lo, alpha.hi], [beta.lo, beta.hi]],
        parameter_box: [band, [ub.lo, ub.hi], [vb.lo, vb.hi]],
        position_bounds: std::array::from_fn(|k| [position[k].lo, position[k].hi]),
        crossing_sign: if increasing { 1 } else { -1 },
    };
    Ok(Attempt {
        cells: used,
        report: Some(Report {
            roots: vec![root],
            unresolved: vec![],
            cells: used,
            excluded: 0,
            complete: true,
        }),
    })
}
