//! Direct curve and surface evaluation, knot/degree edits, extrude, revolve, and bounds.
use super::*;

pub(super) fn dispatch(v: Value, op: &str) -> Result<Value> {
    if op.starts_with("curve_") || op == "extrude" || op == "revolve" {
        let c: curve::Curve = field(&v, "curve")?;
        return match op {
            "curve_validate" => {
                c.validate()?;
                Ok(Value::Null)
            }
            "curve_evaluate" => encode(c.evaluate(field(&v, "u")?)?),
            "curve_evaluate_points" => {
                encode(c.evaluate_points(&field::<Vec<f64>>(&v, "parameters")?)?)
            }
            "curve_sample" => encode(c.sample_uniform(
                field(&v, "segments")?,
                optional_field::<bool>(&v, "includeEnd")?.unwrap_or(true),
            )?),
            "curve_insert" => encode(c.insert(field(&v, "u")?, field(&v, "count")?)?),
            "curve_trim" => encode(c.trim(field(&v, "a")?, field(&v, "b")?)?),
            "curve_split" => encode(c.split(field(&v, "u")?)?),
            "curve_reverse" => encode(c.reverse()?),
            "curve_elevate" => encode(c.elevate(field(&v, "degree")?)?),
            "curve_decompose" => encode(c.decompose()?),
            "curve_decomposition_audit" => {
                let r = crate::curve_decomposition_certificate::inspect(
                    &c,
                    field(&v, "span")?,
                    &field(&v, "retained")?,
                    field(&v, "maxProducts")?,
                )?;
                Ok(json!({
                    "errorUpper": r.error_upper,
                    "products": r.products,
                    "reason": r.reason,
                    "method": "original-span-bernstein-decomposition",
                    "continuousBound": false
                }))
            }
            "curve_bounds" => encode(c.bounds()?),
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
    match op {
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
        "surface_bounds" => encode(s.bounds()?),
        _ => Err(input("Unknown NURBS operation")),
    }
}
