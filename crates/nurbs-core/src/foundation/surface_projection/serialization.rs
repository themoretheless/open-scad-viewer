use super::*;
use value_codec::{Serialize, Value, json};

impl Serialize for SurfaceProjectionProof {
    fn to_value(&self) -> Value {
        match self {
            Self::DistanceSeparation => {
                json!({"method":"strict-global-distance-interval-separation","winnerIndex":0})
            }
            Self::Affine {
                gram_determinant_lower,
            } => {
                json!({"method":"strict-convex-affine-Gram","gramDeterminantLower":gram_determinant_lower})
            }
            Self::Krawczyk {
                root,
                excluded_boxes,
                separated_by_lower_bound,
                global_distance_upper,
            } => json!({
                "method":"Krawczyk-interval-operator-with-global-lower-bound-exclusion",
                "root":root,"excludedBoxes":excluded_boxes,"separatedByLowerBound":separated_by_lower_bound,
                "globalDistanceUpper":global_distance_upper
            }),
        }
    }
}

impl Serialize for SurfaceProjectionCandidate {
    fn to_value(&self) -> Value {
        let classification = match self.classification {
            SurfaceCandidateClassification::Affine => "certified_unique_affine_projection",
            SurfaceCandidateClassification::KrawczykOrSeparated => {
                "certified_unique_krawczyk_or_separated"
            }
            SurfaceCandidateClassification::GlobalCandidateBox => "certified_global_candidate_box",
        };
        json!({"parameterBox":self.parameter_box,"point":self.point,"distanceLower":self.distance_lower,
            "distanceUpper":self.distance_upper,"classification":classification})
    }
}
impl Serialize for BoundaryProjection {
    fn to_value(&self) -> Value {
        json!({"edge":self.edge,"certificate":self.certificate})
    }
}
impl Serialize for SurfaceProjection {
    fn to_value(&self) -> Value {
        json!({"version":"nurbs-foundation/4","status":self.status,"globalDistanceUpper":self.global_distance_upper,
            "candidates":self.candidates,"boundaryReductions":self.boundary_reductions,
            "uniquenessProof":self.uniqueness_proof,
            "coverage":{"method":"2d-outward-hull-subdivision-with-Krawczyk-separation","complete":true,
                "subdivisions":self.subdivisions,"resourceLimit":super::super::MAX_CERTIFICATE_CELLS},
            "evidence":super::super::tolerance_evidence(&self.tolerance)})
    }
}
