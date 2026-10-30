//! Checkable certificates of the *represented* positive-weight planar profile.
//!
//! The verifier does not trust the profile validator, intersection search,
//! floating-point area, a hash, or a caller-supplied "certified" flag. It
//! reconstructs exact rational homogeneous subdivision from every source bit
//! and checks every span and every unordered pair. Hull witnesses prove a
//! simultaneous homotopy to a simple polygon, preserving orientation/nesting.
//! This certifies neither construction history nor a three-dimensional solid.
use crate::{Curve, Error, Result};
use num_rational::BigRational as Q;
use num_traits::{One, Zero};

mod agreement;
pub use agreement::{
    ProjectiveCurveAgreementCertificate, certify_projective_curve_agreement,
    verify_projective_curve_agreement,
};
mod algebraic_intersections;
mod arithmetic;
mod intersections;
pub use algebraic_intersections::{
    AlgebraicFactorWitness, AlgebraicIntersectionCertificate, AlgebraicRootWitness,
    ExactPolynomial, ExactRatio, VerifiedAlgebraicIntersections, certify_algebraic_intersections,
    verify_algebraic_intersections,
};
mod image_agreement;
mod multispan;
pub use image_agreement::{
    CurveImageCertificate, ImageCoverageWitness, ImageRelationWitness, VerifiedCurveImage,
    certify_curve_image_agreement, verify_curve_image_agreement,
};
mod branches;
mod span_intersections;
pub(crate) use branches::critical_samples;
pub use branches::{BranchCut, BranchSlab, ParameterBranchDecomposition};
pub(crate) fn exact_polynomial_nonnegative(p: &[Q]) -> Result<bool> {
    if arithmetic::at(p, &Q::zero()) < Q::zero() || arithmetic::at(p, &Q::one()) < Q::zero() {
        return Ok(false);
    }
    Ok(critical_samples(&[p.to_vec()])?
        .iter()
        .all(|x| arithmetic::at(p, x) >= Q::zero()))
}
pub use span_intersections::{
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
    SourceContactParameter, SpanIntersectionCertificate, SpanPairCertificate, VerifiedCurveOverlap,
    VerifiedSpanIntersections, assemble_original_contact_atlas,
    assemble_original_filled_boundaries, assemble_original_filled_regions,
    assemble_original_physical_faces, bind_original_periodic_pieces_to_surface,
    bind_original_seams_to_surface, certify_span_intersections, classify_original_shore_carrier,
    compare_event_parameters, own_branch_neighbourhood, own_curve_event_nodes, own_event_branch,
    own_original_atlas_pairs, own_original_boundary_winding, own_original_carrier_cycles,
    own_original_carrier_graph, own_original_carrier_sides,
    own_original_carrier_sides_automatically, own_original_carriers, own_original_chart_events,
    own_original_chart_intervals, own_original_chart_vertices, own_original_common_clips,
    own_original_common_dictionary, own_original_common_partition, own_original_contact_port_order,
    own_original_filled_boundaries, own_original_filled_regions,
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
    verify_span_intersections,
};
pub use span_intersections::{VerifiedBranchGraph, VerifiedParameterLocus};
mod nonlinear;
pub use agreement::{
    CurveAgreementCertificate, VerifiedCurveAgreement, certify_curve_agreement,
    verify_curve_agreement,
};
pub use intersections::{
    CurveIntersectionCertificate, VerifiedCurveIntersections, certify_curve_intersections,
    verify_curve_intersections,
};
pub use multispan::{
    MultispanAgreementCertificate, SpanAgreementWitness, certify_multispan_agreement,
    verify_multispan_agreement,
};
pub use nonlinear::{
    NonlinearCurveAgreementCertificate, certify_nonlinear_curve_agreement,
    verify_nonlinear_curve_agreement,
};
mod contact;
mod extraction;
mod measure;
mod minkowski;
mod offset;
mod offset_regions;
pub use minkowski::{
    ExactRegionRings, MinkowskiRegionCertificate, VerifiedMinkowskiRegion,
    certify_minkowski_disk_offset, certify_minkowski_sum, verify_minkowski_region,
};
mod offset_wire;
pub use contact::{
    EndpointContactCertificate, VerifiedEndpointContacts, certify_endpoint_contacts,
    verify_endpoint_contacts,
};
pub use offset::{
    OffsetCurveCertificate, VerifiedOffsetCurve, certify_offset_curve, verify_offset_curve,
};
pub use offset_regions::{
    OffsetRegionCertificate, VerifiedOffsetRegions, certify_offset_regions, verify_offset_regions,
};
pub use offset_wire::{
    OffsetWireCertificate, OffsetWirePart, VerifiedOffsetWire, certify_offset_wire,
    verify_offset_wire,
};

