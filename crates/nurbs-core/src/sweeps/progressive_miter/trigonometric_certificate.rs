//! Bounded outward-rounded sin/cos enclosures for sweep frame certification.
//! No platform sin/cos value is treated as an error-free reference. Pi is
//! enclosed with Machin's identity and alternating rational arctangent series.
use crate::{Result, check, distance_bounds::Interval as I};
use std::sync::OnceLock;
#[derive(Clone, Debug)]
pub struct Report {
    pub sin: [f64; 2],
    pub cos: [f64; 2],
    /// False means the certified global [-1,1] range was needed. It is not a
    /// tight enclosure and must not be substituted for budget acceptance.
    pub reduced: bool,
}
fn atan_reciprocal(denominator: f64) -> Result<I> {
    let x = I::point(1.).div(I::point(denominator))?;
    let square = x.mul(x)?;
    let mut power = x;
    let mut sum = I::point(0.);
    for k in 0..24 {
        let term = power.div(I::point((2 * k + 1) as f64))?;
        sum = if k % 2 == 0 {
            sum.add(term)?
        } else {
            sum.sub(term)?
        };
        power = power.mul(square)?;
    }
    // The next omitted alternating term bounds the error for 0 < x < 1.
    let remainder = power.div(I::point(49.))?.hi;
    sum.add(I::new(-remainder, remainder)?)
}
fn quarter_turn() -> Result<I> {
    static PI_HALF: OnceLock<Result<I>> = OnceLock::new();
    PI_HALF
        .get_or_init(|| {
            atan_reciprocal(5.)?
                .mul(I::point(8.))?
                .sub(atan_reciprocal(239.)?.mul(I::point(2.))?)
        })
        .clone()
}
fn remainder(radius: f64, degree: usize) -> Result<f64> {
    let mut value = I::point(1.);
    for k in 1..=degree {
        value = value.mul(I::point(radius))?.div(I::point(k as f64))?;
    }
    Ok(value.hi)
}
fn taylor(x: I) -> Result<(I, I)> {
    let square = x.mul(x)?;
    let mut sin = x;
    let mut sin_term = x;
    let mut cos = I::point(1.);
    let mut cos_term = cos;
    for k in 1..=12 {
        sin_term = sin_term
            .mul(square)?
            .div(I::point(((2 * k) * (2 * k + 1)) as f64))?;
        cos_term = cos_term
            .mul(square)?
            .div(I::point(((2 * k - 1) * (2 * k)) as f64))?;
        if k % 2 == 1 {
            sin = sin.sub(sin_term)?;
            cos = cos.sub(cos_term)?;
        } else {
            sin = sin.add(sin_term)?;
            cos = cos.add(cos_term)?;
        }
    }
    let radius = x.lo.abs().max(x.hi.abs());
    let sr = remainder(radius, 26)?;
    let cr = remainder(radius, 25)?;
    Ok((
        sin.add(I::new(-sr, sr)?)?.intersect(-1., 1.)?,
        cos.add(I::new(-cr, cr)?)?.intersect(-1., 1.)?,
    ))
}
fn negative(x: I) -> I {
    I {
        lo: -x.hi,
        hi: -x.lo,
    }
}
/// Encloses mathematical sin/cos at every real angle in the given binary64
/// interval. Any chosen integer quarter-turn reduction is an exact identity;
/// its nearest-quadrant selection is only an accuracy heuristic.
pub fn certify(angle: [f64; 2]) -> Result<Report> {
    check(
        angle[0].is_finite() && angle[1].is_finite() && angle[0] <= angle[1],
        "Trigonometric certificate requires a finite ordered interval",
    )?;
    let wide = || Report {
        sin: [-1., 1.],
        cos: [-1., 1.],
        reduced: false,
    };
    if angle[1] - angle[0] > 1. || angle[0].abs().max(angle[1].abs()) > 1e12 {
        return Ok(wide());
    }
    let midpoint = angle[0] * 0.5 + angle[1] * 0.5;
    let turns = (midpoint / std::f64::consts::FRAC_PI_2).round();
    // The bounded integer is exactly representable; no uncertainty is hidden
    // in its multiplication by the independently enclosed mathematical pi/2.
    let reduced = I::new(angle[0], angle[1])?.sub(quarter_turn()?.mul(I::point(turns))?)?;
    if reduced.lo.abs().max(reduced.hi.abs()) > 2. {
        return Ok(wide());
    }
    let (s, c) = taylor(reduced)?;
    let (s, c) = match (turns as i64).rem_euclid(4) {
        0 => (s, c),
        1 => (c, negative(s)),
        2 => (negative(s), negative(c)),
        _ => (negative(c), s),
    };
    Ok(Report {
        sin: [s.lo, s.hi],
        cos: [c.lo, c.hi],
        reduced: true,
    })
}
/// Encloses the principal atan2 angle of a rectangle. Rectangles touching
/// the origin or crossing the negative-axis branch cut are unresolved.
/// No platform atan/atan2 result participates in the enclosure.
pub fn certify_atan2(y: [f64; 2], x: [f64; 2]) -> Result<Option<[f64; 2]>> {
    let y = I::new(y[0], y[1])?;
    let x = I::new(x[0], x[1])?;
    if (x.lo <= 0. && x.hi >= 0. && y.lo <= 0. && y.hi >= 0.)
        || (x.lo < 0. && y.lo < 0. && y.hi >= 0.)
    {
        return Ok(None);
    }
    fn atan_small(z: I) -> Result<I> {
        let square = z.mul(z)?;
        let mut power = z;
        let mut sum = I::point(0.);
        for k in 0..80 {
            let term = power.div(I::point((2 * k + 1) as f64))?;
            sum = if k % 2 == 0 {
                sum.add(term)?
            } else {
                sum.sub(term)?
            };
            power = power.mul(square)?;
        }
        let rem = power.lo.abs().max(power.hi.abs());
        let rem = I::point(rem).div(I::point(161.))?.hi;
        sum.add(I::new(-rem, rem)?)
    }
    fn first_quadrant(a: f64, b: f64) -> Result<I> {
        if b == 0. {
            return quarter_turn();
        }
        let inverted = a > b;
        let z = if inverted {
            I::point(b).div(I::point(a))?
        } else {
            I::point(a).div(I::point(b))?
        };
        let v = if z.hi <= 0.5 {
            atan_small(z)?
        } else {
            let reduced = z.sub(I::point(1.))?.div(z.add(I::point(1.))?)?;
            quarter_turn()?
                .mul(I::point(0.5))?
                .add(atan_small(reduced)?)?
        };
        if inverted {
            quarter_turn()?.sub(v)
        } else {
            Ok(v)
        }
    }
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for yy in [y.lo, y.hi] {
        for xx in [x.lo, x.hi] {
            let mut angle = first_quadrant(yy.abs(), xx.abs())?;
            if xx < 0. {
                angle = quarter_turn()?.mul(I::point(2.))?.sub(angle)?;
            }
            if yy < 0. {
                angle = negative(angle);
            }
            lo = lo.min(angle.lo);
            hi = hi.max(angle.hi);
        }
    }
    // Away from origin and branch cut, angle extrema on a rectangle occur
    // at corners: each edge has a derivative of constant sign.
    Ok(Some([lo, hi]))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn contains(range: [f64; 2], value: f64) {
        assert!(
            range[0] <= value && value <= range[1],
            "{value} outside {range:?}"
        );
    }
    #[test]
    fn independently_encloses_pi_and_exact_zero() {
        let p = quarter_turn().unwrap();
        assert!(p.lo < 1.5707963267948966 && p.hi > 1.5707963267948966);
        assert!(p.hi - p.lo < 1e-12);
        let r = certify([0., 0.]).unwrap();
        contains(r.sin, 0.);
        contains(r.cos, 1.);
        assert!(r.sin[1] - r.sin[0] < 1e-12);
    }
    #[test]
    fn interval_covers_all_quadrants_and_interior_probes() {
        for center in [
            -10000., -6.3, -3.2, -1.57, -0.01, 0.78, 1.57, 3.14, 6.28, 10000.,
        ] {
            let lo = center - 0.013;
            let hi = center + 0.019;
            let r = certify([lo, hi]).unwrap();
            assert!(r.reduced);
            for i in 0..101 {
                let x = lo + (hi - lo) * i as f64 / 100.;
                contains(r.sin, x.sin());
                contains(r.cos, x.cos());
            }
        }
    }
    #[test]
    fn wide_and_huge_angles_keep_a_certified_global_range() {
        for angle in [[-10., 10.], [1e15, 1e15], [f64::MAX, f64::MAX]] {
            let r = certify(angle).unwrap();
            assert!(!r.reduced);
            assert_eq!(r.sin, [-1., 1.]);
            assert_eq!(r.cos, [-1., 1.]);
        }
        assert!(certify([f64::NAN, 1.]).is_err());
        assert!(certify([2., 1.]).is_err());
    }
    #[test]
    fn narrow_many_turns_enclose_reference_without_assuming_libm_rounding() {
        for x in [-1e8, -1e6, -100., 0.1, 100., 1e6, 1e8] {
            let r = certify([x, x]).unwrap();
            contains(r.sin, x.sin());
            contains(r.cos, x.cos());
            assert!(r.sin[1] - r.sin[0] < 1e-4);
        }
    }
    #[test]
    fn atan2_rectangles_cover_quadrants_axes_and_unresolved_branch() {
        for (y, x) in [
            ([0.2, 0.3], [1., 2.]),
            ([-0.3, -0.2], [1., 2.]),
            ([0.2, 0.3], [-2., -1.]),
            ([-0.3, -0.2], [-2., -1.]),
            ([1., 2.], [-0.1, 0.1]),
            ([-2., -1.], [-0.1, 0.1]),
            ([-0.1, 0.1], [1., 2.]),
            ([0., 0.], [-2., -1.]),
        ] {
            let r = certify_atan2(y, x).unwrap().unwrap();
            for i in 0..11 {
                for j in 0..11 {
                    let yy = y[0] + (y[1] - y[0]) * i as f64 / 10.;
                    let xx = x[0] + (x[1] - x[0]) * j as f64 / 10.;
                    contains(r, yy.atan2(xx));
                }
            }
        }
        assert!(certify_atan2([-1., 1.], [-2., -1.]).unwrap().is_none());
        assert!(certify_atan2([0., 1.], [0., 1.]).unwrap().is_none());
        let r = certify_atan2([1., 1.], [1., 1.]).unwrap().unwrap();
        let pi4 = quarter_turn().unwrap().mul(I::point(0.5)).unwrap();
        assert!(r[0] <= pi4.lo && r[1] >= pi4.hi);
        assert!(r[1] - r[0] < 1e-12);
    }
}
