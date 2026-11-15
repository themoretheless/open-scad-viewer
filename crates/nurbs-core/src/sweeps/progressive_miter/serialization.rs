//! Shared full miter diagnostics for level and owned-body transports.
use super::Report;
use value_codec::{Serialize, Value, json};

impl Serialize for Report {
    fn to_value(&self) -> Value {
        json!({
            "accepted":self.accepted,
            "phaseResolved":self.phase_resolved,
            "frameTransportCertified":self.frame_transport.certified,
            "frameTransportReason":self.frame_transport.reason,
            "steps":self.steps,
            "sections":self.sections,
            "stations":self.stations,
            "sampledControlDeviation":self.error.sampled_control_deviation,
            "continuousErrorUpper":self.error.continuous_upper,
            "certifiedErrorUpper":self.error.certified_upper,
            "endpointContourErrorUpper":self.error.endpoint_contour_upper,
            "errorCertificateCells":self.error.certificate_cells,
            "errorCertificateReason":self.error.certificate_reason,
            "profileRegularityCertified":self.regularity.profile_certified,
            "wallRegularityCertified":self.regularity.wall_certified,
            "regularityCells":self.regularity.cells,
            "unresolvedWallPatches":self.regularity.unresolved_wall_patches,
            "affineLawsApplied":self.applied.affine_laws,
            "authoredFramesApplied":self.applied.authored_frames,
            "orientationGuideApplied":self.applied.orientation_guide,
            "continuousErrorMethod":if self.applied.orientation_guide && self.applied.authored_frames {
                "interval-authored-axis-guide-frame-interpolation"
            } else if self.applied.orientation_guide {
                "interval-guide-frame-interpolation"
            } else if self.applied.authored_frames {
                "interval-authored-frame-interpolation"
            } else if self.applied.affine_laws {
                "interval-affine-law-interpolation"
            } else {
                "rational-law-derivative-interpolation-real-arithmetic"
            },
            "budget":self.budget,
            "closedPath":self.closed_path,
            "holonomyCorrectionRadians":self.holonomy_correction,
            "continuousBound":false,
            "roundingCertified":false,
            "seamContinuity":if self.closed_path {"C0"} else {"open"},
            "method":"progressive-miter-fourfold-section-refinement"
        })
    }
}
