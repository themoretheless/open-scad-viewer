use super::*;
use value_codec::{Serialize, Value, json};

impl Serialize for ProjectionCandidate {
    fn to_value(&self) -> Value {
        let classification = match self.classification {
            CandidateClassification::Endpoint => "endpoint",
            CandidateClassification::SimpleStationary => "simple_stationary",
            CandidateClassification::MultipleOrClusteredStationary => {
                "multiple_or_clustered_stationary"
            }
        };
        json!({"domain":self.domain,"parameterInterval":self.parameter_interval,"point":self.point,
            "distanceLower":self.distance_lower,"distanceUpper":self.distance_upper,
            "classification":classification,"rootVariation":self.root_variation})
    }
}
impl Serialize for CurveProjection {
    fn to_value(&self) -> Value {
        json!({"version":"nurbs-foundation/3","status":self.status,
            "globalDistanceUpper":self.global_distance_upper,"candidates":self.candidates,
            "coverage":{"method":"Bernstein-sign-variation","endpointsIncluded":true,
                "stationaryContinuum":self.stationary_continuum,"resourceLimit":super::super::MAX_CERTIFICATE_CELLS},
            "uniquenessProof":self.winner_index.map(|index| json!({"winnerIndex":index,"method":"strict-global-distance-interval-separation"})),
            "evidence":super::super::tolerance_evidence(&self.tolerance)})
    }
}

impl Serialize for ProjectionStatus {
    fn to_value(&self) -> Value {
        let status = match self {
            Self::Unique => "unique",
            Self::Nonunique => "nonunique",
            Self::NonuniqueOrUnresolved => "nonunique_or_unresolved",
            Self::IsolatedCandidate => "isolated_candidate",
        };
        status.to_value()
    }
}
