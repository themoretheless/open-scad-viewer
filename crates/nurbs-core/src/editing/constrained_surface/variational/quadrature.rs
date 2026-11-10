//! Bounded Gauss integration over the union of ORIGINAL and trial knot spans.
use super::*;
fn gauss(n: usize) -> Result<Vec<(f64, f64)>> {
    let mut out = Vec::new();
    for i in 0..n {
        let mut x = (std::f64::consts::PI * (i as f64 + 0.75) / (n as f64 + 0.5)).cos();
        let mut converged = false;
        for _ in 0..32 {
            let (value, derivative) = legendre(n, x);
            let next = x - value / derivative;
            if !next.is_finite() {
                return Err(error("Gauss node construction overflow"));
            }
            if (next - x).abs() < 2e-15 {
                x = next;
                converged = true;
                break;
            }
            x = next;
        }
        let (_, derivative) = legendre(n, x);
        let weight = 2. / ((1. - x * x) * derivative * derivative);
        if !converged || !weight.is_finite() || weight <= 0. || x.abs() >= 1. {
            return Err(error("Gauss nodes numerically unresolved"));
        }
        out.push((x, weight));
    }
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    Ok(out)
}
fn legendre(n: usize, x: f64) -> (f64, f64) {
    let (mut prev, mut current) = (1., x);
    for k in 2..=n {
        let next = ((2 * k - 1) as f64 * x * current - (k - 1) as f64 * prev) / k as f64;
        prev = current;
        current = next;
    }
    (current, n as f64 * (x * current - prev) / (x * x - 1.))
}
fn axis(knots: &[f64], o: Options) -> Result<Vec<(f64, f64)>> {
    let nodes = gauss(o.quadrature_order)?;
    let mut result = Vec::new();
    for w in knots.windows(2) {
        if w[0] >= w[1] {
            continue;
        }
        for i in 0..o.subdivisions {
            let a = w[0] + (w[1] - w[0]) * i as f64 / o.subdivisions as f64;
            let b = w[0] + (w[1] - w[0]) * (i + 1) as f64 / o.subdivisions as f64;
            let half = (b - a) * 0.5;
            for &(x, weight) in &nodes {
                let t = a + half * (x + 1.);
                let weight = half * weight;
                if !(t > a && t < b) || !weight.is_finite() || weight <= 0. {
                    return Err(error(
                        "Original knot span has no resolved interior quadrature",
                    ));
                }
                if result.len() == o.max_sites {
                    return Err(error("Original knot-span quadrature allowance exhausted"));
                }
                result.push((t, weight));
            }
        }
    }
    Ok(result)
}
fn breaks(a: &[f64], pa: usize, na: usize, b: &[f64], pb: usize, nb: usize) -> Vec<f64> {
    let mut k = a[pa..=na]
        .iter()
        .chain(&b[pb..=nb])
        .copied()
        .collect::<Vec<_>>();
    k.sort_by(f64::total_cmp);
    k.dedup_by(|a, b| *a == *b);
    k
}
pub(super) fn sites(s: &Surface, reference: &Surface, o: Options) -> Result<Vec<([f64; 2], f64)>> {
    let u = axis(
        &breaks(
            &s.knots_u,
            s.degree_u,
            s.control_points.len(),
            &reference.knots_u,
            reference.degree_u,
            reference.control_points.len(),
        ),
        o,
    )?;
    let v = axis(
        &breaks(
            &s.knots_v,
            s.degree_v,
            s.control_points[0].len(),
            &reference.knots_v,
            reference.degree_v,
            reference.control_points[0].len(),
        ),
        o,
    )?;
    if u.is_empty() || v.is_empty() || u.len().saturating_mul(v.len()) > o.max_sites {
        return Err(error(
            "Original/trial tensor quadrature allowance exhausted",
        ));
    }
    Ok(u.iter()
        .flat_map(|&(u, wu)| v.iter().map(move |&(v, wv)| ([u, v], wu * wv)))
        .collect())
}
