//! Exact source-basis reversal, never rounded knot insertion or refitting.
use super::*;
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
    if n > 33 || p > 32 {
        return Ok((false, 0));
    }
    let clamped = !periodic
        && n == p + 1
        && k[..=p].iter().all(|&x| x == k[p])
        && k[n..].iter().all(|&x| x == k[n]);
    let mut reflected = k.clone();
    let mut work = 0;
    if !clamped {
        use cad_predicates::{
            AuthoredScalar, Limits, ParameterIdentity, PredicateContext, SourceArena,
            ToleranceContext, reflected_parameter_identity,
        };
        let mut values = k.clone();
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
        for i in 0..k.len() {
            let mut selected = None;
            for &candidate in &proposals[i] {
                let decision = reflected_parameter_identity(
                    &mut ctx,
                    [p, n, k.len() - 1 - i, candidate].map(|j| source.leaf(j).unwrap()),
                )
                .map_err(|e| crate::invalid(e.to_string()))?;
                match decision.outcome {
                    ParameterIdentity::Equal => {
                        selected = Some(candidate);
                        break;
                    }
                    ParameterIdentity::Different => {}
                    ParameterIdentity::Indeterminate(_) => return Ok((false, ctx.work_used())),
                }
            }
            let Some(index) = selected else {
                return Ok((false, ctx.work_used()));
            };
            reflected[i] = values[index];
        }
        work = ctx.work_used();
    }
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
    if target.validate().is_err() {
        return Ok((false, work));
    }
    *s = target;
    Ok((true, work))
}