pub(crate) fn nonstationary_span(c: &Curve, axis: usize) -> Result<bool> {
    source_bits(&[vec![c.clone()]], 1e-7)?;
    if axis > 1 || c.control_points.len() != c.degree + 1 {
        return Ok(false);
    }
    let nets = extraction::nets(&[vec![c.clone()]])?;
    Ok(nets.len() == 1 && arithmetic::no_stationary(&nets[0].1, axis))
}
pub use measure::{
    ExactAreaInterval, ProfileMeasureCertificate, VerifiedMeasuredProfile, certify_measure,
    verify_measure,
};
#[cfg(test)]
mod extended_tests;
mod linear_arrangement;
#[cfg(test)]
mod measure_tests;
mod orientation;
mod point_winding;
mod producer;
pub(crate) mod topology_prisms;
mod winding;
pub use linear_arrangement::{LinearWindingArrangement, regularize_linear_winding_arrangement};
pub use point_winding::{
    PointWindingCertificate, RayCellWitness, RayRootWitness, VerifiedPointWinding,
    certify_point_winding, certify_rational_point_winding, verify_point_winding,
    verify_rational_point_winding,
};
pub use winding::{
    ApproximatePeriodicWindingRegions, CoherentPeriodicWindingRegions, PeriodicWindingRegions,
    SurfacePeriodicRegions, WindingComponent, WindingRegularization,
    bind_periodic_regions_to_surface, regularize_approximated_periodic_winding,
    regularize_coherent_periodic_winding, regularize_nonzero_winding, regularize_periodic_winding,
    regularize_projective_coherent_periodic_winding,
};
pub use winding::{
    ClockMappedPeriodicRegions, ClockMappedSurfaceRegions, ExactPeriodicContactRegions,
    LinearPeriodicContactRegions, MultispanChartAlias, MultispanPeriodicRegions,
    MultispanSurfaceRegions, OwnedClockMappedFaces, OwnedMultispanPeriodicFaces,
    OwnedPeriodicFaces, PeriodicFaceLimits, assemble_clock_mapped_surface_faces,
    assemble_multispan_periodic_surface_faces, assemble_periodic_surface_faces,
    bind_clock_mapped_regions_to_surface, bind_multispan_regions_to_surface,
    regularize_clock_mapped_periodic_winding, regularize_exact_periodic_contacts,
    regularize_linear_periodic_contacts, regularize_multispan_periodic_winding,
};
#[cfg(feature = "codec")]
mod serialization;
pub use producer::certify;

pub const VERSION: u32 = 1;
const MAX_SPANS: usize = 128;
const MAX_PIECES: usize = 256;
const MAX_DEPTH: u8 = 8;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileCertificate {
    pub version: u32,
    /// Lossless canonical source bits, including layout, knots and tolerance.
    /// Hex strings survive JSON/JavaScript without 64-bit integer rounding.
    pub source: String,
    pub subdivision_depth: u8,
    /// One independently checked strictly monotone coordinate per exact piece.
    pub monotone_axes: Vec<u8>,
    /// Complete lexicographic unordered-pair census; missing entries fail.
    /// 0/1 = coordinate normal; 2+8*i+j = legacy combined-hull edge normal.
    /// 256+32*i+j covers the larger nets of degree 4..8. 65535 is algebraic.
    pub separating_normals: Vec<u16>,
    pub orientation: Vec<i8>,
    pub nesting_depth: Vec<usize>,
    /// None uses the proved hull homotopy; Some(d) checks exact Green bounds
    /// after d dyadic subdivisions. Polynomial integrals are always exact.
    pub orientation_depth: Option<u8>,
}

