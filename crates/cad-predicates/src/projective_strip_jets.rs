//! Exact sufficient G1/G2 with constant transverse reparameterization.
//! Common homogeneous boundary, projective first jets and second-jet span
//! identities are checked coefficientwise, without rounded divisions.
use crate::{
    Algebra, AuthoredScalar, BezierIdentity, BezierIdentityDecision, Expansion, InputError,
    LeafRef, PredicateContext, Reason, Sign, exact_inputs,
};

pub fn rational_projective_strip_jet_identity(
    ctx: &mut PredicateContext<'_>,
    a: &[Vec<[LeafRef; 4]>],
    b: &[Vec<[LeafRef; 4]>],
    scale: LeafRef,
    order: usize,
) -> Result<BezierIdentityDecision, InputError> {
    if !(1..=2).contains(&order)
        || !(2..=33).contains(&a.len())
        || a.len() != b.len()
        || a.iter().chain(b).any(|row| !(2..=33).contains(&row.len()))
        || a.iter().any(|row| row.len() != a[0].len())
        || b.iter().any(|row| row.len() != b[0].len())
    {
        return Err(InputError::InvalidInput(
            "Invalid projective boundary strip",
        ));
    }
    let leaves = a
        .iter()
        .chain(b)
        .flat_map(|row| row.iter().flatten())
        .copied()
        .chain(std::iter::once(scale));
    let mut values = leaves
        .map(|r| ctx.resolve(r).cloned())
        .collect::<Result<Vec<AuthoredScalar>, _>>()?;
    // exact_inputs uses a common positive denominator and power-of-two scale.
    // Keep its representation of one: XYZ*weight and weight*unit must have
    // equal algebraic degree, and normalScale must be divided by this unit
    // through cross multiplication, never rounded division.
    values.push(AuthoredScalar::Binary64Bits(1f64.to_bits()));
    let result = (|| -> Result<BezierIdentity, Reason> {
        ctx.charge(values.len() as u64)?;
        let x = exact_inputs(&values, ctx)?;
        let s = &x[x.len() - 2];
        let unit = x.last().unwrap();
        if s.sign() != Sign::Positive
            || x[..x.len() - 2]
                .chunks_exact(4)
                .any(|p| p[3].sign() != Sign::Positive)
        {
            return Ok(BezierIdentity::Indeterminate(Reason::MissingProof));
        }
        let pa = a[0].len() - 1;
        let pb = b[0].len() - 1;
        let start_b = a.len() * (pa + 1) * 4;
        // Quadratic conic strips with cross weights [1,w,1] admit an exact
        // coefficientwise sufficient relation. E is the common endpoint,
        // Q the adjacent pole and R the opposite pole. Q_a+Q_b=2E proves G1;
        // R_a-R_b=Q_a-Q_b additionally proves G2, with constant projective
        // coefficients lambda=4(w-1), nu=2(2w-1)/w, mu=-2(w-1)nu.
        // Positive w avoids division singularities; these equations are checked
        // without division. Regularity remains the native caller's obligation.
        if pa == 2 && pb == 2 && s.sub(unit, ctx)?.sign() == Sign::Zero {
            let w = &x[7];
            let two = Expansion::integer(2, ctx)?;
            let mut canonical = true;
            for row in 0..a.len() {
                for start in [row * 12, start_b + row * 12] {
                    canonical &= x[start + 3].sub(unit, ctx)?.sign() == Sign::Zero;
                    canonical &= x[start + 7].sub(w, ctx)?.sign() == Sign::Zero;
                    canonical &= x[start + 11].sub(unit, ctx)?.sign() == Sign::Zero;
                }
                if !canonical {break;}
            }
            if canonical {
                for row in 0..a.len() {
                    let aa = row * 12;
                    let bb = start_b + row * 12;
                    for axis in 0..3 {
                        canonical &= x[aa + axis].sub(&x[bb + axis], ctx)?.sign() == Sign::Zero;
                        canonical &= x[aa + 4 + axis].add(&x[bb + 4 + axis], ctx)?
                            .sub(&x[aa + axis].mul(&two, ctx)?, ctx)?.sign() == Sign::Zero;
                        if order == 2 {
                            canonical &= x[aa + 8 + axis].sub(&x[bb + 8 + axis], ctx)?
                                .sub(&x[aa + 4 + axis].sub(&x[bb + 4 + axis], ctx)?, ctx)?.sign() == Sign::Zero;
                        }
                    }
                    if !canonical {break;}
                }
                if canonical {return Ok(BezierIdentity::Equal);}
            }
        }
        let zero = || std::array::from_fn(|_| Expansion::scalar(0.));
        let build = |side: usize,
                     row: usize,
                     ctx: &mut PredicateContext<'_>|
         -> Result<[[Expansion; 4]; 3], Reason> {
            let (start, p, normalizer) = if side == 0 {
                (0, pa, &x[start_b + 3])
            } else {
                (start_b, pb, &x[3])
            };
            let homogeneous =
                |layer: usize, ctx: &mut PredicateContext<'_>| -> Result<[Expansion; 4], Reason> {
                    let at = start + (row * (p + 1) + layer) * 4;
                    let mut h = zero();
                    for axis in 0..4 {
                        h[axis] = if axis == 3 {
                            x[at + 3].mul(unit, ctx)?
                        } else {
                            x[at + axis].mul(&x[at + 3], ctx)?
                        };
                        h[axis] = h[axis].mul(normalizer, ctx)?;
                    }
                    Ok(h)
                };
            let h0 = homogeneous(0, ctx)?;
            let h1 = homogeneous(1, ctx)?;
            let h2 = if p > 1 { homogeneous(2, ctx)? } else { zero() };
            let mut j1 = zero();
            let mut j2 = zero();
            let first = Expansion::integer(p as u64, ctx)?;
            let second = Expansion::integer((p * (p - 1)) as u64, ctx)?;
            for axis in 0..4 {
                j1[axis] = h1[axis].sub(&h0[axis], ctx)?.mul(&first, ctx)?;
                if p > 1 {
                    j2[axis] = h2[axis]
                        .sub(&h1[axis], ctx)?
                        .sub(&h1[axis].sub(&h0[axis], ctx)?, ctx)?
                        .mul(&second, ctx)?;
                }
            }
            Ok([h0, j1, j2])
        };
        let mut aa = Vec::new();
        let mut bb = Vec::new();
        for row in 0..a.len() {
            aa.push(build(0, row, ctx)?);
            bb.push(build(1, row, ctx)?);
        }
        let mut residual1 = Vec::new();
        let mut residual2 = Vec::new();
        let s2 = s.mul(s, ctx)?;
        let unit2 = unit.mul(unit, ctx)?;
        for row in 0..a.len() {
            let mut first = zero();
            let mut second = zero();
            for axis in 0..4 {
                if aa[row][0][axis].sub(&bb[row][0][axis], ctx)?.sign() != Sign::Zero {
                    return Ok(BezierIdentity::Different);
                }
                first[axis] = bb[row][1][axis]
                    .mul(unit, ctx)?
                    .add(&aa[row][1][axis].mul(s, ctx)?, ctx)?;
                if order == 2 {
                    second[axis] = bb[row][2][axis]
                        .mul(&unit2, ctx)?
                        .sub(&aa[row][2][axis].mul(&s2, ctx)?, ctx)?;
                }
            }
            residual1.push(first);
            residual2.push(second);
        }
        // R1=lambda H, with one constant lambda across the entire seam.
        let lambda_n = &residual1[0][3];
        let lambda_d = &aa[0][0][3];
        for row in 0..a.len() {
            for axis in 0..4 {
                if residual1[row][axis]
                    .mul(lambda_d, ctx)?
                    .sub(&aa[row][0][axis].mul(lambda_n, ctx)?, ctx)?
                    .sign()
                    != Sign::Zero
                {
                    return Ok(BezierIdentity::Different);
                }
            }
        }
        if order == 1 {
            return Ok(BezierIdentity::Equal);
        }
        // R2=mu H+nu JA1. Solve two independent coordinates once, then
        // check every coefficient by cross multiplication; no division occurs.
        let mut solution = None;
        'select: for row in 0..a.len() {
            for axis in 0..3 {
                let h = &aa[row][0];
                let j = &aa[row][1];
                let r = &residual2[row];
                let determinant = h[3]
                    .mul(&j[axis], ctx)?
                    .sub(&h[axis].mul(&j[3], ctx)?, ctx)?;
                if determinant.sign() != Sign::Zero {
                    let mu = r[3]
                        .mul(&j[axis], ctx)?
                        .sub(&r[axis].mul(&j[3], ctx)?, ctx)?;
                    let nu = h[3]
                        .mul(&r[axis], ctx)?
                        .sub(&h[axis].mul(&r[3], ctx)?, ctx)?;
                    solution = Some((determinant, mu, nu));
                    break 'select;
                }
            }
        }
        let Some((d, mu, nu)) = solution else {
            return Ok(BezierIdentity::Indeterminate(Reason::MissingProof));
        };
        for row in 0..a.len() {
            for axis in 0..4 {
                let residual = residual2[row][axis]
                    .mul(&d, ctx)?
                    .sub(&aa[row][0][axis].mul(&mu, ctx)?, ctx)?
                    .sub(&aa[row][1][axis].mul(&nu, ctx)?, ctx)?;
                if residual.sign() != Sign::Zero {
                    return Ok(BezierIdentity::Different);
                }
            }
        }
        Ok(BezierIdentity::Equal)
    })();
    let result = result.and_then(|r| ctx.charge(0).map(|_| r));
    Ok(BezierIdentityDecision {
        outcome: result.unwrap_or_else(BezierIdentity::Indeterminate),
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}

/// Exact C-infinity across internal knots of a degree-one along basis when
/// every homogeneous cross coefficient has the same derivative on each span.
/// This is sufficient, and retains original authored controls and knot values.
pub fn rational_linear_strip_along_identity(
    ctx: &mut PredicateContext<'_>,
    strip: &[Vec<[LeafRef; 4]>],
    knots: &[LeafRef],
) -> Result<BezierIdentityDecision, InputError> {
    if !(3..=33).contains(&strip.len())
        || knots.len() != strip.len() + 2
        || strip
            .iter()
            .any(|row| row.len() != strip[0].len() || !(2..=33).contains(&row.len()))
    {
        return Err(InputError::InvalidInput("Invalid linear along strip"));
    }
    let mut values = strip
        .iter()
        .flat_map(|row| row.iter().flatten())
        .chain(knots.iter())
        .map(|r| ctx.resolve(*r).cloned())
        .collect::<Result<Vec<_>, _>>()?;
    values.push(AuthoredScalar::Binary64Bits(1f64.to_bits()));
    let result = (|| -> Result<BezierIdentity, Reason> {
        ctx.charge(values.len() as u64)?;
        let x = exact_inputs(&values, ctx)?;
        let unit = x.last().unwrap();
        let columns = strip[0].len();
        let start = strip.len() * columns * 4;
        if x[..start]
            .chunks_exact(4)
            .any(|p| p[3].sign() != Sign::Positive)
        {
            return Ok(BezierIdentity::Indeterminate(Reason::MissingProof));
        }
        let h = |row: usize, column: usize, axis: usize, ctx: &mut PredicateContext<'_>| {
            let at = (row * columns + column) * 4;
            x[at + 3].mul(if axis == 3 { unit } else { &x[at + axis] }, ctx)
        };
        for row in 1..strip.len() - 1 {
            let left = x[start + row + 1].sub(&x[start + row], ctx)?;
            let right = x[start + row + 2].sub(&x[start + row + 1], ctx)?;
            if left.sign() != Sign::Positive || right.sign() != Sign::Positive {
                return Ok(BezierIdentity::Indeterminate(Reason::MissingProof));
            }
            for column in 0..columns {
                for axis in 0..4 {
                    let mid = h(row, column, axis, ctx)?;
                    let dl = mid.sub(&h(row - 1, column, axis, ctx)?, ctx)?;
                    let dr = h(row + 1, column, axis, ctx)?.sub(&mid, ctx)?;
                    if dl.mul(&right, ctx)?.sub(&dr.mul(&left, ctx)?, ctx)?.sign() != Sign::Zero {
                        return Ok(BezierIdentity::Different);
                    }
                }
            }
        }
        Ok(BezierIdentity::Equal)
    })();
    let result = result.and_then(|r| ctx.charge(0).map(|_| r));
    Ok(BezierIdentityDecision {
        outcome: result.unwrap_or_else(BezierIdentity::Indeterminate),
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}

#[cfg(test)]
mod conic_tests {
    use super::*;
    use crate::{SourceArena, ToleranceContext, Limits};
    fn audit(weight_scale:f64, damage:bool, varying:bool, order:usize, max_work:u64)->BezierIdentityDecision {
        let mut values=Vec::new();
        for side in 0..2 {
            for row in 0..6 {
                let c=[row as f64/8.,0.,row as f64];
                let a=[1.,0.,row as f64/16.];let b=[0.,1.,row as f64/8.];
                let sign=if side==0 {1.} else {-1.};
                for layer in 0..3 {
                    for axis in 0..3 {
                        let mut p=c[axis]+if layer<2 {b[axis]} else {0.}+if layer>0 {sign*a[axis]} else {0.};
                        if damage&&side==1&&row==4&&layer==2&&axis==0 {p+=f64::EPSILON;}
                        values.push(p);
                    }
                    values.push(weight_scale*if layer==1 {if varying&&row==4 {0.5} else {std::f64::consts::FRAC_1_SQRT_2}} else {1.});
                }
            }
        }
        values.push(1.);
        let source=SourceArena::authored("conic-strip",1,values.into_iter().map(|v|AuthoredScalar::Binary64Bits(v.to_bits())).collect()).unwrap();
        let refs=|start:usize|(0..6).map(|row|(0..3).map(|layer|std::array::from_fn(|k|source.leaf(start+row*12+layer*4+k).unwrap())).collect()).collect::<Vec<_>>();
        let tolerance=ToleranceContext::default_valid();
        let mut ctx=PredicateContext::new(&source,&tolerance,Limits {max_work,..Limits::default()},None);
        rational_projective_strip_jet_identity(&mut ctx,&refs(0),&refs(72),source.leaf(144).unwrap(),order).unwrap()
    }
    #[test]
    fn canonical_relation_agrees_with_the_general_projective_predicate() {
        let fast=audit(1.,false,false,2,1000000);
        let general=audit(2.,false,false,2,1000000);
        assert_eq!(fast.outcome,BezierIdentity::Equal);
        assert_eq!(general.outcome,BezierIdentity::Equal);
        assert!(fast.work_used<general.work_used);
        assert!(matches!(audit(1.,false,false,2,fast.work_used-1).outcome,BezierIdentity::Indeterminate(Reason::ResourceLimit)));
    }
    #[test]
    fn tiny_second_jet_damage_and_varying_weights_are_not_promoted() {
        assert_eq!(audit(1.,true,false,1,1000000).outcome,BezierIdentity::Equal);
        assert_eq!(audit(1.,true,false,2,1000000).outcome,BezierIdentity::Different);
        assert_eq!(audit(1.,false,true,1,1000000).outcome,BezierIdentity::Different);
    }
}
