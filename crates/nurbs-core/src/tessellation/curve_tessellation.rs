//! Adaptive chords with conservative continuous original-curve error bounds.
use crate::{
    Result, check,
    curve::Curve,
    curve_distance::enclosure,
    distance_bounds::{Interval, box_distance},
    resource,
};
#[derive(Clone, Debug)]
pub struct Segment {
    pub domain: [f64; 2],
    pub points: [Vec<f64>; 2],
    pub error_upper: f64,
    pub within_tolerance: bool,
}
#[derive(Clone, Debug)]
pub struct Report {
    pub segments: Vec<Segment>,
    pub within_tolerance: bool,
    pub cells: usize,
    pub error_upper: f64,
}
/// Cover all active knot spans with chords. Bounds include evaluator rounding
/// by comparing the original interval image to the actual returned binary64
/// chord endpoints. Work/precision limits retain uncertified segments explicitly.
pub fn tessellate(curve: &Curve, tolerance: f64, max_cells: usize) -> Result<Report> {
    curve.validate()?;
    check(
        tolerance.is_finite() && tolerance > 0.,
        "Tessellation tolerance must be positive and finite",
    )?;
    check(
        (1..=100000).contains(&max_cells),
        "Tessellation needs 1..100000 cells",
    )?;
    let [a, b] = curve.domain();
    for &t in &curve.knots {
        if a < t && t < b {
            check(
                curve.knots.iter().filter(|&&k| k == t).count() <= curve.degree,
                "Continuous tessellation requires continuous interior knots",
            )?;
        }
    }
    let mut pending: Vec<_> = (curve.degree..curve.control_points.len())
        .filter(|&i| curve.knots[i] < curve.knots[i + 1])
        .map(|i| (i, [curve.knots[i], curve.knots[i + 1]]))
        .collect();
    if pending.len() > max_cells {
        return Err(resource(
            "Initial tessellation spans exceed the cell budget",
        ));
    }
    let mut cells = pending.len();
    let mut segments = Vec::new();
    while let Some((span, domain)) = pending.pop() {
        let points = [
            curve.evaluate(domain[0])?.point,
            curve.evaluate(domain[1])?.point,
        ];
        let chord = points[0]
            .iter()
            .zip(&points[1])
            .map(|(&x, &y)| Interval::new(x.min(y), x.max(y)))
            .collect::<Result<Vec<_>>>()?;
        let image = enclosure(curve, span, Interval::new(domain[0], domain[1])?)?;
        let error_upper = box_distance(&image, &chord)?.1;
        let m = domain[0] * 0.5 + domain[1] * 0.5;
        if error_upper > tolerance && cells + 2 <= max_cells && domain[0] < m && m < domain[1] {
            pending.push((span, [domain[0], m]));
            pending.push((span, [m, domain[1]]));
            cells += 2;
        } else {
            segments.push(Segment {
                domain,
                points,
                error_upper,
                within_tolerance: error_upper <= tolerance,
            });
        }
    }
    segments.sort_by(|a, b| a.domain[0].total_cmp(&b.domain[0]));
    Ok(Report {
        within_tolerance: segments.iter().all(|x| x.within_tolerance),
        error_upper: segments.iter().map(|x| x.error_upper).fold(0., f64::max),
        segments,
        cells,
    })
}
