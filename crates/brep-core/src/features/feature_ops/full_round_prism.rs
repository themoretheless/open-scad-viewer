//! Full-width rounds on proven XY extrusions with retained rear profiles.
//! The replaced strip must be isolated from every unmodified profile carrier.
use super::{FullRoundReport, dot, error, line, plane, sub};
use crate::{Model, Result, solid_audit::audit_solid};
use std::collections::BTreeSet;
pub mod groups;

fn endpoint(c: &nurbs_core::curve::Curve, end: bool) -> Result<[f64; 2]> {
    let d = c.domain();
    let p = c.evaluate(d[usize::from(end)])?.point;
    Ok([p[0], p[1]])
}

fn owns_segment(
    source: &Model,
    face: usize,
    curve: &nurbs_core::curve::Curve,
    z: f64,
) -> Result<bool> {
    let ends = [endpoint(curve, false)?, endpoint(curve, true)?];
    let near = |p: &[f64], q: [f64; 2]| {
        (p[0] - q[0]).hypot(p[1] - q[1]).hypot(p[2] - z) <= source.tolerance_mm
    };
    let f = &source.faces[face];
    Ok(std::iter::once(f.outer)
        .chain(f.holes.iter().copied())
        .any(|w| {
            source.loops[w].coedges.iter().any(|c| {
                let edge = &source.edges[c.edge];
                if edge.curve.degree != 1 || edge.curve.control_points.len() != 2 {
                    return false;
                }
                let a = &edge.curve.control_points[0];
                let b = &edge.curve.control_points[1];
                (near(a, ends[0]) && near(b, ends[1])) || (near(a, ends[1]) && near(b, ends[0]))
            })
        }))
}

