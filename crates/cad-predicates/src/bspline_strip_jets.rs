//! Original B-spline homogeneous jets, including nonclamped/periodic storage.
//! Neither knot insertion nor a rounded derivative supplies any premise.
use crate::bspline_internal_jets::{Rational as Q, series};
use crate::{
    Algebra, AuthoredScalar, BezierIdentity, BezierIdentityDecision, Expansion, InputError,
    LeafRef, ParameterDecision, ParameterIdentity, PredicateContext, Reason, Sign, exact_inputs,
};

/// One-sided jet on a specified original, nonempty knot span.
#[derive(Clone, Copy)]
pub struct BsplineStripLocation {
    pub span: usize,
    pub parameter: LeafRef,
    pub inward_positive: bool,
}

/// Affine equivalence of the entire represented basis, including exterior knots.
pub fn affine_bspline_knot_basis_identity(
    ctx: &mut PredicateContext<'_>,
    knots: [&[LeafRef]; 2],
    degree: usize,
) -> Result<ParameterDecision, InputError> {
    if degree == 0
        || degree > 32
        || knots[0].len() != knots[1].len()
        || knots[0].len() < 2 * (degree + 1)
        || knots[0].len() > 66
    {
        return Err(InputError::InvalidInput(
            "Invalid affine B-spline knot bases",
        ));
    }
    let leaves = knots
        .into_iter()
        .flatten()
        .map(|r| ctx.resolve(*r).cloned())
        .collect::<Result<Vec<_>, _>>()?;
    let result = (|| {
        ctx.charge(leaves.len() as u64)?;
        let x = exact_inputs(&leaves, ctx)?;
        let len = knots[0].len();
        let n = len - degree - 1;
        let a = &x[..len];
        let b = &x[len..];
        let aw = a[n].sub(&a[degree], ctx)?;
        let bw = b[n].sub(&b[degree], ctx)?;
        if aw.sign() != Sign::Positive || bw.sign() != Sign::Positive {
            return Ok(ParameterIdentity::Indeterminate(Reason::MissingProof));
        }
        for i in 0..len {
            if a[i]
                .sub(&a[degree], ctx)?
                .mul(&bw, ctx)?
                .sub(&b[i].sub(&b[degree], ctx)?.mul(&aw, ctx)?, ctx)?
                .sign()
                != Sign::Zero
            {
                return Ok(ParameterIdentity::Different);
            }
        }
        ctx.charge(0)?;
        Ok(ParameterIdentity::Equal)
    })();
    Ok(ParameterDecision {
        outcome: result.unwrap_or_else(ParameterIdentity::Indeterminate),
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}

/// Sufficient geometric G1/G2 in an independently established common along basis.
/// Transverse coordinates are normalized to each original active domain.
pub fn rational_bspline_projective_strip_jet_identity(
    ctx: &mut PredicateContext<'_>,
    strips: [&[Vec<[LeafRef; 4]>]; 2],
    knots: [&[LeafRef]; 2],
    degrees: [usize; 2],
    locations: [BsplineStripLocation; 2],
    scale: LeafRef,
    order: usize,
    propose_exact_scale: bool,
) -> Result<BezierIdentityDecision, InputError> {
    let [a, b] = strips;
    if !matches!(order, 1 | 2) || !(2..=33).contains(&a.len()) || a.len() != b.len() {
        return Err(InputError::InvalidInput(
            "Invalid B-spline projective strip",
        ));
    }
    for side in 0..2 {
        let n = strips[side][0].len();
        let p = degrees[side];
        let l = locations[side];
        if !(1..=32).contains(&p)
            || !(p + 1..=33).contains(&n)
            || strips[side].iter().any(|r| r.len() != n)
            || knots[side].len() != n + p + 1
            || l.span < p
            || l.span >= n
        {
            return Err(InputError::InvalidInput(
                "Invalid B-spline strip basis or span",
            ));
        }
    }
    let mut leaves = Vec::new();
    let mut starts = [0; 2];
    let mut ks = [0; 2];
    let mut ts = [0; 2];
    for side in 0..2 {
        starts[side] = leaves.len();
        leaves.extend(strips[side].iter().flatten().flatten().copied());
        ks[side] = leaves.len();
        leaves.extend_from_slice(knots[side]);
        ts[side] = leaves.len();
        leaves.push(locations[side].parameter);
    }
    let si = leaves.len();
    leaves.push(scale);
    let mut values = leaves
        .iter()
        .map(|r| ctx.resolve(*r).cloned())
        .collect::<Result<Vec<_>, _>>()?;
    values.push(AuthoredScalar::Binary64Bits(1f64.to_bits()));
    let result = (|| -> Result<BezierIdentity, Reason> {
        ctx.charge(values.len() as u64)?;
        let x = exact_inputs(&values, ctx)?;
        let unit = x.last().unwrap();
        let homogeneous_unit = Q::scalar(unit.mul(unit, ctx)?);
        let mut scale = Q::scalar(x[si].clone()).div_positive(&Q::scalar(unit.clone()), ctx)?;
        if scale.n.sign() != Sign::Positive {
            return Err(Reason::MissingProof);
        }
        let mut jets = Vec::new();
        for side in 0..2 {
            let n = strips[side][0].len();
            let p = degrees[side];
            let l = locations[side];
            let u = &x[ks[side]..ks[side] + knots[side].len()];
            let t = &x[ts[side]];
            let width = u[n].sub(&u[p], ctx)?;
            if width.sign() != Sign::Positive
                || u[l.span + 1].sub(&u[l.span], ctx)?.sign() != Sign::Positive
                || t.sub(&u[l.span], ctx)?.sign() == Sign::Negative
                || u[l.span + 1].sub(t, ctx)?.sign() == Sign::Negative
            {
                return Err(Reason::MissingProof);
            }
            for pair in u.windows(2) {
                if pair[1].sub(&pair[0], ctx)?.sign() == Sign::Negative {
                    return Err(Reason::MissingProof);
                }
            }
            let signed = if l.inward_positive {
                width
            } else {
                Expansion::scalar(0.).sub(&width, ctx)?
            };
            let width = Q::scalar(signed);
            let mut side_jets = Vec::new();
            for row in 0..a.len() {
                let at = starts[side] + row * n * 4;
                let controls = &x[at..at + n * 4];
                if controls
                    .chunks_exact(4)
                    .any(|c| c[3].sign() != Sign::Positive)
                {
                    return Err(Reason::MissingProof);
                }
                let mut s = series(controls, u, p, l.span, t, order, ctx)?;
                let mut factor = Q::scalar(Expansion::scalar(1.));
                for q in 0..=order {
                    if q > 0 {
                        factor = factor
                            .mul(&width, ctx)?
                            .mul(&Q::scalar(Expansion::scalar(q as f64)), ctx)?;
                    }
                    for k in 0..4 {
                        s[q][k] = s[q][k].mul(&factor, ctx)?;
                        // Euclidean XYZ*W and W*unit must have the same degree
                        // in the common scale returned by exact_inputs.
                        if k == 3 {
                            s[q][k] = s[q][k].mul(&Q::scalar(unit.clone()), ctx)?;
                        }
                        // Remove the arbitrary source-unit scale exactly before
                        // projective products, without rounded derivatives.
                        s[q][k] = s[q][k].div_positive(&homogeneous_unit, ctx)?;
                    }
                }
                if s[0][3].n.sign() != Sign::Positive {
                    return Err(Reason::MissingProof);
                }
                side_jets.push(s);
            }
            jets.push(side_jets);
        }
        let gauges = [jets[1][0][0][3].clone(), jets[0][0][0][3].clone()];
        for side in 0..2 {
            for row in &mut jets[side] {
                for q in row {
                    for c in q {
                        *c = c.mul(&gauges[side], ctx)?;
                    }
                }
            }
        }
        let [a, b] = [&jets[0], &jets[1]];
        if propose_exact_scale {
            // Original one-sided Cartesian velocity numerators supply only a
            // positive rational proposal. Every coefficient is still checked
            // below; neither one point nor a derivative ratio issues credit.
            let mut ratio = None;
            for k in 0..3 {
                let va = a[0][1][k]
                    .mul(&a[0][0][3], ctx)?
                    .sub(&a[0][0][k].mul(&a[0][1][3], ctx)?, ctx)?;
                if va.n.sign() == Sign::Zero {
                    continue;
                }
                let vb = b[0][1][k]
                    .mul(&b[0][0][3], ctx)?
                    .sub(&b[0][0][k].mul(&b[0][1][3], ctx)?, ctx)?;
                let sign = Q::scalar(Expansion::scalar(if va.n.sign() == Sign::Positive {
                    1.
                } else {
                    -1.
                }));
                let numerator = Q::zero()
                    .sub(&vb, ctx)?
                    .mul(&sign, ctx)?
                    .mul(&a[0][0][3].mul(&a[0][0][3], ctx)?, ctx)?;
                let denominator = va
                    .mul(&sign, ctx)?
                    .mul(&b[0][0][3].mul(&b[0][0][3], ctx)?, ctx)?;
                let candidate = numerator.div_positive(&denominator, ctx)?;
                if candidate.n.sign() != Sign::Positive {
                    return Err(Reason::MissingProof);
                }
                ratio = Some(candidate);
                break;
            }
            scale = ratio.ok_or(Reason::MissingProof)?;
        }
        let scale2 = scale.mul(&scale, ctx)?;
        let mut r1 = Vec::new();
        let mut r2 = Vec::new();
        for row in 0..a.len() {
            let mut first = std::array::from_fn::<_, 4, _>(|_| Q::zero());
            let mut second = std::array::from_fn::<_, 4, _>(|_| Q::zero());
            for k in 0..4 {
                if !a[row][0][k].equal(&b[row][0][k], ctx)? {
                    return Ok(BezierIdentity::Different);
                }
                first[k] = b[row][1][k].add(&a[row][1][k].mul(&scale, ctx)?, ctx)?;
                if order == 2 {
                    second[k] = b[row][2][k].sub(&a[row][2][k].mul(&scale2, ctx)?, ctx)?;
                }
            }
            r1.push(first);
            r2.push(second);
        }
        for row in 0..a.len() {
            for k in 0..4 {
                if !r1[row][k]
                    .mul(&a[0][0][3], ctx)?
                    .equal(&a[row][0][k].mul(&r1[0][3], ctx)?, ctx)?
                {
                    return Ok(BezierIdentity::Different);
                }
            }
        }
        if order == 1 {
            return Ok(BezierIdentity::Equal);
        }
        let mut solution = None;
        'find: for row in 0..a.len() {
            for k in 0..3 {
                let h = &a[row][0];
                let j = &a[row][1];
                let r = &r2[row];
                let d = h[3].mul(&j[k], ctx)?.sub(&h[k].mul(&j[3], ctx)?, ctx)?;
                if d.n.sign() != Sign::Zero {
                    let mu = r[3].mul(&j[k], ctx)?.sub(&r[k].mul(&j[3], ctx)?, ctx)?;
                    let nu = h[3].mul(&r[k], ctx)?.sub(&h[k].mul(&r[3], ctx)?, ctx)?;
                    solution = Some((d, mu, nu));
                    break 'find;
                }
            }
        }
        let Some((d, mu, nu)) = solution else {
            return Err(Reason::MissingProof);
        };
        for row in 0..a.len() {
            for k in 0..4 {
                let v = r2[row][k]
                    .mul(&d, ctx)?
                    .sub(&a[row][0][k].mul(&mu, ctx)?, ctx)?
                    .sub(&a[row][1][k].mul(&nu, ctx)?, ctx)?;
                if v.n.sign() != Sign::Zero {
                    return Ok(BezierIdentity::Different);
                }
            }
        }
        ctx.charge(0)?;
        Ok(BezierIdentity::Equal)
    })();
    Ok(BezierIdentityDecision {
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
    fn original_bezier_subcase_matches_existing_projective_predicate_under_rational_scale() {
        for (change, expected) in [(false, true), (true, false)] {
            let mut values = Vec::new();
            for side in 0..2 {
                for row in 0..2 {
                    for j in 0..3 {
                        let y = if side == 0 {
                            -6. + 3. * j as f64
                        } else {
                            7. * j as f64
                        };
                        values.extend([
                            row as f64,
                            y,
                            if change && side == 1 && j == 2 {
                                0.125
                            } else {
                                0.
                            },
                            if row == 0 { 1. } else { 2. },
                        ]);
                    }
                }
            }
            let ki = values.len();
            values.extend([0.1, 0.1, 0.1, 0.9, 0.9, 0.9]);
            let si = values.len();
            values.push(1.);
            let mut authored = values
                .iter()
                .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
                .collect::<Vec<_>>();
            authored[si] = AuthoredScalar::RationalConstant {
                numerator: 7,
                denominator: 3,
            };
            let source = SourceArena::authored("bspline-bezier-parity", 1, authored).unwrap();
            let refs = |side: usize, reverse: bool| {
                (0..2)
                    .map(|i| {
                        (0..3)
                            .map(|j| {
                                std::array::from_fn(|k| {
                                    source
                                        .leaf(
                                            side * 24
                                                + 4 * (i * 3 + if reverse { 2 - j } else { j })
                                                + k,
                                        )
                                        .unwrap()
                                })
                            })
                            .collect()
                    })
                    .collect::<Vec<_>>()
            };
            let a = refs(0, false);
            let b = refs(1, false);
            let inward = refs(0, true);
            let knots = (0..6)
                .map(|i| source.leaf(ki + i).unwrap())
                .collect::<Vec<_>>();
            let tolerance = ToleranceContext::default_valid();
            let mut ctx = PredicateContext::new(&source, &tolerance, Limits::default(), None);
            let old = crate::rational_projective_strip_jet_identity(
                &mut ctx,
                &inward,
                &b,
                source.leaf(si).unwrap(),
                2,
            )
            .unwrap();
            let run = |budget| {
                let mut ctx = PredicateContext::new(
                    &source,
                    &tolerance,
                    Limits {
                        max_work: budget,
                        ..Limits::default()
                    },
                    None,
                );
                rational_bspline_projective_strip_jet_identity(
                    &mut ctx,
                    [&a, &b],
                    [&knots, &knots],
                    [2, 2],
                    [
                        BsplineStripLocation {
                            span: 2,
                            parameter: knots[3],
                            inward_positive: false,
                        },
                        BsplineStripLocation {
                            span: 2,
                            parameter: knots[2],
                            inward_positive: true,
                        },
                    ],
                    source.leaf(si).unwrap(),
                    2,
                    false,
                )
                .unwrap()
            };
            let new = run(1000000);
            assert_eq!(new.outcome, old.outcome);
            assert_eq!(new.outcome == BezierIdentity::Equal, expected);
            if expected {
                assert_ne!(run(new.work_used - 1).outcome, BezierIdentity::Equal);
            }
            assert_ne!(run(0).outcome, BezierIdentity::Equal);
        }
    }
}
