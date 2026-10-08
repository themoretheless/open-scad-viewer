use super::*;
pub(super) fn profile_request(v: &Value, profiles: &Value) -> Result<Value> {
    let o = &v["options"];
    let mut req = json!({"profiles":profiles,"path":v["path"],"scale":law(&v["scale"],false)?,"twist":law(&v["twist"],true)?,"normal":o["normal"],"orientation":value(o,"orientation",json!("rmf")),"spacing":value(o,"spacing",json!("parameter")),"initial_sections":value(o,"initialSections",json!(5)),"max_sections":value(o,"maxSections",json!(257)),"max_deviation":o["maxDeviation"],"length_tolerance":value(o,"lengthTolerance",json!(0.001)),"length_max_cells":value(o,"lengthMaxCells",json!(100000))});
    for (source, target) in [
        ("axisScale", "axis_scale"),
        ("centerLaw", "center_law"),
        ("frameAxis", "frame_axis"),
        ("frameNormal", "frame_normal"),
    ] {
        if !o[source].is_null() {
            req[target] = law(&o[source], false)?;
        }
    }
    // Preserve explicitly authored proof and endpoint-correction budgets.
    for (source, target) in [
        ("rmfTransportSteps", "rmf_transport_steps"),
        ("errorMaxCells", "error_max_cells"),
        ("errorMaxProducts", "error_max_products"),
    ] {
        if let Some(value) = o.get(source) { req[target] = value.clone(); }
    }
    if let Some(correction) = o.get("capCorrection") {
        for (source, target) in [
            ("tolerance", "cap_correction_tolerance"),
            ("quantum", "cap_correction_quantum"),
            ("maxWork", "cap_correction_max_work"),
        ] {
            if let Some(value) = correction.get(source) { req[target] = value.clone(); }
        }
    }
    if !o["orientationGuide"].is_null() {
        req["orientation_guide"] = o["orientationGuide"].clone();
        if !o["contactAnchor"].is_null() {
            req["contact_profile"] = value(&o["contactAnchor"], "profileIndex", json!(0));
            req["contact_parameter"] = o["contactAnchor"]["parameter"].clone();
        }
    }
    Ok(req)
}
pub(super) fn bounded_request(v: &Value, miter: bool) -> Result<(Value, Vec<usize>, usize)> {
    let loops: Vec<Vec<Value>> = field(v, "loops")?;
    if loops.is_empty() || loops.len() > 16 || loops.iter().any(Vec::is_empty) {
        return Err(input("Progressive body needs 1..16 nonempty loops"));
    }
    let profiles = json!(loops.iter().flatten().cloned().collect::<Vec<_>>());
    let sizes = loops.iter().map(Vec::len).collect();
    let mut spans = 0;
    for p in profiles.as_array().unwrap() {
        spans += nurbs("curve_decompose", json!({"curve":p}))?
            .as_array()
            .ok_or_else(|| input("Invalid native decomposition"))?
            .len();
    }
    if spans < 1 || spans > 64 {
        return Err(input("Progressive body exceeds64 section spans"));
    }
    let mut request = if miter {
        miter_request(v, &profiles)?
    } else {
        profile_request(v, &profiles)?
    };
    let (maximum, initial) = if miter {
        let points: Vec<[f64; 3]> = field(v, "points")?;
        let closed = yes(&v["options"], "closed");
        let edges = points
            .len()
            .checked_sub(usize::from(!closed))
            .filter(|&n| n > 0)
            .ok_or_else(|| input("Invalid miter sites"))?;
        let max = field::<usize>(&request, "max_steps")?
            .min(1024 / edges)
            .min((1024 - if closed { 0 } else { 2 }) / (edges * spans));
        request["max_steps"] = json!(max);
        (max, field::<usize>(&request, "initial_steps")?)
    } else {
        let max = field::<usize>(&request, "max_sections")?;
        if max > 1025 {
            return Err(input("Progressive body needs at most1025 sections"));
        }
        let initial = field::<usize>(&request, "initial_sections")?;
        let mut preview = request.clone();
        preview["preview_sections"] = json!(initial);
        let closed = yes(
            &nurbs("surface_progressive_sweep_level", preview)?["report"],
            "closedPath",
        );
        let max = max.min((1024 - if closed { 0 } else { 2 }) / spans + 1);
        request["max_sections"] = json!(max);
        (max, initial)
    };
    if initial > maximum {
        return Err(input(if miter {
            "Progressive miter initial steps exceed face budget"
        } else {
            "Progressive body initial sections exceed face budget"
        }));
    }
    Ok((request, sizes, maximum))
}
pub fn profile_body(v: Value) -> Result<Value> {
    profile_body_from(v, None, None)
}
pub(super) fn profile_body_from(v: Value, accepted: Option<Value>, prepared: Option<Value>) -> Result<Value> {
    let (mut request, _) = if let Some(request) = prepared {
        (request, ())
    } else {
        let (request, _, _) = bounded_request(&v, false)?;
        (request, ())
    };
    if accepted.as_ref().is_some_and(|level| !yes(&level["report"], "accepted")) {
        return Err(input("Progressive body sampled refinement exceeds budget"));
    }
    request["loops"] = v["loops"].clone();
    // Rebuild from the native constructor-owned retained sections. Re-lofting
    // sampled rows here would discard correction and full-boundary evidence.
    let mut body = call("brep_nurbs_progressive_profile_body", request)?;
    let model = body["model"].clone();
    let closed = yes(&body["approximation"]["report"], "closedPath");
    let caps = cap_faces(&model, closed)?;
    let volume = call("brep_sweep_volume_audit",
        merge(json!({"model":model,"capFaces":caps}), &volume_budgets()))?;
    body["volume"] = volume.clone();
    body["approximation"]["report"]["volume"] = volume.clone();
    body["approximation"]["report"]["wallRegularityCertified"] = volume["allFacesInjective"].clone();
    body["approximation"]["report"]["seamContinuity"] = json!("C0");
    Ok(body)
}

