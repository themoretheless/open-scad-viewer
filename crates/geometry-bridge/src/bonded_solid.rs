use crate::{Result, Value, field, input, json, require_exact_fields};
use mechanics_core::bonded_solid::{Bond, Model, Tet};

pub fn solve(v: Value) -> Result<Value> {
    require_exact_fields(
        &v,
        &[
            "op",
            "nodesMm",
            "tets",
            "bonds",
            "restrained",
            "forcesN",
            "safetyFactor",
            "profile",
            "serviceTempC",
        ],
        "bonded solid",
    )?;
    for (key, max) in [
        ("nodesMm", 125),
        ("tets", 400),
        ("bonds", 200),
        ("restrained", 125),
        ("forcesN", 125),
    ] {
        if v[key]
            .as_array()
            .is_none_or(|a| a.is_empty() || a.len() > max)
        {
            return Err(input(format!("{key} exceeds bonded solid bounds")));
        }
    }
    let profile = crate::print_strength::parse_profile(&v["profile"])?;
    let warnings = mechanics_core::print_strength::validate_profile(&profile)?;
    let service: f64 = field(&v, "serviceTempC")?;
    if !service.is_finite() || service <= -273.15 {
        return Err(input(
            "Provide a finite service temperature above absolute zero",
        ));
    }
    let mut tets = Vec::new();
    for t in v["tets"].as_array().unwrap() {
        require_exact_fields(
            t,
            &["nodes", "region", "youngMpa", "poisson"],
            "tetrahedron",
        )?;
        let infill = match t["region"].as_str() {
            Some("shell") => false,
            Some("infill") => true,
            _ => return Err(input("Tet region must be shell or infill")),
        };
        tets.push(Tet {
            nodes: field(t, "nodes")?,
            infill,
            young_mpa: field(t, "youngMpa")?,
            poisson: field(t, "poisson")?,
        });
    }
    let mut bonds = Vec::new();
    for b in v["bonds"].as_array().unwrap() {
        require_exact_fields(
            b,
            &[
                "shell",
                "infill",
                "normalStiffnessMpaPerMm",
                "shearStiffnessMpaPerMm",
                "tensionMpa",
                "shearMpa",
                "compressionMpa",
                "propertySource",
            ],
            "bond",
        )?;
        if b["propertySource"]
            .as_str()
            .is_none_or(|s| s.trim().is_empty() || s.len() > 512)
        {
            return Err(input(
                "Each bond requires a calibration source up to 512 bytes",
            ));
        }
        bonds.push(Bond {
            shell: field(b, "shell")?,
            infill: field(b, "infill")?,
            normal_stiffness_mpa_per_mm: field(b, "normalStiffnessMpaPerMm")?,
            shear_stiffness_mpa_per_mm: field(b, "shearStiffnessMpaPerMm")?,
            tension_mpa: field(b, "tensionMpa")?,
            shear_mpa: field(b, "shearMpa")?,
            compression_mpa: field(b, "compressionMpa")?,
        });
    }
    let r = mechanics_core::bonded_solid::solve(&Model {
        nodes_mm: field(&v, "nodesMm")?,
        tets,
        bonds,
        restrained: field(&v, "restrained")?,
        forces_n: field(&v, "forcesN")?,
        safety_factor: field(&v, "safetyFactor")?,
    })?;
    let bonds:Vec<_>=r.bonds.iter().map(|b|json!({"areaMm2":b.area_mm2,"normal":b.normal,"forceOnShellN":b.force_on_shell_n,"openingMm":b.opening_mm,"normalTractionMpa":b.normal_traction_mpa,"shearTractionMpa":b.shear_traction_mpa,"utilization":b.utilization})).collect();
    Ok(
        json!({"modelKind":"explicit-tet4-intact-bonds-v1","geometryBinding":"explicit-mesh-not-cad-verified",
        "displacementsMm":r.linear.displacements_mm,"reactionsN":r.linear.reactions_n,
        "maxDeflectionMm":r.linear.max_deflection_mm,"maxRelativeResidual":r.linear.max_relative_residual,
        "stressesMpa":r.stresses_mpa,"volumesMm3":r.volumes_mm3,"bonds":bonds,
        "limitReached":r.limit_reached,"profileWarnings":warnings}),
    )
}
