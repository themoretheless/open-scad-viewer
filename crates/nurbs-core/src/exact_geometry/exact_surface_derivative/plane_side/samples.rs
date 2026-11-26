//! Certified root samples on a surface-plane intersection graph.
use super::{
    PResult, PlaneNet, Q, Refusal, Result, Surface, SurfacePlaneGraph, Work, check, extract_net,
    graph::graph, validate,
};
use crate::exact_geometry::exact_projective::UvAxis;
use num_traits::{One, Signed, Zero};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfacePlaneGraphSample {
    /// Exact normalized coordinate in the transverse direction.
    pub transverse: Q,
    /// Closed normalized root interval in the solved direction.
    pub root: [Q; 2],
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfacePlaneGraphSamples {
    pub domain: [[f64; 2]; 2],
    pub graph: SurfacePlaneGraph,
    pub samples: Vec<SurfacePlaneGraphSample>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfacePlaneGraphSampleReport {
    pub outcome: std::result::Result<SurfacePlaneGraphSamples, Refusal>,
    pub work_used: u64,
}

#[derive(Clone, Copy, Debug)]
pub struct SurfacePlaneGraphSampleOptions {
    /// Original nonempty U and V knot span indices.
    pub span: [usize; 2],
    pub solve_axis: UvAxis,
    pub max_normalized_width: f64,
    pub max_work: u64,
}

/// Isolate the unique graph root at each authored normalized transverse
/// parameter. The entire selected cell is qualified before sampling.
/// Rational bisection retains exact bracket endpoints, including edge roots.
pub fn sample_surface_plane_graph(
    surface: &Surface,
    normal: [f64; 3],
    offset: f64,
    transverse: &[Q],
    options: SurfacePlaneGraphSampleOptions,
) -> Result<SurfacePlaneGraphSampleReport> {
    validate(surface, normal, offset, options.max_work)?;
    let domain = cell_domain(surface, options.span)?;
    check(
        !transverse.is_empty()
            && transverse.len() <= 1024
            && transverse
                .iter()
                .all(|q| !q.is_negative() && q <= &Q::one())
            && options.max_normalized_width.is_finite()
            && options.max_normalized_width > 0.
            && options.max_normalized_width <= 1.,
        "Graph sampling needs 1..1024 normalized coordinates and a width in (0, 1]",
    )?;
    let mut work = Work {
        used: 0,
        limit: options.max_work,
    };
    let outcome = (|| {
        let (net, graph) = graph_cell(
            surface,
            normal,
            offset,
            options.span,
            domain,
            options.solve_axis,
            &mut work,
        )?;
        let width = work.lift(options.max_normalized_width)?;
        let samples = transverse
            .iter()
            .map(|transverse| {
                let polynomial = slice(&net, options.solve_axis, transverse, &mut work)?;
                let root = isolate(
                    &polynomial,
                    &width,
                    graph.derivative[0].is_positive(),
                    &mut work,
                )?;
                Ok(SurfacePlaneGraphSample {
                    transverse: transverse.clone(),
                    root,
                })
            })
            .collect::<PResult<Vec<_>>>()?;
        Ok(SurfacePlaneGraphSamples {
            domain,
            graph,
            samples,
        })
    })();
    Ok(SurfacePlaneGraphSampleReport {
        outcome,
        work_used: work.used,
    })
}

pub(super) fn cell_domain(surface: &Surface, span: [usize; 2]) -> Result<[[f64; 2]; 2]> {
    let [u, v] = span;
    check(
        (surface.degree_u..surface.control_points.len()).contains(&u)
            && (surface.degree_v..surface.control_points[0].len()).contains(&v)
            && surface.knots_u[u] < surface.knots_u[u + 1]
            && surface.knots_v[v] < surface.knots_v[v + 1],
        "Graph sampling requires a nonempty original knot cell",
    )?;
    Ok([
        [surface.knots_u[u], surface.knots_u[u + 1]],
        [surface.knots_v[v], surface.knots_v[v + 1]],
    ])
}

pub(super) fn graph_cell(
    surface: &Surface,
    [x, y, z]: [f64; 3],
    offset: f64,
    span: [usize; 2],
    domain: [[f64; 2]; 2],
    solve_axis: UvAxis,
    work: &mut Work,
) -> PResult<(PlaneNet, SurfacePlaneGraph)> {
    let plane = [
        work.lift(x)?,
        work.lift(y)?,
        work.lift(z)?,
        -work.lift(offset)?,
    ];
    let net = extract_net(surface, span, domain, &plane, work)?;
    let graph = graph(&net, solve_axis, work)?.ok_or(Refusal::UnprovedIdentity)?;
    Ok((net, graph))
}

pub(super) fn slice(net: &PlaneNet, axis: UvAxis, t: &Q, work: &mut Work) -> PResult<Vec<Q>> {
    let [nu, nv] = net.size;
    let (count, transverse_count) = match axis {
        UvAxis::U => (nu, nv),
        UvAxis::V => (nv, nu),
    };
    let mut output = Vec::with_capacity(count);
    for i in 0..count {
        let terms = (0..transverse_count)
            .map(|j| match axis {
                UvAxis::U => net.numerator[i * nv + j].clone(),
                UvAxis::V => net.numerator[j * nv + i].clone(),
            })
            .collect::<Vec<_>>();
        output.push(evaluate(&terms, t, work)?);
    }
    Ok(output)
}

pub(super) fn evaluate(coefficients: &[Q], t: &Q, work: &mut Work) -> PResult<Q> {
    if t.is_zero() {
        return Ok(coefficients[0].clone());
    }
    if t.is_one() {
        return Ok(coefficients[coefficients.len() - 1].clone());
    }
    let mut terms = coefficients.to_vec();
    for level in 1..terms.len() {
        for i in 0..terms.len() - level {
            let delta = work.op(&terms[i + 1], &terms[i], |a, b| a - b)?;
            let offset = work.op(&delta, t, |a, b| a * b)?;
            terms[i] = work.op(&terms[i], &offset, |a, b| a + b)?;
        }
    }
    Ok(terms.remove(0))
}

pub(super) fn isolate(
    coefficients: &[Q],
    width: &Q,
    increasing: bool,
    work: &mut Work,
) -> PResult<[Q; 2]> {
    let zero = Q::zero();
    let one = Q::one();
    if coefficients[0].is_zero() {
        return Ok([zero.clone(), zero]);
    }
    if coefficients[coefficients.len() - 1].is_zero() {
        return Ok([one.clone(), one]);
    }
    let two = Q::from_integer(2.into());
    let mut region = [zero, one];
    while work.op(&region[1], &region[0], |a, b| a - b)? > *width {
        let t = work.op(&region[0], &region[1], |a, b| (a + b) / &two)?;
        let value = evaluate(coefficients, &t, work)?;
        if value.is_zero() {
            return Ok([t.clone(), t]);
        }
        region[usize::from(value.is_positive() == increasing)] = t;
    }
    Ok(region)
}
