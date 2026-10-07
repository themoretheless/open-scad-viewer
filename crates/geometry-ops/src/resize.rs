//! Axis scales from resolved size targets and aggregate geometric extents.
#[derive(Debug, Clone, PartialEq)]
pub struct Selection<const D: usize> {
    pub scales: [f64; D],
    pub invalid_axis: Option<usize>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub node: usize,
    pub axis: Option<usize>,
}
/// Extents of nonempty child bounds. Empty children are omitted by the adapter.
pub fn aggregate_extents<const D: usize>(bounds: &[([f64; D], [f64; D])]) -> [f64; D] {
    let mut min = [f64::INFINITY; D];
    let mut max = [f64::NEG_INFINITY; D];
    for (lower, upper) in bounds {
        for axis in 0..D {
            // Preserve Math.min/Math.max propagation for malformed bounds.
            min[axis] = if min[axis].is_nan() || lower[axis].is_nan() {
                f64::NAN
            } else {
                min[axis].min(lower[axis])
            };
            max[axis] = if max[axis].is_nan() || upper[axis].is_nan() {
                f64::NAN
            } else {
                max[axis].max(upper[axis])
            };
        }
    }
    std::array::from_fn(|axis| max[axis] - min[axis])
}
/// Missing or nonpositive targets leave the axis unconstrained. Automatic axes
/// inherit the largest authored ratio. A positive target needs positive extent.
pub fn resolve<const D: usize>(
    targets: [Option<f64>; D],
    extents: [Option<f64>; D],
    automatic: [bool; D],
) -> Selection<D> {
    let mut direct = [None; D];
    for axis in 0..D {
        if let Some(target) = targets[axis].filter(|v| v.is_finite() && *v > 0.) {
            let Some(extent) = extents[axis].filter(|v| v.is_finite() && *v > 0.) else {
                return Selection {
                    scales: [1.; D],
                    invalid_axis: Some(axis),
                };
            };
            direct[axis] = Some(target / extent);
        }
    }
    let aspect = direct
        .iter()
        .flatten()
        .copied()
        .reduce(f64::max)
        .unwrap_or(1.);
    Selection {
        scales: std::array::from_fn(|axis| {
            direct[axis].unwrap_or(if automatic[axis] { aspect } else { 1. })
        }),
        invalid_axis: None,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn aggregate_bounds_include_separation_and_propagate_invalid_axes() {
        assert_eq!(
            aggregate_extents(&[([5., -2.], [7., 1.]), ([-3., 4.], [0., 6.])]),
            [10., 8.]
        );
        assert_eq!(aggregate_extents::<2>(&[]), [f64::NEG_INFINITY; 2]);
        let extents = aggregate_extents(&[([f64::NAN, 0.], [1., 2.]), ([0., 1.], [4., 3.])]);
        assert!(extents[0].is_nan());
        assert_eq!(extents[1], 3.);
    }
    #[test]
    fn preserves_explicit_ratios_and_automatic_axes() {
        assert_eq!(
            resolve(
                [Some(10.), None, Some(3.)],
                [Some(5.), Some(6.), Some(6.)],
                [false, true, true]
            )
            .scales,
            [2., 2., 0.5]
        );
        assert_eq!(
            resolve([Some(10.), None], [Some(0.), None], [false, true]).invalid_axis,
            Some(0)
        );
        assert_eq!(
            resolve([None, Some(0.)], [None, None], [true, true]).scales,
            [1., 1.]
        );
        assert_eq!(
            resolve(
                [Some(f64::MAX), None],
                [Some(f64::MIN_POSITIVE), None],
                [false, true]
            )
            .scales,
            [f64::INFINITY; 2]
        );
    }
}

impl Diagnostic {
    pub fn message(&self) -> String {
        match self.axis {
            Some(axis) => {
                format!("resize cannot map a zero-width axis {axis} to a positive target")
            }
            None => "resize newsize must be a vector".to_string(),
        }
    }
}
