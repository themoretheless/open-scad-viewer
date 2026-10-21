//! Periodic-UV seam search, split candidates, and conservative admission.
//! Exact event proofs and UV deviation do not imply trim-region topology.
use crate::{Result, check, curve::Curve};
mod approximate_chart;
pub use approximate_chart::{
    ApproximateChartAssembly, ApproximateChartLimits, assemble_approximated_chart,
};
mod clock;
pub use clock::{CurveClockCorrespondence, verify_curve_clock_correspondence};
mod exact_chart;
pub use exact_chart::{ExactSourceChart, assemble_exact_source_chart, ClockAliasSpec, ClockMappedSourceChart, assemble_clock_mapped_source_chart};
mod exact_partition;
pub use exact_partition::{ExactPartitionIdentity, verify_exact_partition_identity};
mod anchor;
mod coherent;
pub use anchor::{AnchorLimits, AnchorTransfer, verify_anchor_winding};
pub use coherent::{
    CoherentAnchorTransfer, ProjectiveCoherentAnchorTransfer, ProjectiveCurveCorrespondence,
    ProjectivePartitionHomotopy, verify_coherent_anchor_winding,
    verify_projective_coherent_anchor_winding, verify_projective_curve_correspondence,
    verify_projective_partition_homotopy,
};
mod contact;
pub use contact::{
    ContactOwnership, ContactOwnershipReport, ContactPort, PoweredBendCoefficient,
    inspect_contact_ownership,
};
mod chart;
mod double_chart;
pub use double_chart::{ChartAssemblyLimits, assemble_periodic_chart};
mod surface_chart;
pub use surface_chart::{
    SurfaceChartAssembly, SurfaceChartLimits, SurfaceChartStatus, SurfaceUvAssembly,
    SurfaceUvLimits, assemble_on_surface,
};
mod support;
pub(crate) use support::seam as supporting_seam_identity;
pub use support::{
    SurfaceCorrespondence, SurfaceCorrespondenceLimits, SurfaceSeamProof,
    verify_surface_correspondence, verify_surface_seam_identity,
};
mod isotopy;
mod weighted;
pub use weighted::{CandidateImageAlias, compact_image_alias_candidates, ProjectiveCoherentWeightedChartAssembly, assemble_projective_coherent_weighted_chart,
    ApproximateWeightedChartAssembly, CoherentWeightedChartAssembly, WeightedChartAssembly,
    WeightedChartEdge, WeightedChartGraph, WeightedSource, assemble_approximated_weighted_chart,
    assemble_coherent_weighted_chart, assemble_weighted_periodic_chart, inspect_weighted_winding,
};
mod winding;
pub use chart::{
    ChartAssembly, ChartBoundaryGraph, ChartCalibration, ChartEdge, assemble_single_seam,
};
pub use isotopy::{
    ISOTOPY_DIRECTIONS, IsotopyCellProof, IsotopyLimits, IsotopyLoopProof, IsotopyReport,
    verify_isotopy,
};
pub use winding::{
    ImageWinding, PeriodicWindingReason, PeriodicWindingReport, inspect_periodic_winding,
};
#[derive(Clone, Debug, PartialEq)]
pub struct Piece {
    pub source_domain: [f64; 2],
    /// Original lifted UV = local UV + translation, in real arithmetic.
    pub translation: [f64; 2],
    pub tile: [i32; 2],
    pub curve: Curve,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    pub pieces: Vec<Piece>,
    /// Sufficient continuous chart containment of the materialized candidates.
    /// This is not a certificate of their agreement with the original curve.
    pub candidate_hulls_contained: bool,
}
/// Split at explicitly supplied seam parameters and translate each result by
/// its tile index. Both axes use positive finite chart periods. Use tile zero
/// for an axis that does not wrap. Cuts must be strictly increasing interior
/// parameters; tiles.len()=cuts.len()+1. At most max_pieces (1..256) pieces.
/// Input must be a lifted, nonperiodic 2D UV curve; no modulo jump guessing.
/// Each piece retains original knot parameters and rational weights. Endpoint
/// ownership is closed on both sides so callers can retain distinct UV seam
/// representatives. Periodic surface and contour topology are not inferred.
pub fn split_candidates(
    source: &Curve,
    chart: [[f64; 2]; 2],
    cuts: &[f64],
    tiles: &[[i32; 2]],
    max_pieces: usize,
) -> Result<Report> {
    source.validate()?;
    check(
        !source.periodic && source.control_points[0].len() == 2,
        "Periodic trim needs a nonperiodic lifted 2D UV curve",
    )?;
    check(
        (1..=256).contains(&max_pieces) && cuts.len() < max_pieces && tiles.len() == cuts.len() + 1,
        "Periodic trim piece budget or tile count is invalid",
    )?;
    check(
        chart
            .iter()
            .all(|r| r.iter().all(|x| x.is_finite()) && r[0] < r[1] && (r[1] - r[0]).is_finite()),
        "UV chart periods must be positive finite",
    )?;
    let [a, b] = source.domain();
    check(
        cuts.iter().all(|t| t.is_finite() && a < *t && *t < b)
            && cuts.windows(2).all(|w| w[0] < w[1]),
        "Seam cuts must be ordered distinct interior parameters",
    )?;
    let mut pieces = Vec::with_capacity(tiles.len());
    let mut contained = true;
    for (idx, tile) in tiles.iter().enumerate() {
        let range = [
            if idx == 0 { a } else { cuts[idx - 1] },
            if idx == cuts.len() { b } else { cuts[idx] },
        ];
        let translation =
            std::array::from_fn(|axis| tile[axis] as f64 * (chart[axis][1] - chart[axis][0]));
        check(
            translation.iter().all(|x| x.is_finite()),
            "UV tile translation overflows",
        )?;
        let mut curve =
            match crate::exact_curve_segments::copy_at_full_knots(source, range[0], range[1]) {
                Some(curve) => curve,
                None => source.trim(range[0], range[1])?,
            };
        for p in &mut curve.control_points {
            for axis in 0..2 {
                p[axis] -= translation[axis];
            }
        }
        curve.validate()?;
        contained &= curve
            .control_points
            .iter()
            .all(|p| (0..2).all(|axis| chart[axis][0] <= p[axis] && p[axis] <= chart[axis][1]));
        pieces.push(Piece {
            source_domain: [range[0], range[1]],
            translation,
            tile: *tile,
            curve,
        });
    }
    Ok(Report {
        pieces,
        candidate_hulls_contained: contained,
    })
}