/// Only returned after independent verification of the supplied source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedProfile {
    spans: usize,
    pieces: usize,
    nesting_depth: Vec<usize>,
}
impl VerifiedProfile {
    pub fn spans(&self) -> usize {
        self.spans
    }
    pub fn pieces(&self) -> usize {
        self.pieces
    }
    pub fn nesting_depth(&self) -> &[usize] {
        &self.nesting_depth
    }
}

fn refused(message: &str) -> Error {
    Error::new("BREP_PROFILE_CERTIFICATE_UNPROVEN", message)
}



#[derive(Clone)]
struct Piece {
    wire: usize,
    net: Vec<[Q; 3]>,
    points: Vec<[Q; 2]>,
}
impl Piece {
    fn from_net(wire: usize, net: Vec<[Q; 3]>) -> Self {
        let points = net.iter().map(|p| [&p[0] / &p[2], &p[1] / &p[2]]).collect();
        Self { wire, net, points }
    }
}

fn split(net: &[[Q; 3]]) -> (Vec<[Q; 3]>, Vec<[Q; 3]>) {
    let mut level = net.to_vec();
    let mut left = Vec::with_capacity(net.len());
    let mut right = Vec::with_capacity(net.len());
    left.push(level[0].clone());
    right.push(level.last().unwrap().clone());
    let two = Q::from_integer(2.into());
    while level.len() > 1 {
        for i in 0..level.len() - 1 {
            level[i] = std::array::from_fn(|k| (&level[i][k] + &level[i + 1][k]) / &two);
        }
        level.pop();
        left.push(level[0].clone());
        right.push(level.last().unwrap().clone());
    }
    right.reverse();
    (left, right)
}



fn normal(a: &Piece, b: &Piece, witness: u16) -> Option<[Q; 2]> {
    match witness {
        0 => Some([Q::one(), Q::zero()]),
        1 => Some([Q::zero(), Q::one()]),
        _ => {
            // Preserve legacy cubic codes 2..65. Higher-degree nets use a
            // disjoint 32-wide table; unused/invalid codes are refused.
            let (i, j) = if (2..66).contains(&witness) {
                (((witness - 2) / 8) as usize, ((witness - 2) % 8) as usize)
            } else if (256..1280).contains(&witness) {
                (
                    ((witness - 256) / 32) as usize,
                    ((witness - 256) % 32) as usize,
                )
            } else {
                return None;
            };
            let get = |idx: usize| -> Option<&[Q; 2]> {
                if idx < a.points.len() {
                    a.points.get(idx)
                } else {
                    b.points.get(idx - a.points.len())
                }
            };
            let (p, q) = (get(i)?, get(j)?);
            let n = [&p[1] - &q[1], &q[0] - &p[0]];
            (!n.iter().all(Zero::is_zero)).then_some(n)
        }
    }
}

fn normal_candidates(a: &Piece, b: &Piece) -> impl Iterator<Item = u16> {
    let count = a.points.len() + b.points.len();
    (0..2).chain((0..count).flat_map(move |i| {
        (i + 1..count).map(move |j| {
            if count <= 8 {
                2 + 8 * i as u16 + j as u16
            } else {
                256 + 32 * i as u16 + j as u16
            }
        })
    }))
}

fn adjacent(p: &[Piece], i: usize, j: usize) -> bool {
    if p[i].wire != p[j].wire {
        return false;
    }
    j == i + 1
        || (p[..i].iter().all(|q| q.wire != p[i].wire)
            && p[j + 1..].iter().all(|q| q.wire != p[i].wire))
}

