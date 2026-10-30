//! Authored-map proof, independent of source graph/inverse chart families.
use crate::bspline_internal_jets::{Rational as Q, Series};
use crate::bspline_strip_nonlinear as p;
use crate::{
    Algebra, AuthoredScalar, BezierIdentity, BsplineParameterMap, Expansion, LeafRef,
    PredicateContext, Reason, Sign, exact_inputs,
};
fn one() -> Q {
    Q::scalar(Expansion::scalar(1.))
}
fn normalized(
    ctx: &mut PredicateContext<'_>,
    knots: &[LeafRef],
    n: usize,
    degree: usize,
) -> Result<Vec<Q>, Reason> {
    let leaves = knots
        .iter()
        .map(|r| ctx.resolve(*r).cloned())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| Reason::MissingProof)?;
    ctx.charge(leaves.len() as u64)?;
    let x = exact_inputs(&leaves, ctx)?;
    let width = Q::scalar(x[n].sub(&x[degree], ctx)?);
    let k = x
        .iter()
        .map(|v| Q::scalar(v.sub(&x[degree], ctx)?).div_positive(&width, ctx))
        .collect::<Result<Vec<_>, _>>()?;
    for pair in k.windows(2) {
        if pair[1].sub(&pair[0], ctx)?.n.sign() == Sign::Negative {
            return Err(Reason::MissingProof);
        }
    }
    Ok(k)
}
fn original_span(
    u: &[Q],
    p: usize,
    n: usize,
    interval: [&Q; 2],
    ctx: &mut PredicateContext<'_>,
) -> Result<usize, Reason> {
    for i in p..n {
        if u[i + 1].sub(&u[i], ctx)?.n.sign() == Sign::Positive
            && interval[0].sub(&u[i], ctx)?.n.sign() != Sign::Negative
            && u[i + 1].sub(interval[1], ctx)?.n.sign() != Sign::Negative
        {
            return Ok(i);
        }
    }
    Err(Reason::MissingProof)
}
pub(crate) fn compare(
    ctx: &mut PredicateContext<'_>,
    jets: &[Vec<Series>],
    knots: [&[LeafRef]; 2],
    degrees: [usize; 2],
    map: BsplineParameterMap<'_>,
    scale: &Q,
    order: usize,
) -> Result<BezierIdentity, Reason> {
    let u = [
        normalized(ctx, knots[0], jets[0].len(), degrees[0])?,
        normalized(ctx, knots[1], jets[1].len(), degrees[1])?,
    ];
    let mk = normalized(ctx, map.knots, map.controls.len(), map.degree)?;
    let mut leaves = map
        .controls
        .iter()
        .flatten()
        .map(|r| ctx.resolve(*r).cloned())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| Reason::MissingProof)?;
    leaves.push(AuthoredScalar::Binary64Bits(1f64.to_bits()));
    ctx.charge(leaves.len() as u64)?;
    let x = exact_inputs(&leaves, ctx)?;
    let unit = x.last().unwrap();
    let unit2 = Q::scalar(unit.mul(unit, ctx)?);
    let mut map_jets = Vec::new();
    for pair in x[..x.len() - 1].chunks_exact(2) {
        if pair[1].sign() != Sign::Positive {
            return Err(Reason::MissingProof);
        }
        map_jets.push(vec![[
            Q::scalar(pair[0].mul(&pair[1], ctx)?).div_positive(&unit2, ctx)?,
            Q::zero(),
            Q::zero(),
            Q::scalar(pair[1].clone()).div_positive(&Q::scalar(unit.clone()), ctx)?,
        ]]);
    }
    // Exact union of B/map knots: every original interval is included once.
    let mut breaks: Vec<Q> = Vec::new();
    for k in u[1][degrees[1]..=jets[1].len()]
        .iter()
        .chain(&mk[map.degree..=map.controls.len()])
    {
        let mut at = 0;
        while at < breaks.len() && breaks[at].sub(k, ctx)?.n.sign() == Sign::Negative {
            at += 1;
        }
        if at == breaks.len() || !breaks[at].equal(k, ctx)? {
            breaks.insert(at, k.clone());
        }
    }
    let mut a_spans = Vec::new();
    for i in degrees[0]..jets[0].len() {
        if u[0][i + 1].sub(&u[0][i], ctx)?.n.sign() == Sign::Positive {
            a_spans.push((i, p::span(&jets[0], &u[0], degrees[0], i, ctx)?));
        }
    }
    let mut previous = Q::zero();
    for cell in breaks.windows(2) {
        let interval = [&cell[0], &cell[1]];
        let bi = original_span(&u[1], degrees[1], jets[1].len(), interval, ctx)?;
        let mi = original_span(&mk, map.degree, map.controls.len(), interval, ctx)?;
        let b = p::span(&jets[1], &u[1], degrees[1], bi, ctx)?;
        let m = p::span(&map_jets, &mk, map.degree, mi, ctx)?;
        let (n, d) = (&m[0][0], &m[0][3]);
        if !p::positive(d, interval[0], interval[1], ctx)? {
            return Err(Reason::MissingProof);
        }
        let slope = p::sub(
            &p::mul(&p::derivative(n, ctx)?, d, ctx)?,
            &p::mul(n, &p::derivative(d, ctx)?, ctx)?,
            ctx,
        )?;
        if !p::positive(&slope, interval[0], interval[1], ctx)? {
            return Err(Reason::MissingProof);
        }
        let start =
            p::value(n, interval[0], ctx)?.div_positive(&p::value(d, interval[0], ctx)?, ctx)?;
        let end =
            p::value(n, interval[1], ctx)?.div_positive(&p::value(d, interval[1], ctx)?, ctx)?;
        if !start.equal(&previous, ctx)? || one().sub(&end, ctx)?.n.sign() == Sign::Negative {
            return Err(Reason::MissingProof);
        }
        let mut covered = false;
        for (ai, a) in &a_spans {
            if end.sub(&u[0][*ai], ctx)?.n.sign() != Sign::Positive
                || u[0][ai + 1].sub(&start, ctx)?.n.sign() != Sign::Positive
            {
                continue;
            }
            covered = true;
            let mut composed = Vec::new();
            for h in a {
                let mut row = std::array::from_fn(|_| vec![Q::zero()]);
                for k in 0..4 {
                    row[k] = p::compose(&h[k], n, d, degrees[0], ctx)?;
                }
                composed.push(row);
            }
            if !p::relation(&composed, &b, scale, order, ctx)? {
                return Ok(BezierIdentity::Different);
            }
        }
        if !covered {
            return Err(Reason::MissingProof);
        }
        previous = end;
    }
    if !previous.equal(&one(), ctx)? {
        return Err(Reason::MissingProof);
    }
    ctx.charge(0)?;
    Ok(BezierIdentity::Equal)
}
