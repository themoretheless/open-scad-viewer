//! Exact scalar root coverage of an original positive-weight B-spline span.
use super::{Rational, series};
use crate::{
    Algebra, AuthoredScalar, ContextIdentity, Expansion, InputError, LeafRef, PredicateContext,
    Reason, Sign,
    planar_bezier_roots::{isolate, power},
};
#[derive(Clone, Debug)]
pub struct OriginalPlaneRoot {
    pub interval: [f64; 2],
    pub multiplicity: usize,
}
#[derive(Clone, Debug)]
pub enum OriginalPlaneRootsOutcome {
    Roots(Vec<OriginalPlaneRoot>),
    Coincident,
    Indeterminate(Reason),
}
#[derive(Clone, Debug)]
pub struct OriginalPlaneRootsDecision {
    pub outcome: OriginalPlaneRootsOutcome,
    pub work_used: u64,
    pub context: ContextIdentity,
}
/// Complete distinct roots and scalar multiplicities on the original active
/// span, returned in normalized span coordinates. Original binary64 source
/// leaves only. Exact homogeneous polynomial construction and fraction-free
/// Sturm/GCD isolation share the caller's finite predicate context.
pub fn rational_bspline_plane_roots(
    ctx: &mut PredicateContext<'_>,
    controls: &[[LeafRef; 4]],
    knots: &[LeafRef],
    degree: usize,
    span: usize,
    plane: [LeafRef; 4],
    tolerance: f64,
) -> Result<OriginalPlaneRootsDecision, InputError> {
    let n = controls.len();
    if !(1..=25).contains(&degree)
        || n <= degree
        || n > 256
        || knots.len() != n + degree + 1
        || span < degree
        || span >= n
        || !tolerance.is_finite()
        || tolerance <= 0.
        || tolerance >= 1.
    {
        return Err(InputError::InvalidInput(
            "Invalid original plane root input",
        ));
    }
    let source = controls
        .iter()
        .flatten()
        .chain(knots)
        .copied()
        .chain(plane)
        .map(|r| ctx.resolve(r).cloned())
        .collect::<Result<Vec<_>, _>>()?;
    let values = source
        .iter()
        .map(|v| match v {
            AuthoredScalar::Binary64Bits(bits) => Some(f64::from_bits(*bits)),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()
        .ok_or(InputError::InvalidInput(
            "Original plane roots require binary64 leaves",
        ))?;
    let tail = n * 4 + knots.len();
    let uv = &values[n * 4..tail];
    if values[..n * 4].chunks_exact(4).any(|p| p[3] <= 0.)
        || uv.windows(2).any(|p| p[0] > p[1])
        || uv[degree] >= uv[n]
        || uv[span] >= uv[span + 1]
        || values[tail..tail + 3].iter().all(|v| *v == 0.)
    {
        return Err(InputError::InvalidInput(
            "Invalid source weights, knots, active span or plane",
        ));
    }
    let computed = (|| {
        ctx.charge(source.len() as u64)?;
        let x: Vec<_> = values.iter().map(|v| Expansion::scalar(*v)).collect();
        let u = &x[n * 4..tail];
        let bezier = uv[span - degree + 1] == uv[span] && uv[span + degree] == uv[span + 1];
        let polynomial = if bezier {
            let mut residuals = Vec::new();
            for i in span - degree..=span {
                let mut f = Expansion::scalar(0.).sub(&x[tail + 3], ctx)?;
                for axis in 0..3 {
                    f = f.add(&x[i * 4 + axis].mul(&x[tail + axis], ctx)?, ctx)?;
                }
                residuals.push(f.mul(&x[i * 4 + 3], ctx)?);
            }
            if residuals.iter().all(|v| v.sign() == Sign::Zero) {
                return Ok(OriginalPlaneRootsOutcome::Coincident);
            }
            if residuals[0].sign() != Sign::Zero {
                let mut geometric = true;
                for i in 2..residuals.len() {
                    if residuals[i]
                        .mul(&residuals[0], ctx)?
                        .sub(&residuals[i - 1].mul(&residuals[1], ctx)?, ctx)?
                        .sign()
                        != Sign::Zero
                    {
                        geometric = false;
                        break;
                    }
                }
                if geometric {
                    // Exact binomial identity: Hi=H0*r^i gives
                    // H0*((1-q)+r*q)^p, including non-midpoint contacts.
                    let linear = vec![residuals[0].clone(), residuals[1].sub(&residuals[0], ctx)?];
                    let roots = isolate(&linear, tolerance, ctx)?
                        .into_iter()
                        .map(|r| OriginalPlaneRoot {
                            interval: r.interval,
                            multiplicity: degree,
                        })
                        .collect();
                    return Ok(OriginalPlaneRootsOutcome::Roots(roots));
                }
            }
            power(&residuals, ctx)?
        } else {
            let h = series(&x[..n * 4], u, degree, span, &u[span], degree, ctx)?;
            let width = u[span + 1].sub(&u[span], ctx)?;
            let mut factor = Expansion::scalar(1.);
            let mut coefficients = Vec::new();
            for q in 0..=degree {
                let mut value = Rational::zero();
                for axis in 0..3 {
                    value = value.add(
                        &h[q][axis].mul(&Rational::scalar(x[tail + axis].clone()), ctx)?,
                        ctx,
                    )?;
                }
                value = value.sub(
                    &h[q][3].mul(&Rational::scalar(x[tail + 3].clone()), ctx)?,
                    ctx,
                )?;
                value.n = value.n.mul(&factor, ctx)?;
                if value.d.sign() != Sign::Positive {
                    return Err(Reason::MissingProof);
                }
                coefficients.push(value);
                if q < degree {
                    factor = factor.mul(&width, ctx)?;
                }
            }
            let mut polynomial = Vec::new();
            for i in 0..coefficients.len() {
                let mut value = coefficients[i].n.clone();
                if value.sign() != Sign::Zero {
                    for (j, c) in coefficients.iter().enumerate() {
                        if i != j {
                            value = value.mul(&c.d, ctx)?;
                        }
                    }
                }
                polynomial.push(value);
            }
            polynomial
        };
        if polynomial.iter().all(|v| v.sign() == Sign::Zero) {
            return Ok(OriginalPlaneRootsOutcome::Coincident);
        }
        let roots = isolate(&polynomial, tolerance, ctx)?
            .into_iter()
            .map(|r| OriginalPlaneRoot {
                interval: r.interval,
                multiplicity: r.multiplicity,
            })
            .collect();
        Ok(OriginalPlaneRootsOutcome::Roots(roots))
    })()
    .and_then(|v| ctx.charge(0).map(|_| v));
    Ok(OriginalPlaneRootsDecision {
        outcome: computed.unwrap_or_else(OriginalPlaneRootsOutcome::Indeterminate),
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Limits, SourceArena, ToleranceContext};
    fn query(
        z: &[f64],
        w: &[f64],
        knots: &[f64],
        p: usize,
        span: usize,
        tol: f64,
        budget: u64,
    ) -> OriginalPlaneRootsDecision {
        let mut values = Vec::new();
        for i in 0..z.len() {
            values.extend([i as f64, 0., z[i], w[i]]);
        }
        let start = values.len();
        values.extend(knots);
        let plane = values.len();
        values.extend([0., 0., 1., 0.]);
        let arena = SourceArena::authored(
            "original-span-root-test",
            1,
            values
                .into_iter()
                .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
                .collect(),
        )
        .unwrap();
        let controls: Vec<_> = (0..z.len())
            .map(|i| std::array::from_fn(|j| arena.leaf(4 * i + j).unwrap()))
            .collect();
        let knots: Vec<_> = (0..knots.len())
            .map(|i| arena.leaf(start + i).unwrap())
            .collect();
        let plane = std::array::from_fn(|i| arena.leaf(plane + i).unwrap());
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
        rational_bspline_plane_roots(&mut ctx, &controls, &knots, p, span, plane, tol).unwrap()
    }
    #[test]
    fn original_bezier_mixed_orders_are_complete_and_not_sampled() {
        let r = query(
            &[90., -120., 145., -150., 120., -80.],
            &[1.; 6],
            &[0., 0., 0., 0., 0., 0., 1., 1., 1., 1., 1., 1.],
            5,
            5,
            1e-7,
            1000000,
        );
        let OriginalPlaneRootsOutcome::Roots(roots) = r.outcome else {
            panic!("{r:?}")
        };
        assert_eq!(roots.len(), 2);
        assert_eq!(roots[0].multiplicity, 3);
        assert_eq!(roots[1].multiplicity, 2);
        assert!(roots[0].interval[0] <= 1. / 3. && 1. / 3. <= roots[0].interval[1]);
        assert_eq!(roots[1].interval, [0.75, 0.75]);
    }
    #[test]
    fn original_nonbezier_span_and_degree_25_contact_keep_true_orders() {
        let r = query(
            &[8., 0., -2., 56.],
            &[1.; 4],
            &[-3., -2., -1., 0., 1., 2., 3., 4.],
            3,
            3,
            1e-7,
            1000000,
        );
        let OriginalPlaneRootsOutcome::Roots(roots) = r.outcome else {
            panic!("{r:?}")
        };
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].multiplicity, 2);
        assert!(roots[0].interval[0] <= 1. / 3. && 1. / 3. <= roots[0].interval[1]);
        let z: Vec<_> = (0..=25)
            .map(|i| if i % 2 == 0 { 1. } else { -1. })
            .collect();
        let w: Vec<_> = (0..=25).map(|i| 2_f64.powi(i)).collect();
        let mut knots = vec![0.; 26];
        knots.extend(vec![1.; 26]);
        let r = query(&z, &w, &knots, 25, 25, 1e-7, 1000000);
        let OriginalPlaneRootsOutcome::Roots(roots) = r.outcome else {
            panic!("{r:?}")
        };
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].multiplicity, 25);
    }
    #[test]
    fn scalar_isolation_separates_closed_endpoint_root_and_close_interior_root() {
        // 10*q*(q-e), Bernstein degree 2 coefficients [0,-5e,10-10e].
        let e = 2_f64.powi(-20);
        let r = query(
            &[0., -5. * e, 10. - 10. * e],
            &[1.; 3],
            &[0., 0., 0., 1., 1., 1.],
            2,
            2,
            0.01,
            1000000,
        );
        let OriginalPlaneRootsOutcome::Roots(roots) = r.outcome else {
            panic!("{r:?}")
        };
        assert_eq!(roots.len(), 2);
        assert_eq!(roots[0].interval, [0., 0.]);
        assert!(
            roots[1].interval[0] > 0. && roots[1].interval[0] <= e && e <= roots[1].interval[1]
        );
        let r = query(
            &[1., -2., 4.],
            &[1.; 3],
            &[0., 0., 0., 1., 1., 1.],
            2,
            2,
            1e-7,
            0,
        );
        assert!(matches!(
            r.outcome,
            OriginalPlaneRootsOutcome::Indeterminate(Reason::ResourceLimit)
        ));
    }
}
