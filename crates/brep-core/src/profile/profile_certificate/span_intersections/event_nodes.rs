//! Source-owned algebraic event positions. Exact factors/maps stay attached to
//! both ORIGINAL cells; rounded event parameter enclosures are not node identity.
mod overlap_components;
pub use overlap_components::{
    OriginalChartCut, OriginalChartCutOrigin, OriginalChartInterval, OriginalChartRegion,
    OriginalChartShoreCell, OriginalChartShoreEvent, OriginalCommonClip,
    OriginalCommonClipBoundary, OriginalCommonClipOrigin, OriginalCommonContributor,
    OriginalCommonCut, OriginalCommonCutOrigin, OriginalCommonEdge, OriginalCommonEdgeUse,
    OriginalCommonPartitionRef, OriginalCommonSegment, OriginalOverlapComponent,
    OriginalOverlapContact, OriginalOverlapContactRef, OriginalOverlapSplit, OriginalPairCut,
    OriginalPairCutOrigin, OriginalPeriodicCut, OriginalPeriodicCutOrigin, OriginalPeriodicEvent,
    OriginalPeriodicJoinKind, OriginalPeriodicPiece, OriginalPeriodicPieceCoverage,
    OriginalPeriodicPoint, OriginalPeriodicShoreCell, OriginalPeriodicSourceUse,
    OriginalPeriodicTransition, OriginalPeriodicWire, OriginalPeriodicWireJoin,
    OriginalPeriodicWireUse, OriginalPointPreimage, OriginalSourceCommonUse, OriginalSourceCut,
    OriginalSourceCutOrigin, OriginalSourceParameter, OriginalUnifiedCarrier, OriginalUnifiedEdge,
    OriginalUnifiedEdgeCarrier, OriginalUnifiedIncidence, OriginalUnifiedPort,
    OriginalUnifiedVertex, OwnedOriginalChartEvents, OwnedOriginalChartIntervals,
    OwnedOriginalCommonClips, OwnedOriginalCommonDictionary, OwnedOriginalCommonPartition,
    OwnedOriginalOverlapComponents, OwnedOriginalOverlapContacts, OwnedOriginalOverlapPartition,
    OwnedOriginalPairCuts, OwnedOriginalPeriodicEmbedding, OwnedOriginalPeriodicPieces,
    OwnedOriginalPeriodicSurface, OwnedOriginalPeriodicWires, OwnedOriginalPointPullback,
    OwnedOriginalSourceCarriers, OwnedOriginalUnifiedBoundaryWinding, OwnedOriginalUnifiedCycles,
    OwnedOriginalUnifiedFilled, OwnedOriginalUnifiedGraph, OwnedOriginalUnifiedPorts,
    OwnedOriginalUnifiedRegions, OwnedOriginalUnifiedRotation, OwnedOriginalUnifiedSides,
    bind_original_periodic_pieces_to_surface, own_original_chart_events,
    own_original_chart_intervals, own_original_common_clips, own_original_common_dictionary,
    own_original_common_partition, own_original_overlap_components, own_original_overlap_contacts,
    own_original_overlap_partition, own_original_pair_cuts, own_original_periodic_embedding,
    own_original_periodic_pieces, own_original_periodic_wires, own_original_point_pullback,
    own_original_source_carriers, own_original_unified_boundary_winding,
    own_original_unified_cycles, own_original_unified_filled, own_original_unified_graph,
    own_original_unified_ports, own_original_unified_regions, own_original_unified_rotation,
    own_original_unified_sides, own_original_unified_sides_automatically,
};
mod physical_faces;
pub use physical_faces::{
    OriginalPhysicalEdge, OriginalPhysicalEdgeUse, OriginalPhysicalFace,
    OriginalPhysicalFaceLimits, OriginalPhysicalFacePatch, OriginalPhysicalTrace,
    OriginalPhysicalVertex, OwnedOriginalPhysicalFaces, assemble_original_physical_faces,
    own_original_physical_faces,
};
mod atlas_pairs;
pub use atlas_pairs::{
    OriginalAtlasPair, OriginalAtlasPairLimits, OriginalAtlasPairProof, OwnedOriginalAtlasPairs,
    OwnedOriginalGlobalChartEmbedding, own_original_atlas_pairs,
    own_original_global_chart_embedding,
};
mod injective_atlas;
pub use injective_atlas::{
    OriginalInjectiveAtlasLimits, OriginalInjectiveChart, OwnedOriginalInjectiveChartAtlas,
    own_original_injective_chart_atlas,
};
mod surface_regularity;
pub use surface_regularity::{
    OriginalSurfaceRegularityRecord, OwnedOriginalRegularTrimSurface,
    own_original_regular_trim_surface,
};
mod chart_coverage;
pub use chart_coverage::{
    OriginalChartCoverageCell, OwnedOriginalTrimChartCoverage, own_original_trim_chart_coverage,
};
mod periodic_topology;
pub use periodic_topology::{
    OriginalQuotientCoedge, OriginalQuotientEdge, OriginalQuotientVertexLink,
    OwnedOriginalPeriodicTopology, own_original_periodic_topology,
};
mod surface_seams;
pub use surface_seams::{OwnedOriginalSurfaceSeams, bind_original_seams_to_surface};
mod seam_dictionary;
pub use seam_dictionary::{
    OriginalSeamUse, OwnedOriginalSeamDictionary, own_original_seam_dictionary,
};
mod shore_carrier;
pub use shore_carrier::{
    OriginalShoreMembership, OwnedOriginalShoreCarrier, OwnedOriginalShoreMembership,
    OwnedOriginalShorePair, classify_original_shore_carrier, own_original_shore_carrier,
    own_original_shore_pair,
};
mod chart_vertices;
pub use chart_vertices::{
    OriginalChartVertexLimits, OwnedOriginalChartVertices, own_original_chart_vertices,
};
mod filled_regions;
pub use filled_regions::{
    OriginalFilledRegion, OwnedOriginalFilledRegions, assemble_original_filled_regions,
    own_original_filled_regions,
};
mod boundary_winding;
pub use boundary_winding::{OwnedOriginalBoundaryWinding, own_original_boundary_winding};
mod automatic_boundaries;
pub use automatic_boundaries::{
    OriginalBoundaryAssemblyLimits, assemble_original_filled_boundaries,
    own_original_carrier_sides_automatically,
};
mod filled_boundaries;
pub use filled_boundaries::{OwnedOriginalFilledBoundaries, own_original_filled_boundaries};
mod carrier_sides;
pub use carrier_sides::{OwnedOriginalCarrierSides, own_original_carrier_sides};
mod carrier_cycles;
pub use carrier_cycles::{OwnedOriginalCarrierCycles, own_original_carrier_cycles};
mod carrier_graph;
pub use carrier_graph::{
    OriginalCarrierIncidence, OriginalCarrierVertex, OwnedOriginalCarrierGraph,
    own_original_carrier_graph,
};
mod carriers;
use super::super::algebraic_intersections::{ExactPolynomial, polynomial};
use super::*;
pub use carriers::{
    OriginalAtomicCarrier, OriginalCarrierCut, OwnedOriginalCarriers, own_original_carriers,
};
mod atlas;
mod branches;
mod neighbourhood;
pub use neighbourhood::{
    BranchNeighbourhoodLimits, BranchNeighbourhoodRecord, OwnedBranchNeighbourhood,
    own_branch_neighbourhood,
};
pub(super) mod port_order;
pub use branches::{LeadingBranchRay, OwnedEventBranch, own_event_branch, try_own_event_branch};
pub use port_order::{
    ContactPortOrderLimits, OriginalContactPort, OwnedContactPortOrder,
    own_original_contact_port_order,
};
mod parameter_order;
use arithmetic::event_coordinates as coordinates;
pub use atlas::{
    ContactAtlasLimits, ContactOccurrence, OriginalContactAtlas, SourceContactParameter,
    assemble_original_contact_atlas,
};
pub use parameter_order::{
    EventParameterRef, ParameterOrderLimits, QualifiedParameterOrder, compare_event_parameters,
};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlgebraicEventPosition {
    /// Coefficients in ascending primitive-root powers, exact ratios.
    pub primitive: ExactPolynomial,
    pub isolating_interval: [ExactRatio; 2],
    /// Actual natural-domain parameters of the original two NURBS sources.
    pub parameters: [ExactPolynomial; 2],
    /// Each coordinate is numerator(alpha)/denominator(alpha). No rounded point.
    pub position: [[ExactPolynomial; 2]; 2],
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlgebraicEventNode {
    pub source_cells: [usize; 2],
    pub pair: usize,
    pub factor: Option<usize>,
    pub root: usize,
    pub stationary: bool,
    pub exact: AlgebraicEventPosition,
}
#[derive(Clone, Debug)]
pub struct CurveEventNodes {
    sources: [Curve; 2],
    census: SpanIntersectionCertificate,
    nodes: Vec<AlgebraicEventNode>,
}
fn make(
    a: &Cell,
    b: &Cell,
    f: &Vec<Q>,
    t: &Vec<Q>,
    k: u8,
    interval: &[Q; 2],
    pair: usize,
    source_cells: [usize; 2],
    factor: Option<usize>,
    root: usize,
    stationary: bool,
) -> Result<AlgebraicEventNode> {
    let (parameters, position) = coordinates::coordinates(
        &a.net,
        &b.net,
        [a.domain.clone(), b.domain.clone()],
        f,
        t,
        k,
        interval,
    )?;
    Ok(AlgebraicEventNode {
        source_cells,
        pair,
        factor,
        root,
        stationary,
        exact: AlgebraicEventPosition {
            primitive: polynomial(f),
            isolating_interval: interval.clone().map(|q| ratio(&q)),
            parameters: parameters.map(|p| polynomial(&p)),
            position: position.map(|p| p.map(|q| polynomial(&q))),
        },
    })
}
fn assemble(
    a: &Curve,
    b: &Curve,
    c: &SpanIntersectionCertificate,
) -> Result<Vec<AlgebraicEventNode>> {
    let ((nodes,), _, _) = assemble_verified(a, b, c)?;
    Ok(nodes)
}
fn assemble_verified(
    a: &Curve,
    b: &Curve,
    c: &SpanIntersectionCertificate,
) -> Result<((Vec<AlgebraicEventNode>,), Vec<Cell>, Vec<Cell>)> {
    if ![1, 2, 3].contains(&c.version)
        || c.pairs.len() > 4096
        || c.source != source(a, b, c.self_pair, c.maximum_parameter_width)?
    {
        return Err(refused("Span intersection source expression changed"));
    }
    let x = cells(a)?;
    let y = cells(b)?;
    verify_cells(&x, &y, c)?;
    let nodes = assemble_cells(&x, &y, c)?;
    Ok(((nodes,), x, y))
}
fn require_finite_common_family(c: &SpanIntersectionCertificate) -> Result<()> {
    if c.pairs.iter().any(|p| {
        !matches!(
            p.kind.as_str(),
            "events" | "stationary" | "separate" | "range" | "overlap"
        )
    }) {
        return Err(refused(
            "Finite event-node owner requires an admitted finite/common relation family",
        ));
    }
    Ok(())
}
pub(super) fn assemble_cells(
    x: &[Cell],
    y: &[Cell],
    c: &SpanIntersectionCertificate,
) -> Result<Vec<AlgebraicEventNode>> {
    require_finite_common_family(c)?;
    let mut nodes = Vec::new();
    for (pair, p) in c.pairs.iter().enumerate() {
        let a = &x[p.first];
        let b = &y[p.second];
        if matches!(p.kind.as_str(), "events" | "stationary") {
            for (fi, f) in decode_factors(&p.factors)?.iter().enumerate() {
                let Some(t) = &f.map else {
                    continue;
                };
                for (ri, r) in f.roots.iter().enumerate() {
                    if !matches!(r.kind.as_str(), "event" | "diagonal") {
                        continue;
                    }
                    let stationary = r.kind == "diagonal";
                    let s = coordinates::first_parameter(t, p.shear);
                    // Same exact half-open original knot ownership as the
                    // complete source census. No float midpoint deduplication.
                    if !stationary
                        && ((p.first + 1 < x.len()
                            && coordinates::equals_one(&s, &f.polynomial, &r.interval)?)
                            || (p.second + 1 < y.len()
                                && coordinates::equals_one(t, &f.polynomial, &r.interval)?))
                    {
                        continue;
                    }
                    nodes.push(make(
                        a,
                        b,
                        &f.polynomial,
                        t,
                        p.shear,
                        &r.interval,
                        pair,
                        [p.first, p.second],
                        Some(fi),
                        ri,
                        stationary,
                    )?);
                }
            }
        } else if p.kind == "range" {
            for (ri, e) in range(a, b, p.axis as usize)?.2.into_iter().enumerate() {
                if (e.first[0] == Q::one() && p.first + 1 < x.len())
                    || (e.second[0] == Q::one() && p.second + 1 < y.len())
                {
                    continue;
                }
                // Endpoint events use alpha=0 with constant natural maps.
                // s=alpha-k*t cannot encode an independent constant s, so
                // restrict source cells exactly to that endpoint before make.
                let point_net = |n: &Vec<[Q; 3]>, v: &Q| {
                    let h = if v.is_zero() {
                        n[0].clone()
                    } else {
                        n.last().unwrap().clone()
                    };
                    vec![h.clone(), h]
                };
                let first = &a.domain[0] + (&a.domain[1] - &a.domain[0]) * &e.first[0];
                let second = &b.domain[0] + (&b.domain[1] - &b.domain[0]) * &e.second[0];
                let aa = Cell {
                    net: point_net(&a.net, &e.first[0]),
                    domain: [first.clone(), first],
                };
                let bb = Cell {
                    net: point_net(&b.net, &e.second[0]),
                    domain: [second.clone(), second],
                };
                nodes.push(make(
                    &aa,
                    &bb,
                    &vec![Q::zero(), Q::one()],
                    &vec![Q::zero()],
                    0,
                    &[Q::zero(), Q::zero()],
                    pair,
                    [p.first, p.second],
                    None,
                    ri,
                    false,
                )?);
            }
        }
        if nodes.len() > 4096 {
            return Err(refused("Original algebraic event node allowance exceeded"));
        }
    }
    Ok(nodes)
}
impl CurveEventNodes {
    pub fn sources(&self) -> &[Curve; 2] {
        &self.sources
    }
    pub fn census(&self) -> &SpanIntersectionCertificate {
        &self.census
    }
    pub fn nodes(&self) -> &[AlgebraicEventNode] {
        &self.nodes
    }
    /// Exact endpoints for every verified original common-image cell pair.
    /// Each tuple is (census pair index, [lower-image endpoint, upper-image endpoint]).
    /// These bound a component and are not isolated intersections. Native source
    /// parameters keep their own clocks, including distinct algebraic fields.
    pub fn overlap_endpoints(&self) -> Result<Vec<(usize, [AlgebraicEventNode; 2])>> {
        let ((fresh,), first, second) =
            assemble_verified(&self.sources[0], &self.sources[1], &self.census)?;
        if self.nodes != fresh {
            return Err(refused(
                "Original algebraic event nodes were omitted or rebound",
            ));
        }
        overlap_endpoints_cells(&first, &second, &self.census)
    }

    /// Complete original pair/cell proof replay, all event selections and exact
    /// symbolic natural parameters and shared original point definitions.
    pub fn reverify_source(&self) -> Result<()> {
        let fresh = assemble(&self.sources[0], &self.sources[1], &self.census)?;
        if self.nodes != fresh {
            return Err(refused(
                "Original algebraic event nodes were omitted or rebound",
            ));
        }
        Ok(())
    }
}
/// Own every isolated/stationary original event with exact algebraic point
/// coordinates. The full original census (including overlap receipts) is saved;
/// overlap branches are not falsely converted into isolated nodes. Nodes from
/// different pair owners still need independently proved identity/port merging
/// before a global arrangement or physical topology can be claimed.
pub fn own_curve_event_nodes(
    a: &Curve,
    b: &Curve,
    census: &SpanIntersectionCertificate,
) -> Result<CurveEventNodes> {
    let nodes = assemble(a, b, census)?;
    Ok(CurveEventNodes {
        sources: [a.clone(), b.clone()],
        census: census.clone(),
        nodes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn algebraic_node_replay_rejects_rebound_geometry_parameters_and_census() {
        let a = Curve::from_polyline(vec![vec![0., 0.], vec![1., 1.]]).unwrap();
        let b = Curve::from_polyline(vec![vec![1., 1.], vec![2., 0.]]).unwrap();
        let c = certify_span_intersections(&a, &b, false, 1e-12).unwrap();
        let owner = own_curve_event_nodes(&a, &b, &c).unwrap();
        owner.reverify_source().unwrap();
        let mut bad = owner.clone();
        bad.nodes.clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.nodes[0].exact.parameters[0] = vec![["0".into(), "1".into()]];
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.nodes[0].exact.position[0][1].clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.nodes[0].exact.isolating_interval[1] = ["1".into(), "1".into()];
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.nodes[0].exact.primitive.clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.nodes[0].source_cells = [1, 0];
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.nodes[0].stationary = true;
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.census.pairs.clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = owner;
        bad.sources[0].control_points[1][0] = 1_f64.next_up();
        assert!(bad.reverify_source().is_err());
    }
}

// Called only after complete source/cell replay by the respective owner.
pub(super) fn overlap_endpoints_cells(
    first: &[Cell],
    second: &[Cell],
    c: &SpanIntersectionCertificate,
) -> Result<Vec<(usize, [AlgebraicEventNode; 2])>> {
    require_finite_common_family(c)?;
    let mut out = Vec::new();
    for (pair, receipt) in c.pairs.iter().enumerate() {
        if receipt.kind != "overlap" {
            continue;
        }
        let a = &first[receipt.first];
        let b = &second[receipt.second];
        let width = width(a, b, c.maximum_parameter_width);
        // reverify_source above independently proves the common-image
        // relation on BOTH coordinates and its complete monotone coverage.
        let (lo, hi, _) = range(a, b, receipt.axis as usize)?;
        let mut ends = Vec::new();
        for value in [lo, hi] {
            let (shear, factors) =
                engine::coordinate_boundary(&a.net, &b.net, receipt.axis as usize, &value, &width)?;
            let mut selected = Vec::new();
            for (factor, f) in factors.iter().enumerate() {
                let Some(t) = &f.map else { continue };
                for (root, r) in f.roots.iter().enumerate() {
                    if r.kind == "event" {
                        // make independently checks BOTH actual spatial
                        // coordinate equalities in the selected extension.
                        selected.push(make(
                            a,
                            b,
                            &f.polynomial,
                            t,
                            shear,
                            &r.interval,
                            pair,
                            [receipt.first, receipt.second],
                            Some(factor),
                            root,
                            false,
                        )?);
                    }
                }
            }
            if selected.len() != 1 {
                return Err(refused(
                    "Original overlap endpoint parameter census is not unique",
                ));
            }
            ends.push(selected.pop().unwrap());
        }
        out.push((
            pair,
            ends.try_into()
                .map_err(|_| refused("Incomplete overlap endpoints"))?,
        ));
    }
    Ok(out)
}
