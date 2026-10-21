//! Two-axis assembly: exact seam cancellation plus certified winding constant.
use super::chart::{
    ChartAssembly, ChartBoundaryGraph, ChartCalibration, ChartEdge, endpoint, vertices,
};
use super::{PeriodicWindingReport, Report, inspect_periodic_winding, verify_lifted_region};
use crate::{Result, check, curve::Curve};
mod corner;
mod flux;
#[derive(Clone, Copy, Debug)]
pub struct ChartAssemblyLimits {
    pub max_segments: usize,
    pub max_links: usize,
    pub max_edges: usize,
    /// Per source/candidate winding query; cells are shared between both.
    pub max_images: usize,
    pub max_cells: usize,
    pub tolerance_uv: f64,
}
pub(super) fn cycles(edges: &[ChartEdge], unique: bool) -> Option<Vec<Vec<usize>>> {
    let next = if unique {
        super::inspect_contact_ownership(edges, 1024)
            .ok()?
            .successors?
    } else {
        let points = vertices(edges);
        if points
            .iter()
            .any(|v| v.starts.is_empty() || v.starts.len() != v.ends.len())
        {
            return None;
        }
        let mut next = vec![0; edges.len()];
        for v in points {
            for (end, start) in v.ends.iter().zip(&v.starts) {
                next[*end] = *start;
            }
        }
        next
    };
    let mut seen = vec![false; edges.len()];
    let mut loops = Vec::with_capacity(edges.len().min(8));
    for start in 0..edges.len() {
        if seen[start] {
            continue;
        }
        let mut current = start;
        let mut cycle = Vec::with_capacity(edges.len());
        loop {
            if seen[current] {
                if current != start {
                    return None;
                }
                break;
            }
            seen[current] = true;
            cycle.push(current);
            current = next[current];
        }
        loops.push(cycle);
    }
    Some(loops)
}
fn direction(axis: usize, side: usize) -> i32 {
    if (axis == 0 && side == 1) || (axis == 1 && side == 0) {
        1
    } else {
        -1
    }
}
fn coefficient(
    edges: &[ChartEdge],
    axis: usize,
    side: usize,
    chart: [[f64; 2]; 2],
    range: [f64; 2],
) -> i32 {
    let trans = 1 - axis;
    let mut value = 0;
    for edge in edges {
        let a = endpoint(&edge.curve, false);
        let b = endpoint(&edge.curve, true);
        if a[axis] != chart[axis][side] || b[axis] != chart[axis][side] {
            continue;
        }
        if a[trans].min(b[trans]) <= range[0] && range[1] <= a[trans].max(b[trans]) {
            value += if a[trans] < b[trans] {
                direction(axis, side)
            } else {
                -direction(axis, side)
            };
        }
    }
    value
}
/// Assemble both UV seams. Exact source/partition region identity is required.
/// Opposite connector chains fix the boundary, and certified source/candidate
/// winding at anchor fixes its otherwise unknown full-chart integer constant.
/// The perimeter is subdivided and opposite coefficients canceled before final
/// cycles are published. Corner endpoint ambiguities and multiplicity remain
/// unresolved. Output is oriented boundary cycles, not regular B-rep faces.
pub fn assemble_periodic_chart(
    sources: &[Curve],
    partitions: &[Report],
    chart: [[f64; 2]; 2],
    anchor: [f64; 2],
    limits: ChartAssemblyLimits,
) -> Result<ChartAssembly> {
    validate_limits(chart, anchor, limits)?;
    let region = verify_lifted_region(
        sources,
        partitions,
        chart,
        limits.max_segments,
        limits.max_links,
    )?;
    if !region.periodic_winding_preserved {
        return Ok(super::chart::refuse(
            ChartAssembly {
                source_region: region,
                chart_winding_preserved: false,
                graph: None,
                anchor_band: None,
                calibration: None,
                reason: None,
            },
            "input-region-unproven",
        ));
    }
    let graph = assemble_candidate(partitions, chart, anchor, limits, true, || {
        let source_loops: Vec<_> = sources.iter().map(|c| vec![c.clone()]).collect();
        inspect_periodic_winding(
            &source_loops,
            chart,
            anchor,
            limits.tolerance_uv,
            limits.max_images,
            limits.max_cells,
        )
    })?;
    Ok(ChartAssembly {
        source_region: region,
        chart_winding_preserved: graph.chart_winding_preserved,
        graph: graph.graph,
        anchor_band: None,
        calibration: graph.calibration,
        reason: graph.reason,
    })
}
pub(super) fn validate_limits(
    chart: [[f64; 2]; 2],
    anchor: [f64; 2],
    limits: ChartAssemblyLimits,
) -> Result<()> {
    check(
        (1..=512).contains(&limits.max_edges)
            && (1..=4096).contains(&limits.max_images)
            && (1..=100000).contains(&limits.max_cells)
            && limits.tolerance_uv.is_finite()
            && limits.tolerance_uv > 0.
            && chart
                .iter()
                .all(|r| r.iter().all(|x| x.is_finite()) && r[0] < r[1])
            && (0..2).all(|a| {
                anchor[a].is_finite() && chart[a][0] < anchor[a] && anchor[a] < chart[a][1]
            }),
        "Invalid two-axis assembly limits or interior anchor",
    )
}
#[derive(Clone, Debug)]
pub(super) struct CandidateAssembly {
    pub(super) chart_winding_preserved: bool,
    pub(super) graph: Option<ChartBoundaryGraph>,
    pub(super) calibration: Option<ChartCalibration>,
    pub(super) reason: Option<&'static str>,
}
fn refuse(mut out: CandidateAssembly, reason: &'static str) -> CandidateAssembly {
    out.reason = Some(reason);
    out
}
/// Private boundary-chain assembly shared by exact and tolerance-qualified
/// admission. Callers must prove the source query applies to the candidate
/// lifted chain, its lattice translations, chart hulls and exact closure.
pub(super) fn assemble_candidate(
    partitions: &[Report],
    chart: [[f64; 2]; 2],
    anchor: [f64; 2],
    limits: ChartAssemblyLimits,
    owned_cycles: bool,
    source_query: impl FnOnce() -> Result<PeriodicWindingReport>,
) -> Result<CandidateAssembly> {
    let mut out = CandidateAssembly {
        chart_winding_preserved: false,
        graph: None,
        calibration: None,
        reason: None,
    };
    let source_count = partitions.iter().map(|p| p.pieces.len()).sum::<usize>();
    if source_count > limits.max_edges {
        return Ok(refuse(out, "edge-budget"));
    }
    let mut edges = Vec::new();
    for (li, p) in partitions.iter().enumerate() {
        for (pi, piece) in p.pieces.iter().enumerate() {
            for axis in 0..2 {
                for side in 0..2 {
                    if piece
                        .curve
                        .control_points
                        .iter()
                        .all(|v| v[axis] == chart[axis][side])
                    {
                        return Ok(refuse(out, "seam-coincidence-unproven"));
                    }
                }
            }
            edges.push(ChartEdge {
                curve: piece.curve.clone(),
                source: Some([li, pi]),
            });
        }
    }
    let original = vertices(&edges);
    let mut sides = [[Vec::new(), Vec::new()], [Vec::new(), Vec::new()]];
    let mut has_corner = false;
    let mut has_multibranch = false;
    for (i, v) in original.iter().enumerate() {
        match (v.starts.len(), v.ends.len()) {
            (starts, ends) if starts > 0 && starts == ends => continue,
            (1, 0) | (0, 1) => {}
            _ => has_multibranch = true,
        }
        let locations: Vec<_> = (0..2)
            .flat_map(|axis| (0..2).map(move |side| (axis, side)))
            .filter(|&(axis, side)| v.point[axis] == chart[axis][side])
            .collect();
        if locations.len() == 2 {
            has_corner = true;
            continue;
        }
        if locations.len() != 1 {
            return Ok(refuse(out, "interior-endpoint-unproven"));
        }
        let (axis, side) = locations[0];
        sides[axis][side].push(i);
    }
    let mut raw_pairs = Vec::new();
    if has_corner || has_multibranch {
        let connectors = if has_multibranch {
            flux::build(&original, chart, limits.max_edges - source_count)?
        } else {
            corner::build(&original, chart, limits.max_edges - source_count)?
        };
        match connectors {
            corner::CornerConnectors::Proven {
                edges: added,
                pairs,
            } => {
                let base = edges.len();
                raw_pairs.extend(pairs.into_iter().map(|[a, b]| [a + base, b + base]));
                edges.extend(added);
            }
            corner::CornerConnectors::Unresolved(reason) => return Ok(refuse(out, reason)),
        }
    } else {
        for axis in 0..2 {
            let trans = 1 - axis;
            for side in &mut sides[axis] {
                side.sort_by(|a, b| {
                    original[*a].point[trans].total_cmp(&original[*b].point[trans])
                });
            }
            if sides[axis][0].len() != sides[axis][1].len() || sides[axis][1].len() % 2 != 0 {
                return Ok(refuse(out, "seam-pairing-unproven"));
            }
            let incoming = |i: usize| !original[i].ends.is_empty();
            let mut used = vec![false; original.len()];
            for pair in sides[axis][1].chunks_exact(2) {
                if incoming(pair[0]) == incoming(pair[1]) {
                    return Ok(refuse(out, "seam-order-unproven"));
                }
                let (from, to) = if incoming(pair[0]) {
                    (pair[0], pair[1])
                } else {
                    (pair[1], pair[0])
                };
                let a = original[from].point;
                let b = original[to].point;
                let left_from = sides[axis][0]
                    .iter()
                    .copied()
                    .find(|&i| incoming(i) && original[i].point[trans] == b[trans]);
                let left_to = sides[axis][0]
                    .iter()
                    .copied()
                    .find(|&i| !incoming(i) && original[i].point[trans] == a[trans]);
                let (Some(lf), Some(lt)) = (left_from, left_to) else {
                    return Ok(refuse(out, "opposite-seam-unproven"));
                };
                if used[lf] || used[lt] {
                    return Ok(refuse(out, "seam-alias-unproven"));
                }
                used[lf] = true;
                used[lt] = true;
                if edges.len() + 2 > limits.max_edges {
                    return Ok(refuse(out, "edge-budget"));
                }
                let first = edges.len();
                edges.push(ChartEdge {
                    curve: Curve::from_polyline(vec![a.to_vec(), b.to_vec()])?,
                    source: None,
                });
                edges.push(ChartEdge {
                    curve: Curve::from_polyline(vec![
                        original[lf].point.to_vec(),
                        original[lt].point.to_vec(),
                    ])?,
                    source: None,
                });
                raw_pairs.push([first, first + 1]);
            }
            if sides[axis][0].iter().any(|&i| !used[i]) {
                return Ok(refuse(out, "incomplete-seam-coverage"));
            }
        }
    }
    let Some(raw_cycles) = cycles(&edges, false) else {
        return Ok(refuse(out, "raw-cycle-coverage-unproven"));
    };
    if edges.len() > 256 {
        return Ok(refuse(out, "calibration-curve-budget"));
    }
    let raw_loops: Vec<Vec<Curve>> = raw_cycles
        .iter()
        .map(|cycle| cycle.iter().map(|&i| edges[i].curve.clone()).collect())
        .collect();
    let source_query = source_query()?;
    let source_winding = source_query.winding;
    let cells = source_query.cells;
    out.calibration = Some(ChartCalibration {
        point: anchor,
        source: source_query,
        candidate: None,
        offset: None,
        raw_graph: ChartBoundaryGraph {
            edges: edges.clone(),
            loops: raw_cycles,
            connector_pairs: raw_pairs,
        },
    });
    let Some(source_winding) = source_winding else {
        return Ok(refuse(out, "source-anchor-unproven"));
    };
    if cells == limits.max_cells {
        return Ok(refuse(out, "anchor-work-budget"));
    }
    let candidate_query = inspect_periodic_winding(
        &raw_loops,
        chart,
        anchor,
        limits.tolerance_uv,
        limits.max_images,
        limits.max_cells - cells,
    )?;
    let candidate_winding = candidate_query.winding;
    out.calibration.as_mut().unwrap().candidate = Some(candidate_query);
    let Some(candidate_winding) = candidate_winding else {
        return Ok(refuse(out, "candidate-anchor-unproven"));
    };
    let Some(offset) = source_winding.checked_sub(candidate_winding) else {
        return Ok(refuse(out, "winding-overflow"));
    };
    out.calibration.as_mut().unwrap().offset = Some(offset);
    let mut assembled = edges[..source_count].to_vec();
    let mut connector_pairs = Vec::new();
    for axis in 0..2 {
        let trans = 1 - axis;
        let mut levels = vec![chart[trans][0], chart[trans][1]];
        for v in &original {
            if v.point[axis] == chart[axis][0] || v.point[axis] == chart[axis][1] {
                levels.push(v.point[trans]);
            }
        }
        levels.sort_by(f64::total_cmp);
        levels.dedup();
        for range in levels.windows(2) {
            let r = [range[0], range[1]];
            let a = coefficient(&edges[source_count..], axis, 0, chart, r);
            let b = coefficient(&edges[source_count..], axis, 1, chart, r);
            if a != b {
                return Ok(refuse(out, "perimeter-periodicity-unproven"));
            }
            let Some(value) = offset.checked_add(a) else {
                return Ok(refuse(out, "winding-overflow"));
            };
            if value == 0 {
                continue;
            }
            if owned_cycles && value != 1 && value != -1 {
                return Ok(refuse(out, "perimeter-multiplicity-unproven"));
            }
            let copies = value.unsigned_abs() as usize;
            if copies > (limits.max_edges - assembled.len()) / 2 {
                return Ok(refuse(out, "edge-budget"));
            }
            for _ in 0..copies {
                let first = assembled.len();
                for side in 0..2 {
                    let increasing = direction(axis, side) * value > 0;
                    let mut start = [0.; 2];
                    let mut end = [0.; 2];
                    start[axis] = chart[axis][side];
                    end[axis] = chart[axis][side];
                    start[trans] = if increasing { r[0] } else { r[1] };
                    end[trans] = if increasing { r[1] } else { r[0] };
                    assembled.push(ChartEdge {
                        curve: Curve::from_polyline(vec![start.to_vec(), end.to_vec()])?,
                        source: None,
                    });
                }
                connector_pairs.push([first, first + 1]);
            }
        }
    }
    let Some(loops) = cycles(&assembled, owned_cycles) else {
        return Ok(refuse(out, "assembled-contact-unproven"));
    };
    out.graph = Some(ChartBoundaryGraph {
        edges: assembled,
        loops,
        connector_pairs,
    });
    out.chart_winding_preserved = true;
    Ok(out)
}
