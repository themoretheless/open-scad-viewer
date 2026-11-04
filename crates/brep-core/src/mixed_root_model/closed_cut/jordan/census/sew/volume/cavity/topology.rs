use super::*;
pub(super) fn assemble(
    outer: &closed::QualifiedClosedSolid,
    inner: &QualifiedPairedRootSolid,
) -> Result<(
    MixedBrep,
    [crate::mixed_root_model::MixedTopologyMap; 2],
    MixedBrep,
)> {
    let original = MixedBrep::from_closed_solid(outer)?;
    let (joined, mut lineage) = original.concatenate(inner.model())?.into_parts();
    let mut t = joined.topology().clone();
    let source = inner.model().topology();
    if !source.bodies[0].inner_shells.is_empty() {
        return Err(unproved("Paired cavity source has foreign nested shells"));
    }
    for shell in 0..source.shells.len() {
        lineage[1].reverse_shell(shell)?;
        let target = lineage[1]
            .shell(shell)
            .ok_or_else(|| unproved("Missing cavity shell transfer"))?;
        for f in &mut t.shells[target].faces {
            f.reversed = !f.reversed;
        }
    }
    let cavity = lineage[1]
        .shell(source.bodies[0].outer_shell)
        .ok_or_else(|| unproved("Missing cavity outer shell transfer"))?;
    let removed = lineage[1]
        .body(0)
        .ok_or_else(|| unproved("Missing cavity body transfer"))?;
    if removed + 1 != t.bodies.len() {
        return Err(unproved("Cavity body transfer has foreign owners"));
    }
    t.bodies.pop();
    t.bodies[0].inner_shells.push(cavity);
    lineage[1].reparent_body(0, 0)?;
    let model = MixedBrep::from_topology(t)?;
    for map in &mut lineage {
        map.retarget(&model);
    }
    Ok((model, lineage, original))
}
