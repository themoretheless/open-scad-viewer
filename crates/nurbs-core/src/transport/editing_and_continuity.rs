//! Distance, trimming, reparameterization, resource probes, sculpting, Coons, and continuity.
use super::*;

pub(super) fn try_dispatch(v: &Value, op: &str) -> Option<Result<Value>> {
    if let Some(res) = try_dispatch_distance_trim_and_probes(v, op) {
        return Some(res);
    }
    try_dispatch_sculpt_coons_and_continuity(v, op)
}

fn try_dispatch_distance_trim_and_probes(v: &Value, op: &str) -> Option<Result<Value>> {
    let res = match op {
        "surface_distance" => (|| {
            Ok(surface_distance::distance(
                &field(v, "a")?,
                &field(v, "b")?,
                field(v, "toleranceMm")?,
                field(v, "maxCells")?,
            )?
            .to_value())
        })(),
        "curve_distance" => (|| {
            Ok(curve_distance::distance(
                &field(v, "a")?,
                &field(v, "b")?,
                field(v, "toleranceMm")?,
                field(v, "maxCells")?,
            )?
            .to_value())
        })(),
        "curve_trim_screen_point" => (|| {
            trim_point::trim_at_screen_point(
                &field(v, "curve")?,
                &field::<Vec<f64>>(v, "point")?,
                &field(v, "matrix")?,
                &field::<String>(v, "keep")?,
                field(v, "radius")?,
            )
        })(),
        "curve_trim_point" => (|| {
            trim_point::trim_at_point(
                &field(v, "curve")?,
                &field::<Vec<f64>>(v, "point")?,
                &field::<String>(v, "keep")?,
                field(v, "maxDistance")?,
            )
        })(),
        "curve_reparameterize_exact" => (|| {
            foundation::reparameterize_curve(
                &field(v, "curve")?,
                field(v, "domain")?,
                optional_field(v, "tolerance")?,
            )
        })(),
        "curve_reparameterized_evaluate" => (|| {
            foundation::evaluate_reparameterized_curve(
                &field(v, "curve")?,
                &v["mapping"],
                field(v, "u")?,
                optional_field(v, "tolerance")?,
            )
        })(),
        "curve_materialize_reparameterization_bounded" => (|| {
            foundation::materialize_reparameterized_curve_bounded(
                &field(v, "curve")?,
                &v["mapping"],
                field(v, "errorBudget")?,
                field(v, "maxCells")?,
                field(v, "maxMapEvaluations")?,
            )
        })(),
        "curve_materialize_reparameterization" => (|| {
            foundation::materialize_reparameterized_curve(
                &field(v, "curve")?,
                &v["mapping"],
                optional_field(v, "tolerance")?,
            )
        })(),
        "surface_surface_verify_coverage" => ss_intersection::verify_ss_coverage(&v["report"]),
        "nurbs_intersection_resource_probe" => (|| {
            intersection::resource_boundary_probe(field(v, "degree")?, field(v, "controls")?)
        })(),
        "nurbs_ss_resource_probe" => (|| {
            ss_intersection::ss_resource_probe(
                field(v, "degreeU")?,
                field(v, "degreeV")?,
                field(v, "controls")?,
            )
        })(),
        _ => return None,
    };
    Some(res)
}

fn try_dispatch_sculpt_coons_and_continuity(v: &Value, op: &str) -> Option<Result<Value>> {
    let res = match op {
        "nurbs_deform_curve" => (|| {
            encode(edit::deform_curve(
                &field(v, "curve")?,
                &field(v, "deformation")?,
            )?)
        })(),
        "nurbs_deform_surface" => (|| {
            encode(edit::deform_surface(
                &field(v, "surface")?,
                &field(v, "deformation")?,
            )?)
        })(),
        "nurbs_brush_curve" => {
            (|| encode(edit::brush_curve(&field(v, "curve")?, &field(v, "brush")?)?))()
        }
        "nurbs_sculpt_curve" => (|| {
            encode(edit::sculpt_curve(
                &field(v, "curve")?,
                &field(v, "brush")?,
            )?)
        })(),
        "nurbs_sculpt_surface" => (|| {
            encode(edit::sculpt_surface(
                &field(v, "surface")?,
                &field(v, "brush")?,
            )?)
        })(),
        "nurbs_brush_surface" => (|| {
            encode(edit::brush_surface(
                &field(v, "surface")?,
                &field(v, "brush")?,
            )?)
        })(),
        "basis" => (|| {
            encode(curve::basis(
                field(v, "degree")?,
                &field::<Vec<f64>>(v, "knots")?,
                field(v, "controlCount")?,
                field(v, "u")?,
                v["periodic"].as_bool().unwrap_or(false),
            )?)
        })(),
        "curve_prepare_coons_weights" => (|| {
            let result =
                coons::checked_endpoint_weights(&field(v, "curve")?, field(v, "maxError")?)?;
            Ok(json!({
                "curve": result.curve,
                "report": {
                    "accepted": result.accepted,
                    "errorUpper": result.error_upper,
                    "budget": field::<f64>(v, "maxError")?,
                    "wholeCurve": true,
                    "method": "outward-homogeneous-Bernstein-difference"
                }
            }))
        })(),
        "surface_coons_patch" => {
            (|| encode(coons::patch(&field::<Vec<curve::Curve>>(v, "boundaries")?)?))()
        }
        "curve_match_g1" => (|| {
            continuity::curve_match::checked(
                &field(v, "reference")?,
                &field(v, "edited")?,
                &field::<String>(v, "referenceEnd")?,
                &field::<String>(v, "editedEnd")?,
                optional_field::<f64>(v, "maxAngleDegrees")?.unwrap_or(1e-6),
            )
        })(),
        "surface_prepare_seams" => (|| {
            continuity::preparation::checked_with_conversion(
                &field(v, "reference")?,
                &field(v, "edited")?,
                &field::<String>(v, "referenceBoundary")?,
                &field::<String>(v, "editedBoundary")?,
                optional_field::<bool>(v, "reverse")?.unwrap_or(false),
                field(v, "maxError")?,
                optional_field::<bool>(v, "openPeriodic")?.unwrap_or(false),
            )
        })(),
        "surface_match_jets" => (|| {
            continuity::match_surface_jets_checked(
                &field(v, "reference")?,
                &field(v, "edited")?,
                &field::<String>(v, "referenceBoundary")?,
                &field::<String>(v, "editedBoundary")?,
                field(v, "order")?,
                field(v, "scale")?,
                optional_field::<bool>(v, "reverse")?.unwrap_or(false),
                optional_field::<f64>(v, "maxError")?.unwrap_or(1e-6),
            )
        })(),
        _ => return None,
    };
    Some(res)
}
