//! Exact positive Möbius changes of parameter on original Bézier spans.
//!
//! Multiplying the i-th homogeneous coefficient by r^i substitutes
//! r*u/(1-u+r*u) in the Bernstein quotient. A common factor on the
//! whole span is immaterial. These operations preserve the oriented
//! image of every span and hence the path in normalized physical arc.
use super::*;

fn exponent(w: f64) -> Option<i32> {
    (w.is_normal() && w > 0.).then(|| ((w.to_bits() >> 52) & 2047) as i32)
}

fn shift_weight(w: f64, shift: i32) -> Option<f64> {
    let next = exponent(w)?.checked_add(shift)?;
    if !(1..2047).contains(&next) {
        return None;
    }
    // Both exponents describe normal binary64 values. Replacing only
    // the exponent bits is an exact multiplication by a power of two.
    let fraction = w.to_bits() & ((1_u64 << 52) - 1);
    Some(f64::from_bits(fraction | (next as u64) << 52))
}

fn condition(path: &Curve, spent: &mut usize, max: usize) -> Option<Curve> {
    let degree = i32::try_from(path.degree).ok()?;
    if degree == 0 || path.periodic {
        return None;
    }
    let first = exponent(*path.weights.first()?)?;
    let last = exponent(*path.weights.last()?)?;
    // A multispan loop may gain and then lose the same exponent.
    if path.weights.len() == path.degree + 1 && (last - first).abs() < degree {
        return None;
    }
    let charge = path.weights.len().checked_add(path.knots.len())?;
    if charge > max.checked_sub(*spent)? {
        return None;
    }
    *spent += charge;

    let p = path.degree;
    let n = path.control_points.len();
    let [a, b] = path.domain();
    if (n - 1) % p != 0
        || path.knots[..=p].iter().any(|x| *x != a)
        || path.knots[n..].iter().any(|x| *x != b)
    {
        return None;
    }
    let mut weights = path.weights.clone();
    let mut changed = false;
    let mut gauge = 0_i32;
    for head in (0..n - 1).step_by(p) {
        let left = path.knots[head + p];
        let right = path.knots[head + p + 1];
        if left >= right || path.knots[head + 1..=head + p].iter().any(|x| *x != left) {
            return None;
        }
        let tail = head + p;
        let slope = (exponent(path.weights[tail])? - exponent(path.weights[head])?) / degree;
        changed |= slope != 0;
        for i in 0..=p {
            let shift = gauge.checked_sub(slope.checked_mul(i as i32)?)?;
            weights[head + i] = shift_weight(path.weights[head + i], shift)?;
        }
        // Match the weight at the endpoint shared with the next span.
        gauge = gauge.checked_sub(slope.checked_mul(degree)?)?;
    }
    if !changed {
        return None;
    }
    Some(Curve {
        weights,
        ..path.clone()
    })
}

pub(super) fn conditioned(
    s: &Sweep<'_>,
    closed: bool,
    spent: &mut usize,
    max: usize,
) -> Option<Curve> {
    if !closed
        || s.options.spacing == Spacing::Parameter
        || s.options.orientation != Orientation::Frenet
        || s.frame_laws.is_some()
        || s.orientation_guide.is_some()
        || s.contact_point.is_some()
    {
        return None;
    }
    condition(s.path, spent, max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conditioning_preserves_shared_bezier_endpoints_and_refuses_rounding() {
        let path = Curve {
            degree: 2,
            knots: vec![-3., -3., -3., 1., 1., 5., 5., 5.],
            control_points: vec![
                vec![0., 0., 0.],
                vec![1., 0., 0.],
                vec![2., 1., 0.],
                vec![1., 2., 1.],
                vec![0., 0., 0.],
            ],
            weights: vec![1., 2., 4., 16., 64.],
            periodic: false,
        };
        let charge = path.weights.len() + path.knots.len();
        let mut spent = 0;
        assert!(condition(&path, &mut spent, charge - 1).is_none());
        assert_eq!(spent, 0);
        let normalized = condition(&path, &mut spent, charge).unwrap();
        assert_eq!(spent, charge);
        assert_eq!(normalized.weights, vec![1.; 5]);
        assert_eq!(normalized.knots, path.knots);
        assert_eq!(normalized.control_points, path.control_points);
        normalized.validate().unwrap();

        let mut returning_clock = path.clone();
        returning_clock.weights = vec![1., 2., 4., 2., 1.];
        let mut spent = 0;
        let normalized = condition(&returning_clock, &mut spent, charge).unwrap();
        assert_eq!(normalized.weights, vec![1.; 5]);
        assert_eq!(spent, charge);

        let mut wider_basis = path.clone();
        wider_basis.knots[4] = 2.;
        let mut spent = 0;
        assert!(condition(&wider_basis, &mut spent, charge).is_none());
        assert!(shift_weight(f64::MIN_POSITIVE, -1).is_none());
        assert!(shift_weight(f64::MAX, 1).is_none());
    }
}
