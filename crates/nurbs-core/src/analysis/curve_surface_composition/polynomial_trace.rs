//! Exact coefficient proposal for polynomial Bezier supports along a straight
//! authored pcurve. Cancel zero leading powers BEFORE converting to Bernstein;
//! a rounded high-degree elevation must not become the shared 3D edge.
use super::*;
use num_rational::BigRational as Q;
use num_traits::{One, ToPrimitive, Zero};
type P = Vec<Q>;
struct Work(usize);
impl Work {
    fn charge(&mut self, q: &Q) -> Option<()> {
        self.0 = self.0.checked_sub(1)?;
        (q.numer().bits() <= 8192 && q.denom().bits() <= 8192).then_some(())
    }
}
fn q(x: f64) -> Option<Q> {
    Q::from_float(x)
}
fn integer(n: usize) -> Q {
    Q::from_integer(n.into())
}
fn binomial(n: usize, k: usize) -> Q {
    let mut c = 1usize;
    for j in 0..k {
        c = c * (n - j) / (j + 1);
    }
    integer(c)
}
fn mul(a: &P, b: &P, w: &mut Work) -> Option<P> {
    let mut c = vec![Q::zero(); a.len() + b.len() - 1];
    for (i, x) in a.iter().enumerate() {
        for (j, y) in b.iter().enumerate() {
            if x.is_zero() || y.is_zero() {
                continue;
            }
            c[i + j] += x * y;
            w.charge(&c[i + j])?;
        }
    }
    Some(c)
}
fn powers(a: &P, n: usize, w: &mut Work) -> Option<Vec<P>> {
    let mut p = vec![vec![Q::one()]];
    for i in 0..n {
        p.push(mul(&p[i], a, w)?);
    }
    Some(p)
}
pub(super) fn candidate(p: &Curve, s: &Surface) -> Option<Curve> {
    let [du, dv] = [s.degree_u, s.degree_v];
    if p.control_points[0].len() != 2
        || s.control_points[0][0].len() != 3
        || p.periodic
        || s.periodic_u
        || s.periodic_v
        || p.degree != 1
        || p.control_points.len() != 2
        || p.weights[0] != p.weights[1]
        || du > 8
        || dv > 8
        || du + dv < 2
        || !bezier(&p.knots, p.degree, p.control_points.len())
        || !bezier(&s.knots_u, du, s.control_points.len())
        || !bezier(&s.knots_v, dv, s.control_points[0].len())
        || s.weights.iter().flatten().any(|v| *v != s.weights[0][0])
    {
        return None;
    }
    let domains = [
        [s.knots_u[du], s.knots_u[du + 1]],
        [s.knots_v[dv], s.knots_v[dv + 1]],
    ];
    let mut w = Work(200_000);
    let mut basis = Vec::new();
    for (axis, n) in [(0, du), (1, dv)] {
        let d = domains[axis];
        if p.control_points
            .iter()
            .any(|v| v[axis] < d[0] || v[axis] > d[1])
        {
            return None;
        }
        let width = q(d[1])? - q(d[0])?;
        let a = (q(p.control_points[0][axis])? - q(d[0])?) / &width;
        let b = (q(p.control_points[1][axis])? - q(p.control_points[0][axis])?) / &width;
        w.charge(&a)?;
        w.charge(&b)?;
        let up = powers(&vec![a.clone(), b.clone()], n, &mut w)?;
        let down = powers(&vec![Q::one() - a, -b], n, &mut w)?;
        let mut row = Vec::new();
        for i in 0..=n {
            let factor = binomial(n, i);
            let mut poly = mul(&up[i], &down[n - i], &mut w)?;
            for c in &mut poly {
                *c *= &factor;
                w.charge(c)?;
            }
            row.push(poly);
        }
        basis.push(row);
    }
    let mut coords = vec![vec![Q::zero(); du + dv + 1]; 3];
    for i in 0..=du {
        for j in 0..=dv {
            let b = mul(&basis[0][i], &basis[1][j], &mut w)?;
            for k in 0..3 {
                let x = q(s.control_points[i][j][k])?;
                for (out, c) in coords[k].iter_mut().zip(&b) {
                    *out += &x * c;
                    w.charge(out)?;
                }
            }
        }
    }
    let degree = coords
        .iter()
        .map(|p| p.iter().rposition(|c| !c.is_zero()).unwrap_or(0))
        .max()?
        .max(1);
    let mut points = Vec::new();
    for i in 0..=degree {
        let mut point = Vec::new();
        for p in &coords {
            let mut c = Q::zero();
            for k in 0..=i {
                c += &p[k] * binomial(i, k) / binomial(degree, k);
                w.charge(&c)?;
            }
            let f = c.to_f64().filter(|v| v.is_finite())?;
            if q(f)? != c {
                return None;
            }
            point.push(f);
        }
        points.push(point);
    }
    let d = p.domain();
    let c = Curve {
        degree,
        control_points: points,
        weights: vec![1.; degree + 1],
        knots: [vec![d[0]; degree + 1], vec![d[1]; degree + 1]].concat(),
        periodic: false,
    };
    c.validate().ok()?;
    Some(c)
}
