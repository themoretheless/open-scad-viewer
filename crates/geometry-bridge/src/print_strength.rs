use crate::{Result, Value, field, input, json, require_exact_fields};
use mechanics_core::print_strength::{Limits, PrintProfile, screen, validate_profile};

pub fn screening(v: Value) -> Result<Value> {
    require_exact_fields(
        &v,
        &["op", "areasMm2", "forcesN", "limits"],
        "axial screening",
    )?;
    let areas = v["areasMm2"]
        .as_array()
        .ok_or_else(|| input("areasMm2 must be an array"))?;
    let forces = v["forcesN"]
        .as_array()
        .ok_or_else(|| input("forcesN must be an array"))?;
    if areas.len() > 400
        || forces.len() > 32
        || forces
            .iter()
            .any(|r| r.as_array().is_none_or(|r| r.len() > 400))
    {
        return Err(input("Axial screening exceeds 400 members / 32 responses"));
    }
    let limits = &v["limits"];
    require_exact_fields(
        limits,
        &["tensionMpa", "compressionMpa", "safetyFactor"],
        "axial limits",
    )?;
    let rows = screen(
        &field::<Vec<f64>>(&v, "areasMm2")?,
        &field::<Vec<Vec<f64>>>(&v, "forcesN")?,
        &Limits {
            tension_mpa: field(limits, "tensionMpa")?,
            compression_mpa: field(limits, "compressionMpa")?,
            safety_factor: field(limits, "safetyFactor")?,
        },
    )?;
    Ok(json!(
        rows.into_iter()
            .map(
                |r| json!({"memberIndex":r.member_index,"utilization":r.utilization,
        "governingResponse":r.governing_response,"stressMpa":r.stress_mpa,"status":r.status})
            )
            .collect::<Vec<_>>()
    ))
}

pub fn profile(v: Value) -> Result<Value> {
    require_exact_fields(&v, &["op", "profile"], "print profile request")?;
    let p = &v["profile"];
    require_exact_fields(
        p,
        &[
            "material",
            "grade",
            "propertySource",
            "nozzleMm",
            "lineWidthMm",
            "layerHeightMm",
            "nozzleTempC",
            "bedTempC",
        ],
        "print profile",
    )?;
    for key in ["material", "grade", "propertySource"] {
        if p[key].as_str().is_none_or(|s| s.len() > 512) {
            return Err(input("Print profile strings must be at most 512 bytes"));
        }
    }
    let warnings = validate_profile(&PrintProfile {
        material: field(p, "material")?,
        grade: field(p, "grade")?,
        property_source: field(p, "propertySource")?,
        nozzle_mm: field(p, "nozzleMm")?,
        line_width_mm: field(p, "lineWidthMm")?,
        layer_height_mm: field(p, "layerHeightMm")?,
        nozzle_temp_c: field(p, "nozzleTempC")?,
        bed_temp_c: field(p, "bedTempC")?,
    })?;
    Ok(json!({"profile":p,"warnings":warnings,"propertyModel":"user-supplied-isotropic-axial"}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn screening_dispatch_is_strict_and_bounded() {
        let v = json!({"op":"truss_screen","areasMm2":[2],"forcesN":[[100]],"limits":{"tensionMpa":80,"compressionMpa":40,"safetyFactor":2}});
        assert_eq!(
            crate::dispatch(v.clone()).unwrap()[0]["utilization"],
            json!(1.25)
        );
        let mut bad = v.clone();
        bad["forcesN"] = json!(vec![vec![0.]; 33]);
        assert!(crate::dispatch(bad).is_err());
        let mut bad = v;
        bad["limits"]["temperatureFactor"] = json!(1);
        assert!(crate::dispatch(bad).is_err());
    }
}
