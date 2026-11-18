//! Exact original-corner fan registration.
use super::corner::Corner;
use crate::source_vertex_links::Node;
use cad_predicates::{
    AuthoredScalar, Limits, Outcome, PredicateContext, RayFanOutcome, Sign, SourceArena,
    ToleranceContext,
};
use std::collections::{BTreeMap, BTreeSet};
pub(super) struct Report {
    pub proven: bool,
    pub exact_work: u64,
}
fn pair(a: Node, b: Node) -> (Node, Node) {
    if a < b { (a, b) } else { (b, a) }
}
pub(super) fn certify(
    corners: &[Corner<'_>],
    cycle: &[Node],
    axis: usize,
    max_work: u64,
) -> Report {
    let mut out = Report {
        proven: false,
        exact_work: 0,
    };
    if axis >= 3 || corners.len() != cycle.len() || !(3..=4096).contains(&cycle.len()) {
        return out;
    }
    let origin = corners[0].point;
    let mut pairs = BTreeSet::new();
    let mut points = vec![origin];
    for c in corners {
        if c.point != origin || !pairs.insert(pair(c.rays[0].0, c.rays[1].0)) {
            return out;
        }
        points.extend(c.rays.iter().map(|(_, p)| *p));
    }
    for i in 0..cycle.len() {
        if !pairs.remove(&pair(cycle[i], cycle[(i + 1) % cycle.len()])) {
            return out;
        }
    }
    if !pairs.is_empty() {
        return out;
    }
    let Ok(arena) = SourceArena::authored(
        "original-natural-vertex-fan",
        1,
        points
            .iter()
            .flatten()
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    ) else {
        return out;
    };
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &arena,
        &tolerance,
        Limits {
            max_work: max_work.min(cad_predicates::MAX_WORK),
            ..Default::default()
        },
        None,
    );
    let axes = [(axis + 1) % 3, (axis + 2) % 3];
    let refs = |i| {
        [
            arena.leaf(3 * i + axes[0]).unwrap(),
            arena.leaf(3 * i + axes[1]).unwrap(),
        ]
    };
    let mut rays = BTreeMap::new();
    let proven = (|| -> Option<bool> {
        for (ci, c) in corners.iter().enumerate() {
            for (ri, (node, _)) in c.rays.iter().enumerate() {
                let index = 1 + 2 * ci + ri;
                if let Some(&old) = rays.get(node) {
                    let r =
                        cad_predicates::orient2d(&mut ctx, refs(0), refs(old), refs(index)).ok()?;
                    if r.outcome != Outcome::Sign(Sign::Zero) {
                        return None;
                    }
                    let k = axes.into_iter().find(|&k| points[old][k] != origin[k])?;
                    if points[index][k] == origin[k]
                        || (points[old][k] > origin[k]) != (points[index][k] > origin[k])
                    {
                        return None;
                    }
                } else {
                    rays.insert(*node, index);
                }
            }
        }
        if rays.len() != cycle.len() {
            return None;
        }
        let refs = cycle
            .iter()
            .map(|n| rays.get(n).map(|&i| refs(i)))
            .collect::<Option<Vec<_>>>()?;
        let r = cad_predicates::oriented_ray_fan(
            &mut ctx,
            [arena.leaf(axes[0]).unwrap(), arena.leaf(axes[1]).unwrap()],
            &refs,
        )
        .ok()?;
        Some(matches!(
            r.outcome,
            RayFanOutcome::OneStrictTurn(Sign::Positive | Sign::Negative)
        ))
    })()
    .unwrap_or(false);
    out.exact_work = ctx.work_used();
    out.proven = proven;
    out
}
