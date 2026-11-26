//! Section bounds for an original curvature contour against a model plane.
use super::{CurvatureGapContourSpec, CurvatureGapGraph, certify_curvature_gap_world_polyline, *};
use crate::{distance_bounds::Interval as I, numerics::interval_vec3::norm};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CurvatureGapPlaneStatus {
    /// Opposite strict signs at the true endpoints prove a crossing.
    Crossing,
    /// Contact is not ruled out. This includes grazing and coincidence.
    Possible,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CurvatureGapPlaneEvent {
    pub segment: usize,
    /// Closed bounds in the absolute transverse knot clock.
    pub transverse: [f64; 2],
    pub status: CurvatureGapPlaneStatus,
}

#[derive(Clone, Debug)]
pub struct CurvatureGapPlaneAudit {
    pub graph: Option<CurvatureGapGraph>,
    /// All plane roots lie in these intervals. Empty proves no roots.
    pub events: Option<Vec<CurvatureGapPlaneEvent>>,
    pub work_used: u64,
    pub refusal: Option<Refusal>,
}

/// Localize the intersection of a certified world contour with
/// `dot(normal, point) = offset`. For each chord, the continuous world
/// error expands its affine plane value by `norm(normal) * error`.
/// Crossing certifies existence on that segment, without a root count.
pub fn section_curvature_gap_contour(
    spec: CurvatureGapContourSpec<'_>,
    normal: [f64; 3],
    offset: f64,
    max_work: u64,
) -> Result<CurvatureGapPlaneAudit> {
    check(
        normal.iter().all(|x| x.is_finite())
            && normal.iter().any(|&x| x != 0.)
            && offset.is_finite(),
        "Contour plane requires a finite nonzero normal and finite offset",
    )?;
    let world = certify_curvature_gap_world_polyline(
        spec.surface,
        spec.span,
        spec.axis,
        spec.level,
        spec.transverse,
        spec.root_depth,
        max_work,
    )?;
    let mut report = CurvatureGapPlaneAudit {
        graph: world.uv.graph,
        events: None,
        work_used: world.uv.work_used,
        refusal: world.uv.refusal,
    };
    let (Some(points), Some(errors)) = (world.points, world.world_error_upper) else {
        return Ok(report);
    };
    let normal = normal.map(I::point);
    let length = norm(normal)?;
    let signed = points
        .into_iter()
        .map(|point| {
            let mut value = I::point(-offset);
            for (x, n) in point.into_iter().zip(normal) {
                value = value.add(I::point(x).mul(n)?)?;
            }
            Ok(value)
        })
        .collect::<Result<Vec<_>>>()?;
    let mut events = Vec::new();
    for (i, (pair, error)) in signed.windows(2).zip(errors).enumerate() {
        let [a, b] = [pair[0], pair[1]];
        let radius = length.mul(I::point(error))?.hi;
        let Some([left, right]) = clip_below([a.lo, b.lo], radius)?
            .zip(clip_below([-a.hi, -b.hi], radius)?)
            .and_then(|(below, above)| {
                let range = [below[0].max(above[0]), below[1].min(above[1])];
                (range[0] <= range[1]).then_some(range)
            })
        else {
            continue;
        };
        let [start, end] = [spec.transverse[i], spec.transverse[i + 1]];
        let width = I::point(end).sub(I::point(start))?;
        let map = |t| I::point(start).add(I::point(t).mul(width)?);
        let transverse = [map(left)?.lo.max(start), map(right)?.hi.min(end)];
        let crossing = (a.lo > radius && b.hi < -radius) || (b.lo > radius && a.hi < -radius);
        events.push(CurvatureGapPlaneEvent {
            segment: i,
            transverse,
            status: if crossing {
                CurvatureGapPlaneStatus::Crossing
            } else {
                CurvatureGapPlaneStatus::Possible
            },
        });
    }
    report.events = Some(events);
    Ok(report)
}

// Enclose t in [0, 1] for which the affine function through a and b is <= limit.
fn clip_below([a, b]: [f64; 2], limit: f64) -> Result<Option<[f64; 2]>> {
    match (a <= limit, b <= limit) {
        (true, true) => Ok(Some([0., 1.])),
        (false, false) => Ok(None),
        _ => {
            let lo = I::point(a.min(b));
            let width = I::point(a.max(b)).sub(lo)?;
            if width.lo <= 0. {
                return Ok(Some([0., 1.]));
            }
            let cut = I::point(limit).sub(lo)?.div(width)?.hi.clamp(0., 1.);
            if a < b {
                Ok(Some([0., cut]))
            } else {
                let start = I::point(1.).sub(I::point(cut))?.lo.max(0.);
                Ok(Some([start, 1.]))
            }
        }
    }
}
