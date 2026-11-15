//! Exact source-basis reversal, never rounded knot insertion or refitting.
use super::*;
use cad_predicates::{
    AuthoredScalar, Limits, ParameterIdentity, PredicateContext, SourceArena, ToleranceContext,
    reflected_parameter_identity,
};
/// Largest reversible control count along the free direction.
const MAX_REVERSAL_CONTROL_POINTS: usize = 33;
/// Largest reversible degree along the free direction.
const MAX_REVERSAL_DEGREE: usize = 32;
/// Active-domain knot indices `p` and `n` of the reversed direction.
struct KnotRow<'a> {
    p: usize,
    n: usize,
    k: &'a [f64],
}
impl KnotRow<'_> {
    fn clamped(&self, periodic: bool) -> bool {
        let Self { p, n, k } = *self;
        !periodic
            && n == p + 1
            && k[..=p].iter().all(|&x| x == k[p])
            && k[n..].iter().all(|&x| x == k[n])
    }
    /// Candidate binary64 values per reflected knot; arithmetic only proposes.
    fn proposals(&self) -> (Vec<f64>, Vec<Vec<usize>>) {
        let Self { p, n, k } = *self;
        let mut values = k.to_vec();
        let mut proposals = Vec::new();
        for i in 0..k.len() {
            let old = k[k.len() - 1 - i];
            let a = k[p];
            let b = k[n];
            let mut row = vec![i];
            if a == -b {
                // Exact sign reversal is the best proposal for a symmetric
                // active domain, even when an interior knot is subnormal.
                row.insert(0, values.len());
                values.push(-old);
            }
            // All arithmetic is only a proposal. Exact expansions below must
            // independently prove a+b-old-new=0 before any stored bit changes.
            for v in [(a + b) - old, a + (b - old), b + (a - old)] {
                if v.is_finite() && !row.iter().any(|&j| values[j].to_bits() == v.to_bits()) {
                    row.push(values.len());
                    values.push(v);
                }
            }
            proposals.push(row);
        }
        (values, proposals)
    }
    /// Exactly proven reflected knots, or `None` when any knot stays unproven,
    /// with the predicate work spent either way.
    fn reflect(&self, max_work: u64) -> Result<(Option<Vec<f64>>, u64)> {
        let Self { p, n, k } = *self;
        let (values, proposals) = self.proposals();
        let source = SourceArena::authored(
            "owned-bspline-knot-reversal",
            1,
            values
                .iter()
                .copied()
                .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
                .collect(),
        )
        .map_err(|e| crate::invalid(e.to_string()))?;
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(
            &source,
            &tolerance,
            Limits {
                max_work: max_work.min(cad_predicates::MAX_WORK),
                ..Limits::default()
            },
            None,
        );
        let leaf = |j: usize| source.leaf(j).map_err(|e| crate::invalid(e.to_string()));
        let mut reflected = k.to_vec();
        for (i, row) in proposals.iter().enumerate() {
            let mut selected = None;
            for &candidate in row {
                let leaves = [leaf(p)?, leaf(n)?, leaf(k.len() - 1 - i)?, leaf(candidate)?];
                let decision = reflected_parameter_identity(&mut ctx, leaves)
                    .map_err(|e| crate::invalid(e.to_string()))?;
                match decision.outcome {
                    ParameterIdentity::Equal => {
                        selected = Some(candidate);
                        break;
                    }
                    ParameterIdentity::Different => {}
                    ParameterIdentity::Indeterminate(_) => return Ok((None, ctx.work_used())),
                }
            }
            let Some(index) = selected else {
                return Ok((None, ctx.work_used()));
            };
            reflected[i] = values[index];
        }
        Ok((Some(reflected), ctx.work_used()))
    }
}
fn reversed_surface(s: &Surface, free: usize, reflected: Vec<f64>) -> Surface {
    let mut target = s.clone();
    if free == 0 {
        target.control_points.reverse();
        target.weights.reverse();
        target.knots_u = reflected;
    } else {
        for r in &mut target.control_points {
            r.reverse();
        }
        for r in &mut target.weights {
            r.reverse();
        }
        target.knots_v = reflected;
    }
    target
}
pub(super) fn reverse_along(s: &mut Surface, label: &str, max_work: u64) -> Result<(bool, u64)> {
    if s.validate().is_err() {
        return Ok((false, 0));
    }
    let free = usize::from(label.starts_with('u'));
    let (p, n, k, periodic) = if free == 0 {
        (s.degree_u, s.control_points.len(), &s.knots_u, s.periodic_u)
    } else {
        (
            s.degree_v,
            s.control_points[0].len(),
            &s.knots_v,
            s.periodic_v,
        )
    };
    if n > MAX_REVERSAL_CONTROL_POINTS || p > MAX_REVERSAL_DEGREE {
        return Ok((false, 0));
    }
    let row = KnotRow { p, n, k };
    let (reflected, work) = if row.clamped(periodic) {
        (k.clone(), 0)
    } else {
        match row.reflect(max_work)? {
            (Some(reflected), work) => (reflected, work),
            (None, work) => return Ok((false, work)),
        }
    };
    let target = reversed_surface(s, free, reflected);
    if target.validate().is_err() {
        return Ok((false, work));
    }
    *s = target;
    Ok((true, work))
}