// Exact positive-weight hull proof. All open Bernstein coefficients are
// positive, so a nonflat net strictly leaves a supporting plane in its entire
// open parameter interval, even at a repeated endpoint or stationary cusp.


fn cross(a: &[Q; 2], b: &[Q; 2], p: &[Q; 2]) -> Q {
    (&b[0] - &a[0]) * (&p[1] - &a[1]) - (&b[1] - &a[1]) * (&p[0] - &a[0])
}
fn polygon_winding(poly: &[&[Q; 2]], point: &[Q; 2]) -> Result<i32> {
    let mut winding = 0i32;
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
        let c = cross(a, b, point);
        if c.is_zero()
            && (*(&a[0]).min(&b[0]) <= point[0]
                && point[0] <= *(&a[0]).max(&b[0]))
            && (*(&a[1]).min(&b[1]) <= point[1]
                && point[1] <= *(&a[1]).max(&b[1]))
        {
            return Err(refused(
                "Certificate nesting query lies on a polygon boundary",
            ));
        }
        if a[1] <= point[1] && point[1] < b[1] && c > Q::zero() {
            winding += 1;
        }
        if b[1] <= point[1] && point[1] < a[1] && c < Q::zero() {
            winding -= 1;
        }
    }
    if winding.abs() > 1 {
        return Err(refused("Certificate polygon is not simple"));
    }
    Ok(winding)
}
fn polygon_claims(pieces: &[Piece], count: usize) -> Result<(Vec<i8>, Vec<usize>)> {
    let rings: Vec<Vec<_>> = (0..count)
        .map(|w| {
            pieces
                .iter()
                .filter(|p| p.wire == w)
                .map(|p| &p.points[0])
                .collect()
        })
        .collect();
    let mut signs = Vec::with_capacity(count);
    let mut depths = Vec::with_capacity(count);
    for (w, ring) in rings.iter().enumerate() {
        let area = (0..ring.len()).fold(Q::zero(), |s, i| {
            let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
            s + &a[0] * &b[1] - &a[1] * &b[0]
        });
        signs.push(if area > Q::zero() {
            1
        } else if area < Q::zero() {
            -1
        } else {
            0
        });
        let point = ring[0];
        let mut depth = 0;
        for (other, poly) in rings.iter().enumerate() {
            if other == w {
                continue;
            }
            let winding = polygon_winding(poly, point)?;
            depth += usize::from(winding != 0);
        }
        depths.push(depth);
    }
    Ok((signs, depths))
}

