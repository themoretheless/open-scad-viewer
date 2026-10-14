//! Native degree-one interpolation and conservative curve approximation.
use super::{Curve, Result, ToleranceContext, context, next_up};
#[cfg(feature = "codec")]
pub mod serialization;
pub struct PolylineInterpolation {
    pub curve: Curve,
    pub tolerance: ToleranceContext,
}
pub struct CurveApproximation {
    pub curve: Curve,
    pub hausdorff_error_upper: f64,
    pub within_entity_tolerance: bool,
    pub tolerance: ToleranceContext,
}
/// Exact interpolation of the supplied data sites as a degree-one spline.
pub fn interpolate_polyline_report(
    points: Vec<Vec<f64>>,
    tolerance: Option<ToleranceContext>,
) -> Result<PolylineInterpolation> {
    let curve = Curve::from_polyline(points)?;
    let tolerance = context(tolerance);
    Ok(PolylineInterpolation { curve, tolerance })
}
/// Conservative piecewise-linear approximation bounded by each span hull diameter.
pub fn approximate_curve_report(
    curve: &Curve,
    tolerance: Option<ToleranceContext>,
) -> Result<CurveApproximation> {
    curve.validate()?;
    let tolerance = context(tolerance);
    let segments = curve.decompose()?;
    let mut points = Vec::new();
    let mut error: f64 = 0.;
    for (index, segment) in segments.iter().enumerate() {
        let c = segment.definition();
        let [a, b] = segment.domain();
        if index == 0 {
            points.push(c.evaluate(a)?.point);
        }
        points.push(c.evaluate(b)?.point);
        for first in &c.control_points {
            for second in &c.control_points {
                error = error.max(
                    first
                        .iter()
                        .zip(second)
                        .map(|(a, b)| (a - b) * (a - b))
                        .sum::<f64>()
                        .sqrt(),
                );
            }
        }
    }
    let approximation = Curve::from_polyline(points)?;
    let hausdorff_error_upper = next_up(error);
    let within_entity_tolerance =
        hausdorff_error_upper <= tolerance.entity_error_bounds().maximum_mm;
    Ok(CurveApproximation {
        curve: approximation,
        hausdorff_error_upper,
        within_entity_tolerance,
        tolerance,
    })
}
