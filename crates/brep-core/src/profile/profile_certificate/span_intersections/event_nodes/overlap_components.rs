//! Full original common-image cell components, with independent source clocks.
mod periodic_embedding;
pub use periodic_embedding::{OwnedOriginalPeriodicEmbedding, own_original_periodic_embedding};
mod periodic_wires;
pub use periodic_wires::{
    OriginalPeriodicJoinKind, OriginalPeriodicSourceUse, OriginalPeriodicWire,
    OriginalPeriodicWireJoin, OriginalPeriodicWireUse, OwnedOriginalPeriodicWires,
    own_original_periodic_wires,
};
mod periodic_surface;
pub use periodic_surface::{
    OriginalPeriodicPieceCoverage, OwnedOriginalPeriodicSurface,
    bind_original_periodic_pieces_to_surface,
};
mod periodic_pieces;
pub use periodic_pieces::{
    OriginalPeriodicCut, OriginalPeriodicCutOrigin, OriginalPeriodicEvent, OriginalPeriodicPiece,
    OriginalPeriodicPoint, OriginalPeriodicShoreCell, OriginalPeriodicTransition,
    OwnedOriginalPeriodicPieces, own_original_periodic_pieces,
};
mod chart_intervals;
pub use chart_intervals::{
    OriginalChartCut, OriginalChartCutOrigin, OriginalChartInterval, OriginalChartRegion,
    OwnedOriginalChartIntervals, own_original_chart_intervals,
};
mod chart_events;
pub use chart_events::{
    OriginalChartShoreCell, OriginalChartShoreEvent, OwnedOriginalChartEvents,
    own_original_chart_events,
};
mod unified_regions;
pub use unified_regions::{OwnedOriginalUnifiedRegions, own_original_unified_regions};
mod unified_boundary_winding;
pub use unified_boundary_winding::{
    OwnedOriginalUnifiedBoundaryWinding, own_original_unified_boundary_winding,
};
mod unified_filled;
pub use unified_filled::{OwnedOriginalUnifiedFilled, own_original_unified_filled};
mod unified_sides;
pub use unified_sides::{
    OwnedOriginalUnifiedSides, own_original_unified_sides, own_original_unified_sides_automatically,
};
mod unified_cycles;
pub use unified_cycles::{OwnedOriginalUnifiedCycles, own_original_unified_cycles};
mod unified_rotation;
pub use unified_rotation::{OwnedOriginalUnifiedRotation, own_original_unified_rotation};
mod unified_ports;
pub use unified_ports::{
    OriginalUnifiedPort, OwnedOriginalUnifiedPorts, own_original_unified_ports,
};
mod unified_graph;
pub use unified_graph::{
    OriginalUnifiedEdge, OriginalUnifiedEdgeCarrier, OriginalUnifiedIncidence,
    OriginalUnifiedVertex, OwnedOriginalUnifiedGraph, own_original_unified_graph,
};
mod source_carriers;
pub use source_carriers::{
    OriginalSourceCommonUse, OriginalSourceCut, OriginalSourceCutOrigin, OriginalSourceParameter,
    OriginalUnifiedCarrier, OwnedOriginalSourceCarriers, own_original_source_carriers,
};
mod dictionary;
pub use dictionary::{
    OriginalCommonEdge, OriginalCommonEdgeUse, OriginalCommonPartitionRef,
    OwnedOriginalCommonDictionary, own_original_common_dictionary,
};
mod contributors;
pub use contributors::{
    OriginalCommonContributor, OriginalCommonCut, OriginalCommonCutOrigin, OriginalCommonSegment,
    OwnedOriginalCommonPartition, own_original_common_partition,
};
mod common_clip;
pub use common_clip::{
    OriginalCommonClip, OriginalCommonClipBoundary, OriginalCommonClipOrigin,
    OwnedOriginalCommonClips, own_original_common_clips,
};
mod point_pullback;
pub use point_pullback::{
    OriginalPointPreimage, OwnedOriginalPointPullback, own_original_point_pullback,
};
mod pair_cuts;
pub use pair_cuts::{
    OriginalPairCut, OriginalPairCutOrigin, OwnedOriginalPairCuts, own_original_pair_cuts,
};
mod multi_partition;
use super::*;
pub use multi_partition::{
    OriginalOverlapContactRef, OwnedOriginalOverlapPartition, own_original_overlap_partition,
};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalOverlapComponent {
    pub pair: usize,
    pub source_cells: [usize; 2],
    pub axis: usize,
    pub image_range: [ExactRatio; 2],
    /// Lower then upper projected image. Each node owns both natural cuts.
    pub endpoints: [AlgebraicEventNode; 2],
    /// Increasing natural parameter versus increasing projected image.
    pub directions: [i8; 2],
}
impl OriginalOverlapComponent {
    pub fn reversed(&self) -> bool {
        self.directions[0] != self.directions[1]
    }
    /// Signed contributions in the common increasing-image orientation.
    /// Values are not inferred from geometry: callers supply their wire labels.
    pub fn oriented_contributions(&self, weights: [i64; 2]) -> Result<[i64; 2]> {
        let mut out = [0; 2];
        for i in 0..2 {
            out[i] = weights[i]
                .checked_mul(self.directions[i].into())
                .ok_or_else(|| refused("Original common-carrier contribution overflow"))?;
        }
        Ok(out)
    }
}
#[derive(Clone, Debug)]
pub struct OwnedOriginalOverlapComponents {
    owner: Arc<CurveEventNodes>,
    components: Vec<OriginalOverlapComponent>,
}
fn derive(owner: &CurveEventNodes) -> Result<Vec<OriginalOverlapComponent>> {
    // Endpoint derivation replays every original pair, including both coordinate
    // divisibilities and complete common monotone image coverage.
    let endpoints = owner.overlap_endpoints()?;
    let a = cells(&owner.sources()[0])?;
    let b = cells(&owner.sources()[1])?;
    let mut out = Vec::new();
    for (pair, ends) in endpoints {
        let p = &owner.census().pairs[pair];
        let left = &a[p.first];
        let right = &b[p.second];
        let axis = p.axis as usize;
        let (lo, hi, _) = range(left, right, axis)?;
        let direction = |cell: &Cell| -> Result<i8> {
            let first = point(&cell.net, 0)[axis].clone();
            let last = point(&cell.net, 1)[axis].clone();
            if last == first {
                return Err(refused("Common-image source direction is degenerate"));
            }
            Ok(if last > first { 1 } else { -1 })
        };
        out.push(OriginalOverlapComponent {
            pair,
            source_cells: [p.first, p.second],
            axis,
            image_range: [ratio(&lo), ratio(&hi)],
            endpoints: ends,
            directions: [direction(left)?, direction(right)?],
        });
    }
    Ok(out)
}
impl OwnedOriginalOverlapComponents {
    pub fn owner(&self) -> &Arc<CurveEventNodes> {
        &self.owner
    }
    pub fn components(&self) -> &[OriginalOverlapComponent] {
        &self.components
    }
    pub fn reverify_source(&self) -> Result<()> {
        if self.components != derive(&self.owner)? {
            return Err(refused("Original common-image component ownership changed"));
        }
        Ok(())
    }
}
/// Own every original overlap cell component, not just matching endpoints.
/// This keeps partial overlaps and reversed/nonlinear source clocks exact.
/// Global third-curve subdivision and merging of multiple contributors require
/// a further arrangement owner; this does not publish Boolean topology.
pub fn own_original_overlap_components(
    owner: Arc<CurveEventNodes>,
) -> Result<OwnedOriginalOverlapComponents> {
    let components = derive(&owner)?;
    Ok(OwnedOriginalOverlapComponents { owner, components })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replay_rejects_component_endpoint_direction_and_census_tampering() {
        let a = Curve::from_polyline(vec![vec![0., 0.], vec![2., 0.]]).unwrap();
        let b = Curve::from_polyline(vec![vec![3., 0.], vec![1., 0.]]).unwrap();
        let census = certify_span_intersections(&a, &b, false, 1e-12).unwrap();
        let o = own_original_overlap_components(Arc::new(
            own_curve_event_nodes(&a, &b, &census).unwrap(),
        ))
        .unwrap();
        o.reverify_source().unwrap();
        let mut bad = o.clone();
        bad.components.clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = o.clone();
        bad.components[0].directions[1] = 1;
        assert!(bad.reverify_source().is_err());
        let mut bad = o.clone();
        bad.components[0].endpoints.swap(0, 1);
        assert!(bad.reverify_source().is_err());
        let mut bad = o.clone();
        bad.components[0].source_cells[0] = 99;
        assert!(bad.reverify_source().is_err());
        let mut bad = o.clone();
        bad.components[0].image_range[0] = ["0".into(), "1".into()];
        assert!(bad.reverify_source().is_err());
        assert!(
            o.components()[0]
                .oriented_contributions([0, i64::MIN])
                .is_err()
        );
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalOverlapContact {
    /// Node references in the two original source/third-curve pair owners.
    pub nodes: [usize; 2],
    /// Exact common-component endpoint, or None for an interior split.
    pub endpoint: Option<usize>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OriginalOverlapSplit {
    Endpoint(usize),
    ContactGroup(usize),
}
#[derive(Clone, Debug)]
pub struct OwnedOriginalOverlapContacts {
    overlap: Arc<OwnedOriginalOverlapComponents>,
    component: usize,
    contacts: [Arc<CurveEventNodes>; 2],
    limits: ParameterOrderLimits,
    cuts: Vec<OriginalOverlapContact>,
    groups: Vec<Vec<usize>>,
    segments: Vec<[OriginalOverlapSplit; 2]>,
}
fn parameter_cmp(
    a: &AlgebraicEventNode,
    side_a: usize,
    b: &AlgebraicEventNode,
    side_b: usize,
    l: ParameterOrderLimits,
) -> Result<std::cmp::Ordering> {
    use super::super::super::algebraic_intersections::{decode_polynomial, decode_ratio};
    let x = &a.exact;
    let y = &b.exact;
    arithmetic::parameter_order::compare(
        &decode_polynomial(&x.primitive)?,
        &decode_polynomial(&x.parameters[side_a])?,
        &[
            decode_ratio(&x.isolating_interval[0])?,
            decode_ratio(&x.isolating_interval[1])?,
        ],
        &decode_polynomial(&y.primitive)?,
        &decode_polynomial(&y.parameters[side_b])?,
        &[
            decode_ratio(&y.isolating_interval[0])?,
            decode_ratio(&y.isolating_interval[1])?,
        ],
        l.max_refinements,
        l.max_resultant_size,
    )
}
fn contact_cuts(
    overlap: &OwnedOriginalOverlapComponents,
    component: usize,
    contacts: &[Arc<CurveEventNodes>; 2],
    l: ParameterOrderLimits,
) -> Result<Vec<OriginalOverlapContact>> {
    use std::cmp::Ordering;
    if l.max_refinements > 512 || !(1..=128).contains(&l.max_resultant_size) {
        return Err(refused("Invalid common-carrier contact comparison budget"));
    }
    overlap.reverify_source()?;
    let c = overlap
        .components()
        .get(component)
        .ok_or_else(|| refused("Unknown original overlap component"))?;
    for side in 0..2 {
        contacts[side].reverify_source()?;
        if contacts[side].sources()[0] != overlap.owner().sources()[side]
            || contacts[side].sources()[1] != contacts[1 - side].sources()[1]
            || contacts[side].census().self_pair
            || contacts[side]
                .census()
                .pairs
                .iter()
                .any(|p| p.kind == "overlap")
        {
            return Err(refused(
                "Common-carrier contacts require two complete original source/third-curve isolated censuses",
            ));
        }
    }
    let mut selected = [Vec::new(), Vec::new()];
    for side in 0..2 {
        let ends = if c.directions[side] > 0 {
            [0, 1]
        } else {
            [1, 0]
        };
        for (i, n) in contacts[side].nodes().iter().enumerate() {
            let lo = parameter_cmp(n, 0, &c.endpoints[ends[0]], side, l)?;
            let hi = parameter_cmp(n, 0, &c.endpoints[ends[1]], side, l)?;
            if lo != Ordering::Less && hi != Ordering::Greater {
                let endpoint = if lo == Ordering::Equal {
                    Some(ends[0])
                } else if hi == Ordering::Equal {
                    Some(ends[1])
                } else {
                    None
                };
                selected[side].push((i, endpoint));
            }
        }
    }
    if selected[0].len() > 4096 || selected[1].len() > 4096 {
        return Err(refused("Common-carrier contact budget exceeded"));
    }
    if selected[0].len() != selected[1].len() {
        return Err(refused("Common-carrier contact censuses disagree"));
    }
    if selected[0].len().saturating_mul(selected[1].len()) > 65536 {
        return Err(refused("Common-carrier contact comparison budget exceeded"));
    }
    let mut used = vec![false; selected[1].len()];
    let mut out = Vec::new();
    for &(a, endpoint) in &selected[0] {
        let mut matches = Vec::new();
        for (j, &(b, other_endpoint)) in selected[1].iter().enumerate() {
            // Same original third-source parameter proves the same spatial
            // event; parameter boxes or rounded coordinates never do.
            if parameter_cmp(&contacts[0].nodes()[a], 1, &contacts[1].nodes()[b], 1, l)?
                == Ordering::Equal
            {
                if endpoint != other_endpoint {
                    return Err(refused("Common-carrier endpoint correspondence disagrees"));
                }
                matches.push((j, b));
            }
        }
        if matches.len() != 1 {
            return Err(refused(
                "Common-carrier third-source contact correspondence is not unique",
            ));
        }
        let (j, b) = matches[0];
        if used[j] {
            return Err(refused("Common-carrier contact used more than once"));
        }
        used[j] = true;
        let cut = OriginalOverlapContact {
            nodes: [a, b],
            endpoint,
        };
        let mut pos = out.len();
        while pos > 0 {
            let previous: &OriginalOverlapContact = &out[pos - 1];
            let order = parameter_cmp(
                &contacts[0].nodes()[previous.nodes[0]],
                0,
                &contacts[0].nodes()[a],
                0,
                l,
            )?;
            if order == Ordering::Equal {
                break;
            }
            if (order == Ordering::Less) == (c.directions[0] > 0) {
                break;
            }
            pos -= 1;
        }
        out.insert(pos, cut);
    }
    if used.iter().any(|v| !v) {
        return Err(refused(
            "Common-carrier contact census has an unmatched traversal",
        ));
    }
    Ok(out)
}
fn partition_contacts(
    cuts: &[OriginalOverlapContact],
    contacts: &[Arc<CurveEventNodes>; 2],
    limits: ParameterOrderLimits,
) -> Result<(Vec<Vec<usize>>, Vec<[OriginalOverlapSplit; 2]>)> {
    use std::cmp::Ordering;
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for (index, cut) in cuts.iter().enumerate() {
        let same = if let Some(previous) = groups.last() {
            let prior = &cuts[previous[0]];
            let a = parameter_cmp(
                &contacts[0].nodes()[prior.nodes[0]],
                0,
                &contacts[0].nodes()[cut.nodes[0]],
                0,
                limits,
            )?;
            let b = parameter_cmp(
                &contacts[1].nodes()[prior.nodes[1]],
                0,
                &contacts[1].nodes()[cut.nodes[1]],
                0,
                limits,
            )?;
            if (a == Ordering::Equal) != (b == Ordering::Equal) {
                return Err(refused(
                    "Common-carrier split equality disagrees between source clocks",
                ));
            }
            if a == Ordering::Equal && prior.endpoint != cut.endpoint {
                return Err(refused("Common-carrier split endpoint identity disagrees"));
            }
            a == Ordering::Equal
        } else {
            false
        };
        if same {
            groups.last_mut().unwrap().push(index);
        } else {
            groups.push(vec![index]);
        }
    }
    let mut bounds = vec![OriginalOverlapSplit::Endpoint(0)];
    for (group, occurrences) in groups.iter().enumerate() {
        if cuts[occurrences[0]].endpoint.is_none() {
            bounds.push(OriginalOverlapSplit::ContactGroup(group));
        }
    }
    bounds.push(OriginalOverlapSplit::Endpoint(1));
    let segments = bounds.windows(2).map(|b| [b[0], b[1]]).collect();
    Ok((groups, segments))
}
impl OwnedOriginalOverlapContacts {
    pub fn overlap(&self) -> &Arc<OwnedOriginalOverlapComponents> {
        &self.overlap
    }
    pub fn component(&self) -> usize {
        self.component
    }
    pub fn contacts(&self) -> &[Arc<CurveEventNodes>; 2] {
        &self.contacts
    }
    /// Complete contact correspondence, ordered by increasing common image.
    pub fn cuts(&self) -> &[OriginalOverlapContact] {
        &self.cuts
    }
    /// All third-source traversals at each exact physical cut, retaining multiplicity.
    pub fn groups(&self) -> &[Vec<usize>] {
        &self.groups
    }
    /// Complete nonempty partition in increasing common-image orientation.
    pub fn segments(&self) -> &[[OriginalOverlapSplit; 2]] {
        &self.segments
    }
    /// The original algebraic position plus the index of its natural parameter.
    /// A contact cut uses its source/third pair's parameter zero; an overlap
    /// endpoint contains both common-source parameters in one joint extension.
    pub fn split_parameter(
        &self,
        split: OriginalOverlapSplit,
        source: usize,
    ) -> Result<(&AlgebraicEventPosition, usize)> {
        if source > 1 {
            return Err(refused("Invalid common-carrier source"));
        }
        match split {
            OriginalOverlapSplit::Endpoint(i) => {
                let node = self.overlap.components()[self.component]
                    .endpoints
                    .get(i)
                    .ok_or_else(|| refused("Invalid common-carrier endpoint"))?;
                Ok((&node.exact, source))
            }
            OriginalOverlapSplit::ContactGroup(g) => {
                let i = *self
                    .groups
                    .get(g)
                    .and_then(|v| v.first())
                    .ok_or_else(|| refused("Invalid common-carrier contact group"))?;
                let node = &self.contacts[source].nodes()[self.cuts[i].nodes[source]];
                Ok((&node.exact, 0))
            }
        }
    }
    pub fn reverify_source(&self) -> Result<()> {
        if self.cuts != contact_cuts(&self.overlap, self.component, &self.contacts, self.limits)? {
            return Err(refused("Original common-carrier contact cuts changed"));
        }
        let (groups, segments) = partition_contacts(&self.cuts, &self.contacts, self.limits)?;
        if groups != self.groups || segments != self.segments {
            return Err(refused("Original common-carrier partition changed"));
        }
        Ok(())
    }
}
pub fn own_original_overlap_contacts(
    overlap: Arc<OwnedOriginalOverlapComponents>,
    component: usize,
    contacts: [Arc<CurveEventNodes>; 2],
    limits: ParameterOrderLimits,
) -> Result<OwnedOriginalOverlapContacts> {
    let cuts = contact_cuts(&overlap, component, &contacts, limits)?;
    let (groups, segments) = partition_contacts(&cuts, &contacts, limits)?;
    Ok(OwnedOriginalOverlapContacts {
        overlap,
        component,
        contacts,
        limits,
        cuts,
        groups,
        segments,
    })
}

#[cfg(test)]
mod contact_audits {
    use super::*;
    #[test]
    fn replay_rejects_contact_omission_endpoint_and_source_rebinding() {
        let a = Curve::from_polyline(vec![vec![0., 0.], vec![2., 0.]]).unwrap();
        let b = Curve::from_polyline(vec![vec![3., 0.], vec![1., 0.]]).unwrap();
        let t = Curve::from_polyline(vec![vec![1.5, -1.], vec![1.5, 1.]]).unwrap();
        let nodes = |a: &Curve, b: &Curve| {
            let c = certify_span_intersections(a, b, false, 1e-12).unwrap();
            Arc::new(own_curve_event_nodes(a, b, &c).unwrap())
        };
        let o = Arc::new(own_original_overlap_components(nodes(&a, &b)).unwrap());
        let x = own_original_overlap_contacts(
            o,
            0,
            [nodes(&a, &t), nodes(&b, &t)],
            ParameterOrderLimits::default(),
        )
        .unwrap();
        x.reverify_source().unwrap();
        let mut bad = x.clone();
        bad.cuts.clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.cuts[0].endpoint = Some(0);
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.cuts[0].nodes[0] = 99;
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.contacts.swap(0, 1);
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.groups.clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.segments[0].swap(0, 1);
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.segments.pop();
        assert!(bad.reverify_source().is_err());
    }
}
