//! Bounded arclength sampling of piecewise linear paths.
//!
//! Paths contain 2–4096 points of the same dimension (2D or 3D).
//! Up to 4096 finite fractions are clamped to `[0, 1]`.
//! ```
//! use geometry_ops::path_sampling;
//! let path = vec![vec![0., 0.], vec![3., 0.], vec![3., 4.]];
//! let sampled = path_sampling::sample(&path, &[0.5]).unwrap();
//! assert_eq!(sampled, vec![vec![3., 0.5]]);
//! ```
use crate::{Result, fail as input};
pub fn sample(path: &[Vec<f64>], fractions: &[f64]) -> Result<Vec<Vec<f64>>> {
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
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn samples_by_arclength_and_clamps_outside_fractions() {
        let path = vec![vec![0., 0.], vec![0., 0.], vec![3., 0.], vec![3., 4.]];
        assert_eq!(
            sample(&path, &[-1., 3. / 7., 5. / 7., 2.]).unwrap(),
            vec![vec![0., 0.], vec![3., 0.], vec![3., 2.], vec![3., 4.]]
        );
        assert!(sample(&[vec![0., 0.], vec![1., 0., 0.]], &[0.5]).is_err());
    }
}
