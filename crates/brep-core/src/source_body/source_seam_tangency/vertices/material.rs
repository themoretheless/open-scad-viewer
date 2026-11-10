//! Complete original material trim collar, including all other contours.
use super::corner::Corner;
use crate::{source_contour_proposal::SourceRegion, source_world_wire::Wire};
use nurbs_core::trim_domain::Location;
pub(super) struct Report {
    pub rectangle: Option<[[f64; 2]; 2]>,
    pub steps: usize,
    pub domain_cells: usize,
}
pub(super) fn certify(
    corner: &Corner<'_>,
    wires: &[Wire],
    region: &SourceRegion,
    tolerance_uv: f64,
    max_steps: usize,
    max_cells: usize,
) -> Report {
    let mut out = Report {
        rectangle: None,
        steps: 0,
        domain_cells: 0,
    };
    let mut rectangle = corner.rectangle;
    let disjoint =
        |a: [[f64; 2]; 2], b: [[f64; 2]; 2]| (0..2).any(|k| a[k][1] < b[k][0] || b[k][1] < a[k][0]);
    for (wi, wire) in wires.iter().enumerate() {
        for (ei, f) in wire.edges().iter().enumerate() {
            if out.steps == max_steps {
                return out;
            }
            out.steps += 1;
            if wi == corner.wire && corner.edges.contains(&ei) {
                continue;
            }
            let mut hull = [[f64::INFINITY, f64::NEG_INFINITY]; 2];
            // Positive original rational basis functions put the COMPLETE
            // original UV curve in this hull, hence also its root restriction.
            for p in &f.curve().control_points {
                if out.steps == max_steps {
                    return out;
                }
                out.steps += 1;
                for k in 0..2 {
                    hull[k][0] = hull[k][0].min(p[k]);
                    hull[k][1] = hull[k][1].max(p[k]);
                }
            }
            if disjoint(rectangle, hull) {
                continue;
            }
            let axis = (0..2).find(|&k| hull[k][0] > corner.uv[k] || hull[k][1] < corner.uv[k]);
            let Some(k) = axis else {
                return out;
            };
            if hull[k][0] > corner.uv[k] {
                let cut = corner.uv[k] * 0.5 + hull[k][0] * 0.5;
                if !(corner.uv[k] < cut && cut < hull[k][0]) {
                    return out;
                }
                rectangle[k][1] = rectangle[k][1].min(cut);
            } else {
                let cut = corner.uv[k] * 0.5 + hull[k][1] * 0.5;
                if !(hull[k][1] < cut && cut < corner.uv[k]) {
                    return out;
                }
                rectangle[k][0] = rectangle[k][0].max(cut);
            }
            if !disjoint(rectangle, hull) {
                return out;
            }
        }
    }
    let q = rectangle.map(|d| d[0] * 0.5 + d[1] * 0.5);
    if !(0..2).all(|k| rectangle[k][0] < q[k] && q[k] < rectangle[k][1]) || max_cells == 0 {
        return out;
    }
    // This is a certified component-location witness, not sampled geometry:
    // every other original boundary was proved disjoint from the rectangle;
    // the two owned natural sides are on its boundary, not in its interior.
    let Ok(r) = crate::source_contour_winding::classify(
        region.loops(),
        [[q[0]; 2], [q[1]; 2]],
        tolerance_uv,
        max_cells,
    ) else {
        out.domain_cells = max_cells;
        return out;
    };
    out.domain_cells = r.cells;
    if r.location == Location::Inside && r.winding == Some(region.chart_winding()) {
        out.rectangle = Some(rectangle);
    }
    out
}
