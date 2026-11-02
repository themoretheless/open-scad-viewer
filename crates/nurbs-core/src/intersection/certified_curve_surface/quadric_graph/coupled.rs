//! Joint inverse of a positive rational bilinear parallelogram projection.
use super::*;
use num_traits::Zero;
/// Flattened weights [w00,w01,w10,w11]. Let a,b be projected means and z=P11.
/// The four positive barycentric probabilities satisfy
/// z(1-a-b+z) = K(a-z)(b-z), K=w00*w11/(w01*w10).
/// The odds ratio is strictly increasing over the feasible z interval,
/// proving existence and uniqueness. This also proves global coverage.
fn inverse(w: &[Q; 4], a: &Q, b: &Q) -> [[Q; 2]; 2] {
    let mobius = |x: &Q, l: &Q, r: &Q| l * x / (r * (q(1.) - x) + l * x);
    if a.is_zero() || *a == q(1.) {
        let i = if a.is_zero() { 0 } else { 2 };
        let v = mobius(b, &w[i], &w[i + 1]);
        return [[a.clone(), a.clone()], [v.clone(), v]];
    }
    if b.is_zero() || *b == q(1.) {
        let j = usize::from(!b.is_zero());
        let u = mobius(a, &w[j], &w[j + 2]);
        return [[u.clone(), u], [b.clone(), b.clone()]];
    }
    let mut lo = (a + b - q(1.)).max(q(0.));
    let mut hi = a.clone().min(b.clone());
    for _ in 0..64 {
        let z = (&lo + &hi) / q(2.);
        let f = &w[1] * &w[2] * &z * (q(1.) - a - b + &z) - &w[0] * &w[3] * (a - &z) * (b - &z);
        if f.is_zero() {
            lo = z.clone();
            hi = z;
            break;
        }
        if f < Q::zero() {
            lo = z;
        } else {
            hi = z;
        }
    }
    let uv = |z: &Q| {
        let p00 = q(1.) - a - b + z;
        let p10 = a - z;
        let p01 = b - z;
        [
            &w[0] * &p10 / (&w[2] * &p00 + &w[0] * &p10),
            &w[0] * &p01 / (&w[1] * &p00 + &w[0] * &p01),
        ]
    };
    let low = uv(&hi);
    let high = uv(&lo);
    std::array::from_fn(|i| [low[i].clone(), high[i].clone()])
}
/// The log-partition Hessian is the covariance of the four full-support
/// corner states, hence positive definite in the interior. Inverse diagonal
/// derivatives are positive; cross derivatives have the fixed opposite sign
/// of w00*w11-w01*w10. Therefore each inverse coordinate has its extrema at
/// rectangle corners. Edge limits are the positive Mobius maps above.
pub(super) fn enclosure(w: &[Q; 4], r: &[[Q; 2]; 2]) -> [[Q; 2]; 2] {
    let corners: Vec<_> = (0..2)
        .flat_map(|i| (0..2).map(move |j| inverse(w, &r[0][i], &r[1][j])))
        .collect();
    std::array::from_fn(|axis| {
        [
            corners.iter().map(|c| c[axis][0].clone()).min().unwrap(),
            corners.iter().map(|c| c[axis][1].clone()).max().unwrap(),
        ]
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn forward(w: &[Q; 4], u: &Q, v: &Q) -> [Q; 2] {
        let h = [
            &w[0] * (q(1.) - u) * (q(1.) - v),
            &w[1] * (q(1.) - u) * v,
            &w[2] * u * (q(1.) - v),
            &w[3] * u * v,
        ];
        let d = h.iter().cloned().sum::<Q>();
        [(&h[2] + &h[3]) / &d, (&h[1] + &h[3]) / d]
    }
    #[test]
    fn joint_inverse_and_rectangle_cover_independent_rational_forward_map() {
        for raw in [[1., 3., 2., 7.], [1., 3., 2., 1.], [7., 7., 7., 7.]] {
            let w = raw.map(q);
            for i in 0..=8 {
                for j in 0..=8 {
                    let u = q(i as f64) / q(8.);
                    let v = q(j as f64) / q(8.);
                    let a = forward(&w, &u, &v);
                    let r = inverse(&w, &a[0], &a[1]);
                    assert!(r[0][0] <= u && u <= r[0][1]);
                    assert!(r[1][0] <= v && v <= r[1][1]);
                    let ranges =
                        a.map(|x| [(x.clone() - q(0.125)).max(q(0.)), (x + q(0.125)).min(q(1.))]);
                    let e = enclosure(&w, &ranges);
                    assert!(e[0][0] <= u && u <= e[0][1]);
                    assert!(e[1][0] <= v && v <= e[1][1]);
                }
            }
        }
    }
}
