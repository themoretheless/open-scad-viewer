use crate::{Result, field};
use value_codec::{Value, json};

pub(super) fn rack(v: &Value, body: bool) -> Result<Value> {
    let degrees: f64 = field(v, "pressure_angle")?;
    let spec = nurbs_core::rack::Spec {
        teeth: field(v, "teeth")?,
        module: field(v, "module")?,
        pressure_angle_radians: degrees.to_radians(),
        backlash: field(v, "backlash")?,
        addendum: field(v, "addendum")?,
        dedendum: field(v, "dedendum")?,
        backing_height: field(v, "backing_height")?,
    };
    let (profile, model) = if body {
        let r = brep_core::rack::extrude(spec, field(v, "z_min")?, field(v, "z_max")?)?;
        (r.profile, Some(r.model))
    } else {
        (nurbs_core::rack::profile(spec)?, None)
    };
    Ok(
        json!({"curve":profile.boundary,"model":model,"pitchMm":profile.pitch,"widthMm":profile.width,
        "toothThicknessAtPitchMm":profile.tooth_thickness_at_pitch,"toothCentersMm":profile.tooth_centers}),
    )
}

pub(super) fn bevel(v: &Value) -> Result<Value> {
    let pressure: f64 = field(v, "pressure_angle")?;
    let cone: f64 = field(v, "pitch_cone_angle")?;
    let spec = nurbs_core::involute::BevelGearSpec {
        teeth: field(v, "teeth")?,
        module: field(v, "module")?,
        pitch_cone_angle: cone.to_radians(),
        pressure_angle: pressure.to_radians(),
        face_width: field(v, "face_width")?,
        addendum: field(v, "addendum")?,
        dedendum: field(v, "dedendum")?,
        backlash: field(v, "backlash")?,
    };
    let g = brep_core::gear::bevel_with_verified_profile(
        spec,
        field(v, "tolerance_mm")?,
        Default::default(),
    )?;
    Ok(
        json!({"model":g.model(),"worldErrorUpperMm":g.world_error_upper(),"solidGeometryCertified":g.solid_geometry_certified()}),
    )
}
