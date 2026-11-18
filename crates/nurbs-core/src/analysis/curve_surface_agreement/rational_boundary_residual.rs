//! Exact binary64-source Bernstein cross products. Positive denominator
//! coefficients turn coefficientwise residual ratios into a whole-cell bound.
//! A common rational factor need not be fitted or divided out. Exhaustion
//! yields no certificate; this module cannot issue a mismatch witness.
use super::*;
use num_rational::BigRational as Q;
use num_traits::{One, Signed, Zero};
type Net = Vec<[Q; 4]>;
pub(super) struct Work {
    left: usize,
}
impl Work {
    pub(super) fn new(cells: usize) -> Self {
        Self {
            left: cells.saturating_mul(8192).min(1_000_000),
        }
    }
    fn keep(&mut self, x: Q) -> Option<Q> {
        self.left = self.left.checked_sub(1)?;
        (x.numer().bits() <= 8192 && x.denom().bits() <= 8192).then_some(x)
    }
    fn input(&mut self, x: f64) -> Option<Q> {
        self.keep(Q::from_float(x)?)
    }
    fn add(&mut self, a: &Q, b: &Q) -> Option<Q> {
        self.keep(a + b)
    }
    fn sub(&mut self, a: &Q, b: &Q) -> Option<Q> {
        self.keep(a - b)
    }
    fn mul(&mut self, a: &Q, b: &Q) -> Option<Q> {
        self.keep(a * b)
    }
    fn div(&mut self, a: &Q, b: &Q) -> Option<Q> {
        if b.is_zero() { None } else { self.keep(a / b) }
    }
}
fn net(points: &[Vec<f64>], weights: &[f64], w: &mut Work) -> Option<Net> {
    points
        .iter()
        .zip(weights)
        .map(|(p, &v)| {
            let weight = w.input(v)?;
            if weight <= Q::zero() {
                return None;
            }
            let mut row = std::array::from_fn(|_| Q::zero());
            row[3] = weight.clone();
            for k in 0..3 {
                let x = w.input(p[k])?;
                row[k] = w.mul(&x, &weight)?;
            }
            Some(row)
        })
        .collect()
}
fn ratio(x: f64, a: f64, b: f64, w: &mut Work) -> Option<Q> {
    let x = w.input(x)?;
    let a = w.input(a)?;
    let b = w.input(b)?;
    let num = w.sub(&x, &a)?;
    let den = w.sub(&b, &a)?;
    w.div(&num, &den)
}
fn restrict(source: &Net, a: &Q, b: &Q, w: &mut Work) -> Option<Net> {
    if a < &Q::zero() || a > &Q::one() || b < &Q::zero() || b > &Q::one() {
        return None;
    }
    let n = source.len() - 1;
    let mut out = Vec::new();
    for k in 0..=n {
        let mut row = source.clone();
        for stage in 0..n {
            let t = if stage < n - k { a } else { b };
            let r = w.sub(&Q::one(), t)?;
            for i in 0..row.len() - 1 {
                for axis in 0..4 {
                    let x = w.mul(&row[i][axis], &r)?;
                    let y = w.mul(&row[i + 1][axis], t)?;
                    row[i][axis] = w.add(&x, &y)?;
                }
            }
            row.pop();
        }
        out.push(row[0].clone());
    }
    Some(out)
}
fn choose(n: usize, k: usize) -> usize {
    (0..k).fold(1usize, |v, i| v * (n - i) / (i + 1))
}
/// Source cell saturation and original affine clock eligibility are checked by
/// the caller. All restrictions below use exact ratios of the source floats.
/// One boundary row and knot span of the surface, the curve parameter interval `[a, b]` and the
/// surface parameter range `[lo, hi]` it is compared over.
#[derive(Clone, Copy, Debug)]
pub(super) struct ResidualWindow {
    pub row: usize,
    pub span: usize,
    pub a: f64,
    pub b: f64,
    pub lo: f64,
    pub hi: f64,
}
pub(super) fn within(
    c: &Curve,
    s: &Surface,
    window: ResidualWindow,
    tolerance: f64,
    w: &mut Work,
) -> Option<bool> {
    let ResidualWindow { row, span, a, b, lo, hi } = window;
    let p = c.degree;
    let q = s.degree_u;
    if p > 16 || q > 16 || c.control_points.len() != p + 1 {
        return None;
    }
    let ca = net(&c.control_points, &c.weights, w)?;
    let points = (span - q..=span)
        .map(|i| s.control_points[i][row].clone())
        .collect::<Vec<_>>();
    let weights = (span - q..=span)
        .map(|i| s.weights[i][row])
        .collect::<Vec<_>>();
    let cb = net(&points, &weights, w)?;
    let left = ratio(lo, a, b, w)?;
    let right = ratio(hi, a, b, w)?;
    let ca = restrict(&ca, &left, &right, w)?;
    let left = ratio(lo, s.knots_u[span], s.knots_u[span + 1], w)?;
    let right = ratio(hi, s.knots_u[span], s.knots_u[span + 1], w)?;
    let cb = restrict(&cb, &left, &right, w)?;
    compare(&ca, &cb, tolerance, w)
}
fn compare(ca: &Net, cb: &Net, tolerance: f64, w: &mut Work) -> Option<bool> {
    let p = ca.len() - 1;
    let q = cb.len() - 1;
    let mut error = std::array::from_fn::<_, 3, _>(|_| Q::zero());
    for k in 0..=p + q {
        let mut denominator = Q::zero();
        let mut residual = std::array::from_fn::<_, 3, _>(|_| Q::zero());
        for i in 0..=p {
            if i > k || k - i > q {
                continue;
            }
            let j = k - i;
            let factor = Q::new(
                (choose(p, i) * choose(q, j)).into(),
                choose(p + q, k).into(),
            );
            let d = w.mul(&ca[i][3], &cb[j][3])?;
            let d = w.mul(&d, &factor)?;
            denominator = w.add(&denominator, &d)?;
            for axis in 0..3 {
                let x = w.mul(&ca[i][axis], &cb[j][3])?;
                let y = w.mul(&cb[j][axis], &ca[i][3])?;
                let r = w.sub(&x, &y)?;
                let r = w.mul(&r, &factor)?;
                residual[axis] = w.add(&residual[axis], &r)?;
            }
        }
        if denominator <= Q::zero() {
            return None;
        }
        for axis in 0..3 {
            let r = w.div(&residual[axis].abs(), &denominator)?;
            if r > error[axis] {
                error[axis] = r;
            }
        }
    }
    let e = w.add(&error[0], &error[1])?;
    let e = w.add(&e, &error[2])?;
    let tolerance = w.input(tolerance)?;
    Some(e <= tolerance)
}

