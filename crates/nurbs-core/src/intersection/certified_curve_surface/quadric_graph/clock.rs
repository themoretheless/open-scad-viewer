//! Positive weighted Bernstein clocks, with exact inverse enclosures.
use super::*;
use num_traits::Zero;
mod spline;
pub(super) enum Clock {
    Projective([Q; 2]),
    Bernstein(Vec<Q>),
    Ordered(Vec<[Q; 2]>),
    Spline(Vec<spline::Span>),
}
impl Clock {
    pub(super) fn original(weights: Vec<Q>) -> Self {
        let geometric =
            (2..weights.len()).all(|i| &weights[i] * &weights[0] == &weights[i - 1] * &weights[1]);
        if geometric {
            Self::Projective([weights[0].clone(), weights[1].clone()])
        } else {
            Self::Bernstein(weights)
        }
    }
    pub(super) fn bspline(
        p: usize,
        knots: &[f64],
        weights: Vec<Q>,
        ordinates: &[Q],
    ) -> Result<Self> {
        Ok(Self::Spline(spline::original(
            p, knots, weights, ordinates,
        )?))
    }
    /// Positive weights and ordered Euclidean ordinates y_i imply strict
    /// increase: pair terms (j-i)(y_j-y_i)w_iw_j B_iB_j are nonnegative,
    /// with a strictly positive endpoint pair throughout (0,1).
    pub(super) fn ordered(poles: Vec<[Q; 2]>) -> Option<Self> {
        if poles.len() < 2 || poles.iter().any(|h| h[1] <= q(0.)) || !poles[0][0].is_zero() {
            return None;
        }
        let last = poles.last()?;
        if last[0] != last[1]
            || !poles
                .windows(2)
                .all(|h| &h[0][0] * &h[1][1] <= &h[1][0] * &h[0][1])
        {
            return None;
        }
        Some(Self::Ordered(poles))
    }
    pub(super) fn identity(&self) -> bool {
        matches!(self,Self::Projective(w) if w[0]==w[1])
    }
    pub(super) fn projective(&self) -> bool {
        matches!(self, Self::Projective(_))
    }
    /// For f(u)=sum(i/p)*w_i*B_i / W, exactly
    /// p*u*(1-u)*W²*f' = sum_{i<j}(j-i)²*w_i*w_j*B_i*B_j > 0
    /// in (0,1). Endpoint derivatives are w1/w0 and w_(p-1)/w_p.
    /// Thus every positive original weight vector gives a bijection [0,1].
    /// Geometric weights have the exact Mobius shortcut. Otherwise binary64
    /// midpoints only propose cuts; exact rational de Casteljau comparisons
    /// certify an enclosing inverse interval. No approximate root is assumed.
    pub(super) fn inverse(&self, a: &Q) -> [Q; 2] {
        if a.is_zero() {
            return [q(0.), q(0.)];
        }
        if *a == q(1.) {
            return [q(1.), q(1.)];
        }
        match self {
            Self::Projective([w0, w1]) => {
                let x = w0 * a / (w1 * (q(1.) - a) + w0 * a);
                [x.clone(), x]
            }
            Self::Bernstein(_) | Self::Spline(_) | Self::Ordered(_) => {
                let mut lo = 0.;
                let mut hi = 1.;
                for _ in 0..64 {
                    let mid = (lo + hi) * 0.5;
                    if mid <= lo || mid >= hi {
                        break;
                    }
                    let value = match self {
                        Self::Bernstein(w) => evaluate(w, &q(mid)),
                        Self::Ordered(h) => evaluate_poles(h, &q(mid)),
                        Self::Spline(s) => spline::evaluate(s, &q(mid)),
                        _ => unreachable!(),
                    };
                    if value == *a {
                        return [q(mid), q(mid)];
                    }
                    if value < *a {
                        lo = mid;
                    } else {
                        hi = mid;
                    }
                }
                [q(lo), q(hi)]
            }
        }
    }
}
fn evaluate(w: &[Q], t: &Q) -> Q {
    let p = w.len() - 1;
    let d: Vec<[Q; 2]> = w
        .iter()
        .enumerate()
        .map(|(i, w)| [q(i as f64) * w / q(p as f64), w.clone()])
        .collect();
    evaluate_poles(&d, t)
}
fn evaluate_poles(poles: &[[Q; 2]], t: &Q) -> Q {
    let p = poles.len() - 1;
    let a = t.clone();
    let b = q(1.) - &a;
    let mut d = poles.to_vec();
    for r in 1..=p {
        for i in 0..=p - r {
            d[i] = std::array::from_fn(|k| &b * &d[i][k] + &a * &d[i + 1][k]);
        }
    }
    &d[0][0] / &d[0][1]
}

#[cfg(test)]
mod tests {
    use super::*;
    fn independent(w: &[Q], t: &Q) -> Q {
        fn pow(x: &Q, n: usize) -> Q {
            (0..n).fold(q(1.), |a, _| a * x)
        }
        let p = w.len() - 1;
        let mut a = Q::zero();
        let mut b = Q::zero();
        for (i, w) in w.iter().enumerate() {
            let choose = (0..i).fold(1u64, |c, k| c * (p - k) as u64 / (k + 1) as u64);
            let h = q(choose as f64) * pow(t, i) * pow(&(q(1.) - t), p - i) * w;
            a += q(i as f64) * &h / q(p as f64);
            b += h;
        }
        a / b
    }
    #[test]
    fn all_supported_degrees_and_subnormal_fractions_have_exact_inverse_enclosures() {
        for p in 1..=8 {
            let weights: Vec<_> = (0..=p).map(|i| q((i * i + 2 * i + 2) as f64)).collect();
            let clock = Clock::original(weights.clone());
            for a in [q(0.), q(f64::from_bits(1)), q(0.25), q(0.5), q(0.75), q(1.)] {
                let b = clock.inverse(&a);
                assert!(b[0] >= q(0.) && b[1] <= q(1.) && b[0] <= b[1]);
                assert!(
                    independent(&weights, &b[0]) <= a && a <= independent(&weights, &b[1]),
                    "degree {p}, fraction {a}, inverse {b:?}"
                );
            }
        }
    }
}

#[cfg(test)]
#[test]
fn ordered_original_clock_inverse_matches_independent_bernstein_sum() {
    for p in 1..=8 {
        let poles: Vec<[Q; 2]> = (0..=p)
            .map(|i| {
                let w = q((i * i + 2) as f64);
                [q((i * i) as f64) * &w / q((p * p) as f64), w]
            })
            .collect();
        let c = Clock::ordered(poles.clone()).unwrap();
        let direct = |t: &Q| {
            let mut n = q(0.);
            let mut d = q(0.);
            for i in 0..=p {
                let bin = (0..i).fold(1u64, |c, j| c * (p - j) as u64 / (j + 1) as u64);
                let a = (0..i).fold(q(bin as f64), |a, _| a * t)
                    * (0..p - i).fold(q(1.), |a, _| a * (q(1.) - t));
                n += &a * &poles[i][0];
                d += a * &poles[i][1];
            }
            n / d
        };
        for a in [q(0.), q(f64::from_bits(1)), q(0.25), q(0.5), q(0.75), q(1.)] {
            let e = c.inverse(&a);
            assert!(direct(&e[0]) <= a && a <= direct(&e[1]), "degree {p}");
        }
    }
    assert!(Clock::ordered(vec![[q(0.), q(1.)], [q(2.), q(1.)], [q(1.), q(1.)]]).is_none());
    assert!(Clock::ordered(vec![[q(0.), q(1.)], [q(0.), q(0.)], [q(1.), q(1.)]]).is_none());
}
