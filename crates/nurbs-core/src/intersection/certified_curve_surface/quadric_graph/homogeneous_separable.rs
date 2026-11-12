//! Exact original tensor identities against independent boundary clocks.
use super::*;
mod bspline;
fn choose(n: usize, k: usize) -> Q {
    q((0..k).fold(1u64, |c, j| c * (n - j) as u64 / (j + 1) as u64) as f64)
}
pub(super) fn prove(
    s: &Surface,
    axes: [usize; 2],
    frame: &AffineFrame,
) -> Option<[super::clock::Clock; 2]> {
    let (p, r) = (s.degree_u, s.degree_v);
    if s.control_points.len() != p + 1 || s.control_points[0].len() != r + 1 {
        return bspline::prove(s, axes, frame);
    }
    // Channels [projected-u numerator, projected-v numerator, W], all
    // authored Euclidean source poles multiplied by their ORIGINAL weights.
    let h: Vec<Vec<[Q; 3]>> = (0..=p)
        .map(|i| (0..=r).map(|j| pole(s, i, j, axes, frame)).collect())
        .collect();
    let uc: Vec<_> = (0..=p)
        .map(|i| [h[i][0][0].clone(), h[i][0][2].clone()])
        .collect();
    let vc: Vec<_> = (0..=r)
        .map(|j| [h[0][j][1].clone(), h[0][j][2].clone()])
        .collect();
    let clocks = [
        super::clock::Clock::certified(uc.clone())?,
        super::clock::Clock::certified(vc.clone())?,
    ];
    if !certificate(&h, &uc, &vc) {
        return None;
    }
    Some(clocks)
}
pub(super) fn pole(
    s: &Surface,
    i: usize,
    j: usize,
    axes: [usize; 2],
    frame: &AffineFrame,
) -> [Q; 3] {
    let AffineFrame { origin: o, u, v, det } = frame;
    let x = q(s.control_points[i][j][axes[0]]) - &o[0];
    let y = q(s.control_points[i][j][axes[1]]) - &o[1];
    let w = q(s.weights[i][j]);
    [
        (&x * &v[1] - &y * &v[0]) / det * &w,
        (&u[0] * &y - &u[1] * &x) / det * &w,
        w,
    ]
}
fn certificate(h: &[Vec<[Q; 3]>], uc: &[[Q; 2]], vc: &[[Q; 2]]) -> bool {
    let (p, r) = (h.len() - 1, h[0].len() - 1);
    // Au*Du_boundary-W*Nu_boundary = 0 in degree (2p,r).
    // Common positive binomial denominators can be omitted for equality.
    // No sampled points, rounded extracted poles or caller factors are used.
    for n in 0..=2 * p {
        for j in 0..=r {
            let mut residual = q(0.);
            for i in 0..=p {
                if n < i || n - i > p {
                    continue;
                }
                let k = n - i;
                residual += choose(p, i)
                    * choose(p, k)
                    * (&h[i][j][0] * &uc[k][1] - &h[i][j][2] * &uc[k][0]);
            }
            if residual != q(0.) {
                return false;
            }
        }
    }
    for i in 0..=p {
        for n in 0..=2 * r {
            let mut residual = q(0.);
            for j in 0..=r {
                if n < j || n - j > r {
                    continue;
                }
                let k = n - j;
                residual += choose(r, j)
                    * choose(r, k)
                    * (&h[i][j][1] * &vc[k][1] - &h[i][j][2] * &vc[k][0]);
            }
            if residual != q(0.) {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_tensor_cross_identities_and_boundary_inverses_cover_all_degree_pairs() {
        for p in 1..=8 {
            for r in 1..=8 {
                let mut s = Surface {
                    degree_u: p,
                    degree_v: r,
                    knots_u: [vec![0.; p + 1], vec![1.; p + 1]].concat(),
                    knots_v: [vec![0.; r + 1], vec![1.; r + 1]].concat(),
                    control_points: (0..=p)
                        .map(|i| {
                            (0..=r)
                                .map(|j| vec![(i * i) as f64, (j * j) as f64, 0.])
                                .collect()
                        })
                        .collect(),
                    weights: vec![vec![1.; r + 1]; p + 1],
                    periodic_u: false,
                    periodic_v: false,
                };
                let o = [q(0.), q(0.)];
                let u = [q((p * p) as f64), q(0.)];
                let v = [q(0.), q((r * r) as f64)];
                let det = q((p * p * r * r) as f64);
                let frame = AffineFrame { origin: o, u, v, det };
                let c = prove(&s, [0, 1], &frame).unwrap();
                for (axis, d) in [p, r].into_iter().enumerate() {
                    let a = q(0.25) + q(0.25) / q(d as f64);
                    assert_eq!(c[axis].inverse(&a), [q(0.5), q(0.5)]);
                }
                s.control_points[1][1][0] = f64::from_bits(1f64.to_bits() + 1);
                assert!(prove(&s, [0, 1], &frame).is_none());
            }
        }
    }
}
