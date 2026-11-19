use crate::{Result, field};
use nurbs_core::thread::{Hand, Kind, Spec};
use value_codec::{Value, json};

pub(super) fn build(value: &Value, internal: bool) -> Result<Value> {
    let spec = Spec {
        diameter: field(value, "diameter")?,
        pitch: field(value, "pitch")?,
        clearance: field(value, "clearance")?,
        starts: field(value, "starts")?,
        turns: field(value, "turns")?,
        error_budget: field(value, "tolerance_mm")?,
        hand: if field(value, "left_handed")? {
            Hand::Left
        } else {
            Hand::Right
        },
        kind: if internal {
            Kind::Internal
        } else {
            Kind::External
        },
    };
    let span = [field(value, "z_min")?, field(value, "z_max")?];
    let body = if internal {
        brep_core::thread::finite_internal_with_certificates(
            spec,
            span,
            field(value, "outer_radius")?,
            Default::default(),
        )?
    } else {
        brep_core::thread::finite_external_with_certificates(spec, span, Default::default())?
    };
    let patch_error = body.patch_certificate().error_upper;
    let axial_error = body.trim_certificate().axial_error_band_upper;
    let boundary_agreement = body.boundary_agreement().complete;
    let retained_walls = body.original_wall_indices().len();
    Ok(
        json!({"model":body.into_model(),"patchErrorUpperMm":patch_error,"axialErrorBandUpperMm":axial_error,"boundaryAgreement":boundary_agreement,"originalWallCount":retained_walls}),
    )
}
