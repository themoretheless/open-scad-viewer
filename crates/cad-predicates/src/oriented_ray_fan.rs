//! Exact degree-one angular fan of original projected tangent controls.
//! This is an angular premise, not a source-surface or material certificate.
use crate::{
    AuthoredScalar, ContextIdentity, InputError, LeafRef, Outcome, PredicateContext, Reason, Sign,
    orient2d,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RayFanOutcome {
    /// Every consecutive oriented cone is strict and smaller than pi, and the
    /// complete cycle makes exactly one turn in this orientation.
    OneStrictTurn(Sign),
    /// The supplied cycle does not meet THAT strict-cone hypothesis. A general
    /// non-strict fan may still be valid; no embeddedness refusal is inferred.
    NotOneStrictTurn,
    Unproved(Reason),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RayFanDecision {
    pub outcome: RayFanOutcome,
    pub work_used: u64,
    pub context: ContextIdentity,
}
/// Original controls remain points: no rounded tangent differences or angles
/// become authored leaves. Exact determinants classify every adjacent cone.
/// Direct comparisons of original finite binary64 coordinates locate the
/// positive-X cut without subtraction. Counting its directed crossings gives
/// the integer winding, so a two-turn fan cannot masquerade as one-sheeted.
pub fn oriented_ray_fan(
    ctx: &mut PredicateContext<'_>,
    origin: [LeafRef; 2],
    rays: &[[LeafRef; 2]],
) -> Result<RayFanDecision, InputError> {
    if rays.len() > 4096 {
        return Err(InputError::InvalidInput("Bound the original ray fan"));
    }
    // Validate every provenance/type before any work-limit shortcut.
    let mut points = Vec::with_capacity(rays.len() + 1);
    for p in std::iter::once(origin).chain(rays.iter().copied()) {
        let mut q = [0.; 2];
        for axis in 0..2 {
            let AuthoredScalar::Binary64Bits(bits) = ctx.resolve(p[axis])? else {
                return Err(InputError::InvalidInput(
                    "Ray fan needs original binary64 controls",
                ));
            };
            q[axis] = f64::from_bits(*bits);
        }
        points.push(q);
    }
    let result = (|| -> Result<RayFanOutcome, Reason> {
        ctx.charge((2 * points.len()) as u64)?;
        if rays.len() < 3 {
            return Ok(RayFanOutcome::NotOneStrictTurn);
        }
        let half = |p: [f64; 2]| {
            usize::from(!(p[1] > points[0][1] || (p[1] == points[0][1] && p[0] >= points[0][0])))
        };
        let mut orientation = None;
        let mut cuts = 0;
        for i in 0..rays.len() {
            let j = (i + 1) % rays.len();
            let decision =
                orient2d(ctx, origin, rays[i], rays[j]).map_err(|_| Reason::MissingProof)?;
            let sign = match decision.outcome {
                Outcome::Sign(Sign::Zero) => return Ok(RayFanOutcome::NotOneStrictTurn),
                Outcome::Sign(sign) => sign,
                Outcome::Indeterminate(reason) => return Err(reason),
            };
            if orientation.is_some_and(|previous| previous != sign) {
                return Ok(RayFanOutcome::NotOneStrictTurn);
            }
            orientation = Some(sign);
            let (a, b) = (half(points[i + 1]), half(points[j + 1]));
            cuts += usize::from(match sign {
                Sign::Positive => a == 1 && b == 0,
                Sign::Negative => a == 0 && b == 1,
                Sign::Zero => false,
            });
        }
        Ok(if cuts == 1 {
            RayFanOutcome::OneStrictTurn(orientation.unwrap())
        } else {
            RayFanOutcome::NotOneStrictTurn
        })
    })()
    .and_then(|r| ctx.charge(0).map(|_| r));
    Ok(RayFanDecision {
        outcome: result.unwrap_or_else(RayFanOutcome::Unproved),
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Limits, SourceArena, ToleranceContext};
    fn run(points: &[[f64; 2]], max_work: u64) -> RayFanDecision {
        let arena = SourceArena::authored(
            "original-ray-fan",
            1,
            std::iter::once([0., 0.])
                .chain(points.iter().copied())
                .flatten()
                .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
                .collect(),
        )
        .unwrap();
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(
            &arena,
            &tolerance,
            Limits {
                max_work,
                ..Default::default()
            },
            None,
        );
        let refs = |i| [arena.leaf(2 * i).unwrap(), arena.leaf(2 * i + 1).unwrap()];
        oriented_ray_fan(
            &mut ctx,
            refs(0),
            &(1..=points.len()).map(refs).collect::<Vec<_>>(),
        )
        .unwrap()
    }
    #[test]
    fn angular_degree_distinguishes_one_turn_two_turns_and_backtracking() {
        let rays = [[1., 0.], [0., 1.], [-1., 0.], [0., -1.]];
        assert_eq!(
            run(&rays, 100000).outcome,
            RayFanOutcome::OneStrictTurn(Sign::Positive)
        );
        let reversed = rays.into_iter().rev().collect::<Vec<_>>();
        assert_eq!(
            run(&reversed, 100000).outcome,
            RayFanOutcome::OneStrictTurn(Sign::Negative)
        );
        let twice = rays.into_iter().chain(rays).collect::<Vec<_>>();
        assert_eq!(run(&twice, 100000).outcome, RayFanOutcome::NotOneStrictTurn);
        assert_eq!(
            run(&[rays[0], rays[1], rays[0], rays[3]], 100000).outcome,
            RayFanOutcome::NotOneStrictTurn
        );
        assert_eq!(
            run(&[[0., 0.], rays[1], rays[2], rays[3]], 100000).outcome,
            RayFanOutcome::NotOneStrictTurn
        );
    }
    #[test]
    fn complete_angular_degree_has_exact_and_exhausted_work_boundaries() {
        let rays = [[1., 0.], [-1., 1.], [-1., -1.]];
        let full = run(&rays, 100000);
        assert_eq!(full.outcome, RayFanOutcome::OneStrictTurn(Sign::Positive));
        assert_eq!(run(&rays, full.work_used).outcome, full.outcome);
        for work in [0, full.work_used - 1] {
            let short = run(&rays, work);
            assert!(matches!(short.outcome, RayFanOutcome::Unproved(_)));
            assert!(short.work_used <= work);
        }
    }
    #[test]
    fn fan_provenance_is_checked_even_with_zero_work() {
        let arena =
            SourceArena::authored("fan-a", 1, vec![AuthoredScalar::Binary64Bits(0)]).unwrap();
        let other =
            SourceArena::authored("fan-b", 1, vec![AuthoredScalar::Binary64Bits(0)]).unwrap();
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(
            &arena,
            &tolerance,
            Limits {
                max_work: 0,
                ..Default::default()
            },
            None,
        );
        assert!(oriented_ray_fan(&mut ctx, [other.leaf(0).unwrap(); 2], &[]).is_err());
    }
}
