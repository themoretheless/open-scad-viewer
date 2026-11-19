use super::*;
use brep_core::feature_ops::periodic_trim_sheet::{
    PeriodicTrimEdgeRange, build_exact_periodic_trim_sheet_with_input_bindings,
};
use num_rational::BigRational;
use value_codec::{Value, json};

fn ratio(t: &BigRational) -> Value {
    json!({"numerator":t.numer().to_string(),"denominator":t.denom().to_string()})
}

fn parameter(request: &Value, key: &str) -> Result<BigRational> {
    let text: String = crate::field(request, key)?;
    let digits = |s: &str| (1..=1234).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit());
    if !text
        .split_once('/')
        .is_some_and(|(n, d)| digits(n) && digits(d) && d.bytes().any(|b| b != b'0'))
    {
        return Err(input("Expected a rational trim parameter"));
    }
    text.parse()
        .map_err(|_| input("Invalid rational trim parameter"))
}

fn range_wire(range: &PeriodicTrimEdgeRange) -> Value {
    json!({"edgeId":range.edge_id,"inputParameters":range.input_parameters.each_ref().map(ratio),
        "edgeParameters":range.edge_parameters.each_ref().map(ratio)})
}

pub fn periodic_trim_binding_query(request: &Value, op: &str) -> Result<Value> {
    let surface = crate::field(request, "surface")?;
    let contours: Vec<Vec<[f64; 2]>> = crate::field(request, "contours")?;
    let paths: Vec<_> = contours.iter().map(Vec::as_slice).collect();
    let limit: u64 = crate::field(request, "max_work")?;
    if !(1..=1_000_000).contains(&limit) {
        return Err(input("Invalid trim binding work limit"));
    }
    let sheet = build_exact_periodic_trim_sheet_with_input_bindings(
        &surface,
        &paths,
        crate::field(request, "tolerance_mm")?,
        limit,
    )?;
    let built = &sheet.bound_sheet.sheet;
    let mut work = built.work_used;
    if built.model.is_none() {
        return Ok(
            json!({"result":null,"workUsed":work,"refusal":built.refusal.as_ref().map(|r|format!("{r:?}"))}),
        );
    }
    let mut refusal = None;
    let result = if op == "locate_edge_parameter" {
        let hit = sheet.locate_edge_parameter(
            crate::field(request, "edge_id")?,
            &parameter(request, "exact_parameter")?,
        )?;
        json!(hit.map(
            |h| json!({"contour":h.contour,"segment":h.segment,"parameter":ratio(&h.parameter)})
        ))
    } else {
        let contour = crate::field(request, "contour")?;
        let segment = crate::field(request, "segment")?;
        if op == "locate_input_parameter" {
            let hits = sheet.locate_input_parameter(
                contour,
                segment,
                &parameter(request, "exact_parameter")?,
            )?;
            json!(
                hits.iter()
                    .map(|h| json!({"edgeId":h.edge_id,"parameter":ratio(&h.parameter)}))
                    .collect::<Vec<_>>()
            )
        } else {
            let start = parameter(request, "range_start")?;
            let end = parameter(request, "range_end")?;
            let range = [&start, &end];
            let remaining = limit.saturating_sub(work);
            match op {
                "locate_input_range" => json!(
                    sheet
                        .locate_input_range(contour, segment, range)?
                        .iter()
                        .map(range_wire)
                        .collect::<Vec<_>>()
                ),
                _ if remaining == 0 => {
                    refusal = Some("WorkLimit".to_owned());
                    Value::Null
                }
                "extract_input_range_curves" => {
                    let r = sheet.extract_input_range_curves(contour, segment, range, remaining)?;
                    work += r.work_used;
                    refusal = r.refusal.map(|e| format!("{e:?}"));
                    json!(r.pieces.map(|ps| {
                        ps.into_iter()
                            .map(|p| json!({"range":range_wire(&p.range),"curve":p.curve}))
                            .collect::<Vec<_>>()
                    }))
                }
                "extract_input_range_curve_pairs" => {
                    let r = sheet
                        .extract_input_range_curve_pairs(contour, segment, range, remaining)?;
                    work += r.work_used;
                    refusal = r.refusal.map(|e| format!("{e:?}"));
                    json!(r.pieces.map(|ps|ps.into_iter().map(|p|json!({"range":range_wire(&p.world.range),"curve":p.world.curve,"faceId":p.face_id,"pcurve":p.pcurve})).collect::<Vec<_>>()))
                }
                _ => return Err(input("Unknown trim binding operation")),
            }
        }
    };
    Ok(json!({"result":result,"workUsed":work,"refusal":refusal}))
}