/// Search-free independent checker. No numerical geometry implementation is
/// called. Every supplied witness is checked using exact integer/rational
/// arithmetic; an unproved or malformed claim returns an error.
pub fn verify(
    loops: &[Vec<Curve>],
    tolerance: f64,
    c: &ProfileCertificate,
) -> Result<VerifiedProfile> {
    let (verified, orientation) = verify_geometry(loops, tolerance, c)?;
    if orientation
        .iter()
        .zip(verified.nesting_depth())
        .any(|(s, d)| (*s == 1) != (*d % 2 == 0))
    {
        return Err(refused(
            "Certificate orientation or material-left nesting claim is false",
        ));
    }
    Ok(verified)
}
fn verify_geometry(
    loops: &[Vec<Curve>],
    tolerance: f64,
    c: &ProfileCertificate,
) -> Result<(VerifiedProfile, Vec<i8>)> {
    if c.version != VERSION || c.source != source_bits(loops, tolerance)? {
        return Err(refused(
            "Certificate version or exact source bits do not match",
        ));
    }
    let p = pieces(loops, c.subdivision_depth)?;
    if c.monotone_axes.len() != p.len()
        || c.separating_normals.len() != p.len() * p.len().saturating_sub(1) / 2
    {
        return Err(refused("Certificate omits or adds span/pair claims"));
    }
    for (piece, axis) in p.iter().zip(&c.monotone_axes) {
        if *axis > 1 || !arithmetic::monotone(&piece.net, *axis as usize)? {
            return Err(refused("Certificate monotonicity claim is false"));
        }
    }
    let mut pair = 0;
    let mut algebraic = false;
    for i in 0..p.len() {
        for j in i + 1..p.len() {
            if c.separating_normals[pair] == u16::MAX {
                // Other-wire hull separation also proves that nesting query
                // points remain outside every span throughout its homotopy.
                if p[i].wire != p[j].wire
                    || !arithmetic::endpoint_only(&p[i].net, &p[j].net, adjacent(&p, i, j))?
                {
                    return Err(refused("Certificate algebraic separation claim is false"));
                }
                algebraic = true;
            } else if c.separating_normals[pair] == u16::MAX - 1 {
                if p[i].wire != p[j].wire
                    || !arithmetic::endpoint_census(&p[i].net, &p[j].net, adjacent(&p, i, j))?
                {
                    return Err(refused("General algebraic endpoint census is false"));
                }
                algebraic = true;
            } else {
                let n = normal(&p[i], &p[j], c.separating_normals[pair])
                    .ok_or_else(|| refused("Invalid separator witness"))?;
                if !separated(&p[i], &p[j], &n, adjacent(&p, i, j)) {
                    return Err(refused("Certificate separation claim is false"));
                }
            }
            pair += 1;
        }
    }
    let (mut orientation, nesting_depth) = polygon_claims(&p, loops.len())?;
    if let Some(depth) = c.orientation_depth {
        orientation = orientation::check(loops, depth)?;
    } else if algebraic || orientation.contains(&0) {
        return Err(refused(
            "Algebraic contacts require a separate exact orientation proof",
        ));
    }
    if c.orientation != orientation || c.nesting_depth != nesting_depth {
        return Err(refused(
            "Certificate orientation or material-left nesting claim is false",
        ));
    }
    Ok((
        VerifiedProfile {
            spans: p.len() / (1usize << c.subdivision_depth),
            pieces: p.len(),
            nesting_depth,
        },
        orientation,
    ))
}

#[cfg(test)]
#[path = "profile_certificate/tests/mod.rs"]
mod tests;

pub use span_intersections::{OwnedTranslatedSpanIntersections, own_translated_span_intersections};

pub use span_intersections::{
    OriginalPieceEvent, OriginalPieceEventLocation, OwnedPeriodicPairEvents,
    own_periodic_pair_events,
};

pub use span_intersections::{
    OriginalPeriodicCommonClip, OwnedPeriodicCommonClips, own_periodic_common_clips,
};

pub use span_intersections::{
    OriginalPeriodicArrangementCut, OriginalPeriodicArrangementInterval,
    OriginalPeriodicArrangementOrigin, OriginalPeriodicArrangementUse,
    OwnedPeriodicArrangementCuts, own_periodic_arrangement_cuts,
};

pub use span_intersections::{
    OriginalPeriodicAlignedCut, OriginalPeriodicAlignedOrigin, OriginalPeriodicIntervalAlias,
    OriginalPeriodicIntervalBinding, OwnedPeriodicAlignedIntervals, own_periodic_aligned_intervals,
};

pub use span_intersections::{
    OriginalPeriodicChartCarrier, OriginalPeriodicChartCutRef, OriginalPeriodicChartEdge,
    OriginalPeriodicChartIncidence, OriginalPeriodicChartRelation,
    OriginalPeriodicChartRelationOrigin, OriginalPeriodicChartVertex, OwnedPeriodicChartGraph,
    own_periodic_chart_graph,
};

pub use span_intersections::own_periodic_pair_events_translated;

pub use span_intersections::{
    OriginalPeriodicSeamNode, OriginalPeriodicSeamPair, OwnedPeriodicSeamContacts,
    own_periodic_seam_contacts,
};

pub use span_intersections::own_periodic_quotient_arrangement_cuts;

pub use span_intersections::{OwnedPeriodicQuotientGraph, own_periodic_quotient_graph};

pub use span_intersections::{
    OriginalPeriodicVertexChart, OriginalPeriodicVertexChartUse, OwnedPeriodicVertexCharts,
    own_periodic_vertex_charts,
};

