//! Projected graph proposals, including non-axis-aligned height directions.
//! Cage residuals propose a coordinate frame; only original lift admission
//! authorizes a pcurve. No graph classification is exposed as a certificate.
use super::{Curve, Result, Surface, domains};
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|i| a[i] * b[i]).sum()
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}
fn norm(a: [f64; 3]) -> f64 {
    a[0].hypot(a[1]).hypot(a[2])
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn greville(knots: &[f64], degree: usize, n: usize, domain: [f64; 2]) -> Vec<f64> {
    (0..n)
        .map(|i| {
            (knots[i + 1..=i + degree].iter().sum::<f64>() / degree as f64 - domain[0])
                / (domain[1] - domain[0])
        })
        .collect()
}
pub(super) fn candidates(c: &Curve, s: &Surface) -> Result<(Vec<Curve>, usize)> {
    if s.periodic_u || s.periodic_v || s.weights.iter().flatten().any(|w| *w != s.weights[0][0]) {
        return Ok((vec![], 0));
    }
    let d = domains(s);
    let o = s.evaluate(d[0][0], d[1][0])?.point;
    let a = sub(s.evaluate(d[0][1], d[1][0])?.point, o);
    let b = sub(s.evaluate(d[0][0], d[1][1])?.point, o);
    let gu = greville(&s.knots_u, s.degree_u, s.control_points.len(), d[0]);
    let gv = greville(&s.knots_v, s.degree_v, s.control_points[0].len(), d[1]);
    let scale = s
        .control_points
        .iter()
        .flatten()
        .flatten()
        .map(|x| x.abs())
        .fold(1f64, f64::max);
    let mut bases = vec![
        [[1., 0., 0.], [0., 1., 0.]],
        [[1., 0., 0.], [0., 0., 1.]],
        [[0., 1., 0.], [0., 0., 1.]],
    ];
    let mut height = [0.; 3];
    for (i, row) in s.control_points.iter().enumerate() {
        for (j, p) in row.iter().enumerate() {
            let r = std::array::from_fn(|k| p[k] - o[k] - a[k] * gu[i] - b[k] * gv[j]);
            if norm(r) > norm(height) {
                height = r;
            }
        }
    }
    let height_norm = norm(height);
    if height_norm.is_finite() && height_norm > 1024. * f64::EPSILON * scale {
        let n = height.map(|x| x / height_norm);
        let mut x = sub(a, n.map(|v| v * dot(a, n)));
        if norm(x) <= 1e-10 * norm(a) {
            x = sub(b, n.map(|v| v * dot(b, n)));
        }
        let length = norm(x);
        if length.is_finite() && length > 0. {
            let x = x.map(|v| v / length);
            bases.push([x, cross(n, x)]);
        }
    }
    let mut proposals = Vec::new();
    for basis in bases {
        let u = basis.map(|e| dot(e, a));
        let v = basis.map(|e| dot(e, b));
        let det = u[0] * v[1] - u[1] * v[0];
        if !det.is_finite() || det.abs() <= 1e-14 * u[0].hypot(u[1]) * v[0].hypot(v[1]) {
            continue;
        }
        let okay = s.control_points.iter().enumerate().all(|(i, row)| {
            row.iter().enumerate().all(|(j, p)| {
                let q = sub([p[0], p[1], p[2]], o);
                (0..2).all(|k| {
                    (dot(basis[k], q) - u[k] * gu[i] - v[k] * gv[j]).abs()
                        <= 128. * f64::EPSILON * scale
                })
            })
        });
        if !okay {
            continue;
        }
        let mut pc = c.clone();
        pc.control_points = c
            .control_points
            .iter()
            .map(|p| {
                let delta = sub([p[0], p[1], p[2]], o);
                let q = basis.map(|e| dot(e, delta));
                let x = (q[0] * v[1] - q[1] * v[0]) / det;
                let y = (u[0] * q[1] - u[1] * q[0]) / det;
                vec![
                    d[0][0] + x * (d[0][1] - d[0][0]),
                    d[1][0] + y * (d[1][1] - d[1][0]),
                ]
            })
            .collect();
        if pc.validate().is_ok() && !proposals.contains(&pc) {
            proposals.push(pc);
        }
    }
    Ok((proposals, 3))
}

/// Candidate-only basis reduction. Every pole must satisfy the exact authored
/// affine progression; small nonzero curvature is never treated as roundoff.
pub(super) fn compact_line(c: &Curve) -> Option<Curve> {
    use num_rational::BigRational as Q;
    if c.periodic
        || !(2..=32).contains(&c.degree)
        || c.control_points.len() != c.degree + 1
        || c.weights.iter().any(|w| *w != c.weights[0])
    {
        return None;
    }
    let n = c.degree;
    let d = c.domain();
    if c.knots[..=n].iter().any(|t| *t != d[0]) || c.knots[n + 1..].iter().any(|t| *t != d[1]) {
        return None;
    }
    for (i, p) in c.control_points.iter().enumerate() {
        for (k, &x) in p.iter().enumerate() {
            let value = Q::from_float(x)? * Q::from_integer(n.into());
            let expected = Q::from_float(c.control_points[0][k])? * Q::from_integer((n - i).into())
                + Q::from_float(c.control_points[n][k])? * Q::from_integer(i.into());
            if value != expected {
                return None;
            }
        }
    }
    Some(Curve {
        degree: 1,
        knots: vec![d[0], d[0], d[1], d[1]],
        control_points: vec![c.control_points[0].clone(), c.control_points[n].clone()],
        weights: vec![1.; 2],
        periodic: false,
    })
}
