mod periodic_chart_winding;
pub(in crate::profile_certificate::span_intersections) mod periodic_replay;
pub use periodic_chart_cycles::construct_periodic_chart_winding;
pub use periodic_chart_winding::{
    OwnedPeriodicChartWinding, PeriodicChartWindingLimits, own_periodic_chart_winding,
};
mod periodic_chart_cycles;
pub use periodic_chart_cycles::{
    OriginalPeriodicChartCycleEdge, OriginalPeriodicChartCycleEdgeKind,
    OriginalPeriodicChartCycleVertex, OriginalPeriodicChartVertexKey, OwnedPeriodicChartCycles,
    PeriodicChartCycleLimits, own_periodic_chart_cycles,
};
mod periodic_shore_material_intervals;
pub use periodic_shore_material_intervals::{
    OriginalPeriodicShoreContact, OriginalPeriodicShoreContactKind,
    OwnedPeriodicShoreMaterialIntervals, PeriodicShoreMaterialIntervalLimits,
    own_periodic_shore_material_intervals,
};
mod periodic_shore_material_samples;
pub use periodic_shore_material_samples::{
    OriginalPeriodicShoreMaterialSample, OriginalPeriodicShoreSampleOrigin,
    OwnedPeriodicShoreMaterialSamples, PeriodicShoreMaterialSampleLimits,
    own_periodic_shore_material_samples,
};
mod periodic_chart_shores;
pub use periodic_chart_shores::{
    OriginalPeriodicNativeShore, OriginalPeriodicNativeShorePoint, OriginalPeriodicNativeShoreRole,
    OwnedPeriodicNativeChartShores, PeriodicNativeShoreLimits, own_periodic_native_chart_shores,
};
mod periodic_chart_corners;
pub use periodic_chart_corners::{
    OriginalPeriodicNativeChartCorner, OriginalPeriodicNativeCornerVertex,
    OwnedPeriodicNativeChartCorners, own_periodic_native_chart_corners,
};
mod periodic_chart_copies;
pub use periodic_chart_copies::{
    OriginalPeriodicSourceChartCopy, OriginalPeriodicSourceChartWorldPoint,
    OwnedPeriodicSourceChartCopies, PeriodicSourceChartCopyLimits,
    own_periodic_source_chart_copies,
};
mod periodic_support_incidence;
pub use periodic_support_incidence::{
    OriginalPeriodicSurfaceBoundaryFan, OriginalPeriodicSurfaceEndpointUse,
    OriginalPeriodicSurfaceIncidenceEdge, OriginalPeriodicSurfaceIncidenceLoop,
    OriginalPeriodicSurfaceIncidenceVertex, OwnedPeriodicSupportIncidence,
    own_periodic_support_incidence,
};
mod periodic_support_traces;
pub use periodic_support_traces::{
    OriginalPeriodicSurfaceTrace, OriginalPeriodicSurfaceTraceEndpoint,
    OriginalPeriodicSurfaceTracePoint, OwnedPeriodicSupportTraces, own_periodic_support_traces,
};
mod periodic_region_search;
pub use periodic_region_search::{
    PeriodicRegionSearchLimits, PeriodicRegionSearchReport, PeriodicRegionSearchStop,
    construct_periodic_support_regions,
};
mod periodic_material_clearance;
mod periodic_support_regions;
pub use periodic_support_regions::{
    OriginalPeriodicCochainTerm, OriginalPeriodicMaterialComponent, OriginalPeriodicRegionProbe,
    OriginalPeriodicSeparatingCochain, OwnedPeriodicSupportRegions, OwnedPeriodicSupportSignatures,
    own_periodic_support_regions, own_periodic_support_signatures,
};
mod periodic_support_path;
pub use periodic_support_path::{OwnedPeriodicSupportPath, own_periodic_support_path};
mod periodic_support_cycles;
pub use periodic_support_cycles::{
    OriginalPeriodicSupportSector, OriginalPeriodicSupportVertex, OwnedPeriodicSupportCycles,
    OwnedPeriodicSupportLifts, own_periodic_support_cycles, own_periodic_support_lifts,
};
mod periodic_support_material;
pub use periodic_support_material::{
    OriginalPeriodicNaturalCarrier, OwnedPeriodicSupportMaterialRule,
    OwnedPeriodicSupportSelection, OwnedPeriodicSupportSideProbes, OwnedPeriodicSupportSides,
    own_periodic_support_material_rule, own_periodic_support_selection,
    own_periodic_support_side_probes, own_periodic_support_sides,
};
mod periodic_support_boundaries;
pub use periodic_support_boundaries::{
    OriginalNaturalBoundaryCurve, OriginalPeriodicNaturalBoundary, OriginalPeriodicSupportWireRole,
    OwnedPeriodicSupportBoundaries, own_periodic_support_boundaries,
};
mod periodic_material_cycles;
pub use periodic_material_cycles::{
    OriginalPeriodicMaterialSector, OwnedPeriodicMaterialCycles, own_periodic_material_cycles,
};
mod periodic_material_sides;
pub use periodic_material_sides::{
    OwnedPeriodicChartSideProbes, OwnedPeriodicMaterialSides, own_periodic_chart_side_probes,
    own_periodic_material_sides,
};
mod periodic_material;
pub use periodic_material::{
    OriginalPeriodicBooleanOp, OriginalPeriodicFillRule, OriginalPeriodicOperand,
    OriginalPeriodicOperandPeriods, OwnedPeriodicMaterialRule, OwnedPeriodicMaterialSample,
    own_periodic_material_rule, own_periodic_material_sample,
};
mod periodic_relative_winding;
pub use periodic_relative_winding::{
    OriginalPeriodicPathCrossing, OwnedPeriodicRelativeWinding, own_periodic_relative_winding,
};
mod periodic_wire_jumps;
pub use periodic_wire_jumps::{
    OriginalPeriodicBalanceTerm, OriginalPeriodicEdgeJump, OriginalPeriodicVertexBalance,
    OriginalPeriodicWireContribution, OriginalPeriodicWireFlux, OriginalPeriodicWireJump,
    OwnedPeriodicWireJumps, own_periodic_wire_jumps,
};
mod periodic_boundary_lifts;
pub use periodic_boundary_lifts::{
    OriginalPeriodicBoundaryPart, OriginalPeriodicBoundaryWalk, OwnedPeriodicBoundaryLifts,
    own_periodic_boundary_lifts,
};
mod periodic_cycles;
pub use periodic_cycles::{OwnedPeriodicCycles, own_periodic_cycles};
mod periodic_rotation;
pub use periodic_rotation::{
    OriginalPeriodicTangentWitness, OwnedPeriodicRotation, own_periodic_rotation,
};
mod periodic_ports;
pub use periodic_ports::{OriginalPeriodicPort, OwnedPeriodicPorts, own_periodic_ports};
mod periodic_vertex_charts;
pub use periodic_vertex_charts::{
    OriginalPeriodicVertexChart, OriginalPeriodicVertexChartUse, OwnedPeriodicVertexCharts,
    own_periodic_vertex_charts,
};
mod periodic_seam_contacts;
pub use periodic_seam_contacts::{
    OriginalPeriodicSeamNode, OriginalPeriodicSeamPair, OwnedPeriodicSeamContacts,
    own_periodic_seam_contacts,
};
mod periodic_graph;
pub use periodic_graph::{
    OriginalPeriodicChartCarrier, OriginalPeriodicChartCutRef, OriginalPeriodicChartEdge,
    OriginalPeriodicChartIncidence, OriginalPeriodicChartRelation,
    OriginalPeriodicChartRelationOrigin, OriginalPeriodicChartVertex, OwnedPeriodicChartGraph,
    own_periodic_chart_graph,
};
mod periodic_alignment;
pub use periodic_alignment::{
    OriginalPeriodicAlignedCut, OriginalPeriodicAlignedOrigin, OriginalPeriodicIntervalAlias,
    OriginalPeriodicIntervalBinding, OwnedPeriodicAlignedIntervals, own_periodic_aligned_intervals,
};
mod periodic_arrangement;
pub use periodic_arrangement::{
    OriginalPeriodicArrangementCut, OriginalPeriodicArrangementInterval,
    OriginalPeriodicArrangementOrigin, OriginalPeriodicArrangementUse,
    OwnedPeriodicArrangementCuts, own_periodic_arrangement_cuts,
};
mod periodic_events;
pub use periodic_events::{
    OriginalPieceEvent, OriginalPieceEventLocation, OwnedPeriodicPairEvents,
    own_periodic_pair_events,
};
// Original multi-span source geometry with exact rational translations.
use super::*;
#[derive(Clone, Debug)]
pub struct OwnedTranslatedSpanIntersections {
    sources: [Curve; 2],
    offsets: [[ExactRatio; 2]; 2],
    certificate: SpanIntersectionCertificate,
}
fn context(
    a: &Curve,
    b: &Curve,
    offsets: &[[ExactRatio; 2]; 2],
    self_pair: bool,
    w: f64,
) -> Result<(String, Vec<Cell>, Vec<Cell>)> {
    let original = source(a, b, self_pair, w)?;
    let decode = |s: &[ExactRatio; 2]| -> Result<[Q; 2]> {
        Ok([
            algebraic_intersections::decode_ratio(&s[0])?,
            algebraic_intersections::decode_ratio(&s[1])?,
        ])
    };
    let shifts = [decode(&offsets[0])?, decode(&offsets[1])?];
    if self_pair && shifts[0] != shifts[1] {
        return Err(refused(
            "Translated self-pair requires identical source translations",
        ));
    }
    let translate = |curve: &Curve, shift: &[Q; 2]| -> Result<Vec<Cell>> {
        cells(curve)?
            .into_iter()
            .map(|c| {
                Ok(Cell {
                    net: arithmetic::shore_events::translated_net(&c.net, shift)?,
                    domain: c.domain,
                })
            })
            .collect()
    };
    let key = format!(
        "translated-span-v1/{original}/{}/{}/{}/{}",
        shifts[0][0], shifts[0][1], shifts[1][0], shifts[1][1]
    );
    Ok((key, translate(a, &shifts[0])?, translate(b, &shifts[1])?))
}
impl OwnedTranslatedSpanIntersections {
    pub fn sources(&self) -> &[Curve; 2] {
        &self.sources
    }
    pub fn offsets(&self) -> &[[ExactRatio; 2]; 2] {
        &self.offsets
    }
    pub fn certificate(&self) -> &SpanIntersectionCertificate {
        &self.certificate
    }
    /// Exact original natural parameters and translated positions, not the
    /// outward floating-point parameter boxes returned by the census summary.
    pub fn exact_nodes(&self) -> Result<Vec<AlgebraicEventNode>> {
        let (_, x, y) = self.verified_cells()?;
        event_nodes::assemble_cells(&x, &y, &self.certificate)
    }
    /// Complete lower/upper-image endpoints of every admitted common component.
    pub fn exact_overlap_endpoints(&self) -> Result<Vec<(usize, [AlgebraicEventNode; 2])>> {
        let (_, x, y) = self.verified_cells()?;
        event_nodes::overlap_endpoints_cells(&x, &y, &self.certificate)
    }
    pub(super) fn exact_nodes_and_overlaps(
        &self,
    ) -> Result<(
        String,
        Vec<AlgebraicEventNode>,
        Vec<(usize, [AlgebraicEventNode; 2])>,
    )> {
        let (key, x, y) = self.verified_cells()?;
        let nodes = event_nodes::assemble_cells(&x, &y, &self.certificate)?;
        let overlaps = event_nodes::overlap_endpoints_cells(&x, &y, &self.certificate)?;
        Ok((key, nodes, overlaps))
    }
    fn verified_cells(&self) -> Result<(String, Vec<Cell>, Vec<Cell>)> {
        let c = &self.certificate;
        let (key, x, y) = context(
            &self.sources[0],
            &self.sources[1],
            &self.offsets,
            c.self_pair,
            c.maximum_parameter_width,
        )?;
        if c.version != 1 || c.source != key {
            return Err(refused("Translated span source expression changed"));
        }
        verify_cells(&x, &y, c)?;
        Ok((key, x, y))
    }
    pub fn reverify_source(&self) -> Result<VerifiedSpanIntersections> {
        let c = &self.certificate;
        let (key, x, y) = context(
            &self.sources[0],
            &self.sources[1],
            &self.offsets,
            c.self_pair,
            c.maximum_parameter_width,
        )?;
        if c.version != 1 || c.source != key {
            return Err(refused("Translated span source expression changed"));
        }
        verify_cells(&x, &y, c)
    }
}
/// Complete census across all ORIGINAL knot cells, using unchanged source clocks.
/// Positive common intervals retain their exact image range and nonlinear
/// correspondence. Algebraic piece-bound clipping and face assembly are separate.
pub fn own_translated_span_intersections(
    a: &Curve,
    b: &Curve,
    offsets: &[[ExactRatio; 2]; 2],
    self_pair: bool,
    w: f64,
) -> Result<OwnedTranslatedSpanIntersections> {
    let (key, x, y) = context(a, b, offsets, self_pair, w)?;
    let mut pairs = Vec::new();
    for i in 0..x.len() {
        for j in 0..y.len() {
            if self_pair && i > j {
                continue;
            }
            pairs.push(witness(&x[i], &y[j], i, j, self_pair && i == j, w)?);
        }
    }
    let owner = OwnedTranslatedSpanIntersections {
        sources: [a.clone(), b.clone()],
        offsets: offsets.clone(),
        certificate: SpanIntersectionCertificate {
            version: 1,
            source: key,
            self_pair,
            maximum_parameter_width: w,
            pairs,
        },
    };
    verify_cells(&x, &y, &owner.certificate)?;
    Ok(owner)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn shifts() -> [[ExactRatio; 2]; 2] {
        let z = ratio(&Q::zero());
        [
            [z.clone(), z.clone()],
            [ratio(&Q::new(1.into(), 3.into())), z],
        ]
    }
    #[test]
    fn exact_translated_common_boundary_retains_irrational_original_clock() {
        let a = Curve {
            degree: 2,
            knots: vec![2., 2., 2., 6., 6., 6.],
            control_points: vec![vec![0., 0.], vec![0., 0.], vec![1., 0.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let b = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
        let o = own_translated_span_intersections(&a, &b, &shifts(), false, 1e-12).unwrap();
        let ends = o.exact_overlap_endpoints().unwrap();
        assert_eq!(ends.len(), 1);
        let low = &ends[0].1[0].exact;
        assert_ne!(low.isolating_interval[0], low.isolating_interval[1]);
        let sub = |a: &[Q], b: &[Q]| {
            let mut out = vec![Q::zero(); a.len().max(b.len())];
            for (i, q) in a.iter().enumerate() {
                out[i] += q;
            }
            for (i, q) in b.iter().enumerate() {
                out[i] -= q;
            }
            while out.last().is_some_and(|q| q.is_zero()) {
                out.pop();
            }
            out
        };
        let product = |a: &[Q], b: &[Q]| {
            let mut out = vec![Q::zero(); a.len() + b.len() - 1];
            for (i, x) in a.iter().enumerate() {
                for (j, y) in b.iter().enumerate() {
                    out[i + j] += x * y;
                }
            }
            out
        };
        let decode = algebraic_intersections::decode_polynomial;
        let f = decode(&low.primitive).unwrap();
        let t = decode(&low.parameters[0]).unwrap();
        let root = low
            .isolating_interval
            .clone()
            .map(|r| algebraic_intersections::decode_ratio(&r).unwrap());
        // 3(t-2)^2 -16 == 0 in the selected original-clock field.
        let delta = sub(&t, &[Q::from_integer(2.into())]);
        let square = product(&delta, &delta)
            .iter()
            .map(|q| q * Q::from_integer(3.into()))
            .collect::<Vec<_>>();
        assert!(
            arithmetic::event_coordinates::zero_at(
                &sub(&square, &[Q::from_integer(16.into())]),
                &f,
                &root
            )
            .unwrap()
        );
        let x = low.position[0].clone().map(|p| decode(&p).unwrap());
        let triple = x[0]
            .iter()
            .map(|q| q * Q::from_integer(3.into()))
            .collect::<Vec<_>>();
        assert!(arithmetic::event_coordinates::zero_at(&sub(&triple, &x[1]), &f, &root).unwrap());
        assert!(o.exact_nodes().unwrap().is_empty());
    }
    #[test]
    fn common_translated_cells_keep_original_domains_and_exact_image() {
        let a = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.], vec![2., 0.]]).unwrap();
        let b = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
        let owner = own_translated_span_intersections(&a, &b, &shifts(), false, 1e-10).unwrap();
        let verified = owner.reverify_source().unwrap();
        assert_eq!(verified.overlaps().len(), 2);
        assert_eq!(verified.overlaps()[0].image_range[0], shifts()[1][0]);
        assert_eq!(verified.overlaps()[0].first_domain, [0., 1.]);
        assert_eq!(verified.overlaps()[1].first_domain, [1., 2.]);
        assert_eq!(
            verified.overlaps()[1].image_range[1],
            ratio(&Q::new(4.into(), 3.into()))
        );
        let mut damaged = owner.clone();
        damaged.certificate.pairs.pop();
        assert!(damaged.reverify_source().is_err());
        let mut damaged = owner.clone();
        damaged.offsets[1][0] = ratio(&Q::one());
        assert!(damaged.reverify_source().is_err());
        let mut damaged = owner.clone();
        damaged.certificate.pairs[0].relation.clear();
        assert!(damaged.reverify_source().is_err());
        assert!(own_translated_span_intersections(&a, &a, &shifts(), true, 1e-10).is_err());
    }
    #[test]
    fn translated_finite_events_use_original_natural_parameters() {
        let a = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.], vec![2., 0.]]).unwrap();
        let b = Curve::from_polyline(vec![vec![0., -1.], vec![0., 1.]]).unwrap();
        let owner = own_translated_span_intersections(&a, &b, &shifts(), false, 1e-10).unwrap();
        let events = owner.reverify_source().unwrap();
        assert_eq!(events.events().len(), 1);
        let p = events.events()[0];
        assert!(p[0][0] <= 1. / 3. && p[0][1] >= 1. / 3.);
        assert!(p[1][0] <= 0.5 && p[1][1] >= 0.5);
    }
}

