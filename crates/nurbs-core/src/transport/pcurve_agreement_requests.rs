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
fn orientation(outcome: proof::PcurveOrientationOutcome) -> Value {
    use proof::{PcurveEdgeAgreement as Edge, PcurveOrientationOutcome as Orientation};
    match outcome {
        Orientation::Compared(Edge::Equal) => json!({"status":"equal"}),
        Orientation::Compared(Edge::Different { parameter, coordinate }) =>
            json!({"status":"different","parameter":parameter,"coordinate":coordinate}),
        Orientation::Compared(Edge::Refused(reason)) =>
            json!({"status":"comparisonRefused","reason":format!("{reason:?}")}),
        Orientation::ClockRefused(reason) =>
            json!({"status":"clockRefused","reason":format!("{reason:?}")}),
    }
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
    match op {
        "compare_seam_orientations" => {
            let report = proof::compare_pcurve_seam_orientations(a, b, budget)?;
            return Ok(json!({"forward":orientation(report.forward),
                             "reversed":orientation(report.reversed),"workUsed":report.work_used}));
        }
        "check_seam_gap" => {
            use proof::PcurveGapOutcome::*;
            let r = proof::certify_pcurve_seam_gap(a, b, field(v, "tolerance_mm")?, budget)?;
            let outcome = match r.outcome {
                WithinTolerance => json!({"status":"within"}),
                ExceedsTolerance { parameter } => json!({"status":"exceeds","parameter":parameter}),
                Refused(reason) => return Err(input(format!("Gap proof refused: {reason:?}"))),
            };
            return Ok(json!({"outcome":outcome,"workUsed":r.work_used}));
        }
        "check_seam_angle" => {
            use proof::PcurveAngleOutcome::*;
            let r =
                proof::certify_pcurve_seam_angle(a, b, field(v, "min_cosine_squared")?, budget)?;
            let outcome = match r.outcome {
                WithinTolerance => json!({"status":"within"}),
                ExceedsTolerance { parameter } => json!({"status":"exceeds","parameter":parameter}),
                Singular { parameter, face } => {
                    json!({"status":"singular","parameter":parameter,"face":face})
                }
                Refused(reason) => return Err(input(format!("Angle proof refused: {reason:?}"))),
            };
            return Ok(json!({"outcome":outcome,"workUsed":r.work_used}));
        }
        "isolate_seam_gap" => {
            let r = proof::isolate_pcurve_seam_gap(
                a,
                b,
                field(v, "tolerance_mm")?,
                field(v, "parameter_width")?,
                budget,
            )?;
            let bands = r
                .bands
                .ok_or_else(|| input(format!("Gap isolation refused: {:?}", r.refusal)))?;
            return Ok(json!({"outcome":band_report(bands),"workUsed":r.work_used}));
        }
        "isolate_seam_angle" => {
            use proof::PcurveAngleBandsOutcome::*;
            let r = proof::isolate_pcurve_seam_angle(
                a,
                b,
                field(v, "min_cosine_squared")?,
                field(v, "parameter_width")?,
                budget,
            )?;
            let outcome = match r.outcome {
                Bands(bands) => band_report(bands),
                Singular { parameter, face } => {
                    json!({"status":"singular","parameter":parameter,"face":face})
                }
                Refused(reason) => {
                    return Err(input(format!("Angle isolation refused: {reason:?}")));
                }
            };
            return Ok(json!({"outcome":outcome,"workUsed":r.work_used}));
        }
        _ => {}
    }
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

fn band_report(bands: Vec<proof::PcurveGapBand>) -> Value {
    let bands = bands
        .into_iter()
        .map(|band| {
            let endpoint =
                |b: proof::PcurveGapBoundary| json!({"bracket":b.bracket,"included":b.included});
            json!({"start":endpoint(band.start),"end":endpoint(band.end)})
        })
        .collect::<Vec<_>>();
    json!({"status":"bands","bands":bands})
}
