//! Lossless integer-weighted chart chains, distinct from owned face cycles.
use super::{
    ChartAssemblyLimits, ChartBoundaryGraph, ChartCalibration, PeriodicWindingReason,
    PeriodicWindingReport, RegionTransfer, Report, double_chart, inspect_periodic_winding,
    verify_lifted_region,
};
use crate::{Result, check, curve::Curve, exact_products::sum_sign, resource};
#[derive(Clone, Debug, PartialEq)]
pub struct WeightedSource {
    pub expanded_edge: usize,
    pub source: Option<[usize; 2]>,
    /// Sign relative to the materialized representative's orientation.
    pub orientation: i8,
}
#[derive(Clone, Debug, PartialEq)]
pub struct WeightedChartEdge {
    pub curve: Curve,
    pub coefficient: usize,
    pub contributions: Vec<WeightedSource>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct WeightedChartGraph {
    pub edges: Vec<WeightedChartEdge>,
    /// Closed oriented chains; each dictionary index occurs coefficient times.
    pub loops: Vec<Vec<usize>>,
    /// Exact opposite encoding groups removed with zero integer coefficient.
    pub cancellations: Vec<Vec<WeightedSource>>,
    /// IDs in original expanded construction, traceable through contributions.
    pub connector_pairs: Vec<[usize; 2]>,
    pub original_edges: usize,
    pub expanded_edges: usize,
}
#[derive(Clone, Debug)]
pub struct WeightedChartAssembly {
    pub source_region: RegionTransfer,
    pub winding_preserved: bool,
    pub graph: Option<WeightedChartGraph>,
    pub calibration: Option<ChartCalibration>,
    pub reason: Option<&'static str>,
}
fn reverse_encoding(a: &Curve, b: &Curve) -> bool {
    super::coherent::normalized_function(a, b, true)
}

/// A proposed oriented image relation, NOT a geometric certificate. Indices
/// refer to the actual input expanded graph; callers must prove image equality
/// and traversal independently before treating the result as source-owned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CandidateImageAlias {
    pub edges: [usize; 2],
    pub orientation: i8,
}
/// Recompose integer flow under caller-proposed image aliases. Like
/// split_candidates this publishes only candidate geometry/provenance; it does
/// not authorize source winding, interior contacts, regions or physical faces.
/// This boundary lets B-rep reuse native compaction after its own exact image
/// proofs without introducing a NURBS-kernel dependency on B-rep certificates.
pub fn compact_image_alias_candidates(
    graph: &ChartBoundaryGraph,
    aliases: &[CandidateImageAlias],
) -> Result<Option<WeightedChartGraph>> {
    check(
        graph.edges.len() <= 4096 && aliases.len() <= 256,
        "Image alias candidate size exceeded",
    )?;
    for edge in &graph.edges {
        edge.curve.validate()?;
        check(
            edge.curve.control_points[0].len() == 2,
            "Image alias graph must be planar",
        )?;
    }
    let mut used = graph.loops.iter().flatten().copied().collect::<Vec<_>>();
    used.sort_unstable();
    check(
        used == (0..graph.edges.len()).collect::<Vec<_>>()
            && graph
                .connector_pairs
                .iter()
                .all(|p| p[0] != p[1] && p.iter().all(|&i| i < graph.edges.len())),
        "Image alias graph omits, repeats or misreferences an expanded edge",
    )?;
    for a in aliases {
        check(
            a.edges[0] != a.edges[1]
                && a.edges.iter().all(|&i| i < graph.edges.len())
                && matches!(a.orientation, -1 | 1),
            "Invalid candidate image alias",
        )?;
    }
    Ok(compact_with(graph.clone(), |group, edge| {
        aliases.iter().find_map(|alias| {
            let a = &graph.edges[alias.edges[0]].curve;
            let b = &graph.edges[alias.edges[1]].curve;
            ((&group.curve == a && &edge.curve == b) || (&group.curve == b && &edge.curve == a))
                .then_some(alias.orientation)
        })
    }))
}