pub use periodic_events::{
    OriginalPeriodicCommonClip, OwnedPeriodicCommonClips, own_periodic_common_clips,
};

pub use periodic_events::own_periodic_pair_events_translated;

pub use periodic_arrangement::own_periodic_quotient_arrangement_cuts;

pub use periodic_graph::{OwnedPeriodicQuotientGraph, own_periodic_quotient_graph};

pub use periodic_material_sides::{
    OriginalPeriodicSelectedEdge, OwnedPeriodicBoundarySelection, own_periodic_boundary_selection,
};

pub use periodic_material_sides::{
    OriginalPeriodicSideView, OwnedPeriodicSideProbes, own_periodic_physical_side_probes,
};

pub use periodic_material_cycles::{OwnedPeriodicMaterialLifts, own_periodic_material_lifts};

mod periodic_chart_cycle_sides;
pub use periodic_chart_cycle_sides::{
    OriginalPeriodicChartCycleSides, OwnedPeriodicChartCycleSides, PeriodicChartCycleOrientation,
    PeriodicChartCycleSideLimits, own_periodic_chart_cycle_sides,
};

mod periodic_chart_strict_patches;
pub use periodic_chart_strict_patches::{
    OriginalPeriodicChartStrictPatch, OwnedPeriodicChartStrictPatches,
    PeriodicChartStrictPatchLimits, own_periodic_chart_strict_patches,
};

