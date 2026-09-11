//! Own binary64 rational B-spline kernel. No C/C++ or geometry dependency.
//! No polygon-core, WASM, browser or application dependency.
//! CAD B-rep over these curves/surfaces lives in `brep-core`.
#![feature(
    try_blocks,
    gen_blocks,
    yield_expr,
    super_let,
    deref_patterns,
    yeet_expr
)]
#![allow(unused_features)]
pub mod curve;
pub mod edit;
pub mod surface;
use value_codec::{Deserialize, Serialize};
use value_codec::{Value, json};

pub use math_core::{Error, Result};
pub(crate) const INVALID_INPUT: &str = "NURBS_INVALID_INPUT";
pub(crate) const NUMERIC_ERROR: &str = "NURBS_NUMERIC_ERROR";
pub(crate) const RESOURCE_LIMIT: &str = "NURBS_RESOURCE_LIMIT";
pub(crate) fn input(message: impl Into<String>) -> Error {
    Error::new(INVALID_INPUT, message)
}
pub(crate) fn numeric_err(message: impl Into<String>) -> Error {
    Error::new(NUMERIC_ERROR, message)
}
pub(crate) fn resource(message: impl Into<String>) -> Error {
    Error::new(RESOURCE_LIMIT, message)
}
fn check(condition: bool, message: &str) -> Result<()> {
    math_core::ensure(condition, INVALID_INPUT, message)
}
fn numeric(condition: bool, message: &str) -> Result<()> {
    math_core::ensure(condition, NUMERIC_ERROR, message)
}
fn field<T: for<'a> Deserialize<'a>>(v: &Value, k: &str) -> Result<T> {
    value_codec::from_value(v[k].clone()).map_err(|e| input(format!("Invalid {k}: {e}")))
}
fn encode<T: Serialize>(v: T) -> Result<Value> {
    value_codec::to_value(v).map_err(|e| numeric_err(e.to_string()))
}
pub fn dispatch(v: Value) -> Result<Value> {
    let op: String = field(&v, "op")?;
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
#[cfg(test)]
mod tests;
