//! Sufficient exact C1/C2 identity of rational Bezier boundary strips.
//! Rows follow the seam; columns point inward from the clamped boundary.
//! This predicate alone proves neither regularity nor general G1/G2.
use crate::{
    Algebra, AuthoredScalar, BezierIdentity, BezierIdentityDecision, Expansion, InputError,
    LeafRef, PredicateContext, Sign, exact_inputs,
};

pub fn rational_bezier_strip_jet_identity(
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
        return Err(InputError::InvalidInput("Invalid Bezier boundary strip"));
    }
    let leaves = a
        .iter()
        .chain(b)
        .flat_map(|row| row.iter().flatten())
        .copied()
        .chain(std::iter::once(scale))
        .collect::<Vec<_>>();
    let mut values = leaves
        .iter()
        .map(|r| ctx.resolve(*r).cloned())
        .collect::<Result<Vec<AuthoredScalar>, _>>()?;
    values.push(AuthoredScalar::Binary64Bits(1f64.to_bits()));
    let result = (|| {
        ctx.charge(values.len() as u64)?;
        let x = exact_inputs(&values, ctx)?;
        let scale = &x[x.len() - 2];
        let unit = x.last().unwrap();
        if scale.sign() != Sign::Positive
            || x[..x.len() - 2]
                .chunks_exact(4)
                .any(|p| p[3].sign() != Sign::Positive)
        {
            return Ok(BezierIdentity::Indeterminate(crate::Reason::MissingProof));
        }
        let pa = a[0].len() - 1;
        let pb = b[0].len() - 1;
        let start_b = a.len() * (pa + 1) * 4;
        for i in 0..a.len() {
            let h = |side: usize, layer: usize, axis: usize| -> Expansion {
                let (start, p) = if side == 0 { (0, pa) } else { (start_b, pb) };
                let at = start + (i * (p + 1) + layer) * 4;
                if axis == 3 {
                    x[at + 3].clone()
                } else {
                    x[at + axis].clone()
                }
            };
            // Original Euclidean coordinates and weights must agree. This
            // deliberately avoids accepting differently scaled homogeneous nets.
            for axis in 0..4 {
                if h(0, 0, axis).sub(&h(1, 0, axis), ctx)?.sign() != Sign::Zero {
                    return Ok(BezierIdentity::Different);
                }
            }
            for axis in 0..4 {
                let jet = |side: usize, q: usize, ctx: &mut PredicateContext<'_>| {
                    let p = if side == 0 { pa } else { pb };
                    let weighted = |layer: usize, ctx: &mut PredicateContext<'_>| {
                        if axis == 3 {
                            Ok(h(side, layer, 3))
                        } else {
                            h(side, layer, axis).mul(&h(side, layer, 3), ctx)
                        }
                    };
                    let v0 = weighted(0, ctx)?;
                    let v1 = weighted(1, ctx)?;
                    if q == 1 {
                        return v1
                            .sub(&v0, ctx)?
                            .mul(&Expansion::integer(p as u64, ctx)?, ctx);
                    }
                    if p == 1 {
                        return Ok(Expansion::scalar(0.));
                    }
                    let v2 = weighted(2, ctx)?;
                    v2.sub(&v1, ctx)?
                        .sub(&v1.sub(&v0, ctx)?, ctx)?
                        .mul(&Expansion::integer((p * (p - 1)) as u64, ctx)?, ctx)
                };
                for q in 1..=order {
                    let ja = jet(0, q, ctx)?;
                    let jb = jet(1, q, ctx)?;
                    let expected = ja.mul(scale, ctx)?;
                    let actual = jb.mul(unit, ctx)?;
                    let difference = if q == 1 {
                        actual.add(&expected, ctx)?
                    } else {
                        actual
                            .mul(unit, ctx)?
                            .sub(&expected.mul(scale, ctx)?, ctx)?
                    };
                    if difference.sign() != Sign::Zero {
                        return Ok(BezierIdentity::Different);
                    }
                }
            }
        }
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
    fn audit(perturb: bool, limits: Limits) -> BezierIdentity {
        let mut values = Vec::new();
        for side in 0..2 {
            for x in [0., 1.] {
                for layer in 0..2 {
                    let mut y: f64 = if layer == 0 {
                        1.
                    } else if side == 0 {
                        0.
                    } else {
                        2.
                    };
                    if perturb && side == 1 && layer == 1 {
                        y = f64::from_bits(y.to_bits() + 1);
                    }
                    values.extend([x, y, 0., 1.]);
                }
            }
        }
        values.push(1.);
        let arena = SourceArena::authored(
            "strip-jets-test",
            1,
            values
                .iter()
                .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
                .collect(),
        )
        .unwrap();
        let strip = |start: usize| {
            (0usize..2)
                .map(|i| {
                    (0usize..2)
                        .map(|j| {
                            std::array::from_fn(|k| {
                                arena.leaf(start + (i * 2 + j) * 4 + k).unwrap()
                            })
                        })
                        .collect()
                })
                .collect::<Vec<_>>()
        };
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(&arena, &tolerance, limits, None);
        rational_bezier_strip_jet_identity(
            &mut ctx,
            &strip(0),
            &strip(16),
            arena.leaf(32).unwrap(),
            2,
        )
        .unwrap()
        .outcome
    }
    #[test]
    fn exact_linear_second_jets_and_one_ulp_difference() {
        assert_eq!(audit(false, Limits::default()), BezierIdentity::Equal);
        assert_eq!(audit(true, Limits::default()), BezierIdentity::Different);
        assert!(matches!(
            audit(
                false,
                Limits {
                    max_work: 0,
                    ..Limits::default()
                }
            ),
            BezierIdentity::Indeterminate(_)
        ));
    }
    fn strips(
        a: &[Vec<[f64; 4]>],
        b: &[Vec<[f64; 4]>],
        scale: f64,
        order: usize,
    ) -> BezierIdentity {
        let values = a
            .iter()
            .chain(b)
            .flat_map(|r| r.iter().flatten())
            .copied()
            .chain(std::iter::once(scale))
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect::<Vec<_>>();
        let count = values.len();
        let arena = SourceArena::authored("rational-strip-test", 1, values).unwrap();
        let refs = |start: usize, s: &[Vec<[f64; 4]>]| {
            s.iter()
                .enumerate()
                .map(|(i, row)| {
                    row.iter()
                        .enumerate()
                        .map(|(j, _)| {
                            std::array::from_fn(|k| {
                                arena.leaf(start + (i * row.len() + j) * 4 + k).unwrap()
                            })
                        })
                        .collect()
                })
                .collect::<Vec<_>>()
        };
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
        rational_bezier_strip_jet_identity(
            &mut ctx,
            &refs(0, a),
            &refs(a.len() * a[0].len() * 4, b),
            arena.leaf(count - 1).unwrap(),
            order,
        )
        .unwrap()
        .outcome
    }
    #[test]
    fn rational_seam_weights_scale_and_second_order_mismatch() {
        let a = (0..3)
            .map(|i| {
                [1., 0.5, 0.]
                    .map(|y| [i as f64, y, 0., [1., 0.5, 2.][i]])
                    .to_vec()
            })
            .collect::<Vec<_>>();
        let b = (0..3)
            .map(|i| {
                [1., 1.5, 2.]
                    .map(|y| [i as f64, y, 0., [1., 0.5, 2.][i]])
                    .to_vec()
            })
            .collect::<Vec<_>>();
        assert_eq!(strips(&a, &b, 1., 2), BezierIdentity::Equal);
        let mut scaled = b.clone();
        for row in &mut scaled {
            row[1][1] = 2.;
            row[2][1] = 3.;
        }
        assert_eq!(strips(&a, &scaled, 2., 2), BezierIdentity::Equal);
        assert_eq!(strips(&a, &scaled, 1., 1), BezierIdentity::Different);
        let mut bent = b.clone();
        bent[1][2][2] = 0.25;
        assert_eq!(strips(&a, &bent, 1., 1), BezierIdentity::Equal);
        assert_eq!(strips(&a, &bent, 1., 2), BezierIdentity::Different);
        let linear = a.iter().map(|row| vec![row[0], row[2]]).collect::<Vec<_>>();
        assert_eq!(strips(&linear, &b, 1., 2), BezierIdentity::Equal);
        let mut bad = b.clone();
        bad[0][0][3] = 2.;
        assert_eq!(strips(&a, &bad, 1., 2), BezierIdentity::Different);
    }
}
