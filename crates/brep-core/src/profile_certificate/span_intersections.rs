//! Complete per-knot-span census of isolated and coincident curve components.
//! Algebraic source cells never pass through rounded temporary NURBS controls.
use super::*;
mod translated;
pub use translated::{OwnedTranslatedSpanIntersections, own_translated_span_intersections};
mod event_nodes;
pub use event_nodes::{OwnedOriginalPeriodicEmbedding, own_original_periodic_embedding, OriginalPeriodicSourceUse, OriginalPeriodicWireUse, OriginalPeriodicJoinKind, OriginalPeriodicWireJoin, OriginalPeriodicWire, OwnedOriginalPeriodicWires, own_original_periodic_wires, OriginalPeriodicPieceCoverage, OwnedOriginalPeriodicSurface, bind_original_periodic_pieces_to_surface, OriginalPeriodicEvent, OriginalPeriodicShoreCell, OriginalPeriodicCutOrigin, OriginalPeriodicCut, OriginalPeriodicPiece, OriginalPeriodicTransition, OriginalPeriodicPoint, OwnedOriginalPeriodicPieces, own_original_periodic_pieces, OriginalChartCutOrigin, OriginalChartCut, OriginalChartRegion, OriginalChartInterval, OwnedOriginalChartIntervals, own_original_chart_intervals, OriginalChartShoreEvent, OriginalChartShoreCell, OwnedOriginalChartEvents, own_original_chart_events, OwnedOriginalUnifiedRegions, own_original_unified_regions, OwnedOriginalUnifiedBoundaryWinding, own_original_unified_boundary_winding, OwnedOriginalUnifiedFilled, own_original_unified_filled, OwnedOriginalUnifiedSides, own_original_unified_sides, own_original_unified_sides_automatically, OwnedOriginalUnifiedCycles, own_original_unified_cycles, OwnedOriginalUnifiedRotation, own_original_unified_rotation, OriginalUnifiedPort, OwnedOriginalUnifiedPorts, own_original_unified_ports, OriginalUnifiedVertex, OriginalUnifiedEdgeCarrier, OriginalUnifiedEdge, OriginalUnifiedIncidence, OwnedOriginalUnifiedGraph, own_original_unified_graph, OriginalSourceParameter, OriginalSourceCutOrigin, OriginalSourceCut, OriginalSourceCommonUse, OriginalUnifiedCarrier, OwnedOriginalSourceCarriers, own_original_source_carriers, OriginalCommonPartitionRef, OriginalCommonEdgeUse, OriginalCommonEdge, OwnedOriginalCommonDictionary, own_original_common_dictionary, OriginalCommonCutOrigin, OriginalCommonCut, OriginalCommonContributor, OriginalCommonSegment, OwnedOriginalCommonPartition, own_original_common_partition, OriginalCommonClipOrigin, OriginalCommonClipBoundary, OriginalCommonClip, OwnedOriginalCommonClips, own_original_common_clips, OriginalPointPreimage, OwnedOriginalPointPullback, own_original_point_pullback, OriginalPairCutOrigin, OriginalPairCut, OwnedOriginalPairCuts, own_original_pair_cuts, OriginalOverlapContactRef, OwnedOriginalOverlapPartition, own_original_overlap_partition, OriginalOverlapSplit, OriginalOverlapContact, OwnedOriginalOverlapContacts, own_original_overlap_contacts, OriginalOverlapComponent, OwnedOriginalOverlapComponents, own_original_overlap_components, OriginalPhysicalVertex, OriginalPhysicalTrace, OriginalPhysicalEdgeUse, OriginalPhysicalEdge, OriginalPhysicalFacePatch, OriginalPhysicalFace, OwnedOriginalPhysicalFaces, OriginalPhysicalFaceLimits, own_original_physical_faces, assemble_original_physical_faces, OwnedOriginalGlobalChartEmbedding, own_original_global_chart_embedding, OriginalAtlasPairLimits, OriginalAtlasPairProof, OriginalAtlasPair, OwnedOriginalAtlasPairs, own_original_atlas_pairs, OriginalInjectiveAtlasLimits, OriginalInjectiveChart, OwnedOriginalInjectiveChartAtlas, own_original_injective_chart_atlas, OriginalSurfaceRegularityRecord, OwnedOriginalRegularTrimSurface, own_original_regular_trim_surface, OriginalChartCoverageCell, OwnedOriginalTrimChartCoverage, own_original_trim_chart_coverage, OriginalQuotientCoedge, OriginalQuotientEdge, OriginalQuotientVertexLink, OwnedOriginalPeriodicTopology, own_original_periodic_topology, OwnedOriginalSurfaceSeams, bind_original_seams_to_surface, OriginalSeamUse, OwnedOriginalSeamDictionary, own_original_seam_dictionary, OriginalShoreMembership, OwnedOriginalShoreMembership, classify_original_shore_carrier, OwnedOriginalShorePair, own_original_shore_pair, OwnedOriginalShoreCarrier, own_original_shore_carrier, OriginalChartVertexLimits, OwnedOriginalChartVertices, own_original_chart_vertices, assemble_original_filled_regions, OriginalFilledRegion, OwnedOriginalFilledRegions, own_original_filled_regions, OwnedOriginalBoundaryWinding, own_original_boundary_winding, OriginalBoundaryAssemblyLimits, own_original_carrier_sides_automatically, assemble_original_filled_boundaries, OwnedOriginalFilledBoundaries, own_original_filled_boundaries, OwnedOriginalCarrierSides, own_original_carrier_sides, OwnedOriginalCarrierCycles, own_original_carrier_cycles, OriginalCarrierVertex, OriginalCarrierIncidence, OwnedOriginalCarrierGraph, own_original_carrier_graph, OriginalCarrierCut, OriginalAtomicCarrier, OwnedOriginalCarriers, own_original_carriers, BranchNeighbourhoodLimits, BranchNeighbourhoodRecord, OwnedBranchNeighbourhood, own_branch_neighbourhood, ContactPortOrderLimits, OriginalContactPort, OwnedContactPortOrder, own_original_contact_port_order, try_own_event_branch, LeadingBranchRay, OwnedEventBranch, own_event_branch, ContactAtlasLimits, ContactOccurrence, SourceContactParameter, OriginalContactAtlas, assemble_original_contact_atlas, EventParameterRef, ParameterOrderLimits, QualifiedParameterOrder, compare_event_parameters, AlgebraicEventNode, AlgebraicEventPosition, CurveEventNodes, own_curve_event_nodes};
use algebraic_intersections::{ExactRatio, decode_factors, encode_factors, outward, ratio};
use arithmetic::algebraic_events as engine;
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
fn witness(
    a: &Cell,
    b: &Cell,
    i: usize,
    j: usize,
    diagonal: bool,
    w: f64,
) -> Result<SpanPairCertificate> {
    let mut p = SpanPairCertificate {
        first: i,
        second: j,
        kind: String::new(),
        axis: 0,
        shear: 0,
        factors: Vec::new(),
        relation: Vec::new(),
        branches: None,
    };
    for axis in 0..2 {
        if separated(a, b, axis) {
            p.kind = "separate".into();
            p.axis = axis as u8;
            return Ok(p);
        }
        if diagonal && arithmetic::monotone(&a.net, axis)? {
            p.kind = "stationary".into();
            p.axis = axis as u8;
            p.factors = encode_factors(&engine::stationary(&a.net, &width(a, b, w))?);
            return Ok(p);
        }
        if !diagonal && let Ok((lo, hi, _)) = range(a, b, axis) {
            if lo >= hi {
                p.kind = "range".into();
                p.axis = axis as u8;
                return Ok(p);
            }
            if let Ok(h) = arithmetic::bivariate::partial_relation(&a.net, &b.net, axis) {
                p.kind = "overlap".into();
                p.axis = axis as u8;
                p.relation = h
                    .iter()
                    .map(|row| row.iter().map(ratio).collect())
                    .collect();
                return Ok(p);
            }
        }
    }
    if let Some((h, residual)) = arithmetic::bivariate::component_system(&a.net, &b.net, diagonal)?
    {
        if h.is_empty() {
            p.kind = "whole-square".into();
            return Ok(p);
        }
        let (k, f) = engine::produce_system(&residual, diagonal, &width(a, b, w))?;
        p.kind = "locus".into();
        p.shear = k;
        p.factors = encode_factors(&f);
        p.relation = h
            .iter()
            .map(|row| row.iter().map(ratio).collect())
            .collect();
        return Ok(p);
    }
    let (k, f) = engine::produce(&a.net, &b.net, diagonal, &width(a, b, w))?;
    p.kind = "events".into();
    p.shear = k;
    p.factors = encode_factors(&f);
    Ok(p)
}
pub fn certify_span_intersections(
    a: &Curve,
    b: &Curve,
    self_pair: bool,
    w: f64,
) -> Result<SpanIntersectionCertificate> {
    let source = source(a, b, self_pair, w)?;
    let x = cells(a)?;
    let y = cells(b)?;
    let mut pairs = Vec::new();
    for i in 0..x.len() {
        for j in 0..y.len() {
            if self_pair && i > j {
                continue;
            }
            let mut pair = witness(&x[i], &y[j], i, j, self_pair && i == j, w)?;
            if matches!(pair.kind.as_str(), "overlap" | "locus" | "whole-square") {
                let h = if pair.kind == "whole-square" {
                    Vec::new()
                } else {
                    decode_locus(&pair.relation)?
                };
                pair.branches = Some(branches::decompose(
                    &h,
                    self_pair.then(|| local_diagonal(&x[i], &y[j])),
                    &width(&x[i], &y[j], w),
                )?);
            }
            pairs.push(pair);
        }
    }
    let c = SpanIntersectionCertificate {
        version: 3,
        source,
        self_pair,
        maximum_parameter_width: w,
        pairs,
    };
    verify_span_intersections(a, b, &c)?;
    Ok(c)
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
fn verify_cells(x: &[Cell], y: &[Cell], c: &SpanIntersectionCertificate) -> Result<VerifiedSpanIntersections> {
    if c.pairs.len() > 4096 {
        return Err(refused("Translated span pair census exceeds allowance"));
    }
    let mut n = 0;
    let mut out = VerifiedSpanIntersections {
        events: Vec::new(),
        singular: Vec::new(),
        overlaps: Vec::new(),
        algebraic_loci: Vec::new(),
        stationary_spans: Vec::new(),
        branch_decompositions: Vec::new(),
    };
    for i in 0..x.len() {
        for j in 0..y.len() {
            if c.self_pair && i > j {
                continue;
            }
            let p = c
                .pairs
                .get(n)
                .ok_or_else(|| refused("Multi-span intersection pair is missing"))?;
            n += 1;
            if p.first != i || p.second != j || p.axis > 1 {
                return Err(refused(
                    "Multi-span intersection pairs are reordered or false",
                ));
            }
            let a = &x[i];
            let b = &y[j];
            let diagonal = c.self_pair && i == j;
            if !matches!(p.kind.as_str(), "events" | "locus") && p.shear != 0
                || !matches!(p.kind.as_str(), "overlap" | "locus") && !p.relation.is_empty()
                || !matches!(p.kind.as_str(), "events" | "stationary" | "locus")
                    && !p.factors.is_empty()
            {
                return Err(refused("Extraneous multi-span pair proof claims"));
            }
            let events = match p.kind.as_str() {
                "locus" | "whole-square" => {
                    if c.version < 2 {
                        return Err(refused(
                            "Algebraic parameter loci require span certificate version >=2",
                        ));
                    }
                    let (h, residual) =
                        arithmetic::bivariate::component_system(&a.net, &b.net, diagonal)?
                            .ok_or_else(|| refused("False common parameter locus"))?;
                    let whole = h.is_empty();
                    if whole != (p.kind == "whole-square")
                        || (!whole && decode_locus(&p.relation)? != h)
                    {
                        return Err(refused("Incomplete or false common parameter component"));
                    }
                    let domain = |cell: &Cell| -> Result<[f64; 2]> {
                        Ok([
                            outward(&cell.domain[0], false)?,
                            outward(&cell.domain[1], true)?,
                        ])
                    };
                    out.algebraic_loci.push(VerifiedParameterLocus {
                        first_span: i,
                        second_span: j,
                        first_domain: domain(a)?,
                        second_domain: domain(b)?,
                        whole_square: whole,
                        diagonal_excluded: c.self_pair,
                        relation: p.relation.clone(),
                    });
                    if whole && diagonal {
                        out.stationary_spans.push(domain(a)?)
                    }
                    if whole {
                        Vec::new()
                    } else {
                        let w = width(a, b, c.maximum_parameter_width);
                        let mut events = engine::verify_system(
                            &residual,
                            diagonal,
                            &w,
                            p.shear,
                            &decode_factors(&p.factors)?,
                        )?;
                        if diagonal {
                            let stationary = engine::stationary(&a.net, &w)?;
                            events.extend(engine::verify_stationary(&a.net, &w, &stationary)?);
                        }
                        events
                    }
                }
                "separate" => {
                    if !separated(a, b, p.axis as usize) {
                        return Err(refused("False exact source hull separation"));
                    }
                    Vec::new()
                }
                "stationary" => {
                    if !diagonal || !arithmetic::monotone(&a.net, p.axis as usize)? {
                        return Err(refused("False diagonal injectivity claim"));
                    }
                    engine::verify_stationary(
                        &a.net,
                        &width(a, b, c.maximum_parameter_width),
                        &decode_factors(&p.factors)?,
                    )?
                }
                "range" => {
                    if diagonal {
                        return Err(refused("Self diagonal needs a stationary census"));
                    }
                    let (lo, hi, events) = range(a, b, p.axis as usize)?;
                    if lo < hi {
                        return Err(refused("False separated image ranges"));
                    }
                    events
                }
                "overlap" => {
                    if diagonal {
                        return Err(refused("A self diagonal cannot become a common component"));
                    }
                    let (lo, hi, _) = range(a, b, p.axis as usize)?;
                    let h = nonlinear::decode(&p.relation)?;
                    if lo >= hi
                        || !arithmetic::bivariate::verify_partial_relation(
                            &a.net,
                            &b.net,
                            p.axis as usize,
                            &h,
                        )?
                    {
                        return Err(refused(
                            "Common curve component or its image coverage is false",
                        ));
                    }
                    let domain = |c: &Cell| {
                        [
                            outward(&c.domain[0], false).unwrap(),
                            outward(&c.domain[1], true).unwrap(),
                        ]
                    };
                    out.overlaps.push(VerifiedCurveOverlap {
                        first_span: i,
                        second_span: j,
                        first_domain: domain(a),
                        second_domain: domain(b),
                        axis: p.axis,
                        image_range: [ratio(&lo), ratio(&hi)],
                        relation: p.relation.clone(),
                    });
                    Vec::new()
                }
                "events" => engine::verify(
                    &a.net,
                    &b.net,
                    diagonal,
                    &width(a, b, c.maximum_parameter_width),
                    p.shear,
                    &decode_factors(&p.factors)?,
                )?,
                _ => return Err(refused("Unknown multi-span pair proof kind")),
            };
            if matches!(p.kind.as_str(), "overlap" | "locus" | "whole-square") && c.version == 3 {
                let h = if p.kind == "whole-square" {
                    Vec::new()
                } else {
                    decode_locus(&p.relation)?
                };
                let graph = branches::decompose(
                    &h,
                    c.self_pair.then(|| local_diagonal(a, b)),
                    &width(a, b, c.maximum_parameter_width),
                )?;
                if p.branches.as_ref() != Some(&graph) {
                    return Err(refused("False or incomplete parameter branch graph"));
                }
                out.branch_decompositions.push(VerifiedBranchGraph {
                    first_span: i,
                    second_span: j,
                    decomposition: graph,
                });
            } else if p.branches.is_some() {
                return Err(refused("Extraneous parameter branch graph"));
            }
            let global = |c: &Cell, p: &[Q; 2]| -> Result<[f64; 2]> {
                let f = |t: &Q| &c.domain[0] + (&c.domain[1] - &c.domain[0]) * t;
                Ok([outward(&f(&p[0]), false)?, outward(&f(&p[1]), true)?])
            };
            for e in events {
                if e.singular {
                    out.singular.push(global(a, &e.first)?);
                    continue;
                }
                if (e.first == [Q::one(), Q::one()] && i + 1 < x.len())
                    || (e.second == [Q::one(), Q::one()] && j + 1 < y.len())
                {
                    continue;
                }
                let first = global(a, &e.first)?;
                let second = global(b, &e.second)?;
                if c.self_pair && first[0] == first[1] && first == second {
                    continue;
                }
                out.events.push([first, second]);
            }
        }
    }
    if n != c.pairs.len() {
        return Err(refused("Multi-span intersection census has extra pairs"));
    }
    out.events.sort_by(|a, b| {
        a[0][0]
            .total_cmp(&b[0][0])
            .then(a[1][0].total_cmp(&b[1][0]))
    });
    out.events.dedup();
    out.singular.sort_by(|a, b| a[0].total_cmp(&b[0]));
    out.singular.dedup();
    Ok(out)
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
mod tests {
    use super::*;
    #[test]
    fn complete_multi_span_components_and_isolated_knots() {
        let a = Curve::from_polyline(vec![vec![0., 0.], vec![4., 0.], vec![4., 4.], vec![0., 4.]])
            .unwrap();
        let b = Curve::from_polyline(vec![vec![2., 0.], vec![4., 0.], vec![4., 2.], vec![4., 4.]])
            .unwrap();
        let c = certify_span_intersections(&a, &b, false, 1e-12).unwrap();
        let r = verify_span_intersections(&a, &b, &c).unwrap();
        assert!(r.overlaps.len() >= 3);
        let mut bad = c;
        bad.pairs.pop();
        assert!(verify_span_intersections(&a, &b, &bad).is_err());
        let a = Curve::from_polyline(vec![vec![0., 0.], vec![3., 3.], vec![6., 0.]]).unwrap();
        let b = Curve::from_polyline(vec![vec![0., 1.], vec![6., 1.]]).unwrap();
        let c = certify_span_intersections(&a, &b, false, 1e-12).unwrap();
        assert_eq!(
            verify_span_intersections(&a, &b, &c).unwrap().events.len(),
            2
        );
    }
}

#[cfg(test)]
mod locus_tests;
pub use translated::{OriginalPieceEventLocation, OriginalPieceEvent, OwnedPeriodicPairEvents, own_periodic_pair_events};

pub use translated::{OriginalPeriodicCommonClip, OwnedPeriodicCommonClips, own_periodic_common_clips};

pub use translated::{OriginalPeriodicArrangementOrigin, OriginalPeriodicArrangementCut, OriginalPeriodicArrangementUse, OriginalPeriodicArrangementInterval, OwnedPeriodicArrangementCuts, own_periodic_arrangement_cuts};

pub use translated::{OriginalPeriodicAlignedOrigin, OriginalPeriodicAlignedCut, OriginalPeriodicIntervalBinding, OriginalPeriodicIntervalAlias, OwnedPeriodicAlignedIntervals, own_periodic_aligned_intervals};

pub use translated::{OriginalPeriodicChartCutRef, OriginalPeriodicChartRelationOrigin, OriginalPeriodicChartRelation, OriginalPeriodicChartIncidence, OriginalPeriodicChartVertex, OriginalPeriodicChartCarrier, OriginalPeriodicChartEdge, OwnedPeriodicChartGraph, own_periodic_chart_graph};

pub use translated::own_periodic_pair_events_translated;

pub use translated::{OriginalPeriodicSeamNode, OriginalPeriodicSeamPair, OwnedPeriodicSeamContacts, own_periodic_seam_contacts};

pub use translated::own_periodic_quotient_arrangement_cuts;

pub use translated::{OwnedPeriodicQuotientGraph, own_periodic_quotient_graph};

pub use translated::{OriginalPeriodicVertexChartUse, OriginalPeriodicVertexChart, OwnedPeriodicVertexCharts, own_periodic_vertex_charts};

pub use translated::{OriginalPeriodicPort, OwnedPeriodicPorts, own_periodic_ports};

pub use translated::{OriginalPeriodicTangentWitness, OwnedPeriodicRotation, own_periodic_rotation};

pub use translated::{OwnedPeriodicCycles, own_periodic_cycles};

pub use translated::{OriginalPeriodicBoundaryPart, OriginalPeriodicBoundaryWalk, OwnedPeriodicBoundaryLifts, own_periodic_boundary_lifts};

pub use translated::{OriginalPeriodicWireContribution, OriginalPeriodicWireJump, OriginalPeriodicEdgeJump, OriginalPeriodicBalanceTerm, OriginalPeriodicVertexBalance, OriginalPeriodicWireFlux, OwnedPeriodicWireJumps, own_periodic_wire_jumps};

pub use translated::{OriginalPeriodicPathCrossing, OwnedPeriodicRelativeWinding, own_periodic_relative_winding};

pub use translated::{OriginalPeriodicFillRule, OriginalPeriodicOperand, OriginalPeriodicBooleanOp, OriginalPeriodicOperandPeriods, OwnedPeriodicMaterialRule, OwnedPeriodicMaterialSample, own_periodic_material_rule, own_periodic_material_sample};

pub use translated::{OwnedPeriodicChartSideProbes, OwnedPeriodicMaterialSides, own_periodic_chart_side_probes, own_periodic_material_sides};

pub use translated::{OriginalPeriodicSelectedEdge, OwnedPeriodicBoundarySelection, own_periodic_boundary_selection};

pub use translated::{OwnedPeriodicSideProbes, OriginalPeriodicSideView, own_periodic_physical_side_probes};

pub use translated::{OriginalPeriodicMaterialSector, OwnedPeriodicMaterialCycles, own_periodic_material_cycles};

pub use translated::{OwnedPeriodicMaterialLifts, own_periodic_material_lifts};

pub use translated::{OriginalPeriodicSupportWireRole, OriginalNaturalBoundaryCurve, OriginalPeriodicNaturalBoundary, OwnedPeriodicSupportBoundaries, own_periodic_support_boundaries};

pub use translated::{OriginalPeriodicNaturalCarrier, OwnedPeriodicSupportSideProbes, own_periodic_support_side_probes, OwnedPeriodicSupportMaterialRule, own_periodic_support_material_rule, OwnedPeriodicSupportSides, own_periodic_support_sides, OwnedPeriodicSupportSelection, own_periodic_support_selection};

pub use translated::{OriginalPeriodicSupportVertex, OriginalPeriodicSupportSector, OwnedPeriodicSupportCycles, own_periodic_support_cycles, OwnedPeriodicSupportLifts, own_periodic_support_lifts};

pub use translated::{OwnedPeriodicSupportPath, own_periodic_support_path};

pub use translated::{OriginalPeriodicCochainTerm, OriginalPeriodicSeparatingCochain, OriginalPeriodicRegionProbe, OwnedPeriodicSupportSignatures, own_periodic_support_signatures, OriginalPeriodicMaterialComponent, OwnedPeriodicSupportRegions, own_periodic_support_regions};

pub use translated::{PeriodicRegionSearchLimits, PeriodicRegionSearchStop, PeriodicRegionSearchReport, construct_periodic_support_regions};
pub use translated::{OriginalPeriodicSurfaceTrace, OriginalPeriodicSurfaceTracePoint, OriginalPeriodicSurfaceTraceEndpoint, OwnedPeriodicSupportTraces, own_periodic_support_traces};

pub use translated::{OriginalPeriodicSurfaceEndpointUse, OriginalPeriodicSurfaceBoundaryFan, OriginalPeriodicSurfaceIncidenceVertex, OriginalPeriodicSurfaceIncidenceEdge, OriginalPeriodicSurfaceIncidenceLoop, OwnedPeriodicSupportIncidence, own_periodic_support_incidence};

pub use translated::{PeriodicSourceChartCopyLimits, OriginalPeriodicSourceChartCopy, OriginalPeriodicSourceChartWorldPoint, OwnedPeriodicSourceChartCopies, own_periodic_source_chart_copies};

pub use translated::{OriginalPeriodicNativeChartCorner, OriginalPeriodicNativeCornerVertex, OwnedPeriodicNativeChartCorners, own_periodic_native_chart_corners};

pub use translated::{OriginalPeriodicNativeShoreRole, OriginalPeriodicNativeShorePoint, OriginalPeriodicNativeShore, PeriodicNativeShoreLimits, OwnedPeriodicNativeChartShores, own_periodic_native_chart_shores};

pub use translated::{OriginalPeriodicShoreSampleOrigin, OriginalPeriodicShoreMaterialSample, PeriodicShoreMaterialSampleLimits, OwnedPeriodicShoreMaterialSamples, own_periodic_shore_material_samples};

pub use translated::{OriginalPeriodicShoreContactKind, OriginalPeriodicShoreContact, PeriodicShoreMaterialIntervalLimits, OwnedPeriodicShoreMaterialIntervals, own_periodic_shore_material_intervals};

pub use translated::{OriginalPeriodicChartVertexKey, OriginalPeriodicChartCycleVertex, OriginalPeriodicChartCycleEdgeKind, OriginalPeriodicChartCycleEdge, PeriodicChartCycleLimits, OwnedPeriodicChartCycles, own_periodic_chart_cycles};

pub use translated::{PeriodicChartWindingLimits, OwnedPeriodicChartWinding, own_periodic_chart_winding, construct_periodic_chart_winding};

pub use translated::{OriginalPeriodicChartCycleSides,OwnedPeriodicChartCycleSides,PeriodicChartCycleOrientation,PeriodicChartCycleSideLimits,own_periodic_chart_cycle_sides};

pub use translated::{OriginalPeriodicChartStrictPatch,OwnedPeriodicChartStrictPatches,PeriodicChartStrictPatchLimits,own_periodic_chart_strict_patches};

pub use translated::{OriginalPeriodicChartBoundaryLoop, OriginalPeriodicChartLoopContact, OwnedPeriodicChartBoundaryLoops, PeriodicChartBoundaryLoopLimits, own_periodic_chart_boundary_loops};

pub use translated::{OwnedPeriodicChartLoopWinding, own_periodic_chart_loop_winding};

pub use translated::{PeriodicChartFanPatchLimits, OwnedPeriodicChartFanPatches, own_periodic_chart_fan_patches};

pub use translated::{PeriodicSewnFaceLimits, OriginalPeriodicSewnVertex, OriginalPeriodicSewnEdge, OriginalPeriodicSewnCoedge, OriginalPeriodicSewnFace, OwnedPeriodicSewnFaces, own_periodic_sewn_faces};
