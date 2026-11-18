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
    let allowance = budget.min(cad_predicates::MAX_WORK) as usize;
    let side =
        crate::shared_boundary::chart_segments::natural_segment(coedge, &mut work, allowance)
            .or_else(|| restricted_segment(coedge, &mut work, allowance))
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
                    let lift = [a, b].map(|p| {
                        std::array::from_fn(|i| i32::from(periodic[i] && p[i] == high[i]))
                    });
                    if trim.periodic_lift != lift {
                        return None;
                    }
                }
                let free = usize::from(label.starts_with('u'));
                Some((label, (a[free] < b[free]) != coedge.reversed))
            });
    (side, work as u64)
}

fn restricted_segment(coedge: &Coedge, work: &mut usize, limit: usize) -> Option<[[f64; 2]; 2]> {
    let [a, b] = coedge.trim()?.pcurve_parameter;
    let range = [a.min(b), a.max(b)];
    if coedge.pcurve.periodic || range == coedge.pcurve.domain() || *work == limit {
        return None;
    }
    let restriction = nurbs_core::editing::exact_curve_merge::clamp_exact_curve_on(
        &coedge.pcurve,
        range,
        (limit - *work) as u64,
    )
    .ok()?;
    *work += restriction.work_used as usize;
    let restricted = Coedge::new(coedge.edge, coedge.reversed, restriction.curve?);
    let mut segment =
        crate::shared_boundary::chart_segments::natural_segment(&restricted, work, limit)?;
    if a > b {
        segment.reverse();
    }
    Some(segment)
}

/// Prove a selected periodic join on exact native sides and the full world edge.
pub(crate) fn selected_periodic_join(
    model: &Model,
    faces: [usize; 2],
    uses: [&Coedge; 2],
    work: &mut u64,
    limit: u64,
) -> Result<
    Option<(
        nurbs_core::surface_g2_pcurves::Positions,
        nurbs_core::surface_g2::Orientation,
    )>,
> {
    use cad_predicates::BezierIdentity;
    use nurbs_core::{continuity::ProjectiveSeam, curve_surface_agreement, surface_g2_pcurves};
    let surfaces = faces.map(|face| &model.faces[face].surface);
    let edge = &model.edges[uses[0].edge].curve;
    let mut sides = [None; 2];
    for ((c, surface), side) in uses.into_iter().zip(surfaces).zip(&mut sides) {
        let (found, used) = full_side(model, surface, c, limit - *work);
        *work += used;
        *side = found;
    }
    let [Some((a, ad)), Some((b, bd))] = sides else {
        return Ok(None);
    };
    let same_face = faces[0] == faces[1];
    if same_face
        && (!opposite_sides(surfaces[0], a, b) || ad != bd || uses[0].reversed == uses[1].reversed)
    {
        return Ok(None);
    }
    let first_work = *work;
    let mut evidence = std::array::from_fn(|_| surface_g2_pcurves::PositionEvidence {
        exact: None,
        interval: None,
    });
    for ((c, surface), entry) in uses.into_iter().zip(surfaces).zip(&mut evidence) {
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
            let proof = curve_surface_agreement::verify_exact_trimmed(
                edge,
                &c.pcurve,
                surface,
                c.reversed,
                trim.curve_parameter,
                trim.pcurve_parameter,
                remaining,
            )?;
            if proof.is_none() {
                crate::boundary_agreement::restricted_periodic_lift(
                    edge,
                    c,
                    surface,
                    trim.pcurve_parameter,
                    work,
                    limit,
                )?
            } else {
                proof
            }
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
    let patches = if same_face {
        let (opened, used) = open(surfaces[0], limit - *work)?;
        *work += used;
        let Some(opened) = opened else {
            return Ok(None);
        };
        vec![opened.into_owned()]
    } else {
        let Some([left, mut right]) = open_pair(surfaces, work, limit)? else {
            return Ok(None);
        };
        if ad != bd {
            let (proved, used) = super::reverse_along(&mut right, b, limit - *work)?;
            *work += used;
            if !proved {
                return Ok(None);
            }
        }
        vec![left, right]
    };
    let seams = [ProjectiveSeam {
        patches: [0, usize::from(!same_face)],
        boundaries: [a.into(), b.into()],
        order: 2,
        normal_scale: 1.,
    }];
    let proof = super::graph_jets::qualify(&patches, &seams, limit - *work)?;
    *work += proof.exact_work;
    // For glued charts the two inward transverse directions are opposite.
    // Restore the original right-hand normal sign after any tangent reversal.
    let same =
        (super::graph_jets::inward_sign(a) != super::graph_jets::inward_sign(b)) != (ad != bd);
    let orientation = if same {
        nurbs_core::surface_g2::Orientation::Same
    } else {
        nurbs_core::surface_g2::Orientation::Opposite
    };
    Ok(proof.certified.then_some((
        surface_g2_pcurves::Positions {
            proven: true,
            upper: Some(0.),
            cells: 0,
            exact_work,
            sides: evidence,
        },
        orientation,
    )))
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

pub(super) fn open_pair(
    surfaces: [&Surface; 2],
    work: &mut u64,
    limit: u64,
) -> Result<Option<[Surface; 2]>> {
    let (left, used) = open(surfaces[0], limit - *work)?;
    *work += used;
    let Some(left) = left else { return Ok(None) };
    let (right, used) = open(surfaces[1], limit - *work)?;
    *work += used;
    Ok(right.map(|right| [left.into_owned(), right.into_owned()]))
}

/// Prepared charts for one immutable-model audit. A reflected entry is
/// keyed by the free axis, so either side on that axis uses the same chart.
pub(super) struct Charts {
    indices: Vec<[Option<usize>; 3]>,
    pub(super) patches: Vec<Surface>,
}

impl Charts {
    pub(super) fn new(faces: usize) -> Self {
        Self {
            indices: vec![[None; 3]; faces],
            patches: Vec::new(),
        }
    }

    pub(super) fn prepared(&self, face: usize) -> Option<&Surface> {
        self.indices[face][0].map(|index| &self.patches[index])
    }

    pub(super) fn native(
        &mut self,
        model: &Model,
        face: usize,
        work: &mut u64,
        limit: u64,
    ) -> Result<Option<usize>> {
        if let Some(index) = self.indices[face][0] {
            return Ok(Some(index));
        }
        let (opened, used) = open(&model.faces[face].surface, limit - *work)?;
        *work += used;
        let Some(opened) = opened else {
            return Ok(None);
        };
        let index = self.patches.len();
        self.patches.push(opened.into_owned());
        self.indices[face][0] = Some(index);
        Ok(Some(index))
    }

    pub(super) fn reflected(
        &mut self,
        face: usize,
        native: usize,
        side: &str,
        work: &mut u64,
        limit: u64,
    ) -> Result<Option<usize>> {
        let slot = if side.starts_with('u') { 1 } else { 2 };
        if let Some(index) = self.indices[face][slot] {
            return Ok(Some(index));
        }
        let mut patch = self.patches[native].clone();
        let (proved, used) = super::reverse_along(&mut patch, side, limit - *work)?;
        *work += used;
        if !proved {
            return Ok(None);
        }
        let index = self.patches.len();
        self.patches.push(patch);
        self.indices[face][slot] = Some(index);
        Ok(Some(index))
    }
}

pub(super) fn opposite_sides(surface: &Surface, a: &str, b: &str) -> bool {
    match (a, b) {
        ("uMin", "uMax") | ("uMax", "uMin") => surface.periodic_u,
        ("vMin", "vMax") | ("vMax", "vMin") => surface.periodic_v,
        _ => false,
    }
}
