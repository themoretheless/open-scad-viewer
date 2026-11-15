//! Exact rational composition of a retained pcurve with its tensor surface.
use super::{
    PcurvePullbackRefusal,
    chart_nonnegative::midpoint_coordinate,
    mobius::validate_clock,
    pcurve_mobius::{
        ChartInterval, active_spans, original_span, scale, store_target, validate_chart,
    },
    pcurve_relayout::{exact_breaks, grid_cuts},
};
use crate::{
    Result,
    core::bounded_rational::Work,
    curve::Curve,
    editing::{
        exact_derivative::{PResult, Refusal, add},
        exact_surface_derivative as tensor,
        rational_factor::polynomial::{self as uni, Poly},
    },
    surface::Surface,
};
use num_rational::BigRational as Q;
use num_traits::One;

type LiftResult<T> = std::result::Result<T, PcurvePullbackRefusal>;

#[derive(Clone, Debug)]
pub struct PcurveLiftReport {
    /// Ordered 3D curves on all covered spans, or absent on refusal.
    pub pieces: Option<Vec<Curve>>,
    pub work_used: u64,
    pub refusal: Option<PcurvePullbackRefusal>,
}

/// Materialize the complete three-dimensional image S(U(t), V(t)) of a
/// retained pcurve in its original absolute curve parameter. Each output
/// piece belongs to an original tensor knot cell. Internal chart contacts
/// must have exactly representable cut parameters. Original one-sided
/// pcurve traces remain ordered, and a curve lying on a grid line uses the
/// lower adjacent cell. Each stored rational piece is verified over its
/// entire interval.
pub fn lift_pcurve_to_surface(
    surface: &Surface,
    pcurve: &Curve,
    max_work: u64,
) -> Result<PcurveLiftReport> {
    surface.validate()?;
    validate_clock(&[], max_work)?;
    let axes = [
        CellAxis::new(
            surface.degree_u,
            &surface.knots_u,
            surface.control_points.len(),
        ),
        CellAxis::new(
            surface.degree_v,
            &surface.knots_v,
            surface.control_points[0].len(),
        ),
    ];
    validate_chart(pcurve, axes.each_ref().map(CellAxis::domain))?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let (pieces, refusal) = match lift(surface, pcurve, &axes, &mut work) {
        Ok(pieces) => (Some(pieces), None),
        Err(reason) => (None, Some(reason)),
    };
    Ok(PcurveLiftReport {
        pieces,
        refusal,
        work_used: work.used,
    })
}

struct CellAxis {
    breaks: Vec<f64>,
    spans: Vec<usize>,
}

impl CellAxis {
    fn new(degree: usize, knots: &[f64], count: usize) -> Self {
        let spans: Vec<_> = (degree..count)
            .filter(|&span| knots[span] < knots[span + 1])
            .collect();
        let mut breaks = vec![knots[degree]];
        breaks.extend(spans.iter().map(|&span| knots[span + 1]));
        Self { breaks, spans }
    }

    fn domain(&self) -> [f64; 2] {
        [self.breaks[0], self.breaks[self.breaks.len() - 1]]
    }

    fn locate(
        &self,
        coordinate: &Poly,
        weight: &Poly,
        exact: &[Q],
        degree: usize,
        work: &mut Work,
    ) -> LiftResult<CellInput> {
        let x = midpoint_coordinate(coordinate, weight, work)?;
        if x < exact[0] || x > exact[exact.len() - 1] {
            return Err(PcurvePullbackRefusal::UnprovedChartDomain);
        }
        let cell = exact[1..].partition_point(|end| *end < x);
        let domain = [self.breaks[cell], self.breaks[cell + 1]];
        let interval = ChartInterval::new(domain, work)?;
        let [numerator, denominator] = interval
            .contained(coordinate, weight, degree, work)?
            .ok_or(PcurvePullbackRefusal::UnprovedChartDomain)?;
        let width = work
            .op(&exact[cell + 1], &exact[cell], |a, b| a - b)
            .map_err(Refusal::from)?;
        Ok(CellInput {
            span: self.spans[cell],
            domain,
            width,
            numerator,
            denominator,
        })
    }
}

struct CellInput {
    span: usize,
    domain: [f64; 2],
    width: Q,
    numerator: Poly,
    denominator: Poly,
}

fn lift(
    surface: &Surface,
    pcurve: &Curve,
    axes: &[CellAxis; 2],
    work: &mut Work,
) -> LiftResult<Vec<Curve>> {
    let exact = exact_breaks([&axes[0].breaks, &axes[1].breaks], work)?;
    let mut pieces = Vec::new();
    for (span, original) in active_spans(pcurve) {
        let h = original_span(pcurve, span, original, work)?;
        let cuts = grid_cuts(&h, &exact, original, work)?;
        for pair in cuts.windows(2) {
            let parameter = [pair[0], pair[1]];
            let h = original_span(pcurve, span, parameter, work)?;
            let u = axes[0].locate(&h[0], &h[2], &exact[0], pcurve.degree, work)?;
            let v = axes[1].locate(&h[1], &h[2], &exact[1], pcurve.degree, work)?;
            let target = compose(surface, u, v, work)?;
            pieces.push(store_target(parameter, &target, work)?);
        }
    }
    Ok(pieces)
}

fn compose(surface: &Surface, u: CellInput, v: CellInput, work: &mut Work) -> PResult<Vec<Poly>> {
    let input = tensor::extraction::cell(
        surface,
        u.span,
        v.span,
        [u.domain, v.domain],
        &[u.width, v.width],
        work,
    )
    .map_err(|reason| match reason {
        tensor::Refusal::ResourceLimit => Refusal::ResourceLimit,
        tensor::Refusal::NotRepresentable => Refusal::NotRepresentable,
        tensor::Refusal::OutputDegreeLimit => Refusal::OutputDegreeLimit,
        tensor::Refusal::UnprovedDenominator => Refusal::UnprovedDenominator,
        tensor::Refusal::InvalidOutput => Refusal::InvalidOutput,
        tensor::Refusal::UnprovedIdentity => Refusal::UnprovedIdentity,
    })?;
    // Clear each axis denominator to its tensor degree, leaving the same
    // common polynomial factor on all four homogeneous coordinates.
    let bu = powers(u.numerator, u.denominator, surface.degree_u, work)?;
    let bv = powers(v.numerator, v.denominator, surface.degree_v, work)?;
    input
        .iter()
        .map(|channel| {
            channel
                .iter()
                .try_fold(Vec::new(), |sum, (&(i, j), coefficient)| {
                    let monomial = uni::multiply(&bu[i], &bv[j], work)?;
                    add(&sum, &scale(&monomial, coefficient, work)?, false, work)
                })
        })
        .collect()
}

fn powers(n: Poly, d: Poly, degree: usize, work: &mut Work) -> PResult<Vec<Poly>> {
    fn sequence(base: Poly, degree: usize, work: &mut Work) -> PResult<Vec<Poly>> {
        let mut powers = vec![vec![Q::one()]];
        for i in 0..degree {
            powers.push(uni::multiply(&powers[i], &base, work)?);
        }
        Ok(powers)
    }
    let n = sequence(n, degree, work)?;
    let d = sequence(d, degree, work)?;
    n.iter()
        .zip(d.iter().rev())
        .map(|(n, d)| uni::multiply(n, d, work).map_err(Into::into))
        .collect()
}
