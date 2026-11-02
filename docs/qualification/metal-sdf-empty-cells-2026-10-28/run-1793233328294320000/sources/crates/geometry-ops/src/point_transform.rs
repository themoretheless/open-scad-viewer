//! Legacy mixed 2D/3D point transforms around the arithmetic centroid.
use crate::{Result, fail};
fn finite(v: impl IntoIterator<Item = f64>) -> Result<()> {
    if v.into_iter().all(f64::is_finite) {
        Ok(())
    } else {
        Err(fail("Sketch geometry exceeds finite numeric range."))
    }
}
pub fn transform_points(
    points: &[Vec<f64>],
    delta: &[f64],
    angle: f64,
    scale: f64,
) -> Result<Vec<Vec<f64>>> {
    if points.is_empty()
        || points.len() > 300000
        || !(2..=3).contains(&delta.len())
        || !angle.is_finite()
        || !scale.is_finite()
        || scale <= 0.
    {
        return Err(fail("Invalid transform."));
    }
    finite(delta.iter().copied())?;
    for p in points {
        if !(2..=3).contains(&p.len()) {
            return Err(fail("Transform requires 2D or 3D points."));
        }
        finite(p.iter().copied())?;
    }
    // Divide before summation so a representable centroid does not overflow merely
    // because the unscaled coordinate sum exceeds binary64 range.
    let center: [f64; 3] = std::array::from_fn(|k| {
        points
            .iter()
            .map(|p| p.get(k).copied().unwrap_or(0.) / points.len() as f64)
            .sum()
    });
    finite(center)?;
    let (s, c) = angle.to_radians().sin_cos();
    let mut result = Vec::with_capacity(points.len());
    for p in points {
        let x = (p[0] - center[0]) * scale;
        let y = (p[1] - center[1]) * scale;
        let mut q = vec![
            center[0] + c * x - s * y + delta[0],
            center[1] + s * x + c * y + delta[1],
        ];
        if p.len() == 3 {
            q.push(center[2] + (p[2] - center[2]) * scale + delta.get(2).copied().unwrap_or(0.));
        }
        finite(q.iter().copied())?;
        result.push(q);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mixed_dimensions_and_overflow_admission() {
        let p = vec![vec![0., 0.], vec![2., 0., 4.]];
        let q = transform_points(&p, &[1., 2., 3.], 0., 2.).unwrap();
        assert_eq!(q, vec![vec![0., 2.], vec![4., 2., 9.]]);
        assert!(transform_points(&p, &[0., 0.], f64::NAN, 1.).is_err());
    }
}
