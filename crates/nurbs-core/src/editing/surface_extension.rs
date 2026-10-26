//! Tensor-product surface extension through analytic continuation of every
//! parameter line. The original trim is not inferred or changed here.
use crate::curve_extension::{Continuity, End, extend_curve};
use crate::surface::{Axis, Surface};
use crate::Result;
use std::cell::Cell;

#[derive(Clone, Debug)]
pub struct SurfaceExtensionReport {
    pub surface: Surface,
    pub achieved_continuity: Continuity,
    /// Conservative maximum of the underlying curve growth metrics.
    pub max_extrapolation_growth: f64,
    /// False when at least one parameter line exceeds the curve stability limit.
    pub stable: bool,
}

/// Extend one clamped, non-periodic surface axis by a physical arc-length
/// estimate. `Surface::edit_axis` rejects inconsistent parameter intervals.
pub fn extend_surface(
    source: &Surface,
    axis: Axis,
    end: End,
    length: f64,
    continuity: Continuity,
) -> Result<SurfaceExtensionReport> {
    let growth = Cell::new(0_f64);
    let stable = Cell::new(true);
    let surface = source.edit_axis(axis, |curve| {
        let report = extend_curve(curve, end, length, continuity)?;
        growth.set(growth.get().max(report.extrapolation_growth));
        stable.set(stable.get() && report.stable);
        Ok(report.curve)
    })?;
    Ok(SurfaceExtensionReport {
        surface,
        achieved_continuity: continuity,
        max_extrapolation_growth: growth.get(),
        stable: stable.get(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plane_axis_extension_reaches_the_requested_physical_boundary() {
        let source = crate::patches::plane([0.; 3], [2., 0., 0.], [0., 1., 0.]).unwrap();
        let report = extend_surface(&source, Axis::U, End::Stop, 0.5, Continuity::C2).unwrap();
        assert!(report.stable);
        assert_eq!(report.achieved_continuity, Continuity::C2);
        let end = report.surface.knots_u[report.surface.control_points.len()];
        assert!(end > 1.);
        for (u, v) in [(0., 0.), (1., 0.5), (end, 0.), (end, 1.)] {
            let point = report.surface.evaluate(u, v).unwrap().point;
            assert!((point[1] - v).abs() < 1e-12 && point[2].abs() < 1e-12);
        }
        let far = report.surface.evaluate(end, 0.5).unwrap().point;
        assert!((far[0] - 2.5).abs() < 1e-12);
    }

    #[test]
    fn oscillating_high_degree_extension_is_explicitly_unstable() {
        let degree = 20;
        let source = Surface {
            degree_u: degree,
            degree_v: 1,
            knots_u: [vec![0.; degree + 1], vec![1.; degree + 1]].concat(),
            knots_v: vec![0., 0., 1., 1.],
            control_points: (0..=degree)
                .map(|i| {
                    [0., 1.]
                        .map(|z| {
                            vec![
                                i as f64 / degree as f64,
                                if i % 2 == 0 { 0.5 } else { -0.5 },
                                z,
                            ]
                        })
                        .to_vec()
                })
                .collect(),
            weights: vec![vec![1.; 2]; degree + 1],
            periodic_u: false,
            periodic_v: false,
        };
        let report = extend_surface(&source, Axis::U, End::Stop, 0.001, Continuity::G2).unwrap();
        assert!(!report.stable);
        assert!(report.max_extrapolation_growth > 16.);
    }
}
