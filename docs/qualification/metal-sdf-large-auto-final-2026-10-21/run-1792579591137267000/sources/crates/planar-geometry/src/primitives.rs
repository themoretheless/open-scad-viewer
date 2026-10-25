//! Sampled primitive contours. Winding and fill are resolved by `rings::normalize`.
use crate::{Result, check, limits, rings::Rings};

/// Axis-aligned rectangle with signed dimensions and optional centering.
/// A collapsed dimension produces no contour.
pub fn rectangle(size: [f64; 2], center: bool) -> Result<Rings> {
    let [x, y] = size;
    if x == 0. || y == 0. {
        return Ok(vec![]);
    }
    Ok(vec![rectangle_corners(size, center)?.to_vec()])
}

/// Authored rectangle corners, including collapsed edges for exact graph recording.
/// Consumers decide whether a collapsed contour represents empty geometry.
pub fn rectangle_corners(size: [f64; 2], center: bool) -> Result<[[f64; 2]; 4]> {
    check(
        size.into_iter().all(f64::is_finite),
        "Invalid rectangle size",
    )?;
    let [x, y] = size;
    let [a, b] = if center { [-x / 2., -y / 2.] } else { [0., 0.] };
    Ok([[a, b], [a + x, b], [a + x, b + y], [a, b + y]])
}

/// Regular sampled circle. Fractional segment counts keep the requested angular
/// step and truncate the point count, as in the legacy CAD API.
pub fn circle(radius: f64, segments: f64) -> Result<Rings> {
    check(segments.is_finite(), "Invalid circle segment count")?;
    let count = segments.max(0.).floor();
    if count < 3. {
        return Ok(vec![]);
    }
    check(
        count <= limits::PATH_SEGMENTS as f64,
        "Circle segment budget exceeded",
    )?;
    check(radius.is_finite(), "Invalid circle radius")?;
    let points = (0..count as usize)
        .map(|i| {
            let angle = i as f64 * 2. * std::f64::consts::PI / segments;
            [radius * angle.cos(), radius * angle.sin()]
        })
        .collect();
    Ok(vec![points])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rings;
    #[test]
    fn rectangle_keeps_signed_winding_and_centering() {
        let r = rectangle([-2., 3.], true).unwrap();
        assert_eq!(r[0], vec![[1., -1.5], [-1., -1.5], [-1., 1.5], [1., 1.5]]);
        assert_eq!(rings::area(&r[0]), -6.);
        assert!(rings::planar(&r, &vec![], "union").unwrap().is_empty());
        assert!(rectangle([0., 3.], false).unwrap().is_empty());
        assert_eq!(
            rectangle_corners([0., 3.], false).unwrap(),
            [[0., 0.], [0., 0.], [0., 3.], [0., 3.]]
        );
        assert!(rectangle_corners([f64::NAN, 3.], false).is_err());
    }
    #[test]
    fn circle_preserves_sampling_and_bounded_output() {
        let r = circle(-2., 4.).unwrap();
        assert_eq!(r[0].len(), 4);
        assert!((rings::area(&r[0]) - 8.).abs() < 1e-12);
        let fractional = circle(2., 3.5).unwrap();
        assert_eq!(fractional[0].len(), 3);
        assert!((fractional[0][1][0] - 2. * (2. * std::f64::consts::PI / 3.5).cos()).abs() < 1e-12);
        assert!(circle(2., 2.).unwrap().is_empty());
        assert!(circle(2., limits::PATH_SEGMENTS as f64 + 1.).is_err());
    }
}
