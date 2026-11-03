//! Original multi-span triangular projections; no rounded extracted poles.
use super::super::homogeneous_affine::bspline::{restrict, Work, MAX_RESTRICTION_WORK};
use super::*;
pub(in crate::intersection::certified_curve_surface::quadric_graph) struct Spline {
    axis: usize,
    clock: super::super::clock::Clock,
    degrees: [usize; 2],
    knots: [Vec<Q>; 2],
    conditional_knots: Vec<f64>,
    ordinates: Vec<Q>,
    weights: Vec<Vec<Q>>,
}
fn continuous(k: &[f64], p: usize, n: usize) -> bool {
    let mut i = p + 1;
    while i < n {
        let mut j = i + 1;
        while j < n && k[j] == k[i] {
            j += 1;
        }
        if j - i > p {
            return false;
        }
        i = j;
    }
    true
}
pub(super) fn prove(
    s: &Surface,
    axes: [usize; 2],
    o: &[Q; 2],
    u: &[Q; 2],
    v: &[Q; 2],
    det: &Q,
) -> Option<Spline> {
    prove_with_limit(s, axes, o, u, v, det, MAX_RESTRICTION_WORK)
}
fn prove_with_limit(
    s: &Surface,
    axes: [usize; 2],
    o: &[Q; 2],
    u: &[Q; 2],
    v: &[Q; 2],
    det: &Q,
    limit: u64,
) -> Option<Spline> {
    let (nu, nv) = (s.control_points.len(), s.control_points[0].len());
    if !continuous(&s.knots_u, s.degree_u, nu) || !continuous(&s.knots_v, s.degree_v, nv) {
        return None;
    }
    let h: Vec<Vec<[Q; 3]>> = (0..nu)
        .map(|i| {
            (0..nv)
                .map(|j| super::super::homogeneous_separable::pole(s, i, j, axes, o, u, v, det))
                .collect()
        })
        .collect();
    let mut work = Work::new(limit);
    for axis in 0..2 {
        let (n, m, p, r, ku, kv) = if axis == 0 {
            (nu, nv, s.degree_u, s.degree_v, &s.knots_u, &s.knots_v)
        } else {
            (nv, nu, s.degree_v, s.degree_u, &s.knots_v, &s.knots_u)
        };
        let at = |i: usize, j: usize| if axis == 0 { &h[i][j] } else { &h[j][i] };
        let c: Vec<_> = (0..m).map(|j| &at(0, j)[1 - axis] / &at(0, j)[2]).collect();
        if c[0] != q(0.) || c[m - 1] != q(1.) || c.windows(2).any(|x| x[0] > x[1]) {
            continue;
        }
        if !(0..n).all(|i| {
            (0..m).all(|j| at(i, j)[1 - axis] == &at(i, j)[2] * &c[j] && at(i, j)[2] > q(0.))
        }) {
            continue;
        }
        // Knot insertion interpolates adjacent homogeneous poles with
        // positive coefficients, preserving ordered Euclidean ordinates.
        // Each nonempty original span must have nonconstant active poles;
        // its restricted rational Bernstein clock is then strictly increasing.
        // C0 joins plus endpoints 0/1 give a global bijection for EVERY
        // positive conditional weight vector, not just sampled fibers.
        if (r..m).any(|j| kv[j] < kv[j + 1] && c[j - r] >= c[j]) {
            continue;
        }
        let boundary: Vec<[Q; 2]> = (0..n)
            .map(|i| [at(i, 0)[axis].clone(), at(i, 0)[2].clone()])
            .collect();
        let Ok(clock) = super::super::clock::Clock::bspline(
            p,
            ku,
            boundary.iter().map(|h| h[1].clone()).collect(),
            &boundary.iter().map(|h| &h[0] / &h[1]).collect::<Vec<_>>(),
        ) else {
            continue;
        };
        let k: Vec<_> = ku.iter().map(|x| q(*x)).collect();
        let mut independent = true;
        for span in p..n {
            if k[span] == k[span + 1] {
                continue;
            }
            let b = restrict(boundary[span - p..=span].to_vec(), &k, p, span, &mut work)?;
            for j in 0..m {
                let a = restrict(
                    (span - p..=span)
                        .map(|i| [at(i, j)[axis].clone(), at(i, j)[2].clone()])
                        .collect(),
                    &k,
                    p,
                    span,
                    &mut work,
                )?;
                work.charge(((2 * p + 1) * (p + 1) * 20) as u64)?;
                for z in 0..=2 * p {
                    let mut residual = q(0.);
                    for i in 0..=p {
                        if z >= i && z - i <= p {
                            let t = z - i;
                            residual += super::choose(p, i)
                                * super::choose(p, t)
                                * (&a[i][0] * &b[t][1] - &a[i][1] * &b[t][0]);
                        }
                    }
                    independent &= residual == q(0.);
                }
            }
        }
        if !independent {
            continue;
        }
        return Some(Spline {
            axis,
            clock,
            degrees: [p, r],
            knots: [k, kv.iter().map(|x| q(*x)).collect()],
            conditional_knots: kv.clone(),
            ordinates: c,
            weights: (0..n)
                .map(|i| (0..m).map(|j| at(i, j)[2].clone()).collect())
                .collect(),
        });
    }
    None
}
impl Spline {
    pub(super) fn enclosure(&self, r: &[[Q; 2]; 2]) -> Result<[[Q; 2]; 2]> {
        self.enclose(r)
            .ok_or_else(|| crate::input("Triangular spline inverse proof exhausted"))
    }
    fn enclose(&self, r: &[[Q; 2]; 2]) -> Option<[[Q; 2]; 2]> {
        self.enclose_with_limit(r, MAX_RESTRICTION_WORK)
    }
    fn enclose_with_limit(&self, r: &[[Q; 2]; 2], limit: u64) -> Option<[[Q; 2]; 2]> {
        let axis = self.axis;
        let lo = self.clock.inverse(&r[axis][0]);
        let hi = self.clock.inverse(&r[axis][1]);
        let independent = [lo[0].clone(), hi[1].clone()];
        let (p, d) = (self.degrees[0], self.degrees[1]);
        let n = self.weights.len();
        let m = self.ordinates.len();
        let k = &self.knots[0];
        let width = &k[n] - &k[p];
        let natural = independent.clone().map(|x| &k[p] + x * &width);
        let mut dependent = [q(1.), q(0.)];
        let mut found = false;
        let mut work = Work::new(limit);
        for span in p..n {
            if k[span] == k[span + 1] || k[span + 1] < natural[0] || k[span] > natural[1] {
                continue;
            }
            let local = [
                (natural[0].clone().max(k[span].clone()) - &k[span]) / (&k[span + 1] - &k[span]),
                (natural[1].clone().min(k[span + 1].clone()) - &k[span])
                    / (&k[span + 1] - &k[span]),
            ];
            let mut rows = vec![vec![q(0.); m]; p + 1];
            for j in 0..m {
                let column = restrict(
                    (span - p..=span)
                        .map(|i| [self.weights[i][j].clone()])
                        .collect(),
                    k,
                    p,
                    span,
                    &mut work,
                )?;
                let restricted = super::super::coupled_bernstein::restrict(
                    column.into_iter().map(|h| [h[0].clone(), q(1.)]).collect(),
                    &local,
                );
                for i in 0..=p {
                    rows[i][j] = restricted[i][0].clone();
                }
            }
            for row in rows {
                // The conditional basis remains the ORIGINAL global B-spline
                // basis. Only independent-direction positive weights change.
                work.charge((m * (d + 1) * (d + 1) * (d + 1) * 16) as u64)?;
                let clock = super::super::clock::Clock::bspline(
                    d,
                    &self.conditional_knots,
                    row,
                    &self.ordinates,
                )
                .ok()?;
                dependent[0] = dependent[0]
                    .clone()
                    .min(clock.inverse(&r[1 - axis][0])[0].clone());
                dependent[1] = dependent[1]
                    .clone()
                    .max(clock.inverse(&r[1 - axis][1])[1].clone());
                found = true;
            }
        }
        if !found {
            return None;
        }
        let mut uv = [independent, dependent];
        if axis == 1 {
            uv.swap(0, 1);
        }
        Some(uv)
    }
}

#[cfg(test)]
mod tests;