#[cfg(test)]
mod current_kernel_scope_tests {
    use super::*;
    #[test]
    fn ordinary_complete_loci_do_not_become_finite_translated_event_authority() {
        let curve = |p: Vec<Vec<f64>>| {
            let degree = p.len() - 1;
            Curve {
                degree,
                weights: vec![1.; p.len()],
                control_points: p,
                knots: [vec![0.; degree + 1], vec![1.; degree + 1]].concat(),
                periodic: false,
            }
        };
        let a = curve(vec![vec![0.5, 0.], vec![0.5, 0.]]);
        let b = curve(vec![vec![0., 0.], vec![0., 0.], vec![1., 0.]]);
        let proof = certify_span_intersections(&a, &b, false, 1e-10).unwrap();
        assert!(
            !verify_span_intersections(&a, &b, &proof)
                .unwrap()
                .algebraic_loci()
                .is_empty()
        );
        assert!(own_curve_event_nodes(&a, &b, &proof).is_err());
        let z = ratio(&Q::zero());
        let offsets = [[z.clone(), z.clone()], [z.clone(), z]];
        assert!(own_translated_span_intersections(&a, &b, &offsets, false, 1e-10).is_err());
        let retrace = curve(vec![vec![1., 0.], vec![-1., 0.], vec![1., 0.]]);
        let proof = certify_span_intersections(&retrace, &retrace, true, 1e-10).unwrap();
        assert!(
            !verify_span_intersections(&retrace, &retrace, &proof)
                .unwrap()
                .algebraic_loci()
                .is_empty()
        );
        assert!(own_curve_event_nodes(&retrace, &retrace, &proof).is_err());
        assert!(
            own_translated_span_intersections(&retrace, &retrace, &offsets, true, 1e-10).is_err()
        );
    }
}

mod periodic_chart_boundary_loops;
pub use periodic_chart_boundary_loops::{
    OriginalPeriodicChartBoundaryLoop, OriginalPeriodicChartLoopContact,
    OwnedPeriodicChartBoundaryLoops, PeriodicChartBoundaryLoopLimits,
    own_periodic_chart_boundary_loops,
};

mod periodic_chart_loop_winding;
pub use periodic_chart_loop_winding::{
    OwnedPeriodicChartLoopWinding, own_periodic_chart_loop_winding,
};

mod periodic_chart_fan_patches;
pub use periodic_chart_fan_patches::{
    OwnedPeriodicChartFanPatches, PeriodicChartFanPatchLimits, own_periodic_chart_fan_patches,
};

mod periodic_sewn_faces;
pub use periodic_sewn_faces::{
    OriginalPeriodicSewnCoedge, OriginalPeriodicSewnEdge, OriginalPeriodicSewnFace,
    OriginalPeriodicSewnVertex, OwnedPeriodicSewnFaces, PeriodicSewnFaceLimits,
    own_periodic_sewn_faces,
};