pub use span_intersections::{OriginalPeriodicPort, OwnedPeriodicPorts, own_periodic_ports};

pub use span_intersections::{
    OriginalPeriodicTangentWitness, OwnedPeriodicRotation, own_periodic_rotation,
};

pub use span_intersections::{OwnedPeriodicCycles, own_periodic_cycles};

pub use span_intersections::{
    OriginalPeriodicBoundaryPart, OriginalPeriodicBoundaryWalk, OwnedPeriodicBoundaryLifts,
    own_periodic_boundary_lifts,
};

pub use span_intersections::{
    OriginalPeriodicBalanceTerm, OriginalPeriodicEdgeJump, OriginalPeriodicVertexBalance,
    OriginalPeriodicWireContribution, OriginalPeriodicWireFlux, OriginalPeriodicWireJump,
    OwnedPeriodicWireJumps, own_periodic_wire_jumps,
};

pub use span_intersections::{
    OriginalPeriodicPathCrossing, OwnedPeriodicRelativeWinding, own_periodic_relative_winding,
};

pub use span_intersections::{
    OriginalPeriodicBooleanOp, OriginalPeriodicFillRule, OriginalPeriodicOperand,
    OriginalPeriodicOperandPeriods, OwnedPeriodicMaterialRule, OwnedPeriodicMaterialSample,
    own_periodic_material_rule, own_periodic_material_sample,
};

pub use span_intersections::{
    OwnedPeriodicChartSideProbes, OwnedPeriodicMaterialSides, own_periodic_chart_side_probes,
    own_periodic_material_sides,
};

pub use span_intersections::{
    OriginalPeriodicSelectedEdge, OwnedPeriodicBoundarySelection, own_periodic_boundary_selection,
};

pub use span_intersections::{
    OriginalPeriodicSideView, OwnedPeriodicSideProbes, own_periodic_physical_side_probes,
};

pub use span_intersections::{
    OriginalPeriodicMaterialSector, OwnedPeriodicMaterialCycles, own_periodic_material_cycles,
};

pub use span_intersections::{OwnedPeriodicMaterialLifts, own_periodic_material_lifts};

pub use span_intersections::{
    OriginalNaturalBoundaryCurve, OriginalPeriodicNaturalBoundary, OriginalPeriodicSupportWireRole,
    OwnedPeriodicSupportBoundaries, own_periodic_support_boundaries,
};

pub use span_intersections::{
    OriginalPeriodicNaturalCarrier, OwnedPeriodicSupportMaterialRule,
    OwnedPeriodicSupportSelection, OwnedPeriodicSupportSideProbes, OwnedPeriodicSupportSides,
    own_periodic_support_material_rule, own_periodic_support_selection,
    own_periodic_support_side_probes, own_periodic_support_sides,
};

pub use span_intersections::{
    OriginalPeriodicSupportSector, OriginalPeriodicSupportVertex, OwnedPeriodicSupportCycles,
    OwnedPeriodicSupportLifts, own_periodic_support_cycles, own_periodic_support_lifts,
};

pub use span_intersections::{OwnedPeriodicSupportPath, own_periodic_support_path};

pub use span_intersections::{
    OriginalPeriodicCochainTerm, OriginalPeriodicMaterialComponent, OriginalPeriodicRegionProbe,
    OriginalPeriodicSeparatingCochain, OwnedPeriodicSupportRegions, OwnedPeriodicSupportSignatures,
    own_periodic_support_regions, own_periodic_support_signatures,
};

pub use span_intersections::{
    OriginalPeriodicSurfaceTrace, OriginalPeriodicSurfaceTraceEndpoint,
    OriginalPeriodicSurfaceTracePoint, OwnedPeriodicSupportTraces, own_periodic_support_traces,
};
pub use span_intersections::{
    PeriodicRegionSearchLimits, PeriodicRegionSearchReport, PeriodicRegionSearchStop,
    construct_periodic_support_regions,
};

