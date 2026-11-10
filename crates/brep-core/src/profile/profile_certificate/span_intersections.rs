//! Complete per-knot-span census of isolated and coincident curve components.
//! Algebraic source cells never pass through rounded temporary NURBS controls.
use super::*;
mod translated;
pub use translated::{OwnedTranslatedSpanIntersections, own_translated_span_intersections};
mod event_nodes;
use algebraic_intersections::{ExactRatio, decode_factors, encode_factors, outward, ratio};
use arithmetic::algebraic_events as engine;
pub use event_nodes::{
    AlgebraicEventNode, AlgebraicEventPosition, BranchNeighbourhoodLimits,
    BranchNeighbourhoodRecord, ContactAtlasLimits, ContactOccurrence, ContactPortOrderLimits,
    CurveEventNodes, EventParameterRef, LeadingBranchRay, OriginalAtlasPair,
    OriginalAtlasPairLimits, OriginalAtlasPairProof, OriginalAtomicCarrier,
    OriginalBoundaryAssemblyLimits, OriginalCarrierCut, OriginalCarrierIncidence,
    OriginalCarrierVertex, OriginalChartCoverageCell, OriginalChartCut, OriginalChartCutOrigin,
    OriginalChartInterval, OriginalChartRegion, OriginalChartShoreCell, OriginalChartShoreEvent,
    OriginalChartVertexLimits, OriginalCommonClip, OriginalCommonClipBoundary,
    OriginalCommonClipOrigin, OriginalCommonContributor, OriginalCommonCut,
    OriginalCommonCutOrigin, OriginalCommonEdge, OriginalCommonEdgeUse, OriginalCommonPartitionRef,
    OriginalCommonSegment, OriginalContactAtlas, OriginalContactPort, OriginalFilledRegion,
    OriginalInjectiveAtlasLimits, OriginalInjectiveChart, OriginalOverlapComponent,
    OriginalOverlapContact, OriginalOverlapContactRef, OriginalOverlapSplit, OriginalPairCut,
    OriginalPairCutOrigin, OriginalPeriodicCut, OriginalPeriodicCutOrigin, OriginalPeriodicEvent,
    OriginalPeriodicJoinKind, OriginalPeriodicPiece, OriginalPeriodicPieceCoverage,
    OriginalPeriodicPoint, OriginalPeriodicShoreCell, OriginalPeriodicSourceUse,
    OriginalPeriodicTransition, OriginalPeriodicWire, OriginalPeriodicWireJoin,
    OriginalPeriodicWireUse, OriginalPhysicalEdge, OriginalPhysicalEdgeUse, OriginalPhysicalFace,
    OriginalPhysicalFaceLimits, OriginalPhysicalFacePatch, OriginalPhysicalTrace,
    OriginalPhysicalVertex, OriginalPointPreimage, OriginalQuotientCoedge, OriginalQuotientEdge,
    OriginalQuotientVertexLink, OriginalSeamUse, OriginalShoreMembership, OriginalSourceCommonUse,
    OriginalSourceCut, OriginalSourceCutOrigin, OriginalSourceParameter,
    OriginalSurfaceRegularityRecord, OriginalUnifiedCarrier, OriginalUnifiedEdge,
    OriginalUnifiedEdgeCarrier, OriginalUnifiedIncidence, OriginalUnifiedPort,
    OriginalUnifiedVertex, OwnedBranchNeighbourhood, OwnedContactPortOrder, OwnedEventBranch,
    OwnedOriginalAtlasPairs, OwnedOriginalBoundaryWinding, OwnedOriginalCarrierCycles,
    OwnedOriginalCarrierGraph, OwnedOriginalCarrierSides, OwnedOriginalCarriers,
    OwnedOriginalChartEvents, OwnedOriginalChartIntervals, OwnedOriginalChartVertices,
    OwnedOriginalCommonClips, OwnedOriginalCommonDictionary, OwnedOriginalCommonPartition,
    OwnedOriginalFilledBoundaries, OwnedOriginalFilledRegions, OwnedOriginalGlobalChartEmbedding,
    OwnedOriginalInjectiveChartAtlas, OwnedOriginalOverlapComponents, OwnedOriginalOverlapContacts,
    OwnedOriginalOverlapPartition, OwnedOriginalPairCuts, OwnedOriginalPeriodicEmbedding,
    OwnedOriginalPeriodicPieces, OwnedOriginalPeriodicSurface, OwnedOriginalPeriodicTopology,
    OwnedOriginalPeriodicWires, OwnedOriginalPhysicalFaces, OwnedOriginalPointPullback,
    OwnedOriginalRegularTrimSurface, OwnedOriginalSeamDictionary, OwnedOriginalShoreCarrier,
    OwnedOriginalShoreMembership, OwnedOriginalShorePair, OwnedOriginalSourceCarriers,
    OwnedOriginalSurfaceSeams, OwnedOriginalTrimChartCoverage, OwnedOriginalUnifiedBoundaryWinding,
    OwnedOriginalUnifiedCycles, OwnedOriginalUnifiedFilled, OwnedOriginalUnifiedGraph,
    OwnedOriginalUnifiedPorts, OwnedOriginalUnifiedRegions, OwnedOriginalUnifiedRotation,
    OwnedOriginalUnifiedSides, ParameterOrderLimits, QualifiedParameterOrder,
    SourceContactParameter, assemble_original_contact_atlas, assemble_original_filled_boundaries,
    assemble_original_filled_regions, assemble_original_physical_faces,
    bind_original_periodic_pieces_to_surface, bind_original_seams_to_surface,
    classify_original_shore_carrier, compare_event_parameters, own_branch_neighbourhood,
    own_curve_event_nodes, own_event_branch, own_original_atlas_pairs,
    own_original_boundary_winding, own_original_carrier_cycles, own_original_carrier_graph,
    own_original_carrier_sides, own_original_carrier_sides_automatically, own_original_carriers,
    own_original_chart_events, own_original_chart_intervals, own_original_chart_vertices,
    own_original_common_clips, own_original_common_dictionary, own_original_common_partition,
    own_original_contact_port_order, own_original_filled_boundaries, own_original_filled_regions,
    own_original_global_chart_embedding, own_original_injective_chart_atlas,
    own_original_overlap_components, own_original_overlap_contacts, own_original_overlap_partition,
    own_original_pair_cuts, own_original_periodic_embedding, own_original_periodic_pieces,
    own_original_periodic_topology, own_original_periodic_wires, own_original_physical_faces,
    own_original_point_pullback, own_original_regular_trim_surface, own_original_seam_dictionary,
    own_original_shore_carrier, own_original_shore_pair, own_original_source_carriers,
    own_original_trim_chart_coverage, own_original_unified_boundary_winding,
    own_original_unified_cycles, own_original_unified_filled, own_original_unified_graph,
    own_original_unified_ports, own_original_unified_regions, own_original_unified_rotation,
    own_original_unified_sides, own_original_unified_sides_automatically, try_own_event_branch,
};
#[derive(Clone, Debug, PartialEq)]
pub struct SpanPairCertificate {
    pub first: usize,
    pub second: usize,
    pub kind: String,
    pub axis: u8,
    pub shear: u8,
    pub factors: Vec<AlgebraicFactorWitness>,
    pub relation: Vec<Vec<ExactRatio>>,
    pub branches: Option<ParameterBranchDecomposition>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct SpanIntersectionCertificate {
    pub version: u32,
    pub source: String,
    pub self_pair: bool,
    pub maximum_parameter_width: f64,
    pub pairs: Vec<SpanPairCertificate>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct VerifiedCurveOverlap {
    pub first_span: usize,
    pub second_span: usize,
    pub first_domain: [f64; 2],
    pub second_domain: [f64; 2],
    pub axis: u8,
    pub image_range: [ExactRatio; 2],
    pub relation: Vec<Vec<ExactRatio>>,
}
#[derive(Clone, Debug)]
pub struct VerifiedSpanIntersections {
    events: Vec<[[f64; 2]; 2]>,
    singular: Vec<[f64; 2]>,
    overlaps: Vec<VerifiedCurveOverlap>,
    algebraic_loci: Vec<VerifiedParameterLocus>,
    stationary_spans: Vec<[f64; 2]>,
    branch_decompositions: Vec<VerifiedBranchGraph>,
}
#[derive(Clone, Debug)]
pub struct VerifiedBranchGraph {
    pub first_span: usize,
    pub second_span: usize,
    pub decomposition: ParameterBranchDecomposition,
}
/// An exact real zero locus restricted to the two source span domains. This
/// includes vertical/horizontal fibers, retracing components and exceptional
/// real points. It is not a sampled list or an asserted smooth branch graph.
#[derive(Clone, Debug)]
pub struct VerifiedParameterLocus {
    pub first_span: usize,
    pub second_span: usize,
    pub first_domain: [f64; 2],
    pub second_domain: [f64; 2],
    pub whole_square: bool,
    pub diagonal_excluded: bool,
    pub relation: Vec<Vec<ExactRatio>>,
}
impl VerifiedSpanIntersections {
    pub fn events(&self) -> &[[[f64; 2]; 2]] {
        &self.events
    }
    pub fn singular_parameters(&self) -> &[[f64; 2]] {
        &self.singular
    }
    pub fn overlaps(&self) -> &[VerifiedCurveOverlap] {
        &self.overlaps
    }
    pub fn algebraic_loci(&self) -> &[VerifiedParameterLocus] {
        &self.algebraic_loci
    }
    pub fn stationary_spans(&self) -> &[[f64; 2]] {
        &self.stationary_spans
    }
    pub fn branch_decompositions(&self) -> &[VerifiedBranchGraph] {
        &self.branch_decompositions
    }
}
#[derive(Clone)]
struct Cell {
    net: Vec<[Q; 3]>,
    domain: [Q; 2],
}
fn local_diagonal(a: &Cell, b: &Cell) -> [Q; 2] {
    [
        (&a.domain[0] - &b.domain[0]) / (&b.domain[1] - &b.domain[0]),
        (&a.domain[1] - &a.domain[0]) / (&b.domain[1] - &b.domain[0]),
    ]
}
fn decode_locus(v: &[Vec<ExactRatio>]) -> Result<Vec<Vec<Q>>> {
    if v.is_empty()
        || v.len() > 18
        || v.iter().any(|r| r.len() > 257)
        || v.iter().map(Vec::len).sum::<usize>() > 4096
    {
        return Err(refused("Parameter locus size budget exceeded"));
    }
    v.iter()
        .map(|r| {
            r.iter()
                .map(algebraic_intersections::decode_ratio)
                .collect()
        })
        .collect()
}
fn cells(c: &Curve) -> Result<Vec<Cell>> {
    // Validate before indexing coordinates or converting stored floats to Q.
    source_bits(&[vec![c.clone()]], 1e-7)?;
    let nets = extraction::nets(&[vec![c.clone()]])?;
    let d = c.domain();
    let mut knots: Vec<_> = c
        .knots
        .iter()
        .copied()
        .filter(|k| *k >= d[0] && *k <= d[1])
        .collect();
    knots.dedup();
    if nets.len() != knots.len() - 1 || nets.len() > 64 {
        return Err(refused(
            "Algebraic multi-span cell census exceeds 64 per source",
        ));
    }
    Ok(nets
        .into_iter()
        .zip(knots.windows(2))
        .map(|((_, net), p)| Cell {
            net,
            domain: [Q::from_float(p[0]).unwrap(), Q::from_float(p[1]).unwrap()],
        })
        .collect())
}
fn source(a: &Curve, b: &Curve, self_pair: bool, width: f64) -> Result<String> {
    if !width.is_finite() || !(1e-18..=1e-3).contains(&width) {
        return Err(refused("Invalid multi-span algebraic parameter width"));
    }
    let sa = source_bits(&[vec![a.clone()]], 1e-7)?;
    let sb = source_bits(&[vec![b.clone()]], 1e-7)?;
    if self_pair && sa != sb {
        return Err(refused(
            "A self-intersection certificate requires identical source bits",
        ));
    }
    Ok(format!(
        "span-intersection-v1/{}/{:016x}/{sa}/{sb}",
        u8::from(self_pair),
        width.to_bits()
    ))
}
fn point(n: &[[Q; 3]], side: usize) -> [Q; 2] {
    let p = if side == 0 { &n[0] } else { n.last().unwrap() };
    [&p[0] / &p[2], &p[1] / &p[2]]
}
fn separated(a: &Cell, b: &Cell, axis: usize) -> bool {
    let x: Vec<_> = a.net.iter().map(|p| &p[axis] / &p[2]).collect();
    let y: Vec<_> = b.net.iter().map(|p| &p[axis] / &p[2]).collect();
    x.iter().max().unwrap() < y.iter().min().unwrap()
        || y.iter().max().unwrap() < x.iter().min().unwrap()
}
fn range(a: &Cell, b: &Cell, axis: usize) -> Result<(Q, Q, Vec<engine::Event>)> {
    if axis > 1 || !arithmetic::monotone(&a.net, axis)? || !arithmetic::monotone(&b.net, axis)? {
        return Err(refused("False common monotone projection claim"));
    }
    let x = [point(&a.net, 0), point(&a.net, 1)];
    let y = [point(&b.net, 0), point(&b.net, 1)];
    let lo = x[0][axis]
        .clone()
        .min(x[1][axis].clone())
        .max(y[0][axis].clone().min(y[1][axis].clone()));
    let hi = x[0][axis]
        .clone()
        .max(x[1][axis].clone())
        .min(y[0][axis].clone().max(y[1][axis].clone()));
    let mut events = Vec::new();
    if lo == hi {
        for i in 0..2 {
            for j in 0..2 {
                if x[i][axis] == lo && x[i] == y[j] {
                    let s = Q::from_integer(i.into());
                    let t = Q::from_integer(j.into());
                    events.push(engine::Event {
                        first: [s.clone(), s],
                        second: [t.clone(), t],
                        singular: false,
                    });
                }
            }
        }
    }
    Ok((lo, hi, events))
}
fn width(a: &Cell, b: &Cell, w: f64) -> Q {
    Q::from_float(w).unwrap()
        / (&a.domain[1] - &a.domain[0])
            .max(&b.domain[1] - &b.domain[0])
            .max(Q::one())
}


pub fn verify_span_intersections(
    a: &Curve,
    b: &Curve,
    c: &SpanIntersectionCertificate,
) -> Result<VerifiedSpanIntersections> {
    if ![1, 2, 3].contains(&c.version)
        || c.source != source(a, b, c.self_pair, c.maximum_parameter_width)?
        || c.pairs.len() > 4096
    {
        return Err(refused(
            "Multi-span intersection source or pair census mismatch",
        ));
    }
    let x = cells(a)?;
    let y = cells(b)?;
    verify_cells(&x, &y, c)
}

#[cfg(feature = "codec")]
mod serialization {
    use super::*;
    use value_codec::{Deserialize, Serialize, Value, error, json};
    impl Serialize for VerifiedBranchGraph {
        fn to_value(&self) -> Value {
            json!({"firstSpan":self.first_span,"secondSpan":self.second_span,"decomposition":self.decomposition})
        }
    }
    impl Serialize for VerifiedParameterLocus {
        fn to_value(&self) -> Value {
            json!({"firstSpan":self.first_span,"secondSpan":self.second_span,"firstDomain":self.first_domain,"secondDomain":self.second_domain,"wholeSquare":self.whole_square,"diagonalExcluded":self.diagonal_excluded,"relation":self.relation})
        }
    }
    impl Serialize for SpanPairCertificate {
        fn to_value(&self) -> Value {
            let mut v = json!({"first":self.first,"second":self.second,"kind":self.kind,"axis":self.axis,"shear":self.shear,"factors":self.factors,"relation":self.relation});
            if let Some(b) = &self.branches {
                v["branches"] = b.to_value();
            }
            v
        }
    }
    impl<'de> Deserialize<'de> for SpanPairCertificate {
        fn from_value(v: Value) -> value_codec::Result<Self> {
            let mut keys = vec![
                "first", "second", "kind", "axis", "shear", "factors", "relation",
            ];
            if v.get("branches").is_some() {
                keys.push("branches")
            }
            if v.as_object()
                .is_none_or(|o| o.len() != keys.len() || keys.iter().any(|k| !o.contains_key(*k)))
                || v["factors"].as_array().is_none_or(|a| a.len() > 256)
                || v["relation"].as_array().is_none_or(|a| a.len() > 18)
            {
                return Err(error("Span pair certificate schema mismatch"));
            }
            Ok(Self {
                first: Deserialize::from_value(v["first"].clone())?,
                second: Deserialize::from_value(v["second"].clone())?,
                kind: Deserialize::from_value(v["kind"].clone())?,
                axis: Deserialize::from_value(v["axis"].clone())?,
                shear: Deserialize::from_value(v["shear"].clone())?,
                factors: Deserialize::from_value(v["factors"].clone())?,
                relation: Deserialize::from_value(v["relation"].clone())?,
                branches: v
                    .get("branches")
                    .cloned()
                    .map(Deserialize::from_value)
                    .transpose()?,
            })
        }
    }
    impl Serialize for SpanIntersectionCertificate {
        fn to_value(&self) -> Value {
            json!({"version":self.version,"source":self.source,"selfPair":self.self_pair,"maximumParameterWidth":self.maximum_parameter_width,"pairs":self.pairs,"scope":"represented-multispan-curve-intersection-components"})
        }
    }
    impl<'de> Deserialize<'de> for SpanIntersectionCertificate {
        fn from_value(v: Value) -> value_codec::Result<Self> {
            let keys = [
                "version",
                "source",
                "selfPair",
                "maximumParameterWidth",
                "pairs",
                "scope",
            ];
            if v.as_object()
                .is_none_or(|o| o.len() != keys.len() || keys.iter().any(|k| !o.contains_key(*k)))
                || v["scope"].as_str()
                    != Some("represented-multispan-curve-intersection-components")
                || v["source"].as_str().is_none_or(|s| s.len() > 140000)
                || v["pairs"].as_array().is_none_or(|a| a.len() > 4096)
            {
                return Err(error("Multi-span component certificate schema mismatch"));
            }
            Ok(Self {
                version: Deserialize::from_value(v["version"].clone())?,
                source: Deserialize::from_value(v["source"].clone())?,
                self_pair: Deserialize::from_value(v["selfPair"].clone())?,
                maximum_parameter_width: Deserialize::from_value(
                    v["maximumParameterWidth"].clone(),
                )?,
                pairs: Deserialize::from_value(v["pairs"].clone())?,
            })
        }
    }
    impl Serialize for VerifiedCurveOverlap {
        fn to_value(&self) -> Value {
            json!({"firstSpan":self.first_span,"secondSpan":self.second_span,"firstDomain":self.first_domain,"secondDomain":self.second_domain,"axis":self.axis,"imageRange":self.image_range,"relation":self.relation})
        }
    }
}
#[cfg(test)]
#[path = "span_intersections/tests/mod.rs"]
mod tests;

#[cfg(test)]
mod locus_tests;
pub use translated::{
    OriginalPieceEvent, OriginalPieceEventLocation, OwnedPeriodicPairEvents,
    own_periodic_pair_events,
};

pub use translated::{
    OriginalPeriodicCommonClip, OwnedPeriodicCommonClips, own_periodic_common_clips,
};

pub use translated::{
    OriginalPeriodicArrangementCut, OriginalPeriodicArrangementInterval,
    OriginalPeriodicArrangementOrigin, OriginalPeriodicArrangementUse,
    OwnedPeriodicArrangementCuts, own_periodic_arrangement_cuts,
};

pub use translated::{
    OriginalPeriodicAlignedCut, OriginalPeriodicAlignedOrigin, OriginalPeriodicIntervalAlias,
    OriginalPeriodicIntervalBinding, OwnedPeriodicAlignedIntervals, own_periodic_aligned_intervals,
};

pub use translated::{
    OriginalPeriodicChartCarrier, OriginalPeriodicChartCutRef, OriginalPeriodicChartEdge,
    OriginalPeriodicChartIncidence, OriginalPeriodicChartRelation,
    OriginalPeriodicChartRelationOrigin, OriginalPeriodicChartVertex, OwnedPeriodicChartGraph,
    own_periodic_chart_graph,
};

pub use translated::own_periodic_pair_events_translated;

pub use translated::{
    OriginalPeriodicSeamNode, OriginalPeriodicSeamPair, OwnedPeriodicSeamContacts,
    own_periodic_seam_contacts,
};

pub use translated::own_periodic_quotient_arrangement_cuts;

pub use translated::{OwnedPeriodicQuotientGraph, own_periodic_quotient_graph};

pub use translated::{
    OriginalPeriodicVertexChart, OriginalPeriodicVertexChartUse, OwnedPeriodicVertexCharts,
    own_periodic_vertex_charts,
};

pub use translated::{OriginalPeriodicPort, OwnedPeriodicPorts, own_periodic_ports};

pub use translated::{
    OriginalPeriodicTangentWitness, OwnedPeriodicRotation, own_periodic_rotation,
};

pub use translated::{OwnedPeriodicCycles, own_periodic_cycles};

pub use translated::{
    OriginalPeriodicBoundaryPart, OriginalPeriodicBoundaryWalk, OwnedPeriodicBoundaryLifts,
    own_periodic_boundary_lifts,
};

pub use translated::{
    OriginalPeriodicBalanceTerm, OriginalPeriodicEdgeJump, OriginalPeriodicVertexBalance,
    OriginalPeriodicWireContribution, OriginalPeriodicWireFlux, OriginalPeriodicWireJump,
    OwnedPeriodicWireJumps, own_periodic_wire_jumps,
};

pub use translated::{
    OriginalPeriodicPathCrossing, OwnedPeriodicRelativeWinding, own_periodic_relative_winding,
};

pub use translated::{
    OriginalPeriodicBooleanOp, OriginalPeriodicFillRule, OriginalPeriodicOperand,
    OriginalPeriodicOperandPeriods, OwnedPeriodicMaterialRule, OwnedPeriodicMaterialSample,
    own_periodic_material_rule, own_periodic_material_sample,
};

pub use translated::{
    OwnedPeriodicChartSideProbes, OwnedPeriodicMaterialSides, own_periodic_chart_side_probes,
    own_periodic_material_sides,
};

pub use translated::{
    OriginalPeriodicSelectedEdge, OwnedPeriodicBoundarySelection, own_periodic_boundary_selection,
};

pub use translated::{
    OriginalPeriodicSideView, OwnedPeriodicSideProbes, own_periodic_physical_side_probes,
};

pub use translated::{
    OriginalPeriodicMaterialSector, OwnedPeriodicMaterialCycles, own_periodic_material_cycles,
};

pub use translated::{OwnedPeriodicMaterialLifts, own_periodic_material_lifts};

pub use translated::{
    OriginalNaturalBoundaryCurve, OriginalPeriodicNaturalBoundary, OriginalPeriodicSupportWireRole,
    OwnedPeriodicSupportBoundaries, own_periodic_support_boundaries,
};

pub use translated::{
    OriginalPeriodicNaturalCarrier, OwnedPeriodicSupportMaterialRule,
    OwnedPeriodicSupportSelection, OwnedPeriodicSupportSideProbes, OwnedPeriodicSupportSides,
    own_periodic_support_material_rule, own_periodic_support_selection,
    own_periodic_support_side_probes, own_periodic_support_sides,
};

pub use translated::{
    OriginalPeriodicSupportSector, OriginalPeriodicSupportVertex, OwnedPeriodicSupportCycles,
    OwnedPeriodicSupportLifts, own_periodic_support_cycles, own_periodic_support_lifts,
};

pub use translated::{OwnedPeriodicSupportPath, own_periodic_support_path};

pub use translated::{
    OriginalPeriodicCochainTerm, OriginalPeriodicMaterialComponent, OriginalPeriodicRegionProbe,
    OriginalPeriodicSeparatingCochain, OwnedPeriodicSupportRegions, OwnedPeriodicSupportSignatures,
    own_periodic_support_regions, own_periodic_support_signatures,
};

pub use translated::{
    OriginalPeriodicSurfaceTrace, OriginalPeriodicSurfaceTraceEndpoint,
    OriginalPeriodicSurfaceTracePoint, OwnedPeriodicSupportTraces, own_periodic_support_traces,
};
pub use translated::{
    PeriodicRegionSearchLimits, PeriodicRegionSearchReport, PeriodicRegionSearchStop,
    construct_periodic_support_regions,
};

pub use translated::{
    OriginalPeriodicSurfaceBoundaryFan, OriginalPeriodicSurfaceEndpointUse,
    OriginalPeriodicSurfaceIncidenceEdge, OriginalPeriodicSurfaceIncidenceLoop,
    OriginalPeriodicSurfaceIncidenceVertex, OwnedPeriodicSupportIncidence,
    own_periodic_support_incidence,
};

pub use translated::{
    OriginalPeriodicSourceChartCopy, OriginalPeriodicSourceChartWorldPoint,
    OwnedPeriodicSourceChartCopies, PeriodicSourceChartCopyLimits,
    own_periodic_source_chart_copies,
};

pub use translated::{
    OriginalPeriodicNativeChartCorner, OriginalPeriodicNativeCornerVertex,
    OwnedPeriodicNativeChartCorners, own_periodic_native_chart_corners,
};

pub use translated::{
    OriginalPeriodicNativeShore, OriginalPeriodicNativeShorePoint, OriginalPeriodicNativeShoreRole,
    OwnedPeriodicNativeChartShores, PeriodicNativeShoreLimits, own_periodic_native_chart_shores,
};

pub use translated::{
    OriginalPeriodicShoreMaterialSample, OriginalPeriodicShoreSampleOrigin,
    OwnedPeriodicShoreMaterialSamples, PeriodicShoreMaterialSampleLimits,
    own_periodic_shore_material_samples,
};

pub use translated::{
    OriginalPeriodicShoreContact, OriginalPeriodicShoreContactKind,
    OwnedPeriodicShoreMaterialIntervals, PeriodicShoreMaterialIntervalLimits,
    own_periodic_shore_material_intervals,
};

pub use translated::{
    OriginalPeriodicChartCycleEdge, OriginalPeriodicChartCycleEdgeKind,
    OriginalPeriodicChartCycleVertex, OriginalPeriodicChartVertexKey, OwnedPeriodicChartCycles,
    PeriodicChartCycleLimits, own_periodic_chart_cycles,
};

pub use translated::{
    OwnedPeriodicChartWinding, PeriodicChartWindingLimits, construct_periodic_chart_winding,
    own_periodic_chart_winding,
};

pub use translated::{
    OriginalPeriodicChartCycleSides, OwnedPeriodicChartCycleSides, PeriodicChartCycleOrientation,
    PeriodicChartCycleSideLimits, own_periodic_chart_cycle_sides,
};

pub use translated::{
    OriginalPeriodicChartStrictPatch, OwnedPeriodicChartStrictPatches,
    PeriodicChartStrictPatchLimits, own_periodic_chart_strict_patches,
};

pub use translated::{
    OriginalPeriodicChartBoundaryLoop, OriginalPeriodicChartLoopContact,
    OwnedPeriodicChartBoundaryLoops, PeriodicChartBoundaryLoopLimits,
    own_periodic_chart_boundary_loops,
};

pub use translated::{OwnedPeriodicChartLoopWinding, own_periodic_chart_loop_winding};

pub use translated::{
    OwnedPeriodicChartFanPatches, PeriodicChartFanPatchLimits, own_periodic_chart_fan_patches,
};

pub use translated::{
    OriginalPeriodicSewnCoedge, OriginalPeriodicSewnEdge, OriginalPeriodicSewnFace,
    OriginalPeriodicSewnVertex, OwnedPeriodicSewnFaces, PeriodicSewnFaceLimits,
    own_periodic_sewn_faces,
};

#[path="span_intersections/algorithm_witness.rs"]
mod algorithm_witness;
use algorithm_witness::witness;
#[path="span_intersections/algorithm_certify_span_intersections.rs"]
mod algorithm_certify_span_intersections;
pub use algorithm_certify_span_intersections::certify_span_intersections;
#[path="span_intersections/algorithm_verify_cells.rs"]
mod algorithm_verify_cells;
use algorithm_verify_cells::verify_cells;
