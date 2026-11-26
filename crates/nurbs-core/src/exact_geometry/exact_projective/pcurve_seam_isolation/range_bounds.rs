//! Exact inner and outer contour image bounds on a support line.
use super::{
    PResult, Poly, Q, Work, active_spans, bracket_midpoint, interior_brackets, line_equation,
    original_span, stationary_brackets,
};
use crate::curve::Curve;
use num_traits::{One, Zero};

#[derive(Default)]
pub(crate) struct SpanLineBounds {
    pub(crate) proven: Vec<[Q; 2]>,
    pub(crate) possible: Vec<[Q; 2]>,
}

pub(crate) fn span_line_bounds(
    source: &Curve,
    equation: &[Q; 3],
    axis: usize,
    width: f64,
    work: &mut Work,
) -> PResult<SpanLineBounds> {
    let mut bounds = SpanLineBounds::default();
    let zero = Q::zero();
    let one = Q::one();
    for (span, parameter) in active_spans(source) {
        let homogeneous = original_span(source, span, parameter, work)?;
        let contact = line_equation(&homogeneous, equation, work)?;
        let component = &homogeneous[axis];
        let weight = &homogeneous[2];
        let minimum_weight = source.weights[span - source.degree..=span]
            .iter()
            .copied()
            .reduce(f64::min)
            .expect("active span has weights");
        let minimum_weight = work.lift(minimum_weight)?;
        let coordinate = |range: &[Q; 2], work: &mut Work| {
            let numerator = evaluate_range(component, range, work)?;
            let [lo, hi] = evaluate_range(weight, range, work)?;
            // Active B-spline basis values sum to one and are nonnegative.
            let denominator = [
                lo.max(minimum_weight.clone()),
                hi.max(minimum_weight.clone()),
            ];
            combine(&numerator, &denominator, |n, d| n / d, work)
        };
        if contact.is_empty() {
            let mut proven = coordinate(&[zero.clone(), zero.clone()], work)?;
            extend(&mut proven, coordinate(&[one.clone(), one.clone()], work)?);
            let mut possible = proven.clone();
            for bracket in stationary_brackets(component, weight, width, work)? {
                let witness = bracket_midpoint(&bracket, work)?;
                extend(&mut proven, coordinate(&[witness.clone(), witness], work)?);
                extend(&mut possible, coordinate(&bracket, work)?);
            }
            bounds.proven.push(proven);
            bounds.possible.push(possible);
        } else {
            let mut roots = interior_brackets(&contact, width, work)?;
            if contact[0].is_zero() {
                roots.push([zero.clone(), zero.clone()]);
            }
            if super::super::chart_nonnegative::sum(&contact, work)?.is_zero() {
                roots.push([one.clone(), one.clone()]);
            }
            for bracket in roots {
                let range = coordinate(&bracket, work)?;
                if bracket[0] == bracket[1] {
                    bounds.proven.push(range.clone());
                }
                bounds.possible.push(range);
            }
        }
    }
    Ok(bounds)
}

fn evaluate_range(poly: &Poly, parameter: &[Q; 2], work: &mut Work) -> PResult<[Q; 2]> {
    if parameter[0] == parameter[1] {
        let value = super::super::chart_nonnegative::evaluate(poly, &parameter[0], work)?;
        return Ok([value.clone(), value]);
    }
    let mut coeffs = poly.iter().rev();
    let initial = coeffs.next().cloned().unwrap_or_default();
    let mut range = [initial.clone(), initial];
    for coefficient in coeffs {
        let [lo, hi] = combine(&range, parameter, |a, b| a * b, work)?;
        range = [
            work.op(&lo, coefficient, |a, b| a + b)?,
            work.op(&hi, coefficient, |a, b| a + b)?,
        ];
    }
    Ok(range)
}

fn combine(a: &[Q; 2], b: &[Q; 2], op: impl Fn(&Q, &Q) -> Q, work: &mut Work) -> PResult<[Q; 2]> {
    let mut out = {
        let value = work.op(&a[0], &b[0], &op)?;
        [value.clone(), value]
    };
    for (x, y) in [(&a[0], &b[1]), (&a[1], &b[0]), (&a[1], &b[1])] {
        let value = work.op(x, y, &op)?;
        extend(&mut out, [value.clone(), value]);
    }
    Ok(out)
}

fn extend(range: &mut [Q; 2], [lo, hi]: [Q; 2]) {
    if lo < range[0] {
        range[0] = lo;
    }
    if hi > range[1] {
        range[1] = hi;
    }
}
