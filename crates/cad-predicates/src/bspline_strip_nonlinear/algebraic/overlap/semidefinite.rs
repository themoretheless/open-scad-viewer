//! Sufficient exact weighted-square certificates, not a numerical SOS fit.
//! Each subtraction is checked in Q[s,t]; failure grants no sign evidence.
use super::*;
fn subtract(
    p: &mut B,
    key: (usize, usize),
    q: Q,
    ctx: &mut PredicateContext<'_>,
) -> Result<(), Reason> {
    let q = p.remove(&key).unwrap_or_else(Q::zero).sub(&q, ctx)?;
    if q.n.sign() != Sign::Zero {
        p.insert(key, q);
    }
    Ok(())
}
/// Either p or -p must be an exact sum of positive rational multiples of
/// nonzero polynomial squares. Interior zeros and intersecting zero sets are
/// permitted; the caller must also prove strict coordinate monotonicity.
pub(super) fn certificate(p: &B, ctx: &mut PredicateContext<'_>) -> Result<bool, Reason> {
    let Some((_, leading)) = p.last_key_value() else {
        return Ok(false);
    };
    let degree = p.keys().map(|&(i, j)| i + j).max().unwrap();
    let mut p = p.clone();
    if leading.n.sign() == Sign::Negative {
        for q in p.values_mut() {
            *q = Q::zero().sub(q, ctx)?;
        }
    }
    while let Some((&(i, j), weight)) = p.last_key_value() {
        ctx.charge(1)?;
        if weight.n.sign() != Sign::Positive || i % 2 != 0 || j % 2 != 0 {
            return Ok(false);
        }
        let weight = weight.clone();
        let lead = (i / 2, j / 2);
        let mut root = B::from([(lead, one())]);
        p.remove(&(i, j));
        while let Some((&(a, b), c)) = p.last_key_value() {
            ctx.charge(1)?;
            if a < lead.0 || b < lead.1 {
                break;
            }
            let term = (a - lead.0, b - lead.1);
            if term >= lead {
                break;
            }
            if term.0 + term.1 > degree / 2 || root.contains_key(&term) {
                return Ok(false);
            }
            let c = c.div_positive(&weight.mul(&integer(2), ctx)?, ctx)?;
            let twice = weight.mul(&integer(2), ctx)?.mul(&c, ctx)?;
            for (&(x, y), q) in &root {
                subtract(&mut p, (x + term.0, y + term.1), q.mul(&twice, ctx)?, ctx)?;
            }
            subtract(
                &mut p,
                (2 * term.0, 2 * term.1),
                weight.mul(&c.mul(&c, ctx)?, ctx)?,
                ctx,
            )?;
            root.insert(term, c);
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AuthoredScalar, Limits, SourceArena, ToleranceContext};
    #[test]
    fn crossing_zero_sets_need_exact_square_weights_not_a_nearly_psd_fit() {
        let arena = SourceArena::authored("sos", 1, vec![AuthoredScalar::Binary64Bits(0)]).unwrap();
        let tol = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(&arena, &tol, Limits::default(), None);
        let p = B::from([((2, 0), one()), ((1, 1), integer(2)), ((0, 2), one())]);
        assert!(certificate(&p, &mut ctx).unwrap());
        let mut indefinite = p.clone();
        indefinite.insert((0, 2), Q::scalar(Expansion::scalar(1f64.next_down())));
        assert!(!certificate(&indefinite, &mut ctx).unwrap());
        let mut opposite = p.clone();
        for q in opposite.values_mut() {
            *q = Q::zero().sub(q, &mut ctx).unwrap();
        }
        assert!(certificate(&opposite, &mut ctx).unwrap());
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
}
