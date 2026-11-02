//! Exact homogeneous blossom restriction and monotone rational span census.
use super::*;
pub(super) struct Span {
    domain: [Q; 2],
    poles: Vec<[Q; 2]>,
}
pub(super) fn original(
    p: usize,
    knots: &[f64],
    weights: Vec<Q>,
    ordinates: &[Q],
) -> Result<Vec<Span>> {
    let n = weights.len();
    let k: Vec<_> = knots.iter().map(|x| q(*x)).collect();
    let h: Vec<[Q; 2]> = weights
        .iter()
        .zip(ordinates)
        .map(|(w, x)| [w * x, w.clone()])
        .collect();
    let mut spans: Vec<Span> = Vec::new();
    for span in p..n {
        if k[span] == k[span + 1] {
            continue;
        }
        let mut poles = Vec::new();
        for b in 0..=p {
            let mut d = h[span - p..=span].to_vec();
            for r in 1..=p {
                let t = &k[span + usize::from(r > p - b)];
                for j in (r..=p).rev() {
                    let i = span - p + j;
                    let den = &k[i + p - r + 1] - &k[i];
                    check(
                        den > Q::zero(),
                        "Degenerate original rational graph clock basis",
                    )?;
                    let a = (t - &k[i]) / den;
                    let z = q(1.) - &a;
                    d[j] = std::array::from_fn(|axis| &z * &d[j - 1][axis] + &a * &d[j][axis]);
                }
            }
            poles.push(d[p].clone());
        }
        check(
            poles.iter().all(|h| h[1] > Q::zero()),
            "Unproved positive restricted graph clock",
        )?;
        let x: Vec<_> = poles.iter().map(|h| &h[0] / &h[1]).collect();
        // For ordered ordinates y_i and positive weights, W² f' has
        // positive pair terms (j-i)(y_j-y_i) w_i w_j B_i B_j / t(1-t).
        // Nondecreasing poles and distinct endpoints imply strict increase
        // in the open span, including zero endpoint derivative cases.
        check(
            x.windows(2).all(|w| w[0] <= w[1]) && x[0] < x[p],
            "Unproved monotone original rational graph clock span",
        )?;
        if let Some(previous) = spans.last() {
            let end = &previous.poles[p];
            check(
                &end[0] / &end[1] == x[0],
                "Discontinuous original rational graph clock",
            )?;
        } else {
            check(x[0].is_zero(), "Graph clock does not start at zero")?;
        }
        spans.push(Span {
            domain: [
                (&k[span] - &k[p]) / (&k[n] - &k[p]),
                (&k[span + 1] - &k[p]) / (&k[n] - &k[p]),
            ],
            poles,
        });
    }
    let last = spans
        .last()
        .ok_or_else(|| crate::input("Empty rational graph clock"))?;
    check(
        last.poles[p][0] == last.poles[p][1],
        "Graph clock does not end at one",
    )?;
    Ok(spans)
}
pub(super) fn evaluate(spans: &[Span], t: &Q) -> Q {
    let s = spans
        .iter()
        .find(|s| *t <= s.domain[1])
        .unwrap_or_else(|| spans.last().unwrap());
    let a = (t - &s.domain[0]) / (&s.domain[1] - &s.domain[0]);
    let b = q(1.) - &a;
    let mut d = s.poles.clone();
    let p = d.len() - 1;
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
    // Independent Cox basis sum, rather than blossom restriction/de Casteljau.
    fn basis(i: usize, p: usize, k: &[Q], t: &Q) -> Q {
        if p == 0 {
            return if k[i] <= *t && *t < k[i + 1] {
                q(1.)
            } else {
                q(0.)
            };
        }
        let mut v = q(0.);
        if k[i + p] > k[i] {
            v += (t - &k[i]) / (&k[i + p] - &k[i]) * basis(i, p - 1, k, t);
        }
        if k[i + p + 1] > k[i + 1] {
            v += (&k[i + p + 1] - t) / (&k[i + p + 1] - &k[i + 1]) * basis(i + 1, p - 1, k, t);
        }
        v
    }
    #[test]
    fn original_multispan_restrictions_and_inverse_match_independent_basis_all_degrees() {
        for p in 1..=8 {
            let mut knots = vec![0.; p + 1];
            knots.extend([0.25, 0.75]);
            knots.extend(vec![1.; p + 1]);
            let k: Vec<_> = knots.iter().map(|x| q(*x)).collect();
            let n = p + 3;
            let x: Vec<_> = (0..n)
                .map(|i| (i + 1..=i + p).map(|j| k[j].clone()).sum::<Q>() / q(p as f64))
                .collect();
            let w: Vec<_> = (0..n).map(|i| q((i + 2) as f64)).collect();
            let spans = original(p, &knots, w.clone(), &x).unwrap();
            let clock = Clock::bspline(p, &knots, w.clone(), &x).unwrap();
            for t in [0., 0.125, 0.25, 0.5, 0.75, 0.875] {
                let t = q(t);
                let d: Vec<_> = (0..n).map(|i| basis(i, p, &k, &t) * &w[i]).collect();
                let a = (0..n).map(|i| &d[i] * &x[i]).sum::<Q>() / d.iter().cloned().sum::<Q>();
                assert_eq!(evaluate(&spans, &t), a, "degree {p}");
                let enclosure = clock.inverse(&a);
                assert!(enclosure[0] <= t && t <= enclosure[1], "degree {p}");
            }
        }
    }
}
