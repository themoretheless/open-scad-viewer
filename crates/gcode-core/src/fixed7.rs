//! Exact fixed-point formatting of extrusion values: bit-identical to
//! `format!("{v:.7}")` but without a per-move String allocation or float
//! re-parse.

use crate::{Result, number};

/// Writes `format!("{v:.7}")` into `buf` and returns the value in 1e-7 mm
/// units, so monotonicity checks become integer comparisons.
///
/// Fast path (`|v| < 5e8`): the binary64 value is scaled by 1e7 with exact
/// u128 arithmetic and rounded half-to-even on its exact decimal expansion —
/// the same rounding std's float formatter applies — so the emitted text is
/// identical. Below 5e8 adjacent 1e-7 decimals are wider apart than one ulp,
/// which makes parsing injective there; comparing the returned integer units
/// is therefore equivalent to comparing the re-parsed f64 values the previous
/// code used. Outside that range (unreachable within the output budget in
/// practice) we fall back to the std formatter plus a re-parse, preserving
/// the exact old behavior including the `GCODE_INVALID_NUMBER` error.
pub(crate) fn push_fixed7(buf: &mut String, v: f64) -> Result<i64> {
    if v.abs() < 5e8 {
        // NaN and infinities fail this bound and take the fallback.
        return Ok(push_fixed7_exact(buf, v));
    }
    let text = format!("{v:.7}");
    let parsed = number(&text)?;
    buf.push_str(&text);
    // |parsed| >= 5e8 keeps the relative round-trip error far below half a
    // unit, so this recovers the exact units of the emitted decimal.
    Ok((parsed * 1e7).round() as i64)
}

fn push_fixed7_exact(buf: &mut String, v: f64) -> i64 {
    use std::fmt::Write;
    let negative = v.is_sign_negative();
    let bits = v.abs().to_bits();
    let biased = ((bits >> 52) & 0x7ff) as i32;
    let (mantissa, exp2) = if biased == 0 {
        (bits & 0xf_ffff_ffff_ffff, -1074)
    } else {
        ((bits & 0xf_ffff_ffff_ffff) | 0x10_0000_0000_0000, biased - 1075)
    };
    // The exact value is mantissa * 2^exp2; scale by 1e7, round half-to-even.
    let scaled = mantissa as u128 * 10_000_000;
    let units = if exp2 >= 0 {
        scaled << exp2 // |v| < 5e8 keeps this below 2^63
    } else {
        let shift = (-exp2) as u32;
        if shift >= 128 {
            0
        } else {
            let quotient = scaled >> shift;
            let remainder = scaled - (quotient << shift);
            let half = 1u128 << (shift - 1);
            if remainder > half || (remainder == half && quotient & 1 == 1) {
                quotient + 1
            } else {
                quotient
            }
        }
    };
    let units = units as u64;
    if negative {
        // Covers -0.0 and tiny negatives rounding to zero: std prints the
        // sign whenever the sign bit is set.
        buf.push('-');
    }
    write!(buf, "{}.{:07}", units / 10_000_000, units % 10_000_000)
        .expect("writing to a String cannot fail");
    if negative {
        -(units as i64)
    } else {
        units as i64
    }
}

#[cfg(test)]
mod tests {
    use super::push_fixed7;

    fn check(v: f64) {
        let mut buf = String::new();
        let units = push_fixed7(&mut buf, v).unwrap();
        let reference = format!("{v:.7}");
        assert_eq!(buf, reference, "text mismatch for {v:e} ({v:?})");
        // Oracle parses the decimal text exactly (integer arithmetic), unlike
        // an f64 round-trip which double-rounds at large magnitudes.
        let (int_part, frac_part) = reference.split_once('.').unwrap();
        let negative = int_part.starts_with('-');
        let int_abs: i64 = int_part.trim_start_matches('-').parse().unwrap();
        let mut expected = int_abs * 10_000_000 + frac_part.parse::<i64>().unwrap();
        if negative {
            expected = -expected;
        }
        assert_eq!(units, expected, "units mismatch for {v:e} ({v:?})");
    }

    #[test]
    fn matches_std_format_edge_cases() {
        for v in [
            0.0,
            -0.0,
            1.0,
            -1.0,
            0.1,
            0.2,
            1e-9,
            -1e-9,
            4.9e-324,
            -4.9e-324,
            2.2250738585072014e-308,
            0.00390625,   // exact half-unit tie, rounds to even (down)
            0.01171875,   // exact half-unit tie, rounds to even (up)
            0.01953125,   // exact half-unit tie, odd quotient
            5e-8,
            1.5e-7,
            0.0078125,    // exactly representable at 7 decimals
            123456.78901234567,
            1e6,
            1e7,
            499_999_999.99999994,
            499_999_999.5,
            4.99999999e8,
        ] {
            check(v);
        }
    }

    #[test]
    fn matches_std_format_decimal_sweep() {
        // Dense sweep of exactly representable-ish decimals near the unit grid.
        for u in (0..200_000i64).step_by(1) {
            check(u as f64 * 1e-7);
            check(u as f64 / 10_000_000.0);
        }
        for u in (0..100_000i64).step_by(7) {
            check(u as f64 * 0.1);
            check(u as f64 * 1e-7 * 1.5);
        }
    }

    #[test]
    fn matches_std_format_random_bit_patterns() {
        let mut state = 0x9e37_79b9_7f4a_7c15u64;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let mut checked = 0;
        while checked < 300_000 {
            let v = f64::from_bits(next());
            if !v.is_finite() || v.abs() >= 5e8 {
                continue;
            }
            check(v);
            checked += 1;
        }
    }
}
