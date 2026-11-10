//! Joint admission from immutable, constructor-owned sweep evidence.
//! A distance bound, retained Solid, or local differential proof alone cannot
//! manufacture this token. No caller-supplied report is accepted by this API.
use super::{CertifiedBody, ClosedBody, RationalClosedBody, SmoothClosedBody};
use crate::Model;
use nurbs_core::progressive_sweep::IdealVolumeReport;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    InvalidOrder,
    ContinuousBound,
    RetainedGlobal,
    OriginalGlobal,
    OriginalG1,
    OriginalG2,
    RetainedG1,
    RetainedG2,
}
pub struct Certificate<'a> {
    model: &'a Model,
    error_upper: f64,
    order: usize,
}
impl<'a> Certificate<'a> {
    pub fn model(&self) -> &'a Model {
        self.model
    }
    pub fn boundary_error_upper(&self) -> f64 {
        self.error_upper
    }
    pub fn order(&self) -> usize {
        self.order
    }
}
fn global(p: Option<&IdealVolumeReport>) -> bool {
    p.is_some_and(|p| {
        p.embedding_certified && p.regularity_certified && p.parameter_topology_preserved
    })
}
struct Premises {
    bound: bool,
    within: bool,
    error: Option<f64>,
    retained: bool,
    original: bool,
    source_g1: bool,
    source_g2: bool,
    g1: bool,
    g2: bool,
}
fn admit(model: &Model, p: Premises, order: usize) -> Result<Certificate<'_>, Refusal> {
    if order > 2 {
        return Err(Refusal::InvalidOrder);
    }
    let Some(error) = p.error.filter(|e| e.is_finite() && *e >= 0.) else {
        return Err(Refusal::ContinuousBound);
    };
    if !p.bound || !p.within {
        return Err(Refusal::ContinuousBound);
    }
    if !p.retained {
        return Err(Refusal::RetainedGlobal);
    }
    if !p.original {
        return Err(Refusal::OriginalGlobal);
    }
    if order >= 1 && !p.source_g1 {
        return Err(Refusal::OriginalG1);
    }
    if order == 2 && !p.source_g2 {
        return Err(Refusal::OriginalG2);
    }
    if order >= 1 && !p.g1 {
        return Err(Refusal::RetainedG1);
    }
    if order == 2 && !p.g2 {
        return Err(Refusal::RetainedG2);
    }
    Ok(Certificate {
        model,
        error_upper: error,
        order,
    })
}
impl CertifiedBody {
    pub fn joint_certificate(&self, order: usize) -> Result<Certificate<'_>, Refusal> {
        let r = self.report();
        admit(
            self.model(),
            Premises {
                bound: r.continuous_bound,
                within: r.within_budget,
                error: r.boundary_error_upper,
                retained: r.certified
                    && r.regularity
                        .as_ref()
                        .is_some_and(|p| p.all_faces_regular_modulo_poles)
                    && r.volume.as_ref().is_some_and(|p| p.proven),
                original: global(r.ideal_volume.as_ref()),
                source_g1: false,
                source_g2: false,
                g1: false,
                g2: false,
            },
            order,
        )
    }
}
impl ClosedBody {
    pub fn joint_certificate(&self, order: usize) -> Result<Certificate<'_>, Refusal> {
        let r = self.report();
        admit(
            self.model(),
            Premises {
                bound: r.continuous_bound,
                within: r.within_budget,
                error: r.boundary_error_upper,
                retained: r.certified
                    && r.regularity
                        .as_ref()
                        .is_some_and(|p| p.all_faces_regular_modulo_poles)
                    && r.volume.as_ref().is_some_and(|p| p.proven),
                original: r.material.certified && global(r.ideal_volume.as_ref()),
                source_g1: false,
                source_g2: false,
                g1: false,
                g2: false,
            },
            order,
        )
    }
}
impl SmoothClosedBody {
    pub fn joint_certificate(&self, order: usize) -> Result<Certificate<'_>, Refusal> {
        let r = self.report();
        admit(
            self.model(),
            Premises {
                bound: r.continuous_bound,
                within: r.within_budget,
                error: r.boundary_error_upper,
                retained: r.certified
                    && r.topology_preserved
                    && r.shell_roles_certified
                    && r.binding.as_ref().is_some_and(|p| p.certified)
                    && global(r.volume.as_ref()),
                original: r.original.certified
                    && r.original.material.certified
                    && global(r.original.volume.as_ref()),
                source_g1: r.original.g1_certified(),
                source_g2: r.original.g2_certified(),
                g1: r.smoothness.as_ref().is_some_and(|p| p.g1_certified),
                g2: r.smoothness.as_ref().is_some_and(|p| p.g2_certified),
            },
            order,
        )
    }
}
impl RationalClosedBody {
    pub fn joint_certificate(&self, order: usize) -> Result<Certificate<'_>, Refusal> {
        let r = self.report();
        let exact = r.certified
            && r.source_boundary_exact
            && r.topology_preserved
            && r.binding.as_ref().is_some_and(|p| p.certified)
            && global(r.volume.as_ref());
        admit(
            self.model(),
            Premises {
                bound: r.source_boundary_exact,
                within: r.certified,
                error: r.boundary_error_upper,
                retained: exact,
                original: exact,
                source_g1: r.source_g1,
                source_g2: r.source_g2,
                g1: r.smoothness.as_ref().is_some_and(|p| p.g1_certified),
                g2: r.smoothness.as_ref().is_some_and(|p| p.g2_certified),
            },
            order,
        )
    }
}
