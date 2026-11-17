//! Outward bounds on the signed area of projected planar face contours.
use super::planar_trim_projection::ProjectedTrimmedFace;
use crate::{Result, check, distance_bounds::Interval, trim::planar_area};
use num_rational::BigRational as Q;
use num_traits::ToPrimitive;

#[derive(Clone, Debug)]
pub struct PlanarFaceAreaReport {
    pub signed_area: [f64; 2],
    pub converged: bool,
    pub cells: usize,
}

/// Integrate the signed boundary area and convert to physical plane area
/// using the exact affine inverse. Orient holes opposite their outer loop.
/// The sign follows the retained coordinate order. The interval encloses
/// the Green integral, so interpreting it as material area requires valid
/// simple contours and the intended winding. Tolerance applies to the
/// returned interval width in squared model units.
pub fn measure_projected_face_area(
    face: &ProjectedTrimmedFace,
    tolerance: f64,
    max_cells: usize,
) -> Result<PlanarFaceAreaReport> {
    check(
        tolerance.is_finite()
            && tolerance > 0.
            && (1..=100_000).contains(&max_cells)
            && (1..=256).contains(&face.loops.len()),
        "Projected area needs contours, a positive tolerance and a bounded cell budget",
    )?;
    let (scale, _) = plane_metric(&face.projection.slopes)?;

    // Validate before indexing knot vectors and reserve enough cells to
    // cover all unprocessed contours even if the first needs refinement.
    let mut minimum = Vec::with_capacity(face.loops.len());
    for curves in &face.loops {
        check(
            (1..=256).contains(&curves.len()),
            "Projected area contour size is out of bounds",
        )?;
        let mut spans = 0;
        for curve in curves {
            curve.validate()?;
            spans += (curve.degree..curve.control_points.len())
                .filter(|&i| curve.knots[i] < curve.knots[i + 1])
                .count();
        }
        minimum.push(spans);
    }
    let mut reserved: usize = minimum.iter().sum();
    check(
        reserved <= max_cells,
        "Projected area cell budget must cover all original spans",
    )?;
    let per_loop = tolerance / scale.hi / face.loops.len() as f64;
    check(
        per_loop > 0.,
        "Projected area tolerance cannot be resolved in the chart",
    )?;

    let mut area = Interval::point(0.);
    let mut cells = 0;
    for (curves, minimum) in face.loops.iter().zip(minimum) {
        reserved -= minimum;
        let report = planar_area::measure(curves, per_loop, max_cells - cells - reserved)?;
        let [lo, hi] = report.area_interval_mm2;
        area = area.add(Interval::new(lo, hi)?)?;
        cells += report.cells;
    }
    let physical = area.mul(scale)?;
    let converged = (physical.hi - physical.lo).next_up() <= tolerance;
    Ok(PlanarFaceAreaReport {
        signed_area: [physical.lo, physical.hi],
        converged,
        cells,
    })
}

pub(super) fn plane_metric(slopes: &[Q; 2]) -> Result<(Interval, [Interval; 2])> {
    let [u, v] = slopes.each_ref().map(rational_interval);
    let (u, v) = (u?, v?);
    let norm_squared = Interval::point(1.).add(u.mul(u)?)?.add(v.mul(v)?)?;
    let scale = Interval::new(
        norm_squared.lo.max(1.).sqrt().next_down(),
        norm_squared.hi.sqrt().next_up(),
    )?;
    Ok((scale, [u, v]))
}

pub(super) fn rational_interval(q: &Q) -> Result<Interval> {
    let mid = q
        .to_f64()
        .filter(|x| x.is_finite())
        .ok_or_else(|| crate::input("Projected plane slope is outside binary64 range"))?;
    let lo = mid.next_down();
    let hi = mid.next_up();
    let lo_q = Q::from_float(lo);
    let hi_q = Q::from_float(hi);
    check(
        lo_q.as_ref().is_some_and(|x| x <= q) && hi_q.as_ref().is_some_and(|x| q <= x),
        "Projected plane slope could not be enclosed",
    )?;
    Interval::new(lo, hi)
}
