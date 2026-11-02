//! Independent full-face exclusion/unique-root ownership after discovery.
use super::{Boundary, Proof, domains};
use crate::{
    Result,
    surface::Surface,
    surface_offset::{self, PairDomain},
};
use std::collections::VecDeque;
#[derive(Debug, PartialEq)]
pub struct OwnedBox {
    pub domain: PairDomain,
    pub endpoint: usize,
}
#[derive(Debug, PartialEq)]
pub struct FaceCoverage {
    pub owned: Vec<OwnedBox>,
    pub pending: Vec<PairDomain>,
    pub visited: usize,
    pub excluded: usize,
    pub normal_span_visits: usize,
    pub root_coverage_proven: bool,
    pub absence_proven: bool,
}
impl FaceCoverage {
    pub(super) fn empty() -> Self {
        Self {
            owned: vec![],
            pending: vec![],
            visited: 0,
            excluded: 0,
            normal_span_visits: 0,
            root_coverage_proven: false,
            absence_proven: false,
        }
    }
}
fn contains(a: PairDomain, b: PairDomain) -> bool {
    (0..2).all(|s| (0..2).all(|k| a[s][k][0] <= b[s][k][0] && a[s][k][1] >= b[s][k][1]))
}
pub(super) fn certify(
    surfaces: [&Surface; 2],
    distances: [f64; 2],
    boundary: Boundary,
    endpoints: &[Proof],
    max_boxes: usize,
    spans: usize,
) -> Result<FaceCoverage> {
    let mut full = surfaces.map(domains);
    full[boundary.source][boundary.axis] =
        [full[boundary.source][boundary.axis][usize::from(boundary.high)]; 2];
    let widths = full.map(|s| s.map(|r| r[1] - r[0]));
    let mut queue = VecDeque::from([full]);
    let mut out = FaceCoverage::empty();
    while let Some(cell) = queue.pop_front() {
        if out.visited == max_boxes {
            out.pending.push(cell);
            out.pending.extend(queue);
            break;
        }
        out.visited += 1;
        // A covered box need not contain a root. IF it does, the independently
        // certified unique endpoint owns it. Merely intersecting a tube is not
        // ownership, and endpoints on a different chart face cannot supply it.
        if let Some((index, _)) = endpoints.iter().enumerate().find(|(_, p)| {
            p.boundary() == boundary && p.witness().is_some() && contains(p.original_tube(), cell)
        }) {
            out.owned.push(OwnedBox {
                domain: cell,
                endpoint: index,
            });
            continue;
        }
        let a = surface_offset::bounds(surfaces[0], cell[0], distances[0], spans)?;
        let b = surface_offset::bounds(surfaces[1], cell[1], distances[1], spans)?;
        out.normal_span_visits += a.spans + b.spans;
        if let (Some(a), Some(b)) = (a.image, b.image) {
            if crate::surface_distance::enclosure_distance(&a, &b)?.0 > 0. {
                out.excluded += 1;
                continue;
            }
        }
        let (s, k, _) = (0..2)
            .flat_map(|s| (0..2).map(move |k| (s, k)))
            .filter(|(s, k)| widths[*s][*k] > 0.)
            .map(|(s, k)| (s, k, (cell[s][k][1] - cell[s][k][0]) / widths[s][k]))
            .max_by(|a, b| a.2.total_cmp(&b.2))
            .expect("three positive free coordinates");
        let r = cell[s][k];
        let mid = r[0] * 0.5 + r[1] * 0.5;
        if mid <= r[0] || mid >= r[1] {
            out.pending.push(cell);
            continue;
        }
        for range in [[r[0], mid], [mid, r[1]]] {
            let mut next = cell;
            next[s][k] = range;
            queue.push_back(next);
        }
    }
    out.root_coverage_proven = out.pending.is_empty();
    out.absence_proven = out.root_coverage_proven && out.owned.is_empty();
    Ok(out)
}
