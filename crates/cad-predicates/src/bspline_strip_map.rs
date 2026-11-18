//! Authored-map proof, independent of source graph/inverse chart families.
use crate::bspline_internal_jets::{Rational as Q, Series, StripJets};
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
// A fully repeated original knot cell is already a Bernstein basis.
// Exact forward differences avoid generic polynomial de Boor expansion. Every
// endpoint knot is checked before using this formula; other cells fall back.
pub(crate) fn span(
    jets: &[Series],
    u: &[Q],
    degree: usize,
    i: usize,
    ctx: &mut PredicateContext<'_>,
) -> Result<Vec<p::H>, Reason> {
    for j in 0..degree {
        if !u[i - j].equal(&u[i], ctx)? || !u[i + 1 + j].equal(&u[i + 1], ctx)? {
            return p::span(jets, u, degree, i, ctx);
        }
    }
    let width = u[i + 1].sub(&u[i], ctx)?;
    let linear = vec![
        Q::zero().sub(&u[i], ctx)?.div_positive(&width, ctx)?,
        one().div_positive(&width, ctx)?,
    ];
    let mut result = Vec::new();
    for q in 0..jets[0].len() {
        let mut h = std::array::from_fn(|_| vec![Q::zero()]);
        for k in 0..4 {
            let mut differences = (i - degree..=i)
                .map(|j| jets[j][q][k].clone())
                .collect::<Vec<_>>();
            let mut coefficients = Vec::new();
            let mut binomial = 1u64;
            for r in 0..=degree {
                coefficients
                    .push(differences[0].mul(&Q::scalar(Expansion::scalar(binomial as f64)), ctx)?);
                differences = differences
                    .windows(2)
                    .map(|v| v[1].sub(&v[0], ctx))
                    .collect::<Result<Vec<_>, _>>()?;
                if r < degree {
                    binomial = binomial * (degree - r) as u64 / (r + 1) as u64;
                }
            }
            let mut polynomial = vec![Q::zero()];
            for c in coefficients.into_iter().rev() {
                polynomial = p::add(&p::mul(&polynomial, &linear, ctx)?, &vec![c], ctx)?;
            }
            h[k] = polynomial;
        }
        result.push(h);
    }
    Ok(result)
}
pub(crate) fn compare(
    ctx: &mut PredicateContext<'_>,
    strips: StripJets<'_>,
    knots: [&[LeafRef]; 2],
    map: BsplineParameterMap<'_>,
    constant_position: bool,
) -> Result<BezierIdentity, Reason> {
    compare_impl(ctx, strips, knots, map, constant_position, false)
}
fn compare_impl(
    ctx: &mut PredicateContext<'_>,
    strips: StripJets<'_>,
    knots: [&[LeafRef]; 2],
    map: BsplineParameterMap<'_>,
    constant_position: bool,
    restriction: bool,
) -> Result<BezierIdentity, Reason> {
    let StripJets {
        jets,
        degrees,
        scale,
        order,
    } = strips;
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
        if !constant_position && u[0][i + 1].sub(&u[0][i], ctx)?.n.sign() == Sign::Positive {
            a_spans.push((i, span(&jets[0], &u[0], degrees[0], i, ctx)?));
        }
    }
    let mut b_cache: Option<(usize, Vec<p::H>)> = None;
    let mut m_cache: Option<(usize, Vec<p::H>)> = None;
    let mut previous = Q::zero();
    let mut started = false;
    let mut direction = None;
    for cell in breaks.windows(2) {
        let interval = [&cell[0], &cell[1]];
        let bi = original_span(&u[1], degrees[1], jets[1].len(), interval, ctx)?;
        let mi = original_span(&mk, map.degree, map.controls.len(), interval, ctx)?;
        if !constant_position && b_cache.as_ref().is_none_or(|(i, _)| *i != bi) {
            b_cache = Some((bi, span(&jets[1], &u[1], degrees[1], bi, ctx)?));
        }
        if m_cache.as_ref().is_none_or(|(i, _)| *i != mi) {
            m_cache = Some((mi, span(&map_jets, &mk, map.degree, mi, ctx)?));
        }

        let m = &m_cache.as_ref().unwrap().1;
        let (n, d) = (&m[0][0], &m[0][3]);
        if !p::positive(d, interval[0], interval[1], ctx)? {
            return Err(Reason::MissingProof);
        }
        let slope = p::sub(
            &p::mul(&p::derivative(n, ctx)?, d, ctx)?,
            &p::mul(n, &p::derivative(d, ctx)?, ctx)?,
            ctx,
        )?;
        let forward = p::positive(&slope, interval[0], interval[1], ctx)?;
        if !forward
            && (!restriction || !p::positive(&p::neg(&slope, ctx)?, interval[0], interval[1], ctx)?)
        {
            return Err(Reason::MissingProof);
        }
        if restriction && direction.is_some_and(|old| old != forward) {
            return Err(Reason::MissingProof);
        }
        direction = Some(forward);
        let start =
            p::value(n, interval[0], ctx)?.div_positive(&p::value(d, interval[0], ctx)?, ctx)?;
        let end =
            p::value(n, interval[1], ctx)?.div_positive(&p::value(d, interval[1], ctx)?, ctx)?;
        if restriction {
            if start.n.sign() == Sign::Negative
                || end.n.sign() == Sign::Negative
                || one().sub(&start, ctx)?.n.sign() == Sign::Negative
                || one().sub(&end, ctx)?.n.sign() == Sign::Negative
                || (started && !start.equal(&previous, ctx)?)
            {
                return Err(Reason::MissingProof);
            }
        } else if !start.equal(&previous, ctx)? || one().sub(&end, ctx)?.n.sign() == Sign::Negative
        {
            return Err(Reason::MissingProof);
        }
        started = true;
        let (lower, upper) = if forward {
            (&start, &end)
        } else {
            (&end, &start)
        };
        let mut covered = constant_position;
        for (ai, a) in &a_spans {
            if upper.sub(&u[0][*ai], ctx)?.n.sign() != Sign::Positive
                || u[0][ai + 1].sub(lower, ctx)?.n.sign() != Sign::Positive
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
            if !p::relation(&composed, &b_cache.as_ref().unwrap().1, scale, order, ctx)? {
                return Ok(BezierIdentity::Different);
            }
        }
        if !covered {
            return Err(Reason::MissingProof);
        }
        previous = end;
    }
    if (!restriction && !previous.equal(&one(), ctx)?) || (restriction && !started) {
        return Err(Reason::MissingProof);
    }
    ctx.charge(0)?;
    Ok(BezierIdentity::Equal)
}