pub(super) struct Group {
    pub(super) curve: Curve,
    reverse: Option<Curve>,
    net: i32,
    contributions: Vec<WeightedSource>,
}
pub(super) fn compact(graph: ChartBoundaryGraph) -> Option<WeightedChartGraph> {
    compact_with(graph, |_, _| None)
}
pub(super) fn compact_with(
    graph: ChartBoundaryGraph,
    image_alias: impl Fn(&Group, &super::ChartEdge) -> Option<i8>,
) -> Option<WeightedChartGraph> {
    let original_edges = graph.edges.len();
    let mut groups: Vec<Group> = Vec::new();
    for (index, edge) in graph.edges.into_iter().enumerate() {
        let found = groups.iter().enumerate().find_map(|(i, g)| {
            if super::coherent::normalized_function(&g.curve, &edge.curve, false) {
                Some((i, 1i8))
            } else if reverse_encoding(&g.curve, &edge.curve) {
                Some((i, -1))
            } else {
                image_alias(g, &edge).map(|sign| (i, sign))
            }
        });
        let contribution = |sign| WeightedSource {
            expanded_edge: index,
            source: edge.source,
            orientation: sign,
        };
        if let Some((i, sign)) = found {
            let g = &mut groups[i];
            g.net += i32::from(sign);
            g.contributions.push(contribution(sign));
            if sign < 0 && g.reverse.is_none() {
                g.reverse = Some(edge.curve);
            }
        } else {
            groups.push(Group {
                curve: edge.curve.clone(),
                reverse: None,
                net: 1,
                contributions: vec![contribution(1)],
            });
        }
    }
    let mut edges = Vec::new();
    let mut cancellations = Vec::new();
    for mut g in groups {
        if g.net == 0 {
            cancellations.push(g.contributions);
            continue;
        }
        let curve = if g.net > 0 {
            g.curve
        } else {
            for p in &mut g.contributions {
                p.orientation = -p.orientation;
            }
            g.reverse?
        };
        edges.push(WeightedChartEdge {
            curve,
            coefficient: g.net.unsigned_abs() as usize,
            contributions: g.contributions,
        });
    }
    let uses: Vec<_> = edges
        .iter()
        .enumerate()
        .flat_map(|(i, e)| std::iter::repeat_n(i, e.coefficient))
        .collect();
    // Recompose the remaining balanced flow after exact cancellation. No
    // reverse knot arithmetic or angular/regular-face ownership is inferred.
    let expanded: Vec<_> = uses
        .iter()
        .map(|&i| super::ChartEdge {
            curve: edges[i].curve.clone(),
            source: None,
        })
        .collect();
    let loops = if expanded.is_empty() {
        Vec::new()
    } else {
        double_chart::cycles(&expanded, false)?
            .into_iter()
            .map(|c| c.into_iter().map(|i| uses[i]).collect())
            .collect()
    };
    Some(WeightedChartGraph {
        edges,
        loops,
        cancellations,
        connector_pairs: graph.connector_pairs,
        original_edges,
        expanded_edges: uses.len(),
    })
}
/// Exact source winding transfer with arbitrary bounded integer boundary
/// multiplicity. Equal encodings merge; exact reversed encodings cancel.
/// Geometry is copied from original materialized edges, never refitted or
/// reconstructed through rounded knot reflection. Does not assert owned,
/// regular faces or a regularization of arbitrary intersecting boundaries.
pub fn assemble_weighted_periodic_chart(
    sources: &[Curve],
    partitions: &[Report],
    chart: [[f64; 2]; 2],
    anchor: [f64; 2],
    limits: ChartAssemblyLimits,
) -> Result<WeightedChartAssembly> {
    double_chart::validate_limits(chart, anchor, limits)?;
    let region = verify_lifted_region(
        sources,
        partitions,
        chart,
        limits.max_segments,
        limits.max_links,
    )?;
    let mut out = WeightedChartAssembly {
        source_region: region,
        winding_preserved: false,
        graph: None,
        calibration: None,
        reason: None,
    };
    if !out.source_region.periodic_winding_preserved {
        out.reason = Some("input-region-unproven");
        return Ok(out);
    }
    let assembled =
        double_chart::assemble_candidate(partitions, chart, anchor, limits, false, || {
            let loops: Vec<_> = sources.iter().map(|c| vec![c.clone()]).collect();
            inspect_periodic_winding(
                &loops,
                chart,
                anchor,
                limits.tolerance_uv,
                limits.max_images,
                limits.max_cells,
            )
        })?;
    out.calibration = assembled.calibration;
    out.reason = assembled.reason;
    if !assembled.chart_winding_preserved {
        return Ok(out);
    }
    out.graph = assembled.graph.and_then(compact);
    if out.graph.is_none() {
        out.reason = Some("weighted-flow-unproven");
        return Ok(out);
    }
    out.winding_preserved = true;
    Ok(out)
}
/// Classify the compact chain with its exact integer multiplicity. Validates
/// all dictionary uses and closed chains before forwarding to the continuous
/// winding classifier. At most 256 expanded curves in this query API.
pub fn inspect_weighted_winding(
    graph: &WeightedChartGraph,
    chart: [[f64; 2]; 2],
    point: [f64; 2],
    tolerance_uv: f64,
    max_images: usize,
    max_cells: usize,
) -> Result<PeriodicWindingReport> {
    check(
        graph.edges.len() <= 512
            && graph.loops.len() <= 512
            && graph.loops.iter().all(|l| !l.is_empty() && l.len() <= 512)
            && graph.loops.iter().map(Vec::len).sum::<usize>() <= 512
            && graph
                .edges
                .iter()
                .all(|e| (1..=512).contains(&e.coefficient))
            && graph.loops.iter().flatten().all(|i| *i < graph.edges.len())
            && chart
                .iter()
                .all(|r| r.iter().all(|x| x.is_finite()) && r[0] < r[1])
            && point.iter().all(|x| x.is_finite())
            && tolerance_uv.is_finite()
            && tolerance_uv > 0.
            && (1..=4096).contains(&max_images)
            && (1..=100000).contains(&max_cells),
        "Invalid weighted winding graph, query or budgets",
    )?;
    let mut counts = vec![0usize; graph.edges.len()];
    for &i in graph.loops.iter().flatten() {
        counts[i] += 1;
    }
    check(
        counts
            .iter()
            .zip(&graph.edges)
            .all(|(n, e)| *n == e.coefficient),
        "Weighted cycles must use each edge with its integer coefficient",
    )?;
    if graph.edges.is_empty() {
        return Ok(PeriodicWindingReport {
            winding: Some(0),
            reason: PeriodicWindingReason::Complete,
            cells: 0,
            required_images: Some(0),
            images: Vec::new(),
        });
    }
    if counts.iter().sum::<usize>() > 256 {
        return Err(resource("Weighted winding expansion exceeds 256 curves"));
    }
    let loops: Vec<Vec<Curve>> = graph
        .loops
        .iter()
        .map(|cycle| {
            cycle
                .iter()
                .map(|&i| graph.edges[i].curve.clone())
                .collect()
        })
        .collect();
    inspect_periodic_winding(&loops, chart, point, tolerance_uv, max_images, max_cells)
}

