//! Exact reduction of clamped degree-one curves with a constant weight gauge.
//!
//! Collinearity is checked in (original parameter, original coordinate), not
//! merely in world space. Candidate coordinates, weights and knots are copied
//! from the source; no refit or rounded interpolation constructs a candidate.
use crate::{Result, check, curve::Curve};
use cad_predicates::{
    AuthoredScalar, Outcome, PredicateContext, Reason, Sign, SourceArena, ToleranceContext,
};

#[derive(Clone, Debug)]
pub struct Report {
    pub curve: Curve,
    pub retained_source_controls: Vec<usize>,
    pub removed_source_controls: Vec<usize>,
    /// False means the search stopped early; all returned removals are still
    /// proven exact identities of the nominal parametric definition.
    pub complete: bool,
    pub reason: Option<Reason>,
    pub work_used: u64,
}

struct Work {
    remaining: u64,
    used: u64,
}
impl Work {
    fn axis(&mut self, source: &Curve, indices: [usize; 3], axis: usize) -> Result<Outcome> {
        if self.remaining == 0 {
            return Ok(Outcome::Indeterminate(Reason::ResourceLimit));
        }
        let values = indices
            .iter()
            .flat_map(|&i| {
                [
                    AuthoredScalar::Binary64Bits(source.knots[i + 1].to_bits()),
                    AuthoredScalar::Binary64Bits(source.control_points[i][axis].to_bits()),
                ]
            })
            .collect();
        let arena = SourceArena::authored("linear-control-simplification", 0, values)
            .map_err(|e| crate::input(format!("Linear simplification source: {e}")))?;
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(
            &arena,
            &tolerance,
            cad_predicates::Limits {
                max_work: self.remaining,
                ..Default::default()
            },
            None,
        );
        let p = |i: usize| {
            [
                arena.leaf(2 * i).expect("local source index"),
                arena.leaf(2 * i + 1).expect("local source index"),
            ]
        };
        let decision = cad_predicates::orient2d(&mut ctx, p(0), p(1), p(2))
            .map_err(|e| crate::input(format!("Linear simplification predicate: {e}")))?;
        let charged = ctx.work_used();
        self.remaining = self
            .remaining
            .checked_sub(charged)
            .ok_or_else(|| crate::input("Invalid predicate work accounting"))?;
        self.used += charged;
        Ok(decision.outcome)
    }
}

/// Remove redundant linear controls while preserving the complete nominal
/// parameterized definition, including endpoints, domain and traversal speed.
///
/// Requires nonperiodic degree-one storage, clamped endpoint knots, strictly
/// increasing interior sites and identical positive weights. `max_work` is a
/// shared predicate budget in 1..=1000000. Input validation/copying are bounded
/// by the existing Curve limits and are outside that arithmetic-work counter.
///
/// Exact identity concerns nominal real-valued definitions. Evaluating two
/// different representations in binary64 may still round differently. This
/// report is not a B-rep/topology or source-history certificate.
pub fn simplify(source: &Curve, max_work: u64) -> Result<Report> {
    source.validate()?;
    check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Linear simplification work must lie in 1..=1000000",
    )?;
    check(
        source.degree == 1 && !source.periodic,
        "Exact linear simplification requires nonperiodic degree-one storage",
    )?;
    let n = source.control_points.len();
    check(
        source.knots[0] == source.knots[1] && source.knots[n] == source.knots[n + 1],
        "Exact linear simplification requires clamped endpoints",
    )?;
    check(
        source.knots[1..=n].windows(2).all(|w| w[0] < w[1]),
        "Exact linear simplification requires distinct ordered sites",
    )?;
    check(
        source.weights.iter().all(|&w| w == source.weights[0]),
        "Exact linear simplification requires one constant weight gauge",
    )?;
    let mut work = Work {
        remaining: max_work,
        used: 0,
    };
    let mut retained = Vec::with_capacity(n);
    let mut reason = None;
    'scan: for i in 0..n {
        retained.push(i);
        while retained.len() >= 3 {
            let len = retained.len();
            let indices = [retained[len - 3], retained[len - 2], retained[len - 1]];
            let mut redundant = true;
            for axis in 0..source.control_points[0].len() {
                match work.axis(source, indices, axis)? {
                    Outcome::Sign(Sign::Zero) => {}
                    Outcome::Sign(_) => {
                        redundant = false;
                        break;
                    }
                    Outcome::Indeterminate(why) => {
                        reason = Some(why);
                        retained.extend(i + 1..n);
                        break 'scan;
                    }
                }
            }
            if !redundant {
                break;
            }
            retained.remove(len - 2);
        }
    }
    let mut kept = vec![false; n];
    for &i in &retained {
        kept[i] = true;
    }
    let removed_source_controls = (0..n).filter(|&i| !kept[i]).collect();
    let mut knots = Vec::with_capacity(retained.len() + 2);
    knots.push(source.knots[0]);
    knots.extend(retained.iter().map(|&i| source.knots[i + 1]));
    knots.push(source.knots[n + 1]);
    let curve = Curve {
        degree: 1,
        knots,
        control_points: retained
            .iter()
            .map(|&i| source.control_points[i].clone())
            .collect(),
        weights: retained.iter().map(|&i| source.weights[i]).collect(),
        periodic: false,
    };
    curve.validate()?;
    Ok(Report {
        curve,
        retained_source_controls: retained,
        removed_source_controls,
        complete: reason.is_none(),
        reason,
        work_used: work.used,
    })
}
