//! One rigorously enclosed anchor; this diagnostic alone supplies no full-carrier authority.
use super::{Carrier, Limits, interior};
use nurbs_core::{
    Error, Result,
    interval_eval::{self, Interval as I},
};
pub(crate) struct AnchorReport {
    pub positive: bool,
    pub curve_spans: usize,
    pub normal_spans: usize,
    pub reason: &'static str,
}
pub(crate) fn inspect<E: Carrier>(edge: &E, sign: i32, limits: Limits) -> Result<AnchorReport> {
    if limits.curve_spans > 100000 || limits.normal_spans > 100000 {
        return Err(Error::new(
            "BREP_SOURCE_MATERIAL_ORIENTATION",
            "Bound anchor work",
        ));
    }
    let mut out = AnchorReport {
        positive: false,
        curve_spans: 0,
        normal_spans: 0,
        reason: "source-material-orientation-unproven",
    };
    // The canonical carrier may be root-trimmed. A fixed 0.5 could lie
    // outside its admitted restriction; choose a point strictly between
    // certified endpoint enclosures instead of sampling an unowned span.
    let ends = edge.fragment(0).parameter_bounds();
    let ends = (|| -> Result<[I; 2]> {
        Ok([
            edge.map_parameter(0, I::new(ends[0][0], ends[0][1])?)?,
            edge.map_parameter(0, I::new(ends[1][0], ends[1][1])?)?,
        ])
    })();
    let Ok([a, b]) = ends else {
        return Ok(out);
    };
    let Some(anchor) = interior(a, b) else {
        return Ok(out);
    };
    let mut normals = Vec::new();
    for side in 0..2 {
        let f = edge.fragment(side);
        let c = f.curve();
        let t = match edge.inverse_parameter(side, I::point(anchor)) {
            Ok(t) => t,
            Err(_) => return Ok(out),
        };
        let count = (c.degree..c.control_points.len())
            .filter(|&i| {
                c.knots[i] < c.knots[i + 1] && c.knots[i] <= t.hi && c.knots[i + 1] >= t.lo
            })
            .count();
        if count > limits.curve_spans - out.curve_spans {
            return Ok(out);
        }
        out.curve_spans += count;
        let uv = match interval_eval::evaluate_interval(c, t) {
            Ok(uv) => uv,
            Err(_) => return Ok(out),
        };
        let s = f.surface();
        let ranges = [
            [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
            [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
        ];
        let mut rect = [[0.; 2]; 2];
        for axis in 0..2 {
            let q = match uv[axis].intersect(ranges[axis][0], ranges[axis][1]) {
                Ok(q) => q,
                Err(_) => return Ok(out),
            };
            rect[axis] = [q.lo, q.hi];
        }
        let remaining = limits.normal_spans - out.normal_spans;
        if remaining == 0 {
            return Ok(out);
        }
        let (normal, used) = match nurbs_core::normal_alignment::normal_bounds(s, rect, remaining) {
            Ok(r) => r,
            Err(_) => {
                out.normal_spans = limits.normal_spans;
                return Ok(out);
            }
        };
        out.normal_spans += used;
        let Some(normal) = normal else {
            return Ok(out);
        };
        normals.push(normal);
    }
    let mut dot = I::point(0.);
    for k in 0..3 {
        dot = dot.add(
            I::new(normals[0][k][0], normals[0][k][1])?
                .mul(I::new(normals[1][k][0], normals[1][k][1])?)?,
        )?;
    }
    if sign == -1 {
        dot = I::new(-dot.hi, -dot.lo)?;
    }
    if sign != 1 && sign != -1 {
        return Ok(out);
    }
    if dot.lo <= 0. {
        out.reason = if dot.hi < 0. {
            "source-material-smooth-seam-opposed-normals"
        } else {
            "source-material-smooth-seam-anchor-unproven"
        };
        return Ok(out);
    }
    out.positive = true;
    out.reason = "source-material-oriented-anchor-qualified";
    Ok(out)
}
