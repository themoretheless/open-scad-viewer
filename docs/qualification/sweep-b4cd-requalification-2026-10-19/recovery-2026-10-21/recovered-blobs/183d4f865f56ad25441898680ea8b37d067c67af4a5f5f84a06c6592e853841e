//! Exact sufficient Cartesian endpoint C1/C2 identity of clamped B-splines.
//! Homogeneous jets may differ by one positive constant endpoint scale.
//! All inputs are original coordinates, weights and knots; no extracted jets.
use crate::{
    Algebra, AuthoredScalar, BezierIdentity, BezierIdentityDecision, Expansion, InputError,
    LeafRef, PredicateContext, Sign, exact_inputs,
};

pub fn rational_bspline_endpoint_jet_identity(
    ctx: &mut PredicateContext<'_>,
    controls: &[[LeafRef; 4]],
    knots: &[LeafRef],
    degree: usize,
    order: usize,
) -> Result<BezierIdentityDecision, InputError> {
    if !(1..=32).contains(&degree)
        || !(1..=2).contains(&order)
        || controls.len() < degree + 1
        || knots.len() != controls.len() + degree + 1
    {
        return Err(InputError::InvalidInput(
            "Invalid clamped B-spline endpoint jets",
        ));
    }
    let leaves = controls
        .iter()
        .flatten()
        .chain(knots)
        .copied()
        .collect::<Vec<_>>();
    let values = leaves
        .iter()
        .map(|r| ctx.resolve(*r).cloned())
        .collect::<Result<Vec<AuthoredScalar>, _>>()?;
    let result = (|| {
        ctx.charge(values.len() as u64)?;
        let x = exact_inputs(&values, ctx)?;
        let n = controls.len() - 1;
        let u = &x[controls.len() * 4..];
        let a = &u[degree];
        let b = &u[n + 1];
        if b.sub(a, ctx)?.sign() != Sign::Positive
            || x[..controls.len() * 4]
                .chunks_exact(4)
                .any(|h| h[3].sign() != Sign::Positive)
        {
            return Ok(BezierIdentity::Indeterminate(crate::Reason::MissingProof));
        }
        for pair in u.windows(2) {
            if pair[1].sub(&pair[0], ctx)?.sign() == Sign::Negative {
                return Ok(BezierIdentity::Indeterminate(crate::Reason::MissingProof));
            }
        }
        for t in &u[..degree + 1] {
            if t.sub(a, ctx)?.sign() != Sign::Zero {
                return Ok(BezierIdentity::Indeterminate(crate::Reason::MissingProof));
            }
        }
        for t in &u[n + 1..] {
            if t.sub(b, ctx)?.sign() != Sign::Zero {
                return Ok(BezierIdentity::Indeterminate(crate::Reason::MissingProof));
            }
        }
        let s0 = u[degree + 1].sub(a, ctx)?;
        let e0 = b.sub(&u[n], ctx)?;
        if s0.sign() != Sign::Positive || e0.sign() != Sign::Positive {
            return Ok(BezierIdentity::Indeterminate(crate::Reason::MissingProof));
        }
        // A single positive homogeneous scale at the seam preserves all
        // Cartesian jets. Keep its original endpoint weights cross-multiplied.
        let ws = &x[3];
        let we = &x[n * 4 + 3];
        for axis in 0..4 {
            let h =
                |i: usize, ctx: &mut PredicateContext<'_>| -> Result<Expansion, crate::Reason> {
                    if axis == 3 {
                        Ok(x[i * 4 + 3].clone())
                    } else {
                        x[i * 4 + axis].mul(&x[i * 4 + 3], ctx)
                    }
                };
            let hs = h(0, ctx)?;
            let he = h(n, ctx)?;
            if he.mul(ws, ctx)?.sub(&hs.mul(we, ctx)?, ctx)?.sign() != Sign::Zero {
                return Ok(BezierIdentity::Different);
            }
            let ds = h(1, ctx)?.sub(&hs, ctx)?;
            let de = he.sub(&h(n - 1, ctx)?, ctx)?;
            if de
                .mul(&s0, ctx)?
                .mul(ws, ctx)?
                .sub(&ds.mul(&e0, ctx)?.mul(we, ctx)?, ctx)?
                .sign()
                != Sign::Zero
            {
                return Ok(BezierIdentity::Different);
            }
            if order == 2 && degree >= 2 {
                let s1 = u[degree + 2].sub(a, ctx)?;
                let e1 = b.sub(&u[n - 1], ctx)?;
                let ds1 = h(2, ctx)?.sub(&h(1, ctx)?, ctx)?;
                let de1 = h(n - 1, ctx)?.sub(&h(n - 2, ctx)?, ctx)?;
                let end = de
                    .mul(&e1, ctx)?
                    .sub(&de1.mul(&e0, ctx)?, ctx)?
                    .mul(&s0, ctx)?
                    .mul(&s0, ctx)?
                    .mul(&s1, ctx)?;
                let start = ds1
                    .mul(&s0, ctx)?
                    .sub(&ds.mul(&s1, ctx)?, ctx)?
                    .mul(&e0, ctx)?
                    .mul(&e0, ctx)?
                    .mul(&e1, ctx)?;
                if end.mul(ws, ctx)?.sub(&start.mul(we, ctx)?, ctx)?.sign() != Sign::Zero {
                    return Ok(BezierIdentity::Different);
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
    fn audit(
        perturb: bool,
        changed_knots: bool,
        max_work: u64,
        order: usize,
    ) -> BezierIdentityDecision {
        // Cubic, four spans, simple internal knots: global basis C2.
        // Endpoint first/second homogeneous jets agree on equal end widths.
        let mut points = [
            [0., 0., 1.],
            [0.125, 0., 1.],
            [0.375, 0.125, 1.],
            [0., 0.25, 1.],
            [-0.375, 0.125, 1.],
            [-0.125, 0., 1.],
            [0., 0., 1.],
        ];
        if perturb {
            points[2][0] = 0.375_f64.next_up();
        }
        let mut knots = vec![0., 0., 0., 0., 0.25, 0.5, 0.75, 1., 1., 1., 1.];
        if changed_knots {
            knots[4] = 0.125;
        }
        let mut values = points
            .iter()
            .flat_map(|p| [p[0], p[1], p[2], 1.])
            .collect::<Vec<f64>>();
        values.extend(knots);
        let arena = SourceArena::authored(
            "endpoint-test",
            1,
            values
                .iter()
                .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
                .collect(),
        )
        .unwrap();
        let controls = (0..7)
            .map(|i| std::array::from_fn(|j| arena.leaf(i * 4 + j).unwrap()))
            .collect::<Vec<_>>();
        let knots = (28..values.len())
            .map(|i| arena.leaf(i).unwrap())
            .collect::<Vec<_>>();
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(
            &arena,
            &tolerance,
            Limits {
                max_work,
                ..Limits::default()
            },
            None,
        );
        rational_bspline_endpoint_jet_identity(&mut ctx, &controls, &knots, 3, order).unwrap()
    }
    #[test]
    fn projective_scale_must_match_all_original_endpoint_jets() {
        let audit = |changed_weight: Option<usize>, order, max_work| {
            let points = [
                [0., 0., 1.],
                [0.125, 0., 1.],
                [0.375, 0.125, 1.],
                [0., 0.25, 1.],
                [-0.375, 0.125, 1.],
                [-0.125, 0., 1.],
                [0., 0., 1.],
            ];
            let mut weights = [1_f64, 1., 1., 1.5, 2., 2., 2.];
            if let Some(i) = changed_weight {
                weights[i] = weights[i].next_up();
            }
            let mut values = points
                .iter()
                .zip(weights)
                .flat_map(|(p, w)| [p[0], p[1], p[2], w])
                .collect::<Vec<_>>();
            values.extend([0., 0., 0., 0., 0.25, 0.5, 0.75, 1., 1., 1., 1.]);
            let arena = SourceArena::authored(
                "projective-endpoint-test",
                1,
                values
                    .iter()
                    .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
                    .collect(),
            )
            .unwrap();
            let controls = (0..7)
                .map(|i| std::array::from_fn(|j| arena.leaf(i * 4 + j).unwrap()))
                .collect::<Vec<_>>();
            let knots = (28..values.len())
                .map(|i| arena.leaf(i).unwrap())
                .collect::<Vec<_>>();
            let tolerance = ToleranceContext::default_valid();
            let mut ctx = PredicateContext::new(
                &arena,
                &tolerance,
                Limits {
                    max_work,
                    ..Limits::default()
                },
                None,
            );
            rational_bspline_endpoint_jet_identity(&mut ctx, &controls, &knots, 3, order).unwrap()
        };
        let good = audit(None, 2, 1000000);
        assert_eq!(good.outcome, BezierIdentity::Equal);
        assert_eq!(
            audit(Some(4), 2, 1000000).outcome,
            BezierIdentity::Different
        );
        assert_eq!(audit(Some(4), 1, 1000000).outcome, BezierIdentity::Equal);
        assert_eq!(
            audit(Some(5), 1, 1000000).outcome,
            BezierIdentity::Different
        );
        assert!(matches!(
            audit(None, 2, good.work_used - 1).outcome,
            BezierIdentity::Indeterminate(_)
        ));
    }
    #[test]
    fn unequal_end_spans_use_original_parameter_derivative_scaling() {
        let points = [
            [0., 0., 1.],
            [0.0625, 0., 1.],
            [0.3125, 0., 1.],
            [0., 0.25, 1.],
            [-0.375, 0., 1.],
            [-0.125, 0., 1.],
            [0., 0., 1.],
        ];
        let mut values = points
            .iter()
            .flat_map(|p| [p[0], p[1], p[2], 1.])
            .collect::<Vec<f64>>();
        values.extend([0., 0., 0., 0., 0.125, 0.5, 0.75, 1., 1., 1., 1.]);
        let arena = SourceArena::authored(
            "asymmetric-endpoints",
            1,
            values
                .iter()
                .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
                .collect(),
        )
        .unwrap();
        let controls = (0..7)
            .map(|i| std::array::from_fn(|j| arena.leaf(i * 4 + j).unwrap()))
            .collect::<Vec<_>>();
        let knots = (28..values.len())
            .map(|i| arena.leaf(i).unwrap())
            .collect::<Vec<_>>();
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
        assert_eq!(
            rational_bspline_endpoint_jet_identity(&mut ctx, &controls, &knots, 3, 2)
                .unwrap()
                .outcome,
            BezierIdentity::Equal
        );
    }
    #[test]
    fn multispan_original_jets_detect_one_bit_and_knot_parameterization() {
        let good = audit(false, false, 1000000, 2);
        assert_eq!(good.outcome, BezierIdentity::Equal);
        assert_eq!(
            audit(true, false, 1000000, 2).outcome,
            BezierIdentity::Different
        );
        assert_eq!(
            audit(true, false, 1000000, 1).outcome,
            BezierIdentity::Equal
        );
        assert_eq!(
            audit(false, true, 1000000, 1).outcome,
            BezierIdentity::Different
        );
        assert!(matches!(
            audit(false, false, good.work_used - 1, 2).outcome,
            BezierIdentity::Indeterminate(_)
        ));
        assert!(matches!(
            audit(false, false, 0, 2).outcome,
            BezierIdentity::Indeterminate(_)
        ));
    }
}
