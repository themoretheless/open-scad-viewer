use super::*;
use crate::wing_loft::{self, TipClosure, WingSection};

pub(super) fn build(v: &Value) -> Result<Value> {
    let profiles: Vec<curve::Curve> = field(v, "profiles")?;
    let stations: Vec<[f64; 5]> = field(v, "wing_stations")?;
    check(
        profiles.len() == stations.len(),
        "Each wing station requires a profile",
    )?;
    let sections: Vec<_> = profiles
        .into_iter()
        .zip(stations)
        .map(
            |(profile, [span, twist, dihedral, sweep, taper])| WingSection {
                profile,
                span,
                twist: twist.to_radians(),
                dihedral: dihedral.to_radians(),
                sweep: sweep.to_radians(),
                taper,
            },
        )
        .collect();
    let tip = match v["wing_tip"].as_str() {
        Some("open") => TipClosure::None,
        Some("collapse") => TipClosure::Collapse,
        Some("dome") => TipClosure::Dome {
            height: field(v, "tip_height_mm")?,
        },
        _ => return Err(crate::input("Choose an open, collapse, or dome wing tip")),
    };
    let surface = if matches!(tip, TipClosure::None) {
        wing_loft::loft_wing(&sections)?
    } else {
        wing_loft::loft_wing_with_tip(&sections, &tip)?
    };
    let sampled = wing_loft::check_folding(&surface, field(v, "fold_grid")?)?;
    Ok(json!({
        "surface":encode(surface)?,
        "stations":sections.len(),
        "tip":v["wing_tip"].clone(),
        "grid":sampled.grid,
        "flipped":sampled.flipped,
        "degenerate":sampled.degenerate,
        "minimumSampledNormalMagnitude":sampled.min_normal_magnitude,
        "globalInjectivityCertified":false,
        "solidTopologyCertified":false
    }))
}
