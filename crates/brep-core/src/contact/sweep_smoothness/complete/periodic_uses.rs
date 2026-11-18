//! Whole opposite-side uses of one periodic source chart.
use crate::{Coedge, Model, Result, Surface};
use nurbs_core::{
    editing::exact_periodic_surface::open_exact_periodic_surface_axis, surface::Axis,
};
use std::borrow::Cow;

pub(super) fn full_side(
    model: &Model,
    surface: &Surface,
    coedge: &Coedge,
    budget: u64,
) -> (Option<(&'static str, bool)>, u64) {
    let world = model.edges[coedge.edge].curve.domain();
    let trim = coedge.trim();
    if trim.is_some_and(|t| t.curve_parameter != world) {
        return (None, 0);
    }
    if trim.is_none_or(|t| t.pcurve_parameter == coedge.pcurve.domain()) {
        if let Some(side) = super::coedge_side(surface, &coedge.pcurve, coedge.reversed) {
            return (Some(side), 0);
        }
    }
    if budget == 0 {
        return (None, 0);
    }
    let mut work = 0;
    let side = crate::shared_boundary::chart_segments::natural_segment(
        coedge,
        &mut work,
        budget.min(cad_predicates::MAX_WORK) as usize,
    )
    .and_then(|[a, b]| {
        use super::super::{Axis, boundary_from_endpoints};
        let label = boundary_from_endpoints(surface, &a, &b, Axis::Profile)
            .or_else(|| boundary_from_endpoints(surface, &a, &b, Axis::Station))?;
        if let Some(trim) = trim {
            let high = [
                surface.knots_u[surface.control_points.len()],
                surface.knots_v[surface.control_points[0].len()],
            ];
            let periodic = [surface.periodic_u, surface.periodic_v];
            let lift =
                [a, b].map(|p| std::array::from_fn(|i| i32::from(periodic[i] && p[i] == high[i])));
            if trim.periodic_lift != lift {
                return None;
            }
        }
        let free = usize::from(label.starts_with('u'));
        Some((label, (a[free] < b[free]) != coedge.reversed))
    });
    (side, work as u64)
}

pub(super) fn open(surface: &Surface, budget: u64) -> Result<(Option<Cow<'_, Surface>>, u64)> {
    let mut work = 0;
    let periodic = [surface.periodic_u, surface.periodic_v];
    let mut chart = Cow::Borrowed(surface);
    for (axis, wraps) in [Axis::U, Axis::V].into_iter().zip(periodic) {
        if !wraps {
            continue;
        }
        let remaining = (budget - work).min(cad_predicates::MAX_WORK);
        if remaining == 0 {
            return Ok((None, work));
        }
        let opened = open_exact_periodic_surface_axis(&chart, axis, remaining)?;
        work += opened.work_used;
        let Some(surface) = opened.surface else {
            return Ok((None, work));
        };
        chart = Cow::Owned(surface);
    }
    Ok((Some(chart), work))
}

pub(super) fn opposite_sides(surface: &Surface, a: &str, b: &str) -> bool {
    match (a, b) {
        ("uMin", "uMax") | ("uMax", "uMin") => surface.periodic_u,
        ("vMin", "vMax") | ("vMax", "vMin") => surface.periodic_v,
        _ => false,
    }
}
