//! Preserve the affine denominator dependency of an inverse homography.
use super::*;
pub(super) fn fraction(q: I, weights: [f64; 2]) -> Result<I> {
    let w0 = I::point(weights[0]);
    let w1 = I::point(weights[1]);
    let numerator = q.mul(w0)?;
    // (1-q)*w1+q*w0 is mathematically affine in q. Independent interval
    // evaluation of its two terms loses positivity on a full [0,1] input,
    // including one widened by outward endpoint normalization. Do not clip q:
    // genuine retained-domain tails and real inverse poles must remain visible.
    let mut denominator = w1.add(q.mul(w0.sub(w1)?)?)?;
    if q.lo >= 0. && q.hi <= 1. {
        denominator =
            denominator.intersect(weights[0].min(weights[1]), weights[0].max(weights[1]))?;
    }
    numerator.div_signed(denominator)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full_interval_with_outward_endpoint_slack_preserves_positive_denominator() {
        for weights in [[1., 1.], [1., 2.], [2., 1.], [0.5, 4.]] {
            let r = fraction(I::new(-f64::EPSILON, 1. + f64::EPSILON).unwrap(), weights).unwrap();
            assert!(r.lo <= 0. && r.hi >= 1.);
        }
    }
    #[test]
    fn unclipped_tails_remain_visible_and_contain_the_inverse() {
        for weights in [[1., 2.], [2., 1.], [3., 5.]] {
            for [lo, hi] in [[-0.25, -0.125], [1.125, 1.25]] {
                let r = fraction(I::new(lo, hi).unwrap(), weights).unwrap();
                for i in 0..=16 {
                    let q = lo + (hi - lo) * i as f64 / 16.;
                    let t = q * weights[0] / (weights[1] + q * (weights[0] - weights[1]));
                    assert!(r.lo <= t && t <= r.hi);
                }
                if lo > 1. {
                    assert!(r.hi > 1.);
                } else {
                    assert!(r.lo < 0.);
                }
            }
        }
    }
    #[test]
    fn actual_inverse_poles_and_crossing_intervals_still_refuse() {
        for (q, w) in [
            (I::point(2.), [1., 2.]),
            (I::point(-1.), [2., 1.]),
            (I::new(-2., 3.).unwrap(), [1., 2.]),
        ] {
            assert!(fraction(q, w).is_err());
        }
    }
}