/// Approximate source-to-candidate topology and original anchor transfer are
/// retained separately from the candidate's exact weighted graph winding.
#[derive(Clone, Debug)]
pub struct ApproximateWeightedChartAssembly {
    pub transfer: super::AnchorTransfer,
    pub candidate_winding_preserved: bool,
    pub graph: Option<WeightedChartGraph>,
    pub calibration: Option<ChartCalibration>,
    pub reason: Option<&'static str>,
}
/// Weighted candidate assembly after the same continuous source/candidate
/// isotopy, periodic image separation and anchor transfer as the unweighted
/// approximate path. Balanced flow decomposition is not a regular-face claim.
/// Coincident source loops still require an independently admissible isotopy;
/// incomplete topology never publishes a graph.
pub fn assemble_approximated_weighted_chart(
    sources: &[Curve],
    partitions: &[Report],
    chart: [[f64; 2]; 2],
    anchor: [f64; 2],
    limits: super::ApproximateChartLimits,
) -> Result<ApproximateWeightedChartAssembly> {
    let result = super::approximate_chart::assemble_weighted_candidate(
        sources, partitions, chart, anchor, limits,
    )?;
    let graph = if result.candidate_winding_preserved {
        result.graph.and_then(compact)
    } else {
        None
    };
    let admitted = result.candidate_winding_preserved && graph.is_some();
    Ok(ApproximateWeightedChartAssembly {
        transfer: result.transfer,
        candidate_winding_preserved: admitted,
        graph,
        calibration: result.calibration,
        reason: if result.candidate_winding_preserved && !admitted {
            Some("weighted-flow-unproven")
        } else {
            result.reason
        },
    })
}

