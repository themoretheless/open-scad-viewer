//! Full natural-domain root coverage by freshly certified offset-contact paths.
use super::{ContactBand, PairDomain, bounds};
use crate::{Result, check, offset_contact_path as path, surface::Surface};
use std::collections::VecDeque;
mod discovery;
pub use discovery::{DiscoveryLimits, DiscoveryReport, discover};
pub mod graph;

#[derive(Clone, Copy)]
pub struct PathRequest {
    pub axis: usize,
    pub drive: [f64; 2],
    pub seeds: [[[f64; 2]; 2]; 2],
    pub limits: path::Limits,
}
#[derive(Clone, Copy)]
pub struct Limits {
    pub boxes: usize,
    pub spans_per_bound: usize,
    /// Shared visited-path-cell allowance; each request gets at least one.
    pub path_cells: usize,
}
pub struct OwnedBox {
    pub domain: PairDomain,
    pub path: usize,
    pub segment: usize,
}
pub struct Report {
    pub paths: Vec<path::Report>,
    /// If a root lies in this box, it belongs to this proved path segment.
    /// Assignment does not claim the box itself contains a root.
    pub covered: Vec<OwnedBox>,
    /// Every remaining unexcluded region, including all unvisited boxes.
    pub pending: Vec<PairDomain>,
    pub visited_boxes: usize,
    pub excluded_boxes: usize,
    pub normal_span_visits: usize,
    pub path_cells: usize,
    pub root_coverage_proven: bool,
    pub absence_proven: bool,
    pub reason: &'static str,
}
fn domain(s: &Surface) -> [[f64; 2]; 2] {
    [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ]
}
fn contains(outer: PairDomain, inner: PairDomain) -> bool {
    (0..2).all(|s| {
        (0..2).all(|k| inner[s][k][0] >= outer[s][k][0] && inner[s][k][1] <= outer[s][k][1])
    })
}
/// Prove that ALL intersections over both complete natural source charts lie
/// on the returned fresh continuous paths. Requests contain numerical seeds and
/// work limits, never caller-positive certificates. Every request is replayed;
/// all natural parameter pairs are then either excluded by complete offset
/// enclosures or assigned wholly inside one freshly proved uniqueness tube.
/// No sample, nearby endpoint or partly overlapping tube supplies coverage.
///
/// This proves root-set coverage, not connected-component counts, path graph
/// topology, trimmed membership, support extension, a fillet envelope or solid.
/// Natural-boundary/tangent/pole regions remain pending unless these same
/// complete gates prove them. Source charts must be continuous/nonperiodic.
pub fn certify(
    surfaces: [&Surface; 2],
    distances: [f64; 2],
    requests: &[PathRequest],
    limits: Limits,
) -> Result<Report> {
    for s in surfaces {
        s.validate()?;
    }
    check(
        distances.iter().all(|d| d.is_finite() && *d != 0.)
            && distances[0].abs() == distances[1].abs(),
        "Root coverage requires equal nonzero signed offset magnitudes",
    )?;
    check(
        requests.len() <= 32
            && (1..=100000).contains(&limits.boxes)
            && (1..=100000).contains(&limits.spans_per_bound)
            && (1..=1000000).contains(&limits.path_cells)
            && limits.path_cells >= requests.len(),
        "Choose bounded global root-coverage and path work",
    )?;
    let natural = surfaces.map(domain);
    check(
        (0..2).all(|s| {
            !surfaces[s].periodic_u
                && !surfaces[s].periodic_v
                && super::source::source_continuous_for_offset(surfaces[s], natural[s])
                && natural[s]
                    .iter()
                    .all(|r| r[0] < r[1] && (r[1] - r[0]).is_finite())
        }),
        "Root coverage requires finite continuous nonperiodic natural charts",
    )?;
    let mut out = Report {
        paths: vec![],
        covered: vec![],
        pending: vec![],
        visited_boxes: 0,
        excluded_boxes: 0,
        normal_span_visits: 0,
        path_cells: 0,
        root_coverage_proven: false,
        absence_proven: false,
        reason: "coverage-unresolved",
    };
    let mut tubes = Vec::new();
    for (index, request) in requests.iter().enumerate() {
        let mut work = request.limits;
        // Reserve one cell for every later request, so invalid later inputs
        // cannot be hidden by an earlier path consuming the entire allowance.
        work.cells = work
            .cells
            .min(limits.path_cells - out.path_cells - (requests.len() - index - 1));
        let report = path::certify(
            surfaces,
            distances,
            request.axis,
            request.drive,
            request.seeds,
            work,
        )?;
        out.path_cells += report.visited;
        if report.continuous_path_proven {
            for (segment, cell) in report.cells.iter().enumerate() {
                if matches!(cell.verdict, ContactBand::ContinuousBranch(_)) {
                    let mut first = [[0.; 2]; 2];
                    first[request.axis] = cell.drive;
                    first[1 - request.axis] = cell.first_other;
                    tubes.push(([first, cell.second], index, segment));
                }
            }
        }
        out.paths.push(report);
    }
    let widths = natural.map(|d| d.map(|r| r[1] - r[0]));
    let mut queue = VecDeque::from([natural]);
    while let Some(cell) = queue.pop_front() {
        if out.visited_boxes == limits.boxes {
            out.pending.push(cell);
            out.pending.extend(queue);
            out.reason = "coverage-work-limit";
            break;
        }
        out.visited_boxes += 1;
        if let Some((_, path, segment)) = tubes.iter().find(|(tube, _, _)| contains(*tube, cell)) {
            out.covered.push(OwnedBox {
                domain: cell,
                path: *path,
                segment: *segment,
            });
            continue;
        }
        let a = bounds(surfaces[0], cell[0], distances[0], limits.spans_per_bound)?;
        let b = bounds(surfaces[1], cell[1], distances[1], limits.spans_per_bound)?;
        out.normal_span_visits += a.spans + b.spans;
        if let (Some(a), Some(b)) = (a.image, b.image) {
            if crate::surface_distance::enclosure_distance(&a, &b)?.0 > 0. {
                out.excluded_boxes += 1;
                continue;
            }
        }
        let (side, axis) = (0..2)
            .flat_map(|s| (0..2).map(move |k| (s, k)))
            .max_by(|&(s, k), &(t, j)| {
                ((cell[s][k][1] - cell[s][k][0]) / widths[s][k])
                    .total_cmp(&((cell[t][j][1] - cell[t][j][0]) / widths[t][j]))
            })
            .unwrap();
        let [lo, hi] = cell[side][axis];
        let mid = lo * 0.5 + hi * 0.5;
        if mid <= lo || mid >= hi {
            out.pending.push(cell);
            out.reason = "coverage-precision-limit";
            continue;
        }
        for range in [[lo, mid], [mid, hi]] {
            let mut next = cell;
            next[side][axis] = range;
            queue.push_back(next);
        }
    }
    out.root_coverage_proven = out.pending.is_empty();
    out.absence_proven = out.root_coverage_proven && out.covered.is_empty();
    if out.root_coverage_proven {
        out.reason = if out.absence_proven {
            "all-offset-pairs-excluded"
        } else {
            "all-offset-roots-covered"
        };
    }
    Ok(out)
}
