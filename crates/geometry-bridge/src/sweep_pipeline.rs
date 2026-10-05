//! Native ownership of sweep correction and Solid admission decisions.
use crate::{Result, field, input};
use std::collections::BTreeSet;
use value_codec::{Value, json};
fn call(op: &str, mut value: Value) -> Result<Value> {
    value["op"] = json!(op);
    crate::dispatch(value)
}
fn nurbs(op: &str, mut value: Value) -> Result<Value> {
    value["op"] = json!(op);
    nurbs_core::dispatch(value)
}
fn merge(mut target: Value, source: &Value) -> Value {
    if let Some(fields) = source.as_object() {
        for (k, v) in fields {
            target[k.as_str()] = v.clone();
        }
    }
    target
}
pub fn volume_budgets() -> Value {
    json!({"toleranceUv":1e-8,"maxExactWork":1000000,"maxTrimPairs":10000,"maxTrimCells":100000,"maxTrimDomainCells":1000000,"maxSpans":1000,"maxLinearCells":20000,"maxPairs":10000,"maxCells":100000,"maxDomainCells":1000000,"cellsPerPair":1000,"domainCellsPerPair":10000,"capBudgets":{"maxWalls":1024,"maxExactWork":1000000,"maxChartCells":1000,"maxTrimPairs":100000,"maxTrimCells":100000,"maxTrimDomainCells":1000000},"nestingPairs":1000,"nestingCells":100000,"nestingDomainCells":1000000,"orientationCells":100000,"orientationDomainCells":1000000,"orientationSpans":100})
}
pub fn admission(v: Value) -> Result<Value> {
    let document: Value = value_codec::from_str(&field::<String>(&v, "documentJson")?)
        .map_err(|_| input("Invalid sweep provenance document"))?;
    let nodes = document["nodes"]
        .as_array()
        .ok_or_else(|| input("Missing sweep provenance nodes"))?;
    let mut id = field::<String>(&v, "nodeId")?;
    let mut visited = BTreeSet::new();
    let node = loop {
        if !visited.insert(id.clone()) {
            return Err(input("Cyclic sweep transform provenance."));
        }
        let Some(node) = nodes.iter().find(|n| n["id"].as_str() == Some(id.as_str())) else {
            if visited.len() > 1 {
                return Err(input("Missing sweep transform source."));
            }
            return Ok(Value::Null);
        };
        if node["op"].as_str() != Some("transform") {
            break node;
        }
        id = field(node, "input").map_err(|_| input("Missing sweep transform source."))?;
    };
    let op = node["op"].as_str().unwrap_or("");
    if !["brep_progressive_miter_sweep", "brep_miter_sweep"].contains(&op) {
        return Ok(Value::Null);
    }
    let model: brep_core::Model = field(&v, "model")?;
    let caps = if node["closed"] == json!(true) {
        vec![]
    } else {
        if model.faces.len() < 2 {
            return Err(input("Sweep requires endpoint caps"));
        }
        vec![model.faces.len() - 2, model.faces.len() - 1]
    };
    let report = call(
        "brep_sweep_volume_audit",
        merge(json!({"model":model,"capFaces":caps}), &volume_budgets()),
    )?;
    if report["solidGeometryCertified"] != json!(true) {
        let stage = if report["boundaryEmbeddingCertified"] != json!(true) {
            "boundary embedding"
        } else if report["nesting"]["rolesConsistent"] != json!(true) {
            "shell nesting"
        } else {
            "material orientation"
        };
        return Err(input(&format!(
            "{} Solid geometry could not be proved: {stage}.",
            if op == "brep_progressive_miter_sweep" {
                "Progressive sweep"
            } else {
                "Miter sweep"
            }
        )));
    }
    Ok(report)
}
fn cap_correction(
    sections: &Value,
    points: &[[f64; 3]],
    closed: bool,
    options: &Value,
) -> Result<Value> {
    if closed {
        return Err(input("Closed miter has no caps to correct"));
    }
    let count = sections
        .as_array()
        .ok_or_else(|| input("Missing miter sections"))?
        .len();
    if points.len() < 2 || count < 2 {
        return Err(input("Cap correction needs endpoint sections"));
    }
    let mut corrections = Vec::new();
    for end in [false, true] {
        let at = if end { points.len() - 1 } else { 0 };
        let from = if end { points.len() - 2 } else { 0 };
        let to = if end { points.len() - 1 } else { 1 };
        let direction: [f64; 3] = std::array::from_fn(|k| points[to][k] - points[from][k]);
        let axis = (0..3)
            .reduce(|a, b| {
                if direction[a].abs() >= direction[b].abs() {
                    a
                } else {
                    b
                }
            })
            .unwrap();
        if !direction[axis].is_finite() || direction[axis] == 0. {
            return Err(input("Cap correction needs a nonzero endpoint direction"));
        }
        let free: Vec<_> = (0..3).filter(|&k| k != axis).collect();
        let coefficients = [
            -direction[free[0]] / direction[axis],
            -direction[free[1]] / direction[axis],
        ];
        let offset = points[at][axis]
            - coefficients[0] * points[at][free[0]]
            - coefficients[1] * points[at][free[1]];
        corrections.push(json!({"section":if end{count-1}else{0},"plane":{"axis":axis,"coefficients":coefficients,"offset":offset},"quantum":options["quantum"],"tolerance":options["tolerance"]}));
    }
    let result = nurbs(
        "sweep_project_sections",
        json!({"sections":sections,"corrections":corrections,"maxWork":options["maxWork"]}),
    )?;
    if result["sections"].is_null() || result["wallDisplacementUpper"].is_null() {
        return Err(input(&format!(
            "Miter cap correction unproved: {}",
            result["reason"]
        )));
    }
    Ok(result)
}
pub fn correct(v: Value) -> Result<Value> {
    let sections = &v["sections"];
    let points: Vec<[f64; 3]> = field(&v, "points")?;
    let options = &v["options"];
    let closed = options["closed"] == json!(true);
    if options["circleCorrection"].is_null() {
        return if options["capCorrection"].is_null() {
            Ok(Value::Null)
        } else {
            cap_correction(sections, &points, closed, &options["capCorrection"])
        };
    }
    let circle = nurbs(
        "sweep_repair_circle_sections",
        merge(json!({"sections":sections}), &options["circleCorrection"]),
    )?;
    if circle["sections"].is_null() || circle["wallDisplacementUpper"].is_null() {
        return Err(input(&format!(
            "Miter circle section correction unproved: {}",
            circle["reason"]
        )));
    }
    let cap = if options["capCorrection"].is_null() {
        Value::Null
    } else {
        cap_correction(
            &circle["sections"],
            &points,
            closed,
            &options["capCorrection"],
        )?
    };
    let cap_upper = if cap.is_null() {
        json!(0)
    } else {
        cap["wallDisplacementUpper"].clone()
    };
    let upper = nurbs(
        "sweep_error_upper_compose",
        json!({"kind":"add","a":cap_upper,"b":circle["wallDisplacementUpper"]}),
    )?["errorUpper"]
        .clone();
    if upper.is_null() {
        return Err(input(
            "Miter combined section correction displacement unproved",
        ));
    }
    Ok(
        json!({"sections":if cap.is_null(){&circle["sections"]}else{&cap["sections"]},"wallDisplacementUpper":upper,"exactPlanarSections":if cap.is_null(){json!([])}else{cap["exactPlanarSections"].clone()},"work":field::<u64>(&circle,"work")?+if cap.is_null(){0}else{field::<u64>(&cap,"work")?},"reason":"bounded-circle-section-interpolation"}),
    )
}
fn value(v: &Value, key: &str, default: Value) -> Value {
    v.get(key)
        .filter(|x| !x.is_null())
        .cloned()
        .unwrap_or(default)
}
fn yes(v: &Value, key: &str) -> bool {
    v[key] == json!(true)
}
fn add(a: Value, b: Value) -> Result<Value> {
    Ok(nurbs(
        "sweep_error_upper_compose",
        json!({"kind":"add","a":a,"b":b}),
    )?["errorUpper"]
        .clone())
}
fn law(v: &Value, angle: bool) -> Result<Value> {
    let values = v["values"]
        .as_array()
        .ok_or_else(|| input("Missing sweep law values"))?;
    let poles: Vec<Value> = values
        .iter()
        .map(|x| {
            if x.is_array() {
                Ok(x.clone())
            } else {
                let x: f64 = value_codec::from_value(x.clone())
                    .map_err(|_| input("Invalid scalar sweep law"))?;
                Ok(json!([
                    if angle {
                        x * std::f64::consts::PI / 180.
                    } else {
                        x
                    },
                    0.,
                    0.
                ]))
            }
        })
        .collect::<Result<_>>()?;
    Ok(
        json!({"degree":v["degree"],"knots":v["knots"],"controlPoints":poles,"weights":v["weights"],"periodic":false}),
    )
}
fn miter_request(v: &Value, profiles: &Value) -> Result<Value> {
    let o = &v["options"];
    let mut request = json!({"profiles":profiles,"points":v["points"],"scale":law(&v["scale"],false)?,"twist":law(&v["twist"],true)?,"normal":o["normal"],"closed":value(o,"closed",json!(false)),"miter_limit":value(o,"miterLimit",json!(4)),"initial_steps":value(o,"initialSteps",json!(1)),"max_steps":value(o,"maxSteps",json!(64)),"max_deviation":o["maxDeviation"]});
    for (source, target) in [
        ("axisScale", "axis_scale"),
        ("centerLaw", "center_law"),
        ("frameAxis", "frame_axis"),
        ("frameNormal", "frame_normal"),
    ] {
        if !o[source].is_null() {
            request[target] = law(&o[source], false)?;
        }
    }
    if !o["orientationGuide"].is_null() {
        request["orientation_guide"] = o["orientationGuide"].clone();
    }
    Ok(request)
}
fn partition(rows: &Value, sizes: &[usize]) -> Result<Value> {
    let mut nested = Vec::new();
    for row in rows
        .as_array()
        .ok_or_else(|| input("Missing sweep sections"))?
    {
        let row = row
            .as_array()
            .ok_or_else(|| input("Invalid sweep section"))?;
        if row.len() != sizes.iter().sum() {
            return Err(input("Sweep section profile count changed"));
        }
        let mut offset = 0;
        let mut loops = Vec::new();
        for &size in sizes {
            loops.push(json!(row[offset..offset + size].to_vec()));
            offset += size;
        }
        nested.push(json!(loops));
    }
    Ok(json!(nested))
}
fn cap_faces(model: &Value, closed: bool) -> Result<Value> {
    if closed {
        return Ok(json!([]));
    }
    let n = model["faces"]
        .as_array()
        .ok_or_else(|| input("Missing B-rep faces"))?
        .len();
    if n < 2 {
        return Err(input("Missing endpoint cap faces"));
    }
    Ok(json!([n - 2, n - 1]))
}
#[derive(Clone)]
struct Owner {
    model: Value,
    certificate: Value,
    sections: Option<Value>,
    sharp: Option<Value>,
    authoring: Value,
}
thread_local! {static OWNERS:std::cell::RefCell<(u64,std::collections::BTreeMap<u64,Owner>)>=std::cell::RefCell::new((0,std::collections::BTreeMap::new()));}
fn own(
    mut body: Value,
    sections: Option<Value>,
    sharp: Option<Value>,
    authoring: Value,
) -> Result<Value> {
    let owner = Owner {
        model: body["model"].clone(),
        certificate: body["boundaryCertificate"].clone(),
        sections,
        sharp,
        authoring,
    };
    let id = OWNERS.with(|store| {
        let mut store = store.borrow_mut();
        if store.1.len() >= 4096 {
            return Err(input("Native sweep ownership budget exhausted"));
        }
        store.0 = store
            .0
            .checked_add(1)
            .ok_or_else(|| input("Native sweep owner identifiers exhausted"))?;
        let id = store.0;
        store.1.insert(id, owner);
        Ok(id)
    })?;
    body["_nativeOwner"] = json!(id.to_string());
    Ok(body)
}
fn equivalent(a: &Value, b: &Value) -> bool {
    if a.is_number() && b.is_number() {
        return a.as_f64() == b.as_f64();
    }
    match (a.as_array(), b.as_array(), a.as_object(), b.as_object()) {
        (Some(a), Some(b), _, _) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| equivalent(a, b))
        }
        (_, _, Some(a), Some(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(k, a)| b.get(k).is_some_and(|b| equivalent(a, b)))
        }
        _ => a == b,
    }
}
fn owner(v: &Value) -> Result<Owner> {
    let id = field::<String>(v, "owner")
        .map_err(|_| input("Sweep requires constructor-owned native evidence"))?
        .parse::<u64>()
        .map_err(|_| input("Invalid native sweep owner"))?;
    let owner = OWNERS
        .with(|store| store.borrow().1.get(&id).cloned())
        .ok_or_else(|| input("Sweep requires constructor-owned native evidence"))?;
    if !equivalent(&owner.model, &v["source"]["model"])
        || !equivalent(&owner.certificate, &v["source"]["boundaryCertificate"])
    {
        return Err(input(
            "Sweep requires unchanged constructor-owned boundary evidence",
        ));
    }
    Ok(owner)
}
pub fn release(v: Value) -> Result<Value> {
    let id = field::<String>(&v, "owner")?
        .parse::<u64>()
        .map_err(|_| input("Invalid native sweep owner"))?;
    OWNERS.with(|store| {
        store.borrow_mut().1.remove(&id);
    });
    Ok(Value::Null)
}
pub fn progressive_miter(v: Value) -> Result<Value> {
    progressive_miter_from(v, None)
}
fn progressive_miter_from(v: Value, accepted: Option<Value>) -> Result<Value> {
    let loops: Vec<Vec<Value>> = field(&v, "loops")?;
    if loops.is_empty() || loops.len() > 16 || loops.iter().any(Vec::is_empty) {
        return Err(input("Miter body needs 1..16 nonempty loops"));
    }
    let profiles = json!(loops.iter().flatten().cloned().collect::<Vec<_>>());
    let sizes: Vec<_> = loops.iter().map(Vec::len).collect();
    let o = &v["options"];
    let closed = yes(o, "closed");
    let points: Vec<[f64; 3]> = field(&v, "points")?;
    let edges = points
        .len()
        .checked_sub(usize::from(!closed))
        .ok_or_else(|| input("Invalid miter sites"))?;
    let mut spans = 0;
    for profile in profiles.as_array().unwrap() {
        spans += nurbs("curve_decompose", json!({"curve":profile}))?
            .as_array()
            .ok_or_else(|| input("Invalid native decomposition"))?
            .len();
    }
    if edges < 1 || spans < 1 || spans > 64 {
        return Err(input("Progressive miter body exceeds its site/span budget"));
    }
    let mut request = miter_request(&v, &profiles)?;
    let maximum = field::<usize>(&request, "max_steps")?
        .min(1024 / edges)
        .min((1024 - if closed { 0 } else { 2 }) / (edges * spans));
    if maximum < field::<usize>(&request, "initial_steps")? {
        return Err(input("Progressive miter initial steps exceed face budget"));
    }
    request["max_steps"] = json!(maximum);
    let approximation = match accepted {
        Some(level) => level,
        None => nurbs("curve_progressive_miter", request.clone())?,
    };
    let report = &approximation["report"];
    if report["frameTransportCertified"] == json!(false) {
        return Err(input(
            "Progressive miter frame transport could not be proved; review the path, normal and miter limit",
        ));
    }
    if report["certifiedErrorUpper"].is_null() {
        return Err(input(format!(
            "Progressive miter wall error bound could not be proved: {}",
            report["errorCertificateReason"]
        )));
    }
    if report["profileRegularityCertified"] == json!(false) {
        return Err(input(
            "Progressive miter profile tangent regularity could not be proved",
        ));
    }
    if report["wallRegularityCertified"] == json!(false) {
        return Err(input(
            "Progressive miter retained wall Jacobian regularity could not be proved",
        ));
    }
    if !yes(report, "accepted") || approximation["sections"].is_null() {
        return Err(input(
            "Progressive miter refinement/phase budget not met within the body face budget",
        ));
    }
    let mut correction =
        correct(json!({"sections":approximation["sections"],"points":points,"options":o}))?;
    let retained = if correction.is_null() {
        approximation["sections"].clone()
    } else {
        correction["sections"].clone()
    };
    if let Some(fields) = correction.as_object_mut() {
        fields.remove("sections");
    }
    let wall_defaults = json!({"clearance":0.,"distanceTolerance":0.001,"maxInjectivityCells":1000,"maxPairs":1000,"maxPairCells":1000});
    let wall_request = merge(
        merge(
            request.clone(),
            &json!({"sections":retained,"loopSizes":sizes}),
        ),
        &value(o, "wallAuditBudgets", wall_defaults.clone()),
    );
    let wall_audit = nurbs("curve_progressive_miter_wall_audit", wall_request)?;
    let sections = partition(&retained, &sizes)?;
    let model = call(
        if closed {
            "brep_nurbs_periodic_section_loft"
        } else {
            "brep_nurbs_rational_section_loft"
        },
        json!({"sections":sections}),
    )?;
    let mut cap_domains = Value::Null;
    if !closed {
        let rows = sections.as_array().unwrap();
        let budgets = value(
            o,
            "contourAuditBudgets",
            json!({"tolerance":0.001,"maxPairs":1000,"maxCells":1000}),
        );
        let mut reports = Vec::new();
        for row in [&rows[0], rows.last().unwrap()] {
            let mut req = merge(json!({"loops":row}), &budgets);
            req["maxExactWork"] = json!(1000000);
            reports.push(nurbs("sweep_contour_audit", req)?);
        }
        cap_domains = json!(reports);
    }
    let correspondence = call(
        "brep_sweep_retained_correspondence_audit",
        json!({"model":model,"sections":sections,"closed":closed,"maxFaces":1024,"maxExactWork":1000000}),
    )?;
    let decomposition = if yes(&correspondence, "exact") {
        Value::Null
    } else {
        call(
            "brep_sweep_retained_decomposition_audit",
            json!({"model":model,"sections":sections,"closed":closed,"maxProducts":value(&o["retainedDecompositionBudgets"],"maxProducts",json!(100000)),"maxFaces":value(&o["retainedDecompositionBudgets"],"maxFaces",json!(1024)),"maxExactWork":1000000}),
        )?
    };
    let decomp_error = if yes(&correspondence, "exact") {
        json!(0)
    } else {
        decomposition["wallErrorUpper"].clone()
    };
    if !correction.is_null() && decomp_error.is_null() {
        return Err(input(
            "Corrected progressive miter retained wall correspondence unproved",
        ));
    }
    let wall_error = add(
        add(
            report["certifiedErrorUpper"].clone(),
            value(&correction, "wallDisplacementUpper", json!(0)),
        )?,
        decomp_error,
    )?;
    let budget: f64 = field(o, "maxDeviation")?;
    if !correction.is_null() && (wall_error.is_null() || wall_error.as_f64().unwrap() > budget) {
        return Err(input(
            "Corrected progressive miter retained wall error exceeds max_deviation or is unproved",
        ));
    }
    let caps = cap_faces(&model, closed)?;
    let volume_budgets = value(o, "volumeBudgets", volume_budgets());
    let cap_budgets = &volume_budgets["capBudgets"];
    let rows = sections.as_array().unwrap();
    let endpoints = json!([rows[0], rows.last().unwrap()]);
    let retained_caps = if closed {
        Value::Null
    } else {
        call(
            "brep_sweep_retained_caps_audit",
            merge(
                json!({"model":model,"endpoints":endpoints,"maxEdges":1024}),
                cap_budgets,
            ),
        )?
    };
    let cap_decomposition = if closed || yes(&retained_caps, "exact") {
        Value::Null
    } else {
        call(
            "brep_sweep_retained_cap_decomposition_audit",
            merge(
                json!({"model":model,"endpoints":endpoints,"maxProducts":value(&o["retainedDecompositionBudgets"],"maxProducts",json!(100000)),"maxEdges":1024}),
                cap_budgets,
            ),
        )?
    };
    if !correction.is_null()
        && !closed
        && !yes(&retained_caps, "exact")
        && !yes(&cap_decomposition, "certified")
    {
        return Err(input(
            "Corrected progressive miter filled retained cap regions unproved",
        ));
    }
    let ideal_domains = if closed {
        Value::Null
    } else {
        nurbs(
            "curve_progressive_miter_cap_domains",
            merge(
                merge(request.clone(), &json!({"loopSizes":sizes})),
                &value(
                    o,
                    "capDomainBudgets",
                    json!({"tolerance":0.001,"maxPairs":1000,"maxCells":10000,"maxExactWork":1000000}),
                ),
            ),
        )?
    };
    let mut projection = Value::Null;
    let mut parallelism = Value::Null;
    if !closed {
        let faces = model["faces"].as_array().unwrap();
        let req = merge(
            request.clone(),
            &json!({"caps":[faces[faces.len()-2]["surface"],faces[faces.len()-1]["surface"]],"maxCells":value(&o["capProjectionBudgets"],"maxCells",json!(10000)),"maxExactWork":value(&o["capProjectionBudgets"],"maxExactWork",json!(1000000))}),
        );
        projection = nurbs("curve_progressive_miter_cap_projection", req.clone())?;
        parallelism = nurbs("curve_progressive_miter_cap_parallelism", req)?;
    }
    let cap_error = if closed {
        Value::Null
    } else {
        nurbs("sweep_filled_cap_error_upper",json!({"parallelPlanesCertified":parallelism["parallel"],"idealCapDomainsCertified":yes(&ideal_domains,"idealCapDomainsCertified"),"retainedCapRegionsExact":yes(&retained_caps,"exact")||yes(&cap_decomposition,"certified"),"decompositionErrorUpper":if yes(&retained_caps,"exact"){json!([0,0])}else{cap_decomposition["capErrorUpper"].clone()},"projectionNormalDots":projection["normalDots"],"endpointContourErrorUpper":report["endpointContourErrorUpper"],"correctionDisplacementUpper":value(&correction,"wallDisplacementUpper",json!(0))}))?["capErrorUpper"].clone()
    };
    let certificate = nurbs(
        "sweep_boundary_certificate",
        json!({"wall":wall_error,"caps":cap_error,"closed":closed,"budget":budget}),
    )?;
    if certificate["withinBudget"] == json!(false) {
        return Err(input(
            "Progressive miter complete boundary error exceeds max_deviation",
        ));
    }
    let charts = call(
        "brep_sweep_retained_wall_charts_audit",
        json!({"model":model,"capFaces":caps,"maxCells":value(o,"retainedWallMaxInjectivityCells",json!(1000))}),
    )?;
    if !correction.is_null() && !yes(&charts, "allChartsCertified") {
        return Err(input(
            "Corrected progressive miter retained wall regularity unproved",
        ));
    }
    let cap_pairs = if closed {
        Value::Null
    } else {
        call(
            "brep_sweep_cap_pairs_audit",
            json!({"model":model,"capFaces":caps,"budgets":value(o,"capPairAuditBudgets",wall_defaults)}),
        )?
    };
    let contacts = if closed {
        Value::Null
    } else {
        call(
            "brep_sweep_cap_evidence_audit",
            json!({"model":model,"capFaces":caps,"maxWalls":value(o,"capWallMaxWalls",json!(1024)),"boundaryOptions":{"tolerance":1e-9,"maxProducts":100000,"maxCells":1000,"maxWork":1000000}}),
        )?
    };
    let mut embedding_budgets = volume_budgets.clone();
    for (k, val) in [
        ("maxTrimPairs", 1000),
        ("maxTrimCells", 10000),
        ("maxTrimDomainCells", 100000),
        ("maxLinearCells", 1000),
        ("maxPairs", 1000),
        ("maxCells", 10000),
        ("maxDomainCells", 100000),
        ("cellsPerPair", 16),
        ("domainCellsPerPair", 1000),
    ] {
        embedding_budgets[k] = json!(val);
    }
    let embedding = if closed {
        Value::Null
    } else {
        call(
            "brep_sweep_embedding_audit",
            merge(
                json!({"model":model,"capFaces":caps}),
                &value(o, "embeddingBudgets", embedding_budgets),
            ),
        )?
    };
    let volume = call(
        "brep_sweep_volume_audit",
        merge(json!({"model":model,"capFaces":caps}), &volume_budgets),
    )?;
    let smoothness = call(
        "brep_miter_profile_smoothness_audit",
        json!({"model":model,"capFaces":caps,"maxWork":2000000}),
    )?;
    let authoring = json!({"authoredFramesApplied":report["authoredFramesApplied"],"orientationGuideApplied":report["orientationGuideApplied"],"affineLawsApplied":report["affineLawsApplied"]});
    let steps: usize = field(report, "steps")?;
    if rows.len() != edges * steps + 1 {
        return Err(input(
            "Miter station ownership requires complete uniform span coverage",
        ));
    }
    let sharp: Vec<_> = (0..if closed { edges } else { edges - 1 })
        .map(|i| (i + usize::from(!closed)) * steps)
        .collect();
    let mut body = json!({"model":model,"approximation":approximation,"profileSmoothness":smoothness,"boundaryCertificate":certificate,"retainedCapDecomposition":cap_decomposition,"retainedDecomposition":decomposition,"capParallelism":parallelism,"boundaryErrorWithinBudget":certificate["withinBudget"],"boundaryErrorUpper":certificate["errorUpper"],"filledCapErrorUpper":cap_error,"idealCapDomains":ideal_domains,"capProjection":projection,"retainedWallErrorUpper":wall_error,"wallAudit":wall_audit,"retainedCorrespondence":correspondence,"retainedCaps":retained_caps,"retainedWallCharts":charts,"capDomains":cap_domains,"capContacts":contacts,"capPairs":cap_pairs,"embedding":embedding,"volume":volume,"globalEmbeddingCertified":false});
    if !correction.is_null() {
        body["sectionCorrection"] = correction;
    }
    own(body, Some(sections), Some(json!(sharp)), authoring)
}
pub fn miter(v: Value) -> Result<Value> {
    let loops: Vec<Vec<Value>> = field(&v, "loops")?;
    if loops.is_empty() || loops.len() > 16 || loops.iter().any(Vec::is_empty) {
        return Err(input("Miter body needs 1..16 nonempty loops"));
    }
    let profiles = json!(loops.iter().flatten().cloned().collect::<Vec<_>>());
    let sizes: Vec<_> = loops.iter().map(Vec::len).collect();
    let closed = yes(&v, "closed");
    let sections = nurbs(
        "curve_miter_sections",
        json!({"profiles":profiles,"points":v["points"],"normal":v["normal"],"miter_limit":value(&v,"miterLimit",json!(4)),"closed":closed}),
    )?;
    let mut correction = if v["capCorrection"].is_null() {
        Value::Null
    } else {
        cap_correction(
            &sections,
            &field::<Vec<[f64; 3]>>(&v, "points")?,
            closed,
            &v["capCorrection"],
        )?
    };
    let retained = if correction.is_null() {
        sections
    } else {
        correction["sections"].clone()
    };
    if let Some(fields) = correction.as_object_mut() {
        fields.remove("sections");
    }
    let nested = partition(&retained, &sizes)?;
    let model = call(
        if closed {
            "brep_nurbs_periodic_section_loft"
        } else {
            "brep_nurbs_rational_section_loft"
        },
        json!({"sections":nested}),
    )?;
    let correspondence = if correction.is_null() {
        Value::Null
    } else {
        call(
            "brep_sweep_retained_correspondence_audit",
            json!({"model":model,"sections":nested,"closed":closed,"maxFaces":1024,"maxExactWork":1000000}),
        )?
    };
    if !correction.is_null() && !yes(&correspondence, "exact") {
        return Err(input(
            "Corrected miter retained wall correspondence unproved",
        ));
    }
    let caps = cap_faces(&model, closed)?;
    let volume = call(
        "brep_sweep_volume_audit",
        merge(json!({"model":model,"capFaces":caps}), &volume_budgets()),
    )?;
    let regularity = nurbs(
        "sweep_profile_regularity_audit",
        json!({"profiles":profiles,"maxCells":10000}),
    )?;
    let charts = call(
        "brep_sweep_retained_wall_charts_audit",
        json!({"model":model,"capFaces":caps,"maxCells":20000}),
    )?;
    let smoothness = call(
        "brep_miter_profile_smoothness_audit",
        json!({"model":model,"capFaces":caps,"maxWork":2000000}),
    )?;
    let mut report = json!({"profileSmoothness":smoothness,"method":"polyline-miter-sections","sections":retained.as_array().unwrap().len(),"closedPath":closed,"globalEmbeddingCertified":false,"roundingCertified":false,"continuousBound":false,"profileRegularityCertified":regularity["spanwiseRegular"],"profileRegularity":regularity,"wallRegularityCertified":charts["allChartsCertified"],"retainedWallCharts":charts,"volume":volume});
    if !correction.is_null() {
        report["sectionCorrection"] = correction;
        report["retainedCorrespondence"] = correspondence;
    }
    Ok(json!({"model":model,"report":report}))
}
fn audit_placed(model: &Value, certificate: &Value) -> Result<(Value, Value, Value)> {
    let caps = cap_faces(model, yes(certificate, "closed"))?;
    let smoothness = call(
        "brep_miter_profile_smoothness_audit",
        json!({"model":model,"capFaces":caps,"maxWork":2000000}),
    )?;
    let charts = call(
        "brep_sweep_retained_wall_charts_audit",
        json!({"model":model,"capFaces":caps,"maxCells":100000}),
    )?;
    let volume = call(
        "brep_sweep_volume_audit",
        merge(json!({"model":model,"capFaces":caps}), &volume_budgets()),
    )?;
    if !yes(&charts, "allChartsCertified") || !yes(&volume, "solidGeometryCertified") {
        return Err(input("Reconstructed Solid geometry unproved"));
    }
    Ok((smoothness, charts, volume))
}
fn complete(certificate: &Value, message: &str) -> Result<()> {
    if !yes(certificate, "continuousBound") || !yes(certificate, "withinBudget") {
        return Err(input(message));
    }
    Ok(())
}
fn boundary_fields(body: Value, certificate: &Value) -> Value {
    merge(
        body,
        &json!({"boundaryCertificate":certificate,"continuousBound":certificate["continuousBound"],"boundaryErrorUpper":certificate["errorUpper"],"boundaryErrorWithinBudget":certificate["withinBudget"],"budget":certificate["budget"],"filledCapErrorUpper":certificate["filledCapErrorUpper"],"retainedWallErrorUpper":certificate["wallErrorUpper"],"globalEmbeddingCertified":false}),
    )
}
pub fn transform(v: Value) -> Result<Value> {
    let source = owner(&v)?;
    let o = &v["options"];
    complete(
        &source.certificate,
        "Affine source complete boundary bound unproved",
    )?;
    let placement = call(
        "brep_nurbs_affine_lattice",
        json!({"model":source.model,"matrix":v["matrix"],"quantum":o["quantum"],"maxWork":o["maxWork"]}),
    )?;
    if placement["model"].is_null()
        || placement["operatorNormUpper"].is_null()
        || placement["arithmeticErrorUpper"].as_f64() != Some(0.)
    {
        return Err(input(format!(
            "Exact affine placement unproved: {}",
            placement["reason"]
        )));
    }
    let scale = |upper: &Value| -> Result<Value> {
        if upper.is_null() {
            Ok(Value::Null)
        } else {
            Ok(nurbs(
                "sweep_error_upper_compose",
                json!({"kind":"multiply","a":placement["operatorNormUpper"],"b":upper}),
            )?["errorUpper"]
                .clone())
        }
    };
    let caps = if let Some(caps) = source.certificate["filledCapErrorUpper"].as_array() {
        let values = caps.iter().map(scale).collect::<Result<Vec<_>>>()?;
        if values.iter().any(Value::is_null) {
            Value::Null
        } else {
            json!(values)
        }
    } else {
        Value::Null
    };
    let certificate = nurbs(
        "sweep_boundary_certificate",
        json!({"wall":scale(&source.certificate["wallErrorUpper"])?,"caps":caps,"closed":source.certificate["closed"],"budget":o["maxDeviation"]}),
    )?;
    complete(
        &certificate,
        "Affine complete boundary error exceeds max_deviation or is unproved",
    )?;
    let model = &placement["model"];
    let (smoothness, charts, volume) = audit_placed(model, &certificate)?;
    let body = boundary_fields(
        json!({"model":model,"placement":placement,"profileSmoothness":smoothness,"retainedWallCharts":charts,"volume":volume,"wallRegularityCertified":charts["allChartsCertified"],"profileRegularityCertified":charts["allChartsCertified"]}),
        &certificate,
    );
    own(body, None, None, Value::Null)
}
pub fn smooth(mut v: Value, reconstruct: bool) -> Result<Value> {
    let source = owner(&v)?;
    let o = v["options"].clone();
    if reconstruct {
        v["sections"] = source.sections.clone().ok_or_else(|| {
            input("Station reconstruction requires constructor-owned retained sections")
        })?;
        v["sharp"] = source.sharp.clone().ok_or_else(|| {
            input("Station reconstruction requires constructor-owned retained sections")
        })?;
    }
    complete(
        &source.certificate,
        "Station smoothing source complete boundary bound unproved",
    )?;
    if source.certificate["wallErrorUpper"].is_null() {
        return Err(input(
            "Station smoothing source complete boundary bound unproved",
        ));
    }
    let closed = yes(&source.certificate, "closed");
    let identity = call(
        "brep_nurbs_section_loft_source_audit",
        json!({"model":source.model,"sections":v["sections"],"closed":closed}),
    )?;
    if !yes(&identity, "geometryAndTopologyIdentical") {
        return Err(input(
            "Station smoothing sections do not reproduce the certified source body",
        ));
    }
    let candidate = call(
        "brep_nurbs_smooth_station_walls",
        json!({"sections":v["sections"],"sharp":v["sharp"],"closed":closed,"quantum":o["quantum"],"tolerance":o["wallTolerance"],"maxWork":o["maxWork"]}),
    )?;
    if candidate["sides"].is_null() || candidate["wallDisplacementUpper"].is_null() {
        return Err(input(format!(
            "Station smoothing candidate unproved: {}",
            candidate["reason"]
        )));
    }
    let certificate = nurbs(
        "sweep_boundary_certificate",
        json!({"wall":add(source.certificate["wallErrorUpper"].clone(),candidate["wallDisplacementUpper"].clone())?,"caps":source.certificate["filledCapErrorUpper"],"closed":closed,"budget":o["maxDeviation"]}),
    )?;
    complete(
        &certificate,
        "Smoothed complete boundary error exceeds max_deviation or is unproved",
    )?;
    let model = call(
        "brep_nurbs_section_loft_surfaces",
        json!({"sections":v["sections"],"sides":candidate["sides"],"closed":closed}),
    )?;
    let caps = cap_faces(&model, closed)?;
    for face in caps.as_array().unwrap() {
        let i = face.as_u64().unwrap() as usize;
        if !equivalent(&model["faces"][i], &source.model["faces"][i]) {
            return Err(input("Station smoothing changed a filled cap"));
        }
        let f = &model["faces"][i];
        let wires = std::iter::once(&f["outer"]).chain(f["holes"].as_array().unwrap());
        for wire in wires {
            let wire = wire.as_u64().unwrap() as usize;
            if !equivalent(&model["loops"][wire], &source.model["loops"][wire]) {
                return Err(input("Station smoothing changed cap trims"));
            }
            for coedge in model["loops"][wire]["coedges"].as_array().unwrap() {
                let edge = coedge["edge"].as_u64().unwrap() as usize;
                if !equivalent(&model["edges"][edge], &source.model["edges"][edge]) {
                    return Err(input("Station smoothing changed a cap boundary edge"));
                }
                for vertex in model["edges"][edge]["vertices"].as_array().unwrap() {
                    let vertex = vertex.as_u64().unwrap() as usize;
                    if !equivalent(
                        &model["vertices"][vertex],
                        &source.model["vertices"][vertex],
                    ) {
                        return Err(input("Station smoothing changed a cap boundary vertex"));
                    }
                }
            }
        }
    }
    let (smoothness, charts, volume) = audit_placed(&model, &certificate)?;
    let body = boundary_fields(
        merge(
            json!({"method":"bounded-miter-station-reconstruction","model":model,"candidate":candidate,"sharpStationIndices":v["sharp"],"profileSmoothness":smoothness,"retainedWallCharts":charts,"volume":volume,"wallRegularityCertified":charts["allChartsCertified"],"profileRegularityCertified":charts["allChartsCertified"]}),
            &source.authoring,
        ),
        &certificate,
    );
    own(body, None, None, Value::Null)
}
fn profile_request(v: &Value, profiles: &Value) -> Result<Value> {
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
    if !o["orientationGuide"].is_null() {
        req["orientation_guide"] = o["orientationGuide"].clone();
        if !o["contactAnchor"].is_null() {
            req["contact_profile"] = value(&o["contactAnchor"], "profileIndex", json!(0));
            req["contact_parameter"] = o["contactAnchor"]["parameter"].clone();
        }
    }
    Ok(req)
}
fn bounded_request(v: &Value, miter: bool) -> Result<(Value, Vec<usize>, usize)> {
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
fn profile_body_from(v: Value, accepted: Option<Value>, prepared: Option<Value>) -> Result<Value> {
    use nurbs_core::{
        curve::Curve,
        progressive_sweep::{MultiSweep, Options, Orientation, Spacing},
    };
    let (request, sizes) = if let Some(request) = prepared {
        let loops: Vec<Vec<Value>> = field(&v, "loops")?;
        (request, loops.iter().map(Vec::len).collect())
    } else {
        let (request, sizes, _) = bounded_request(&v, false)?;
        (request, sizes)
    };
    let approximation = match accepted {
        Some(level) => level,
        None => nurbs("surface_progressive_sweep_profiles", request.clone())?,
    };
    let report = &approximation["report"];
    if !yes(report, "accepted") {
        return Err(input("Progressive body sampled refinement exceeds budget"));
    }
    let profiles: Vec<Curve> = field(&request, "profiles")?;
    let path: Curve = field(&request, "path")?;
    let scale: Curve = field(&request, "scale")?;
    let twist: Curve = field(&request, "twist")?;
    let orientation = match field::<String>(&request, "orientation")?.as_str() {
        "rmf" => Orientation::RotationMinimizing,
        "fixed" | "authored" => Orientation::Fixed,
        "fixed_normal" => Orientation::FixedNormal,
        "frenet" => Orientation::Frenet,
        "corrected_frenet" => Orientation::CorrectedFrenet,
        _ => return Err(input("Unknown sweep orientation")),
    };
    let spacing = match field::<String>(&request, "spacing")?.as_str() {
        "parameter" => Spacing::Parameter,
        "arc_length" => Spacing::ArcLength {
            tolerance: field(&request, "length_tolerance")?,
            max_cells: field(&request, "length_max_cells")?,
        },
        _ => return Err(input("Unknown sweep station spacing")),
    };
    let options = Options {
        normal: field(&request, "normal")?,
        orientation,
        spacing,
        initial_sections: field(&request, "initial_sections")?,
        max_sections: field(&request, "max_sections")?,
        max_deviation: field(&request, "max_deviation")?,
    };
    let constant = |v: [f64; 3]| {
        field::<Curve>(
            &json!({"curve":{"degree":1,"knots":[0.,0.,1.,1.],"controlPoints":[v,v],"weights":[1.,1.],"periodic":false}}),
            "curve",
        )
    };
    let axes = if request["axis_scale"].is_null() {
        constant([1.; 3])?
    } else {
        field(&request, "axis_scale")?
    };
    let center = if request["center_law"].is_null() {
        constant([0.; 3])?
    } else {
        field(&request, "center_law")?
    };
    let frame_axis = if request["frame_axis"].is_null() {
        None
    } else {
        Some(field::<Curve>(&request, "frame_axis")?)
    };
    let frame_normal = if request["frame_normal"].is_null() {
        None
    } else {
        Some(field::<Curve>(&request, "frame_normal")?)
    };
    let guide = if request["orientation_guide"].is_null() {
        None
    } else {
        Some(field::<Curve>(&request, "orientation_guide")?)
    };
    let authored = request["orientation"] == json!("authored");
    let mut sweep = MultiSweep::new(&profiles, &path, &scale, &twist, options)?;
    if let Some(g) = guide.as_ref() {
        sweep = if request["contact_parameter"].is_null() {
            sweep.with_orientation_guide(g)?
        } else {
            sweep.with_contact_guide(
                g,
                value(&request, "contact_profile", json!(0))
                    .as_u64()
                    .ok_or_else(|| input("Invalid contact profile"))? as usize,
                field(&request, "contact_parameter")?,
            )?
        };
    }
    if authored {
        sweep = sweep.with_frame_laws(
            frame_axis
                .as_ref()
                .ok_or_else(|| input("Authored sweep requires frameAxis"))?,
            frame_normal
                .as_ref()
                .ok_or_else(|| input("Authored sweep requires frameNormal"))?,
        )?;
    }
    if authored
        || guide.is_some()
        || !request["axis_scale"].is_null()
        || !request["center_law"].is_null()
    {
        sweep = sweep.with_affine_laws(&axes, &center)?;
    }
    let count: usize = field(report, "sections")?;
    let by_profile = sweep.sections_at(count)?;
    let rows: Vec<Value> = (0..count)
        .map(|station| {
            json!(
                by_profile
                    .iter()
                    .map(|profile| profile[station].clone())
                    .collect::<Vec<_>>()
            )
        })
        .collect();
    let sections = partition(&json!(rows), &sizes)?;
    let closed = yes(report, "closedPath");
    let model = call(
        if closed {
            "brep_nurbs_periodic_section_loft"
        } else {
            "brep_nurbs_rational_section_loft"
        },
        json!({"sections":sections}),
    )?;
    Ok(json!({"model":model,"approximation":approximation,"globalEmbeddingCertified":false}))
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

fn miter_wall_preview(preview: Value) -> Result<Value> {
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
        return nurbs(
            &operation,
            json!({"profile":v["profile"],"path":v["path"],"scale":law(&v["scale"],false)?,"origin":v["origin"],"normal":v["normal"],"sections":v["sections"],"max_deviation":v["maxDeviation"]}),
        );
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
