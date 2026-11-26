use super::*;
use crate::editing::exact_projective as proof;

fn agreement(r: proof::PcurveEdgeAgreementReport) -> Result<Value> {
    use proof::PcurveEdgeAgreement::*;
    let outcome = match r.outcome {
        Equal => json!({"status":"equal"}),
        Different {
            parameter,
            coordinate,
        } => json!({"status":"different","parameter":parameter,"coordinate":coordinate}),
        Refused(reason) => return Err(input(format!("Trace comparison refused: {reason:?}"))),
    };
    Ok(json!({"outcome":outcome,"workUsed":r.work_used}))
}
pub(super) fn apply(v: &Value, op: &str) -> Result<Value> {
    let surface = field(v, "surface")?;
    let trace = field(v, "trace")?;
    let budget = field(v, "max_work")?;
    if op == "verify_uv_edge" {
        return agreement(proof::compare_pcurve_edge(
            &surface,
            &trace,
            &field(v, "edge")?,
            budget,
        )?);
    }
    let right = field(v, "right")?;
    let right_trace = field(v, "right_trace")?;
    let a = (&surface, &trace);
    let b = (&right, &right_trace);
    if op == "compare_uv_traces" {
        return agreement(proof::compare_surface_pcurve_seam(a, b, budget)?);
    }
    let r = proof::compare_pcurve_tangent_planes(a, b, budget)?;
    use proof::PcurveTangentOutcome::*;
    let outcome = match r.outcome {
        Equal => json!({"status":"equal"}),
        PositionMismatch {
            parameter,
            coordinate,
        } => json!({"status":"positionMismatch","parameter":parameter,"coordinate":coordinate}),
        Singular { parameter, face } => {
            json!({"status":"singular","parameter":parameter,"face":face})
        }
        TangentPlaneMismatch { parameter } => {
            json!({"status":"tangentPlaneMismatch","parameter":parameter})
        }
        Refused(reason) => return Err(input(format!("Tangent comparison refused: {reason:?}"))),
    };
    Ok(json!({"outcome":outcome,"workUsed":r.work_used}))
}
