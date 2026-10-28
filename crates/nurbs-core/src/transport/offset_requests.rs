//! Offset diagnostics cross the wire; private geometric authority does not.
use super::*;
use crate::surface_offset::{
    ContinuousOffsetDecision as D, ContinuousOffsetLimits, ContinuousOffsetStop as S,
};
pub(super) fn inspect(v: &Value) -> Result<Value> {
    let r = crate::surface_offset::inspect_continuous_offset(
        &field(v, "source")?,
        &field(v, "candidate")?,
        field(v, "distance")?,
        field(v, "tolerance")?,
        ContinuousOffsetLimits {
            max_cells: field(v, "maxCells")?,
            max_depth: field(v, "maxDepth")?,
        },
    )?
    .report;
    let decision = match r.decision {
        D::WithinTolerance => "within_tolerance",
        D::ExceedsTolerance => "exceeds_tolerance",
        D::Unresolved => "unresolved",
    };
    let stop = match r.stop {
        S::Tolerance => "tolerance",
        S::Witness => "witness",
        S::WorkLimit => "work_limit",
        S::DepthLimit => "depth_limit",
        S::PrecisionLimit => "precision_limit",
    };
    Ok(
        json!({"decision":decision,"stop":stop,"errorUpper":r.error_upper,"errorLower":r.error_lower,
        "witnessUv":r.witness_uv,"cells":r.cells,"acceptedCells":r.accepted_cells,
        "continuousBound":r.error_upper.is_some(),"sourceNormalsProven":r.source_normals_proven,
        "idealLocalFoldFree":r.ideal_local_fold_free,"candidateLocallyRegular":r.candidate_locally_regular,
        "topologyAuthority":false,"globalEmbeddingCertified":false,
        "method":"original-span-interval-mean-value"}),
    )
}