/// Exact vector-curve composition; does not certify spatial regularity or Solid.
/// Even constant-curve equality still proves the authored map's full cover.
pub fn rational_bspline_mapped_curve_identity(
    ctx: &mut PredicateContext<'_>,
    curves: [&[[LeafRef; 4]]; 2],
    knots: [&[LeafRef]; 2],
    degrees: [usize; 2],
    map: BsplineParameterMap<'_>,
) -> Result<crate::BezierIdentityDecision, crate::InputError> {
    mapped_curve_impl(ctx, curves, knots, degrees, map, false)
}
/// Exact C_B(t)=C_A(map(t)) on the COMPLETE B/map domain, with a strict
/// monotone map into a SUBINTERVAL of normalized A. This does not prove
/// full A coverage. Both increasing and decreasing restrictions are supported.
pub fn rational_bspline_mapped_curve_restriction_identity(
    ctx: &mut PredicateContext<'_>,
    curves: [&[[LeafRef; 4]]; 2],
    knots: [&[LeafRef]; 2],
    degrees: [usize; 2],
    map: BsplineParameterMap<'_>,
) -> Result<crate::BezierIdentityDecision, crate::InputError> {
    mapped_curve_impl(ctx, curves, knots, degrees, map, true)
}
fn mapped_curve_impl(
    ctx: &mut PredicateContext<'_>,
    curves: [&[[LeafRef; 4]]; 2],
    knots: [&[LeafRef]; 2],
    degrees: [usize; 2],
    map: BsplineParameterMap<'_>,
    restriction: bool,
) -> Result<crate::BezierIdentityDecision, crate::InputError> {
    for i in 0..2 {
        if !(1..=32).contains(&degrees[i])
            || !(degrees[i] + 1..=33).contains(&curves[i].len())
            || knots[i].len() != curves[i].len() + degrees[i] + 1
        {
            return Err(crate::InputError::InvalidInput(
                "Invalid mapped original vector curve",
            ));
        }
    }
    if !(1..=32).contains(&map.degree)
        || !(map.degree + 1..=33).contains(&map.controls.len())
        || map.knots.len() != map.controls.len() + map.degree + 1
    {
        return Err(crate::InputError::InvalidInput(
            "Invalid original clock map",
        ));
    }
    for r in curves
        .iter()
        .flat_map(|c| c.iter().flatten())
        .chain(knots.iter().flat_map(|k| k.iter()))
        .chain(map.controls.iter().flatten())
        .chain(map.knots)
    {
        ctx.resolve(*r)?;
    }
    let mut input = curves
        .iter()
        .flat_map(|c| c.iter().flatten())
        .map(|r| ctx.resolve(*r).cloned())
        .collect::<Result<Vec<_>, _>>()?;
    input.push(AuthoredScalar::Binary64Bits(1f64.to_bits()));
    let result = (|| -> Result<BezierIdentity, Reason> {
        ctx.charge(input.len() as u64)?;
        let x = exact_inputs(&input, ctx)?;
        let unit = x.last().unwrap();
        let unit2 = Q::scalar(unit.mul(unit, ctx)?);
        if x[..x.len() - 1]
            .chunks_exact(4)
            .any(|p| p[3].sign() != Sign::Positive)
        {
            return Err(Reason::MissingProof);
        }
        let mut constant = true;
        for row in x[..x.len() - 1].chunks_exact(4) {
            for k in 0..3 {
                if row[k].sub(&x[k], ctx)?.sign() != Sign::Zero {
                    constant = false;
                }
            }
        }
        let mut jets = Vec::new();
        let mut offset = 0;
        for c in curves {
            let mut rows = Vec::new();
            for _ in c {
                let mut h = std::array::from_fn(|_| Q::zero());
                for k in 0..4 {
                    h[k] = Q::scalar(
                        x[offset + 3].mul(if k == 3 { unit } else { &x[offset + k] }, ctx)?,
                    )
                    .div_positive(&unit2, ctx)?;
                }
                rows.push(vec![h]);
                offset += 4;
            }
            jets.push(rows);
        }
        compare_impl(
            ctx,
            StripJets {
                jets: &jets,
                degrees,
                scale: &one(),
                order: 0,
            },
            knots,
            map,
            constant,
            restriction,
        )
    })();
    Ok(crate::BezierIdentityDecision {
        outcome: result.unwrap_or_else(BezierIdentity::Indeterminate),
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}

/// Equal constant Cartesian positive-weight rational control nets. This proves
/// only constant images; no knot validity, covering, or parameter-map premise.
pub fn rational_control_nets_constant_identity(
    ctx: &mut PredicateContext<'_>,
    nets: [&[[LeafRef; 4]]; 2],
) -> Result<crate::BezierIdentityDecision, crate::InputError> {
    if nets.iter().any(|n| n.is_empty() || n.len() > 33) {
        return Err(crate::InputError::InvalidInput(
            "Invalid constant control nets",
        ));
    }
    let values = nets
        .iter()
        .flat_map(|n| n.iter().flatten())
        .map(|r| ctx.resolve(*r).cloned())
        .collect::<Result<Vec<_>, _>>()?;
    let result = (|| -> Result<BezierIdentity, Reason> {
        ctx.charge(values.len() as u64)?;
        let x = exact_inputs(&values, ctx)?;
        for row in x.chunks_exact(4) {
            if row[3].sign() != Sign::Positive {
                return Err(Reason::MissingProof);
            }
            for k in 0..3 {
                if row[k].sub(&x[k], ctx)?.sign() != Sign::Zero {
                    return Ok(BezierIdentity::Different);
                }
            }
        }
        Ok(BezierIdentity::Equal)
    })();
    Ok(crate::BezierIdentityDecision {
        outcome: result.unwrap_or_else(BezierIdentity::Indeterminate),
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Limits, SourceArena, ToleranceContext};
    #[test]
    fn original_bernstein_forward_differences_match_generic_de_boor() {
        let source = SourceArena::authored(
            "original-bernstein-fixture",
            1,
            vec![AuthoredScalar::Binary64Bits(0)],
        )
        .unwrap();
        let tolerance = ToleranceContext::default_valid();
        for degree in 1..=4 {
            let mut ctx = PredicateContext::new(&source, &tolerance, Limits::default(), None);
            let scalar = |v: f64| Q::scalar(Expansion::scalar(v));
            let mut knots = vec![scalar(-0.5); degree + 1];
            knots.extend(vec![scalar(1.5); degree + 1]);
            let jets = (0..=degree)
                .map(|i| vec![std::array::from_fn(|k| scalar((i * i + 3 * k + 1) as f64))])
                .collect::<Vec<_>>();
            let fast = span(&jets, &knots, degree, degree, &mut ctx).unwrap();
            let generic = p::span(&jets, &knots, degree, degree, &mut ctx).unwrap();
            for (a, b) in fast.iter().zip(&generic) {
                for k in 0..4 {
                    assert!(p::zero(&p::sub(&a[k], &b[k], &mut ctx).unwrap()));
                }
            }
        }
    }
    #[test]
    fn constant_nets_require_every_cartesian_coordinate_and_positive_weights() {
        let vals = [2., 3., 4., 1., 2., 3., 4., 0.5, 2., 3., 4., 2., 0., -1.];
        let source = SourceArena::authored(
            "constant-original-nets",
            1,
            vals.map(|v: f64| AuthoredScalar::Binary64Bits(v.to_bits()))
                .to_vec(),
        )
        .unwrap();
        let a = [[0, 1, 2, 3], [4, 5, 6, 7]].map(|r| r.map(|i| source.leaf(i).unwrap()));
        let b = [[8, 9, 10, 11]].map(|r| r.map(|i| source.leaf(i).unwrap()));
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(&source, &tolerance, Limits::default(), None);
        let proof = rational_control_nets_constant_identity(&mut ctx, [&a, &b]).unwrap();
        assert_eq!(proof.outcome, BezierIdentity::Equal);
        let mut limits = Limits::default();
        limits.max_work = proof.work_used - 1;
        let mut ctx = PredicateContext::new(&source, &tolerance, limits, None);
        assert_ne!(
            rational_control_nets_constant_identity(&mut ctx, [&a, &b])
                .unwrap()
                .outcome,
            BezierIdentity::Equal
        );
        let mut changed = b;
        changed[0][0] = source.leaf(9).unwrap();
        let mut ctx = PredicateContext::new(&source, &tolerance, Limits::default(), None);
        assert_eq!(
            rational_control_nets_constant_identity(&mut ctx, [&a, &changed])
                .unwrap()
                .outcome,
            BezierIdentity::Different
        );
        for weight in [12, 13] {
            let mut invalid = b;
            invalid[0][3] = source.leaf(weight).unwrap();
            let mut ctx = PredicateContext::new(&source, &tolerance, Limits::default(), None);
            assert_ne!(
                rational_control_nets_constant_identity(&mut ctx, [&a, &invalid])
                    .unwrap()
                    .outcome,
                BezierIdentity::Equal
            );
        }
    }
}

#[cfg(test)]
mod restriction_tests {
    use super::*;
    #[test]
    fn mapped_curve_restriction_does_not_upgrade_full_cover_contract() {
        fn run(b: [f64; 2], map: [f64; 2], restriction: bool) -> BezierIdentity {
            let mut values = Vec::new();
            for x in [0., 1., b[0], b[1]] {
                values.extend([x, 0., 0., 1.]);
            }
            values.extend([0., 0., 1., 1., map[0], 1., map[1], 1.]);
            let source = crate::SourceArena::authored(
                "partial-map-contract",
                1,
                values
                    .iter()
                    .map(|x: &f64| AuthoredScalar::Binary64Bits(x.to_bits()))
                    .collect(),
            )
            .unwrap();
            let first = (0..2)
                .map(|i| std::array::from_fn(|k| source.leaf(4 * i + k).unwrap()))
                .collect::<Vec<_>>();
            let second = (0..2)
                .map(|i| std::array::from_fn(|k| source.leaf(8 + 4 * i + k).unwrap()))
                .collect::<Vec<_>>();
            let knots = (16..20)
                .map(|i| source.leaf(i).unwrap())
                .collect::<Vec<_>>();
            let controls = (0..2)
                .map(|i| {
                    [
                        source.leaf(20 + 2 * i).unwrap(),
                        source.leaf(21 + 2 * i).unwrap(),
                    ]
                })
                .collect::<Vec<_>>();
            let tolerance = crate::ToleranceContext::default_valid();
            let mut ctx =
                PredicateContext::new(&source, &tolerance, crate::Limits::default(), None);
            let map = BsplineParameterMap {
                controls: &controls,
                knots: &knots,
                degree: 1,
            };
            let result = if restriction {
                rational_bspline_mapped_curve_restriction_identity(
                    &mut ctx,
                    [&first, &second],
                    [&knots, &knots],
                    [1, 1],
                    map,
                )
            } else {
                rational_bspline_mapped_curve_identity(
                    &mut ctx,
                    [&first, &second],
                    [&knots, &knots],
                    [1, 1],
                    map,
                )
            };
            result.unwrap().outcome
        }
        assert_eq!(run([0., 0.5], [0., 0.5], true), BezierIdentity::Equal);
        assert_ne!(run([0., 0.5], [0., 0.5], false), BezierIdentity::Equal);
        assert_eq!(run([0.5, 0.], [0.5, 0.], true), BezierIdentity::Equal);
        assert_ne!(run([-0.25, 0.5], [-0.25, 0.5], true), BezierIdentity::Equal);
        assert_eq!(run([0., 0.625], [0., 0.5], true), BezierIdentity::Different);
        assert_eq!(run([0., 1.], [0., 1.], false), BezierIdentity::Equal);
    }
}
