//! General NURBS surface intersection graphs built from typed query reports.
use crate::coverage_verifier::CoverageAudit;
use cad_predicates::{ToleranceContext, ToleranceSpecIdentity};
use nurbs_core::{
    Error, Result,
    intersection::ContactClass,
    ss_intersection::{SurfaceSurfaceComponent, SurfaceSurfaceIntersection},
};
#[cfg(feature = "codec")]
mod serialization;
#[cfg(feature = "codec")]
pub use serialization::{branch_graph_from_ss_report, rational_traces_from_ss_report};
fn refuse(message: &str) -> Error {
    Error::new("BREP_NURBS_SS_GENERAL_REFUSED", message)
}
pub const GENERAL_SS_CAPABILITY: &str = "nurbs-ss/1";
#[derive(Clone, Debug, PartialEq)]
pub enum GeneralBranchKind {
    Curve,
    Overlap,
    Unknown(String),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraceCorrespondence {
    ExactAffine,
    IntervalCertified,
}
#[derive(Clone, Debug, PartialEq)]
pub enum GeneralPcurve {
    Absent,
    Line {
        endpoints: [[f64; 2]; 2],
        correspondence: TraceCorrespondence,
    },
    RationalTrace {
        endpoints: [[f64; 2]; 2],
    },
    Point([f64; 2]),
}
#[derive(Clone, Debug, PartialEq)]
pub struct GeneralSsBranch {
    pub id: usize,
    pub kind: GeneralBranchKind,
    pub closed: bool,
    pub contact_class: Option<ContactClass>,
    pub junction: bool,
    pub material_sides: [i8; 2],
    pub coedge_trim: Option<brep_topology::CoedgeTrim>,
    pub pcurve_first: GeneralPcurve,
    pub pcurve_second: GeneralPcurve,
    pub uv_samples: [Option<Vec<[f64; 2]>>; 2],
}
#[derive(Clone, Debug, PartialEq)]
pub struct GeneralBranchGraph {
    pub components: Vec<GeneralSsBranch>,
    pub context: ToleranceSpecIdentity,
    pub complete: bool,
    pub permits_topology_authorship: bool,
    pub boolean_mutation_authority: bool,
    pub missed_branch_proof: bool,
}
impl GeneralBranchGraph {
    pub fn permits_topology_authorship(&self) -> bool {
        self.permits_topology_authorship
            && self.complete
            && self.missed_branch_proof
            && !self.boolean_mutation_authority
            && self.components.iter().all(|c| {
                matches!(
                    c.contact_class,
                    Some(
                        ContactClass::Transverse
                            | ContactClass::Coincident
                            | ContactClass::OddTangency
                            | ContactClass::EvenTangency
                            | ContactClass::Boundary
                    )
                )
            })
    }
}
/// Consume the native NURBS report without serializing its geometry or evidence.
pub fn branch_graph_from_surface_intersection(
    report: &SurfaceSurfaceIntersection,
    context: &ToleranceContext,
) -> Result<GeneralBranchGraph> {
    if report.tolerance.spec_identity() != context.spec_identity() {
        return Err(refuse("General SS report ToleranceContext mismatch"));
    }
    let components = report
        .components
        .iter()
        .enumerate()
        .map(|(id, component)| {
            use GeneralPcurve::*;
            let (kind, closed, contact, junction, trim, pcurves, samples) = match component {
                SurfaceSurfaceComponent::Exact(b) => (
                    GeneralBranchKind::Curve,
                    false,
                    ContactClass::Transverse,
                    b.junction.unwrap_or(false),
                    b.coedge_trim,
                    [
                        Line {
                            endpoints: b.first_trace,
                            correspondence: TraceCorrespondence::ExactAffine,
                        },
                        Line {
                            endpoints: b.second_trace,
                            correspondence: TraceCorrespondence::ExactAffine,
                        },
                    ],
                    b.samples.as_slice(),
                ),
                SurfaceSurfaceComponent::Continued(b) => (
                    GeneralBranchKind::Curve,
                    b.closed,
                    b.contact,
                    b.junction,
                    b.coedge_trim,
                    [
                        RationalTrace {
                            endpoints: [b.first_uv, b.last_uv],
                        },
                        RationalTrace {
                            endpoints: [b.first_st, b.last_st],
                        },
                    ],
                    b.samples.as_slice(),
                ),
                SurfaceSurfaceComponent::Tangency(b) => (
                    GeneralBranchKind::Curve,
                    false,
                    b.contact,
                    b.junction,
                    b.coedge_trim,
                    [Point(b.sample.uv_first), Point(b.sample.uv_second)],
                    std::slice::from_ref(&b.sample),
                ),
                SurfaceSurfaceComponent::Overlap(b) => (
                    GeneralBranchKind::Overlap,
                    false,
                    ContactClass::Coincident,
                    false,
                    b.coedge_trim,
                    [Absent, Absent],
                    &[][..],
                ),
            };
            let [pcurve_first, pcurve_second] = pcurves;
            let is_overlap = kind == GeneralBranchKind::Overlap;
            GeneralSsBranch {
                id,
                kind,
                closed,
                contact_class: Some(contact),
                junction,
                material_sides: [1, -1],
                coedge_trim: Some(trim),
                pcurve_first,
                pcurve_second,
                uv_samples: [
                    if is_overlap {
                        None
                    } else {
                        Some(samples.iter().map(|s| s.uv_first).collect())
                    },
                    if is_overlap {
                        None
                    } else {
                        Some(samples.iter().map(|s| s.uv_second).collect())
                    },
                ],
            }
        })
        .collect();
    let complete = report.complete();
    Ok(GeneralBranchGraph {
        components,
        context: context.spec_identity(),
        complete,
        permits_topology_authorship: complete,
        boolean_mutation_authority: false,
        missed_branch_proof: complete,
    })
}
/// Independent coverage audit for general SS BranchGraph consumers.
pub fn verify_general_ss_branch_graph(
    graph: &GeneralBranchGraph,
    context: &ToleranceContext,
) -> Result<CoverageAudit> {
    if graph.context != context.spec_identity() {
        return Err(refuse("General SS BranchGraph ToleranceContext mismatch"));
    }
    if graph.boolean_mutation_authority {
        return Err(refuse(
            "General SS BranchGraph must not carry Boolean mutation authority",
        ));
    }
    if graph.complete && !graph.missed_branch_proof {
        return Err(refuse(
            "Complete general SS BranchGraph requires missed-branch proof",
        ));
    }
    if !graph.complete && graph.permits_topology_authorship {
        return Err(refuse(
            "Incomplete general SS BranchGraph cannot permit topology authorship",
        ));
    }
    Ok(CoverageAudit {
        complete: graph.complete,
        component_count: graph.components.len(),
        unresolved_count: if graph.complete { 0 } else { 1 },
        notes: vec![
            "general_ss_branch_graph_verified",
            "no_graph_patch_iso_fixture",
            "boolean_mutation_deferred",
        ],
    })
}

/// Lift native UV samples and exact line traces into the retained arrangement.
pub fn rational_traces_from_branch_graph(
    graph: &GeneralBranchGraph,
) -> Vec<crate::uv_arrangement::LiftedUvPrimitive> {
    rational_traces_from_branches(&graph.components)
}
fn rational_traces_from_branches(
    components: &[GeneralSsBranch],
) -> Vec<crate::uv_arrangement::LiftedUvPrimitive> {
    use crate::uv_arrangement::{LiftedUvGeometry, LiftedUvPrimitive};
    let mut out = Vec::new();
    for component in components {
        for (support, pcurve) in [&component.pcurve_first, &component.pcurve_second]
            .into_iter()
            .enumerate()
        {
            let samples = if let Some(samples) = &component.uv_samples[support] {
                samples.clone()
            } else if let GeneralPcurve::Line { endpoints, .. } = pcurve {
                endpoints.to_vec()
            } else {
                vec![]
            };
            if samples.len() < 2 {
                continue;
            }
            out.push(LiftedUvPrimitive {
                edge_id: component.id * 2 + support,
                geometry: LiftedUvGeometry::RationalCurvedTrace {
                    samples,
                    closed: component.closed,
                    correspondence: if matches!(
                        pcurve,
                        GeneralPcurve::Line {
                            correspondence: TraceCorrespondence::ExactAffine,
                            ..
                        }
                    ) {
                        "exact_affine"
                    } else {
                        "interval_certified"
                    },
                    overlap: component.kind == GeneralBranchKind::Overlap,
                    singular: component.contact_class == Some(ContactClass::PoleOrSingular),
                },
            });
        }
    }
    out
}
#[cfg(test)]
mod tests;
