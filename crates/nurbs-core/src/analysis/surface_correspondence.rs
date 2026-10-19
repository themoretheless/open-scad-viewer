//! Whole-domain distance under an authored common UV correspondence.
//! Positive rational carriers may have different degrees, knots and weights.
//! This bounds corresponding images (hence Hausdorff distance on any identical
//! trimmed subdomain), not injectivity, regularity or a source motion law.
use crate::surface_injectivity::{PolarPoly, polar_poly_add, polar_poly_mul};
use crate::{
    Result, check,
    distance_bounds::{Interval as I, midpoint},
    surface::Surface,
};

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub cells: usize,
    pub products: usize,
}
#[derive(Clone, Debug)]
pub struct Report {
    pub error_upper: Option<f64>,
    pub within_budget: bool,
    pub cells: usize,
    pub products: usize,
    pub reason: Option<&'static str>,
}
fn domain(s: &Surface) -> [[f64; 2]; 2] {
    [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ]
}
fn norm_upper(v: [I; 3]) -> Result<f64> {
    let mut sum = I::point(0.);
    for x in v {
        let a = I::point(x.lo.abs().max(x.hi.abs()));
        sum = sum.add(a.mul(a)?)?;
    }
    Ok(sum.hi.sqrt().next_up())
}
fn extraction_cost(s: &Surface) -> usize {
    let (p, q) = (s.degree_u, s.degree_v);
    6 * (p + 1) * (q + 1) + 8 * (q + 1) * (p + 1) * (q * (q + 1) / 2 + p * (p + 1) / 2)
}
fn upper(a: &Surface, b: &Surface, spans: [[usize; 2]; 2], rect: [[f64; 2]; 2]) -> Result<f64> {
    let mut nets = [
        crate::curve_surface_composition::surface_net_on(a, spans[0], rect)?,
        crate::curve_surface_composition::surface_net_on(b, spans[1], rect)?,
    ];
    // A common translation cancels before multiplying, limiting loss at large origins.
    let origin = &a.control_points[spans[0][0] - a.degree_u][spans[0][1] - a.degree_v];
    for net in &mut nets {
        for h in net.iter_mut().flatten() {
            for k in 0..3 {
                h[k] = h[k].sub(I::point(origin[k]).mul(h[3])?)?;
            }
        }
    }
    let component = |n: usize, k: usize| -> PolarPoly {
        nets[n]
            .iter()
            .map(|r| r.iter().map(|h| h[k]).collect())
            .collect()
    };
    let wa = component(0, 3);
    let wb = component(1, 3);
    let denominator = polar_poly_mul(&wa, &wb)?;
    check(
        denominator.iter().flatten().all(|w| w.lo > 0.),
        "Correspondence denominator not enclosed positive",
    )?;
    let mut numerators: [PolarPoly; 3] = std::array::from_fn(|_| Vec::new());
    for (k, n) in numerators.iter_mut().enumerate() {
        let x = polar_poly_mul(&component(0, k), &wb)?;
        let y = polar_poly_mul(&component(1, k), &wa)?;
        *n = polar_poly_add(&x, &y, -1.)?;
    }
    // N/D is a convex combination of N_i/D_i with positive weights D_i B_i.
    // Dividing corresponding coefficients retains their shared dependence,
    // e.g. a translation stays small even with strongly varying rational weights.
    let mut maximum = 0_f64;
    for (i, row) in denominator.iter().enumerate() {
        for (j, &w) in row.iter().enumerate() {
            let delta = [
                numerators[0][i][j].div(w)?,
                numerators[1][i][j].div(w)?,
                numerators[2][i][j].div(w)?,
            ];
            maximum = maximum.max(norm_upper(delta)?);
        }
    }
    Ok(maximum)
}
/// All unfinished cells invalidate the bound. Work is shared over the complete
/// common knot partition, never restarted per span or subdivision child.
pub fn inspect(a: &Surface, b: &Surface, tolerance: f64, limits: Limits) -> Result<Report> {
    a.validate()?;
    b.validate()?;
    check(
        tolerance.is_finite()
            && tolerance >= 0.
            && limits.cells <= 100000
            && limits.products <= 1000000,
        "Invalid correspondence tolerance/work limits",
    )?;
    let mut out = Report {
        error_upper: None,
        within_budget: false,
        cells: 0,
        products: 0,
        reason: Some("work-limit"),
    };
    if domain(a) != domain(b) || a.periodic_u != b.periodic_u || a.periodic_v != b.periodic_v {
        out.reason = Some("parameter-domain-mismatch");
        return Ok(out);
    }
    if limits.cells == 0 {
        return Ok(out);
    }
    if a.degree_u == b.degree_u
        && a.degree_v == b.degree_v
        && a.knots_u == b.knots_u
        && a.knots_v == b.knots_v
        && a.weights == b.weights
    {
        // Identical positive rational basis: difference is a convex combination
        // of original control differences. No rounded decomposition is needed.
        let cost = 3 * a.control_points.len() * a.control_points[0].len();
        if cost > limits.products {
            return Ok(out);
        }
        out.cells = 1;
        out.products = cost;
        let mut maximum = 0_f64;
        for (pa, pb) in a
            .control_points
            .iter()
            .flatten()
            .zip(b.control_points.iter().flatten())
        {
            if pa == pb {
                continue;
            }
            let d = [0, 1, 2]
                .map(|k| I::point(pa[k]).sub(I::point(pb[k])))
                .into_iter()
                .collect::<Result<Vec<_>>>()?;
            maximum = maximum.max(norm_upper([d[0], d[1], d[2]])?);
        }
        out.error_upper = Some(maximum);
        out.within_budget = maximum <= tolerance;
        out.reason = if out.within_budget {
            None
        } else {
            Some("error-budget-exceeded")
        };
        return Ok(out);
    }
    let knots = |axis: usize| {
        let (ka, kb) = if axis == 0 {
            (&a.knots_u, &b.knots_u)
        } else {
            (&a.knots_v, &b.knots_v)
        };
        let d = domain(a)[axis];
        let mut k: Vec<_> = ka
            .iter()
            .chain(kb)
            .copied()
            .filter(|x| *x >= d[0] && *x <= d[1])
            .collect();
        k.sort_by(f64::total_cmp);
        k.dedup();
        k
    };
    let u = knots(0);
    let v = knots(1);
    if (u.len() - 1) * (v.len() - 1) > limits.cells {
        return Ok(out);
    }
    let mut stack = Vec::new();
    let span = |s: &Surface, axis: usize, r: [f64; 2]| {
        let (p, k, n) = if axis == 0 {
            (s.degree_u, &s.knots_u, s.control_points.len())
        } else {
            (s.degree_v, &s.knots_v, s.control_points[0].len())
        };
        (p..n)
            .find(|&i| k[i] <= r[0] && r[1] <= k[i + 1] && k[i] < k[i + 1])
            .unwrap()
    };
    for x in u.windows(2) {
        for y in v.windows(2) {
            let r = [[x[0], x[1]], [y[0], y[1]]];
            stack.push((
                r,
                [
                    [span(a, 0, r[0]), span(a, 1, r[1])],
                    [span(b, 0, r[0]), span(b, 1, r[1])],
                ],
            ));
        }
    }
    let cost = extraction_cost(a)
        + extraction_cost(b)
        + 35 * (a.degree_u + 1) * (a.degree_v + 1) * (b.degree_u + 1) * (b.degree_v + 1)
        + 6 * (a.degree_u + b.degree_u + 1) * (a.degree_v + b.degree_v + 1);
    let mut maximum = 0_f64;
    while let Some((rect, spans)) = stack.pop() {
        if out.cells == limits.cells || cost > limits.products - out.products {
            return Ok(out);
        }
        out.cells += 1;
        out.products += cost;
        let value = match upper(a, b, spans, rect) {
            Ok(v) if v.is_finite() => v,
            _ => {
                out.reason = Some("numeric-enclosure-unresolved");
                return Ok(out);
            }
        };
        if value <= tolerance {
            maximum = maximum.max(value);
            continue;
        }
        let full = domain(a);
        let mut axes = [0, 1];
        axes.sort_by(|&i, &j| {
            ((rect[j][1] - rect[j][0]) / (full[j][1] - full[j][0]))
                .total_cmp(&((rect[i][1] - rect[i][0]) / (full[i][1] - full[i][0])))
        });
        let Some((axis, middle)) = axes.into_iter().find_map(|axis| {
            let m = midpoint(rect[axis][0], rect[axis][1]);
            (rect[axis][0] < m && m < rect[axis][1]).then_some((axis, m))
        }) else {
            out.reason = Some("parameter-resolution-exhausted");
            return Ok(out);
        };
        let mut left = rect;
        let mut right = rect;
        left[axis][1] = middle;
        right[axis][0] = middle;
        stack.push((right, spans));
        stack.push((left, spans));
    }
    out.error_upper = Some(maximum);
    out.within_budget = true;
    out.reason = None;
    Ok(out)
}
