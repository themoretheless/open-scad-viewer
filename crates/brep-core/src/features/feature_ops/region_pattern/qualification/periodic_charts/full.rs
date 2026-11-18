//! Full-lap quotient admission needs actual paired seam ownership and a proved
//! global quotient carrier. Neither rectangle bounds nor a flag suffice.
use super::*;
use std::collections::BTreeMap;
pub(super) fn verify(
    model: &Model,
    face: usize,
    bounds: [[f64; 2]; 2],
    traces: &[ChartTrace<'_>],
    natural: [[f64; 2]; 2],
    remaining: &mut usize,
    pairs: &mut usize,
    max_pairs: usize,
) -> Result<Option<bool>> {
    let f = &model.faces[face];
    let s = &f.surface;
    let flags = [s.periodic_u, s.periodic_v];
    let full = (0..2)
        .filter(|&k| {
            flags[k] && q(bounds[k][1]) - q(bounds[k][0]) == q(natural[k][1]) - q(natural[k][0])
        })
        .collect::<Vec<_>>();
    if full.len() != 1 {
        return Ok(None);
    }
    let axis = full[0];
    if flags[1 - axis]
        || bounds[1 - axis][0] < natural[1 - axis][0]
        || bounds[1 - axis][1] > natural[1 - axis][1]
        || bounds[1 - axis][0] >= bounds[1 - axis][1]
    {
        return Err(error(
            "Full-lap face needs one bounded outer quotient rectangle",
        ));
    }
    let edges = rectangle_sides(&model.loops[f.outer].coedges, bounds, traces)?;
    verify_shores(&edges, axis)?;
    if *pairs >= max_pairs || *remaining == 0 {
        return Err(error("Full-lap quotient proof work exhausted"));
    }
    let r = nurbs_core::surface_injectivity::certify_periodic_extrusion(
        s,
        axis,
        max_pairs - *pairs,
        *remaining,
    )?;
    charge(remaining, r.cells)?;
    *pairs += r.pairs;
    if !r.proven {
        return Err(error(format!(
            "Full-lap original quotient injectivity remains unproven: {}",
            r.reason
        )));
    }
    if let Some(end) = r.collapsed_end {
        let other = 1 - axis;
        let shore = natural[other][end];
        if bounds[other][end] == shore {
            let apex = r
                .source_frame
                .ok_or_else(|| error("Collapsed quotient has no original source frame"))?[3];
            let mut vertex = None;
            for (c, trace) in model.loops[f.outer].coedges.iter().zip(traces) {
                if !trace.poles().all(|p| p[other] == shore) {
                    continue;
                }
                let e = &model.edges[c.edge];
                if !e.degenerate
                    || e.vertices[0] != e.vertices[1]
                    || model.vertices[e.vertices[0]].point != apex
                    || e.curve
                        .control_points
                        .iter()
                        .any(|p| p.as_slice() != apex.as_slice())
                {
                    return Err(error(
                        "Collapsed periodic shore needs exact actual degenerate pole-edge authority",
                    ));
                }
                if vertex.is_some_and(|v| v != e.vertices[0]) {
                    return Err(error("Collapsed periodic shore has several pole vertices"));
                }
                vertex = Some(e.vertices[0]);
            }
            if vertex.is_none() {
                return Err(error(
                    "Collapsed periodic shore has no actual pole incidence",
                ));
            }
        }
    }
    Ok(Some(r.regular_interior))
}

pub(super) fn rectangle_sides(
    coedges: &[crate::Coedge],
    bounds: [[f64; 2]; 2],
    traces: &[ChartTrace<'_>],
) -> Result<[BTreeMap<usize, bool>; 4]> {
    if coedges.len() != traces.len() {
        return Err(error("Full-lap contour trace census is incomplete"));
    }
    let mut ranges: [Vec<[f64; 2]>; 4] = std::array::from_fn(|_| Vec::new());
    let mut edges: [BTreeMap<usize, bool>; 4] = std::array::from_fn(|_| BTreeMap::new());
    for (c, trace) in coedges.iter().zip(traces) {
        let (side, range) = match trace {
            ChartTrace::Segment([a, b]) => {
                let side = (0..4)
                    .find(|i| a[i / 2] == bounds[i / 2][i % 2] && a[i / 2] == b[i / 2])
                    .ok_or_else(|| error("Active full-lap UV boundary is not a rectangle side"))?;
                let k = 1 - side / 2;
                if a[k] == b[k] {
                    return Err(error("Active full-lap boundary side is empty"));
                }
                (side, [a[k].min(b[k]), a[k].max(b[k])])
            }
            ChartTrace::Curve(p) => {
                let d = p.domain();
                let degree = p.degree;
                let n = p.control_points.len();
                if !p.knots[..=degree].iter().all(|k| *k == d[0])
                    || !p.knots[n..].iter().all(|k| *k == d[1])
                {
                    return Err(error(
                        "Full-lap rectangle needs exact clamped actual UV endpoints",
                    ));
                }
                let side = (0..4)
                    .find(|i| {
                        p.control_points
                            .iter()
                            .all(|v| v[i / 2] == bounds[i / 2][i % 2])
                    })
                    .ok_or_else(|| error("Full-lap UV boundary is not an exact rectangle side"))?;
                let k = 1 - side / 2;
                let a = p.control_points[0][k];
                let b = p.control_points[n - 1][k];
                let range = [a.min(b), a.max(b)];
                if a == b
                    || p.control_points
                        .iter()
                        .any(|v| v[k] < range[0] || v[k] > range[1])
                {
                    return Err(error(
                        "Full-lap rectangle side has unproved original coverage",
                    ));
                }
                (side, range)
            }
        };
        ranges[side].push(range);
        if edges[side].insert(c.edge, c.reversed).is_some() {
            return Err(error("A quotient side repeats an actual edge"));
        }
    }
    for side in 0..4 {
        ranges[side].sort_by(|a, b| a[0].total_cmp(&b[0]));
        let k = 1 - side / 2;
        let mut end = bounds[k][0];
        for r in &ranges[side] {
            if r[0] != end {
                return Err(error(
                    "Full-lap boundary side coverage has a gap or overlap",
                ));
            }
            end = r[1];
        }
        if end != bounds[k][1] {
            return Err(error("Full-lap boundary side coverage is incomplete"));
        }
    }
    Ok(edges)
}

pub(super) fn verify_shores(edges: &[BTreeMap<usize, bool>; 4], axis: usize) -> Result<()> {
    let shores = [&edges[axis * 2], &edges[axis * 2 + 1]];
    if shores[0].is_empty()
        || shores[0].len() != shores[1].len()
        || shores[0]
            .iter()
            .any(|(edge, rev)| shores[1].get(edge) != Some(&!*rev))
    {
        return Err(error(
            "Full-lap shores must be opposite uses of the same actual seam edges",
        ));
    }
    Ok(())
}
