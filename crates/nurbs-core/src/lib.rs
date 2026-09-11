//! Own binary64 rational B-spline kernel. No C/C++ or geometry dependency.
//! No polygon-core, WASM, browser or application dependency.
//! CAD B-rep over these curves/surfaces lives in `brep-kernel`.
pub mod curve;
pub mod edit;
pub mod surface;
use value_codec::{json, Value};
use value_codec::{Deserialize, Serialize};

#[derive(Debug)]
pub struct Error {
    pub code: &'static str,
    pub message: String,
}
impl value_codec::Serialize for Error {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert("code".into(), value_codec::Serialize::to_value(&self.code));
        object.insert(
            "message".into(),
            value_codec::Serialize::to_value(&self.message),
        );
        value_codec::Value::Object(object)
    }
}
pub type Result<T> = std::result::Result<T, Error>;
impl Error {
    fn input(message: impl Into<String>) -> Self {
        Self {
            code: "NURBS_INVALID_INPUT",
            message: message.into(),
        }
    }
    fn numeric(message: impl Into<String>) -> Self {
        Self {
            code: "NURBS_NUMERIC_ERROR",
            message: message.into(),
        }
    }
    fn resource(message: impl Into<String>) -> Self {
        Self {
            code: "NURBS_RESOURCE_LIMIT",
            message: message.into(),
        }
    }
}
fn check(condition: bool, message: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(Error::input(message))
    }
}
fn numeric(condition: bool, message: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(Error::numeric(message))
    }
}
fn field<T: for<'a> Deserialize<'a>>(v: &Value, k: &str) -> Result<T> {
    value_codec::from_value(v[k].clone()).map_err(|e| Error::input(format!("Invalid {k}: {e}")))
}
fn encode<T: Serialize>(v: T) -> Result<Value> {
    value_codec::to_value(v).map_err(|e| Error::numeric(e.to_string()))
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
            _ => Err(Error::input("Unknown curve operation")),
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
        _ => Err(Error::input("Unknown NURBS operation")),
    }
}
fn response(result: Result<Value>) -> String {
    match result {
        Ok(value) => json!({"ok":true,"value":value}),
        Err(error) => json!({"ok":false,"error":error}),
    }
    .to_string()
}
/// Versioned, bounded JSON boundary; no host geometry fallback.
pub fn execute(input: &str) -> String {
    if input.len() > 2 * 1024 * 1024 {
        return response(Err(Error::resource("NURBS request exceeds 2 MiB")));
    }
    response(
        value_codec::from_str(input)
            .map_err(|e| Error::input(e.to_string()))
            .and_then(dispatch),
    )
}
#[cfg(test)]
mod tests;
