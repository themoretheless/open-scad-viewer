//! Exact Cartesian one-sided C1..C4 jets at original B-spline knots.
//! Formal de Boor series use rational expansions of original source leaves.
//! No floating-point knot insertion, derivative evaluation or sampling.
use crate::{
    exact_inputs, Algebra, AuthoredScalar, BezierIdentity, BezierIdentityDecision, Expansion,
    InputError, LeafRef, PredicateContext, Reason, Sign,
};
#[derive(Clone)]
struct Rational {
    n: Expansion,
    d: Expansion,
}
impl Rational {
    fn scalar(n: Expansion) -> Self {
        Self {
            n,
            d: Expansion::scalar(1.),
        }
    }
    fn zero() -> Self {
        Self::scalar(Expansion::scalar(0.))
    }
    fn add(&self, b: &Self, ctx: &mut PredicateContext<'_>) -> Result<Self, Reason> {
        if self.d.sub(&b.d, ctx)?.sign() == Sign::Zero {
            return Ok(Self {
                n: self.n.add(&b.n, ctx)?,
                d: self.d.clone(),
            });
        }
        Ok(Self {
            n: self.n.mul(&b.d, ctx)?.add(&b.n.mul(&self.d, ctx)?, ctx)?,
            d: self.d.mul(&b.d, ctx)?,
        })
    }
    fn sub(&self, b: &Self, ctx: &mut PredicateContext<'_>) -> Result<Self, Reason> {
        self.add(
            &Self {
                n: Expansion::scalar(0.).sub(&b.n, ctx)?,
                d: b.d.clone(),
            },
            ctx,
        )
    }
    fn mul(&self, b: &Self, ctx: &mut PredicateContext<'_>) -> Result<Self, Reason> {
        if self.n.sign() == Sign::Zero || b.n.sign() == Sign::Zero {
            ctx.charge(1)?;
            return Ok(Self::zero());
        }
        Ok(Self {
            n: self.n.mul(&b.n, ctx)?,
            d: self.d.mul(&b.d, ctx)?,
        })
    }
    fn div_positive(&self, b: &Self, ctx: &mut PredicateContext<'_>) -> Result<Self, Reason> {
        if b.n.sign() != Sign::Positive || b.d.sign() != Sign::Positive {
            return Err(Reason::MissingProof);
        }
        if self.n.sign() == Sign::Zero {
            ctx.charge(1)?;
            return Ok(Self::zero());
        }
        if self.d.sub(&b.d, ctx)?.sign() == Sign::Zero {
            return Ok(Self {
                n: self.n.clone(),
                d: b.n.clone(),
            });
        }
        Ok(Self {
            n: self.n.mul(&b.d, ctx)?,
            d: self.d.mul(&b.n, ctx)?,
        })
    }
    fn equal(&self, b: &Self, ctx: &mut PredicateContext<'_>) -> Result<bool, Reason> {
        Ok(self
            .n
            .mul(&b.d, ctx)?
            .sub(&b.n.mul(&self.d, ctx)?, ctx)?
            .sign()
            == Sign::Zero)
    }
}
type Series = Vec<[Rational; 4]>;
fn series(
    x: &[Expansion],
    u: &[Expansion],
    p: usize,
    span: usize,
    t: &Expansion,
    order: usize,
    ctx: &mut PredicateContext<'_>,
) -> Result<Series, Reason> {
    let mut rows = Vec::with_capacity(p + 1);
    for j in 0..=p {
        let i = span - p + j;
        let mut row = (0..=order)
            .map(|_| std::array::from_fn(|_| Rational::zero()))
            .collect::<Series>();
        for k in 0..4 {
            row[0][k] = Rational::scalar(if k == 3 {
                x[i * 4 + 3].clone()
            } else {
                x[i * 4 + k].mul(&x[i * 4 + 3], ctx)?
            });
        }
        rows.push(row);
    }
    for r in 1..=p {
        for j in (r..=p).rev() {
            let at = span - p + j;
            let den = Rational::scalar(u[span + 1 + j - r].sub(&u[at], ctx)?);
            let alpha = Rational::scalar(t.sub(&u[at], ctx)?).div_positive(&den, ctx)?;
            let beta = Rational::scalar(Expansion::scalar(1.)).sub(&alpha, ctx)?;
            let linear = Rational::scalar(Expansion::scalar(1.)).div_positive(&den, ctx)?;
            let a = &rows[j - 1];
            let b = &rows[j];
            let mut out = (0..=order)
                .map(|_| std::array::from_fn(|_| Rational::zero()))
                .collect::<Series>();
            for q in 0..=order {
                for k in 0..4 {
                    out[q][k] = a[q][k]
                        .mul(&beta, ctx)?
                        .add(&b[q][k].mul(&alpha, ctx)?, ctx)?;
                    if q > 0 {
                        out[q][k] = out[q][k]
                            .add(&b[q - 1][k].sub(&a[q - 1][k], ctx)?.mul(&linear, ctx)?, ctx)?;
                    }
                }
            }
            rows[j] = out;
        }
    }
    Ok(rows.pop().unwrap())
}
fn cartesian(
    h: &Series,
    order: usize,
    ctx: &mut PredicateContext<'_>,
) -> Result<Vec<[Rational; 3]>, Reason> {
    let mut out = (0..=order)
        .map(|_| std::array::from_fn(|_| Rational::zero()))
        .collect::<Vec<_>>();
    for q in 0..=order {
        for k in 0..3 {
            let mut value = h[q][k].clone();
            for i in 1..=q {
                value = value.sub(&h[i][3].mul(&out[q - i][k], ctx)?, ctx)?;
            }
            out[q][k] = value.div_positive(&h[0][3], ctx)?;
        }
    }
    Ok(out)
}
fn bspline_jet_identity(
    ctx: &mut PredicateContext<'_>,
    controls: &[[LeafRef; 4]],
    knots: &[LeafRef],
    degree: usize,
    order: usize,
    endpoints: bool,
) -> Result<BezierIdentityDecision, InputError> {
    if !(1..=32).contains(&degree)
        || !(1..=4).contains(&order)
        || controls.len() < degree + 1
        || knots.len() != controls.len() + degree + 1
    {
        return Err(InputError::InvalidInput("Invalid B-spline internal jets"));
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
        let n = controls.len();
        if x[..n * 4]
            .chunks_exact(4)
            .any(|h| h[3].sign() != Sign::Positive)
            || u[n].sub(&u[degree], ctx)?.sign() != Sign::Positive
        {
            return Ok(BezierIdentity::Indeterminate(Reason::MissingProof));
        }
        for pair in u.windows(2) {
            if pair[1].sub(&pair[0], ctx)?.sign() == Sign::Negative {
                return Ok(BezierIdentity::Indeterminate(Reason::MissingProof));
            }
        }
        if endpoints {
            let mut first = degree;
            while first < n && u[first + 1].sub(&u[first], ctx)?.sign() == Sign::Zero {
                first += 1;
            }
            let mut last = n - 1;
            while last > degree && u[last + 1].sub(&u[last], ctx)?.sign() == Sign::Zero {
                last -= 1;
            }
            if first > last {
                return Ok(BezierIdentity::Indeterminate(Reason::MissingProof));
            }
            let start = cartesian(
                &series(&x, u, degree, first, &u[degree], order, ctx)?,
                order,
                ctx,
            )?;
            let end = cartesian(&series(&x, u, degree, last, &u[n], order, ctx)?, order, ctx)?;
            for q in 0..=order {
                for k in 0..3 {
                    if !start[q][k].equal(&end[q][k], ctx)? {
                        return Ok(BezierIdentity::Different);
                    }
                }
            }
            return Ok(BezierIdentity::Equal);
        }
        let mut span = degree;
        while span < n {
            if u[span + 1].sub(&u[span], ctx)?.sign() != Sign::Positive {
                span += 1;
                continue;
            }
            let mut right = span + 1;
            while right < n && u[right + 1].sub(&u[right], ctx)?.sign() == Sign::Zero {
                right += 1;
            }
            if right >= n {
                break;
            }
            // An original degree-p B-spline basis is C^(p-m) at a knot of
            // multiplicity m. Positive weights, checked above in exact
            // arithmetic, keep its rational denominator positive. Therefore
            // the Cartesian quotient inherits those jets for every authored
            // control net; no endpoint expansion or rounded proxy is needed.
            // Knot comparisons and source validation remain charged to ctx.
            if degree.saturating_sub(right - span) >= order {
                span = right;
                continue;
            }
            let t = &u[span + 1];
            let left = cartesian(&series(&x, u, degree, span, t, order, ctx)?, order, ctx)?;
            let next = cartesian(&series(&x, u, degree, right, t, order, ctx)?, order, ctx)?;
            for q in 0..=order {
                for k in 0..3 {
                    if !left[q][k].equal(&next[q][k], ctx)? {
                        return Ok(BezierIdentity::Different);
                    }
                }
            }
            span = right;
        }
        Ok(BezierIdentity::Equal)
    })();
    Ok(BezierIdentityDecision {
        outcome: result.unwrap_or_else(BezierIdentity::Indeterminate),
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}

/// Exact Cartesian C1..C4 at every original internal knot.
pub fn rational_bspline_internal_jet_identity(
    ctx: &mut PredicateContext<'_>,
    controls: &[[LeafRef; 4]],
    knots: &[LeafRef],
    degree: usize,
    order: usize,
) -> Result<BezierIdentityDecision, InputError> {
    bspline_jet_identity(ctx, controls, knots, degree, order, false)
}
/// Exact Cartesian endpoint jets on the active original knot domain, including
/// nonclamped bases and independently varying endpoint homogeneous scales.
/// Does not prove internal joins, regularity, periodic construction or Solid.
pub fn rational_bspline_cartesian_endpoint_jet_identity(
    ctx: &mut PredicateContext<'_>,
    controls: &[[LeafRef; 4]],
    knots: &[LeafRef],
    degree: usize,
    order: usize,
) -> Result<BezierIdentityDecision, InputError> {
    bspline_jet_identity(ctx, controls, knots, degree, order, true)
}
#[cfg(test)]
#[path = "tests/bspline_internal_jets.rs"]
mod tests;

// A nonzero common velocity component is a smooth direction chart. For
// arc length, first chart jets use the positive endpoint speed ratio;
// second jets use r_tt/Q-r_t(v.a)/Q². Every operation is rational.
fn direction_equal(
    l: &[[Rational; 3]],
    r: &[[Rational; 3]],
    order: usize,
    arc: bool,
    ctx: &mut PredicateContext<'_>,
) -> Result<bool, Reason> {
    let Some(axis) = (0..3).find(|&k| l[1][k].n.sign() != Sign::Zero) else {
        return Err(Reason::MissingProof);
    };
    let sign = match l[1][axis].n.sign() {
        Sign::Positive => 1.,
        Sign::Negative => -1.,
        _ => return Err(Reason::MissingProof),
    };
    let chart = |x: &[[Rational; 3]],
                 ctx: &mut PredicateContext<'_>|
     -> Result<(Vec<(Rational, Rational, Rational)>, Rational), Reason> {
        let c = Rational::scalar(Expansion::scalar(sign));
        let mut v = Vec::new();
        let mut a = Vec::new();
        let mut j = Vec::new();
        for k in 0..3 {
            v.push(x[1][k].mul(&c, ctx)?);
            a.push(x[2][k].mul(&Rational::scalar(Expansion::scalar(2. * sign)), ctx)?);
            j.push(if order == 2 {
                x[3][k].mul(&Rational::scalar(Expansion::scalar(6. * sign)), ctx)?
            } else {
                Rational::zero()
            });
        }
        if v[axis].n.sign() != Sign::Positive || v[axis].d.sign() != Sign::Positive {
            return Err(Reason::MissingProof);
        }
        let mut q = Rational::zero();
        let mut va = Rational::zero();
        for k in 0..3 {
            q = q.add(&v[k].mul(&v[k], ctx)?, ctx)?;
            va = va.add(&v[k].mul(&a[k], ctx)?, ctx)?;
        }
        let two = Rational::scalar(Expansion::scalar(2.));
        let mut out = Vec::new();
        for k in 0..3 {
            let z = v[k].div_positive(&v[axis], ctx)?;
            let d = a[k]
                .sub(&z.mul(&a[axis], ctx)?, ctx)?
                .div_positive(&v[axis], ctx)?;
            let dd = if order == 2 {
                j[k].sub(&z.mul(&j[axis], ctx)?, ctx)?
                    .sub(&two.mul(&d, ctx)?.mul(&a[axis], ctx)?, ctx)?
                    .div_positive(&v[axis], ctx)?
            } else {
                Rational::zero()
            };
            let dd = if arc && order == 2 {
                dd.div_positive(&q, ctx)?
                    .sub(&d.mul(&va, ctx)?.div_positive(&q.mul(&q, ctx)?, ctx)?, ctx)?
            } else {
                dd
            };
            out.push((z, d, dd));
        }
        Ok((out, v[axis].clone()))
    };
    let (lc, lv) = chart(l, ctx)?;
    let (rc, rv) = chart(r, ctx)?;
    let ratio = rv.div_positive(&lv, ctx)?;
    for k in 0..3 {
        if !lc[k].0.equal(&rc[k].0, ctx)? {
            return Ok(false);
        }
        let first = if arc {
            lc[k].1.mul(&ratio, ctx)?
        } else {
            lc[k].1.clone()
        };
        if !first.equal(&rc[k].1, ctx)? || (order == 2 && !lc[k].2.equal(&rc[k].2, ctx)?) {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Exact C1/C2 of a nonzero unit-direction field at original internal knots
/// or the active endpoints. Arc-length chart jets use positive speed ratios.
/// This does not prove whole-domain regularity, position continuity, surface
/// smoothness or closed transport holonomy; those remain caller premises.
pub fn rational_bspline_direction_jet_identity(
    ctx: &mut PredicateContext<'_>,
    controls: &[[LeafRef; 4]],
    knots: &[LeafRef],
    degree: usize,
    order: usize,
    endpoints: bool,
    arc_length: bool,
) -> Result<BezierIdentityDecision, InputError> {
    if !(1..=32).contains(&degree)
        || !(1..=2).contains(&order)
        || controls.len() < degree + 1
        || knots.len() != controls.len() + degree + 1
    {
        return Err(InputError::InvalidInput("Invalid B-spline internal jets"));
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
        let n = controls.len();
        if x[..n * 4]
            .chunks_exact(4)
            .any(|h| h[3].sign() != Sign::Positive)
            || u[n].sub(&u[degree], ctx)?.sign() != Sign::Positive
        {
            return Ok(BezierIdentity::Indeterminate(Reason::MissingProof));
        }
        for pair in u.windows(2) {
            if pair[1].sub(&pair[0], ctx)?.sign() == Sign::Negative {
                return Ok(BezierIdentity::Indeterminate(Reason::MissingProof));
            }
        }
        if endpoints {
            let mut first = degree;
            while first < n && u[first + 1].sub(&u[first], ctx)?.sign() == Sign::Zero {
                first += 1;
            }
            let mut last = n - 1;
            while last > degree && u[last + 1].sub(&u[last], ctx)?.sign() == Sign::Zero {
                last -= 1;
            }
            if first > last {
                return Ok(BezierIdentity::Indeterminate(Reason::MissingProof));
            }
            let start = cartesian(
                &series(&x, u, degree, first, &u[degree], order + 1, ctx)?,
                order + 1,
                ctx,
            )?;
            let end = cartesian(
                &series(&x, u, degree, last, &u[n], order + 1, ctx)?,
                order + 1,
                ctx,
            )?;
            if !direction_equal(&start, &end, order, arc_length, ctx)? {
                return Ok(BezierIdentity::Different);
            }
            return Ok(BezierIdentity::Equal);
        }
        let mut span = degree;
        while span < n {
            if u[span + 1].sub(&u[span], ctx)?.sign() != Sign::Positive {
                span += 1;
                continue;
            }
            let mut right = span + 1;
            while right < n && u[right + 1].sub(&u[right], ctx)?.sign() == Sign::Zero {
                right += 1;
            }
            if right >= n {
                break;
            }
            let t = &u[span + 1];
            let left = cartesian(
                &series(&x, u, degree, span, t, order + 1, ctx)?,
                order + 1,
                ctx,
            )?;
            let next = cartesian(
                &series(&x, u, degree, right, t, order + 1, ctx)?,
                order + 1,
                ctx,
            )?;
            if !direction_equal(&left, &next, order, arc_length, ctx)? {
                return Ok(BezierIdentity::Different);
            }
            span = right;
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
mod direction_tests {
    use super::*;
    use crate::{Limits, SourceArena, ToleranceContext};
    fn audit(
        p: &[[f64; 4]],
        u: &[f64],
        degree: usize,
        order: usize,
        endpoints: bool,
        arc: bool,
        work: u64,
    ) -> BezierIdentityDecision {
        let mut v = p.iter().flatten().copied().collect::<Vec<_>>();
        let start = v.len();
        v.extend_from_slice(u);
        let arena = SourceArena::authored(
            "original-direction-jets",
            1,
            v.into_iter()
                .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
                .collect(),
        )
        .unwrap();
        let controls = (0..p.len())
            .map(|i| std::array::from_fn(|j| arena.leaf(i * 4 + j).unwrap()))
            .collect::<Vec<_>>();
        let knots = (0..u.len())
            .map(|i| arena.leaf(start + i).unwrap())
            .collect::<Vec<_>>();
        let tol = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(
            &arena,
            &tol,
            Limits {
                max_work: work,
                ..Limits::default()
            },
            None,
        );
        rational_bspline_direction_jet_identity(
            &mut ctx, &controls, &knots, degree, order, endpoints, arc,
        )
        .unwrap()
    }
    #[test]
    fn direction_jets_keep_speed_changes_separate_and_refuse_reversal_and_partial_work() {
        let p = [[0., 0., 0., 1.], [1., 0., 0., 1.], [4., 0., 0., 1.]];
        let knots = [2., 2., 3., 5., 5.];
        for arc in [false, true] {
            for endpoints in [false, true] {
                let r = audit(&p, &knots, 1, 2, endpoints, arc, 1000000);
                assert_eq!(r.outcome, BezierIdentity::Equal);
                assert!(matches!(
                    audit(&p, &knots, 1, 2, endpoints, arc, r.work_used - 1).outcome,
                    BezierIdentity::Indeterminate(_)
                ));
                let mut bad = p;
                bad[2][0] = 0.;
                assert_ne!(
                    audit(&bad, &knots, 1, 2, endpoints, arc, 1000000).outcome,
                    BezierIdentity::Equal
                );
                let mut stopped = p;
                stopped[2][0] = 1.;
                assert_ne!(
                    audit(&stopped, &knots, 1, 2, endpoints, arc, 1000000).outcome,
                    BezierIdentity::Equal
                );
            }
        }
    }
    #[test]
    fn rational_conic_direction_curvature_does_not_infer_second_arc_jet() {
        let p = [
            [1., 0., 0., 1.],
            [1., 1., 0., 0.5],
            [0., 1., 0., 1.],
            [-1., 1., 0., 0.5],
            [-1., 0., 0., 1.],
            [-1., -1., 0., 0.5],
            [0., -1., 0., 1.],
            [1., -1., 0., 0.5],
            [1., 0., 0., 1.],
        ];
        let u = [2., 2., 2., 2.75, 2.75, 3.5, 3.5, 4.25, 4.25, 5., 5., 5.];
        for endpoints in [false, true] {
            assert_eq!(
                audit(&p, &u, 2, 1, endpoints, true, 1000000).outcome,
                BezierIdentity::Equal
            );
            assert_eq!(
                audit(&p, &u, 2, 2, endpoints, true, 1000000).outcome,
                BezierIdentity::Different
            );
        }
    }
}
