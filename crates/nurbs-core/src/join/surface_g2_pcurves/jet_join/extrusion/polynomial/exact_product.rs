//! Exact preprocessing of original product profiles, before interval rounding.
//! Shape acceptance still belongs to Pair::bounds and original support checks.
use super::*;
use num_rational::BigRational as Q;
use num_traits::{One, ToPrimitive, Zero};
type H = [Vec<Q>; 4];
pub(in super::super) struct Work {
    left: usize,
}
impl Work {
    pub(in super::super) fn new(cells: usize) -> Self {
        Self {
            left: cells.saturating_mul(128).min(1_000_000),
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
fn ratio(x: f64, a: f64, b: f64, w: &mut Work) -> Option<Q> {
    let x = w.input(x)?;
    let a = w.input(a)?;
    let b = w.input(b)?;
    let n = w.sub(&x, &a)?;
    let d = w.sub(&b, &a)?;
    w.div(&n, &d)
}
fn restrict(h: &H, ends: [Q; 2], w: &mut Work) -> Option<H> {
    let n = h[0].len() - 1;
    let mut out: H = std::array::from_fn(|_| Vec::new());
    for axis in 0..4 {
        for k in 0..=n {
            let mut p = h[axis].clone();
            for stage in 0..n {
                let t = &ends[usize::from(stage >= n - k)];
                let r = w.sub(&Q::one(), t)?;
                for j in 0..p.len() - 1 {
                    let a = w.mul(&p[j], &r)?;
                    let b = w.mul(&p[j + 1], t)?;
                    p[j] = w.add(&a, &b)?;
                }
                p.pop();
            }
            out[axis].push(p[0].clone());
        }
    }
    Some(out)
}
fn elevate(h: &mut H, degree: usize, w: &mut Work) -> Option<()> {
    while h[0].len() < degree + 1 {
        for poly in h.iter_mut() {
            let n = poly.len();
            let mut p = vec![poly[0].clone()];
            for i in 1..n {
                let t = Q::new(i.into(), n.into());
                let r = w.sub(&Q::one(), &t)?;
                let a = w.mul(&poly[i - 1], &t)?;
                let b = w.mul(&poly[i], &r)?;
                p.push(w.add(&a, &b)?);
            }
            p.push(poly.last()?.clone());
            *poly = p;
        }
    }
    Some(())
}
fn choose(n: usize, k: usize) -> u128 {
    (0..k).fold(1, |v, i| v * (n - i) as u128 / (i + 1) as u128)
}
fn product(a: &[Q], b: &[Q], w: &mut Work) -> Option<Vec<Q>> {
    let p = a.len() - 1;
    let q = b.len() - 1;
    let mut out = Vec::new();
    for k in 0..=p + q {
        let mut sum = Q::zero();
        for i in 0..=p {
            if i > k || k - i > q {
                continue;
            }
            let f = Q::new(
                (choose(p, i) * choose(q, k - i)).into(),
                choose(p + q, k).into(),
            );
            let t = w.mul(&a[i], &b[k - i])?;
            let t = w.mul(&t, &f)?;
            sum = w.add(&sum, &t)?;
        }
        out.push(sum);
    }
    Some(out)
}
fn enclosure(q: &Q, w: &mut Work) -> Option<I> {
    let x = q.to_f64().filter(|x| x.is_finite())?;
    if w.input(x)? == *q {
        return Some(I::point(x));
    }
    let mut lo = x;
    let mut hi = x;
    for _ in 0..4 {
        if w.input(lo)? > *q {
            lo = lo.next_down();
        }
        if w.input(hi)? < *q {
            hi = hi.next_up();
        }
        if lo.is_finite() && hi.is_finite() && w.input(lo)? <= *q && w.input(hi)? >= *q {
            return I::new(lo, hi).ok();
        }
    }
    None
}
fn intervals(h: &H, w: &mut Work) -> Option<Homogeneous> {
    let mut out: Homogeneous = std::array::from_fn(|_| Vec::new());
    for k in 0..4 {
        for q in &h[k] {
            out[k].push(enclosure(q, w)?);
        }
    }
    Some(out)
}
fn difference(p: &[Q], w: &mut Work) -> Option<Vec<Q>> {
    if p.len() < 2 {
        return Some(vec![Q::zero()]);
    }
    let degree = Q::from_integer((p.len() - 1).into());
    p.windows(2)
        .map(|p| {
            let d = w.sub(&p[1], &p[0])?;
            w.mul(&d, &degree)
        })
        .collect()
}
fn elevate_poly(p: &mut Vec<Q>, degree: usize, w: &mut Work) -> Option<()> {
    while p.len() < degree + 1 {
        let n = p.len();
        let mut out = vec![p[0].clone()];
        for i in 1..n {
            let t = Q::new(i.into(), n.into());
            let r = w.sub(&Q::one(), &t)?;
            let a = w.mul(&p[i - 1], &t)?;
            let b = w.mul(&p[i], &r)?;
            out.push(w.add(&a, &b)?);
        }
        out.push(p.last()?.clone());
        *p = out;
    }
    Some(())
}
fn exact_jets(h: &H, w: &mut Work) -> Option<ExactJets> {
    let dw = difference(&h[3], w)?;
    let squared = product(&h[3], &h[3], w)?;
    let cubed = product(&squared, &h[3], w)?;
    let mut first: [Vec<I>; 3] = std::array::from_fn(|_| Vec::new());
    let mut second: [Vec<I>; 3] = std::array::from_fn(|_| Vec::new());
    for k in 0..3 {
        let dx = difference(&h[k], w)?;
        let a = product(&dx, &h[3], w)?;
        let b = product(&h[k], &dw, w)?;
        let mut f = a
            .iter()
            .zip(&b)
            .map(|(a, b)| w.sub(a, b))
            .collect::<Option<Vec<_>>>()?;
        let df = difference(&f, w)?;
        let a = product(&df, &h[3], w)?;
        let b = product(&f, &dw, w)?;
        let mut g = Vec::new();
        for (a, b) in a.iter().zip(&b) {
            let twice = w.add(b, b)?;
            g.push(w.sub(a, &twice)?);
        }
        elevate_poly(&mut f, squared.len() - 1, w)?;
        elevate_poly(&mut g, cubed.len() - 1, w)?;
        for x in &g {
            second[k].push(enclosure(x, w)?);
        }
        for x in &f {
            first[k].push(enclosure(x, w)?);
        }
    }
    let first_weights = squared
        .iter()
        .map(|q| enclosure(q, w))
        .collect::<Option<Vec<_>>>()?;
    let second_weights = cubed
        .iter()
        .map(|q| enclosure(q, w))
        .collect::<Option<Vec<_>>>()?;
    Some(ExactJets {
        first,
        second,
        first_weights,
        second_weights,
    })
}
/// An original product profile: `profile` extruded along `axis` over the
/// path interval `path`, measured from `origin`.
#[derive(Clone, Copy)]
pub(in super::super) struct ProductSource<'a> {
    pub profile: &'a Curve,
    pub path: [f64; 2],
    pub axis: usize,
    pub origin: [f64; 3],
}
/// One candidate surface span and control row, with the `u` interval `clock`
/// that maps onto the source path interval.
#[derive(Clone, Copy)]
pub(in super::super) struct SurfaceCell<'a> {
    pub surface: &'a Surface,
    pub span: usize,
    pub row: usize,
    pub clock: [f64; 2],
}
pub(in super::super) fn prepare(
    source: ProductSource<'_>,
    cell: SurfaceCell<'_>,
    w: &mut Work,
) -> Option<Pair> {
    let ProductSource {
        profile,
        path,
        axis,
        origin,
    } = source;
    let SurfaceCell {
        surface: s,
        span,
        row,
        clock,
    } = cell;
    let p = profile.degree;
    let q = s.degree_u;
    if p > 16
        || q > 16
        || profile.control_points.len() != p + 1
        || axis >= 3
        || span < q
        || span >= s.control_points.len()
    {
        return None;
    }
    let mut a: H = std::array::from_fn(|_| Vec::new());
    let mut b: H = std::array::from_fn(|_| Vec::new());
    for i in 0..=p {
        let weight = w.input(profile.weights[i])?;
        for k in 0..3 {
            let x = if k == axis {
                Q::zero()
            } else {
                let x = w.input(profile.control_points[i][k])?;
                let o = w.input(origin[k])?;
                w.sub(&x, &o)?
            };
            a[k].push(w.mul(&x, &weight)?);
        }
        a[3].push(weight);
    }
    let mut ends = Vec::new();
    let d = profile.domain();
    for u in [s.knots_u[span], s.knots_u[span + 1]] {
        let t = ratio(u, clock[0], clock[1], w)?;
        let x = w.input(path[0])?;
        let y = w.input(path[1])?;
        let delta = w.sub(&y, &x)?;
        let offset = w.mul(&delta, &t)?;
        let x = w.add(&x, &offset)?;
        let lo = w.input(d[0])?;
        let hi = w.input(d[1])?;
        let num = w.sub(&x, &lo)?;
        let den = w.sub(&hi, &lo)?;
        ends.push(w.div(&num, &den)?);
    }
    a = restrict(&a, [ends[0].clone(), ends[1].clone()], w)?;
    for i in span - q..=span {
        let weight = w.input(s.weights[i][row])?;
        for k in 0..3 {
            let x = if k == axis {
                Q::zero()
            } else {
                let x = w.input(s.control_points[i][row][k])?;
                let o = w.input(origin[k])?;
                w.sub(&x, &o)?
            };
            b[k].push(w.mul(&x, &weight)?);
        }
        b[3].push(weight);
    }
    let degree = p.max(q);
    elevate(&mut a, degree, w)?;
    elevate(&mut b, degree, w)?;
    let mut e: [Vec<I>; 3] = std::array::from_fn(|_| Vec::new());
    let mut residual: H = std::array::from_fn(|_| Vec::new());
    for k in 0..3 {
        let ba = product(&b[k], &a[3], w)?;
        let ab = product(&a[k], &b[3], w)?;
        for (x, y) in ba.iter().zip(&ab) {
            let d = w.sub(x, y)?;
            e[k].push(enclosure(&d, w)?);
            residual[k].push(d);
        }
    }
    let d = product(&a[3], &b[3], w)?;
    residual[3] = d.clone();
    let d = d
        .iter()
        .map(|q| enclosure(q, w))
        .collect::<Option<Vec<_>>>()?;
    let ai = intervals(&a, w)?;
    let bi = intervals(&b, w)?;
    let derivatives = match (exact_jets(&a, w), exact_jets(&b, w)) {
        (Some(a), Some(b)) => Some([a, b]),
        _ => None,
    };
    let delta_derivatives = exact_jets(&residual, w);
    Some(Pair {
        delta_derivatives,
        a: ai,
        b: bi,
        e,
        d,
        derivatives,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_common_factor_cancels_before_rounding_but_real_bend_does_not() {
        let c = Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 0., 0.]]).unwrap();
        let mut s = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: [0., 0.25, 1.]
                .iter()
                .map(|&x| vec![vec![x, 0., 0.], vec![x, 0., 1.]])
                .collect(),
            weights: [1., 2., 3.].iter().map(|&w| vec![w; 2]).collect(),
            periodic_u: false,
            periodic_v: false,
        };
        let p = prepare(
            ProductSource {
                profile: &c,
                path: [0., 1.],
                axis: 2,
                origin: [0.; 3],
            },
            SurfaceCell {
                surface: &s,
                span: 2,
                row: 0,
                clock: [0., 1.],
            },
            &mut Work::new(1000),
        )
        .unwrap();
        assert!(p.derivatives.is_some());
        let delta = p.delta_derivatives.as_ref().unwrap();
        assert!(
            delta
                .first
                .iter()
                .chain(&delta.second)
                .flatten()
                .all(|v| v.lo == 0. && v.hi == 0.)
        );
        assert!(p.e.iter().flatten().all(|x| x.lo == 0. && x.hi == 0.));
        let b = p.bounds([0.25, 0.75], 2).unwrap().unwrap();
        assert!(b[0][1] <= 1e-10 && b[1][1] <= 1e-10, "{b:?}");
        assert!(
            prepare(
                ProductSource {
                    profile: &c,
                    path: [0., 1.],
                    axis: 2,
                    origin: [0.; 3],
                },
                SurfaceCell {
                    surface: &s,
                    span: 2,
                    row: 0,
                    clock: [0., 1.],
                },
                &mut Work { left: 0 },
            )
            .is_none()
        );
        s.control_points[1][0][1] = 1e-4;
        let p = prepare(
            ProductSource {
                profile: &c,
                path: [0., 1.],
                axis: 2,
                origin: [0.; 3],
            },
            SurfaceCell {
                surface: &s,
                span: 2,
                row: 0,
                clock: [0., 1.],
            },
            &mut Work::new(1000),
        )
        .unwrap();
        assert!(p.e.iter().flatten().any(|x| x.lo != 0. || x.hi != 0.));
        let b = p.bounds([0.5, 0.5], 2).unwrap().unwrap();
        assert!(b[0][0] > 1e-8 || b[1][0] > 1e-8, "{b:?}");
    }
}
