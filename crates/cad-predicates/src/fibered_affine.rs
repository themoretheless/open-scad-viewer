//! Exact ORIGINAL coefficient premises for ordered affine fibers.
//! No patch injectivity, frame rank, boundary smoothness or Solid credit here.
use crate::{
    Algebra, AuthoredScalar, ContextIdentity, Expansion, InputError, LeafRef, PredicateContext,
    Reason, Sign,
};

#[derive(Clone, Debug)]
pub struct FiberedAffineDecision {
    pub direction: Option<Sign>,
    /// Vanishing height derivative orders at the two open traversal ends.
    pub endpoint_flat_orders: Option<[usize; 2]>,
    pub work_used: u64,
    pub context: ContextIdentity,
    pub reason: Option<Reason>,
}
fn dot(
    a: &[Expansion],
    b: &[Expansion],
    ctx: &mut PredicateContext<'_>,
) -> Result<Expansion, Reason> {
    let mut r = Expansion::scalar(0.);
    for k in 0..3 {
        r = r.add(&a[k].mul(&b[k], ctx)?, ctx)?;
    }
    Ok(r)
}
fn det(a: &[Expansion], ctx: &mut PredicateContext<'_>) -> Result<Expansion, Reason> {
    let mut r = Expansion::scalar(0.);
    for (i, j, k, positive) in [
        (0, 4, 8, true),
        (1, 5, 6, true),
        (2, 3, 7, true),
        (2, 4, 6, false),
        (1, 3, 8, false),
        (0, 5, 7, false),
    ] {
        let t = a[i].mul(&a[j], ctx)?.mul(&a[k], ctx)?;
        r = if positive {
            r.add(&t, ctx)?
        } else {
            r.sub(&t, ctx)?
        };
    }
    Ok(r)
}
/// Every leaf is original binary64, with no common normalization of quantities
/// having different polynomial units. A pose control contains three XYZ matrix
/// columns, XYZ translation and positive weight. Pieces have 2..9 controls.
///
/// Proves Q-origin is in ker(source_normal), n_world*A is parallel to that
/// normal for EVERY original pose control, complete pose C0 joins, and strict
/// rational height ordering on every piece, under one shared exact ledger.
/// For positive weights, the derivative numerator is
/// sum_{i<j}(j-i)wi wj(hj-hi)Bi Bj/[t(1-t)]. Nondecreasing height
/// controls make every term nonnegative and one positive on the open piece.
/// Flat endpoint jets do not remove strict order.
pub fn binary_fibered_affine_premises(
    ctx: &mut PredicateContext<'_>,
    carrier: &[[LeafRef; 3]],
    origin: [LeafRef; 3],
    source_normal: [LeafRef; 3],
    world_rows: [[LeafRef; 3]; 3],
    pieces: &[Vec<[LeafRef; 13]>],
) -> Result<FiberedAffineDecision, InputError> {
    if carrier.is_empty()
        || carrier.len() > 10000
        || pieces.is_empty()
        || pieces.len() > 256
        || pieces.iter().any(|p| !(2..=9).contains(&p.len()))
    {
        return Err(InputError::InvalidInput(
            "Invalid bounded affine fiber coefficient layout",
        ));
    }
    let refs = source_normal
        .iter()
        .chain(origin.iter())
        .chain(world_rows.iter().flatten())
        .chain(carrier.iter().flatten())
        .chain(pieces.iter().flatten().flatten())
        .copied()
        .collect::<Vec<_>>();
    let values = refs
        .iter()
        .map(|&r| ctx.resolve(r).cloned())
        .collect::<Result<Vec<_>, _>>()?;
    if values
        .iter()
        .any(|v| !matches!(v, AuthoredScalar::Binary64Bits(_)))
    {
        return Err(InputError::InvalidInput(
            "Affine fiber premises require literal binary64 coefficients",
        ));
    }
    let raw = values
        .iter()
        .map(|v| match v {
            AuthoredScalar::Binary64Bits(b) => f64::from_bits(*b),
            _ => unreachable!(),
        })
        .collect::<Vec<_>>();
    let Some(pivot) = (0..3).find(|&k| raw[k] != 0.) else {
        return Err(InputError::InvalidInput(
            "Nonzero source plane normal required",
        ));
    };
    if raw[12..15].iter().all(|v| *v == 0.) {
        return Err(InputError::InvalidInput(
            "Nonzero world height normal required",
        ));
    }
    let motion = 15 + carrier.len() * 3;
    if raw[motion..].chunks_exact(13).any(|p| p[12] <= 0.) {
        return Err(InputError::InvalidInput(
            "Positive rational pose weights required",
        ));
    }
    let computed = (|| -> Result<(Sign, [usize; 2]), Reason> {
        ctx.charge(raw.len() as u64)?;
        let a = raw
            .iter()
            .copied()
            .map(Expansion::scalar)
            .collect::<Vec<_>>();
        if det(&a[6..15], ctx)?.sign() == Sign::Zero {
            return Err(Reason::MissingProof);
        }
        for point in a[15..motion].chunks_exact(3) {
            let q = (0..3)
                .map(|k| point[k].sub(&a[3 + k], ctx))
                .collect::<Result<Vec<_>, _>>()?;
            if dot(&a[..3], &q, ctx)?.sign() != Sign::Zero {
                return Err(Reason::MissingProof);
            }
        }
        let mut direction = None;
        let mut cursor = motion;
        let mut previous: Option<usize> = None;
        let mut flats = [0; 2];
        for (piece_id, piece) in pieces.iter().enumerate() {
            if let Some(last) = previous {
                ctx.charge(12)?;
                if raw[cursor..cursor + 12] != raw[last..last + 12] {
                    return Err(Reason::MissingProof);
                }
            }
            let mut heights = Vec::new();
            for _ in piece {
                let rows = (0..3)
                    .map(|col| dot(&a[12..15], &a[cursor + col * 3..cursor + col * 3 + 3], ctx))
                    .collect::<Result<Vec<_>, _>>()?;
                for j in 0..3 {
                    let residual = rows[j]
                        .mul(&a[pivot], ctx)?
                        .sub(&rows[pivot].mul(&a[j], ctx)?, ctx)?;
                    if residual.sign() != Sign::Zero {
                        return Err(Reason::MissingProof);
                    }
                }
                heights.push(dot(&a[12..15], &a[cursor + 9..cursor + 12], ctx)?);
                previous = Some(cursor);
                cursor += 13;
            }
            let mut differences = Vec::new();
            let mut strict = false;
            for pair in heights.windows(2) {
                let sign = pair[1].sub(&pair[0], ctx)?.sign();
                differences.push(sign);
                if sign != Sign::Zero {
                    strict = true;
                    if direction.is_some_and(|d| d != sign) {
                        return Err(Reason::MissingProof);
                    }
                    direction = Some(sign);
                }
            }
            if !strict {
                return Err(Reason::MissingProof);
            }
            if piece_id == 0 {
                flats[0] = differences.iter().take_while(|s| **s == Sign::Zero).count();
            }
            if piece_id + 1 == pieces.len() {
                flats[1] = differences
                    .iter()
                    .rev()
                    .take_while(|s| **s == Sign::Zero)
                    .count();
            }
        }
        Ok((direction.ok_or(Reason::MissingProof)?, flats))
    })();
    let (direction, endpoint_flat_orders, reason) = match computed {
        Ok((s, f)) => (Some(s), Some(f), None),
        Err(r) => (None, None, Some(r)),
    };
    Ok(FiberedAffineDecision {
        direction,
        endpoint_flat_orders,
        reason,
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Limits, SourceArena, ToleranceContext};
    use std::sync::atomic::AtomicBool;
    fn values() -> Vec<f64> {
        let mut v = vec![0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 1.];
        for q in [[0., 0., 0.], [0., 1., 0.], [1., 0., 0.], [1., 1., 0.]] {
            v.extend(q);
        }
        for (z, w) in [0., 0., 0., 1., 1., 1.]
            .into_iter()
            .zip([1., 2., 3., 4., 5., 6.])
        {
            v.extend([1., 0., 0., 0., 1., 0., 0., 0., 0., 0., 0., z, w]);
        }
        v
    }
    fn run(
        a: &SourceArena,
        max: u64,
        cancel: Option<&AtomicBool>,
    ) -> Result<FiberedAffineDecision, InputError> {
        let t = ToleranceContext::default_valid();
        let mut c = PredicateContext::new(
            a,
            &t,
            Limits {
                max_work: max,
                ..Default::default()
            },
            cancel,
        );
        let leaf = |i| a.leaf(i).unwrap();
        binary_fibered_affine_premises(
            &mut c,
            &(0..4)
                .map(|i| std::array::from_fn(|j| leaf(15 + 3 * i + j)))
                .collect::<Vec<_>>(),
            std::array::from_fn(|j| leaf(3 + j)),
            std::array::from_fn(|j| leaf(j)),
            std::array::from_fn(|i| std::array::from_fn(|j| leaf(6 + 3 * i + j))),
            &[(0..6)
                .map(|i| std::array::from_fn(|j| leaf(27 + 13 * i + j)))
                .collect()],
        )
    }
    fn arena(v: Vec<f64>) -> SourceArena {
        SourceArena::authored(
            "fibered-exact-test",
            1,
            v.into_iter()
                .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
                .collect(),
        )
        .unwrap()
    }
    #[test]
    fn exact_original_fiber_premises_keep_flat_ends_and_one_shared_work_ledger() {
        let a = arena(values());
        let full = run(&a, 1000000, None).unwrap();
        assert_eq!(full.direction, Some(Sign::Positive));
        assert_eq!(full.endpoint_flat_orders, Some([2, 2]));
        for max in [0, full.work_used - 1] {
            let r = run(&a, max, None).unwrap();
            assert!(r.direction.is_none());
            assert!(r.work_used <= max);
        }
        let cancelled = AtomicBool::new(true);
        let r = run(&a, 1000000, Some(&cancelled)).unwrap();
        assert_eq!(r.reason, Some(Reason::Cancelled));
        assert!(r.direction.is_none());
        let mut damaged = values();
        damaged[27 + 13 + 2] = f64::from_bits(1);
        assert!(
            run(&arena(damaged), 1000000, None)
                .unwrap()
                .direction
                .is_none()
        );
    }
    #[test]
    fn malformed_original_tail_and_rational_leaves_are_not_zero_budget_refusals() {
        let mut v = values();
        *v.last_mut().unwrap() = 0.;
        assert!(run(&arena(v), 0, None).is_err());
        let mut v = values()
            .into_iter()
            .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
            .collect::<Vec<_>>();
        v[0] = AuthoredScalar::RationalConstant {
            numerator: 0,
            denominator: 1,
        };
        let a = SourceArena::authored("fibered-rational-not-binary", 1, v).unwrap();
        assert!(run(&a, 0, None).is_err());
    }
}

/// Cartesian C1/C2 of two ORIGINAL rational affine pose pieces in their local
/// increasing unit clocks. Equal-duration global clock factors cancel. This
/// checks all twelve affine fields together, not independent geometric speed
/// ratios. No global embedding, boundary Gk or frame rank is inferred.
pub fn binary_affine_pose_endpoint_jets(
    ctx: &mut PredicateContext<'_>,
    a: &[[LeafRef; 13]],
    b: &[[LeafRef; 13]],
    order: usize,
) -> Result<crate::BezierIdentityDecision, InputError> {
    use crate::{BezierIdentity, BezierIdentityDecision};
    if !(2..=9).contains(&a.len()) || !(2..=9).contains(&b.len()) || !(1..=2).contains(&order) {
        return Err(InputError::InvalidInput(
            "Invalid original affine pose endpoint layout",
        ));
    }
    let raw = a
        .iter()
        .rev()
        .chain(b)
        .flatten()
        .copied()
        .map(|r| ctx.resolve(r).cloned())
        .collect::<Result<Vec<_>, _>>()?;
    if raw
        .iter()
        .any(|v| !matches!(v, AuthoredScalar::Binary64Bits(_)))
    {
        return Err(InputError::InvalidInput(
            "Literal original binary64 pose coefficients required",
        ));
    }
    let values = raw
        .iter()
        .map(|v| match v {
            AuthoredScalar::Binary64Bits(b) => f64::from_bits(*b),
            _ => unreachable!(),
        })
        .collect::<Vec<_>>();
    if values.chunks_exact(13).any(|p| p[12] <= 0.) {
        return Err(InputError::InvalidInput(
            "Positive original pose weights required",
        ));
    }
    let computed = (|| -> Result<BezierIdentity, Reason> {
        ctx.charge(values.len() as u64)?;
        let x = values
            .iter()
            .copied()
            .map(Expansion::scalar)
            .collect::<Vec<_>>();
        let build = |start: usize,
                     n: usize,
                     k: usize,
                     ctx: &mut PredicateContext<'_>|
         -> Result<(Expansion, Expansion, Expansion), Reason> {
            let d = Expansion::integer((n - 1) as u64, ctx)?;
            let two = Expansion::integer(2, ctx)?;
            let w = x[start + 12].clone();
            let w1 = x[start + 25].clone();
            let h = x[start + k].mul(&w, ctx)?;
            let h1 = x[start + 13 + k].mul(&w1, ctx)?;
            let dw = w1.sub(&w, ctx)?.mul(&d, ctx)?;
            let dh = h1.sub(&h, ctx)?.mul(&d, ctx)?;
            let first = dh.mul(&w, ctx)?.sub(&h.mul(&dw, ctx)?, ctx)?;
            let mut second = Expansion::scalar(0.);
            if order == 2 {
                let (ddw, ddh) = if n >= 3 {
                    let scale = d.mul(&Expansion::integer((n - 2) as u64, ctx)?, ctx)?;
                    let w2 = &x[start + 38];
                    let h2 = x[start + 26 + k].mul(w2, ctx)?;
                    (
                        w2.sub(&w1.mul(&two, ctx)?, ctx)?
                            .add(&w, ctx)?
                            .mul(&scale, ctx)?,
                        h2.sub(&h1.mul(&two, ctx)?, ctx)?
                            .add(&h, ctx)?
                            .mul(&scale, ctx)?,
                    )
                } else {
                    (Expansion::scalar(0.), Expansion::scalar(0.))
                };
                second = ddh
                    .mul(&w, ctx)?
                    .mul(&w, ctx)?
                    .sub(&h.mul(&ddw, ctx)?.mul(&w, ctx)?, ctx)?
                    .sub(&dh.mul(&w, ctx)?.mul(&dw, ctx)?.mul(&two, ctx)?, ctx)?
                    .add(&h.mul(&dw, ctx)?.mul(&dw, ctx)?.mul(&two, ctx)?, ctx)?;
            }
            Ok((w, first, second))
        };
        let offset = 13 * a.len();
        for k in 0..12 {
            ctx.charge(1)?;
            if x[k].sub(&x[offset + k], ctx)?.sign() != Sign::Zero {
                return Ok(BezierIdentity::Different);
            }
            let (wa, da, ea) = build(0, a.len(), k, ctx)?;
            let (wb, db, eb) = build(offset, b.len(), k, ctx)?;
            let wa2 = wa.mul(&wa, ctx)?;
            let wb2 = wb.mul(&wb, ctx)?;
            if da.mul(&wb2, ctx)?.add(&db.mul(&wa2, ctx)?, ctx)?.sign() != Sign::Zero {
                return Ok(BezierIdentity::Different);
            }
            if order == 2
                && ea
                    .mul(&wb2, ctx)?
                    .mul(&wb, ctx)?
                    .sub(&eb.mul(&wa2, ctx)?.mul(&wa, ctx)?, ctx)?
                    .sign()
                    != Sign::Zero
            {
                return Ok(BezierIdentity::Different);
            }
        }
        Ok(BezierIdentity::Equal)
    })();
    Ok(BezierIdentityDecision {
        outcome: computed.unwrap_or_else(BezierIdentity::Indeterminate),
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}

#[cfg(test)]
mod motion_jet_tests {
    use super::*;
    use crate::{BezierIdentity, Limits, SourceArena, ToleranceContext};
    fn run(
        a: &[[f64; 13]],
        b: &[[f64; 13]],
        order: usize,
        max: u64,
    ) -> crate::BezierIdentityDecision {
        let values = a
            .iter()
            .chain(b)
            .flatten()
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect();
        let source = SourceArena::authored("motion-jets", 1, values).unwrap();
        let controls = (0..a.len() + b.len())
            .map(|i| std::array::from_fn(|j| source.leaf(i * 13 + j).unwrap()))
            .collect::<Vec<_>>();
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(
            &source,
            &tolerance,
            Limits {
                max_work: max,
                ..Default::default()
            },
            None,
        );
        binary_affine_pose_endpoint_jets(
            &mut ctx,
            &controls[..a.len()],
            &controls[a.len()..],
            order,
        )
        .unwrap()
    }
    fn pose(z: f64, w: f64) -> [f64; 13] {
        [1., 0., 0., 0., 1., 0., 0., 0., 0., 0., 0., z, w]
    }
    #[test]
    fn different_degrees_original_cartesian_motion_jets_and_g1_only_rational_join() {
        let a = [pose(0., 1.), pose(0.5, 1.), pose(1., 1.)];
        let b = [
            pose(1., 1.),
            pose(1.25, 1.),
            pose(1.5, 1.),
            pose(1.75, 1.),
            pose(2., 1.),
        ];
        let full = run(&a, &b, 2, 1000000);
        assert_eq!(full.outcome, BezierIdentity::Equal);
        for max in [0, full.work_used - 1] {
            let r = run(&a, &b, 2, max);
            assert_ne!(r.outcome, BezierIdentity::Equal);
            assert!(r.work_used <= max);
        }
        let a = [pose(0., 1.), pose(1., 1.)];
        let b = [pose(1., 2.), pose(3., 1.)];
        assert_eq!(run(&a, &b, 1, 1000000).outcome, BezierIdentity::Equal);
        assert_eq!(run(&a, &b, 2, 1000000).outcome, BezierIdentity::Different);
    }
}
