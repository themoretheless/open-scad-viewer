//! Exact original-curve plane residual sign at an authored binary64 parameter.
use super::{Rational, series};
use crate::{
    Algebra, AuthoredScalar, Decision, Expansion, InputError, LeafRef, Outcome, PredicateContext,
    Reason, Sign, Stage,
};
/// Evaluate the homogeneous plane residual by rational de Boor expansions.
/// Original binary64 controls, weights, knots, plane and parameter only; no
/// rounded extracted point or floating division is admitted as geometry.
/// The context bounds expansion terms, work, cancellation and deadline.
pub fn rational_bspline_plane_incidence(
    ctx: &mut PredicateContext<'_>,
    controls: &[[LeafRef; 4]],
    knots: &[LeafRef],
    degree: usize,
    span: usize,
    parameter: LeafRef,
    plane: [LeafRef; 4],
) -> Result<Decision, InputError> {
    let n = controls.len();
    if !(1..=25).contains(&degree)
        || n <= degree
        || n > 256
        || knots.len() != n + degree + 1
        || span < degree
        || span >= n
    {
        return Err(InputError::InvalidInput(
            "Invalid original B-spline plane query",
        ));
    }
    let values = controls
        .iter()
        .flatten()
        .chain(knots)
        .copied()
        .chain(plane)
        .chain([parameter])
        .map(|r| ctx.resolve(r).cloned())
        .collect::<Result<Vec<_>, _>>()?;
    let numbers = values
        .iter()
        .map(|v| match v {
            AuthoredScalar::Binary64Bits(bits) => Some(f64::from_bits(*bits)),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()
        .ok_or(InputError::InvalidInput(
            "Plane query requires original binary64 leaves",
        ))?;
    let u = &numbers[n * 4..n * 4 + knots.len()];
    let tail = n * 4 + knots.len();
    let t = numbers[tail + 4];
    if numbers[..n * 4].chunks_exact(4).any(|p| p[3] <= 0.)
        || u.windows(2).any(|p| p[0] > p[1])
        || u[degree] >= u[n]
        || u[span] >= u[span + 1]
        || t < u[span]
        || t > u[span + 1]
        || numbers[tail..tail + 3].iter().all(|v| *v == 0.)
    {
        return Err(InputError::InvalidInput(
            "Invalid weights, active span, parameter or plane normal",
        ));
    }
    let result = (|| {
        ctx.charge(values.len() as u64)?;
        // Do not normalize all leaves by one common scale: coordinates,
        // weights, normal and offset have different algebraic roles.
        let x: Vec<_> = numbers.iter().map(|v| Expansion::scalar(*v)).collect();
        let u = &x[n * 4..n * 4 + knots.len()];
        let h = series(&x[..n * 4], u, degree, span, &x[tail + 4], 0, ctx)?;
        if h[0][3].n.sign() != Sign::Positive || h[0][3].d.sign() != Sign::Positive {
            return Err(Reason::MissingProof);
        }
        let mut residual = Rational::zero();
        for axis in 0..3 {
            residual = residual.add(
                &h[0][axis].mul(&Rational::scalar(x[tail + axis].clone()), ctx)?,
                ctx,
            )?;
        }
        residual = residual.sub(
            &h[0][3].mul(&Rational::scalar(x[tail + 3].clone()), ctx)?,
            ctx,
        )?;
        if residual.d.sign() != Sign::Positive {
            return Err(Reason::MissingProof);
        }
        Ok(residual.n.sign())
    })();
    Ok(Decision {
        outcome: result
            .map(Outcome::Sign)
            .unwrap_or_else(Outcome::Indeterminate),
        stage: Stage::ExactExpansion,
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Limits, SourceArena, ToleranceContext};
    fn query(
        delta: f64,
        t: f64,
        normal: [f64; 3],
        offset: f64,
        budget: u64,
    ) -> Result<Decision, InputError> {
        let z = [-8., 1., 0., 3.];
        let w = [1., 2., 4., 8.];
        let mut values = Vec::new();
        for i in 0..4 {
            values.extend([
                i as f64,
                0.,
                7. - 3. * i as f64 + z[i] + if i == 0 { delta } else { 0. },
                w[i],
            ]);
        }
        values.extend([-3., -2., -1., 0., 1., 2., 3., 4.]);
        values.extend(normal);
        values.extend([offset, t]);
        let arena = SourceArena::authored(
            "original-cubic-plane",
            1,
            values
                .into_iter()
                .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
                .collect(),
        )?;
        let controls: Vec<_> = (0..4)
            .map(|i| std::array::from_fn(|j| arena.leaf(i * 4 + j).unwrap()))
            .collect();
        let knots: Vec<_> = (0..8).map(|i| arena.leaf(16 + i).unwrap()).collect();
        let plane = std::array::from_fn(|i| arena.leaf(24 + i).unwrap());
        let parameter = arena.leaf(28)?;
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(
            &arena,
            &tolerance,
            Limits {
                max_work: budget,
                ..Limits::default()
            },
            None,
        );
        rational_bspline_plane_incidence(&mut ctx, &controls, &knots, 3, 3, parameter, plane)
    }
    #[test]
    fn original_cubic_plane_incidence_proves_nondyadic_basis_zero_and_preserves_residual() {
        let zero = query(0., 0., [3., -2., 1.], 7., 100000).unwrap();
        assert_eq!(zero.outcome, Outcome::Sign(Sign::Zero));
        assert!(zero.work_used > 0);
        assert_eq!(
            query(0., 0.5, [3., -2., 1.], 7., 100000).unwrap().outcome,
            Outcome::Sign(Sign::Positive)
        );
        assert_eq!(
            query(0., 0.5, [-6., 4., -2.], -14., 100000)
                .unwrap()
                .outcome,
            Outcome::Sign(Sign::Negative)
        );
        assert_eq!(
            query(2_f64.powi(-30), 0., [3., -2., 1.], 7., 100000)
                .unwrap()
                .outcome,
            Outcome::Sign(Sign::Positive)
        );
        assert_eq!(
            query(0., 0., [3., -2., 1.], 7. + 2_f64.powi(-30), 100000)
                .unwrap()
                .outcome,
            Outcome::Sign(Sign::Negative)
        );
    }
    #[test]
    fn original_plane_query_budget_and_invalid_parameter_do_not_return_a_sign() {
        let r = query(0., 0., [3., -2., 1.], 7., 0).unwrap();
        assert_eq!(r.outcome, Outcome::Indeterminate(Reason::ResourceLimit));
        assert_eq!(r.work_used, 0);
        assert!(query(0., 2., [3., -2., 1.], 7., 100000).is_err());
        assert!(query(0., 0., [0.; 3], 7., 100000).is_err());
    }
}

/// Exact sign of H1²-H0*H2 for an original positive-weight quadratic Bezier
/// plane numerator Hi=wi*(normal·Pi-offset). This does not by itself assert
/// interior root location; the caller must separately qualify coefficient signs.
pub fn rational_quadratic_plane_discriminant(
    ctx: &mut PredicateContext<'_>,
    controls: [[LeafRef; 4]; 3],
    plane: [LeafRef; 4],
) -> Result<Decision, InputError> {
    let values = controls
        .iter()
        .flatten()
        .copied()
        .chain(plane)
        .map(|r| ctx.resolve(r).cloned())
        .collect::<Result<Vec<_>, _>>()?;
    let numbers = values
        .iter()
        .map(|v| match v {
            AuthoredScalar::Binary64Bits(bits) => Some(f64::from_bits(*bits)),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()
        .ok_or(InputError::InvalidInput(
            "Discriminant requires original binary64 leaves",
        ))?;
    if numbers[..12].chunks_exact(4).any(|p| p[3] <= 0.) || numbers[12..15].iter().all(|v| *v == 0.)
    {
        return Err(InputError::InvalidInput(
            "Invalid quadratic weights or plane normal",
        ));
    }
    let result = (|| {
        ctx.charge(values.len() as u64)?;
        let x: Vec<_> = numbers.iter().map(|v| Expansion::scalar(*v)).collect();
        let mut residuals = Vec::new();
        for i in 0..3 {
            let mut value = Expansion::scalar(0.).sub(&x[15], ctx)?;
            for axis in 0..3 {
                value = value.add(&x[i * 4 + axis].mul(&x[12 + axis], ctx)?, ctx)?;
            }
            residuals.push(value.mul(&x[i * 4 + 3], ctx)?);
        }
        Ok(residuals[1]
            .mul(&residuals[1], ctx)?
            .sub(&residuals[0].mul(&residuals[2], ctx)?, ctx)?
            .sign())
    })();
    Ok(Decision {
        outcome: result
            .map(Outcome::Sign)
            .unwrap_or_else(Outcome::Indeterminate),
        stage: Stage::ExactExpansion,
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}

#[cfg(test)]
mod discriminant_tests {
    use super::*;
    use crate::{Limits, SourceArena, ToleranceContext};
    fn query(delta: f64, orientation: f64, budget: u64) -> Result<Decision, InputError> {
        let mut values = vec![0., 0., 6., 1., 1., 0., 3., 2., 2., 0., 4. + delta, 4.];
        values.extend([orientation, 0., orientation, 5. * orientation]);
        let arena = SourceArena::authored(
            "original-quadratic-discriminant",
            1,
            values
                .into_iter()
                .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
                .collect(),
        )?;
        let controls =
            std::array::from_fn(|i| std::array::from_fn(|j| arena.leaf(i * 4 + j).unwrap()));
        let plane = std::array::from_fn(|i| arena.leaf(12 + i).unwrap());
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(
            &arena,
            &tolerance,
            Limits {
                max_work: budget,
                ..Limits::default()
            },
            None,
        );
        rational_quadratic_plane_discriminant(&mut ctx, controls, plane)
    }
    #[test]
    fn original_discriminant_distinguishes_double_root_gap_and_two_roots() {
        for scale in [1., -2.] {
            let r = query(0., scale, 100000).unwrap();
            assert_eq!(r.outcome, Outcome::Sign(Sign::Zero));
            assert!(r.work_used > 0);
            assert_eq!(
                query(2_f64.powi(-30), scale, 100000).unwrap().outcome,
                Outcome::Sign(Sign::Negative)
            );
            assert_eq!(
                query(-2_f64.powi(-30), scale, 100000).unwrap().outcome,
                Outcome::Sign(Sign::Positive)
            );
        }
    }
    #[test]
    fn discriminant_budget_or_invalid_normal_never_becomes_zero_proof() {
        let r = query(0., 1., 0).unwrap();
        assert_eq!(r.outcome, Outcome::Indeterminate(Reason::ResourceLimit));
        assert_eq!(r.work_used, 0);
        assert!(query(0., 0., 100000).is_err());
    }
}
