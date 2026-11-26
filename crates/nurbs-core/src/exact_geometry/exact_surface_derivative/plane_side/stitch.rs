//! Exact section connectivity across an ordered strip of original knot cells.
use super::{
    PResult, Q, Refusal, Result, Surface, SurfacePlaneGraph, Work, check,
    samples::{cell_domain, graph_cell, isolate, slice},
    validate,
};
use crate::exact_geometry::exact_projective::UvAxis;
use num_traits::{One, Signed, Zero};

#[derive(Clone, Copy, Debug)]
pub struct SurfacePlaneGraphStripOptions {
    pub solve_axis: UvAxis,
    pub solved_span: usize,
    /// Inclusive original knot-span index range along the transverse axis.
    pub transverse_spans: [usize; 2],
    pub max_normalized_width: f64,
    pub max_work: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfacePlaneGraphStripCell {
    pub span: [usize; 2],
    pub domain: [[f64; 2]; 2],
    pub graph: SurfacePlaneGraph,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfacePlaneGraphJunction {
    /// Original knot coordinate along the transverse direction.
    pub transverse: f64,
    /// One shared root interval in the original solved knot clock.
    pub solved: [Q; 2],
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfacePlaneGraphStrip {
    pub cells: Vec<SurfacePlaneGraphStripCell>,
    /// Shared root certificates between consecutive cells in strip order.
    pub junctions: Vec<SurfacePlaneGraphJunction>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfacePlaneGraphStripReport {
    pub outcome: std::result::Result<SurfacePlaneGraphStrip, Refusal>,
    pub work_used: u64,
}

/// Prove continuity of one section branch across a consecutive knot strip.
/// The one-sided homogeneous plane traces must agree exactly on each
/// intervening edge. A shared polynomial with a unique root certifies the
/// same UV endpoint, and the validated B-spline is continuous there.
pub fn stitch_surface_plane_graph_strip(
    surface: &Surface,
    normal: [f64; 3],
    offset: f64,
    options: SurfacePlaneGraphStripOptions,
) -> Result<SurfacePlaneGraphStripReport> {
    validate(surface, normal, offset, options.max_work)?;
    let solved = usize::from(options.solve_axis == UvAxis::V);
    let spans = [
        surface.degree_u..surface.control_points.len(),
        surface.degree_v..surface.control_points[0].len(),
    ];
    let [first, last] = options.transverse_spans;
    check(
        first <= last
            && spans[solved].contains(&options.solved_span)
            && spans[1 - solved].contains(&first)
            && spans[1 - solved].contains(&last)
            && options.max_normalized_width.is_finite()
            && options.max_normalized_width > 0.
            && options.max_normalized_width <= 1.,
        "Graph strip requires valid knot spans and a normalized width in (0, 1]",
    )?;
    let mut work = Work {
        used: 0,
        limit: options.max_work,
    };
    let outcome = stitch(surface, normal, offset, options, solved, &mut work);
    Ok(SurfacePlaneGraphStripReport {
        outcome,
        work_used: work.used,
    })
}

fn stitch(
    surface: &Surface,
    normal: [f64; 3],
    offset: f64,
    options: SurfacePlaneGraphStripOptions,
    solved: usize,
    work: &mut Work,
) -> PResult<SurfacePlaneGraphStrip> {
    let transverse = 1 - solved;
    let knots = [&surface.knots_u, &surface.knots_v];
    let span = options.solved_span;
    let origin = work.lift(knots[solved][span])?;
    let end = work.lift(knots[solved][span + 1])?;
    let width = work.op(&end, &origin, |a, b| a - b)?;
    if width.is_zero() {
        return Err(Refusal::InvalidOutput);
    }
    let tolerance = work.lift(options.max_normalized_width)?;
    let mut cells: Vec<SurfacePlaneGraphStripCell> = Vec::new();
    let mut junctions = Vec::new();
    let mut previous = None;
    let [first, last] = options.transverse_spans;
    for k in first..=last {
        if knots[transverse][k] == knots[transverse][k + 1] {
            continue;
        }
        let mut spans = [span; 2];
        spans[transverse] = k;
        let domain = cell_domain(surface, spans).map_err(|_| Refusal::InvalidOutput)?;
        let (net, graph) = graph_cell(
            surface,
            normal,
            offset,
            spans,
            domain,
            options.solve_axis,
            work,
        )?;
        if let Some(prior) = previous {
            let lower = slice(&net, options.solve_axis, &Q::zero(), work)?;
            if prior != lower {
                return Err(Refusal::UnprovedIdentity);
            }
            let [lo, hi] = isolate(&lower, &tolerance, graph.derivative[0].is_positive(), work)?;
            let absolute = |t: &Q, work: &mut Work| -> PResult<Q> {
                let delta = work.op(&width, t, |a, b| a * b)?;
                Ok(work.op(&origin, &delta, |a, b| a + b)?)
            };
            junctions.push(SurfacePlaneGraphJunction {
                transverse: domain[transverse][0],
                solved: [absolute(&lo, work)?, absolute(&hi, work)?],
            });
        }
        previous = Some(slice(&net, options.solve_axis, &Q::one(), work)?);
        cells.push(SurfacePlaneGraphStripCell {
            span: spans,
            domain,
            graph,
        });
    }
    if cells.is_empty() {
        return Err(Refusal::InvalidOutput);
    }
    Ok(SurfacePlaneGraphStrip { cells, junctions })
}
