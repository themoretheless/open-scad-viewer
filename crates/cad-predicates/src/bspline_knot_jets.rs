//! Exact sufficient C1/C2 joins of original piecewise-Bezier B-splines.
//! A positive constant homogeneous scale relates the one-sided jets.
//! No knot insertion or rounded derivative is an input to this predicate.
use crate::{
    exact_inputs, Algebra, AuthoredScalar, BezierIdentity, BezierIdentityDecision, Expansion,
    InputError, LeafRef, PredicateContext, Sign,
};

pub fn rational_piecewise_bezier_knot_jet_identity(
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
            "Invalid piecewise-Bezier knot jets",
        ));
    }
    let values = controls
        .iter()
        .flatten()
        .chain(knots)
        .copied()
        .map(|r| ctx.resolve(r).cloned())
        .collect::<Result<Vec<AuthoredScalar>, _>>()?;
    let result = (|| {
        ctx.charge(values.len() as u64)?;
        let x = exact_inputs(&values, ctx)?;
        let u = &x[controls.len() * 4..];
        if x[..controls.len() * 4]
            .chunks_exact(4)
            .any(|h| h[3].sign() != Sign::Positive)
        {
            return Ok(BezierIdentity::Indeterminate(crate::Reason::MissingProof));
        }
        let mut groups = vec![(0usize, 1usize)];
        for i in 1..u.len() {
            match u[i].sub(&u[i - 1], ctx)?.sign() {
                Sign::Negative => {
                    return Ok(BezierIdentity::Indeterminate(crate::Reason::MissingProof))
                }
                Sign::Zero => groups.last_mut().unwrap().1 += 1,
                Sign::Positive => groups.push((i, 1)),
            }
        }
        if groups.len() < 2
            || groups[0].1 != degree + 1
            || groups.last().unwrap().1 != degree + 1
            || groups[1..groups.len() - 1]
                .iter()
                .any(|g| g.1 != degree && g.1 != degree + 1)
        {
            return Ok(BezierIdentity::Indeterminate(crate::Reason::MissingProof));
        }
        let mut left_start = 0usize;
        for join in 1..groups.len() - 1 {
            let right_start = left_start + groups[join].1;
            if right_start + degree >= controls.len() {
                return Ok(BezierIdentity::Indeterminate(crate::Reason::MissingProof));
            }
            let wl = u[groups[join].0].sub(&u[groups[join - 1].0], ctx)?;
            let wr = u[groups[join + 1].0].sub(&u[groups[join].0], ctx)?;
            let le = left_start + degree;
            let ws = &x[right_start * 4 + 3];
            let we = &x[le * 4 + 3];
            for axis in 0..4 {
                let h = |i: usize,
                         ctx: &mut PredicateContext<'_>|
                 -> Result<Expansion, crate::Reason> {
                    if axis == 3 {
                        Ok(x[i * 4 + 3].clone())
                    } else {
                        x[i * 4 + axis].mul(&x[i * 4 + 3], ctx)
                    }
                };
                let he = h(le, ctx)?;
                let hs = h(right_start, ctx)?;
                if he.mul(ws, ctx)?.sub(&hs.mul(we, ctx)?, ctx)?.sign() != Sign::Zero {
                    return Ok(BezierIdentity::Different);
                }
                let dl = he.sub(&h(le - 1, ctx)?, ctx)?;
                let dr = h(right_start + 1, ctx)?.sub(&hs, ctx)?;
                if dl
                    .mul(&wr, ctx)?
                    .mul(ws, ctx)?
                    .sub(&dr.mul(&wl, ctx)?.mul(we, ctx)?, ctx)?
                    .sign()
                    != Sign::Zero
                {
                    return Ok(BezierIdentity::Different);
                }
                if order == 2 && degree >= 2 {
                    let ddl = dl.sub(&h(le - 1, ctx)?.sub(&h(le - 2, ctx)?, ctx)?, ctx)?;
                    let ddr = h(right_start + 2, ctx)?
                        .sub(&h(right_start + 1, ctx)?, ctx)?
                        .sub(&dr, ctx)?;
                    if ddl
                        .mul(&wr, ctx)?
                        .mul(&wr, ctx)?
                        .mul(ws, ctx)?
                        .sub(&ddr.mul(&wl, ctx)?.mul(&wl, ctx)?.mul(we, ctx)?, ctx)?
                        .sign()
                        != Sign::Zero
                    {
                        return Ok(BezierIdentity::Different);
                    }
                }
            }
            left_start = right_start;
        }
        if left_start + degree + 1 != controls.len() {
            return Ok(BezierIdentity::Indeterminate(crate::Reason::MissingProof));
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
        points: &[[f64; 4]],
        knots: &[f64],
        order: usize,
        max_work: u64,
    ) -> BezierIdentityDecision {
        let mut values = points.iter().flatten().copied().collect::<Vec<_>>();
        let start = values.len();
        values.extend_from_slice(knots);
        let arena = SourceArena::authored(
            "piecewise-original",
            1,
            values
                .into_iter()
                .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
                .collect(),
        )
        .unwrap();
        let controls = (0..points.len())
            .map(|i| std::array::from_fn(|j| arena.leaf(i * 4 + j).unwrap()))
            .collect::<Vec<_>>();
        let knots = (0..knots.len())
            .map(|i| arena.leaf(start + i).unwrap())
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
        rational_piecewise_bezier_knot_jet_identity(&mut ctx, &controls, &knots, 2, order).unwrap()
    }
    #[test]
    fn repeated_knot_positive_projective_scale_needs_exact_jets_and_shared_work() {
        let points = [
            [0., 0., 1., 1.],
            [0.125, 0., 1., 1.],
            [0.25, 0., 1., 1.],
            [0.25, 0., 1., 2.],
            [0.375, 0., 1., 2.],
            [0.5, 0., 1., 2.],
        ];
        let knots = [0., 0., 0., 0.5, 0.5, 0.5, 1., 1., 1.];
        let proof = audit(&points, &knots, 2, 1000000);
        assert_eq!(proof.outcome, BezierIdentity::Equal);
        assert_ne!(
            audit(&points, &knots, 2, proof.work_used - 1).outcome,
            BezierIdentity::Equal
        );
        assert_ne!(audit(&points, &knots, 2, 0).outcome, BezierIdentity::Equal);
        let mut wrong = points;
        wrong[5][0] = wrong[5][0].next_up();
        assert_eq!(
            audit(&wrong, &knots, 1, 1000000).outcome,
            BezierIdentity::Equal
        );
        assert_eq!(
            audit(&wrong, &knots, 2, 1000000).outcome,
            BezierIdentity::Different
        );
        wrong = points;
        wrong[3][0] = wrong[3][0].next_up();
        assert_eq!(
            audit(&wrong, &knots, 1, 1000000).outcome,
            BezierIdentity::Different
        );
        wrong = points;
        wrong[4][3] = 0.;
        assert_ne!(
            audit(&wrong, &knots, 2, 1000000).outcome,
            BezierIdentity::Equal
        );
        let short = [0., 0., 0., 0.25, 0.25, 0.25, 1., 1., 1.];
        assert_eq!(
            audit(&points, &short, 1, 1000000).outcome,
            BezierIdentity::Different
        );
    }
}
