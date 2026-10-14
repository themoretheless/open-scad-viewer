//! Validate finite public sweep evidence before the host displays its claims.
//! This does not replace native geometric Solid admission.
use crate::{Result, field};
use std::collections::BTreeSet;
use value_codec::{Value, json};
fn yes(v: &Value, k: &str) -> bool {
    v[k] == json!(true)
}
fn named(v: &Value, k: &str, s: &str) -> bool {
    v[k].as_str() == Some(s)
}
fn count(v: &Value) -> Option<u64> {
    let x = v.as_f64()?;
    (x.is_finite() && x >= 0. && x.fract() == 0. && x <= 9007199254740991.).then_some(x as u64)
}
fn len(v: &Value) -> Option<usize> {
    Some(v.as_array()?.len())
}
fn empty(v: &Value) -> bool {
    len(v) == Some(0)
}
fn bound(v: &Value) -> Option<f64> {
    v.as_f64().filter(|x| x.is_finite() && *x >= 0.)
}
fn extraction(v: &Value) -> bool {
    let Some(edges) = v["edgeIds"].as_array() else {
        return false;
    };
    let ids: Option<Vec<_>> = edges.iter().map(count).collect();
    yes(v, "extractionComplete")
        && empty(&v["unclassifiedFaces"])
        && empty(&v["unpairedEdges"])
        && !edges.is_empty()
        && ids.is_some_and(|ids| ids.iter().collect::<BTreeSet<_>>().len() == ids.len())
}
fn audit(
    v: &Value,
    order: u64,
    budget: u64,
    edges: usize,
    complete: bool,
    allow_higher: bool,
) -> bool {
    complete
        && named(v, "method", "constant-projective-strip-jets")
        && yes(v, "exactG1G2Certified")
        && count(&v["certifiedOrder"]) == Some(order)
        && count(&v["exactWork"]).is_some_and(|x| x <= budget)
        && len(&v["seams"]) == Some(edges)
        && empty(&v["unresolvedSeams"])
        && v["seams"].as_array().is_some_and(|seams| {
            seams.iter().all(|s| {
                yes(s, "certified")
                    && yes(s, "exactIdentity")
                    && yes(s, "regularityCertified")
                    && (count(&s["certifiedOrder"]) == Some(order)
                        || allow_higher && order == 1 && count(&s["certifiedOrder"]) == Some(2))
            })
        })
}
pub fn read(v: Value) -> Result<Value> {
    let a = &v["artifact"];
    if !named(a, "kind", "brep") {
        return Ok(Value::Null);
    }
    let Ok(text) = field::<String>(a, "geometryJson") else {
        return Ok(Value::Null);
    };
    let Ok(document) = value_codec::from_str::<Value>(&text) else {
        return Ok(Value::Null);
    };
    // Profile-body source error has its own scope and display contract. Its
    // accompanying volume report is not a retained-miter boundary certificate.
    if document["sweepBodyBoundaryEvidence"].is_object() {
        return Ok(Value::Null);
    }
    let e = &document["sweepEvidence"];
    if !e.is_object() || e["volume"]["solidGeometryCertified"].as_bool().is_none() {
        return Ok(Value::Null);
    }
    let upper = bound(&e["boundaryErrorUpper"]);
    let budget = bound(&e["boundaryErrorBudget"]);
    let within = upper.zip(budget).and_then(|(upper, budget)| {
        e["boundaryErrorWithinBudget"]
            .as_bool()
            .filter(|&within| within == (upper <= budget))
    });
    let certificate = &e["boundaryCertificate"];
    let complete_bound = yes(e, "continuousBound")
        && named(certificate, "method", "retained-sweep-boundary-union")
        && named(certificate, "scope", "boundary-set-hausdorff")
        && yes(certificate, "continuousBound")
        && upper.is_some()
        && budget.is_some()
        && bound(&certificate["errorUpper"]) == upper
        && bound(&certificate["budget"]) == budget
        && within.is_some()
        && certificate["withinBudget"].as_bool() == within;
    let mut result = json!({"nodeId":a["nodeId"],"solidGeometryCertified":e["volume"]["solidGeometryCertified"],"boundaryErrorUpper":upper,"boundaryErrorBudget":budget,"boundaryErrorWithinBudget":within,"continuousBound":complete_bound,"profileRegularityCertified":yes(e,"profileRegularityCertified"),"wallRegularityCertified":yes(e,"wallRegularityCertified")});
    let s = &e["profileSmoothness"];
    let p = &s["profile"];
    let declared = named(s, "method", "retained-miter-profile-joins")
        && named(s, "scope", "wall-profile-seams")
        && s["edgeIds"].is_array()
        && p["seams"].is_array();
    if !declared {
        return Ok(result);
    }
    let n = len(&s["edgeIds"]).unwrap();
    let complete = extraction(s);
    let maximum = if s.get("maxWork").is_none() {
        Some(1000000)
    } else {
        count(&s["maxWork"])
    }
    .filter(|&x| x <= 2000000);
    let profile_g2 = maximum.is_some_and(|m| audit(p, 2, m, n, complete, true));
    let profile_work = count(&p["exactWork"]);
    let g1_work = count(&s["g1Audit"]["exactWork"]);
    let profile_g1 = profile_g2
        || maximum.zip(profile_work).is_some_and(|(m, pw)| {
            pw <= m
                && yes(s, "profileG1Certified")
                && named(s, "g1Method", "exact-projective-audit")
                && g1_work.and_then(|w| pw.checked_add(w)) == count(&s["exactWork"])
                && audit(&s["g1Audit"], 1, m - pw, n, complete, true)
        });
    let station = &s["station"];
    let station_declared = named(station, "method", "retained-miter-station-joins")
        && named(station, "scope", "wall-station-seams")
        && station["edgeIds"].is_array();
    let station_complete = station_declared && extraction(station);
    let station_n = len(&station["edgeIds"]).unwrap_or(0);
    let station_budget = maximum
        .zip(count(&s["exactWork"]))
        .and_then(|(m, pw)| {
            let g1 = if s["g1Audit"].is_null() {
                Some(0)
            } else {
                g1_work
            }?;
            if profile_work?.checked_add(g1)? != pw || pw > m {
                return None;
            }
            let remaining = m - pw;
            if count(&station["maxWork"]) != Some(remaining) {
                return None;
            }
            let work = count(&station["exactWork"])?;
            let g2 = count(&station["g2"]["exactWork"])?;
            let g1 = if station["g1Audit"].is_null() {
                Some(0)
            } else {
                count(&station["g1Audit"]["exactWork"])
            }?;
            if g2.checked_add(g1)? != work
                || work > remaining
                || pw.checked_add(work) != count(&s["totalExactWork"])
            {
                return None;
            }
            Some((remaining, g2))
        })
        .filter(|_| station_declared);
    let station_g2 = station_budget
        .is_some_and(|(m, _)| audit(&station["g2"], 2, m, station_n, station_complete, false));
    let station_g1 = station_g2
        || station_budget.is_some_and(|(m, g2)| {
            g2 <= m
                && yes(station, "stationG1Certified")
                && audit(
                    &station["g1Audit"],
                    1,
                    m - g2,
                    station_n,
                    station_complete,
                    false,
                )
        });
    result["profileG2Certified"] = json!(profile_g2);
    result["profileG1Certified"] = json!(profile_g1);
    result["profileSeamCount"] = json!(len(&p["seams"]).unwrap());
    result["stationContinuity"] = json!(if station_g2 {
        "G2"
    } else if station_g1 {
        "G1"
    } else {
        "C0"
    });
    if station_declared {
        result["stationG1Certified"] = json!(station_g1);
        result["stationG2Certified"] = json!(station_g2);
        result["stationSeamCount"] = json!(station_n);
    }
    if [Some("C0"), Some("absent")].contains(&s["capContinuity"].as_str()) {
        result["capContinuity"] = s["capContinuity"].clone();
    }
    Ok(result)
}
