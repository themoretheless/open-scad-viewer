//! Original triangular rational charts: one independent clock, one conditional.
use super::*;
pub(super) struct Triangular {
    axis: usize,
    clock: super::clock::Clock,
    ordinates: Vec<Q>,
    weights: Vec<Vec<Q>>,
}
impl Triangular {
    pub(super) fn prove(
        s: &Surface,
        axes: [usize; 2],
        o: &[Q; 2],
        u: &[Q; 2],
        v: &[Q; 2],
        det: &Q,
    ) -> Option<Self> {
        let (p, r) = (s.degree_u, s.degree_v);
        if s.control_points.len() != p + 1 || s.control_points[0].len() != r + 1 {
            return None;
        }
        let h: Vec<Vec<[Q; 3]>> = (0..=p)
            .map(|i| {
                (0..=r)
                    .map(|j| super::homogeneous_separable::pole(s, i, j, axes, o, u, v, det))
                    .collect()
            })
            .collect();
        for axis in 0..2 {
            let (n, m) = if axis == 0 { (p, r) } else { (r, p) };
            let at = |i: usize, j: usize| if axis == 0 { &h[i][j] } else { &h[j][i] };
            let boundary: Vec<_> = (0..=n)
                .map(|i| {
                    let a = at(i, 0);
                    [a[axis].clone(), a[2].clone()]
                })
                .collect();
            let Some(clock) = super::clock::Clock::certified(boundary.clone()) else {
                continue;
            };
            // A_independent * D_boundary - W * N_boundary == 0 on the
            // ORIGINAL tensor. Equality is a full Bernstein coefficient census.
            let mut independent = true;
            for k in 0..=2 * n {
                for j in 0..=m {
                    let mut residual = q(0.);
                    for i in 0..=n {
                        if k < i || k - i > n {
                            continue;
                        }
                        let z = k - i;
                        let a = at(i, j);
                        residual += choose(n, i)
                            * choose(n, z)
                            * (&a[axis] * &boundary[z][1] - &a[2] * &boundary[z][0]);
                    }
                    independent &= residual == q(0.);
                }
            }
            if !independent {
                continue;
            }
            let c: Vec<_> = (0..=m)
                .map(|j| {
                    let a = at(0, j);
                    &a[1 - axis] / &a[2]
                })
                .collect();
            if c[0] != q(0.) || c[m] != q(1.) || c.windows(2).any(|x| x[0] > x[1]) {
                continue;
            }
            if !(0..=n).all(|i| {
                (0..=m).all(|j| {
                    let a = at(i, j);
                    a[1 - axis] == &a[2] * &c[j] && a[2] > q(0.)
                })
            }) {
                continue;
            }
            // Positive conditional weights and nonconstant ordered ordinates
            // imply strictly positive covariance on (0,1). Thus every fixed
            // independent parameter has exactly one dependent preimage.
            return Some(Self {
                axis,
                clock,
                ordinates: c,
                weights: (0..=n)
                    .map(|i| (0..=m).map(|j| at(i, j)[2].clone()).collect())
                    .collect(),
            });
        }
        None
    }
    pub(super) fn enclosure(&self, r: &[[Q; 2]; 2]) -> Result<[[Q; 2]; 2]> {
        let a = self.axis;
        let lo = self.clock.inverse(&r[a][0]);
        let hi = self.clock.inverse(&r[a][1]);
        let independent = [lo[0].clone(), hi[1].clone()];
        let mut w = self.weights.clone();
        // Restriction preserves a positive Bernstein mixture. At every
        // independent parameter the conditional clock is a positive mixture
        // of these row clocks. Min/max row inverses therefore enclose it;
        // no sign assumption on mixed derivatives is used.
        for j in 0..self.ordinates.len() {
            let column = super::coupled_bernstein::restrict(
                w.iter().map(|row| [row[j].clone(), q(1.)]).collect(),
                &independent,
            );
            for (i, h) in column.into_iter().enumerate() {
                w[i][j] = h[0].clone();
            }
        }
        let mut dependent = [q(1.), q(0.)];
        for row in w {
            let poles = row
                .into_iter()
                .zip(&self.ordinates)
                .map(|(w, c)| [c * &w, w])
                .collect();
            let clock = super::clock::Clock::certified(poles)
                .ok_or_else(|| crate::input("Conditional clock certificate exhausted"))?;
            dependent[0] = dependent[0]
                .clone()
                .min(clock.inverse(&r[1 - a][0])[0].clone());
            dependent[1] = dependent[1]
                .clone()
                .max(clock.inverse(&r[1 - a][1])[1].clone());
        }
        let mut uv = [independent, dependent];
        if a == 1 {
            uv.swap(0, 1);
        }
        Ok(uv)
    }
}
fn choose(n: usize, k: usize) -> Q {
    q((0..k).fold(1u64, |a, j| a * (n - j) as u64 / (j + 1) as u64) as f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn basis(p: usize, i: usize, t: &Q) -> Q {
        (0..i).fold(choose(p, i), |a, _| a * t) * (0..p - i).fold(q(1.), |a, _| a * (q(1.) - t))
    }
    #[test]
    fn every_degree_pair_retains_conditional_preimages_with_independent_tensor_sum() {
        for p in 1..=8 {
            for m in 1..=8 {
                let weights: Vec<Vec<Q>> = (0..=p)
                    .map(|i| {
                        (0..=m)
                            .map(|j| q((1 + (i + 2 * j) * (i + j + 3) % 7) as f64))
                            .collect()
                    })
                    .collect();
                let ordinates: Vec<Q> = (0..=m)
                    .map(|j| q((j * j) as f64) / q((m * m) as f64))
                    .collect();
                for axis in 0..2 {
                    let chart = Triangular {
                        axis,
                        clock: super::super::clock::Clock::original(vec![q(1.), q(1.)]),
                        ordinates: ordinates.clone(),
                        weights: weights.clone(),
                    };
                    for (u, v) in [
                        (0., 0.),
                        (1., 1.),
                        (0., 0.5),
                        (1., 0.5),
                        (0.5, 0.),
                        (0.5, 1.),
                        (0.5, 0.25),
                    ] {
                        let (u, v) = (q(u), q(v));
                        let mut n = q(0.);
                        let mut d = q(0.);
                        for i in 0..=p {
                            for j in 0..=m {
                                let h = &weights[i][j] * basis(p, i, &u) * basis(m, j, &v);
                                n += &h * &ordinates[j];
                                d += h;
                            }
                        }
                        let b = n / d;
                        let mut input = [[u.clone(), u.clone()], [b.clone(), b]];
                        if axis == 1 {
                            input.swap(0, 1);
                        }
                        let uv = chart.enclosure(&input).unwrap();
                        assert!(uv[axis][0] <= u && u <= uv[axis][1], "{p},{m}");
                        assert!(
                            uv[1 - axis][0] <= v && v <= uv[1 - axis][1],
                            "{p},{m}: {uv:?}"
                        );
                    }
                    // A non-singleton independent interval must enclose all
                    // conditional fibers, not only its midpoint or endpoints.
                    let mut input = [[q(0.25), q(0.75)], [q(0.25), q(0.75)]];
                    if axis == 1 {
                        input.swap(0, 1);
                    }
                    let uv = chart.enclosure(&input).unwrap();
                    for (u, v) in [(q(0.25), q(0.5)), (q(0.5), q(0.5)), (q(0.75), q(0.5))] {
                        let mut n = q(0.);
                        let mut d = q(0.);
                        for i in 0..=p {
                            for j in 0..=m {
                                let h = &weights[i][j] * basis(p, i, &u) * basis(m, j, &v);
                                n += &h * &ordinates[j];
                                d += h;
                            }
                        }
                        let b = n / d;
                        if q(0.25) <= b && b <= q(0.75) {
                            assert!(uv[1 - axis][0] <= v && v <= uv[1 - axis][1]);
                        }
                    }
                }
            }
        }
    }
}