#[derive(Clone, Debug)]
pub struct CoherentWeightedChartAssembly {
    pub transfer: super::CoherentAnchorTransfer,
    pub candidate_winding_preserved: bool,
    pub graph: Option<WeightedChartGraph>,
    pub calibration: Option<ChartCalibration>,
    pub reason: Option<&'static str>,
}
/// Original copy-coherent weighted topology; no full-copy embedding is claimed.
/// Representative, original-source and raw-candidate classification share one
/// allowance. Geometry is retained from actual candidate dictionary members.
fn assemble_coherent_weighted_chart_impl(
    sources: &[Curve],
    partitions: &[Report],
    chart: [[f64; 2]; 2],
    anchor: [f64; 2],
    limits: super::ApproximateChartLimits,
    projective: Option<&super::ProjectiveCoherentAnchorTransfer>,
) -> Result<CoherentWeightedChartAssembly> {
    let mut common = ChartAssemblyLimits {
        max_segments: 1,
        max_links: 1,
        max_edges: limits.max_edges,
        max_images: limits.max_images,
        max_cells: limits.max_cells,
        tolerance_uv: limits.tolerance_uv,
    };
    double_chart::validate_limits(chart, anchor, common)?;
    let transfer = if let Some(p) = projective {
        p.transfer.clone()
    } else {
        super::verify_coherent_anchor_winding(
            sources,
            partitions,
            chart,
            anchor,
            super::AnchorLimits {
                isotopy: limits.isotopy,
                tolerance_uv: limits.tolerance_uv,
                max_images: limits.max_images,
                max_cells: limits.max_cells,
                max_checks: limits.max_checks,
            },
        )?
    };
    let mut out = CoherentWeightedChartAssembly {
        transfer,
        candidate_winding_preserved: false,
        graph: None,
        calibration: None,
        reason: None,
    };
    if !out.transfer.coherent_source_winding_preserved {
        out.reason = out.transfer.reason;
        return Ok(out);
    }
    let periods = chart.map(|r| crate::exact_curve_segments::exact_add(r[1], -r[0]));
    let [Some(u), Some(v)] = periods else {
        out.reason = Some("periodic-translation-unproven");
        return Ok(out);
    };
    for piece in partitions.iter().flat_map(|p| &p.pieces) {
        if (0..2).any(|a| {
            sum_sign(&[
                (piece.translation[a], 1.),
                (-f64::from(piece.tile[a]), [u, v][a]),
            ]) != std::cmp::Ordering::Equal
        }) {
            out.reason = Some("periodic-translation-unproven");
            return Ok(out);
        }
        if piece
            .curve
            .control_points
            .iter()
            .any(|p| (0..2).any(|a| p[a] < chart[a][0] || p[a] > chart[a][1]))
        {
            out.reason = Some("candidate-chart-unproven");
            return Ok(out);
        }
    }
    let Some(query) = out.transfer.original_source.clone() else {
        out.reason = Some("original-anchor-unproven");
        return Ok(out);
    };
    let extra = out
        .transfer
        .classification_cells
        .saturating_sub(query.cells);
    let Some(remaining) = limits.max_cells.checked_sub(extra).filter(|n| *n > 0) else {
        out.reason = Some("classification-budget");
        return Ok(out);
    };
    common.max_cells = remaining;
    let assembled =
        double_chart::assemble_candidate(partitions, chart, anchor, common, false, || Ok(query))?;
    out.calibration = assembled.calibration;
    out.reason = assembled.reason;
    if assembled.chart_winding_preserved {
        out.graph = assembled.graph.and_then(|graph| {
            compact_with(graph, |g, edge| {
                let p = projective?;
                let [ga, gp] = g.contributions.first()?.source?;
                let [ea, ep] = edge.source?;
                if g.curve != partitions.get(ga)?.pieces.get(gp)?.curve
                    || edge.curve != partitions.get(ea)?.pieces.get(ep)?.curve
                {
                    return None;
                }
                for receipt in p.correspondences() {
                    for (a, ap, b, bp) in [(ga, gp, ea, ep), (ea, ep, ga, gp)] {
                        let reverse = receipt.orientation() < 0;
                        if receipt.matches(
                            &sources[a],
                            &sources[b],
                            &partitions[a],
                            &partitions[b],
                            reverse,
                        ) && bp
                            == if reverse {
                                partitions[b].pieces.len() - 1 - ap
                            } else {
                                ap
                            }
                        {
                            return Some(receipt.orientation());
                        }
                    }
                }
                None
            })
        });
        out.candidate_winding_preserved = out.graph.is_some();
        if out.graph.is_none() {
            out.reason = Some("weighted-flow-unproven");
        }
    }
    Ok(out)
}

