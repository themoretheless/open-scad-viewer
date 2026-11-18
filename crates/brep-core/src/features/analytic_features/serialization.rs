//! Wire encoding for native geometry results.
use super::*;

impl value_codec::Serialize for AuditedFeatureResult {
    fn to_value(&self) -> value_codec::Value {
        value_codec::json!({
            "model":self.model,
            "certificate":{
                "capability":self.feature.capability,
                "complete":self.feature.complete,
                "notes":self.feature.notes
            },
            "context":{
                "version":self.context.version,
                "canonical":self.context.canonical
            },
            "evidenceClaimCount":self.evidence.claims.len(),
            "audit":{
                "ok":self.audit.ok,
                "bodyCount":self.audit.body_count,
                "shellCount":self.audit.shell_count,
                "selfIntersectionPairsCandidate": self.audit.self_intersection_pairs_candidate,
                "selfIntersectionComplete": self.audit.self_intersection_complete,
                "selfIntersectionPairsChecked":self.audit.self_intersection_pairs_checked,
                "notes":self.audit.notes
            },
            "changeSet":self.change_set,
            "namingComplete":self.naming_complete
        })
    }
}
