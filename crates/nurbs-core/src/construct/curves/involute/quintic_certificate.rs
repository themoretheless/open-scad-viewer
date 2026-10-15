//! Original stored-pole qualification against endpoint-Hermite involute arcs.
use crate::{
    Result, check,
    curve::Curve,
    distance_bounds::{Interval as I, box_distance},
};
#[derive(Clone, Debug)]
pub struct QuinticCertificate {
    pub error_upper: f64,
    pub hermite_remainder_upper: f64,
    pub control_difference_upper: f64,
    pub spans: usize,
    /// The bound includes stored control and knot rounding, and does not rely
    /// on platform sin/cos accuracy. It concerns the exact stored NURBS curve;
    /// rounding of a future caller's point-evaluation algorithm is separate.
    pub rounding_included: bool,
}
fn neg(x: I) -> Result<I> {
    I::new(-x.hi, -x.lo)
}
fn scale(x: I, k: f64) -> Result<I> {
    x.mul(I::point(k))
}
/// Taylor polynomials at zero, with a Lagrange remainder. No libm call is used
/// as evidence. The common degree-63 remainder |x|^64/64! bounds both series.
pub(super) fn trig(x: I) -> Result<(I, I)> {
    check(
        x.lo >= -9. && x.hi <= 9.,
        "Involute certificate angle enclosure must lie within -9..9",
    )?;
    let xx = neg(x.mul(x)?)?;
    let mut st = x;
    let mut ct = I::point(1.);
    let mut sin = st;
    let mut cos = ct;
    for k in 1..32 {
        st = st.mul(xx)?.div(I::point((2 * k * (2 * k + 1)) as f64))?;
        ct = ct.mul(xx)?.div(I::point((2 * k * (2 * k - 1)) as f64))?;
        sin = sin.add(st)?;
        cos = cos.add(ct)?;
    }
    // sin has degree 63, cos degree 62; the degree-63 Taylor polynomial of
    // cosine has the same coefficients because its odd coefficient is zero.
    let a = I::point(x.lo.abs().max(x.hi.abs()));
    let mut remainder = I::point(1.);
    for k in 1..=64 {
        remainder = remainder.mul(a)?.div(I::point(k as f64))?;
    }
    let r = I::new(-remainder.hi, remainder.hi)?;
    Ok((
        sin.add(r)?.intersect(-1., 1.)?,
        cos.add(r)?.intersect(-1., 1.)?,
    ))
}
fn jets(theta: I, r: I, center: [f64; 3]) -> Result<[[I; 3]; 3]> {
    let (s, c) = trig(theta)?;
    let z = I::point(0.);
    Ok([
        [
            I::point(center[0]).add(r.mul(c.add(theta.mul(s)?)?)?)?,
            I::point(center[1]).add(r.mul(s.sub(theta.mul(c)?)?)?)?,
            I::point(center[2]),
        ],
        [r.mul(theta.mul(c)?)?, r.mul(theta.mul(s)?)?, z],
        [
            r.mul(c.sub(theta.mul(s)?)?)?,
            r.mul(s.add(theta.mul(c)?)?)?,
            z,
        ],
    ])
}
/// Enclose the deviation of an actual stored, unit-weight quintic Bezier chain
/// from C(theta)=center+r*(cos(theta)+theta*sin(theta),
/// sin(theta)-theta*cos(theta),0), theta=start+(end-start)*u.
/// The original knots determine every span's true angular width. Maximum
/// pole discrepancy plus an outward endpoint-Hermite remainder bounds the
/// complete curve, not just selected samples. Accepts 1..51 C0 quintic spans.
pub fn certify_quintic(
    curve: &Curve,
    center: [f64; 3],
    radius: f64,
    start: f64,
    end: f64,
) -> Result<QuinticCertificate> {
    certify_impl(curve, center, radius, start, end, None)
}
/// Certify a rotated/reflected stored 2D or 3D involute. The nominal geometry
/// uses the supplied binary64 radius, angles and center as exact input data.
/// This does not certify any earlier derivation of those parameters from a
/// module, tooth count or pressure angle. Reflection must be -1 or +1.
/// Descending start/end angles are supported for reversed flanks.
pub fn certify_quintic_placed(
    curve: &Curve,
    center: [f64; 3],
    radius: f64,
    start: f64,
    end: f64,
    angle: f64,
    reflection: f64,
) -> Result<QuinticCertificate> {
    check(
        angle.is_finite() && angle.abs() <= 8. && (reflection == 1. || reflection == -1.),
        "Placed involute needs an angle within -8..8 and reflection +/-1",
    )?;
    certify_impl(curve, center, radius, start, end, Some((angle, reflection)))
}
fn certify_impl(
    curve: &Curve,
    center: [f64; 3],
    radius: f64,
    start: f64,
    end: f64,
    placement: Option<(f64, f64)>,
) -> Result<QuinticCertificate> {
    check(
        radius.is_finite() && start.is_finite() && end.is_finite(),
        "Quintic parameters must be finite",
    )?;
    certify_boxed(
        curve,
        center,
        I::point(radius),
        I::point(start),
        I::point(end),
        placement.map(|(angle, reflection)| (I::point(angle), reflection)),
    )
}
/// Certify every nominal involute in the supplied parameter intervals, including
/// upstream parameter uncertainty. Boxes must be independently justified by
/// their owner; this function proves actual stored-curve deviation for the whole
/// family and does not trust the platform trigonometric implementation.
pub fn certify_quintic_parameter_box(
    curve: &Curve,
    center: [f64; 3],
    radius: [f64; 2],
    start: [f64; 2],
    end: [f64; 2],
    angle: [f64; 2],
    reflection: f64,
) -> Result<QuinticCertificate> {
    let rotation = I::new(angle[0], angle[1])?;
    check(
        rotation.lo >= -8. && rotation.hi <= 8. && (reflection == 1. || reflection == -1.),
        "Involute family needs bounded placement and reflection +/-1",
    )?;
    certify_boxed(
        curve,
        center,
        I::new(radius[0], radius[1])?,
        I::new(start[0], start[1])?,
        I::new(end[0], end[1])?,
        Some((rotation, reflection)),
    )
}
fn certify_boxed(
    curve: &Curve,
    center: [f64; 3],
    radius: I,
    start: I,
    end: I,
    placement: Option<(I, f64)>,
) -> Result<QuinticCertificate> {
    curve.validate()?;
    check(
        center.iter().all(|x| x.is_finite())
            && radius.lo > 0.
            && start.lo >= -8.
            && start.hi <= 8.
            && end.lo >= -8.
            && end.hi <= 8.
            && (start.hi < end.lo || (placement.is_some() && start.lo > end.hi)),
        "Quintic certification needs finite center, positive radius and -8 <= start < end <= 8",
    )?;
    let count = curve.control_points.len();
    check(
        curve.degree == 5
            && count >= 6
            && (count - 1) % 5 == 0
            && count <= 256
            && curve.weights.iter().all(|w| *w == 1.)
            && !curve.periodic
            && curve
                .control_points
                .iter()
                .all(|p| p.len() == 3 || (placement.is_some() && p.len() == 2 && center[2] == 0.)),
        "Quintic certificate needs 1..51 unit-weight 3D Bezier spans",
    )?;
    let n = (count - 1) / 5;
    let breaks: Vec<_> = (0..=n).map(|i| curve.knots[5 + 5 * i]).collect();
    check(
        breaks[0] == 0. && breaks[n] == 1. && breaks.windows(2).all(|w| w[0] < w[1]),
        "Quintic certificate needs an increasing normalized domain",
    )?;
    let mut expected = vec![0.; 6];
    for &u in &breaks[1..n] {
        expected.extend(std::iter::repeat_n(u, 5));
    }
    expected.extend(std::iter::repeat_n(1., 6));
    check(
        curve.knots == expected,
        "Quintic certificate requires full C0 Bezier span ownership",
    )?;
    let rotation = placement.map(|p| trig(p.0)).transpose()?;
    let reflection = placement.map_or(1., |p| p.1);
    let r = radius;
    let range = end.sub(start)?;
    let mut difference = 0_f64;
    let mut remainder = 0_f64;
    for i in 0..n {
        let theta = |u| start.add(range.mul(I::point(u))?);
        let a = jets(theta(breaks[i])?, r, center)?;
        let b = jets(theta(breaks[i + 1])?, r, center)?;
        let h = range.mul(I::point(breaks[i + 1]).sub(I::point(breaks[i]))?)?;
        let h5 = h.div(I::point(5.))?;
        let hh20 = h.mul(h)?.div(I::point(20.))?;
        let mut poles = [[I::point(0.); 3]; 6];
        for k in 0..3 {
            poles[0][k] = a[0][k];
            poles[5][k] = b[0][k];
            poles[1][k] = a[0][k].add(h5.mul(a[1][k])?)?;
            poles[2][k] = a[0][k]
                .add(scale(h5.mul(a[1][k])?, 2.)?)?
                .add(hh20.mul(a[2][k])?)?;
            poles[3][k] = b[0][k]
                .sub(scale(h5.mul(b[1][k])?, 2.)?)?
                .add(hh20.mul(b[2][k])?)?;
            poles[4][k] = b[0][k].sub(h5.mul(b[1][k])?)?;
        }
        if let Some((sin, cos)) = rotation {
            for p in &mut poles {
                let x = p[0];
                let y = scale(p[1], reflection)?;
                p[0] = cos.mul(x)?.sub(sin.mul(y)?)?;
                p[1] = sin.mul(x)?.add(cos.mul(y)?)?;
            }
        }
        for j in 0..6 {
            let mut stored: Vec<_> = curve.control_points[5 * i + j]
                .iter()
                .map(|x| I::point(*x))
                .collect();
            if stored.len() == 2 {
                stored.push(I::point(0.));
            }
            difference = difference.max(box_distance(&stored, &poles[j])?.1);
        }
        let ta = theta(breaks[i])?;
        let tb = theta(breaks[i + 1])?;
        let t = I::new(ta.lo.min(tb.lo), ta.hi.max(tb.hi))?;
        let norm = box_distance(&[I::point(5.), t], &[I::point(0.); 2])?.1;
        let sqrt2 = box_distance(&[I::point(1.); 2], &[I::point(0.); 2])?.1;
        let h2 = h.mul(h)?;
        let bound = r
            .mul(I::point(norm))?
            .mul(h2)?
            .mul(h2)?
            .mul(h2)?
            .mul(I::point(sqrt2))?
            .div(I::point(46080.))?;
        remainder = remainder.max(bound.hi);
    }
    Ok(QuinticCertificate {
        error_upper: I::point(difference).add(I::point(remainder))?.hi,
        hermite_remainder_upper: remainder,
        control_difference_upper: difference,
        spans: n,
        rounding_included: true,
    })
}
