//! Exact absolute curvature bound of an ORIGINAL positive-weight XYZ Bezier.
//! No chart joining, boundary contact, material or solid authority is inferred.
use crate::natural_meridian_g2::algebra::{P, add, choose, derivative, mul, power};
use crate::{
    Algebra, AuthoredScalar, ContextIdentity, Expansion as E, InputError, LeafRef,
    PredicateContext, Reason, Sign,
};
mod filter;
mod symmetric_quartic;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CurvatureBoundOutcome {
    Within,
    Exceeds,
    Indeterminate(Reason),
}
#[derive(Clone, Debug)]
pub struct CurvatureBoundDecision {
    pub outcome: CurvatureBoundOutcome,
    pub regularity_proven: bool,
    pub cells: usize,
    pub work_used: u64,
    pub witness_parameter: Option<f64>,
    pub context: ContextIdentity,
}
fn square(p: &P, ctx: &mut PredicateContext<'_>) -> Result<P, Reason> {
    mul(p, p, ctx)
}
fn monotone_regular(raw: &[AuthoredScalar]) -> bool {
    let points = raw[..raw.len() - 1]
        .chunks_exact(4)
        .map(|p| {
            std::array::from_fn::<_, 3, _>(|k| match p[k] {
                AuthoredScalar::Binary64Bits(b) => f64::from_bits(b),
                _ => unreachable!(),
            })
        })
        .collect::<Vec<_>>();
    let n = points.len();
    if points[0] == points[1] || points[n - 1] == points[n - 2] {
        return false;
    }
    // For positive weights, the rational Bernstein derivative has the sign
    // of Cov(P_i,i). A nonconstant ordered coordinate gives a strictly signed
    // derivative on (0,1). Noncoincident first/last pole pairs separately give
    // nonzero endpoint derivatives. These are ORIGINAL binary64 comparisons,
    // with no rounded difference or sampled regularity claim.
    (0..3).any(|k| {
        points[0][k] != points[n - 1][k]
            && (points.windows(2).all(|p| p[0][k] <= p[1][k])
                || points.windows(2).all(|p| p[0][k] >= p[1][k]))
    })
}
fn bernstein(p: &P, ctx: &mut PredicateContext<'_>) -> Result<P, Reason> {
    let n = p.len() - 1;
    if n > 54 {
        return Err(Reason::MissingProof);
    }
    // A single positive n! clears EVERY power-to-Bernstein denominator.
    // No rounded division or independently rescaled coefficient is used.
    let mut factorial = vec![E::scalar(1.)];
    for i in 1..=n {
        factorial.push(factorial[i - 1].mul(&E::integer(i as u64, ctx)?, ctx)?);
    }
    // A common exact binary scale keeps the factorial-cleared coefficients
    // representable. It never rescales individual signs independently.
    let top = factorial[n].enclosure(ctx)?.hi;
    let exponent = ((top.to_bits() >> 52) & 0x7ff) as i32 - 1023;
    let shift = (exponent - 16).max(0);
    let common = E::scalar(f64::from_bits(((1023 - shift) as u64) << 52));
    let mut out = Vec::new();
    for k in 0..=n {
        let mut value = E::scalar(0.);
        for (i, a) in p.iter().enumerate().take(k + 1) {
            let factor = factorial[i]
                .mul(&common, ctx)?
                .mul(&factorial[n - i], ctx)?
                .mul(&E::integer(choose(k, i), ctx)?, ctx)?;
            value = value.add(&a.mul(&factor, ctx)?, ctx)?;
        }
        out.push(value);
    }
    Ok(out)
}
fn split(p: &P, ctx: &mut PredicateContext<'_>) -> Result<(P, P), Reason> {
    let mut row = p.clone();
    let mut a = vec![row[0].clone()];
    let mut b = vec![row.last().unwrap().clone()];
    for n in (1..p.len()).rev() {
        for i in 0..n {
            row[i] = row[i].add(&row[i + 1], ctx)?.mul(&E::scalar(0.5), ctx)?;
        }
        a.push(row[0].clone());
        b.push(row[n - 1].clone());
    }
    b.reverse();
    Ok((a, b))
}
/// Curvature is tested by K² (N·N)³ - W⁴ |N×N'|² >= 0,
/// where N = H'W-HW'. Positive authored weights prove W>0; a separate
/// positive Bernstein speed numerator proves regularity on the CLOSED curve.
/// Input controls are Cartesian XYZ plus weight, degrees 1..5. Every positive
/// result covers [0,1]; exact dyadic point signs may only establish rejection.
pub fn rational_bezier_curvature_bound(
    ctx: &mut PredicateContext<'_>,
    controls: &[[LeafRef; 4]],
    maximum: LeafRef,
    max_cells: usize,
) -> Result<CurvatureBoundDecision, InputError> {
    if !(2..=6).contains(&controls.len()) || max_cells > 4096 {
        return Err(InputError::InvalidInput(
            "Choose Bezier degree 1..5 and at most 4096 curvature cells",
        ));
    }
    let raw = controls
        .iter()
        .flatten()
        .chain(std::iter::once(&maximum))
        .map(|&v| ctx.resolve(v).cloned())
        .collect::<Result<Vec<_>, _>>()?;
    if raw
        .iter()
        .any(|v| !matches!(v, AuthoredScalar::Binary64Bits(_)))
    {
        return Err(InputError::InvalidInput(
            "Curvature needs original binary64 control and limit leaves",
        ));
    }
    let mut regular = false;
    let mut cells = 0;
    let mut witness = None;
    let outcome = (|| -> Result<CurvatureBoundOutcome, Reason> {
        ctx.charge(raw.len() as u64)?;
        if max_cells == 0 {
            return Err(Reason::ResourceLimit);
        }
        let values = raw
            .iter()
            .map(|v| match v {
                AuthoredScalar::Binary64Bits(b) => E::scalar(f64::from_bits(*b)),
                _ => unreachable!(),
            })
            .collect::<Vec<_>>();
        let k = values.last().unwrap();
        if k.sign() != Sign::Positive {
            return Err(Reason::MissingProof);
        }
        let mut h: [P; 4] = std::array::from_fn(|_| Vec::new());
        for point in values[..values.len() - 1].chunks_exact(4) {
            if point[3].sign() != Sign::Positive {
                return Err(Reason::MissingProof);
            }
            for axis in 0..3 {
                h[axis].push(point[axis].sub(&values[axis], ctx)?.mul(&point[3], ctx)?);
            }
            h[3].push(point[3].clone());
        }
        ctx.charge(3 * controls.len() as u64)?;
        let monotone = monotone_regular(&raw);
        if monotone {
            regular = true;
            if let Some(outcome) =
                symmetric_quartic::inspect(&values, max_cells, &mut cells, &mut witness, ctx)?
            {
                return Ok(outcome);
            }
        }
        // One common homogeneous gauge is removed/recentered before the
        // degree-12 curvature inequality. Source geometry and K are unchanged.
        let mut all = h.iter().flatten().cloned().collect::<Vec<_>>();
        E::reduce_projective(&mut all, ctx)?;
        E::normalize_common(&mut all, ctx)?;
        let gauge = E::scalar(f64::from_bits(((1023 + 32) as u64) << 52));
        let mut values = all.into_iter();
        for component in &mut h {
            for x in component {
                *x = values.next().unwrap().mul(&gauge, ctx)?;
            }
        }
        let w = power(&h[3], ctx)?;
        let wd = derivative(&w, ctx)?;
        let mut n: [P; 3] = std::array::from_fn(|_| Vec::new());
        for axis in 0..3 {
            let x = power(&h[axis], ctx)?;
            n[axis] = add(
                &mul(&derivative(&x, ctx)?, &w, ctx)?,
                &mul(&x, &wd, ctx)?,
                true,
                ctx,
            )?;
        }
        if monotone {
            regular = true;
            if filter::within(k, &w, &n, max_cells, &mut cells, ctx)? {
                return Ok(CurvatureBoundOutcome::Within);
            }
        }
        let mut speed = vec![E::scalar(0.)];
        for p in &n {
            speed = add(&speed, &square(p, ctx)?, false, ctx)?;
        }
        if !monotone {
            let sb = bernstein(&speed, ctx)?;
            if sb.first().unwrap().sign() != Sign::Positive
                || sb.last().unwrap().sign() != Sign::Positive
                || sb.iter().any(|x| x.sign() == Sign::Negative)
            {
                return Err(Reason::MissingProof);
            }
            regular = true;
            if filter::within(k, &w, &n, max_cells, &mut cells, ctx)? {
                return Ok(CurvatureBoundOutcome::Within);
            }
        }
        let nd = [
            derivative(&n[0], ctx)?,
            derivative(&n[1], ctx)?,
            derivative(&n[2], ctx)?,
        ];
        // Cheap exact ORIGINAL point witnesses before expanding the full
        // degree-54 inequality. Positive point signs grant no admission.
        for t in [0., 0.5, 1.] {
            if cells >= max_cells {
                return Err(Reason::ResourceLimit);
            }
            cells += 1;
            let evaluate = |p: &P, c: &mut PredicateContext<'_>| -> Result<E, Reason> {
                let mut value = E::scalar(0.);
                for coefficient in p.iter().rev() {
                    value = value.mul(&E::scalar(t), c)?.add(coefficient, c)?;
                }
                Ok(value)
            };
            let v = [
                evaluate(&n[0], ctx)?,
                evaluate(&n[1], ctx)?,
                evaluate(&n[2], ctx)?,
            ];
            let d = [
                evaluate(&nd[0], ctx)?,
                evaluate(&nd[1], ctx)?,
                evaluate(&nd[2], ctx)?,
            ];
            let mut ss = E::scalar(0.);
            let mut cc = E::scalar(0.);
            for i in 0..3 {
                ss = ss.add(&v[i].mul(&v[i], ctx)?, ctx)?;
                let a = (i + 1) % 3;
                let b = (i + 2) % 3;
                let cross = v[a].mul(&d[b], ctx)?.sub(&v[b].mul(&d[a], ctx)?, ctx)?;
                cc = cc.add(&cross.mul(&cross, ctx)?, ctx)?;
            }
            let ww = evaluate(&w, ctx)?;
            let w2 = ww.mul(&ww, ctx)?;
            let lhs = k.mul(k, ctx)?.mul(&ss.mul(&ss, ctx)?.mul(&ss, ctx)?, ctx)?;
            let rhs = w2.mul(&w2, ctx)?.mul(&cc, ctx)?;
            if lhs.sub(&rhs, ctx)?.sign() == Sign::Negative {
                witness = Some(t);
                return Ok(CurvatureBoundOutcome::Exceeds);
            }
        }
        let mut cross2 = vec![E::scalar(0.)];
        for axis in 0..3 {
            let a = (axis + 1) % 3;
            let b = (axis + 2) % 3;
            let cross = add(
                &mul(&n[a], &nd[b], ctx)?,
                &mul(&n[b], &nd[a], ctx)?,
                true,
                ctx,
            )?;
            cross2 = add(&cross2, &square(&cross, ctx)?, false, ctx)?;
        }
        let lhs = mul(
            &vec![k.mul(k, ctx)?],
            &mul(&square(&speed, ctx)?, &speed, ctx)?,
            ctx,
        )?;
        let rhs = mul(&square(&square(&w, ctx)?, ctx)?, &cross2, ctx)?;
        let q = bernstein(&add(&lhs, &rhs, true, ctx)?, ctx)?;
        let mut stack = vec![(q, [0., 1.], 0usize)];
        while let Some((p, t, depth)) = stack.pop() {
            if cells >= max_cells {
                return Err(Reason::ResourceLimit);
            }
            cells += 1;
            ctx.charge(p.len() as u64)?;
            if p.iter().all(|x| x.sign() != Sign::Negative) {
                continue;
            }
            if p[0].sign() == Sign::Negative {
                witness = Some(t[0]);
                return Ok(CurvatureBoundOutcome::Exceeds);
            }
            if p.last().unwrap().sign() == Sign::Negative {
                witness = Some(t[1]);
                return Ok(CurvatureBoundOutcome::Exceeds);
            }
            if depth == 24 {
                return Err(Reason::MissingProof);
            }
            let (a, b) = split(&p, ctx)?;
            let mid = t[0] * 0.5 + t[1] * 0.5;
            if a.last().unwrap().sign() == Sign::Negative {
                witness = Some(mid);
                return Ok(CurvatureBoundOutcome::Exceeds);
            }
            stack.push((b, [mid, t[1]], depth + 1));
            stack.push((a, [t[0], mid], depth + 1));
        }
        Ok(CurvatureBoundOutcome::Within)
    })()
    .and_then(|v| ctx.charge(0).map(|_| v));
    Ok(CurvatureBoundDecision {
        outcome: outcome.unwrap_or_else(CurvatureBoundOutcome::Indeterminate),
        regularity_proven: regular,
        cells,
        work_used: ctx.work_used(),
        witness_parameter: witness,
        context: ctx.identity(),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Limits, SourceArena, ToleranceContext};
    fn run(points: &[[f64; 4]], limit: f64, cells: usize, work: u64) -> CurvatureBoundDecision {
        let mut raw = points.iter().flatten().copied().collect::<Vec<_>>();
        raw.push(limit);
        let arena = SourceArena::authored(
            "original-Bezier-curvature",
            1,
            raw.into_iter()
                .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
                .collect(),
        )
        .unwrap();
        let c = (0..points.len())
            .map(|i| std::array::from_fn(|k| arena.leaf(4 * i + k).unwrap()))
            .collect::<Vec<_>>();
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(
            &arena,
            &tolerance,
            Limits {
                max_work: work,
                ..Default::default()
            },
            None,
        );
        rational_bezier_curvature_bound(&mut ctx, &c, arena.leaf(4 * points.len()).unwrap(), cells)
            .unwrap()
    }
    #[test]
    fn an_exact_rational_circle_qualifies_on_the_limit_in_different_gauges() {
        for gauge in [1e-12, 1., 1e12] {
            let c = [
                [0., 1., 0., gauge],
                [0., 0., 0., gauge],
                [1., 0., 0., 2. * gauge],
            ];
            let r = run(&c, 1., 4096, 1000000);
            assert_eq!(r.outcome, CurvatureBoundOutcome::Within, "{r:?}");
            assert!(r.regularity_proven);
            assert_eq!(
                run(&c, 0.5, 4096, 1000000).outcome,
                CurvatureBoundOutcome::Exceeds
            );
        }
    }
    #[test]
    fn complete_parabolic_bound_and_inside_violation_are_distinct() {
        let c = [[0., 0., 0., 1.], [0.5, 0., 0., 1.], [1., 1., 0., 1.]];
        assert_eq!(
            run(&c, 2., 4096, 1000000).outcome,
            CurvatureBoundOutcome::Within
        );
        let r = run(&c, 1., 4096, 1000000);
        assert_eq!(r.outcome, CurvatureBoundOutcome::Exceeds);
        assert_eq!(r.witness_parameter, Some(0.));
        let degenerate = [[1., 1., 1., 1.]; 3];
        assert!(matches!(
            run(&degenerate, 10., 4096, 1000000).outcome,
            CurvatureBoundOutcome::Indeterminate(_)
        ));
    }
    #[test]
    fn exact_endpoint_witnesses_share_the_cell_budget() {
        let c = [[1., 1., 0., 1.], [0.5, 0., 0., 1.], [0., 0., 0., 1.]];
        let r = run(&c, 1., 4, 1000000);
        assert_eq!(r.outcome, CurvatureBoundOutcome::Exceeds, "{r:?}");
        assert_eq!(r.witness_parameter, Some(1.));
        assert_eq!(r.cells, 4);
        let short = run(&c, 1., 3, 1000000);
        assert_eq!(
            short.outcome,
            CurvatureBoundOutcome::Indeterminate(Reason::ResourceLimit)
        );
        assert!(short.cells <= 3);
    }
    #[test]
    fn endpoint_stationarity_and_interior_reversal_are_not_monotone_proofs() {
        for c in [
            [[0., 0., 0., 1.], [0., 0., 0., 1.], [1., 0., 0., 1.]],
            [[0., 0., 0., 1.], [1., 0., 0., 1.], [0., 0., 0., 1.]],
        ] {
            let r = run(&c, 10., 4096, 1000000);
            assert!(!r.regularity_proven, "{r:?}");
            assert!(
                matches!(r.outcome, CurvatureBoundOutcome::Indeterminate(_)),
                "{r:?}"
            );
        }
    }
    #[test]
    fn exhausted_work_or_cells_never_grants_a_curvature_certificate() {
        let c = [[0., 1., 0., 1.], [0., 0., 0., 1.], [1., 0., 0., 2.]];
        let r = run(&c, 1., 4096, 1000000);
        assert_eq!(r.outcome, CurvatureBoundOutcome::Within);
        for work in [0, 1, r.work_used - 1] {
            let q = run(&c, 1., 4096, work);
            assert_eq!(
                q.outcome,
                CurvatureBoundOutcome::Indeterminate(Reason::ResourceLimit)
            );
            assert!(q.work_used <= work);
        }
        assert_eq!(
            run(&c, 1., 0, 1000000).outcome,
            CurvatureBoundOutcome::Indeterminate(Reason::ResourceLimit)
        );
    }
    #[test]
    fn an_interior_violation_cannot_hide_behind_safe_endpoints() {
        // Endpoint curvature sqrt(1/8), midpoint curvature 1. The endpoint
        // signs alone are therefore insufficient at a limit of 3/4.
        let c = [[-1., 0.5, 0., 1.], [0., -0.5, 0., 1.], [1., 0.5, 0., 1.]];
        let r = run(&c, 0.75, 4096, 1000000);
        assert_eq!(r.outcome, CurvatureBoundOutcome::Exceeds, "{r:?}");
        assert_eq!(r.witness_parameter, Some(0.5));
        assert!(r.regularity_proven);
        let complete = run(&c, 1.01, 4096, 1000000);
        assert_eq!(
            complete.outcome,
            CurvatureBoundOutcome::Within,
            "{complete:?}"
        );
        assert!(
            complete.cells > 1,
            "This positive field needs an interval cover"
        );
        let short = run(&c, 1.01, complete.cells - 1, 1000000);
        assert!(
            matches!(short.outcome, CurvatureBoundOutcome::Indeterminate(_)),
            "{short:?}"
        );
    }
}
