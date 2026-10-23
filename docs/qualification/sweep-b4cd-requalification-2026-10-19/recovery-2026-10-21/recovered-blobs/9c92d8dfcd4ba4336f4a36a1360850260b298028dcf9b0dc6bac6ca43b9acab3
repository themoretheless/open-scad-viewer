//! Point-driven trimming retains one side of an open curve without resampling it.
use crate::{Result, check, curve::Curve, foundation};
#[path = "trim_point/serialization.rs"]
#[cfg(feature = "codec")]
mod serialization;
#[cfg(feature = "codec")]
pub use serialization::{trim_at_point, trim_at_screen_point};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectionSpace {
    WorldMillimeters,
    CssPixels,
}

#[derive(Debug)]
pub struct PointTrim {
    pub curve: Curve,
    pub parameter: f64,
    pub parameter_interval: [f64; 2],
    pub point: Vec<f64>,
    pub distance_upper: f64,
    pub kept_domain: [f64; 2],
    pub projection: foundation::CurveProjection,
    pub projection_space: ProjectionSpace,
}

pub fn trim_at_point_report(
    curve: &Curve,
    point: &[f64],
    keep: &str,
    max_distance: f64,
) -> Result<PointTrim> {
    trim_with_projection(curve, curve, point, keep, max_distance)
}

/// Orthographic world-to-CSS-pixel affine rows; the selected parameter trims the original curve.
pub fn trim_at_screen_point_report(
    curve: &Curve,
    point: &[f64],
    matrix: &[[f64; 4]; 2],
    keep: &str,
    radius: f64,
) -> Result<PointTrim> {
    curve.validate()?;
    check(
        point.len() == 2 && point.iter().all(|x| x.is_finite()),
        "Screen trim requires a finite CSS-pixel point",
    )?;
    check(
        matrix.iter().flatten().all(|x| x.is_finite()),
        "Screen projection must be finite",
    )?;
    check(
        matches!(curve.control_points[0].len(), 2 | 3),
        "Screen trim requires a 2D or 3D curve",
    )?;
    let u = matrix[0];
    let v = matrix[1];
    let cross = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    check(
        cross.iter().all(|x| x.is_finite()) && cross.iter().any(|x| *x != 0.),
        "Screen projection axes must be independent",
    )?;
    let mut projected = curve.clone();
    projected.control_points = curve
        .control_points
        .iter()
        .map(|p| {
            matrix
                .iter()
                .map(|row| {
                    row[3]
                        + row[0] * p[0]
                        + row[1] * p[1]
                        + row[2] * p.get(2).copied().unwrap_or(0.)
                })
                .collect()
        })
        .collect();
    let mut result = trim_with_projection(curve, &projected, point, keep, radius)?;
    result.projection_space = ProjectionSpace::CssPixels;
    Ok(result)
}

