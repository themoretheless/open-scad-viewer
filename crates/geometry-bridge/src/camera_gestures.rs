//! Browser gesture transport; all camera arithmetic belongs to math-core.
use super::{Result, Value, field, input};
use math_core::camera_gestures::{self as gestures, State};
use value_codec::json;
fn number(v: &Value, name: &str) -> Result<f64> {
    if let Some(n) = v[name].as_f64() {
        return Ok(n);
    }
    match v[name].as_str() {
        Some("NaN") => Ok(f64::NAN),
        Some("Infinity") => Ok(f64::INFINITY),
        Some("-Infinity") => Ok(f64::NEG_INFINITY),
        _ => Err(input("Invalid gesture number")),
    }
}
pub fn dispatch(v: Value) -> Result<Value> {
    let action = v["action"]
        .as_str()
        .ok_or_else(|| input("Missing gesture action"))?;
    if action == "clamp" {
        return Ok(json!(gestures::clamp_distance(number(&v, "distance")?)));
    }
    if action == "wheel" {
        return Ok(json!(gestures::wheel(
            number(&v, "distance")?,
            number(&v, "delta")?
        )));
    }
    let s = &v["state"];
    let state = State {
        yaw: field(s, "yaw")?,
        pitch: field(s, "pitch")?,
        dist: field(s, "dist")?,
        tx: field(s, "tx")?,
        ty: field(s, "ty")?,
        tz: field(s, "tz")?,
    };
    let result = match action {
        "orbit" => gestures::orbit(state, field(&v, "dx")?, field(&v, "dy")?),
        "pan" => gestures::pan(
            state,
            field(&v, "dx")?,
            field(&v, "dy")?,
            field(&v, "height")?,
            field(&v, "fov")?,
        ),
        "pinch" => gestures::pinch(
            field(&v, "points")?,
            state,
            field(&v, "height")?,
            field(&v, "fov")?,
        ),
        _ => return Err(input("Unknown camera gesture")),
    }
    .ok_or_else(|| input("Invalid or unrepresentable camera gesture"))?;
    Ok(
        json!({"yaw":result.yaw,"pitch":result.pitch,"dist":result.dist,"tx":result.tx,"ty":result.ty,"tz":result.tz}),
    )
}
