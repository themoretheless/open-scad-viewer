//! Native evidence and application decisions for surface boundary jet matching.
use crate::surface::Surface;

pub enum SeamRegularity {
    NonC1Basis {
        unresolved_intervals: Vec<[f64; 2]>,
    },
    Bernstein {
        domain: [f64; 2],
        spans: usize,
        inspected_cells: usize,
        accepted_cells: usize,
        unresolved_count: usize,
        unresolved_intervals: Vec<[f64; 2]>,
        resource_limit_reached: bool,
    },
}
impl SeamRegularity {
    pub fn certified(&self) -> bool {
        match self {
            Self::NonC1Basis { .. } => false,
            Self::Bernstein {
                unresolved_count,
                resource_limit_reached,
                ..
            } => *unresolved_count == 0 && !resource_limit_reached,
        }
    }
    pub fn unresolved_intervals_mut(&mut self) -> &mut Vec<[f64; 2]> {
        match self {
            Self::NonC1Basis {
                unresolved_intervals,
            }
            | Self::Bernstein {
                unresolved_intervals,
                ..
            } => unresolved_intervals,
        }
    }
}
pub struct SurfaceJetErrorBounds {
    pub position_upper: f64,
    pub first_derivative_upper: f64,
    pub second_derivative_upper: Option<f64>,
    pub mixed_derivative_upper: Option<f64>,
}
impl SurfaceJetErrorBounds {
    pub fn maximum(&self) -> f64 {
        self.position_upper
            .max(self.first_derivative_upper)
            .max(self.second_derivative_upper.unwrap_or(0.))
            .max(self.mixed_derivative_upper.unwrap_or(0.))
    }
}
pub enum SurfaceJetDecisionReason {
    UnprovenRegularity,
    UnprovenTangentialSmoothness,
    DeviationExceedsBudget,
    Accepted,
}
pub struct SurfaceJetDecision {
    pub accepted: bool,
    pub max_error: f64,
    pub error_upper: f64,
    pub tangential_smoothness_certified: bool,
    pub reason: SurfaceJetDecisionReason,
}
pub struct SurfaceJetReport {
    pub continuity_order: usize,
    pub normal_scale: f64,
    pub reference_regularity: SeamRegularity,
    pub edited_regularity: SeamRegularity,
    pub error_bounds: SurfaceJetErrorBounds,
    pub reversed: bool,
    pub decision: Option<SurfaceJetDecision>,
}
impl SurfaceJetReport {
    pub fn regularity_certified(&self) -> bool {
        self.reference_regularity.certified() && self.edited_regularity.certified()
    }
}
pub struct SurfaceJetMatch {
    pub surface: Surface,
    pub report: SurfaceJetReport,
}
