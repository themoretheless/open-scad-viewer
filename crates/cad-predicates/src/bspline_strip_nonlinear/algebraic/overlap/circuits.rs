//! Exact nonnegative two/three-term arithmetic-geometric mean certificates.
//! No Gram matrix, square roots, numerical fitting, or sampled sign acceptance.
use super::*;
fn power(x: &Q, n: usize, ctx: &mut PredicateContext<'_>) -> Result<Q, Reason> {
    let mut out = one();
    for _ in 0..n {
        out = out.mul(x, ctx)?;
    }
    Ok(out)
}
fn allocation(
    c: &Q,
    weights: &[Q],
    degree: usize,
    ctx: &mut PredicateContext<'_>,
) -> Result<Option<Q>, Reason> {
    let arity = weights.len();
    let mut bound = power(&integer(arity), arity, ctx)?;
    for w in weights {
        bound = bound.mul(w, ctx)?;
    }
    // Exact rational allocations only. Every proposal satisfies the ORIGINAL
    // AM-GM coefficient inequality before consuming any polynomial term.
    for k in (1..=degree).rev() {
        ctx.charge(1)?;
        let proposed = if k == degree {
            c.clone()
        } else {
            c.mul(&integer(k), ctx)?
                .div_positive(&integer(degree), ctx)?
        };
        if bound.sub(&power(&proposed, arity, ctx)?, ctx)?.n.sign() != Sign::Negative {
            return Ok(Some(proposed));
        }
    }
    Ok(None)
}
fn orient(p: &B, ctx: &mut PredicateContext<'_>) -> Result<B, Reason> {
    let negative = p
        .last_key_value()
        .is_some_and(|(_, q)| q.n.sign() == Sign::Negative);
    p.iter()
        .map(|(&key, q)| {
            Ok((
                key,
                if negative {
                    Q::zero().sub(q, ctx)?
                } else {
                    q.clone()
                },
            ))
        })
        .collect()
}
// Necessary checks reject impossible GLOBAL certificates quickly. They NEVER
// grant sign evidence: only the exact circuit decomposition below can do so.
fn possible(p: &B, ctx: &mut PredicateContext<'_>) -> Result<bool, Reason> {
    for (x, y) in [
        (0, 0),
        (1, 0),
        (0, 1),
        (1, 1),
        (-1, 0),
        (0, -1),
        (-1, -1),
        (1, -1),
        (-1, 1),
    ] {
        let mut v = Q::zero();
        for (&(i, j), q) in p {
            if x == 0 && i > 0 || y == 0 && j > 0 {
                continue;
            }
            let negative = (x < 0 && i % 2 == 1) ^ (y < 0 && j % 2 == 1);
            v = if negative {
                v.sub(q, ctx)?
            } else {
                v.add(q, ctx)?
            };
        }
        if v.n.sign() == Sign::Negative {
            return Ok(false);
        }
    }
    let half = one().div_positive(&integer(2), ctx)?;
    let mut v = Q::zero();
    for (&(i, j), q) in p {
        v = v.add(&q.mul(&power(&half, i + j, ctx)?, ctx)?, ctx)?;
    }
    Ok(v.n.sign() != Sign::Negative)
}
fn complement(
    n: (usize, usize),
    factor: usize,
    terms: &[(usize, usize)],
) -> Option<(usize, usize)> {
    let (mut x, mut y) = (n.0.checked_mul(factor)?, n.1.checked_mul(factor)?);
    for &(i, j) in terms {
        x = x.checked_sub(i)?;
        y = y.checked_sub(j)?;
    }
    Some((x, y))
}
fn consume(
    p: &mut B,
    negative: (usize, usize),
    keys: &[(usize, usize)],
    amount: Q,
    ctx: &mut PredicateContext<'_>,
) -> Result<(), Reason> {
    for key in keys {
        p.remove(key);
    }
    let remaining = p
        .remove(&negative)
        .ok_or(Reason::MissingProof)?
        .add(&amount, ctx)?;
    if remaining.n.sign() != Sign::Zero {
        p.insert(negative, remaining);
    }
    Ok(())
}
pub(super) fn certificate(p: &B, ctx: &mut PredicateContext<'_>) -> Result<bool, Reason> {
    if !p.values().any(|q| q.n.sign() != Sign::Zero) {
        return Ok(false);
    }
    ctx.charge(p.len() as u64)?;
    let mut p = orient(p, ctx)?;
    if !possible(&p, ctx)? {
        return Ok(false);
    }
    let degree = p.keys().map(|&(i, j)| i + j).max().unwrap();
    if degree == 0 {
        return Ok(p.values().all(|q| q.n.sign() == Sign::Positive));
    }
    loop {
        ctx.charge(1)?;
        if p.iter()
            .any(|(&(i, j), q)| q.n.sign() == Sign::Positive && (i % 2 != 0 || j % 2 != 0))
        {
            return Ok(false);
        }
        let Some((&negative, c)) = p.iter().find(|(_, q)| q.n.sign() == Sign::Negative) else {
            return Ok(true);
        };
        let c = Q::zero().sub(c, ctx)?;
        let positive = p
            .iter()
            .filter(|(_, q)| q.n.sign() == Sign::Positive)
            .map(|(&key, q)| (key, q.clone()))
            .collect::<B>();
        let keys = positive.keys().copied().collect::<Vec<_>>();
        let mut found = None;
        for &a in &keys {
            ctx.charge(1)?;
            let Some(b) = complement(negative, 2, &[a]) else {
                continue;
            };
            if b <= a {
                continue;
            }
            if let Some(weight) = positive.get(&b) {
                if let Some(amount) =
                    allocation(&c, &[positive[&a].clone(), weight.clone()], degree, ctx)?
                {
                    found = Some((vec![a, b], amount));
                    break;
                }
            }
        }
        if found.is_none() {
            'triples: for (i, &a) in keys.iter().enumerate() {
                for &b in &keys[i + 1..] {
                    ctx.charge(1)?;
                    let Some(d) = complement(negative, 3, &[a, b]) else {
                        continue;
                    };
                    if d <= b {
                        continue;
                    }
                    if let Some(weight) = positive.get(&d) {
                        if let Some(amount) = allocation(
                            &c,
                            &[positive[&a].clone(), positive[&b].clone(), weight.clone()],
                            degree,
                            ctx,
                        )? {
                            found = Some((vec![a, b, d], amount));
                            break 'triples;
                        }
                    }
                }
            }
        }
        let Some((keys, amount)) = found else {
            return Ok(false);
        };
        consume(&mut p, negative, &keys, amount, ctx)?;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AuthoredScalar, Limits, SourceArena, ToleranceContext};
    #[test]
    fn motzkin_is_certified_without_squares_and_one_ulp_negative_refuses() {
        let arena =
            SourceArena::authored("circuits", 1, vec![AuthoredScalar::Binary64Bits(0)]).unwrap();
        let tol = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(&arena, &tol, Limits::default(), None);
        let p = B::from([
            ((4, 2), one()),
            ((2, 4), one()),
            ((0, 0), one()),
            ((2, 2), Q::scalar(Expansion::scalar(-3.))),
        ]);
        assert!(!semidefinite::certificate(&p, &mut ctx).unwrap());
        assert!(certificate(&p, &mut ctx).unwrap());
        let mut bad = p.clone();
        bad.insert((0, 0), Q::scalar(Expansion::scalar(1f64.next_down())));
        assert!(!certificate(&bad, &mut ctx).unwrap());
        let mut none = PredicateContext::new(
            &arena,
            &tol,
            Limits {
                max_work: 0,
                ..Limits::default()
            },
            None,
        );
        assert!(certificate(&p, &mut none).is_err());
    }
    #[test]
    fn rational_divided_difference_uses_two_exact_allocated_circuits() {
        let arena = SourceArena::authored(
            "rational-circuits",
            1,
            vec![AuthoredScalar::Binary64Bits(0)],
        )
        .unwrap();
        let tol = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(&arena, &tol, Limits::default(), None);
        let p = B::from([
            ((2, 2), one()),
            ((2, 0), one()),
            ((0, 2), one()),
            ((1, 1), Q::scalar(Expansion::scalar(-8.))),
            ((0, 0), integer(9)),
        ]);
        assert!(!semidefinite::certificate(&p, &mut ctx).unwrap());
        assert!(certificate(&p, &mut ctx).unwrap());
    }
}

