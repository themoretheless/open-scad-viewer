//! Inclusive saved-net bounds for Z-axis helical sweeps.
use crate::{
    Result, check,
    curve::Curve,
    distance_bounds::{Interval as I, box_distance},
    surface::Surface,
};
#[derive(Clone, Debug)]
pub struct Certificate {
    pub error_upper: f64,
    pub control_difference_upper: f64,
    pub interpolation_remainder_upper: f64,
    pub spans: usize,
    pub rounding_included: bool,
}
fn neg(x: I) -> Result<I> {
    I::new(-x.hi, -x.lo)
}
fn jet(p: &[f64], v: I, height: I, start: I, twist: I) -> Result<[[I; 3]; 3]> {
    let (sin, cos) = crate::involute::certified_trig(start.add(twist.mul(v)?)?)?;
    let x = cos.mul(I::point(p[0]))?.sub(sin.mul(I::point(p[1]))?)?;
    let y = sin.mul(I::point(p[0]))?.add(cos.mul(I::point(p[1]))?)?;
    let tt = twist.mul(twist)?;
    Ok([
        [x, y, I::point(p[2]).add(height.mul(v)?)?],
        [neg(twist.mul(y)?)?, twist.mul(x)?, height],
        [neg(tt.mul(x)?)?, neg(tt.mul(y)?)?, I::point(0.)],
    ])
}
/// Bound the actual saved surface against every helical sweep in independently
/// justified parameter boxes. The original positive rational U basis is retained
/// exactly, making a maximum control-row bound valid over the complete U domain.
/// V must be a normalized ruled span or full C0 quintic Bezier chain. No libm
/// trigonometric value or sampled residual is used as proof.
pub fn certify_stored(
    c: &Curve,
    s: &Surface,
    height: [f64; 2],
    start: [f64; 2],
    twist: [f64; 2],
) -> Result<Certificate> {
    c.validate()?;
    s.validate()?;
    check(
        c.control_points[0].len() == 3 && c.control_points.len() <= 256,
        "Helical certificate needs a bounded 3D profile",
    )?;
    check(
        s.degree_u == c.degree
            && s.knots_u == c.knots
            && s.periodic_u == c.periodic
            && !s.periodic_v
            && s.control_points.len() == c.control_points.len(),
        "Helical certificate must retain the exact original U basis",
    )?;
    check(
        s.weights
            .iter()
            .zip(&c.weights)
            .all(|(row, w)| row.iter().all(|v| v == w)),
        "Helical certificate requires constant original weights along V",
    )?;
    let height = I::new(height[0], height[1])?;
    let start = I::new(start[0], start[1])?;
    let twist = I::new(twist[0], twist[1])?;
    let degree = s.degree_v;
    let count = s.control_points[0].len();
    check(
        degree == 1 || degree == 5,
        "Helical certificate supports ruled or quintic V",
    )?;
    check(
        count >= degree + 1 && (count - 1) % degree == 0 && count <= 256,
        "Invalid bounded helical span count",
    )?;
    let n = (count - 1) / degree;
    let breaks: Vec<_> = (0..=n).map(|i| s.knots_v[degree + i * degree]).collect();
    check(
        breaks[0] == 0. && breaks[n] == 1. && breaks.windows(2).all(|x| x[0] < x[1]),
        "Helical certificate requires normalized increasing V spans",
    )?;
    let mut expected = vec![0.; degree + 1];
    for &b in &breaks[1..] {
        expected.extend(std::iter::repeat_n(b, degree));
    }
    expected.push(1.);
    check(
        s.knots_v == expected,
        "Helical certificate requires full original Bezier span ownership",
    )?;
    let mut difference = 0_f64;
    let mut remainder = 0_f64;
    // Component Hermite remainder -> Euclidean vector bound.
    let sqrt2 = I::point(box_distance(&[I::point(1.); 2], &[I::point(0.); 2])?.1);
    let divisor = I::point(if degree == 5 { 46080. } else { 8. });
    let exp = if degree == 5 { 6 } else { 2 };
    for (i, p) in c.control_points.iter().enumerate() {
        let radius = box_distance(&[I::point(p[0]), I::point(p[1])], &[I::point(0.); 2])?.1;
        let mut a = jet(p, I::point(breaks[0]), height, start, twist)?;
        for k in 0..n {
            let b = jet(p, I::point(breaks[k + 1]), height, start, twist)?;
            let h = I::point(breaks[k + 1]).sub(I::point(breaks[k]))?;
            let mut poles = vec![[I::point(0.); 3]; degree + 1];
            poles[0] = a[0];
            poles[degree] = b[0];
            if degree == 5 {
                let h5 = h.div(I::point(5.))?;
                let hh20 = h.mul(h)?.div(I::point(20.))?;
                for d in 0..3 {
                    poles[1][d] = a[0][d].add(h5.mul(a[1][d])?)?;
                    poles[2][d] = a[0][d]
                        .add(h5.mul(a[1][d])?.mul(I::point(2.))?)?
                        .add(hh20.mul(a[2][d])?)?;
                    poles[3][d] = b[0][d]
                        .sub(h5.mul(b[1][d])?.mul(I::point(2.))?)?
                        .add(hh20.mul(b[2][d])?)?;
                    poles[4][d] = b[0][d].sub(h5.mul(b[1][d])?)?;
                }
            }
            for (j, pole) in poles.iter().enumerate() {
                let cp = &s.control_points[i][degree * k + j];
                let stored = [I::point(cp[0]), I::point(cp[1]), I::point(cp[2])];
                difference = difference.max(box_distance(&stored, pole)?.1);
            }
            let t = twist.mul(h)?;
            let absolute = I::point(t.lo.abs().max(t.hi.abs()));
            let mut power = I::point(1.);
            for _ in 0..exp {
                power = power.mul(absolute)?;
            }
            let bound = I::point(radius).mul(power)?.div(divisor)?;
            remainder = remainder.max(bound.mul(sqrt2)?.hi);
            a = b;
        }
    }
    Ok(Certificate {
        error_upper: I::point(difference).add(I::point(remainder))?.hi,
        control_difference_upper: difference,
        interpolation_remainder_upper: remainder,
        spans: n,
        rounding_included: true,
    })
}
