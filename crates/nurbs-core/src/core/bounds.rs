//! Control-point bounds shared by rational curves and surfaces.
#[path = "bounds/serialization.rs"]
#[cfg(feature = "codec")]
mod serialization;
use crate::{check, Result};

#[derive(Clone, Debug, PartialEq)]
pub struct Bounds {
    pub min: Vec<f64>,
    pub max: Vec<f64>,
}

/// Positive rational weights keep the geometry inside its control-point hull.
pub fn from_points(points: &[Vec<f64>]) -> Result<Bounds> {
    check(!points.is_empty(), "Bounds require at least one point.")?;
    let dimension = points[0].len();
    check(dimension > 0, "Bounds require nonempty coordinates.")?;
    check(
        points
            .iter()
            .all(|point| point.len() == dimension && point.iter().all(|x| x.is_finite())),
        "Bounds require finite points of the same dimension.",
    )?;
    let mut min = points[0].clone();
    let mut max = min.clone();
    for point in &points[1..] {
        for axis in 0..dimension {
            min[axis] = min[axis].min(point[axis]);
            max[axis] = max[axis].max(point[axis]);
        }
    }
    Ok(Bounds { min, max })
}

/// Axis-aligned union of 1..2048 finite boxes of the same dimension.
/// Degenerate boxes are valid. Inputs are validated before aggregation.
pub fn union(items: &[Bounds]) -> Result<Bounds> {
    check(
        !items.is_empty() && items.len() <= 2048,
        "Bounds union requires 1..2048 boxes",
    )?;
    let dimension = items[0].min.len();
    check(dimension > 0, "Bounds require nonempty coordinates")?;
    check(
        items.iter().all(|b| {
            b.min.len() == dimension
                && b.max.len() == dimension
                && b.min
                    .iter()
                    .zip(&b.max)
                    .all(|(lo, hi)| lo.is_finite() && hi.is_finite() && lo <= hi)
        }),
        "Bounds union requires finite ordered boxes of the same dimension",
    )?;
    let mut result = items[0].clone();
    for b in &items[1..] {
        for axis in 0..dimension {
            result.min[axis] = result.min[axis].min(b.min[axis]);
            result.max[axis] = result.max[axis].max(b.max[axis]);
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encloses_each_coordinate_in_two_and_three_dimensions() {
        for points in [
            vec![vec![-2., 8.], vec![4., -1.]],
            vec![vec![-2., 8., 0.], vec![4., -1., 3.]],
        ] {
            let result = from_points(&points).unwrap();
            for point in points {
                for (axis, coordinate) in point.iter().enumerate() {
                    assert!(result.min[axis] <= *coordinate && *coordinate <= result.max[axis]);
                }
            }
            assert_eq!(&result.min[..2], &[-2., -1.]);
            assert_eq!(&result.max[..2], &[4., 8.]);
        }
    }

    #[test]
    fn rejects_empty_mixed_and_nonfinite_points() {
        for points in [
            vec![],
            vec![vec![]],
            vec![vec![1.], vec![1., 2.]],
            vec![vec![f64::NAN]],
        ] {
            assert!(from_points(&points).is_err());
        }
    }
}
