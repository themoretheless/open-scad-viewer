//! Exact original-source planar boundary Jacobian sign. Nonnegative Bernstein
//! coefficients with at least one positive coefficient give a strict interior
//! sign. Zero endpoints require separately owned pole coverage, never snapping.
use super::*;
use num_rational::BigRational as Q;
use num_traits::Zero;
type Poly = Vec<Q>;
pub(super) struct Work {
    left: usize,
}
impl Work {
    pub(super) fn new(cells: usize) -> Self {
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
}
fn choose(n: usize, k: usize) -> u128 {
    (0..k).fold(1, |v, i| v * (n - i) as u128 / (i + 1) as u128)
}
fn product(a: &Poly, b: &Poly, w: &mut Work) -> Option<Poly> {
    let p = a.len() - 1;
    let q = b.len() - 1;
    let mut out = Vec::new();
    for k in 0..=p + q {
        let mut sum = Q::zero();
        for i in 0..=p {
            if i > k || k - i > q {
                continue;
            }
            let factor = Q::new(
                (choose(p, i) * choose(q, k - i)).into(),
                choose(p + q, k).into(),
            );
            let v = w.mul(&a[i], &b[k - i])?;
            let v = w.mul(&v, &factor)?;
            sum = w.add(&sum, &v)?;
        }
        out.push(sum);
    }
    Some(out)
}
fn diff(a: &Poly, w: &mut Work) -> Option<Poly> {
    a.windows(2).map(|p| w.sub(&p[1], &p[0])).collect()
}
fn sub(a: &Poly, b: &Poly, w: &mut Work) -> Option<Poly> {
    if a.len() != b.len() {
        return None;
    }
    a.iter().zip(b).map(|(a, b)| w.sub(a, b)).collect()
}
#[derive(Clone, Copy)]
pub(super) struct Sign {
    value: i8,
    zero_start: bool,
    zero_end: bool,
    domain: [f64; 2],
}
impl Sign {
    pub(super) fn on(self, range: [f64; 2]) -> Option<i8> {
        (range[0] >= self.domain[0]
            && range[1] <= self.domain[1]
            && range[0] < range[1]
            && !(self.zero_start && range[0] == self.domain[0])
            && !(self.zero_end && range[1] == self.domain[1]))
            .then_some(self.value)
    }
}
pub(super) fn prepare(
    s: &Surface,
    span: usize,
    end: bool,
    height: usize,
    w: &mut Work,
) -> Option<Sign> {
    let p = s.degree_u;
    let q = s.degree_v;
    let n = s.control_points.len();
    let m = s.control_points[0].len();
    if !(1..=16).contains(&p) || q < 1 || height >= 3 || m != q + 1 || span < p || span >= n {
        return None;
    }
    let row = if end { q } else { 0 };
    let next = if end { q - 1 } else { 1 };
    let ids = span - p..=span;
    // Only actual equal row weights grant the Cartesian transverse quotient.
    if !ids.clone().all(|i| s.weights[i][row] == s.weights[i][next]) {
        return None;
    }
    let weights = ids
        .clone()
        .map(|i| w.input(s.weights[i][row]))
        .collect::<Option<Poly>>()?;
    if weights.iter().any(|v| v <= &Q::zero()) {
        return None;
    }
    let mut fields = Vec::new();
    for axis in [(height + 1) % 3, (height + 2) % 3] {
        let mut a = Vec::new();
        let mut b = Vec::new();
        for (i, weight) in ids.clone().zip(&weights) {
            let x = w.input(s.control_points[i][row][axis])?;
            let y = w.input(s.control_points[i][next][axis])?;
            a.push(w.mul(&x, weight)?);
            let d = if end { w.sub(&x, &y)? } else { w.sub(&y, &x)? };
            b.push(w.mul(&d, weight)?);
        }
        // Omit only the common POSITIVE U-degree/span and V-degree/span
        // scalars. The Jacobian denominator is W^3 > 0; all signs remain.
        let da = diff(&a, w)?;
        let dw = diff(&weights, w)?;
        let aw = product(&da, &weights, w)?;
        let wa = product(&a, &dw, w)?;
        fields.push((sub(&aw, &wa, w)?, b));
    }
    let a = product(&fields[0].0, &fields[1].1, w)?;
    let b = product(&fields[1].0, &fields[0].1, w)?;
    let d = sub(&a, &b, w)?;
    let positive = d.iter().all(|v| v >= &Q::zero()) && d.iter().any(|v| v > &Q::zero());
    let negative = d.iter().all(|v| v <= &Q::zero()) && d.iter().any(|v| v < &Q::zero());
    let value = if positive {
        1
    } else if negative {
        -1
    } else {
        return None;
    };
    Some(Sign {
        value,
        zero_start: d[0].is_zero(),
        zero_end: d.last()?.is_zero(),
        domain: [s.knots_u[span], s.knots_u[span + 1]],
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn collapsed_endpoint_needs_coverage_and_orientation_is_not_snapped() {
        let mut s = Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: (0..3)
                .map(|i| {
                    (0..3)
                        .map(|j| vec![if i == 2 { 1. } else { 0. }, i as f64 * j as f64 / 4., 0.])
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 3]; 3],
            periodic_u: false,
            periodic_v: false,
        };
        let sign = prepare(&s, 2, false, 2, &mut Work::new(100)).unwrap();
        assert_eq!(sign.on([0., 1.]), None);
        assert_eq!(sign.on([1e-100, 1.]), Some(1));
        for row in &mut s.control_points {
            for p in row {
                p[1] = -p[1];
            }
        }
        assert_eq!(
            prepare(&s, 2, false, 2, &mut Work::new(100))
                .unwrap()
                .on([0.1, 1.]),
            Some(-1)
        );
        assert!(prepare(&s, 2, false, 2, &mut Work { left: 0 }).is_none());
        s.weights[1][1] = 2.;
        assert!(prepare(&s, 2, false, 2, &mut Work::new(100)).is_none());
    }
    #[test]
    fn actual_fold_cannot_get_a_one_sign_receipt() {
        let s = Surface {
            degree_u: 1,
            degree_v: 2,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: [0., 1.]
                .iter()
                .map(|&u| {
                    (0..3)
                        .map(|j| vec![u, (u - 0.5) * j as f64 / 2., 0.])
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 3]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        assert!(prepare(&s, 1, false, 2, &mut Work::new(100)).is_none());
    }
}