/// Original exact normalized-function weighted assembly.
pub fn assemble_coherent_weighted_chart(
    sources: &[Curve],
    partitions: &[Report],
    chart: [[f64; 2]; 2],
    anchor: [f64; 2],
    limits: super::ApproximateChartLimits,
) -> Result<CoherentWeightedChartAssembly> {
    assemble_coherent_weighted_chart_impl(sources, partitions, chart, anchor, limits, None)
}
#[derive(Clone, Debug)]
pub struct ProjectiveCoherentWeightedChartAssembly {
    pub assembly: CoherentWeightedChartAssembly,
    pub correspondence: super::ProjectiveCoherentAnchorTransfer,
}
/// Assemble a weighted chart with separately replayed nonlinear alias proofs.
/// Every compacted nonlinear pair binds actual source indices, original paired
/// seam pieces and retained raw graph curves. Connector edges use the existing
/// normalized-function proof. Integer multiplicity and full-source calibration
/// are retained; this is not yet owned-surface-face admission.
pub fn assemble_projective_coherent_weighted_chart(
    sources: &[Curve],
    partitions: &[Report],
    correspondences: &[super::ProjectivePartitionHomotopy],
    chart: [[f64; 2]; 2],
    anchor: [f64; 2],
    limits: super::ApproximateChartLimits,
    max_correspondence_work: u64,
) -> Result<ProjectiveCoherentWeightedChartAssembly> {
    let correspondence = super::verify_projective_coherent_anchor_winding(
        sources,
        partitions,
        correspondences,
        chart,
        anchor,
        super::AnchorLimits {
            isotopy: limits.isotopy,
            tolerance_uv: limits.tolerance_uv,
            max_images: limits.max_images,
            max_cells: limits.max_cells,
            max_checks: limits.max_checks,
        },
        max_correspondence_work,
    )?;
    let assembly = assemble_coherent_weighted_chart_impl(
        sources,
        partitions,
        chart,
        anchor,
        limits,
        Some(&correspondence),
    )?;
    Ok(ProjectiveCoherentWeightedChartAssembly {
        assembly,
        correspondence,
    })
}
