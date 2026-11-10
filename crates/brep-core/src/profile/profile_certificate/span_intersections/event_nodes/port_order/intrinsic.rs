//! Intrinsic same-ray order; source parameter speed never selects topology.
use super::*;
use arithmetic::{branch_neighbourhood::ParameterValue, branch_slice::SliceBranch};
fn data(graph: &OwnedBranchNeighbourhood) -> Result<SliceBranch> {
    let branch = graph.branch();
    let r = branch.reference();
    let node = &branch.owner().nodes()[r.node].exact;
    let actual = cells(&graph.atlas().sources()[graph.source()])?;
    let cell = &actual[branch.source_cell()];
    let record = graph.record();
    Ok(SliceBranch {
        net: cell.net.clone(),
        domain: cell.domain.clone(),
        direction: graph.direction(),
        contact: ParameterValue {
            primitive: decode_polynomial(&node.primitive)?,
            value: decode_polynomial(&node.parameters[r.source])?,
            interval: [
                decode_ratio(&record.refined_primitive_root[0])?,
                decode_ratio(&record.refined_primitive_root[1])?,
            ],
        },
        cut: decode_ratio(&record.natural_cut)?,
        cut_point: [
            decode_ratio(&record.cut_position[0])?,
            decode_ratio(&record.cut_position[1])?,
        ],
    })
}
pub(super) fn compare(
    atlas: &OriginalContactAtlas,
    a: &OriginalContactPort,
    b: &OriginalContactPort,
    l: ContactPortOrderLimits,
) -> Result<Ordering> {
    let axis = usize::from(a.branch.ray().signs[0] == 0);
    let sign = a.branch.ray().signs[axis];
    if sign == 0 || b.branch.ray().signs[axis] != sign {
        return Err(refused(
            "Tangent ports have incompatible actual leading directions",
        ));
    }
    let limits = BranchNeighbourhoodLimits {
        germ_refinements: l.algebraic.max_refinements,
        max_steps: l.max_neighbourhood_steps,
    };
    let atlas = Arc::new(atlas.clone());
    let a = own_branch_neighbourhood(
        atlas.clone(),
        a.source,
        a.parameter_group,
        a.direction,
        axis,
        limits,
    )?;
    let b = own_branch_neighbourhood(
        atlas,
        b.source,
        b.parameter_group,
        b.direction,
        axis,
        limits,
    )?;
    arithmetic::branch_slice::compare(
        data(&a)?,
        data(&b)?,
        axis,
        sign,
        l.algebraic.max_refinements,
        l.algebraic.max_resultant_size,
    )
}
pub(super) fn transverse_sign(port: &OriginalContactPort, max_refinements: usize) -> Result<i8> {
    let branch = &port.branch;
    let r = branch.reference();
    let node = &branch.owner().nodes()[r.node].exact;
    let actual = cells(&branch.owner().sources()[r.source])?;
    let cell = &actual[branch.source_cell()];
    arithmetic::branch_germ::component_sign(
        &cell.net,
        &cell.domain,
        &decode_polynomial(&node.parameters[r.source])?,
        &decode_polynomial(&node.primitive)?,
        &[
            decode_ratio(&node.isolating_interval[0])?,
            decode_ratio(&node.isolating_interval[1])?,
        ],
        branch.direction(),
        1,
        max_refinements,
    )
}
