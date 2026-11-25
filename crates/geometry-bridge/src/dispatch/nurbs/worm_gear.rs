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

fn wheel_spec(
    v: &Value,
) -> Result<(
    nurbs_core::worm_wheel::Spec,
    brep_core::worm::WheelBuildOptions,
)> {
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
    Ok((spec, options))
}

pub(super) fn wheel(v: &Value) -> Result<Value> {
    let (spec, options) = wheel_spec(v)?;
    let body = brep_core::worm::finite_wheel_with_certificates(spec, options)?;
    wheel_report(&body)
}

fn wheel_report(body: &brep_core::worm::WheelBodyCandidate) -> Result<Value> {
    Ok(
        json!({"model":body.model(),"wallErrorUpperMm":body.wall_error_upper(),"solidGeometryCertified":body.solid_geometry_certified()}),
    )
}

enum WheelStage {
    Sides(nurbs_core::worm_wheel::AxialNetworkBuild),
    Caps(nurbs_core::worm_wheel::AxialSideNetwork),
    Body(brep_core::worm::WheelCaps),
}

struct WheelJob {
    id: u32,
    tolerance: f64,
    caps: brep_core::worm::WheelCapLimits,
    stage: WheelStage,
}

thread_local! {
    static WHEEL: std::cell::RefCell<Option<WheelJob>> = const { std::cell::RefCell::new(None) };
    static LAST_ID: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

pub(super) fn wheel_start(v: &Value) -> Result<Value> {
    if WHEEL.with_borrow(|slot| slot.is_some()) {
        return Err(crate::input("Wheel build already active"));
    }
    let (spec, options) = wheel_spec(v)?;
    let build = nurbs_core::worm_wheel::AxialNetworkBuild::new(
        spec,
        options.y_breaks,
        options.stock_radius,
        options.family_breaks,
        options.tolerance,
        options.max_patches_per_family,
    )?;
    let id = LAST_ID
        .get()
        .checked_add(1)
        .ok_or_else(|| crate::input("Wheel build identifiers exhausted"))?;
    LAST_ID.set(id);
    WHEEL.set(Some(WheelJob {
        id,
        tolerance: options.tolerance,
        caps: options.caps,
        stage: WheelStage::Sides(build),
    }));
    Ok(json!(id))
}

pub(super) fn wheel_cancel(v: &Value) -> Result<Value> {
    let id: u32 = field(v, "job")?;
    WHEEL.with_borrow_mut(|slot| {
        if slot.as_ref().is_some_and(|w| w.id == id) {
            *slot = None;
        }
    });
    Ok(Value::Null)
}

pub(super) fn wheel_step(v: &Value) -> Result<Value> {
    let id: u32 = field(v, "job")?;
    let mut job = WHEEL
        .with_borrow_mut(|slot| slot.take_if(|job| job.id == id))
        .ok_or_else(|| crate::input("Unknown wheel build"))?;
    job.stage = match job.stage {
        WheelStage::Sides(mut build) => {
            if build.advance()? {
                WheelStage::Caps(build.finish()?)
            } else {
                WheelStage::Sides(build)
            }
        }
        WheelStage::Caps(sides) => {
            WheelStage::Body(brep_core::worm::wheel_caps(sides, job.tolerance, job.caps)?)
        }
        WheelStage::Body(caps) => {
            return wheel_report(&brep_core::worm::wheel_body_candidate(caps)?);
        }
    };
    WHEEL.set(Some(job));
    Ok(Value::Null)
}
