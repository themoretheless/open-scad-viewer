use super::*;

pub(super) fn dispatch(v: Value, op: String) -> Result<Value> {
if matches!(op.as_str(), "curve_decomposition_audit") {
    if op == "curve_decomposition_audit" {
        let r=crate::curve_decomposition_certificate::inspect(&field(&v,"curve")?,field(&v,"span")?,&field(&v,"retained")?,field(&v,"maxProducts")?)?;
        return Ok(json!({"errorUpper":r.error_upper,"products":r.products,"reason":r.reason,"method":"original-span-bernstein-decomposition","continuousBound":false}));
    }
    }
if matches!(op.as_str(), "surface_boundary_v_tangent_certify") {
    if op == "surface_boundary_v_tangent_certify" {
        return gordon::certify_boundary_v_tangent(&field(&v,"surface")?,
            &field::<Vec<curve::Curve>>(&v,"targets")?,field(&v,"end")?,
            field(&v,"tolerance")?,field(&v,"maxCells")?);
    }
    }
if op == "curve_certify_foundation" {
        return foundation::certify_curve(&field(&v, "curve")?, optional_field(&v, "tolerance")?);
    }
if op == "surface_certify_foundation" {
        return foundation::certify_surface(
            &field(&v, "surface")?,
            optional_field(&v, "tolerance")?,
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
Err(input("Unknown NURBS operation"))
}
