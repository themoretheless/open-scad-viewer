//! Interior isoparametric runout proposals, qualified on immutable supports.
use super::{Boundary, Options, Order, Report, build_fields, canonical, fields_at};
use crate::{
    Result, check,
    curve::Curve,
    surface::{Axis, Surface},
    surface_g2_pcurves,
    surface_join::Decision,
};

#[derive(Clone, Copy)]
pub struct Trace {
    /// Fixed coordinate in the original support chart.
    pub at: f64,
    /// Increasing interval of the other original coordinate.
    pub along: [f64; 2],
}
pub struct TraceReport {
    pub construction: Report,
    pub support_seams: Vec<(usize, usize, surface_g2_pcurves::Report)>,
    pub internal_seams: Vec<crate::surface_g2::Report>,
    pub complete: bool,
    pub within_tolerance: bool,
    pub cells: usize,
    pub exact_work: u64,
}
fn line(points: [[f64; 2]; 2], domain: [f64; 2]) -> Curve {
    Curve {
        degree: 1,
        knots: vec![domain[0], domain[0], domain[1], domain[1]],
        control_points: points.map(|p| p.to_vec()).to_vec(),
        weights: vec![1.; 2],
        periodic: false,
    }
}
/// Build a quintic bridge between selected interior traces. Boundary labels
/// specify the retained side, not a request to crop or mutate a support.
/// All saved boundary positions and geometric jets are freshly checked against
/// ORIGINAL surfaces. Neither admission of a body nor interior embedding is
/// implied by a successful seam report.
pub fn g2_traces(
    supports: [&Surface; 2],
    traces: [Trace; 2],
    o: Options,
    tolerances: [f64; 3],
    max_cells: usize,
    max_exact: u64,
) -> Result<TraceReport> {
    let construction = construct(supports, traces, o, tolerances, max_cells, max_exact)?;
    qualify(
        construction,
        supports,
        traces,
        o,
        tolerances,
        max_cells,
        max_exact,
        None,
    )
}
pub(super) fn construct(
    supports: [&Surface; 2],
    traces: [Trace; 2],
    o: Options,
    tolerances: [f64; 3],
    max_cells: usize,
    max_exact: u64,
) -> Result<Report> {
    check(
        o.gains.iter().all(|g| g.is_finite() && *g > 0.)
            && (1..=256).contains(&o.max_patches)
            && max_cells <= 100000
            && max_exact <= 1000000,
        "Interior blend requires bounded work and positive finite gains",
    )?;
    check(
        tolerances.iter().all(|t| t.is_finite() && *t >= 0.) && tolerances[1] <= 2.,
        "Invalid interior blend tolerances",
    )?;
    let mut input = Vec::new();
    for side in 0..2 {
        let s = canonical(supports[side], o.boundaries[side])?;
        check(
            !s.periodic_u && !s.periodic_v,
            "Interior blend needs nonperiodic original charts",
        )?;
        check(
            s.control_points
                .len()
                .checked_mul(s.control_points[0].len())
                .is_some_and(|n| n <= 4096),
            "Interior blend source control budget exceeded",
        )?;
        let t = traces[side];
        let u = [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]];
        let v = [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]];
        let end = o.boundaries[side].end();
        check(
            t.at.is_finite()
                && t.along.iter().all(|x| x.is_finite())
                && u[0] <= t.along[0]
                && t.along[0] < t.along[1]
                && t.along[1] <= u[1]
                && v[0] <= t.at
                && t.at <= v[1]
                && if end { v[0] < t.at } else { t.at < v[1] },
            "Interior trace needs a nonempty retained support side",
        )?;
        let sign = if end { 1. } else { -1. } * if side == 0 { 1. } else { -1. };
        for c in fields_at(&s, t.at, sign * o.gains[side], false, 2)? {
            let mut c = c.trim(t.along[0], t.along[1])?;
            if o.reverse_u[side] {
                c = c.reverse()?;
            }
            input.push(c);
        }
    }
    build_fields(input, Order::C2, o.max_patches)
}
pub(super) fn qualify(
    construction: Report,
    supports: [&Surface; 2],
    traces: [Trace; 2],
    o: Options,
    tolerances: [f64; 3],
    max_cells: usize,
    max_exact: u64,
    target: Option<&Curve>,
) -> Result<TraceReport> {
    let mut r = TraceReport {
        construction,
        support_seams: Vec::new(),
        internal_seams: Vec::new(),
        complete: true,
        within_tolerance: true,
        cells: 0,
        exact_work: 0,
    };
    for (i, patch) in r.construction.patches.iter().enumerate() {
        let d = [
            patch.knots_u[patch.degree_u],
            patch.knots_u[patch.control_points.len()],
        ];
        for side in 0..2 {
            if r.cells == max_cells {
                r.complete = false;
                r.within_tolerance = false;
                continue;
            }
            let t = traces[side];
            let points = d.map(|u| {
                let u = if o.reverse_u[side] { 1. - u } else { u };
                let along = t.along[0] + (t.along[1] - t.along[0]) * u;
                if matches!(o.boundaries[side], Boundary::UStart | Boundary::UEnd) {
                    [t.at, along]
                } else {
                    [along, t.at]
                }
            });
            let pc = if side == 1 && target.is_some() {
                let target = target.unwrap();
                let td = target.domain();
                let ends =
                    d.map(|u| td[0] + (td[1] - td[0]) * if o.reverse_u[1] { 1. - u } else { u });
                let mut pc = target.trim(ends[0].min(ends[1]), ends[0].max(ends[1]))?;
                if ends[0] > ends[1] {
                    pc = pc.reverse()?;
                }
                pc
            } else {
                line(points, d)
            };
            let patch_pc = line([[d[0], side as f64], [d[1], side as f64]], d);
            let edge = patch.iso(Axis::V, side as f64)?;
            let proof = surface_g2_pcurves::inspect(
                &edge,
                [&pc, &patch_pc],
                [supports[side], patch],
                [false, false],
                tolerances[0],
                tolerances[1],
                tolerances[2],
                max_cells - r.cells,
                max_exact - r.exact_work,
            )?;
            r.cells += proof.geometric.cells;
            r.exact_work += proof.positions.exact_work;
            r.complete &= proof.geometric.decision != Decision::Unresolved;
            r.within_tolerance &=
                proof.positions.proven && proof.geometric.decision == Decision::WithinTolerance;
            r.support_seams.push((i, side, proof));
        }
        if i > 0 {
            if r.cells == max_cells {
                r.complete = false;
                r.within_tolerance = false;
                continue;
            }
            let proof = crate::surface_g2::inspect(
                &r.construction.patches[i - 1],
                crate::surface_join::Boundary::UMax,
                patch,
                crate::surface_join::Boundary::UMin,
                false,
                tolerances[0],
                tolerances[1],
                tolerances[2],
                max_cells - r.cells,
            )?;
            r.cells += proof.cells;
            r.complete &= proof.decision != Decision::Unresolved;
            r.within_tolerance &= proof.decision == Decision::WithinTolerance;
            r.internal_seams.push(proof);
        }
    }
    Ok(r)
}