#[derive(Clone)]
struct Stream {
    request: Value,
    compiled: Value,
    miter: bool,
    raw: bool,
    count: usize,
    maximum: usize,
    finish: bool,
    levels: Vec<Value>,
    last: Value,
}
thread_local! {static STREAMS:std::cell::RefCell<(u64,std::collections::BTreeMap<u64,Stream>)>=std::cell::RefCell::new((0,std::collections::BTreeMap::new()));}
pub fn stream_start(v: Value) -> Result<Value> {
    let kind = field::<String>(&v, "kind")?;
    let miter = kind == "miter" || kind == "raw-miter";
    let raw = kind == "raw-miter" || kind == "raw-profile";
    if !["miter", "profile", "raw-miter", "raw-profile"].contains(&kind.as_str()) {
        return Err(input("Unknown native sweep stream kind"));
    }
    let (req, maximum) = if raw {
        let request = if miter {
            miter_request(&v, &v["profiles"])?
        } else {
            profile_request(&v, &v["profiles"])?
        };
        let maximum = field::<usize>(&request, if miter { "max_steps" } else { "max_sections" })?;
        (request, maximum)
    } else {
        let (request, _, maximum) = bounded_request(&v, miter)?;
        (request, maximum)
    };
    let count = field::<usize>(
        &req,
        if miter {
            "initial_steps"
        } else {
            "initial_sections"
        },
    )?;
    let state = Stream {
        request: v,
        compiled: req,
        miter,
        raw,
        count,
        maximum,
        finish: false,
        levels: Vec::new(),
        last: Value::Null,
    };
    let id = STREAMS.with(|store| {
        let mut store = store.borrow_mut();
        if store.1.len() >= 128 {
            return Err(input("Native sweep stream budget exhausted"));
        }
        store.0 = store
            .0
            .checked_add(1)
            .ok_or_else(|| input("Sweep stream identifiers exhausted"))?;
        let id = store.0;
        store.1.insert(id, state);
        Ok(id)
    })?;
    Ok(json!({"stream":id.to_string()}))
}
pub fn stream_release(v: Value) -> Result<Value> {
    let id = field::<String>(&v, "stream")?
        .parse::<u64>()
        .map_err(|_| input("Invalid sweep stream"))?;
    STREAMS.with(|s| s.borrow_mut().1.remove(&id));
    Ok(Value::Null)
}
pub fn stream_next(v: Value) -> Result<Value> {
    let id = field::<String>(&v, "stream")?
        .parse::<u64>()
        .map_err(|_| input("Invalid sweep stream"))?;
    let mut state = STREAMS
        .with(|s| s.borrow_mut().1.remove(&id))
        .ok_or_else(|| input("Unknown or completed native sweep stream"))?;
    if state.finish {
        let accepted = yes(&state.last["report"], "accepted");
        let approximation = if state.miter {
            json!({"sections":if accepted{state.last["sections"].clone()}else{Value::Null},"report":state.last["report"],"levels":state.levels})
        } else {
            json!({"patches":if accepted{state.last["patches"].clone()}else{Value::Null},"profilePatchRanges":if accepted{state.last["profilePatchRanges"].clone()}else{Value::Null},"report":state.last["report"],"levels":state.levels})
        };
        let result = if state.raw {
            approximation
        } else if state.miter {
            progressive_miter_from(state.request, Some(approximation))?
        } else {
            profile_body_from(state.request, Some(approximation), Some(state.compiled))?
        };
        return Ok(json!({"done":true,"value":result}));
    }
    let mut req = state.compiled.clone();
    req[if state.miter {
        "preview_steps"
    } else {
        "preview_sections"
    }] = json!(state.count);
    let mut preview = nurbs(
        if state.miter {
            "curve_progressive_miter_level"
        } else {
            "surface_progressive_sweep_level"
        },
        req,
    )?;
    state.finish = yes(&preview["report"], "accepted") || state.count >= state.maximum;
    if !state.finish {
        state.count = if state.miter {
            (2 * state.count).min(state.maximum)
        } else {
            (2 * (state.count - 1) + 1).min(state.maximum)
        };
    }
    state.levels.push(preview["report"].clone());
    state.last = preview.clone();
    if state.miter && !state.raw {
        preview = miter_wall_preview(preview)?;
    }
    STREAMS.with(|s| s.borrow_mut().1.insert(id, state));
    Ok(json!({"done":false,"value":preview}))
}