pub use span_intersections::{
    OriginalPeriodicSurfaceBoundaryFan, OriginalPeriodicSurfaceEndpointUse,
    OriginalPeriodicSurfaceIncidenceEdge, OriginalPeriodicSurfaceIncidenceLoop,
    OriginalPeriodicSurfaceIncidenceVertex, OwnedPeriodicSupportIncidence,
    own_periodic_support_incidence,
};

pub use span_intersections::{
    OriginalPeriodicSourceChartCopy, OriginalPeriodicSourceChartWorldPoint,
    OwnedPeriodicSourceChartCopies, PeriodicSourceChartCopyLimits,
    own_periodic_source_chart_copies,
};

pub use span_intersections::{
    OriginalPeriodicNativeChartCorner, OriginalPeriodicNativeCornerVertex,
    OwnedPeriodicNativeChartCorners, own_periodic_native_chart_corners,
};

pub use span_intersections::{
    OriginalPeriodicNativeShore, OriginalPeriodicNativeShorePoint, OriginalPeriodicNativeShoreRole,
    OwnedPeriodicNativeChartShores, PeriodicNativeShoreLimits, own_periodic_native_chart_shores,
};

pub use span_intersections::{
    OriginalPeriodicShoreMaterialSample, OriginalPeriodicShoreSampleOrigin,
    OwnedPeriodicShoreMaterialSamples, PeriodicShoreMaterialSampleLimits,
    own_periodic_shore_material_samples,
};

pub use span_intersections::{
    OriginalPeriodicShoreContact, OriginalPeriodicShoreContactKind,
    OwnedPeriodicShoreMaterialIntervals, PeriodicShoreMaterialIntervalLimits,
    own_periodic_shore_material_intervals,
};

pub use span_intersections::{
    OriginalPeriodicChartCycleEdge, OriginalPeriodicChartCycleEdgeKind,
    OriginalPeriodicChartCycleVertex, OriginalPeriodicChartVertexKey, OwnedPeriodicChartCycles,
    PeriodicChartCycleLimits, own_periodic_chart_cycles,
};

pub use span_intersections::{
    OwnedPeriodicChartWinding, PeriodicChartWindingLimits, construct_periodic_chart_winding,
    own_periodic_chart_winding,
};

pub use span_intersections::{
    OriginalPeriodicChartCycleSides, OwnedPeriodicChartCycleSides, PeriodicChartCycleOrientation,
    PeriodicChartCycleSideLimits, own_periodic_chart_cycle_sides,
};

pub use span_intersections::{
    OriginalPeriodicChartStrictPatch, OwnedPeriodicChartStrictPatches,
    PeriodicChartStrictPatchLimits, own_periodic_chart_strict_patches,
};

pub use span_intersections::{
    OriginalPeriodicChartBoundaryLoop, OriginalPeriodicChartLoopContact,
    OwnedPeriodicChartBoundaryLoops, PeriodicChartBoundaryLoopLimits,
    own_periodic_chart_boundary_loops,
};

pub use span_intersections::{OwnedPeriodicChartLoopWinding, own_periodic_chart_loop_winding};

pub use span_intersections::{
    OwnedPeriodicChartFanPatches, PeriodicChartFanPatchLimits, own_periodic_chart_fan_patches,
};

pub use span_intersections::{
    OriginalPeriodicSewnCoedge, OriginalPeriodicSewnEdge, OriginalPeriodicSewnFace,
    OriginalPeriodicSewnVertex, OwnedPeriodicSewnFaces, PeriodicSewnFaceLimits,
    own_periodic_sewn_faces,
};

#[path="profile_certificate/algorithm_source_bits.rs"]
mod algorithm_source_bits;
use algorithm_source_bits::source_bits;
#[path="profile_certificate/algorithm_pieces.rs"]
mod algorithm_pieces;
use algorithm_pieces::pieces;
#[path="profile_certificate/algorithm_separated.rs"]
mod algorithm_separated;
use algorithm_separated::separated;
