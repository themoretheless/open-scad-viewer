use crate::{Result, field, input};
use nurbs_core::radial_cam::{self, Law, Motion, Request};
use value_codec::{Value, json};

pub(super) fn profile(v: &Value) -> Result<Value> {
    let request = Request {
        center: field(v, "center")?,
        base_radius: field(v, "base_radius")?,
        roller_radius: field(v, "roller_radius")?,
        phase_degrees: field(v, "phase")?,
        tolerance: field(v, "tolerance_mm")?,
        max_spans: field(v, "max_spans")?,
    };
    let angles: Vec<f64> = field(v, "end_angles")?;
    let lifts: Vec<f64> = field(v, "end_lifts")?;
    let laws: Vec<String> = field(v, "laws")?;
    if angles.len() != lifts.len()
        || angles.len() != laws.len()
        || !(1..=64).contains(&angles.len())
    {
        return Err(input(
            "Expected matching cam angles, lifts and motion laws for 1..64 stages",
        ));
    }
    let motions = angles
        .into_iter()
        .zip(lifts)
        .zip(laws)
        .map(|((end_degrees, end_lift), name)| {
            let law = match name.as_str() {
                "dwell" => Law::Dwell,
                "harmonic" => Law::Harmonic,
                "cycloidal" => Law::Cycloidal,
                "polynomial345" => Law::Polynomial345,
                "polynomial4567" => Law::Polynomial4567,
                _ => return Err(input("Unknown cam motion law")),
            };
            Ok(Motion {
                end_degrees,
                end_lift,
                law,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let r = radial_cam::profile(request, &motions)?;
    Ok(
        json!({"curves":r.curves,"spans":r.cells.len(),"stop":format!("{:?}",r.stop),
        "maxSampledErrorMm":r.max_sampled_error,"maxSampledPressureAngleRadians":r.max_sampled_pressure_angle,"sampledUndercut":r.sampled_undercut}),
    )
}
