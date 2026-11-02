//! Sew a generated polygonal graph using source-owned prepared contacts.
//! Root isolation/splitting and whole-body admission remain separate gates.
use super::{
    CentralGraph, ContactCut, ContactReplacement, RemovalAnchor, refused, trim_and_replace_contacts,
};
use crate::{Model, face_cut};
use nurbs_core::Result;

pub struct GraphContactCut<'a> {
    pub face: usize,
    pub endpoints: [usize; 2],
    /// Already isolated common 3D roots and original support-local pcurves.
    /// All cuts together follow the graph's cyclic pcurve order.
    pub segments: &'a [face_cut::Segment],
    pub remove: RemovalAnchor,
    pub smooth: bool,
}
/// Partition original support faces and sew the actual graph into one closed
/// replacement, deriving replacement-side pcurves from the generated graph.
/// All steps are atomic. The existing trimming/sewing kernel rechecks actual
/// saved geometry; sampled residuals or prior constructor flags are ignored.
///
/// This returns C0 closure and requested smooth-edge roles, not G2, curvature,
/// radius or global embedding admission. The complete final model still needs
/// fresh seam/face/global checks. Prepared source roots are mandatory; this
/// function does not discover missing contact endpoints or infer retained sides.
pub fn trim_and_replace_graph_contacts(
    source: &Model,
    cuts: &[GraphContactCut<'_>],
    graph: &CentralGraph,
    reversed: bool,
    tolerance_uv: f64,
    max_cells: usize,
    max_exact: u64,
) -> Result<ContactReplacement> {
    let mapped = map_contacts(cuts, graph)?;
    trim_and_replace_contacts(
        source,
        &mapped,
        &graph.surface,
        reversed,
        tolerance_uv,
        max_cells,
        max_exact,
    )
}

/// Atomic graph trim/sew with fresh qualification on the SAVED body. Proposal
/// residuals never authorize admission. All requested smooth seams must pass
/// whole-curve G2; every retained face must pass |principal curvature| <=10/mm.
/// Explicitly sharp end seams stay outside G2 scope, as recorded in contacts().
/// This is not a radius, thickness or global self-intersection certificate.
pub fn trim_and_replace_graph_contacts_g2(
    source: &Model,
    cuts: &[GraphContactCut<'_>],
    graph: &CentralGraph,
    reversed: bool,
    tolerance_uv: f64,
    limits: super::ContactQualificationLimits,
) -> Result<super::QualifiedContactBody> {
    let mapped = map_contacts(cuts, graph)?;
    super::trim_and_replace_contacts_g2(
        source,
        &mapped,
        &graph.surface,
        reversed,
        tolerance_uv,
        limits,
    )
}

fn map_contacts<'a>(
    cuts: &[GraphContactCut<'a>],
    graph: &'a CentralGraph,
) -> Result<Vec<ContactCut<'a>>> {
    if !(3..=16).contains(&cuts.len()) || !(3..=256).contains(&graph.pcurves.len()) {
        return Err(refused("Need bounded polygonal graph contacts"));
    }
    let count = cuts
        .iter()
        .try_fold(0usize, |n, c| n.checked_add(c.segments.len()))
        .ok_or_else(|| refused("Graph contact count overflow"))?;
    if count != graph.pcurves.len() || cuts.iter().any(|c| c.segments.is_empty()) {
        return Err(refused(
            "Graph pcurves must cover every original contact segment exactly once",
        ));
    }
    let mut start = 0;
    let mut mapped = Vec::new();
    for cut in cuts {
        let end = start + cut.segments.len();
        mapped.push(ContactCut {
            face: cut.face,
            endpoints: cut.endpoints,
            segments: cut.segments,
            replacement_pcurves: &graph.pcurves[start..end],
            remove: cut.remove,
            smooth: cut.smooth,
        });
        start = end;
    }
    Ok(mapped)
}
