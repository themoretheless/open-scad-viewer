//! Literal-output Gram coordinates, exact conic coefficients and winding.
use super::{Curve, POLES, PResult, Q, Refusal, Work};
use num_traits::Zero;
fn dot(a: &[Q], b: &[Q], w: &mut Work) -> PResult<Q> {
    let mut s = Q::zero();
    for (a, b) in a.iter().zip(b) {
        let p = w.op(a, b, |a, b| a * b)?;
        s = w.op(&s, &p, |a, b| a + b)?;
    }
    Ok(s)
}
fn choose(n: usize, k: usize) -> u64 {
    (0..k).fold(1, |a, i| a * (n - i) as u64 / (i + 1) as u64)
}
fn product(a: &[Q], b: &[Q], w: &mut Work) -> PResult<Vec<Q>> {
    let p = a.len() - 1;
    let q = b.len() - 1;
    let mut c = vec![Q::zero(); p + q + 1];
    for (i, x) in a.iter().enumerate() {
        for (j, y) in b.iter().enumerate() {
            let factor = Q::new(
                (choose(p, i) * choose(q, j)).into(),
                choose(p + q, i + j).into(),
            );
            let v = w.op(x, y, |a, b| a * b)?;
            let v = w.op(&v, &factor, |a, b| a * b)?;
            c[i + j] = w.op(&c[i + j], &v, |a, b| a + b)?;
        }
    }
    Ok(c)
}
fn derivative(a: &[Q], w: &mut Work) -> PResult<Vec<Q>> {
    let degree = Q::from_integer((a.len() - 1).into());
    a.windows(2)
        .map(|p| {
            let d = w.op(&p[1], &p[0], |a, b| a - b)?;
            Ok(w.op(&d, &degree, |a, b| a * b)?)
        })
        .collect()
}
pub(super) fn verify(
    center: [f64; 3],
    a: [f64; 3],
    b: [f64; 3],
    c: &Curve,
    w: &mut Work,
) -> PResult<()> {
    // Bind the Bernstein-piece proof to the ACTUAL stored basis and dimensions.
    let expected = [0., 0., 0., 0.25, 0.25, 0.5, 0.5, 0.75, 0.75, 1., 1., 1.];
    if c.degree != 2
        || c.periodic
        || c.knots != expected
        || c.control_points.len() != 9
        || c.weights.len() != 9
        || c.control_points.iter().any(|p| p.len() != 3)
        || c.weights.iter().any(|&x| !x.is_finite() || x <= 0.)
    {
        return Err(Refusal::UnprovedIdentity);
    }
    let center = center
        .into_iter()
        .map(|x| w.lift(x))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let a = a
        .into_iter()
        .map(|x| w.lift(x))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let b = b
        .into_iter()
        .map(|x| w.lift(x))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let aa = dot(&a, &a, w)?;
    let ab = dot(&a, &b, w)?;
    let bb = dot(&b, &b, w)?;
    let aa_bb = w.op(&aa, &bb, |a, b| a * b)?;
    let ab_ab = w.op(&ab, &ab, |a, b| a * b)?;
    let det = w.op(&aa_bb, &ab_ab, |a, b| a - b)?;
    if det <= Q::zero() {
        return Err(Refusal::DependentAxes);
    }
    let mut normal = Vec::new();
    for (i, j) in [(1, 2), (2, 0), (0, 1)] {
        let x = w.op(&a[i], &b[j], |a, b| a * b)?;
        let y = w.op(&a[j], &b[i], |a, b| a * b)?;
        normal.push(w.op(&x, &y, |a, b| a - b)?);
    }
    let mut x = Vec::new();
    let mut y = Vec::new();
    let mut d = Vec::new();
    for (i, point) in c.control_points.iter().enumerate() {
        let weight = w.lift(c.weights[i])?;
        let mut t = Vec::new();
        for axis in 0..3 {
            let value = w.lift(point[axis])?;
            let h = w.op(&value, &weight, |a, b| a * b)?;
            let o = w.op(&center[axis], &weight, |a, b| a * b)?;
            t.push(w.op(&h, &o, |a, b| a - b)?);
        }
        if !dot(&normal, &t, w)?.is_zero() {
            return Err(Refusal::UnprovedIdentity);
        }
        let at = dot(&a, &t, w)?;
        let bt = dot(&b, &t, w)?;
        let l = w.op(&bb, &at, |a, b| a * b)?;
        let r = w.op(&ab, &bt, |a, b| a * b)?;
        x.push(w.op(&l, &r, |a, b| a - b)?);
        let l = w.op(&aa, &bt, |a, b| a * b)?;
        let r = w.op(&ab, &at, |a, b| a * b)?;
        y.push(w.op(&l, &r, |a, b| a - b)?);
        let dw = w.op(&det, &weight, |a, b| a * b)?;
        for (value, sign) in [(&x[i], POLES[i][0]), (&y[i], POLES[i][1])] {
            let expected = w.op(&dw, &Q::from_integer(sign.into()), |a, b| a * b)?;
            if *value != expected {
                return Err(Refusal::UnprovedIdentity);
            }
        }
        d.push(dw);
    }
    for start in [0, 2, 4, 6] {
        let a = &x[start..start + 3];
        let b = &y[start..start + 3];
        let d = &d[start..start + 3];
        let ax = product(a, a, w)?;
        let by = product(b, b, w)?;
        let dd = product(d, d, w)?;
        for ((a, b), d) in ax.iter().zip(&by).zip(&dd) {
            let s = w.op(a, b, |a, b| a + b)?;
            if !w.op(&s, d, |a, b| a - b)?.is_zero() {
                return Err(Refusal::UnprovedIdentity);
            }
        }
        let da = derivative(a, w)?;
        let db = derivative(b, w)?;
        let a_db = product(a, &db, w)?;
        let b_da = product(b, &da, w)?;
        // Strict positive angular numerator, plus canonical quadrant poles,
        // proves traversal of each full quarter exactly once, not just locus.
        for (a, b) in a_db.iter().zip(&b_da) {
            if w.op(a, b, |a, b| a - b)? <= Q::zero() {
                return Err(Refusal::UnprovedIdentity);
            }
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_output_damage_and_changed_basis_receive_no_identity_credit() {
        let c = super::super::ellipse([0.; 3], [1., 0., 0.], [0., 1., 0.], 1_000_000)
            .unwrap()
            .curve
            .unwrap();
        let run = |candidate: &Curve| {
            verify(
                [0.; 3],
                [1., 0., 0.],
                [0., 1., 0.],
                candidate,
                &mut Work {
                    used: 0,
                    limit: 1_000_000,
                },
            )
        };
        assert_eq!(run(&c), Ok(()));
        let mut wrong = c.clone();
        wrong.control_points[1][0] = 1f64.next_up();
        assert_eq!(run(&wrong), Err(Refusal::UnprovedIdentity));
        let mut wrong = c.clone();
        wrong.weights[1] = std::f64::consts::FRAC_1_SQRT_2;
        assert_eq!(run(&wrong), Err(Refusal::UnprovedIdentity));
        let mut wrong = c.clone();
        wrong.knots[3] = 0.2;
        wrong.knots[4] = 0.2;
        assert_eq!(run(&wrong), Err(Refusal::UnprovedIdentity));
        let mut scaled = c;
        for w in &mut scaled.weights {
            *w *= 2.;
        }
        assert_eq!(run(&scaled), Ok(()));
    }
}
