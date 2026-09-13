//! Bounded native arclength sampling of piecewise linear paths.
use super::{Result, Value, encode, field, input};
pub fn sample(v: Value) -> Result<Value> {
    let path: Vec<Vec<f64>> = field(&v, "path")?;
    let fractions: Vec<f64> = field(&v, "fractions")?;
    if !(2..=4096).contains(&path.len()) || fractions.len() > 4096 {
        return Err(input(
            "Path sampling requires 2–4096 points and at most 4096 samples.",
        ));
    }
    let dimension = path[0].len();
    if !(2..=3).contains(&dimension)
        || path
            .iter()
            .any(|p| p.len() != dimension || p.iter().any(|x| !x.is_finite()))
        || fractions.iter().any(|x| !x.is_finite())
    {
        return Err(input(
            "Path sampling requires finite points of matching dimension and finite fractions.",
        ));
    }
    let lengths: Vec<f64> = path
        .windows(2)
        .map(|p| (0..dimension).fold(0_f64, |length, k| length.hypot(p[1][k] - p[0][k])))
        .collect();
    let total: f64 = lengths.iter().sum();
    if !total.is_finite() || total < 1e-8 {
        return Err(input("Zero-length or nonfinite path."));
    }
    let mut out = Vec::with_capacity(fractions.len());
    for fraction in fractions {
        let mut distance = fraction.clamp(0., 1.) * total;
        let mut point = path.last().unwrap().clone();
        for (i, &length) in lengths.iter().enumerate() {
            if distance <= length || i == lengths.len() - 1 {
                let t = if length > 0. { distance / length } else { 0. };
                point = (0..dimension)
                    .map(|k| path[i][k] + (path[i + 1][k] - path[i][k]) * t)
                    .collect();
                break;
            }
            distance -= length;
        }
        if point.iter().any(|x| !x.is_finite()) {
            return Err(input("Path sampling exceeds finite numeric range."));
        }
        out.push(point);
    }
    encode(out)
}
