//! Direct millimetre agreement between actual paired UV/world curves on an
//! original normal-offset source. UV error alone is never a world-error bound.
use crate::{
    Result, check, curve::Curve, curve_surface_agreement as curve_bounds,
    distance_bounds::Interval as I, offset_contact_tangent as tangent, surface::Surface,
    surface_offset,
};
pub(crate) mod fitting;
pub use fitting::{FitAttempt, FitLimits, FitReport, FitStop, fit};
#[derive(Clone, Copy)]
pub struct Limits {
    pub cells: usize,
    pub spans: usize,
}
pub struct Cell {
    pub interval: [f64; 2],
    pub error_upper_mm: Option<f64>,
    pub admitted: bool,
}
pub struct Report {
    pub cells: Vec<Cell>,
    pub visited: usize,
    pub lift_correspondence_proven: bool,
    pub error_upper_mm: Option<f64>,
    pub reason: &'static str,
}
fn bounds(curve: &Curve, r: [f64; 2]) -> Result<Vec<I>> {
    curve_bounds::curve_bounds(curve, I::new(r[0], r[1])?)
}
fn clipped(r: &[I], domain: [[f64; 2]; 2]) -> [[f64; 2]; 2] {
    // Used only after positive-weight control-hull admission proves the WHOLE
    // actual UV curve lies in the original chart. Clip outward enclosure
    // overshoot, never an unqualified candidate or original source coordinate.
    std::array::from_fn(|k| [r[k].lo.max(domain[k][0]), r[k].hi.min(domain[k][1])])
}
/// Bound world(t)-offset_source(uv(t)) on the COMPLETE literal common clock.
/// Actual rounded nets are used. Interval source/UV/world derivatives bound
/// the full residual, including BOTH UV chain-rule terms; a fresh interval
/// center anchor is only one part of that complete error bound.
///
/// This admits curve lift agreement, not root identity, UV sheet selection,
/// offset/source regularity, embedding, trim/material roles or topology.
pub fn certify(
    world: &Curve,
    uv: &Curve,
    source: &Surface,
    distance: f64,
    tolerance_mm: f64,
    limits: Limits,
) -> Result<Report> {
    world.validate()?;
    uv.validate()?;
    source.validate()?;
    check(
        world.control_points[0].len() == 3
            && uv.control_points[0].len() == 2
            && !world.periodic
            && !uv.periodic
            && world.domain() == uv.domain()
            && distance.is_finite()
            && tolerance_mm.is_finite()
            && tolerance_mm > 0.
            && (1..=100000).contains(&limits.cells)
            && (1..=100000).contains(&limits.spans),
        "Choose paired nonperiodic UV/world curves on one literal clock and bounded lift proof work",
    )?;
    let clock = world.domain();
    let mut out = Report {
        cells: vec![],
        visited: 0,
        lift_correspondence_proven: false,
        error_upper_mm: None,
        reason: "lift-work-limit",
    };
    let domain = [
        [
            source.knots_u[source.degree_u],
            source.knots_u[source.control_points.len()],
        ],
        [
            source.knots_v[source.degree_v],
            source.knots_v[source.control_points[0].len()],
        ],
    ];
    let admitted = uv.weights.iter().all(|w| *w > 0.)
        && uv
            .control_points
            .iter()
            .all(|p| (0..2).all(|k| p[k] >= domain[k][0] && p[k] <= domain[k][1]));
    if !admitted {
        out.cells.push(Cell {
            interval: clock,
            error_upper_mm: None,
            admitted: false,
        });
        out.reason = "whole-UV-original-support-unproved";
        return Ok(out);
    }
    let mut lifted_uv = uv.clone();
    for p in &mut lifted_uv.control_points {
        p.push(0.);
    }
    let u = super::lift(&lifted_uv)?;
    let w = super::lift(world)?;
    let footprint = clipped(&bounds(uv, clock)?, domain);
    if !tangent::differentiable(&u, [clock, [0., 0.]], 1)
        || !tangent::differentiable(&w, [clock, [0., 0.]], 1)
        || !tangent::differentiable(source, footprint, 1)
    {
        out.cells.push(Cell {
            interval: clock,
            error_upper_mm: None,
            admitted: false,
        });
        out.reason = "lift-source-or-curve-continuity-unproved";
        return Ok(out);
    }
    let mut queue = vec![clock];
    // The original C1 source has a continuous unit normal wherever its jets
    // certify a nonzero normal. All incident knot-side offset derivatives are
    // enclosed by jacobian_bounds. This continuous piecewise-C1 map is locally
    // Lipschitz; integrating those bounds remains valid across C1 source knots.
    let mut mismatch = false;
    while let Some(range) = queue.pop() {
        out.visited += 1;
        let mut cell = Cell {
            interval: range,
            error_upper_mm: None,
            admitted: false,
        };
        let p = clipped(&bounds(uv, range)?, domain);
        let source_jac = surface_offset::jacobian_bounds(source, p, distance, limits.spans)?;
        let uv_jac = surface_offset::jacobian_bounds(&u, [range, [0., 0.]], 0., limits.spans)?;
        let world_jac = surface_offset::jacobian_bounds(&w, [range, [0., 0.]], 0., limits.spans)?;
        let mid = range[0] * 0.5 + range[1] * 0.5;
        let at = clipped(&bounds(uv, [mid; 2])?, domain);
        let source_at = surface_offset::bounds(source, at, distance, limits.spans)?;
        let mut anchor_mismatch = false;
        if let (Some(s), Some(u), Some(w), Some(center)) = (
            source_jac.derivatives,
            uv_jac.derivatives,
            world_jac.derivatives,
            source_at.image,
        ) {
            let world_at = bounds(world, [mid; 2])?;
            let mut error = [I::point(0.); 3];
            for k in 0..3 {
                error[k] = world_at[k].sub(I::new(center[k][0], center[k][1])?)?;
            }
            anchor_mismatch = tangent::speed(error)?.lo > tolerance_mm;
            mismatch |= anchor_mismatch;
            let delta = I::new(range[0], range[1])?.sub(I::point(mid))?;
            for k in 0..3 {
                let mut mapped = I::point(0.);
                for a in 0..2 {
                    mapped = mapped.add(
                        I::new(s[a][k][0], s[a][k][1])?.mul(I::new(u[0][a][0], u[0][a][1])?)?,
                    )?;
                }
                let derivative = I::new(w[0][k][0], w[0][k][1])?.sub(mapped)?;
                error[k] = error[k].add(derivative.mul(delta)?)?;
            }
            let upper = tangent::speed(error)?.hi;
            cell.error_upper_mm = Some(upper);
            cell.admitted = upper <= tolerance_mm;
        }
        if !cell.admitted
            && !anchor_mismatch
            && mid > range[0]
            && mid < range[1]
            && out.visited + queue.len() + 2 <= limits.cells
        {
            queue.push([range[0], mid]);
            queue.push([mid, range[1]]);
        } else {
            out.cells.push(cell);
        }
    }
    out.lift_correspondence_proven = out.cells.iter().all(|c| c.admitted);
    if out.lift_correspondence_proven {
        out.error_upper_mm = Some(
            out.cells
                .iter()
                .filter_map(|c| c.error_upper_mm)
                .fold(0., f64::max),
        );
        out.reason = "whole-original-normal-offset-lift-proven";
    } else if mismatch {
        out.reason = "actual-world-UV-lift-mismatch";
    }
    Ok(out)
}
