//! Exact transfer to a new piecewise-Bézier UV layout.
use super::{
    BezierSpanLayout, PcurvePullback, PcurvePullbackRefusal,
    chart_nonnegative::midpoint_coordinate,
    mobius::validate_clock,
    mobius_piecewise::validate_layout,
    pcurve_mobius::{
        active_spans, original_span, relocated_axis, remap_component, scale, store_piece,
        validate_chart,
    },
    pcurve_seam_isolation::isolate_power,
};
use crate::{
    Result, check,
    core::bounded_rational::Work,
    curve::Curve,
    exact_geometry::{
        exact_derivative::{Refusal, add},
        rational_factor::polynomial::Poly,
    },
};
use num_rational::BigRational as Q;

/// Move a retained pcurve to the same piecewise map used by
/// `relayout_bezier_surface`. Old and new axes have the same span counts.
/// Each original span is cut at all internal grid contacts that have an
/// exact binary64 parameter. Coincident traces use the lower adjacent
/// cell, whose boundary agrees with the upper cell. Original curve
/// parameters are retained. The operation withholds all output if any
/// cut, chart membership, or storage proof fails.
pub fn pullback_relayout_pcurve(
    source: &Curve,
    old_breaks: [&[f64]; 2],
    layouts: [BezierSpanLayout<'_>; 2],
    max_work: u64,
) -> Result<PcurvePullback> {
    let mut domain = [[0.; 2]; 2];
    for axis in 0..2 {
        let layout = layouts[axis];
        validate_layout(layout)?;
        validate_clock(layout.factors, max_work)?;
        let old = old_breaks[axis];
        check(
            old.len() == layout.breaks.len()
                && old.iter().all(|t| t.is_finite())
                && old.windows(2).all(|pair| pair[0] < pair[1]),
            "Old UV breaks must be finite and increasing, with one interval per new interval",
        )?;
        domain[axis] = [old[0], old[old.len() - 1]];
    }
    validate_chart(source, domain)?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let (pieces, refusal) = match relocate(source, old_breaks, layouts, &mut work) {
        Ok(pieces) => (Some(pieces), None),
        Err(reason) => (None, Some(reason)),
    };
    Ok(PcurvePullback {
        pieces,
        work_used: work.used,
        refusal,
    })
}

type Transfer<T> = std::result::Result<T, PcurvePullbackRefusal>;

fn relocate(
    source: &Curve,
    old: [&[f64]; 2],
    layout: [BezierSpanLayout<'_>; 2],
    work: &mut Work,
) -> Transfer<Vec<Curve>> {
    let exact = exact_breaks(old, work)?;
    let mut out = Vec::new();
    for (span, domain) in active_spans(source) {
        let h = original_span(source, span, domain, work)?;
        let cuts = grid_cuts(&h, &exact, domain, work)?;
        for bounds in cuts.windows(2) {
            let interval = [bounds[0], bounds[1]];
            let h = original_span(source, span, interval, work)?;
            let [u, v] = [
                map_component(
                    &h[0],
                    &h[2],
                    old[0],
                    &exact[0],
                    layout[0],
                    source.degree,
                    work,
                )?,
                map_component(
                    &h[1],
                    &h[2],
                    old[1],
                    &exact[1],
                    layout[1],
                    source.degree,
                    work,
                )?,
            ];
            out.push(store_piece(interval, &u, &v, work)?);
        }
    }
    Ok(out)
}

pub(super) fn exact_breaks(
    breaks: [&[f64]; 2],
    work: &mut Work,
) -> std::result::Result<[Vec<Q>; 2], Refusal> {
    let lift_axis = |axis: &[f64], work: &mut Work| {
        axis.iter()
            .map(|&value| work.lift(value).map_err(Refusal::from))
            .collect::<std::result::Result<Vec<_>, _>>()
    };
    Ok([lift_axis(breaks[0], work)?, lift_axis(breaks[1], work)?])
}

pub(super) fn grid_cuts(
    h: &[Poly],
    axes: &[Vec<Q>; 2],
    domain: [f64; 2],
    work: &mut Work,
) -> Transfer<Vec<f64>> {
    let mut cuts = domain.to_vec();
    for (coordinate, axis) in h.iter().zip(axes) {
        for seam in &axis[1..axis.len() - 1] {
            let power = add(coordinate, &scale(&h[2], seam, work)?, true, work)?;
            if power.is_empty() {
                continue;
            }
            for root in isolate_power(&power, domain, None, work)? {
                let [lo, hi] = root.parameter;
                if lo != hi {
                    return Err(Refusal::UnprovedIdentity.into());
                }
                cuts.push(lo);
            }
        }
    }
    cuts.sort_by(f64::total_cmp);
    cuts.dedup_by(|a, b| *a == *b);
    Ok(cuts)
}

fn map_component(
    coordinate: &Poly,
    weight: &Poly,
    old: &[f64],
    exact: &[Q],
    layout: BezierSpanLayout<'_>,
    degree: usize,
    work: &mut Work,
) -> Transfer<[Poly; 2]> {
    let x = midpoint_coordinate(coordinate, weight, work)?;
    if x < exact[0] || x > exact[exact.len() - 1] {
        return Err(PcurvePullbackRefusal::UnprovedChartDomain);
    }
    let cell = exact[1..].partition_point(|end| *end < x);
    let axis = relocated_axis(
        [old[cell], old[cell + 1]],
        [layout.breaks[cell], layout.breaks[cell + 1]],
        layout.factors[cell],
        work,
    )?;
    remap_component(coordinate, weight, &axis, degree, work)
}