pub(super) fn miter_wall_preview(preview: Value) -> Result<Value> {
    let sections: Vec<Vec<Value>> = field(&preview, "sections")?;
    let mut rows = Vec::new();
    for row in sections {
        let mut curves = Vec::new();
        for curve in row {
            let parts = nurbs("curve_decompose", json!({"curve":curve}))?;
            let parts: Vec<Value> =
                value_codec::from_value(parts).map_err(|e| input(e.to_string()))?;
            curves.push(
                parts
                    .into_iter()
                    .map(|p| p["curve"].clone())
                    .collect::<Vec<_>>(),
            );
        }
        rows.push(curves);
    }
    let mut patches = Vec::new();
    let mut ranges = Vec::new();
    if let Some(first) = rows.first() {
        for p in 0..first.len() {
            let start = patches.len();
            for stations in rows.windows(2) {
                for (a, b) in stations[0][p].iter().zip(&stations[1][p]) {
                    let ac: Vec<Value> = field(a, "controlPoints")?;
                    let bc: Vec<Value> = field(b, "controlPoints")?;
                    let aw: Vec<Value> = field(a, "weights")?;
                    let bw: Vec<Value> = field(b, "weights")?;
                    patches.push(json!({"degreeU":a["degree"],"degreeV":1,"knotsU":a["knots"],"knotsV":[0,0,1,1],"controlPoints":ac.iter().zip(bc).map(|(a,b)|json!([a,b])).collect::<Vec<_>>(),"weights":aw.iter().zip(bw).map(|(a,b)|json!([a,b])).collect::<Vec<_>>(),"periodicU":false,"periodicV":false}));
                }
            }
            ranges.push([start, patches.len()]);
        }
    }
    Ok(
        json!({"preview":true,"patches":patches,"profilePatchRanges":ranges,"report":preview["report"]}),
    )
}

