//! Complete closed SS census, followed by certified finite-face placement of
//! every component. Candidate faces are selected only by whole-contour proofs.
use super::*;
use nurbs_core::ss_intersection as ss;
pub struct ClosedSsNetwork {
    pub models: [Model; 2],
    pub traces: Vec<ss::QualifiedClosedFiberTrace>,
    /// Current faces descended from the respective original support face.
    pub faces: [Vec<usize>; 2],
    /// Cut budgets apply per candidate finite face; at most (branches+1)
    /// candidates per component and side are attempted. Failed attempts never
    /// publish a partially modified model.
    pub placement_attempts: usize,
}
pub fn partition_all_closed_ss(
    first: &Model,
    first_face: usize,
    second: &Model,
    second_face: usize,
    limits: ClosedSsLimits,
) -> Result<ClosedSsNetwork> {
    first.validate()?;
    second.validate()?;
    let a = first
        .faces
        .get(first_face)
        .ok_or_else(|| invalid("Missing first closed SS network face"))?;
    let b = second
        .faces
        .get(second_face)
        .ok_or_else(|| invalid("Missing second closed SS network face"))?;
    if limits.trace.tolerance_world > first.tolerance_mm.min(second.tolerance_mm) {
        return Err(invalid(
            "Closed SS trace tolerance exceeds original model tolerance",
        ));
    }
    let source = ss::FiberSources::new(&a.surface, &b.surface)?;
    let owner = ss::construct_qualified_closed_ss_branches(&source, limits.branches)?;
    if owner.branches().is_empty() || owner.branches().len() > 64 {
        return Err(unproven("No bounded complete original closed census"));
    }
    let mut traces = Vec::new();
    for branch in 0..owner.branches().len() {
        let cycle = ss::order_closed_fiber_cycle(&owner, branch, limits.cycle)?;
        traces.push(ss::construct_closed_fiber_trace(&cycle, limits.trace)?);
    }
    let mut models = [first.clone(), second.clone()];
    let mut faces = [vec![first_face], vec![second_face]];
    let mut placement_attempts = 0;
    for trace in &traces {
        for support in 0..2 {
            let path: Vec<Segment> = trace
                .segments()
                .iter()
                .map(|s| Segment {
                    curve: s.curve().clone(),
                    pcurve: s.pcurves()[support].clone(),
                })
                .collect();
            let mut selected = None;
            for &face in &faces[support] {
                placement_attempts += 1;
                if let Ok(candidate) = closed::partition_closed(
                    &models[support],
                    face,
                    &path,
                    limits.trace.tolerance_uv,
                    limits.cut,
                ) {
                    if selected.is_some() {
                        return Err(unproven(
                            "Closed SS component has ambiguous finite-face ownership",
                        ));
                    }
                    selected = Some(candidate);
                }
            }
            let cut = selected.ok_or_else(|| {
                unproven("A complete original closed component has no proved finite-face placement")
            })?;
            faces[support].push(cut.faces[1]);
            models[support] = cut.model;
        }
    }
    for model in &models {
        model.validate()?;
        if !model.persistent_naming_complete() {
            return Err(unproven(
                "Closed SS network lost persistent topology identities",
            ));
        }
    }
    Ok(ClosedSsNetwork {
        models,
        traces,
        faces,
        placement_attempts,
    })
}