/// Compose an ORIGINAL single rational tensor-Bezier chart with its actual
/// affine pcurve using exact restrictions and a Bernstein diagonal. No fitted
/// iso curve or rounded station is substituted for the source definition.
pub(super) fn affine_within(
    c: &Curve,
    p: &Curve,
    s: &Surface,
    reversed: bool,
    tolerance: f64,
    w: &mut Work,
) -> Option<bool> {
    let clamped = |k: &[f64], d: usize, n: usize| {
        n == d + 1
            && k.len() == 2 * n
            && k[..n].iter().all(|&x| x == k[d])
            && k[n..].iter().all(|&x| x == k[n])
    };
    let (pu, pv) = (s.degree_u, s.degree_v);
    if c.periodic
        || p.periodic
        || s.periodic_u
        || s.periodic_v
        || c.degree > 16
        || pu + pv > 16
        || p.degree != 1
        || p.control_points.len() != 2
        || p.weights[0] != p.weights[1]
        || c.control_points[0].len() != 3
        || p.control_points[0].len() != 2
        || s.control_points[0][0].len() != 3
        || !clamped(&c.knots, c.degree, c.control_points.len())
        || !clamped(&s.knots_u, pu, s.control_points.len())
        || !clamped(&s.knots_v, pv, s.control_points[0].len())
    {
        return None;
    }
    let mut clocks = Vec::new();
    for (axis, knots, d) in [(0, &s.knots_u, pu), (1, &s.knots_v, pv)] {
        let a = ratio(
            p.control_points[usize::from(reversed)][axis],
            knots[d],
            knots[d + 1],
            w,
        )?;
        let b = ratio(
            p.control_points[usize::from(!reversed)][axis],
            knots[d],
            knots[d + 1],
            w,
        )?;
        clocks.push([a, b]);
    }
    let mut columns = Vec::new();
    for j in 0..=pv {
        let points = (0..=pu)
            .map(|i| s.control_points[i][j].clone())
            .collect::<Vec<_>>();
        let weights = (0..=pu).map(|i| s.weights[i][j]).collect::<Vec<_>>();
        columns.push(restrict(
            &net(&points, &weights, w)?,
            &clocks[0][0],
            &clocks[0][1],
            w,
        )?);
    }
    let mut rows = Vec::new();
    for i in 0..=pu {
        let row = columns.iter().map(|c| c[i].clone()).collect::<Net>();
        rows.push(restrict(&row, &clocks[1][0], &clocks[1][1], w)?);
    }
    let mut trace = Vec::new();
    for k in 0..=pu + pv {
        let mut coefficient = std::array::from_fn(|_| Q::zero());
        for i in 0..=pu {
            if i > k || k - i > pv {
                continue;
            }
            let j = k - i;
            let factor = Q::new(
                (choose(pu, i) * choose(pv, j)).into(),
                choose(pu + pv, k).into(),
            );
            for axis in 0..4 {
                let v = w.mul(&rows[i][j][axis], &factor)?;
                coefficient[axis] = w.add(&coefficient[axis], &v)?;
            }
        }
        trace.push(coefficient);
    }
    compare(
        &net(&c.control_points, &c.weights, w)?,
        &trace,
        tolerance,
        w,
    )
}

