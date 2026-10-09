//! Wire encoding for native analysis reports.
use super::*;

impl value_codec::Serialize for CertifiedInterval {
    fn to_value(&self) -> value_codec::Value {
        value_codec::json!({"lower":self.lower,"upper":self.upper})
    }
}

impl value_codec::Serialize for CertifiedMassProperties {
    fn to_value(&self) -> value_codec::Value {
        value_codec::json!({
            "capability":self.capability,
            "status":"certified_enclosure",
            "surfaceAreaMm2":self.surface_area_mm2,
            "volumeMm3":self.volume_mm3,
            "centroid":self.centroid,
            "inertiaMm5":self.inertia_mm5,
            "context":self.context,
            "evidenceClaimCount":self.evidence.claims.len(),
            "audit":{
                "ok":self.audit.ok,
                "bodyCount":self.audit.body_count,
                "shellCount":self.audit.shell_count,
                "selfIntersectionPairsCandidate": self.audit.self_intersection_pairs_candidate,
                "selfIntersectionComplete": self.audit.self_intersection_complete,
                "selfIntersectionPairsChecked":self.audit.self_intersection_pairs_checked
            },
            "changeSet":self.change_set,
            "namingComplete":self.naming_complete,
            "composition":{
                "componentCount":self.component_count,
                "cavityCount":self.cavity_count,
                "signedShellComposition":true
            },
            "proof":self.proof
        })
    }
}

impl value_codec::Serialize for MassProperties {
    fn to_value(&self) -> value_codec::Value {
        let mut out = value_codec::Map::new();
        for (name, value) in [
            ("surfaceAreaMm2", self.surface_area_mm2.to_value()),
            ("signedVolumeMm3", self.signed_volume_mm3.to_value()),
            ("centroid", self.centroid.to_value()),
            ("inertiaMm5", self.inertia_mm5.to_value()),
            ("conservativeBounds", self.conservative_bounds.to_value()),
            (
                "areaErrorEstimateMm2",
                self.area_error_estimate_mm2.to_value(),
            ),
            (
                "volumeErrorEstimateMm3",
                self.volume_error_estimate_mm3.to_value(),
            ),
            ("evaluations", self.evaluations.to_value()),
            ("status", "converged_estimate".to_value()),
            ("solidGeometryStatus", "not_certified".to_value()),
        ] {
            out.insert(name.into(), value);
        }
        value_codec::Value::Object(out)
    }
}
