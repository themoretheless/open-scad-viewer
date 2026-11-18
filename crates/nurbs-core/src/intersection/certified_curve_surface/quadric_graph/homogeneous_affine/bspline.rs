//! Original homogeneous tensor restriction, never binary64 extracted poles.
use super::*;
pub(in crate::intersection::certified_curve_surface::quadric_graph) const MAX_RESTRICTION_WORK:
    u64 = 4_000_000;
pub(in crate::intersection::certified_curve_surface::quadric_graph) struct Work(u64);
impl Work {
    pub(in crate::intersection::certified_curve_surface::quadric_graph) fn new(n: u64) -> Self {
        Self(n)
    }
    pub(in crate::intersection::certified_curve_surface::quadric_graph) fn charge(
        &mut self,
        n: u64,
    ) -> Option<()> {
        self.0 = self.0.checked_sub(n)?;
        Some(())
    }
}
pub(in crate::intersection::certified_curve_surface::quadric_graph) fn restrict<const N: usize>(
    base: Vec<[Q; N]>,
    knots: &[Q],
    p: usize,
    span: usize,
    work: &mut Work,
) -> Option<Vec<[Q; N]>> {
    let mut out = Vec::new();
    for b in 0..=p {
        let mut d = base.clone();
        for r in 1..=p {
            let t = &knots[span + usize::from(r > p - b)];
            for j in (r..=p).rev() {
                work.charge(8 * N as u64)?;
                let i = span - p + j;
                let den = &knots[i + p - r + 1] - &knots[i];
                if den <= q(0.) {
                    return None;
                }
                let a = (t - &knots[i]) / den;
                let z = q(1.) - &a;
                d[j] = std::array::from_fn(|axis| &z * &d[j - 1][axis] + &a * &d[j][axis]);
            }
        }
        out.push(d[p].clone());
    }
    Some(out)
}
pub(super) fn prove(
    s: &Surface,
    axes: [usize; 2],
    origin: &[Q; 2],
    u: &[Q; 2],
    v: &[Q; 2],
) -> bool {
    prove_with_limit(s, axes, origin, u, v, MAX_RESTRICTION_WORK).unwrap_or(false)
}
fn prove_with_limit(
    s: &Surface,
    axes: [usize; 2],
    origin: &[Q; 2],
    u: &[Q; 2],
    v: &[Q; 2],
    limit: u64,
) -> Option<bool> {
    let mut work = Work(limit);
    let (p, r) = (s.degree_u, s.degree_v);
    let (nu, nv) = (s.control_points.len(), s.control_points[0].len());
    let ku: Vec<_> = s.knots_u.iter().map(|x| q(*x)).collect();
    let kv: Vec<_> = s.knots_v.iter().map(|x| q(*x)).collect();
    let wu = &ku[nu] - &ku[p];
    let wv = &kv[nv] - &kv[r];
    for iu in p..nu {
        if ku[iu] == ku[iu + 1] {
            continue;
        }
        for iv in r..nv {
            if kv[iv] == kv[iv + 1] {
                continue;
            }
            work.charge(((p + 1) * (r + 1) * 3 + (p + 2) * (r + 2) * 128) as u64)?;
            let mut h = vec![vec![[q(0.), q(0.), q(0.)]; r + 1]; p + 1];
            for j in 0..=r {
                let base = (iu - p..=iu)
                    .map(|i| {
                        let w = q(s.weights[i][iv - r + j]);
                        [
                            q(s.control_points[i][iv - r + j][axes[0]]) * &w,
                            q(s.control_points[i][iv - r + j][axes[1]]) * &w,
                            w,
                        ]
                    })
                    .collect();
                let c = restrict(base, &ku, p, iu, &mut work)?;
                for i in 0..=p {
                    h[i][j] = c[i].clone();
                }
            }
            for row in &mut h {
                *row = restrict(row.clone(), &kv, r, iv, &mut work)?;
            }
            let a = (&ku[iu] - &ku[p]) / &wu;
            let b = (&kv[iv] - &kv[r]) / &wv;
            let offset = std::array::from_fn(|k| &origin[k] + &u[k] * &a + &v[k] * &b);
            let us = std::array::from_fn(|k| &u[k] * (&ku[iu + 1] - &ku[iu]) / &wu);
            let vs = std::array::from_fn(|k| &v[k] * (&kv[iv + 1] - &kv[iv]) / &wv);
            if !super::certificate(&h, &offset, &us, &vs) {
                return Some(false);
            }
        }
    }
    Some(true)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn multispan_original_basis_certificate_covers_all_degree_pairs_and_bounded_refusal() {
        for p in 1..=8 {
            for r in 1..=8 {
                let ku = [vec![0.; p + 1], vec![1., 3.], vec![4.; p + 1]].concat();
                let kv = [vec![0.; r + 1], vec![1., 3.], vec![4.; r + 1]].concat();
                let (nu, nv) = (p + 3, r + 3);
                let x: Vec<_> = (0..nu)
                    .map(|i| ku[i + 1..=i + p].iter().sum::<f64>())
                    .collect();
                let y: Vec<_> = (0..nv)
                    .map(|j| kv[j + 1..=j + r].iter().sum::<f64>())
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
                    [q(4. * p as f64), q(0.)],
                    [q(0.), q(4. * r as f64)],
                );
                assert!(prove(&s, [0, 1], &o, &u, &v), "degrees {p},{r}");
                assert!(prove_with_limit(&s, [0, 1], &o, &u, &v, 0).is_none());
                s.control_points[1][1][0] = f64::from_bits(s.control_points[1][1][0].to_bits() + 1);
                assert!(!prove(&s, [0, 1], &o, &u, &v));
            }
        }
    }
}
