//! Transport adapter. All intersection, loop and finite-face work is native.
use crate::*;
use value_codec::Serialize;
pub(super) fn partition(v: Value) -> Result<Value> {
    let object = v.as_object().ok_or_else(|| {
        nurbs_core::Error::new("BREP_CLOSED_SS_INPUT", "Expected closed SS request")
    })?;
    let keys = [
        "op",
        "a",
        "b",
        "faceA",
        "faceB",
        "toleranceMm",
        "toleranceUv",
    ];
    if object.len() != keys.len() || keys.iter().any(|k| !object.contains_key(*k)) {
        return Err(nurbs_core::Error::new(
            "BREP_CLOSED_SS_INPUT",
            "Closed SS request fields do not match schema",
        ));
    }
    let a: brep_core::Model = field(&v, "a")?;
    let b: brep_core::Model = field(&v, "b")?;
    let fa: usize = field(&v, "faceA")?;
    let fb: usize = field(&v, "faceB")?;
    let tolerance_world: f64 = field(&v, "toleranceMm")?;
    let tolerance_uv: f64 = field(&v, "toleranceUv")?;
    let mut limits = brep_core::face_cut::ClosedSsLimits::default();
    limits.trace.tolerance_world = tolerance_world;
    limits.trace.tolerance_uv = tolerance_uv;
    let result = brep_core::face_cut::partition_closed_ss(&a, fa, &b, fb, limits)?;
    Ok(
        json!({"a":result.first.model.to_value(),"b":result.second.model.to_value(),
        "facesA":result.first.faces,"facesB":result.second.faces,
        "originalSourceClosedCoverage":true,"materialClassified":false,
        "segments":result.trace.segments().len(),"cells":result.trace.cells(),
        "worldErrorUpper":result.trace.world_error_upper(),"uvErrorUpper":result.trace.uv_error_upper()}),
    )
}
pub(super) fn network(v: Value) -> Result<Value> {
    let object = v.as_object().ok_or_else(|| {
        nurbs_core::Error::new("BREP_CLOSED_SS_INPUT", "Expected closed SS network request")
    })?;
    let required = [
        "op",
        "a",
        "b",
        "faceA",
        "faceB",
        "toleranceMm",
        "toleranceUv",
    ];
    if required.iter().any(|k| !object.contains_key(*k))
        || object
            .keys()
            .any(|k| !required.contains(&k.as_str()) && k != "maxRepairs")
    {
        return Err(nurbs_core::Error::new(
            "BREP_CLOSED_SS_INPUT",
            "Closed SS network request fields do not match schema",
        ));
    }
    let a: brep_core::Model = field(&v, "a")?;
    let b: brep_core::Model = field(&v, "b")?;
    let mut limits = brep_core::face_cut::ClosedSsLimits::default();
    limits.trace.tolerance_world = field(&v, "toleranceMm")?;
    limits.trace.tolerance_uv = field(&v, "toleranceUv")?;
    if object.contains_key("maxRepairs") {
        limits.branches.max_repairs = field(&v, "maxRepairs")?;
    }
    let result = brep_core::face_cut::partition_all_closed_ss(
        &a,
        field(&v, "faceA")?,
        &b,
        field(&v, "faceB")?,
        limits,
    )?;
    let world = result
        .traces
        .iter()
        .map(|t| t.world_error_upper())
        .fold(0_f64, f64::max);
    let uv: [f64; 2] = std::array::from_fn(|k| {
        result
            .traces
            .iter()
            .map(|t| t.uv_error_upper()[k])
            .fold(0_f64, f64::max)
    });
    Ok(
        json!({"a":result.models[0].to_value(),"b":result.models[1].to_value(),"facesA":result.faces[0],"facesB":result.faces[1],
        "originalSourceClosedCoverage":true,"materialClassified":false,"branches":result.traces.len(),
        "segments":result.traces.iter().map(|t|t.segments().len()).sum::<usize>(),"placementAttempts":result.placement_attempts,
        "worldErrorUpper":world,"uvErrorUpper":uv}),
    )
}
