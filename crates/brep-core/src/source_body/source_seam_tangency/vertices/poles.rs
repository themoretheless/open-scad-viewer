//! Complete same-body correspondence of declared poles to native graph disks.
use super::*;
pub(super) enum Disk<'n> {
    C1(&'n super::super::C1PoleNeighborhood),
    C2(&'n super::super::C2PoleNeighborhood),
}
impl Disk<'_> {
    pub fn order(&self) -> usize {
        match self {
            Self::C1(_) => 1,
            Self::C2(_) => 2,
        }
    }
    pub fn vertex(&self) -> usize {
        match self {
            Self::C1(n) => n.vertex(),
            Self::C2(n) => n.vertex(),
        }
    }
    pub fn dropped_axis(&self) -> usize {
        match self {
            Self::C1(n) => n.dropped_axis(),
            Self::C2(n) => n.dropped_axis(),
        }
    }
    pub fn point(&self) -> [f64; 3] {
        match self {
            Self::C1(n) => n.point(),
            Self::C2(n) => n.point(),
        }
    }
    pub fn projected_radius_lower(&self) -> f64 {
        match self {
            Self::C1(n) => n.projected_radius_lower(),
            Self::C2(n) => n.projected_radius_lower(),
        }
    }
    pub fn faces(&self) -> &[usize] {
        match self {
            Self::C1(n) => n.faces(),
            Self::C2(n) => n.faces(),
        }
    }
}
pub(super) fn cover<'n, E: Carrier>(
    body: &Body<E>,
    native: &'n [super::super::C2PoleNeighborhood],
    c1: &'n [super::super::C1PoleNeighborhood],
    out: &mut Report<'_, E>,
    max: usize,
) -> Option<BTreeMap<usize, Disk<'n>>> {
    let shell = body.geometry().shell();
    let mut disks = BTreeMap::new();
    for n in native.iter().map(Disk::C2).chain(c1.iter().map(Disk::C1)) {
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
