//! Exact implicit continuation through overlapping, independently bounded charts.
//! No algebraic inverse endpoint is approximated or made into a source leaf.
use super::*;
mod bounds;
mod circuits;
mod gram;
mod pullback;
mod semidefinite;
use bounds::Bounds;
mod polynomial;
use polynomial::{common, to_t};
struct Pair {
    cofactors: [B; 3],
}
fn pair(
    a: &[H],
    b: &[H],
    scale: &Q,
    order: usize,
    ctx: &mut PredicateContext<'_>,
) -> Result<Option<Pair>, Reason> {
    let mut residuals = Vec::new();
    let mut f = B::new();
    for k in 0..3 {
        let p = difference(&a[0][k], &b[0][3], &a[0][3], &b[0][k], ctx)?;
        if !p.is_empty() {
            f = if f.is_empty() {
                p.clone()
            } else {
                common(&f, &p, ctx)?
            };
        }
        residuals.push(p);
    }
    // A source-only polynomial or a nonzero constant cannot own a graph cover.
    if !f.keys().any(|&(i, _)| i > 0) || !f.keys().any(|&(_, j)| j > 0) {
        return Ok(None);
    }
    if !jets_mod(a, b, &f, scale, order, ctx)? {
        return Ok(None);
    }
    let mut cofactors = std::array::from_fn(|_| B::new());
    for (k, p) in residuals.into_iter().enumerate() {
        let Some(q) = quotient(p, &f, ctx)? else {
            return Ok(None);
        };
        // The sign/zero-set certificate is invariant under a nonzero scalar. Remove an exact
        // positive scalar gauge before repeated Bernstein subdivision.
        let mut q = q;
        if let Some(pivot) = q.values().next().cloned() {
            let pivot = if pivot.n.sign() == Sign::Negative {
                Q::zero().sub(&pivot, ctx)?
            } else {
                pivot
            };
            for c in q.values_mut() {
                *c = c.div_positive(&pivot, ctx)?;
            }
        }
        cofactors[k] = q;
    }
    Ok(Some(Pair { cofactors }))
}
fn point(h: &[H], t: &Q, ctx: &mut PredicateContext<'_>) -> Result<[Q; 3], Reason> {
    let d = value(&h[0][3], t, ctx)?;
    let mut p = std::array::from_fn(|_| Q::zero());
    for k in 0..3 {
        p[k] = value(&h[0][k], t, ctx)?.div_positive(&d, ctx)?;
    }
    Ok(p)
}
fn same(a: &[Q; 3], b: &[Q; 3], ctx: &mut PredicateContext<'_>) -> Result<bool, Reason> {
    for k in 0..3 {
        if !a[k].equal(&b[k], ctx)? {
            return Ok(false);
        }
    }
    Ok(true)
}
fn bernstein(p: &P, ctx: &mut PredicateContext<'_>) -> Result<P, Reason> {
    let n = p.len() - 1;
    if p.iter().skip(1).all(|q| q.n.sign() == Sign::Zero) {
        return Ok(vec![p[0].clone(); p.len()]);
    }
    let mut out = Vec::new();
    for i in 0..=n {
        let mut b = Q::zero();
        let mut ratio = one();
        for k in 0..=i {
            if k > 0 {
                ratio = ratio
                    .mul(&integer(i + 1 - k), ctx)?
                    .div_positive(&integer(n + 1 - k), ctx)?;
            }
            if p[k].n.sign() != Sign::Zero {
                b = b.add(&p[k].mul(&ratio, ctx)?, ctx)?;
            }
        }
        out.push(b);
    }
    Ok(out)
}
struct Cell {
    span: usize,
    part: usize,
    first: [Q; 3],
    last: [Q; 3],
}
fn cells(
    spans: &[(usize, Vec<H>)],
    u: &[Q],
    level: usize,
    ctx: &mut PredicateContext<'_>,
) -> Result<Vec<Cell>, Reason> {
    let mut out: Vec<Cell> = Vec::new();
    let parts = 1usize << level;
    for (span, (i, h)) in spans.iter().enumerate() {
        let width = u[i + 1]
            .sub(&u[*i], ctx)?
            .div_positive(&integer(parts), ctx)?;
        for j in 0..parts {
            let lo = u[*i].add(&width.mul(&integer(j), ctx)?, ctx)?;
            let hi = u[*i].add(&width.mul(&integer(j + 1), ctx)?, ctx)?;
            let first = point(h, &lo, ctx)?;
            let last = point(h, &hi, ctx)?;
            if let Some(prev) = out.last() {
                if !same(&prev.last, &first, ctx)? {
                    return Err(Reason::MissingProof);
                }
            }
            out.push(Cell {
                span,
                part: j,
                first,
                last,
            });
        }
    }
    Ok(out)
}
/// Continue the unique positively oriented correspondence through overlapping
/// local charts. At every node at least one parameter is an exact rational
/// original/subdivision endpoint; the other may be an unrepresented algebraic
/// root. Its Cartesian point is known by exact divisibility and uniqueness.
pub(crate) fn compare(
    ctx: &mut PredicateContext<'_>,
    spans: &[Vec<(usize, Vec<H>)>],
    u: &[Vec<Q>],
    scale: &Q,
    order: usize,
) -> Result<BezierIdentity, Reason> {
    // Adjacent ORIGINAL intervals may have been cut through one polynomial
    // jet. Exact coefficient identity proves their union without altering the
    // authored surface or dropping its independent intrinsic-knot checks.
    let mut runs = Vec::new();
    let mut knots = Vec::new();
    for side in 0..2 {
        let reps = jet_representatives(&spans[side], ctx)?;
        let mut rows: Vec<(usize, Vec<H>)> = Vec::new();
        let mut us: Vec<Q> = Vec::new();
        for (j, (i, h)) in spans[side].iter().enumerate() {
            if j > 0
                && reps[j] == reps[j - 1]
                && u[side][spans[side][j - 1].0 + 1].equal(&u[side][*i], ctx)?
            {
                *us.last_mut().unwrap() = u[side][i + 1].clone();
            } else {
                if let Some(last) = us.last() {
                    if !last.equal(&u[side][*i], ctx)? {
                        return Err(Reason::MissingProof);
                    }
                } else {
                    us.push(u[side][*i].clone());
                }
                rows.push((rows.len(), h.clone()));
                us.push(u[side][i + 1].clone());
            }
        }
        runs.push(rows);
        knots.push(us);
    }
    // A common positive diagonal world-coordinate change preserves every
    // Cartesian equality and the normal-jet collinearity identities. Apply it
    // to ALL jet orders on BOTH sides, never to the authored source geometry.
    // Original regularity is still independently audited in its original frame.
    for axis in 0..3 {
        let pivot = runs
            .iter()
            .flat_map(|side| side.iter())
            .flat_map(|(_, h)| h.iter())
            .flat_map(|h| h[axis].iter())
            .find(|q| q.n.sign() != Sign::Zero)
            .cloned();
        if let Some(pivot) = pivot {
            let pivot = if pivot.n.sign() == Sign::Negative {
                Q::zero().sub(&pivot, ctx)?
            } else {
                pivot
            };
            if !pivot.equal(&one(), ctx)? {
                for side in &mut runs {
                    for (_, jets) in side {
                        for h in jets {
                            for q in &mut h[axis] {
                                *q = q.div_positive(&pivot, ctx)?;
                            }
                        }
                    }
                }
            }
        }
    }
    continue_cover(ctx, &runs, &knots, scale, order)
}
fn continue_cover(
    ctx: &mut PredicateContext<'_>,
    spans: &[Vec<(usize, Vec<H>)>],
    u: &[Vec<Q>],
    scale: &Q,
    order: usize,
) -> Result<BezierIdentity, Reason> {
    let representatives = [
        jet_representatives(&spans[0], ctx)?,
        jet_representatives(&spans[1], ctx)?,
    ];
    let mut cache: BTreeMap<(usize, usize), Option<Pair>> = BTreeMap::new();
    let mut spent_cells = 0usize;
    let mut bounds = Bounds::default();
    // Existing 4096-cell/depth-12 ceilings; every failed refinement also spends
    // this caller's one exact-work ledger. No fresh budget on a chart switch.
    for level in 0..=12 {
        let count = (spans[0].len() + spans[1].len()) * (1usize << level);
        if spent_cells + count > 4096 {
            return Err(Reason::MissingProof);
        }
        spent_cells += count;
        ctx.charge(count as u64)?;
        let a = cells(&spans[0], &u[0], level, ctx)?;
        let b = cells(&spans[1], &u[1], level, ctx)?;
        if a.is_empty()
            || b.is_empty()
            || !same(&a[0].first, &b[0].first, ctx)?
            || !same(&a.last().unwrap().last, &b.last().unwrap().last, ctx)?
        {
            return Err(Reason::MissingProof);
        }
        let mut current = a[0].first.clone();
        let (mut i, mut j) = (0, 0);
        while i < a.len() && j < b.len() {
            let (x, y) = (&a[i], &b[j]);
            let key = (representatives[0][x.span], representatives[1][y.span]);
            if !cache.contains_key(&key) {
                cache.insert(
                    key,
                    pair(&spans[0][x.span].1, &spans[1][y.span].1, scale, order, ctx)?,
                );
            }
            let Some(proof) = cache[&key].as_ref() else {
                return Err(Reason::MissingProof);
            };
            let mut advance = None;
            let mut axes = [0, 1, 2];
            axes.sort_by_key(|&k| {
                proof.cofactors[k]
                    .keys()
                    .map(|&(i, j)| i + j)
                    .max()
                    .unwrap_or(usize::MAX)
            });
            for axis in axes {
                let sign = x.last[axis].sub(&x.first[axis], ctx)?.n.sign();
                if sign == Sign::Zero || y.last[axis].sub(&y.first[axis], ctx)?.n.sign() != sign {
                    continue;
                }
                let factor = if sign == Sign::Positive {
                    one()
                } else {
                    Q::zero().sub(&one(), ctx)?
                };
                let mut inside = true;
                for cell in [x, y] {
                    if current[axis]
                        .sub(&cell.first[axis], ctx)?
                        .mul(&factor, ctx)?
                        .n
                        .sign()
                        == Sign::Negative
                        || cell.last[axis]
                            .sub(&current[axis], ctx)?
                            .mul(&factor, ctx)?
                            .n
                            .sign()
                            != Sign::Positive
                    {
                        inside = false;
                        break;
                    }
                }
                if !inside
                    || !bounds.slope(0, x.span, axis, level, x.part, &spans[0], &u[0], sign, ctx)?
                    || !bounds.slope(1, y.span, axis, level, y.part, &spans[1], &u[1], sign, ctx)?
                {
                    continue;
                }
                if !bounds.cofactor(
                    bounds::BoxKey { a: x.span, b: y.span, axis: axis, level: level, pa: x.part, pb: y.part },
                    &proof.cofactors[axis],
                    bounds::StripKnots { spans, u },
                    ctx,
                )? {
                    continue;
                }
                let end = x.last[axis]
                    .sub(&y.last[axis], ctx)?
                    .mul(&factor, ctx)?
                    .n
                    .sign();
                if end == Sign::Zero && !same(&x.last, &y.last, ctx)? {
                    return Err(Reason::MissingProof);
                }
                advance = Some(end);
                break;
            }
            let Some(end) = advance else {
                break;
            };
            if end != Sign::Positive {
                current = x.last.clone();
                i += 1;
            }
            if end != Sign::Negative {
                current = y.last.clone();
                j += 1;
            }
        }
        if i == a.len() && j == b.len() {
            ctx.charge(0)?;
            return Ok(BezierIdentity::Equal);
        }
        // Reaching just one final endpoint leaves an uncovered original tail.
        if i == a.len() || j == b.len() {
            return Err(Reason::MissingProof);
        }
    }
    Err(Reason::MissingProof)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AuthoredScalar, Limits, SourceArena, ToleranceContext};
    fn setup(
        ctx: &mut PredicateContext<'_>,
        ramified: bool,
    ) -> (Vec<Vec<(usize, Vec<H>)>>, Vec<Vec<Q>>, Q) {
        let half = one().div_positive(&integer(2), ctx).unwrap();
        let mut spans = Vec::new();
        let mut knots = Vec::new();
        for side in 0..2 {
            // An open parameter interval whose endpoints meet in space. Neither
            // x=q-q³ nor y=q² is monotone on this complete regular curve.
            let mut clock = if side == 0 {
                vec![Q::zero(), half.clone(), half.clone()]
            } else {
                vec![Q::zero(), half.clone(), Q::zero(), half.clone()]
            };
            clock = scaled(&clock, &integer(2), ctx).unwrap();
            clock[0] = Q::zero().sub(&one(), ctx).unwrap();
            let q2 = mul(&clock, &clock, ctx).unwrap();
            let q3 = mul(&q2, &clock, ctx).unwrap();
            let h = vec![
                [
                    if ramified {
                        q3
                    } else {
                        sub(&clock, &q3, ctx).unwrap()
                    },
                    if ramified {
                        add(&q2, &clock, ctx).unwrap()
                    } else {
                        q2
                    },
                    vec![Q::zero()],
                    vec![one()],
                ],
                [
                    vec![Q::zero()],
                    vec![Q::zero()],
                    vec![one()],
                    vec![Q::zero()],
                ],
                std::array::from_fn(|_| vec![Q::zero()]),
            ];
            // Deliberately staggered original cuts; none of the interior
            // Cartesian endpoints agrees with an endpoint on the other side.
            let us = if side == 0 {
                vec![Q::zero(), half.clone(), one()]
            } else {
                vec![
                    Q::zero(),
                    one().div_positive(&integer(4), ctx).unwrap(),
                    one(),
                ]
            };
            spans.push((0..us.len() - 1).map(|i| (i, h.clone())).collect());
            knots.push(us);
        }
        (spans, knots, Q::zero().sub(&one(), ctx).unwrap())
    }
    #[test]
    fn irrational_ramification_node_has_one_exact_cover_without_regularization() {
        let arena =
            SourceArena::authored("ramified", 1, vec![AuthoredScalar::Binary64Bits(0)]).unwrap();
        let tol = ToleranceContext::default_valid();
        let mut build = PredicateContext::new(&arena, &tol, Limits::default(), None);
        let (spans, u, scale) = setup(&mut build, true);
        let mut ctx = PredicateContext::new(&arena, &tol, Limits::default(), None);
        assert!(
            matches!(
                compare(&mut ctx, &spans, &u, &scale, 2),
                Ok(BezierIdentity::Equal)
            ),
            "work={}",
            ctx.work_used()
        );
        let work = ctx.work_used();
        println!("ramified-exact-work={work}");
        let mut short = PredicateContext::new(
            &arena,
            &tol,
            Limits {
                max_work: work - 1,
                ..Limits::default()
            },
            None,
        );
        assert!(compare(&mut short, &spans, &u, &scale, 2).is_err());
    }
    #[test]
    fn implicit_overlap_changes_axes_without_shared_chart_endpoints() {
        let arena =
            SourceArena::authored("overlap", 1, vec![AuthoredScalar::Binary64Bits(0)]).unwrap();
        let tol = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(&arena, &tol, Limits::default(), None);
        let (spans, u, scale) = setup(&mut ctx, false);
        let mut proof = PredicateContext::new(&arena, &tol, Limits::default(), None);
        assert!(
            matches!(
                compare(&mut proof, &spans, &u, &scale, 2),
                Ok(BezierIdentity::Equal)
            ),
            "work={}",
            proof.work_used()
        );
        let work = proof.work_used();
        println!("positive-overlap-exact-work={work}");
        let mut short = PredicateContext::new(
            &arena,
            &tol,
            Limits {
                max_work: work - 1,
                ..Limits::default()
            },
            None,
        );
        assert!(compare(&mut short, &spans, &u, &scale, 2).is_err());
        // Reversal starts at the SAME Cartesian closure point. A coordinate
        // coincidence alone must not jump to the other real algebraic branch.
        let mut reversed = spans.clone();
        let linear = vec![one(), Q::zero().sub(&one(), &mut ctx).unwrap()];
        for (_, jets) in &mut reversed[1] {
            for h in jets {
                for p in h {
                    *p = compose(p, &linear, &vec![one()], p.len() - 1, &mut ctx).unwrap();
                }
            }
        }
        reversed[1].reverse();
        for (i, row) in reversed[1].iter_mut().enumerate() {
            row.0 = i;
        }
        let mut reversed_u = u.clone();
        reversed_u[1] = u[1]
            .iter()
            .rev()
            .map(|q| one().sub(q, &mut ctx).unwrap())
            .collect();
        let mut negative = PredicateContext::new(&arena, &tol, Limits::default(), None);
        assert!(compare(&mut negative, &reversed, &reversed_u, &scale, 2).is_err());
        let mut broken = spans.clone();
        broken[1][1].1[1][2] = vec![integer(2)];
        let mut fresh = PredicateContext::new(&arena, &tol, Limits::default(), None);
        assert!(compare(&mut fresh, &broken, &u, &scale, 1).is_err());
    }
}
