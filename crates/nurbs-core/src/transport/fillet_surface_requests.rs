use super::*;
use crate::offset::fillet_surfaces::{self, FilletOptions, RadiusLaw};

pub(super) fn build(v: &Value, variable: bool) -> Result<Value> {
    let a = field(v, "surface")?;
    let b = field(v, "target")?;
    let options = FilletOptions {
        side_a: field(v, "side_a")?,
        side_b: field(v, "side_b")?,
        tolerance: field(v, "tolerance_mm")?,
        max_march_steps: field(v, "max_march_steps")?,
        max_sections: field(v, "max_sections")?,
    };
    let result = if variable {
        let stations: Vec<[f64; 2]> = field(v, "radius_stations")?;
        check(
            (2..=33).contains(&stations.len()),
            "Fillet requires 2..=33 radius stations",
        )?;
        check(
            stations.first().unwrap()[0] == 0. && stations.last().unwrap()[0] == 1.,
            "Fillet radius stations must cover the normalized interval [0,1]",
        )?;
        let law = if let [[0., start], [1., end]] = stations.as_slice() {
            RadiusLaw::Linear {
                start: *start,
                end: *end,
            }
        } else {
            RadiusLaw::Spline(
                stations
                    .into_iter()
                    .map(|[t, radius]| (t, radius))
                    .collect(),
            )
        };
        fillet_surfaces::variable_radius_fillet(&a, &b, law, &options)?
    } else {
        fillet_surfaces::rolling_ball_fillet(&a, &b, field(v, "radius")?, &options)?
    };
    let failure = result.failure.map(|f| {
        json!({
            "reason":format!("{:?}", f.reason), "parameter":f.t, "detail":f.detail
        })
    });
    let spine: Vec<_> = result
        .spine
        .into_iter()
        .map(|s| {
            json!({
                "t":s.t, "center":s.center, "radiusMm":s.radius,
                "contactA":s.contact_a, "contactB":s.contact_b, "uvA":s.uv_a, "uvB":s.uv_b
            })
        })
        .collect();
    Ok(json!({
        "surface":encode(result.surface)?,
        "failure":failure,
        "contactPcurves":encode(result.contact_pcurves)?,
        "spine":spine,
        "sampledRadiusRangeMm":result.radius_range,
        "warnings":result.warnings,
        "sampledSkinDeviationMm":result.skin_deviation_estimate,
        "closedSpine":result.closed_spine,
        "continuousContactCertified":false,
        "solidTopologyCertified":false
    }))
}
