//! Original shared-edge parameter proposals; fresh owning split gates follow.
use super::{Ledger, refused};
use crate::{Model, Result, edge_partition};
use nurbs_core::{
    curve::Curve,
    foundation::{ProjectionStatus, project_curve_report},
};
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|i| a[i] * b[i]).sum()
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}
fn norm(a: [f64; 3]) -> f64 {
    a[0].hypot(a[1]).hypot(a[2])
}
fn parameter(curve: &Curve, point: [f64; 3], tol: f64) -> Result<f64> {
    let report = project_curve_report(curve, &point, None)?;
    if report.status != ProjectionStatus::Unique {
        return Err(refused("Original edge contact projection is not unique"));
    }
    let best = report
        .candidates
        .iter()
        .min_by(|a, b| a.distance_upper.total_cmp(&b.distance_upper))
        .ok_or_else(|| refused("No original edge parameter candidate"))?;
    let domain = curve.domain();
    let mut t = best.parameter_interval[0] / 2. + best.parameter_interval[1] / 2.;
    for _ in 0..32 {
        let e = curve.evaluate(t)?;
        let p = [e.point[0], e.point[1], e.point[2]];
        let d = sub(p, point);
        if norm(d) <= tol {
            break;
        }
        let v =
            e.d1.ok_or_else(|| refused("Original contact edge derivative unresolved"))?;
        let v = [v[0], v[1], v[2]];
        let speed = dot(v, v);
        if !speed.is_finite() || speed <= 0. {
            return Err(refused("Singular contact edge"));
        }
        let next = (t - dot(d, v) / speed).clamp(domain[0], domain[1]);
        if next == t {
            break;
        }
        t = next;
    }
    let e = curve.evaluate(t)?.point;
    if t <= domain[0] || t >= domain[1] || norm(sub([e[0], e[1], e[2]], point)) > tol {
        return Err(refused(
            "Contact corner must lie inside its original support/cap edge",
        ));
    }
    Ok(t)
}
fn use_parameter(
    model: &Model,
    face: usize,
    wire: usize,
    coedge: usize,
    t: f64,
    edge: usize,
    point: [f64; 3],
) -> Result<f64> {
    let c = &model.loops[wire].coedges[coedge];
    let pc = &c.pcurve;
    let e = model.edges[edge].curve.domain();
    let d = pc.domain();
    let mut fraction = (t - e[0]) / (e[1] - e[0]);
    if c.reversed {
        fraction = 1. - fraction;
    }
    let mut q = d[0] + fraction * (d[1] - d[0]);
    for _ in 0..32 {
        let uv = pc.evaluate(q)?;
        let s = model.faces[face]
            .surface
            .evaluate(uv.point[0], uv.point[1])?;
        let residual = sub(s.point, point);
        if norm(residual) <= model.tolerance_mm / 16. {
            break;
        }
        let v = uv
            .d1
            .ok_or_else(|| refused("Owned pcurve derivative unresolved"))?;
        let (su, sv) = s
            .first_derivatives()
            .ok_or_else(|| refused("Original support derivative unresolved"))?;
        let tangent = std::array::from_fn(|k| su[k] * v[0] + sv[k] * v[1]);
        let speed = dot(tangent, tangent);
        if !speed.is_finite() || speed <= 0. {
            return Err(refused("Singular original edge use"));
        }
        let next = (q - dot(residual, tangent) / speed).clamp(d[0], d[1]);
        if next == q {
            break;
        }
        q = next;
    }
    let uv = pc.evaluate(q)?;
    let p = model.faces[face]
        .surface
        .evaluate(uv.point[0], uv.point[1])?
        .point;
    if q <= d[0] || q >= d[1] || norm(sub(p, point)) > model.tolerance_mm / 8. {
        return Err(refused(
            "Original face use does not locate the shared edge parameter",
        ));
    }
    Ok(q)
}
pub(super) fn split(
    model: &Model,
    edge: usize,
    point: [f64; 3],
    work: &mut Ledger,
) -> Result<edge_partition::Partition> {
    let t = parameter(&model.edges[edge].curve, point, model.tolerance_mm / 16.)?;
    let e = model.edges[edge].curve.evaluate(t)?.point;
    let owned = [e[0], e[1], e[2]];
    let mut cuts = Vec::new();
    for (face, f) in model.faces.iter().enumerate() {
        for wire in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
            for (coedge, c) in model.loops[wire].coedges.iter().enumerate() {
                if c.edge == edge {
                    cuts.push(edge_partition::PcurveCut {
                        wire,
                        coedge,
                        parameter: use_parameter(model, face, wire, coedge, t, edge, owned)?,
                    });
                }
            }
        }
    }
    if cuts.len() != 2 {
        return Err(refused(
            "Contact corner must split exactly two owned face uses",
        ));
    }
    let result = edge_partition::partition(model, edge, t, &cuts, work.allowance(1)?)?;
    work.charge(result.cells, 0)?;
    Ok(result)
}