#[cfg(test)]
mod continuation_tests {
    use super::*;
    use crate::{AuthoredScalar, Limits, SourceArena, ToleranceContext};
    #[test]
    fn rational_ramified_curve_has_a_whole_box_circuit_owner() {
        let arena =
            SourceArena::authored("circuit-map", 1, vec![AuthoredScalar::Binary64Bits(0)]).unwrap();
        let tol = ToleranceContext::default_valid();
        let mut build = PredicateContext::new(&arena, &tol, Limits::default(), None);
        let n = vec![Q::zero(), integer(9), Q::zero(), one()];
        let d = vec![one(), Q::zero(), one()];
        let y = vec![Q::zero(), Q::zero(), one(), Q::zero(), one()];
        let h = vec![
            [n, y, vec![Q::zero()], d.clone()],
            [vec![Q::zero()], vec![Q::zero()], d, vec![Q::zero()]],
            std::array::from_fn(|_| vec![Q::zero()]),
        ];
        let lo = Q::zero().sub(&integer(2), &mut build).unwrap();
        let hi = integer(2);
        let spans = vec![vec![(0, h.clone())], vec![(0, h)]];
        let knots = vec![vec![lo.clone(), hi.clone()], vec![lo, hi]];
        let cofactor = B::from([
            ((2, 2), one()),
            ((2, 0), one()),
            ((0, 2), one()),
            ((1, 1), Q::scalar(Expansion::scalar(-8.))),
            ((0, 0), integer(9)),
        ]);
        assert!(!semidefinite::certificate(&cofactor, &mut build).unwrap());
        let mut bounds = Bounds::default();
        assert!(
            bounds
                .cofactor(0, 0, 0, 0, 0, 0, &cofactor, &spans, &knots, &mut build)
                .unwrap()
        );
        let scale = Q::zero().sub(&one(), &mut build).unwrap();
        let mut ctx = PredicateContext::new(&arena, &tol, Limits::default(), None);
        assert!(
            matches!(
                super::super::compare(&mut ctx, &spans, &knots, &scale, 2),
                Ok(BezierIdentity::Equal)
            ),
            "work={}",
            ctx.work_used()
        );
        let work = ctx.work_used();
        println!("circuit-map-exact-work={work}");
        let mut short = PredicateContext::new(
            &arena,
            &tol,
            Limits {
                max_work: work - 1,
                ..Limits::default()
            },
            None,
        );
        assert!(super::super::compare(&mut short, &spans, &knots, &scale, 2).is_err());
    }
}
