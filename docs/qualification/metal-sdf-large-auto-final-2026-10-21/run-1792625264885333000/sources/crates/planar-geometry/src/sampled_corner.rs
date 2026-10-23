//! Bounded sampled corner editing for legacy closed polylines.
use crate::{Result, error};
type P = [f64; 2];
fn cross(a: P, b: P, c: P) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}
pub fn validate(points: &[P]) -> Result<()> {
    if points.len() < 3
        || points.len() > 512
        || points
            .iter()
            .flatten()
            .any(|v| !v.is_finite() || v.abs() > 1e6)
    {
        return Err(error("Invalid contour or point budget exceeded."));
    }
    let n = points.len();
    for i in 0..n {
        let (a, b) = (points[i], points[(i + 1) % n]);
        if (a[0] - b[0]).hypot(a[1] - b[1]) < 1e-8 {
            return Err(error("Contour has a zero-length edge."));
        }
        for j in i + 2..n {
            if i == 0 && j == n - 1 {
                continue;
            }
            let (c, d) = (points[j], points[(j + 1) % n]);
            if a[0].max(b[0]) + 1e-9 < c[0].min(d[0])
                || c[0].max(d[0]) + 1e-9 < a[0].min(b[0])
                || a[1].max(b[1]) + 1e-9 < c[1].min(d[1])
                || c[1].max(d[1]) + 1e-9 < a[1].min(b[1])
            {
                continue;
            }
            if cross(a, b, c) * cross(a, b, d) <= 0. && cross(c, d, a) * cross(c, d, b) <= 0. {
                return Err(error(
                    "Radius creates a self-intersection. Use a smaller radius.",
                ));
            }
        }
    }
    Ok(())
}
#[derive(Clone, Copy)]
pub enum Kind {
    Fillet,
    Dogear,
}
pub fn corner(points: &[P], vertex: usize, radius: f64, kind: Kind) -> Result<Vec<P>> {
    if vertex >= points.len() {
        return Err(error("Select a corner of a closed contour."));
    }
    if !radius.is_finite() || !(0.01..=1e6).contains(&radius) {
        return Err(error("Radius must be at least 0.01 mm."));
    }
    validate(points)?;
    let n = points.len();
    let (v, prev, next) = (
        points[vertex],
        points[(vertex + n - 1) % n],
        points[(vertex + 1) % n],
    );
    let la = (prev[0] - v[0]).hypot(prev[1] - v[1]);
    let lb = (next[0] - v[0]).hypot(next[1] - v[1]);
    let u = [(prev[0] - v[0]) / la, (prev[1] - v[1]) / la];
    let w = [(next[0] - v[0]) / lb, (next[1] - v[1]) / lb];
    let theta = (u[0] * w[0] + u[1] * w[1]).clamp(-1., 1.).acos();
    let pi = std::f64::consts::PI;
    let dogear = matches!(kind, Kind::Dogear);
    if theta < 0.01 || pi - theta < 0.01 {
        return Err(error("Select a non-collinear corner."));
    }
    if dogear && (theta - pi / 2.).abs() > 0.001 {
        return Err(error("DogEar requires a right-angle corner."));
    }
    let distance = if dogear {
        std::f64::consts::SQRT_2 * radius
    } else {
        radius / (theta / 2.).tan()
    };
    if distance >= la.min(lb) - 1e-8 {
        return Err(error(
            "Radius exceeds the adjacent edges. Use a smaller radius.",
        ));
    }
    let a = std::array::from_fn::<_, 2, _>(|k| v[k] + u[k] * distance);
    let b = std::array::from_fn::<_, 2, _>(|k| v[k] + w[k] * distance);
    let center = std::array::from_fn::<_, 2, _>(|k| {
        if dogear {
            (a[k] + b[k]) / 2.
        } else {
            v[k] + (u[k] + w[k]) * distance / (1. + theta.cos())
        }
    });
    let start = (a[1] - center[1]).atan2(a[0] - center[0]);
    let turn = cross(prev, v, next).signum();
    let sweep = turn * if dogear { pi } else { pi - theta };
    let segments = (sweep.abs() / (pi / if dogear { 72. } else { 36. })).ceil() as usize;
    let mut out = points[..vertex].to_vec();
    for i in 0..=segments {
        out.push(if i == 0 {
            a
        } else if i == segments {
            b
        } else {
            let angle = start + sweep * i as f64 / segments as f64;
            [
                center[0] + radius * angle.cos(),
                center[1] + radius * angle.sin(),
            ]
        });
    }
    out.extend_from_slice(&points[vertex + 1..]);
    validate(&out)?;
    Ok(out)
}
