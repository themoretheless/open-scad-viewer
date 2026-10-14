//! Single-seam boundary assembly with exact oriented seam cancellation.
use super::winding::PeriodicWindingReport;
use super::{RegionTransfer, Report, verify_lifted_region};
use crate::{Result, check, curve::Curve};

#[derive(Clone, Debug)]
pub struct ChartEdge {
    pub curve: Curve,
    /// Original [loop, piece]; None for an added chart-boundary connector.
    pub source: Option<[usize; 2]>,
}
#[derive(Clone, Debug)]
pub struct ChartBoundaryGraph {
    pub edges: Vec<ChartEdge>,
    /// Ordered edge indices, each edge used exactly once.
    pub loops: Vec<Vec<usize>>,
    /// Oppositely oriented exact periodic copies of seam connectors.
    pub connector_pairs: Vec<[usize; 2]>,
}
#[derive(Clone, Debug)]
pub struct ChartCalibration {
    pub point: [f64; 2],
    pub source: PeriodicWindingReport,
    pub candidate: Option<PeriodicWindingReport>,
    pub offset: Option<i32>,
    pub raw_graph: ChartBoundaryGraph,
}
#[derive(Clone, Debug)]
pub struct ChartAssembly {
    pub source_region: RegionTransfer,
    pub chart_winding_preserved: bool,
    pub graph: Option<ChartBoundaryGraph>,
    /// Nonempty open transverse strip outside every source/candidate region.
    pub anchor_band: Option<[f64; 2]>,
    pub calibration: Option<ChartCalibration>,
    pub reason: Option<&'static str>,
}
pub(super) struct Vertex {
    pub(super) point: [f64; 2],
    pub(super) starts: Vec<usize>,
    pub(super) ends: Vec<usize>,
}
pub(super) fn endpoint(c: &Curve, end: bool) -> [f64; 2] {
    let p = if end {
        c.control_points.last().unwrap()
    } else {
        &c.control_points[0]
    };
    [p[0], p[1]]
}
pub(super) fn vertices(edges: &[ChartEdge]) -> Vec<Vertex> {
    let mut out: Vec<Vertex> = Vec::new();
    for (index, edge) in edges.iter().enumerate() {
        for end in [false, true] {
            let point = endpoint(&edge.curve, end);
            let vertex = match out.iter().position(|v| v.point == point) {
                Some(i) => i,
                None => {
                    out.push(Vertex {
                        point,
                        starts: Vec::new(),
                        ends: Vec::new(),
                    });
                    out.len() - 1
                }
            };
            if end {
                out[vertex].ends.push(index);
            } else {
                out[vertex].starts.push(index);
            }
        }
    }
    out
}
pub(super) fn refuse(mut out: ChartAssembly, reason: &'static str) -> ChartAssembly {
    out.reason = Some(reason);
    out
}
/// Assemble local oriented boundary cycles for one wrapped UV axis. Exact
/// path/region identity is required. The transverse axis must not wrap and
/// must have an empty strip inside the chart. That strip fixes the winding
/// constant after all opposite seam connectors cancel as oriented chains.
/// Alternating unmatched seam endpoints are paired; contacts, overlaps and
/// non-alternating multiplicity remain unresolved. Returns boundary cycles,
/// not regular B-rep faces or certified simple loops. max_edges 1..512.
pub fn assemble_single_seam(
    sources: &[Curve],
    partitions: &[Report],
    chart: [[f64; 2]; 2],
    wrap_axis: usize,
    max_segments: usize,
    max_links: usize,
    max_edges: usize,
) -> Result<ChartAssembly> {
    check(
        wrap_axis < 2 && (1..=512).contains(&max_edges),
        "Invalid chart assembly axis or edge budget",
    )?;
    let region = verify_lifted_region(sources, partitions, chart, max_segments, max_links)?;
    let mut out = ChartAssembly {
        source_region: region,
        chart_winding_preserved: false,
        graph: None,
        anchor_band: None,
        calibration: None,
        reason: None,
    };
    if !out.source_region.periodic_winding_preserved {
        return Ok(refuse(out, "input-region-unproven"));
    }
    let transverse = 1 - wrap_axis;
    if partitions
        .iter()
        .any(|p| p.pieces.iter().any(|piece| piece.tile[transverse] != 0))
    {
        return Ok(refuse(out, "transverse-wrap-unproven"));
    }
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for c in sources.iter().chain(
        partitions
            .iter()
            .flat_map(|p| p.pieces.iter().map(|piece| &piece.curve)),
    ) {
        for point in &c.control_points {
            lo = lo.min(point[transverse]);
            hi = hi.max(point[transverse]);
        }
    }
    if lo < chart[transverse][0] || hi > chart[transverse][1] {
        return Ok(refuse(out, "transverse-source-outside-chart"));
    }
    out.anchor_band = if hi < chart[transverse][1] {
        Some([hi, chart[transverse][1]])
    } else if chart[transverse][0] < lo {
        Some([chart[transverse][0], lo])
    } else {
        None
    };
    if out.anchor_band.is_none() {
        return Ok(refuse(out, "empty-anchor-strip-unproven"));
    }
    let total = partitions.iter().map(|p| p.pieces.len()).sum::<usize>();
    if total > max_edges {
        return Ok(refuse(out, "edge-budget"));
    }
    let mut edges = Vec::with_capacity(total);
    for (loop_index, p) in partitions.iter().enumerate() {
        for (piece_index, piece) in p.pieces.iter().enumerate() {
            if [chart[wrap_axis][0], chart[wrap_axis][1]]
                .iter()
                .any(|boundary| {
                    piece
                        .curve
                        .control_points
                        .iter()
                        .all(|point| point[wrap_axis] == *boundary)
                })
            {
                return Ok(refuse(out, "seam-coincidence-unproven"));
            }
            edges.push(ChartEdge {
                curve: piece.curve.clone(),
                source: Some([loop_index, piece_index]),
            });
        }
    }
    let original_vertices = vertices(&edges);
    let mut sides = [Vec::new(), Vec::new()];
    for (i, v) in original_vertices.iter().enumerate() {
        match (v.starts.len(), v.ends.len()) {
            (1, 1) => continue,
            (1, 0) | (0, 1) => {}
            _ => return Ok(refuse(out, "contact-vertex-unproven")),
        }
        let side = if v.point[wrap_axis] == chart[wrap_axis][0] {
            0
        } else if v.point[wrap_axis] == chart[wrap_axis][1] {
            1
        } else {
            return Ok(refuse(out, "seam-endpoint-unproven"));
        };
        sides[side].push(i);
    }
    for side in &mut sides {
        side.sort_by(|a, b| {
            original_vertices[*a].point[transverse]
                .total_cmp(&original_vertices[*b].point[transverse])
        });
    }
    if sides[0].len() != sides[1].len() || sides[1].len() % 2 != 0 {
        return Ok(refuse(out, "seam-pairing-unproven"));
    }
    let mut paired = vec![false; original_vertices.len()];
    let mut connector_pairs = Vec::new();
    for pair in sides[1].chunks_exact(2) {
        let incoming = |index: usize| !original_vertices[index].ends.is_empty();
        if incoming(pair[0]) == incoming(pair[1]) {
            return Ok(refuse(out, "seam-order-unproven"));
        }
        let (from, to) = if incoming(pair[0]) {
            (pair[0], pair[1])
        } else {
            (pair[1], pair[0])
        };
        let a = original_vertices[from].point;
        let b = original_vertices[to].point;
        let opposite_from = sides[0]
            .iter()
            .copied()
            .find(|&i| incoming(i) && original_vertices[i].point[transverse] == b[transverse]);
        let opposite_to = sides[0]
            .iter()
            .copied()
            .find(|&i| !incoming(i) && original_vertices[i].point[transverse] == a[transverse]);
        let (Some(opposite_from), Some(opposite_to)) = (opposite_from, opposite_to) else {
            return Ok(refuse(out, "opposite-seam-unproven"));
        };
        if paired[opposite_from] || paired[opposite_to] {
            return Ok(refuse(out, "seam-alias-unproven"));
        }
        paired[opposite_from] = true;
        paired[opposite_to] = true;
        if edges.len() + 2 > max_edges {
            return Ok(refuse(out, "edge-budget"));
        }
        let first = edges.len();
        edges.push(ChartEdge {
            curve: Curve::from_polyline(vec![a.to_vec(), b.to_vec()])?,
            source: None,
        });
        edges.push(ChartEdge {
            curve: Curve::from_polyline(vec![
                original_vertices[opposite_from].point.to_vec(),
                original_vertices[opposite_to].point.to_vec(),
            ])?,
            source: None,
        });
        connector_pairs.push([first, first + 1]);
    }
    if sides[0].iter().any(|&i| !paired[i]) {
        return Ok(refuse(out, "incomplete-seam-coverage"));
    }
    let assembled_vertices = vertices(&edges);
    if assembled_vertices
        .iter()
        .any(|v| v.starts.len() != 1 || v.ends.len() != 1)
    {
        return Ok(refuse(out, "assembled-contact-unproven"));
    }
    let mut next = vec![0; edges.len()];
    for v in assembled_vertices {
        next[v.ends[0]] = v.starts[0];
    }
    let mut seen = vec![false; edges.len()];
    let mut loops = Vec::new();
    for start in 0..edges.len() {
        if seen[start] {
            continue;
        }
        let mut current = start;
        let mut cycle = Vec::new();
        loop {
            if seen[current] {
                if current != start {
                    return Ok(refuse(out, "cycle-coverage-unproven"));
                }
                break;
            }
            seen[current] = true;
            cycle.push(current);
            current = next[current];
        }
        loops.push(cycle);
    }
    // Every original piece occurs once, connector partners cancel exactly
    // modulo the authored period, and both winding sums vanish in anchor_band.
    out.graph = Some(ChartBoundaryGraph {
        edges,
        loops,
        connector_pairs,
    });
    out.chart_winding_preserved = true;
    Ok(out)
}
