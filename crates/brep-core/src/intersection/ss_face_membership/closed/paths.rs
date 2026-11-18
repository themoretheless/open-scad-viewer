use super::*;
/// Source incidence, not endpoint proximity, establishes the cyclic order.
pub(super) fn cycle(
    curves: &QualifiedGraphCurves,
    edges: &[usize],
    used: &mut usize,
    max: usize,
) -> Result<Vec<(usize, bool)>> {
    let original = curves.original();
    let mut remaining: std::collections::BTreeSet<_> = edges.iter().copied().collect();
    if remaining.len() != edges.len() || remaining.len() < 2 {
        return Err(unproven(
            "Closed SS contour needs distinct owned branch edges",
        ));
    }
    let first = *remaining.first().unwrap();
    let start = original.edges()[first].vertices[0];
    let mut at = start;
    let mut order = Vec::new();
    while !remaining.is_empty() {
        charge(used, max, remaining.len())?;
        let candidates: Vec<_> = remaining
            .iter()
            .copied()
            .filter(|&e| original.edges()[e].vertices.contains(&at))
            .collect();
        let edge = if order.is_empty() {
            first
        } else if candidates.len() == 1 {
            candidates[0]
        } else {
            return Err(unproven(
                "Original closed SS incidence is branched or disconnected",
            ));
        };
        let ports = original.edges()[edge].vertices;
        let reverse = ports[0] != at;
        at = ports[usize::from(!reverse)];
        order.push((edge, reverse));
        remaining.remove(&edge);
        if at == start && !remaining.is_empty() {
            return Err(unproven("One source branch contains several cycles"));
        }
    }
    if at != start {
        return Err(unproven("Original SS branch ports do not close"));
    }
    Ok(order)
}
pub(super) fn segments(
    curves: &QualifiedGraphCurves,
    order: &[(usize, bool)],
    side: usize,
) -> Result<Vec<Segment>> {
    order
        .iter()
        .map(|&(ei, reverse)| {
            let e = &curves.edges()[ei];
            // Unit single-span clocks make reversal an exact pole/weight reversal;
            // no rounded knot arithmetic may inherit the original proxy proof.
            for c in [&e.world, &e.pcurves[side]] {
                if c.domain() != [0., 1.] || c.knots.iter().any(|&k| k != 0. && k != 1.) {
                    return Err(unproven(
                        "Closed SS source clock reversal needs exact single-span unit knots",
                    ));
                }
            }
            Ok(Segment {
                curve: if reverse {
                    e.world.reverse()?
                } else {
                    e.world.clone()
                },
                pcurve: if reverse {
                    e.pcurves[side].reverse()?
                } else {
                    e.pcurves[side].clone()
                },
            })
        })
        .collect()
}
