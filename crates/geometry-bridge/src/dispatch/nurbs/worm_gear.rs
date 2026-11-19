use crate::{Result, field};
use nurbs_core::worm::{Hand, Spec};
use value_codec::{Value, json};

fn source(v: &Value) -> Result<Spec> {
    let pressure: f64 = field(v, "pressure_angle")?;
    Ok(Spec {
        axial_module: field(v, "module")?,
        diameter_quotient: field(v, "diameter_quotient")?,
        pressure_angle: pressure.to_radians(),
        dedendum_factor: field(v, "dedendum_factor")?,
        backlash: field(v, "backlash")?,
        starts: field(v, "starts")?,
        turns: field(v, "turns")?,
        hand: if field(v, "left_handed")? {
            Hand::Left
        } else {
            Hand::Right
        },
        error_budget: field(v, "tolerance_mm")?,
    })
}

pub(super) fn worm(v: &Value) -> Result<Value> {
    let limits = [field(v, "z_min")?, field(v, "z_max")?];
    let body = brep_core::worm::finite_with_certificates(source(v)?, limits, Default::default())?;
    Ok(
        json!({"model":body.model(),"wallErrorUpperMm":body.source().source().certificate().error_upper,
        "originalWallCount":body.original_wall_indices().len(),"signedVolumeMm3":body.mass().signed_volume_mm3,
        "boundaryAgreement":body.boundary_agreement().complete,"solidGeometryCertified":body.solid_geometry_certified()}),
    )
}

pub(super) fn wheel(v: &Value) -> Result<Value> {
    let spec = nurbs_core::worm_wheel::Spec {
        worm: source(v)?,
        teeth: field(v, "teeth")?,
        start: field(v, "start_index")?,
        axial_shift: field(v, "axial_shift")?,
    };
    let options = brep_core::worm::WheelBuildOptions {
        y_breaks: field(v, "y_breaks")?,
        stock_radius: field(v, "stock_radius")?,
        tolerance: field(v, "tolerance_mm")?,
        ..Default::default()
    };
    let body = brep_core::worm::finite_wheel_with_certificates(spec, options)?;
    Ok(
        json!({"model":body.model(),"wallErrorUpperMm":body.wall_error_upper(),"solidGeometryCertified":body.solid_geometry_certified()}),
    )
}
