//! Exact Cox recurrence on each original nonempty span. No sampled inversion,
//! rounded knot insertion or fitted transformed control net enters this ideal.
use super::*;
fn add_linear(out: &mut [Q], input: &[Q], a: &Q, b: &Q, w: &mut Work) -> Result<()> {
    for (i, c) in input.iter().enumerate() {
        w.charge()?;
        out[i] += c * a;
        w.coefficient(&out[i])?;
        w.charge()?;
        out[i + 1] += c * b;
        w.coefficient(&out[i + 1])?;
    }
    Ok(())
}
fn basis(knots: &[f64], degree: usize, span: usize, w: &mut Work) -> Result<Vec<Vec<Q>>> {
    check(
        span >= degree && span + degree + 1 < knots.len() && knots[span] < knots[span + 1],
        "Invalid original exact SS span",
    )?;
    let a = q(knots[span])?;
    let width = q(knots[span + 1])? - &a;
    let start = span - degree;
    let mut base = vec![vec![Q::zero()]; degree + 2];
    base[degree][0] = Q::one();
    for d in 1..=degree {
        let mut next = vec![vec![Q::zero(); d + 1]; degree + 2];
        for local in degree - d..=degree {
            let i = start + local;
            let left = q(knots[i + d])? - q(knots[i])?;
            if !left.is_zero() {
                add_linear(
                    &mut next[local],
                    &base[local],
                    &((&a - q(knots[i])?) / &left),
                    &(&width / &left),
                    w,
                )?;
            }
            let right = q(knots[i + d + 1])? - q(knots[i + 1])?;
            if !right.is_zero() {
                add_linear(
                    &mut next[local],
                    &base[local + 1],
                    &((q(knots[i + d + 1])? - &a) / &right),
                    &(-&width / &right),
                    w,
                )?;
            }
        }
        base = next;
    }
    base.truncate(degree + 1);
    Ok(base)
}
pub(super) fn homogeneous(
    s: &Surface,
    spans: [usize; 2],
    support: usize,
    w: &mut Work,
) -> Result<[ExactSsPolynomial; 4]> {
    let u = basis(&s.knots_u, s.degree_u, spans[0], w)?;
    let v = basis(&s.knots_v, s.degree_v, spans[1], w)?;
    let mut out = std::array::from_fn(|_| ExactSsPolynomial::default());
    for local_u in 0..=s.degree_u {
        for local_v in 0..=s.degree_v {
            let i = spans[0] - s.degree_u + local_u;
            let j = spans[1] - s.degree_v + local_v;
            let weight = q(s.weights[i][j])?;
            for axis in 0..4 {
                let h = if axis < 3 {
                    w.charge()?;
                    q(s.control_points[i][j][axis])? * &weight
                } else {
                    weight.clone()
                };
                w.coefficient(&h)?;
                for (a, uc) in u[local_u].iter().enumerate() {
                    if uc.is_zero() {
                        continue;
                    }
                    for (b, vc) in v[local_v].iter().enumerate() {
                        if vc.is_zero() {
                            continue;
                        }
                        w.charge()?;
                        let coefficient = uc * vc * &h;
                        w.coefficient(&coefficient)?;
                        let mut e = [0; 4];
                        e[2 * support] = a;
                        e[2 * support + 1] = b;
                        out[axis].insert(e, coefficient, w)?;
                    }
                }
            }
        }
    }
    Ok(out)
}
