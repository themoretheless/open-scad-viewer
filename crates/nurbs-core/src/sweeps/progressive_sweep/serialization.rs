use super::{Approximation, Level, LevelReport, MultiApproximation, MultiLevel};
use value_codec::{Serialize, Value, json};

impl Serialize for super::RetainedDecompositionReport {
    fn to_value(&self) -> Value {
        let joins = self.joins.iter().map(|(pair,p)| json!({"leftPatch":pair[0],"rightPatch":pair[1],
            "certified":p.certified,"exactIdentity":p.exact_identity,"regularityCertified":p.regularity_certified,
            "exactWork":p.work,"reason":p.reason})).collect::<Vec<_>>();
        json!({"requestedOrder":self.order,"expectedJoins":self.expected_joins,"checkedJoins":joins.len(),
            "coverageComplete":self.coverage_complete,"decompositionJoinsCertified":self.all_joins_certified,
            "exactWork":self.exact_work,"joins":joins,"reason":self.reason,
            "method":"exact-retained-decomposition-strip-jets","scope":"within-source-profile-decomposition-only",
            "allProfileJoinsCertified":false,"closedProfileSeamsCertified":false,
            "sourceFrameSmoothnessCertified":false,"capJoinsCertified":false,"continuousBound":false,"solidCertified":false})
    }
}
impl Serialize for super::RetainedDecompositionSmoothnessReport {
    fn to_value(&self) -> Value {
        json!({"g2":self.g2,"g1":self.g1,"decompositionG1Certified":self.g1_certified,
            "exactWork":self.exact_work,"maxExactWork":self.max_work,
            "method":"exact-retained-decomposition-smoothness","scope":"within-source-profile-decomposition-only",
            "allProfileJoinsCertified":false,"closedProfileSeamsCertified":false,
            "sourceFrameSmoothnessCertified":false,"capJoinsCertified":false,"continuousBound":false,"solidCertified":false})
    }
}

impl Serialize for LevelReport {
    fn to_value(&self) -> Value {
        json!({
            "accepted":self.accepted,
            "sections":self.sections,
            "stations":self.stations,
            "sampledControlDeviation":self.sampled_control_deviation,
            "budget":self.budget,
            "closedPath":self.closed_path,
            "lengthResidualUpper":self.length_residual_upper,
            "continuousBound":self.continuous_bound,
            "roundingCertified":self.continuous_bound,
            "continuousErrorUpper":self.continuous_error_upper,
            "originalSectionEndpointErrorUpper":self.original_section_endpoint_error_upper,
            "endpointContourErrorUpper":self.endpoint_contour_error_upper,
            "knownProfileErrorUpper":self.known_profile_error_upper,
            "errorCertificateCells":self.error_certificate_cells,
            "decompositionProducts":self.decomposition_products,
            "errorCertificateReason":self.error_certificate_reason,
            "continuousErrorScope":"retained-patches-relative-to-original-profile-transport",
            "seamContinuity":if self.closed_path {"C0"} else {"open"},
            "method":"progressive-fourfold-section-refinement"
        })
    }
}

impl Serialize for Approximation {
    fn to_value(&self) -> Value {
        json!({"patches":self.patches,"report":self.levels.last(),"levels":self.levels})
    }
}

impl Serialize for MultiApproximation {
    fn to_value(&self) -> Value {
        json!({"patches":self.patches,"profilePatchRanges":self.profile_patch_ranges,
            "report":self.levels.last(),"levels":self.levels})
    }
}

impl Serialize for Level {
    fn to_value(&self) -> Value {
        json!({"patches":self.patches,"report":self.report,"preview":true})
    }
}
impl Serialize for MultiLevel {
    fn to_value(&self) -> Value {
        json!({"patches":self.patches,"profilePatchRanges":self.profile_patch_ranges,
            "report":self.report,"preview":true})
    }
}

impl Serialize for super::RetainedSeamReport {
    fn to_value(&self)->Value {json!({"patch":self.patch,"station":self.station,"closure":self.closure,
        "c0Identity":self.c0_identity,"certified":self.certified,"regularityCertified":self.regularity_certified,
        "exactWork":self.exact_work,"reason":self.reason})}
}
impl Serialize for super::RetainedSmoothnessReport {
    fn to_value(&self)->Value {json!({"requestedOrder":self.order,"allStationSeamsCertified":self.all_station_seams_certified,
        "exactWork":self.exact_work,"seams":self.seams,"reason":self.reason,
        "method":"exact-retained-station-strip-jets","scope":"retained-station-seams-only",
        "sourceFrameSmoothnessCertified":false,"profileJoinsCertified":false,"capJoinsCertified":false,"solidCertified":false})}
}
