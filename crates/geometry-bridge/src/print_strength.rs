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

fn parse_profile(p: &Value) -> Result<PrintProfile> {
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
    Ok(PrintProfile {
        material: field(p, "material")?,
        grade: field(p, "grade")?,
        property_source: field(p, "propertySource")?,
        nozzle_mm: field(p, "nozzleMm")?,
        line_width_mm: field(p, "lineWidthMm")?,
        layer_height_mm: field(p, "layerHeightMm")?,
        nozzle_temp_c: field(p, "nozzleTempC")?,
        bed_temp_c: field(p, "bedTempC")?,
    })
}

pub fn profile(v: Value) -> Result<Value> {
    require_exact_fields(&v, &["op", "profile"], "print profile request")?;
    let p = &v["profile"];
    let warnings = validate_profile(&parse_profile(p)?)?;
    Ok(json!({"profile":p,"warnings":warnings,"propertyModel":"user-supplied-isotropic-axial"}))
}

pub fn thermal(v: Value) -> Result<Value> {
    use mechanics_core::thermal_strength::{MAX_SAMPLES, Properties, Sample, evaluate};
    require_exact_fields(
        &v,
        &[
            "op",
            "profile",
            "calibrationProfile",
            "samples",
            "serviceTempC",
        ],
        "thermal request",
    )?;
    let values = v["samples"]
        .as_array()
        .ok_or_else(|| input("samples must be an array"))?;
    if values.len() > MAX_SAMPLES {
        return Err(input("Thermal calibration exceeds 64 samples"));
    }
    let samples = values
        .iter()
        .map(|s| {
            require_exact_fields(
                s,
                &[
                    "nozzleTempC",
                    "serviceTempC",
                    "youngMpa",
                    "tensionMpa",
                    "compressionMpa",
                ],
                "thermal sample",
            )?;
            Ok(Sample {
                nozzle_temp_c: field(s, "nozzleTempC")?,
                service_temp_c: field(s, "serviceTempC")?,
                properties: Properties {
                    young_mpa: field(s, "youngMpa")?,
                    tension_mpa: field(s, "tensionMpa")?,
                    compression_mpa: field(s, "compressionMpa")?,
                },
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let result = evaluate(
        &parse_profile(&v["profile"])?,
        &parse_profile(&v["calibrationProfile"])?,
        &samples,
        field(&v, "serviceTempC")?,
    )?;
    Ok(
        json!({"modelKind":"measured-bilinear-isotropic-axial-v1","youngMpa":result.properties.young_mpa,
        "tensionMpa":result.properties.tension_mpa,"compressionMpa":result.properties.compression_mpa,
        "nozzleBracketC":result.nozzle_bracket_c,"serviceBracketC":result.service_bracket_c}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn thermal_dispatch_preserves_properties_and_refuses_schema_drift() {
        let p = json!({"material":"PETG","grade":"test","propertySource":"synthetic","nozzleMm":0.4,"lineWidthMm":0.45,"layerHeightMm":0.2,"nozzleTempC":240,"bedTempC":80});
        let v = json!({"op":"thermal_strength","profile":p,"calibrationProfile":p,"serviceTempC":40,"samples":[
            {"nozzleTempC":240,"serviceTempC":20,"youngMpa":2000,"tensionMpa":40,"compressionMpa":60},
            {"nozzleTempC":240,"serviceTempC":60,"youngMpa":1000,"tensionMpa":20,"compressionMpa":30}]});
        assert_eq!(
            crate::dispatch(v.clone()).unwrap()["youngMpa"],
            json!(1500.)
        );
        let mut bad = v.clone();
        bad["samples"][0]["temperatureFactor"] = json!(1.);
        assert!(crate::dispatch(bad).is_err());
        let mut bad = v;
        bad["serviceTempC"] = json!(80.);
        assert_eq!(
            crate::dispatch(bad).unwrap_err().code,
            "THERMAL_OUT_OF_RANGE"
        );
    }
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
