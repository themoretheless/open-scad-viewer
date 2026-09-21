//! Piecewise bilinear interpolation of measured isotropic axial properties.
//! No extrapolation, polymer defaults, creep, thermal strain or failure-law inference.
use crate::print_strength::{PrintProfile, validate_profile};
use crate::{Error, Result};

pub const MAX_SAMPLES: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Properties {
    pub young_mpa: f64,
    pub tension_mpa: f64,
    pub compression_mpa: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct Sample {
    pub nozzle_temp_c: f64,
    pub service_temp_c: f64,
    pub properties: Properties,
}

#[derive(Debug)]
pub struct ThermalResult {
    pub properties: Properties,
    pub nozzle_bracket_c: [f64; 2],
    pub service_bracket_c: [f64; 2],
}

fn invalid(message: &str) -> Error {
    Error::new("THERMAL_INVALID_INPUT", message)
}

fn bracket(axis: &[f64], query: f64) -> Result<[f64; 2]> {
    if query < axis[0] || query > axis[axis.len() - 1] {
        return Err(Error::new(
            "THERMAL_OUT_OF_RANGE",
            "Temperature is outside the measured range; extrapolation is disabled",
        ));
    }
    if let Some(&value) = axis.iter().find(|&&v| v == query) {
        return Ok([value, value]);
    }
    let upper = axis.partition_point(|&v| v < query);
    Ok([axis[upper - 1], axis[upper]])
}

fn blend(a: Properties, b: Properties, bracket: [f64; 2], query: f64) -> Properties {
    if bracket[0] == bracket[1] {
        return a;
    }
    let weight = (query - bracket[0]) / (bracket[1] - bracket[0]);
    let interpolate = |x, y| (1. - weight) * x + weight * y;
    Properties {
        young_mpa: interpolate(a.young_mpa, b.young_mpa),
        tension_mpa: interpolate(a.tension_mpa, b.tension_mpa),
        compression_mpa: interpolate(a.compression_mpa, b.compression_mpa),
    }
}

pub fn evaluate(
    profile: &PrintProfile,
    calibration: &PrintProfile,
    samples: &[Sample],
    service_temp_c: f64,
) -> Result<ThermalResult> {
    validate_profile(profile)?;
    validate_profile(calibration)?;
    // Nozzle temperature is an independent measured axis. All remaining process
    // identifiers must match; a new material/process needs its own calibration.
    if profile.material != calibration.material
        || profile.grade != calibration.grade
        || profile.property_source != calibration.property_source
        || profile.nozzle_mm != calibration.nozzle_mm
        || profile.line_width_mm != calibration.line_width_mm
        || profile.layer_height_mm != calibration.layer_height_mm
        || profile.bed_temp_c != calibration.bed_temp_c
    {
        return Err(Error::new(
            "THERMAL_PROFILE_MISMATCH",
            "Calibration does not match the material, source, nozzle, line, layer or bed temperature",
        ));
    }
    if samples.len() < 2
        || samples.len() > MAX_SAMPLES
        || !service_temp_c.is_finite()
        || service_temp_c <= -273.15
    {
        return Err(invalid(
            "Provide 2-64 calibration points and a finite service temperature above absolute zero",
        ));
    }
    let mut nozzles = Vec::with_capacity(samples.len());
    let mut services = Vec::with_capacity(samples.len());
    for (index, sample) in samples.iter().enumerate() {
        if !sample.service_temp_c.is_finite()
            || sample.service_temp_c <= -273.15
            || ![
                sample.nozzle_temp_c,
                sample.properties.young_mpa,
                sample.properties.tension_mpa,
                sample.properties.compression_mpa,
            ]
            .into_iter()
            .all(|v| v.is_finite() && v > 0.)
        {
            return Err(invalid(
                "Measured temperatures and positive mechanical properties must be finite",
            ));
        }
        if samples[..index].iter().any(|s| {
            s.nozzle_temp_c == sample.nozzle_temp_c && s.service_temp_c == sample.service_temp_c
        }) {
            return Err(invalid("Duplicate calibration temperatures"));
        }
        nozzles.push(sample.nozzle_temp_c);
        services.push(sample.service_temp_c);
    }
    nozzles.sort_by(f64::total_cmp);
    nozzles.dedup();
    services.sort_by(f64::total_cmp);
    services.dedup();
    if nozzles.len() * services.len() != samples.len() {
        return Err(invalid(
            "Calibration must include every nozzle/service temperature pair in its grid",
        ));
    }
    let nozzle = bracket(&nozzles, profile.nozzle_temp_c)?;
    let service = bracket(&services, service_temp_c)?;
    let at = |n, s| {
        samples
            .iter()
            .find(|p| p.nozzle_temp_c == n && p.service_temp_c == s)
            .unwrap()
            .properties
    };
    let low = blend(
        at(nozzle[0], service[0]),
        at(nozzle[1], service[0]),
        nozzle,
        profile.nozzle_temp_c,
    );
    let high = blend(
        at(nozzle[0], service[1]),
        at(nozzle[1], service[1]),
        nozzle,
        profile.nozzle_temp_c,
    );
    let properties = blend(low, high, service, service_temp_c);
    if ![
        properties.young_mpa,
        properties.tension_mpa,
        properties.compression_mpa,
    ]
    .into_iter()
    .all(|v| v.is_finite() && v > 0.)
    {
        return Err(Error::new(
            "THERMAL_NUMERIC_RANGE",
            "Interpolated properties exceed the numeric range",
        ));
    }
    Ok(ThermalResult {
        properties,
        nozzle_bracket_c: nozzle,
        service_bracket_c: service,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn profile() -> PrintProfile {
        PrintProfile {
            material: "PETG".into(),
            grade: "Synthetic fixture".into(),
            property_source: "Test only".into(),
            nozzle_mm: 0.4,
            line_width_mm: 0.45,
            layer_height_mm: 0.2,
            nozzle_temp_c: 250.,
            bed_temp_c: 80.,
        }
    }
    fn samples() -> Vec<Sample> {
        [
            (240., 20., 2000.),
            (240., 60., 1000.),
            (260., 20., 3000.),
            (260., 60., 1500.),
        ]
        .map(|(n, t, e)| Sample {
            nozzle_temp_c: n,
            service_temp_c: t,
            properties: Properties {
                young_mpa: e,
                tension_mpa: e / 50.,
                compression_mpa: e / 25.,
            },
        })
        .to_vec()
    }
    #[test]
    fn bilinear_and_exact_points_and_order() {
        let p = profile();
        let mut s = samples();
        let result = evaluate(&p, &p, &s, 40.).unwrap();
        assert_eq!(
            result.properties,
            Properties {
                young_mpa: 1875.,
                tension_mpa: 37.5,
                compression_mpa: 75.
            }
        );
        assert_eq!(result.nozzle_bracket_c, [240., 260.]);
        s.reverse();
        assert_eq!(
            evaluate(&p, &p, &s, 40.).unwrap().properties,
            result.properties
        );
        let mut q = p.clone();
        q.nozzle_temp_c = 240.;
        assert_eq!(
            evaluate(&q, &p, &s, 20.).unwrap().properties.young_mpa,
            2000.
        );
    }
    #[test]
    fn one_axis_requires_exact_fixed_temperature() {
        let mut p = profile();
        p.nozzle_temp_c = 240.;
        let s = &samples()[..2];
        assert_eq!(
            evaluate(&p, &p, s, 40.).unwrap().properties.young_mpa,
            1500.
        );
        p.nozzle_temp_c = 241.;
        assert_eq!(
            evaluate(&p, &p, s, 40.).unwrap_err().code,
            "THERMAL_OUT_OF_RANGE"
        );
    }
    #[test]
    fn refuses_missing_duplicate_invalid_and_extrapolated_data() {
        let p = profile();
        let s = samples();
        assert!(evaluate(&p, &p, &s[..3], 40.).is_err());
        let mut duplicate = s.clone();
        duplicate[3] = duplicate[0];
        assert!(evaluate(&p, &p, &duplicate, 40.).is_err());
        for t in [-273.15, f64::NAN, 19., 61.] {
            assert!(evaluate(&p, &p, &s, t).is_err());
        }
        let mut bad = s.clone();
        bad[0].properties.tension_mpa = 0.;
        assert!(evaluate(&p, &p, &bad, 40.).is_err());
        assert!(evaluate(&p, &p, &vec![s[0]; 65], 40.).is_err());
        let mut changed = p.clone();
        changed.bed_temp_c = 90.;
        assert_eq!(
            evaluate(&changed, &p, &s, 40.).unwrap_err().code,
            "THERMAL_PROFILE_MISMATCH"
        );
    }
}
