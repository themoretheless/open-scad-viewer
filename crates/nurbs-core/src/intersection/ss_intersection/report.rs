//! Native surface intersection report, component classification and junctions.
use super::{
    ContactClass, ContinuationSample, ContinuedBranch, ExactBranch, OverlapRegion,
    UnresolvedSurfaceIntersection, branches, junctions,
};
use cad_predicates::ToleranceContext;

pub struct TangencyBranch {
    pub contact: ContactClass,
    pub sample: ContinuationSample,
    pub geometry_enclosure: [[f64; 2]; 3],
    pub coedge_trim: brep_topology::CoedgeTrim,
    pub junction: bool,
}
pub enum SurfaceSurfaceComponent {
    Exact(ExactBranch),
    Overlap(OverlapRegion),
    Continued(ContinuedBranch),
    Tangency(TangencyBranch),
}
pub struct SurfaceSurfaceIntersection {
    pub components: Vec<SurfaceSurfaceComponent>,
    pub unresolved: Vec<UnresolvedSurfaceIntersection>,
    pub boxes_visited: usize,
    pub bernstein_excluded: usize,
    pub krawczyk_isolated: usize,
    pub tolerance: ToleranceContext,
}
impl SurfaceSurfaceIntersection {
    pub fn complete(&self) -> bool {
        self.unresolved.is_empty()
    }

    pub fn summary(&self) -> branches::Summary {
        use branches::{BranchDescriptor, BranchKind, Contact};
        let descriptors = self
            .components
            .iter()
            .map(|component| {
                let (kind, contact, closed) = match component {
                    SurfaceSurfaceComponent::Exact(_) => {
                        (BranchKind::Curve, ContactClass::Transverse, false)
                    }
                    SurfaceSurfaceComponent::Overlap(_) => {
                        (BranchKind::Overlap, ContactClass::Coincident, false)
                    }
                    SurfaceSurfaceComponent::Continued(branch) => {
                        (BranchKind::Curve, branch.contact, branch.closed)
                    }
                    SurfaceSurfaceComponent::Tangency(branch) => {
                        (BranchKind::Curve, branch.contact, false)
                    }
                };
                BranchDescriptor {
                    kind,
                    closed,
                    contact: match contact {
                        ContactClass::Transverse
                        | ContactClass::Coincident
                        | ContactClass::Boundary
                        | ContactClass::OddTangency
                        | ContactClass::EvenTangency => Contact::Admitted,
                        ContactClass::PoleOrSingular => Contact::Singular,
                        _ => Contact::Unknown,
                    },
                }
            })
            .collect::<Vec<_>>();
        branches::summarize(&descriptors, self.complete())
    }
}
pub(super) fn mark_junctions(components: &mut [SurfaceSurfaceComponent], floor: f64) {
    let samples = components
        .iter()
        .enumerate()
        .flat_map(|(index, component)| {
            let samples: &[ContinuationSample] = match component {
                SurfaceSurfaceComponent::Exact(branch) => &branch.samples,
                SurfaceSurfaceComponent::Continued(branch) => &branch.samples,
                SurfaceSurfaceComponent::Tangency(branch) => std::slice::from_ref(&branch.sample),
                SurfaceSurfaceComponent::Overlap(_) => &[],
            };
            samples.iter().map(move |sample| (index, sample.point))
        })
        .collect::<Vec<_>>();
    for index in junctions::junction_components(&samples, floor) {
        match &mut components[index] {
            SurfaceSurfaceComponent::Exact(branch) => branch.junction = Some(true),
            SurfaceSurfaceComponent::Continued(branch) => branch.junction = true,
            SurfaceSurfaceComponent::Tangency(branch) => branch.junction = true,
            SurfaceSurfaceComponent::Overlap(_) => {}
        }
    }
}
