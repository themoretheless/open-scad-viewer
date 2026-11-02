//! Complete same-body correspondence of declared poles to native graph disks.
use super::*;
pub(super) fn cover<'n, E: Carrier>(
    body: &Body<E>,
    native: &'n [super::super::C2PoleNeighborhood],
    out: &mut Report<'_, E>,
    max: usize,
) -> Option<BTreeMap<usize, &'n super::super::C2PoleNeighborhood>> {
    let shell = body.geometry().shell();
    let mut disks = BTreeMap::new();
    for n in native {
        if !step(out, max) {
            return None;
        }
        if !n.projected_radius_lower().is_finite()
            || n.projected_radius_lower() <= 0.
            || disks.insert(n.vertex(), n).is_some()
        {
            return None;
        }
    }
    let mut declared = std::collections::BTreeSet::new();
    for (a, pole) in shell.poles() {
        if !step(out, max) {
            return None;
        }
        let vertex = shell.vertices()[a.face][a.wire][a.edge][0];
        let Some(n) = disks.get(&vertex) else {
            out.reason = "source-complete-pole-disks-unproven";
            return None;
        };
        if n.point() != pole.point() || !n.faces().contains(&a.face) {
            return None;
        }
        declared.insert(vertex);
    }
    if declared.len() != disks.len() {
        return None;
    }
    for v in disks.keys() {
        if !step(out, max) {
            return None;
        }
        if !body
            .geometry()
            .topology()
            .links
            .iter()
            .any(|l| l.vertex == *v)
        {
            return None;
        }
    }
    Some(disks)
}
