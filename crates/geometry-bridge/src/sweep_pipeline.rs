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
    if !["brep_progressive_sweep", "brep_progressive_miter_sweep", "brep_miter_sweep"].contains(&op) {
        return Ok(Value::Null);
    }
    let model: brep_core::Model = field(&v, "model")?;
    // A failed exact prerequisite cannot be rescued by pair searches. Reject
    // it before spending their budget, while charging the replay below to the
    // remaining exact-boundary work allowance.
    let mut budgets = volume_budgets();
    let exact_budget: u64 = field(&budgets, "maxExactWork")?;
    let agreement = brep_core::boundary_agreement::verify_exact(&model, exact_budget)?;
    if !agreement.all_equal || !agreement.all_joins_exact {
        return Err(input("Sweep Solid geometry could not be proved: exact boundary agreement."));
    }
    let remaining = exact_budget.saturating_sub(agreement.work);
    if remaining == 0 {
        return Err(input("Sweep Solid geometry could not be proved: exact boundary work budget."));
    }
    budgets["maxExactWork"] = json!(remaining);
    // Profile closure is constructor-derived, not a trusted author-supplied flag.
    // The fresh full-model proof covers every actual face and trimmed domain.
    let caps = if op == "brep_progressive_sweep" || node["closed"] == json!(true) {
        vec![]
    } else {
        if model.faces.len() < 2 {
            return Err(input("Sweep requires endpoint caps"));
        }
        vec![model.faces.len() - 2, model.faces.len() - 1]
    };
    let report = call(
        "brep_sweep_volume_audit",
        merge(json!({"model":model,"capFaces":caps}), &budgets),
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
            if op == "brep_progressive_sweep" || op == "brep_progressive_miter_sweep" {
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
mod miter;
mod profile;
pub(super) use miter::*;
pub(super) use profile::*;

#[cfg(test)]
#[path="tests/sweep_profile_admission.rs"]
mod profile_admission_tests;
