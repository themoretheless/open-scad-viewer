//! Plane expressions assembled from original affine-patch source leaves.
use super::*;
/// Isolate original B-spline roots against the exact plane of four authored
/// patch vertices [00,10,01,11]. The parallelogram identity is proved exactly.
/// No rounded normal/offset is introduced. Roots refer to the infinite plane;
/// finite patch membership and surface/solid admission are separate proofs.
pub fn rational_bspline_affine_patch_roots(
    ctx: &mut PredicateContext<'_>,
    controls: &[[LeafRef; 4]],
    knots: &[LeafRef],
    degree: usize,
    span: usize,
    patch: [[LeafRef; 3]; 4],
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
        return Err(InputError::InvalidInput("Invalid affine patch root input"));
    }
    let refs: Vec<_> = controls
        .iter()
        .flatten()
        .chain(knots)
        .copied()
        .chain(patch.into_iter().flatten())
        .collect();
    let values = refs
        .iter()
        .map(|&r| match ctx.resolve(r)? {
            AuthoredScalar::Binary64Bits(b) => Ok(f64::from_bits(*b)),
            _ => Err(InputError::InvalidInput(
                "Patch roots require original binary64 leaves",
            )),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let tail = n * 4 + knots.len();
    let uv = &values[n * 4..tail];
    if values[..n * 4].chunks_exact(4).any(|p| p[3] <= 0.)
        || uv.windows(2).any(|p| p[0] > p[1])
        || uv[degree] >= uv[n]
        || uv[span] >= uv[span + 1]
    {
        return Err(InputError::InvalidInput("Invalid original curve source"));
    }
    let computed = (|| {
        ctx.charge(values.len() as u64)?;
        let x: Vec<_> = values.iter().map(|v| Expansion::scalar(*v)).collect();
        let mut u = Vec::new();
        let mut v = Vec::new();
        for k in 0..3 {
            u.push(x[tail + 3 + k].sub(&x[tail + k], ctx)?);
            v.push(x[tail + 6 + k].sub(&x[tail + k], ctx)?);
            if x[tail + 9 + k]
                .sub(&x[tail + 3 + k], ctx)?
                .sub(&v[k], ctx)?
                .sign()
                != Sign::Zero
            {
                return Err(Reason::MissingProof);
            }
        }
        let mut plane = std::array::from_fn(|_| Expansion::scalar(0.));
        for k in 0..3 {
            plane[k] = u[(k + 1) % 3]
                .mul(&v[(k + 2) % 3], ctx)?
                .sub(&u[(k + 2) % 3].mul(&v[(k + 1) % 3], ctx)?, ctx)?;
        }
        if plane[..3].iter().all(|n| n.sign() == Sign::Zero) {
            return Err(Reason::MissingProof);
        }
        for k in 0..3 {
            plane[3] = plane[3].add(&plane[k].mul(&x[tail + k], ctx)?, ctx)?;
        }
        solve::solve(
            ctx,
            ExactSpan {
                x: &x[..tail],
                uv,
                n,
                degree,
                span,
            },
            &plane,
            tolerance,
        )
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
        knots: &[f64],
        degree: usize,
        span: usize,
        budget: u64,
        perturb: bool,
    ) -> OriginalPlaneRootsDecision {
        let mut values = vec![];
        for (i, &z) in z.iter().enumerate() {
            values.extend([0.1, i as f64 / 3., z, 1.]);
        }
        let k = values.len();
        values.extend(knots);
        let p = values.len();
        // The normal contains exact 1.1-0.1, whose primitive mantissa is
        // not binary64-representable. Its expression stays in expansions.
        values.extend([
            0.1,
            0.,
            0.,
            1.1,
            0.,
            0.1,
            0.1,
            1.,
            0.,
            if perturb { 1.1f64.next_up() } else { 1.1 },
            1.,
            0.1,
        ]);
        let arena = SourceArena::authored(
            "original-affine-patch-roots",
            1,
            values
                .into_iter()
                .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
                .collect(),
        )
        .unwrap();
        let controls: Vec<_> = (0..z.len())
            .map(|i| std::array::from_fn(|j| arena.leaf(i * 4 + j).unwrap()))
            .collect();
        let ks: Vec<_> = (0..knots.len())
            .map(|i| arena.leaf(k + i).unwrap())
            .collect();
        let patch =
            std::array::from_fn(|i| std::array::from_fn(|j| arena.leaf(p + i * 3 + j).unwrap()));
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
        rational_bspline_affine_patch_roots(&mut ctx, &controls, &ks, degree, span, patch, 1e-8)
            .unwrap()
    }
    #[test]
    fn nonrepresentable_original_plane_has_a_proved_double_root() {
        let r = query(
            &[0.25, -0.25, 0.25],
            &[0., 0., 0., 1., 1., 1.],
            2,
            2,
            100000,
            false,
        );
        let OriginalPlaneRootsOutcome::Roots(roots) = r.outcome else {
            panic!("{r:?}")
        };
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].multiplicity, 2);
        assert_eq!(roots[0].interval, [0.5, 0.5]);
    }
    #[test]
    fn original_nonbezier_span_keeps_the_double_root_order() {
        let r = query(
            &[8., 0., -2., 56.],
            &[-3., -2., -1., 0., 1., 2., 3., 4.],
            3,
            3,
            1000000,
            false,
        );
        let OriginalPlaneRootsOutcome::Roots(roots) = r.outcome else {
            panic!("{r:?}")
        };
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].multiplicity, 2);
        assert!(roots[0].interval[0] <= 1. / 3. && 1. / 3. <= roots[0].interval[1]);
    }
    #[test]
    fn missing_affine_identity_and_exhausted_work_never_admit_roots() {
        let z = [0.25, -0.25, 0.25];
        let knots = [0., 0., 0., 1., 1., 1.];
        assert!(matches!(
            query(&z, &knots, 2, 2, 100000, true).outcome,
            OriginalPlaneRootsOutcome::Indeterminate(Reason::MissingProof)
        ));
        assert!(matches!(
            query(&z, &knots, 2, 2, 0, false).outcome,
            OriginalPlaneRootsOutcome::Indeterminate(Reason::ResourceLimit)
        ));
    }
}
