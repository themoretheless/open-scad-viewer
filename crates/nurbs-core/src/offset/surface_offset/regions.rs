//! Closed original-UV strips from admitted contact traces and natural rails.
use crate::{
    Error, Result, check, curve::Curve, offset_contact_pcurve as fit, offset_patch_boundary,
    surface::Surface, trim_domain,
};
fn refused(s: &str) -> Error {
    Error::new("NURBS_OFFSET_UV_REGION_REFUSED", s)
}
pub struct Strip {
    source: Surface,
    trace: Curve,
    far_side: usize,
    reversed: bool,
    map: Surface,
    boundary: [Curve; 4],
}
impl Strip {
    pub fn source(&self) -> &Surface {
        &self.source
    }
    pub fn contact_trace(&self) -> &Curve {
        &self.trace
    }
    pub fn map(&self) -> &Surface {
        &self.map
    }
    /// Four CCW curves with exact joins in ORIGINAL source UV coordinates.
    pub fn boundary(&self) -> &[Curve; 4] {
        &self.boundary
    }
    pub fn far_boundary(&self) -> usize {
        self.far_side
    }
    pub fn contact_boundary(&self) -> usize {
        (self.far_side + 2) % 4
    }
    /// Map's driving coordinate runs opposite to the original trace clock.
    pub fn clock_reversed(&self) -> bool {
        self.reversed
    }
    pub fn reverify(&self) -> Result<()> {
        let fresh = build_strip(&self.source, &self.trace, self.far_side)?;
        check(
            fresh.map == self.map
                && fresh.boundary == self.boundary
                && fresh.reversed == self.reversed,
            "Saved UV region differs from original reconstruction",
        )
    }
}
fn candidate(source: &Surface, trace: &Curve, far_side: usize, reversed: bool) -> Surface {
    let fixed = if far_side == 0 || far_side == 2 { 1 } else { 0 };
    let high = far_side == 1 || far_side == 2;
    let natural = super::mapped::domain(source);
    let value = natural[fixed][usize::from(high)];
    let n = trace.control_points.len();
    let mut points =
        vec![vec![vec![0.; 3]; if fixed == 0 { n } else { 2 }]; if fixed == 0 { 2 } else { n }];
    let mut weights =
        vec![vec![1.; if fixed == 0 { n } else { 2 }]; if fixed == 0 { 2 } else { n }];
    for t in 0..n {
        let original = if reversed { n - 1 - t } else { t };
        for across in 0..2 {
            let (i, j) = if fixed == 0 { (across, t) } else { (t, across) };
            points[i][j][..2].copy_from_slice(&trace.control_points[original]);
            if across == usize::from(high) {
                points[i][j][fixed] = value;
            }
            weights[i][j] = trace.weights[original];
        }
    }
    let linear = vec![0., 0., 1., 1.];
    Surface {
        degree_u: if fixed == 0 { 1 } else { trace.degree },
        degree_v: if fixed == 0 { trace.degree } else { 1 },
        knots_u: if fixed == 0 {
            linear.clone()
        } else {
            trace.knots.clone()
        },
        knots_v: if fixed == 0 {
            trace.knots.clone()
        } else {
            linear
        },
        control_points: points,
        weights,
        periodic_u: false,
        periodic_v: false,
    }
}
/// Build the full closed UV strip between a trace and its projection onto one
/// literal natural source boundary (0 v-low, 1 u-high, 2 v-high, 3 u-low).
/// The map's coordinate axes retain the source's axis roles; the contact lies
/// on the opposite natural map side. No sampled polygon or endpoint welding
/// supplies region admission. Whole-map strong monotonicity/orientation and
/// original-domain hull containment admit an injective closed rectangle image.
/// Exact canonical boundary extraction then supplies its closed Jordan loop.
///
/// Full rational Bezier traces only. Reversing a trace preserves literal knots
/// and is explicitly recorded. This proves an untrimmed source UV region, not
/// contact correspondence, physical/world injectivity, existing material-loop
/// membership or a BRep trim. Use fit_pair for original contact correspondence.
pub fn build_strip(source: &Surface, trace: &Curve, far_side: usize) -> Result<Strip> {
    source.validate()?;
    trace.validate()?;
    check(
        far_side < 4
            && !trace.periodic
            && trace.control_points[0].len() == 2
            && (1..=16).contains(&trace.degree)
            && trace.control_points.len() == trace.degree + 1
            && trace.knots.len() == 2 * (trace.degree + 1),
        "UV strips require one natural boundary and a full rational Bezier trace",
    )?;
    let d = trace.domain();
    check(
        trace.knots[..=trace.degree].iter().all(|x| *x == d[0])
            && trace.knots[trace.degree + 1..].iter().all(|x| *x == d[1]),
        "Contact trace must retain its literal clamped domain",
    )?;
    let mut last = String::new();
    for reversed in [false, true] {
        let map = candidate(source, trace, far_side, reversed);
        if let Err(e) = super::parametric::validate_map(source, &map) {
            last = e.to_string();
            continue;
        }
        let canonical = offset_patch_boundary::extract(&map)?
            .ok_or_else(|| refused("UV map has no exact canonical boundary"))?;
        let boundary = canonical.coedges.map(|c| {
            let mut p = c.curve;
            for point in &mut p.control_points {
                point.truncate(2);
            }
            if c.reversed {
                p.control_points.reverse();
                p.weights.reverse();
            }
            p
        });
        if trim_domain::exact_loop_joins(&boundary)? != Some(true) {
            return Err(refused("Original UV loop joins are not exact"));
        }
        return Ok(Strip {
            source: source.clone(),
            trace: trace.clone(),
            far_side,
            reversed,
            map,
            boundary,
        });
    }
    Err(refused(&format!(
        "UV region injectivity/orientation/domain remains unproven: {last}"
    )))
}
pub struct Pair {
    sources: [Surface; 2],
    distances: [f64; 2],
    axis: usize,
    drive: [f64; 2],
    free: [f64; 2],
    second: [[f64; 2]; 2],
    far_sides: [usize; 2],
    tolerance_uv: f64,
    limits: fit::FitLimits,
    fitting: fit::FitReport,
    strips: [Strip; 2],
}
impl Pair {
    pub fn fitting(&self) -> &fit::FitReport {
        &self.fitting
    }
    pub fn strips(&self) -> &[Strip; 2] {
        &self.strips
    }
    pub fn reverify(&self) -> Result<()> {
        let fresh = fit_pair(
            crate::offset::contact_pcurve::OffsetFaces { surfaces: self.sources.each_ref(), distances: self.distances },
            super::contact_curve::ContactDomain { axis: self.axis, drive: self.drive, free: self.free, second: self.second },
            self.far_sides,
            self.tolerance_uv,
            self.limits,
        )?;
        for i in 0..2 {
            check(
                fresh.strips[i].map == self.strips[i].map
                    && fresh.strips[i].boundary == self.strips[i].boundary
                    && fresh.strips[i].reversed == self.strips[i].reversed,
                "Saved contact regions differ from fresh original branch reconstruction",
            )?;
        }
        Ok(())
    }
}
/// Freshly fit BOTH original contact traces, then qualify BOTH closed UV strips.
/// No partial region pair is published. Side choice is explicit; original BRep
/// incidence/material-sector inference, world seams and closed solids remain
/// independent downstream obligations.
#[allow(clippy::too_many_arguments)]
pub fn fit_pair(
    faces: crate::offset::contact_pcurve::OffsetFaces<'_>,
    domain: super::contact_curve::ContactDomain,
    far_sides: [usize; 2],
    tolerance_uv: f64,
    limits: fit::FitLimits,
) -> Result<Pair> {
    let crate::offset::contact_pcurve::OffsetFaces { surfaces: sources, distances } = faces;
    let super::contact_curve::ContactDomain { axis, drive, free, second } = domain;
    check(
        far_sides.iter().all(|s| *s < 4),
        "Choose literal natural source boundaries",
    )?;
    let fitting = fit::fit_branch(
        fit::OffsetFaces {
            surfaces: sources,
            distances,
        },
        axis,
        drive,
        free,
        second,
        tolerance_uv,
        limits,
    )?;
    let traces = fitting.curves.as_ref().ok_or_else(|| {
        refused(&format!(
            "Original UV contact curves remain unqualified: {:?}",
            fitting.stop
        ))
    })?;
    let strips = [
        build_strip(sources[0], &traces[0], far_sides[0])?,
        build_strip(sources[1], &traces[1], far_sides[1])?,
    ];
    Ok(Pair {
        sources: sources.map(Clone::clone),
        distances,
        axis,
        drive,
        free,
        second,
        far_sides,
        tolerance_uv,
        limits,
        fitting,
        strips,
    })
}
