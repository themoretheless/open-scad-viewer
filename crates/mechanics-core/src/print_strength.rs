//! Axial screening and explicit print-process provenance, not an FDM constitutive model.
use crate::truss::MAX_MEMBERS;
use crate::{Error, Result};

pub struct Limits {
    pub tension_mpa: f64,
    pub compression_mpa: f64,
    pub safety_factor: f64,
}

#[derive(Debug, PartialEq)]
pub struct Screening {
    pub member_index: usize,
    pub utilization: f64,
    pub governing_response: usize,
    pub stress_mpa: f64,
    pub status: &'static str,
}

pub fn screen(areas: &[f64], forces: &[Vec<f64>], limits: &Limits) -> Result<Vec<Screening>> {
    let positive = |v: f64| v.is_finite() && v > 0.;
    if areas.is_empty()
        || areas.len() > MAX_MEMBERS
        || forces.is_empty()
        || forces.len() > 32
        || !areas.iter().copied().all(positive)
        || ![
            limits.tension_mpa,
            limits.compression_mpa,
            limits.safety_factor,
        ]
        .into_iter()
        .all(positive)
        || limits.safety_factor < 1.
        || forces
            .iter()
            .any(|row| row.len() != areas.len() || row.iter().any(|v| !v.is_finite()))
    {
        return Err(Error::new(
            "TRUSS_SCREENING_INVALID_INPUT",
            "Provide bounded finite forces, positive areas and limits, and a safety factor >= 1",
        ));
    }
    let mut rows = Vec::with_capacity(areas.len());
    for (member_index, area) in areas.iter().enumerate() {
        let mut row = Screening {
            member_index,
            utilization: -1.,
            governing_response: 0,
            stress_mpa: 0.,
            status: "",
        };
        for (index, response) in forces.iter().enumerate() {
            let stress = response[member_index] / area;
            let limit = if stress < 0. {
                limits.compression_mpa
            } else {
                limits.tension_mpa
            };
            let utilization = stress.abs() / limit * limits.safety_factor;
            if !stress.is_finite() || !utilization.is_finite() {
                return Err(Error::new(
                    "TRUSS_SCREENING_NUMERIC_RANGE",
                    "Axial screening numeric range exceeded",
                ));
            }
            if utilization > row.utilization {
                row.utilization = utilization;
                row.governing_response = index;
                row.stress_mpa = stress;
            }
        }
        row.status = if row.utilization > 1. {
            "overloaded"
        } else if row.utilization < 0.3 {
            "low-demand"
        } else {
            "within-axial-limit"
        };
        rows.push(row);
    }
    rows.sort_by(|a, b| {
        b.utilization
            .total_cmp(&a.utilization)
            .then(a.member_index.cmp(&b.member_index))
    });
    Ok(rows)
}

#[derive(Clone, Debug)]
pub struct PrintProfile {
    pub material: String,
    pub grade: String,
    pub property_source: String,
    pub nozzle_mm: f64,
    pub line_width_mm: f64,
    pub layer_height_mm: f64,
    pub nozzle_temp_c: f64,
    pub bed_temp_c: f64,
}

/// Validates provenance and emits process advisories without inventing temperature
/// or polymer strength multipliers. The caller binds measured E/limits to this profile.
pub fn validate_profile(p: &PrintProfile) -> Result<Vec<&'static str>> {
    if !matches!(
        p.material.as_str(),
        "PLA" | "PETG" | "ABS" | "ASA" | "PA" | "PC" | "TPU" | "custom"
    ) || [&p.grade, &p.property_source]
        .iter()
        .any(|s| s.trim().is_empty() || s.len() > 512)
        || [
            p.nozzle_mm,
            p.line_width_mm,
            p.layer_height_mm,
            p.nozzle_temp_c,
        ]
        .iter()
        .any(|v| !v.is_finite() || *v <= 0.)
        || !p.bed_temp_c.is_finite()
        || p.bed_temp_c < 0.
    {
        return Err(Error::new(
            "PRINT_PROFILE_INVALID_INPUT",
            "Specify material, grade, property source, positive nozzle/line/layer dimensions and nozzle temperature, and nonnegative bed temperature",
        ));
    }
    let mut warnings = Vec::new();
    if p.layer_height_mm / p.nozzle_mm > 0.8 {
        warnings.push("layer-above-80-percent-nozzle");
    }
    if p.line_width_mm <= p.layer_height_mm {
        warnings.push("line-width-not-greater-than-layer");
    }
    Ok(warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn envelope_preserves_sign_and_ranks_demand() {
        let limits = Limits {
            tension_mpa: 80.,
            compression_mpa: 40.,
            safety_factor: 2.,
        };
        let result = screen(&[2., 2.], &[vec![100., 0.], vec![-60., 80.]], &limits).unwrap();
        assert_eq!(
            result[0],
            Screening {
                member_index: 0,
                utilization: 1.5,
                governing_response: 1,
                stress_mpa: -30.,
                status: "overloaded"
            }
        );
        assert_eq!(result[1].status, "within-axial-limit");
        assert_eq!(
            screen(&[2.], &[vec![0.]], &limits).unwrap()[0].status,
            "low-demand"
        );
        assert!(screen(&[f64::MIN_POSITIVE], &[vec![f64::MAX]], &limits).is_err());
        assert!(screen(&[2.], &[vec![f64::NAN]], &limits).is_err());
        assert!(screen(&[2.], &[], &limits).is_err());
    }
    #[test]
    fn print_profile_is_explicit_and_never_supplies_strength() {
        let mut p = PrintProfile {
            material: "PETG".into(),
            grade: "Example spool".into(),
            property_source: "Coupon test".into(),
            nozzle_mm: 0.4,
            line_width_mm: 0.45,
            layer_height_mm: 0.2,
            nozzle_temp_c: 240.,
            bed_temp_c: 80.,
        };
        assert!(validate_profile(&p).unwrap().is_empty());
        p.layer_height_mm = 0.4;
        assert_eq!(
            validate_profile(&p).unwrap(),
            vec!["layer-above-80-percent-nozzle"]
        );
        p.line_width_mm = 0.3;
        assert_eq!(validate_profile(&p).unwrap().len(), 2);
        p.material = "unknown".into();
        assert!(validate_profile(&p).is_err());
        p.material = "custom".into();
        p.property_source.clear();
        assert!(validate_profile(&p).is_err());
    }
}
