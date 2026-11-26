use super::*;
use crate::editing::exact_projective as seam;

pub(super) fn apply(v: &Value, op: &str) -> Result<Value> {
    let curve = field(v, "source")?;
    if op == "isolate_uv_line" {
        let r = seam::isolate_pcurve_line(
            &curve,
            field(v, "line_coefficients")?,
            field(v, "parameter_width")?,
            field(v, "max_work")?,
        )?;
        return isolation(curve, r);
    }
    let axis = match field::<String>(v, "direction")?.as_str() {
        "u" => seam::UvAxis::U,
        "v" => seam::UvAxis::V,
        _ => return Err(input("Unknown UV seam axis")),
    };
    let at = field(v, "seam_coordinate")?;
    let budget = field(v, "max_work")?;
    match op {
        "classify_uv_seam" => {
            let r = seam::classify_pcurve_seam(&curve, axis, at, budget)?;
            let spans = r
                .spans
                .ok_or_else(|| input(format!("Seam classification refused: {:?}", r.refusal)))?;
            let spans = spans.into_iter().map(|span| {
                let contact = match span.contact {
                    seam::PcurveSeamContact::Coincident => json!({"kind":"coincident"}),
                    seam::PcurveSeamContact::Discrete {crossings,touches,at_start,at_end} =>
                        json!({"kind":"discrete","crossings":crossings,"touches":touches,"atStart":at_start,"atEnd":at_end}),
                };
                json!({"domain":span.parameter,"contact":contact})
            }).collect::<Vec<_>>();
            Ok(json!({"curve":curve,"spans":spans,"workUsed":r.work_used}))
        }
        "isolate_uv_seam" => {
            let r =
                seam::isolate_pcurve_seam(&curve, axis, at, field(v, "parameter_width")?, budget)?;
            isolation(curve, r)
        }
        _ => {
            let r = seam::split_pcurve_at_seam(&curve, axis, at, budget)?;
            let mut pieces = r
                .pieces
                .ok_or_else(|| input(format!("Seam split refused: {:?}", r.refusal)))?;
            let index = field(v, "piece")?;
            let count = pieces.len();
            if index >= count {
                return Err(input("Selected piece is outside the seam split"));
            }
            let piece = pieces.swap_remove(index);
            let side = match piece.side {
                seam::PcurveSeamSide::Below => "below",
                seam::PcurveSeamSide::On => "on",
                seam::PcurveSeamSide::Above => "above",
            };
            Ok(
                json!({"curve":piece.curve,"domain":piece.curve.domain(),"pieceCount":count,"side":side,"workUsed":r.work_used}),
            )
        }
    }
}

fn isolation(curve: crate::curve::Curve, r: seam::PcurveSeamIsolationReport) -> Result<Value> {
    let spans = r
        .spans
        .ok_or_else(|| input(format!("Seam isolation refused: {:?}", r.refusal)))?;
    let spans = spans
        .into_iter()
        .map(|span| {
            let contact = match span.contact {
                seam::PcurveSeamIsolation::Coincident => json!({"kind":"coincident"}),
                seam::PcurveSeamIsolation::Discrete {
                    roots,
                    at_start,
                    at_end,
                } => {
                    let roots = roots
                        .iter()
                        .map(|r| json!({"domain":r.parameter,"crosses":r.crosses}))
                        .collect::<Vec<_>>();
                    json!({"kind":"discrete","roots":roots,"atStart":at_start,"atEnd":at_end})
                }
            };
            json!({"domain":span.parameter,"contact":contact})
        })
        .collect::<Vec<_>>();
    Ok(json!({"curve":curve,"spans":spans,"workUsed":r.work_used}))
}
