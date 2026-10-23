//! Exact sign of a small sum of original finite binary64 products.
pub(crate) fn sum_sign(terms: &[(f64, f64)]) -> std::cmp::Ordering {
    debug_assert!(terms.len() <= 8 && terms.iter().all(|(a, b)| a.is_finite() && b.is_finite()));
    fn parts(x: f64) -> (bool, u64, i32) {
        let bits = x.to_bits();
        let e = ((bits >> 52) & 2047) as i32;
        let m = bits & ((1_u64 << 52) - 1);
        (
            bits >> 63 != 0,
            if e == 0 { m } else { m | (1_u64 << 52) },
            if e == 0 { -1074 } else { e - 1075 },
        )
    }
    let mut positive = [0_u64; 68];
    let mut negative = [0_u64; 68];
    for &(a, b) in terms {
        let (sa, ma, ea) = parts(a);
        let (sb, mb, eb) = parts(b);
        let product = u128::from(ma) * u128::from(mb);
        let target = if sa == sb {
            &mut positive
        } else {
            &mut negative
        };
        for bit in 0..106 {
            if product & (1_u128 << bit) == 0 {
                continue;
            }
            let position = (ea + eb + 2148) as usize + bit;
            let mut limb = position / 64;
            let mut carry = 1_u64 << (position % 64);
            loop {
                let (sum, overflow) = target[limb].overflowing_add(carry);
                target[limb] = sum;
                if !overflow {
                    break;
                }
                limb += 1;
                carry = 1;
            }
        }
    }
    positive.iter().rev().cmp(negative.iter().rev())
}
#[cfg(test)]
mod tests {
    use super::sum_sign;
    use std::cmp::Ordering::*;
    #[test]
    fn exact_sign_retains_residual_after_cancellation_underflow_and_overflow() {
        let tiny = f64::from_bits(1);
        assert_eq!(
            sum_sign(&[(1e9, 1.), (1., 2_f64.powi(-40)), (-1e9, 1.)]),
            Greater
        );
        assert_eq!(sum_sign(&[(tiny, tiny)]), Greater);
        assert_eq!(sum_sign(&[(-tiny, tiny)]), Less);
        assert_eq!(
            sum_sign(&[(1e308, 1e308), (-1e308, 1e308), (tiny, 1.)]),
            Greater
        );
        assert_eq!(sum_sign(&[(1e308, 1e308), (-1e308, 1e308)]), Equal);
    }
}
