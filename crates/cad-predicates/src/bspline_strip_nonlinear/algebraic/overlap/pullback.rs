//! Exact triangular polynomial automorphisms extend the monomial sign cones.
//! T(s,t)=(s,t +/- s^2) is globally bijective with polynomial inverse. Thus
//! a sign certificate for p composed with T proves the same sign for p.
//! No inverse root, approximate coordinate, or altered source is admitted.
use super::*;

fn transform(
    p: &B,
    swap: bool,
    negative: bool,
    ctx: &mut PredicateContext<'_>,
) -> Result<B, Reason> {
    let sign = if negative {
        Q::zero().sub(&one(), ctx)?
    } else {
        one()
    };
    let mut out = B::new();
    for (&(i, j), coefficient) in p {
        ctx.charge(1)?;
        let (i, j) = if swap { (j, i) } else { (i, j) };
        let mut term = B::from([((i, 0), coefficient.clone())]);
        for _ in 0..j {
            let mut next = B::new();
            for ((a, b), q) in term {
                ctx.charge(1)?;
                let a2 = a.checked_add(2).ok_or(Reason::MissingProof)?;
                let b1 = b.checked_add(1).ok_or(Reason::MissingProof)?;
                next = combine(next, B::from([((a, b1), q.clone())]), &one(), ctx)?;
                next = combine(next, B::from([((a2, b), q)]), &sign, ctx)?;
            }
            term = next;
        }
        if swap {
            term = term.into_iter().map(|((a, b), q)| ((b, a), q)).collect();
        }
        out = combine(out, term, &one(), ctx)?;
    }
    Ok(out)
}

pub(super) fn certificate(p: &B, ctx: &mut PredicateContext<'_>) -> Result<bool, Reason> {
    if p.is_empty() {
        return Ok(false);
    }
    let ds = p.keys().map(|&(i, _)| i).max().unwrap_or(0);
    let dt = p.keys().map(|&(_, j)| j).max().unwrap_or(0);
    // Undo the likely degree-raising shear first; this is only proposal order.
    let first = dt > ds;
    for swap in [first, !first] {
        for negative in [false, true] {
            let q = transform(p, swap, negative, ctx)?;
            // The inverse identity is checked on the ORIGINAL polynomial.
            // A failed exact reconstruction grants no evidence.
            let restored = transform(&q, swap, !negative, ctx)?;
            if !combine(restored, p.clone(), &Q::zero().sub(&one(), ctx)?, ctx)?.is_empty() {
                return Ok(false);
            }
            if semidefinite::certificate(&q, ctx)? || circuits::certificate(&q, ctx)? {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AuthoredScalar, Limits, SourceArena, ToleranceContext};
    #[test]
    fn nonlinear_motzkin_pullbacks_outside_direct_cones_preserve_refusals() {
        let arena =
            SourceArena::authored("triangular-sign", 1, vec![AuthoredScalar::Binary64Bits(0)])
                .unwrap();
        let tol = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(&arena, &tol, Limits::default(), None);
        let m = B::from([
            ((4, 2), one()),
            ((2, 4), one()),
            ((0, 0), one()),
            ((2, 2), Q::scalar(Expansion::scalar(-3.))),
        ]);
        for swap in [false, true] {
            for negative in [false, true] {
                let p = transform(&m, swap, negative, &mut ctx).unwrap();
                assert!(!semidefinite::certificate(&p, &mut ctx).unwrap());
                assert!(!circuits::certificate(&p, &mut ctx).unwrap());
                let mut proof = PredicateContext::new(&arena, &tol, Limits::default(), None);
                assert!(certificate(&p, &mut proof).unwrap());
                let work = proof.work_used();
                let spans = vec![vec![(0, Vec::new())], vec![(0, Vec::new())]];
                let lo = Q::zero();
                let knots = vec![vec![lo.clone(), one()], vec![lo, one()]];
                let mut bounds = Bounds::default();
                let mut owner = PredicateContext::new(&arena, &tol, Limits::default(), None);
                let admitted = bounds.cofactor(0,0,0,0,0,0,&p,&spans,&knots,&mut owner);
                assert!(matches!(admitted, Ok(true)), "swap={swap} negative={negative} result={admitted:?} work={}", owner.work_used());
                let mut short = PredicateContext::new(
                    &arena,
                    &tol,
                    Limits {
                        max_work: work - 1,
                        ..Limits::default()
                    },
                    None,
                );
                assert!(certificate(&p, &mut short).is_err());
                let mut bad = p.clone();
                bad.insert((0, 0), Q::scalar(Expansion::scalar(1f64.next_down())));
                let mut refuse = PredicateContext::new(&arena, &tol, Limits::default(), None);
                assert!(!certificate(&bad, &mut refuse).unwrap());
            }
        }
        assert!(!certificate(&B::new(), &mut ctx).unwrap());
    }
}
