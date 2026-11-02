//! Complete explicit mixed C0/G1/G2 edge coverage of an actual native model.
//! Global embedding/material roles remain independent constructor obligations.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    C0,
    G1,
    G2,
}
impl Class {
    pub fn order(self) -> usize {
        match self {
            Self::C0 => 0,
            Self::G1 => 1,
            Self::G2 => 2,
        }
    }
}
#[derive(Clone, Copy)]
pub struct Limits {
    pub incidence_steps: usize,
    pub exact_work: u64,
    pub agreement_work: u64,
    pub regularity_cells: usize,
}
pub struct Report {
    pub certified: bool,
    pub classes: Vec<Class>,
    pub steps: usize,
    pub smooth_edges: Vec<usize>,
    pub c0_edges: Vec<usize>,
    pub uncertain_edge: Option<usize>,
    pub agreement: Option<crate::boundary_agreement::ExactReport>,
    pub regularity: Option<crate::boundary_embedding::regularity::Report>,
    pub vertex_links_complete: bool,
    pub proof: Option<ProjectiveSeamCollectionReport>,
    pub exact_work: u64,
    pub reason: &'static str,
}
fn step(out: &mut Report, max: usize) -> bool {
    if out.steps == max {
        out.reason = "classified-boundary-incidence-work-limit";
        false
    } else {
        out.steps += 1;
        true
    }
}
/// Every actual edge requires a class and exactly two owned uses. C0 still
/// needs exact world/UV agreement, closed native vertex links and regular face
/// interiors. Smooth edges require fresh exact original rational tensor jets
/// at THEIR requested order. No failed G2 request is downgraded to G1 or C0.
/// One coefficient ledger covers every smooth edge, including closed seams.
pub fn inspect(model: &Model, classes: &[Class], limits: Limits) -> Result<Report> {
    model.validate_boundary_diagnostic_inputs()?;
    require(
        classes.len() == model.edges.len()
            && !classes.is_empty()
            && limits.incidence_steps <= 1000000
            && limits.exact_work <= continuity::MAX_PROJECTIVE_SEAM_COLLECTION_WORK
            && limits.agreement_work <= 1000000
            && limits.regularity_cells <= 100000,
        "Classify every actual edge with bounded shared work",
    )?;
    let mut out = Report {
        certified: false,
        classes: classes.to_vec(),
        steps: 0,
        smooth_edges: Vec::new(),
        c0_edges: Vec::new(),
        uncertain_edge: None,
        agreement: None,
        regularity: None,
        vertex_links_complete: false,
        proof: None,
        exact_work: 0,
        reason: "classified-boundary-unproven",
    };
    let mut uses = vec![Vec::new(); model.edges.len()];
    for (face, f) in model.faces.iter().enumerate() {
        if !step(&mut out, limits.incidence_steps) {
            return Ok(out);
        }
        for wire in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
            for c in &model.loops[wire].coedges {
                if !step(&mut out, limits.incidence_steps) {
                    return Ok(out);
                }
                let label = boundary(&f.surface, &c.pcurve, Axis::Profile)
                    .or_else(|| boundary(&f.surface, &c.pcurve, Axis::Station));
                let direction = label.map(|l| {
                    let free = usize::from(l.starts_with('u'));
                    (c.pcurve.control_points.last().unwrap()[free]
                        > c.pcurve.control_points[0][free])
                        != c.reversed
                });
                uses[c.edge].push((face, label, direction));
            }
        }
    }
    let mut patches = Vec::new();
    let mut seams = Vec::new();
    for (edge, owned) in uses.iter().enumerate() {
        out.uncertain_edge = Some(edge);
        if !step(&mut out, limits.incidence_steps) {
            return Ok(out);
        }
        if owned.len() != 2 {
            out.reason = "classified-boundary-two-uses-unproven";
            return Ok(out);
        }
        if classes[edge] == Class::C0 {
            out.c0_edges.push(edge);
            continue;
        }
        let [(a, Some(ab), Some(ad)), (b, Some(bb), Some(bd))] = owned.as_slice() else {
            out.reason = "classified-boundary-natural-carrier-unproven";
            return Ok(out);
        };
        let left = model.faces[*a].surface.clone();
        let mut right = model.faces[*b].surface.clone();
        if ad != bd && !reverse_along(&mut right, bb) {
            out.reason = "classified-boundary-clock-unproven";
            return Ok(out);
        }
        let start = patches.len();
        patches.push(left);
        patches.push(right);
        seams.push(ProjectiveSeam {
            patches: [start, start + 1],
            boundaries: [(*ab).into(), (*bb).into()],
            order: classes[edge].order(),
            normal_scale: 1.,
        });
        out.smooth_edges.push(edge);
    }
    out.uncertain_edge = None;
    if limits.agreement_work == 0 {
        out.reason = "classified-boundary-agreement-work-limit";
        return Ok(out);
    }
    let agreement = crate::boundary_agreement::verify_exact(model, limits.agreement_work)?;
    let equal = agreement.all_equal && agreement.all_joins_exact;
    out.agreement = Some(agreement);
    if !equal {
        out.reason = "classified-boundary-position-unproven";
        return Ok(out);
    }
    out.vertex_links_complete = vertex_links(model);
    if !out.vertex_links_complete {
        out.reason = "classified-boundary-vertex-links-unproven";
        return Ok(out);
    }
    let regularity =
        crate::boundary_embedding::regularity::inspect(model, limits.regularity_cells)?;
    let regular = regularity.all_faces_regular;
    out.regularity = Some(regularity);
    if !regular {
        out.reason = "classified-boundary-face-regularity-unproven";
        return Ok(out);
    }
    if !seams.is_empty() {
        let proof = continuity::inspect_projective_seam_collection_with_rational_scale_proposals(
            &patches,
            &seams,
            limits.exact_work,
        )?;
        out.exact_work = proof.exact_work;
        let proven = proof.certified;
        out.proof = Some(proof);
        if !proven {
            out.reason = "classified-boundary-exact-jets-unproven";
            return Ok(out);
        }
    }
    out.certified = true;
    out.reason = "complete-explicit-actual-boundary-classes-qualified";
    Ok(out)
}

#[cfg(test)]
mod tests;
