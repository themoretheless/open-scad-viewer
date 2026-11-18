//! All original tensor spans compared with ORIGINAL global boundary clocks.
use super::super::homogeneous_affine::bspline::{restrict, Work, MAX_RESTRICTION_WORK};
use super::*;
pub(super) fn prove(
    s: &Surface,
    axes: [usize; 2],
    frame: &AffineFrame,
) -> Option<[super::super::clock::Clock; 2]> {
    prove_with_limit(s, axes, frame, MAX_RESTRICTION_WORK)
}
fn prove_with_limit(
    s: &Surface,
    axes: [usize; 2],
    frame: &AffineFrame,
    limit: u64,
) -> Option<[super::super::clock::Clock; 2]> {
    let (p, r) = (s.degree_u, s.degree_v);
    let (nu, nv) = (s.control_points.len(), s.control_points[0].len());
    let mut work = Work::new(limit);
    let uc: Vec<[Q; 2]> = (0..nu)
        .map(|i| {
            let h = super::pole(s, i, 0, axes, frame);
            [h[0].clone(), h[2].clone()]
        })
        .collect();
    let vc: Vec<[Q; 2]> = (0..nv)
        .map(|j| {
            let h = super::pole(s, 0, j, axes, frame);
            [h[1].clone(), h[2].clone()]
        })
        .collect();
    let clock = |degree, knots: &[f64], c: &[[Q; 2]]| {
        super::super::clock::Clock::bspline(
            degree,
            knots,
            c.iter().map(|h| h[1].clone()).collect(),
            &c.iter().map(|h| &h[0] / &h[1]).collect::<Vec<_>>(),
        )
        .ok()
    };
    let clocks = [clock(p, &s.knots_u, &uc)?, clock(r, &s.knots_v, &vc)?];
    let ku: Vec<_> = s.knots_u.iter().map(|x| q(*x)).collect();
    let kv: Vec<_> = s.knots_v.iter().map(|x| q(*x)).collect();
    for iu in p..nu {
        if ku[iu] == ku[iu + 1] {
            continue;
        }
        let cu = restrict(uc[iu - p..=iu].to_vec(), &ku, p, iu, &mut work)?;
        for iv in r..nv {
            if kv[iv] == kv[iv + 1] {
                continue;
            }
            work.charge(
                ((p + 1) * (r + 1) * 80
                    + (2 * p + 1) * (r + 1) * (p + 1) * 20
                    + (2 * r + 1) * (p + 1) * (r + 1) * 20) as u64,
            )?;
            let cv = restrict(vc[iv - r..=iv].to_vec(), &kv, r, iv, &mut work)?;
            let mut h = vec![vec![[q(0.), q(0.), q(0.)]; r + 1]; p + 1];
            for j in 0..=r {
                let base = (iu - p..=iu)
                    .map(|i| super::pole(s, i, iv - r + j, axes, frame))
                    .collect();
                let c = restrict(base, &ku, p, iu, &mut work)?;
                for i in 0..=p {
                    h[i][j] = c[i].clone();
                }
            }
            for row in &mut h {
                *row = restrict(row.clone(), &kv, r, iv, &mut work)?;
            }
            // Surface cell and each original global boundary are restricted
            // independently to the SAME original u/v span. Local cell edges
            // never replace the owner boundary clock.
            if !super::certificate(&h, &cu, &cv) {
                return None;
            }
        }
    }
    Some(clocks)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn basis(i: usize, p: usize, k: &[Q], t: &Q) -> Q {
        if p == 0 {
            return if k[i] <= *t && *t < k[i + 1] {
                q(1.)
            } else {
                q(0.)
            };
        }
        let mut a = q(0.);
        if k[i + p] > k[i] {
            a += (t - &k[i]) / (&k[i + p] - &k[i]) * basis(i, p - 1, k, t);
        }
        if k[i + p + 1] > k[i + 1] {
            a += (&k[i + p + 1] - t) / (&k[i + p + 1] - &k[i + 1]) * basis(i + 1, p - 1, k, t);
        }
        a
    }
    #[test]
    fn original_multispan_tensor_clock_certificate_covers_all_degrees_and_independent_cox_inverse()
    {
        for p in 1..=8 {
            for r in 1..=8 {
                let ku = [vec![0.; p + 1], vec![1., 3.], vec![4.; p + 1]].concat();
                let kv = [vec![0.; r + 1], vec![1., 3.], vec![4.; r + 1]].concat();
                let (nu, nv) = (p + 3, r + 3);
                let x: Vec<f64> = (0..nu)
                    .map(|i| ku[i + 1..=i + p].iter().sum::<f64>().powi(2))
                    .collect();
                let y: Vec<f64> = (0..nv)
                    .map(|j| kv[j + 1..=j + r].iter().sum::<f64>().powi(2))
                    .collect();
                let mut s = Surface {
                    degree_u: p,
                    degree_v: r,
                    knots_u: ku,
                    knots_v: kv,
                    control_points: x
                        .iter()
                        .map(|x| y.iter().map(|y| vec![*x, *y, 0.]).collect())
                        .collect(),
                    weights: vec![vec![1.; nv]; nu],
                    periodic_u: false,
                    periodic_v: false,
                };
                let (o, u, v) = (
                    [q(0.), q(0.)],
                    [q((16 * p * p) as f64), q(0.)],
                    [q(0.), q((16 * r * r) as f64)],
                );
                let det = &u[0] * &v[1];
                let frame = AffineFrame { origin: o.clone(), u: u.clone(), v: v.clone(), det };
                let c = prove(&s, [0, 1], &frame).unwrap();
                for (axis, (degree, knots, values, end)) in
                    [(p, &s.knots_u, &x, &u[0]), (r, &s.knots_v, &y, &v[1])]
                        .into_iter()
                        .enumerate()
                {
                    let k: Vec<_> = knots.iter().map(|x| q(*x)).collect();
                    let a = (0..values.len())
                        .map(|i| basis(i, degree, &k, &q(2.)) * q(values[i]) / end)
                        .sum::<Q>();
                    assert_eq!(c[axis].inverse(&a), [q(0.5), q(0.5)], "degrees {p},{r}");
                }
                assert!(prove_with_limit(&s, [0, 1], &frame, 0).is_none());
                s.control_points[1][1][0] = f64::from_bits(s.control_points[1][1][0].to_bits() + 1);
                assert!(prove(&s, [0, 1], &frame).is_none());
            }
        }
    }
}
