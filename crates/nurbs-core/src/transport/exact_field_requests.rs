use super::*;
use crate::editing::exact_derivative;

pub(super) fn curve_field(v: &Value, op: &str) -> Result<Value> {
    let source = field(v, "source")?;
    let budget = field(v, "max_work")?;
    let (pieces, error_upper, work_used) = if op.starts_with("bounded_") {
        use crate::editing::bounded_differential as bounded;
        let r = match op {
            "bounded_curve_torsion_field" => {
                bounded::torsion(&source, field(v, "field_error")?, budget)?
            }
            _ => {
                let kind = match op {
                    "bounded_curve_derivative_field" => {
                        bounded::Field::Derivative(field(v, "order")?)
                    }
                    "bounded_curve_speed_squared_field" => bounded::Field::SquaredSpeed,
                    "bounded_curve_binormal_field" => bounded::Field::Binormal,
                    "bounded_curve_curvature_squared_field" => bounded::Field::SquaredCurvature,
                    _ => return Err(input("Unknown bounded curve field")),
                };
                bounded::curve(&source, kind, field(v, "field_error")?, budget)?
            }
        };
        (
            r.pieces
                .ok_or_else(|| input(format!("Bounded field refused: {:?}", r.refusal)))?,
            r.error_upper,
            r.work_used,
        )
    } else {
        let r = match op {
            "curve_derivative_field" => {
                exact_derivative::curve_order(&source, field(v, "order")?, budget)?
            }
            "curve_speed_squared_field" => exact_derivative::squared_speed(&source, budget)?,
            "curve_binormal_field" => exact_derivative::binormal(&source, budget)?,
            "curve_curvature_squared_field" => {
                exact_derivative::squared_curvature(&source, budget)?
            }
            "curve_torsion_field" => exact_derivative::torsion(&source, budget)?,
            _ => return Err(input("Unknown exact curve field")),
        };
        (
            r.pieces.ok_or_else(|| {
                input(format!("Exact field construction refused: {:?}", r.refusal))
            })?,
            None,
            r.work_used,
        )
    };
    let index: usize = field(v, "piece")?;
    let curve = pieces
        .get(index)
        .ok_or_else(|| input("Unknown exact field span"))?;
    let [a, b] = curve.domain();
    let parameter: f64 = field(v, "parameter")?;
    check(
        (a..=b).contains(&parameter),
        "Evaluation parameter is outside the selected field span",
    )?;
    Ok(
        json!({"curve":curve,"spanCount":pieces.len(),"domain":[a,b],"parameter":parameter,
        "value":curve.evaluate(parameter)?.point,"workUsed":work_used,"continuousErrorUpper":error_upper}),
    )
}

pub(super) fn surface_field(v: &Value, op: &str) -> Result<Value> {
    use crate::editing::exact_surface_derivative as field_ops;
    use crate::surface::Axis;
    let source = field(v, "source")?;
    let budget = field(v, "max_work")?;
    let (patches, error_upper, work_used) = if op.starts_with("bounded_") {
        use crate::editing::bounded_surface_differential as bounded;
        let r = match op {
            "bounded_surface_gaussian_field" => {
                bounded::gaussian_curvature(&source, field(v, "field_error")?, budget)?
            }
            "bounded_surface_mean_squared_field" => {
                bounded::squared_mean_curvature(&source, field(v, "field_error")?, budget)?
            }
            "bounded_surface_principal_gap_squared_field" => {
                bounded::squared_principal_curvature_gap(&source, field(v, "field_error")?, budget)?
            }
            _ => {
                let kind = match op {
                    "bounded_surface_partial_field" => {
                        match field::<String>(v, "direction")?.as_str() {
                            "u" => bounded::Field::Partial(Axis::U),
                            "v" => bounded::Field::Partial(Axis::V),
                            _ => return Err(input("Unknown derivative direction")),
                        }
                    }
                    "bounded_surface_mixed_partial_field" => {
                        bounded::Field::Mixed(field(v, "orders")?)
                    }
                    "bounded_surface_area_normal_field" => bounded::Field::AreaNormal,
                    "bounded_surface_metric_field" => bounded::Field::Metric,
                    _ => return Err(input("Unknown bounded surface field")),
                };
                bounded::surface(&source, kind, field(v, "field_error")?, budget)?
            }
        };
        (
            r.patches
                .ok_or_else(|| input(format!("Bounded field refused: {:?}", r.refusal)))?,
            r.error_upper,
            r.work_used,
        )
    } else {
        let result = match op {
            "surface_partial_field" => {
                let direction: String = field(v, "direction")?;
                let axis = match direction.as_str() {
                    "u" => Axis::U,
                    "v" => Axis::V,
                    _ => return Err(input("Unknown surface derivative direction")),
                };
                field_ops::surface(&source, axis, budget)?
            }
            "surface_area_normal_field" => field_ops::area_normal(&source, budget)?,
            "surface_metric_field" => field_ops::metric(&source, budget)?,
            "surface_mixed_partial_field" => {
                field_ops::mixed_partial(&source, field(v, "orders")?, budget)?
            }
            "surface_gaussian_field" => field_ops::gaussian_curvature(&source, budget)?,
            "surface_mean_squared_field" => field_ops::squared_mean_curvature(&source, budget)?,
            "surface_principal_gap_squared_field" => {
                field_ops::squared_principal_curvature_gap(&source, budget)?
            }
            _ => return Err(input("Unknown exact surface field")),
        };
        (
            result.patches.ok_or_else(|| {
                input(format!(
                    "Exact field construction refused: {:?}",
                    result.refusal
                ))
            })?,
            None,
            result.work_used,
        )
    };
    let patch: usize = field(v, "patch")?;
    let surface = patches
        .get(patch)
        .ok_or_else(|| input("Unknown field patch"))?;
    let u_range = [
        surface.knots_u[surface.degree_u],
        surface.knots_u[surface.control_points.len()],
    ];
    let v_range = [
        surface.knots_v[surface.degree_v],
        surface.knots_v[surface.control_points[0].len()],
    ];
    let [u, w]: [f64; 2] = field(v, "uv")?;
    check(
        (u_range[0]..=u_range[1]).contains(&u) && (v_range[0]..=v_range[1]).contains(&w),
        "UV outside selected field patch",
    )?;
    Ok(
        json!({"surface":surface,"patchCount":patches.len(),"domain":[u_range,v_range],"uv":[u,w],
        "value":surface.evaluate(u,w)?.point,"workUsed":work_used,"continuousErrorUpper":error_upper}),
    )
}
