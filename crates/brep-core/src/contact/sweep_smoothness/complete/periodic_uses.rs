//! Whole opposite-side uses of one periodic source chart.
use crate::{Coedge, Model, Result, Surface};
use nurbs_core::{
    editing::exact_periodic_surface::open_exact_periodic_surface_axis, surface::Axis,
};
use std::borrow::Cow;

pub(super) fn full_use(model: &Model, coedge: &Coedge) -> bool {
    coedge.trim().is_none_or(|trim| {
        trim.curve_parameter == model.edges[coedge.edge].curve.domain()
            && trim.pcurve_parameter == coedge.pcurve.domain()
    })
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
