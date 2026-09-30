//! Optional bounded JSON request boundary.
use super::*;
use value_codec::{Deserialize, Serialize, Value, json};

fn field<T: for<'a> Deserialize<'a>>(v: &Value, k: &str) -> Result<T> {
    value_codec::from_value(v[k].clone()).map_err(|e| input(format!("Invalid {k}: {e}")))
}
fn encode<T: Serialize>(v: T) -> Result<Value> {
    value_codec::to_value(v).map_err(|e| numeric_err(e.to_string()))
}
fn optional_field<T: for<'a> Deserialize<'a>>(v: &Value, k: &str) -> Result<Option<T>> {
    match v.get(k) {
        Some(value) if !value.is_null() => value_codec::from_value(value.clone())
            .map(Some)
            .map_err(|e| input(format!("Invalid {k}: {e}"))),
        _ => Ok(None),
    }
}
pub fn dispatch(v: Value) -> Result<Value> {
    let op: String = field(&v, "op")?;
    if op == "curve_certify_foundation" {
        return foundation::certify_curve(&field(&v, "curve")?, optional_field(&v, "tolerance")?);
    }
    if op == "surface_certify_foundation" {
        return foundation::certify_surface(
            &field(&v, "surface")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "surface_distance" {
        return Ok(surface_distance::distance(
            &field(&v, "a")?,
            &field(&v, "b")?,
            field(&v, "toleranceMm")?,
            field(&v, "maxCells")?,
        )?
        .to_value());
    }
    if op == "curve_distance" {
        return Ok(curve_distance::distance(
            &field(&v, "a")?,
            &field(&v, "b")?,
            field(&v, "toleranceMm")?,
            field(&v, "maxCells")?,
        )?
        .to_value());
    }
    if op == "curve_trim_screen_point" {
        return trim_point::trim_at_screen_point(
            &field(&v, "curve")?,
            &field::<Vec<f64>>(&v, "point")?,
            &field(&v, "matrix")?,
            &field::<String>(&v, "keep")?,
            field(&v, "radius")?,
        );
    }
    if op == "curve_trim_point" {
        return trim_point::trim_at_point(
            &field(&v, "curve")?,
            &field::<Vec<f64>>(&v, "point")?,
            &field::<String>(&v, "keep")?,
            field(&v, "maxDistance")?,
        );
    }
    if op == "curve_project_certified" {
        return foundation::project_curve(
            &field(&v, "curve")?,
            &field::<Vec<f64>>(&v, "point")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "surface_project_certified" {
        return foundation::project_surface(
            &field(&v, "surface")?,
            field(&v, "point")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "curve_interpolate_certified" {
        return foundation::interpolate_polyline(
            field(&v, "points")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "curve_approximate_certified" {
        return foundation::approximate_curve(
            &field(&v, "curve")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "curve_reparameterize_exact" {
        return foundation::reparameterize_curve(
            &field(&v, "curve")?,
            field(&v, "domain")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "curve_insert_certified" {
        return foundation::certify_exact_edit(
            &field(&v, "curve")?,
            "insert",
            field(&v, "u")?,
            field(&v, "count")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "curve_elevate_certified" {
        return foundation::certify_exact_edit(
            &field(&v, "curve")?,
            "elevate",
            0.,
            field(&v, "degree")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "curve_remove_certified" {
        return foundation::remove_curve_knot(
            &field(&v, "curve")?,
            field(&v, "u")?,
            field(&v, "maxError")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "curve_rebuild_certified" {
        return foundation::rebuild_curve(
            &field(&v, "curve")?,
            field(&v, "degree")?,
            field(&v, "controlCount")?,
            field(&v, "maxError")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "curve_reduce_certified" {
        return foundation::reduce_curve_degree(
            &field(&v, "curve")?,
            field(&v, "degree")?,
            field(&v, "maxError")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "surface_rebuild_certified" {
        return foundation::rebuild_surface(
            &field(&v, "surface")?,
            field(&v, "axis")?,
            field(&v, "degree")?,
            field(&v, "controlCount")?,
            field(&v, "maxError")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "surface_remove_certified" || op == "surface_reduce_certified" {
        return foundation::reduce_surface_axis(
            &field(&v, "surface")?,
            field(&v, "axis")?,
            if op == "surface_remove_certified" {
                "remove"
            } else {
                "reduce"
            },
            v.get("u").and_then(Value::as_f64).unwrap_or(0.),
            v.get("degree").and_then(Value::as_u64).unwrap_or(1) as usize,
            field(&v, "maxError")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "curve_periodic_edit_certified" {
        let operation: String = field(&v, "operation")?;
        return foundation::edit_periodic_curve(
            &field(&v, "curve")?,
            &operation,
            v.get("u").and_then(Value::as_f64).unwrap_or(0.),
            v.get("degree").and_then(Value::as_u64).unwrap_or(1) as usize,
            field(&v, "maxError")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "curve_periodic_split_certified" {
        return foundation::split_periodic_curve(
            &field(&v, "curve")?,
            field(&v, "u")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "surface_periodic_edit_certified" {
        let operation: String = field(&v, "operation")?;
        return foundation::edit_periodic_surface(
            &field(&v, "surface")?,
            field(&v, "axis")?,
            &operation,
            v.get("u").and_then(Value::as_f64).unwrap_or(0.),
            v.get("degree").and_then(Value::as_u64).unwrap_or(1) as usize,
            field(&v, "maxError")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "curve_reparameterization_certify" {
        return foundation::certify_reparameterization(
            &v["mapping"],
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "curve_reparameterized_evaluate" {
        return foundation::evaluate_reparameterized_curve(
            &field(&v, "curve")?,
            &v["mapping"],
            field(&v, "u")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "curve_fit_certified" {
        return foundation::fit_curve_points(
            field(&v, "points")?,
            field(&v, "controlCount")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "surface_interpolate_certified" || op == "surface_fit_certified" {
        return foundation::interpolate_surface_grid(
            field(&v, "points")?,
            op == "surface_fit_certified",
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "curve_materialize_reparameterization" {
        return foundation::materialize_reparameterized_curve(
            &field(&v, "curve")?,
            &v["mapping"],
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "curve_fit_cloud_certified" {
        return foundation::fit_curve_cloud_certified(
            field(&v, "points")?,
            field(&v, "controlCount")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "surface_fit_cloud_certified" {
        return foundation::fit_surface_cloud_certified(
            field(&v, "points")?,
            field(&v, "controlsU")?,
            field(&v, "controlsV")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "curve_curve_intersect_certified" {
        return intersection::intersect_curve_curve(
            &field(&v, "first")?,
            &field(&v, "second")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "curve_surface_intersect_certified" {
        return intersection::intersect_curve_surface(
            &field(&v, "curve")?,
            &field(&v, "surface")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "surface_surface_intersect_certified" {
        return ss_intersection::intersect_surface_surface(
            &field(&v, "first")?,
            &field(&v, "second")?,
            optional_field(&v, "tolerance")?,
        );
    }
    if op == "surface_surface_verify_coverage" {
        return ss_intersection::verify_ss_coverage(&v["report"]);
    }
    if op == "nurbs_intersection_resource_probe" {
        return intersection::resource_boundary_probe(field(&v, "degree")?, field(&v, "controls")?);
    }
    if op == "nurbs_ss_resource_probe" {
        return ss_intersection::ss_resource_probe(
            field(&v, "degreeU")?,
            field(&v, "degreeV")?,
            field(&v, "controls")?,
        );
    }
    if op == "nurbs_deform_curve" {
        return encode(edit::deform_curve(
            &field(&v, "curve")?,
            &field(&v, "deformation")?,
        )?);
    }
    if op == "nurbs_deform_surface" {
        return encode(edit::deform_surface(
            &field(&v, "surface")?,
            &field(&v, "deformation")?,
        )?);
    }
    if op == "nurbs_brush_curve" {
        return encode(edit::brush_curve(
            &field(&v, "curve")?,
            &field(&v, "brush")?,
        )?);
    }
    if op == "nurbs_sculpt_curve" {
        return encode(edit::sculpt_curve(
            &field(&v, "curve")?,
            &field(&v, "brush")?,
        )?);
    }
    if op == "nurbs_sculpt_surface" {
        return encode(edit::sculpt_surface(
            &field(&v, "surface")?,
            &field(&v, "brush")?,
        )?);
    }
    if op == "nurbs_brush_surface" {
        return encode(edit::brush_surface(
            &field(&v, "surface")?,
            &field(&v, "brush")?,
        )?);
    }
    if op == "basis" {
        return encode(curve::basis(
            field(&v, "degree")?,
            &field::<Vec<f64>>(&v, "knots")?,
            field(&v, "controlCount")?,
            field(&v, "u")?,
            v["periodic"].as_bool().unwrap_or(false),
        )?);
    }
    if op == "curve_prepare_coons_weights" {
        let result=coons::checked_endpoint_weights(&field(&v,"curve")?,field(&v,"maxError")?)?;
        return Ok(json!({"curve":result.curve,"report":{"accepted":result.accepted,
            "errorUpper":result.error_upper,"budget":field::<f64>(&v,"maxError")?,
            "wholeCurve":true,"method":"outward-homogeneous-Bernstein-difference"}}));
    }
    if op == "surface_coons_patch" {
        return encode(coons::patch(&field::<Vec<curve::Curve>>(
            &v,
            "boundaries",
        )?)?);
    }
    if op == "curve_match_g1" {
        return continuity::curve_match::checked(
            &field(&v, "reference")?,
            &field(&v, "edited")?,
            &field::<String>(&v, "referenceEnd")?,
            &field::<String>(&v, "editedEnd")?,
            optional_field::<f64>(&v, "maxAngleDegrees")?.unwrap_or(1e-6),
        );
    }
    if op == "surface_prepare_seams" {
        return continuity::preparation::checked_with_conversion(
            &field(&v, "reference")?,
            &field(&v, "edited")?,
            &field::<String>(&v, "referenceBoundary")?,
            &field::<String>(&v, "editedBoundary")?,
            optional_field::<bool>(&v, "reverse")?.unwrap_or(false),
            field(&v, "maxError")?,
            optional_field::<bool>(&v, "openPeriodic")?.unwrap_or(false),
        );
    }
    if op == "surface_match_jets" {
        return continuity::match_surface_jets_checked(
            &field(&v, "reference")?,
            &field(&v, "edited")?,
            &field::<String>(&v, "referenceBoundary")?,
            &field::<String>(&v, "editedBoundary")?,
            field(&v, "order")?,
            field(&v, "scale")?,
            optional_field::<bool>(&v, "reverse")?.unwrap_or(false),
            optional_field::<f64>(&v, "maxError")?.unwrap_or(1e-6),
        );
    }
    if op == "surface_framed_sweep" {
        return framed_sweep::checked_sweep(
            &field(&v, "profile")?,
            &field(&v, "path")?,
            field(&v, "normal")?,
            field(&v, "sections")?,
            field(&v, "maxDeviation")?,
        );
    }
    if op == "surface_sweep" {
        return encode(surface::sweep(&field(&v, "profile")?, &field(&v, "path")?)?);
    }
    if op == "loft_aligned" {
        return encode(surface::loft_aligned(&field::<Vec<curve::Curve>>(
            &v, "curves",
        )?)?);
    }
    if op == "loft" {
        return encode(surface::loft(&field::<Vec<curve::Curve>>(&v, "curves")?)?);
    }
    if op.starts_with("curve_") || op == "extrude" || op == "revolve" {
        let c: curve::Curve = field(&v, "curve")?;
        return match op.as_str() {
            "curve_validate" => {
                c.validate()?;
                Ok(Value::Null)
            }
            "curve_evaluate" => encode(c.evaluate(field(&v, "u")?)?),
            "curve_insert" => encode(c.insert(field(&v, "u")?, field(&v, "count")?)?),
            "curve_trim" => encode(c.trim(field(&v, "a")?, field(&v, "b")?)?),
            "curve_split" => encode(c.split(field(&v, "u")?)?),
            "curve_reverse" => encode(c.reverse()?),
            "curve_elevate" => encode(c.elevate(field(&v, "degree")?)?),
            "curve_decompose" => encode(c.decompose()?),
            "curve_bounds" => c.bounds(),
            "extrude" => encode(surface::extrude(&c, field(&v, "vector")?)?),
            "revolve" => encode(surface::revolve(
                &c,
                field(&v, "origin")?,
                field(&v, "axis")?,
                field(&v, "angle")?,
            )?),
            _ => Err(input("Unknown curve operation")),
        };
    }
    let s: surface::Surface = field(&v, "surface")?;
    match op.as_str() {
        "surface_validate" => {
            s.validate()?;
            Ok(Value::Null)
        }
        "surface_evaluate" => encode(s.evaluate(field(&v, "u")?, field(&v, "v")?)?),
        "surface_insert" => encode(s.edit_axis(field(&v, "axis")?, |c| {
            c.insert(field(&v, "u")?, field(&v, "count")?)
        })?),
        "surface_elevate" => {
            encode(s.edit_axis(field(&v, "axis")?, |c| c.elevate(field(&v, "degree")?))?)
        }
        "surface_reverse" => encode(s.edit_axis(field(&v, "axis")?, |c| c.reverse())?),
        "surface_trim" => encode(s.trim(field(&v, "bounds")?)?),
        "surface_iso" => encode(s.iso(field(&v, "axis")?, field(&v, "u")?)?),
        "surface_bounds" => s.bounds(),
        _ => Err(input("Unknown NURBS operation")),
    }
}
fn error_value(error: &Error) -> Value {
    json!({"code": error.code, "message": error.message})
}
fn response(result: Result<Value>) -> String {
    match result {
        Ok(value) => json!({"ok":true,"value":value}),
        Err(error) => json!({"ok":false,"error": error_value(&error)}),
    }
    .to_string()
}
/// Versioned, bounded JSON boundary; no host geometry fallback.
pub fn execute(request: &str) -> String {
    if request.len() > 2 * 1024 * 1024 {
        return response(Err(resource("NURBS request exceeds 2 MiB")));
    }
    response(
        value_codec::from_str(request)
            .map_err(|e| input(e.to_string()))
            .and_then(dispatch),
    )
}
