//! Rust-only ownership and reporting for a closed original material volume.
use super::*;
pub(super) fn construct(
    sweep: &MultiSweep<'_>,
    loop_sizes: &[usize],
    sections: usize,
    tolerance_uv: f64,
    error_budget: f64,
    boundary: owned::Limits,
    ideal: owned::IdealLimits,
    v: &Value,
) -> Result<Value> {
    let material = LoftCap {
        surface: field(&v["material"], "surface")?,
        trims: field(&v["material"], "trims")?,
    };
    let m = &v["limits"]["material"];
    let attempt = owned::construct_closed_with_ideal_atlas(
        sweep,
        loop_sizes,
        sections,
        &material,
        tolerance_uv,
        error_budget,
        owned::ClosedLimits {
            boundary,
            ideal,
            wall_faces: field(&v["limits"], "maxWallFaces")?,
            material: owned::MaterialLimits {
                pairs: field(m, "maxPairs")?,
                cells: field(m, "maxCells")?,
                domain_cells: field(m, "maxDomainCells")?,
            },
        },
    )?;
    let joint=match &attempt {
        owned::ClosedAttempt::Certified(body)=>[0,1,2].map(|order|body.joint_certificate(order).is_ok()),
        owned::ClosedAttempt::Refused(_)=>[false;3],
    };
    let (model, r) = match &attempt {
        owned::ClosedAttempt::Certified(body) => (Some(body.model()), body.report()),
        owned::ClosedAttempt::Refused(r) => (None, r.as_ref()),
    };
    Ok(
        json!({"model":model,"jointContinuousGlobalCertified":joint[0],"jointContinuousGlobalG1Certified":joint[1],"jointContinuousGlobalG2Certified":joint[2],"certified":r.certified,"continuousBound":r.continuous_bound,
        "withinBudget":r.within_budget,"boundaryErrorUpper":r.boundary_error_upper,
        "decompositionErrorUpper":r.decomposition_error_upper,"wall":r.wall,
        "sourceCells":r.source_cells,"sourceProducts":r.source_products,"ownershipExactWork":r.ownership_exact_work,
        "scope":"original-closed-sweep-law-and-material-region","physicalCapCount":0,"filledCapErrorUpper":null,
        "reason":r.reason,"smoothnessCertified":false,"seamContinuity":"C0",
        "material":{"certified":r.material.certified,"boundSpans":r.material.bound_spans,"exactWork":r.material.exact_work,
            "trim":r.material.trim.as_ref().map(|t|json!({"valid":t.valid,"reason":t.reason,"winding":t.winding,
                "pairs":t.pairs,"cells":t.cells,"domainCells":t.domain_cells,"problemLoops":t.problem_loops})),
            "profiles":r.material.profiles.iter().map(|p|json!({"spanwiseRegular":p.spanwise_regular,"cells":p.cells,"reason":p.reason})).collect::<Vec<_>>()},
        "retainedWallOwnership":r.retained_walls.as_ref().map(|w|json!({"certified":w.certified,"faceCoverageCertified":w.face_coverage_certified,
            "coefficientFamilyCertified":w.coefficient_family_certified,"inspectedFaces":w.inspected_faces,"exactWork":w.exact_work,"reason":w.reason})),
        "idealBodyEmbeddingCertified":r.ideal_volume.as_ref().is_some_and(|p|p.embedding_certified),
        "idealBody":r.ideal_volume.as_ref().map(|p|json!({"proofMethod":"atlas","embeddingCertified":p.embedding_certified,
            "volumeRegularityCertified":p.regularity_certified,"parameterTopologyPreserved":p.parameter_topology_preserved,
            "cells":p.cells,"exactWork":p.exact_work,"boxes":p.boxes,"projection":p.projection,"reason":p.reason})),
        "regularity":r.regularity.as_ref().map(|p|json!({"proven":p.all_faces_regular_modulo_poles,"cells":p.cells,"unresolvedFaces":p.unresolved_faces})),
        "solidGeometryCertified":r.volume.as_ref().is_some_and(|p|p.proven)}),
    )
}
