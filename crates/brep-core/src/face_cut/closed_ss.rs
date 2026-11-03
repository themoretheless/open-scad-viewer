//! Original-source closed SS cycles to paired finite-face BRep subdivisions.
//! Both source faces retain both sectors. This does not classify material.
use super::*;
use nurbs_core::ss_intersection as ss;
#[derive(Clone, Copy, Debug, Default)]
pub struct ClosedSsLimits {
    pub branches: ss::ClosedSSBuildLimits,
    pub cycle: ss::ClosedFiberCycleLimits,
    pub trace: ss::ClosedFiberTraceLimits,
    pub cut: ClosedCutLimits,
}
pub struct ClosedSsPartition {
    pub first: ClosedPartition,
    pub second: ClosedPartition,
    pub trace: ss::QualifiedClosedFiberTrace,
}
/// Partition one wholly interior regular closed original SS component on both
/// finite faces. All natural-product roots must belong to this one component;
/// extra components, face-boundary contacts and singularities remain refusals.
pub fn partition_closed_ss(
    first: &Model,
    first_face: usize,
    second: &Model,
    second_face: usize,
    limits: ClosedSsLimits,
) -> Result<ClosedSsPartition> {
    first.validate()?;
    second.validate()?;
    let a = first
        .faces
        .get(first_face)
        .ok_or_else(|| invalid("Missing first closed SS face"))?;
    let b = second
        .faces
        .get(second_face)
        .ok_or_else(|| invalid("Missing second closed SS face"))?;
    if limits.trace.tolerance_world > first.tolerance_mm.min(second.tolerance_mm) {
        return Err(invalid(
            "Closed SS trace tolerance exceeds original model tolerance",
        ));
    }
    let source = ss::FiberSources::new(&a.surface, &b.surface)?;
    let owner = ss::construct_qualified_closed_ss_branches(&source, limits.branches)?;
    if owner.branches().len() != 1 {
        return Err(unproven(
            "Paired closed SS cut requires exactly one complete original component",
        ));
    }
    let cycle = ss::order_closed_fiber_cycle(&owner, 0, limits.cycle)?;
    let trace = ss::construct_closed_fiber_trace(&cycle, limits.trace)?;
    let paths: [Vec<Segment>; 2] = std::array::from_fn(|k| {
        trace
            .segments()
            .iter()
            .map(|s| Segment {
                curve: s.curve().clone(),
                pcurve: s.pcurves()[k].clone(),
            })
            .collect()
    });
    let first = closed::partition_closed(
        first,
        first_face,
        &paths[0],
        limits.trace.tolerance_uv,
        limits.cut,
    )?;
    let second = closed::partition_closed(
        second,
        second_face,
        &paths[1],
        limits.trace.tolerance_uv,
        limits.cut,
    )?;
    Ok(ClosedSsPartition {
        first,
        second,
        trace,
    })
}