/// Public adapters pass authored laws unchanged; conversion belongs to the kernel.
pub fn constructor(v: Value) -> Result<Value> {
    let operation = field::<String>(&v, "operation")?;
    if ["surface_scaled_sweep", "surface_profile_sweep"].contains(&operation.as_str()) {
        let mut request=json!({"profile":v["profile"],"path":v["path"],"scale":law(&v["scale"],false)?,"origin":v["origin"],"normal":v["normal"],"sections":v["sections"],"max_deviation":v["maxDeviation"]});
        if let Some(cells)=v.get("maxCells") {request["maxCells"]=cells.clone();}
        return nurbs(&operation,request);
    }
    let miter = operation.starts_with("curve_progressive_miter");
    if ![
        "curve_progressive_miter",
        "curve_progressive_miter_level",
        "curve_progressive_miter_cap_projection",
        "curve_progressive_miter_cap_parallelism",
        "curve_progressive_miter_cap_domains",
        "curve_progressive_miter_wall_audit",
        "surface_progressive_sweep",
        "surface_progressive_sweep_profiles",
        "surface_progressive_sweep_level",
    ]
    .contains(&operation.as_str())
    {
        return Err(input("Unsupported sweep constructor operation"));
    }
    let profiles = if operation == "surface_progressive_sweep" {
        json!([v["profile"]])
    } else {
        v["profiles"].clone()
    };
    let mut request = if miter {
        miter_request(&v, &profiles)?
    } else {
        profile_request(&v, &profiles)?
    };
    if operation == "surface_progressive_sweep" {
        request["profile"] = v["profile"].clone();
    }
    for key in [
        "preview_steps",
        "preview_sections",
        "caps",
        "maxCells",
        "maxExactWork",
        "loopSizes",
        "tolerance",
        "maxPairs",
        "sections",
        "clearance",
        "distanceTolerance",
        "maxInjectivityCells",
        "maxPairCells",
    ] {
        if let Some(value) = v.get(key) {
            request[key] = value.clone();
        }
    }
    nurbs(&operation, request)
}

/// Compatibility payload projections still perform every law conversion natively.
pub fn law_payload(v: Value) -> Result<Value> {
    let options = &v["options"];
    let kind = field::<String>(&v, "kind")?;
    match kind.as_str() {
        "guide" => {
            let mut result = json!({});
            if let Some(guide) = options.get("orientationGuide") {
                result["orientation_guide"] = guide.clone();
                if !options["contactAnchor"].is_null() {
                    result["contact_profile"] =
                        value(&options["contactAnchor"], "profileIndex", json!(0));
                    result["contact_parameter"] = options["contactAnchor"]["parameter"].clone();
                }
            }
            Ok(result)
        }
        "frame" => {
            if options["orientation"] != json!("authored") {
                return Ok(json!({}));
            }
            if options["frameAxis"].is_null() || options["frameNormal"].is_null() {
                return Err(input("Authored sweep requires frameAxis and frameNormal"));
            }
            Ok(
                json!({"frame_axis":law(&options["frameAxis"],false)?,"frame_normal":law(&options["frameNormal"],false)?}),
            )
        }
        "affine" => Ok(
            json!({"axis_scale":if options["axisScale"].is_null(){Value::Null}else{law(&options["axisScale"],false)?},"center_law":if options["centerLaw"].is_null(){Value::Null}else{law(&options["centerLaw"],false)?}}),
        ),
        _ => Err(input("Unknown sweep law payload")),
    }
}
