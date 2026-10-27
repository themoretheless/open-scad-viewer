//! Source-bound uniform pole-plane limits. These do not assert regular quotient
//! charts, G2 curvature, or replace the ordinary seam collection's pole refusal.
use crate::{
    source_collapsed_boundary::CollapsedBoundary, source_seam_tangency::Carrier,
    source_shell_incidence::Address, source_volume::Body,
};
use nurbs_core::{Error, Result, factored_surface_normals as normals, surface_join::Boundary};
use std::collections::{BTreeMap, BTreeSet};
pub struct PolePlane<'a> {
    pole: &'a CollapsedBoundary,
    limit: normals::PlaneLimit<'a>,
}
impl PolePlane<'_> {
    pub fn pole(&self) -> &CollapsedBoundary {
        self.pole
    }
    pub fn limit(&self) -> &normals::PlaneLimit<'_> {
        &self.limit
    }
}
pub struct Report<'a> {
    pub certificate: Option<PolePlane<'a>>,
    pub work: u64,
    pub reason: &'static str,
}
/// Recompute from the actual unchanged original collapsed restriction. Natural
/// U/V boundary membership is checked on original positive-weight controls;
/// whole-boundary proof also covers retained root-ended subintervals.
pub fn qualify(pole: &CollapsedBoundary, max_work: u64) -> Result<Report<'_>> {
    if !(1..=100000000).contains(&max_work) {
        return Err(Error::new(
            "BREP_SOURCE_POLE_PLANE",
            "Choose bounded pole-plane work",
        ));
    }
    let surface = pole.source().surface();
    let curve = pole.source().curve();
    let domains = [
        [
            surface.knots_u[surface.degree_u],
            surface.knots_u[surface.control_points.len()],
        ],
        [
            surface.knots_v[surface.degree_v],
            surface.knots_v[surface.control_points[0].len()],
        ],
    ];
    let mut out = Report {
        certificate: None,
        work: 0,
        reason: "source-pole-plane-natural-boundary-unproven",
    };
    let mut boundary = None;
    for (axis, bounds) in domains.iter().enumerate() {
        for end in 0..2 {
            if curve.control_points.iter().all(|p| p[axis] == bounds[end]) {
                boundary = Some(match (axis, end) {
                    (0, 0) => Boundary::UMin,
                    (0, _) => Boundary::UMax,
                    (1, 0) => Boundary::VMin,
                    _ => Boundary::VMax,
                });
                break;
            }
        }
        if boundary.is_some() {
            break;
        }
    }
    let Some(boundary) = boundary else {
        return Ok(out);
    };
    let prepared = normals::prepare_plane_limit(surface, boundary, max_work)?;
    out.work = prepared.work;
    out.reason = prepared.reason;
    let Some(limit) = prepared.certificate else {
        return Ok(out);
    };
    if limit.point() != pole.point() {
        out.reason = "source-pole-plane-point-ownership-unproven";
        return Ok(out);
    }
    out.certificate = Some(PolePlane { pole, limit });
    out.reason = "source-pole-uniform-plane-limit-qualified";
    Ok(out)
}
pub struct BodyPlaneLimits<'a, E: Carrier> {
    body: &'a Body<E>,
    poles: Vec<(Address, PolePlane<'a>)>,
    vertices: Vec<usize>,
}
impl<E: Carrier> BodyPlaneLimits<'_, E> {
    pub fn body(&self) -> &Body<E> {
        self.body
    }
    pub fn poles(&self) -> &[(Address, PolePlane<'_>)] {
        &self.poles
    }
    pub fn vertices(&self) -> &[usize] {
        &self.vertices
    }
}
pub struct BodyReport<'a, E: Carrier> {
    pub certificate: Option<BodyPlaneLimits<'a, E>>,
    pub work: u64,
    pub completed_poles: usize,
    pub comparisons: usize,
    pub uncertain_vertex: Option<usize>,
    pub reason: &'static str,
}
/// Prove one exact plane limit at every fully polar owned vertex. EVERY incident
/// face must contribute its original collapsed boundary; incomplete fans refuse.
/// All individual and cross-chart identities share one exact-work allowance.
/// This is a plane-limit token, never an assertion of quotient regularity/G2.
pub fn qualify_body_plane_limits<E: Carrier>(
    body: &Body<E>,
    max_work: u64,
) -> Result<BodyReport<'_, E>> {
    if !(1..=100000000).contains(&max_work) {
        return Err(Error::new(
            "BREP_SOURCE_POLE_PLANE",
            "Choose bounded whole-body pole-plane work",
        ));
    }
    let shell = body.geometry().shell();
    if shell.poles().is_empty() {
        return Err(Error::new(
            "BREP_SOURCE_POLE_PLANE",
            "Select a body with owned collapsed strata",
        ));
    }
    let mut out = BodyReport {
        certificate: None,
        work: 0,
        completed_poles: 0,
        comparisons: 0,
        uncertain_vertex: None,
        reason: "source-pole-plane-fan-work-limit",
    };
    let mut proofs = Vec::new();
    let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (address, pole) in shell.poles() {
        let ids = shell.vertices()[address.face][address.wire][address.edge];
        out.uncertain_vertex = Some(ids[0]);
        if ids[0] != ids[1] {
            out.reason = "source-pole-plane-vertex-ownership-unproven";
            return Ok(out);
        }
        if out.work == max_work {
            return Ok(out);
        }
        let proof = qualify(pole, max_work - out.work)?;
        out.work += proof.work;
        let Some(proof) = proof.certificate else {
            out.reason = proof.reason;
            return Ok(out);
        };
        if shell.faces()[address.face]
            .iter()
            .flat_map(|w| w.edges())
            .any(|f| f.surface() != pole.source().surface())
        {
            out.reason = "source-pole-plane-face-source-unproven";
            return Ok(out);
        }
        groups.entry(ids[0]).or_default().push(proofs.len());
        proofs.push((*address, proof));
        out.completed_poles += 1;
    }
    // One incidence pass; do not rescan the entire shell for every pole.
    let mut incident_faces = groups
        .keys()
        .map(|&v| (v, BTreeSet::new()))
        .collect::<BTreeMap<_, _>>();
    for (face, wires) in shell.vertices().iter().enumerate() {
        for pair in wires.iter().flatten() {
            for vertex in pair {
                if let Some(faces) = incident_faces.get_mut(vertex) {
                    faces.insert(face);
                }
            }
        }
    }
    let mut vertices = Vec::new();
    for (vertex, indices) in groups {
        out.uncertain_vertex = Some(vertex);
        let incident = &incident_faces[&vertex];
        let covered = indices
            .iter()
            .map(|&i| proofs[i].0.face)
            .collect::<BTreeSet<_>>();
        if incident != &covered {
            out.reason = "source-pole-plane-fan-uncovered-face";
            return Ok(out);
        }
        for &index in &indices[1..] {
            if out.work == max_work {
                out.reason = "source-pole-plane-fan-work-limit";
                return Ok(out);
            }
            if proofs[index].1.pole.point() != proofs[indices[0]].1.pole.point() {
                out.reason = "source-pole-plane-fan-point-unproven";
                return Ok(out);
            }
            let pair = normals::compare_plane_limits(
                &proofs[indices[0]].1.limit,
                &proofs[index].1.limit,
                max_work - out.work,
            )?;
            out.work += pair.work;
            out.comparisons += 1;
            if pair.same_plane != Some(true) {
                out.reason = "source-pole-plane-fan-common-plane-unproven";
                return Ok(out);
            }
        }
        vertices.push(vertex);
    }
    out.certificate = Some(BodyPlaneLimits {
        body,
        poles: proofs,
        vertices,
    });
    out.uncertain_vertex = None;
    out.reason = "source-body-uniform-pole-plane-limits-qualified";
    Ok(out)
}
#[cfg(test)]
mod tests;
