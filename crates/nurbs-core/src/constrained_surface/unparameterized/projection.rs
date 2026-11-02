//! Numerical inverse candidates, with source rational basis fitting and refined
//! cubic UV bases. These objects have NO original-curve lift authority.
use super::*;
use crate::{
    curve::basis,
    robust_solvers::{self, SolverOptions},
};
fn domains(s: &Surface) -> [[f64; 2]; 2] {
    [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ]
}
fn parameter(c: &Curve, t: f64) -> f64 {
    let d = c.domain();
    (d[0] + t * (d[1] - d[0])).clamp(d[0], d[1])
}
fn distance(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(a, b)| (a - b) * (a - b)).sum()
}
fn inverse(
    s: &Surface,
    point: [f64; 3],
    previous: [f64; 2],
    iterations: usize,
) -> Result<([f64; 2], usize)> {
    let d = domains(s);
    let mut seeds = vec![
        previous,
        [(d[0][0] + d[0][1]) * 0.5, (d[1][0] + d[1][1]) * 0.5],
    ];
    for u in d[0] {
        for v in d[1] {
            seeds.push([u, v]);
        }
    }
    let mut best = None;
    let mut count = 0;
    for seed in seeds {
        count += 1;
        if let Ok((u, v, _)) = robust_solvers::trust_region_point_inversion_surface(
            s,
            point,
            seed[0],
            seed[1],
            SolverOptions {
                max_iterations: iterations,
                ..SolverOptions::default()
            },
        ) {
            let at = s.evaluate(u, v)?;
            let loss = distance(&at.point, &point);
            if loss.is_finite() && best.as_ref().is_none_or(|(_, l)| loss < *l) {
                best = Some(([u, v], loss));
            }
        }
    }
    best.map(|(uv, _)| (uv, count))
        .ok_or_else(|| error("No finite local UV inverse proposal"))
}
pub(super) fn propose(
    c: &Curve,
    s: &Surface,
    segments: usize,
    iterations: usize,
    smooth: bool,
) -> Result<(Curve, usize)> {
    let d = domains(s);
    let n = c.control_points.len();
    // Every original source break is also a new UV basis break. Sample each
    // candidate span densely enough for the cubic fallback least-squares fit.
    let cd = c.domain();
    let mut sites = (0..=segments)
        .map(|i| i as f64 / segments as f64)
        .collect::<Vec<_>>();
    sites.extend(
        c.knots
            .iter()
            .filter(|t| **t > cd[0] && **t < cd[1])
            .map(|t| (t - cd[0]) / (cd[1] - cd[0])),
    );
    sites.sort_by(f64::total_cmp);
    sites.dedup();
    let mut times = (0..=2 * n)
        .map(|i| i as f64 / (2 * n) as f64)
        .collect::<Vec<_>>();
    for pair in sites.windows(2) {
        for i in 0..=4 {
            times.push(pair[0] + (pair[1] - pair[0]) * i as f64 / 4.);
        }
    }
    times.sort_by(f64::total_cmp);
    times.dedup();
    if times.len() > 1537 {
        return Err(error("Unknown UV inverse sampling allowance exhausted"));
    }
    let mut previous = [(d[0][0] + d[0][1]) * 0.5, (d[1][0] + d[1][1]) * 0.5];
    let mut uv = Vec::new();
    let mut work = 0;
    for &t in &times {
        let p = c.evaluate(parameter(c, t))?.point;
        let (next, cost) = inverse(s, [p[0], p[1], p[2]], previous, iterations)?;
        work += cost;
        uv.push(next);
        previous = next;
    }
    if smooth {
        if let Some(p) = fitted(c, &times, &uv, d)? {
            return Ok((p, work));
        }
    }
    let knots = std::iter::repeat_n(0., 4)
        .chain(sites.iter().copied().skip(1).take(sites.len() - 2))
        .chain(std::iter::repeat_n(1., 4))
        .collect::<Vec<_>>();
    let controls = knots.len() - 4;
    if controls > 256 {
        return Err(error("Unknown UV cubic basis allowance exhausted"));
    }
    let template = Curve {
        degree: 3,
        knots,
        control_points: vec![vec![0.; 2]; controls],
        weights: vec![1.; controls],
        periodic: false,
    };
    let p = fitted(&template, &times, &uv, d)?
        .ok_or_else(|| error("Unknown UV cubic basis rank unresolved"))?;
    Ok((p, work))
}
fn fitted(
    template: &Curve,
    times: &[f64],
    uv: &[[f64; 2]],
    d: [[f64; 2]; 2],
) -> Result<Option<Curve>> {
    let n = template.control_points.len();
    let matrix = times
        .iter()
        .map(|t| {
            let b = basis(
                template.degree,
                &template.knots,
                n,
                parameter(template, *t),
                false,
            )?;
            let weighted = b
                .basis
                .iter()
                .zip(&template.weights)
                .map(|(b, w)| b * w)
                .collect::<Vec<_>>();
            let sum = weighted.iter().sum::<f64>();
            Ok(weighted.into_iter().map(|b| b / sum).collect::<Vec<_>>())
        })
        .collect::<Result<Vec<_>>>()?;
    let qr = robust_solvers::rrqr(&matrix, None)?;
    if qr.diagnostics.rank != n {
        return Ok(None);
    }
    let u = qr.solve_least_squares(&uv.iter().map(|p| p[0]).collect::<Vec<_>>())?;
    let v = qr.solve_least_squares(&uv.iter().map(|p| p[1]).collect::<Vec<_>>())?;
    let mut p = template.clone();
    // Hull clipping creates a NEW candidate; only final whole-curve original
    // admission can accept the actual saved trace after this operation.
    p.control_points = u
        .into_iter()
        .zip(v)
        .map(|(u, v)| vec![u.clamp(d[0][0], d[0][1]), v.clamp(d[1][0], d[1][1])])
        .collect();
    p.validate()?;
    Ok(Some(p))
}
