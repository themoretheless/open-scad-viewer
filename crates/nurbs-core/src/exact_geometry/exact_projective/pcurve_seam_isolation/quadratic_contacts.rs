//! Canonical algebraic clock values for contacts of degree at most two.
use super::super::chart_nonnegative::{evaluate, sum};
use super::{
    PResult, Poly, Q, Work, active_spans, line_equation, original_span, rational_interior_roots,
};
use crate::{
    curve::Curve,
    exact_geometry::{
        exact_derivative::Refusal,
        rational_factor::polynomial::{divide, multiply},
    },
};
use num_traits::{One, Signed, Zero};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlgebraicBoundaryClock {
    Rational(Q),
    /// The selected root of t² + linear·t + constant, with rational coefficients.
    Quadratic {
        linear: Q,
        constant: Q,
        upper_root: bool,
    },
}

pub(crate) fn quadratic_span_contacts(
    curve: &Curve,
    equation: &[Q; 3],
    axis: usize,
    origin: &Q,
    step: &Q,
    complete: bool,
    work: &mut Work,
) -> PResult<Vec<AlgebraicBoundaryClock>> {
    let mut points = Vec::new();
    let one = Q::one();
    let zero = Q::zero();
    for (span, parameter) in active_spans(curve) {
        let homogeneous = original_span(curve, span, parameter, work)?;
        let contact = line_equation(&homogeneous, equation, work)?;
        if complete {
            if contact.is_empty() || contact.len() > 3 {
                return Err(Refusal::UnprovedIdentity);
            }
            let mut rational = rational_interior_roots(&contact, work)?;
            if contact[0].is_zero() {
                rational.push(zero.clone());
            }
            if sum(&contact, work)?.is_zero() {
                rational.push(one.clone());
            }
            for parameter in rational {
                let value = evaluate(&homogeneous[axis], &parameter, work)?;
                let weight = evaluate(&homogeneous[2], &parameter, work)?;
                if !weight.is_positive() {
                    return Err(Refusal::UnprovedDenominator);
                }
                let coordinate = work.op(&value, &weight, |a, b| a / b)?;
                let offset = work.op(&coordinate, origin, |a, b| a - b)?;
                points.push(AlgebraicBoundaryClock::Rational(work.op(
                    &offset,
                    step,
                    |a, b| a / b,
                )?));
            }
        }
        let [constant, linear, lead] = contact.as_slice() else {
            continue;
        };
        let linear = work.op(linear, lead, |a, b| a / b)?;
        let constant = work.op(constant, lead, |a, b| a / b)?;
        if !has_two_irrational_roots(&linear, &constant, work)? {
            continue;
        }
        let polynomial = [constant, linear, one.clone()];
        let mut roots = Vec::with_capacity(2);
        for upper in [false, true] {
            if !root_lt(&polynomial, upper, &zero, work)?
                && root_lt(&polynomial, upper, &one, work)?
            {
                roots.push(upper);
            }
        }
        if roots.is_empty() {
            continue;
        }
        let [a, b] = clock_expression(
            &homogeneous[axis],
            &homogeneous[2],
            &polynomial,
            origin,
            step,
            work,
        )?;
        let image = image_polynomial(&polynomial, &a, &b, work)?;
        for upper in roots {
            points.push(match &image {
                Some([linear, constant]) => AlgebraicBoundaryClock::Quadratic {
                    linear: linear.clone(),
                    constant: constant.clone(),
                    upper_root: upper != b.is_negative(),
                },
                None => AlgebraicBoundaryClock::Rational(a.clone()),
            });
        }
    }
    Ok(points)
}

fn has_two_irrational_roots(linear: &Q, constant: &Q, work: &mut Work) -> PResult<bool> {
    let square = work.op(linear, linear, |a, b| a * b)?;
    let four = Q::from_integer(4.into());
    let offset = work.op(constant, &four, |a, b| a * b)?;
    let discriminant = work.op(&square, &offset, |a, b| a - b)?;
    if !discriminant.is_positive() {
        return Ok(false);
    }
    work.charge(1 + (discriminant.numer().bits() + discriminant.denom().bits()) / 64)?;
    let approx = Q::new(discriminant.numer().sqrt(), discriminant.denom().sqrt());
    Ok(work.op(&approx, &approx, |a, b| a * b)? != discriminant)
}

fn root_lt([constant, linear, _]: &[Q; 3], upper: bool, q: &Q, work: &mut Work) -> PResult<bool> {
    let two = Q::from_integer(2.into());
    let doubled = work.op(q, &two, |a, b| a * b)?;
    let left_of_vertex = doubled < -linear;
    if upper == left_of_vertex {
        return Ok(!upper);
    }
    let term = work.op(q, linear, |a, b| a + b)?;
    let product = work.op(q, &term, |a, b| a * b)?;
    let value = work.op(&product, constant, |a, b| a + b)?;
    Ok(if upper {
        value.is_positive()
    } else {
        value.is_negative()
    })
}

// Reduce N/W modulo the irreducible contact polynomial. The conjugate
// of d + e·t is (d - e·linear) - e·t, and their product is rational.
fn clock_expression(
    numerator: &Poly,
    weights: &Poly,
    polynomial: &[Q; 3],
    origin: &Q,
    step: &Q,
    work: &mut Work,
) -> PResult<[Q; 2]> {
    let divisor = polynomial.to_vec();
    let (_, mut weight) = divide(weights, &divisor, work)?;
    weight.resize(2, Q::zero());
    let e_linear = work.op(&weight[1], &polynomial[1], |a, b| a * b)?;
    let conjugate = vec![work.op(&weight[0], &e_linear, |a, b| a - b)?, -&weight[1]];
    let (_, norm) = divide(&multiply(&weight, &conjugate, work)?, &divisor, work)?;
    let [norm] = norm.as_slice() else {
        return Err(Refusal::UnprovedDenominator);
    };
    let denominator = work.op(norm, step, |a, b| a * b)?;
    let (_, mut numerator) = divide(&multiply(numerator, &conjugate, work)?, &divisor, work)?;
    numerator.resize(2, Q::zero());
    let offset = work.op(norm, origin, |a, b| a * b)?;
    let intercept = work.op(&numerator[0], &offset, |a, b| a - b)?;
    Ok([
        work.op(&intercept, &denominator, |a, b| a / b)?,
        work.op(&numerator[1], &denominator, |a, b| a / b)?,
    ])
}

fn image_polynomial(
    [constant, linear, _]: &[Q; 3],
    a: &Q,
    b: &Q,
    work: &mut Work,
) -> PResult<Option<[Q; 2]>> {
    if b.is_zero() {
        return Ok(None);
    }
    let linear_b = work.op(linear, b, |a, b| a * b)?;
    let doubled = work.op(a, a, |a, b| a + b)?;
    let image_linear = work.op(&linear_b, &doubled, |a, b| a - b)?;
    let ab = work.op(a, &linear_b, |a, b| a - b)?;
    let left = work.op(a, &ab, |a, b| a * b)?;
    let b2 = work.op(b, b, |a, b| a * b)?;
    let right = work.op(constant, &b2, |a, b| a * b)?;
    let image_constant = work.op(&left, &right, |a, b| a + b)?;
    Ok(Some([image_linear, image_constant]))
}