/// Exact affine-plane lift of an ORIGINAL rational pcurve, including its
/// nonconstant weights and full natural clock. Fourth-corner equality is exact.
pub(super) fn rational_plane_within(
    c: &Curve,
    p: &Curve,
    s: &Surface,
    reversed: bool,
    tolerance: f64,
    w: &mut Work,
) -> Option<bool> {
    let clamped = |k: &[f64], d: usize, n: usize| {
        n == d + 1
            && k.len() == 2 * n
            && k[..n].iter().all(|&x| x == k[d])
            && k[n..].iter().all(|&x| x == k[n])
    };
    if c.periodic
        || p.periodic
        || s.periodic_u
        || s.periodic_v
        || c.degree > 16
        || p.degree > 16
        || s.degree_u != 1
        || s.degree_v != 1
        || s.control_points.len() != 2
        || s.control_points[0].len() != 2
        || c.control_points[0].len() != 3
        || p.control_points[0].len() != 2
        || s.control_points[0][0].len() != 3
        || !clamped(&c.knots, c.degree, c.control_points.len())
        || !clamped(&p.knots, p.degree, p.control_points.len())
        || !clamped(&s.knots_u, 1, 2)
        || !clamped(&s.knots_v, 1, 2)
        || !s.weights.iter().flatten().all(|&x| x == s.weights[0][0])
    {
        return None;
    }
    let mut origin = Vec::new();
    let mut du = Vec::new();
    let mut dv = Vec::new();
    for k in 0..3 {
        let o = w.input(s.control_points[0][0][k])?;
        let a = w.input(s.control_points[1][0][k])?;
        let b = w.input(s.control_points[0][1][k])?;
        let end = w.input(s.control_points[1][1][k])?;
        let u = w.sub(&a, &o)?;
        let v = w.sub(&b, &o)?;
        let e = w.add(&o, &u)?;
        let e = w.add(&e, &v)?;
        if e != end {
            return None;
        }
        origin.push(o);
        du.push(u);
        dv.push(v);
    }
    let mut trace = Vec::new();
    for index in 0..p.control_points.len() {
        let i = if reversed {
            p.control_points.len() - 1 - index
        } else {
            index
        };
        let u = ratio(p.control_points[i][0], s.knots_u[1], s.knots_u[2], w)?;
        let v = ratio(p.control_points[i][1], s.knots_v[1], s.knots_v[2], w)?;
        if u < Q::zero() || u > Q::one() || v < Q::zero() || v > Q::one() {
            return None;
        }
        let weight = w.input(p.weights[i])?;
        if weight <= Q::zero() {
            return None;
        }
        let mut row = std::array::from_fn(|_| Q::zero());
        row[3] = weight.clone();
        for k in 0..3 {
            let a = w.mul(&du[k], &u)?;
            let b = w.mul(&dv[k], &v)?;
            let x = w.add(&origin[k], &a)?;
            let x = w.add(&x, &b)?;
            row[k] = w.mul(&x, &weight)?;
        }
        trace.push(row);
    }
    compare(
        &net(&c.control_points, &c.weights, w)?,
        &trace,
        tolerance,
        w,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rational_pcurve_on_signed_normalized_plane_preserves_original_weights() {
        let mut s = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![10., 0., 0.], vec![10., 10., 0.]],
                vec![vec![0., 0., 0.], vec![0., 10., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let p = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0.1, 0.1], vec![0.3, 0.3], vec![0.6, 0.8]],
            weights: vec![1., 2., 3.],
            periodic: false,
        };
        let mut c = Curve {
            degree: 2,
            knots: p.knots.clone(),
            control_points: vec![vec![9., 1., 0.], vec![7., 3., 0.], vec![4., 8., 0.]],
            weights: p.weights.clone(),
            periodic: false,
        };
        assert_eq!(
            rational_plane_within(&c, &p, &s, false, 1e-12, &mut Work::new(100)),
            Some(true)
        );
        assert_eq!(
            verify(&c, &p, &s, false, 1e-12, 100).unwrap().status,
            Status::WithinTolerance
        );
        c.control_points[1][2] = 0.01;
        assert_eq!(
            rational_plane_within(&c, &p, &s, false, 1e-12, &mut Work::new(100)),
            Some(false)
        );
        s.control_points[1][1][2] = 0.001;
        assert!(rational_plane_within(&c, &p, &s, false, 1e-12, &mut Work::new(100)).is_none());
    }
    #[test]
    fn interior_affine_tensor_trace_retains_both_coordinates_and_orientation() {
        let s = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 1.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let p = Curve::from_polyline(vec![vec![0.2, 0.3], vec![0.8, 0.7]]).unwrap();
        let c = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![
                vec![0.2, 0.3, 0.06],
                vec![0.5, 0.5, 0.19],
                vec![0.8, 0.7, 0.56],
            ],
            weights: vec![1.; 3],
            periodic: false,
        };
        assert_eq!(
            affine_within(&c, &p, &s, false, 1e-12, &mut Work::new(100)),
            Some(true)
        );
        let reversed = p.reverse().unwrap();
        assert_ne!(
            affine_within(&c, &reversed, &s, false, 1e-12, &mut Work::new(100)),
            Some(true)
        );
        assert_eq!(
            affine_within(&c, &reversed, &s, true, 1e-12, &mut Work::new(100)),
            Some(true)
        );
        assert!(affine_within(&c, &p, &s, false, 1e-12, &mut Work { left: 0 }).is_none());
        let mut wrong = c.clone();
        wrong.control_points[1][2] += 0.01;
        assert_eq!(
            affine_within(&wrong, &p, &s, false, 1e-12, &mut Work::new(100)),
            Some(false)
        );
        assert_eq!(
            verify(&c, &p, &s, false, 1e-12, 100).unwrap().status,
            Status::WithinTolerance
        );
    }
    #[test]
    fn nonconstant_common_denominator_is_cancelled_without_fitting() {
        let c = Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 0., 0.]]).unwrap();
        let mut s = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: [0., 0.25, 1.]
                .iter()
                .map(|&x| vec![vec![x, 0., 0.], vec![x, 1., 0.]])
                .collect(),
            weights: [1., 2., 3.].iter().map(|&w| vec![w; 2]).collect(),
            periodic_u: false,
            periodic_v: false,
        };
        assert_eq!(
            within(
                &c,
                &s,
                ResidualWindow { row: 0, span: 2, a: 0., b: 1., lo: 0., hi: 1. },
                0.,
                &mut Work::new(1),
            ),
            Some(true)
        );
        let p = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
        assert_eq!(
            boundary_coefficients::inspect(&c, &p, &s, false, 1e-12, 1)
                .unwrap()
                .unwrap()
                .status,
            Status::WithinTolerance
        );
        assert_eq!(
            within(
                &c,
                &s,
                ResidualWindow { row: 0, span: 2, a: 0., b: 1., lo: 0., hi: 1. },
                1e-12,
                &mut Work { left: 0 },
            ),
            None
        );
        s.control_points[1][0][2] = 0.01;
        assert_eq!(
            within(
                &c,
                &s,
                ResidualWindow { row: 0, span: 2, a: 0., b: 1., lo: 0., hi: 1. },
                1e-12,
                &mut Work::new(1),
            ),
            Some(false)
        );
        assert_ne!(
            boundary_coefficients::inspect(&c, &p, &s, false, 1e-12, 1)
                .unwrap()
                .unwrap()
                .status,
            Status::WithinTolerance
        );
    }
}
