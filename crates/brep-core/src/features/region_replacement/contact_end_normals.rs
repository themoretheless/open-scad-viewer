//! Necessary endpoint condition for a regular finite G2 replacement fan.
use crate::{TopoId, face_cut::ContactRetention};
use nurbs_core::{Result, curve_differential::Side, interval_eval::Interval as I};
use super::patch_set::{G2_NORMAL_TOLERANCE, MAX_SEW_PATCHES};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndNormalDecision {
    Unresolved,
    /// A necessary normal test only; never G2 or construction admission.
    NoObstructionProven,
    /// Every regular replacement fan under the current seam/patch limits
    /// violates the necessary endpoint triangle inequality.
    ExpandEndRegion,
}
#[derive(Debug)]
pub struct EndNormalDiagnostic {
    pub vertex: TopoId,
    pub source_edge: TopoId,
    pub towards_source_vertex: TopoId,
    pub source_faces: [TopoId; 2],
    pub normals: Vec<[[f64; 2]; 3]>,
    pub unsigned_normal_gap_lower: Option<f64>,
    pub decision: EndNormalDecision,
}
#[derive(Debug)]
pub struct ContactEndNormals {
    pub ends: Vec<EndNormalDiagnostic>,
    pub maximum_new_patches: usize,
    pub seam_normal_tolerance: f64,
    pub maximum_chain_normal_error: f64,
}
impl ContactEndNormals {
    pub fn needs_expansion(&self) -> bool {
        self.ends.iter().any(|e| e.decision == EndNormalDecision::ExpandEndRegion)
    }
}
fn gap_lower(a: [[f64; 2]; 3], b: [[f64; 2]; 3], sign: f64) -> Result<f64> {
    let mut squared = I::point(0.);
    for k in 0..3 {
        let delta = I::new(a[k][0], a[k][1])?
            .sub(I::new(b[k][0], b[k][1])?.mul(I::point(sign))?)?;
        let lower = if delta.lo > 0. { delta.lo } else if delta.hi < 0. { -delta.hi } else { 0. };
        squared = squared.add(I::point(lower).mul(I::point(lower))?)?;
    }
    Ok(squared.lo.max(0.).sqrt().next_down().max(0.))
}
/// Inspect actual saved source-use endpoints with interval shape-operator jets.
/// At a common physical vertex, each regular patch has one continuous normal.
/// A fan of at most P new patches has at most P+1 G2 seams from one retained
/// support to the other. Even allowing the seam tolerance to accumulate, the
/// unsigned source-normal separation cannot exceed (P+1)*normal_tolerance.
/// A strict interval lower witness exceeding that upper bound requires changing
/// the end region. This is conditional on regular G2 patches; it does not rule
/// out singular/non-G2 constructions or authorize any other endpoint geometry.
pub fn inspect_contact_end_normals(retention: &ContactRetention) -> Result<ContactEndNormals> {
    let roots = retention.roots();
    let plan = retention.plan();
    let source = roots.partition().source();
    let edge_identity = roots.branch().edge;
    let edge = source.1.edges.iter().position(|id| *id == edge_identity)
        .ok_or_else(|| super::refused("Original contact edge identity missing"))?;
    let faces = [roots.branch().contacts[0].face, roots.branch().contacts[1].face];
    let maximum = I::point(G2_NORMAL_TOLERANCE)
        .mul(I::point((MAX_SEW_PATCHES + 1) as f64))?.hi;
    let mut ends = Vec::new();
    for (end, vertex) in roots.endpoints().into_iter().enumerate() {
        let mut diagnostic = EndNormalDiagnostic {
            vertex: plan.model.1.vertices[vertex], source_edge: edge_identity,
            towards_source_vertex: source.1.vertices[source.edges[edge].vertices[end]],
            source_faces: faces, normals: vec![], unsigned_normal_gap_lower: None,
            decision: EndNormalDecision::Unresolved,
        };
        for source_face in faces {
            let Some(region) = plan.regions.iter().find(|r|
                roots.model().1.faces[r.source_face] == source_face) else { break; };
            let uses = region.retained_faces.iter().flat_map(|&f| {
                let face = &plan.model.faces[f];
                std::iter::once(face.outer).chain(face.holes.iter().copied())
                    .flat_map(move |w| plan.model.loops[w].coedges.iter().map(move |c| (f, c)))
            }).filter(|(_, c)| plan.model.edges[c.edge].vertices[usize::from(c.reversed)] == vertex)
                .collect::<Vec<_>>();
            if uses.len() != 1 { break; }
            let (face, co) = uses[0];
            let pc = &co.pcurve;
            let n = pc.control_points.len();
            let domain = pc.domain();
            if pc.periodic || pc.knots[..=pc.degree].iter().any(|k| *k != domain[0])
                || pc.knots[n..].iter().any(|k| *k != domain[1])
                || co.trim().is_some_and(|t| t.pcurve_parameter != domain || t.periodic_lift != [[0;2];2]) {
                break;
            }
            let uv = [pc.control_points[0][0], pc.control_points[0][1]];
            let shape = nurbs_core::surface_shape_operator::at(&plan.model.faces[face].surface,
                uv, [Side::Automatic; 2])?;
            let Some(normal) = shape.normal else { break; };
            diagnostic.normals.push(normal);
        }
        if diagnostic.normals.len() == 2 {
            let lower = gap_lower(diagnostic.normals[0], diagnostic.normals[1], 1.)?
                .min(gap_lower(diagnostic.normals[0], diagnostic.normals[1], -1.)?);
            diagnostic.unsigned_normal_gap_lower = Some(lower);
            diagnostic.decision = if lower > maximum {
                EndNormalDecision::ExpandEndRegion
            } else { EndNormalDecision::NoObstructionProven };
        }
        ends.push(diagnostic);
    }
    Ok(ContactEndNormals { ends, maximum_new_patches: MAX_SEW_PATCHES,
        seam_normal_tolerance: G2_NORMAL_TOLERANCE, maximum_chain_normal_error: maximum })
}
