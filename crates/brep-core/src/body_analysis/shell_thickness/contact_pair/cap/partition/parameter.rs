//! Numerical parameter proposal only; complete original geometry admits the cut.
use super::require;
use crate::Result;
use nurbs_core::curve::Curve;
#[derive(Clone, Copy)]
pub struct Search {
    pub budget: usize,
    pub evaluations: usize,
}
fn distance(p: &[f64], q: [f64; 3]) -> f64 {
    (0..3).map(|i| (p[i] - q[i]).powi(2)).sum()
}
pub(super) fn propose(c: &Curve, point: [f64; 3], budget: usize) -> Result<(f64, usize)> {
    require(
        (17..=4096).contains(&budget) && point.iter().all(|x| x.is_finite()),
        "Choose finite vertex and 17..4096 parameter-proposal evaluations",
    )?;
    let d = c.domain();
    let mut best = (f64::INFINITY, d[0]);
    let mut used = 0;
    for i in 0..=16 {
        let t = if i == 16 {
            d[1]
        } else {
            d[0] + (d[1] - d[0]) * (i as f64 / 16.)
        };
        let p = c.evaluate(t)?;
        used += 1;
        let dist = distance(&p.point, point);
        if dist < best.0 {
            best = (dist, t);
        }
    }
    let mut t = best.1;
    while used < budget {
        let p = c.evaluate(t)?;
        used += 1;
        let dist = distance(&p.point, point);
        if dist < best.0 {
            best = (dist, t);
        }
        let Some(v) = p.d1 else { break };
        let denominator = v.iter().map(|x| x * x).sum::<f64>();
        let numerator = (0..3).map(|i| (p.point[i] - point[i]) * v[i]).sum::<f64>();
        if denominator <= 0. || !denominator.is_finite() || !numerator.is_finite() {
            break;
        }
        let next = (t - numerator / denominator).clamp(d[0], d[1]);
        if next == t {
            break;
        }
        t = next;
    }
    require(
        best.0.is_finite() && d[0] < best.1 && best.1 < d[1],
        "No interior cap-edge parameter proposal",
    )?;
    Ok((best.1, used))
}
