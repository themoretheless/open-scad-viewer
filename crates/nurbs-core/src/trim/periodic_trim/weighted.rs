//! Lossless integer-weighted chart chains, distinct from owned face cycles.
use super::{
    ChartAssemblyLimits, ChartBoundaryGraph, ChartCalibration, PeriodicWindingReason,
    PeriodicWindingReport, RegionTransfer, Report, double_chart, inspect_periodic_winding,
    verify_lifted_region,
};
use crate::{Result, check, curve::Curve, exact_products::sum_sign, resource};
#[derive(Clone, Debug)]
pub struct WeightedSource {
    pub expanded_edge: usize,
    pub source: Option<[usize; 2]>,
    /// Sign relative to the materialized representative's orientation.
    pub orientation: i8,
}
#[derive(Clone, Debug)]
pub struct WeightedChartEdge {
    pub curve: Curve,
    pub coefficient: usize,
    pub contributions: Vec<WeightedSource>,
}
#[derive(Clone, Debug)]
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
    if a.degree != b.degree
        || a.periodic
        || b.periodic
        || a.domain() != b.domain()
        || a.knots.len() != b.knots.len()
        || a.weights.iter().rev().ne(b.weights.iter())
        || a.control_points.iter().rev().ne(b.control_points.iter())
    {
        return false;
    }
    let [start, end] = a.domain();
    a.knots.iter().zip(b.knots.iter().rev()).all(|(&x, &y)| {
        sum_sign(&[(x, 1.), (y, 1.), (-start, 1.), (-end, 1.)]) == std::cmp::Ordering::Equal
    })
}
struct Group {
    curve: Curve,
    reverse: Option<Curve>,
    net: i32,
    contributions: Vec<WeightedSource>,
}
fn compact(graph: ChartBoundaryGraph) -> Option<WeightedChartGraph> {
    let original_edges = graph.edges.len();
    let mut groups: Vec<Group> = Vec::new();
    for (index, edge) in graph.edges.into_iter().enumerate() {
        let found = groups.iter().enumerate().find_map(|(i, g)| {
            if g.curve == edge.curve {
                Some((i, 1i8))
            } else if reverse_encoding(&g.curve, &edge.curve) {
                Some((i, -1))
            } else {
                None
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