fn trim_with_projection(
    curve: &Curve,
    projected: &Curve,
    point: &[f64],
    keep: &str,
    max_distance: f64,
) -> Result<PointTrim> {
    curve.validate()?;
    check(
        !curve.periodic,
        "Point trim requires a non-periodic open curve",
    )?;
    check(
        matches!(keep, "start" | "end"),
        "Choose which curve endpoint to retain",
    )?;
    check(
        max_distance.is_finite() && max_distance > 0.,
        "Point trim requires a positive finite capture distance",
    )?;
    let tolerance = cad_predicates::ToleranceContext::default_valid();
    let [a, b] = curve.domain();
    let first = curve.evaluate(a)?.point;
    let last = curve.evaluate(b)?.point;
    let separation = first
        .iter()
        .zip(&last)
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f64>()
        .sqrt();
    check(
        separation > tolerance.spatial_bounds().absolute_mm,
        "Point trim requires an open curve with distinct endpoints",
    )?;
    let projection = foundation::project_curve_report(projected, point, Some(tolerance))?;
    check(
        projection.status == foundation::ProjectionStatus::Unique,
        "Point trim projection is ambiguous or unresolved; choose another point",
    )?;
    let winner = projection
        .winner_index
        .ok_or_else(|| crate::input("Unique projection has no winning candidate"))?;
    let candidate = &projection.candidates[winner];
    let distance = candidate.distance_upper;
    check(
        distance <= max_distance,
        "Point is outside the curve capture distance",
    )?;
    let lo = candidate.parameter_interval[0];
    let hi = candidate.parameter_interval[1];
    let floor = projection.tolerance.parametric_bounds().floor;
    check(
        lo > a + floor && hi < b - floor,
        "Choose a point strictly inside the curve, away from its endpoints",
    )?;
    let parameter = lo + (hi - lo) * 0.5;
    let bounds = if keep == "start" {
        [a, parameter]
    } else {
        [parameter, b]
    };
    let trimmed = curve.trim(bounds[0], bounds[1])?;
    let cut_point = curve.evaluate(parameter)?.point;
    Ok(PointTrim {
        curve: trimmed,
        parameter,
        parameter_interval: [lo, hi],
        point: cut_point,
        distance_upper: distance,
        kept_domain: bounds,
        projection,
        projection_space: ProjectionSpace::WorldMillimeters,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "codec")]
    use value_codec::{Serialize, json};

    #[test]
    #[cfg(feature = "codec")]
    fn trims_both_sides_of_a_rational_line_without_changing_the_source() {
        let curve = Curve {
            degree: 1,
            knots: vec![2., 2., 6., 6.],
            control_points: vec![vec![0., 0., 0.], vec![12., 0., 0.]],
            weights: vec![1., 3.],
            periodic: false,
        };
        let before = curve.to_value();
        for keep in ["start", "end"] {
            let result = trim_at_point(&curve, &[3., 0.01, 0.], keep, 0.02).unwrap();
            let trimmed: Curve = value_codec::from_value(result["curve"].clone()).unwrap();
            let bounds = trimmed.domain();
            let cut = trimmed
                .evaluate(if keep == "start" {
                    bounds[1]
                } else {
                    bounds[0]
                })
                .unwrap()
                .point;
            assert!((cut[0] - 3.).abs() < 1e-10);
            assert!(cut[1].abs() < 1e-12);
            for i in 0..=32 {
                let t = bounds[0] + (bounds[1] - bounds[0]) * i as f64 / 32.;
                let a = trimmed.evaluate(t).unwrap().point;
                let b = curve.evaluate(t).unwrap().point;
                assert!(a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-10));
            }
        }
        assert_eq!(curve.to_value(), before);
    }

    #[test]
    #[cfg(feature = "codec")]
    fn retains_the_exact_rational_arc() {
        let curve = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![10., 0.], vec![10., 10.], vec![0., 10.]],
            weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
            periodic: false,
        };
        let result = trim_at_point(&curve, &[8., 6.], "end", 0.01).unwrap();
        let trimmed: Curve = value_codec::from_value(result["curve"].clone()).unwrap();
        let [a, b] = trimmed.domain();
        assert!((trimmed.evaluate(a).unwrap().point[0] - 8.).abs() < 1e-7);
        for i in 0..=32 {
            let p = trimmed
                .evaluate(a + (b - a) * i as f64 / 32.)
                .unwrap()
                .point;
            assert!((p[0].hypot(p[1]) - 10.).abs() < 1e-10);
        }
    }

    #[test]
    #[cfg(feature = "transport")]
    fn dispatch_returns_a_trim_report_and_preserves_the_cut_interval() {
        let curve = Curve::from_polyline(vec![vec![0., 0.], vec![10., 0.]]).unwrap();
        let result=crate::dispatch(json!({"op":"curve_trim_point","curve":curve,"point":[4.,0.],"keep":"start","maxDistance":0.01})).unwrap();
        assert_eq!(result["projection"]["status"], "unique");
        let parameter = result["parameter"].as_f64().unwrap();
        assert!(result["parameterInterval"][0].as_f64().unwrap() <= parameter);
        assert!(result["parameterInterval"][1].as_f64().unwrap() >= parameter);
        assert!((result["point"][0].as_f64().unwrap() - 4.).abs() < 1e-10);
    }

    #[test]
    #[cfg(feature = "codec")]
    fn refuses_ambiguous_distant_endpoint_and_closed_inputs() {
        let curve = Curve::from_polyline(vec![vec![-1., 0.], vec![0., 1.], vec![1., 0.]]).unwrap();
        assert!(trim_at_point(&curve, &[0., 0.], "start", 2.).is_err());
        assert!(trim_at_point(&curve, &[-1., 0.], "start", 0.1).is_err());
        assert!(trim_at_point(&curve, &[0., 100.], "end", 0.1).is_err());
        assert!(trim_at_point(&curve, &[0., 1.], "other", 0.1).is_err());
        assert!(trim_at_point(&curve, &[0., 1.], "start", f64::NAN).is_err());
        let closed = Curve::from_polyline(vec![vec![0., 0.], vec![1., 1.], vec![0., 0.]]).unwrap();
        assert!(trim_at_point(&closed, &[0.5, 0.5], "start", 0.1).is_err());
    }
}

