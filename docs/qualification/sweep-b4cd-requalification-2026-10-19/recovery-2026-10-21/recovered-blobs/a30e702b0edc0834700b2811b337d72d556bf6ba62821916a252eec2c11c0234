//! Saved shared endpoints precede, never inherit, intersection qualification.
use super::{IntersectionCurve, invalid, unproven};
use crate::Model;
use nurbs_core::{Result, curve::Curve, curve_surface_agreement as agreement, surface::Surface};
pub fn bind_intersection_ends(
    source: &Model,
    face: usize,
    endpoints: [usize; 2],
    candidate: &IntersectionCurve,
    second: &Surface,
    max_cells: usize,
) -> Result<IntersectionCurve> {
    source.validate()?;
    second.validate()?;
    if !(2..=100000).contains(&max_cells)
        || endpoints[0] == endpoints[1]
        || endpoints.iter().any(|&v| v >= source.vertices.len())
    {
        return Err(invalid(
            "Need distinct shared endpoints and bounded fresh lift work",
        ));
    }
    let support = source
        .faces
        .get(face)
        .ok_or_else(|| invalid("Missing endpoint support face"))?;
    let clamped = |c: &Curve| {
        !c.periodic
            && c.knots[..=c.degree].iter().all(|&x| x == c.domain()[0])
            && c.knots[c.control_points.len()..]
                .iter()
                .all(|&x| x == c.domain()[1])
    };
    candidate.curve.validate()?;
    for pc in &candidate.pcurves {
        pc.validate()?;
    }
    if !clamped(&candidate.curve) || candidate.pcurves.iter().any(|pc| !clamped(pc)) {
        return Err(unproven(
            "Owned-end binding needs original clamped representations",
        ));
    }
    let mut curve = candidate.curve.clone();
    let mut pcurves = candidate.pcurves.clone();
    for (i, vertex) in endpoints.into_iter().enumerate() {
        let uses = source.loops[support.outer]
            .coedges
            .iter()
            .filter(|c| source.edges[c.edge].vertices[usize::from(c.reversed)] == vertex)
            .collect::<Vec<_>>();
        if uses.len() != 1 {
            return Err(unproven(
                "Endpoint needs unique original outer-loop ownership",
            ));
        }
        if !clamped(&uses[0].pcurve) || uses[0].pcurve.control_points[0].len() != 2 {
            return Err(unproven(
                "Owned endpoint chart needs a clamped original boundary use",
            ));
        }
        let cv = if i == 0 {
            0
        } else {
            curve.control_points.len() - 1
        };
        let uv = if i == 0 {
            0
        } else {
            pcurves[0].control_points.len() - 1
        };
        curve.control_points[cv] = source.vertices[vertex].point.to_vec();
        pcurves[0].control_points[uv] = uses[0].pcurve.control_points[0].clone();
    }
    let first = agreement::verify(
        &curve,
        &pcurves[0],
        &support.surface,
        false,
        source.tolerance_mm / 2.,
        max_cells / 2,
    )?;
    if first.status != agreement::Status::WithinTolerance {
        return Err(unproven(
            "Owned endpoint edits fail the full original source lift",
        ));
    }
    let second = agreement::verify(
        &curve,
        &pcurves[1],
        second,
        false,
        source.tolerance_mm / 2.,
        max_cells - first.cells,
    )?;
    if second.status != agreement::Status::WithinTolerance {
        return Err(unproven(
            "Owned endpoint edits fail the full second-support lift",
        ));
    }
    let cells = first.cells + second.cells;
    Ok(IntersectionCurve {
        curve,
        pcurves,
        agreements: [first, second],
        cells,
    })
}
