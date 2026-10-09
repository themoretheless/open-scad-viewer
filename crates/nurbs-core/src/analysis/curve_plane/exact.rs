//! Exact homogeneous plane residuals and knot contact identities.
use super::*;

// Exact binary64 products accumulated as integer magnitudes. The smallest
// product exponent is -2148; up to eight finite products fit in 68 u64 limbs.
pub(super) fn exact_sum_zero(terms: &[(f64, f64)]) -> bool {
    crate::exact_products::sum_sign(terms) == std::cmp::Ordering::Equal
}
pub(super) fn exact_plane_zero(p: &[f64], n: [f64; 3], o: f64) -> bool {
    exact_sum_zero(&[(n[0], p[0]), (n[1], p[1]), (n[2], p[2]), (-o, 1.)])
}
// Four binary64 factors have at most 212 mantissa bits. Their exact sum
// occupies at most 8396 bits, including carry for the sixteen terms below.
pub(super) fn exact_four_products_zero(terms: &[[f64; 4]]) -> bool {
    let mut positive = [0_u64; 136];
    let mut negative = [0_u64; 136];
    for term in terms {
        let mut product = [0_u64; 4];
        product[0] = 1;
        let mut exponent = 0_i32;
        let mut sign = false;
        for x in term {
            let bits = x.to_bits();
            sign ^= bits >> 63 != 0;
            let e = ((bits >> 52) & 2047) as i32;
            let m = (bits & ((1_u64 << 52) - 1)) | if e == 0 { 0 } else { 1_u64 << 52 };
            exponent += if e == 0 { -1074 } else { e - 1075 };
            let mut carry = 0_u128;
            for limb in &mut product {
                let value = u128::from(*limb) * u128::from(m) + carry;
                *limb = value as u64;
                carry = value >> 64;
            }
            debug_assert_eq!(carry, 0);
        }
        let target = if sign { &mut negative } else { &mut positive };
        let shift = (exponent + 4296) as usize;
        for (index, limb) in product.iter().enumerate() {
            for bit in 0..64 {
                if limb & (1_u64 << bit) == 0 {
                    continue;
                }
                let position = shift + index * 64 + bit;
                let mut index = position / 64;
                let mut carry = 1_u64 << (position % 64);
                loop {
                    let (sum, overflow) = target[index].overflowing_add(carry);
                    target[index] = sum;
                    if !overflow {
                        break;
                    }
                    index += 1;
                    carry = 1;
                }
            }
        }
    }
    positive == negative
}
pub(super) fn exact_linear_parameter(c: &Curve, span: usize, t: f64, n: [f64; 3], o: f64) -> bool {
    if c.degree != 1 {
        return false;
    }
    let [lo, hi] = [c.knots[span], c.knots[span + 1]];
    let [wa, wb] = [c.weights[span - 1], c.weights[span]];
    let a = &c.control_points[span - 1];
    let b = &c.control_points[span];
    exact_weighted_pair(a, b, wa, wb, lo, hi, t, n, o)
}
pub(super) fn exact_quadratic_knot(c: &Curve, t: f64, n: [f64; 3], o: f64) -> bool {
    if c.degree != 2 {
        return false;
    }
    let Some(k) = (2..=c.control_points.len()).find(|&k| c.knots[k] == t) else {
        return false;
    };
    if c.knots[k - 1] >= t || c.knots[k + 1] <= t {
        return false;
    }
    // At a simple quadratic knot only these two basis functions survive.
    // Their common denominator U[k+1]-U[k-1] is strictly positive.
    exact_weighted_pair(
        &c.control_points[k - 2],
        &c.control_points[k - 1],
        c.weights[k - 2],
        c.weights[k - 1],
        c.knots[k - 1],
        c.knots[k + 1],
        t,
        n,
        o,
    )
}
#[allow(clippy::too_many_arguments)]
pub(super) fn exact_weighted_pair(
    a: &[f64],
    b: &[f64],
    wa: f64,
    wb: f64,
    lo: f64,
    hi: f64,
    t: f64,
    n: [f64; 3],
    o: f64,
) -> bool {
    // Exact homogeneous residual: wa(hi-t)(n·a-o)+wb(t-lo)(n·b-o).
    // Expanding products avoids rounded differences and constructed points.
    let mut terms = [[0.; 4]; 16];
    for axis in 0..3 {
        terms[axis * 4] = [wa, hi, n[axis], a[axis]];
        terms[axis * 4 + 1] = [-wa, t, n[axis], a[axis]];
        terms[axis * 4 + 2] = [wb, t, n[axis], b[axis]];
        terms[axis * 4 + 3] = [-wb, lo, n[axis], b[axis]];
    }
    terms[12] = [-wa, hi, o, 1.];
    terms[13] = [wa, t, o, 1.];
    terms[14] = [-wb, t, o, 1.];
    terms[15] = [wb, lo, o, 1.];
    exact_four_products_zero(&terms)
}