#[cfg(test)]
mod screen_tests {
    use super::*;
    #[test]
    #[cfg(feature = "codec")]
    fn screen_pick_trims_original_depth_and_uses_pixel_capture_distance() {
        let curve = Curve::from_polyline(vec![vec![0., 0., 10.], vec![10., 0., 20.]]).unwrap();
        let matrix = [[2., 0., 0., 100.], [0., 3., 0., 200.]];
        let result = trim_at_screen_point(&curve, &[108., 201.], &matrix, "end", 2.).unwrap();
        let point: Vec<f64> = value_codec::from_value(result["point"].clone()).unwrap();
        for (a, b) in point.iter().zip([4., 0., 14.]) {
            assert!((a - b).abs() < 1e-10);
        }
        assert!(result["distanceUpperPx"].as_f64().unwrap() <= 2.);
        assert!(result.get("distanceUpperMm").is_none());
        assert_eq!(result["projectionSpace"], "css-pixels");
        assert!(trim_at_screen_point(&curve, &[108., 204.], &matrix, "end", 2.).is_err());
        let trimmed: Curve = value_codec::from_value(result["curve"].clone()).unwrap();
        assert_eq!(
            trimmed.evaluate(trimmed.domain()[1]).unwrap().point,
            vec![10., 0., 20.]
        );
    }
    #[test]
    #[cfg(feature = "codec")]
    fn refuses_ambiguous_screen_depth_and_degenerate_projection() {
        let curve =
            Curve::from_polyline(vec![vec![-1., 0., 0.], vec![0., 1., 1.], vec![1., 0., 2.]])
                .unwrap();
        let matrix = [[1., 0., 0., 0.], [0., 1., 0., 0.]];
        assert!(trim_at_screen_point(&curve, &[0., 0.], &matrix, "start", 2.).is_err());
        assert!(
            trim_at_screen_point(
                &curve,
                &[0., 0.],
                &[[1., 0., 0., 0.], [2., 0., 0., 0.]],
                "start",
                2.
            )
            .is_err()
        );
        let depth = Curve::from_polyline(vec![vec![0., 0., 0.], vec![0., 0., 10.]]).unwrap();
        assert!(trim_at_screen_point(&depth, &[0., 0.], &matrix, "end", 2.).is_err());
    }
}

#[cfg(test)]
mod subdivision_regression {
    use super::*;
    #[test]
    fn retains_stationary_root_on_subdivision_boundary_for_tilted_arc() {
        let curve = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![10., 0., 10.], vec![10., 10., 15.], vec![0., 10., 15.]],
            weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
            periodic: false,
        };
        let point = curve.evaluate(0.5).unwrap().point;
        let result = trim_at_point_report(&curve, &point, "start", 0.01).unwrap();
        assert!((result.parameter - 0.5).abs() < 1e-12);
        assert!(result.distance_upper < 1e-10);
        let matrix = [[2., 0., 0., 100.], [0., 3., 0.5, 200.]];
        let screen = [100. + 2. * point[0], 200. + 3. * point[1] + 0.5 * point[2]];
        let result = trim_at_screen_point_report(&curve, &screen, &matrix, "end", 2.).unwrap();
        assert!((result.parameter - 0.5).abs() < 1e-12);
    }
}