/// Root coverage for one materialized binary64 seam coordinate.
#[derive(Clone, Debug)]
pub struct SeamCoverage {
    pub axis: usize,
    pub tile_boundary: i32,
    pub coordinate: f64,
    pub roots: crate::curve_plane::Report,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchStatus {
    Complete,
    Unresolved,
    WorkLimit,
}
#[derive(Clone, Debug)]
pub struct SeamSearch {
    pub status: SearchStatus,
    pub seams: Vec<SeamCoverage>,
    pub work: usize,
    /// Required levels counted so far; a lower bound if enumeration exceeds max_seams.
    pub total_seams: usize,
}
/// Automatically enumerate seam levels covering the original positive-weight
/// UV control hull and inspect all of them using original-curve continuous
/// root coverage. Root intervals are NOT collapsed into guessed split points.
/// Coincident intervals remain in the report and prevent isolated-root success.
/// max_seams is 1..4096, max_work 1..100000 shared across all inspected seams.
/// Seams are the explicitly returned binary64 coordinates, not a certificate
/// for exact real-arithmetic chart-origin + integer*period construction.
pub fn inspect_seams(
    source: &Curve,
    chart: [[f64; 2]; 2],
    parameter_tolerance: f64,
    max_seams: usize,
    max_work: usize,
) -> Result<SeamSearch> {
    use crate::distance_bounds::Interval as I;
    source.validate()?;
    check(
        !source.periodic && source.control_points[0].len() == 2,
        "Seam search requires a nonperiodic lifted UV curve",
    )?;
    check(
        chart
            .iter()
            .all(|r| r.iter().all(|x| x.is_finite()) && r[0] < r[1] && (r[1] - r[0]).is_finite())
            && parameter_tolerance.is_finite()
            && parameter_tolerance > 0.
            && (1..=4096).contains(&max_seams)
            && (1..=100000).contains(&max_work),
        "Invalid seam search chart, tolerance or work limits",
    )?;
    let mut levels = Vec::new();
    for axis in 0..2 {
        let (lo, hi) = source
            .control_points
            .iter()
            .map(|p| p[axis])
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
                (lo.min(v), hi.max(v))
            });
        let period = chart[axis][1] - chart[axis][0];
        let q = I::new(lo, hi)?
            .sub(I::point(chart[axis][0]))?
            .div(I::point(period))?;
        // Extra exterior levels protect enumeration against seam materialization
        // rounding. Strict ordering below rejects unrepresentable lattices.
        let first = q.lo.floor() - 1.;
        let last = q.hi.ceil() + 1.;
        check(
            first >= i32::MIN as f64 && last <= i32::MAX as f64,
            "Seam tile index exceeds i32 range",
        )?;
        if last - first + 1. > max_seams as f64
            || levels.len() + (last - first + 1.) as usize > max_seams
        {
            return Ok(SeamSearch {
                status: SearchStatus::WorkLimit,
                seams: Vec::new(),
                work: 0,
                total_seams: levels.len() + (last - first + 1.) as usize,
            });
        }
        let mut previous = f64::NEG_INFINITY;
        for k in first as i32..=last as i32 {
            let coordinate = chart[axis][0] + k as f64 * period;
            check(
                coordinate.is_finite() && coordinate > previous,
                "Seam lattice collapses at coordinate precision",
            )?;
            previous = coordinate;
            levels.push((axis, k, coordinate));
        }
    }
    let mut spatial = source.clone();
    for p in &mut spatial.control_points {
        p.push(0.);
    }
    let spans = (source.degree..source.control_points.len())
        .filter(|&i| source.knots[i] < source.knots[i + 1])
        .count();
    let mut report = SeamSearch {
        status: SearchStatus::Complete,
        seams: Vec::with_capacity(levels.len()),
        work: 0,
        total_seams: levels.len(),
    };
    for (axis, k, coordinate) in levels {
        let remaining = max_work - report.work;
        if remaining < spans {
            report.status = SearchStatus::WorkLimit;
            break;
        }
        let mut normal = [0.; 3];
        normal[axis] = 1.;
        let roots = crate::curve_plane::inspect(
            &spatial,
            normal,
            coordinate,
            parameter_tolerance,
            remaining,
        )?;
        report.work += roots.work;
        if !roots.all_roots_isolated {
            report.status = SearchStatus::Unresolved;
        }
        report.seams.push(SeamCoverage {
            axis,
            tile_boundary: k,
            coordinate,
            roots,
        });
    }
    Ok(report)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionStatus {
    WithinTolerance,
    Mismatch,
    Unresolved,
}
#[derive(Clone, Debug)]
pub struct Admission {
    pub status: AdmissionStatus,
    pub work: usize,
    /// Outward bound for the complete partition, infinity until fully covered.
    pub error_upper: f64,
}
/// Verify the original lifted curve against every materialized local piece
/// plus its stored translation. Uses original span interval bounds, never a
/// trimmed copy of the source as reference. Includes translation rounding via
/// outward interval arithmetic. This certifies UV deviation only, not topology
/// or world-space deviation on a supporting surface. tolerance is in UV units.
/// All pieces must form an exact ordered partition of the source active domain.
pub fn verify_partition(
    source: &Curve,
    partition: &Report,
    tolerance: f64,
    max_work: usize,
) -> Result<Admission> {
    use crate::distance_bounds::{Interval as I, box_distance};
    source.validate()?;
    check(
        source.control_points[0].len() == 2
            && !source.periodic
            && tolerance.is_finite()
            && tolerance > 0.
            && (1..=100000).contains(&max_work)
            && !partition.pieces.is_empty()
            && partition.pieces.len() <= 256,
        "Invalid UV partition verification input or limits",
    )?;
    let mut pending = Vec::new();
    let mut end = source.domain()[0];
    for (index, piece) in partition.pieces.iter().enumerate() {
        piece.curve.validate()?;
        check(
            piece.curve.control_points[0].len() == 2
                && !piece.curve.periodic
                && piece.source_domain == piece.curve.domain()
                && piece.source_domain[0] == end
                && piece.source_domain[0] < piece.source_domain[1]
                && piece.translation.iter().all(|x| x.is_finite()),
            "UV pieces must form an ordered original-parameter partition",
        )?;
        end = piece.source_domain[1];
        let mut breaks: Vec<_> = source
            .knots
            .iter()
            .chain(&piece.curve.knots)
            .copied()
            .filter(|t| piece.source_domain[0] <= *t && *t <= end)
            .collect();
        breaks.extend(piece.source_domain);
        breaks.sort_by(f64::total_cmp);
        breaks.dedup();
        for range in breaks.windows(2) {
            let a = (source.degree..source.control_points.len())
                .find(|&i| source.knots[i] <= range[0] && range[0] < source.knots[i + 1])
                .ok_or_else(|| crate::input("Partition lies outside original curve"))?;
            let b = (piece.curve.degree..piece.curve.control_points.len())
                .find(|&i| piece.curve.knots[i] <= range[0] && range[0] < piece.curve.knots[i + 1])
                .unwrap();
            pending.push((index, a, b, [range[0], range[1]]));
        }
    }
    check(
        end == source.domain()[1],
        "UV partition must cover the complete original domain",
    )?;
    let mut work = 0;
    let mut error_upper = 0f64;
    while let Some((index, a, b, range)) = pending.pop() {
        if work == max_work {
            return Ok(Admission {
                status: AdmissionStatus::Unresolved,
                work,
                error_upper: f64::INFINITY,
            });
        }
        work += 1;
        if let Ok(upper)=crate::curve_deviation::translated_cell_upper(
            source,a,&partition.pieces[index].curve,b,range,&partition.pieces[index].translation) {
            if upper<=tolerance {error_upper=error_upper.max(upper);continue;}
        }
        let t = I::new(range[0], range[1])?;
        let original = crate::curve_distance::enclosure(source, a, t)?;
        let piece = &partition.pieces[index];
        let translated: Vec<_> = crate::curve_distance::enclosure(&piece.curve, b, t)?
            .into_iter()
            .zip(piece.translation)
            .map(|(coordinate, shift)| coordinate.add(I::point(shift)))
            .collect::<Result<_>>()?;
        let (lower, upper) = box_distance(&original, &translated)?;
        if lower > tolerance {
            return Ok(Admission {
                status: AdmissionStatus::Mismatch,
                work,
                error_upper: f64::INFINITY,
            });
        }
        if upper <= tolerance {
            error_upper = error_upper.max(upper);
            continue;
        }
        let middle = range[0] * 0.5 + range[1] * 0.5;
        if middle <= range[0] || middle >= range[1] {
            return Ok(Admission {
                status: AdmissionStatus::Unresolved,
                work,
                error_upper: f64::INFINITY,
            });
        }
        pending.push((index, a, b, [range[0], middle]));
        pending.push((index, a, b, [middle, range[1]]));
    }
    Ok(Admission {
        status: AdmissionStatus::WithinTolerance,
        work,
        error_upper,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitStatus {
    VerifiedUvPartition,
    SearchIncomplete,
    IntervalRootNeedsConstruction,
    LatticeRoundingUnresolved,
    PieceLimit,
    TileContainmentUnresolved,
    DeviationUnresolved,
    ConnectivityUnresolved,
    EventOrderUnresolved,
}
/// Exact parameter proof for an event previously represented by an interval.
#[derive(Clone, Debug)]
pub struct ExactSeamEvent {
    pub axis: usize,
    pub coordinate: f64,
    pub original_bounds: [f64; 2],
    pub parameter: f64,
}
/// Proposed seam parameter inside a proved root interval. Not an exact root.
#[derive(Clone, Debug)]
pub struct ApproximatedEvent {
    pub axis: usize,
    pub coordinate: f64,
    pub parameter_bounds: [f64; 2],
    pub parameter: f64,
}
#[derive(Clone, Debug)]
pub struct SplitReport {
    pub status: SplitStatus,
    pub search: SeamSearch,
    pub resolved_events: Vec<ExactSeamEvent>,
    pub approximated_events: Vec<ApproximatedEvent>,
    pub parameter_attempts: usize,
    pub admission: Option<Admission>,
    pub connectivity: Option<Connectivity>,
    /// Published only after event completeness, chart hulls and UV admission.
    /// Does not certify topology of a closed trim region on a surface.
    pub partition: Option<Report>,
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkStatus {
    Exact,
    Gap,
    Unresolved,
}
#[derive(Clone, Debug)]
pub struct Link {
    pub pieces: [usize; 2],
    pub closure: bool,
    pub status: LinkStatus,
}
#[derive(Clone, Debug)]
pub struct Connectivity {
    pub status: LinkStatus,
    pub links: Vec<Link>,
    pub expected_links: usize,
    /// Some requests closure with this lifted winding; None is an open chain.
    pub winding: Option<[i32; 2]>,
}


#[derive(Clone, Debug)]
pub struct PartitionIdentity {
    pub exact: bool,
    pub compared_segments: usize,
    pub reason: Option<&'static str>,
}
/// Sufficient exact parameterwise identity of a lifted partition, using exact
/// original knot insertion and exact candidate refinement/translation. Refusal
/// is inconclusive: different encodings can still represent the same curve.
/// This proves source/candidate path identity, not per-chart region clipping.
pub fn verify_identity(
    source: &Curve,
    partition: &Report,
    max_segments: usize,
) -> Result<PartitionIdentity> {
    source.validate()?;
    check(
        !source.periodic
            && source.control_points[0].len() == 2
            && !partition.pieces.is_empty()
            && partition.pieces.len() <= 256
            && (1..=4096).contains(&max_segments),
        "Invalid exact partition input or segment budget",
    )?;
    let mut end = source.domain()[0];
    let mut cuts = Vec::new();
    for (i, piece) in partition.pieces.iter().enumerate() {
        piece.curve.validate()?;
        check(
            !piece.curve.periodic
                && piece.curve.control_points[0].len() == 2
                && piece.source_domain == piece.curve.domain()
                && piece.source_domain[0] == end
                && piece.translation.iter().all(|x| x.is_finite()),
            "Exact partition must be ordered and cover original parameters",
        )?;
        end = piece.source_domain[1];
        if i + 1 < partition.pieces.len() {
            cuts.push(end);
        }
    }
    check(
        end == source.domain()[1],
        "Exact partition must cover complete original domain",
    )?;
    let mut out = PartitionIdentity {
        exact: false,
        compared_segments: 0,
        reason: None,
    };
    let Some(original) = crate::exact_curve_segments::inspect_with_cuts(source, &cuts) else {
        out.reason = Some("original-refinement-unproven");
        return Ok(out);
    };
    if original.len() > max_segments {
        out.reason = Some("segment-budget");
        return Ok(out);
    }
    for piece in &partition.pieces {
        let Some(candidates) = crate::exact_curve_segments::inspect(&piece.curve) else {
            out.reason = Some("candidate-refinement-unproven");
            return Ok(out);
        };
        for mut candidate in candidates {
            if out.compared_segments >= original.len() {
                out.reason = Some("segment-layout-unproven");
                return Ok(out);
            }
            for p in &mut candidate.control_points {
                for axis in 0..2 {
                    let Some(value) =
                        crate::exact_curve_segments::exact_add(p[axis], piece.translation[axis])
                    else {
                        out.reason = Some("translation-rounding-unproven");
                        return Ok(out);
                    };
                    p[axis] = value;
                }
            }
            if candidate != original[out.compared_segments] {
                out.reason = Some("segment-identity-unproven");
                return Ok(out);
            }
            out.compared_segments += 1;
        }
    }
    if out.compared_segments != original.len() {
        out.reason = Some("incomplete-segment-coverage");
        return Ok(out);
    }
    out.exact = true;
    Ok(out)
}



#[derive(Clone, Debug)]
pub struct RegionLoopProof {
    pub source_closed: Option<bool>,
    pub connectivity: Option<Connectivity>,
    pub identity: Option<PartitionIdentity>,
}
#[derive(Clone, Debug)]
pub struct RegionTransfer {
    /// Exact equality of summed winding functions in the lifted UV plane,
    /// off their common boundary. No simplicity or nesting is inferred.
    pub lifted_winding_preserved: bool,
    /// Equality of periodic winding sums, conditional on the authored UV periods.
    pub periodic_winding_preserved: bool,
    pub loops: Vec<RegionLoopProof>,
    pub compared_segments: usize,
    pub links: usize,
    pub reason: Option<&'static str>,
}
/// Sufficient exact transfer of nonzero-winding region semantics, including
/// oriented holes, in lifted UV. Every source loop must be exactly closed,
/// every partition exactly connected/closed with zero lifted winding, and each
/// reconstructed path exactly identical. Then all off-boundary winding sums
/// agree, without sampling or guessing hole roles. This does not assemble
/// per-chart face loops or prove correspondence to a periodic 3D surface.
/// Up to 64 source loops, 256 total pieces, shared segment/link allowances.
pub fn verify_lifted_region(
    sources: &[Curve],
    partitions: &[Report],
    chart: [[f64; 2]; 2],
    max_segments: usize,
    max_links: usize,
) -> Result<RegionTransfer> {
    check(
        !sources.is_empty()
            && sources.len() <= 64
            && sources.len() == partitions.len()
            && partitions.iter().map(|p| p.pieces.len()).sum::<usize>() <= 256
            && (1..=4096).contains(&max_segments)
            && (1..=256).contains(&max_links)
            && chart
                .iter()
                .all(|r| r.iter().all(|x| x.is_finite()) && r[0] < r[1]),
        "Invalid lifted region loops or shared budgets",
    )?;
    let mut out = RegionTransfer {
        lifted_winding_preserved: false,
        periodic_winding_preserved: false,
        loops: Vec::new(),
        compared_segments: 0,
        links: 0,
        reason: None,
    };
    for (source, partition) in sources.iter().zip(partitions) {
        source.validate()?;
        check(
            !source.periodic && source.control_points[0].len() == 2,
            "Lifted region requires nonperiodic 2D authored loops",
        )?;
        let closure = crate::trim_domain::exact_loop_joins(std::slice::from_ref(source))?;
        let mut proof = RegionLoopProof {
            source_closed: closure,
            connectivity: None,
            identity: None,
        };
        if closure != Some(true) {
            out.loops.push(proof);
            out.reason = Some("source-closure-unproven");
            return Ok(out);
        }
        if out.links == max_links {
            out.loops.push(proof);
            out.reason = Some("link-budget");
            return Ok(out);
        }
        let connectivity =
            inspect_connectivity(partition, chart, Some([0, 0]), max_links - out.links)?;
        let connected = connectivity.status == LinkStatus::Exact;
        out.links += connectivity.links.len();
        proof.connectivity = Some(connectivity);
        if !connected {
            out.loops.push(proof);
            out.reason = Some("candidate-closure-unproven");
            return Ok(out);
        }
        // Connectivity has already proved exact chart-period subtraction.
        let periods = [chart[0][1] - chart[0][0], chart[1][1] - chart[1][0]];
        let periodic_translations = partition.pieces.iter().all(|piece| {
            (0..2).all(|axis| {
                crate::exact_products::sum_sign(&[
                    (piece.translation[axis], 1.),
                    (-f64::from(piece.tile[axis]), periods[axis]),
                ]) == std::cmp::Ordering::Equal
            })
        });
        if !periodic_translations {
            out.loops.push(proof);
            out.reason = Some("periodic-translation-unproven");
            return Ok(out);
        }
        let contained =
            partition.pieces.iter().all(|piece| {
                piece.curve.control_points.iter().all(|p| {
                    (0..2).all(|axis| chart[axis][0] <= p[axis] && p[axis] <= chart[axis][1])
                })
            });
        if !contained {
            out.loops.push(proof);
            out.reason = Some("candidate-chart-unproven");
            return Ok(out);
        }
        if out.compared_segments == max_segments {
            out.loops.push(proof);
            out.reason = Some("segment-budget");
            return Ok(out);
        }
        let identity = verify_identity(source, partition, max_segments - out.compared_segments)?;
        let identical = identity.exact;
        out.compared_segments += identity.compared_segments;
        proof.identity = Some(identity);
        out.loops.push(proof);
        if !identical {
            out.reason = Some("path-identity-unproven");
            return Ok(out);
        }
    }
    out.lifted_winding_preserved = true;
    out.periodic_winding_preserved = true;
    Ok(out)
}

#[path = "periodic_trim/seam_partition.rs"]
mod seam_partition;
pub use seam_partition::split_verified;
pub use seam_partition::split_approximated;

#[path = "periodic_trim/seam_connectivity.rs"]
mod seam_connectivity;
pub use seam_connectivity::inspect_connectivity;
