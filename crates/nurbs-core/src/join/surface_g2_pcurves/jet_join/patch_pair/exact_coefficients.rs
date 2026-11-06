//! Exact rational preprocessing of ORIGINAL binary64 boundary coefficients.
//! Subtract the two source jet polynomials before outward conversion. No source
//! pole, weight, clock or shape is changed; no geometric receipt is issued here.
use super::*;
use num_rational::BigRational as Q;
use num_traits::{One, ToPrimitive, Zero};
type P = Vec<[Q; 3]>;
struct Work {
    left: usize,
}
impl Work {
    fn keep(&mut self, q: Q) -> Option<Q> {
        self.left = self.left.checked_sub(1)?;
        (q.numer().bits() <= 8192 && q.denom().bits() <= 8192).then_some(q)
    }
    fn input(&mut self, x: f64) -> Option<Q> {
        self.keep(Q::from_float(x)?)
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
fn integer(v: usize) -> Q {
    Q::from_integer(v.into())
}
fn difference(p: &P, width: &Q, w: &mut Work) -> Option<P> {
    let n = integer(p.len() - 1);
    let mut out = Vec::new();
    for pair in p.windows(2) {
        let mut v = std::array::from_fn(|_| Q::zero());
        for k in 0..3 {
            let a = w.sub(&pair[1][k], &pair[0][k])?;
            let b = w.mul(&a, &n)?;
            v[k] = w.div(&b, width)?;
        }
        out.push(v);
    }
    Some(out)
}
struct ExactSide {
    u: P,
    uu: P,
    uv: P,
}
fn prepare(s: &Surface, end: bool, w: &mut Work) -> Option<ExactSide> {
    let p = s.degree_u;
    let q = s.degree_v;
    let ids = if end { [p, p - 1, p - 2] } else { [0, 1, 2] };
    let w0 = w.input(s.weights[ids[0]][0])?;
    let w1 = w.input(s.weights[ids[1]][0])?;
    let w2 = w.input(s.weights[ids[2]][0])?;
    let r1 = w.div(&w1, &w0)?;
    let r2 = w.div(&w2, &w0)?;
    let alpha = w.mul(&integer(p), &r1)?;
    let gamma = w.mul(&integer(p * (p - 1)), &r2)?;
    let beta0 = w.sub(&alpha, &Q::one())?;
    let beta = w.mul(&integer(2), &beta0)?;
    let hi = w.input(s.knots_u[p + 1])?;
    let lo = w.input(s.knots_u[p])?;
    let width = w.sub(&hi, &lo)?;
    let squared = w.mul(&width, &width)?;
    let hi = w.input(s.knots_v[q + 1])?;
    let lo = w.input(s.knots_v[q])?;
    let vwidth = w.sub(&hi, &lo)?;
    let sign = if end { -Q::one() } else { Q::one() };
    let mut u = Vec::new();
    let mut uu = Vec::new();
    for j in 0..=q {
        let mut first = std::array::from_fn(|_| Q::zero());
        let mut second = first.clone();
        for k in 0..3 {
            let p0 = w.input(s.control_points[ids[0]][j][k])?;
            let p1 = w.input(s.control_points[ids[1]][j][k])?;
            let p2 = w.input(s.control_points[ids[2]][j][k])?;
            let d1 = w.sub(&p1, &p0)?;
            let d2 = w.sub(&p2, &p0)?;
            let inward = w.mul(&alpha, &d1)?;
            let a = w.mul(&gamma, &d2)?;
            let b = w.mul(&beta, &inward)?;
            let a = w.sub(&a, &b)?;
            second[k] = w.div(&a, &squared)?;
            let a = w.mul(&sign, &inward)?;
            first[k] = w.div(&a, &width)?;
        }
        u.push(first);
        uu.push(second);
    }
    let uv = difference(&u, &vwidth, w)?;
    Some(ExactSide { u, uu, uv })
}
fn enclosure(q: &Q, w: &mut Work) -> Option<I> {
    let x = q.to_f64().filter(|x| x.is_finite())?;
    let exact = w.input(x)?;
    if exact == *q {
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
fn intervals(p: &P, w: &mut Work) -> Option<Polynomial> {
    p.iter()
        .map(|row| {
            Some([
                enclosure(&row[0], w)?,
                enclosure(&row[1], w)?,
                enclosure(&row[2], w)?,
            ])
        })
        .collect()
}
fn delta(a: &P, b: &P, w: &mut Work) -> Option<Polynomial> {
    let mut out = Vec::new();
    for (a, b) in a.iter().zip(b) {
        let x = w.sub(&b[0], &a[0])?;
        let y = w.sub(&b[1], &a[1])?;
        let z = w.sub(&b[2], &a[2])?;
        out.push([enclosure(&x, w)?, enclosure(&y, w)?, enclosure(&z, w)?]);
    }
    Some(out)
}
pub(super) struct Prepared {
    pub a: Side,
    pub b: Side,
    pub v: Polynomial,
    pub vv: Polynomial,
    pub delta: [Polynomial; 3],
    #[cfg(test)]
    pub used: usize,
}
pub(super) fn prepare_pair(left: &Surface, right: &Surface, allowance: usize) -> Option<Prepared> {
    let mut work = Work { left: allowance };
    let w = &mut work;
    let a = prepare(left, true, w)?;
    let b = prepare(right, false, w)?;
    let q = left.degree_v;
    let hi = w.input(left.knots_v[q + 1])?;
    let lo = w.input(left.knots_v[q])?;
    let width = w.sub(&hi, &lo)?;
    let mut seam = Vec::new();
    for p in left.control_points.last()? {
        seam.push([w.input(p[0])?, w.input(p[1])?, w.input(p[2])?]);
    }
    let v = difference(&seam, &width, w)?;
    let vv = difference(&v, &width, w)?;
    let delta = [
        delta(&a.u, &b.u, w)?,
        delta(&a.uu, &b.uu, w)?,
        delta(&a.uv, &b.uv, w)?,
    ];
    let a = Side {
        u: intervals(&a.u, w)?,
        uu: intervals(&a.uu, w)?,
        uv: intervals(&a.uv, w)?,
    };
    let b = Side {
        u: intervals(&b.u, w)?,
        uu: intervals(&b.uu, w)?,
        uv: intervals(&b.uv, w)?,
    };
    let v = intervals(&v, w)?;
    let vv = intervals(&vv, w)?;
    Some(Prepared {
        a,
        b,
        v,
        vv,
        delta,
        #[cfg(test)]
        used: allowance - work.left,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn pair() -> [Surface; 2] {
        let weight = 2f64.powi(30);
        let near = 2f64.powi(-30);
        let far = 4. - 2f64.powi(-29);
        [
            (vec![-far, -near, 0.], vec![weight, weight, 1.], -1.),
            (vec![0., near, far], vec![1., weight, weight], 0.),
        ]
        .map(|(x, w, start)| Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: vec![start, start, start, start + 1., start + 1., start + 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: x
                .iter()
                .map(|&x| [0., 0.5, 1.].iter().map(|&y| vec![x, y, x]).collect())
                .collect(),
            weights: w.iter().map(|&w| vec![w; 3]).collect(),
            periodic_u: false,
            periodic_v: false,
        })
    }
    #[test]
    fn original_large_term_cancellation_is_exact_and_not_approximate_snapping() {
        let [a, mut b] = pair();
        let before = [a.clone(), b.clone()];
        let p = prepare_pair(&a, &b, 10000).unwrap();
        assert!(
            p.delta
                .iter()
                .flatten()
                .flatten()
                .all(|x| x.lo == 0. && x.hi == 0.)
        );
        assert!(p.used < 10000 && p.used > 0);
        let proof = inspect_original_patch_pair(&a, &b, 1e-5, 1e-4, 2000).unwrap();
        assert_eq!(proof.decision, Decision::WithinTolerance);
        assert!(proof.regularity_proven);
        assert_eq!([a.clone(), b.clone()], before);
        for p in &mut b.control_points[2] {
            p[2] += 1e-12;
        }
        let proof = inspect_original_patch_pair(&a, &b, 1e-5, 1e-6, 2000).unwrap();
        assert_eq!(proof.decision, Decision::CorrespondenceExceedsTolerance);
        assert!(prepare_pair(&a, &b, 1).is_none());
    }
    #[test]
    fn outward_conversion_is_checked_against_the_exact_fraction() {
        let q = Q::new(1.into(), 3.into());
        let mut w = Work { left: 100 };
        let bound = enclosure(&q, &mut w).unwrap();
        assert!(Q::from_float(bound.lo).unwrap() <= q && Q::from_float(bound.hi).unwrap() >= q);
        assert_eq!(enclosure(&Q::zero(), &mut w).unwrap().lo, 0.);
    }
}
