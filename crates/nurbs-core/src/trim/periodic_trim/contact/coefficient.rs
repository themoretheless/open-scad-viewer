//! Powered radial coefficients without fractional powers or root estimates.
use crate::{
    Result,
    curve::Curve,
    distance_bounds::{Interval as I, box_distance},
};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PoweredBendCoefficient {
    /// The reduced denominator of m/k, also of the angular radial exponent.
    pub power: usize,
    /// Outward enclosure of |angular_leading_coefficient|^power.
    pub magnitude: [f64; 2],
}
fn gcd(mut a: usize, mut b: usize) -> usize {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}
fn positive(v: I) -> Result<I> {
    I::new(v.lo.max(0.), v.hi)
}
fn pow(v: I, n: usize) -> Result<I> {
    let mut out = I::point(1.);
    for _ in 0..n {
        out = positive(out.mul(v)?)?;
    }
    Ok(out)
}
fn choose(p: usize, i: usize) -> u64 {
    let mut value = 1u64;
    for j in 1..=i {
        value = value * (p + 1 - j) as u64 / j as u64;
    }
    value
}
/// Leading coefficient of N_i,p in original first-span unit coordinates,
/// multiplied by wi/w0. At a clamped endpoint all earlier offset controls
/// vanish, so no lower-order quotient terms contribute to the leading term.
fn beta(c: &Curve, incoming: bool, i: usize) -> Result<I> {
    let n = c.control_points.len();
    let p = c.degree;
    let index = if incoming { n - 1 - i } else { i };
    let origin = if incoming { n - 1 } else { 0 };
    let [a, b] = c.domain();
    let gap = |j: usize| {
        if incoming {
            I::point(b).sub(I::point(c.knots[n - j]))
        } else {
            I::point(c.knots[p + j]).sub(I::point(a))
        }
    };
    let h1 = gap(1)?;
    let mut out = I::point(c.weights[index])
        .div(I::point(c.weights[origin]))?
        .mul(I::point(choose(p, i) as f64))?;
    // h1/h1 is exactly one. Later knot gaps are at least h1.
    for j in 2..=i {
        out = positive(out.mul(h1.div(gap(j)?)?.intersect(0., 1.)?)?)?;
    }
    Ok(out)
}
pub(super) fn enclose(
    c: &Curve,
    incoming: bool,
    k: usize,
    normal: Option<usize>,
    regular: Option<[f64; 2]>,
) -> Option<PoweredBendCoefficient> {
    if let Some([lo, hi]) = regular {
        let minimum = if lo > 0. {
            lo
        } else if hi < 0. {
            -hi
        } else {
            0.
        };
        return Some(PoweredBendCoefficient {
            power: 1,
            magnitude: [minimum, lo.abs().max(hi.abs())],
        });
    }
    let m = normal?;
    if k == 0 || m <= k {
        return None;
    }
    let common = gcd(m, k);
    let power = k / common;
    let tangent_power = m / common;
    let calculate = || -> Result<I> {
        let n = c.control_points.len();
        let index = |j: usize| if incoming { n - 1 - j } else { j };
        let point = |j: usize| {
            [
                I::point(c.control_points[index(j)][0]),
                I::point(c.control_points[index(j)][1]),
            ]
        };
        let o = point(0);
        let r = point(k);
        let q = point(m);
        let r = [r[0].sub(o[0])?, r[1].sub(o[1])?];
        let q = [q[0].sub(o[0])?, q[1].sub(o[1])?];
        let (lo, hi) = box_distance(&r, &[I::point(0.); 2])?;
        let norm = I::new(lo, hi)?;
        let cross = r[0].mul(q[1])?.sub(r[1].mul(q[0])?)?;
        let minimum = if cross.lo > 0. {
            cross.lo
        } else if cross.hi < 0. {
            -cross.hi
        } else {
            0.
        };
        let absolute = I::new(minimum, cross.lo.abs().max(cross.hi.abs()))?;
        let tangent = positive(beta(c, incoming, k)?.mul(norm)?)?;
        let normal = positive(beta(c, incoming, m)?.mul(absolute)?.div(norm)?)?;
        // radius ~ A*t^k, normal ~ B*t^m, angle ~ C*radius^((m-k)/k).
        // For m/k=M/K, |C|^K=B^K/A^M. No fractional power is computed.
        pow(normal, power)?.div(pow(tangent, tangent_power)?)
    };
    calculate().ok().map(|v| PoweredBendCoefficient {
        power,
        magnitude: [v.lo.max(0.), v.hi],
    })
}
