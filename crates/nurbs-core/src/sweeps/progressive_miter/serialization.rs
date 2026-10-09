//! Shared full miter diagnostics for level and owned-body transports.
use super::Report;
use value_codec::{Serialize, Value, json};

impl Serialize for Report {
    fn to_value(&self) -> Value {
        json!({
            "accepted":self.accepted,
            "phaseResolved":self.phase_resolved,
            "frameTransportCertified":self.frame_transport_certified,
            "frameTransportReason":self.frame_transport_reason,
            "steps":self.steps,
            "sections":self.sections,
            "stations":self.stations,
            "sampledControlDeviation":self.sampled_control_deviation,
            "continuousErrorUpper":self.continuous_error_upper,
            "certifiedErrorUpper":self.certified_error_upper,
            "endpointContourErrorUpper":self.endpoint_contour_error_upper,
            "errorCertificateCells":self.error_certificate_cells,
            "errorCertificateReason":self.error_certificate_reason,
            "profileRegularityCertified":self.profile_regularity_certified,
            "wallRegularityCertified":self.wall_regularity_certified,
            "regularityCells":self.regularity_cells,
            "unresolvedWallPatches":self.unresolved_wall_patches,
            "affineLawsApplied":self.affine_laws_applied,
            "authoredFramesApplied":self.authored_frames_applied,
            "orientationGuideApplied":self.orientation_guide_applied,
            "continuousErrorMethod":if self.orientation_guide_applied && self.authored_frames_applied {
                "interval-authored-axis-guide-frame-interpolation"
            } else if self.orientation_guide_applied {
                "interval-guide-frame-interpolation"
            } else if self.authored_frames_applied {
                "interval-authored-frame-interpolation"
            } else if self.affine_laws_applied {
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
