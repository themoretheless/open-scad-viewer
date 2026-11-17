//! Certified physical arc lengths of planar face boundaries.
use super::{
    curve_measure::length_under_plane_metric, exact_affine_hull::validate_projection_axes,
    planar_face_area::rational_interval, planar_trim_projection::ProjectedTrimmedFace,
};
use crate::{Result, check, distance_bounds::Interval, trim::trim_domain::exact_loop_joins};

#[derive(Clone, Debug)]
pub struct PlanarFacePerimeterReport {
    pub perimeter: [f64; 2],
    /// Length bounds in contour order, regardless of winding.
    pub contours: Vec<[f64; 2]>,
    pub within_tolerance: bool,
    pub cells: usize,
}

/// Bound XYZ boundary length from original rational projected trim curves.
/// The affine inverse is applied to interval jets and requires no stored
/// binary64 3D poles. All contours add length, including holes. Tolerance
/// refers to the total enclosure width in model units.
pub fn bound_projected_face_perimeter(
    face: &ProjectedTrimmedFace,
    tolerance: f64,
    max_cells: usize,
) -> Result<PlanarFacePerimeterReport> {
    validate_projection_axes(&face.projection)?;
    check(
        (1..=256).contains(&face.loops.len())
            && (1..=100_000).contains(&max_cells)
            && tolerance.is_finite()
            && tolerance > 0.,
        "Planar perimeter needs closed contours, positive tolerance and bounded work",
    )?;
    let slopes = [
        rational_interval(&face.projection.slopes[0])?,
        rational_interval(&face.projection.slopes[1])?,
    ];
    let mut reserved = 0;
    let mut count = 0;
    for contour in &face.loops {
        check(
            exact_loop_joins(contour)? == Some(true),
            "Planar perimeter needs exact closed joins",
        )?;
        count += contour.len();
        for curve in contour {
            reserved += active_spans(curve);
        }
        check(
            reserved <= max_cells,
            "Perimeter budget cannot cover all original spans",
        )?;
    }
    let per_curve = tolerance / (2. * count as f64);
    check(
        per_curve > 0.,
        "Perimeter tolerance is below parameter precision",
    )?;
    let mut total = Interval::point(0.);
    let mut contours = Vec::with_capacity(face.loops.len());
    let mut cells = 0;
    for contour in &face.loops {
        let mut length = Interval::point(0.);
        for curve in contour {
            reserved -= active_spans(curve);
            let report =
                length_under_plane_metric(curve, slopes, per_curve, max_cells - cells - reserved)?;
            length = length.add(Interval::new(report.bounds[0], report.bounds[1])?)?;
            cells += report.cells;
        }
        contours.push([length.lo.max(0.), length.hi]);
        total = total.add(length)?;
    }
    let perimeter = [total.lo.max(0.), total.hi];
    Ok(PlanarFacePerimeterReport {
        perimeter,
        contours,
        cells,
        within_tolerance: (perimeter[1] - perimeter[0]).next_up() <= tolerance,
    })
}

fn active_spans(curve: &crate::curve::Curve) -> usize {
    curve.knots[curve.degree..=curve.control_points.len()]
        .windows(2)
        .filter(|pair| pair[0] < pair[1])
        .count()
}
