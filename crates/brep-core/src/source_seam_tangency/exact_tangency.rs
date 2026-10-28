//! Exact first-order compatibility with full original regular-normal coverage.
use super::{Carrier, Limits, validate_limits};
use nurbs_core::{
    Error, Result,
    interval_eval::{self, Interval as I},
};
pub struct ExactTangencySeam<E: Carrier> {
    edge: E,
}
impl<E: Carrier> ExactTangencySeam<E> {
    pub fn edge(&self) -> &E {
        &self.edge
    }
    pub fn sine_squared_bounds(&self) -> [f64; 2] {
        [0.; 2]
    }
}
pub struct ExactTangencyReport<E: Carrier> {
    pub seam: Option<ExactTangencySeam<E>>,
    pub exact_work: u64,
    pub cells: usize,
    pub curve_spans: usize,
    pub normal_spans: usize,
    pub accepted_cells: usize,
    pub uncertain_canonical: Option<[f64; 2]>,
    pub reason: &'static str,
}
/// Fresh original gradient identity only. Regular or factored-limit normal
/// coverage and body ownership must be independently retained by the caller.
pub(super) fn identity<E: Carrier>(
    edge: &E,
    axis: usize,
    max: u64,
) -> Result<(bool, u64, &'static str)> {
    let (Some(a), Some(b)) = (edge.exact_parameter_curve(0), edge.exact_parameter_curve(1)) else {
        return Ok((false, 0, "exact-tangent-original-maps-unproven"));
    };
    let r = nurbs_core::factored_surface_normals::prepare_exact_meridian_tangency(
        [edge.fragment(0).surface(), edge.fragment(1).surface()],
        [a.0, b.0],
        [a.1, b.1],
        [a.2, b.2],
        axis,
        max,
    )?;
    Ok((r.certificate.is_some(), r.work, r.reason))
}
fn regular<E: Carrier>(
    edge: &E,
    q: [f64; 2],
    limits: &Limits,
    out: &mut ExactTangencyReport<E>,
) -> Result<Option<bool>> {
    for side in 0..2 {
        let f = edge.fragment(side);
        let c = f.curve();
        let t = edge.inverse_parameter(side, I::new(q[0], q[1])?)?;
        let spans = (c.degree..c.control_points.len())
            .filter(|&i| {
                c.knots[i] < c.knots[i + 1] && c.knots[i] <= t.hi && c.knots[i + 1] >= t.lo
            })
            .count();
        if spans > limits.curve_spans - out.curve_spans {
            return Ok(None);
        }
        out.curve_spans += spans;
        let uv = interval_eval::evaluate_interval(c, t)?;
        let s = f.surface();
        let domains = [
            [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
            [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
        ];
        let mut query = [[0.; 2]; 2];
        for axis in 0..2 {
            let r = uv[axis].intersect(domains[axis][0], domains[axis][1])?;
            query[axis] = [r.lo, r.hi];
        }
        if out.normal_spans == limits.normal_spans {
            return Ok(None);
        }
        // Exact layout admission requires one original Bezier span. Pay that
        // visit BEFORE arithmetic, so numeric failures also retain its cost.
        out.normal_spans += 1;
        let (normal, used) = nurbs_core::normal_alignment::normal_bounds(s, query, 1)?;
        let Some(normal) = normal.filter(|_| used == 1) else {
            return Ok(Some(false));
        };
        let normal = normal.map(|r| I::new(r[0], r[1]));
        let normal = [normal[0].clone()?, normal[1].clone()?, normal[2].clone()?];
        if !normal.iter().any(|n| n.lo > 0. || n.hi < 0.) {
            return Ok(Some(false));
        }
    }
    Ok(Some(true))
}
/// Exact G1 tangent-plane agreement on the complete closed regular carrier.
/// First-order original identities, rather than sampled normals or a distance
/// tolerance, supply equality. Independent full native normal coverage extends
/// equality through isolated zeros of the proposed graph projection. No second
/// derivatives, Hessian compatibility, pole neighborhoods or face embedding
/// are inferred. Singular endpoints and exhausted work return no partial seam.
pub fn qualify_tangency_exact<E: Carrier>(
    edge: &E,
    limits: Limits,
    axis: usize,
    max_exact_work: u64,
) -> Result<ExactTangencyReport<E>> {
    validate_limits(&limits)?;
    if axis > 2 || !(1..=100000000).contains(&max_exact_work) {
        return Err(Error::new(
            "BREP_SOURCE_EXACT_TANGENCY",
            "Choose a world projection and bounded exact work",
        ));
    }
    let mut out = ExactTangencyReport {
        seam: None,
        exact_work: 0,
        cells: 0,
        curve_spans: 0,
        normal_spans: 0,
        accepted_cells: 0,
        uncertain_canonical: None,
        reason: "exact-tangent-work-limit",
    };
    let (equal, work, reason) = identity(edge, axis, max_exact_work)?;
    out.exact_work = work;
    if !equal {
        out.reason = reason;
        return Ok(out);
    }
    let ends = edge.fragment(0).parameter_bounds();
    let lo = ends.iter().map(|r| r[0]).fold(f64::INFINITY, f64::min);
    let hi = ends.iter().map(|r| r[1]).fold(f64::NEG_INFINITY, f64::max);
    let coverage = edge.map_parameter(0, I::new(lo, hi)?)?.intersect(0., 1.)?;
    let mut queue = vec![[coverage.lo, coverage.hi]];
    while let Some(q) = queue.pop() {
        out.uncertain_canonical = Some(q);
        if out.cells == limits.cells {
            return Ok(out);
        }
        out.cells += 1;
        match regular(edge, q, &limits, &mut out) {
            Ok(Some(true)) => {
                out.accepted_cells += 1;
                continue;
            }
            Ok(None) => return Ok(out),
            Ok(Some(false)) => {}
            Err(e) if e.code == "NURBS_NUMERIC_ERROR" => {}
            Err(e) => return Err(e),
        }
        let mid = q[0] * 0.5 + q[1] * 0.5;
        if !(q[0] < mid && mid < q[1]) {
            out.reason = "exact-tangent-resolution-limit";
            return Ok(out);
        }
        queue.push([mid, q[1]]);
        queue.push([q[0], mid]);
    }
    out.seam = Some(ExactTangencySeam { edge: edge.clone() });
    out.uncertain_canonical = None;
    out.reason = "exact-original-regular-carrier-g1-qualified";
    Ok(out)
}
#[cfg(test)]
mod tests;
