//! Exact residual Bernstein hulls and a squared-rational upper float search.
use super::*;
fn subtract(a: &Poly, b: &Poly, w: &mut Work) -> PResult<Poly> {
    let mut out = vec![Q::zero(); a.len().max(b.len())];
    for (i, x) in out.iter_mut().enumerate() {
        let zero = Q::zero();
        *x = w.op(
            a.get(i).unwrap_or(&zero),
            b.get(i).unwrap_or(&zero),
            |a, b| a - b,
        )?;
    }
    poly::trim(&mut out);
    Ok(out)
}
pub(super) fn squared_error(
    c: &Curve,
    target: &[Poly],
    denominator: &[Q],
    w: &mut Work,
) -> PResult<Q> {
    let dim = target.len() - 1;
    let mut channels = vec![Vec::new(); dim + 1];
    for (point, &weight) in c.control_points.iter().zip(&c.weights) {
        let weight = w.lift(weight)?;
        for axis in 0..dim {
            let x = w.lift(point[axis])?;
            channels[axis].push(w.op(&x, &weight, |a, b| a * b)?);
        }
        channels[dim].push(weight);
    }
    let lower = w.op(
        denominator.iter().min().unwrap(),
        channels[dim].iter().min().unwrap(),
        |a, b| a * b,
    )?;
    if !lower.is_positive() {
        return Err(Refusal::UnprovedDenominator);
    }
    let stored = channels
        .iter()
        .map(|p| poly::power(p, w).map_err(Refusal::from))
        .collect::<PResult<Vec<_>>>()?;
    let mut sum = Q::zero();
    for axis in 0..dim {
        let a = poly::multiply(&target[axis], &stored[dim], w)?;
        let b = poly::multiply(&stored[axis], &target[dim], w)?;
        let residual = subtract(&a, &b, w)?;
        let degree = residual.len().saturating_sub(1);
        if degree > 50 {
            return Err(Refusal::OutputDegreeLimit);
        }
        let bound = poly::bernstein(&residual, degree, w)?
            .into_iter()
            .map(|x| x.abs())
            .max()
            .unwrap();
        let error = w.op(&bound, &lower, |a, b| a / b)?;
        let squared = w.op(&error, &error, |a, b| a * b)?;
        sum = w.op(&sum, &squared, |a, b| a + b)?;
    }
    Ok(sum)
}
pub(in crate::editing) fn ceil_sqrt(squared: &Q, max_error: f64, w: &mut Work) -> PResult<f64> {
    if squared.is_zero() {
        return Ok(0.);
    }
    // Positive finite f64 bit order is numeric order. Admission already proved
    // max_error² >= squared. Exact comparisons avoid underflow in a float sqrt.
    let (mut low, mut high) = (0u64, max_error.to_bits());
    while high - low > 1 {
        let middle = low + (high - low) / 2;
        let x = w.lift(f64::from_bits(middle))?;
        let square = w.op(&x, &x, |a, b| a * b)?;
        w.charge(
            1 + (square.numer().bits()
                + square.denom().bits()
                + squared.numer().bits()
                + squared.denom().bits())
                / 64,
        )?;
        if square >= *squared {
            high = middle;
        } else {
            low = middle;
        }
    }
    Ok(f64::from_bits(high))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn least_upper(squared: Q, limit: f64) {
        let mut w = Work {
            used: 0,
            limit: 1_000_000,
        };
        let upper = ceil_sqrt(&squared, limit, &mut w).unwrap();
        let exact = Q::from_float(upper).unwrap();
        assert!(&exact * &exact >= squared);
        assert!(upper <= limit);
        if upper > 0. {
            let previous = Q::from_float(upper.next_down()).unwrap();
            assert!(&previous * &previous < squared);
        }
    }
    #[test]
    fn upper_search_handles_subnormal_error_and_underflowing_squared_error() {
        let min = Q::from_float(f64::from_bits(1)).unwrap();
        least_upper(&min * &min, 1e-300);
        least_upper((&min * &min) / Q::from_integer(4.into()), 1e-300);
    }
    #[test]
    fn upper_search_rounds_a_nonbinary_rational_outward() {
        least_upper(Q::new(2.into(), 3.into()), 1.);
    }
    #[test]
    fn upper_search_consumes_the_shared_work_budget() {
        let mut w = Work { used: 0, limit: 1 };
        assert_eq!(
            ceil_sqrt(&Q::new(2.into(), 3.into()), 1., &mut w),
            Err(Refusal::ResourceLimit)
        );
    }
}
