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

/// Prove a selected same-face wrap using original edge ownership and the
/// exact opened-chart jet owner. Both uses must traverse the full world edge.
pub(crate) fn selected_wrap(
    model: &Model,
    face: usize,
    uses: [&Coedge; 2],
    work: &mut u64,
    limit: u64,
) -> Result<Option<nurbs_core::surface_g2_pcurves::Positions>> {
    use cad_predicates::BezierIdentity;
    use nurbs_core::{continuity::ProjectiveSeam, curve_surface_agreement, surface_g2_pcurves};
    let surface = &model.faces[face].surface;
    let edge = &model.edges[uses[0].edge].curve;
    let mut sides = [None; 2];
    for (c, side) in uses.into_iter().zip(&mut sides) {
        let (found, used) = full_side(model, surface, c, limit - *work);
        *work += used;
        *side = found;
    }
    let [Some((a, ad)), Some((b, bd))] = sides else {
        return Ok(None);
    };
    if !opposite_sides(surface, a, b) || ad != bd || uses[0].reversed == uses[1].reversed {
        return Ok(None);
    }
    let first_work = *work;
    let mut evidence = std::array::from_fn(|_| surface_g2_pcurves::PositionEvidence {
        exact: None,
        interval: None,
    });
    for (c, entry) in uses.into_iter().zip(&mut evidence) {
        let remaining = limit - *work;
        if remaining == 0 {
            return Ok(None);
        }
        let exact = if let Some(trim) = c.trim() {
            if crate::boundary_agreement::periodic_lifts(surface, &c.pcurve, trim.pcurve_parameter)?
                != trim.periodic_lift
            {
                return Ok(None);
            }
            curve_surface_agreement::verify_exact_trimmed(
                edge,
                &c.pcurve,
                surface,
                c.reversed,
                trim.curve_parameter,
                trim.pcurve_parameter,
                remaining,
            )?
        } else {
            curve_surface_agreement::verify_exact(edge, &c.pcurve, surface, c.reversed, remaining)?
        };
        let Some(exact) = exact else {
            return Ok(None);
        };
        *work += exact.work_used;
        if exact.outcome != BezierIdentity::Equal {
            return Ok(None);
        }
        entry.exact = Some(exact);
    }
    let exact_work = *work - first_work;
    let (opened, used) = open(surface, limit - *work)?;
    *work += used;
    let Some(opened) = opened else {
        return Ok(None);
    };
    let seams = [ProjectiveSeam {
        patches: [0, 0],
        boundaries: [a.into(), b.into()],
        order: 2,
        normal_scale: 1.,
    }];
    let proof = super::graph_jets::qualify(&[opened.into_owned()], &seams, limit - *work)?;
    *work += proof.exact_work;
    Ok(proof.certified.then_some(surface_g2_pcurves::Positions {
        proven: true,
        upper: Some(0.),
        cells: 0,
        exact_work,
        sides: evidence,
    }))
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