/// Admit a single recognized extrusion. Three selected planar sides must be
/// consecutive in the outer profile: parallel supports and their transverse
/// end. Other rational profile curves and holes must lie behind the round's
/// contact line by a complete positive-weight control-hull bound.
pub fn build(source: &Model, faces: [usize; 3]) -> Result<FullRoundReport> {
    source.validate()?;
    if source.bodies.len() != 1
        || source.shells.len() != 1
        || faces.into_iter().collect::<BTreeSet<_>>().len() != 3
    {
        return Err(error(
            "Full Round requires one body and three distinct faces",
        ));
    }
    let p = crate::prism::recognize(source)?
        .ok_or_else(|| error("Full Round requires a proven XY profile extrusion"))?;
    let planes = faces
        .map(|f| plane(source, f))
        .into_iter()
        .collect::<Result<Vec<_>>>()?;
    let (origin, na) = planes[0];
    let (_, middle) = planes[1];
    let (_, nb) = planes[2];
    if dot(na, nb) > -1. + 1e-10
        || dot(na, middle).abs() > 1e-10
        || na[2].abs() > 1e-10
        || middle[2].abs() > 1e-10
    {
        return Err(error(
            "Full Round needs opposed vertical supports and a perpendicular end",
        ));
    }
    let x = na.map(|v| -v);
    // In XY a CCW outer loop traverses the end from right to left.
    let y = [-x[1], x[0], 0.];
    let sign = dot(y, middle);
    if sign.abs() < 1. - 1e-10 {
        return Err(error("Unresolved Full Round profile frame"));
    }
    let project = |q: [f64; 2]| {
        let v = sub([q[0], q[1], origin[2]], origin);
        [dot(x, v), dot(middle, v)]
    };
    let tol = source.tolerance_mm * 8.;
    let mut candidates = Vec::new();
    for (loop_id, wire) in p.loops.iter().enumerate() {
        for (i, c) in wire.iter().enumerate() {
            if c.degree != 1 || c.control_points.len() != 2 {
                continue;
            }
            if c.control_points
                .iter()
                .all(|q| dot(middle, sub([q[0], q[1], origin[2]], planes[1].0)).abs() <= tol)
            {
                candidates.push((loop_id, i));
            }
        }
    }
    if candidates.len() != 1 || candidates[0].0 != 0 {
        return Err(error(
            "Full Round requires one complete outer middle profile segment",
        ));
    }
    let (_, i) = candidates[0];
    let wire = &p.loops[0];
    if !owns_segment(source, faces[1], &wire[i], p.z_max)? {
        return Err(error(
            "Selected middle face does not own the replaced profile segment",
        ));
    }
    let n = wire.len();
    if n < 4 {
        return Err(error("Insufficient profile sides"));
    }
    let prev = (i + n - 1) % n;
    let next = (i + 1) % n;
    for (index, support) in [
        (prev, if sign > 0. { 2 } else { 0 }),
        (next, if sign > 0. { 0 } else { 2 }),
    ] {
        let c = &wire[index];
        if !owns_segment(source, faces[support], c, p.z_max)? {
            return Err(error(
                "Selected support face does not own the adjoining profile segment",
            ));
        }
        if c.degree != 1
            || c.control_points.len() != 2
            || c.control_points.iter().any(|q| {
                dot(
                    planes[support].1,
                    sub([q[0], q[1], origin[2]], planes[support].0),
                )
                .abs()
                    > tol
            })
        {
            return Err(error(
                "Selected supports must adjoin the middle along straight profile segments",
            ));
        }
    }
    let a = endpoint(&wire[i], false)?;
    let b = endpoint(&wire[i], true)?;
    let pa = project(a);
    let pb = project(b);
    let radius = (pa[0] - pb[0]).abs() / 2.;
    let contact_y = pa[1] - radius;
    if !radius.is_finite() || radius <= tol || (pa[1] - pb[1]).abs() > tol {
        return Err(error("Degenerate Full Round width or middle segment"));
    }
    // Prove strip isolation on entire rational carriers, including holes.
    for (l, curves) in p.loops.iter().enumerate() {
        for (j, c) in curves.iter().enumerate() {
            if l == 0 && [prev, i, next].contains(&j) {
                continue;
            }
            if c.weights.iter().any(|w| *w <= 0.)
                || c.control_points
                    .iter()
                    .any(|q| project([q[0], q[1]])[1] > contact_y - tol)
            {
                return Err(error(
                    "Unmodified profile enters the Full Round replacement strip",
                ));
            }
        }
    }
    let contact = |q: [f64; 2]| [q[0] - radius * middle[0], q[1] - radius * middle[1]];
    let ca = contact(a);
    let cb = contact(b);
    let center = [(ca[0] + cb[0]) / 2., (ca[1] + cb[1]) / 2., 0.];
    let start = (ca[1] - center[1]).atan2(ca[0] - center[0]).to_degrees();
    let sweep = 180.;
    let mut result_wire = Vec::new();
    for (j, c) in wire.iter().enumerate() {
        if j == prev {
            result_wire.push(line(endpoint(c, false)?, ca)?);
        } else if j == i {
            for k in 0..2 {
                let mut arc = nurbs_core::primitives::circle_arc(
                    center,
                    [0., 0., 1.],
                    radius,
                    start + k as f64 * 90.,
                    sweep / 2.,
                )?;
                for q in &mut arc.control_points {
                    q.truncate(2);
                }
                result_wire.push(arc);
            }
        } else if j == next {
            result_wire.push(line(cb, endpoint(c, true)?)?);
        } else {
            result_wire.push(c.clone());
        }
    }
    let mut loops = p.loops;
    loops[0] = result_wire;
    let mut model = crate::prism::extrude(&loops, p.z_min, p.z_max)?;
    super::full_round_caps::world_charts(&mut model)?;
    model.tolerance_mm = source.tolerance_mm;
    model.inherit_topology_ids(&[source]);
    model.validate()?;
    let audit = audit_solid(&model)?;
    if !audit.ok {
        return Err(error("Full Round solid audit did not qualify"));
    }
    let crown = [
        center[0] + radius * middle[0],
        center[1] + radius * middle[1],
    ];
    let seams = [ca, crown, cb].map(|q| [p.z_min, p.z_max].map(|z| [q[0], q[1], z]));
    let centers = [p.z_min, p.z_max].map(|z| [center[0], center[1], z]);
    let tangency = super::full_round_seams::qualify(&model, seams, centers, radius)?;
    Ok(FullRoundReport {
        model,
        replaced_face: faces[1],
        support_faces: [faces[0], faces[2]],
        radius_mm: radius,
        audit,
        tangency,
    })
}
